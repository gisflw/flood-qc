//! Keep native NetCDF lifecycle tests in their own executable. Subprocesses
//! spawned by unrelated unit tests can inherit HDF5 descriptors and retain file
//! locks after the writer closes. This test executable must not spawn children.
use chrono::Timelike;
use mgb_ops::app::{
    InundationProcessRequest, InundationProcessStatus, OperationControl, process_inundation,
    process_workspace_inundation,
};
use mgb_ops::store::{FLOOD_MASK_FILE, InundationLevel, InundationMask, digest_file};
use std::{fs, path::Path};
use terrio::{DataType, Raster, RasterConfig, RasterWriteOptions, raster::RasterData};

const WIDTH: usize = 2055;
const HEIGHT: usize = 5;
const FLOOD_SOURCE_FILE: &str = "flood_source.nc";
const FLOOD_SOURCE_DIAGNOSTIC: &str = "flood_source_unavailable.txt";

fn flood_source(path: &Path) {
    let mut file = netcdf::create(path).unwrap();
    file.add_attribute("schema_version", 1_i32).unwrap();
    file.add_attribute("historical_count", 1_i64).unwrap();
    for (name, value) in [
        ("artifact_kind", "flood_source"),
        ("units", "m"),
        ("operation_timezone", "America/Sao_Paulo"),
        ("source_id", "inundation-test"),
        ("crs", "EPSG:4326"),
        (
            "processing_json",
            r#"{"source_ids":["inundation-test"],"processing":{"flood_source":{"scenario":"reference","depth_datum":"model_channel_bed"}}}"#,
        ),
    ] {
        file.add_attribute(name, value).unwrap();
    }
    for (name, size) in [("time", 4), ("bounds", 2), ("basin", 1)] {
        file.add_dimension(name, size).unwrap();
    }
    let mut time = file.add_variable::<i64>("time", &["time"]).unwrap();
    for (name, value) in [
        ("units", "seconds since 2025-03-05 10:00:00"),
        ("calendar", "proleptic_gregorian"),
        ("bounds", "time_bounds"),
    ] {
        time.put_attribute(name, value).unwrap();
    }
    time.put_values(&[3600_i64, 7200, 10800, 14400], ..)
        .unwrap();
    file.add_variable::<i64>("time_bounds", &["time", "bounds"])
        .unwrap()
        .put_values(&[0_i64, 3600, 3600, 7200, 7200, 10800, 10800, 14400], ..)
        .unwrap();
    file.add_variable::<i64>("mini_id", &["basin"])
        .unwrap()
        .put_values(&[1_i64], ..)
        .unwrap();
    for (name, dimensions, values) in [
        ("depth", &["time", "basin"][..], &[3_f32, 4., 4., 3.][..]),
        ("channel_depth", &["basin"][..], &[2_f32][..]),
    ] {
        let mut variable = file.add_variable::<f32>(name, dimensions).unwrap();
        variable.put_attribute("units", "m").unwrap();
        variable.put_values(values, ..).unwrap();
    }
    file.close().unwrap();
}

fn raster(path: &Path, x_min: f64, epsg: Option<u32>, nodata: f64, data: RasterData) {
    let data_type = match data {
        RasterData::F32(_) => DataType::F32,
        RasterData::I32(_) => DataType::I32,
        _ => panic!("unexpected fixture data type"),
    };
    let raster = Raster::from_data_native(
        RasterConfig {
            cols: WIDTH,
            rows: HEIGHT,
            bands: 1,
            x_min,
            y_min: -30.,
            cell_size: 0.001,
            cell_size_y: Some(0.001),
            data_type,
            nodata,
            crs: epsg
                .map(|code| {
                    terrio::crs::SpatialRef::from_epsg(code)
                        .unwrap()
                        .to_crs_info()
                })
                .unwrap_or_default(),
            ..Default::default()
        },
        data,
    )
    .unwrap();
    terrio::write_raster(
        &raster,
        path.to_str().unwrap(),
        &RasterWriteOptions::default(),
    )
    .unwrap();
}

fn hand(path: &Path, x_min: f64, epsg: Option<u32>) {
    let mut data = vec![0.; WIDTH * HEIGHT];
    data[..6].copy_from_slice(&[10., 5., -9999., 0., 10., -10.]);
    data[WIDTH - 1] = 10.;
    raster(path, x_min, epsg, -9999., RasterData::F32(data));
}

