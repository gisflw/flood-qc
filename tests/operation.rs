use chrono::NaiveDateTime;
use mgb_ops::{
    ObservationBatch, ObservationSample, ObservationSeries, ObservationStore, ObservationValue,
    Station, StationOffset,
};
use std::{fs, path::Path, process::Command};
static TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn serialize_fixture() -> std::sync::MutexGuard<'static, ()> {
    // Each test owns its workspace. A failed fixture must not block the rest.
    TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}
fn time(value: &str) -> NaiveDateTime {
    NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M:%S").unwrap()
}
fn fixture(root: &Path) -> ObservationBatch {
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
    let batch = ObservationBatch {
        stations: vec![station],
        series: vec![series],
        samples,
        replacements: vec![],
        offsets: vec![StationOffset {
            station_id: "s".into(),
            mini_id: 1,
            offset_m: 0.609,
        }],
    };
    db.write_batch(&batch).unwrap();
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
    batch
}
fn configure_workspace(root: &Path, batch: &ObservationBatch) {
    fs::write(
        root.join("config.toml"),
        r#"
[run]
time_zone = "UTC"
reference_time = "2026-01-01T02:00:00"
lookback_days = 0
forecast_days = 1
[spatial]
bounds = [-1.0,-1.0,1.0,1.0]
resolution_degrees = 1.0
[model]
executable = "engine"
input_dir = "Input"
omp_threads = 1
[precipitation_interpolation]
nearest_stations = 1
"#,
    )
    .unwrap();

    mgb_ops::app::initialize_workspace(root).unwrap();
    ObservationStore::open_writable(&root.join("operation.sqlite"))
        .unwrap()
        .write_batch(batch)
        .unwrap();
}

#[test]
fn prepared_snapshot_is_reusable_and_scenarios_are_isolated() {
    let _guard = serialize_fixture();
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
    assert_eq!(
        mgb_ops::discover_trials(&path).unwrap(),
        std::slice::from_ref(&two.run_root)
    );
    assert!(!one.run_root.exists());
    assert_eq!(
        mgb_ops::read_trial(&two.run_root).unwrap().scenarios.len(),
        4
    );
    let rainfall = mgb_ops::app::rainfall_cache_info(&path).unwrap().unwrap();
    assert_eq!(Some(rainfall.generation.as_ref()), two.run_root.file_name());
    assert_eq!(fs::read_dir(path.join("runs/history")).unwrap().count(), 1);
    let stations =
        mgb_ops::store::read_station_results(&two.run_root.join("results/zero/station.nc"))
            .unwrap();
    assert_eq!(stations.level_cm, [40, 40, 40]);
    assert_eq!(stations.level_valid, [true, true, true]);
}

