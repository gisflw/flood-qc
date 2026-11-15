use crate::analysis::{
    ObservedPreparation, PrecipitationQcPolicy, prepare_observed_forcing, select_observation_series,
};
use crate::store::{
    BasinGeometry, IdwSettings, IntervalAxis, ObservationError, ObservationInputs,
    ObservationSelection, ObservationStore, RegularGridSpec, StationOffset,
};
use std::path::Path;

pub struct ObservedRequest<'a> {
    pub path: &'a Path,
    pub selection: &'a ObservationSelection,
    pub series_ids: &'a [String],
    pub replacement_ids: &'a [String],
    pub timeline: &'a IntervalAxis,
    pub grid: &'a RegularGridSpec,
    pub basins: &'a BasinGeometry,
    pub interpolation: &'a IdwSettings,
    pub qc: &'a PrecipitationQcPolicy,
}
pub struct ObservedInputs {
    pub inputs: ObservationInputs,
    pub offsets: Vec<StationOffset>,
    pub preparation: ObservedPreparation,
}

pub fn prepare_observations(
    request: &ObservedRequest<'_>,
) -> Result<ObservedInputs, ObservationError> {
    let observation_store = ObservationStore::open_read_only(request.path)?;
    let (mut inputs, offsets) = observation_store.read_selection(
        request.selection,
        request.timeline,
        request.replacement_ids,
    )?;
    let chosen = select_observation_series(&inputs.series, request.series_ids)?;
    if chosen.is_empty() {
        return Err(ObservationError("selection has no rainfall series".into()));
    }
    inputs
        .series
        .retain(|s| s.variable != "rain" || chosen.iter().any(|c| c.series_id == s.series_id));
    inputs.samples.retain(|sample| {
        inputs
            .series
            .iter()
            .any(|series| series.series_id == sample.series_id)
    });
    for offset in &offsets {
        if !request.basins.mini_ids.contains(&offset.mini_id) {
            return Err(ObservationError(format!(
                "station {}: unknown mini ID {}",
                offset.station_id, offset.mini_id
            )));
        }
    }
    let preparation = prepare_observed_forcing(
        &inputs,
        request.timeline,
        request.grid,
        request.basins,
        request.interpolation,
        request.qc,
    )?;
    Ok(ObservedInputs {
        inputs,
        offsets,
        preparation,
    })
}

use super::operation::{ExecutionRecipe, OperationError, PreparationReport, PreparationRequest};
use super::request::read_request;
use crate::analysis::{ForecastPreparationRequest, prepare_forecast, prepare_zero_forecast};
use crate::store::{
    BasinForcing, ForecastCatalog, ObservationBatch, OperationManifest, ScenarioRecord,
    collect_inputs, copy_tree, digest_file, finalize_prepared, sync_tree, write_basin_forcing,
    write_grid_frame, write_grid_series, write_run_manifest,
};
use std::fs::{self, File};

fn source_error(path: &Path, error: impl ToString) -> OperationError {
    OperationError::Source {
        path: path.to_path_buf(),
        message: error.to_string(),
    }
}
fn write_json(path: &Path, value: &impl serde::Serialize) -> Result<(), OperationError> {
    let file = File::create(path).map_err(|e| source_error(path, e))?;
    serde_json::to_writer_pretty(&file, value).map_err(|e| source_error(path, e))?;
    file.sync_all().map_err(|e| source_error(path, e))
}
fn join_forcing(history: &BasinForcing, forecast: &BasinForcing) -> Result<BasinForcing, String> {
    history.validate().map_err(|e| e.to_string())?;
    forecast.validate().map_err(|e| e.to_string())?;
    if history.basins != forecast.basins
        || history.timeline.time_zone != forecast.timeline.time_zone
        || history.timeline.bounds.last().map(|v| v.end)
            != forecast.timeline.bounds.first().map(|v| v.start)
    {
        return Err("forcing geometry, timezone or interval join mismatch".into());
    }
    let mut combined = history.clone();
    combined
        .timeline
        .bounds
        .extend(forecast.timeline.bounds.iter().cloned());
    combined.values.extend_from_slice(&forecast.values);
    combined.valid.extend_from_slice(&forecast.valid);
    combined
        .provenance
        .source_ids
        .extend(forecast.provenance.source_ids.iter().cloned());
    if combined
        .values
        .iter()
        .zip(&combined.valid)
        .any(|(value, valid)| !valid || !value.is_finite() || *value < 0.0)
    {
        return Err("missing, nonfinite or negative engine rainfall".into());
    }
    combined.validate().map_err(|e| e.to_string())?;
    Ok(combined)
}

