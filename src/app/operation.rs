use crate::model::{
    EnginePlan, HistoryResults, decode_outputs, quantize_levels, read_history, reference_levels,
    run_engine, write_metadata, write_rainfall,
};
use crate::store::{
    ScenarioFailure, ScenarioRecord, StationOffset, copy_tree, copy_working_inputs, digest_file,
    read_basin_forcing, read_operation_manifest, sync_tree, verify_inputs, write_run_manifest,
};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, OpenOptions},
    path::{Path, PathBuf},
};

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
    pub status: BatchStatus,
    pub scenarios: Vec<ScenarioExecution>,
}
#[derive(Debug, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum OperationError {
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
    pub document: serde_json::Value,
    pub historical_count: usize,
    pub forecast_count: usize,
    pub selected_station_ids: Vec<String>,
    pub offsets: Vec<StationOffset>,
    pub omp_threads: usize,
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
) -> Result<(), String> {
    use crate::store::{ObservationSelection, ObservationStore};
    if recipe.historical_count == 0 || recipe.forecast_count == 0 || recipe.omp_threads == 0 {
        return Err("invalid prepared timing or thread count".into());
    }
    let count = recipe
        .historical_count
        .checked_add(recipe.forecast_count)
        .ok_or("timeline overflow")?;
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
    let correction_values = object
        .remove("correction_values")
        .ok_or("missing correction values")?;
    let document: super::request::PreparationDocument =
        serde_json::from_value(document).map_err(|error| error.to_string())?;
    let mut retained = super::request::parse_request(
        fs::read(root.join("request.toml")).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?
    .document;
    retained.observed.path = document.observed.path.clone();
    retained.geometry.mini_gtp = document.geometry.mini_gtp.clone();
    retained.engine.executable = document.engine.executable.clone();
    retained.engine.input_dir = document.engine.input_dir.clone();
    retained.forecasts.catalog = document.forecasts.catalog.clone();
    if serde_json::to_value(&retained).map_err(|error| error.to_string())?
        != serde_json::to_value(&document).map_err(|error| error.to_string())?
    {
        return Err("recipe differs from retained request".into());
    }
    if document.schema_version != 1
        || document.step_seconds != 3600
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
    let expected_corrections: Vec<crate::store::ForecastCorrection> =
        serde_json::from_value(correction_values).map_err(|error| error.to_string())?;
    let mut actual_corrections = Vec::new();
    let mut catalog = None;
    for scenario in &document.scenarios {
        let corrections = scenario.correction_ids.as_deref().unwrap_or_default();
        match (
            scenario.kind.as_str(),
            scenario.asset_id.as_deref(),
            corrections.is_empty(),
        ) {
            ("zero", None, true) => continue,
            ("raw", Some(asset), true) | ("corrected", Some(asset), false) if !asset.is_empty() => {
                if !root
                    .join("sources/native")
                    .join(format!("{asset}.nc"))
                    .is_file()
                    || !root
                        .join("sources/original")
                        .join(format!("{asset}.grib2"))
                        .is_file()
                {
                    return Err("missing retained forecast asset".into());
                }
                if catalog.is_none() {
                    catalog = Some(
                        crate::store::ForecastCatalog::open_read_only(
                            root,
                            Path::new("sources/forecast.sqlite"),
                        )
                        .map_err(|error| error.to_string())?,
                    );
                }
                actual_corrections.extend(
                    catalog
                        .as_ref()
                        .unwrap()
                        .load_corrections(asset, corrections)
                        .map_err(|error| error.to_string())?,
                );
            }
            _ => return Err("invalid scenario forecast selection".into()),
        }
    }
    if actual_corrections != expected_corrections {
        return Err("forecast corrections differ from retained catalog".into());
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
        provider_codes: document.observed.provider_codes,
        station_ids: document.observed.station_ids,
    };
    let observations = ObservationStore::open_read_only(&root.join("sources/observations.sqlite"))
        .map_err(|e| e.to_string())?;
    let (inputs, offsets) = observations
        .read_selection_with_stations(
            &selection,
            &historical,
            &document.observed.replacement_ids,
            &recipe.selected_station_ids,
        )
        .map_err(|e| e.to_string())?;
    if serde_json::to_value(&inputs.series).map_err(|e| e.to_string())? != selected_series
        || serde_json::to_value(&inputs.stations).map_err(|e| e.to_string())? != selected_stations
        || serde_json::to_value(&inputs.replacements).map_err(|e| e.to_string())?
            != replacement_values
    {
        return Err("observation records disagree with recipe".into());
    }
    if offsets != recipe.offsets
        || offsets
            .iter()
            .any(|offset| !first.basins.mini_ids.contains(&offset.mini_id))
    {
        return Err("station offset records disagree with snapshot".into());
    }
    Ok(())
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
    copy_working_inputs(&snapshot.join("baseline/Input"), &input)?;
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
    Ok(EnginePlan {
        executable: snapshot.join("baseline/engine"),
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
    let lock = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(workspace.join(".operation.lock"))
        .map_err(|e| store_error(&workspace, e))?;
    lock.try_lock().map_err(|e| match e {
        std::fs::TryLockError::WouldBlock => OperationError::Busy {
            workspace: workspace.clone(),
        },
        std::fs::TryLockError::Error(e) => store_error(&workspace, e),
    })?;
    let manifest_identity = digest_file(&prepared.join("manifest.json")).map_err(|message| {
        OperationError::InvalidPrepared {
            path: prepared.clone(),
            message,
        }
    })?;
    let mut manifest =
        read_operation_manifest(&prepared).map_err(|e| OperationError::InvalidPrepared {
            path: prepared.clone(),
            message: e,
        })?;
    let recipe: ExecutionRecipe = serde_json::from_value(manifest.recipe.clone()).map_err(|e| {
        OperationError::InvalidPrepared {
            path: prepared.clone(),
            message: e.to_string(),
        }
    })?;
    validate_recipe(&prepared, &recipe, &manifest.scenarios).map_err(|message| {
        OperationError::InvalidPrepared {
            path: prepared.clone(),
            message,
        }
    })?;
    let run_base = workspace.join("data/runs");
    fs::create_dir_all(&run_base).map_err(|e| store_error(&workspace, e))?;
    let run_root = run_base.join(format!(
        "{}-{}",
        chrono::Utc::now().format("%Y%m%dT%H%M%S%.9f"),
        std::process::id()
    ));
    fs::create_dir(&run_root).map_err(|e| store_error(&run_root, e))?;
    let snapshot = run_root.join("snapshot");
    copy_tree(&prepared, &snapshot).map_err(|e| store_error(&run_root, e))?;
    verify_inputs(&snapshot, &manifest.inputs).map_err(|e| store_error(&run_root, e))?;
    if digest_file(&snapshot.join("manifest.json")).map_err(|e| store_error(&run_root, e))?
        != manifest_identity
    {
        return Err(store_error(
            &run_root,
            "prepared manifest changed while freezing",
        ));
    }
    manifest.state = "running".into();
    manifest.created_at = now(&recipe);
    manifest.finished_at = None;
    for item in &mut manifest.inputs {
        item.path = Path::new("snapshot").join(&item.path);
    }
    write_run_manifest(&run_root, &manifest).map_err(|e| store_error(&run_root, e))?;
    for scenario in &mut manifest.scenarios {
        scenario.state = "running".into();
        scenario.started_at = Some(now(&recipe));
    }
    write_run_manifest(&run_root, &manifest).map_err(|e| store_error(&run_root, e))?;
    fs::create_dir(run_root.join("scenarios")).map_err(|e| store_error(&run_root, e))?;
    fs::create_dir(run_root.join("results")).map_err(|e| store_error(&run_root, e))?;
    let records = execute_scenarios(&run_root, &snapshot, &recipe, &manifest.scenarios);
    manifest.scenarios = records;
    let complete = manifest
        .scenarios
        .iter()
        .all(|item| item.state == "completed");
    sync_tree(&run_root).map_err(|e| store_error(&run_root, e))?;
    std::fs::File::open(&run_base)
        .and_then(|file| file.sync_all())
        .map_err(|e| store_error(&run_root, e))?;
    manifest.state = if complete { "completed" } else { "failed" }.into();
    manifest.finished_at = Some(now(&recipe));
    write_run_manifest(&run_root, &manifest).map_err(|e| store_error(&run_root, e))?;
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
                .collect(),
            error: record.error.clone(),
        })
        .collect();
    Ok(ExecutionReport {
        run_artifact: run_root.join("manifest.json"),
        run_root,
        status: if complete {
            BatchStatus::Completed
        } else {
            BatchStatus::Failed
        },
        scenarios,
    })
}

fn execute_scenarios(
    root: &Path,
    snapshot: &Path,
    recipe: &ExecutionRecipe,
    requested: &[ScenarioRecord],
) -> Vec<ScenarioRecord> {
    let first = &requested[0].id;
    let history = prepare_worker(snapshot, root, first, true, recipe).and_then(|plan| {
        let outcome = run_engine(&plan)?;
        if let Some(error) = outcome.error {
            return Err(format!("history run: {error}"));
        }
        let count = recipe
            .historical_count
            .checked_mul(
                read_basin_forcing(&snapshot.join("forcing").join(format!("{first}.nc")))
                    .map_err(|e| e.to_string())?
                    .basins
                    .mini_ids
                    .len(),
            )
            .ok_or("history length overflow")?;
        Ok(read_history(&plan.output, count)?)
    });
    let history = match history {
        Ok(value) => value,
        Err(error) => {
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
    let plans: Vec<_> = requested
        .iter()
        .map(|item| prepare_worker(snapshot, root, &item.id, false, recipe))
        .collect();
    std::thread::scope(|scope| {
        let handles: Vec<_> = requested
            .iter()
            .zip(plans)
            .map(|(item, plan)| {
                let history = &history;
                scope.spawn(move || run_scenario(root, snapshot, recipe, &item.id, plan, history))
            })
            .collect();
        handles
            .into_iter()
            .zip(requested)
            .map(|(handle, item)| match handle.join() {
                Ok(record) => record,
                Err(_) => {
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
    plan: Result<EnginePlan, String>,
    history: &HistoryResults,
) -> ScenarioRecord {
    let mut record = scenario_record(id.into());
    record.state = "running".into();
    record.started_at = Some(now(recipe));
    let result = (|| -> Result<(), String> {
        let plan = plan?;
        record.stdout = Some(Path::new("scenarios").join(id).join("logs/stdout.log"));
        record.stderr = Some(Path::new("scenarios").join(id).join("logs/stderr.log"));
        let outcome = run_engine(&plan)?;
        record.exit_code = outcome.exit_code;
        record.signal = outcome.signal;
        if let Some(error) = outcome.error {
            return Err(error.to_string());
        }
        let mut forcing = read_basin_forcing(&snapshot.join("forcing").join(format!("{id}.nc")))
            .map_err(|e| e.to_string())?;
        let mut hashes = String::new();
        for path in [
            plan.input.join("PARHIG.hig"),
            plan.input.join("Previsao.meta"),
            plan.input.join("CHUVABIN.hig"),
            plan.input.join("MINI.gtp"),
            snapshot.join("baseline/engine"),
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
            digest_file(&snapshot.join("baseline/engine"))?.0.into(),
        );
        processing.insert("snapshot_sha256".into(), snapshot_hash.into());
        forcing.provenance.processing = processing.into();
        let raw = decode_outputs(
            &plan.output,
            forcing,
            recipe.historical_count,
            recipe.forecast_count,
            history,
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
        sync_tree(&result_dir)?;
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
    record
}
