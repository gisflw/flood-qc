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
        file.write_all(&super::draft::record_bytes(path, value)?)
            .map_err(|e| e.to_string())?;
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
    pub id: String,
    pub created_at: String,
    pub engine: InputReference,
    pub recipe: serde_json::Value,
    pub files: Vec<InputReference>,
    #[serde(default)]
    pub objects: Vec<RegisteredFile>,
    #[serde(default)]
    pub observation_snapshot: Option<InputReference>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegisteredFile {
    pub path: PathBuf,
    pub object: String,
    #[serde(default)]
    pub member: Option<PathBuf>,
}

fn object_filename(member: &InputReference) -> String {
    member
        .path
        .extension()
        .map(|e| format!("data.{}", e.to_string_lossy()))
        .unwrap_or_else(|| "data".into())
}
fn object_root(root: &Path) -> Result<PathBuf, String> {
    root.ancestors()
        .find(|p| p.join("assets/objects").is_dir())
        .map(|p| p.join("assets/objects"))
        .ok_or_else(|| "registered input objects are unavailable".into())
}
pub(crate) fn input_member(
    root: &Path,
    manifest: &InputSetManifest,
    member: &InputReference,
) -> Result<PathBuf, String> {
    let direct = root.join(&member.path);
    if direct.is_file() {
        return Ok(direct);
    }
    let object = manifest
        .objects
        .iter()
        .find(|o| o.path == member.path)
        .ok_or("missing registered input member")?;
    if !valid_id(&object.object) {
        return Err("invalid object identity".into());
    }
    let relative = object
        .member
        .clone()
        .unwrap_or_else(|| object_filename(member).into());
    if relative
        .components()
        .any(|c| !matches!(c, std::path::Component::Normal(_)))
    {
        return Err("invalid registered object member".into());
    }
    let base = object_root(root)?.join(&object.object);
    let mut path = base;
    for component in relative.components() {
        path.push(component);
        if fs::symlink_metadata(&path)
            .map_err(|e| e.to_string())?
            .file_type()
            .is_symlink()
        {
            return Err("symlink in registered object".into());
        }
    }
    Ok(path)
}
fn register_files(
    workspace: &Path,
    source: &Path,
    manifest: &mut InputSetManifest,
) -> Result<(), String> {
    let base = workspace.join("assets/objects");
    fs::create_dir_all(&base).map_err(|e| e.to_string())?;
    let mut objects = Vec::new();
    for file in &manifest.files {
        let src = input_member(source, manifest, file)?;
        if super::digest_file(&src)? != (file.sha256.clone(), file.size) {
            return Err("input changed before registration".into());
        }
        let canonical = if let Ok(relative) = file.path.strip_prefix("sources/forecasts") {
            Some(("forecasts".to_owned(), relative.to_path_buf()))
        } else if let Ok(relative) = file.path.strip_prefix("sources/imerg") {
            relative
                .file_stem()
                .and_then(|n| n.to_str())
                .map(|id| (id.to_owned(), PathBuf::from("grid.nc")))
        } else {
            None
        };
        if let Some((object, member)) = canonical {
            let mut member = member;
            if object == "forecasts"
                && !base.join(&object).join(&member).is_file()
                && let Some(parent) = member.parent()
            {
                let candidate = parent.join("native.nc");
                if base.join(&object).join(&candidate).is_file()
                    && super::digest_file(&base.join(&object).join(&candidate))?
                        == (file.sha256.clone(), file.size)
                {
                    member = candidate;
                }
            }
            let existing = base.join(&object).join(&member);
            if existing.is_file()
                && super::digest_file(&existing)? == (file.sha256.clone(), file.size)
            {
                objects.push(RegisteredFile {
                    path: file.path.clone(),
                    object,
                    member: Some(member),
                });
                continue;
            }
        }
        let label: String = file
            .path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() {
                    c.to_ascii_lowercase()
                } else {
                    '-'
                }
            })
            .take(28)
            .collect();
        let id = format!("input-{label}-{}", &file.sha256[..24]);
        let destination = base.join(&id);
        if !destination.exists() {
            super::publish_directory(&destination, |stage| {
                super::copy_staged_tree(&src, &stage.join(object_filename(file)))?;
                if super::digest_file(&stage.join(object_filename(file)))?
                    != (file.sha256.clone(), file.size)
                {
                    return Err("input changed while registering".into());
                }
                write_draft(&stage.join("object.toml"), file)
            })?;
        }
        let recorded: InputReference = read_draft(&destination.join("object.toml"))?;
        if recorded.sha256 != file.sha256
            || recorded.size != file.size
            || super::digest_file(&destination.join(object_filename(&recorded)))?
                != (file.sha256.clone(), file.size)
        {
            return Err("registered object failed integrity verification".into());
        }
        objects.push(RegisteredFile {
            path: file.path.clone(),
            object: id,
            member: Some(object_filename(&recorded).into()),
        });
    }
    manifest.objects = objects;
    Ok(())
}
pub(crate) fn register_imported_selection(
    workspace: &Path,
    source: &Path,
    mut manifest: InputSetManifest,
) -> Result<PathBuf, String> {
    register_files(workspace, source, &mut manifest)?;
    let base = input_cache(workspace);
    fs::create_dir_all(&base).map_err(|e| e.to_string())?;
    let destination = base.join(&manifest.id);
    super::publish_directory(&destination, |stage| {
        write_draft(&stage.join("input-set.toml"), &manifest)
    })?;
    read_input_set(&destination)?;
    write_draft(
        &base.join("current.toml"),
        &CurrentInputSet {
            id: manifest.id,
            saved_to: None,
        },
    )?;
    Ok(destination)
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
    workspace.join("assets/selections")
}
pub fn current_input_set(workspace: &Path) -> Result<Option<CurrentInputSet>, String> {
    let path = input_cache(workspace).join("current.toml");
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
    let manifest: InputSetManifest = read_draft(&root.join("input-set.toml"))?;
    if !valid_id(&manifest.id) {
        return Err("unsupported input set".into());
    }
    if manifest
        .files
        .iter()
        .any(|f| !(f.path.starts_with("sources") || f.path.starts_with("baseline/Input")))
    {
        return Err("input set contains a generated artifact or invalid path".into());
    }
    if manifest.files.is_empty() {
        return Err("empty input selection".into());
    }
    let mut seen = std::collections::HashSet::new();
    for member in &manifest.files {
        if !seen.insert(&member.path)
            || member
                .path
                .components()
                .any(|c| !matches!(c, std::path::Component::Normal(_)))
        {
            return Err("invalid or duplicate input member".into());
        }
        let path = input_member(root, &manifest, member)?;
        if super::digest_file(&path)? != (member.sha256.clone(), member.size) {
            return Err("input-set payload does not match its complete index".into());
        }
    }
    if let Some(snapshot) = &manifest.observation_snapshot
        && (snapshot.path != Path::new("sources/observations.sqlite")
            || super::digest_file(&root.join(&snapshot.path))?
                != (snapshot.sha256.clone(), snapshot.size))
    {
        return Err("invalid portable observation snapshot".into());
    }
    if root.join("sources").is_dir() || root.join("baseline/Input").is_dir() {
        let mut actual = super::operation::collect_selected_inputs(
            root,
            &[Path::new("baseline/Input"), Path::new("sources")],
        )?;
        if let Some(snapshot) = &manifest.observation_snapshot {
            actual.retain(|m| m.path != snapshot.path);
        }
        if actual != manifest.files {
            return Err("input-set payload has unexpected or missing members".into());
        }
    } else if manifest.objects.len() != manifest.files.len() {
        return Err("registration member index is incomplete".into());
    }
    if !manifest
        .files
        .iter()
        .any(|file| file.path == Path::new("sources/recipe.toml"))
    {
        return Err("missing indexed input-set recipe".into());
    }
    let recipe_member = manifest
        .files
        .iter()
        .find(|m| m.path == Path::new("sources/recipe.toml"))
        .ok_or("missing recipe")?;
    let recorded_recipe: serde_json::Value =
        read_draft(&input_member(root, &manifest, recipe_member)?)?;
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
    manifest: InputSetManifest,
    published: bool,
}

