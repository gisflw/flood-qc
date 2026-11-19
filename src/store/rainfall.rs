use super::operation::{input_file, verify_input_contents};
use super::{InputReference, TimeInterval, collect_inputs, sync_directory, sync_tree, write_draft};
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use std::fs::{self, File, Metadata, OpenOptions};
use std::path::{Path, PathBuf};

fn root(workspace: &Path) -> PathBuf {
    workspace.join("data/cache/rainfall")
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RainfallCacheInfo {
    pub generation: String,
    pub reference: NaiveDateTime,
    pub time_zone: String,
    pub coverage: TimeInterval,
    pub scenarios: Vec<String>,
    pub(crate) zero_scenarios: Vec<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Active {
    schema_version: u32,
    info: RainfallCacheInfo,
    inputs: Vec<InputReference>,
}

fn safe_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

fn active(root: &Path) -> Result<Option<Active>, String> {
    let file = match File::open(root.join("active.json")) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.to_string()),
    };
    let value: Active = serde_json::from_reader(file).map_err(|error| error.to_string())?;
    if value.schema_version != 1
        || !value.info.generation.starts_with("generation-")
        || !safe_name(&value.info.generation)
        || value.info.time_zone.parse::<chrono_tz::Tz>().is_err()
        || value.info.coverage.start >= value.info.reference
        || value.info.coverage.end <= value.info.reference
        || value.info.scenarios.is_empty()
        || value.info.scenarios.iter().any(|name| !safe_name(name))
        || value
            .info
            .scenarios
            .iter()
            .collect::<std::collections::HashSet<_>>()
            .len()
            != value.info.scenarios.len()
        || value
            .info
            .zero_scenarios
            .iter()
            .any(|name| !value.info.scenarios.contains(name))
    {
        return Err("Invalid active rainfall cache metadata".into());
    }
    Ok(Some(value))
}

/// Read only the atomic pointer for a cheap refresh probe.
pub(crate) fn rainfall_cache_info(workspace: &Path) -> Result<Option<RainfallCacheInfo>, String> {
    let root = root(workspace);
    let Some(value) = active(&root)? else {
        return Ok(None);
    };
    if !root.join("access.lock").is_file() || !root.join(&value.info.generation).is_dir() {
        // The generation could have retired after the original pointer read.
        if active(&root)?.is_some_and(|current| current.info.generation == value.info.generation) {
            return Err("Active rainfall cache generation or access lock is missing".into());
        }
        return Ok(active(&root)?.map(|current| current.info));
    }
    Ok(Some(value.info))
}

/// Read a generation while its files are protected from retirement.
pub(crate) fn read_rainfall_cache<T>(
    workspace: &Path,
    read: impl FnOnce(&RainfallCacheInfo, &RainfallCacheFiles<'_>) -> Result<T, String>,
) -> Result<Option<T>, String> {
    let root = root(workspace);
    let _lock = match File::open(root.join("access.lock")) {
        Ok(file) => {
            file.lock_shared().map_err(|error| error.to_string())?;
            file
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return if root
                .join("active.json")
                .try_exists()
                .map_err(|e| e.to_string())?
            {
                Err("Active rainfall cache has no access lock".into())
            } else {
                Ok(None)
            };
        }
        Err(error) => return Err(error.to_string()),
    };
    let Some(active) = active(&root)? else {
        return Ok(None);
    };
    let generation = root.join(&active.info.generation);
    if !fs::symlink_metadata(&generation).is_ok_and(|entry| entry.is_dir()) {
        return Err("Active rainfall cache generation is missing or invalid".into());
    }
    read(
        &active.info,
        &RainfallCacheFiles {
            root: generation,
            inputs: &active.inputs,
        },
    )
    .map(Some)
}

pub(crate) struct RainfallCacheFiles<'a> {
    root: PathBuf,
    inputs: &'a [InputReference],
}