fn catchments(path: &Path, last_id: i32) {
    let mut data = vec![0; WIDTH * HEIGHT];
    data[..6].copy_from_slice(&[1, 1, 1, 1, -9999, 1]);
    data[WIDTH - 1] = last_id;
    raster(path, -51., Some(4618), -9999., RasterData::I32(data));
}

fn fixture(dir: &Path) -> InundationProcessRequest {
    let scenario_dir = dir.join("results/reference");
    fs::create_dir_all(&scenario_dir).unwrap();
    let manifest = serde_json::json!({"state":"completed","recipe":{},"inputs":[],"scenarios":[{"id":"reference","state":"completed"}],"created_at":"2025-03-05T10:00:00-03:00"});
    mgb_ops::store::write_draft(&dir.join("manifest.toml"), &manifest).unwrap();
    flood_source(&scenario_dir.join(FLOOD_SOURCE_FILE));
    let request = InundationProcessRequest {
        run: dir.into(),
        scenario: "reference".into(),
        hand: dir.join("hand.tif"),
        catchments: dir.join("catchments.tif"),
        replace: false,
    };
    hand(&request.hand, -51., Some(4618));
    catchments(&request.catchments, 1);
    request
}

#[test]
fn processes_and_reuses_a_compressed_scenario_mask() {
    fs::create_dir_all("runs").unwrap();
    let dir = tempfile::tempdir_in("runs").unwrap();
    let request = fixture(dir.path());
    let control = OperationControl::default();
    let report = process_inundation(request.clone(), &control).unwrap();
    assert_eq!(report.status, InundationProcessStatus::Created);
    assert_eq!((report.historical_count, report.forecast_count), (1, 3));
    let mask = InundationMask::open(&report.path).unwrap();
    assert_eq!(mask.metadata().epsg, 4618);
    assert_eq!(mask.metadata().timeline.time_zone, "America/Sao_Paulo");
    assert_eq!(mask.metadata().timeline.bounds[0].start.hour(), 10);
    assert_eq!(mask.metadata().provenance.scenario, "reference");
    assert_eq!(mask.metadata().provenance.vertical_datum, "Z0");
    assert_eq!(mask.metadata().provenance.hand_vertical_shift_m, 0);
    assert!(mask.metadata().provenance.hand_in_decimetres);
    assert!(mask.metadata().provenance.include_nonpositive_hand);
    assert_eq!(
        mask.metadata().provenance.hand_sha256,
        digest_file(&request.hand).unwrap().0
    );
    assert_eq!(
        mask.read_window(InundationLevel::Full, 0, 0, 0, 6, 1)
            .unwrap(),
        [0, 1, 255, 1, 255, 1],
    );
    assert_eq!(
        mask.read_window(InundationLevel::Overview4, 0, 0, 0, 2, 1)
            .unwrap(),
        [13, 6],
    );
    for (level, expected) in [
        (InundationLevel::Full, [0, 1, 1, 0]),
        (InundationLevel::Overview4, [0, 8, 8, 0]),
        (InundationLevel::Overview16, [0, 3, 3, 0]),
    ] {
        let x = (WIDTH - 1) / level.factor();
        for (time, value) in expected.into_iter().enumerate() {
            assert_eq!(mask.read_window(level, time, x, 0, 1, 1).unwrap(), [value]);
        }
    }
    assert!(
        mask.read_window(InundationLevel::Full, 0, WIDTH, 0, 1, 1)
            .is_err()
    );
    assert!(
        mask.read_window(InundationLevel::Full, 4, 0, 0, 1, 1)
            .is_err()
    );
    let file = terrio::multidim::open(report.path.to_str().unwrap()).unwrap();
    let mask_variable = file.array("inundation_mask").unwrap();
    assert_eq!(
        mask_variable.info.chunks.as_deref(),
        Some(&[1, HEIGHT, 256][..])
    );
    assert_eq!(
        mask.read_window(InundationLevel::Full, 1, 1024, 0, 4, 1)
            .unwrap(),
        [255; 4],
    );
    let original = digest_file(&report.path).unwrap();
    assert_eq!(
        process_inundation(request.clone(), &control)
            .unwrap()
            .status,
        InundationProcessStatus::Reused
    );
    assert_eq!(digest_file(&report.path).unwrap(), original);

    drop(mask_variable);
    drop(file);
    drop(mask);
    let mut corrupted = netcdf::append(&report.path).unwrap();
    corrupted
        .variable_mut("inundation_mask")
        .unwrap()
        .put_value(2_u8, [1, 0, 1])
        .unwrap();
    corrupted.close().unwrap();
    let refused = process_inundation(request, &control).unwrap_err();
    assert!(refused.to_string().contains("--replace"));
}

