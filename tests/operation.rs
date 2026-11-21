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
    assert_eq!(
        mgb_ops::discover_trials(&path).unwrap(),
        [two.run_root.clone()]
    );
    assert!(!one.run_root.exists());
    assert_eq!(
        mgb_ops::read_trial(&two.run_root).unwrap().scenarios.len(),
        4
    );
    let rainfall = mgb_ops::app::rainfall_cache_info(&path).unwrap().unwrap();
    assert_eq!(Some(rainfall.generation.as_ref()), two.run_root.file_name());
    assert_eq!(
        fs::read_dir(path.join("data/run-manifests"))
            .unwrap()
            .count(),
        2
    );
    let copied = mgb_ops::Draft::import_manifest(path.clone(), &path, &two.run_root).unwrap();
    assert_eq!(copied.document.operation.scenarios.len(), 4);
    let stations =
        mgb_ops::store::read_station_results(&two.run_root.join("results/zero/station.nc"))
            .unwrap();
    assert_eq!(stations.level_cm, [40, 40, 40]);
    assert_eq!(stations.level_valid, [true, true, true]);
}

#[test]
fn optional_flood_source_retains_exact_depth_and_reports_invalid_scenarios() {
    let _guard = TEST_LOCK.lock().unwrap();
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
    let _guard = TEST_LOCK.lock().unwrap();
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
    assert!(
        mgb_ops::export_run(&trial.run_root, None, &source.join("data/cache/run/bundle")).is_err()
    );
    let bundle = source.join("bundle");
    mgb_ops::export_run(&trial.run_root, Some(&prepared.prepared), &bundle).unwrap();
    assert!(!bundle.join("preparation/baseline/engine").exists());
    assert!(mgb_ops::export_run(&trial.run_root, None, &bundle).is_err());
    for path in ["Input", "prepared", "data/cache/run"] {
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
    let cache = target.join("data/cache/run");
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
    let repeated = mgb_ops::regenerate_run(
        &target,
        &imported.root,
        &source.join("engine"),
        &Default::default(),
    )
    .unwrap();
    assert_ne!(repeated.run_root, imported.root);
    let raw = "scenarios/zero/Output/YTUDO.MGB";
    assert_eq!(
        fs::read(repeated.run_root.join(raw)).unwrap(),
        fs::read(imported.root.join(raw)).unwrap()
    );
    fs::write(bundle.join(raw), b"corrupt").unwrap();
    let other = source.join("other-receiver");
    fs::create_dir(&other).unwrap();
    assert!(mgb_ops::import_run_to(&other, &bundle, &other.join("rejected")).is_err());
    assert!(mgb_ops::discover_trials(&other).unwrap().is_empty());
}

#[test]
fn canonical_forecast_reaches_frozen_preparation_and_execution() {
    use mgb_ops::app::{CanonicalForecastDocument, import_canonical_forecast};
    use mgb_ops::store::{
        CanonicalForecastMetadata, GridSeries, IntervalAxis, Provenance, RegularGridSpec,
        TimeInterval, write_grid_series,
    };

    let _guard = TEST_LOCK.lock().unwrap();
    fs::create_dir_all("runs").unwrap();
    let dir = tempfile::tempdir_in("runs").unwrap();
    let root = dir.path().canonicalize().unwrap();
    fixture(&root);
    let grid = RegularGridSpec::new([-1.0, -1.0, 1.0, 1.0], 1.0).unwrap();
    let cells = grid.latitudes.len() * grid.longitudes.len();
    let source = root.join("forecast.nc");
    write_grid_series(
        &source,
        &GridSeries {
            timeline: IntervalAxis {
                time_zone: "UTC".into(),
                bounds: vec![TimeInterval {
                    start: time("2026-01-01T02:00:00"),
                    end: time("2026-01-01T03:00:00"),
                }],
            },
            grid,
            values: vec![2.0; cells],
            valid: vec![true; cells],
            provenance: Provenance {
                source_ids: vec!["canonical-forecast".into()],
                processing: serde_json::Value::Object(serde_json::Map::new()),
            },
        },
    )
    .unwrap();
    let asset = import_canonical_forecast(
        &root,
        CanonicalForecastDocument {
            catalog: "forecast.sqlite".into(),
            artifact_root: "forecast-assets".into(),
            source: source.clone(),
            metadata: CanonicalForecastMetadata {
                provider: "fixture".into(),
                model: "saved".into(),
                product: "hourly".into(),
                cycle_utc: "2026-01-01T02:00:00Z".parse().unwrap(),
            },
        },
    )
    .unwrap();
    let request = root.join("request.toml");
    let contents = fs::read_to_string(&request)
        .unwrap()
        .replace("unused.sqlite", "forecast.sqlite")
        .replace(
            "id = \"other\"\nkind = \"zero\"",
            &format!(
                "id = \"other\"\nkind = \"raw\"\nasset_id = \"{}\"",
                asset.asset_id
            ),
        );
    fs::write(&request, contents).unwrap();
    fs::remove_file(source).unwrap();
    let mut operation = mgb_ops::app::OperationalRun::open(
        root.clone(),
        &request,
        time("2026-01-01T03:00:00").and_utc(),
    )
    .unwrap();
    assert!(
        operation
            .validate()
            .iter()
            .any(|error| error.contains("MissingTimeCoverage"))
    );
    assert!(
        operation
            .accept_inputs()
            .unwrap_err()
            .contains("Scenario other")
    );
    assert!(!operation.has_accepted_inputs());
    operation.set_reference("2026-01-01T02:00:00".into());
    operation.accept_inputs().unwrap();
    assert!(mgb_ops::app::rainfall_cache_info(&root).unwrap().is_none());
    operation
        .run_all(&mgb_ops::OperationControl::default())
        .unwrap();
    let generation = mgb_ops::app::rainfall_cache_info(&root).unwrap().unwrap();
    let query = |scenario, period| {
        mgb_ops::app::query_cached_rainfall(&root, scenario, &period, None, false)
            .unwrap()
            .unwrap()
    };
    use mgb_ops::app::RainfallPeriod::{Explicit, SignedHours};
    assert_eq!(query("zero", SignedHours(1)).values, vec![Some(0.); cells]);
    let forecast = query("other", SignedHours(1));
    assert_eq!(forecast.values[5], Some(2.));
    assert_eq!(query("other", SignedHours(-1)).values[5], Some(1.));
    let combined = query(
        "other",
        Explicit {
            start: time("2026-01-01T01:00:00"),
            end: time("2026-01-01T03:00:00"),
        },
    );
    assert_eq!(combined.info, generation);
    assert_eq!(combined.values[5], Some(3.));
    assert_eq!(query("other", SignedHours(2)).values, vec![None; cells]);
    assert!(!root.join("data/cache/preparations").exists());
    let prepared = mgb_ops::prepare(mgb_ops::PreparationRequest {
        workspace: root.clone(),
        invocation_dir: root.clone(),
        request_file: request,
        output: "prepared".into(),
    })
    .unwrap();
    let assets = prepared.prepared.join("sources/original");
    assert!(assets.join(format!("{}.nc", asset.asset_id)).is_file());
    fs::remove_dir_all(root.join("forecast-assets")).unwrap();
    let trial = mgb_ops::execute_prepared(mgb_ops::ExecutionRequest {
        workspace: root.clone(),
        invocation_dir: root.clone(),
        prepared: prepared.prepared,
    })
    .unwrap();
    let basin =
        mgb_ops::store::read_result_series(&trial.run_root.join("results/other/basin.nc")).unwrap();
    assert_eq!(
        mgb_ops::app::rainfall_cache_info(&root).unwrap(),
        Some(generation)
    );
    assert_eq!(basin.precipitation.last(), Some(&2.0));
}

#[test]
fn failed_scenario_archives_sibling_outcome_and_lock_rejects_overlap() {
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
    let Err(mgb_ops::OperationError::BatchFailed { report }) = run() else {
        panic!("expected failure")
    };
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
    assert_eq!(cli_result["report"]["status"], "failed");
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
    assert!(!run_root.exists());
    let manifest = fs::read_dir(path.join("data/run-manifests"))
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
    let _guard = TEST_LOCK.lock().unwrap();
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
    let mut catalog = ForecastCatalog::open(
        &path,
        Path::new("data/forecasts.sqlite"),
        Path::new("data/forecasts"),
    )
    .unwrap();
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
    document.forecasts.catalog = "data/forecasts.sqlite".into();
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
    let copied =
        mgb_ops::Draft::import_manifest(path.clone(), &path, Path::new("registered")).unwrap();
    assert_eq!(
        copied
            .document
            .instructions
            .replacements
            .as_ref()
            .unwrap()
            .len(),
        replacements.len()
    );
    assert_eq!(
        copied.document.instructions.corrections[0]
            .as_ref()
            .unwrap()
            .len(),
        corrections.len()
    );
    let mut unaccepted =
        mgb_ops::Draft::from_preparation(path.clone(), &path, document.clone()).unwrap();
    unaccepted.document.operation.geometry = None;
    unaccepted.document.operation.engine = None;
    unaccepted
        .save(&path.join("unaccepted.json"), true)
        .unwrap();
    let native_path = path.join(&asset.native_artifact);
    let moved = native_path.with_extension("missing");
    fs::rename(&native_path, &moved).unwrap();
    let opened = mgb_ops::Draft::open(path.clone(), &path, Path::new("unaccepted.json")).unwrap();
    assert!(
        opened
            .findings
            .iter()
            .any(|finding| finding.starts_with("forecast:0 unavailable"))
    );
    assert!(opened.document.sources.accepted.is_empty());
    fs::rename(&moved, &native_path).unwrap();
    let mut draft =
        mgb_ops::Draft::from_preparation(path.clone(), &path, document.clone()).unwrap();
    draft.refresh_forecast(0).unwrap();
    let preview = draft.preview_forecast(0).unwrap();
    let prepared_frame = mgb_ops::store::read_grid_frame(
        &path.join("registered/checkpoints/forecast/corrected/0000.nc"),
    )
    .unwrap();
    assert_eq!(preview.corrected[0], prepared_frame);
    let catalog = ForecastCatalog::open(
        &path,
        Path::new("data/forecasts.sqlite"),
        Path::new("data/forecasts"),
    )
    .unwrap();
    catalog
        .save_correction(&correction("unrelated", 4.))
        .unwrap();
    drop(catalog);
    assert!(draft.preview_forecast(0).is_ok());
    let mut invalid_forecast = draft.clone();
    let mut bad_correction = corrections[0].clone();
    bad_correction.multiplication_factor = 0.0;
    invalid_forecast.document.instructions.corrections =
        vec![Some(vec![mgb_ops::DraftCorrection {
            draft_id: "bad".into(),
            registered_origin: None,
            value: bad_correction,
        }])];
    assert!(
        invalid_forecast
            .validate_preparation()
            .iter()
            .any(|v| v.contains("invalid correction"))
    );
    assert!(invalid_forecast.preview_forecast(0).is_err());
    draft.refresh_inputs().unwrap();
    let before_observations = draft.preview_observations().unwrap();
    let before_forecast = draft.preview_forecast(0).unwrap();
    draft.save(&path.join("materialize.json"), true).unwrap();
    for (source, scenario) in [("observations", None), ("forecast", Some("0"))] {
        let mut command = Command::new(env!("CARGO_BIN_EXE_mgb-ops"));
        command.current_dir(&path).args([
            "draft",
            "materialize",
            "--workspace",
            ".",
            "--draft",
            "materialize.json",
            "--source",
            source,
        ]);
        if let Some(scenario) = scenario {
            command.args(["--scenario", scenario]);
        }
        assert!(command.output().unwrap().status.success());
    }
    draft.materialize_replacements().unwrap();
    draft.materialize_corrections(0).unwrap();
    let cli_materialized =
        mgb_ops::Draft::open(path.clone(), &path, Path::new("materialize.json")).unwrap();
    assert_eq!(draft.document, cli_materialized.document);
    assert!(draft.preview_observations().is_err());
    assert!(draft.preview_forecast(0).is_err());
    draft.refresh_inputs().unwrap();
    assert_eq!(
        before_observations.rows,
        draft.preview_observations().unwrap().rows
    );
    assert_eq!(
        before_forecast.corrected,
        draft.preview_forecast(0).unwrap().corrected
    );
    let draft_replacements = draft.document.instructions.replacements.as_mut().unwrap();
    draft_replacements[0].value.reason.clear();
    draft_replacements[1].value.reason.clear();
    let mut excluded = draft_replacements[1].clone();
    excluded.draft_id = "excluded".into();
    excluded.registered_origin = None;
    excluded.value.replacement_id = "excluded".into();
    excluded.value.value = None;
    draft_replacements.insert(1, excluded);
    for correction in draft.document.instructions.corrections[0].as_mut().unwrap() {
        correction.value.reason.clear();
        correction.value.editor = None;
    }
    let before_observations = draft.preview_observations().unwrap();
    let before_forecast = draft.preview_forecast(0).unwrap();
    assert_eq!(before_observations.rows[1].effective_value, Some(4.0));
    let obs_count = |path: &Path| -> i64 {
        rusqlite::Connection::open(path.join("observations.sqlite"))
            .unwrap()
            .query_row("SELECT count(*) FROM observation_replacement", [], |r| {
                r.get(0)
            })
            .unwrap()
    };
    let forecast_count = |path: &Path| -> i64 {
        rusqlite::Connection::open(path.join("data/forecasts.sqlite"))
            .unwrap()
            .query_row("SELECT count(*) FROM correction", [], |r| r.get(0))
            .unwrap()
    };
    let counts = (obs_count(&path), forecast_count(&path));
    draft
        .prepare(path.clone(), "before-registration".into())
        .unwrap();
    assert_eq!(counts, (obs_count(&path), forecast_count(&path)));
    let selection = mgb_ops::RegistrationSelection {
        replacements: vec!["period".into(), "excluded".into(), "point".into()],
        corrections: vec![
            mgb_ops::RegistrationCorrectionSelection {
                scenario: 0,
                draft_id: "first".into(),
            },
            mgb_ops::RegistrationCorrectionSelection {
                scenario: 0,
                draft_id: "second".into(),
            },
        ],
    };
    draft
        .save(&path.join("registration-draft.json"), true)
        .unwrap();
    fs::write(
        path.join("selection.json"),
        serde_json::to_vec(&selection).unwrap(),
    )
    .unwrap();
    let partial = mgb_ops::RegistrationSelection {
        replacements: vec!["period".into()],
        corrections: vec![mgb_ops::RegistrationCorrectionSelection {
            scenario: 0,
            draft_id: "unknown".into(),
        }],
    };
    fs::write(
        path.join("partial.json"),
        serde_json::to_vec(&partial).unwrap(),
    )
    .unwrap();
    let cli_partial = Command::new(env!("CARGO_BIN_EXE_mgb-ops"))
        .current_dir(&path)
        .args([
            "draft",
            "register",
            "--workspace",
            ".",
            "--draft",
            "registration-draft.json",
            "--selection",
            "partial.json",
            "--json",
        ])
        .output()
        .unwrap();
    assert!(!cli_partial.status.success());
    let partial_result: serde_json::Value = serde_json::from_slice(&cli_partial.stdout).unwrap();
    assert_eq!(partial_result["ok"], false);
    assert_eq!(
        partial_result["result"],
        serde_json::to_value(draft.register_selected(&partial)).unwrap()
    );
    assert_eq!(
        partial_result["result"]["observations"]["mappings"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert!(partial_result["result"]["forecasts"]["error"].is_string());
    let report = draft.register_selected(&selection);
    assert!(report.succeeded());
    assert_eq!(
        before_observations.rows,
        draft.preview_observations().unwrap().rows
    );
    assert_eq!(
        before_forecast.corrected,
        draft.preview_forecast(0).unwrap().corrected
    );
    draft
        .prepare(path.clone(), "after-registration".into())
        .unwrap();
    let before =
        mgb_ops::store::read_basin_forcing(&path.join("before-registration/forcing/corrected.nc"))
            .unwrap();
    let after =
        mgb_ops::store::read_basin_forcing(&path.join("after-registration/forcing/corrected.nc"))
            .unwrap();
    assert_eq!(before, after);
    let cli = Command::new(env!("CARGO_BIN_EXE_mgb-ops"))
        .current_dir(&path)
        .args([
            "draft",
            "register",
            "--workspace",
            ".",
            "--draft",
            "registration-draft.json",
            "--selection",
            "selection.json",
            "--json",
        ])
        .output()
        .unwrap();
    assert!(cli.status.success());
    let result: serde_json::Value = serde_json::from_slice(&cli.stdout).unwrap();
    assert_eq!(result["result"], serde_json::to_value(&report).unwrap());
    let mapping = &report.observations.mappings[0];
    fs::write(
        path.join("mapping.json"),
        serde_json::to_vec(mapping).unwrap(),
    )
    .unwrap();
    let mut api_copy =
        mgb_ops::Draft::open(path.clone(), &path, Path::new("registration-draft.json")).unwrap();
    api_copy.copy_registered_for_edit(mapping).unwrap();
    let cli_copy = Command::new(env!("CARGO_BIN_EXE_mgb-ops"))
        .current_dir(&path)
        .args([
            "draft",
            "copy-registered-for-edit",
            "--workspace",
            ".",
            "--draft",
            "registration-draft.json",
            "--mapping",
            "mapping.json",
            "--json",
        ])
        .output()
        .unwrap();
    assert!(cli_copy.status.success());
    let copy_result: serde_json::Value = serde_json::from_slice(&cli_copy.stdout).unwrap();
    assert_eq!(
        copy_result["result"],
        serde_json::to_value(&api_copy.document).unwrap()
    );
    let saved_bytes = fs::read(path.join("registration-draft.json")).unwrap();
    let mut wrong = serde_json::to_value(mapping).unwrap();
    wrong["catalog_id"] = "missing".into();
    fs::write(
        path.join("mapping.json"),
        serde_json::to_vec(&wrong).unwrap(),
    )
    .unwrap();
    let rejected = Command::new(env!("CARGO_BIN_EXE_mgb-ops"))
        .current_dir(&path)
        .args([
            "draft",
            "copy-registered-for-edit",
            "--workspace",
            ".",
            "--draft",
            "registration-draft.json",
            "--mapping",
            "mapping.json",
            "--json",
        ])
        .output()
        .unwrap();
    assert!(!rejected.status.success());
    assert_eq!(
        fs::read(path.join("registration-draft.json")).unwrap(),
        saved_bytes
    );
    let obs = rusqlite::Connection::open(path.join("observations.sqlite")).unwrap();
    obs.execute("DELETE FROM observation_replacement", [])
        .unwrap();
    drop(obs);
    let cat = rusqlite::Connection::open(path.join("data/forecasts.sqlite")).unwrap();
    cat.execute("DELETE FROM correction", []).unwrap();
    drop(cat);
    let source_obs = fs::read(path.join("observations.sqlite")).unwrap();
    let source_catalog = fs::read(path.join("data/forecasts.sqlite")).unwrap();
    let typed =
        |output: &str,
         replacements: Vec<ObservationReplacement>,
         corrections: Vec<ForecastCorrection>| mgb_ops::TypedPreparationRequest {
            workspace: path.clone(),
            invocation_dir: path.clone(),
            output: output.into(),
            document: document.clone(),
            replacements: Some(replacements),
            corrections: vec![Some(corrections)],
        };
    mgb_ops::prepare_typed(typed("supplied", replacements.clone(), corrections.clone())).unwrap();
    let first =
        mgb_ops::store::read_basin_forcing(&path.join("registered/forcing/corrected.nc")).unwrap();
    let second =
        mgb_ops::store::read_basin_forcing(&path.join("supplied/forcing/corrected.nc")).unwrap();
    assert_eq!(first, second);
    let mut shared = typed("shared", replacements.clone(), corrections.clone());
    let mut sibling = shared.document.scenarios[0].clone();
    sibling.id = "sibling".into();
    shared.document.scenarios.push(sibling);
    shared.corrections.push(Some(corrections.clone()));
    mgb_ops::prepare_typed(shared.clone()).unwrap();
    for scenario in ["corrected", "sibling"] {
        let forcing = mgb_ops::store::read_basin_forcing(
            &path.join("shared/forcing").join(format!("{scenario}.nc")),
        )
        .unwrap();
        assert_eq!(forcing, second);
    }
    shared.output = "conflicting-correction".into();
    shared.corrections[1].as_mut().unwrap()[0].multiplication_factor = 5.;
    assert!(mgb_ops::prepare_typed(shared).is_err());
    assert_eq!(&second.values[..2], &[2., 4.]);
    assert!((second.values[2] - 12.).abs() < 1e-12);
    for file in [
        "checkpoints/observed-grid.nc",
        "checkpoints/forecast/corrected/working.nc",
    ] {
        assert_eq!(
            mgb_ops::store::read_grid_series(&path.join("registered").join(file)).unwrap(),
            mgb_ops::store::read_grid_series(&path.join("supplied").join(file)).unwrap()
        );
    }
    assert_eq!(
        mgb_ops::store::read_grid_frame(
            &path.join("registered/checkpoints/forecast/corrected/0000.nc")
        )
        .unwrap(),
        mgb_ops::store::read_grid_frame(
            &path.join("supplied/checkpoints/forecast/corrected/0000.nc")
        )
        .unwrap()
    );
    for file in [
        "baseline/engine",
        "baseline/Input/MINI.gtp",
        &format!("sources/native/{}.nc", asset.asset_id),
        &format!("sources/original/{}.grib2", asset.asset_id),
    ] {
        assert_eq!(
            fs::read(path.join("registered").join(file)).unwrap(),
            fs::read(path.join("supplied").join(file)).unwrap()
        );
    }
    for file in ["checkpoints/effective.json", "checkpoints/qc.json"] {
        assert_eq!(
            fs::read(path.join("registered").join(file)).unwrap(),
            fs::read(path.join("supplied").join(file)).unwrap()
        );
    }
    let registered: serde_json::Value =
        serde_json::from_slice(&fs::read(path.join("registered/manifest.json")).unwrap()).unwrap();
    let supplied: serde_json::Value =
        serde_json::from_slice(&fs::read(path.join("supplied/manifest.json")).unwrap()).unwrap();
    assert_eq!(
        registered["recipe"]["document"],
        supplied["recipe"]["document"]
    );
    assert_eq!(
        registered["recipe"]["source_identities"],
        supplied["recipe"]["source_identities"]
    );
    assert!(registered["recipe"].get("inline_replacements").is_none());
    assert!(registered["recipe"].get("inline_corrections").is_none());
    assert_eq!(supplied["recipe"]["inline_replacements"], true);
    assert_eq!(
        supplied["recipe"]["inline_corrections"],
        serde_json::json!([0])
    );
    assert!(
        mgb_ops::prepare_typed(typed(
            "duplicate-replacement",
            vec![replacements[0].clone(), replacements[0].clone()],
            corrections.clone()
        ))
        .is_err()
    );
    assert!(
        mgb_ops::prepare_typed(typed(
            "duplicate-correction",
            replacements.clone(),
            vec![corrections[0].clone(), corrections[0].clone()]
        ))
        .is_err()
    );
    let mut wrong_asset = corrections.clone();
    wrong_asset[0].asset_id = "other".into();
    assert!(
        mgb_ops::prepare_typed(typed("cross-asset", replacements.clone(), wrong_asset)).is_err()
    );
    let mut unknown = typed("unknown", replacements.clone(), corrections.clone());
    unknown.replacements = None;
    assert!(mgb_ops::prepare_typed(unknown).is_err());
    let mut invalid_replacement = replacements[0].clone();
    invalid_replacement.variable = "level".into();
    let mut unknown = typed("unknown-correction", replacements, corrections);
    unknown.corrections = vec![None];
    assert!(mgb_ops::prepare_typed(unknown).is_err());
    let mut invalid = typed("invalid", vec![], vec![]);
    invalid.document.scenarios[0].kind = "zero".into();
    invalid.document.scenarios[0].asset_id = None;
    invalid.corrections = vec![None];
    invalid.replacements = Some(vec![invalid_replacement]);
    assert!(mgb_ops::prepare_typed(invalid).is_err());
    for name in [
        "duplicate-replacement",
        "duplicate-correction",
        "conflicting-correction",
        "cross-asset",
        "unknown",
        "unknown-correction",
        "invalid",
    ] {
        assert!(!path.join(name).exists());
    }
    assert_eq!(
        source_obs,
        fs::read(path.join("observations.sqlite")).unwrap()
    );
    assert_eq!(
        source_catalog,
        fs::read(path.join("data/forecasts.sqlite")).unwrap()
    );
    assert!(!path.join("data/runs").exists());
    fs::remove_dir_all(path.join("Input")).unwrap();
    fs::remove_file(path.join("engine")).unwrap();
    fs::remove_file(path.join("observations.sqlite")).unwrap();
    fs::remove_dir_all(path.join("data/forecasts")).unwrap();
    fs::remove_file(path.join("data/forecasts.sqlite")).unwrap();
    let executed = mgb_ops::execute_prepared(mgb_ops::ExecutionRequest {
        workspace: path.clone(),
        invocation_dir: path.clone(),
        prepared: "supplied".into(),
    })
    .unwrap();
    assert!(matches!(executed.status, mgb_ops::BatchStatus::Completed));
    let supplied_result =
        mgb_ops::store::read_result_series(&executed.run_root.join("results/corrected/basin.nc"))
            .unwrap();
    let registered_run = mgb_ops::execute_prepared(mgb_ops::ExecutionRequest {
        workspace: path.clone(),
        invocation_dir: path.clone(),
        prepared: "registered".into(),
    })
    .unwrap();
    let registered_result = mgb_ops::store::read_result_series(
        &registered_run.run_root.join("results/corrected/basin.nc"),
    )
    .unwrap();
    assert_eq!(supplied_result, registered_result);
}

#[test]
fn draft_preview_accepts_selected_content_and_preserves_order() {
    let _guard = TEST_LOCK.lock().unwrap();
    fs::create_dir_all("runs").unwrap();
    let dir = tempfile::tempdir_in("runs").unwrap();
    let root = dir.path().canonicalize().unwrap();
    fixture(&root);
    rusqlite::Connection::open(root.join("observations.sqlite"))
        .unwrap()
        .execute(
            "UPDATE station SET latitude = NULL, longitude = NULL WHERE station_id = 's'",
            [],
        )
        .unwrap();
    let mut draft =
        mgb_ops::Draft::import_toml(root.clone(), &root, Path::new("request.toml")).unwrap();
    assert!(draft.preview_observations().is_err());
    draft.refresh_observations().unwrap();
    let initial = draft.preview_observations().unwrap();
    assert_eq!(initial.rows[0].raw_value, Some(1.0));
    assert_eq!(
        (initial.rows[0].latitude, initial.rows[0].longitude),
        (None, None)
    );
    draft.refresh_observations().unwrap();
    assert!(!draft.preview_is_current(&initial.generation));
    let replacement = |id: &str, value| mgb_ops::DraftReplacement {
        draft_id: id.into(),
        registered_origin: None,
        value: mgb_ops::ObservationReplacement {
            replacement_id: id.into(),
            station_id: "s".into(),
            variable: "rain".into(),
            kind: mgb_ops::ReplacementKind::Point,
            start_at: time("2026-01-01T01:00:00"),
            end_at: None,
            value,
            reason: "test".into(),
        },
    };
    draft.document.instructions.replacements =
        Some(vec![replacement("a", Some(2.0)), replacement("b", None)]);
    assert!(!draft.preview_is_current(&initial.generation));
    let edited = draft.preview_observations().unwrap();
    assert_eq!(edited.rows[0].effective_value, None);
    draft
        .document
        .instructions
        .replacements
        .as_mut()
        .unwrap()
        .reverse();
    let reversed = draft.preview_observations().unwrap();
    assert_eq!(reversed.rows[0].effective_value, Some(2.0));
    draft.save(&root.join("draft.json"), true).unwrap();
    let connection = rusqlite::Connection::open(root.join("observations.sqlite")).unwrap();
    connection
        .execute(
            "INSERT INTO station VALUES ('other', 'other', '2', 'other', 0, 0)",
            [],
        )
        .unwrap();
    assert!(draft.preview_observations().is_ok());
    connection
        .execute("UPDATE observation SET value = 8 WHERE series_id = 'r'", [])
        .unwrap();
    assert!(draft.preview_observations().is_err());
    let reopened = mgb_ops::Draft::open(root.clone(), &root, Path::new("draft.json")).unwrap();
    assert!(reopened.findings.iter().any(|v| v.contains("changed")));
    draft.refresh_observations().unwrap();
    assert_eq!(
        draft.preview_observations().unwrap().rows[0].raw_value,
        Some(8.0)
    );
}

#[test]
fn draft_preparation_requires_refresh_and_freezes_current_inputs() {
    let _guard = TEST_LOCK.lock().unwrap();
    fs::create_dir_all("runs").unwrap();
    let dir = tempfile::tempdir_in("runs").unwrap();
    let root = dir.path().canonicalize().unwrap();
    fixture(&root);
    let mut draft =
        mgb_ops::Draft::import_toml(root.clone(), &root, Path::new("request.toml")).unwrap();
    assert!(draft.prepare(root.clone(), "unaccepted".into()).is_err());
    draft.refresh_inputs().unwrap();
    let original = draft.preview_observations().unwrap();
    let mut invalid = draft.clone();
    invalid.document.schema_version = 3;
    assert!(invalid.preview_observations().is_err());
    assert!(
        invalid
            .validate_preparation()
            .iter()
            .any(|v| v.contains("schema"))
    );
    assert!(invalid.prepare(root.clone(), "bad-version".into()).is_err());
    invalid = draft.clone();
    invalid.document.operation.step_seconds = Some(1800);
    assert!(invalid.preview_observations().is_err());
    invalid.document.operation.step_seconds = None;
    assert!(invalid.preview_observations().is_ok());
    invalid = draft.clone();
    invalid.document.instructions.replacements = Some(vec![mgb_ops::DraftReplacement {
        draft_id: "bad".into(),
        registered_origin: None,
        value: mgb_ops::ObservationReplacement {
            replacement_id: "bad".into(),
            station_id: "s".into(),
            variable: "level".into(),
            kind: mgb_ops::ReplacementKind::Point,
            start_at: time("2026-01-01T01:00:00"),
            end_at: None,
            value: None,
            reason: "test".into(),
        },
    }]);
    assert!(
        invalid
            .validate_preparation()
            .iter()
            .any(|v| v.contains("invalid bounds or variable"))
    );
    assert!(invalid.preview_observations().is_err());
    assert!(
        invalid
            .prepare(root.clone(), "bad-replacement".into())
            .is_err()
    );
    invalid.document.instructions.replacements.as_mut().unwrap()[0]
        .value
        .variable = "rain".into();
    let preview_missing = invalid.preview_observations().unwrap();
    invalid.document.instructions.replacements.as_mut().unwrap()[0]
        .value
        .value = Some(f64::NAN);
    assert!(invalid.generation().is_err());
    assert!(!invalid.preview_is_current(&preview_missing.generation));
    assert!(!invalid.preview_is_current(&original.generation));
    assert!(invalid.preview_observations().is_err());
    assert!(invalid.prepare(root.clone(), "nonfinite".into()).is_err());
    fs::write(root.join("Input/control.txt"), "changed").unwrap();
    assert!(draft.prepare(root.clone(), "changed".into()).is_err());
    assert!(!root.join("changed").exists());
    draft.refresh_inputs().unwrap();
    draft.prepare(root.clone(), "prepared".into()).unwrap();
    assert!(root.join("prepared/manifest.json").exists());
}

#[test]
fn draft_cli_and_api_preview_agree() {
    let _guard = TEST_LOCK.lock().unwrap();
    fs::create_dir_all("runs").unwrap();
    let dir = tempfile::tempdir_in("runs").unwrap();
    let root = dir.path().canonicalize().unwrap();
    fixture(&root);
    let mut draft =
        mgb_ops::Draft::import_toml(root.clone(), &root, Path::new("request.toml")).unwrap();
    draft.refresh_inputs().unwrap();
    draft.save(&root.join("draft.json"), true).unwrap();
    draft.document.visualization = Some(mgb_ops::DraftVisualization {
        map_position: Some([0.0, 0.0, 3.0]),
        ..Default::default()
    });
    draft.save(&root.join("draft.json"), true).unwrap();
    let save = Command::new(env!("CARGO_BIN_EXE_mgb-ops"))
        .current_dir(&root)
        .args([
            "draft",
            "save",
            "--workspace",
            ".",
            "--draft",
            "draft.json",
            "--output",
            "copy.json",
            "--omit-visualization",
            "--json",
        ])
        .output()
        .unwrap();
    assert!(save.status.success());
    let copied = mgb_ops::Draft::open(root.clone(), &root, Path::new("copy.json")).unwrap();
    assert!(copied.document.visualization.is_none());
    let output: serde_json::Value = serde_json::from_slice(&save.stdout).unwrap();
    assert_eq!(
        output["result"],
        serde_json::to_value(copied.document).unwrap()
    );
    let expected = serde_json::to_value(draft.preview_observations().unwrap()).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_mgb-ops"))
        .current_dir(&root)
        .args([
            "draft",
            "preview-observations",
            "--workspace",
            ".",
            "--draft",
            "draft.json",
            "--json",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let result: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(result["ok"], true);
    assert_eq!(result["result"], expected);
    let mut incomplete = mgb_ops::Draft::new(root.clone(), &root);
    incomplete
        .save(&root.join("incomplete.json"), true)
        .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_mgb-ops"))
        .current_dir(&root)
        .args([
            "draft",
            "validate",
            "--workspace",
            ".",
            "--draft",
            "incomplete.json",
            "--json",
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let result: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(result["ok"], false);
    assert!(result["result"].as_array().is_some_and(|v| !v.is_empty()));
}

#[test]
fn incomplete_draft_open_reports_unaccepted_missing_sources_in_api_and_cli() {
    let _guard = TEST_LOCK.lock().unwrap();
    fs::create_dir_all("runs").unwrap();
    let dir = tempfile::tempdir_in("runs").unwrap();
    let root = dir.path().canonicalize().unwrap();
    let mut draft = mgb_ops::Draft::new(root.clone(), &root);
    draft.document.operation.observed = Some(mgb_ops::DraftObserved {
        path: Some("missing.sqlite".into()),
        ..Default::default()
    });
    draft.document.operation.forecasts = Some(mgb_ops::DraftForecasts {
        catalog: Some("forecast.sqlite".into()),
    });
    draft
        .document
        .operation
        .scenarios
        .push(mgb_ops::DraftScenario {
            asset_id: Some("selected".into()),
            ..Default::default()
        });
    draft.document.operation.geometry = Some(mgb_ops::DraftGeometry {
        mini_gtp: Some("missing.gtp".into()),
    });
    draft.document.operation.engine = Some(mgb_ops::DraftEngine {
        executable: Some("missing-engine".into()),
        input_dir: Some("missing-input".into()),
        omp_threads: None,
    });
    draft.save(Path::new("draft.json"), true).unwrap();
    let opened = mgb_ops::Draft::open(".".into(), &root, Path::new("draft.json")).unwrap();
    for name in [
        "observations unavailable",
        "forecast catalog unavailable",
        "geometry unavailable",
        "engine unavailable",
        "engine inputs unavailable",
    ] {
        assert!(
            opened
                .findings
                .iter()
                .any(|finding| finding.starts_with(name)),
            "missing finding for {name}"
        );
    }
    assert!(opened.document.sources.accepted.is_empty());
    assert!(!root.join("missing.sqlite").exists());
    assert!(!root.join("forecast.sqlite").exists());
    let output = Command::new(env!("CARGO_BIN_EXE_mgb-ops"))
        .current_dir(&root)
        .args([
            "draft",
            "open",
            "--workspace",
            ".",
            "--draft",
            "draft.json",
            "--json",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        json["result"]["findings"],
        serde_json::to_value(opened.findings).unwrap()
    );
}

#[test]
fn cancellation_reaps_engines_archives_outcomes_and_releases_lock() {
    use std::time::{Duration, Instant};
    let _guard = TEST_LOCK.lock().unwrap();
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
        let deadline = Instant::now() + Duration::from_secs(15);
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
    let _guard = TEST_LOCK.lock().unwrap();
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
    let _guard = TEST_LOCK.lock().unwrap();
    fs::create_dir_all("runs").unwrap();
    let dir = tempfile::tempdir_in("runs").unwrap();
    let root = dir.path().canonicalize().unwrap();
    fixture(&root);
    fs::write(root.join("Input/control.txt"), "sleep:.history\n").unwrap();
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
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(15);
    let (attempt, pid) = 'waiting: loop {
        if let Ok(entries) = fs::read_dir(root.join("data/cache/run")) {
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
    let _guard = TEST_LOCK.lock().unwrap();
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
        let deadline = Instant::now() + Duration::from_secs(15);
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
fn run_uses_current_draft_and_cleans_temporary_preparation() {
    let _guard = TEST_LOCK.lock().unwrap();
    fs::create_dir_all("runs").unwrap();
    let dir = tempfile::tempdir_in("runs").unwrap();
    let root = dir.path().canonicalize().unwrap();
    fixture(&root);
    let mut draft =
        mgb_ops::Draft::import_toml(root.clone(), &root, Path::new("request.toml")).unwrap();
    draft.refresh_inputs().unwrap();
    let control = mgb_ops::OperationControl::default();
    let first = mgb_ops::run_draft(&draft, &control).unwrap();
    let cache = mgb_ops::read_trial(&first.run_root).unwrap();
    assert!(!root.join("data/cache/preparations").exists());
    assert!(fs::read_dir(root.join("data")).unwrap().all(|entry| {
        !entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".preparation-")
    }));
    assert!(cache.preparation_sha256.is_some());
    let prepared = draft
        .prepare(root.clone(), "separate-prepared".into())
        .unwrap();
    fs::remove_dir_all(root.join("Input")).unwrap();
    fs::remove_file(root.join("engine")).unwrap();
    fs::remove_file(root.join("observations.sqlite")).unwrap();
    draft.document.operation.step_seconds = Some(1800);
    let second = mgb_ops::execute_prepared(mgb_ops::ExecutionRequest {
        workspace: root.clone(),
        invocation_dir: root.clone(),
        prepared: prepared.prepared,
    })
    .unwrap();
    assert_ne!(first.run_root, second.run_root);
    control.cancel();
    assert!(!control.is_cancelled());
    assert!(!first.run_root.exists());
    assert!(second.run_root.join("results/zero/basin.nc").exists());
}

#[test]
fn cancellation_during_finalization_preserves_the_publication_boundary() {
    use std::time::{Duration, Instant};
    let _guard = TEST_LOCK.lock().unwrap();
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
        let deadline = Instant::now() + Duration::from_secs(15);
        loop {
            if control.progress().is_some_and(|progress| {
                matches!(progress.stage, mgb_ops::OperationStage::Finalization)
            }) {
                break;
            }
            assert!(Instant::now() < deadline, "finalization did not start");
            std::thread::yield_now();
        }
        control.cancel();
        let result = worker.join().unwrap();
        match result {
            Err(mgb_ops::OperationError::Cancelled { .. })
            | Err(mgb_ops::OperationError::BatchFailed { .. }) => {
                assert!(mgb_ops::discover_trials(&root).unwrap().is_empty());
                let path = fs::read_dir(root.join("data/run-manifests"))
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
                    [report.run_root.clone()]
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
    let _guard = TEST_LOCK.lock().unwrap();
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
        let deadline = Instant::now() + Duration::from_secs(15);
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

#[cfg(unix)]
#[test]
fn cli_uncontrolled_draft_import_keeps_default_sigint() {
    use std::os::unix::process::ExitStatusExt;
    let _guard = TEST_LOCK.lock().unwrap();
    fs::create_dir_all("runs").unwrap();
    let dir = tempfile::tempdir_in("runs").unwrap();
    let root = dir.path().canonicalize().unwrap();
    let fifo = root.join("request.fifo");
    assert!(
        Command::new("mkfifo")
            .arg(&fifo)
            .status()
            .unwrap()
            .success()
    );
    let mut cli = Command::new(env!("CARGO_BIN_EXE_mgb-ops"))
        .args(["draft", "import-toml", "--workspace"])
        .arg(&root)
        .arg("--request")
        .arg(&fifo)
        .arg("--output")
        .arg(root.join("draft.json"))
        .spawn()
        .unwrap();
    let writer = fs::OpenOptions::new().write(true).open(&fifo).unwrap();
    assert!(
        Command::new("kill")
            .args(["-INT", &cli.id().to_string()])
            .status()
            .unwrap()
            .success()
    );
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    let status = loop {
        if let Some(status) = cli.try_wait().unwrap() {
            break status;
        }
        if std::time::Instant::now() >= deadline {
            cli.kill().unwrap();
            cli.wait().unwrap();
            panic!("SIGINT was ignored");
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    };
    drop(writer);
    assert_eq!(status.signal(), Some(2));
}

#[test]
fn reference_horizons_prepare_execute_and_incomplete_draft_preview() {
    let _guard = TEST_LOCK.lock().unwrap();
    fs::create_dir_all("runs").unwrap();
    let dir = tempfile::tempdir_in("runs").unwrap();
    let root = dir.path().canonicalize().unwrap();
    fixture(&root);
    let mut db = ObservationStore::open_writable(&root.join("observations.sqlite")).unwrap();
    db.write_batch(&ObservationBatch {
        stations: vec![],
        series: vec![],
        replacements: vec![],
        offsets: vec![],
        samples: vec![ObservationSample {
            series_id: "r".into(),
            observed_at: time("2026-01-01T00:00:00"),
            value: Some(ObservationValue::RainMm(1.0)),
        }],
    })
    .unwrap();
    let text = fs::read_to_string(root.join("request.toml")).unwrap();
    let text = text.replace("schema_version = 1", "schema_version = 2")
        .replace("step_seconds = 3600\nhistorical_first_end = \"2026-01-01T01:00:00\"\nhistorical_last_end = \"2026-01-01T02:00:00\"\nforecast_first_end = \"2026-01-01T03:00:00\"\nforecast_last_end = \"2026-01-01T03:00:00\"", "observed_horizon_days = 0\nforecast_horizon_days = 1");
    fs::write(root.join("request.toml"), text).unwrap();
    let mut draft =
        mgb_ops::Draft::import_toml(root.clone(), &root, Path::new("request.toml")).unwrap();
    assert_eq!(draft.document.schema_version, 2);
    assert_eq!(draft.document.operation.observed_horizon_days, Some(0));
    assert!(draft.document.operation.historical_first_end.is_none());
    draft.save(Path::new("draft.json"), true).unwrap();
    let mut local = mgb_ops::Draft::open(root.clone(), &root, Path::new("draft.json")).unwrap();
    local.document.operation.forecast_horizon_days = None;
    local.document.operation.engine = None;
    for horizon in [None, Some(0), Some(u32::MAX)] {
        local.document.operation.forecast_horizon_days = horizon;
        local.refresh_observations().unwrap();
        assert_eq!(local.preview_observations().unwrap().rows.len(), 3);
    }
    draft.refresh_inputs().unwrap();
    draft
        .save_preparation_toml(Path::new("export.toml"))
        .unwrap();
    let exported = fs::read_to_string(root.join("export.toml")).unwrap();
    assert!(exported.contains("observed_horizon_days"));
    assert!(!exported.contains("historical_first_end"));
    let exported_draft =
        mgb_ops::Draft::import_toml(root.clone(), &root, Path::new("export.toml")).unwrap();
    assert_eq!(exported_draft.document.operation, draft.document.operation);
    draft.prepare(root.clone(), "prepared-v2".into()).unwrap();
    let report = mgb_ops::app::execute_prepared(mgb_ops::app::ExecutionRequest {
        workspace: root.clone(),
        invocation_dir: root.clone(),
        prepared: "prepared-v2".into(),
    })
    .unwrap();
    assert!(matches!(
        report.status,
        mgb_ops::app::BatchStatus::Completed
    ));
    assert_eq!(
        mgb_ops::read_trial(&report.run_root)
            .unwrap()
            .scenarios
            .len(),
        4
    );
    let retained = mgb_ops::Draft::import_manifest(root.clone(), &root, &report.run_root).unwrap();
    assert_eq!(
        retained.document.operation.reference_time,
        draft.document.operation.reference_time
    );
    assert_eq!(retained.document.operation.observed_horizon_days, Some(0));
    assert_eq!(retained.document.operation.forecast_horizon_days, Some(1));
}

#[test]
fn legacy_invalid_recipe_remains_importable_for_repair() {
    let _guard = TEST_LOCK.lock().unwrap();
    fs::create_dir_all("runs").unwrap();
    let dir = tempfile::tempdir_in("runs").unwrap();
    let root = dir.path().canonicalize().unwrap();
    fixture(&root);
    let original = fs::read_to_string(root.join("request.toml")).unwrap();
    fs::write(
        root.join("request.toml"),
        original.replace("omp_threads = 1", "omp_threads = 0"),
    )
    .unwrap();
    let draft =
        mgb_ops::Draft::import_toml(root.clone(), &root, Path::new("request.toml")).unwrap();
    assert_eq!(draft.document.schema_version, 1);
    assert_eq!(
        draft
            .document
            .operation
            .engine
            .as_ref()
            .unwrap()
            .omp_threads,
        Some(0)
    );
    assert_eq!(
        draft.document.operation.historical_first_end.as_deref(),
        Some("2026-01-01T01:00:00")
    );
    assert!(!draft.validate_preparation().is_empty());
}

#[test]
fn standalone_operation_accepts_a_saved_recipe_and_runs_without_a_draft() {
    let _guard = TEST_LOCK.lock().unwrap();
    fs::create_dir_all("runs").unwrap();
    let dir = tempfile::tempdir_in("runs").unwrap();
    let workspace = dir.path().canonicalize().unwrap();
    fixture(&workspace);
    fs::write(
        workspace.join("Input/MINI.gtp"),
        "CatID Mini Xcen Ycen MiniJus Area_(km2)\n1 1 0 0 -1 7\n",
    )
    .unwrap();
    let request = workspace.join("request.toml");
    let saved = fs::read(&request).unwrap();
    let now = time("2026-01-02T13:37:00").and_utc();
    let mut operation =
        mgb_ops::app::OperationalRun::open(workspace.clone(), &request, now).unwrap();
    assert_eq!(operation.reference(), "2026-01-02T13:00:00");
    assert_eq!(
        operation.coverage().unwrap(),
        [
            "2026-01-02T12:00:00",
            "2026-01-02T13:00:00",
            "2026-01-02T14:00:00",
            "2026-01-02T14:00:00"
        ]
    );
    operation.set_reference("2026-01-01T02:00:00".into());
    let control = mgb_ops::app::OperationControl::default();
    assert!(operation.run_all(&control).is_err());
    assert!(
        operation
            .validate()
            .iter()
            .any(|finding| finding.contains("not been accepted"))
    );
    assert!(mgb_ops::discover_trials(&workspace).unwrap().is_empty());
    operation.accept_inputs().unwrap();
    assert!(operation.validate().is_empty());

    let report = operation
        .run_all(&mgb_ops::app::OperationControl::default())
        .unwrap();
    assert_eq!(
        mgb_ops::read_trial(&report.run_root)
            .unwrap()
            .scenarios
            .len(),
        4
    );
    assert!(!workspace.join("data/cache/preparations").exists());

    let active = mgb_ops::app::rainfall_cache_info(&workspace)
        .unwrap()
        .unwrap();
    assert_eq!(active.reference, time("2026-01-01T02:00:00"));
    let rainfall = mgb_ops::app::query_cached_rainfall(
        &workspace,
        "zero",
        &mgb_ops::app::RainfallPeriod::SignedHours(1),
        Some(1),
        true,
    )
    .unwrap()
    .unwrap();
    let summary = rainfall.summary.unwrap();
    assert_eq!(summary.total_area_km2, 7.);
    assert_eq!(summary.mean_mm, Some(0.));

    fs::write(workspace.join("Input/control.txt"), "fail:other\n").unwrap();
    operation.accept_inputs().unwrap();
    assert!(
        operation
            .run_all(&mgb_ops::OperationControl::default())
            .is_err()
    );
    assert_eq!(
        mgb_ops::app::rainfall_cache_info(&workspace).unwrap(),
        Some(active.clone())
    );

    operation.set_reference("2026-01-01T01:00:00".into());
    assert!(!operation.has_accepted_inputs());
    assert!(
        operation
            .run_all(&mgb_ops::app::OperationControl::default())
            .is_err()
    );
    assert_eq!(fs::read(&request).unwrap(), saved);
    fs::write(workspace.join("Input/control.txt"), "").unwrap();
    operation.set_reference("2026-01-01T02:00:00".into());
    operation.accept_inputs().unwrap();
    operation
        .run_all(&mgb_ops::OperationControl::default())
        .unwrap();
    let switched = mgb_ops::app::rainfall_cache_info(&workspace)
        .unwrap()
        .unwrap();
    assert_ne!(switched.generation, active.generation);
    assert_eq!(switched.reference, time("2026-01-01T02:00:00"));
    assert!(!workspace.join("data/cache/preparations").exists());
    let after_switch = mgb_ops::app::query_cached_rainfall(
        &workspace,
        "zero",
        &mgb_ops::app::RainfallPeriod::SignedHours(1),
        Some(1),
        true,
    )
    .unwrap()
    .unwrap();
    assert_eq!(after_switch.info, switched);
    assert_eq!(after_switch.summary.unwrap().mean_mm, Some(0.));

    let cache = workspace.join("data/cache/run");
    assert!(!cache.join(active.generation).exists());
    assert_eq!(
        fs::read_dir(&cache)
            .unwrap()
            .filter(|entry| entry
                .as_ref()
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with("generation-"))
            .count(),
        1
    );
    fs::write(
        cache
            .join(&switched.generation)
            .join("rainfall/checkpoints/observed-grid.nc"),
        "invalid",
    )
    .unwrap();
    assert!(
        mgb_ops::app::query_cached_rainfall(
            &workspace,
            "zero",
            &mgb_ops::app::RainfallPeriod::SignedHours(-1),
            Some(1),
            true,
        )
        .is_err()
    );
}

#[test]
fn standalone_day_horizons_recalculate_from_local_midnight() {
    let _guard = TEST_LOCK.lock().unwrap();
    fs::create_dir_all("runs").unwrap();
    let dir = tempfile::tempdir_in("runs").unwrap();
    let workspace = dir.path().canonicalize().unwrap();
    fixture(&workspace);
    let request = workspace.join("request.toml");
    let recipe = fs::read_to_string(&request).unwrap();
    let settings = recipe.split_once("[observed]").unwrap().1;
    fs::write(
        &request,
        format!(
            r#"schema_version = 2
time_zone = "Asia/Tokyo"
reference_time = "2026-01-01T02:00:00"
observed_horizon_days = 1
forecast_horizon_days = 2
[observed]{settings}"#
        ),
    )
    .unwrap();
    let mut operation = mgb_ops::app::OperationalRun::open(
        workspace,
        &request,
        time("2026-01-01T17:46:00").and_utc(),
    )
    .unwrap();
    assert_eq!(operation.time_zone(), "Asia/Tokyo");
    assert_eq!(operation.reference(), "2026-01-02T02:00:00");
    assert_eq!(
        operation.coverage().unwrap(),
        [
            "2026-01-01T00:00:00",
            "2026-01-02T02:00:00",
            "2026-01-02T03:00:00",
            "2026-01-04T03:00:00"
        ]
    );
    operation.set_reference("2025-12-25T01:30:00".into());
    assert!(operation.coverage().is_err());
    operation.set_reference("2025-12-25T01:00:00".into());
    assert_eq!(operation.coverage().unwrap()[0], "2025-12-24T00:00:00");
}

#[test]
fn replay_from_standalone_recipe_requires_identical_sources() {
    let _guard = TEST_LOCK.lock().unwrap();
    fs::create_dir_all("runs").unwrap();
    let temporary = tempfile::tempdir_in("runs").unwrap();
    let workspace = temporary.path().canonicalize().unwrap();
    fixture(&workspace);
    let prepared = mgb_ops::prepare(mgb_ops::PreparationRequest {
        workspace: workspace.clone(),
        invocation_dir: workspace.clone(),
        request_file: "request.toml".into(),
        output: "prepared".into(),
    })
    .unwrap();
    let manifest = workspace.join("recipe.json");
    fs::copy(prepared.prepared.join("manifest.json"), &manifest).unwrap();
    fs::remove_dir_all(&prepared.prepared).unwrap();

    let editable =
        mgb_ops::Draft::import_manifest(workspace.clone(), &workspace, &manifest).unwrap();
    assert_eq!(
        editable.document.operation.reference_time.as_deref(),
        Some("2026-01-01T02:00:00")
    );
    assert!(editable.document.sources.accepted.is_empty());
    let control = mgb_ops::OperationControl::default();
    let first = mgb_ops::replay_run(&workspace, &manifest, &control).unwrap();
    assert!(matches!(first.status, mgb_ops::BatchStatus::Completed));
    assert!(first.run_root.join("results/zero/basin.nc").is_file());
    let id = first
        .run_root
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .strip_prefix("generation-")
        .unwrap();
    let archived = workspace
        .join("data/run-manifests")
        .join(format!("{id}.json"));
    assert!(mgb_ops::Draft::import_manifest(workspace.clone(), &workspace, &archived).is_ok());
    let replayed = mgb_ops::replay_run(&workspace, &archived, &control).unwrap();
    assert_ne!(first.run_root, replayed.run_root);
    assert!(!first.run_root.exists());

    let legacy = workspace.join("legacy-recipe.json");
    let mut legacy_manifest: mgb_ops::store::OperationManifest =
        mgb_ops::store::read_draft(&manifest).unwrap();
    legacy_manifest.inputs.clear();
    legacy_manifest
        .recipe
        .as_object_mut()
        .unwrap()
        .remove("source_identities");
    mgb_ops::store::write_draft(&legacy, &legacy_manifest).unwrap();
    let legacy_replayed = mgb_ops::replay_run(&workspace, &legacy, &control).unwrap();
    assert!(
        legacy_replayed
            .run_root
            .join("results/zero/basin.nc")
            .is_file()
    );

    fs::write(workspace.join("Input/control.txt"), "fail").unwrap();
    let error = mgb_ops::replay_run(&workspace, &archived, &control).unwrap_err();
    assert!(
        format!("{error}").contains("recorded operation recipe or source identities changed"),
        "{error}"
    );
    assert!(legacy_replayed.run_root.exists());
}
