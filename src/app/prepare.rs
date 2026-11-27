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
use crate::store::ObservationReplacement;
use crate::store::{
    BasinForcing, ForecastCatalog, ForecastSnapshotSelection, OperationManifest, ScenarioRecord,
    collect_inputs, copy_staged_tree, digest_file, finalize_prepared, sync_tree,
    write_basin_forcing, write_grid_series, write_run_manifest,
};
use std::fs;
use std::path::PathBuf;

fn source_error(path: &Path, error: impl ToString) -> OperationError {
    OperationError::Source {
        path: path.to_path_buf(),
        message: error.to_string(),
    }
}
fn write_json(path: &Path, value: &impl serde::Serialize) -> Result<(), OperationError> {
    crate::store::write_draft(path, value).map_err(|e| source_error(path, e))
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
}

fn resolved_from_document(
    document: &PreparationDocument,
) -> super::operational_run::ResolvedScenarios {
    let mut result: super::operational_run::ResolvedScenarios = Vec::new();
    for scenario in &document.scenarios {
        let forecast = scenario.asset_id.clone().or_else(|| {
            scenario.base.as_ref().and_then(|id| {
                document
                    .scenarios
                    .iter()
                    .position(|value| &value.id == id)
                    .and_then(|index| result.get(index))
                    .and_then(|item| item.forecast.clone())
            })
        });
        result.push(super::operational_run::ResolvedScenario {
            forecast,
            base: scenario.base.clone(),
            edits: scenario.edits.clone(),
        });
    }
    result
}

pub fn prepare_typed(
    request: TypedPreparationRequest,
) -> Result<PreparationReport, OperationError> {
    prepare_typed_internal(request, None, &super::OperationControl::default())
}

pub(super) fn prepare_workspace(
    request: TypedPreparationRequest,
    scenarios: super::operational_run::ResolvedScenarios,
    control: &super::OperationControl,
) -> Result<PreparationReport, OperationError> {
    prepare_typed_internal(request, Some(scenarios), control)
}

pub(super) fn capture_workspace(
    mut request: TypedPreparationRequest,
    scenarios: super::operational_run::ResolvedScenarios,
    control: &super::OperationControl,
) -> Result<PreparationReport, OperationError> {
    if scenarios.len() != request.document.scenarios.len() {
        return Err(source_error(
            &request.output,
            "resolved scenario membership differs",
        ));
    }
    if let Some(values) = &request.replacements {
        request.document.observed.replacement_ids =
            values.iter().map(|v| v.replacement_id.clone()).collect();
    }
    let bytes = toml::to_string(&request.document)
        .map_err(|e| source_error(&request.output, e))?
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
    let id = output
        .file_name()
        .and_then(|v| v.to_str())
        .and_then(|v| v.strip_prefix('.'))
        .ok_or_else(|| source_error(&output, "invalid input-set staging name"))?
        .to_string();
    if output.exists() {
        return Err(source_error(&output, "destination already exists"));
    }
    fs::create_dir(&output).map_err(|e| source_error(&output, e))?;
    let result = (|| {
        fs::create_dir(output.join("sources")).map_err(|e| source_error(&output, e))?;
        fs::create_dir(output.join("baseline")).map_err(|e| source_error(&output, e))?;
        control.check()?;
        let captured = capture_sources(
            &output,
            &workspace,
            &accepted,
            request.replacements,
            Path::new("typed request"),
            Some(scenarios),
            control,
        )?;
        let files = collect_inputs(&output).map_err(|e| source_error(&output, e))?;
        let manifest = crate::store::InputSetManifest {
            objects: Vec::new(),
            observation_snapshot: None,
            id,
            created_at: chrono::Utc::now().to_rfc3339(),
            engine: captured.recipe.engine_identity.clone(),
            recipe: serde_json::to_value(&captured.recipe).map_err(|e| source_error(&output, e))?,
            files,
        };
        crate::store::write_cache_json(&output.join("input-set.toml"), &manifest)
            .map_err(|e| source_error(&output, e))?;
        let manifest_sha256 = digest_file(&output.join("input-set.toml"))
            .map_err(|e| source_error(&output, e))?
            .0;
        Ok(PreparationReport {
            prepared: output.clone(),
            manifest_sha256,
            scenario_ids: accepted
                .document
                .scenarios
                .iter()
                .map(|s| s.id.clone())
                .collect(),
            artifacts: manifest
                .files
                .iter()
                .map(|f| output.join(&f.path))
                .collect(),
        })
    })();
    if result.is_err() {
        let _ = fs::remove_dir_all(&output);
    }
    result
}

