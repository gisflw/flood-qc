use super::{InputReference, collect_inputs, copy_tree};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::{Component, Path};

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct BundleManifest {
    pub version: u32,
    pub run_id: String,
    pub preparation_engine: Option<InputReference>,
    pub files: Vec<InputReference>,
}

fn portable_name(path: &Path) -> Result<String, String> {
    let mut components = Vec::new();
    for part in path.components() {
        let Component::Normal(part) = part else {
            return Err("bundle paths must be relative".into());
        };
        let part = part.to_str().ok_or("bundle paths must be UTF-8")?;
        if part.ends_with([' ', '.'])
            || part.contains(['\\', ':', '<', '>', '"', '|', '?', '*'])
            || part.chars().any(char::is_control)
        {
            return Err(format!("nonportable bundle path: {}", path.display()));
        }
        components.push(part);
    }
    if components.is_empty() {
        return Err("empty bundle path".into());
    }
    Ok(components.join("/"))
}

pub(crate) fn payload_index(root: &Path) -> Result<Vec<InputReference>, String> {
    if !fs::symlink_metadata(root).is_ok_and(|metadata| metadata.is_dir()) {
        return Err("expected a real bundle directory".into());
    }
    let mut files = collect_inputs(root)?;
    files.retain(|file| file.path != Path::new("bundle.json"));
    let mut paths = HashSet::new();
    for file in &mut files {
        let name = portable_name(&file.path)?;
        if !paths.insert(name.to_lowercase()) {
            return Err(format!("case-insensitive bundle path collision: {name}"));
        }
        file.path = name.into();
    }
    files.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(files)
}

pub(crate) fn validate_bundle(root: &Path) -> Result<BundleManifest, String> {
    let actual = payload_index(root)?;
    let bundle: BundleManifest = super::read_draft(&root.join("bundle.json"))?;
    if bundle.version != 1
        || bundle
            .run_id
            .as_bytes()
            .first()
            .is_none_or(|b| !b.is_ascii_alphanumeric())
        || bundle.run_id.ends_with('.')
        || !bundle
            .run_id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
    {
        return Err("unsupported bundle version or invalid run ID".into());
    }
    if bundle.files != actual {
        return Err("bundle file paths, lengths, or digests differ from its index".into());
    }
    Ok(bundle)
}

pub(crate) fn copy_member(root: &Path, relative: &Path, stage: &Path) -> Result<(), String> {
    portable_name(relative)?;
    let mut source = root.to_path_buf();
    for component in relative.components() {
        source.push(component);
        let metadata = fs::symlink_metadata(&source)
            .map_err(|error| format!("{}: {error}", source.display()))?;
        if metadata.file_type().is_symlink() {
            return Err(format!("symlink in bundle source: {}", source.display()));
        }
    }
    let destination = stage.join(relative);
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    copy_tree(&source, &destination)
}
