use super::{
    FLOOD_SOURCE_DIAGNOSTIC, OperationManifest, read_draft, sync_directory, sync_tree, write_draft,
};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File, OpenOptions},
    io::{ErrorKind, Read, Seek, SeekFrom},
    path::{Path, PathBuf},
};

const JOURNAL: &str = "attempt.toml";
const RETAINED: &str = ".retained";

#[derive(Serialize, Deserialize)]
struct ActiveRun {
    id: String,
}

pub(crate) fn run_cache_root(workspace: &Path) -> PathBuf {
    workspace.join("results")
}

pub(crate) fn is_managed_run_path(workspace: &Path, resolved: &Path) -> bool {
    [workspace.join("results"), workspace.join("work")]
        .iter()
        .any(|root| resolved.starts_with(root.canonicalize().unwrap_or_else(|_| root.clone())))
}

fn valid_run_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 96
        && id.as_bytes()[0].is_ascii_alphanumeric()
        && id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
        && id != "."
        && id != ".."
}

pub(crate) fn active_run(workspace: &Path) -> Result<Option<PathBuf>, String> {
    let root = run_cache_root(workspace);
    let pointer = root.join("active.toml");
    if !pointer.try_exists().map_err(|error| error.to_string())? {
        return Ok(None);
    }
    let active: ActiveRun = read_draft(&pointer)?;
    if !valid_run_id(&active.id) {
        return Err("Invalid active run pointer".into());
    }
    Ok(Some(root.join(active.id)))
}

/// Reserve a dated identity under the workspace operation lock.
pub(crate) fn allocate_run_id(
    workspace: &Path,
    reference: chrono::NaiveDateTime,
    time_zone: &str,
) -> Result<String, String> {
    use chrono::{Offset, TimeZone};
    use std::io::Write;
    let zone: chrono_tz::Tz = time_zone
        .parse()
        .map_err(|e: chrono_tz::ParseError| e.to_string())?;
    let zoned = zone
        .from_local_datetime(&reference)
        .single()
        .ok_or("ambiguous/nonexistent operational reference")?;
    let directory = workspace.join("runs");
    fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
    let base = reference.format("%Y%m%d-%H").to_string();
    for index in 1u32.. {
        let id = if index == 1 {
            base.clone()
        } else {
            format!("{base}-{index:02}")
        };
        let path = directory.join(format!("{id}.toml"));
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(mut file) => {
                let mut initial = RunAttemptManifest::new(id.clone());
                initial.reference_time = Some(zoned.to_rfc3339());
                initial.time_zone = Some(zone.name().into());
                initial.utc_offset_seconds = Some(zoned.offset().fix().local_minus_utc());
                let document = super::draft::record_bytes(&path, &initial)?;
                file.write_all(&document)
                    .and_then(|()| file.sync_all())
                    .map_err(|e| e.to_string())?;
                sync_directory(&directory)?;
                return Ok(id);
            }
            Err(error) if error.kind() == ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error.to_string()),
        }
    }
    unreachable!()
}

#[cfg(test)]
pub(crate) fn new_run_stage(workspace: &Path) -> Result<PathBuf, String> {
    let id = allocate_run_id(workspace, chrono::Utc::now().naive_utc(), "UTC")?;
    new_run_stage_for(workspace, &id)
}

pub(crate) fn new_run_stage_for(workspace: &Path, id: &str) -> Result<PathBuf, String> {
    if !valid_run_id(id) {
        return Err("Invalid run ID".into());
    }
    let work = workspace.join("work");
    let results = run_cache_root(workspace);
    fs::create_dir_all(&work)
        .and_then(|()| fs::create_dir_all(&results))
        .map_err(|e| e.to_string())?;
    access_lock(&results, true)?;
    let path = work.join(id);
    fs::create_dir(&path).map_err(|e| e.to_string())?;
    sync_directory(&work)?;
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
        Err(error) if error.kind() == ErrorKind::NotFound && active_run(workspace)?.is_none() => {
            return Ok(None);
        }
        Err(error) => return Err(error.to_string()),
    };
    lock.lock_shared().map_err(|error| error.to_string())?;
    let Some(path) = active_run(workspace)? else {
        return Ok(None);
    };
    if !fs::symlink_metadata(&path).is_ok_and(|entry| entry.is_dir()) {
        return Err("Active run results are missing or invalid".into());
    }
    read(&path).map(Some)
}

#[cfg(test)]
pub(crate) fn publish_run(workspace: &Path, stage: &Path) -> Result<PathBuf, String> {
    if !stage.join(JOURNAL).exists() {
        let mut attempt = RunAttemptManifest::new(run_stage_id(stage)?.into());
        attempt.finish("completed", None);
        write_run_journal(stage, &attempt)?;
    }
    publish_run_checked(workspace, stage, |commit| commit())
}