impl StagedPreparation {
    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    pub(crate) fn publish(mut self) -> Result<PathBuf, String> {
        let workspace = self
            .base
            .parent()
            .and_then(Path::parent)
            .ok_or("invalid selections root")?;
        register_imported_selection(workspace, &self.path, self.manifest.clone())?;
        write_cache_json(&self.path.join("input-set.toml"), &self.manifest)?;
        self.published = true;
        Ok(self.path.clone())
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
    let work = workspace.join("work");
    fs::create_dir_all(&work).map_err(|e| e.to_string())?;
    let staged = work.join(format!(".inputs-{id}"));
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
        if super::digest_file(&staged.join("manifest.toml"))? != *manifest_identity {
            return Err("prepared manifest changed while freezing".into());
        }
        super::read_operation_manifest(&staged)?;
        if super::digest_file(&staged.join("manifest.toml"))? != *manifest_identity {
            return Err("prepared manifest changed while freezing".into());
        }
        Ok(())
    })();
    if let Err(error) = stage_result {
        let _ = fs::remove_dir_all(&staged);
        return Err(error);
    }
    let manifest = InputSetManifest {
        id: id.into(),
        created_at: operation.created_at.clone(),
        engine,
        recipe: operation.recipe.clone(),
        objects: Vec::new(),
        observation_snapshot: None,
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
    let destination = register_imported_selection(workspace, staged, manifest)?;
    fs::remove_dir_all(staged).map_err(|e| e.to_string())?;
    Ok(destination)
}

