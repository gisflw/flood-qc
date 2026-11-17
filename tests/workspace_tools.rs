use chrono::{Duration, NaiveDate};
use mgb_ops::{
    HourlyWindow, RainfallSource, WorkspaceConfig, WorkspaceEngine, WorkspaceSpatial,
    read_workspace_config, resolve_hourly_window, signed_rainfall_window, write_workspace_config,
};

#[test]
fn signed_rainfall_matches_explicit_windows() {
    let reference = NaiveDate::from_ymd_opt(2026, 1, 1)
        .unwrap()
        .and_hms_opt(12, 0, 0)
        .unwrap();
    for (hours, source, start, end) in [
        (
            -3,
            RainfallSource::Observed,
            reference - Duration::hours(3),
            reference,
        ),
        (
            3,
            RainfallSource::Forecast,
            reference,
            reference + Duration::hours(3),
        ),
    ] {
        let (actual_source, actual) = signed_rainfall_window("UTC", reference, hours).unwrap();
        assert_eq!(actual_source, source);
        assert_eq!(
            actual,
            resolve_hourly_window("UTC", &HourlyWindow::Explicit { start, end }).unwrap()
        );
    }
    assert!(signed_rainfall_window("UTC", reference, 0).is_err());
    assert!(signed_rainfall_window("bad zone", reference, 1).is_err());
    assert!(signed_rainfall_window("UTC", reference + Duration::minutes(1), 1).is_err());
}

#[test]
fn workspace_config_roundtrips_and_failed_save_preserves_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("workspace.toml");
    let config = WorkspaceConfig {
        spatial: Some(WorkspaceSpatial {
            gpkg_path: Some("data/region.gpkg".into()),
        }),
        engine: Some(WorkspaceEngine {
            executable: Some("engine".into()),
            input_dir: None,
        }),
    };
    write_workspace_config(&path, &config).unwrap();
    assert_eq!(read_workspace_config(&path).unwrap(), config);
    let bytes = std::fs::read(&path).unwrap();
    let invalid = WorkspaceConfig::default();
    assert!(write_workspace_config(&path, &invalid).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    std::fs::write(&path, "[spatial]\nunknown = 'x'\n").unwrap();
    assert!(read_workspace_config(&path).is_err());
}

#[test]
fn draft_uses_selected_workspace_config() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("config")).unwrap();
    std::fs::write(
        dir.path().join("config/workspace.toml"),
        "[engine]\nexecutable = 'wrong'\n",
    )
    .unwrap();
    let selected = WorkspaceConfig {
        spatial: None,
        engine: Some(WorkspaceEngine {
            executable: Some("selected".into()),
            input_dir: None,
        }),
    };
    let draft = mgb_ops::Draft::new_with_workspace_config(
        dir.path().to_path_buf(),
        dir.path(),
        Some(&selected),
    );
    let engine = draft.document.operation.engine.unwrap();
    assert_eq!(engine.executable.unwrap(), std::path::Path::new("selected"));
    assert_eq!(
        engine.input_dir.unwrap(),
        std::path::Path::new("mgb_runner/Input")
    );
}

#[test]
fn discovers_retained_rainfall_without_creating_caches() {
    let dir = tempfile::tempdir().unwrap();
    assert!(
        mgb_ops::discover_rainfall_grids(dir.path())
            .unwrap()
            .is_empty()
    );
    assert!(!dir.path().join("data").exists());
    let root = dir.path().join("data/cache/preparations/one");
    std::fs::create_dir_all(root.join("checkpoints/forecast/raw")).unwrap();
    std::fs::write(root.join("manifest.json"), "{}").unwrap();
    std::fs::write(root.join("checkpoints/observed-grid.nc"), "").unwrap();
    std::fs::write(root.join("checkpoints/forecast/raw/working.nc"), "").unwrap();
    assert_eq!(
        mgb_ops::discover_rainfall_grids(dir.path()).unwrap().len(),
        2
    );
}

#[test]
fn operation_toml_save_keeps_existing_bytes_when_draft_instructions_cannot_be_represented() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("operation.toml");
    std::fs::write(&path, "existing").unwrap();
    let mut draft = mgb_ops::Draft::new(dir.path().to_path_buf(), dir.path());
    draft
        .document
        .instructions
        .corrections
        .push(Some(Vec::new()));
    assert!(draft.save_preparation_toml(&path).is_err());
    assert_eq!(std::fs::read_to_string(path).unwrap(), "existing");
}