/// Writers hold the workspace operation lock. Readers hold the results access lock.
pub(crate) fn publish_run_checked(
    workspace: &Path,
    stage: &Path,
    checked_commit: impl FnOnce(&dyn Fn() -> Result<(), String>) -> Result<(), String>,
) -> Result<PathBuf, String> {
    let root = run_cache_root(workspace);
    let id = run_stage_id(stage)?;
    if stage.parent() != Some(workspace.join("work").as_path())
        || !stage.join("manifest.toml").is_file()
    {
        return Err("Invalid staged run directory".into());
    }
    let attempt = read_run_attempt(&stage.join(JOURNAL))?;
    if attempt.attempt_id != id || attempt.state != "completed" {
        return Err("Published run journal does not match its results".into());
    }
    sync_tree(stage)?;
    let lock = access_lock(&root, true)?;
    lock.lock().map_err(|error| error.to_string())?;
    let destination = root.join(id);
    if destination.exists() {
        return Err("Run results already exist".into());
    }
    fs::rename(stage, &destination).map_err(|e| e.to_string())?;
    sync_directory(&root)?;
    sync_directory(&workspace.join("work"))?;
    if let Err(error) = checked_commit(&|| {
        write_draft(&root.join("active.toml"), &ActiveRun { id: id.into() })?;
        sync_directory(&root)
    }) {
        // The pointer is the commit boundary. Never move away a committed result.
        if active_run(workspace)?.as_deref() != Some(destination.as_path()) {
            let _ = fs::rename(&destination, stage);
        }
        return Err(error);
    }
    if sync_directory(&root).is_ok() {
        let _ = recover_locked(workspace, &root, Some(&destination));
    }
    Ok(destination)
}

pub(crate) fn run_stage_id(path: &Path) -> Result<&str, String> {
    path.file_name()
        .and_then(|name| name.to_str())
        .filter(|id| valid_run_id(id))
        .ok_or_else(|| "Invalid run identity".into())
}

/// Retain or release a successful result directory; manifests and inputs remain.
pub fn keep_run_results(workspace: &Path, id: &str, keep: bool) -> Result<(), String> {
    if !valid_run_id(id) {
        return Err("Invalid run ID".into());
    }
    let root = run_cache_root(workspace);
    let lock = access_lock(&root, false)?;
    lock.lock().map_err(|e| e.to_string())?;
    let path = root.join(id);
    if !fs::symlink_metadata(&path).is_ok_and(|m| m.is_dir())
        || !path.join("manifest.toml").is_file()
    {
        return Err("Run results do not exist".into());
    }
    let marker = path.join(RETAINED);
    if keep {
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(false)
            .open(marker)
            .map_err(|e| e.to_string())?;
        file.sync_all().map_err(|e| e.to_string())?;
    } else if marker.exists() {
        fs::remove_file(marker).map_err(|e| e.to_string())?;
    }
    sync_directory(&path)
}

pub fn retained_run_results(workspace: &Path) -> Result<Vec<PathBuf>, String> {
    let root = run_cache_root(workspace);
    if !root.exists() {
        return Ok(Vec::new());
    }
    let lock = access_lock(&root, false)?;
    lock.lock_shared().map_err(|e| e.to_string())?;
    let mut paths = Vec::new();
    for entry in fs::read_dir(root).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if entry.file_type().map_err(|e| e.to_string())?.is_dir() && path.join(RETAINED).is_file() {
            paths.push(path);
        }
    }
    paths.sort();
    Ok(paths)
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RunAttemptManifest {
    pub attempt_id: String,
    #[serde(default)]
    pub reference_time: Option<String>,
    #[serde(default)]
    pub time_zone: Option<String>,
    #[serde(default)]
    pub utc_offset_seconds: Option<i32>,
    pub state: String,
    pub operation: Option<OperationManifest>,
    pub created_at: String,
    pub finished_at: Option<String>,
    pub error: Option<String>,
    pub diagnostics: Vec<RunDiagnostic>,
}