/// Resolve shared immutable members into disposable execution storage.
pub(crate) fn materialize_input_set(
    root: &Path,
    destination: &Path,
) -> Result<InputSetManifest, String> {
    let manifest = read_input_set(root)?;
    publish_directory(destination, |stage| {
        for member in &manifest.files {
            let destination = stage.join(&member.path);
            fs::create_dir_all(destination.parent().ok_or("invalid input member")?)
                .map_err(|e| e.to_string())?;
            super::copy_staged_tree(&input_member(root, &manifest, member)?, &destination)?;
        }
        if let Some(snapshot) = &manifest.observation_snapshot {
            super::copy_staged_tree(&root.join(&snapshot.path), &stage.join(&snapshot.path))?;
        }
        let mut local = manifest.clone();
        local.objects.clear();
        write_draft(&stage.join("input-set.toml"), &local)?;
        read_input_set(stage)
    })
}

pub fn save_input_set(root: &Path, destination: &Path) -> Result<InputSetManifest, String> {
    let manifest = read_input_set(root)?;
    if destination.starts_with(root) || root.starts_with(destination) {
        return Err("save inputs to an independent directory".into());
    }
    publish_directory(destination, |stage| {
        let mut exported = manifest.clone();
        for file in &manifest.files {
            let src = input_member(root, &manifest, file)?;
            let destination = stage.join(&file.path);
            fs::create_dir_all(destination.parent().ok_or("invalid input member")?)
                .map_err(|e| e.to_string())?;
            super::copy_staged_tree(&src, &destination)?;
        }
        exported.objects.clear();
        if let Some(existing) = &manifest.observation_snapshot {
            super::copy_staged_tree(&root.join(&existing.path), &stage.join(&existing.path))?;
        } else if let Some(revision) = manifest.recipe["document"]["observed"]["revision"].as_i64()
            && !stage.join("sources/observations.sqlite").is_file()
        {
            let source = manifest.recipe["document"]["observed"]["path"]
                .as_str()
                .ok_or("missing observation registry path")?;
            let source = if Path::new(source).ends_with("assets/registry.sqlite") {
                root.ancestors()
                    .find(|p| p.join("assets/registry.sqlite").is_file())
                    .map(|p| p.join("assets/registry.sqlite"))
                    .unwrap_or_else(|| source.into())
            } else {
                source.into()
            };
            let store = super::ObservationStore::open_revision(&source, revision)
                .map_err(|e| e.to_string())?;
            if let Some(identity) = manifest.recipe["observation_registry_id"].as_str()
                && store.registry_id().map_err(|e| e.to_string())? != identity
            {
                return Err("observation registry identity differs from pinned inputs".into());
            }
            let target = stage.join("sources/observations.sqlite");
            let doc = &manifest.recipe["document"];
            let selected: Vec<super::Station> =
                serde_json::from_value(doc["selected_stations"].clone())
                    .map_err(|e| e.to_string())?;
            let selection = super::ObservationSelection {
                provider_codes: Vec::new(),
                station_ids: selected.iter().map(|s| s.station_id.clone()).collect(),
            };
            let start = doc["historical_first_end"]
                .as_str()
                .ok_or("missing historical start")?
                .parse::<chrono::NaiveDateTime>()
                .map_err(|e| e.to_string())?;
            let end = doc["historical_last_end"]
                .as_str()
                .ok_or("missing historical end")?
                .parse::<chrono::NaiveDateTime>()
                .map_err(|e| e.to_string())?;
            let mut at = start;
            let mut bounds = Vec::new();
            while at <= end {
                bounds.push(super::TimeInterval {
                    start: at - chrono::Duration::hours(1),
                    end: at,
                });
                at += chrono::Duration::hours(1);
            }
            let timeline = super::IntervalAxis {
                time_zone: doc["time_zone"].as_str().ok_or("missing timezone")?.into(),
                bounds,
            };
            let extra: Vec<String> = serde_json::from_value(doc["levels"]["station_ids"].clone())
                .map_err(|e| e.to_string())?;
            if selection.provider_codes.is_empty() && selection.station_ids.is_empty() {
                store.export_revision(&target)
            } else {
                store.snapshot_revision(&target, &selection, &timeline, &[], &extra)
            }
            .map_err(|e| e.to_string())?;
            let (sha256, size) = super::digest_file(&target)?;
            exported.observation_snapshot = Some(InputReference {
                path: "sources/observations.sqlite".into(),
                kind: "sources".into(),
                sha256,
                size,
            });
        }
        write_draft(&stage.join("input-set.toml"), &exported)?;
        read_input_set(stage)?;
        Ok(exported)
    })
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
    if ["cache", "assets", "runs", "work", "results"]
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
        &input_cache(workspace).join("current.toml"),
        &CurrentInputSet {
            id: current.id,
            saved_to: Some(destination.canonicalize().map_err(|e| e.to_string())?),
        },
    )?;
    Ok(result)
}
pub(crate) fn discard_current_inputs(workspace: &Path) -> Result<(), String> {
    let pointer = input_cache(workspace).join("current.toml");
    match fs::remove_file(pointer) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}