impl RainfallCacheFiles<'_> {
    pub(crate) fn decode<'a, T>(
        &self,
        relative: &Path,
        cached: &'a mut Option<DecodedRainfallFile<T>>,
        decode: impl FnOnce(&Path) -> Result<T, String>,
    ) -> Result<&'a T, String> {
        let mut entries = self.inputs.iter().filter(|entry| entry.path == relative);
        let entry = entries
            .next()
            .ok_or_else(|| format!("Missing rainfall cache member {}", relative.display()))?;
        if entries.next().is_some() {
            return Err(format!(
                "Duplicate rainfall cache member {}",
                relative.display()
            ));
        }
        let (path, metadata) = input_file(&self.root, entry)?;
        let version = FileVersion::read(&metadata);
        let previous = cached
            .take()
            .filter(|previous| previous.path == path && previous.input == *entry);
        let unchanged = previous
            .as_ref()
            .is_some_and(|previous| version.is_some() && previous.version == version);
        if !unchanged {
            verify_input_contents(&path, entry)?;
        }
        let value = match previous {
            Some(previous) => previous.value,
            None => decode(&path)?,
        };
        if !unchanged
            && version.is_some()
            && FileVersion::read(&input_file(&self.root, entry)?.1) != version
        {
            return Err(format!(
                "Rainfall cache member changed while reading {}",
                relative.display()
            ));
        }
        Ok(&cached
            .insert(DecodedRainfallFile {
                path,
                input: entry.clone(),
                version,
                value,
            })
            .value)
    }
}

pub(crate) struct DecodedRainfallFile<T> {
    path: PathBuf,
    input: InputReference,
    version: Option<FileVersion>,
    value: T,
}

#[derive(PartialEq, Eq)]
struct FileVersion {
    device: u64,
    inode: u64,
    size: u64,
    modified: (i64, i64),
    changed: (i64, i64),
}

impl FileVersion {
    #[cfg(unix)]
    fn read(metadata: &Metadata) -> Option<Self> {
        use std::os::unix::fs::MetadataExt;
        Some(Self {
            device: metadata.dev(),
            inode: metadata.ino(),
            size: metadata.len(),
            modified: (metadata.mtime(), metadata.mtime_nsec()),
            changed: (metadata.ctime(), metadata.ctime_nsec()),
        })
    }

    #[cfg(not(unix))]
    fn read(_metadata: &Metadata) -> Option<Self> {
        // With no inode/change-time fingerprint, verify the expected digest on reuse.
        None
    }
}

/// Holds an unlisted generation until the operation has finished successfully.
pub(crate) struct StagedRainfallCache {
    root: PathBuf,
    generation: PathBuf,
    active: Active,
    published: bool,
}

impl StagedRainfallCache {
    /// Workspace writers already hold the operational lock while staging and publishing.
    pub(crate) fn build(
        workspace: &Path,
        reference: NaiveDateTime,
        time_zone: String,
        coverage: TimeInterval,
        scenarios: Vec<String>,
        zero_scenarios: Vec<String>,
        build: impl FnOnce(&Path) -> Result<(), String>,
    ) -> Result<Self, String> {
        let root = root(workspace);
        fs::create_dir_all(&root).map_err(|error| error.to_string())?;
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(root.join("access.lock"))
            .map_err(|error| error.to_string())?;
        lock.lock().map_err(|error| error.to_string())?;
        // Keep the listed generation, and remove leftovers from interrupted updates.
        // An invalid pointer does not prevent an explicit rebuild.
        if let Ok(old) = active(&root) {
            retire(
                &root,
                old.as_ref().map(|active| active.info.generation.as_str()),
            )?;
        }
        drop(lock);

        let generation = format!(
            "generation-{}-{}",
            chrono::Utc::now().format("%Y%m%dT%H%M%S%9f"),
            std::process::id()
        );
        let path = root.join(&generation);
        fs::create_dir(&path).map_err(|error| error.to_string())?;
        let mut staged = Self {
            root,
            generation: path,
            active: Active {
                schema_version: 1,
                info: RainfallCacheInfo {
                    generation,
                    reference,
                    time_zone,
                    coverage,
                    scenarios,
                    zero_scenarios,
                },
                inputs: Vec::new(),
            },
            published: false,
        };
        build(&staged.generation)?;
        staged.active.inputs = collect_inputs(&staged.generation)?;
        sync_tree(&staged.generation)?;
        sync_directory(&staged.root)?;
        Ok(staged)
    }

    pub(crate) fn publish(mut self) -> Result<RainfallCacheInfo, String> {
        let _lock = OpenOptions::new()
            .read(true)
            .write(true)
            .open(self.root.join("access.lock"))
            .map_err(|error| error.to_string())?;
        _lock.lock().map_err(|error| error.to_string())?;
        write_draft(&self.root.join("active.json"), &self.active)?;
        self.published = true;
        // The pointer has committed. An interrupted retirement is retried before staging.
        if sync_directory(&self.root).is_ok() {
            let _ = retire(&self.root, Some(&self.active.info.generation));
        }
        Ok(self.active.info.clone())
    }
}