pub fn prepare(request: PreparationRequest) -> Result<PreparationReport, OperationError> {
    let resolve = |path: &Path| {
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            request.invocation_dir.join(path)
        }
    };
    let workspace = resolve(&request.workspace)
        .canonicalize()
        .map_err(|e| source_error(&request.workspace, e))?;
    let request_path = resolve(&request.request_file);
    let output = resolve(&request.output);
    let accepted =
        read_request(&workspace, &request_path).map_err(|e| OperationError::InvalidRequest {
            field: e.field,
            message: e.message,
        })?;
    if output.exists() {
        return Err(OperationError::InvalidRequest {
            field: "output".into(),
            message: "destination already exists".into(),
        });
    }
    let parent = output
        .parent()
        .ok_or_else(|| OperationError::InvalidRequest {
            field: "output".into(),
            message: "destination has no parent".into(),
        })?;
    let name = output
        .file_name()
        .ok_or_else(|| OperationError::InvalidRequest {
            field: "output".into(),
            message: "destination has no name".into(),
        })?
        .to_string_lossy();
    let stage = parent.join(format!(
        ".{name}.staging-{}-{}",
        std::process::id(),
        chrono::Utc::now().format("%Y%m%d%H%M%S%.9f")
    ));
    fs::create_dir(&stage).map_err(|e| source_error(&stage, e))?;
    let result = (|| -> Result<PreparationReport, OperationError> {
        for dir in [
            "sources",
            "sources/native",
            "sources/original",
            "baseline",
            "forcing",
            "checkpoints",
            "checkpoints/forecast",
        ] {
            fs::create_dir(stage.join(dir)).map_err(|e| source_error(&stage, e))?;
        }
        fs::write(stage.join("request.toml"), &accepted.request_bytes)
            .map_err(|e| source_error(&stage, e))?;
        let doc = &accepted.document;
        let geometry = crate::model::read_basin_geometry(&doc.geometry.mini_gtp)
            .map_err(|e| source_error(&doc.geometry.mini_gtp, e))?;
        let selection = ObservationSelection {
            provider_codes: doc.observed.provider_codes.clone(),
            station_ids: doc.observed.station_ids.clone(),
        };
        let store = ObservationStore::open_read_only(&doc.observed.path)
            .map_err(|e| source_error(&doc.observed.path, e))?;
        let (mut inputs, offsets) = store
            .read_selection_with_stations(
                &selection,
                &accepted.historical,
                &doc.observed.replacement_ids,
                &doc.levels.station_ids,
            )
            .map_err(|e| source_error(&doc.observed.path, e))?;
        let selected = select_observation_series(
            &inputs
                .series
                .iter()
                .filter(|series| {
                    inputs.stations.iter().any(|station| {
                        station.station_id == series.station_id
                            && (selection.provider_codes.contains(&station.provider_code)
                                || selection.station_ids.contains(&station.station_id))
                    })
                })
                .cloned()
                .collect::<Vec<_>>(),
            &doc.observed.series_ids,
        )
        .map_err(|e| source_error(&doc.observed.path, e))?;
        if selected.is_empty() {
            return Err(source_error(
                &doc.observed.path,
                "selection has no rainfall series",
            ));
        }
        inputs.series.retain(|series| {
            series.variable != "rain"
                || selected
                    .iter()
                    .any(|chosen| chosen.series_id == series.series_id)
        });
        inputs.samples.retain(|sample| {
            inputs
                .series
                .iter()
                .any(|series| series.series_id == sample.series_id)
        });
        for offset in &offsets {
            if !geometry.mini_ids.contains(&offset.mini_id) {
                return Err(source_error(
                    &doc.observed.path,
                    "offset references unknown basin",
                ));
            }
        }
        let observed = prepare_observed_forcing(
            &inputs,
            &accepted.historical,
            &accepted.grid,
            &geometry,
            &accepted.interpolation,
            &accepted.qc,
        )
        .map_err(|e| source_error(&doc.observed.path, e))?;
        let obs_path = stage.join("sources/observations.sqlite");
        let mut subset = ObservationStore::create(&obs_path, &doc.time_zone)
            .map_err(|e| source_error(&obs_path, e))?;
        subset
            .write_batch(&ObservationBatch {
                stations: inputs.stations.clone(),
                series: inputs.series.clone(),
                samples: inputs.samples.clone(),
                replacements: inputs.replacements.clone(),
                offsets: offsets.clone(),
            })
            .map_err(|e| source_error(&obs_path, e))?;
        drop(subset);
        write_json(
            &stage.join("checkpoints/effective.json"),
            &observed.effective,
        )?;
        write_json(&stage.join("checkpoints/qc.json"), &observed.qc)?;
        write_grid_series(
            &stage.join("checkpoints/observed-grid.nc"),
            &observed.working,
        )
        .map_err(|e| source_error(&stage, e))?;
        write_basin_forcing(&stage.join("forcing/observed.nc"), &observed.basin)
            .map_err(|e| source_error(&stage, e))?;
        let selections: Vec<_> = doc
            .scenarios
            .iter()
            .filter_map(|scenario| {
                scenario.asset_id.as_ref().map(|id| {
                    (
                        id.clone(),
                        scenario.correction_ids.clone().unwrap_or_default(),
                    )
                })
            })
            .collect();
        let mut selected_forecasts = if selections.is_empty() {
            Vec::new().into_iter()
        } else {
            let catalog = ForecastCatalog::open_read_only(&workspace, &doc.forecasts.catalog)
                .map_err(|e| source_error(&doc.forecasts.catalog, e))?;
            let selected = catalog
                .snapshot_selected(&stage.join("sources/forecast.sqlite"), &selections)
                .map_err(|e| source_error(&doc.forecasts.catalog, e))?;
            for (asset, _, _) in &selected {
                for (source, target) in [
                    (
                        &asset.native_artifact,
                        stage
                            .join("sources/native")
                            .join(format!("{}.nc", asset.asset_id)),
                    ),
                    (
                        &asset.retained_source,
                        stage
                            .join("sources/original")
                            .join(format!("{}.grib2", asset.asset_id)),
                    ),
                ] {
                    if !target.exists() {
                        let source = workspace.join(source);
                        copy_tree(&source, &target).map_err(|e| source_error(&source, e))?;
                        if digest_file(&source).map_err(|e| source_error(&source, e))?
                            != digest_file(&target).map_err(|e| source_error(&target, e))?
                        {
                            return Err(source_error(&source, "source changed while copied"));
                        }
                    }
                }
            }
            for (asset, _, corrections) in &selected {
                catalog
                    .load_selected(
                        &asset.asset_id,
                        &corrections
                            .iter()
                            .map(|c| c.correction_id.clone())
                            .collect::<Vec<_>>(),
                    )
                    .map_err(|e| source_error(&doc.forecasts.catalog, e))?;
            }
            selected.into_iter()
        };
        let mut correction_values = Vec::new();
        for scenario in &doc.scenarios {
            let forecast = if scenario.kind == "zero" {
                prepare_zero_forecast(&accepted.forecast, &geometry)
                    .map_err(|e| source_error(&request_path, e))?
            } else {
                let (_, native, corrections) = selected_forecasts
                    .next()
                    .ok_or_else(|| source_error(&request_path, "missing selected forecast"))?;
                correction_values.extend(corrections.clone());
                let prepared = prepare_forecast(
                    &native,
                    &corrections,
                    &ForecastPreparationRequest {
                        timeline: accepted.forecast.clone(),
                        working_grid: accepted.grid.clone(),
                        basins: geometry.clone(),
                        interpolation: accepted.interpolation.clone(),
                    },
                )
                .map_err(|e| source_error(&request_path, e))?;
                let checkpoint = stage.join("checkpoints/forecast").join(&scenario.id);
                fs::create_dir(&checkpoint).map_err(|e| source_error(&checkpoint, e))?;
                write_grid_series(&checkpoint.join("working.nc"), &prepared.working)
                    .map_err(|e| source_error(&checkpoint, e))?;
                for (index, frame) in prepared.transformed.iter().enumerate() {
                    write_grid_frame(&checkpoint.join(format!("{index:04}.nc")), frame)
                        .map_err(|e| source_error(&checkpoint, e))?;
                }
                prepared.basin
            };
            let combined = join_forcing(&observed.basin, &forecast)
                .map_err(|e| source_error(&request_path, e))?;
            write_basin_forcing(
                &stage.join("forcing").join(format!("{}.nc", scenario.id)),
                &combined,
            )
            .map_err(|e| source_error(&stage, e))?;
        }
        copy_tree(&doc.engine.input_dir, &stage.join("baseline/Input"))
            .map_err(|e| source_error(&doc.engine.input_dir, e))?;
        copy_tree(&doc.engine.executable, &stage.join("baseline/engine"))
            .map_err(|e| source_error(&doc.engine.executable, e))?;
        let baseline_geometry =
            crate::model::read_basin_geometry(&stage.join("baseline/Input/MINI.gtp"))
                .map_err(|e| source_error(&stage, e))?;
        if geometry.mini_ids != baseline_geometry.mini_ids
            || geometry.longitudes != baseline_geometry.longitudes
            || geometry.latitudes != baseline_geometry.latitudes
        {
            return Err(source_error(
                &doc.geometry.mini_gtp,
                "baseline geometry differs from selected geometry",
            ));
        }
        let mut document = serde_json::to_value(doc).map_err(|e| source_error(&request_path, e))?;
        let object = document
            .as_object_mut()
            .ok_or_else(|| source_error(&request_path, "invalid request object"))?;
        object.insert(
            "selected_series".into(),
            serde_json::to_value(&inputs.series).map_err(|e| source_error(&request_path, e))?,
        );
        object.insert(
            "selected_stations".into(),
            serde_json::to_value(&inputs.stations).map_err(|e| source_error(&request_path, e))?,
        );
        object.insert(
            "replacement_values".into(),
            serde_json::to_value(&inputs.replacements)
                .map_err(|e| source_error(&request_path, e))?,
        );
        object.insert(
            "correction_values".into(),
            serde_json::to_value(correction_values).map_err(|e| source_error(&request_path, e))?,
        );
        let recipe = ExecutionRecipe {
            document,
            historical_count: accepted.historical.bounds.len(),
            forecast_count: accepted.forecast.bounds.len(),
            selected_station_ids: doc.levels.station_ids.clone(),
            offsets,
            omp_threads: doc.engine.omp_threads,
        };
        let inputs = collect_inputs(&stage).map_err(|e| source_error(&stage, e))?;
        let scenarios = doc
            .scenarios
            .iter()
            .map(|scenario| ScenarioRecord {
                id: scenario.id.clone(),
                state: "pending".into(),
                started_at: None,
                finished_at: None,
                exit_code: None,
                signal: None,
                error: None,
                stdout: None,
                stderr: None,
                basin_result: None,
                station_result: None,
            })
            .collect();
        let manifest = OperationManifest {
            schema_version: 1,
            state: "prepared".into(),
            recipe: serde_json::to_value(recipe).map_err(|e| source_error(&stage, e))?,
            inputs,
            scenarios,
            created_at: chrono::Utc::now()
                .with_timezone(
                    &doc.time_zone
                        .parse::<chrono_tz::Tz>()
                        .map_err(|e| source_error(&request_path, e))?,
                )
                .format("%Y-%m-%dT%H:%M:%S")
                .to_string(),
            finished_at: None,
            error: None,
        };
        sync_tree(&stage).map_err(|e| source_error(&stage, e))?;
        write_run_manifest(&stage, &manifest).map_err(|e| source_error(&stage, e))?;
        let (manifest_sha256, _) =
            digest_file(&stage.join("manifest.json")).map_err(|e| source_error(&stage, e))?;
        finalize_prepared(&stage, &output).map_err(|e| source_error(&output, e))?;
        Ok(PreparationReport {
            prepared: output.clone(),
            manifest_sha256,
            scenario_ids: doc
                .scenarios
                .iter()
                .map(|scenario| scenario.id.clone())
                .collect(),
            artifacts: manifest
                .inputs
                .iter()
                .map(|entry| output.join(&entry.path))
                .collect(),
        })
    })();
    if let Err(error) = &result {
        let _ = fs::write(stage.join("failure.txt"), error.to_string());
    }
    result
}