#[cfg(unix)]
#[test]
fn cli_streams_live_engine_output_without_holding_up_publication() {
    use std::{
        io::{BufRead, BufReader, Read},
        process::Stdio,
        time::{Duration, Instant},
    };
    let _guard = TEST_LOCK.lock().unwrap();
    fs::create_dir_all("runs").unwrap();
    let directory = tempfile::tempdir_in("runs").unwrap();
    let root = directory.path().canonicalize().unwrap();
    let mut batch = fixture(&root);
    batch.samples.push(ObservationSample {
        series_id: "r".into(),
        observed_at: time("2026-01-01T00:00:00"),
        value: Some(ObservationValue::RainMm(1.0)),
    });
    configure_workspace(&root, &batch);
    fs::write(
        root.join("Input/control.txt"),
        "log:.history\nwait:.history\nlog:zero\nwait:zero\nlarge:zero\n",
    )
    .unwrap();

    let prepare = |destination: &Path| {
        Command::new(env!("CARGO_BIN_EXE_mgb-ops"))
            .args(["operation", "prepare", "--workspace"])
            .arg(&root)
            .arg("--output")
            .arg(destination)
            .arg("--json")
            .output()
            .unwrap()
    };
    let prepared = root.join("prepared");
    let preparation = prepare(&prepared);
    assert!(preparation.status.success());
    let report: serde_json::Value = serde_json::from_slice(&preparation.stdout).unwrap();
    assert_eq!(report["prepared"].as_str(), prepared.to_str());
    let progress = String::from_utf8(preparation.stderr).unwrap();
    assert!(progress.contains("Preparing operation\n"));
    assert!(progress.contains("Reference time: 2026-01-01T02:00:00 (UTC)\n"));
    assert!(progress.contains("Scenarios: zero\n"));
    assert!(progress.contains("Prepared: "));

    let execute = |path: &Path| {
        let mut command = Command::new(env!("CARGO_BIN_EXE_mgb-ops"));
        command
            .args(["operation", "execute", "--workspace"])
            .arg(&root)
            .arg("--prepared")
            .arg(path);
        command
    };
    let mut child = execute(&prepared).stdout(Stdio::piped()).spawn().unwrap();
    let mut output = BufReader::new(child.stdout.take().unwrap());
    let cache = root.join("cache/run");
    let published = || mgb_ops::app::rainfall_cache_info(&root).unwrap().is_some();
    let release = |scenario: &str| {
        let worker = fs::read_dir(&cache)
            .unwrap()
            .map(|entry| entry.unwrap().path().join("scenarios").join(scenario))
            .find(|path| path.is_dir())
            .unwrap();
        assert!(!published());
        fs::write(worker.join("continue"), b"").unwrap();
    };
    assert!(
        output
            .by_ref()
            .lines()
            .any(|line| { line.unwrap() == "[MGB history stderr] diagnostic for .history" })
    );
    release(".history");
    assert!(
        output
            .by_ref()
            .lines()
            .any(|line| { line.unwrap() == "[MGB scenario zero stderr] diagnostic for zero" })
    );
    release("zero");

    // Hold the pipe full until the operation commits, then drain the console.
    let deadline = Instant::now() + Duration::from_secs(90);
    while !published() {
        assert!(
            Instant::now() < deadline,
            "console output blocked publication"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
    let mut rest = String::new();
    output.read_to_string(&mut rest).unwrap();
    assert!(child.wait().unwrap().success());
    assert!(rest.contains("bytes of console output"));
    assert!(rest.contains("Scenario zero: completed"));
    assert!(rest.contains("Finalizing run"));
    assert!(rest.contains("Run artifact: "));
    assert!(rest.lines().all(|line| line.len() < 17 * 1024));

    let published = mgb_ops::discover_trials(&root).unwrap().pop().unwrap();
    let mut expected = b"output for zero\r\n".to_vec();
    expected.extend(std::iter::repeat_n(b'x', 2_000_000));
    assert_eq!(
        fs::read(published.join("scenarios/zero/logs/stdout.log")).unwrap(),
        expected
    );
    let mut expected = b"diagnostic for zero\n".to_vec();
    expected.extend(std::iter::repeat_n(b'y', 2_000_000));
    assert_eq!(
        fs::read(published.join("scenarios/zero/logs/stderr.log")).unwrap(),
        expected
    );

    fs::write(root.join("Input/control.txt"), "log:.history\nlog:zero\n").unwrap();
    let second_preparation = root.join("prepared-fast");
    assert!(prepare(&second_preparation).status.success());
    let fast = execute(&second_preparation).arg("--json").output().unwrap();
    assert!(fast.status.success());
    let fast_report: serde_json::Value = serde_json::from_slice(&fast.stdout).unwrap();
    assert_eq!(fast_report["status"], "completed");
    let diagnostics = String::from_utf8(fast.stderr).unwrap();
    assert!(diagnostics.contains("[MGB history stdout] output for .history\n"));
    assert!(diagnostics.contains("[MGB scenario zero stdout] output for zero\n"));
    assert!(diagnostics.contains("[MGB scenario zero stderr] diagnostic for zero\n"));

    // Close the progress pipe while the CLI continues writing its JSON report.
    let mut child = execute(&second_preparation)
        .arg("--json")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    drop(child.stderr.take());
    let closed = child.wait_with_output().unwrap();
    assert!(closed.status.success());
    let closed_report: serde_json::Value = serde_json::from_slice(&closed.stdout).unwrap();
    let published = mgb_ops::discover_trials(&root).unwrap();
    assert_eq!(published.len(), 1);
    assert_eq!(closed_report["run_root"].as_str(), published[0].to_str());
    assert_ne!(closed_report["run_root"], fast_report["run_root"]);
}

#[test]
fn optional_flood_source_retains_exact_depth_and_reports_invalid_scenarios() {
    let _guard = serialize_fixture();
    fs::create_dir_all("runs").unwrap();
    let dir = tempfile::tempdir_in("runs").unwrap();
    let path = dir.path().canonicalize().unwrap();
    fixture(&path);

    let cell = [
        (8, "CatID", "1"),
        (8, "Mini", "1"),
        (15, "Xcen", "0.00"),
        (15, "Ycen", "0.00"),
        (8, "Sub", "1"),
        (15, "Area_(km2)", "1.00"),
        (15, "AreaM_(km2)", "1.00"),
        (15, "Ltr_(km)", "1.00"),
        (15, "Str_(m/km)", "1.00"),
        (15, "Lrl_(km)", "1.00"),
        (15, "Srl_(m/km)", "1.00"),
        (8, "MiniJus", "-1"),
        (8, "Ordem", "1"),
        (5, "Hdr", "1"),
        (10, "Width", "2.00"),
        (10, "Depth", "8.00"),
        (10, "NMAN", "0.04"),
        (10, "BLC_01", "100.00"),
    ];
    let header: String = cell
        .iter()
        .map(|(width, name, _)| format!("{name:>width$}"))
        .collect();
    let row: String = cell
        .iter()
        .map(|(width, _, value)| format!("{value:>width$}"))
        .collect();
    fs::write(path.join("Input/Cell.hig"), format!("{header}\n{row}\n")).unwrap();
    fs::write(
        path.join("Input/COTA_AREA.FLP"),
        "Minibacia Z0(m) Zfp(m) Areafp(km2)\n1 10 11 1\n",
    )
    .unwrap();
    fs::write(
        path.join("Input/control.txt"),
        "hand\nhand-bad:other\nhand-missing:third\nhand-length:fourth\n",
    )
    .unwrap();
    let prepared = mgb_ops::prepare(mgb_ops::PreparationRequest {
        workspace: path.clone(),
        invocation_dir: path.clone(),
        request_file: "request.toml".into(),
        output: "prepared".into(),
    })
    .unwrap();
    let trial = mgb_ops::execute_prepared(mgb_ops::ExecutionRequest {
        workspace: path.clone(),
        invocation_dir: path.clone(),
        prepared: prepared.prepared.clone(),
    })
    .unwrap();

    let source = trial.run_root.join("results/zero/flood_source.nc");
    let dataset = terrio::multidim::open(source.to_str().unwrap()).unwrap();
    let depth = dataset.array("depth").unwrap();
    assert_eq!(depth.info.data_type, terrio::DataType::F32);
    assert_eq!(
        depth.read().unwrap().bytes,
        [1.5_f32, 2.5, 3.5]
            .into_iter()
            .flat_map(f32::to_le_bytes)
            .collect::<Vec<_>>()
    );
    let channel = dataset.array("channel_depth").unwrap();
    assert_eq!(channel.read().unwrap().bytes, 8.0_f32.to_le_bytes());
    assert_eq!(dataset.root.attributes["historical_count"], 2);
    let canonical =
        mgb_ops::store::read_result_series(&trial.run_root.join("results/zero/basin.nc")).unwrap();
    assert_eq!(canonical.raw_level_cm, [350, 450, 550]);
    assert!(trial.scenarios[0].results.contains(&source));
    assert!(
        mgb_ops::read_trial(&trial.run_root).unwrap().scenarios[0]
            .results
            .contains(&source)
    );
    assert!(
        !trial
            .run_root
            .join("scenarios/zero/Output/HANDTUDO.MGB")
            .exists()
    );
    assert!(!trial.run_root.join("scenarios/.history/Output").exists());
    for (id, issue) in [
        ("other", "historical suffix differs"),
        ("third", "HANDTUDO.MGB"),
        ("fourth", "incorrect stream length"),
    ] {
        let results = trial.run_root.join("results").join(id);
        assert!(results.join("basin.nc").is_file());
        assert!(!results.join("flood_source.nc").exists());
        assert!(
            fs::read_to_string(results.join("flood_source_unavailable.txt"))
                .unwrap()
                .contains(issue)
        );
    }
    let bundle = path.join("bundle");
    mgb_ops::export_run(&trial.run_root, Some(&prepared.prepared), &bundle).unwrap();
    assert_eq!(
        fs::read(bundle.join("results/zero/flood_source.nc")).unwrap(),
        fs::read(source).unwrap()
    );
}

#[test]
fn run_bundle_round_trips_and_regenerates_without_original_sources() {
    let _guard = serialize_fixture();
    fs::create_dir_all("runs").unwrap();
    let dir = tempfile::tempdir_in("runs").unwrap();
    let source = dir.path().canonicalize().unwrap();
    fixture(&source);
    let prepared = mgb_ops::prepare(mgb_ops::PreparationRequest {
        workspace: source.clone(),
        invocation_dir: source.clone(),
        request_file: "request.toml".into(),
        output: "prepared".into(),
    })
    .unwrap();
    let trial = mgb_ops::execute_prepared(mgb_ops::ExecutionRequest {
        workspace: source.clone(),
        invocation_dir: source.clone(),
        prepared: prepared.prepared.clone(),
    })
    .unwrap();
    assert!(mgb_ops::export_run(&trial.run_root, None, &source.join("cache/run/bundle")).is_err());
    let bundle = source.join("bundle");
    mgb_ops::export_run(&trial.run_root, Some(&prepared.prepared), &bundle).unwrap();
    assert!(!bundle.join("preparation/baseline/engine").exists());
    assert!(mgb_ops::export_run(&trial.run_root, None, &bundle).is_err());
    for path in ["Input", "prepared", "cache/run"] {
        fs::remove_dir_all(source.join(path)).unwrap();
    }
    fs::remove_file(source.join("observations.sqlite")).unwrap();
    let target = source.join("receiver");
    fs::create_dir(&target).unwrap();
    let destination = target.join("saved-run");
    let imported = mgb_ops::import_run_to(&target, &bundle, &destination).unwrap();
    assert_eq!(imported.root, destination);
    assert!(imported.has_preparation);
    assert!(mgb_ops::import_run_to(&target, &bundle, &destination).is_err());
    let cache = target.join("cache/run");
    fs::create_dir_all(&cache).unwrap();
    assert!(mgb_ops::import_run_to(&target, &bundle, &cache.join("imported")).is_err());
    assert!(mgb_ops::discover_trials(&target).unwrap().is_empty());
    fs::write(source.join("wrong-engine"), b"different").unwrap();
    assert!(
        mgb_ops::regenerate_run(
            &target,
            &imported.root,
            &source.join("wrong-engine"),
            &Default::default()
        )
        .is_err()
    );
    let repeated = Command::new(env!("CARGO_BIN_EXE_mgb-ops"))
        .args(["run", "regenerate", "--workspace"])
        .arg(&target)
        .arg("--bundle")
        .arg(&imported.root)
        .arg("--engine")
        .arg(source.join("engine"))
        .output()
        .unwrap();
    let progress = String::from_utf8(repeated.stderr).unwrap();
    assert!(repeated.status.success(), "{progress}");
    assert!(progress.contains("Running shared MGB history"));
    let report: serde_json::Value = serde_json::from_slice(&repeated.stdout).unwrap();
    let repeated_root = Path::new(report["run_root"].as_str().unwrap());
    assert_ne!(repeated_root, imported.root);
    let raw = "scenarios/zero/Output/YTUDO.MGB";
    assert_eq!(
        fs::read(repeated_root.join(raw)).unwrap(),
        fs::read(imported.root.join(raw)).unwrap()
    );
    fs::write(bundle.join(raw), b"corrupt").unwrap();
    let other = source.join("other-receiver");
    fs::create_dir(&other).unwrap();
    assert!(mgb_ops::import_run_to(&other, &bundle, &other.join("rejected")).is_err());
    assert!(mgb_ops::discover_trials(&other).unwrap().is_empty());
}

#[test]
fn failed_scenario_archives_sibling_outcome_and_lock_rejects_overlap() {
    let _guard = serialize_fixture();
    use std::fs::OpenOptions;
    fs::create_dir_all("runs").unwrap();
    let dir = tempfile::tempdir_in("runs").unwrap();
    let path = dir.path().canonicalize().unwrap();
    fixture(&path);
    fs::write(path.join("Input/control.txt"), "log:other\nfail:other\n").unwrap();
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
    let Err(mgb_ops::OperationError::BatchFailed { report }) = run() else {
        panic!("expected failure")
    };
    let cli_run = Command::new(env!("CARGO_BIN_EXE_mgb-ops"))
        .current_dir("/")
        .args(["operation", "execute", "--workspace"])
        .arg(&path)
        .arg("--prepared")
        .arg(path.join("prepared"))
        .arg("--json")
        .output()
        .unwrap();
    assert_eq!(cli_run.status.code(), Some(1));
    let cli_result: serde_json::Value = serde_json::from_slice(&cli_run.stdout).unwrap();
    assert_eq!(cli_result["report"]["status"], "failed");
    let progress = String::from_utf8(cli_run.stderr).unwrap();
    assert!(progress.contains("[MGB scenario other stdout] output for other"));
    assert!(progress.contains("Scenario other: failed"));
    assert!(progress.contains(cli_result["report"]["run_artifact"].as_str().unwrap()));
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
    assert!(mgb_ops::discover_trials(&path).unwrap().is_empty());
    assert!(
        report
            .scenarios
            .iter()
            .all(|scenario| scenario.results.is_empty())
    );
    assert_eq!(report.scenarios[1].exit_code, Some(3));
    let manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(report.run_artifact).unwrap()).unwrap();
    assert_eq!(manifest["state"], "failed");
    assert_eq!(manifest["operation"]["scenarios"][0]["state"], "completed");
    fs::write(path.join("Input/control.txt"), "").unwrap();
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
fn malformed_output_archives_completed_siblings_and_bounded_log_tails() {
    let _guard = serialize_fixture();
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
    let result = mgb_ops::app::execute_prepared(mgb_ops::app::ExecutionRequest {
        workspace: path.clone(),
        invocation_dir: path.clone(),
        prepared: "prepared".into(),
    })
    .unwrap_err();
    let mgb_ops::OperationError::BatchFailed { report } = result else {
        panic!("expected failure")
    };
    assert!(matches!(report.status, mgb_ops::app::BatchStatus::Failed));
    assert!(
        report.scenarios[1]
            .error
            .as_ref()
            .unwrap()
            .message
            .contains("incorrect stream length")
    );
    let manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(report.run_artifact).unwrap()).unwrap();
    assert_eq!(manifest["operation"]["scenarios"][2]["state"], "completed");
    let diagnostic = manifest["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["path"] == "scenarios/third/logs/stdout.log")
        .unwrap();
    assert_eq!(diagnostic["tail"].as_str().unwrap(), "x".repeat(4096));
    assert!(mgb_ops::discover_trials(&path).unwrap().is_empty());
}

#[test]
fn final_manifest_failure_reaps_children_and_archives_attempt() {
    let _guard = serialize_fixture();
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
    assert!(!run_root.exists());
    let manifest = fs::read_dir(path.join("runs/history"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let manifest: serde_json::Value = serde_json::from_slice(&fs::read(manifest).unwrap()).unwrap();
    assert_eq!(manifest["state"], "failed");
    assert_eq!(manifest["operation"]["scenarios"][0]["state"], "completed");
    assert!(mgb_ops::discover_trials(&path).unwrap().is_empty());
}

#[test]
fn lock_is_contended_by_another_process() {
    use std::io::{BufRead, BufReader};
    use std::process::Stdio;
    let _guard = serialize_fixture();
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
    let _guard = serialize_fixture();
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
    let _guard = serialize_fixture();
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
    let result = mgb_ops::app::execute_prepared(mgb_ops::app::ExecutionRequest {
        workspace: path.clone(),
        invocation_dir: path.clone(),
        prepared: "prepared".into(),
    })
    .unwrap_err();
    let mgb_ops::OperationError::BatchFailed { report } = result else {
        panic!("expected failure")
    };
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
    let _guard = serialize_fixture();
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
        for baseline in [path.join("Input"), path.join("prepared/baseline/Input")] {
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
            assert!(
                !report
                    .run_root
                    .join("scenarios")
                    .join(scenario)
                    .join("Input")
                    .exists()
            );
        }
    }
    assert_eq!(fs::read(path.join("Input/PARHIG.hig")).unwrap(), original);
    assert!(!report.run_root.join("snapshot").exists());
}

#[test]
fn supplied_instructions_match_registered_and_frozen_execution() {
    use chrono::{TimeZone, Utc};
    use mgb_ops::store::{
        ForecastCatalog, ForecastCorrection, ForecastProvider, ForecastSource, GridSeries,
        IntervalAxis, Provenance, RegularGridSpec, TimeInterval,
    };
    use mgb_ops::{ObservationReplacement, ReplacementKind};
    use sha2::{Digest, Sha256};
    let _guard = serialize_fixture();
    fs::create_dir_all("runs").unwrap();
    let dir = tempfile::tempdir_in("runs").unwrap();
    let path = dir.path().canonicalize().unwrap();
    fixture(&path);
    let replacements = vec![
        ObservationReplacement {
            replacement_id: "period".into(),
            station_id: "s".into(),
            variable: "rain".into(),
            kind: ReplacementKind::Period,
            start_at: time("2026-01-01T01:00:00"),
            end_at: Some(time("2026-01-01T03:00:00")),
            value: Some(2.0),
            reason: "period".into(),
        },
        ObservationReplacement {
            replacement_id: "point".into(),
            station_id: "s".into(),
            variable: "rain".into(),
            kind: ReplacementKind::Point,
            start_at: time("2026-01-01T02:00:00"),
            end_at: None,
            value: Some(4.0),
            reason: "point".into(),
        },
    ];
    let db = rusqlite::Connection::open(path.join("observations.sqlite")).unwrap();
    db.execute(
        "UPDATE observation SET value=NULL WHERE observed_at='2026-01-01T01:00:00'",
        [],
    )
    .unwrap();
    for r in &replacements {
        db.execute(
            "INSERT INTO observation_replacement VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
            rusqlite::params![
                r.replacement_id,
                r.station_id,
                r.variable,
                if r.kind == ReplacementKind::Point {
                    "point"
                } else {
                    "period"
                },
                r.start_at.format("%Y-%m-%dT%H:%M:%S").to_string(),
                r.end_at.map(|t| t.format("%Y-%m-%dT%H:%M:%S").to_string()),
                r.value,
                r.reason
            ],
        )
        .unwrap();
    }
    drop(db);
    let source_bytes = b"forecast fixture";
    fs::write(path.join("forecast.grib2"), source_bytes).unwrap();
    let hash = format!("{:x}", Sha256::digest(source_bytes));
    let settings = serde_json::json!({"provider":"noaa","model":"gfs","product":"fc","cycle_utc":"2026-01-01T02:00:00Z","operation_timezone":"UTC","crop_bbox":[-1.,-1.,1.,1.]});
    let mut identity = Sha256::new();
    identity.update(hash.as_bytes());
    identity.update(serde_json::to_vec(&settings).unwrap());
    let source_id = format!("{:x}", identity.finalize());
    let source = ForecastSource {
        source_id: source_id.clone(),
        provider: ForecastProvider::Noaa,
        model: "gfs".into(),
        product: "fc".into(),
        cycle_utc: Utc.with_ymd_and_hms(2026, 1, 1, 2, 0, 0).unwrap(),
        sha256: hash,
        byte_length: source_bytes.len() as u64,
        normalization_json: settings.clone(),
    };
    let native = GridSeries {
        timeline: IntervalAxis {
            time_zone: "UTC".into(),
            bounds: vec![TimeInterval {
                start: time("2026-01-01T02:00:00"),
                end: time("2026-01-01T03:00:00"),
            }],
        },
        grid: RegularGridSpec::new([-1., -1., 1., 1.], 1.).unwrap(),
        values: vec![2.; 16],
        valid: vec![true; 16],
        provenance: Provenance {
            source_ids: vec![source_id],
            processing: serde_json::json!({"normalization":settings}),
        },
    };
    let mut catalog =
        ForecastCatalog::open(&path, Path::new("forecast.sqlite"), Path::new("assets")).unwrap();
    let asset = catalog
        .register_native(&path.join("forecast.grib2"), &source, &native)
        .unwrap();
    let correction = |id: &str, factor| ForecastCorrection {
        correction_id: id.into(),
        asset_id: asset.asset_id.clone(),
        t0_hour: 0,
        t1_hour: 1,
        shift_lat_pixels: 0.,
        shift_lon_pixels: 0.,
        rotation_deg: 0.,
        multiplication_factor: factor,
        editor: None,
        reason: "adjust".into(),
        created_at: "2026-01-01T02:00:00Z".into(),
    };
    let corrections = vec![correction("first", 2.), correction("second", 3.)];
    for c in &corrections {
        catalog.save_correction(c).unwrap();
    }
    drop(catalog);
    let mut document: mgb_ops::PreparationDocument =
        toml::from_str(&fs::read_to_string(path.join("request.toml")).unwrap()).unwrap();
    document.observed.replacement_ids = replacements
        .iter()
        .map(|r| r.replacement_id.clone())
        .collect();
    document.forecasts.catalog = "forecast.sqlite".into();
    document.scenarios.truncate(1);
    document.scenarios[0].id = "corrected".into();
    document.scenarios[0].kind = "corrected".into();
    document.scenarios[0].asset_id = Some(asset.asset_id.clone());
    document.scenarios[0].correction_ids = Some(
        corrections
            .iter()
            .map(|c| c.correction_id.clone())
            .collect(),
    );
    fs::write(
        path.join("registered.toml"),
        toml::to_string(&document).unwrap(),
    )
    .unwrap();
    mgb_ops::prepare(mgb_ops::PreparationRequest {
        workspace: path.clone(),
        invocation_dir: path.clone(),
        request_file: "registered.toml".into(),
        output: "registered".into(),
    })
    .unwrap();
    let report = mgb_ops::execute_prepared(mgb_ops::ExecutionRequest {
        workspace: path.clone(),
        invocation_dir: path.clone(),
        prepared: "registered".into(),
    })
    .unwrap();
    assert_eq!(report.scenarios.len(), 1);
    assert!(report.run_root.join("results/corrected/basin.nc").is_file());
}

#[test]
fn cancellation_reaps_engines_archives_outcomes_and_releases_lock() {
    use std::time::{Duration, Instant};
    let _guard = serialize_fixture();
    fs::create_dir_all("runs").unwrap();
    let dir = tempfile::tempdir_in("runs").unwrap();
    let root = dir.path().canonicalize().unwrap();
    fixture(&root);
    fs::write(root.join("Input/control.txt"), "sleep:other\nsleep:third\n").unwrap();
    mgb_ops::prepare(mgb_ops::PreparationRequest {
        workspace: root.clone(),
        invocation_dir: root.clone(),
        request_file: "request.toml".into(),
        output: "prepared".into(),
    })
    .unwrap();
    let control = mgb_ops::OperationControl::default();
    std::thread::scope(|scope| {
        let worker = scope.spawn(|| {
            mgb_ops::execute_prepared_controlled(
                mgb_ops::ExecutionRequest {
                    workspace: root.clone(),
                    invocation_dir: root.clone(),
                    prepared: "prepared".into(),
                },
                &control,
            )
        });
        let deadline = Instant::now() + Duration::from_secs(90);
        let (attempt, pids) = loop {
            let progress = control.progress();
            if let Some(progress) = progress
                && let Some(attempt) = progress.attempt
            {
                let one = attempt.join("scenarios/other/pid");
                let two = attempt.join("scenarios/third/pid");
                if one.is_file() && two.is_file() && attempt.join("results/zero/basin.nc").is_file()
                {
                    break (
                        attempt,
                        vec![
                            fs::read_to_string(one).unwrap(),
                            fs::read_to_string(two).unwrap(),
                        ],
                    );
                }
            }
            assert!(Instant::now() < deadline, "engines did not start");
            std::thread::sleep(Duration::from_millis(20));
        };
        let second = mgb_ops::execute_prepared(mgb_ops::ExecutionRequest {
            workspace: root.clone(),
            invocation_dir: root.clone(),
            prepared: "prepared".into(),
        });
        assert!(matches!(second, Err(mgb_ops::OperationError::Busy { .. })));
        control.cancel();
        let result = worker.join().unwrap();
        let Err(mgb_ops::OperationError::BatchFailed { report }) = result else {
            panic!("expected cancellation")
        };
        assert!(!attempt.exists());
        #[cfg(target_os = "linux")]
        for pid in pids {
            assert!(!Path::new("/proc").join(pid.trim()).exists());
        }
        let manifest: serde_json::Value =
            serde_json::from_slice(&fs::read(report.run_artifact).unwrap()).unwrap();
        assert_eq!(manifest["state"], "cancelled");
        assert_eq!(manifest["operation"]["scenarios"][0]["state"], "completed");
        assert!(mgb_ops::discover_trials(&root).unwrap().is_empty());
        let lock = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(root.join(".operation.lock"))
            .unwrap();
        lock.try_lock().unwrap();
    });
}

#[test]
fn cancelled_preparation_does_not_publish_partial_tree_and_tail_is_bounded() {
    let _guard = serialize_fixture();
    fs::create_dir_all("runs").unwrap();
    let dir = tempfile::tempdir_in("runs").unwrap();
    let root = dir.path().canonicalize().unwrap();
    fixture(&root);
    let control = mgb_ops::OperationControl::default();
    control.cancel();
    let result = mgb_ops::prepare_controlled(
        mgb_ops::PreparationRequest {
            workspace: root.clone(),
            invocation_dir: root.clone(),
            request_file: "request.toml".into(),
            output: "cancelled".into(),
        },
        &control,
    );
    assert!(result.is_err());
    assert!(!root.join("cancelled").exists());
    let log = root.join("large.log");
    fs::write(&log, vec![b'x'; 100_000]).unwrap();
    assert_eq!(mgb_ops::tail_log(&log, usize::MAX).unwrap().len(), 65536);
    assert_eq!(mgb_ops::tail_log(&log, 7).unwrap(), b"xxxxxxx");
}

#[cfg(unix)]
#[test]
fn cli_sigint_cancels_history_and_reaps_child() {
    use std::time::{Duration, Instant};
    let _guard = serialize_fixture();
    fs::create_dir_all("runs").unwrap();
    let dir = tempfile::tempdir_in("runs").unwrap();
    let root = dir.path().canonicalize().unwrap();
    fixture(&root);
    fs::write(
        root.join("Input/control.txt"),
        "log:.history\nsleep:.history\n",
    )
    .unwrap();
    mgb_ops::prepare(mgb_ops::PreparationRequest {
        workspace: root.clone(),
        invocation_dir: root.clone(),
        request_file: "request.toml".into(),
        output: "prepared".into(),
    })
    .unwrap();
    let cli = Command::new(env!("CARGO_BIN_EXE_mgb-ops"))
        .args(["operation", "execute", "--workspace"])
        .arg(&root)
        .arg("--prepared")
        .arg(root.join("prepared"))
        .arg("--json")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(90);
    let (attempt, pid) = 'waiting: loop {
        if let Ok(entries) = fs::read_dir(root.join("cache/run")) {
            for entry in entries.flatten() {
                let attempt = entry.path();
                if let Ok(pid) = fs::read_to_string(attempt.join("scenarios/.history/pid")) {
                    break 'waiting (attempt, pid);
                }
            }
        }
        assert!(Instant::now() < deadline, "history did not start");
        std::thread::sleep(Duration::from_millis(20));
    };
    assert!(
        Command::new("kill")
            .args(["-INT", &cli.id().to_string()])
            .status()
            .unwrap()
            .success()
    );
    let output = cli.wait_with_output().unwrap();
    assert!(!output.status.success());
    let result: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(result["kind"], "batch_failed");
    let progress = String::from_utf8(output.stderr).unwrap();
    assert!(progress.contains("[MGB history stdout] output for .history"));
    assert!(progress.contains("Run failed: operation cancelled"));
    assert!(!attempt.exists());
    let archive = Path::new(result["report"]["run_artifact"].as_str().unwrap());
    let manifest: serde_json::Value = serde_json::from_slice(&fs::read(archive).unwrap()).unwrap();
    assert_eq!(manifest["state"], "cancelled");
    assert_eq!(
        manifest["operation"]["scenarios"][0]["error"]["stage"],
        "history"
    );

    #[cfg(target_os = "linux")]
    assert!(!Path::new("/proc").join(pid.trim()).exists());
    let lock = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(root.join(".operation.lock"))
        .unwrap();
    lock.try_lock().unwrap();
}

#[test]
fn cancellation_during_preparation_removes_staging_and_preserves_existing_preparation() {
    use std::time::{Duration, Instant};
    let _guard = serialize_fixture();
    fs::create_dir_all("runs").unwrap();
    let dir = tempfile::tempdir_in("runs").unwrap();
    let root = dir.path().canonicalize().unwrap();
    fixture(&root);
    mgb_ops::prepare(mgb_ops::PreparationRequest {
        workspace: root.clone(),
        invocation_dir: root.clone(),
        request_file: "request.toml".into(),
        output: "previous".into(),
    })
    .unwrap();
    fs::write(root.join("Input/large.bin"), vec![1u8; 24_000_000]).unwrap();
    let control = mgb_ops::OperationControl::default();
    std::thread::scope(|scope| {
        let worker = scope.spawn(|| {
            mgb_ops::prepare_controlled(
                mgb_ops::PreparationRequest {
                    workspace: root.clone(),
                    invocation_dir: root.clone(),
                    request_file: "request.toml".into(),
                    output: "cancelled".into(),
                },
                &control,
            )
        });
        let deadline = Instant::now() + Duration::from_secs(90);
        loop {
            if fs::read_dir(&root).unwrap().flatten().any(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with(".cancelled.staging-")
            }) {
                break;
            }
            assert!(
                Instant::now() < deadline,
                "preparation staging did not appear"
            );
            std::thread::sleep(Duration::from_millis(1));
        }
        control.cancel();
        assert!(worker.join().unwrap().is_err());
    });
    assert!(!root.join("cancelled").exists());
    assert!(root.join("previous/manifest.json").exists());
    assert!(!fs::read_dir(&root).unwrap().flatten().any(|entry| {
        entry
            .file_name()
            .to_string_lossy()
            .starts_with(".cancelled.staging-")
    }));
}

#[test]
fn cancellation_during_finalization_preserves_the_publication_boundary() {
    use std::time::{Duration, Instant};
    let _guard = serialize_fixture();
    fs::create_dir_all("runs").unwrap();
    let dir = tempfile::tempdir_in("runs").unwrap();
    let root = dir.path().canonicalize().unwrap();
    fixture(&root);
    fs::write(
        root.join("Input/control.txt"),
        "large:zero\nlarge:other\nlarge:third\nlarge:fourth\n",
    )
    .unwrap();
    mgb_ops::prepare(mgb_ops::PreparationRequest {
        workspace: root.clone(),
        invocation_dir: root.clone(),
        request_file: "request.toml".into(),
        output: "prepared".into(),
    })
    .unwrap();
    let control = mgb_ops::OperationControl::default();
    std::thread::scope(|scope| {
        let worker = scope.spawn(|| {
            mgb_ops::execute_prepared_controlled(
                mgb_ops::ExecutionRequest {
                    workspace: root.clone(),
                    invocation_dir: root.clone(),
                    prepared: "prepared".into(),
                },
                &control,
            )
        });
        let deadline = Instant::now() + Duration::from_secs(180);
        loop {
            if worker.is_finished()
                || control.progress().is_some_and(|progress| {
                    matches!(progress.stage, mgb_ops::OperationStage::Finalization)
                })
            {
                break;
            }
            assert!(Instant::now() < deadline, "finalization did not start");
            std::thread::sleep(Duration::from_millis(1));
        }
        control.cancel();
        let result = worker.join().unwrap();
        match result {
            Err(mgb_ops::OperationError::Cancelled { .. })
            | Err(mgb_ops::OperationError::BatchFailed { .. }) => {
                assert!(mgb_ops::discover_trials(&root).unwrap().is_empty());
                let path = fs::read_dir(root.join("runs/history"))
                    .unwrap()
                    .next()
                    .unwrap()
                    .unwrap()
                    .path();
                let manifest: serde_json::Value =
                    serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
                assert_eq!(manifest["state"], "cancelled");
            }
            Ok(report) => {
                assert!(matches!(report.status, mgb_ops::BatchStatus::Completed));
                assert!(!control.is_cancelled());
                assert_eq!(
                    mgb_ops::discover_trials(&root).unwrap(),
                    std::slice::from_ref(&report.run_root)
                );
                assert_eq!(
                    mgb_ops::read_trial(&report.run_root)
                        .unwrap()
                        .scenarios
                        .len(),
                    4
                );
            }
            other => panic!("unexpected result: {other:?}"),
        }
    });
}

#[test]
fn failed_worker_tree_is_removed_after_workers_stop_and_lock_releases() {
    use std::time::{Duration, Instant};
    let _guard = serialize_fixture();
    fs::create_dir_all("runs").unwrap();
    let dir = tempfile::tempdir_in("runs").unwrap();
    let root = dir.path().canonicalize().unwrap();
    fixture(&root);
    fs::write(root.join("Input/control.txt"), "sleep:other\n").unwrap();
    mgb_ops::prepare(mgb_ops::PreparationRequest {
        workspace: root.clone(),
        invocation_dir: root.clone(),
        request_file: "request.toml".into(),
        output: "prepared".into(),
    })
    .unwrap();
    let control = mgb_ops::OperationControl::default();
    std::thread::scope(|scope| {
        let worker = scope.spawn(|| {
            mgb_ops::execute_prepared_controlled(
                mgb_ops::ExecutionRequest {
                    workspace: root.clone(),
                    invocation_dir: root.clone(),
                    prepared: "prepared".into(),
                },
                &control,
            )
        });
        let deadline = Instant::now() + Duration::from_secs(90);
        let attempt = loop {
            if let Some(attempt) = control.progress().and_then(|p| p.attempt)
                && attempt.join("scenarios/other/pid").is_file()
            {
                break attempt;
            }
            assert!(Instant::now() < deadline, "scenario did not start");
            std::thread::sleep(Duration::from_millis(10));
        };
        fs::remove_dir_all(attempt.join("scenarios/.history")).unwrap();
        fs::write(attempt.join("scenarios/.history"), b"blocked").unwrap();
        control.cancel();
        let result = worker.join().unwrap();
        let Err(mgb_ops::OperationError::BatchFailed { report }) = result else {
            panic!("expected cancelled batch")
        };
        assert!(report.run_artifact.is_file());
        assert!(!attempt.exists());
        let lock = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(root.join(".operation.lock"))
            .unwrap();
        lock.try_lock().unwrap();
    });
}

#[test]
fn workspace_parameters_resolve_cycles_and_freeze_forecast_corrections_once() {
    use chrono::Duration;
    use mgb_ops::app::{ForecastAdjustment, OperationalRun, RunScenario};
    use mgb_ops::store::{
        CanonicalForecastMetadata, ForecastCatalog, GridSeries, IntervalAxis, Provenance,
        RegularGridSpec, TimeInterval,
    };

    let _guard = serialize_fixture();
    fs::create_dir_all("runs").unwrap();
    let fixture_dir = tempfile::tempdir_in("runs").unwrap();
    let root = fixture_dir.path().canonicalize().unwrap();
    let mut batch = fixture(&root);
    batch.samples.push(ObservationSample {
        series_id: "r".into(),
        observed_at: time("2026-01-01T00:00:00"),
        value: Some(ObservationValue::RainMm(1.0)),
    });
    let database = root.join("operation.sqlite");
    configure_workspace(&root, &batch);

    let reference = time("2026-01-01T02:00:00");
    let grid = RegularGridSpec::new([-1.0, -1.0, 1.0, 1.0], 1.0).unwrap();
    let cells = grid.latitudes.len() * grid.longitudes.len();
    let mut catalog = ForecastCatalog::open(&root, &database, Path::new("assets")).unwrap();
    for (product, cycle_hour, hours) in [
        ("hourly", 0, 25),
        ("hourly", 2, 25),
        ("hourly", 3, 25),
        ("short", 2, 24),
        ("alternate", 2, 25),
    ] {
        let source = root.join(format!("source-{product}-{cycle_hour}.nc"));
        mgb_ops::store::write_grid_series(
            &source,
            &GridSeries {
                timeline: IntervalAxis {
                    time_zone: "UTC".into(),
                    bounds: vec![TimeInterval {
                        start: reference,
                        end: reference + Duration::hours(hours),
                    }],
                },
                values: vec![2.0 * hours as f64; cells],
                valid: vec![true; cells],
                grid: grid.clone(),
                provenance: Provenance {
                    source_ids: vec!["forecast-fixture".into()],
                    processing: serde_json::json!({}),
                },
            },
        )
        .unwrap();
        catalog
            .register_canonical(
                &source,
                &CanonicalForecastMetadata {
                    provider: "fixture".into(),
                    model: "native".into(),
                    product: product.into(),
                    cycle_utc: time(&format!("2026-01-01T{cycle_hour:02}:00:00")).and_utc(),
                },
            )
            .unwrap();
        fs::remove_file(source).unwrap();
    }
    let mut operation = OperationalRun::open_workspace(root.clone(), reference.and_utc()).unwrap();
    let forecast = "fixture/native/hourly@2026-01-01T02:00:00Z";
    assert_eq!(
        operation.default_scenarios().unwrap(),
        [
            RunScenario::Zero,
            RunScenario::Raw {
                forecast: "fixture/native/alternate@2026-01-01T02:00:00Z".into()
            },
            RunScenario::Raw {
                forecast: forecast.into()
            },
        ]
    );
    assert_eq!(operation.available_forecasts().unwrap().len(), 3);
    let newest = catalog
        .asset_choices()
        .unwrap()
        .into_iter()
        .find(|asset| asset.product == "hourly" && asset.cycle_utc == "2026-01-01T02:00:00Z")
        .unwrap();
    let (newest, _) = catalog.load_native(&newest.asset_id).unwrap();
    let native = root.join(&newest.native_artifact);
    let mut corrupted = fs::read(&native).unwrap();
    let original_bytes = corrupted.clone();
    corrupted[0] ^= 1;
    fs::write(&native, corrupted).unwrap();
    let defaults = operation.default_scenarios().unwrap();
    assert_eq!(
        defaults[2],
        RunScenario::Raw {
            forecast: "fixture/native/hourly@2026-01-01T00:00:00Z".into(),
        }
    );
    fs::write(&native, original_bytes).unwrap();
    let scenario = RunScenario::Corrected {
        forecast: forecast.into(),
        corrections: [2.0, 3.0]
            .into_iter()
            .map(|multiplication_factor| ForecastAdjustment {
                t1_hour: 25,
                multiplication_factor,
                ..Default::default()
            })
            .collect(),
    };
    let preview = operation.preview_forecast(&scenario).unwrap();
    assert_eq!(preview.raw[0].values[0], 50.0);
    assert_eq!(preview.corrected[0].values[0], 300.0);
    let cli = Command::new(env!("CARGO_BIN_EXE_mgb-ops"))
        .current_dir(&root)
        .args([
            "operation",
            "prepare",
            "--workspace",
            ".",
            "--output",
            "prepared",
            "--scenario",
            "zero",
            "--scenario",
            forecast,
            "--scenario",
        ])
        .arg(serde_json::to_string(&scenario).unwrap())
        .arg("--json")
        .output()
        .unwrap();
    assert!(
        cli.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&cli.stdout),
        String::from_utf8_lossy(&cli.stderr)
    );
    let prepared = root.join("prepared");
    let manifest = mgb_ops::store::read_operation_manifest(&prepared).unwrap();
    assert_eq!(manifest.scenarios.len(), 3);
    let corrected = &manifest.scenarios[2].id;
    let edits = &manifest.recipe["workspace_scenarios"][2]["corrections"];
    assert_eq!(edits[0]["multiplication_factor"], 2.0);
    assert_eq!(edits[1]["multiplication_factor"], 3.0);
    assert!(!prepared.join("request.toml").exists());
    assert!(!prepared.join("sources/forecast.sqlite").exists());
    let working = mgb_ops::store::read_grid_series(
        &prepared
            .join("checkpoints/forecast")
            .join(corrected)
            .join("working.nc"),
    )
    .unwrap();
    assert_eq!(working.values[0], preview.corrected[0].values[0] / 25.0);
    assert!(
        !working
            .provenance
            .processing
            .to_string()
            .contains("multiplication_factor")
    );
    let index = rusqlite::Connection::open(&database).unwrap();
    assert!(catalog.correction_choices().unwrap().is_empty());

    operation
        .set_scenarios(&[
            "zero".into(),
            forecast.into(),
            serde_json::to_string(&scenario).unwrap(),
        ])
        .unwrap();
    let run = operation.run_all_report(&mgb_ops::OperationControl::default());
    assert!(run.error.is_none(), "{:?}", run.error);
    let first = run.execution.unwrap();
    let active = mgb_ops::store::read_retained_manifest(&first.run_root).unwrap();
    assert_eq!(
        active.recipe["workspace_scenarios"][2]["corrections"],
        *edits
    );
    let asset_index: String = index
        .query_row("SELECT manifest FROM run_attempt", [], |row| row.get(0))
        .unwrap();
    let indexed: serde_json::Value = serde_json::from_str(&asset_index).unwrap();
    assert_eq!(indexed["state"], "completed");
    assert_eq!(indexed["attempt_id"], first.run_id);

    fs::remove_dir_all(root.join("assets")).unwrap();
    operation.set_scenarios(&[forecast.into()]).unwrap();
    let failed = operation.run_all_report(&mgb_ops::OperationControl::default());
    assert!(failed.error.is_some());
    assert_eq!(
        mgb_ops::discover_trials(&root).unwrap(),
        std::slice::from_ref(&first.run_root)
    );
    let second = mgb_ops::execute_prepared(mgb_ops::ExecutionRequest {
        workspace: root.clone(),
        invocation_dir: root.clone(),
        prepared,
    })
    .unwrap();
    assert_ne!(first.run_root, second.run_root);
    let id = first
        .run_root
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .strip_prefix("generation-")
        .unwrap();
    let archived: String = index
        .query_row(
            "SELECT manifest FROM run_attempt WHERE attempt_id=?1",
            [id],
            |row| row.get(0),
        )
        .unwrap();
    let archive: serde_json::Value = serde_json::from_str(&archived).unwrap();
    let saved: serde_json::Value = serde_json::from_slice(
        &fs::read(root.join("runs/history").join(format!("{id}.json"))).unwrap(),
    )
    .unwrap();
    assert_eq!(archive, saved);
    assert_eq!(
        archive["operation"]["recipe"]["workspace_scenarios"][2]["corrections"],
        *edits
    );
    assert!(!first.run_root.exists());
}
