use std::path::{Path, PathBuf};

/// Find retained working rainfall grids without creating or modifying artifacts.
pub fn discover_rainfall_grids(workspace: &Path) -> Result<Vec<PathBuf>, String> {
    let mut grids = Vec::new();
    for base in [
        workspace.join("data/cache/preparations"),
        workspace.join("data/runs"),
    ] {
        let entries = match std::fs::read_dir(&base) {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(format!("{}: {error}", base.display())),
        };
        for entry in entries {
            let root = entry.map_err(|e| e.to_string())?.path();
            if !root.join("manifest.json").is_file() {
                continue;
            }
            let checkpoints = root.join("checkpoints");
            let observed = checkpoints.join("observed-grid.nc");
            if observed.is_file() {
                grids.push(observed);
            }
            let forecasts = checkpoints.join("forecast");
            match std::fs::read_dir(&forecasts) {
                Ok(entries) => {
                    for entry in entries {
                        let path = entry.map_err(|e| e.to_string())?.path().join("working.nc");
                        if path.is_file() {
                            grids.push(path);
                        }
                    }
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(format!("{}: {error}", forecasts.display())),
            }
        }
    }
    grids.sort();
    Ok(grids)
}
