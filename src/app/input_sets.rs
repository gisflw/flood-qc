use super::{ExecutionReport, OperationControl, OperationError, Session};
use crate::store::{
    CurrentInputSet, InputSetManifest, current_input_root, current_input_set, read_input_set,
};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum InputDisposition {
    Save { destination: PathBuf },
    Discard,
}
fn fail(path: &Path, message: impl ToString) -> OperationError {
    OperationError::Store {
        run_root: path.into(),
        message: message.to_string(),
    }
}
enum InputCacheState {
    Empty,
    Incomplete,
    Current(CurrentInputSet),
}

fn inspect_input_cache(workspace: &Path) -> Result<InputCacheState, String> {
    let cache = workspace.join("cache/inputs");
    match current_input_set(workspace) {
        Err(_) => Ok(InputCacheState::Incomplete),
        Ok(Some(current)) => {
            if cache.join(&current.id).join("input-set.json").is_file() {
                Ok(InputCacheState::Current(current))
            } else {
                Ok(InputCacheState::Incomplete)
            }
        }
        Ok(None) => match fs::symlink_metadata(&cache) {
            Ok(metadata) if metadata.is_dir() => {
                let mut entries = fs::read_dir(&cache).map_err(|e| e.to_string())?;
                if entries.next().is_some() {
                    Ok(InputCacheState::Incomplete)
                } else {
                    Ok(InputCacheState::Empty)
                }
            }
            Ok(_) => Ok(InputCacheState::Incomplete),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                Ok(InputCacheState::Empty)
            }
            Err(error) => Err(error.to_string()),
        },
    }
}

/// Whether replacing or leaving the workspace requires an input decision.
pub fn workspace_inputs_need_decision(workspace: &Path) -> Result<bool, String> {
    Ok(match inspect_input_cache(workspace)? {
        InputCacheState::Empty => false,
        InputCacheState::Incomplete => true,
        InputCacheState::Current(current) => current.saved_to.is_none(),
    })
}

pub(super) fn resolve_input_replacement(
    workspace: &Path,
    control: &OperationControl,
) -> Result<(), OperationError> {
    let cache = workspace.join("cache/inputs");
    match inspect_input_cache(workspace).map_err(|e| fail(&cache, e))? {
        InputCacheState::Empty => Ok(()),
        InputCacheState::Incomplete => {
            if matches!(control.input_disposition(), Some(InputDisposition::Discard)) {
                crate::store::discard_current_inputs(workspace).map_err(|e| fail(&cache, e))
            } else {
                Err(fail(
                    &cache,
                    "incomplete input cache; discard it before continuing",
                ))
            }
        }
        InputCacheState::Current(current) if current.saved_to.is_none() => {
            match control.input_disposition() {
                Some(InputDisposition::Save { destination }) => {
                    crate::store::save_current_inputs(workspace, destination)
                        .map_err(|e| fail(destination, e))?;
                    Ok(())
                }
                Some(InputDisposition::Discard) => Ok(()),
                None => Err(OperationError::InputDecisionRequired {
                    input_set: current.id,
                }),
            }
        }
        InputCacheState::Current(_) => Ok(()),
    }
}

pub fn save_workspace_inputs(
    workspace: &Path,
    destination: &Path,
) -> Result<InputSetManifest, String> {
    let _lock = super::operation::lock_workspace_job(workspace).map_err(|e| e.to_string())?;
    let destination = super::absolute(
        &std::env::current_dir().map_err(|e| e.to_string())?,
        destination,
    );
    let parent = destination
        .parent()
        .ok_or("save destination needs a parent")?
        .canonicalize()
        .map_err(|e| e.to_string())?;
    let destination = parent.join(
        destination
            .file_name()
            .ok_or("save destination needs a name")?,
    );
    if destination.starts_with(workspace.join("cache"))
        || destination.starts_with(workspace.join("assets"))
    {
        return Err("save inputs outside managed cache and acquisition assets".into());
    }
    crate::store::save_current_inputs(workspace, &destination)
}
pub fn discard_workspace_inputs(workspace: &Path) -> Result<(), String> {
    let _lock = super::operation::lock_workspace_job(workspace).map_err(|e| e.to_string())?;
    crate::store::discard_current_inputs(workspace)
}
pub fn capture_workspace_inputs(
    session: &Session,
    control: &OperationControl,
) -> Result<InputSetManifest, OperationError> {
    let (_job, setup) = session
        .start_job_with_setup()
        .map_err(|e| fail(session.workspace(), e))?;
    let operation = setup
        .ok_or_else(|| fail(session.workspace(), "configure the run first"))?
        .operation;
    capture_run_inputs(&operation, control)
}

