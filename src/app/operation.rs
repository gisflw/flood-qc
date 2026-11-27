use crate::model::{
    EnginePlan, EngineStream, FloodHistory, HistoryResults, ProcessOutcome, capture_flood_history,
    capture_flood_source, decode_outputs, quantize_levels, read_history, reference_levels,
    run_engine, write_metadata, write_rainfall,
};
use crate::store::{
    FLOOD_SOURCE_DIAGNOSTIC, FLOOD_SOURCE_FILE, RunAttemptManifest, ScenarioFailure,
    ScenarioRecord, StationOffset, active_run, collect_run_diagnostics, digest_file,
    publish_run_checked, read_basin_forcing, read_operation_manifest, recover_run_cache,
    run_cache_root, run_stage_id, stage_working_inputs, sync_tree, write_run_attempt,
    write_run_journal, write_run_manifest,
};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, OpenOptions},
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationStage {
    Acquisition,
    Preparation,
    History,
    Scenarios,
    Finalization,
}

#[derive(Clone, Debug, Serialize)]
pub struct OperationProgress {
    pub stage: OperationStage,
    pub scenario: Option<String>,
    pub fraction: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    pub attempt: Option<PathBuf>,
    pub scenarios: Vec<ScenarioProgress>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ScenarioProgress {
    pub id: String,
    pub state: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OperationLogStream {
    Stdout,
    Stderr,
}

#[derive(Debug)]
pub enum OperationEvent {
    Stage(OperationStage),
    Download(super::acquisition::DownloadEvent),
    Selection {
        reference_time: String,
        time_zone: String,
        scenario_ids: Vec<String>,
    },
    Scenario(ScenarioProgress),
    EngineOutput {
        /// The scenario ID, or `None` for the shared history run.
        scenario: Option<String>,
        stream: OperationLogStream,
        /// Unmodified bytes. A chunk can end in the middle of a line or character.
        bytes: Vec<u8>,
    },
    EngineFinished {
        scenario: Option<String>,
    },
}

#[derive(Clone)]
struct Observer(Arc<dyn Fn(OperationEvent) + Send + Sync>);

impl std::fmt::Debug for Observer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Observer")
    }
}

#[derive(Clone, Default, Debug)]
pub struct OperationControl {
    cancelled: Arc<AtomicBool>,
    progress: Arc<Mutex<Option<OperationProgress>>>,
    committed: Arc<Mutex<bool>>,
    observer: Option<Observer>,
    force_acquisition: bool,
}

impl OperationControl {
    /// Observe milestones and engine log chunks for a fresh action.
    ///
    /// Calls can originate on concurrent scenario workers. The observer should
    /// return promptly and handle its own output errors without panicking.
    pub fn with_observer(observer: impl Fn(OperationEvent) + Send + Sync + 'static) -> Self {
        Self {
            observer: Some(Observer(Arc::new(observer))),
            ..Self::default()
        }
    }

    pub fn with_force_acquisition(mut self, force: bool) -> Self {
        self.force_acquisition = force;
        self
    }
    pub(super) fn force_acquisition(&self) -> bool {
        self.force_acquisition
    }
    pub fn cancel(&self) {
        let committed = self.committed.lock().unwrap_or_else(|e| e.into_inner());
        if !*committed {
            self.cancelled.store(true, Ordering::Release);
        }
    }
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }
    pub fn progress(&self) -> Option<OperationProgress> {
        self.progress
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }
    pub(super) fn check(&self) -> Result<(), OperationError> {
        if self.is_cancelled() {
            Err(OperationError::Cancelled {
                message: "operation cancelled".into(),
            })
        } else {
            Ok(())
        }
    }
    pub(super) fn set_progress(
        &self,
        stage: OperationStage,
        scenario: Option<String>,
        attempt: Option<PathBuf>,
    ) {
        let changed = {
            let mut progress = self.progress.lock().unwrap_or_else(|e| e.into_inner());
            let changed = progress.as_ref().is_none_or(|value| value.stage != stage);
            let scenarios = progress
                .take()
                .map(|value| value.scenarios)
                .unwrap_or_default();
            *progress = Some(OperationProgress {
                stage,
                scenario,
                fraction: None,
                detail: None,
                attempt,
                scenarios,
            });
            changed
        };
        if changed {
            self.emit(|| OperationEvent::Stage(stage));
        }
    }
    pub(super) fn acquisition_progress(&self, detail: String, completed: usize, total: usize) {
        *self.progress.lock().unwrap_or_else(|e| e.into_inner()) = Some(OperationProgress {
            stage: OperationStage::Acquisition,
            scenario: None,
            fraction: (total > 0).then(|| completed as f64 / total as f64),
            detail: Some(detail),
            attempt: None,
            scenarios: Vec::new(),
        });
    }
    fn scenario_state(&self, id: &str, state: &str) {
        {
            let mut progress = self.progress.lock().unwrap_or_else(|e| e.into_inner());
            let Some(progress) = progress.as_mut() else {
                return;
            };
            if let Some(scenario) = progress.scenarios.iter_mut().find(|s| s.id == id) {
                scenario.state = state.into();
            } else {
                progress.scenarios.push(ScenarioProgress {
                    id: id.into(),
                    state: state.into(),
                });
            }
        }
        self.emit(|| {
            OperationEvent::Scenario(ScenarioProgress {
                id: id.into(),
                state: state.into(),
            })
        });
    }

    fn emit(&self, event: impl FnOnce() -> OperationEvent) {
        if let Some(observer) = &self.observer {
            (observer.0)(event());
        }
    }

    pub(super) fn download(&self, event: impl FnOnce() -> super::acquisition::DownloadEvent) {
        self.emit(|| OperationEvent::Download(event()));
    }

    pub(super) fn selection(&self, document: &super::request::PreparationDocument) {
        self.emit(|| OperationEvent::Selection {
            reference_time: document.reference_time.clone(),
            time_zone: document.time_zone.clone(),
            scenario_ids: document
                .scenarios
                .iter()
                .map(|item| item.id.clone())
                .collect(),
        });
    }

    fn run_engine(
        &self,
        plan: &EnginePlan,
        scenario: Option<&str>,
    ) -> Result<ProcessOutcome, crate::model::ModelError> {
        let Some(observer) = &self.observer else {
            return run_engine(plan, &self.cancelled, None);
        };
        let mut log = |stream, bytes: &[u8]| {
            (observer.0)(OperationEvent::EngineOutput {
                scenario: scenario.map(str::to_owned),
                stream: match stream {
                    EngineStream::Stdout => OperationLogStream::Stdout,
                    EngineStream::Stderr => OperationLogStream::Stderr,
                },
                bytes: bytes.to_vec(),
            });
        };
        let result = run_engine(plan, &self.cancelled, Some(&mut log));
        (observer.0)(OperationEvent::EngineFinished {
            scenario: scenario.map(str::to_owned),
        });
        result
    }
}

