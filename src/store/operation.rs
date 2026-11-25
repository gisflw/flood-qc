use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, Metadata, OpenOptions},
    io::{Read, Write},
    path::{Component, Path, PathBuf},
};

pub(crate) const PREPARED_ENGINE: &str = if cfg!(windows) {
    "baseline/engine.exe"
} else {
    "baseline/engine"
};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InputReference {
    pub path: PathBuf,
    pub kind: String,
    pub size: u64,
    pub sha256: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HotStartManifest {
    pub schema_version: u32,
    pub saved_at: chrono::NaiveDateTime,
    pub time_zone: String,
    pub scenario: String,
    pub engine_sha256: String,
    pub geometry_sha256: String,
    pub soil_sha256: String,
    pub files: Vec<InputReference>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScenarioFailure {
    pub stage: String,
    pub message: String,
    pub log_paths: Vec<PathBuf>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScenarioRecord {
    pub id: String,
    pub state: String,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
    pub exit_code: Option<i32>,
    pub signal: Option<i32>,
    pub error: Option<ScenarioFailure>,
    pub stdout: Option<PathBuf>,
    pub stderr: Option<PathBuf>,
    pub basin_result: Option<PathBuf>,
    pub station_result: Option<PathBuf>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OperationManifest {
    pub schema_version: u32,
    #[serde(default)]
    pub preparation_sha256: Option<String>,
    pub state: String,
    pub recipe: serde_json::Value,
    pub inputs: Vec<InputReference>,
    pub scenarios: Vec<ScenarioRecord>,
    pub created_at: String,
    pub finished_at: Option<String>,
    pub error: Option<String>,
}

pub fn digest_file(path: &Path) -> Result<(String, u64), String> {
    let mut file = File::open(path).map_err(|e| e.to_string())?;
    let mut hasher = Sha256::new();
    let mut size = 0u64;
    let mut buffer = [0; 65536];
    loop {
        let count = file.read(&mut buffer).map_err(|e| e.to_string())?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
        size = size.checked_add(count as u64).ok_or("file size overflow")?;
    }
    Ok((format!("{:x}", hasher.finalize()), size))
}

fn valid_relative(path: &Path) -> bool {
    !path.as_os_str().is_empty()
        && path
            .components()
            .all(|part| matches!(part, Component::Normal(_)))
}

pub fn collect_inputs(root: &Path) -> Result<Vec<InputReference>, String> {
    fn visit(root: &Path, relative: &Path, output: &mut Vec<InputReference>) -> Result<(), String> {
        let mut entries = fs::read_dir(root.join(relative))
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        entries.sort_by_key(std::fs::DirEntry::file_name);
        for entry in entries {
            let path = relative.join(entry.file_name());
            let kind = entry.file_type().map_err(|e| e.to_string())?;
            if kind.is_symlink() {
                return Err(format!("symlink in input tree: {}", path.display()));
            }
            if kind.is_dir() {
                visit(root, &path, output)?;
            } else if kind.is_file() {
                let (sha256, size) = digest_file(&root.join(&path))?;
                output.push(InputReference {
                    kind: path
                        .components()
                        .next()
                        .ok_or("invalid path")?
                        .as_os_str()
                        .to_string_lossy()
                        .into_owned(),
                    path,
                    size,
                    sha256,
                });
            } else {
                return Err(format!("non-file in input tree: {}", path.display()));
            }
        }
        Ok(())
    }
    let mut references = Vec::new();
    visit(root, Path::new(""), &mut references)?;
    Ok(references)
}

pub fn verify_inputs(root: &Path, inputs: &[InputReference]) -> Result<(), String> {
    let mut paths = std::collections::HashSet::new();
    for input in inputs {
        if !paths.insert(&input.path) {
            return Err("invalid input reference".into());
        }
        let (path, _) = input_file(root, input)?;
        verify_input_contents(&path, input)?;
    }
    Ok(())
}

pub(super) fn input_file(
    root: &Path,
    input: &InputReference,
) -> Result<(PathBuf, Metadata), String> {
    if !valid_relative(&input.path)
        || input
            .path
            .components()
            .next()
            .is_none_or(|component| component.as_os_str() != input.kind.as_str())
        || input.sha256.len() != 64
        || !input
            .sha256
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err("invalid input reference".into());
    }
    let mut path = root.to_path_buf();
    let mut components = input.path.components().peekable();
    while let Some(component) = components.next() {
        path.push(component.as_os_str());
        let metadata = fs::symlink_metadata(&path).map_err(|e| e.to_string())?;
        if metadata.file_type().is_symlink() {
            return Err("symlink in input path".into());
        }
        if components.peek().is_none() && metadata.is_file() {
            return Ok((path, metadata));
        }
    }
    Err("input is not a regular file".into())
}

pub(super) fn verify_input_contents(path: &Path, input: &InputReference) -> Result<(), String> {
    let (hash, size) = digest_file(path)?;
    if hash != input.sha256 || size != input.size {
        return Err(format!(
            "input integrity mismatch: {}",
            input.path.display()
        ));
    }
    Ok(())
}

pub fn copy_tree(source: &Path, destination: &Path) -> Result<(), String> {
    copy_tree_inner(source, destination, true)
}

/// Copy into private staging. The caller discards the copy or syncs it before publication.
pub(crate) fn copy_staged_tree(source: &Path, destination: &Path) -> Result<(), String> {
    copy_tree_inner(source, destination, false)
}

fn copy_tree_inner(source: &Path, destination: &Path, sync: bool) -> Result<(), String> {
    let metadata = fs::symlink_metadata(source).map_err(|e| e.to_string())?;
    if metadata.file_type().is_symlink() {
        return Err(format!("symlink not permitted: {}", source.display()));
    }
    if metadata.is_dir() {
        fs::create_dir(destination).map_err(|e| e.to_string())?;
        for entry in fs::read_dir(source).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            copy_tree_inner(&entry.path(), &destination.join(entry.file_name()), sync)?;
        }
        if sync {
            sync_directory(destination)?;
        }
    } else if metadata.is_file() {
        let before = digest_file(source)?;
        let mut input = File::open(source).map_err(|e| e.to_string())?;
        let mut output = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(destination)
            .map_err(|e| e.to_string())?;
        std::io::copy(&mut input, &mut output).map_err(|e| e.to_string())?;
        output
            .set_permissions(metadata.permissions())
            .map_err(|e| e.to_string())?;
        if sync {
            output.sync_all().map_err(|e| e.to_string())?;
        }
        if digest_file(source)? != before || digest_file(destination)? != before {
            return Err(format!("input changed while copying: {}", source.display()));
        }
    } else {
        return Err(format!("invalid input type: {}", source.display()));
    }
    Ok(())
}

pub fn copy_working_inputs(source: &Path, destination: &Path) -> Result<(), String> {
    copy_tree(source, destination)?;
    make_working_input_writable(destination)
}

pub(crate) fn stage_working_inputs(source: &Path, destination: &Path) -> Result<(), String> {
    copy_staged_tree(source, destination)?;
    make_working_input_writable(destination)
}

fn make_working_input_writable(path: &Path) -> Result<(), String> {
    let metadata = fs::metadata(path).map_err(|e| e.to_string())?;
    let mut permissions = metadata.permissions();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        permissions.set_mode(permissions.mode() | 0o200);
    }
    #[cfg(not(unix))]
    permissions.set_readonly(false);
    fs::set_permissions(path, permissions).map_err(|e| e.to_string())?;
    if metadata.is_dir() {
        for entry in fs::read_dir(path).map_err(|e| e.to_string())? {
            make_working_input_writable(&entry.map_err(|e| e.to_string())?.path())?;
        }
    }
    Ok(())
}

pub fn write_run_manifest(root: &Path, manifest: &OperationManifest) -> Result<(), String> {
    let temporary = root.join("manifest.json.tmp");
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .map_err(|e| e.to_string())?;
    serde_json::to_writer_pretty(&mut file, manifest).map_err(|e| e.to_string())?;
    file.write_all(b"\n").map_err(|e| e.to_string())?;
    file.sync_all().map_err(|e| e.to_string())?;
    fs::rename(&temporary, root.join("manifest.json")).map_err(|e| e.to_string())?;
    sync_directory(root)
}

pub fn read_retained_manifest(root: &Path) -> Result<OperationManifest, String> {
    let file = File::open(root.join("manifest.json")).map_err(|e| e.to_string())?;
    let manifest: OperationManifest = serde_json::from_reader(file).map_err(|e| e.to_string())?;
    if manifest.state == "prepared" {
        return read_operation_manifest(root);
    }
    if manifest.schema_version != 1
        || !["completed", "failed"].contains(&manifest.state.as_str())
        || !manifest
            .scenarios
            .iter()
            .any(|scenario| scenario.state == "completed")
    {
        return Err("invalid retained run manifest".into());
    }
    Ok(manifest)
}

pub fn read_operation_manifest(root: &Path) -> Result<OperationManifest, String> {
    let file = File::open(root.join("manifest.json")).map_err(|e| e.to_string())?;
    let manifest: OperationManifest = serde_json::from_reader(file).map_err(|e| e.to_string())?;
    if manifest.schema_version != 1
        || manifest.state != "prepared"
        || manifest.scenarios.is_empty()
        || manifest
            .scenarios
            .iter()
            .any(|scenario| scenario.id.is_empty() || scenario.state != "pending")
    {
        return Err("invalid prepared manifest".into());
    }
    let mut scenario_ids = std::collections::HashSet::new();
    for scenario in &manifest.scenarios {
        let id = scenario.id.as_bytes();
        if scenario.started_at.is_some()
            || scenario.finished_at.is_some()
            || scenario.exit_code.is_some()
            || scenario.signal.is_some()
            || scenario.error.is_some()
            || scenario.stdout.is_some()
            || scenario.stderr.is_some()
            || scenario.basin_result.is_some()
            || scenario.station_result.is_some()
        {
            return Err("prepared scenario contains outcome fields".into());
        }
        if id.is_empty()
            || id.len() > 64
            || !id[0].is_ascii_alphanumeric()
            || !id
                .iter()
                .all(|byte| byte.is_ascii_alphanumeric() || *byte == b'_' || *byte == b'-')
            || !scenario_ids.insert(&scenario.id)
        {
            return Err("invalid or duplicate scenario ID".into());
        }
        let forcing = Path::new("forcing").join(format!("{}.nc", scenario.id));
        if !manifest.inputs.iter().any(|input| input.path == forcing) {
            return Err("missing scenario forcing reference".into());
        }
    }
    for required in [
        Path::new("request.toml"),
        Path::new(PREPARED_ENGINE),
        Path::new("baseline/Input/MINI.gtp"),
        Path::new("baseline/Input/PARHIG.hig"),
        Path::new("baseline/Input/Previsao.meta"),
    ] {
        if !manifest.inputs.iter().any(|input| input.path == required) {
            return Err(format!("missing input reference: {}", required.display()));
        }
    }
    verify_inputs(root, &manifest.inputs)?;
    let actual = collect_inputs(root)?;
    if actual.iter().any(|entry| {
        entry.path != Path::new("manifest.json")
            && !manifest.inputs.iter().any(|input| input.path == entry.path)
    }) {
        return Err("unreferenced file in prepared tree".into());
    }
    Ok(manifest)
}

pub fn sync_directory(path: &Path) -> Result<(), String> {
    #[cfg(unix)]
    File::open(path)
        .and_then(|file| file.sync_all())
        .map_err(|e| e.to_string())?;
    #[cfg(not(unix))]
    let _ = path;
    Ok(())
}

pub fn sync_tree(path: &Path) -> Result<(), String> {
    let metadata = fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    if metadata.file_type().is_symlink() {
        return Err("symlink in generated tree".into());
    }
    if metadata.is_dir() {
        for entry in fs::read_dir(path).map_err(|e| e.to_string())? {
            sync_tree(&entry.map_err(|e| e.to_string())?.path())?;
        }
        return sync_directory(path);
    } else if !metadata.is_file() {
        return Err("unexpected generated file type".into());
    }
    File::open(path)
        .and_then(|file| file.sync_all())
        .map_err(|e| e.to_string())
}

pub fn finalize_prepared(stage: &Path, destination: &Path) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        use rustix::fs::{CWD, RenameFlags, renameat_with};
        renameat_with(CWD, stage, CWD, destination, RenameFlags::NOREPLACE)
            .map_err(|e| e.to_string())?;
        let parent = destination.parent().ok_or("destination has no parent")?;
        sync_directory(parent)
    }
    #[cfg(windows)]
    {
        atomicwrites::move_atomic(stage, destination).map_err(|e| e.to_string())
    }
    #[cfg(not(any(target_os = "linux", windows)))]
    {
        let _ = (stage, destination);
        Err("atomic preparation finalization is not qualified on this platform".into())
    }
}

