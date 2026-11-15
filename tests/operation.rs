use chrono::NaiveDateTime;
use mgb_ops::{
    ObservationBatch, ObservationSample, ObservationSeries, ObservationStore, ObservationValue,
    Station, StationOffset,
};
use std::{fs, path::Path, process::Command};
static TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
fn time(value: &str) -> NaiveDateTime {
    NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M:%S").unwrap()
}
fn fixture(root: &Path) {
    fs::create_dir_all(root.join("Input")).unwrap();
    fs::write(
        root.join("Input/MINI.gtp"),
        "CatID Mini Xcen Ycen\n1 1 0 0\n",
    )
    .unwrap();
    fs::write(
        root.join("Input/PARHIG.hig"),
        "DIA MES ANO HORA\n1 1 2020 0\nNT DT\n2 3600.\n",
    )
    .unwrap();
    fs::write(
        root.join("Input/Previsao.meta"),
        "!HORIZ(NT)\n2020 1 1 0 1\n!NUMERO DE MINI-BACIAS\n1\n",
    )
    .unwrap();
    fs::write(root.join("Input/control.txt"), "").unwrap();
    let helper = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/support/engine_helper.rs");
    assert!(
        Command::new("rustc")
            .args(["--edition=2024"])
            .arg(helper)
            .arg("-o")
            .arg(root.join("engine"))
            .status()
            .unwrap()
            .success()
    );
    let mut db = ObservationStore::create(&root.join("observations.sqlite"), "UTC").unwrap();
    let station = Station {
        station_id: "s".into(),
        provider_code: "ana".into(),
        station_code: "1".into(),
        name: "station".into(),
        latitude: Some(0.0),
        longitude: Some(0.0),
    };
    let series = ObservationSeries {
        series_id: "r".into(),
        station_id: "s".into(),
        variable: "rain".into(),
        state: "raw".into(),
        unit: "mm".into(),
        interval_seconds: Some(3600),
        created_at: time("2026-01-01T00:00:00"),
    };
    let samples = ["2026-01-01T01:00:00", "2026-01-01T02:00:00"]
        .into_iter()
        .map(|at| ObservationSample {
            series_id: "r".into(),
            observed_at: time(at),
            value: Some(ObservationValue::RainMm(1.0)),
        })
        .collect();
    db.write_batch(&ObservationBatch {
        stations: vec![station],
        series: vec![series],
        samples,
        replacements: vec![],
        offsets: vec![StationOffset {
            station_id: "s".into(),
            mini_id: 1,
            offset_m: 0.609,
        }],
    })
    .unwrap();
    fs::write(
        root.join("request.toml"),
        r#"schema_version = 1
time_zone = "UTC"
reference_time = "2026-01-01T02:00:00"
step_seconds = 3600
historical_first_end = "2026-01-01T01:00:00"
historical_last_end = "2026-01-01T02:00:00"
forecast_first_end = "2026-01-01T03:00:00"
forecast_last_end = "2026-01-01T03:00:00"
[observed]
path = "observations.sqlite"
provider_codes = ["ana"]
station_ids = []
series_ids = []
replacement_ids = []
[geometry]
mini_gtp = "Input/MINI.gtp"
[grid]
bbox = [-1.0,-1.0,1.0,1.0]
resolution_degrees = 1.0
[interpolation]
nearest_stations = 1
power = 2.0
[qc]
high_threshold_mm = 50.0
sequence_lower_mm = 0.0
sequence_upper_mm = 1.0
sequence_min_length = 5
[levels]
station_ids = ["s"]
[engine]
executable = "engine"
input_dir = "Input"
omp_threads = 1
[forecasts]
catalog = "unused.sqlite"
[[scenarios]]
id = "zero"
kind = "zero"
[[scenarios]]
id = "other"
kind = "zero"
[[scenarios]]
id = "third"
kind = "zero"
[[scenarios]]
id = "fourth"
kind = "zero"
"#,
    )
    .unwrap();
}
#[test]
fn prepared_snapshot_is_reusable_and_scenarios_are_isolated() {
    let _guard = TEST_LOCK.lock().unwrap();
    fs::create_dir_all("runs").unwrap();
    let dir = tempfile::tempdir_in("runs").unwrap();
    let path = dir.path().canonicalize().unwrap();
    fixture(&path);
    let request = mgb_ops::app::PreparationRequest {
        workspace: path.clone(),
        invocation_dir: path.clone(),
        request_file: "request.toml".into(),
        output: "prepared".into(),
    };
    let prepared = mgb_ops::app::prepare(request).unwrap();
    assert_eq!(prepared.scenario_ids, ["zero", "other", "third", "fourth"]);
    fs::remove_dir_all(path.join("Input")).unwrap();
    fs::remove_file(path.join("engine")).unwrap();
    fs::remove_file(path.join("observations.sqlite")).unwrap();
    let execute = || {
        mgb_ops::app::execute_prepared(mgb_ops::app::ExecutionRequest {
            workspace: path.clone(),
            invocation_dir: path.clone(),
            prepared: "prepared".into(),
        })
        .unwrap()
    };
    let one = execute();
    let two = execute();
    assert!(matches!(one.status, mgb_ops::app::BatchStatus::Completed));
    assert!(matches!(two.status, mgb_ops::app::BatchStatus::Completed));
    assert_ne!(one.run_root, two.run_root);
    let stations =
        mgb_ops::store::read_station_results(&one.run_root.join("results/zero/station.nc"))
            .unwrap();
    assert_eq!(stations.level_cm, [40, 40, 40]);
    assert_eq!(stations.level_valid, [true, true, true]);
}

