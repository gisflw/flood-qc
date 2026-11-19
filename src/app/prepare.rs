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
use super::request::{
    PreparationDocument, ValidatedRequest, read_request, resolve_request, validate_document,
};
use crate::analysis::{ForecastPreparationRequest, prepare_forecast, prepare_zero_forecast};
use crate::store::{
    BasinForcing, ForecastCatalog, ForecastSnapshotSelection, ObservationBatch, OperationManifest,
    ScenarioRecord, collect_inputs, copy_tree, digest_file, finalize_prepared, sync_tree,
    write_basin_forcing, write_grid_frame, write_grid_series, write_run_manifest,
};
use crate::store::{ForecastCorrection, ObservationReplacement};
use std::fs::{self, File};
use std::path::PathBuf;

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

#[derive(Clone, Debug)]
pub struct TypedPreparationRequest {
    pub workspace: PathBuf,
    pub invocation_dir: PathBuf,
    pub output: PathBuf,
    pub document: PreparationDocument,
    pub replacements: Option<Vec<ObservationReplacement>>,
    /// One entry per scenario; None resolves the scenario's correction IDs.
    pub corrections: Vec<Option<Vec<ForecastCorrection>>>,
}

pub fn prepare_typed(
    request: TypedPreparationRequest,
) -> Result<PreparationReport, OperationError> {
    prepare_typed_checked(request, None, false, &super::OperationControl::default())
}
pub(super) fn prepare_typed_checked(
    mut request: TypedPreparationRequest,
    identities: Option<std::collections::BTreeMap<String, String>>,
    operational: bool,
    control: &super::OperationControl,
) -> Result<PreparationReport, OperationError> {
    if request.corrections.len() != request.document.scenarios.len() {
        return Err(OperationError::InvalidRequest {
            field: "scenarios".into(),
            message: "correction selections must match scenarios".into(),
        });
    }
    if let Some(values) = &request.replacements {
        request.document.observed.replacement_ids =
            values.iter().map(|v| v.replacement_id.clone()).collect();
    }
    for (scenario, values) in request
        .document
        .scenarios
        .iter_mut()
        .zip(&request.corrections)
    {
        if let Some(values) = values {
            scenario.correction_ids =
                Some(values.iter().map(|v| v.correction_id.clone()).collect());
        }
    }
    let bytes = toml::to_string(&request.document)
        .map_err(|e| OperationError::InvalidRequest {
            field: "request".into(),
            message: e.to_string(),
        })?
        .into_bytes();
    let accepted =
        validate_document(request.document, bytes).map_err(|e| OperationError::InvalidRequest {
            field: e.field,
            message: e.message,
        })?;
    let workspace = if request.workspace.is_absolute() {
        request.workspace.clone()
    } else {
        request.invocation_dir.join(&request.workspace)
    }
    .canonicalize()
    .map_err(|e| source_error(&request.workspace, e))?;
    let accepted =
        resolve_request(&workspace, accepted).map_err(|e| OperationError::InvalidRequest {
            field: e.field,
            message: e.message,
        })?;
    let output = if request.output.is_absolute() {
        request.output
    } else {
        request.invocation_dir.join(request.output)
    };
    prepare_accepted(
        workspace,
        output,
        accepted,
        request.replacements,
        request.corrections,
        PathBuf::from("typed request"),
        identities,
        operational,
        control,
    )
}

pub fn prepare(request: PreparationRequest) -> Result<PreparationReport, OperationError> {
    prepare_controlled(request, &super::OperationControl::default())
}
pub fn prepare_controlled(
    request: PreparationRequest,
    control: &super::OperationControl,
) -> Result<PreparationReport, OperationError> {
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
    let corrections = vec![None; accepted.document.scenarios.len()];
    prepare_accepted(
        workspace,
        output,
        accepted,
        None,
        corrections,
        request_path,
        None,
        false,
        control,
    )
}

