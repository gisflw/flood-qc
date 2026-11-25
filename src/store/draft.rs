use serde::{Serialize, de::DeserializeOwned};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::Path,
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT: AtomicU64 = AtomicU64::new(0);

pub fn read_draft<T: DeserializeOwned>(path: &Path) -> Result<T, String> {
    let bytes = fs::read(path).map_err(|e| e.to_string())?;
    serde_json::from_slice(&bytes).map_err(|e| e.to_string())
}

pub fn write_draft<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    let bytes = serde_json::to_vec_pretty(value).map_err(|e| e.to_string())?;
    write_atomic(path, &[bytes.as_slice(), b"\n"].concat())
}

pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    write_file(path, bytes, true)
}

pub fn write_new_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    write_file(path, bytes, false)
}

fn write_file(path: &Path, bytes: &[u8], replace: bool) -> Result<(), String> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let name = path
        .file_name()
        .ok_or("destination needs a filename")?
        .to_string_lossy();
    let temporary = parent.join(format!(
        ".{name}.{}.{}.tmp",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .map_err(|e| e.to_string())?;
    let result = (|| {
        file.write_all(bytes)
            .and_then(|()| file.sync_all())
            .map_err(|e| e.to_string())?;
        drop(file);
        if replace {
            fs::rename(&temporary, path).map_err(|e| e.to_string())
        } else {
            fs::hard_link(&temporary, path).map_err(|e| e.to_string())?;
            fs::remove_file(&temporary).map_err(|e| e.to_string())?;
            super::sync_directory(parent)
        }
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}
