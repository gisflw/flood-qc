use super::{
    FLOOD_SOURCE_DIAGNOSTIC, InundationRecipe, OperationManifest, read_draft, sync_directory,
    sync_tree, write_draft,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs::{self, File, OpenOptions},
    io::{ErrorKind, Read, Seek, SeekFrom},
    path::{Path, PathBuf},
};

const GENERATION_PREFIX: &str = "generation-";
const JOURNAL: &str = "attempt.json";

pub(crate) fn run_cache_root(workspace: &Path) -> PathBuf {
    workspace.join("data/cache/run")
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ActiveRun {
    schema_version: u32,
    generation: String,
}

fn valid_run_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 96
        && id.as_bytes()[0].is_ascii_alphanumeric()
        && id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
}

pub(crate) fn active_run(workspace: &Path) -> Result<Option<PathBuf>, String> {
    let root = run_cache_root(workspace);
    let file = match File::open(root.join("active.json")) {
        Ok(file) => file,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.to_string()),
    };
    let pointer: ActiveRun = serde_json::from_reader(file).map_err(|error| error.to_string())?;
    if pointer.schema_version != 1
        || !pointer
            .generation
            .strip_prefix(GENERATION_PREFIX)
            .is_some_and(valid_run_id)
    {
        return Err("Invalid active run pointer".into());
    }
    Ok(Some(root.join(pointer.generation)))
}

pub(crate) fn new_run_stage(workspace: &Path) -> Result<PathBuf, String> {
    new_run_stage_for(
        workspace,
        &format!(
            "{}-{}",
            chrono::Utc::now().format("%Y%m%dT%H%M%S%.9f"),
            std::process::id()
        ),
    )
}

pub(crate) fn new_run_stage_for(workspace: &Path, id: &str) -> Result<PathBuf, String> {
    if !valid_run_id(id) {
        return Err("Invalid run ID".into());
    }
    let root = run_cache_root(workspace);
    fs::create_dir_all(&root).map_err(|error| error.to_string())?;
    for parent in root.ancestors().take(4).skip(1) {
        sync_directory(parent)?;
    }
    let path = root.join(format!("{GENERATION_PREFIX}{id}"));
    access_lock(&root, true)?;
    fs::create_dir(&path).map_err(|error| error.to_string())?;
    sync_directory(&root)?;
    Ok(path)
}

fn access_lock(root: &Path, create: bool) -> Result<File, String> {
    OpenOptions::new()
        .read(true)
        .write(create)
        .create(create)
        .truncate(false)
        .open(root.join("access.lock"))
        .map_err(|error| error.to_string())
}

pub(crate) fn with_active_run<T>(
    workspace: &Path,
    read: impl FnOnce(&Path) -> Result<T, String>,
) -> Result<Option<T>, String> {
    let root = run_cache_root(workspace);
    let lock = match File::open(root.join("access.lock")) {
        Ok(lock) => lock,
        Err(error) if error.kind() == ErrorKind::NotFound => {
            if active_run(workspace)?.is_none() {
                return Ok(None);
            }
            File::open(root.join("access.lock"))
                .map_err(|error| format!("Active run access lock: {error}"))?
        }
        Err(error) => return Err(error.to_string()),
    };
    lock.lock_shared().map_err(|error| error.to_string())?;
    let Some(path) = active_run(workspace)? else {
        return Ok(None);
    };
    if !fs::symlink_metadata(&path).is_ok_and(|entry| entry.is_dir()) {
        return Err("Active run generation is missing or invalid".into());
    }
    read(&path).map(Some)
}

/// Workspace writers hold the operation lock while staging, publishing and recovering.
pub(crate) fn publish_run(workspace: &Path, generation: &Path) -> Result<(), String> {
    publish_run_checked(workspace, generation, |commit| commit())
}

pub(crate) fn publish_run_checked(
    workspace: &Path,
    generation: &Path,
    checked_commit: impl FnOnce(&dyn Fn() -> Result<(), String>) -> Result<(), String>,
) -> Result<(), String> {
    let root = run_cache_root(workspace);
    let id = run_stage_id(generation)?;
    if generation.parent() != Some(root.as_path()) || !generation.join("manifest.json").is_file() {
        return Err("Invalid staged run directory".into());
    }
    sync_tree(generation)?;
    sync_directory(&root)?;
    let lock = access_lock(&root, true)?;
    lock.lock().map_err(|error| error.to_string())?;
    checked_commit(&|| {
        write_draft(
            &root.join("active.json"),
            &ActiveRun {
                schema_version: 1,
                generation: format!("{GENERATION_PREFIX}{id}"),
            },
        )
    })?;
    // Preserve the previous generation until the contents of the pointer are durable.
    if sync_directory(&root).is_ok() {
        let _ = recover_locked(workspace, &root, Some(generation));
    }
    Ok(())
}