#[test]
fn failed_scenario_preserves_sibling_and_lock_rejects_overlap() {
    let _guard = TEST_LOCK.lock().unwrap();
    use std::fs::OpenOptions;
    fs::create_dir_all("runs").unwrap();
    let dir = tempfile::tempdir_in("runs").unwrap();
    let path = dir.path().canonicalize().unwrap();
    fixture(&path);
    fs::write(path.join("Input/control.txt"), "fail:other\n").unwrap();
    mgb_ops::app::prepare(mgb_ops::app::PreparationRequest {
        workspace: path.clone(),
        invocation_dir: path.clone(),
        request_file: "request.toml".into(),
        output: "prepared".into(),
    })
    .unwrap();
    let lock = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(path.join(".operation.lock"))
        .unwrap();
    lock.try_lock().unwrap();
    let run = || {
        mgb_ops::app::execute_prepared(mgb_ops::app::ExecutionRequest {
            workspace: path.clone(),
            invocation_dir: path.clone(),
            prepared: "prepared".into(),
        })
    };
    assert!(matches!(
        run(),
        Err(mgb_ops::app::OperationError::Busy { .. })
    ));
    drop(lock);
    let report = run().unwrap();
    let cli_prepare = Command::new(env!("CARGO_BIN_EXE_mgb-ops"))
        .current_dir("/")
        .args(["operation", "prepare", "--workspace"])
        .arg(&path)
        .arg("--request")
        .arg(path.join("request.toml"))
        .arg("--output")
        .arg(path.join("prepared-cli"))
        .arg("--json")
        .output()
        .unwrap();
    assert!(
        cli_prepare.status.success(),
        "{}",
        String::from_utf8_lossy(&cli_prepare.stderr)
    );
    let cli_prepared: serde_json::Value = serde_json::from_slice(&cli_prepare.stdout).unwrap();
    assert_eq!(cli_prepared["scenario_ids"].as_array().unwrap().len(), 4);
    assert_eq!(
        mgb_ops::store::read_basin_forcing(&path.join("prepared/forcing/zero.nc")).unwrap(),
        mgb_ops::store::read_basin_forcing(&path.join("prepared-cli/forcing/zero.nc")).unwrap()
    );
    let cli_run = Command::new(env!("CARGO_BIN_EXE_mgb-ops"))
        .current_dir("/")
        .args(["operation", "execute", "--workspace"])
        .arg(&path)
        .arg("--prepared")
        .arg(path.join("prepared-cli"))
        .arg("--json")
        .output()
        .unwrap();
    assert_eq!(cli_run.status.code(), Some(1));
    let cli_result: serde_json::Value = serde_json::from_slice(&cli_run.stdout).unwrap();
    assert_eq!(cli_result["status"], "failed");
    let missing = Command::new(env!("CARGO_BIN_EXE_mgb-ops"))
        .current_dir("/")
        .args(["operation", "execute", "--workspace"])
        .arg(&path)
        .arg("--prepared")
        .arg(path.join("missing"))
        .arg("--json")
        .output()
        .unwrap();
    assert_eq!(missing.status.code(), Some(1));
    let failure: serde_json::Value = serde_json::from_slice(&missing.stdout).unwrap();
    assert_eq!(failure["kind"], "invalid_prepared");
    let usage = Command::new(env!("CARGO_BIN_EXE_mgb-ops"))
        .args(["operation", "execute", "--unknown"])
        .output()
        .unwrap();
    assert_eq!(usage.status.code(), Some(2));
    assert!(matches!(report.status, mgb_ops::app::BatchStatus::Failed));
    assert!(report.run_root.join("results/zero/basin.nc").is_file());
    assert!(!report.run_root.join("results/other/basin.nc").exists());
    assert_eq!(report.scenarios[1].exit_code, Some(3));
    let manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(report.run_artifact).unwrap()).unwrap();
    assert_eq!(manifest["state"], "failed");
    let empty_request = fs::read_to_string(path.join("request.toml"))
        .unwrap()
        .replace("station_ids = [\"s\"]", "station_ids = []");
    fs::write(path.join("empty.toml"), empty_request).unwrap();
    mgb_ops::app::prepare(mgb_ops::app::PreparationRequest {
        workspace: path.clone(),
        invocation_dir: path.clone(),
        request_file: "empty.toml".into(),
        output: "prepared-empty".into(),
    })
    .unwrap();
    let empty = mgb_ops::app::execute_prepared(mgb_ops::app::ExecutionRequest {
        workspace: path.clone(),
        invocation_dir: path.clone(),
        prepared: "prepared-empty".into(),
    })
    .unwrap();
    let station =
        mgb_ops::store::read_station_results(&empty.run_root.join("results/zero/station.nc"))
            .unwrap();
    assert!(station.station_ids.is_empty());
    fs::write(path.join("prepared/request.toml"), "tampered").unwrap();
    assert!(matches!(
        run(),
        Err(mgb_ops::app::OperationError::InvalidPrepared { .. })
    ));
}

