use std::fs;
use std::path::PathBuf;
use std::process::Command;

use mgb_ops::app::{AcquisitionRequest, OperationControl, import_ana_csv};
use mgb_ops::store::{
    IntervalAxis, ObservationSelection, ObservationStore, ObservationValue, TimeInterval,
};

#[test]
fn cli_acquisition_prints_plain_station_outcomes_and_separate_json() {
    fs::create_dir_all("runs").unwrap();
    let dir = tempfile::tempdir_in("runs").unwrap();
    let workspace = dir.path().canonicalize().unwrap();
    fs::write(
        workspace.join("acquisition.toml"),
        "schema_version = 1\ntime_zone = \"UTC\"\n[ana]\ninventory = \"inventory.csv\"\n",
    )
    .unwrap();
    fs::write(
        workspace.join("inventory.csv"),
        "provider_code,station_code,station_name,latitude,longitude,observed_variables\nana,00123,No requests,0,0,none\n",
    )
    .unwrap();
    let acquire = |json| {
        let mut command = Command::new(env!("CARGO_BIN_EXE_mgb-ops"));
        command
            .args(["observation", "acquire", "--workspace"])
            .arg(&workspace)
            .args([
                "--request",
                "acquisition.toml",
                "--reference-time",
                "2026-07-20T16:00:00",
            ])
            .env("TERM", "xterm");
        if json {
            command.arg("--json");
        }
        let result = command.output().unwrap();
        assert!(result.status.success(), "{result:?}");
        result
    };
    let check_progress = |progress| {
        let progress = String::from_utf8(progress).unwrap();
        assert!(progress.contains("Acquiring inputs for 2026-07-20 16:00:00\n"));
        assert_eq!(
            progress
                .matches("ANA 123 | no data | update - | rows 0")
                .count(),
            1
        );
        assert!(progress.ends_with("Acquisition complete\n"));
        assert!(!progress.contains('\x1b'));
    };
    let text = acquire(false);
    check_progress(text.stdout);
    assert!(text.stderr.is_empty());
    let json = acquire(true);
    check_progress(json.stderr);
    let report: serde_json::Value = serde_json::from_slice(&json.stdout).unwrap();
    assert_eq!(report["ana"]["stations"][0]["station_code"], "123");
    assert_eq!(report["ana"]["stations"][0]["normalized_rows"], 0);
}

#[test]
fn legacy_ana_csv_bootstraps_and_revises_raw_samples_idempotently() {
    fs::create_dir_all("runs").unwrap();
    let dir = tempfile::tempdir_in("runs").unwrap();
    let workspace = dir.path().canonicalize().unwrap();
    let request = AcquisitionRequest {
        workspace: workspace.clone(),
        request_file: "acquisition.toml".into(),
        reference_time: Some("2026-07-20T16:00:00".into()),
    };
    fs::write(
        workspace.join("acquisition.toml"),
        r#"schema_version = 1
time_zone = "UTC"
[ana]
inventory = "inventory.csv"
"#,
    )
    .unwrap();
    fs::write(workspace.join("inventory.csv"), "provider_code,station_code,station_name,latitude,longitude,observed_variables\nana,00123,Station,0,0,\"rain,level\"\ninmet,A01,Ignored,0,0,rain\n").unwrap();
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
