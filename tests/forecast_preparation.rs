use chrono::NaiveDate;
use mgb_ops::{
    analysis::prepare_zero_forecast,
    store::{BasinGeometry, IntervalAxis, Provenance, TimeInterval},
};
#[test]
fn zero_forecast_uses_requested_axis_and_basin_order() {
    let start = NaiveDate::from_ymd_opt(2026, 7, 20)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap();
    let timeline = IntervalAxis {
        time_zone: "UTC".into(),
        bounds: vec![
            TimeInterval {
                start,
                end: start + chrono::Duration::hours(1),
            },
            TimeInterval {
                start: start + chrono::Duration::hours(1),
                end: start + chrono::Duration::hours(2),
            },
        ],
    };
    let basins = BasinGeometry {
        mini_ids: vec![2, 1],
        longitudes: vec![0.0, 1.0],
        latitudes: vec![0.0, 1.0],
        source: Provenance {
            source_ids: vec![],
            processing: serde_json::Value::Null,
        },
    };
    let zero = prepare_zero_forecast(&timeline, &basins).unwrap();
    assert_eq!(zero.timeline, timeline);
    assert_eq!(zero.basins.mini_ids, [2, 1]);
    assert_eq!(zero.values, [0.0; 4]);
    assert_eq!(zero.valid, [true; 4]);
    assert!(zero.provenance.source_ids.is_empty());
}

#[test]
fn ordered_corrections_precede_hourly_split_and_idw() {
    use mgb_ops::{
        analysis::{ForecastPreparationRequest, prepare_forecast},
        store::{ForecastCorrection, GridSeries, IdwSettings, RegularGridSpec},
    };
    let start = NaiveDate::from_ymd_opt(2026, 7, 20)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap();
    let native_axis = IntervalAxis {
        time_zone: "UTC".into(),
        bounds: vec![TimeInterval {
            start,
            end: start + chrono::Duration::hours(3),
        }],
    };
    let grid = RegularGridSpec {
        requested_bbox: [0., 0., 2., 2.],
        resolution_degrees: 1.,
        latitudes: vec![0.5, 1.5],
        longitudes: vec![0.5, 1.5],
        effective_bbox: [0., 0., 2., 2.],
    };
    let mut processing = serde_json::Map::new();
    processing.insert(
        "normalization".into(),
        serde_json::json!({"cycle_utc":"2026-07-20T00:00:00Z"}),
    );
    processing.insert("asset_id".into(), "asset".into());
    let provenance = Provenance {
        source_ids: vec!["source".into()],
        processing: processing.into(),
    };
    let native = GridSeries {
        timeline: native_axis,
        grid: grid.clone(),
        values: vec![6., 12., 6., 12.],
        valid: vec![true; 4],
        provenance: provenance.clone(),
    };
    let mut bounds = Vec::new();
    for i in 0..3 {
        bounds.push(TimeInterval {
            start: start + chrono::Duration::hours(i),
            end: start + chrono::Duration::hours(i + 1),
        });
    }
    let basins = BasinGeometry {
        mini_ids: vec![9],
        longitudes: vec![1.],
        latitudes: vec![1.],
        source: provenance,
    };
    let request = ForecastPreparationRequest {
        timeline: IntervalAxis {
            time_zone: "UTC".into(),
            bounds,
        },
        working_grid: grid,
        basins,
        interpolation: IdwSettings {
            nearest_stations: 4,
            power: 2.,
        },
    };
    let instruction = |id: &str, factor| ForecastCorrection {
        correction_id: id.into(),
        asset_id: "asset".into(),
        t0_hour: 0,
        t1_hour: 3,
        shift_lat_pixels: 0.,
        shift_lon_pixels: 0.,
        rotation_deg: 0.,
        multiplication_factor: factor,
        editor: None,
        reason: "adjust".into(),
        created_at: "2026-07-20T00:00:00Z".into(),
    };
    let result = prepare_forecast(
        &native,
        &[instruction("a", 2.), instruction("b", 3.)],
        &request,
    )
    .unwrap();
    assert_eq!(result.correction_coverage[0].native_interval_indices, [0]);
    assert_eq!(
        result.working.values,
        [12., 24., 12., 24., 12., 24., 12., 24., 12., 24., 12., 24.]
    );
    assert!(result.basin.values.iter().all(|v| (v - 18.).abs() < 1e-12));
    let mut partial = instruction("partial", 3.);
    partial.t0_hour = 1;
    let result = prepare_forecast(&native, &[partial], &request).unwrap();
    assert!(
        result.correction_coverage[0]
            .native_interval_indices
            .is_empty()
    );
    assert_eq!(result.working.values[0], 2.);
    let mut overlap = native.clone();
    overlap.timeline.bounds.push(TimeInterval {
        start: start + chrono::Duration::hours(1),
        end: start + chrono::Duration::hours(4),
    });
    overlap.values.extend([6., 12., 6., 12.]);
    overlap.valid.extend([true; 4]);
    let err = prepare_forecast(&overlap, &[], &request).unwrap_err();
    let ambiguous = matches!(
        err,
        mgb_ops::analysis::ForecastPreparationError::AmbiguousInterval(_)
    );
    assert!(ambiguous);
    let mut gap = request.clone();
    gap.timeline.bounds[2] = TimeInterval {
        start: start + chrono::Duration::hours(3),
        end: start + chrono::Duration::hours(4),
    };
    let err = prepare_forecast(&native, &[], &gap).unwrap_err();
    let missing = matches!(
        err,
        mgb_ops::analysis::ForecastPreparationError::MissingTimeCoverage(_)
    );
    assert!(missing);
    let mut missing_cell = native.clone();
    missing_cell.valid[0] = false;
    missing_cell.values[0] = f64::NAN;
    let err = prepare_forecast(&missing_cell, &[], &request).unwrap_err();
    let spatial = matches!(
        err,
        mgb_ops::analysis::ForecastPreparationError::MissingSpatialCoverage(_)
    );
    assert!(spatial);
}
