//! Regional mask qualification is separate from engine execution: frozen engine
//! depths are the inputs, so host-dependent engine drift cannot hide mask checks.
//! Keep this executable free of child launches while native NetCDF files are open.
use chrono::{Duration, NaiveDateTime};
use mgb_ops::app::{
    InundationProcessRequest, InundationProcessStatus, OperationControl, process_inundation,
};
use mgb_ops::store::{InundationLevel, InundationMask, digest_file};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

const LEVELS: [(&str, InundationLevel); 3] = [
    ("inundation_mask", InundationLevel::Full),
    ("inundation_mask_4x", InundationLevel::Overview4),
    ("inundation_mask_16x", InundationLevel::Overview16),
];

#[derive(Deserialize)]
struct Expected {
    schema_version: u32,
    tile_size: usize,
    oracle: Oracle,
    grid: Grid,
    timeline: Timeline,
    sources: BTreeMap<String, String>,
    scenarios: Vec<Scenario>,
}

#[derive(Deserialize)]
struct Oracle {
    script_sha256: String,
}

#[derive(Deserialize)]
struct Grid {
    width: usize,
    height: usize,
    epsg: i32,
    transform: [f64; 6],
}

#[derive(Deserialize)]
struct Timeline {
    time_zone: String,
    first_end: NaiveDateTime,
    historical_count: usize,
    forecast_count: usize,
}

#[derive(Deserialize)]
struct Scenario {
    id: String,
    sha256: BTreeMap<String, Vec<String>>,
}

fn selected_path(name: &str) -> PathBuf {
    PathBuf::from(
        std::env::var_os(name)
            .unwrap_or_else(|| panic!("{name} must explicitly select the RS input")),
    )
    .canonicalize()
    .unwrap()
}

fn assert_identity(path: &Path, expected: &str) {
    assert_eq!(
        digest_file(path).unwrap().0,
        expected,
        "input identity: {}",
        path.display()
    );
}

fn cell_depths(path: &Path) -> (Vec<i64>, Vec<f32>) {
    let text = fs::read_to_string(path).unwrap();
    let mut lines = text.lines();
    let header: Vec<_> = lines.next().unwrap().split_whitespace().collect();
    assert_eq!((header[1], header[15]), ("Mini", "Depth"));
    let (ids, depths): (Vec<_>, Vec<_>) = lines
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            let fields: Vec<_> = line.split_whitespace().collect();
            (
                fields[1].parse::<i64>().unwrap(),
                fields[15].parse::<f32>().unwrap(),
            )
        })
        .unzip();
    assert_eq!(ids.len(), 8649);
    assert_eq!(
        ids.iter().collect::<std::collections::BTreeSet<_>>().len(),
        ids.len()
    );
    assert!(ids.iter().all(|id| *id > 0));
    assert!(depths.iter().all(|value| value.is_finite() && *value > 0.));
    (ids, depths)
}

fn raw_depths(path: &Path, basins: usize, rows: usize) -> Vec<f32> {
    let bytes = fs::read(path).unwrap();
    assert_eq!(bytes.len(), basins * 241 * size_of::<f32>());
    bytes[..basins * rows * size_of::<f32>()]
        .as_chunks::<4>()
        .0
        .iter()
        .map(|bytes| {
            let value = f32::from_le_bytes(*bytes);
            assert!(value.is_finite() && value >= 0.);
            value
        })
        .collect()
}

