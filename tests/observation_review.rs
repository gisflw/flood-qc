use chrono::{Duration, NaiveDateTime};
use mgb_ops::analysis::{AvailabilityState, ReviewVariable};
use mgb_ops::app::{
    ObservationReviewRequest, ReviewWindow, review_observations, spatial_observation_qc,
};
use mgb_ops::store::{
    IdwSettings, ObservationBatch, ObservationReplacement, ObservationSample, ObservationSelection,
    ObservationSeries, ObservationStore, ObservationValue, ReplacementKind, Station,
};

fn time(s: &str) -> NaiveDateTime {
    s.parse().unwrap()
}

fn station(id: &str, longitude: Option<f64>) -> Station {
    Station {
        station_id: id.into(),
        provider_code: "provider".into(),
        station_code: id.into(),
        name: id.into(),
        latitude: longitude.map(|_| 0.0),
        longitude,
    }
}

fn series(
    station: &str,
    variable: &str,
    unit: &str,
    interval_seconds: Option<i64>,
) -> ObservationSeries {
    ObservationSeries {
        series_id: format!("{station}:{variable}"),
        station_id: station.into(),
        variable: variable.into(),
        state: "approved".into(),
        unit: unit.into(),
        interval_seconds,
        created_at: time("2026-02-01T00:00:00"),
    }
}

fn sample(series: &str, at: &str, value: Option<ObservationValue>) -> ObservationSample {
    ObservationSample {
        series_id: series.into(),
        observed_at: time(at),
        value,
    }
}

fn point(station: &str, variable: &str, at: &str, value: Option<f64>) -> ObservationReplacement {
    ObservationReplacement {
        replacement_id: format!("{station}-{variable}-{at}"),
        station_id: station.into(),
        variable: variable.into(),
        kind: ReplacementKind::Point,
        start_at: time(at),
        end_at: None,
        value,
        reason: "review".into(),
    }
}

#[test]
fn flow_availability_keeps_data_states_and_exact_calendar_hours() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("observations.sqlite");
    let mut store = ObservationStore::create(&db, "UTC").unwrap();
    store.upgrade_replacements().unwrap();
    let flow_edit = point("s", "flow", "2026-02-28T21:00:00", None);
    let insert = point("s", "flow", "2026-02-28T23:00:00", Some(0.0));
    let rain_edit = point("s", "rain", "2026-02-28T23:00:00", Some(99.0));
    let mut previous = flow_edit.clone();
    previous.replacement_id = "previous".into();
    previous.kind = ReplacementKind::Period;
    previous.end_at = Some(time("2026-02-28T22:00:00"));
    previous.value = Some(2.0);
    let replacement_ids = [&previous, &flow_edit, &insert, &rain_edit]
        .into_iter()
        .map(|r| r.replacement_id.clone())
        .collect();
    store
        .ingest_batch(&ObservationBatch {
            stations: vec![station("s", Some(0.0)), station("without-flow", None)],
            series: vec![series("s", "flow", "m3/s", None)],
            samples: vec![
                sample(
                    "s:flow",
                    "2026-02-01T00:00:00",
                    Some(ObservationValue::FlowM3s(7.0)),
                ),
                sample("s:flow", "2026-02-28T20:00:00", None),
                sample(
                    "s:flow",
                    "2026-02-28T21:00:00",
                    Some(ObservationValue::FlowM3s(4.0)),
                ),
                sample(
                    "s:flow",
                    "2026-02-28T22:35:00",
                    Some(ObservationValue::FlowM3s(5.0)),
                ),
                sample(
                    "s:flow",
                    "2026-03-01T00:00:00",
                    Some(ObservationValue::FlowM3s(8.0)),
                ),
            ],
            replacements: vec![previous, flow_edit, insert, rain_edit],
            ..Default::default()
        })
        .unwrap();
    let mut request = ObservationReviewRequest {
        database: db,
        selection: ObservationSelection {
            provider_codes: vec!["provider".into()],
            station_ids: Vec::new(),
        },
        series_ids: vec![],
        variable: ReviewVariable::Flow,
        end: time("2026-02-28T23:00:00"),
        window: ReviewWindow::Recent72Hours,
        replacement_ids,
        replacements: None,
    };
    let recent = review_observations(&request).unwrap();
    let s = &recent.stations[0];
    assert_eq!((s.absent, s.null, s.excluded, s.valid), (69, 1, 1, 1));
    assert_eq!(s.hours.len(), 72);
    assert_eq!(s.hours[68].state, AvailabilityState::Null);
    assert_eq!(s.hours[69].state, AvailabilityState::Excluded);
    assert_eq!(s.hours[69].raw_value, Some(4.0));
    assert_eq!(
        s.hours[69].applied_instruction_ids,
        request.replacement_ids[..2]
    );
    assert_eq!(s.hours[70].state, AvailabilityState::Absent);
    assert_eq!(s.hours[71].state, AvailabilityState::Valid);
    assert!(!s.hours[71].raw_present);
    assert_eq!(s.hours[71].effective_value, Some(0.0));
    assert_eq!(recent.stations[1].absent, 72);
    assert_eq!(recent.stations[1].series_id, None);
    request.window = ReviewWindow::Month;
    let monthly = review_observations(&request).unwrap();
    let s = &monthly.stations[0];
    assert_eq!(s.hours.len(), 28 * 24);
    assert_eq!(s.hours[0].observed_at, time("2026-02-01T00:00:00"));
    assert_eq!(s.hours[0].effective_value, Some(7.0));
    assert_eq!(s.hours.last().unwrap().observed_at, request.end);
}

