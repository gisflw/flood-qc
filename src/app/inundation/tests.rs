use super::{
    InundationProcessRequest, InundationProcessStatus, OperationControl, process_inundation,
};
use crate::store::{
    FLOOD_MASK_FILE, FLOOD_SOURCE_DIAGNOSTIC, FLOOD_SOURCE_FILE, FloodSource, IntervalAxis,
    InundationLevel, InundationMask, Provenance, TimeInterval, digest_file, write_flood_source,
};
use chrono::{Duration, NaiveDate, Timelike};
use std::{fs, path::Path};
use terrio::{DataType, Raster, RasterConfig, RasterWriteOptions, raster::RasterData};

const WIDTH: usize = 2055;
const HEIGHT: usize = 5;

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
    data[..5].copy_from_slice(&[1., 0.5, -9999., 0., 1.]);
    data[WIDTH - 1] = 1.;
    raster(path, x_min, epsg, -9999., RasterData::F32(data));
}

fn catchments(path: &Path, last_id: i32) {
    let mut data = vec![0; WIDTH * HEIGHT];
    data[..5].copy_from_slice(&[1, 1, 1, 1, -9999]);
    data[WIDTH - 1] = last_id;
    raster(path, -51., Some(4618), -9999., RasterData::I32(data));
}

fn fixture(dir: &Path) -> InundationProcessRequest {
    let scenario_dir = dir.join("results/reference");
    fs::create_dir_all(&scenario_dir).unwrap();
    fs::write(
        dir.join("manifest.json"),
        r#"{
            "schema_version": 1, "state": "completed", "recipe": {}, "inputs": [],
            "scenarios": [{"id": "reference", "state": "completed"}],
            "created_at": "2025-03-05T10:00:00-03:00"
        }"#,
    )
    .unwrap();
    let origin = NaiveDate::from_ymd_opt(2025, 3, 5)
        .unwrap()
        .and_hms_opt(10, 0, 0)
        .unwrap();
    let source = FloodSource {
        timeline: IntervalAxis {
            time_zone: "America/Sao_Paulo".into(),
            bounds: (0..2)
                .map(|t| TimeInterval {
                    start: origin + Duration::hours(t),
                    end: origin + Duration::hours(t + 1),
                })
                .collect(),
        },
        mini_ids: vec![1],
        depth_m: vec![3., 4.],
        channel_depth_m: vec![2.],
        historical_count: 1,
        provenance: Provenance {
            source_ids: vec![],
            processing: serde_json::from_str(
                r#"{"flood_source": {"scenario": "reference", "depth_datum": "model_channel_bed"}}"#,
            ).unwrap(),
        },
    };
    write_flood_source(&scenario_dir.join(FLOOD_SOURCE_FILE), &source).unwrap();
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
    assert_eq!((report.historical_count, report.forecast_count), (1, 1));
    let mask = InundationMask::open(&report.path).unwrap();
    assert_eq!(mask.metadata().epsg, 4618);
    assert_eq!(mask.metadata().timeline.time_zone, "America/Sao_Paulo");
    assert_eq!(mask.metadata().timeline.bounds[0].start.hour(), 10);
    assert_eq!(mask.metadata().provenance.scenario, "reference");
    assert_eq!(mask.metadata().provenance.vertical_datum, "Z0");
    assert_eq!(mask.metadata().provenance.hand_vertical_shift_m, 0);
    assert_eq!(
        mask.metadata().provenance.hand_sha256,
        digest_file(&request.hand).unwrap().0
    );
    assert_eq!(
        mask.read_window(InundationLevel::Full, 0, 0, 0, 5, 1)
            .unwrap(),
        [0, 1, 255, 255, 255],
    );
    assert_eq!(
        mask.read_window(InundationLevel::Full, 1, WIDTH - 1, 0, 1, 1)
            .unwrap(),
        [1],
    );
    assert_eq!(
        mask.read_window(InundationLevel::Overview4, 0, 0, 0, 2, 1)
            .unwrap(),
        [6, 255],
    );
    assert_eq!(
        mask.read_window(InundationLevel::Overview4, 1, (WIDTH - 1) / 4, 0, 1, 1)
            .unwrap(),
        [8],
    );
    assert_eq!(
        mask.read_window(InundationLevel::Overview16, 1, (WIDTH - 1) / 16, 0, 1, 1)
            .unwrap(),
        [3],
    );
    assert!(
        mask.read_window(InundationLevel::Full, 0, WIDTH, 0, 1, 1)
            .is_err()
    );
    assert!(
        mask.read_window(InundationLevel::Full, 2, 0, 0, 1, 1)
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
        process_inundation(request, &control).unwrap().status,
        InundationProcessStatus::Reused
    );
    assert_eq!(digest_file(&report.path).unwrap(), original);
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
