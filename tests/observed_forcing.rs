use chrono::{Duration, NaiveDateTime};
use mgb_ops::{
    AppliedReplacement, IntervalAxis, ObservationInputs, ObservationReplacement, ObservationSample,
    ObservationSeries, ObservationValue, PrecipitationQcPolicy, ReplacementKind,
    SelectedObservation, Station, TimeInterval, detect_suspect_precipitation,
    effective_observations, select_observation_series,
};
fn time(s: &str) -> NaiveDateTime {
    NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S").unwrap()
}
fn series(id: &str, state: &str) -> ObservationSeries {
    ObservationSeries {
        series_id: id.into(),
        station_id: "s".into(),
        variable: "rain".into(),
        state: state.into(),
        unit: "mm".into(),
        interval_seconds: Some(3600),
        created_at: time("2026-01-01T00:00:00"),
    }
}
#[test]
fn ranking_gaps_and_review_order() {
    let start = time("2026-01-01T00:00:00");
    let axis = IntervalAxis {
        time_zone: "UTC".into(),
        bounds: (0..3)
            .map(|h| TimeInterval {
                start: start + Duration::hours(h),
                end: start + Duration::hours(h + 1),
            })
            .collect(),
    };
    let preferred = series("preferred", "approved");
    let raw = series("raw", "raw");
    assert_eq!(
        select_observation_series(&[raw.clone(), preferred.clone()], &[]).unwrap(),
        vec![preferred.clone()]
    );
    assert!(
        select_observation_series(&[preferred.clone(), series("tie", "approved")], &[]).is_err()
    );
    let replacement = |id: &str, kind, start_at, end_at, value| ObservationReplacement {
        replacement_id: id.into(),
        station_id: "s".into(),
        variable: "rain".into(),
        kind,
        start_at,
        end_at,
        value,
        reason: "review".into(),
    };
    let inputs = ObservationInputs {
        excluded_rain: Default::default(),
        stations: vec![Station {
            station_id: "s".into(),
            provider_code: "ana".into(),
            station_code: "1".into(),
            name: "S".into(),
            latitude: Some(0.0),
            longitude: Some(0.0),
        }],
        series: vec![preferred, raw],
        samples: vec![
            ObservationSample {
                series_id: "raw".into(),
                observed_at: axis.bounds[0].end,
                value: Some(ObservationValue::RainMm(99.0)),
            },
            ObservationSample {
                series_id: "preferred".into(),
                observed_at: axis.bounds[1].end,
                value: Some(ObservationValue::RainMm(0.0)),
            },
        ],
        replacements: vec![
            replacement(
                "period",
                ReplacementKind::Period,
                axis.bounds[0].end,
                Some(axis.bounds[2].end),
                None,
            ),
            replacement(
                "point",
                ReplacementKind::Point,
                axis.bounds[0].end,
                None,
                Some(2.0),
            ),
        ],
    };
    let result = effective_observations(&inputs, &axis).unwrap();
    assert_eq!(result[0].effective_value, Some(2.0));
    assert!(!result[0].raw_present);
    assert_eq!(result[0].applied_instruction_ids, vec!["period", "point"]);
    assert_eq!(result[1].raw_value, Some(0.0));
    assert_eq!(result[1].replacement, AppliedReplacement::Null);
    assert_eq!(result[2].replacement, AppliedReplacement::None);
}
#[test]
fn qc_uses_strict_thresholds_and_consecutive_hours() {
    let start = time("2026-01-01T00:00:00");
    let rows: Vec<_> = (0..6)
        .map(|h| SelectedObservation {
            station_id: "s".into(),
            series_id: "r".into(),
            observed_at: start + Duration::hours(h),
            longitude: Some(0.0),
            latitude: Some(0.0),
            value: Some(if h == 5 { 50.0 } else { 0.5 }),
        })
        .collect();
    let findings = detect_suspect_precipitation(&rows, &PrecipitationQcPolicy::default()).unwrap();
    assert_eq!(findings.len(), 5);
    assert!(findings.iter().all(|f| f.match_type == "sequence"));
    let mut gap = rows;
    gap[2].value = None;
    assert!(
        detect_suspect_precipitation(&gap, &PrecipitationQcPolicy::default())
            .unwrap()
            .is_empty()
    );
}
#[test]
fn preparation_interpolates_zero_and_reports_missing_coverage() {
    use mgb_ops::{
        BasinGeometry, IdwSettings, Provenance, RegularGridSpec, prepare_observed_forcing,
    };
    let start = time("2026-01-01T00:00:00");
    let axis = IntervalAxis {
        time_zone: "UTC".into(),
        bounds: vec![TimeInterval {
            start,
            end: start + Duration::hours(1),
        }],
    };
    let mut inputs = ObservationInputs {
        excluded_rain: Default::default(),
        stations: vec![Station {
            station_id: "s".into(),
            provider_code: "ana".into(),
            station_code: "1".into(),
            name: "S".into(),
            latitude: Some(0.0),
            longitude: Some(0.0),
        }],
        series: vec![series("r", "raw")],
        samples: vec![ObservationSample {
            series_id: "r".into(),
            observed_at: axis.bounds[0].end,
            value: Some(ObservationValue::RainMm(0.0)),
        }],
        replacements: vec![],
    };
    let grid = RegularGridSpec::new([0.0, 0.0, 2.0, 1.0], 1.0).unwrap();
    let basins = BasinGeometry {
        mini_ids: vec![2, 1],
        longitudes: vec![0.0, 1.0],
        latitudes: vec![0.0, 0.0],
        source: Provenance {
            source_ids: vec![],
            processing: serde_json::Value::Null,
        },
    };
    let settings = IdwSettings {
        nearest_stations: 5,
        power: 2.0,
    };
    let out = prepare_observed_forcing(
        &inputs,
        &axis,
        &grid,
        &basins,
        &settings,
        &PrecipitationQcPolicy::default(),
    )
    .unwrap();
    assert_eq!(out.basin.values, vec![0.0, 0.0]);
    assert!(out.working.valid.iter().all(|x| *x));
    inputs.stations[0].latitude = None;
    inputs.stations[0].longitude = None;
    let rows = effective_observations(&inputs, &axis).unwrap();
    assert_eq!((rows[0].latitude, rows[0].longitude), (None, None));
    assert_eq!(rows[0].effective_value, Some(0.));
    assert!(
        prepare_observed_forcing(
            &inputs,
            &axis,
            &grid,
            &basins,
            &settings,
            &PrecipitationQcPolicy::default()
        )
        .is_err()
    );
    inputs.stations[0].latitude = Some(0.);
    assert!(effective_observations(&inputs, &axis).is_err());
    inputs.stations[0].longitude = Some(f64::NAN);
    assert!(effective_observations(&inputs, &axis).is_err());
    inputs.stations[0].longitude = Some(0.);
    inputs.samples.clear();
    assert!(
        prepare_observed_forcing(
            &inputs,
            &axis,
            &grid,
            &basins,
            &settings,
            &PrecipitationQcPolicy::default()
        )
        .is_err()
    );
}
#[test]
fn open_period_overrides_and_invalid_inputs_fail() {
    let start = time("2026-01-01T00:00:00");
    let axis = IntervalAxis {
        time_zone: "UTC".into(),
        bounds: (0..3)
            .map(|h| TimeInterval {
                start: start + Duration::hours(h),
                end: start + Duration::hours(h + 1),
            })
            .collect(),
    };
    let mut inputs = ObservationInputs {
        excluded_rain: Default::default(),
        stations: vec![Station {
            station_id: "s".into(),
            provider_code: "ana".into(),
            station_code: "1".into(),
            name: "S".into(),
            latitude: Some(0.0),
            longitude: Some(0.0),
        }],
        series: vec![series("r", "raw")],
        samples: vec![],
        replacements: vec![
            ObservationReplacement {
                replacement_id: "open".into(),
                station_id: "s".into(),
                variable: "rain".into(),
                kind: ReplacementKind::Period,
                start_at: axis.bounds[0].end,
                end_at: None,
                value: Some(0.0),
                reason: "review".into(),
            },
            ObservationReplacement {
                replacement_id: "point".into(),
                station_id: "s".into(),
                variable: "rain".into(),
                kind: ReplacementKind::Point,
                start_at: axis.bounds[1].end,
                end_at: None,
                value: None,
                reason: "review".into(),
            },
        ],
    };
    let rows = effective_observations(&inputs, &axis).unwrap();
    assert_eq!(
        rows.iter().map(|r| r.effective_value).collect::<Vec<_>>(),
        vec![Some(0.0), None, Some(0.0)]
    );
    let mut numeric = inputs.replacements[1].clone();
    numeric.replacement_id = "numeric".into();
    numeric.value = Some(4.0);
    inputs.replacements.push(numeric);
    let inserted = effective_observations(&inputs, &axis).unwrap();
    assert_eq!(inserted[1].effective_value, Some(4.0));
    assert!(!inserted[1].raw_present);
    inputs.replacements.pop();
    inputs.replacements.push(inputs.replacements[0].clone());
    assert!(effective_observations(&inputs, &axis).is_err());
    inputs.replacements.pop();
    inputs.replacements[0].station_id = "unknown".into();
    assert!(effective_observations(&inputs, &axis).is_err());
    inputs.replacements.clear();
    inputs.samples.push(ObservationSample {
        series_id: "r".into(),
        observed_at: start + Duration::minutes(30),
        value: Some(ObservationValue::RainMm(1.0)),
    });
    assert!(effective_observations(&inputs, &axis).is_err());
    let low = series("raw", "raw");
    let high = series("approved", "approved");
    assert_eq!(
        select_observation_series(&[low.clone(), high], &["raw".into()]).unwrap(),
        vec![low]
    );
    assert!(select_observation_series(&[series("r", "raw")], &["missing".into()]).is_err());
}
#[test]
fn pure_input_rejects_duplicates_orphans_and_incomplete_axes() {
    let start = time("2026-01-01T00:00:00");
    let axis = IntervalAxis {
        time_zone: "UTC".into(),
        bounds: vec![TimeInterval {
            start,
            end: start + Duration::hours(1),
        }],
    };
    let station = Station {
        station_id: "s".into(),
        provider_code: "ana".into(),
        station_code: "1".into(),
        name: "S".into(),
        latitude: Some(0.0),
        longitude: Some(0.0),
    };
    let mut inputs = ObservationInputs {
        excluded_rain: Default::default(),
        stations: vec![station.clone(), station],
        series: vec![series("r", "raw")],
        samples: vec![],
        replacements: vec![],
    };
    assert!(effective_observations(&inputs, &axis).is_err());
    inputs.stations.pop();
    inputs.series.push(inputs.series[0].clone());
    assert!(effective_observations(&inputs, &axis).is_err());
    inputs.series.pop();
    inputs.samples.push(ObservationSample {
        series_id: "orphan".into(),
        observed_at: axis.bounds[0].end,
        value: None,
    });
    assert!(effective_observations(&inputs, &axis).is_err());
    inputs.samples.clear();
    let shifted = IntervalAxis {
        time_zone: "UTC".into(),
        bounds: vec![TimeInterval {
            start: start + Duration::minutes(30),
            end: start + Duration::minutes(90),
        }],
    };
    assert!(effective_observations(&inputs, &shifted).is_err());
    let gap = IntervalAxis {
        time_zone: "UTC".into(),
        bounds: vec![
            axis.bounds[0].clone(),
            TimeInterval {
                start: start + Duration::hours(2),
                end: start + Duration::hours(3),
            },
        ],
    };
    assert!(effective_observations(&inputs, &gap).is_err());
}
#[test]
fn app_selects_one_series_without_orphaning_its_discarded_samples() {
    use mgb_ops::app::{ObservedRequest, prepare_observations};
    use mgb_ops::{
        BasinGeometry, IdwSettings, ObservationBatch, ObservationSelection, ObservationStore,
        Provenance, RegularGridSpec,
    };
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("observations.sqlite");
    let mut store = ObservationStore::create(&path, "UTC").unwrap();
    let start = time("2026-01-01T00:00:00");
    let axis = IntervalAxis {
        time_zone: "UTC".into(),
        bounds: vec![TimeInterval {
            start,
            end: start + Duration::hours(1),
        }],
    };
    store
        .write_batch(&ObservationBatch {
            stations: vec![Station {
                station_id: "s".into(),
                provider_code: "ana".into(),
                station_code: "1".into(),
                name: "S".into(),
                latitude: Some(0.0),
                longitude: Some(0.0),
            }],
            series: vec![series("preferred", "approved"), series("raw", "raw")],
            samples: vec![
                ObservationSample {
                    series_id: "preferred".into(),
                    observed_at: axis.bounds[0].end,
                    value: Some(ObservationValue::RainMm(3.0)),
                },
                ObservationSample {
                    series_id: "raw".into(),
                    observed_at: axis.bounds[0].end,
                    value: Some(ObservationValue::RainMm(99.0)),
                },
            ],
            ..Default::default()
        })
        .unwrap();
    drop(store);
    let selection = ObservationSelection {
        provider_codes: vec!["ana".into()],
        station_ids: vec![],
    };
    let grid = RegularGridSpec::new([0.0, 0.0, 2.0, 1.0], 1.0).unwrap();
    let basins = BasinGeometry {
        mini_ids: vec![1],
        longitudes: vec![0.0],
        latitudes: vec![0.0],
        source: Provenance {
            source_ids: vec![],
            processing: serde_json::Value::Null,
        },
    };
    let interpolation = IdwSettings {
        nearest_stations: 5,
        power: 2.0,
    };
    let qc = PrecipitationQcPolicy::default();
    let request = |ids| ObservedRequest {
        path: &path,
        selection: &selection,
        series_ids: ids,
        replacement_ids: &[],
        timeline: &axis,
        grid: &grid,
        basins: &basins,
        interpolation: &interpolation,
        qc: &qc,
    };
    assert!(
        (prepare_observations(&request(&[]))
            .unwrap()
            .preparation
            .basin
            .values[0]
            - 3.0)
            .abs()
            < 1e-12
    );
    assert!(
        (prepare_observations(&request(&["raw".into()]))
            .unwrap()
            .preparation
            .basin
            .values[0]
            - 99.0)
            .abs()
            < 1e-12
    );
}
