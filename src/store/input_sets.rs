use super::{InputReference, publish_directory, read_draft, write_draft};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};

// Cache pointers need atomic replacement during normal operation, but are disposable
// after a power loss. Durable exports use the separate synced publication path.
pub(crate) fn write_cache_json(path: &Path, value: &impl Serialize) -> Result<(), String> {
    use std::io::Write;
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let parent = path.parent().ok_or("cache file needs a parent")?;
    let name = path
        .file_name()
        .ok_or("cache file needs a name")?
        .to_string_lossy();
    let temporary = parent.join(format!(
        ".{name}.{}.{}.tmp",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let result = (|| -> Result<(), String> {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|e| e.to_string())?;
        serde_json::to_writer_pretty(&mut file, value).map_err(|e| e.to_string())?;
        file.write_all(b"\n").map_err(|e| e.to_string())?;
        drop(file);
        fs::rename(&temporary, path).map_err(|e| e.to_string())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InputSetManifest {
    pub version: u32,
    pub id: String,
    pub created_at: String,
    pub engine: InputReference,
    pub recipe: serde_json::Value,
    pub files: Vec<InputReference>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CurrentInputSet {
    pub id: String,
    pub saved_to: Option<PathBuf>,
}

fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 96
        && id.as_bytes()[0].is_ascii_alphanumeric()
        && !id.ends_with('.')
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_-.".contains(&b))
}

pub(crate) fn input_cache(workspace: &Path) -> PathBuf {
    workspace.join("cache/inputs")
}
pub fn current_input_set(workspace: &Path) -> Result<Option<CurrentInputSet>, String> {
    let path = input_cache(workspace).join("current.json");
    if !path.try_exists().map_err(|e| e.to_string())? {
        return Ok(None);
    }
    let current: CurrentInputSet = read_draft(&path)?;
    if !valid_id(&current.id) {
        return Err("invalid current input-set identity".into());
    }
    Ok(Some(current))
}
pub fn current_input_root(workspace: &Path) -> Result<Option<PathBuf>, String> {
    Ok(current_input_set(workspace)?.map(|set| input_cache(workspace).join(set.id)))
}
pub fn read_input_set(root: &Path) -> Result<InputSetManifest, String> {
    let manifest: InputSetManifest = read_draft(&root.join("input-set.json"))?;
    if manifest.version != 1 || !valid_id(&manifest.id) {
        return Err("unsupported input set".into());
    }
    if manifest
        .files
        .iter()
        .any(|f| !(f.path.starts_with("sources") || f.path.starts_with("baseline/Input")))
    {
        return Err("input set contains a generated artifact or invalid path".into());
    }
    let actual = super::operation::collect_selected_inputs(
        root,
        &[Path::new("baseline/Input"), Path::new("sources")],
    )?;
    if manifest.files.is_empty() {
        return Err("input-set payload does not match its complete index".into());
    }
    super::operation::validate_collected_inputs(root, &manifest.files, actual, &[])?;
    if !manifest
        .files
        .iter()
        .any(|file| file.path == Path::new("sources/recipe.json"))
    {
        return Err("missing indexed input-set recipe".into());
    }
    let recorded_recipe: serde_json::Value = read_draft(&root.join("sources/recipe.json"))?;
    if recorded_recipe != manifest.recipe {
        return Err("input-set recipe differs from indexed recipe".into());
    }
    let recipe = manifest
        .recipe
        .as_object()
        .ok_or("invalid input-set recipe")?;
    if recipe.get("engine_identity")
        != Some(&serde_json::to_value(&manifest.engine).map_err(|e| e.to_string())?)
    {
        return Err("input-set engine identity differs from recipe".into());
    }
    let document = recipe
        .get("document")
        .and_then(|v| v.as_object())
        .ok_or("missing input document")?;
    let scenarios = document
        .get("scenarios")
        .and_then(|v| v.as_array())
        .ok_or("missing scenarios")?;
    let resolved = recipe
        .get("workspace_scenarios")
        .and_then(|v| v.as_array())
        .ok_or("missing resolved scenarios")?;
    if scenarios.is_empty() || scenarios.len() != resolved.len() {
        return Err("invalid resolved scenario membership".into());
    }
    let mut seen = std::collections::HashSet::new();
    for scenario in scenarios {
        let id = scenario
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or("missing scenario ID")?;
        if !valid_id(id) || !seen.insert(id) {
            return Err("invalid scenario identity".into());
        }
        if let Some(base) = scenario.get("base").and_then(|v| v.as_str())
            && (base == id || !seen.contains(base))
        {
            return Err("invalid scenario base".into());
        }
    }
    Ok(manifest)
}

/// A checked private copy. Dropping it before publication removes the stage.
pub(crate) struct StagedPreparation {
    path: PathBuf,
    base: PathBuf,
    id: String,
    manifest: InputSetManifest,
    published: bool,
}

impl StagedPreparation {
    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    pub(crate) fn publish(mut self) -> Result<PathBuf, String> {
        let previous = current_input_set(
            self.base
                .parent()
                .and_then(Path::parent)
                .ok_or("invalid input cache")?,
        )?;
        let destination = self.base.join(&self.id);
        write_cache_json(&self.path.join("input-set.json"), &self.manifest)?;
        super::operation::finalize_cache_stage(&self.path, &destination)?;
        self.path = destination.clone();
        write_cache_json(
            &self.base.join("current.json"),
            &CurrentInputSet {
                id: self.id.clone(),
                saved_to: None,
            },
        )?;
        self.published = true;
        if let Some(previous) = previous
            && previous.id != self.id
        {
            let old = self.base.join(previous.id);
            if old.exists() {
                fs::remove_dir_all(old).map_err(|e| e.to_string())?;
            }
        }
        Ok(destination)
    }
}

impl Drop for StagedPreparation {
    fn drop(&mut self) {
        if !self.published {
            let _ = fs::remove_dir_all(&self.path);
        }
    }
}

/// Create and verify an isolated copy, leaving the current pointer untouched.
pub(crate) fn stage_preparation(
    workspace: &Path,
    prepared: &Path,
    id: &str,
    copy: bool,
    operation: &super::OperationManifest,
    manifest_identity: &(String, u64),
) -> Result<StagedPreparation, String> {
    if !valid_id(id) {
        return Err("invalid input-set identity".into());
    }
    let base = input_cache(workspace);
    fs::create_dir_all(&base).map_err(|e| e.to_string())?;
    let staged = base.join(format!(".{id}"));
    let engine = operation
        .inputs
        .iter()
        .find(|f| f.path == Path::new(super::PREPARED_ENGINE))
        .ok_or("preparation has no engine identity")?
        .clone();
    let stage_result = (|| -> Result<(), String> {
        if copy {
            super::copy_staged_tree(prepared, &staged)?;
        } else {
            fs::rename(prepared, &staged).map_err(|e| e.to_string())?;
        }
        if super::digest_file(&staged.join("manifest.json"))? != *manifest_identity {
            return Err("prepared manifest changed while freezing".into());
        }
        super::read_operation_manifest(&staged)?;
        if super::digest_file(&staged.join("manifest.json"))? != *manifest_identity {
            return Err("prepared manifest changed while freezing".into());
        }
        Ok(())
    })();
    if let Err(error) = stage_result {
        let _ = fs::remove_dir_all(&staged);
        return Err(error);
    }
    let manifest = InputSetManifest {
        version: 1,
        id: id.into(),
        created_at: operation.created_at.clone(),
        engine,
        recipe: operation.recipe.clone(),
        files: operation
            .inputs
            .iter()
            .filter(|f| f.path.starts_with("sources") || f.path.starts_with("baseline/Input"))
            .cloned()
            .collect(),
    };
    Ok(StagedPreparation {
        path: staged,
        base,
        id: id.into(),
        manifest,
        published: false,
    })
}

pub(crate) fn publish_captured_input_set(
    workspace: &Path,
    staged: &Path,
    id: &str,
) -> Result<PathBuf, String> {
    let manifest = read_input_set(staged)?;
    if manifest.id != id {
        return Err("input-set identity mismatch".into());
    }
    let base = input_cache(workspace);
    let previous = current_input_set(workspace)?;
    let destination = base.join(id);
    super::operation::finalize_cache_stage(staged, &destination)?;
    if let Err(error) = write_cache_json(
        &base.join("current.json"),
        &CurrentInputSet {
            id: id.into(),
            saved_to: None,
        },
    ) {
        let _ = fs::remove_dir_all(&destination);
        return Err(error);
    }
    if let Some(previous) = previous
        && previous.id != id
    {
        let old = base.join(previous.id);
        if old.exists() {
            fs::remove_dir_all(old).map_err(|e| e.to_string())?;
        }
    }
    Ok(destination)
}

pub fn save_input_set(root: &Path, destination: &Path) -> Result<InputSetManifest, String> {
    let manifest = read_input_set(root)?;
    if destination.starts_with(root) || root.starts_with(destination) {
        return Err("save inputs to an independent directory".into());
    }
    publish_directory(destination, |stage| {
        for file in &manifest.files {
            super::copy_member(root, &file.path, stage)?;
        }
        write_draft(&stage.join("input-set.json"), &manifest)?;
        read_input_set(stage)?;
        Ok(())
    })?;
    Ok(manifest)
}
pub(crate) fn save_current_inputs(
    workspace: &Path,
    destination: &Path,
) -> Result<InputSetManifest, String> {
    let workspace = workspace.canonicalize().map_err(|e| e.to_string())?;
    let destination = if destination.is_absolute() {
        destination.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|e| e.to_string())?
            .join(destination)
    };
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
    if ["cache", "assets", "runs/history"]
        .iter()
        .any(|managed| destination.starts_with(workspace.join(managed)))
    {
        return Err(
            "save inputs outside managed cache, acquisition assets and manifest archive".into(),
        );
    }
    let workspace = workspace.as_path();
    let current = current_input_set(workspace)?.ok_or("no current inputs")?;
    let root = input_cache(workspace).join(&current.id);
    let result = save_input_set(&root, &destination)?;
    write_cache_json(
        &input_cache(workspace).join("current.json"),
        &CurrentInputSet {
            id: current.id,
            saved_to: Some(destination.canonicalize().map_err(|e| e.to_string())?),
        },
    )?;
    Ok(result)
}
pub(crate) fn discard_current_inputs(workspace: &Path) -> Result<(), String> {
    let parent = workspace.join("cache");
    match fs::symlink_metadata(&parent) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
            return Err("invalid input cache parent".into());
        }
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.to_string()),
    }
    let root = input_cache(workspace);
    match fs::symlink_metadata(&root) {
        Ok(metadata) if metadata.is_dir() => fs::remove_dir_all(root).map_err(|e| e.to_string()),
        Ok(_) => fs::remove_file(root).map_err(|e| e.to_string()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.to_string()),
    }
}