#[derive(Clone, Debug)]
pub struct PreparationRequest {
    pub workspace: PathBuf,
    pub invocation_dir: PathBuf,
    pub request_file: PathBuf,
    pub output: PathBuf,
}
#[derive(Clone, Debug, Serialize)]
pub struct PreparationReport {
    pub prepared: PathBuf,
    pub manifest_sha256: String,
    pub scenario_ids: Vec<String>,
    pub artifacts: Vec<PathBuf>,
}
#[derive(Clone, Debug)]
pub struct ExecutionRequest {
    pub workspace: PathBuf,
    pub invocation_dir: PathBuf,
    pub prepared: PathBuf,
}
#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum BatchStatus {
    Completed,
    Failed,
}
#[derive(Clone, Debug, Serialize)]
pub struct ScenarioExecution {
    pub id: String,
    pub status: BatchStatus,
    pub exit_code: Option<i32>,
    pub signal: Option<i32>,
    pub logs: Vec<PathBuf>,
    pub results: Vec<PathBuf>,
    pub error: Option<ScenarioFailure>,
}
#[derive(Clone, Debug, Serialize)]
pub struct ExecutionReport {
    pub run_root: PathBuf,
    pub run_artifact: PathBuf,
    pub run_id: String,
    pub status: BatchStatus,
    pub scenarios: Vec<ScenarioExecution>,
    pub error: Option<String>,
}
#[derive(Debug, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum OperationError {
    Cancelled {
        message: String,
    },
    BatchFailed {
        report: Box<ExecutionReport>,
    },
    InvalidRequest {
        field: String,
        message: String,
    },
    InvalidPrepared {
        path: PathBuf,
        message: String,
    },
    Busy {
        workspace: PathBuf,
    },
    Source {
        path: PathBuf,
        message: String,
    },
    Store {
        run_root: PathBuf,
        message: String,
    },
    Model {
        scenario: String,
        stage: String,
        message: String,
        log_paths: Vec<PathBuf>,
    },
}
impl std::fmt::Display for OperationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for OperationError {}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ExecutionRecipe {
    pub engine_identity: crate::store::InputReference,
    #[serde(default)]
    pub observation_registry_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_set_id: Option<String>,
    pub document: serde_json::Value,
    pub historical_count: usize,
    pub forecast_count: usize,
    pub selected_station_ids: Vec<String>,
    pub offsets: Vec<StationOffset>,
    #[serde(default)]
    pub registered_replacements: Vec<crate::store::ObservationReplacement>,
    pub omp_threads: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace_scenarios: Option<super::operational_run::ResolvedScenarios>,
}

