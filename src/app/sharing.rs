use super::{ExecutionReport, ExecutionRequest, OperationControl, OperationError};
use crate::store::{
    BundleManifest, InputReference, OperationManifest, PREPARED_ENGINE, copy_member, copy_tree,
    digest_file, payload_index, publish_directory, read_draft, read_operation_manifest,
    read_retained_manifest, validate_bundle, write_draft,
};
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize)]
pub struct RunBundleReport {
    pub root: PathBuf,
    pub run_id: String,
    pub has_preparation: bool,
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
    for managed in ["cache", "runs/history"] {
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

fn engine_reference(manifest: &OperationManifest) -> Result<&InputReference, String> {
    let mut candidates = manifest.inputs.iter().filter(|file| {
        matches!(
            file.path.to_string_lossy().replace('\\', "/").as_str(),
            "baseline/engine" | "baseline/engine.exe"
        )
    });
    let engine = candidates
        .next()
        .ok_or("preparation has no recorded engine")?;
    if candidates.next().is_some() {
        return Err("preparation has multiple engine references".into());
    }
    Ok(engine)
}

fn checked_bundle(root: &Path) -> Result<BundleManifest, String> {
    let bundle = validate_bundle(root)?;
    let trial = super::read_trial(root).map_err(|error| error.to_string())?;
    if let Some(engine) = &bundle.preparation_engine {
        let root = root.join("preparation");
        let manifest_file = root.join("manifest.json");
        let identity = digest_file(&manifest_file)?.0;
        if trial.preparation_sha256.as_deref() != Some(&identity) {
            return Err("bundled preparation does not match the trial".into());
        }
        let manifest: OperationManifest = read_draft(&manifest_file)?;
        if manifest.state != "prepared" || engine_reference(&manifest)? != engine {
            return Err("bundled preparation has an invalid engine reference".into());
        }
        let mut remaining = bundle
            .files
            .iter()
            .filter_map(|entry| {
                let relative = entry.path.to_str()?.strip_prefix("preparation/")?;
                (relative != "manifest.json").then_some((relative, entry))
            })
            .collect::<std::collections::HashMap<_, _>>();
        for input in manifest.inputs.iter().filter(|input| *input != engine) {
            let path = input
                .path
                .to_str()
                .ok_or("non-UTF8 preparation path")?
                .replace('\\', "/");
            let file = remaining
                .remove(path.as_str())
                .ok_or_else(|| format!("missing preparation member {path}"))?;
            if file.size != input.size || file.sha256 != input.sha256 {
                return Err(format!("preparation integrity mismatch: {path}"));
            }
        }
        if !remaining.is_empty() {
            return Err("unreferenced preparation files in bundle".into());
        }
    } else if bundle
        .files
        .iter()
        .any(|file| file.path.starts_with("preparation"))
    {
        return Err("unregistered preparation in bundle".into());
    }
    if bundle.files.iter().any(|file| {
        file.path != Path::new("manifest.json")
            && ["results", "scenarios", "preparation"]
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
    preparation: Option<&Path>,
    destination: &Path,
) -> Result<RunBundleReport, String> {
    let run_path = run.canonicalize().map_err(|e| e.to_string())?;
    let name = run_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or("expected a run directory name")?;
    if let Some(workspace) = run_path.ancestors().nth(3)
        && run_path.parent() == Some(workspace.join("cache/run").as_path())
        && let Some(run_id) = name.strip_prefix("generation-")
    {
        return super::operation::with_active_run(workspace, |active| {
            if active != run_path {
                return Err("selected run is no longer active".into());
            }
            check_detached_destination(workspace, destination)?;
            export_run_directory(run, run_id, preparation, destination)
        })?
        .ok_or_else(|| "workspace has no active run".into());
    }
    export_run_directory(run, name, preparation, destination)
}

fn export_run_directory(
    run: &Path,
    run_id: &str,
    preparation: Option<&Path>,
    destination: &Path,
) -> Result<RunBundleReport, String> {
    let parent = absolute_parent(destination)?;
    if parent.starts_with(run.canonicalize().map_err(|error| error.to_string())?) {
        return Err("bundle destination is inside its source run".into());
    }
    if !fs::symlink_metadata(run).is_ok_and(|metadata| metadata.is_dir()) {
        return Err("expected a real run directory".into());
    }
    let trial = super::read_trial(run).map_err(|error| error.to_string())?;
    let manifest = read_retained_manifest(run)?;
    let prepared = preparation
        .map(|root| -> Result<_, String> {
            if parent.starts_with(root.canonicalize().map_err(|error| error.to_string())?) {
                return Err("bundle destination is inside its source preparation".into());
            }
            let manifest = read_operation_manifest(root)?;
            let digest = digest_file(&root.join("manifest.json"))?.0;
            if trial.preparation_sha256.as_deref() != Some(&digest) {
                return Err("preparation does not match the trial".into());
            }
            let engine = engine_reference(&manifest)?.clone();
            Ok((root, manifest, engine))
        })
        .transpose()?;
    publish_directory(destination, |stage| {
        copy_member(run, Path::new("manifest.json"), stage)?;
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
        if let Some((source, manifest, engine)) = &prepared {
            let root = stage.join("preparation");
            copy_member(source, Path::new("manifest.json"), &root)?;
            for file in manifest.inputs.iter().filter(|file| *file != engine) {
                copy_member(source, &file.path, &root)?;
            }
        }
        let bundle = BundleManifest {
            version: 1,
            run_id: run_id.into(),
            preparation_engine: prepared.map(|(_, _, engine)| engine),
            files: payload_index(stage)?,
        };
        write_draft(&stage.join("bundle.json"), &bundle)?;
        checked_bundle(stage)?;
        Ok(RunBundleReport {
            root: destination.into(),
            run_id: bundle.run_id,
            has_preparation: bundle.preparation_engine.is_some(),
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
            .chain([Path::new("bundle.json")])
        {
            copy_member(source, path, stage)?;
        }
        let staged = checked_bundle(stage)?;
        if staged.files != bundle.files
            || staged.run_id != bundle.run_id
            || staged.preparation_engine != bundle.preparation_engine
        {
            return Err("bundle changed while copying".into());
        }
        Ok(RunBundleReport {
            root: destination.into(),
            run_id: bundle.run_id,
            has_preparation: bundle.preparation_engine.is_some(),
        })
    })
}

pub fn replay_run(
    workspace: &Path,
    manifest_path: &Path,
    control: &OperationControl,
) -> Result<ExecutionReport, OperationError> {
    use super::draft::{ManifestRecipe, read_recipe_manifest};
    use super::operation::{ExecutionRecipe, lock_workspace, run_preparation_locked};
    use super::prepare::{TypedPreparationRequest, prepare_replay};

    control.check()?;
    let invalid = |message| OperationError::InvalidRequest {
        field: "manifest".into(),
        message,
    };
    let workspace = workspace
        .canonicalize()
        .map_err(|error| OperationError::Source {
            path: workspace.into(),
            message: error.to_string(),
        })?;
    if !manifest_path.is_file() {
        return Err(invalid("expected a standalone manifest file".into()));
    }
    let manifest = read_recipe_manifest(manifest_path).map_err(invalid)?;
    let recipe: ExecutionRecipe = serde_json::from_value(manifest.recipe.clone())
        .map_err(|error| invalid(error.to_string()))?;
    if recipe
        .operational_workspace
        .as_ref()
        .is_some_and(|root| root != &workspace)
    {
        return Err(invalid(
            "manifest belongs to a different operational workspace".into(),
        ));
    }
    let ManifestRecipe {
        preparation: document,
        replacements,
        corrections,
    } = ManifestRecipe::decode(&manifest).map_err(invalid)?;
    let replacements = recipe.inline_replacements.then_some(replacements);
    let corrections = corrections
        .into_iter()
        .enumerate()
        .map(|(index, entries)| entries.filter(|_| recipe.inline_corrections.contains(&index)))
        .collect();

    let lock = lock_workspace(&workspace)?;
    run_preparation_locked(control, &workspace, &lock, |workspace, output, control| {
        prepare_replay(
            TypedPreparationRequest {
                workspace: workspace.into(),
                invocation_dir: workspace.into(),
                output,
                document,
                replacements,
                corrections,
            },
            &manifest,
            control,
        )
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
    let required = metadata.preparation_engine.ok_or_else(|| {
        fail("bundle contains results only; a matching retained preparation is required".into())
    })?;
    let engine = engine
        .canonicalize()
        .map_err(|error| fail(error.to_string()))?;
    if digest_file(&engine).map_err(fail)? != (required.sha256, required.size) {
        return Err(fail(
            "local executable does not match the required engine digest".into(),
        ));
    }
    let workspace = workspace
        .canonicalize()
        .map_err(|error| fail(error.to_string()))?;
    let base = workspace.join("cache");
    fs::create_dir_all(&base).map_err(|error| fail(error.to_string()))?;
    let snapshot = base.join(format!(
        ".regenerate-{}-{}",
        std::process::id(),
        chrono::Utc::now().format("%Y%m%d%H%M%S%.9f")
    ));
    publish_directory(&snapshot, |stage| {
        let source = bundle.join("preparation");
        for entry in &metadata.files {
            if let Ok(path) = entry.path.strip_prefix("preparation") {
                copy_member(&source, path, stage)?;
            }
        }
        let expected_engine = Path::new(PREPARED_ENGINE);
        if required.path != expected_engine {
            return Err("retained preparation requires an engine for a different platform".into());
        }
        copy_tree(&engine, &stage.join(required.path))?;
        read_operation_manifest(stage)?;
        Ok(())
    })
    .map_err(fail)?;
    let result = super::execute_prepared_controlled(
        ExecutionRequest {
            invocation_dir: workspace.clone(),
            workspace,
            prepared: snapshot.clone(),
        },
        control,
    );
    if let Err(error) = fs::remove_dir_all(&snapshot) {
        return Err(fail(format!(
            "regeneration cleanup for {} failed: {error}",
            snapshot.display()
        )));
    }
    result
}
