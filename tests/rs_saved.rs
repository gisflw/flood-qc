use mgb_ops::store::{ObservationStore, copy_tree};
use mgb_ops::{
    BatchStatus, ExecutionRequest, NormalizeForecastRequest, PreparationRequest, execute_prepared,
    normalize_forecast, prepare,
};
use std::{
    fs,
    path::{Path, PathBuf},
};

// Full-resolution and overview mask expectations are exercised separately in
// rs_inundation.rs, against frozen depths and an independent raster oracle.
// Keeping that check independent preserves coverage when engine rerun bytes drift.
#[test]
#[ignore = "requires the explicit consolidated RS reference and real engine"]
fn saved_rs_full_workflow() {
    let reference = PathBuf::from(
        std::env::var_os("MGB_RS_REFERENCE")
            .expect("MGB_RS_REFERENCE must name the consolidated saved reference"),
    );
    for path in [
        "case.json",
        "frozen/runner/Input/MINI.gtp",
        "frozen/runner/MGB_Inercial_Prev_RSH",
        "frozen/downloads/ecmwf/ifs_fc_20260720T000000Z.grib2",
        "frozen/downloads/noaa/gfs_fc_20260720T000000Z.grib2",
        "capture/runs/zero/Input/CHUVABIN.hig",
    ] {
        assert!(
            reference.join(path).is_file(),
            "missing required RS prerequisite: {path}"
        );
    }
    fs::create_dir_all("runs").unwrap();
    let dir = tempfile::tempdir_in("runs").unwrap();
    let root = dir.keep().canonicalize().unwrap();
    eprintln!("RS qualification artifacts: {}", root.display());
    let observations = PathBuf::from(
        std::env::var_os("MGB_RS_OBSERVATIONS")
            .expect("MGB_RS_OBSERVATIONS must name an exported canonical observation set"),
    );
    assert!(
        observations.is_dir(),
        "MGB_RS_OBSERVATIONS must be a directory"
    );
    copy_tree(&observations, &root.join("observations")).unwrap();
    let store = ObservationStore::open_read_only(&root.join("observations")).unwrap();
    let station_ids: Vec<_> = store
        .station_locations()
        .unwrap()
        .into_iter()
        .map(|station| station.station.station_id)
        .collect();
    let mut station_ids: Vec<_> = store
        .read_station_offsets(&station_ids)
        .unwrap()
        .into_iter()
        .map(|offset| offset.station_id)
        .collect();
    station_ids.sort();
    assert!(!station_ids.is_empty());
    drop(store);
    copy_tree(&reference.join("frozen/runner/Input"), &root.join("Input")).unwrap();
    copy_tree(
        &reference.join("frozen/runner/MGB_Inercial_Prev_RSH"),
        &root.join("engine"),
    )
    .unwrap();
    let mut assets = Vec::new();
    for (provider, model, source) in [
        (
            "ecmwf",
            "ifs",
            "frozen/downloads/ecmwf/ifs_fc_20260720T000000Z.grib2",
        ),
        (
            "noaa",
            "gfs",
            "frozen/downloads/noaa/gfs_fc_20260720T000000Z.grib2",
        ),
    ] {
        let source_path = root.join(format!("{provider}.grib2"));
        fs::copy(reference.join(source), &source_path).unwrap();
        let text = format!(
            "schema_version = 1\nprovider = \"{provider}\"\nmodel = \"{model}\"\nproduct = \"fc\"\ncycle_utc = \"2026-07-20T00:00:00Z\"\noperation_timezone = \"America/Sao_Paulo\"\nsource = \"{}\"\ncrop_bbox = [-79.13530,-50.61209,-28.67245,-10.08729]\ncatalog = \"forecast-assets\"\nartifact_root = \"forecast-assets\"\n",
            source_path.display()
        );
        let path = root.join(format!("normalize-{provider}.toml"));
        fs::write(&path, text).unwrap();
        assets.push(
            normalize_forecast(NormalizeForecastRequest {
                workspace: root.clone(),
                invocation_dir: root.clone(),
                request_file: path,
            })
            .unwrap()
            .asset_id,
        );
    }
    let stations = station_ids
        .iter()
        .map(|id| format!("\"{id}\""))
        .collect::<Vec<_>>()
        .join(",");
    let request = format!(
        r#"schema_version = 1
time_zone = "America/Sao_Paulo"
reference_time = "2026-07-20T00:00:00"
step_seconds = 3600
historical_first_end = "2026-07-10T00:00:00"
historical_last_end = "2026-07-20T00:00:00"
forecast_first_end = "2026-07-20T01:00:00"
forecast_last_end = "2026-07-25T01:00:00"
[observed]
path = "observations"
provider_codes = ["ana"]
station_ids = []
series_ids = []
replacement_ids = ["rs-null-1","rs-null-2","rs-null-3","rs-null-4","rs-null-5","rs-null-6"]
[geometry]
mini_gtp = "Input/MINI.gtp"
[grid]
bbox = [-58.95016,-34.40217,-48.85759,-26.29721]
resolution_degrees = 0.1
[interpolation]
nearest_stations = 5
power = 2.0
[qc]
high_threshold_mm = 50.0
sequence_lower_mm = 0.0
sequence_upper_mm = 1.0
sequence_min_length = 5
[levels]
station_ids = [{stations}]
[engine]
executable = "engine"
input_dir = "Input"
omp_threads = 1
[forecasts]
catalog = "forecast-assets"
[[scenarios]]
id = "zero"
[[scenarios]]
id = "ecmwf-raw"
asset_id = "{}"
[[scenarios]]
id = "noaa-raw"
asset_id = "{}"
"#,
        assets[0], assets[1]
    );
    fs::write(root.join("request.toml"), request).unwrap();
    prepare(PreparationRequest {
        workspace: root.clone(),
        invocation_dir: root.clone(),
        request_file: "request.toml".into(),
        output: "prepared".into(),
    })
    .unwrap();
    let manifest_path = root.join("prepared/manifest.toml");
    let original_manifest = fs::read(&manifest_path).unwrap();
    let mut altered: serde_json::Value = mgb_ops::store::read_draft(&manifest_path).unwrap();
    altered["recipe"]["document"]["reference_time"] = "2026-07-21T00:00:00".into();
    mgb_ops::store::write_draft(&manifest_path, &altered).unwrap();
    assert!(matches!(
        execute_prepared(ExecutionRequest {
            workspace: root.clone(),
            invocation_dir: root.clone(),
            prepared: "prepared".into()
        }),
        Err(mgb_ops::OperationError::InvalidPrepared { .. })
    ));
    fs::write(&manifest_path, original_manifest).unwrap();
    fs::remove_dir_all(root.join("Input")).unwrap();
    fs::remove_file(root.join("engine")).unwrap();
    fs::remove_dir_all(root.join("observations")).unwrap();
    fs::remove_dir_all(root.join("forecast-assets")).unwrap();
    fs::remove_file(root.join("ecmwf.grib2")).unwrap();
    fs::remove_file(root.join("noaa.grib2")).unwrap();
    let run = |workspace| {
        execute_prepared(ExecutionRequest {
            workspace,
            invocation_dir: root.clone(),
            prepared: "prepared".into(),
        })
        .unwrap()
    };
    let first = run(root.clone());
    assert!(
        matches!(first.status, BatchStatus::Completed),
        "{:?}",
        first.scenarios
    );
    let repeat_workspace = root.join("repeat");
    fs::create_dir(&repeat_workspace).unwrap();
    let second = run(repeat_workspace);
    assert!(
        matches!(second.status, BatchStatus::Completed),
        "{:?}",
        second.scenarios
    );
    let history_hand = fs::read(reference.join("capture/runs/history/Output/HANDTUDO.MGB"))
        .expect("the frozen history flood stream");
    assert_eq!(history_hand.len(), 241 * 8649 * size_of::<f32>());
    for scenario in ["zero", "ecmwf-raw", "noaa-raw"] {
        for (file, headers) in [
            (
                "PARHIG.hig",
                &[&["DIA", "MES", "ANO", "HORA"][..], &["NT", "DT"][..]][..],
            ),
            ("Previsao.meta", &[&["HORIZ(NT)"][..]][..]),
        ] {
            compare_metadata(
                &reference.join("frozen/runner/Input").join(file),
                &reference
                    .join("capture/runs")
                    .join(scenario)
                    .join("Input")
                    .join(file),
                &first
                    .run_root
                    .join("results")
                    .join(scenario)
                    .join("engine-inputs")
                    .join(file),
                headers,
            );
        }
        {
            use sha2::{Digest, Sha256};
            let expected = fs::read(
                reference
                    .join("capture/runs")
                    .join(scenario)
                    .join("Input/CHUVABIN.hig"),
            )
            .unwrap();
            let hash = format!("{:x}", Sha256::digest(expected));
            for trial in [&first, &second] {
                assert_eq!(
                    fs::read_to_string(
                        trial
                            .run_root
                            .join("results")
                            .join(scenario)
                            .join("engine-inputs/CHUVABIN.sha256")
                    )
                    .unwrap(),
                    hash
                );
            }
        }
        for file in [
            "Output/QTUDO_Inercial_Atual.MGB",
            "Output/QTUDO_Inercial_Prev.MGB",
            "Output/YTUDO.MGB",
            "Output/YTUDO_prev.MGB",
        ] {
            let expected =
                fs::read(reference.join("capture/runs").join(scenario).join(file)).unwrap();
            assert!(
                fs::read(first.run_root.join("scenarios").join(scenario).join(file)).unwrap()
                    == expected,
                "{scenario}/{file}"
            );
            assert!(
                fs::read(second.run_root.join("scenarios").join(scenario).join(file)).unwrap()
                    == expected,
                "repeat {scenario}/{file}"
            );
        }
        let generated_basin = first
            .run_root
            .join("results")
            .join(scenario)
            .join("basin.nc");
        let generated_station = first
            .run_root
            .join("results")
            .join(scenario)
            .join("station.nc");
        let basin = mgb_ops::store::read_result_series(&generated_basin).unwrap();
        let station = mgb_ops::store::read_station_results(&generated_station).unwrap();
        assert_eq!(station.station_ids, station_ids);
        let first_time = chrono::NaiveDate::from_ymd_opt(2026, 7, 10)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        assert_eq!(basin.timeline.bounds.len(), 362);
        assert_eq!(station.timeline, basin.timeline);
        for (index, interval) in basin.timeline.bounds.iter().enumerate() {
            assert_eq!(
                interval.end,
                first_time + chrono::Duration::hours(index as i64)
            );
            assert_eq!(interval.start, interval.end - chrono::Duration::hours(1));
        }
        compare_arrays(
            &generated_basin,
            &reference
                .join("capture/checkpoints/results")
                .join(format!("{scenario}-basin.nc")),
            &[
                "mini_id",
                "basin_latitude",
                "basin_longitude",
                "time_segment",
                "precipitation",
                "precipitation_valid",
                "flow",
                "flow_valid",
                "raw_level",
                "raw_level_valid",
            ],
        );
        compare_arrays(
            &generated_station,
            &reference
                .join("capture/checkpoints/results")
                .join(format!("{scenario}-station.nc")),
            &[
                "mini_id",
                "mini_valid",
                "offset_m",
                "offset_valid",
                "level",
                "level_valid",
                "time_segment",
            ],
        );
        for file in ["basin.nc", "station.nc"] {
            assert!(
                fs::read(first.run_root.join("results").join(scenario).join(file)).unwrap()
                    == fs::read(second.run_root.join("results").join(scenario).join(file)).unwrap(),
                "repeat {scenario}/{file}"
            );
        }
        let scenario_hand = fs::read(
            reference
                .join("capture/runs")
                .join(scenario)
                .join("Output/HANDTUDO.MGB"),
        )
        .expect("the frozen scenario flood stream");
        let forecast_hand = &scenario_hand[..121 * 8649 * size_of::<f32>()];
        for trial in [&first, &second] {
            let path = trial
                .run_root
                .join("results")
                .join(scenario)
                .join("flood_source.nc");
            let flood = terrio::multidim::open(path.to_str().unwrap()).unwrap_or_else(|error| {
                let diagnostic =
                    fs::read_to_string(path.with_file_name("flood_source_unavailable.txt"));
                panic!(
                    "could not open {}: {error}; diagnostic {diagnostic:?}",
                    path.display()
                );
            });
            assert_eq!(flood.root.attributes["historical_count"], 241);
            let depth = flood.array("depth").unwrap();
            assert_eq!(depth.shape(), [362, 8649]);
            let bytes = depth.read().unwrap().bytes;
            assert_eq!(
                &bytes[..history_hand.len()],
                history_hand,
                "{scenario} history"
            );
            assert_eq!(
                &bytes[history_hand.len()..],
                forecast_hand,
                "{scenario} forecast"
            );
        }
    }
}