fn prepare_typed_internal(
    mut request: TypedPreparationRequest,
    workspace_scenarios: Option<super::operational_run::ResolvedScenarios>,
    control: &super::OperationControl,
) -> Result<PreparationReport, OperationError> {
    if workspace_scenarios
        .as_ref()
        .is_some_and(|scenarios| scenarios.len() != request.document.scenarios.len())
    {
        return Err(OperationError::InvalidRequest {
            field: "scenarios".into(),
            message: "resolved scenario membership differs".into(),
        });
    }
    if let Some(values) = &request.replacements {
        request.document.observed.replacement_ids =
            values.iter().map(|v| v.replacement_id.clone()).collect();
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
    let workspace_scenarios =
        workspace_scenarios.unwrap_or_else(|| resolved_from_document(&accepted.document));
    prepare_accepted(
        workspace,
        output,
        accepted,
        request.replacements,
        PathBuf::from("typed request"),
        Some(workspace_scenarios),
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
    let scenarios = resolved_from_document(&accepted.document);
    prepare_accepted(
        workspace,
        output,
        accepted,
        None,
        request_path,
        Some(scenarios),
        control,
    )
}

struct CapturedSources {
    geometry: BasinGeometry,
    inputs: ObservationInputs,
    source_replacements: Vec<ObservationReplacement>,
    forecasts: Vec<(crate::store::ForecastAsset, crate::store::GridSeries)>,
    recipe: ExecutionRecipe,
    imerg: Option<crate::store::GridSeries>,
}

#[allow(clippy::too_many_arguments)] // The source selection and recipe are explicit inputs to freezing.
fn capture_sources(
    stage: &Path,
    workspace: &Path,
    accepted: &ValidatedRequest,
    replacements: Option<Vec<ObservationReplacement>>,
    request_path: &Path,
    workspace_scenarios: Option<super::operational_run::ResolvedScenarios>,
    control: &super::OperationControl,
) -> Result<CapturedSources, OperationError> {
    let doc = &accepted.document;
    super::additional::check_engine(doc, &doc.engine.executable)
        .map_err(|e| source_error(&doc.engine.executable, e))?;
    let edited_geometry = doc
        .calibration
        .as_ref()
        .is_some_and(|selection| selection.use_edited_geometry);
    let geometry_digest =
        digest_file(&doc.geometry.mini_gtp).map_err(|e| source_error(&doc.geometry.mini_gtp, e))?;
    let geometry = crate::model::read_basin_geometry(&doc.geometry.mini_gtp)
        .map_err(|e| source_error(&doc.geometry.mini_gtp, e))?;
    if digest_file(&doc.geometry.mini_gtp).map_err(|e| source_error(&doc.geometry.mini_gtp, e))?
        != geometry_digest
    {
        return Err(source_error(
            &doc.geometry.mini_gtp,
            "geometry changed while reading",
        ));
    }
    let soil_identity = super::additional::soil_identity(doc.calibration.as_ref())
        .map_err(|e| source_error(request_path, e))?;
    control.check()?;
    let selection = ObservationSelection {
        provider_codes: doc.observed.provider_codes.clone(),
        station_ids: doc.observed.station_ids.clone(),
    };
    let mut observation_revision = match doc.observed.revision {
        Some(revision) => revision,
        None => ObservationStore::open_read_only(&doc.observed.path)
            .and_then(|db| db.revision())
            .map_err(|e| source_error(&doc.observed.path, e))?,
    };
    let store = ObservationStore::open_revision(&doc.observed.path, observation_revision)
        .map_err(|e| source_error(&doc.observed.path, e))?;
    let extra_stations = super::additional::extra_stations(&store, doc)
        .map_err(|e| source_error(&doc.observed.path, e))?;
    let (mut inputs, offsets) = if selection.provider_codes.is_empty()
        && selection.station_ids.is_empty()
        && extra_stations.is_empty()
        && matches!(
            doc.observed.historical_rainfall,
            super::HistoricalRainfall::Imerg { .. }
        ) {
        (ObservationInputs::default(), Vec::new())
    } else {
        store
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
            .map_err(|e| source_error(&doc.observed.path, e))?
    };
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
    if selected.is_empty()
        && matches!(
            doc.observed.historical_rainfall,
            super::HistoricalRainfall::Stations
        )
    {
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
    if let Some(forcing) = &doc.forcing {
        for (source, name) in [
            (&forcing.reservoir_schedules, "reservoir-schedules.toml"),
            (&forcing.wind_observations, "wind-observations.toml"),
        ] {
            if let Some(source) = source {
                let target = stage.join("sources").join(name);
                copy_staged_tree(source, &target).map_err(|e| source_error(source, e))?;
            }
        }
    }
    let selected_ids: Vec<_> = selected.iter().map(|s| s.series_id.clone()).collect();
    let mut precipitation_flags = store
        .read_precipitation_flags(&selected_ids, &accepted.historical)
        .map_err(|e| source_error(&doc.observed.path, e))?;
    let workspace_selection = doc.observed.path == super::observation_assets(workspace);
    if workspace_selection
        && doc.observed.revision.is_none()
        && matches!(
            doc.observed.historical_rainfall,
            super::HistoricalRainfall::Stations
        )
    {
        precipitation_flags.clear();
        let raw: Vec<_> = inputs
            .samples
            .iter()
            .filter_map(|sample| {
                let series = selected.iter().find(|s| s.series_id == sample.series_id)?;
                let value = match sample.value {
                    Some(crate::store::ObservationValue::RainMm(value)) => Some(value),
                    _ => None,
                };
                Some(crate::analysis::SelectedObservation {
                    station_id: series.station_id.clone(),
                    series_id: series.series_id.clone(),
                    observed_at: sample.observed_at,
                    longitude: None,
                    latitude: None,
                    value,
                })
            })
            .collect();
        let found = crate::analysis::detect_suspect_precipitation(&raw, &accepted.qc)
            .map_err(|e| source_error(&doc.observed.path, e))?;
        for finding in found {
            if let Some(series) = selected.iter().find(|s| s.station_id == finding.station_id) {
                precipitation_flags.push((
                    series.series_id.clone(),
                    finding.observed_at,
                    finding.match_type,
                ));
            }
        }
        let selected_ids: Vec<_> = selected.iter().map(|s| s.series_id.clone()).collect();
        let (excluded, revision) = ObservationStore::open_writable(&doc.observed.path)
            .and_then(|mut db| {
                db.replace_precipitation_flags_for_revision(
                    observation_revision,
                    &selected_ids,
                    &accepted.historical,
                    &precipitation_flags,
                    &inputs.samples,
                )
            })
            .map_err(|e| source_error(&doc.observed.path, e))?;
        inputs.excluded_rain = excluded;
        observation_revision = revision;
    }
    for offset in &offsets {
        if !geometry.mini_ids.contains(&offset.mini_id) {
            return Err(source_error(
                &doc.observed.path,
                "offset references unknown basin",
            ));
        }
    }
    // Retain the exact canonical revision; values are resolved only through this pin.
    if !workspace_selection {
        let pinned = ObservationStore::open_revision(&doc.observed.path, observation_revision)
            .map_err(|e| source_error(&doc.observed.path, e))?;
        pinned
            .export_revision(&stage.join("sources/observations.sqlite"))
            .map_err(|e| source_error(stage, e))?;
    }
    control.check()?;
    let selections: Vec<_> = doc
        .scenarios
        .iter()
        .filter_map(|scenario| {
            scenario
                .asset_id
                .as_ref()
                .map(|id| ForecastSnapshotSelection {
                    asset_id: id.clone(),
                })
        })
        .collect();
    control.check()?;
    let selected_forecasts = if selections.is_empty() {
        Vec::new().into_iter()
    } else {
        let origin = doc
            .forecasts
            .catalog
            .parent()
            .and_then(Path::parent)
            .filter(|root| root.join("input-set.toml").is_file())
            .unwrap_or(workspace);
        let catalog = ForecastCatalog::open_read_only(origin, &doc.forecasts.catalog)
            .map_err(|e| source_error(&doc.forecasts.catalog, e))?;
        let destination = stage.join("sources/forecasts");
        let selected = catalog
            .snapshot_selected_values(&destination, &selections)
            .map_err(|e| source_error(&doc.forecasts.catalog, e))?;
        for (asset, _) in &selected {
            let target = destination.join(&asset.asset_id).join("native.nc");
            if !target.exists() {
                let source = origin.join(&asset.native_artifact);
                copy_staged_tree(&source, &target).map_err(|e| source_error(&source, e))?;
                if digest_file(&target).map_err(|e| source_error(&target, e))?
                    != catalog
                        .native_identity(&asset.asset_id)
                        .map_err(|e| source_error(&source, e))?
                    || digest_file(&source).map_err(|e| source_error(&source, e))?
                        != digest_file(&target).map_err(|e| source_error(&target, e))?
                {
                    return Err(source_error(
                        &source,
                        "native forecast changed while copied",
                    ));
                }
            }
        }
        selected.into_iter()
    };
    copy_staged_tree(&doc.engine.input_dir, &stage.join("baseline/Input"))
        .map_err(|e| source_error(&doc.engine.input_dir, e))?;
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
            && digest_file(&stage_input.join("Cell.hig"))
                .map_err(|e| source_error(&stage_input, e))?
                != geometry_digest
        {
            return Err(source_error(
                &doc.geometry.mini_gtp,
                "geometry changed while freezing",
            ));
        }
        if let Some(expected) = soil_identity {
            let actual = digest_file(&stage_input.join("paruso.hig"))
                .map_err(|e| source_error(&stage_input, e))?;
            if actual != expected {
                return Err(source_error(
                    &stage_input,
                    "soil calibration changed while freezing",
                ));
            }
        }
    }
    control.check()?;
    let baseline_geometry =
        crate::model::read_basin_geometry(&stage.join("baseline/Input/MINI.gtp"))
            .map_err(|e| source_error(stage, e))?;
    if geometry.mini_ids != baseline_geometry.mini_ids
        || geometry.longitudes != baseline_geometry.longitudes
        || geometry.latitudes != baseline_geometry.latitudes
    {
        return Err(source_error(
            &doc.geometry.mini_gtp,
            "baseline geometry differs from selected geometry",
        ));
    }
    let imerg = capture_imerg(workspace, stage, accepted)?;
    let mut document = serde_json::to_value(doc).map_err(|e| source_error(request_path, e))?;
    document["observed"]["revision"] = observation_revision.into();
    document["observed"]["path"] = doc.observed.path.to_string_lossy().to_string().into();
    let object = document
        .as_object_mut()
        .ok_or_else(|| source_error(request_path, "invalid request object"))?;
    object.insert(
        "selected_series".into(),
        serde_json::to_value(&inputs.series).map_err(|e| source_error(request_path, e))?,
    );
    object.insert(
        "selected_stations".into(),
        serde_json::to_value(&inputs.stations).map_err(|e| source_error(request_path, e))?,
    );
    object.insert(
        "replacement_values".into(),
        serde_json::to_value(&inputs.replacements).map_err(|e| source_error(request_path, e))?,
    );
    let (engine_sha256, engine_size) =
        digest_file(&doc.engine.executable).map_err(|e| source_error(&doc.engine.executable, e))?;
    let engine_reference = crate::store::InputReference {
        path: crate::store::PREPARED_ENGINE.into(),
        kind: "baseline".into(),
        sha256: engine_sha256,
        size: engine_size,
    };
    let recipe = ExecutionRecipe {
        observation_registry_id: Some(
            store
                .registry_id()
                .map_err(|e| source_error(&doc.observed.path, e))?,
        ),
        engine_identity: engine_reference,
        input_set_id: None,
        document,
        workspace_scenarios,
        historical_count: accepted.historical.bounds.len(),
        forecast_count: accepted.forecast.bounds.len(),
        selected_station_ids: doc.levels.station_ids.clone(),
        offsets,
        registered_replacements: source_replacements.clone(),
        omp_threads: doc.engine.omp_threads,
    };
    write_json(&stage.join("sources/recipe.toml"), &recipe)?;
    Ok(CapturedSources {
        geometry,
        inputs,
        source_replacements,
        forecasts: selected_forecasts.collect(),
        recipe,
        imerg,
    })
}

#[allow(clippy::too_many_arguments)] // The captured source state and operation configuration are explicit.
fn materialize_sources(
    stage: &Path,
    accepted: &ValidatedRequest,
    captured: CapturedSources,
    request_path: &Path,
    control: &super::OperationControl,
    durable: bool,
) -> Result<PreparationReport, OperationError> {
    let doc = &accepted.document;
    let CapturedSources {
        geometry,
        inputs,
        source_replacements,
        forecasts,
        recipe,
        imerg,
    } = captured;
    if let Some(additional) =
        super::additional::capture(stage, accepted, &inputs, &source_replacements)
            .map_err(|e| source_error(request_path, e))?
    {
        write_json(&stage.join("forcing/additional.toml"), &additional.prepared)?;
    }
    let observed = if let Some(working) = imerg {
        let basin = crate::analysis::interpolate_grid_to_basins(
            &working,
            &geometry,
            &accepted.interpolation,
        )
        .map_err(|e| source_error(stage, e))?;
        crate::analysis::ObservedPreparation {
            effective: Vec::new(),
            qc: Vec::new(),
            provenance: working.provenance.clone(),
            working,
            basin,
            replacement_diagnostics: Vec::new(),
        }
    } else {
        prepare_observed_forcing(
            &inputs,
            &accepted.historical,
            &accepted.grid,
            &geometry,
            &accepted.interpolation,
            &accepted.qc,
        )
        .map_err(|e| source_error(&doc.observed.path, e))?
    };
    write_json(
        &stage.join("checkpoints/effective.toml"),
        &observed.effective,
    )?;
    write_json(&stage.join("checkpoints/qc.toml"), &observed.qc)?;
    write_grid_series(
        &stage.join("checkpoints/observed-grid.nc"),
        &observed.working,
    )
    .map_err(|e| source_error(stage, e))?;
    write_basin_forcing(&stage.join("forcing/observed.nc"), &observed.basin)
        .map_err(|e| source_error(stage, e))?;
    let mut selected_forecasts = forecasts.into_iter();
    let workspace_scenarios = &recipe.workspace_scenarios;
    let mut working_scenarios = std::collections::HashMap::new();
    control.check()?;
    for (index, scenario) in doc.scenarios.iter().enumerate() {
        let selected = if scenario.asset_id.is_some() {
            Some(
                selected_forecasts
                    .next()
                    .ok_or_else(|| source_error(request_path, "missing selected forecast"))?,
            )
        } else {
            None
        };
        let simple_zero =
            scenario.asset_id.is_none() && scenario.base.is_none() && scenario.edits.is_empty();
        let forecast = if simple_zero
            && !doc
                .scenarios
                .iter()
                .any(|s| s.base.as_ref() == Some(&scenario.id))
        {
            crate::analysis::prepare_zero_forecast(&accepted.forecast, &geometry)
                .map_err(|e| source_error(request_path, e))?
        } else {
            let resolved = &workspace_scenarios.as_ref().expect("scenario recipe")[index];
            let working = if let Some(base) = &resolved.base {
                working_scenarios.get(base).cloned().ok_or_else(|| {
                    source_error(request_path, "base scenario must precede variant")
                })?
            } else if let Some((_, native)) = selected.as_ref() {
                crate::analysis::forecast_working_grid(native, &accepted.forecast, &accepted.grid)
                    .map_err(|e| source_error(request_path, e))?
            } else {
                let count = accepted.forecast.bounds.len()
                    * accepted.grid.latitudes.len()
                    * accepted.grid.longitudes.len();
                crate::store::GridSeries {
                    timeline: accepted.forecast.clone(),
                    grid: accepted.grid.clone(),
                    values: vec![0.; count],
                    valid: vec![true; count],
                    provenance: crate::store::Provenance {
                        source_ids: vec![],
                        processing: serde_json::json!({"kind":"zero"}),
                    },
                }
            };
            let working = crate::analysis::apply_scenario_grid_edits(&working, &resolved.edits)
                .map_err(|e| source_error(request_path, e))?;
            let basin = crate::analysis::interpolate_grid_to_basins(
                &working,
                &geometry,
                &accepted.interpolation,
            )
            .map_err(|e| source_error(request_path, e))?;
            let checkpoint = stage.join("checkpoints/forecast").join(&scenario.id);
            fs::create_dir(&checkpoint).map_err(|e| source_error(&checkpoint, e))?;
            write_grid_series(&checkpoint.join("working.nc"), &working)
                .map_err(|e| source_error(&checkpoint, e))?;
            working_scenarios.insert(scenario.id.clone(), working);
            basin
        };
        let combined =
            join_forcing(&observed.basin, &forecast).map_err(|e| source_error(request_path, e))?;
        write_basin_forcing(
            &stage.join("forcing").join(format!("{}.nc", scenario.id)),
            &combined,
        )
        .map_err(|e| source_error(stage, e))?;
    }
    copy_staged_tree(
        &doc.engine.executable,
        &stage.join(crate::store::PREPARED_ENGINE),
    )
    .map_err(|e| source_error(&doc.engine.executable, e))?;
    let staged_engine = stage.join(crate::store::PREPARED_ENGINE);
    let expected = (
        recipe.engine_identity.sha256.clone(),
        recipe.engine_identity.size,
    );
    if digest_file(&staged_engine).map_err(|e| source_error(&staged_engine, e))? != expected {
        return Err(source_error(
            &doc.engine.executable,
            "installed engine changed while staging",
        ));
    }
    super::additional::check_engine(doc, &staged_engine)
        .map_err(|e| source_error(&doc.engine.executable, e))?;
    let mut inputs = collect_inputs(stage).map_err(|e| source_error(stage, e))?;
    inputs.retain(|file| file.path != Path::new("input-set.toml"));
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
        preparation_sha256: None,
        state: "prepared".into(),
        recipe: serde_json::to_value(recipe).map_err(|e| source_error(stage, e))?,
        inputs,
        scenarios,
        created_at: chrono::Utc::now()
            .with_timezone(
                &doc.time_zone
                    .parse::<chrono_tz::Tz>()
                    .map_err(|e| source_error(request_path, e))?,
            )
            .format("%Y-%m-%dT%H:%M:%S")
            .to_string(),
        finished_at: None,
        error: None,
    };
    control.check()?;
    if durable {
        sync_tree(stage).map_err(|e| source_error(stage, e))?;
        write_run_manifest(stage, &manifest).map_err(|e| source_error(stage, e))?;
    } else {
        crate::store::write_cache_json(&stage.join("manifest.toml"), &manifest)
            .map_err(|e| source_error(stage, e))?;
    }
    let (manifest_sha256, _) =
        digest_file(&stage.join("manifest.toml")).map_err(|e| source_error(stage, e))?;
    control.check()?;
    Ok(PreparationReport {
        prepared: stage.to_path_buf(),
        manifest_sha256,
        scenario_ids: doc.scenarios.iter().map(|s| s.id.clone()).collect(),
        artifacts: manifest
            .inputs
            .iter()
            .map(|f| stage.join(&f.path))
            .collect(),
    })
}

pub(super) fn materialize_input_root(
    workspace: &Path,
    root: &Path,
    document: PreparationDocument,
    recipe: ExecutionRecipe,
    control: &super::OperationControl,
) -> Result<PreparationReport, OperationError> {
    let bytes = toml::to_string(&document)
        .map_err(|e| source_error(root, e))?
        .into_bytes();
    let accepted =
        validate_document(document, bytes).map_err(|e| OperationError::InvalidRequest {
            field: e.field,
            message: e.message,
        })?;
    let accepted =
        resolve_request(workspace, accepted).map_err(|e| OperationError::InvalidRequest {
            field: e.field,
            message: e.message,
        })?;
    let doc = &accepted.document;
    let geometry = crate::model::read_basin_geometry(&doc.geometry.mini_gtp)
        .map_err(|e| source_error(&doc.geometry.mini_gtp, e))?;
    let store = match doc.observed.revision {
        Some(revision) => ObservationStore::open_revision(&doc.observed.path, revision),
        None => ObservationStore::open_read_only(&doc.observed.path),
    }
    .map_err(|e| source_error(&doc.observed.path, e))?;
    if let Some(identity) = &recipe.observation_registry_id
        && store
            .registry_id()
            .map_err(|e| source_error(&doc.observed.path, e))?
            != *identity
    {
        return Err(source_error(
            &doc.observed.path,
            "observation registry identity differs from pinned inputs",
        ));
    }
    let selection = ObservationSelection {
        provider_codes: doc.observed.provider_codes.clone(),
        station_ids: doc.observed.station_ids.clone(),
    };
    let extra_stations = super::additional::extra_stations(&store, doc)
        .map_err(|e| source_error(&doc.observed.path, e))?;
    let (mut inputs, _) = if selection.provider_codes.is_empty()
        && selection.station_ids.is_empty()
        && extra_stations.is_empty()
        && matches!(
            doc.observed.historical_rainfall,
            super::HistoricalRainfall::Imerg { .. }
        ) {
        (ObservationInputs::default(), Vec::new())
    } else {
        store
            .read_selection_with_stations(&selection, &accepted.historical, &[], &extra_stations)
            .map_err(|e| source_error(&doc.observed.path, e))?
    };
    let source_replacements = recipe.registered_replacements.clone();
    inputs.replacements = serde_json::from_value(recipe.document["replacement_values"].clone())
        .map_err(|e| source_error(root, e))?;
    let selected: Vec<crate::store::ObservationSeries> =
        serde_json::from_value(recipe.document["selected_series"].clone())
            .map_err(|e| source_error(root, e))?;
    inputs.series.retain(|series| {
        selected
            .iter()
            .any(|chosen| chosen.series_id == series.series_id)
    });
    inputs.samples.retain(|sample| {
        inputs
            .series
            .iter()
            .any(|series| series.series_id == sample.series_id)
    });
    let mut forecasts = Vec::new();
    if doc.scenarios.iter().any(|s| s.asset_id.is_some()) {
        let catalog = ForecastCatalog::open_read_only(root, &doc.forecasts.catalog)
            .map_err(|e| source_error(&doc.forecasts.catalog, e))?;
        for scenario in &doc.scenarios {
            if let Some(id) = &scenario.asset_id {
                forecasts.push(
                    catalog
                        .load_for_run(id)
                        .map_err(|e| source_error(&doc.forecasts.catalog, e))?,
                );
            }
        }
    }
    let captured = CapturedSources {
        geometry,
        inputs,
        source_replacements,
        forecasts,
        recipe,
        imerg: load_captured_imerg(
            root,
            accepted.document.observed.historical_rainfall.clone(),
            &accepted,
        )?,
    };
    let result = (|| {
        for path in ["forcing", "checkpoints", "checkpoints/forecast"] {
            fs::create_dir(root.join(path)).map_err(|e| source_error(root, e))?;
        }
        materialize_sources(root, &accepted, captured, root, control, false)
    })();
    if result.is_err() {
        let _ = crate::store::clean_materialized_inputs(root);
        let _ = fs::remove_file(root.join("manifest.toml"));
    }
    result
}

#[allow(clippy::too_many_arguments)] // The resolved selection and destination are both required during freezing.
fn prepare_accepted(
    workspace: PathBuf,
    output: PathBuf,
    accepted: ValidatedRequest,
    replacements: Option<Vec<ObservationReplacement>>,
    request_path: PathBuf,
    workspace_scenarios: Option<super::operational_run::ResolvedScenarios>,
    control: &super::OperationControl,
) -> Result<PreparationReport, OperationError> {
    control.set_progress(super::OperationStage::Preparation, None, None);
    control.check()?;
    control.selection(&accepted.document);
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
            "baseline",
            "forcing",
            "checkpoints",
            "checkpoints/forecast",
        ] {
            fs::create_dir(stage.join(dir)).map_err(|e| source_error(&stage, e))?;
        }
        if workspace_scenarios.is_none() {
            fs::write(stage.join("request.toml"), &accepted.request_bytes)
                .map_err(|e| source_error(&stage, e))?;
        }
        let captured = capture_sources(
            &stage,
            &workspace,
            &accepted,
            replacements,
            &request_path,
            workspace_scenarios,
            control,
        )?;
        let report =
            materialize_sources(&stage, &accepted, captured, &request_path, control, true)?;
        finalize_prepared(&stage, &output).map_err(|e| source_error(&output, e))?;
        Ok(PreparationReport {
            prepared: output.clone(),
            manifest_sha256: report.manifest_sha256,
            scenario_ids: report.scenario_ids,
            artifacts: report
                .artifacts
                .iter()
                .map(|p| output.join(p.strip_prefix(&stage).expect("staged artifact")))
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

fn capture_imerg(
    workspace: &Path,
    stage: &Path,
    accepted: &ValidatedRequest,
) -> Result<Option<crate::store::GridSeries>, OperationError> {
    let super::HistoricalRainfall::Imerg { object_ids } =
        &accepted.document.observed.historical_rainfall
    else {
        return Ok(None);
    };
    let directory = stage.join("sources/imerg");
    fs::create_dir_all(&directory).map_err(|e| source_error(stage, e))?;
    for id in object_ids {
        let source = crate::store::registered_imerg_path(workspace, id)
            .map_err(|e| source_error(workspace, e))?;
        copy_staged_tree(&source, &directory.join(format!("{id}.nc")))
            .map_err(|e| source_error(&source, e))?;
    }
    load_captured_imerg(
        stage,
        accepted.document.observed.historical_rainfall.clone(),
        accepted,
    )
}
fn load_captured_imerg(
    root: &Path,
    selection: super::HistoricalRainfall,
    accepted: &ValidatedRequest,
) -> Result<Option<crate::store::GridSeries>, OperationError> {
    use chrono::TimeZone;
    let super::HistoricalRainfall::Imerg { object_ids } = selection else {
        return Ok(None);
    };
    let zone: chrono_tz::Tz = accepted
        .document
        .time_zone
        .parse()
        .map_err(|e| source_error(root, e))?;
    let mut combined: Option<crate::store::GridSeries> = None;
    let mut frames = std::collections::BTreeMap::new();
    for id in object_ids {
        let path = root.join("sources/imerg").join(format!("{id}.nc"));
        let native = crate::store::read_grid_series(&path).map_err(|e| source_error(&path, e))?;
        if native.timeline.time_zone != "UTC" || native.timeline.bounds.len() != 24 {
            return Err(source_error(
                &path,
                "IMERG must be a complete registered UTC day",
            ));
        }
        let count = native.grid.latitudes.len() * native.grid.longitudes.len();
        if combined.as_ref().is_some_and(|c| c.grid != native.grid) {
            return Err(source_error(&path, "selected IMERG crops/grids differ"));
        }
        for (index, bound) in native.timeline.bounds.iter().enumerate() {
            let start = chrono::Utc
                .from_utc_datetime(&bound.start)
                .with_timezone(&zone)
                .naive_local();
            let end = chrono::Utc
                .from_utc_datetime(&bound.end)
                .with_timezone(&zone)
                .naive_local();
            let value = (
                crate::store::TimeInterval { start, end },
                native.values[index * count..(index + 1) * count].to_vec(),
                native.valid[index * count..(index + 1) * count].to_vec(),
            );
            if frames.insert(start, value).is_some() {
                return Err(source_error(&path, "selected IMERG days overlap"));
            }
        }
        if let Some(c) = &mut combined {
            c.provenance.source_ids.extend(native.provenance.source_ids);
        } else {
            combined = Some(native);
        }
    }
    let mut native =
        combined.ok_or_else(|| source_error(root, "select at least one registered IMERG day"))?;
    native.timeline.time_zone = zone.name().into();
    native.timeline.bounds.clear();
    native.values.clear();
    native.valid.clear();
    for bound in &accepted.historical.bounds {
        let (actual, values, valid) = frames
            .get(&bound.start)
            .ok_or_else(|| source_error(root, format!("IMERG does not cover {}", bound.start)))?;
        if actual != bound {
            return Err(source_error(root, "IMERG intervals disagree with the run"));
        }
        native.timeline.bounds.push(actual.clone());
        native.values.extend_from_slice(values);
        native.valid.extend_from_slice(valid);
    }
    native.validate().map_err(|e| source_error(root, e))?;
    let working =
        crate::analysis::forecast_working_grid(&native, &accepted.historical, &accepted.grid)
            .map_err(|e| source_error(root, e))?;
    Ok(Some(working))
}