#[test]
fn malformed_output_and_large_logs_leave_siblings_complete() {
    let _guard = TEST_LOCK.lock().unwrap();
    fs::create_dir_all("runs").unwrap();
    let dir = tempfile::tempdir_in("runs").unwrap();
    let path = dir.path().canonicalize().unwrap();
    fixture(&path);
    fs::write(
        path.join("Input/control.txt"),
        "malformed:other\nlarge:third\n",
    )
    .unwrap();
    mgb_ops::app::prepare(mgb_ops::app::PreparationRequest {
        workspace: path.clone(),
        invocation_dir: path.clone(),
        request_file: "request.toml".into(),
        output: "prepared".into(),
    })
    .unwrap();
    let report = mgb_ops::app::execute_prepared(mgb_ops::app::ExecutionRequest {
        workspace: path.clone(),
        invocation_dir: path.clone(),
        prepared: "prepared".into(),
    })
    .unwrap();
    assert!(matches!(report.status, mgb_ops::app::BatchStatus::Failed));
    assert!(
        report.scenarios[1]
            .error
            .as_ref()
            .unwrap()
            .message
            .contains("incorrect stream length")
    );
    assert!(report.run_root.join("results/third/basin.nc").is_file());
    assert_eq!(
        fs::metadata(report.run_root.join("scenarios/third/logs/stdout.log"))
            .unwrap()
            .len(),
        2_000_000
    );
    assert_eq!(
        fs::metadata(report.run_root.join("scenarios/zero/logs/stdout.log"))
            .unwrap()
            .len(),
        0
    );
}

