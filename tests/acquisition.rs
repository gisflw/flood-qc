use std::fs;
use std::path::PathBuf;
use std::process::Command;

use mgb_ops::app::{AcquisitionRequest, OperationControl, import_ana_csv};
use mgb_ops::store::{
    IntervalAxis, ObservationSelection, ObservationStore, ObservationValue, TimeInterval,
};

#[test]
fn cli_acquisition_routes_live_failures_away_from_json() {
    fs::create_dir_all("runs").unwrap();
    let dir = tempfile::tempdir_in("runs").unwrap();
    let workspace = dir.path().canonicalize().unwrap();
    fs::write(
        workspace.join("config.toml"),
        "[run]\ntime_zone = \"UTC\"\n[acquisition]\nobservation_providers = [\"ana\"]\n",
    )
    .unwrap();
    let acquire = |json| {
        let mut command = Command::new(env!("CARGO_BIN_EXE_mgb-ops"));
        command
            .args(["observation", "acquire", "--workspace"])
            .arg(&workspace)
            .args(["--reference-time", "2026-07-20T16:00:00"])
            .env("TERM", "xterm");
        if json {
            command.arg("--json");
        }
        let result = command.output().unwrap();
        assert_eq!(result.status.code(), Some(1), "{result:?}");
        result
    };
    let check_progress = |progress| {
        let progress = String::from_utf8(progress).unwrap();
        assert!(progress.contains("Acquiring inputs for 2026-07-20 16:00:00\n"));
        assert!(
            progress.contains("ANA failed: Import the ANA station inventory before acquisition\n")
        );
        assert!(progress.ends_with("Acquisition finished with errors\n"));
        assert!(!progress.contains('\x1b'));
    };
    let text = acquire(false);
    check_progress(text.stdout);
    assert!(text.stderr.is_empty());
    let json = acquire(true);
    check_progress(json.stderr);
    let report: serde_json::Value = serde_json::from_slice(&json.stdout).unwrap();
    assert_eq!(
        report["ana"]["error"],
        "Import the ANA station inventory before acquisition"
    );
}

#[test]
fn legacy_ana_csv_bootstraps_and_revises_raw_samples_idempotently() {
    fs::create_dir_all("runs").unwrap();
    let dir = tempfile::tempdir_in("runs").unwrap();
    let workspace = dir.path().canonicalize().unwrap();
    let request = AcquisitionRequest {
        workspace: workspace.clone(),
        reference_time: Some("2026-07-20T16:00:00".into()),
    };
    fs::write(
        workspace.join("config.toml"),
        r#"[run]
time_zone = "UTC"
[acquisition]
observation_providers = ["ana"]
"#,
    )
    .unwrap();
    fs::write(workspace.join("inventory.csv"), "provider_code,station_code,station_name,latitude,longitude,observed_variables\nana,00123,Station,0,0,\"rain,level\"\ninmet,A01,Ignored,0,0,rain\n").unwrap();
    mgb_ops::app::import_workspace_inventory(&workspace, &workspace.join("inventory.csv")).unwrap();
    let csv = workspace.join("sample.csv");
    let input = [PathBuf::from("sample.csv")];
    let record = |rain: f64| {
        format!(
            "provider_code,station_code,observed_at,variable_code,value\nana,123,2026-07-20 12:30,rain,{rain}\nana,123,2026-07-20 13:00,level,100.5\n"
        )
    };
    fs::write(&csv, record(3.25)).unwrap();
    let import = || {
        let report = import_ana_csv(&request, &input, &OperationControl::default()).unwrap();
        assert!(report.is_complete(), "{report:?}");
        report.ana.unwrap().stations[0].changed_samples
    };
    assert_eq!(import(), 2);
    assert_eq!(import(), 0);
    fs::write(&csv, record(0.)).unwrap();
    assert_eq!(import(), 1);

    let store = ObservationStore::open_read_only(&workspace.join("operation.sqlite")).unwrap();
    let start = "2026-07-20T15:00:00".parse().unwrap();
    let end = "2026-07-20T16:00:00".parse().unwrap();
    let (inputs, _) = store
        .read_selection_with_stations(
            &ObservationSelection {
                provider_codes: vec!["ana".into()],
                ..Default::default()
            },
            &IntervalAxis {
                time_zone: "UTC".into(),
                bounds: vec![TimeInterval { start, end }],
            },
            &[],
            &[],
        )
        .unwrap();
    assert_eq!(inputs.stations.len(), 1);
    assert_eq!(inputs.stations[0].station_id, "ana:123");
    assert_eq!(inputs.samples.len(), 2);
    assert!(
        inputs
            .samples
            .iter()
            .any(|sample| sample.value == Some(ObservationValue::LevelCm(100)))
    );
    assert!(
        inputs
            .samples
            .iter()
            .any(|sample| sample.value == Some(ObservationValue::RainMm(0.)))
    );
}