pub(crate) fn clean_materialized_inputs(root: &Path) -> Result<(), String> {
    if !root.join("input-set.toml").is_file() {
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
        root.join("manifest.toml"),
    ] {
        if path.exists() {
            fs::remove_file(path).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod registry_tests {
    use super::*;
    #[test]
    fn identical_baseline_members_with_different_extensions_share_one_object() {
        let temporary = tempfile::tempdir().unwrap();
        let workspace = temporary.path();
        let sources = workspace.join("work/source");
        fs::create_dir_all(sources.join("baseline/Input")).unwrap();
        fs::write(sources.join("baseline/Input/MINI.gtp"), b"same geometry").unwrap();
        fs::write(sources.join("baseline/Input/MINI.mgb"), b"same geometry").unwrap();
        let files = super::super::collect_inputs(&sources).unwrap();
        let mut manifest = InputSetManifest {
            id: "test".into(),
            created_at: "2026-01-01T00:00:00Z".into(),
            engine: files[0].clone(),
            recipe: serde_json::Value::Null,
            files,
            objects: Vec::new(),
            observation_snapshot: None,
        };
        register_files(workspace, &sources, &mut manifest).unwrap();
        assert_eq!(manifest.objects[0].object, manifest.objects[1].object);
        let selection = workspace.join("assets/selections/test");
        fs::create_dir_all(&selection).unwrap();
        for file in &manifest.files {
            assert_eq!(
                fs::read(input_member(&selection, &manifest, file).unwrap()).unwrap(),
                b"same geometry"
            );
        }
        assert_eq!(
            fs::read_dir(workspace.join("assets/objects"))
                .unwrap()
                .count(),
            1
        );
    }
}