#[test]
fn final_manifest_failure_reaps_children_and_keeps_running_manifest() {
    let _guard = TEST_LOCK.lock().unwrap();
    fs::create_dir_all("runs").unwrap();
    let dir = tempfile::tempdir_in("runs").unwrap();
    let path = dir.path().canonicalize().unwrap();
    fixture(&path);
    fs::write(path.join("Input/control.txt"), "manifest:other\n").unwrap();
    mgb_ops::app::prepare(mgb_ops::app::PreparationRequest {
        workspace: path.clone(),
        invocation_dir: path.clone(),
        request_file: "request.toml".into(),
        output: "prepared".into(),
    })
    .unwrap();
    let error = mgb_ops::app::execute_prepared(mgb_ops::app::ExecutionRequest {
        workspace: path.clone(),
        invocation_dir: path.clone(),
        prepared: "prepared".into(),
    })
    .unwrap_err();
    let mgb_ops::app::OperationError::Store { run_root, .. } = error else {
        panic!("expected store failure")
    };
    for id in ["zero", "other", "third", "fourth"] {
        assert!(
            run_root
                .join("scenarios")
                .join(id)
                .join("finished")
                .is_file()
        );
    }
    let manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(run_root.join("manifest.json")).unwrap()).unwrap();
    assert_eq!(manifest["state"], "running");
}