fn resolved(invocation: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        invocation.join(path)
    }
}
fn now(recipe: &ExecutionRecipe) -> String {
    let zone = recipe
        .document
        .get("time_zone")
        .and_then(serde_json::Value::as_str)
        .and_then(|value| value.parse::<chrono_tz::Tz>().ok())
        .unwrap_or(chrono_tz::UTC);
    chrono::Utc::now()
        .with_timezone(&zone)
        .format("%Y-%m-%dT%H:%M:%S")
        .to_string()
}
fn store_error(root: &Path, message: impl ToString) -> OperationError {
    OperationError::Store {
        run_root: root.to_path_buf(),
        message: message.to_string(),
    }
}
fn validate_recipe(
    root: &Path,
    recipe: &ExecutionRecipe,
    scenarios: &[ScenarioRecord],
) -> Result<super::request::PreparationDocument, String> {
    use crate::store::{ObservationSelection, ObservationStore};
    if recipe.historical_count == 0 || recipe.forecast_count == 0 || recipe.omp_threads == 0 {
        return Err("invalid prepared timing or thread count".into());
    }
    let count = recipe
        .historical_count
        .checked_add(recipe.forecast_count)
        .ok_or("timeline overflow")?;
    let binding: serde_json::Value = crate::store::read_draft(&root.join("sources/recipe.toml"))?;
    if binding != serde_json::to_value(recipe).map_err(|e| e.to_string())? {
        return Err("recipe differs from frozen input values".into());
    }
    let mut document = recipe.document.clone();
    let object = document.as_object_mut().ok_or("invalid recipe document")?;
    let selected_series = object
        .remove("selected_series")
        .ok_or("missing selected series")?;
    let selected_stations = object
        .remove("selected_stations")
        .ok_or("missing selected stations")?;
    let replacement_values = object
        .remove("replacement_values")
        .ok_or("missing replacement values")?;

    let document: super::request::PreparationDocument =
        serde_json::from_value(document).map_err(|error| error.to_string())?;
    if recipe.workspace_scenarios.is_none() {
        let mut retained = super::request::parse_request(
            fs::read(root.join("request.toml")).map_err(|error| error.to_string())?,
        )
        .map_err(|error| error.to_string())?
        .document;
        retained.observed.path = document.observed.path.clone();
        retained.observed.revision = document.observed.revision;
        retained.geometry.mini_gtp = document.geometry.mini_gtp.clone();
        retained.engine.executable = document.engine.executable.clone();
        retained.engine.input_dir = document.engine.input_dir.clone();
        retained.forecasts.catalog = document.forecasts.catalog.clone();
        if let (Some(requested), Some(resolved)) = (&mut retained.forcing, &document.forcing) {
            requested.reservoir_schedules = resolved.reservoir_schedules.clone();
            requested.wind_observations = resolved.wind_observations.clone();
        }
        if let (Some(requested), Some(resolved)) =
            (&mut retained.calibration, &document.calibration)
        {
            requested.soil = resolved.soil.clone();
        }
        if let (Some(requested), Some(resolved)) = (&mut retained.hot_start, &document.hot_start) {
            requested.use_bundle = resolved.use_bundle.clone();
        }
        if serde_json::to_value(&retained).map_err(|error| error.to_string())?
            != serde_json::to_value(&document).map_err(|error| error.to_string())?
        {
            return Err("recipe differs from retained request".into());
        }
    }
    super::additional::check_engine(&document, &root.join(crate::store::PREPARED_ENGINE))?;
    if document.step_seconds != 3600
        || document.engine.omp_threads != recipe.omp_threads
        || document.levels.station_ids != recipe.selected_station_ids
        || document.scenarios.len() != scenarios.len()
        || document
            .scenarios
            .iter()
            .zip(scenarios)
            .any(|(a, b)| a.id != b.id)
    {
        return Err("recipe and scenario membership disagree".into());
    }
    if let Some(inputs) = &recipe.workspace_scenarios {
        if inputs.len() != document.scenarios.len() {
            return Err("invalid prepared scenario values".into());
        }
        let mut earlier = std::collections::HashSet::new();
        for (scenario, input) in document.scenarios.iter().zip(inputs) {
            if !super::series::valid_id(&scenario.id) || !earlier.insert(scenario.id.clone()) {
                return Err("invalid or duplicate scenario ID".into());
            }
            if let Some(base) = &input.base
                && (base == &scenario.id || !earlier.contains(base))
            {
                return Err("scenario base must name an earlier scenario".into());
            }
            if let Some(asset) = &scenario.asset_id {
                if asset.is_empty()
                    || !asset
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b))
                    || !root
                        .join("sources/forecasts")
                        .join(asset)
                        .join("native.nc")
                        .is_file()
                {
                    return Err("invalid frozen forecast selection".into());
                }
            } else if input.forecast.is_some() && input.base.is_none() {
                return Err("missing frozen base forecast".into());
            }
            if scenario.base != input.base || scenario.edits != input.edits {
                return Err("scenario recipe values disagree".into());
            }
            for edit in &input.edits {
                edit.validate().map_err(|e| e.to_string())?;
            }
        }
    } else {
        return Err("missing scenario recipe".into());
    }
    let mut seen = std::collections::HashSet::new();
    if recipe
        .selected_station_ids
        .iter()
        .any(|id| id.is_empty() || !seen.insert(id))
    {
        return Err("invalid selected stations".into());
    }
    let first = read_basin_forcing(&root.join("forcing").join(format!("{}.nc", scenarios[0].id)))
        .map_err(|e| e.to_string())?;
    if first.timeline.bounds.len() != count || first.timeline.time_zone != document.time_zone {
        return Err("forcing timeline differs from recipe".into());
    }
    let path = root.join("forcing/additional.toml");
    if document
        .forcing
        .as_ref()
        .is_some_and(|doc| !doc.mappings.is_empty())
    {
        let prepared: crate::store::PreparedAdditionalForcing = crate::store::read_draft(&path)?;
        prepared.validate().map_err(|e| e.to_string())?;
        if prepared.timeline != first.timeline {
            return Err("additional forcing timeline differs from rainfall".into());
        }
    } else if path.exists() {
        return Err("additional forcing has no recipe selection".into());
    }
    let parse = |value: &str| {
        chrono::NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M:%S").map_err(|e| e.to_string())
    };
    if first.timeline.bounds[0].end != parse(&document.historical_first_end)?
        || first.timeline.bounds[recipe.historical_count - 1].end
            != parse(&document.historical_last_end)?
        || first.timeline.bounds[recipe.historical_count].end
            != parse(&document.forecast_first_end)?
        || first.timeline.bounds[count - 1].end != parse(&document.forecast_last_end)?
        || document.reference_time != document.historical_last_end
        || first
            .timeline
            .bounds
            .iter()
            .any(|interval| interval.end - interval.start != chrono::Duration::hours(1))
        || first
            .timeline
            .bounds
            .windows(2)
            .any(|window| window[0].end != window[1].start)
    {
        return Err("forcing interval labels disagree with recipe".into());
    }
    let history_len = recipe
        .historical_count
        .checked_mul(first.basins.mini_ids.len())
        .ok_or("forcing shape overflow")?;
    for scenario in scenarios {
        let forcing = read_basin_forcing(&root.join("forcing").join(format!("{}.nc", scenario.id)))
            .map_err(|e| e.to_string())?;
        if forcing.timeline != first.timeline
            || forcing.basins != first.basins
            || forcing.values[..history_len] != first.values[..history_len]
            || forcing.valid[..history_len] != first.valid[..history_len]
            || forcing
                .values
                .iter()
                .zip(&forcing.valid)
                .any(|(value, valid)| !valid || !value.is_finite() || *value < 0.0)
        {
            return Err(format!("scenario {} forcing mismatch", scenario.id));
        }
    }
    let mut historical = first.timeline.clone();
    historical.bounds.truncate(recipe.historical_count);
    let selection = ObservationSelection {
        provider_codes: document.observed.provider_codes.clone(),
        station_ids: document.observed.station_ids.clone(),
    };
    let observation_path = if root.join("sources/observations.sqlite").is_file() {
        root.join("sources/observations.sqlite")
    } else if document.observed.path.ends_with("assets/registry.sqlite") {
        root.ancestors()
            .find(|p| p.join("assets/registry.sqlite").is_file())
            .map(|p| p.join("assets/registry.sqlite"))
            .unwrap_or_else(|| document.observed.path.clone())
    } else {
        document.observed.path.clone()
    };
    let observations = match document.observed.revision {
        Some(revision) => ObservationStore::open_revision(&observation_path, revision),
        None => ObservationStore::open_read_only(&observation_path),
    }
    .map_err(|e| e.to_string())?;
    if let Some(identity) = &recipe.observation_registry_id
        && observations.registry_id().map_err(|e| e.to_string())? != *identity
    {
        return Err("observation registry identity differs from pinned inputs".into());
    }
    let extra_stations = super::additional::extra_stations(&observations, &document)?;
    let (inputs, offsets) = if selection.provider_codes.is_empty()
        && selection.station_ids.is_empty()
        && extra_stations.is_empty()
        && matches!(
            document.observed.historical_rainfall,
            super::HistoricalRainfall::Imerg { .. }
        ) {
        (crate::store::ObservationInputs::default(), Vec::new())
    } else {
        observations
            .read_selection_with_stations(&selection, &historical, &[], &extra_stations)
            .map_err(|e| e.to_string())?
    };
    let selected_series: Vec<crate::store::ObservationSeries> =
        serde_json::from_value(selected_series).map_err(|e| e.to_string())?;
    if selected_series
        .iter()
        .any(|chosen| !inputs.series.contains(chosen))
        || serde_json::to_value(&inputs.stations).map_err(|e| e.to_string())? != selected_stations
    {
        return Err("observation records disagree with recipe".into());
    }
    let replacements: Vec<crate::store::ObservationReplacement> =
        serde_json::from_value(replacement_values).map_err(|e| e.to_string())?;
    for replacement in &replacements {
        crate::store::validate_replacement(replacement).map_err(|e| e.to_string())?;
    }
    if offsets != recipe.offsets
        || offsets
            .iter()
            .any(|offset| !first.basins.mini_ids.contains(&offset.mini_id))
    {
        return Err("station offset records disagree with snapshot".into());
    }
    Ok(document)
}

fn prepare_worker(
    snapshot: &Path,
    root: &Path,
    scenario: &str,
    history: bool,
    recipe: &ExecutionRecipe,
) -> Result<EnginePlan, String> {
    let worker = root
        .join("scenarios")
        .join(if history { ".history" } else { scenario });
    fs::create_dir(&worker).map_err(|e| e.to_string())?;
    let input = worker.join("Input");
    stage_working_inputs(&snapshot.join("baseline/Input"), &input)?;
    let mut forcing = read_basin_forcing(&snapshot.join("forcing").join(format!("{scenario}.nc")))
        .map_err(|e| e.to_string())?;
    let total = recipe
        .historical_count
        .checked_add(recipe.forecast_count)
        .ok_or("invalid counts")?;
    if forcing.timeline.bounds.len() != total
        || recipe.historical_count == 0
        || recipe.forecast_count == 0
    {
        return Err("forcing timeline mismatch".into());
    }
    let first = forcing.timeline.bounds[0].end;
    let forecast_first = forcing.timeline.bounds[recipe.historical_count].end;
    let horizon = if history { 0 } else { recipe.forecast_count };
    if history {
        forcing.timeline.bounds.truncate(recipe.historical_count);
        forcing
            .values
            .truncate(recipe.historical_count * forcing.basins.mini_ids.len());
        forcing
            .valid
            .truncate(recipe.historical_count * forcing.basins.mini_ids.len());
    }
    write_rainfall(&input.join("CHUVABIN.hig"), &forcing)?;
    write_metadata(
        &input,
        first,
        forecast_first,
        forcing.timeline.bounds.len(),
        horizon,
        &forcing.basins,
    )?;
    let additional = snapshot.join("forcing/additional.toml");
    if additional.is_file() {
        let values: crate::store::PreparedAdditionalForcing =
            crate::store::read_draft(&additional)?;
        let selected = if history {
            values
                .prefix(recipe.historical_count)
                .map_err(|e| e.to_string())?
        } else {
            values
        };
        crate::model::write_additional_forcing(&input, &selected)?;
    }
    if !history && let Some(at) = hot_save_time(&recipe.document)? {
        crate::model::write_hot_save_request(&input, at)?;
    }
    Ok(EnginePlan {
        executable: snapshot.join(crate::store::PREPARED_ENGINE),
        cwd: worker.clone(),
        input,
        output: worker.join("Output"),
        logs: worker.join("logs"),
        omp_threads: recipe.omp_threads,
        time_zone: recipe
            .document
            .get("time_zone")
            .and_then(serde_json::Value::as_str)
            .ok_or("missing timezone")?
            .to_string(),
    })
}