impl RunAttemptManifest {
    pub(crate) fn new(attempt_id: String) -> Self {
        Self {
            attempt_id,
            reference_time: None,
            time_zone: None,
            utc_offset_seconds: None,
            state: "preparing".into(),
            operation: None,
            created_at: chrono::Utc::now().to_rfc3339(),
            finished_at: None,
            error: None,
            diagnostics: Vec::new(),
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
                    paths.push(prefix.join("execution.toml"));
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
    let directory = workspace.join("runs");
    fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
    let path = directory.join(format!("{}.toml", attempt.attempt_id));
    write_draft(&path, attempt)?;
    sync_directory(&directory)?;
    Ok(path)
}

pub(crate) fn read_run_attempt(path: &Path) -> Result<RunAttemptManifest, String> {
    let attempt: RunAttemptManifest = read_draft(path)?;
    if !valid_run_id(&attempt.attempt_id) {
        return Err("Invalid run attempt manifest".into());
    }
    Ok(attempt)
}

pub(crate) fn recover_run_cache(workspace: &Path) -> Result<(), String> {
    let root = run_cache_root(workspace);
    if !root.try_exists().map_err(|error| error.to_string())? {
        if !workspace.join("runs").is_dir() && !workspace.join("work").is_dir() {
            return Ok(());
        }
        fs::create_dir_all(&root).map_err(|error| error.to_string())?;
    }
    let lock = access_lock(&root, true)?;
    lock.lock().map_err(|error| error.to_string())?;
    let active = active_run(workspace)?;
    if active.as_ref().is_some_and(|path| {
        !fs::symlink_metadata(path).is_ok_and(|entry| entry.is_dir())
            || !path.join("manifest.toml").is_file()
    }) {
        return Err("Active run results are missing or invalid".into());
    }
    sync_directory(&root)?;
    recover_locked(workspace, &root, active.as_deref())
}

fn archive_interrupted(workspace: &Path, path: &Path) -> Result<(), String> {
    let journal = path.join(JOURNAL);
    if journal.try_exists().map_err(|error| error.to_string())? {
        let mut attempt = read_run_attempt(&journal)?;
        if attempt.attempt_id != run_stage_id(path)? {
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
            attempt.diagnostics = collect_run_diagnostics(path);
        }
        write_run_attempt(workspace, &attempt)?;
    }
    fs::remove_dir_all(path).map_err(|error| error.to_string())
}

fn recover_locked(workspace: &Path, root: &Path, active: Option<&Path>) -> Result<(), String> {
    if let Some(path) = active {
        let journal = path.join(JOURNAL);
        if journal.try_exists().map_err(|error| error.to_string())? {
            let attempt = read_run_attempt(&journal)?;
            if attempt.attempt_id != run_stage_id(path)? || attempt.state != "completed" {
                return Err("Published run journal does not match its results".into());
            }
            write_run_attempt(workspace, &attempt)?;
            fs::remove_file(journal).map_err(|error| error.to_string())?;
            sync_directory(path)?;
        }
    }
    let work = workspace.join("work");
    if work.is_dir() {
        for entry in fs::read_dir(&work).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            if entry.file_type().map_err(|e| e.to_string())?.is_dir()
                && run_stage_id(&entry.path()).is_ok()
            {
                archive_interrupted(workspace, &entry.path())?;
            } else if entry.file_type().map_err(|e| e.to_string())?.is_dir()
                && entry.file_name().to_string_lossy().starts_with(".inputs-")
            {
                fs::remove_dir_all(entry.path()).map_err(|e| e.to_string())?;
            }
        }
        sync_directory(&work)?;
    }
    for entry in fs::read_dir(root).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if !entry.file_type().map_err(|e| e.to_string())?.is_dir()
            || run_stage_id(&path).is_err()
            || Some(path.as_path()) == active
        {
            continue;
        }
        if path.join(JOURNAL).exists() {
            archive_interrupted(workspace, &path)?;
        } else if !path.join(RETAINED).is_file() {
            fs::remove_dir_all(path).map_err(|e| e.to_string())?;
        }
    }
    sync_directory(root)?;
    // A process can stop after reserving a run name but before creating its
    // working directory. Preserve that attempt instead of leaving it preparing.
    let runs = workspace.join("runs");
    if runs.is_dir() {
        for entry in fs::read_dir(&runs).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();
            if !entry.file_type().map_err(|e| e.to_string())?.is_file()
                || path.extension().is_none_or(|ext| ext != "toml")
            {
                continue;
            }
            let mut attempt = read_run_attempt(&path)?;
            if matches!(attempt.state.as_str(), "preparing" | "running") {
                attempt.finish(
                    "interrupted",
                    Some("Operation interrupted before publication".into()),
                );
                if let Some(operation) = &mut attempt.operation {
                    operation.state.clone_from(&attempt.state);
                    operation.finished_at.clone_from(&attempt.finished_at);
                    operation.error.clone_from(&attempt.error);
                }
                write_run_attempt(workspace, &attempt)?;
            }
        }
    }
    Ok(())
}