pub fn publish_directory<T>(
    destination: &Path,
    build: impl FnOnce(&Path) -> Result<T, String>,
) -> Result<T, String> {
    let name = destination.file_name().ok_or("destination has no name")?;
    let parent = destination
        .parent()
        .filter(|path| !path.as_os_str().is_empty());
    let parent = parent
        .unwrap_or(Path::new("."))
        .canonicalize()
        .map_err(|e| e.to_string())?;
    let destination = parent.join(name);
    if fs::symlink_metadata(&destination).is_ok() {
        return Err(format!(
            "destination already exists: {}",
            destination.display()
        ));
    }
    let stage = parent.join(format!(
        ".mgb-ops-{}-{}",
        std::process::id(),
        chrono::Utc::now().format("%Y%m%d%H%M%S%.9f")
    ));
    fs::create_dir(&stage).map_err(|e| e.to_string())?;
    let result = build(&stage).and_then(|value| {
        sync_tree(&stage)?;
        finalize_prepared(&stage, &destination)?;
        Ok(value)
    });
    if result.is_err()
        && let Err(error) = fs::remove_dir_all(&stage)
        && error.kind() != std::io::ErrorKind::NotFound
    {
        return Err(format!(
            "{}; staging cleanup failed: {error}",
            result.err().unwrap()
        ));
    }
    result
}

#[cfg(all(test, any(target_os = "linux", windows)))]
mod tests {
    use super::finalize_prepared;
    #[test]
    fn finalization_never_replaces_existing_directory() {
        std::fs::create_dir_all("runs").unwrap();
        let root = tempfile::tempdir_in("runs").unwrap();
        let stage = root.path().join("stage");
        let destination = root.path().join("destination");
        std::fs::create_dir(&stage).unwrap();
        std::fs::create_dir(&destination).unwrap();
        assert!(finalize_prepared(&stage, &destination).is_err());
        assert!(stage.is_dir());
        assert!(destination.is_dir());
    }
}