pub(crate) fn run_stage_id(path: &Path) -> Result<&str, String> {
    path.file_name()
        .and_then(|name| name.to_str())
        .and_then(|name| name.strip_prefix(GENERATION_PREFIX))
        .filter(|id| valid_run_id(id))
        .ok_or_else(|| "Invalid run cache generation name".into())
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RunAttemptManifest {
    pub schema_version: u32,
    pub attempt_id: String,
    pub state: String,
    pub operation: Option<OperationManifest>,
    pub created_at: String,
    pub finished_at: Option<String>,
    pub error: Option<String>,
    pub diagnostics: Vec<RunDiagnostic>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub inundation: BTreeMap<String, InundationRecipe>,
}

impl RunAttemptManifest {
    pub(crate) fn new(attempt_id: String) -> Self {
        Self {
            schema_version: 1,
            attempt_id,
            state: "preparing".into(),
            operation: None,
            created_at: chrono::Utc::now().to_rfc3339(),
            finished_at: None,
            error: None,
            diagnostics: Vec::new(),
            inundation: BTreeMap::new(),
        }
    }

    pub(crate) fn finish(&mut self, state: &str, error: Option<String>) {
        self.state = state.into();
        self.finished_at = Some(chrono::Utc::now().to_rfc3339());
        self.error = error;
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RunDiagnostic {
    pub path: PathBuf,
    pub tail: String,
}

pub(crate) fn collect_run_diagnostics(root: &Path) -> Vec<RunDiagnostic> {
    let mut paths = Vec::new();
    let mut log_streams = Vec::new();
    if let Ok(scenarios) = fs::read_dir(root.join("scenarios")) {
        for entry in scenarios.flatten() {
            if entry.file_type().is_ok_and(|kind| kind.is_dir()) {
                let prefix = Path::new("scenarios").join(entry.file_name()).join("logs");
                if fs::symlink_metadata(root.join(&prefix)).is_ok_and(|entry| entry.is_dir()) {
                    paths.push(prefix.join("execution.json"));
                    for name in ["stderr.log", "stdout.log"] {
                        log_streams.push(prefix.join(name));
                    }
                }
            }
        }
    }
    if let Ok(results) = fs::read_dir(root.join("results")) {
        for entry in results.flatten() {
            if entry.file_type().is_ok_and(|kind| kind.is_dir()) {
                paths.push(
                    Path::new("results")
                        .join(entry.file_name())
                        .join(FLOOD_SOURCE_DIAGNOSTIC),
                );
            }
        }
    }
    paths.sort();
    log_streams.sort();
    paths.extend(log_streams);
    let mut remaining: usize = 64 * 1024;
    let mut diagnostics = Vec::new();
    for path in paths {
        let file = root.join(&path);
        if remaining == 0 || !fs::symlink_metadata(&file).is_ok_and(|entry| entry.is_file()) {
            continue;
        }
        let bytes = (|| -> std::io::Result<Vec<u8>> {
            let mut file = File::open(file)?;
            let count = remaining.min(4 * 1024) as u64;
            file.seek(SeekFrom::Start(
                file.metadata()?.len().saturating_sub(count),
            ))?;
            let mut bytes = Vec::new();
            file.take(count).read_to_end(&mut bytes)?;
            Ok(bytes)
        })();
        if let Ok(bytes) = bytes
            && !bytes.is_empty()
        {
            remaining -= bytes.len();
            diagnostics.push(RunDiagnostic {
                path,
                tail: String::from_utf8_lossy(&bytes).into_owned(),
            });
        }
    }
    diagnostics
}

pub(crate) fn write_run_journal(path: &Path, attempt: &RunAttemptManifest) -> Result<(), String> {
    write_draft(&path.join(JOURNAL), attempt)?;
    sync_directory(path)
}

pub(crate) fn write_run_attempt(
    workspace: &Path,
    attempt: &RunAttemptManifest,
) -> Result<PathBuf, String> {
    if !valid_run_id(&attempt.attempt_id) {
        return Err("Invalid run attempt ID".into());
    }
    let directory = workspace.join("data/run-manifests");
    fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
    sync_directory(&workspace.join("data"))?;
    let path = directory.join(format!("{}.json", attempt.attempt_id));
    write_draft(&path, attempt)?;
    sync_directory(&directory)?;
    Ok(path)
}

pub(crate) fn read_run_attempt(path: &Path) -> Result<RunAttemptManifest, String> {
    let attempt: RunAttemptManifest = read_draft(path)?;
    if attempt.schema_version != 1 || !valid_run_id(&attempt.attempt_id) {
        return Err("Invalid run attempt manifest".into());
    }
    Ok(attempt)
}

pub(crate) fn recover_run_cache(workspace: &Path) -> Result<(), String> {
    let root = run_cache_root(workspace);
    if !root.try_exists().map_err(|error| error.to_string())? {
        return Ok(());
    }
    let lock = access_lock(&root, true)?;
    lock.lock().map_err(|error| error.to_string())?;
    let active = active_run(workspace)?;
    if active.as_ref().is_some_and(|path| {
        !fs::symlink_metadata(path).is_ok_and(|entry| entry.is_dir())
            || !path.join("manifest.json").is_file()
    }) {
        return Err("Active run generation is missing or invalid".into());
    }
    sync_directory(&root)?;
    recover_locked(workspace, &root, active.as_deref())
}

fn recover_locked(workspace: &Path, root: &Path, active: Option<&Path>) -> Result<(), String> {
    if let Some(path) = active
        && path
            .join(JOURNAL)
            .try_exists()
            .map_err(|error| error.to_string())?
    {
        let journal = read_run_attempt(&path.join(JOURNAL))?;
        if journal.attempt_id != run_stage_id(path)? || journal.state != "completed" {
            return Err("Published run journal does not match its generation".into());
        }
        write_run_attempt(workspace, &journal)?;
        fs::remove_file(path.join(JOURNAL)).map_err(|error| error.to_string())?;
        sync_directory(path)?;
    }
    for entry in fs::read_dir(root).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let path = entry.path();
        if is_pointer_temporary(&path) && entry.file_type().map_err(|e| e.to_string())?.is_file() {
            fs::remove_file(&path).map_err(|error| error.to_string())?;
            continue;
        }
        let Ok(id) = run_stage_id(&path) else {
            continue;
        };
        if Some(path.as_path()) == active || !entry.file_type().map_err(|e| e.to_string())?.is_dir()
        {
            continue;
        }
        let journal = path.join(JOURNAL);
        if journal.try_exists().map_err(|error| error.to_string())? {
            let mut attempt = read_run_attempt(&journal)?;
            if attempt.attempt_id != id {
                return Err("Run journal ID does not match its directory".into());
            }
            if matches!(
                attempt.state.as_str(),
                "preparing" | "running" | "completed"
            ) {
                attempt.finish(
                    "interrupted",
                    Some("Operation interrupted before publication".into()),
                );
                if let Some(operation) = &mut attempt.operation {
                    operation.state.clone_from(&attempt.state);
                    operation.finished_at.clone_from(&attempt.finished_at);
                    operation.error.clone_from(&attempt.error);
                }
                attempt.diagnostics = collect_run_diagnostics(&path);
            }
            write_run_attempt(workspace, &attempt)?;
        }
        fs::remove_dir_all(&path).map_err(|error| error.to_string())?;
    }
    sync_directory(root)
}

fn is_pointer_temporary(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .and_then(|name| name.strip_prefix(".active.json."))
        .and_then(|name| name.strip_suffix(".tmp"))
        .and_then(|name| name.split_once('.'))
        .is_some_and(|(pid, nonce)| pid.parse::<u32>().is_ok() && nonce.parse::<u64>().is_ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recovery_archives_unpublished_work_and_the_committed_run() {
        fs::create_dir_all("runs").unwrap();
        let workspace = tempfile::tempdir_in("runs").unwrap();
        let workspace = workspace.path();
        let active = new_run_stage_for(workspace, "active").unwrap();
        let interrupted = new_run_stage_for(workspace, "interrupted").unwrap();
        for stage in [&active, &interrupted] {
            let mut attempt = RunAttemptManifest::new(run_stage_id(stage).unwrap().into());
            attempt.finish("completed", None);
            write_run_journal(stage, &attempt).unwrap();
            fs::write(stage.join("manifest.json"), "{}").unwrap();
        }
        // Simulate interruption after the pointer replacement and before archiving.
        write_draft(
            &run_cache_root(workspace).join("active.json"),
            &ActiveRun {
                schema_version: 1,
                generation: "generation-active".into(),
            },
        )
        .unwrap();

        let abandoned_pointer = run_cache_root(workspace).join(".active.json.1.1.tmp");
        fs::write(&abandoned_pointer, "incomplete pointer").unwrap();
        recover_run_cache(workspace).unwrap();
        recover_run_cache(workspace).unwrap();
        let pointer_clean = !abandoned_pointer.exists();
        assert!(pointer_clean);
        assert_eq!(
            active_run(workspace).unwrap().as_deref(),
            Some(active.as_path())
        );
        assert!(active.join("manifest.json").is_file());
        let active_journal_clean = !active.join(JOURNAL).exists();
        let interrupted_stage_clean = !interrupted.exists();
        assert!(active_journal_clean);
        assert!(interrupted_stage_clean);
        for (id, state) in [("active", "completed"), ("interrupted", "interrupted")] {
            let archive = workspace
                .join("data/run-manifests")
                .join(format!("{id}.json"));
            let record = read_run_attempt(&archive).unwrap();
            assert_eq!(record.state, state);
        }
    }
}