fn compare_arrays(actual: &Path, expected: &Path, names: &[&str]) {
    let actual = terrio::multidim::open(actual.to_str().unwrap()).unwrap();
    let expected = terrio::multidim::open(expected.to_str().unwrap()).unwrap();
    for name in names {
        let actual = actual.array(name).unwrap();
        let expected = expected.array(name).unwrap();
        assert_eq!(
            actual.info.data_type, expected.info.data_type,
            "{name} type"
        );
        let actual_shape: Vec<_> = actual.info.dimensions.iter().map(|dim| dim.size).collect();
        let expected_shape: Vec<_> = expected
            .info
            .dimensions
            .iter()
            .map(|dim| dim.size)
            .collect();
        assert_eq!(actual_shape, expected_shape, "{name} shape");
        assert!(
            actual.read().unwrap().bytes == expected.read().unwrap().bytes,
            "{name} values"
        );
    }
}

fn compare_metadata(baseline: &Path, expected: &Path, actual: &Path, headers: &[&[&str]]) {
    let base = fs::read(baseline).unwrap();
    let expected = fs::read(expected).unwrap();
    let actual = fs::read(actual).unwrap();
    let lines = |data: &[u8]| {
        data.split_inclusive(|byte| *byte == b'\n')
            .map(<[u8]>::to_vec)
            .collect::<Vec<_>>()
    };
    let base = lines(&base);
    let expected = lines(&expected);
    let actual = lines(&actual);
    assert_eq!(base.len(), actual.len());
    let mut changed = Vec::new();
    for tokens in headers {
        let find = |lines: &[Vec<u8>]| {
            let indices: Vec<_> = lines
                .iter()
                .enumerate()
                .filter(|(_, line)| {
                    tokens.iter().all(|token| {
                        line.windows(token.len())
                            .any(|word| word.eq_ignore_ascii_case(token.as_bytes()))
                    })
                })
                .map(|(index, _)| index)
                .collect();
            assert_eq!(indices.len(), 1);
            (indices[0] + 1..lines.len())
                .find(|index| {
                    let line = lines[*index].trim_ascii();
                    !line.is_empty() && line[0] != b'!'
                })
                .unwrap()
        };
        let actual_index = find(&actual);
        let expected_index = find(&expected);
        assert_eq!(actual_index, expected_index);
        let numbers = |line: &[u8]| {
            std::str::from_utf8(line)
                .unwrap()
                .split_whitespace()
                .map(|value| value.parse::<f64>().unwrap())
                .collect::<Vec<_>>()
        };
        assert_eq!(
            numbers(&actual[actual_index]),
            numbers(&expected[expected_index])
        );
        changed.push(actual_index);
    }
    for index in 0..base.len() {
        if !changed.contains(&index) {
            assert_eq!(
                base[index], actual[index],
                "changed unrelated metadata line {index}"
            );
        }
    }
}