fn retire(root: &Path, keep: Option<&str>) -> Result<(), String> {
    for entry in fs::read_dir(root).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let name = entry.file_name();
        if name.to_str().is_some_and(|name| {
            name.starts_with("generation-") && safe_name(name) && Some(name) != keep
        }) && entry
            .file_type()
            .map_err(|error| error.to_string())?
            .is_dir()
        {
            fs::remove_dir_all(entry.path()).map_err(|error| error.to_string())?;
        }
    }
    Ok(())
}

impl Drop for StagedRainfallCache {
    fn drop(&mut self) {
        if !self.published {
            let _ = fs::remove_dir_all(&self.generation);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;
    use std::time::Duration;

    #[test]
    fn decoded_member_reuses_verified_values_and_detects_replaced_bytes() {
        fs::create_dir_all("runs").unwrap();
        let dir = tempfile::tempdir_in("runs").unwrap();
        let member = Path::new("member");
        let path = dir.path().join(member);
        fs::write(&path, "first").unwrap();
        let inputs = collect_inputs(dir.path()).unwrap();
        let files = RainfallCacheFiles {
            root: dir.path().to_owned(),
            inputs: &inputs,
        };
        let mut cached = None;
        let value = files
            .decode(member, &mut cached, |path| {
                fs::read_to_string(path).map_err(|e| e.to_string())
            })
            .unwrap();
        assert_eq!(value, "first");
        let value = files.decode(member, &mut cached, |_| panic!("decoded an unchanged file"));
        assert_eq!(value.unwrap(), "first");

        let modified = fs::metadata(&path).unwrap().modified().unwrap();
        let replacement = dir.path().join("replacement");
        fs::write(&replacement, "other").unwrap();
        File::options()
            .write(true)
            .open(&replacement)
            .unwrap()
            .set_modified(modified)
            .unwrap();
        fs::rename(replacement, path).unwrap();
        let error = files
            .decode(member, &mut cached, |_| panic!("decoded an invalid file"))
            .unwrap_err();
        assert!(error.contains("input integrity mismatch"));
        assert!(cached.is_none());
    }

    #[test]
    fn an_in_flight_reader_keeps_its_generation_during_publication() {
        fs::create_dir_all("runs").unwrap();
        let workspace = tempfile::tempdir_in("runs").unwrap();
        let reference = chrono::NaiveDate::from_ymd_opt(2026, 1, 1)
            .unwrap()
            .and_hms_opt(1, 0, 0)
            .unwrap();
        let hour = chrono::Duration::hours(1);
        let build = |text: &str| {
            StagedRainfallCache::build(
                workspace.path(),
                reference,
                "UTC".into(),
                TimeInterval {
                    start: reference - hour,
                    end: reference + hour,
                },
                vec!["zero".into()],
                vec!["zero".into()],
                |dir| fs::write(dir.join("member"), text).map_err(|e| e.to_string()),
            )
            .unwrap()
        };
        let first = build("first").publish().unwrap();
        let second = build("second");
        let (reading, reader_ready) = mpsc::channel();
        let (release, reader_release) = mpsc::channel();
        let (writing, writer_ready) = mpsc::channel();
        let (published, writer_done) = mpsc::channel();
        let workspace_path = workspace.path();
        let first_generation = first.generation.as_str();
        std::thread::scope(|scope| {
            scope.spawn(move || {
                let data = read_rainfall_cache(workspace_path, |info, files| {
                    assert_eq!(info.generation, first_generation);
                    files
                        .decode(Path::new("member"), &mut None, |path| {
                            reading.send(()).unwrap();
                            // The timeout lets the scoped reader finish if the writer assertion fails.
                            reader_release.recv_timeout(Duration::from_secs(2)).unwrap();
                            fs::read_to_string(path).map_err(|e| e.to_string())
                        })
                        .cloned()
                })
                .unwrap();
                assert_eq!(data.as_deref(), Some("first"));
            });
            reader_ready.recv().unwrap();
            scope.spawn(move || {
                writing.send(()).unwrap();
                published.send(second.publish().unwrap()).unwrap();
            });
            writer_ready.recv().unwrap();
            let still_reading = writer_done.recv_timeout(Duration::from_millis(50)).is_err();
            release.send(()).unwrap();
            assert!(still_reading);
            let second = writer_done.recv_timeout(Duration::from_secs(2)).unwrap();
            assert_eq!(rainfall_cache_info(workspace.path()).unwrap(), Some(second));
        });
        let first_retired = !root(workspace.path()).join(first.generation).exists();
        assert!(first_retired);
    }
}
