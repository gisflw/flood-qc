use chrono::{Duration, NaiveDate};
use mgb_ops::analysis::{bilinear_resample, interpolate_grid_to_basins, interpolate_stations};
use mgb_ops::{
    BasinGeometry, GridSeries, IdwSettings, IntervalAxis, Provenance, RegularGridSpec, TimeInterval,
};
fn provenance() -> Provenance {
    Provenance {
        source_ids: vec!["fixture".into()],
        processing: serde_json::json!({}),
    }
}
fn axis(zone: &str) -> IntervalAxis {
    let start = NaiveDate::from_ymd_opt(2026, 1, 1)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap();
    IntervalAxis {
        time_zone: zone.into(),
        bounds: vec![TimeInterval {
            start,
            end: start + Duration::hours(1),
        }],
    }
}
#[test]
fn closed_footprints_station_coincidences_and_ties() {
    let grid = RegularGridSpec::new([0., 0., 2., 1.], 1.).unwrap();
    assert_eq!(grid.longitudes, [-0.5, 0.5, 1.5, 2.5]);
    assert_eq!(grid.latitudes, [-0.5, 0.5, 1.5]);
    assert_eq!(grid.effective_bbox, [-1., -1., 3., 2.]);
    let settings = IdwSettings {
        nearest_stations: 2,
        power: 2.,
    };
    let ids = vec!["b".into(), "a".into()];
    assert_eq!(
        interpolate_stations(
            &ids,
            &[0., 2.],
            &[0., 0.],
            &[2., 6.],
            &[1., 0.],
            &[0., 0.],
            &settings
        )
        .unwrap(),
        [4., 2.]
    );
    assert_eq!(
        interpolate_stations(
            &ids,
            &[0., 0.],
            &[0., 0.],
            &[2., 6.],
            &[0.],
            &[0.],
            &settings
        )
        .unwrap(),
        [4.]
    );
    let k1 = IdwSettings {
        nearest_stations: 1,
        power: 2.,
    };
    assert_eq!(
        interpolate_stations(&ids, &[0., 2.], &[0., 0.], &[2., 6.], &[1.], &[0.], &k1).unwrap(),
        [6.]
    );
    assert!(interpolate_stations(&[], &[], &[], &[], &[0.], &[0.], &settings).is_err());
}
#[test]
fn basin_kernel_renormalizes_missing_neighbors_in_basin_order() {
    let grid = RegularGridSpec {
        requested_bbox: [0., 0., 2., 1.],
        resolution_degrees: 2.,
        latitudes: vec![0.],
        longitudes: vec![0., 2.],
        effective_bbox: [-1., -1., 3., 1.],
    };
    let basins = BasinGeometry {
        mini_ids: vec![99, 1],
        longitudes: vec![1., 0.],
        latitudes: vec![0., 0.],
        source: provenance(),
    };
    let mut series = GridSeries {
        timeline: axis("UTC"),
        grid,
        values: vec![2., 6.],
        valid: vec![true, true],
        provenance: provenance(),
    };
    let settings = IdwSettings {
        nearest_stations: 2,
        power: 2.,
    };
    let forcing = interpolate_grid_to_basins(&series, &basins, &settings).unwrap();
    assert_eq!(forcing.basins.mini_ids, [99, 1]);
    assert_eq!(forcing.values[0], 4.);
    series.valid[1] = false;
    assert_eq!(
        interpolate_grid_to_basins(&series, &basins, &settings)
            .unwrap()
            .values,
        [2., 2.]
    );
    series.valid[0] = false;
    assert!(interpolate_grid_to_basins(&series, &basins, &settings).is_err());
}
#[test]
fn bilinear_missing_and_exact_endpoint_rules() {
    let (values, valid) = bilinear_resample(
        &[0., 2., 2., 4.],
        &[true; 4],
        &[0., 1.],
        &[0., 1.],
        &[0.5],
        &[-0.1, 0.5],
    )
    .unwrap();
    assert_eq!(valid, [false, true]);
    assert_eq!(values[1], 2.);
    let (values, valid) = bilinear_resample(
        &[0., 2., 2., 4.],
        &[true, false, false, false],
        &[0., 1.],
        &[0., 1.],
        &[0.],
        &[0.],
    )
    .unwrap();
    assert_eq!(valid, [true]);
    assert_eq!(values, [0.]);
    assert!(bilinear_resample(&[0.; 4], &[true; 4], &[1., 0.], &[0., 1.], &[0.], &[0.]).is_err());
}
#[test]
fn axes_reject_timezone_transitions_and_shape_errors() {
    assert!(axis("America/Sao_Paulo").validate().is_ok());
    let start = NaiveDate::from_ymd_opt(2026, 3, 8)
        .unwrap()
        .and_hms_opt(1, 0, 0)
        .unwrap();
    assert!(
        IntervalAxis {
            time_zone: "America/New_York".into(),
            bounds: vec![TimeInterval {
                start,
                end: start + Duration::hours(3)
            }]
        }
        .validate()
        .is_err()
    );
    let mut grid = RegularGridSpec::new([0., 0., 2., 1.], 1.).unwrap();
    grid.longitudes[1] = f64::NAN;
    assert!(grid.validate().is_err());
    let mut intervals = axis("UTC");
    intervals.bounds.push(intervals.bounds[0].clone());
    assert!(intervals.validate().is_err());
}

#[test]
fn power_two_matches_direct_square_reference() {
    let ids = (0..5).map(|i| i.to_string()).collect::<Vec<_>>();
    let actual = interpolate_stations(
        &ids,
        &[-50.4253, -50.7989, -51.3992, -49.7786, -51.3133],
        &[-29.9094, -28.9653, -28.5981, -27.8028, -27.7381],
        &[0.0, 0.0, 0.42, 0.0, 0.0],
        &[-49.80015999999999],
        &[-30.752169999999996],
        &IdwSettings {
            nearest_stations: 5,
            power: 2.0,
        },
    )
    .unwrap();
    assert_eq!(actual[0].to_bits(), 0x3fa41233aa68e652);
}