#[test]
fn spatial_qc_uses_matched_raw_target_and_effective_neighbors() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("observations.sqlite");
    let mut store = ObservationStore::create(&db, "UTC").unwrap();
    let start = time("2026-02-01T00:00:00");
    store
        .ingest_batch(&ObservationBatch {
            stations: vec![
                station("a", Some(0.0)),
                station("b", Some(1.0)),
                station("c", Some(2.0)),
            ],
            series: ["a", "b", "c"]
                .iter()
                .map(|id| series(id, "rain", "mm", Some(3600)))
                .collect(),
            samples: vec![
                sample(
                    "a:rain",
                    "2026-02-01T01:00:00",
                    Some(ObservationValue::RainMm(10.0)),
                ),
                sample(
                    "a:rain",
                    "2026-02-01T02:00:00",
                    Some(ObservationValue::RainMm(20.0)),
                ),
                sample(
                    "b:rain",
                    "2026-02-01T01:00:00",
                    Some(ObservationValue::RainMm(3.0)),
                ),
                sample(
                    "b:rain",
                    "2026-02-01T02:00:00",
                    Some(ObservationValue::RainMm(4.0)),
                ),
                sample(
                    "b:rain",
                    "2026-02-01T03:00:00",
                    Some(ObservationValue::RainMm(5.0)),
                ),
                sample(
                    "c:rain",
                    "2026-02-01T01:00:00",
                    Some(ObservationValue::RainMm(12.0)),
                ),
            ],
            ..Default::default()
        })
        .unwrap();
    let request = ObservationReviewRequest {
        database: db,
        selection: ObservationSelection {
            provider_codes: vec!["provider".into()],
            station_ids: Vec::new(),
        },
        series_ids: vec![],
        variable: ReviewVariable::Rain,
        end: start + Duration::hours(3),
        window: ReviewWindow::Explicit { start },
        replacement_ids: vec![],
        replacements: Some(vec![
            point("a", "rain", "2026-02-01T01:00:00", Some(99.0)),
            point("b", "rain", "2026-02-01T01:00:00", Some(7.0)),
            point("b", "rain", "2026-02-01T02:00:00", None),
        ]),
    };
    let settings = IdwSettings {
        nearest_stations: 1,
        power: 2.0,
    };
    let single = spatial_observation_qc(&request, Some("a"), &settings).unwrap();
    let all = spatial_observation_qc(&request, None, &settings).unwrap();
    assert_eq!(all.stations.len(), 3);
    assert_eq!(single.stations, all.stations[..1]);
    let a = &single.stations[0];
    assert_eq!((a.expected_hours, a.raw_hours, a.matched_hours), (3, 2, 1));
    assert_eq!(a.raw_total_mm, Some(10.0));
    assert_eq!(a.predicted_total_mm, Some(7.0));
    assert_eq!(a.residual_mm, Some(-3.0));
    assert_eq!(a.hours[0].observed_at, start + Duration::hours(1));
    assert_eq!(all.stations[1].hours[0].predicted_mm, 99.0);
    assert!(spatial_observation_qc(&request, Some("unknown"), &settings).is_err());
}
