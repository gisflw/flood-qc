use chrono::{Duration, NaiveDate};
use mgb_ops::store::{
    BasinGeometry, IntervalAxis, Provenance, ResultSeries, StationResults, TimeInterval,
    read_result_series, read_station_results, write_result_series, write_station_results,
};
use terrio::{DataType, multidim::Array};
fn axis() -> IntervalAxis {
    let reference = NaiveDate::from_ymd_opt(2026, 7, 20)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap();
    IntervalAxis {
        time_zone: "America/Sao_Paulo".into(),
        bounds: vec![
            TimeInterval {
                start: reference - Duration::hours(1),
                end: reference,
            },
            TimeInterval {
                start: reference,
                end: reference + Duration::hours(1),
            },
        ],
    }
}
fn provenance() -> Provenance {
    Provenance {
        source_ids: vec!["retained-inputs".into()],
        processing: serde_json::json!({"reference_time":"2026-07-20T00:00:00","engine_sha256":"a".repeat(64),"snapshot_sha256":"b".repeat(64)}),
    }
}
fn basin() -> ResultSeries {
    ResultSeries {
        timeline: axis(),
        basins: BasinGeometry {
            mini_ids: vec![2, 1],
            longitudes: vec![-50., -51.],
            latitudes: vec![-29., -30.],
            source: Provenance {
                source_ids: vec!["geometry".into()],
                processing: serde_json::Value::Null,
            },
        },
        time_segment: vec![0, 1],
        precipitation: vec![0., 1.234567890123, 0., 2.],
        precipitation_valid: vec![true, true, false, true],
        flow: vec![5., -1., 0., 0.],
        flow_valid: vec![true, true, false, true],
        raw_level_cm: vec![i32::MIN, i32::MAX, 0, -1],
        raw_level_valid: vec![true, true, false, true],
        provenance: provenance(),
    }
}
fn stations() -> StationResults {
    StationResults {
        timeline: axis(),
        time_segment: vec![0, 1],
        station_ids: vec!["shared".into(), "ç\0".into(), "unknown".into()],
        mini_ids: vec![Some(2), Some(2), None],
        offsets_m: vec![Some(0.), Some(-0.609), None],
        level_cm: vec![0, -40, 0, i32::MAX, i32::MIN, 0],
        level_valid: vec![true, true, false, true, true, false],
        provenance: provenance(),
    }
}
#[test]
fn basin_integer_levels_and_float_masks_roundtrip_without_packing() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("basin.nc");
    let expected = basin();
    write_result_series(&path, &expected).unwrap();
    let actual = read_result_series(&path).unwrap();
    assert_eq!(actual.timeline, expected.timeline);
    assert_eq!(actual.basins, expected.basins);
    assert_eq!(actual.time_segment, [0, 1]);
    assert_eq!(actual.raw_level_cm, expected.raw_level_cm);
    assert_eq!(actual.raw_level_valid, expected.raw_level_valid);
    assert_eq!(actual.precipitation[1], 1.234567890123);
    assert!(actual.precipitation[2].is_nan());
    assert_eq!(actual.precipitation_valid, expected.precipitation_valid);
    assert_eq!(actual.flow[1], -1.);
    assert!(actual.flow[2].is_nan());
    assert_eq!(actual.flow[3], 0.);
    assert_eq!(actual.flow_valid, expected.flow_valid);
    let ds = terrio::multidim::open(path.to_str().unwrap()).unwrap();
    assert_eq!(ds.array("raw_level").unwrap().info.data_type, DataType::I32);
    assert_eq!(ds.root.attributes["engine_sha256"], "a".repeat(64));
    assert_eq!(ds.root.attributes["snapshot_sha256"], "b".repeat(64));
    let mut bad = expected.clone();
    bad.raw_level_cm[2] = 1;
    assert!(write_result_series(&dir.path().join("bad.nc"), &bad).is_err());
    let mut ds = ds;
    let index = ds
        .root
        .arrays
        .iter()
        .position(|a| a.info.name == "raw_level")
        .unwrap();
    let info = ds.root.arrays[index].info.clone();
    ds.root.arrays[index] = Array::from_bytes(
        info,
        [i32::MIN, i32::MAX, 1, -1]
            .into_iter()
            .flat_map(i32::to_le_bytes)
            .collect(),
    )
    .unwrap();
    let corrupt = dir.path().join("corrupt.nc");
    ds.write(corrupt.to_str().unwrap()).unwrap();
    assert!(read_result_series(&corrupt).is_err());
}
#[test]
fn station_identity_offsets_and_unknown_mapping_remain_exact() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("station.nc");
    let expected = stations();
    write_station_results(&path, &expected).unwrap();
    assert_eq!(read_station_results(&path).unwrap(), expected);
    let mut ds = terrio::multidim::open(path.to_str().unwrap()).unwrap();
    let index = ds
        .root
        .arrays
        .iter()
        .position(|a| a.info.name == "mini_id")
        .unwrap();
    let info = ds.root.arrays[index].info.clone();
    ds.root.arrays[index] = Array::from_bytes(
        info,
        [2_i64, 2, 2]
            .into_iter()
            .flat_map(i64::to_le_bytes)
            .collect(),
    )
    .unwrap();
    let bad = dir.path().join("bad.nc");
    ds.write(bad.to_str().unwrap()).unwrap();
    assert!(read_station_results(&bad).is_err());
    let mut invalid = expected.clone();
    invalid.level_valid[2] = true;
    assert!(write_station_results(&dir.path().join("invalid.nc"), &invalid).is_err());
    let mut invalid = expected;
    invalid.offsets_m[1] = Some(f64::INFINITY);
    assert!(invalid.validate().is_err());
}

#[test]
fn explicitly_empty_station_selection_roundtrips() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("empty.nc");
    let mut expected = stations();
    expected.station_ids.clear();
    expected.mini_ids.clear();
    expected.offsets_m.clear();
    expected.level_cm.clear();
    expected.level_valid.clear();
    write_station_results(&path, &expected).unwrap();
    assert_eq!(read_station_results(&path).unwrap(), expected);
}