pub fn capture_run_inputs(
    operation: &super::OperationalRun,
    control: &OperationControl,
) -> Result<InputSetManifest, OperationError> {
    let workspace = operation.workspace();
    let _lock = super::operation::lock_workspace(workspace)?;
    resolve_input_replacement(workspace, control)?;
    operation.acquire_locked(control)?;
    let id = format!(
        "inputs-{}-{}",
        std::process::id(),
        chrono::Utc::now().format("%Y%m%d%H%M%S%9f")
    );
    let base = workspace.join("cache/inputs");
    fs::create_dir_all(&base).map_err(|e| fail(&base, e))?;
    let temporary = base.join(format!(".{id}"));
    let result = operation.capture_locked(workspace.into(), temporary.clone(), control);
    let report = result?;
    let root = crate::store::publish_captured_input_set(workspace, &report.prepared, &id)
        .map_err(|e| fail(workspace, e))?;
    read_input_set(&root).map_err(|e| fail(&root, e))
}

/// Rebuilds disposable forcing/scenarios entirely from an input set and a matching installation.
pub fn execute_input_set(
    workspace: &Path,
    inputs: &Path,
    engine: &Path,
    control: &OperationControl,
) -> Result<ExecutionReport, OperationError> {
    let workspace = workspace.canonicalize().map_err(|e| fail(workspace, e))?;
    let lock = super::operation::lock_workspace(&workspace)?;
    super::operation::run_preparation_locked(
        control,
        &workspace,
        &lock,
        |workspace, output, control| {
            materialize_captured(workspace, inputs, engine, output, control)
        },
    )
}

pub(super) fn materialize_captured(
    workspace: &Path,
    inputs: &Path,
    engine: &Path,
    output: PathBuf,
    control: &OperationControl,
) -> Result<super::PreparationReport, OperationError> {
    let inputs = inputs.canonicalize().map_err(|e| fail(inputs, e))?;
    let original = read_input_set(&inputs).map_err(|e| fail(&inputs, e))?;
    let copied = crate::store::save_input_set(&inputs, &output).map_err(|e| fail(&inputs, e))?;
    if serde_json::to_value(&copied).map_err(|e| fail(&inputs, e))?
        != serde_json::to_value(&original).map_err(|e| fail(&inputs, e))?
    {
        let _ = fs::remove_dir_all(&output);
        return Err(fail(&inputs, "input set changed while copying"));
    }
    let result = materialize_current_input_set(workspace, &output, engine, control);
    if result.is_err() {
        let _ = fs::remove_dir_all(&output);
    }
    result
}

pub(super) fn materialize_current_input_set(
    workspace: &Path,
    inputs: &Path,
    engine: &Path,
    control: &OperationControl,
) -> Result<super::PreparationReport, OperationError> {
    let inputs = inputs.canonicalize().map_err(|e| fail(inputs, e))?;
    let frozen = read_input_set(&inputs).map_err(|e| fail(&inputs, e))?;
    let engine = engine.canonicalize().map_err(|e| fail(engine, e))?;
    if crate::store::digest_file(&engine).map_err(|e| fail(&engine, e))?
        != (frozen.engine.sha256.clone(), frozen.engine.size)
    {
        return Err(fail(
            &engine,
            "installed executable does not match the input set",
        ));
    }
    let recipe: super::operation::ExecutionRecipe =
        serde_json::from_value(frozen.recipe).map_err(|e| fail(&inputs, e))?;
    let mut value = recipe.document.clone();
    let object = value
        .as_object_mut()
        .ok_or_else(|| fail(&inputs, "invalid input recipe"))?;
    for key in ["selected_series", "selected_stations", "replacement_values"] {
        object.remove(key);
    }
    let mut document: super::PreparationDocument =
        serde_json::from_value(value).map_err(|e| fail(&inputs, e))?;
    document.observed.path = inputs.join("sources/observations");
    document.forecasts.catalog = inputs.join("sources/forecasts");
    document.engine.executable = engine;
    document.engine.input_dir = inputs.join("baseline/Input");
    document.geometry.mini_gtp = document.engine.input_dir.join(
        if document
            .calibration
            .as_ref()
            .is_some_and(|c| c.use_edited_geometry)
        {
            "Cell.hig"
        } else {
            "MINI.gtp"
        },
    );
    if let Some(calibration) = &mut document.calibration
        && calibration.soil.is_some()
    {
        calibration.soil = Some(document.engine.input_dir.join("paruso.hig"));
    }
    if let Some(forcing) = &mut document.forcing {
        if forcing.reservoir_schedules.is_some() {
            forcing.reservoir_schedules = Some(inputs.join("sources/reservoir-schedules.parquet"));
        }
        if forcing.wind_observations.is_some() {
            forcing.wind_observations = Some(inputs.join("sources/wind-observations.parquet"));
        }
    }
    if recipe.workspace_scenarios.is_none() {
        return Err(fail(&inputs, "input set has no scenario recipe"));
    }
    super::prepare::materialize_input_root(workspace, &inputs, document, recipe, control)
}