#[test]
fn rejects_invalid_rasters_and_unavailable_source_without_publishing() {
    fs::create_dir_all("runs").unwrap();
    let dir = tempfile::tempdir_in("runs").unwrap();
    let request = fixture(dir.path());
    let control = OperationControl::default();
    let scenario = dir.path().join("results/reference");
    for (shift, epsg, error) in [
        (0.000001, Some(4618), "same pixel grid"),
        (0., Some(4326), "same pixel grid"),
        (0., None, "north-up geographic grid"),
    ] {
        hand(&request.hand, -51. + shift, epsg);
        let result = process_inundation(request.clone(), &control).unwrap_err();
        assert!(result.to_string().contains(error), "{result}");
        assert!(scenario.join(FLOOD_MASK_FILE).symlink_metadata().is_err());
    }
    fs::remove_file(scenario.join(FLOOD_SOURCE_FILE)).unwrap();
    fs::write(
        scenario.join(FLOOD_SOURCE_DIAGNOSTIC),
        "HANDTUDO.FLB missing",
    )
    .unwrap();
    let result = process_inundation(request, &control).unwrap_err();
    assert!(result.to_string().contains("HANDTUDO.FLB missing"));
    assert!(scenario.join(FLOOD_MASK_FILE).symlink_metadata().is_err());
}

#[test]
fn preserves_existing_mask_on_conflict_failure_and_cancellation() {
    fs::create_dir_all("runs").unwrap();
    let dir = tempfile::tempdir_in("runs").unwrap();
    let mut request = fixture(dir.path());
    let control = OperationControl::default();
    let report = process_inundation(request.clone(), &control).unwrap();
    let original = digest_file(&report.path).unwrap();
    catchments(&request.catchments, 24);
    assert!(
        process_inundation(request.clone(), &control)
            .unwrap_err()
            .to_string()
            .contains("--replace")
    );
    request.replace = true;
    assert!(
        process_inundation(request.clone(), &control)
            .unwrap_err()
            .to_string()
            .contains("unknown catchment MINI ID 24")
    );
    assert_eq!(digest_file(&report.path).unwrap(), original);

    catchments(&request.catchments, 0);
    let cancelled = OperationControl::default();
    cancelled.cancel();
    assert!(
        process_inundation(request.clone(), &cancelled)
            .unwrap_err()
            .to_string()
            .contains("cancelled")
    );
    assert_eq!(digest_file(&report.path).unwrap(), original);
    assert_eq!(
        process_inundation(request, &control).unwrap().status,
        InundationProcessStatus::Replaced
    );
    assert_eq!(
        InundationMask::open(&report.path)
            .unwrap()
            .read_window(InundationLevel::Full, 1, WIDTH - 1, 0, 1, 1)
            .unwrap(),
        [255]
    );
}

#[test]
fn workspace_inundation_derives_and_reuses_aligned_catchments() {
    fs::create_dir_all("runs").unwrap();
    let dir = tempfile::tempdir_in("runs").unwrap();
    let root = dir.path();
    let request = fixture(root);
    hand(&request.hand, -51., Some(4326));
    fs::write(root.join("mini_catchments.geojson"), r#"{
       "type":"FeatureCollection", "name":"mini_catchments",
       "features":[{"type":"Feature","properties":{"mini_id":1},
       "geometry":{"type":"Polygon","coordinates":[[[-52,-31],[-48,-31],[-48,-29],[-52,-29],[-52,-31]]]}}]
    }"#).unwrap();
    fs::write(
        root.join("config.toml"),
        "[spatial]\nmini_layers='mini_catchments.geojson'\nhand_raster='hand.tif'\n",
    )
    .unwrap();
    let process = || {
        process_workspace_inundation(
            root,
            Some(root.into()),
            "reference".into(),
            false,
            &OperationControl::default(),
        )
    };
    let first = process().unwrap();
    assert_eq!(first.status, InundationProcessStatus::Created);
    let mask = InundationMask::open(&first.path).unwrap();
    assert_eq!(
        mask.read_window(InundationLevel::Full, 0, 0, 0, 6, 1)
            .unwrap(),
        [0, 1, 255, 1, 0, 1]
    );
    assert_eq!(process().unwrap().status, InundationProcessStatus::Reused);
    assert_eq!(fs::read_dir(root.join("cache/spatial")).unwrap().count(), 1);
}