#[test]
fn lock_is_contended_by_another_process() {
    use std::io::{BufRead, BufReader};
    use std::process::Stdio;
    let _guard = TEST_LOCK.lock().unwrap();
    fs::create_dir_all("runs").unwrap();
    let dir = tempfile::tempdir_in("runs").unwrap();
    let path = dir.path().canonicalize().unwrap();
    fixture(&path);
    mgb_ops::app::prepare(mgb_ops::app::PreparationRequest {
        workspace: path.clone(),
        invocation_dir: path.clone(),
        request_file: "request.toml".into(),
        output: "prepared".into(),
    })
    .unwrap();
    let mut child = Command::new(path.join("engine"))
        .arg("lock")
        .arg(path.join(".operation.lock"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut line = String::new();
    BufReader::new(child.stdout.take().unwrap())
        .read_line(&mut line)
        .unwrap();
    assert_eq!(line, "locked\n");
    let result = mgb_ops::app::execute_prepared(mgb_ops::app::ExecutionRequest {
        workspace: path.clone(),
        invocation_dir: path.clone(),
        prepared: "prepared".into(),
    });
    assert!(matches!(
        result,
        Err(mgb_ops::app::OperationError::Busy { .. })
    ));
    drop(child.stdin.take());
    assert!(child.wait().unwrap().success());
}

#[test]
fn prepared_input_rejects_corrupt_json_paths_links_and_hashes() {
    let _guard = TEST_LOCK.lock().unwrap();
    fs::create_dir_all("runs").unwrap();
    let dir = tempfile::tempdir_in("runs").unwrap();
    let path = dir.path().canonicalize().unwrap();
    fixture(&path);
    mgb_ops::app::prepare(mgb_ops::app::PreparationRequest {
        workspace: path.clone(),
        invocation_dir: path.clone(),
        request_file: "request.toml".into(),
        output: "prepared".into(),
    })
    .unwrap();
    let root = path.join("prepared");
    let manifest_path = root.join("manifest.json");
    let original = fs::read(&manifest_path).unwrap();
    let run = || {
        mgb_ops::app::execute_prepared(mgb_ops::app::ExecutionRequest {
            workspace: path.clone(),
            invocation_dir: path.clone(),
            prepared: "prepared".into(),
        })
    };
    for bytes in [b"{".as_slice(), b"[]".as_slice()] {
        fs::write(&manifest_path, bytes).unwrap();
        assert!(matches!(
            run(),
            Err(mgb_ops::app::OperationError::InvalidPrepared { .. })
        ));
    }
    for field in ["path", "sha256"] {
        let mut manifest: serde_json::Value = serde_json::from_slice(&original).unwrap();
        manifest["inputs"][0][field] = if field == "path" {
            "../escape".into()
        } else {
            "0".repeat(64).into()
        };
        fs::write(&manifest_path, serde_json::to_vec(&manifest).unwrap()).unwrap();
        assert!(matches!(
            run(),
            Err(mgb_ops::app::OperationError::InvalidPrepared { .. })
        ));
    }
    let mut manifest: serde_json::Value = serde_json::from_slice(&original).unwrap();
    manifest["recipe"]["document"]["qc"]["high_threshold_mm"] = 99.into();
    fs::write(&manifest_path, serde_json::to_vec(&manifest).unwrap()).unwrap();
    assert!(matches!(
        run(),
        Err(mgb_ops::app::OperationError::InvalidPrepared { .. })
    ));
    fs::write(&manifest_path, original).unwrap();
    let input = root.join("request.toml");
    let backup = fs::read(&input).unwrap();
    fs::write(&input, b"changed").unwrap();
    assert!(matches!(
        run(),
        Err(mgb_ops::app::OperationError::InvalidPrepared { .. })
    ));
    fs::write(&input, backup).unwrap();
    #[cfg(unix)]
    {
        fs::remove_file(&input).unwrap();
        std::os::unix::fs::symlink(path.join("request.toml"), &input).unwrap();
        assert!(matches!(
            run(),
            Err(mgb_ops::app::OperationError::InvalidPrepared { .. })
        ));
    }
}

#[cfg(unix)]
#[test]
fn spawn_failure_is_retained_as_failed_history() {
    use std::os::unix::fs::PermissionsExt;
    let _guard = TEST_LOCK.lock().unwrap();
    fs::create_dir_all("runs").unwrap();
    let dir = tempfile::tempdir_in("runs").unwrap();
    let path = dir.path().canonicalize().unwrap();
    fixture(&path);
    fs::set_permissions(path.join("engine"), fs::Permissions::from_mode(0o644)).unwrap();
    mgb_ops::app::prepare(mgb_ops::app::PreparationRequest {
        workspace: path.clone(),
        invocation_dir: path.clone(),
        request_file: "request.toml".into(),
        output: "prepared".into(),
    })
    .unwrap();
    let report = mgb_ops::app::execute_prepared(mgb_ops::app::ExecutionRequest {
        workspace: path.clone(),
        invocation_dir: path.clone(),
        prepared: "prepared".into(),
    })
    .unwrap();
    assert!(matches!(report.status, mgb_ops::app::BatchStatus::Failed));
    assert!(report.scenarios.iter().all(|scenario| {
        scenario
            .error
            .as_ref()
            .unwrap()
            .message
            .contains("spawn failed")
    }));
}

#[cfg(unix)]
#[test]
fn readonly_baseline_is_copied_to_writable_private_inputs() {
    use std::os::unix::fs::PermissionsExt;
    let _guard = TEST_LOCK.lock().unwrap();
    fs::create_dir_all("runs").unwrap();
    let dir = tempfile::tempdir_in("runs").unwrap();
    let path = dir.path().canonicalize().unwrap();
    fixture(&path);
    fs::create_dir(path.join("Input/states")).unwrap();
    fs::write(path.join("Input/states/state.hot"), b"original state").unwrap();
    for name in ["PARHIG.hig", "states/state.hot"] {
        fs::set_permissions(
            path.join("Input").join(name),
            fs::Permissions::from_mode(0o444),
        )
        .unwrap();
    }
    let original = fs::read(path.join("Input/PARHIG.hig")).unwrap();
    mgb_ops::app::prepare(mgb_ops::app::PreparationRequest {
        workspace: path.clone(),
        invocation_dir: path.clone(),
        request_file: "request.toml".into(),
        output: "prepared".into(),
    })
    .unwrap();
    let report = mgb_ops::app::execute_prepared(mgb_ops::app::ExecutionRequest {
        workspace: path.clone(),
        invocation_dir: path.clone(),
        prepared: "prepared".into(),
    })
    .unwrap();
    assert!(matches!(
        report.status,
        mgb_ops::app::BatchStatus::Completed
    ));
    for name in ["PARHIG.hig", "states/state.hot"] {
        for baseline in [
            path.join("Input"),
            path.join("prepared/baseline/Input"),
            report.run_root.join("snapshot/baseline/Input"),
        ] {
            assert_eq!(
                fs::metadata(baseline.join(name))
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o777,
                0o444
            );
        }
        for scenario in [".history", "zero", "other", "third", "fourth"] {
            assert_ne!(
                fs::metadata(
                    report
                        .run_root
                        .join("scenarios")
                        .join(scenario)
                        .join("Input")
                        .join(name)
                )
                .unwrap()
                .permissions()
                .mode()
                    & 0o200,
                0
            );
        }
    }
    assert_eq!(fs::read(path.join("Input/PARHIG.hig")).unwrap(), original);
    assert_eq!(
        fs::read(report.run_root.join("snapshot/baseline/Input/PARHIG.hig")).unwrap(),
        original
    );
    assert_eq!(
        fs::read(
            report
                .run_root
                .join("snapshot/baseline/Input/states/state.hot")
        )
        .unwrap(),
        b"original state"
    );
}