pub fn current_workspace_inputs(workspace: &Path) -> Result<Option<InputSetManifest>, String> {
    current_input_root(workspace)?
        .map(|root| read_input_set(&root))
        .transpose()
}

#[cfg(test)]
mod recovery_tests {
    use super::*;

    #[test]
    fn explicit_discard_recovers_incomplete_cache_without_deleting_saved_files() {
        fs::create_dir_all("runs").unwrap();
        let temporary = tempfile::tempdir_in("runs").unwrap();
        let workspace = temporary.path();
        let cache = workspace.join("cache/inputs");
        let archive = workspace.join("saved-inputs");
        let empty_requires_decision = workspace_inputs_need_decision(workspace).unwrap();
        let empty_is_clear = !empty_requires_decision;
        assert!(empty_is_clear);
        fs::create_dir_all(&cache).unwrap();
        let empty_directory_is_clear = !workspace_inputs_need_decision(workspace).unwrap();
        assert!(empty_directory_is_clear);
        fs::create_dir_all(cache.join("orphan")).unwrap();
        assert!(workspace_inputs_need_decision(workspace).unwrap());
        fs::create_dir_all(&archive).unwrap();
        fs::write(archive.join("input-set.json"), b"saved").unwrap();
        fs::write(cache.join("current.json"), b"{").unwrap();
        let unspecified = OperationControl::default();
        let save = OperationControl::default().with_input_disposition(InputDisposition::Save {
            destination: workspace.join("other-save"),
        });
        assert!(resolve_input_replacement(workspace, &unspecified).is_err());
        assert!(resolve_input_replacement(workspace, &save).is_err());
        assert!(cache.join("orphan").is_dir());
        let discard = OperationControl::default().with_input_disposition(InputDisposition::Discard);
        resolve_input_replacement(workspace, &discard).unwrap();
        let cache_removed = !cache.exists();
        assert!(cache_removed);
        assert!(archive.join("input-set.json").is_file());

        fs::create_dir_all(cache.join("orphan")).unwrap();
        assert!(resolve_input_replacement(workspace, &unspecified).is_err());
        assert!(resolve_input_replacement(workspace, &save).is_err());
        resolve_input_replacement(workspace, &discard).unwrap();
        let cache_removed = !cache.exists();
        assert!(cache_removed);

        fs::create_dir_all(cache.join("orphan")).unwrap();
        fs::write(
            cache.join("current.json"),
            br#"{"id":"missing","saved_to":null}"#,
        )
        .unwrap();
        assert!(resolve_input_replacement(workspace, &unspecified).is_err());
        assert!(resolve_input_replacement(workspace, &save).is_err());
        resolve_input_replacement(workspace, &discard).unwrap();
        let cache_removed = !cache.exists();
        assert!(cache_removed);
        assert!(archive.join("input-set.json").is_file());
        resolve_input_replacement(workspace, &discard).unwrap();

        fs::create_dir_all(cache.join("valid")).unwrap();
        fs::write(cache.join("valid/input-set.json"), b"present").unwrap();
        fs::write(
            cache.join("current.json"),
            br#"{"id":"valid","saved_to":null}"#,
        )
        .unwrap();
        assert!(workspace_inputs_need_decision(workspace).unwrap());
        resolve_input_replacement(workspace, &discard).unwrap();
        assert!(cache.join("valid/input-set.json").is_file());
        fs::write(
            cache.join("current.json"),
            br#"{"id":"valid","saved_to":"archive"}"#,
        )
        .unwrap();
        let saved_requires_decision = workspace_inputs_need_decision(workspace).unwrap();
        let saved_is_clear = !saved_requires_decision;
        assert!(saved_is_clear);
    }
}