pub(crate) fn with_kept_run<T>(
    workspace: &Path,
    path: &Path,
    read: impl FnOnce(&Path) -> Result<T, String>,
) -> Result<T, String> {
    with_available_run(workspace, path, false, read)
}
pub(crate) fn with_available_run<T>(
    workspace: &Path,
    path: &Path,
    allow_active: bool,
    read: impl FnOnce(&Path) -> Result<T, String>,
) -> Result<T, String> {
    let root = run_cache_root(workspace)
        .canonicalize()
        .map_err(|e| e.to_string())?;
    let lock = access_lock(&root, false)?;
    lock.lock_shared().map_err(|e| e.to_string())?;
    let path = path.canonicalize().map_err(|e| e.to_string())?;
    if path.parent() != Some(root.as_path())
        || !fs::symlink_metadata(&path).is_ok_and(|m| m.is_dir())
    {
        return Err("invalid retained result path".into());
    }
    let kept = fs::symlink_metadata(path.join(RETAINED)).is_ok_and(|m| m.is_file());
    let active = allow_active
        && active_run(workspace)?.is_some_and(|p| p.canonicalize().is_ok_and(|p| p == path));
    if !kept && !active {
        return Err("run results are no longer retained or active".into());
    }
    read(&path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn publication_retention_and_interruption() {
        fs::create_dir_all("runs").unwrap();
        let fixture = tempfile::tempdir_in("runs").unwrap();
        let workspace = fixture.path();
        let first = new_run_stage_for(workspace, "first").unwrap();
        fs::write(first.join("manifest.toml"), b"state = 'completed'\n").unwrap();
        let first = publish_run(workspace, &first).unwrap();
        keep_run_results(workspace, "first", true).unwrap();
        let second = new_run_stage_for(workspace, "second").unwrap();
        fs::write(second.join("manifest.toml"), b"state = 'completed'\n").unwrap();
        let second = publish_run(workspace, &second).unwrap();
        assert_eq!(active_run(workspace).unwrap(), Some(second.clone()));
        let retained = vec![first.clone()];
        assert_eq!(retained_run_results(workspace).unwrap(), retained);
        let interrupted = new_run_stage_for(workspace, "third").unwrap();
        let mut attempt = RunAttemptManifest::new("third".into());
        attempt.finish("completed", None);
        write_run_journal(&interrupted, &attempt).unwrap();
        recover_run_cache(workspace).unwrap();
        assert_eq!(
            read_run_attempt(&workspace.join("runs/third.toml"))
                .unwrap()
                .state,
            "interrupted"
        );
        assert_eq!(active_run(workspace).unwrap(), Some(second));
        assert!(first.exists());
        keep_run_results(workspace, "first", false).unwrap();
        recover_run_cache(workspace).unwrap();
        let removed = !first.exists();
        assert!(removed);
    }
    #[test]
    fn cancellation_before_pointer_keeps_active_and_recovers_attempt() {
        fs::create_dir_all("runs").unwrap();
        let fixture = tempfile::tempdir_in("runs").unwrap();
        let workspace = fixture.path();
        let first = new_run_stage_for(workspace, "first").unwrap();
        fs::write(first.join("manifest.toml"), b"state = 'completed'\n").unwrap();
        let first = publish_run(workspace, &first).unwrap();
        let second = new_run_stage_for(workspace, "second").unwrap();
        fs::write(second.join("manifest.toml"), b"state = 'completed'\n").unwrap();
        let mut attempt = RunAttemptManifest::new("second".into());
        attempt.finish("completed", None);
        write_run_journal(&second, &attempt).unwrap();
        assert!(publish_run_checked(workspace, &second, |_| Err("cancelled".into())).is_err());
        assert_eq!(active_run(workspace).unwrap(), Some(first));
        assert!(second.is_dir());
        recover_run_cache(workspace).unwrap();
        assert_eq!(
            read_run_attempt(&workspace.join("runs/second.toml"))
                .unwrap()
                .state,
            "interrupted"
        );
    }

    #[test]
    fn dated_reservations_are_distinct_and_interrupted_reservations_remain() {
        fs::create_dir_all("runs").unwrap();
        let fixture = tempfile::tempdir_in("runs").unwrap();
        let reference = chrono::NaiveDate::from_ymd_opt(2026, 11, 27)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap();
        assert_eq!(
            allocate_run_id(fixture.path(), reference, "America/Sao_Paulo").unwrap(),
            "20261127-12"
        );
        assert_eq!(
            allocate_run_id(fixture.path(), reference, "America/Sao_Paulo").unwrap(),
            "20261127-12-02"
        );
        recover_run_cache(fixture.path()).unwrap();
        assert_eq!(
            read_run_attempt(&fixture.path().join("runs/20261127-12.toml"))
                .unwrap()
                .state,
            "interrupted"
        );
    }
}
