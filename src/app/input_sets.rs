use super::{ExecutionReport, OperationControl, OperationError, Session};
use crate::store::{InputSetManifest, current_input_root, read_input_set};
use std::{
    fs,
    path::{Path, PathBuf},
};

fn fail(path: &Path, message: impl ToString) -> OperationError {
    OperationError::Store {
        run_root: path.into(),
        message: message.to_string(),
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
    operation.acquire_locked(control)?;
    let id = format!(
        "inputs-{}-{}",
        std::process::id(),
        chrono::Utc::now().format("%Y%m%d%H%M%S%9f")
    );
    let base = workspace.join("work");
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
    let copied =
        crate::store::materialize_input_set(&inputs, &output).map_err(|e| fail(&inputs, e))?;
    if copied.recipe != original.recipe || copied.files != original.files {
        return Err(fail(&inputs, "selection changed while resolving"));
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
    if inputs.join("sources/observations.sqlite").is_file() {
        document.observed.path = inputs.join("sources/observations.sqlite");
    } else if document.observed.path.ends_with("assets/registry.sqlite") {
        document.observed.path = super::observation_assets(workspace);
    }
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
            forcing.reservoir_schedules = Some(inputs.join("sources/reservoir-schedules.toml"));
        }
        if forcing.wind_observations.is_some() {
            forcing.wind_observations = Some(inputs.join("sources/wind-observations.toml"));
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
