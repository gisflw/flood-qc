use super::operation::{input_file, verify_input_contents};
use super::{
    InputReference, TimeInterval, collect_inputs, read_draft, run_stage_id, sync_tree,
    with_active_run, write_draft,
};
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use std::fs::{self, Metadata};
use std::path::{Path, PathBuf};

const RAINFALL_MANIFEST: &str = "manifest.toml";

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
struct RainfallManifest {
    info: RainfallCacheInfo,
    inputs: Vec<InputReference>,
}

fn safe_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

fn validate(value: &RainfallManifest) -> Result<(), String> {
    if run_stage_id(Path::new(&value.info.generation)).is_err()
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
    Ok(())
}

fn read_manifest(generation: &Path) -> Result<Option<RainfallManifest>, String> {
    let root = generation.join("rainfall");
    match fs::symlink_metadata(&root) {
        Ok(metadata) if metadata.is_dir() => (),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        _ => return Err("Active rainfall cache directory is invalid".into()),
    }
    let manifest: RainfallManifest = read_draft(&root.join(RAINFALL_MANIFEST))?;
    validate(&manifest)?;
    if generation.file_name().and_then(|name| name.to_str()) != Some(&manifest.info.generation) {
        return Err("Active rainfall does not belong to the run generation".into());
    }
    Ok(Some(manifest))
}

pub(crate) fn rainfall_cache_info(workspace: &Path) -> Result<Option<RainfallCacheInfo>, String> {
    with_active_run(workspace, |generation| {
        Ok(read_manifest(generation)?.map(|manifest| manifest.info))
    })
    .map(Option::flatten)
}

/// Read rainfall while its run generation is protected from retirement.
pub(crate) fn read_rainfall_cache<T>(
    workspace: &Path,
    read: impl FnOnce(&RainfallCacheInfo, &RainfallCacheFiles<'_>) -> Result<T, String>,
) -> Result<Option<T>, String> {
    with_active_run(workspace, |generation| {
        let Some(manifest) = read_manifest(generation)? else {
            return Ok(None);
        };
        read(
            &manifest.info,
            &RainfallCacheFiles {
                root: generation.join("rainfall"),
                inputs: &manifest.inputs,
            },
        )
        .map(Some)
    })
    .map(Option::flatten)
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

/// Stage rainfall inside the unpublished run generation.
pub(crate) fn stage_rainfall(
    generation: &Path,
    reference: NaiveDateTime,
    time_zone: String,
    coverage: TimeInterval,
    scenarios: Vec<String>,
    zero_scenarios: Vec<String>,
    build: impl FnOnce(&Path) -> Result<(), String>,
) -> Result<RainfallCacheInfo, String> {
    let info = RainfallCacheInfo {
        generation: generation
            .file_name()
            .ok_or("Missing generation name")?
            .to_string_lossy()
            .into_owned(),
        reference,
        time_zone,
        coverage,
        scenarios,
        zero_scenarios,
    };
    let root = generation.join("rainfall");
    fs::create_dir(&root).map_err(|error| error.to_string())?;
    build(&root)?;
    let manifest = RainfallManifest {
        info,
        inputs: collect_inputs(&root)?,
    };
    validate(&manifest)?;
    write_draft(&root.join(RAINFALL_MANIFEST), &manifest)?;
    sync_tree(&root)?;
    Ok(manifest.info)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::{new_run_stage, publish_run};
    use std::fs::File;
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
            let run = new_run_stage(workspace.path()).unwrap();
            let manifest = super::super::OperationManifest {
                preparation_sha256: None,
                state: "completed".into(),
                recipe: serde_json::Value::Null,
                inputs: Vec::new(),
                created_at: reference.to_string(),
                finished_at: Some(reference.to_string()),
                error: None,
                scenarios: vec![super::super::ScenarioRecord {
                    id: "zero".into(),
                    state: "completed".into(),
                    started_at: None,
                    finished_at: None,
                    exit_code: Some(0),
                    signal: None,
                    error: None,
                    stdout: None,
                    stderr: None,
                    basin_result: None,
                    station_result: None,
                }],
            };
            write_draft(&run.join("manifest.toml"), &manifest).unwrap();
            let info = stage_rainfall(
                &run,
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
            .unwrap();
            (run, info)
        };
        let (first_run, first) = build("first");
        publish_run(workspace.path(), &first_run).unwrap();
        let (second_run, second) = build("second");
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
                publish_run(workspace_path, &second_run).unwrap();
                published.send(second).unwrap();
            });
            writer_ready.recv().unwrap();
            let still_reading = writer_done.recv_timeout(Duration::from_millis(50)).is_err();
            release.send(()).unwrap();
            assert!(still_reading);
            let second = writer_done.recv_timeout(Duration::from_secs(30)).unwrap();
            assert_eq!(rainfall_cache_info(workspace.path()).unwrap(), Some(second));
        });
        let first_retired = !first_run.exists();
        assert!(first_retired);
    }
}
