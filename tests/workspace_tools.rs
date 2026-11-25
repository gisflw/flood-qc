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
            ..Default::default()
        }),
        engine: Some(WorkspaceEngine {
            executable: Some("engine".into()),
            input_dir: None,
            ..Default::default()
        }),
        ..Default::default()
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
fn workspace_operation_resolves_config_and_runtime_scenario_choices_read_only() {
    std::fs::create_dir_all("runs").unwrap();
    let dir = tempfile::tempdir_in("runs").unwrap();
    let config = dir.path().join("config.toml");
    let contents = r#"
[run]
time_zone = "America/Sao_Paulo"
lookback_days = 10

[spatial]
bounds = [-59.0,-35.0,-48.0,-26.0]

[acquisition]
forecast_providers = ["ecmwf","noaa"]
"#;
    std::fs::write(&config, contents).unwrap();
    let now = chrono::DateTime::parse_from_rfc3339("2026-07-20T03:45:00Z")
        .unwrap()
        .to_utc();
    let mut operation =
        mgb_ops::app::OperationalRun::open_workspace(dir.path().into(), now).unwrap();
    assert_eq!(operation.reference(), "2026-07-20T00:00:00");
    assert_eq!(
        operation.coverage().unwrap(),
        [
            "2026-07-10T00:00:00",
            "2026-07-20T00:00:00",
            "2026-07-20T01:00:00",
            "2026-07-25T01:00:00",
        ]
    );
    use mgb_ops::app::RunScenario;
    assert_eq!(operation.scenarios().unwrap(), [RunScenario::Zero]);
    assert!(
        operation
            .set_scenarios(&["corrected:ecmwf".into()])
            .is_err()
    );
    let corrected = r#"{"kind":"corrected","forecast":"ecmwf@2026-07-20T00:00:00Z","corrections":[{"t0_hour":3,"t1_hour":9,"multiplication_factor":1.25}]}"#;
    operation
        .set_scenarios(&["zero".into(), corrected.into()])
        .unwrap();
    assert_eq!(
        operation.scenarios().unwrap(),
        [RunScenario::Zero, RunScenario::parse(corrected).unwrap()]
    );
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
    assert_eq!(std::fs::read_to_string(config).unwrap(), contents);
}

#[test]
fn workspace_edit_csv_order_and_rollback_are_shared_with_the_database() {
    std::fs::create_dir_all("runs").unwrap();
    let dir = tempfile::tempdir_in("runs").unwrap();
    let root = dir.path();
    let inventory = root.join("inventory.csv");
    std::fs::write(root.join("config.toml"), "[run]\ntime_zone='UTC'\n").unwrap();
    std::fs::write(&inventory,
        "provider_code,station_code,station_name,latitude,longitude,observed_variables\nana,123,Station,0,0,rain\n",
    ).unwrap();
    mgb_ops::app::import_workspace_inventory(root, &inventory).unwrap();
    let exported_inventory = root.join("exported-inventory.csv");
    assert_eq!(
        mgb_ops::app::export_workspace_inventory(root, &exported_inventory).unwrap(),
        1
    );
    assert_eq!(
        mgb_ops::app::import_workspace_inventory(root, &exported_inventory).unwrap(),
        1
    );
    let input = root.join("edits.csv");
    let output = root.join("export.csv");
    let header = "station_id,variable,kind,start_at,end_at,value,reason\n";
    let rows = "ana:123,rain,point,2026-07-20T00:00:00,,0,first\nana:123,rain,point,2026-07-20T00:00:00,,,second\n";
    std::fs::write(&input, format!("{header}{rows}")).unwrap();
    assert_eq!(
        mgb_ops::app::import_workspace_edits(root, &input).unwrap(),
        2
    );
    std::fs::write(
        &input,
        format!("{header}ana:missing,rain,point,2026-07-20T00:00:00,,0,invalid\n"),
    )
    .unwrap();
    assert!(mgb_ops::app::import_workspace_edits(root, &input).is_err());
    assert_eq!(
        mgb_ops::app::export_workspace_edits(root, &output).unwrap(),
        2
    );
    let records = csv::Reader::from_path(output)
        .unwrap()
        .records()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(&records[0][6], "first");
    assert_eq!(&records[1][6], "second");
    assert_eq!(&records[1][5], "");
}