#[allow(clippy::too_many_arguments)] // Keep accepted inputs and their source identities explicit.
fn prepare_accepted(
    workspace: PathBuf,
    output: PathBuf,
    accepted: ValidatedRequest,
    replacements: Option<Vec<ObservationReplacement>>,
    corrections: Vec<Option<Vec<ForecastCorrection>>>,
    request_path: PathBuf,
    identities: Option<std::collections::BTreeMap<String, String>>,
    operational: bool,
    control: &super::OperationControl,
) -> Result<PreparationReport, OperationError> {
    control.set_progress(super::OperationStage::Preparation, None, None);
    control.check()?;
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
        super::additional::check_engine(doc, &doc.engine.executable)
            .map_err(|e| source_error(&doc.engine.executable, e))?;
        let geometry = crate::model::read_basin_geometry(&doc.geometry.mini_gtp)
            .map_err(|e| source_error(&doc.geometry.mini_gtp, e))?;
        let edited_geometry = doc
            .calibration
            .as_ref()
            .is_some_and(|selection| selection.use_edited_geometry);
        let geometry_digest = if edited_geometry || identities.is_some() {
            Some(
                digest_file(&doc.geometry.mini_gtp)
                    .map_err(|e| source_error(&doc.geometry.mini_gtp, e))?,
            )
        } else {
            None
        };
        let soil_identity = super::additional::soil_identity(doc.calibration.as_ref())
            .map_err(|e| source_error(&request_path, e))?;
        if let Some(identities) = &identities {
            let actual = super::draft::identity(geometry_digest.as_ref().unwrap())
                .map_err(|e| source_error(&doc.geometry.mini_gtp, e))?;
            if identities.get("geometry") != Some(&actual) {
                return Err(source_error(
                    &doc.geometry.mini_gtp,
                    "geometry changed or has not been accepted; refresh inputs",
                ));
            }
            if soil_identity
                .as_ref()
                .is_some_and(|value| identities.get("soil") != Some(value))
            {
                return Err(source_error(
                    &request_path,
                    "soil calibration changed or has not been accepted; refresh inputs",
                ));
            }
        }
        control.check()?;
        let selection = ObservationSelection {
            provider_codes: doc.observed.provider_codes.clone(),
            station_ids: doc.observed.station_ids.clone(),
        };
        let store = ObservationStore::open_read_only(&doc.observed.path)
            .map_err(|e| source_error(&doc.observed.path, e))?;
        let extra_stations = super::additional::extra_stations(&store, doc)
            .map_err(|e| source_error(&doc.observed.path, e))?;
        let (mut inputs, offsets) = store
            .read_selection_with_stations(
                &selection,
                &accepted.historical,
                if replacements.is_some() {
                    &[]
                } else {
                    &doc.observed.replacement_ids
                },
                &extra_stations,
            )
            .map_err(|e| source_error(&doc.observed.path, e))?;
        if let Some(identities) = &identities {
            let actual = super::draft::identity(&offsets)
                .map_err(|e| source_error(&doc.observed.path, e))?;
            if identities.get("offsets") != Some(&actual) {
                return Err(source_error(
                    &doc.observed.path,
                    "station offsets changed or have not been accepted; refresh inputs",
                ));
            }
        }
        let source_replacements = inputs.replacements.clone();
        if let Some(values) = replacements {
            inputs.replacements = values;
        }
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
        if let Some(identities) = &identities {
            let mut source = inputs.clone();
            source.replacements = source_replacements.clone();
            let actual =
                super::draft::observation_identity(&source, &accepted.historical, &selection)
                    .map_err(|e| source_error(&doc.observed.path, e))?;
            if identities.get("observations") != Some(&actual) {
                return Err(source_error(
                    &doc.observed.path,
                    "observation source changed or has not been accepted; refresh inputs",
                ));
            }
        }
        if let Some(additional) =
            super::additional::capture(&accepted, &inputs, &source_replacements)
                .map_err(|e| source_error(&request_path, e))?
        {
            if identities.as_ref().is_some_and(|values| {
                values.get("additional_forcing") != Some(&additional.accepted)
            }) {
                return Err(source_error(
                    &request_path,
                    "additional forcing changed or has not been accepted; refresh inputs",
                ));
            }
            for (name, bytes) in additional.raw_files {
                let path = stage.join("sources").join(name);
                fs::write(&path, bytes).map_err(|e| source_error(&path, e))?;
            }
            write_json(&stage.join("forcing/additional.json"), &additional.prepared)?;
        }
        for offset in &offsets {
            if !geometry.mini_ids.contains(&offset.mini_id) {
                return Err(source_error(
                    &doc.observed.path,
                    "offset references unknown basin",
                ));
            }
        }
        let obs_path = stage.join("sources/observations.sqlite");
        let mut subset = ObservationStore::create(&obs_path, &doc.time_zone)
            .map_err(|e| source_error(&obs_path, e))?;
        if inputs
            .replacements
            .iter()
            .any(|item| item.variable == "flow")
        {
            subset
                .upgrade_replacements()
                .map_err(|e| source_error(&obs_path, e))?;
        }
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
        control.check()?;
        let observed = prepare_observed_forcing(
            &inputs,
            &accepted.historical,
            &accepted.grid,
            &geometry,
            &accepted.interpolation,
            &accepted.qc,
        )
        .map_err(|e| source_error(&doc.observed.path, e))?;
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
            .zip(&corrections)
            .filter_map(|(scenario, supplied)| {
                scenario
                    .asset_id
                    .as_ref()
                    .map(|id| ForecastSnapshotSelection {
                        asset_id: id.clone(),
                        correction_ids: scenario.correction_ids.clone().unwrap_or_default(),
                        corrections: supplied.clone(),
                    })
            })
            .collect();
        control.check()?;
        let mut selected_forecasts = if selections.is_empty() {
            Vec::new().into_iter()
        } else {
            let catalog = ForecastCatalog::open_read_only(&workspace, &doc.forecasts.catalog)
                .map_err(|e| source_error(&doc.forecasts.catalog, e))?;
            let selected = catalog
                .snapshot_selected_values(&stage.join("sources/forecast.sqlite"), &selections)
                .map_err(|e| source_error(&doc.forecasts.catalog, e))?;
            if let Some(identities) = &identities {
                let mut selected_iter = selected.iter();
                for (index, scenario) in doc.scenarios.iter().enumerate() {
                    if scenario.asset_id.is_none() {
                        continue;
                    }
                    let (_, native, values) = selected_iter.next().ok_or_else(|| {
                        source_error(&doc.forecasts.catalog, "missing selected forecast")
                    })?;
                    let empty = Vec::new();
                    let registered = if corrections[index].is_some() {
                        &empty
                    } else {
                        values
                    };
                    let actual = super::draft::identity(&(native, registered))
                        .map_err(|e| source_error(&doc.forecasts.catalog, e))?;
                    if identities.get(&format!("forecast:{index}")) != Some(&actual) {
                        return Err(source_error(
                            &doc.forecasts.catalog,
                            "forecast source changed or has not been accepted; refresh inputs",
                        ));
                    }
                }
            }
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
                        stage.join(asset.snapshot_source_path()),
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
            for ((asset, _, _), selection) in selected.iter().zip(&selections) {
                catalog
                    .load_selected(
                        &asset.asset_id,
                        if selection.corrections.is_some() {
                            &[]
                        } else {
                            &selection.correction_ids
                        },
                    )
                    .map_err(|e| source_error(&doc.forecasts.catalog, e))?;
            }
            selected.into_iter()
        };
        let mut correction_values = Vec::new();
        control.check()?;
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
        copy_tree(
            &doc.engine.executable,
            &stage.join(crate::store::PREPARED_ENGINE),
        )
        .map_err(|e| source_error(&doc.engine.executable, e))?;
        if let Some(identities) = &identities {
            let engine = stage.join(crate::store::PREPARED_ENGINE);
            let inputs = stage.join("baseline/Input");
            let actual_engine = super::draft::identity(
                &digest_file(&engine).map_err(|e| source_error(&engine, e))?,
            )
            .map_err(|e| source_error(&engine, e))?;
            let actual_inputs = super::draft::identity(
                &collect_inputs(&inputs).map_err(|e| source_error(&inputs, e))?,
            )
            .map_err(|e| source_error(&inputs, e))?;
            if identities.get("engine") != Some(&actual_engine)
                || identities.get("engine_inputs") != Some(&actual_inputs)
            {
                return Err(source_error(
                    &doc.engine.input_dir,
                    "engine inputs changed or have not been accepted; refresh inputs",
                ));
            }
        }
        let stage_input = stage.join("baseline/Input");
        let soil = doc
            .calibration
            .as_ref()
            .and_then(|selection| selection.soil.as_deref());
        if edited_geometry || soil.is_some() {
            crate::model::install_calibration(
                &stage_input,
                edited_geometry.then_some(doc.geometry.mini_gtp.as_path()),
                soil,
            )
            .map_err(|e| source_error(&stage_input, e))?;
            if edited_geometry
                && Some(
                    digest_file(&stage_input.join("Cell.hig"))
                        .map_err(|e| source_error(&stage_input, e))?,
                ) != geometry_digest
            {
                return Err(source_error(
                    &doc.geometry.mini_gtp,
                    "geometry changed while freezing",
                ));
            }
            if let Some(expected) = soil_identity {
                let actual = digest_file(&stage_input.join("paruso.hig"))
                    .and_then(|value| super::draft::identity(&value))
                    .map_err(|e| source_error(&stage_input, e))?;
                if actual != expected {
                    return Err(source_error(
                        &stage_input,
                        "soil calibration changed while freezing",
                    ));
                }
            }
        }
        super::additional::check_engine(doc, &stage.join(crate::store::PREPARED_ENGINE))
            .map_err(|e| source_error(&doc.engine.executable, e))?;
        control.check()?;
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
            operational_workspace: operational.then(|| workspace.clone()),
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
            preparation_sha256: None,
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
        control.check()?;
        sync_tree(&stage).map_err(|e| source_error(&stage, e))?;
        write_run_manifest(&stage, &manifest).map_err(|e| source_error(&stage, e))?;
        let (manifest_sha256, _) =
            digest_file(&stage.join("manifest.json")).map_err(|e| source_error(&stage, e))?;
        control.check()?;
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
    if result.is_err() && stage.exists() {
        fs::remove_dir_all(&stage).map_err(|e| {
            source_error(
                &stage,
                format!("preparation failed and cleanup failed: {e}"),
            )
        })?;
    }
    result
}