fn hot_save_time(document: &serde_json::Value) -> Result<Option<chrono::NaiveDateTime>, String> {
    document
        .get("hot_start")
        .and_then(|selection| selection.get("save_at"))
        .and_then(serde_json::Value::as_str)
        .map(|at| super::request::timestamp("hot_start.save_at", at).map_err(|e| e.to_string()))
        .transpose()
}

fn scenario_record(id: String) -> ScenarioRecord {
    ScenarioRecord {
        id,
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
    }
}

pub fn execute_prepared(request: ExecutionRequest) -> Result<ExecutionReport, OperationError> {
    execute_prepared_controlled(request, &OperationControl::default())
}

pub fn execute_prepared_controlled(
    request: ExecutionRequest,
    control: &OperationControl,
) -> Result<ExecutionReport, OperationError> {
    control.check()?;
    control.set_progress(OperationStage::Preparation, None, None);
    let workspace = resolved(&request.invocation_dir, &request.workspace)
        .canonicalize()
        .map_err(|e| OperationError::Source {
            path: request.workspace.clone(),
            message: e.to_string(),
        })?;
    let prepared = resolved(&request.invocation_dir, &request.prepared)
        .canonicalize()
        .map_err(|e| OperationError::InvalidPrepared {
            path: request.prepared.clone(),
            message: e.to_string(),
        })?;
    let lock = lock_workspace(&workspace)?;
    execute_locked(&workspace, &prepared, control, &lock, false)
}

pub(super) fn lock_workspace(workspace: &Path) -> Result<std::fs::File, OperationError> {
    let lock = lock_workspace_job(workspace)?;
    recover_run_cache(workspace).map_err(|error| store_error(workspace, error))?;
    Ok(lock)
}

pub(super) fn lock_workspace_job(workspace: &Path) -> Result<std::fs::File, OperationError> {
    let lock = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(workspace.join(".operation.lock"))
        .map_err(|e| store_error(workspace, e))?;
    lock.try_lock().map_err(|e| match e {
        std::fs::TryLockError::WouldBlock => OperationError::Busy {
            workspace: workspace.to_path_buf(),
        },
        std::fs::TryLockError::Error(e) => store_error(workspace, e),
    })?;
    Ok(lock)
}

pub(super) fn run_preparation_locked(
    control: &OperationControl,
    workspace: &Path,
    _lock: &std::fs::File,
    reference: (chrono::NaiveDateTime, &str),
    prepare: impl FnOnce(&Path, PathBuf, &OperationControl) -> Result<PreparationReport, OperationError>,
) -> Result<ExecutionReport, OperationError> {
    run_attempt(workspace, control, reference, |run_root, attempt| {
        let prepared = run_root.join("preparation");
        let report = prepare(workspace, prepared, control)?;
        let admitted = admit_preparation(&report.prepared)?;
        let staged = crate::store::stage_preparation(
            workspace,
            &report.prepared,
            &attempt.attempt_id,
            false,
            &admitted.manifest,
            &admitted.identity,
        )
        .map_err(|error| store_error(workspace, error))?;
        let admitted = validate_staged_preparation(staged.path(), admitted)?;
        control.check()?;
        let frozen = staged
            .publish()
            .map_err(|error| store_error(workspace, error))?;
        execute_in_stage(&frozen, run_root, attempt, control, admitted)
    })
}

pub(super) fn run_captured_locked(
    control: &OperationControl,
    workspace: &Path,
    engine: &Path,
    reference: chrono::NaiveDateTime,
    time_zone: &str,
    capture: impl FnOnce(&Path, PathBuf, &OperationControl) -> Result<PreparationReport, OperationError>,
) -> Result<ExecutionReport, OperationError> {
    run_attempt(
        workspace,
        control,
        (reference, time_zone),
        |run_root, attempt| {
            let base = workspace.join("work");
            fs::create_dir_all(&base).map_err(|e| store_error(&base, e))?;
            let staged = base.join(format!(".{}", attempt.attempt_id));
            let report = capture(workspace, staged, control)?;
            let frozen = crate::store::publish_captured_input_set(
                workspace,
                &report.prepared,
                &attempt.attempt_id,
            )
            .map_err(|e| store_error(workspace, e))?;
            let metadata: crate::store::InputSetManifest =
                crate::store::read_draft(&frozen.join("input-set.toml"))
                    .map_err(|e| store_error(&frozen, e))?;
            attempt.operation = Some(operation_from_inputs(&metadata));
            write_run_journal(run_root, attempt).map_err(|e| store_error(run_root, e))?;
            let materialized = super::input_sets::materialize_captured(
                workspace,
                &frozen,
                engine,
                run_root.join("preparation"),
                control,
            )?;
            let admitted = admit_preparation(&materialized.prepared)?;
            execute_in_stage(&materialized.prepared, run_root, attempt, control, admitted)
        },
    )
}
fn operation_from_inputs(
    inputs: &crate::store::InputSetManifest,
) -> crate::store::OperationManifest {
    let ids = inputs.recipe["document"]["scenarios"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|s| s["id"].as_str())
        .map(|id| scenario_record(id.to_owned()))
        .collect();
    crate::store::OperationManifest {
        preparation_sha256: None,
        state: "captured".into(),
        recipe: inputs.recipe.clone(),
        inputs: inputs.files.clone(),
        scenarios: ids,
        created_at: inputs.created_at.clone(),
        finished_at: None,
        error: None,
    }
}

pub(super) fn execute_locked(
    workspace: &Path,
    prepared: &Path,
    control: &OperationControl,
    _lock: &std::fs::File,
    _temporary_preparation: bool,
) -> Result<ExecutionReport, OperationError> {
    if !super::observation_assets(workspace).exists() {
        let manifest = read_operation_manifest(prepared).map_err(|message| {
            OperationError::InvalidPrepared {
                path: prepared.to_path_buf(),
                message,
            }
        })?;
        let recipe: ExecutionRecipe = serde_json::from_value(manifest.recipe)
            .map_err(|error| store_error(prepared, error))?;
        let zone: chrono_tz::Tz = recipe.document["time_zone"]
            .as_str()
            .ok_or_else(|| store_error(prepared, "Missing operation timezone"))?
            .parse()
            .map_err(|error| store_error(prepared, error))?;
        super::workspace::initialize_assets(workspace, zone)
            .map_err(|error| store_error(workspace, error))?;
    }
    let admitted = admit_preparation(prepared)?;
    let reference = admitted.recipe.document["reference_time"]
        .as_str()
        .ok_or_else(|| store_error(prepared, "missing run reference"))?
        .parse::<chrono::NaiveDateTime>()
        .map_err(|e| store_error(prepared, e))?;
    let zone = admitted.recipe.document["time_zone"]
        .as_str()
        .ok_or_else(|| store_error(prepared, "missing run timezone"))?
        .to_owned();
    run_attempt(
        workspace,
        control,
        (reference, &zone),
        |run_root, attempt| {
            let staged = crate::store::stage_preparation(
                workspace,
                prepared,
                &attempt.attempt_id,
                true,
                &admitted.manifest,
                &admitted.identity,
            )
            .map_err(|error| store_error(workspace, error))?;
            let admitted = validate_staged_preparation(staged.path(), admitted)?;
            control.check()?;
            let frozen = staged
                .publish()
                .map_err(|error| store_error(workspace, error))?;
            execute_in_stage(&frozen, run_root, attempt, control, admitted)
        },
    )
}

