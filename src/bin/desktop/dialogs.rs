use eframe::egui;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

#[derive(Clone, Default, Serialize, Deserialize)]
struct Locations(BTreeMap<String, PathBuf>);

fn memory_id() -> egui::Id {
    egui::Id::new("desktop-file-dialog-locations")
}
fn workspace_id() -> egui::Id {
    egui::Id::new("desktop-dialog-workspace")
}

pub fn workspace(context: &egui::Context, root: &Path) {
    context.data_mut(|data| data.insert_temp(workspace_id(), root.to_path_buf()));
}

fn existing_directory(path: &Path) -> Option<PathBuf> {
    path.ancestors()
        .find(|p| !p.as_os_str().is_empty() && p.is_dir())
        .map(Path::to_path_buf)
}

fn start(context: &egui::Context, purpose: &str, current: &Path) -> Option<PathBuf> {
    let (saved, workspace) = context.data_mut(|data| {
        (
            data.get_persisted::<Locations>(memory_id())
                .unwrap_or_default()
                .0
                .get(purpose)
                .cloned(),
            data.get_temp::<PathBuf>(workspace_id()),
        )
    });
    let current = if current.is_relative() && !current.as_os_str().is_empty() {
        workspace
            .as_ref()
            .map_or_else(|| current.to_path_buf(), |root| root.join(current))
    } else {
        current.to_path_buf()
    };
    saved
        .as_deref()
        .and_then(existing_directory)
        .or_else(|| existing_directory(&current))
        .or_else(|| workspace.as_deref().and_then(existing_directory))
}

fn remember(context: &egui::Context, purpose: &str, selected: &Path, directory: bool) {
    let path = if directory {
        Some(selected)
    } else {
        selected.parent()
    };
    if let Some(path) = path {
        context.data_mut(|data| {
            let mut locations = data
                .get_persisted::<Locations>(memory_id())
                .unwrap_or_default();
            locations.0.insert(purpose.into(), path.to_path_buf());
            data.insert_persisted(memory_id(), locations);
        });
    }
}

fn dialog(context: &egui::Context, purpose: &str, current: &Path) -> rfd::FileDialog {
    let dialog = rfd::FileDialog::new();
    start(context, purpose, current).map_or(dialog.clone(), |path| dialog.set_directory(path))
}

pub fn folder(context: &egui::Context, purpose: &str, current: &Path) -> Option<PathBuf> {
    let selected = dialog(context, purpose, current).pick_folder()?;
    remember(context, purpose, &selected, true);
    Some(selected)
}

pub fn file(
    context: &egui::Context,
    purpose: &str,
    current: &Path,
    label: &str,
    extensions: &[&str],
) -> Option<PathBuf> {
    let mut dialog = dialog(context, purpose, current);
    if !extensions.is_empty() {
        dialog = dialog.add_filter(label, extensions);
    }
    let selected = dialog.pick_file()?;
    remember(context, purpose, &selected, false);
    Some(selected)
}

pub fn save(context: &egui::Context, purpose: &str, current: &Path, name: &str) -> Option<PathBuf> {
    let selected = dialog(context, purpose, current)
        .set_file_name(name)
        .save_file()?;
    remember(context, purpose, &selected, false);
    Some(selected)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_locations_survive_restart_and_missing_paths_fall_back() {
        let root = tempfile::tempdir().unwrap();
        let gis = root.path().join("gis");
        let imports = root.path().join("imports");
        std::fs::create_dir_all(&gis).unwrap();
        std::fs::create_dir_all(&imports).unwrap();
        let context = egui::Context::default();
        remember(&context, "gis", &gis.join("map.gpkg"), false);
        remember(&context, "imports", &imports, true);
        let saved = context.memory(|memory| serde_json::to_string(memory).unwrap());
        let restarted = egui::Context::default();
        restarted.memory_mut(|memory| *memory = serde_json::from_str(&saved).unwrap());
        assert_eq!(start(&restarted, "gis", Path::new("")), Some(gis.clone()));
        assert_eq!(
            start(&restarted, "imports", Path::new("")),
            Some(imports.clone())
        );
        assert_eq!(
            start(&restarted, "gis", &imports.join("current.csv")),
            Some(gis.clone())
        );
        std::fs::remove_dir(&gis).unwrap();
        assert_eq!(
            start(&restarted, "gis", Path::new("")),
            Some(root.path().to_path_buf())
        );
    }
}
