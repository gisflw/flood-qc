use chrono::NaiveDate;
use mgb_ops::store::{
    BasinForcing, BasinGeometry, GridFrame, GridSeries, IntervalAxis, Provenance, RegularGridSpec,
    TimeInterval, read_basin_forcing, read_grid_frame, read_grid_series, write_basin_forcing,
    write_grid_frame, write_grid_series,
};
use terrio::multidim::Array;

fn provenance() -> Provenance {
    Provenance {
        source_ids: vec!["source-a".into()],
        processing: serde_json::Value::Null,
    }
}
fn axis() -> IntervalAxis {
    let start = NaiveDate::from_ymd_opt(2026, 7, 19)
        .unwrap()
        .and_hms_opt(21, 0, 0)
        .unwrap();
    IntervalAxis {
        time_zone: "America/Sao_Paulo".into(),
        bounds: vec![TimeInterval {
            start,
            end: start + chrono::Duration::hours(3),
        }],
    }
}
fn grid() -> RegularGridSpec {
    RegularGridSpec {
        requested_bbox: [-55.0, -30.0, -54.75, -29.75],
        resolution_degrees: 0.25,
        latitudes: vec![-30.0, -29.75],
        longitudes: vec![-55.0, -54.75],
        effective_bbox: [-55.125, -30.125, -54.625, -29.625],
    }
}
fn check(values: &[f64], valid: &[bool]) {
    assert_eq!(valid, [true, true, false, true]);
    assert_eq!(values[0], 0.0);
    assert_eq!(values[1], 1.234567890123);
    assert!(values[2].is_nan());
    assert_eq!(values[3], 2.0);
}
#[test]
fn grid_basin_and_frame_preserve_canonical_values_and_reject_bad_mask() {
    let dir = tempfile::tempdir().unwrap();
    let g = GridSeries {
        timeline: axis(),
        grid: grid(),
        values: vec![0.0, 1.234567890123, f64::NAN, 2.0],
        valid: vec![true, true, false, true],
        provenance: provenance(),
    };
    let file = dir.path().join("grid.nc");
    write_grid_series(&file, &g).unwrap();
    let got = read_grid_series(&file).unwrap();
    assert_eq!(got.grid, g.grid);
    assert_eq!(got.timeline, g.timeline);
    check(&got.values, &got.valid);
    let frame = GridFrame {
        interval: g.timeline.bounds[0].clone(),
        time_zone: g.timeline.time_zone.clone(),
        grid: g.grid.clone(),
        values: g.values.clone(),
        valid: g.valid.clone(),
        provenance: g.provenance.clone(),
    };
    let file = dir.path().join("frame.nc");
    write_grid_frame(&file, &frame).unwrap();
    let got = read_grid_frame(&file).unwrap();
    assert_eq!(got.grid, frame.grid);
    check(&got.values, &got.valid);
    let basins = BasinGeometry {
        mini_ids: vec![9_007_199_254_740_993, 42, 43, 44],
        latitudes: vec![-30.0, -29.9, -29.8, -29.7],
        longitudes: vec![-55.0, -54.9, -54.8, -54.7],
        source: provenance(),
    };
    let basin = BasinForcing {
        timeline: axis(),
        basins,
        values: g.values.clone(),
        valid: g.valid.clone(),
        provenance: provenance(),
    };
    let file = dir.path().join("basin.nc");
    write_basin_forcing(&file, &basin).unwrap();
    let got = read_basin_forcing(&file).unwrap();
    assert_eq!(got.basins, basin.basins);
    check(&got.values, &got.valid);
    let mut ds = terrio::multidim::open(file.to_str().unwrap()).unwrap();
    let idx = ds
        .root
        .arrays
        .iter()
        .position(|a| a.info.name == "valid")
        .unwrap();
    let info = ds.root.arrays[idx].info.clone();
    ds.root.arrays[idx] = Array::from_bytes(info, vec![1, 1, 1, 1]).unwrap();
    let bad = dir.path().join("bad.nc");
    ds.write(bad.to_str().unwrap()).unwrap();
    assert!(read_basin_forcing(&bad).is_err());
}