fn run_attempt(
    workspace: &Path,
    control: &OperationControl,
    reference: (chrono::NaiveDateTime, &str),
    execute: impl FnOnce(&Path, &mut RunAttemptManifest) -> Result<ExecutionReport, OperationError>,
) -> Result<ExecutionReport, OperationError> {
    control.check()?;
    use chrono::{Offset, TimeZone};
    let (reference, zone) = reference;
    let zone: chrono_tz::Tz = zone.parse().map_err(|e| store_error(workspace, e))?;
    let zoned = zone
        .from_local_datetime(&reference)
        .single()
        .ok_or_else(|| store_error(workspace, "ambiguous/nonexistent run reference"))?;
    let id = crate::store::allocate_run_id(workspace, reference, zone.name())
        .map_err(|e| store_error(workspace, e))?;
    let run_root =
        crate::store::new_run_stage_for(workspace, &id).map_err(|e| store_error(workspace, e))?;
    let mut attempt = RunAttemptManifest::new(
        run_stage_id(&run_root)
            .map_err(|error| store_error(&run_root, error))?
            .into(),
    );
    attempt.reference_time = Some(zoned.to_rfc3339());
    attempt.time_zone = Some(zone.name().into());
    attempt.utc_offset_seconds = Some(zoned.offset().fix().local_minus_utc());
    write_run_journal(&run_root, &attempt).map_err(|error| store_error(&run_root, error))?;
    control.set_progress(OperationStage::Preparation, None, Some(run_root.clone()));
    let result = execute(&run_root, &mut attempt);
    let frozen = workspace
        .join("assets/selections")
        .join(&attempt.attempt_id);
    let cleanup = if frozen.join("input-set.toml").is_file() {
        (|| -> Result<(), OperationError> {
            let input_set: crate::store::InputSetManifest =
                crate::store::read_draft(&frozen.join("input-set.toml"))
                    .map_err(|e| store_error(&frozen, e))?;
            if attempt.operation.is_none() {
                attempt.operation = Some(operation_from_inputs(&input_set));
            }
            if let Some(operation) = &mut attempt.operation {
                operation.inputs = input_set.files;
                operation
                    .recipe
                    .as_object_mut()
                    .ok_or_else(|| store_error(&frozen, "invalid recipe"))?
                    .insert("input_set_id".into(), input_set.id.into());
            }
            crate::store::clean_materialized_inputs(&frozen).map_err(|e| store_error(&frozen, e))
        })()
    } else {
        Ok(())
    };
    let cleanup = cleanup.and_then(|()| {
        let materialized = workspace
            .join("work")
            .join(format!(".inputs-{}", attempt.attempt_id));
        if materialized.exists() {
            fs::remove_dir_all(&materialized).map_err(|e| store_error(&materialized, e))?;
        }
        Ok(())
    });
    let result = result.and_then(|report| cleanup.map(|()| report));

    let result = result.and_then(|report| {
        control.check()?;
        attempt.finish("completed", None);
        write_run_journal(&run_root, &attempt).map_err(|error| store_error(&run_root, error))?;
        publish_run_checked(workspace, &run_root, |commit| {
            let mut committed = control
                .committed
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            control.check().map_err(|error| error.to_string())?;
            commit()?;
            *committed = true;
            Ok(())
        })
        .map_err(|error| store_error(&run_root, error))?;
        let destination = workspace.join("results").join(&attempt.attempt_id);
        let mut report = report;
        fn relocate(path: &mut PathBuf, old: &Path, new: &Path) {
            if let Ok(relative) = path.strip_prefix(old) {
                *path = new.join(relative);
            }
        }
        relocate(&mut report.run_artifact, &run_root, &destination);
        relocate(&mut report.run_root, &run_root, &destination);
        for scenario in &mut report.scenarios {
            for path in scenario.logs.iter_mut().chain(&mut scenario.results) {
                relocate(path, &run_root, &destination);
            }
        }
        Ok(report)
    });
    let mut committed = control
        .committed
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let cancelled = control.is_cancelled();
    *committed = true;
    drop(committed);
    match result {
        Ok(report) => Ok(report),
        Err(mut error) => {
            if cancelled && !matches!(error, OperationError::BatchFailed { .. }) {
                error = OperationError::Cancelled {
                    message: "operation cancelled".into(),
                };
            }
            let message = match &error {
                OperationError::BatchFailed { report } => report.error.clone(),
                _ => Some(error.to_string()),
            };
            let message = if cancelled {
                Some("operation cancelled".into())
            } else {
                message
            };
            let state = if cancelled { "cancelled" } else { "failed" };
            attempt.finish(state, message.clone());
            if let Some(operation) = &mut attempt.operation {
                operation.state = state.into();
                operation
                    .finished_at
                    .get_or_insert_with(|| chrono::Utc::now().to_rfc3339());
                operation.error = message;
            }
            attempt.diagnostics = collect_run_diagnostics(&run_root);
            write_run_journal(&run_root, &attempt)
                .map_err(|error| store_error(&run_root, error))?;
            let archive = write_run_attempt(workspace, &attempt)
                .map_err(|error| store_error(&run_root, error))?;
            fs::remove_dir_all(&run_root).map_err(|error| store_error(&run_root, error))?;
            if let OperationError::BatchFailed { report } = &mut error {
                report.run_root = archive.parent().unwrap_or(workspace).to_path_buf();
                report.run_artifact = archive.clone();
                for item in &mut report.scenarios {
                    item.logs.clear();
                    item.results.clear();
                }
            }
            Err(error)
        }
    }
}

struct PreparedAdmission {
    manifest: crate::store::OperationManifest,
    identity: (String, u64),
    recipe: ExecutionRecipe,
    document: super::request::PreparationDocument,
}

fn admit_preparation(prepared: &Path) -> Result<PreparedAdmission, OperationError> {
    let invalid = |message| OperationError::InvalidPrepared {
        path: prepared.to_path_buf(),
        message,
    };
    let identity = digest_file(&prepared.join("manifest.toml")).map_err(invalid)?;
    let manifest = read_operation_manifest(prepared).map_err(invalid)?;
    let recipe: ExecutionRecipe =
        serde_json::from_value(manifest.recipe.clone()).map_err(|e| invalid(e.to_string()))?;
    let document = validate_recipe(prepared, &recipe, &manifest.scenarios).map_err(invalid)?;
    if digest_file(&prepared.join("manifest.toml")).map_err(invalid)? != identity {
        return Err(invalid(
            "prepared manifest changed during validation".into(),
        ));
    }
    Ok(PreparedAdmission {
        manifest,
        identity,
        recipe,
        document,
    })
}

