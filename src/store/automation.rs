use serde::{Serialize, de::DeserializeOwned};
use std::{
    fs::{self, File},
    path::{Path, PathBuf},
};

pub(crate) fn automation_documents<T: DeserializeOwned>(
    directory: &Path,
) -> Result<Vec<(PathBuf, T)>, String> {
    let entries = match fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(format!("{}: {e}", directory.display())),
    };
    let mut paths = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "toml") {
            if !entry.file_type().map_err(|e| e.to_string())?.is_file() {
                return Err(format!("invalid automation status file {}", path.display()));
            }
            paths.push(path);
        }
    }
    paths.sort();
    paths
        .into_iter()
        .map(|path| {
            let value = super::read_draft(&path).map_err(|e| format!("{}: {e}", path.display()))?;
            Ok((path, value))
        })
        .collect()
}

pub(crate) fn write_automation_document(
    path: &Path,
    document: &impl Serialize,
) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or("automation status needs a parent directory")?;
    if !parent.is_dir() {
        fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
        #[cfg(unix)]
        for directory in parent.ancestors().skip(1).take(3) {
            File::open(directory)
                .and_then(|file| file.sync_all())
                .map_err(|e| format!("{}: {e}", directory.display()))?;
        }
    }
    super::write_draft(path, document)?;
    #[cfg(unix)]
    File::open(parent)
        .and_then(|file| file.sync_all())
        .map_err(|e| format!("{}: {e}", parent.display()))?;
    Ok(())
}
