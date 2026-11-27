use super::{ExecutionReport, OperationControl, OperationError};
use crate::store::{
    BundleManifest, copy_member, payload_index, publish_directory, read_input_set,
    read_retained_manifest, validate_bundle, write_draft,
};
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize)]
pub struct RunBundleReport {
    pub root: PathBuf,
    pub run_id: String,
    pub has_inputs: bool,
}

fn absolute_parent(path: &Path) -> Result<PathBuf, String> {
    let parent = path.parent().filter(|p| !p.as_os_str().is_empty());
    parent
        .unwrap_or(Path::new("."))
        .canonicalize()
        .map_err(|e| e.to_string())
}

fn check_detached_destination(workspace: &Path, destination: &Path) -> Result<(), String> {
    let resolved_destination = absolute_parent(destination)?.join(
        destination
            .file_name()
            .ok_or("destination needs a filename")?,
    );
    let data = workspace.canonicalize().map_err(|e| e.to_string())?;
    let resolved_data = data.canonicalize().unwrap_or_else(|_| data.clone());
    for managed in ["cache", "assets", "runs", "work", "results"] {
        let managed_path = data.join(managed);
        if resolved_destination.starts_with(resolved_data.join(managed))
            || resolved_destination.starts_with(&managed_path)
            || managed_path
                .canonicalize()
                .is_ok_and(|path| resolved_destination.starts_with(path))
        {
            return Err(
                "bundle destination is inside the managed run cache or manifest archive".into(),
            );
        }
    }
    Ok(())
}

fn bind_inputs(
    manifest: &crate::store::OperationManifest,
    inputs: &crate::store::InputSetManifest,
) -> Result<(), String> {
    let mut recipe = manifest.recipe.clone();
    recipe
        .as_object_mut()
        .ok_or("invalid run recipe")?
        .remove("input_set_id");
    if inputs.files != manifest.inputs || inputs.recipe != recipe {
        return Err("input values or recipe do not match the completed run".into());
    }
    Ok(())
}

fn checked_bundle(root: &Path) -> Result<BundleManifest, String> {
    let bundle = validate_bundle(root)?;
    super::read_trial(root).map_err(|error| error.to_string())?;
    let manifest = read_retained_manifest(root)?;
    if root.join("legacy-manifest.json").exists() {
        let reference=manifest.inputs.iter().find(|f|f.path==Path::new("legacy-manifest.json")).ok_or("unregistered legacy manifest evidence")?;
        if manifest.recipe["legacy_manifest_sha256"].as_str()!=Some(reference.sha256.as_str()) {return Err("legacy manifest binding differs".into());}
        crate::store::verify_inputs(root,std::slice::from_ref(reference))?;
    }
    let run_input_id = manifest
        .recipe
        .get("input_set_id")
        .and_then(serde_json::Value::as_str);
    if let Some(id) = &bundle.input_set_id {
        let inputs = read_input_set(&root.join("inputs"))?;
        if &inputs.id != id || run_input_id != Some(id.as_str()) {
            return Err("bundled input set does not match the run".into());
        }
        bind_inputs(&manifest, &inputs)?;
        let mut expected = inputs.files;
        if let Some(snapshot) = inputs.observation_snapshot {
            expected.push(snapshot);
        }
        expected.push(crate::store::InputReference {
            path: "input-set.toml".into(),
            kind: "input-set.toml".into(),
            sha256: crate::store::digest_file(&root.join("inputs/input-set.toml"))?.0,
            size: std::fs::metadata(root.join("inputs/input-set.toml"))
                .map_err(|e| e.to_string())?
                .len(),
        });
        let mut actual: Vec<_> = bundle
            .files
            .iter()
            .filter_map(|entry| {
                entry.path.strip_prefix("inputs").ok().and_then(|path| {
                    let mut value = entry.clone();
                    value.path = path.into();
                    value.kind = path
                        .components()
                        .next()?
                        .as_os_str()
                        .to_string_lossy()
                        .into_owned();
                    Some(value)
                })
            })
            .collect();
        expected.sort_by(|a, b| a.path.cmp(&b.path));
        actual.sort_by(|a, b| a.path.cmp(&b.path));
        if actual != expected {
            return Err("input set has unexpected or missing bundle members".into());
        }
    } else if bundle
        .files
        .iter()
        .any(|file| file.path.starts_with("inputs"))
    {
        return Err("unregistered input set in bundle".into());
    }
    if bundle.files.iter().any(|file| {
        file.path != Path::new("manifest.toml")
            && file.path != Path::new("legacy-manifest.json")
            && ["results", "scenarios", "inputs"]
                .iter()
                .all(|entry| !file.path.starts_with(entry))
    }) {
        return Err("unexpected file in run bundle".into());
    }
    Ok(bundle)
}

fn copy_optional(root: &Path, relative: &Path, stage: &Path) -> Result<(), String> {
    match fs::symlink_metadata(root.join(relative)) {
        Ok(_) => copy_member(root, relative, stage),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.to_string()),
    }
}

pub fn export_run(
    run: &Path,
    inputs: Option<&Path>,
    destination: &Path,
) -> Result<RunBundleReport, String> {
    let run_path = run.canonicalize().map_err(|e| e.to_string())?;
    let name = run_path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or("expected run directory")?;
    if let Some(workspace) = run_path.parent().and_then(Path::parent)
        && run_path.parent() == Some(workspace.join("results").as_path())
    {
        check_detached_destination(workspace, destination)?;
        let lock =
            fs::File::open(workspace.join("results/access.lock")).map_err(|e| e.to_string())?;
        lock.lock_shared().map_err(|e| e.to_string())?;
        if !run_path.join("manifest.toml").is_file() {
            return Err("selected run results are no longer available".into());
        }
        let manifest = read_retained_manifest(&run_path)?;
        let inferred = manifest
            .recipe
            .get("input_set_id")
            .and_then(|v| v.as_str())
            .map(|id| workspace.join("assets/selections").join(id));
        return export_run_directory(&run_path, name, inputs.or(inferred.as_deref()), destination);
    }
    export_run_directory(&run_path, name, inputs, destination)
}