fn validate_staged_preparation(
    root: &Path,
    mut admitted: PreparedAdmission,
) -> Result<PreparedAdmission, OperationError> {
    admitted.document = validate_recipe(root, &admitted.recipe, &admitted.manifest.scenarios)
        .map_err(|message| OperationError::InvalidPrepared {
            path: root.into(),
            message,
        })?;
    if digest_file(&root.join("manifest.toml")).map_err(|error| store_error(root, error))?
        != admitted.identity
    {
        return Err(store_error(
            root,
            "prepared manifest changed during staged validation",
        ));
    }
    Ok(admitted)
}

fn execute_in_stage(
    prepared: &Path,
    run_root: &Path,
    attempt: &mut RunAttemptManifest,
    control: &OperationControl,
    admitted: PreparedAdmission,
) -> Result<ExecutionReport, OperationError> {
    control.check()?;
    let PreparedAdmission {
        mut manifest,
        identity: manifest_identity,
        recipe,
        document,
    } = admitted;
    control.selection(&document);
    manifest.preparation_sha256 = Some(manifest_identity.0.clone());
    manifest.state = "running".into();
    manifest.created_at = now(&recipe);
    manifest.finished_at = None;
    attempt.state = "running".into();
    attempt.operation = Some(manifest.clone());
    write_run_journal(run_root, attempt).map_err(|error| store_error(run_root, error))?;

    let snapshot = prepared.to_path_buf();
    control.check()?;
    if digest_file(&snapshot.join("manifest.toml")).map_err(|error| store_error(run_root, error))?
        != manifest_identity
    {
        return Err(store_error(
            run_root,
            "prepared manifest changed while freezing",
        ));
    }
    fs::create_dir(run_root.join("scenarios"))
        .and_then(|()| fs::create_dir(run_root.join("results")))
        .map_err(|error| store_error(run_root, error))?;
    manifest.scenarios =
        execute_scenarios(run_root, &snapshot, &recipe, &manifest.scenarios, control);
    control.set_progress(
        OperationStage::Finalization,
        None,
        Some(run_root.to_path_buf()),
    );
    let cancelled = control.is_cancelled();
    let complete = manifest
        .scenarios
        .iter()
        .all(|item| item.state == "completed");
    manifest.state = if cancelled {
        "cancelled"
    } else if complete {
        "completed"
    } else {
        "failed"
    }
    .into();
    manifest.finished_at = Some(now(&recipe));
    manifest.error = if cancelled {
        Some("operation cancelled".into())
    } else if !complete {
        Some("one or more scenarios failed".into())
    } else {
        None
    };
    attempt.operation = Some(manifest.clone());
    attempt.diagnostics = collect_run_diagnostics(run_root);
    write_run_journal(run_root, attempt).map_err(|error| store_error(run_root, error))?;
    if complete && !cancelled {
        super::rainfall::stage(run_root, &snapshot, &recipe, &manifest.inputs, control)
            .map_err(|error| store_error(run_root, error))?;
        crate::store::clean_materialized_inputs(&snapshot)
            .map_err(|error| store_error(run_root, error))?;
        for record in &manifest.scenarios {
            let worker = run_root.join("scenarios").join(&record.id);
            clean_worker(&worker, true).map_err(|error| store_error(&worker, error))?;
        }
        let history = run_root.join("scenarios/.history");
        clean_worker(&history, false).map_err(|error| store_error(&history, error))?;
        let input_set: crate::store::InputSetManifest =
            crate::store::read_draft(&snapshot.join("input-set.toml"))
                .map_err(|error| store_error(&snapshot, error))?;
        manifest.inputs = input_set.files;
        manifest
            .recipe
            .as_object_mut()
            .ok_or_else(|| store_error(&snapshot, "invalid recipe"))?
            .insert(
                "input_set_id".into(),
                serde_json::Value::String(input_set.id),
            );
        attempt.operation = Some(manifest.clone());
        write_run_journal(run_root, attempt).map_err(|error| store_error(run_root, error))?;
        write_run_manifest(run_root, &manifest).map_err(|error| store_error(run_root, error))?;
        if snapshot.starts_with(run_root)
            || snapshot
                .parent()
                .is_some_and(|p| p == run_root.parent().unwrap_or(run_root))
        {
            fs::remove_dir_all(&snapshot).map_err(|e| store_error(&snapshot, e))?;
        }
    }

    let scenarios = manifest
        .scenarios
        .iter()
        .map(|record| ScenarioExecution {
            id: record.id.clone(),
            status: if record.state == "completed" {
                BatchStatus::Completed
            } else {
                BatchStatus::Failed
            },
            exit_code: record.exit_code,
            signal: record.signal,
            logs: [record.stdout.as_ref(), record.stderr.as_ref()]
                .into_iter()
                .flatten()
                .map(|p| run_root.join(p))
                .collect(),
            results: [record.basin_result.as_ref(), record.station_result.as_ref()]
                .into_iter()
                .flatten()
                .map(|p| run_root.join(p))
                .chain(
                    (record.state == "completed")
                        .then(|| {
                            run_root
                                .join("results")
                                .join(&record.id)
                                .join(FLOOD_SOURCE_FILE)
                        })
                        .filter(|path| path.is_file()),
                )
                .chain(
                    (record.state == "completed")
                        .then(|| run_root.join("scenarios").join(&record.id).join("state"))
                        .filter(|path| path.is_dir()),
                )
                .collect(),
            error: record.error.clone(),
        })
        .collect();
    let report = ExecutionReport {
        run_artifact: run_root.join("manifest.toml"),
        run_id: attempt.attempt_id.clone(),
        run_root: run_root.to_path_buf(),
        status: if complete && !cancelled {
            BatchStatus::Completed
        } else {
            BatchStatus::Failed
        },
        scenarios,
        error: manifest.error.clone(),
    };
    if cancelled || !complete {
        Err(OperationError::BatchFailed {
            report: Box::new(report),
        })
    } else {
        Ok(report)
    }
}

fn clean_worker(path: &Path, success: bool) -> std::io::Result<()> {
    let entries = match fs::read_dir(path) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error),
    };
    for entry in entries {
        let entry = entry?;
        let name = entry.file_name();
        if name == "logs" || (success && (name == "Output" || name == "state")) {
            continue;
        }
        if entry.file_type()?.is_dir() {
            fs::remove_dir_all(entry.path())?;
        } else {
            fs::remove_file(entry.path())?;
        }
    }
    if success {
        let output = path.join("Output");
        for entry in fs::read_dir(output)? {
            let entry = entry?;
            if [
                "QTUDO_Inercial_Atual.MGB",
                "QTUDO_Inercial_Prev.MGB",
                "YTUDO.MGB",
                "YTUDO_prev.MGB",
            ]
            .iter()
            .any(|name| entry.file_name() == *name)
            {
                continue;
            }
            if entry.file_type()?.is_dir() {
                fs::remove_dir_all(entry.path())?;
            } else {
                fs::remove_file(entry.path())?;
            }
        }
    }
    Ok(())
}