fn flood_source(
    path: &Path,
    scenario: &str,
    expected: &Expected,
    ids: &[i64],
    channel: &[f32],
    depths: &[f32],
) {
    let timeline = &expected.timeline;
    let times = timeline.historical_count + timeline.forecast_count;
    assert_eq!(depths.len(), times * ids.len());
    let mut file = netcdf::create(path).unwrap();
    file.add_attribute("schema_version", 1_i32).unwrap();
    file.add_attribute("historical_count", timeline.historical_count as i64)
        .unwrap();
    let provenance = serde_json::json!({"source_ids":["saved-rs-handtudo"], "processing":{"flood_source":{"scenario":scenario,"depth_datum":"model_channel_bed"}}}).to_string();
    for (name, value) in [
        ("artifact_kind", "flood_source"),
        ("units", "m"),
        ("operation_timezone", timeline.time_zone.as_str()),
        ("source_id", "saved-rs-handtudo"),
        ("crs", "EPSG:4326"),
        ("processing_json", provenance.as_str()),
    ] {
        file.add_attribute(name, value).unwrap();
    }
    for (name, size) in [("time", times), ("bounds", 2), ("basin", ids.len())] {
        file.add_dimension(name, size).unwrap();
    }
    let origin = timeline.first_end - Duration::hours(1);
    let units = format!("seconds since {}", origin.format("%Y-%m-%d %H:%M:%S"));
    let mut time = file.add_variable::<i64>("time", &["time"]).unwrap();
    for (name, value) in [
        ("units", units.as_str()),
        ("calendar", "proleptic_gregorian"),
        ("bounds", "time_bounds"),
    ] {
        time.put_attribute(name, value).unwrap();
    }
    time.put_values(
        &(1..=times as i64).map(|i| i * 3600).collect::<Vec<_>>(),
        ..,
    )
    .unwrap();
    let bounds: Vec<_> = (0..times as i64)
        .flat_map(|i| [i * 3600, (i + 1) * 3600])
        .collect();
    file.add_variable::<i64>("time_bounds", &["time", "bounds"])
        .unwrap()
        .put_values(&bounds, ..)
        .unwrap();
    file.add_variable::<i64>("mini_id", &["basin"])
        .unwrap()
        .put_values(ids, ..)
        .unwrap();
    for (name, dimensions, values) in [
        ("depth", &["time", "basin"][..], depths),
        ("channel_depth", &["basin"][..], channel),
    ] {
        let mut variable = file.add_variable::<f32>(name, dimensions).unwrap();
        variable.put_attribute("units", "m").unwrap();
        variable.put_values(values, ..).unwrap();
    }
    file.close().unwrap();
}

fn compare_mask(path: &Path, expected: &Expected, scenario: &Scenario) {
    let mask = InundationMask::open(path).unwrap();
    let metadata = mask.metadata();
    let grid = &expected.grid;
    let times = expected.timeline.historical_count + expected.timeline.forecast_count;
    assert_eq!(
        (metadata.width, metadata.height, metadata.epsg),
        (grid.width, grid.height, grid.epsg)
    );
    assert_eq!(
        [
            metadata.origin_x,
            metadata.pixel_width,
            0.,
            metadata.origin_y,
            0.,
            metadata.pixel_height
        ],
        grid.transform
    );
    assert_eq!(metadata.timeline.time_zone, expected.timeline.time_zone);
    assert_eq!(
        metadata.historical_count,
        expected.timeline.historical_count
    );
    assert_eq!(metadata.timeline.bounds.len(), times);
    for (index, interval) in metadata.timeline.bounds.iter().enumerate() {
        assert_eq!(
            interval.end,
            expected.timeline.first_end + Duration::hours(index as i64)
        );
        assert_eq!(interval.start, interval.end - Duration::hours(1));
    }
    assert_eq!(metadata.provenance.scenario, scenario.id);
    assert_eq!(metadata.provenance.hand_sha256, expected.sources["hand"]);
    assert_eq!(
        metadata.provenance.catchments_sha256,
        expected.sources["catchments"]
    );
    assert_eq!(
        metadata.provenance.flood_source_sha256,
        digest_file(&path.with_file_name("flood_source.nc"))
            .unwrap()
            .0
    );
    assert_eq!(
        metadata.provenance.run_manifest_sha256,
        digest_file(
            &path
                .parent()
                .unwrap()
                .parent()
                .unwrap()
                .parent()
                .unwrap()
                .join("manifest.toml")
        )
        .unwrap()
        .0
    );
    assert!(metadata.provenance.hand_in_decimetres && metadata.provenance.include_nonpositive_hand);
    assert_eq!(metadata.provenance.vertical_datum, "Z0");
    assert_eq!(metadata.provenance.hand_vertical_shift_m, 0);
    assert!(metadata.provenance.hand_sidecars.is_empty());
    assert!(metadata.provenance.catchments_sidecars.is_empty());
    let file = netcdf::open(path).unwrap();
    for (name, level) in LEVELS {
        let reference = &scenario.sha256[name];
        assert_eq!(reference.len(), times);
        let variable = file.variable(name).unwrap();
        let (width, height) = metadata.dimensions(level);
        assert_eq!(
            variable
                .dimensions()
                .iter()
                .map(|dim| dim.len())
                .collect::<Vec<_>>(),
            [times, height, width]
        );
        let mut hashes = vec![Sha256::new(); times];
        let span = expected.tile_size / level.factor();
        let mut pixels = Vec::new();
        let mut checked_rust_reader = false;
        for y in (0..height).step_by(span) {
            let rows = span.min(height - y);
            for x in (0..width).step_by(span) {
                let columns = span.min(width - x);
                pixels.resize(rows * columns, 0_u8);
                for (time, hash) in hashes.iter_mut().enumerate() {
                    variable
                        .get_values_into(&mut pixels, ([time, y, x], [1, rows, columns]))
                        .unwrap();
                    hash.update(&pixels);
                    if !checked_rust_reader && pixels.iter().any(|pixel| *pixel != 255) {
                        assert_eq!(
                            mask.read_window(level, time, x, y, columns, rows).unwrap(),
                            pixels
                        );
                        checked_rust_reader = true;
                    }
                }
            }
        }
        assert!(checked_rust_reader, "{name} has no covered pixels");
        for (time, (hash, reference)) in hashes.into_iter().zip(reference).enumerate() {
            assert_eq!(
                format!("{:x}", hash.finalize()),
                *reference,
                "{} {name} time {time}",
                scenario.id
            );
        }
        eprintln!("RS mask {}: all {times} {name} frames match", scenario.id);
    }
    file.close().unwrap();
}