fn export_run_directory(
    run: &Path,
    run_id: &str,
    inputs: Option<&Path>,
    destination: &Path,
) -> Result<RunBundleReport, String> {
    let parent = absolute_parent(destination)?;
    if parent.starts_with(run.canonicalize().map_err(|error| error.to_string())?) {
        return Err("bundle destination is inside its source run".into());
    }
    if !fs::symlink_metadata(run).is_ok_and(|metadata| metadata.is_dir()) {
        return Err("expected a real run directory".into());
    }
    let manifest = read_retained_manifest(run)?;
    let selected_inputs = inputs
        .map(|root| -> Result<_, String> {
            if parent.starts_with(root.canonicalize().map_err(|error| error.to_string())?) {
                return Err("bundle destination is inside its source input set".into());
            }
            let inputs = read_input_set(root)?;
            bind_inputs(&manifest, &inputs)?;
            if manifest
                .recipe
                .get("input_set_id")
                .and_then(serde_json::Value::as_str)
                != Some(inputs.id.as_str())
            {
                return Err("input set does not match the run".into());
            }
            Ok((root, inputs))
        })
        .transpose()?;
    publish_directory(destination, |stage| {
        copy_member(run, Path::new("manifest.toml"), stage)?;
        if let Some(reference)=manifest.inputs.iter().find(|f|f.path==Path::new("legacy-manifest.json")) {
            crate::store::verify_inputs(run,std::slice::from_ref(reference))?;
            copy_member(run,Path::new("legacy-manifest.json"),stage)?;
        }
        copy_optional(run, Path::new("scenarios/.history/logs"), stage)?;
        let mut scenarios = std::collections::HashSet::new();
        for scenario in manifest.scenarios {
            if scenario.id.is_empty()
                || scenario.id == ".history"
                || !scenario
                    .id
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
                || !scenarios.insert(scenario.id.clone())
            {
                return Err("invalid or duplicate scenario ID".into());
            }
            let worker = Path::new("scenarios").join(&scenario.id);
            copy_optional(run, &worker.join("logs"), stage)?;
            if scenario.state == "completed" {
                copy_member(run, &Path::new("results").join(scenario.id), stage)?;
                copy_optional(run, &worker.join("Output"), stage)?;
                copy_optional(run, &worker.join("state"), stage)?;
            }
        }
        if let Some((source, inputs)) = &selected_inputs {
            let root = stage.join("inputs");
            let _ = inputs;
            crate::store::save_input_set(source, &root)?;
        }
        let bundle = BundleManifest {
            run_id: run_id.into(),
            input_set_id: selected_inputs
                .as_ref()
                .map(|(_, inputs)| inputs.id.clone()),
            files: payload_index(stage)?,
        };
        write_draft(&stage.join("bundle.toml"), &bundle)?;
        checked_bundle(stage)?;
        Ok(RunBundleReport {
            root: destination.into(),
            run_id: bundle.run_id,
            has_inputs: bundle.input_set_id.is_some(),
        })
    })
}

pub fn import_run(workspace: &Path, source: &Path) -> Result<RunBundleReport, String> {
    let bundle = checked_bundle(source)?;
    let run_base = workspace
        .canonicalize()
        .map_err(|error| error.to_string())?
        .join("saved-runs");
    fs::create_dir_all(&run_base).map_err(|error| error.to_string())?;
    import_checked_run(workspace, source, &run_base.join(&bundle.run_id), bundle)
}

pub fn import_run_to(
    workspace: &Path,
    source: &Path,
    destination: &Path,
) -> Result<RunBundleReport, String> {
    let bundle = checked_bundle(source)?;
    import_checked_run(workspace, source, destination, bundle)
}

fn import_checked_run(
    workspace: &Path,
    source: &Path,
    destination: &Path,
    bundle: BundleManifest,
) -> Result<RunBundleReport, String> {
    let parent = absolute_parent(destination)?;
    let source_root = source.canonicalize().map_err(|e| e.to_string())?;
    if parent.starts_with(source_root) {
        return Err("import destination is inside its source bundle".into());
    }
    check_detached_destination(workspace, destination)?;
    publish_directory(destination, |stage| {
        for path in bundle
            .files
            .iter()
            .map(|file| file.path.as_path())
            .chain([Path::new("bundle.toml")])
        {
            copy_member(source, path, stage)?;
        }
        let staged = checked_bundle(stage)?;
        if staged.files != bundle.files
            || staged.run_id != bundle.run_id
            || staged.input_set_id != bundle.input_set_id
        {
            return Err("bundle changed while copying".into());
        }
        Ok(RunBundleReport {
            root: destination.into(),
            run_id: bundle.run_id,
            has_inputs: bundle.input_set_id.is_some(),
        })
    })
}

pub fn regenerate_run(
    workspace: &Path,
    bundle: &Path,
    engine: &Path,
    control: &OperationControl,
) -> Result<ExecutionReport, OperationError> {
    let fail = |message| OperationError::Store {
        run_root: bundle.into(),
        message,
    };
    control.check()?;
    let metadata = checked_bundle(bundle).map_err(fail)?;
    metadata
        .input_set_id
        .ok_or_else(|| fail("bundle contains results only; an input set is required".into()))?;
    super::execute_input_set(workspace, &bundle.join("inputs"), engine, control)
}