struct SharedHistory {
    hydrology: HistoryResults,
    flood: Result<FloodHistory, String>,
}

fn execute_scenarios(
    root: &Path,
    snapshot: &Path,
    recipe: &ExecutionRecipe,
    requested: &[ScenarioRecord],
    control: &OperationControl,
) -> Vec<ScenarioRecord> {
    control.set_progress(OperationStage::History, None, Some(root.to_path_buf()));
    let first = &requested[0].id;
    let history = control
        .check()
        .map_err(|e| e.to_string())
        .and_then(|()| prepare_worker(snapshot, root, first, true, recipe))
        .and_then(|plan| {
            let outcome = control.run_engine(&plan, None)?;
            if let Some(error) = outcome.error {
                return Err(format!("history run: {error}"));
            }
            let forcing = read_basin_forcing(&snapshot.join("forcing").join(format!("{first}.nc")))
                .map_err(|e| e.to_string())?;
            let count = recipe
                .historical_count
                .checked_mul(forcing.basins.mini_ids.len())
                .ok_or("history length overflow")?;
            let history = read_history(&plan.output, count)?;
            let flood =
                capture_flood_history(&plan, &forcing, recipe.historical_count, &history.level_m);
            Ok(SharedHistory {
                hydrology: history,
                flood,
            })
        });
    let history = match history {
        Ok(value) => value,
        Err(error) => {
            for item in requested {
                control.scenario_state(&item.id, "failed");
            }
            return requested
                .iter()
                .map(|item| {
                    let mut record = scenario_record(item.id.clone());
                    record.state = "failed".into();
                    record.error = Some(ScenarioFailure {
                        stage: "history".into(),
                        message: error.clone(),
                        log_paths: vec![
                            PathBuf::from("scenarios/.history/logs/stdout.log"),
                            PathBuf::from("scenarios/.history/logs/stderr.log"),
                        ],
                    });
                    record.finished_at = Some(now(recipe));
                    record
                })
                .collect();
        }
    };
    control.set_progress(OperationStage::Scenarios, None, Some(root.to_path_buf()));
    for item in requested {
        control.scenario_state(&item.id, "pending");
    }
    std::thread::scope(|scope| {
        let handles: Vec<_> = requested
            .iter()
            .map(|item| {
                let history = &history;
                scope
                    .spawn(move || run_scenario(root, snapshot, recipe, &item.id, history, control))
            })
            .collect();
        handles
            .into_iter()
            .zip(requested)
            .map(|(handle, item)| match handle.join() {
                Ok(record) => record,
                Err(_) => {
                    control.scenario_state(&item.id, "failed");
                    let mut record = scenario_record(item.id.clone());
                    record.state = "failed".into();
                    record.error = Some(ScenarioFailure {
                        stage: "worker".into(),
                        message: "scenario worker panicked".into(),
                        log_paths: Vec::new(),
                    });
                    record.finished_at = Some(now(recipe));
                    record
                }
            })
            .collect()
    })
}

fn run_scenario(
    root: &Path,
    snapshot: &Path,
    recipe: &ExecutionRecipe,
    id: &str,
    history: &SharedHistory,
    control: &OperationControl,
) -> ScenarioRecord {
    control.scenario_state(id, "running");
    let mut record = scenario_record(id.into());
    record.state = "running".into();
    record.started_at = Some(now(recipe));
    let result = (|| -> Result<(), String> {
        control.check().map_err(|e| e.to_string())?;
        let plan = prepare_worker(snapshot, root, id, false, recipe)?;
        control.check().map_err(|e| e.to_string())?;
        record.stdout = Some(Path::new("scenarios").join(id).join("logs/stdout.log"));
        record.stderr = Some(Path::new("scenarios").join(id).join("logs/stderr.log"));
        let outcome = control.run_engine(&plan, Some(id))?;
        record.exit_code = outcome.exit_code;
        record.signal = outcome.signal;
        if let Some(error) = outcome.error {
            return Err(error.to_string());
        }
        control.check().map_err(|e| e.to_string())?;
        let mut forcing = read_basin_forcing(&snapshot.join("forcing").join(format!("{id}.nc")))
            .map_err(|e| e.to_string())?;
        if let Some(at) = hot_save_time(&recipe.document)? {
            let state = plan.cwd.join("state");
            crate::model::package_hot_start(
                &plan.output,
                &state,
                at,
                forcing.basins.mini_ids.len(),
            )?;
            crate::store::write_draft(
                &state.join("state.toml"),
                &crate::store::HotStartManifest {
                    saved_at: at,
                    time_zone: forcing.timeline.time_zone.clone(),
                    scenario: id.into(),
                    engine_sha256: digest_file(&plan.executable)?.0,
                    geometry_sha256: digest_file(&plan.input.join("Cell.hig"))?.0,
                    soil_sha256: digest_file(&plan.input.join("paruso.hig"))?.0,
                    files: crate::store::collect_inputs(&state)?,
                },
            )?;
        }
        let mut hashes = String::new();
        for path in [
            plan.input.join("PARHIG.hig"),
            plan.input.join("Previsao.meta"),
            plan.input.join("CHUVABIN.hig"),
            plan.input.join("MINI.gtp"),
            snapshot.join(crate::store::PREPARED_ENGINE),
        ] {
            hashes.push_str(&digest_file(&path)?.0);
        }
        use sha2::{Digest, Sha256};
        let snapshot_hash = format!("{:x}", Sha256::digest(hashes.as_bytes()));
        let mut processing = serde_json::Map::new();
        processing.insert(
            "reference_time".into(),
            forcing.timeline.bounds[recipe.historical_count - 1]
                .end
                .format("%Y-%m-%dT%H:%M:%S")
                .to_string()
                .into(),
        );
        processing.insert(
            "engine_sha256".into(),
            digest_file(&snapshot.join(crate::store::PREPARED_ENGINE))?
                .0
                .into(),
        );
        processing.insert("snapshot_sha256".into(), snapshot_hash.into());
        if snapshot.join("forcing/additional.toml").is_file()
            || recipe.document.get("calibration").is_some()
        {
            let mut selected = serde_json::Map::new();
            for name in [
                "QOBS_hora.PRN",
                "QSUBST_hora.PRN",
                "vento.txt",
                "Cell.hig",
                "paruso.hig",
            ] {
                let path = plan.input.join(name);
                if path.is_file() {
                    selected.insert(name.into(), digest_file(&path)?.0.into());
                }
            }
            processing.insert("engine_input_sha256".into(), selected.into());
        }
        forcing.provenance.processing = processing.into();
        let raw = decode_outputs(
            &plan.output,
            forcing,
            recipe.historical_count,
            recipe.forecast_count,
            &history.hydrology,
        )?;
        let basin = quantize_levels(&raw)?;
        let station = reference_levels(&raw, &recipe.selected_station_ids, &recipe.offsets)?;
        let result_dir = root.join("results").join(id);
        fs::create_dir(&result_dir).map_err(|e| e.to_string())?;
        crate::store::write_result_series(&result_dir.join("basin.nc"), &basin)
            .map_err(|e| e.to_string())?;
        crate::store::write_station_results(&result_dir.join("station.nc"), &station)
            .map_err(|e| e.to_string())?;
        crate::store::read_result_series(&result_dir.join("basin.nc"))
            .map_err(|e| e.to_string())?;
        crate::store::read_station_results(&result_dir.join("station.nc"))
            .map_err(|e| e.to_string())?;
        let evidence = result_dir.join("engine-inputs");
        fs::create_dir(&evidence).map_err(|e| e.to_string())?;
        for name in ["PARHIG.hig", "Previsao.meta"] {
            fs::copy(plan.input.join(name), evidence.join(name)).map_err(|e| e.to_string())?;
        }
        fs::write(
            evidence.join("CHUVABIN.sha256"),
            digest_file(&plan.input.join("CHUVABIN.hig"))?.0,
        )
        .map_err(|e| e.to_string())?;
        sync_tree(&result_dir)?;
        let flood = history
            .flood
            .as_ref()
            .map_err(|error| format!("shared flood history unavailable: {error}"))
            .and_then(|history| {
                capture_flood_source(&plan, id, history, &raw.forcing, &raw.level_m)
            })
            .and_then(|source| {
                crate::store::write_flood_source(&result_dir.join(FLOOD_SOURCE_FILE), &source)
                    .map_err(|error| error.to_string())
            });
        if let Err(error) = flood {
            let _ = crate::store::write_atomic(
                &result_dir.join(FLOOD_SOURCE_DIAGNOSTIC),
                format!("{error}\n").as_bytes(),
            );
        }
        record.basin_result = Some(Path::new("results").join(id).join("basin.nc"));
        record.station_result = Some(Path::new("results").join(id).join("station.nc"));
        Ok(())
    })();
    record.finished_at = Some(now(recipe));
    match result {
        Ok(()) => record.state = "completed".into(),
        Err(error) => {
            record.state = "failed".into();
            record.error = Some(ScenarioFailure {
                stage: "execution".into(),
                message: error,
                log_paths: [record.stdout.clone(), record.stderr.clone()]
                    .into_iter()
                    .flatten()
                    .collect(),
            });
        }
    }
    control.scenario_state(id, &record.state);
    record
}