#[test]
#[ignore = "requires explicitly selected frozen RS streams and regional GeoTIFFs; reads every pixel at all 362 times"]
fn saved_rs_inundation_masks() {
    let repository = Path::new(env!("CARGO_MANIFEST_DIR"));
    let expected: Expected = serde_json::from_slice(
        &fs::read(repository.join("references/regression/rs-hourly/inundation-expected.json"))
            .unwrap(),
    )
    .unwrap();
    assert_eq!(expected.schema_version, 1);
    assert_eq!(expected.tile_size, 1024);
    assert_eq!(expected.timeline.historical_count, 241);
    assert_eq!(expected.timeline.forecast_count, 121);
    assert_eq!(
        (
            expected.grid.width,
            expected.grid.height,
            expected.grid.epsg
        ),
        (12589, 10610, 4618)
    );
    assert_eq!(expected.timeline.time_zone, "America/Sao_Paulo");
    assert_eq!(
        expected.timeline.first_end.to_string(),
        "2026-07-10 00:00:00"
    );
    assert_eq!(
        expected
            .scenarios
            .iter()
            .map(|s| s.id.as_str())
            .collect::<Vec<_>>(),
        ["zero", "ecmwf-raw", "noaa-raw", "ecmwf-corrected"]
    );
    assert_identity(
        &repository.join("tests/support/capture_rs_inundation.py"),
        &expected.oracle.script_sha256,
    );
    let reference = selected_path("MGB_RS_REFERENCE");
    let hand = selected_path("MGB_RS_HAND");
    let catchments = selected_path("MGB_RS_CATCHMENTS");
    assert_identity(&hand, &expected.sources["hand"]);
    assert_identity(&catchments, &expected.sources["catchments"]);
    let cell_path = reference.join("frozen/runner/Input/Cell.hig");
    assert_identity(&cell_path, &expected.sources["cell"]);
    let (ids, channel) = cell_depths(&cell_path);
    let history_path = reference.join("capture/runs/history/Output/HANDTUDO.MGB");
    assert_identity(&history_path, &expected.sources["history"]);
    let history = raw_depths(&history_path, ids.len(), 241);
    fs::create_dir_all("runs").unwrap();
    let directory = tempfile::Builder::new()
        .prefix("rs-mask-")
        .tempdir_in("runs")
        .unwrap();
    let run = directory.path().canonicalize().unwrap();
    let manifest = serde_json::json!({"state":"completed","recipe":{},"inputs":[],"scenarios":expected.scenarios.iter().map(|s| serde_json::json!({"id":s.id,"state":"completed"})).collect::<Vec<_>>(),"created_at":"2026-07-20T00:00:00-03:00"});
    mgb_ops::store::write_draft(&run.join("manifest.toml"), &manifest).unwrap();
    for scenario in &expected.scenarios {
        let raw_path = reference
            .join("capture/runs")
            .join(&scenario.id)
            .join("Output/HANDTUDO.MGB");
        assert_identity(&raw_path, &expected.sources[&scenario.id]);
        let mut depths = history.clone();
        depths.extend(raw_depths(&raw_path, ids.len(), 121));
        let results = run.join("results").join(&scenario.id);
        fs::create_dir_all(&results).unwrap();
        flood_source(
            &results.join("flood_source.nc"),
            &scenario.id,
            &expected,
            &ids,
            &channel,
            &depths,
        );
        eprintln!("RS mask {}: generating from frozen depths", scenario.id);
        let report = process_inundation(
            InundationProcessRequest {
                run: run.clone(),
                scenario: scenario.id.clone(),
                hand: hand.clone(),
                catchments: catchments.clone(),
                replace: false,
            },
            &OperationControl::default(),
        )
        .unwrap();
        assert_eq!(report.status, InundationProcessStatus::Created);
        eprintln!("RS mask {}: checking all decoded pixels", scenario.id);
        compare_mask(&report.path, &expected, scenario);
        fs::remove_file(report.path).unwrap();
    }
}