pub(crate) fn clean_materialized_inputs(root: &Path) -> Result<(), String> {
    if !root.join("input-set.json").is_file() {
        return Err("missing frozen inputs".into());
    }
    for dir in ["forcing", "checkpoints"] {
        let path = root.join(dir);
        if path.exists() {
            fs::remove_dir_all(path).map_err(|e| e.to_string())?;
        }
    }
    for path in [
        root.join(super::PREPARED_ENGINE),
        root.join("manifest.json"),
    ] {
        if path.exists() {
            fs::remove_file(path).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::{OperationManifest, ScenarioRecord, collect_inputs, digest_file};

    #[test]
    #[cfg(unix)]
    fn discard_does_not_follow_the_input_cache_symlink() {
        fs::create_dir_all("runs").unwrap();
        let temporary = tempfile::tempdir_in("runs").unwrap();
        let workspace = temporary.path().join("workspace");
        let external = temporary.path().join("external");
        fs::create_dir_all(workspace.join("cache")).unwrap();
        fs::create_dir_all(&external).unwrap();
        let external = external.canonicalize().unwrap();
        fs::write(external.join("retained"), b"data").unwrap();
        std::os::unix::fs::symlink(&external, workspace.join("cache/inputs")).unwrap();
        discard_current_inputs(&workspace).unwrap();
        assert!(external.join("retained").is_file());
        discard_current_inputs(&workspace).unwrap();
        assert!(external.join("retained").is_file());
    }

    #[test]
    fn changed_preparation_never_replaces_current_inputs() {
        fs::create_dir_all("runs").unwrap();
        let temp = tempfile::tempdir_in("runs").unwrap();
        let workspace = temp.path();
        let prepared = workspace.join("prepared");
        fs::create_dir_all(prepared.join("baseline/Input")).unwrap();
        fs::create_dir_all(prepared.join("forcing")).unwrap();
        for path in [
            super::super::PREPARED_ENGINE,
            "baseline/Input/MINI.gtp",
            "baseline/Input/PARHIG.hig",
            "baseline/Input/Previsao.meta",
            "forcing/zero.nc",
        ] {
            fs::write(prepared.join(path), b"original").unwrap();
        }
        let operation = OperationManifest {
            schema_version: 1,
            preparation_sha256: None,
            state: "prepared".into(),
            recipe: serde_json::json!({"workspace_scenarios": []}),
            inputs: collect_inputs(&prepared).unwrap(),
            scenarios: vec![ScenarioRecord {
                id: "zero".into(),
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
            }],
            created_at: "now".into(),
            finished_at: None,
            error: None,
        };
        write_draft(&prepared.join("manifest.json"), &operation).unwrap();
        let identity = digest_file(&prepared.join("manifest.json")).unwrap();
        let base = input_cache(workspace);
        fs::create_dir_all(base.join("previous")).unwrap();
        write_draft(
            &base.join("current.json"),
            &CurrentInputSet {
                id: "previous".into(),
                saved_to: None,
            },
        )
        .unwrap();

        fs::write(prepared.join("forcing/zero.nc"), b"modified").unwrap();
        assert!(
            stage_preparation(workspace, &prepared, "next", true, &operation, &identity).is_err()
        );
        assert_eq!(
            current_input_set(workspace).unwrap().unwrap().id,
            "previous"
        );
        assert!(base.join("previous").exists());
        let no_destination = !base.join("next").exists();
        let no_stage = !base.join(".next").exists();
        assert!(no_destination);
        assert!(no_stage);

        fs::write(prepared.join("forcing/zero.nc"), b"original").unwrap();
        fs::write(prepared.join("manifest.json"), b"changed").unwrap();
        assert!(
            stage_preparation(workspace, &prepared, "next", true, &operation, &identity).is_err()
        );
        assert_eq!(
            current_input_set(workspace).unwrap().unwrap().id,
            "previous"
        );
        let no_stage = !base.join(".next").exists();
        assert!(no_stage);
    }
}