#[derive(Clone, Debug, Serialize)]
pub struct TrialCache {
    pub root: PathBuf,
    pub preparation_sha256: Option<String>,
    pub recipe: serde_json::Value,
    pub scenarios: Vec<ScenarioExecution>,
}

pub fn discover_trials(workspace: &Path) -> Result<Vec<PathBuf>, OperationError> {
    active_run(workspace)
        .map(|path| path.into_iter().collect())
        .map_err(|error| store_error(workspace, error))
}

/// Perform an artifact read while the active generation is protected from retirement.
pub fn with_active_run<T>(
    workspace: &Path,
    read: impl FnOnce(&Path) -> Result<T, String>,
) -> Result<Option<T>, String> {
    crate::store::with_active_run(workspace, read)
}

pub fn read_trial(root: &Path) -> Result<TrialCache, OperationError> {
    if let Some(workspace) = root.parent().and_then(Path::parent)
        && root.parent() == Some(run_cache_root(workspace).as_path())
        && run_cache_root(workspace).join("access.lock").is_file()
    {
        return crate::store::with_available_run(workspace, root, true, |path| {
            read_trial_artifacts(path).map_err(|e| e.to_string())
        })
        .map_err(|e| store_error(root, e));
    }
    read_trial_artifacts(root)
}

fn read_trial_artifacts(root: &Path) -> Result<TrialCache, OperationError> {
    let manifest = crate::store::read_retained_manifest(root).map_err(|e| store_error(root, e))?;
    if manifest.state == "prepared" {
        return Err(store_error(root, "expected trial cache"));
    }
    let mut scenarios = Vec::new();
    for record in manifest.scenarios.iter().filter(|s| s.state == "completed") {
        let id = &record.id;
        if id.is_empty()
            || !id
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-')
        {
            return Err(store_error(root, "invalid scenario ID"));
        }
        let basin = root.join("results").join(id).join("basin.nc");
        let station = root.join("results").join(id).join("station.nc");
        let values = crate::store::read_result_series(&basin).map_err(|e| store_error(root, e))?;
        crate::store::read_station_results(&station).map_err(|e| store_error(root, e))?;
        if manifest.preparation_sha256.is_some() {
            for name in [
                "QTUDO_Inercial_Atual.MGB",
                "QTUDO_Inercial_Prev.MGB",
                "YTUDO.MGB",
                "YTUDO_prev.MGB",
            ] {
                if !root
                    .join("scenarios")
                    .join(id)
                    .join("Output")
                    .join(name)
                    .is_file()
                {
                    return Err(store_error(
                        root,
                        format!("missing engine stream for {id}: {name}"),
                    ));
                }
            }
        }
        let mut results = vec![basin, station];
        let flood = root.join("results").join(id).join(FLOOD_SOURCE_FILE);
        if flood.is_file() {
            results.push(flood);
        }
        if let Some(at) =
            hot_save_time(&manifest.recipe["document"]).map_err(|e| store_error(root, e))?
        {
            let state = root.join("scenarios").join(id).join("state");
            let record: crate::store::HotStartManifest =
                crate::store::read_draft(&state.join("state.toml"))
                    .map_err(|e| store_error(root, e))?;
            if record.saved_at != at
                || record.scenario != *id
                || record.time_zone != values.timeline.time_zone
                || values.provenance.processing["engine_sha256"].as_str()
                    != Some(&record.engine_sha256)
            {
                return Err(store_error(
                    root,
                    "state provenance differs from its completed scenario",
                ));
            }
            let actual = crate::store::collect_inputs(&state).map_err(|e| store_error(root, e))?;
            let actual: Vec<_> = actual
                .into_iter()
                .filter(|file| file.path != Path::new("state.toml"))
                .collect();
            if actual != record.files || actual.len() != 6 {
                return Err(store_error(
                    root,
                    "retained state differs from its integrity index",
                ));
            }
            crate::model::validate_hot_start(&state, at, values.basins.mini_ids.len())
                .map_err(|e| store_error(root, e))?;
            results.push(state);
        }
        scenarios.push(ScenarioExecution {
            id: id.clone(),
            status: BatchStatus::Completed,
            exit_code: record.exit_code,
            signal: record.signal,
            logs: ["stdout.log", "stderr.log"]
                .iter()
                .map(|name| root.join("scenarios").join(id).join("logs").join(name))
                .collect(),
            results,
            error: None,
        });
    }
    Ok(TrialCache {
        root: root.into(),
        preparation_sha256: manifest.preparation_sha256,
        recipe: manifest.recipe,
        scenarios,
    })
}

pub fn tail_log(path: &Path, max_bytes: usize) -> std::io::Result<Vec<u8>> {
    use std::io::{Read, Seek, SeekFrom};
    let mut file = std::fs::File::open(path)?;
    let size = file.metadata()?.len();
    let count = max_bytes.min(65536) as u64;
    file.seek(SeekFrom::Start(size.saturating_sub(count)))?;
    let mut bytes = Vec::new();
    file.take(count).read_to_end(&mut bytes)?;
    Ok(bytes)
}
