use chrono::NaiveDateTime;
use mgb_ops::{
    IntervalAxis, ObservationBatch, ObservationSample, ObservationSelection, ObservationSeries,
    ObservationStore, ObservationValue, Station, TimeInterval,
};

fn time(s: &str) -> NaiveDateTime {
    NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S").unwrap()
}
fn station() -> Station {
    Station {
        station_id: "s".into(),
        provider_code: "ana".into(),
        station_code: "1".into(),
        name: "station".into(),
        latitude: Some(0.0),
        longitude: Some(0.0),
    }
}
fn series(
    id: &str,
    variable: &str,
    unit: &str,
    interval_seconds: Option<i64>,
) -> ObservationSeries {
    ObservationSeries {
        series_id: id.into(),
        station_id: "s".into(),
        variable: variable.into(),
        state: "raw".into(),
        unit: unit.into(),
        interval_seconds,
        created_at: time("2026-01-01T00:00:00"),
    }
}
#[test]
fn integer_levels_null_and_atomic_rollback() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("observations.sqlite");
    let mut store = ObservationStore::create(&path, "America/Sao_Paulo").unwrap();
    let at = time("2026-01-01T01:00:00");
    let batch = ObservationBatch {
        stations: vec![station()],
        series: vec![
            series("l", "level", "cm", None),
            series("r", "rain", "mm", Some(3600)),
        ],
        samples: vec![
            ObservationSample {
                series_id: "l".into(),
                observed_at: at,
                value: Some(ObservationValue::LevelCm(i32::MIN)),
            },
            ObservationSample {
                series_id: "r".into(),
                observed_at: at,
                value: Some(ObservationValue::RainMm(0.0)),
            },
        ],
        ..Default::default()
    };
    store.write_batch(&batch).unwrap();
    assert_eq!(
        store.series_choices().unwrap(),
        vec![
            ("l".into(), "s".into(), "level".into()),
            ("r".into(), "s".into(), "rain".into())
        ]
    );
    assert!(store.replacement_choices().unwrap().is_empty());
    let conn = rusqlite::Connection::open(&path).unwrap();
    assert_eq!(
        conn.query_row(
            "SELECT typeof(value) FROM observation WHERE series_id='l'",
            [],
            |r| r.get::<_, String>(0)
        )
        .unwrap(),
        "integer"
    );
    let bad = ObservationBatch {
        stations: vec![Station {
            station_id: "new".into(),
            station_code: "2".into(),
            ..station()
        }],
        samples: vec![ObservationSample {
            series_id: "r".into(),
            observed_at: at,
            value: Some(ObservationValue::RainMm(f64::NAN)),
        }],
        ..Default::default()
    };
    assert!(store.write_batch(&bad).is_err());
    assert_eq!(
        conn.query_row(
            "SELECT count(*) FROM station WHERE station_id='new'",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        0
    );
    drop(conn);
    drop(store);
    let store = ObservationStore::open_read_only(&path).unwrap();
    let (inputs, offsets) = store
        .read_selection(
            &ObservationSelection {
                provider_codes: vec!["ana".into()],
                station_ids: vec![],
            },
            &IntervalAxis {
                time_zone: "America/Sao_Paulo".into(),
                bounds: vec![TimeInterval {
                    start: time("2026-01-01T00:00:00"),
                    end: at,
                }],
            },
            &[],
        )
        .unwrap();
    assert_eq!(inputs.samples, batch.samples);
    assert!(offsets.is_empty());
    assert_eq!(store.time_zone().unwrap(), "America/Sao_Paulo");
}
#[test]
fn fractional_level_and_missing_schema_are_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("observations.sqlite");
    let mut store = ObservationStore::create(&path, "UTC").unwrap();
    store
        .write_batch(&ObservationBatch {
            stations: vec![station()],
            series: vec![series("l", "level", "cm", None)],
            ..Default::default()
        })
        .unwrap();
    drop(store);
    let conn = rusqlite::Connection::open(&path).unwrap();
    conn.execute(
        "INSERT INTO observation VALUES ('l','2026-01-01T01:00:00',1.5)",
        [],
    )
    .unwrap();
    drop(conn);
    let store = ObservationStore::open_read_only(&path).unwrap();
    assert!(
        store
            .read_selection(
                &ObservationSelection {
                    provider_codes: vec!["ana".into()],
                    station_ids: vec![]
                },
                &IntervalAxis {
                    time_zone: "UTC".into(),
                    bounds: vec![TimeInterval {
                        start: time("2026-01-01T00:00:00"),
                        end: time("2026-01-01T02:00:00")
                    }]
                },
                &[]
            )
            .is_err()
    );
    drop(store);
    let conn = rusqlite::Connection::open(&path).unwrap();
    conn.execute_batch("DROP TABLE level_reference").unwrap();
    drop(conn);
    assert!(ObservationStore::open_read_only(&path).is_err());
}
#[test]
fn invalid_batches_and_existing_destination_preserve_data() {
    use mgb_ops::StationOffset;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("observations.sqlite");
    let mut store = ObservationStore::create(&path, "UTC").unwrap();
    assert!(ObservationStore::create(&path, "UTC").is_err());
    let base = ObservationBatch {
        stations: vec![station()],
        series: vec![series("r", "rain", "mm", Some(3600))],
        offsets: vec![StationOffset {
            station_id: "s".into(),
            mini_id: 4,
            offset_m: 0.0,
        }],
        ..Default::default()
    };
    store.write_batch(&base).unwrap();
    assert_eq!(
        store
            .read_station_offsets(&["s".into(), "missing".into()])
            .unwrap(),
        base.offsets
    );
    let at = time("2026-01-01T01:00:00");
    for bad in [
        ObservationBatch {
            series: vec![series("bad", "rain", "cm", Some(3600))],
            ..Default::default()
        },
        ObservationBatch {
            samples: vec![ObservationSample {
                series_id: "orphan".into(),
                observed_at: at,
                value: None,
            }],
            ..Default::default()
        },
        ObservationBatch {
            samples: vec![ObservationSample {
                series_id: "r".into(),
                observed_at: at,
                value: Some(ObservationValue::RainMm(f64::INFINITY)),
            }],
            ..Default::default()
        },
        ObservationBatch {
            stations: vec![station()],
            ..Default::default()
        },
        ObservationBatch {
            offsets: vec![StationOffset {
                station_id: "s".into(),
                mini_id: 4,
                offset_m: f64::INFINITY,
            }],
            ..Default::default()
        },
    ] {
        assert!(store.write_batch(&bad).is_err());
    }
    let good = ObservationBatch {
        samples: vec![ObservationSample {
            series_id: "r".into(),
            observed_at: at,
            value: None,
        }],
        ..Default::default()
    };
    store.write_batch(&good).unwrap();
    assert!(store.write_batch(&good).is_err());
    let (inputs, _) = store
        .read_selection(
            &ObservationSelection {
                provider_codes: vec!["ana".into()],
                station_ids: vec![],
            },
            &IntervalAxis {
                time_zone: "UTC".into(),
                bounds: vec![TimeInterval {
                    start: time("2026-01-01T00:00:00"),
                    end: at,
                }],
            },
            &[],
        )
        .unwrap();
    assert_eq!(inputs.samples, good.samples);
    assert!(
        store
            .read_selection(
                &ObservationSelection {
                    provider_codes: vec![],
                    station_ids: vec!["missing".into()]
                },
                &IntervalAxis {
                    time_zone: "UTC".into(),
                    bounds: vec![TimeInterval {
                        start: time("2026-01-01T00:00:00"),
                        end: at
                    }]
                },
                &[]
            )
            .is_err()
    );
}
#[test]
fn signed_levels_offsets_and_local_dst_fail_atomically() {
    use mgb_ops::StationOffset;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("observations.sqlite");
    let mut store = ObservationStore::create(&path, "America/New_York").unwrap();
    let base = ObservationBatch {
        stations: vec![station()],
        series: vec![series("l", "level", "cm", None)],
        offsets: vec![StationOffset {
            station_id: "s".into(),
            mini_id: 1,
            offset_m: -0.0125,
        }],
        ..Default::default()
    };
    store.write_batch(&base).unwrap();
    let samples = [i32::MIN, -1, 0, i32::MAX]
        .into_iter()
        .enumerate()
        .map(|(i, v)| ObservationSample {
            series_id: "l".into(),
            observed_at: time("2026-01-01T00:00:00") + chrono::Duration::hours(i as i64),
            value: Some(ObservationValue::LevelCm(v)),
        })
        .collect::<Vec<_>>();
    store
        .write_batch(&ObservationBatch {
            samples: samples.clone(),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(
        store.read_station_offsets(&["s".into()]).unwrap(),
        base.offsets
    );
    let invalid = ObservationBatch {
        stations: vec![Station {
            station_id: "new".into(),
            station_code: "2".into(),
            ..station()
        }],
        samples: vec![ObservationSample {
            series_id: "l".into(),
            observed_at: time("2026-11-01T01:30:00"),
            value: Some(ObservationValue::LevelCm(0)),
        }],
        ..Default::default()
    };
    assert!(store.write_batch(&invalid).is_err());
    let interval = IntervalAxis {
        time_zone: "America/New_York".into(),
        bounds: vec![TimeInterval {
            start: time("2025-12-31T23:00:00"),
            end: time("2026-01-01T04:00:00"),
        }],
    };
    let (read, _) = store
        .read_selection(
            &ObservationSelection {
                provider_codes: vec!["ana".into()],
                station_ids: vec![],
            },
            &interval,
            &[],
        )
        .unwrap();
    assert_eq!(read.samples, samples);
    assert_eq!(read.stations.len(), 1);
    drop(store);
    let conn = rusqlite::Connection::open(&path).unwrap();
    assert_eq!(
        conn.query_row(
            "SELECT count(*) FROM observation WHERE typeof(value)='integer'",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        4
    );
    conn.execute(
        "INSERT INTO observation VALUES ('l','2026-01-01T05:00:00',2147483648)",
        [],
    )
    .unwrap();
    drop(conn);
    let store = ObservationStore::open_read_only(&path).unwrap();
    let mut bad = interval;
    bad.bounds[0].end = time("2026-01-01T06:00:00");
    assert!(
        store
            .read_selection(
                &ObservationSelection {
                    provider_codes: vec!["ana".into()],
                    station_ids: vec![]
                },
                &bad,
                &[]
            )
            .is_err()
    );
}
#[test]
fn replacement_ids_must_exist_and_be_unique() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("observations.sqlite");
    let mut store = ObservationStore::create(&path, "UTC").unwrap();
    store
        .write_batch(&ObservationBatch {
            stations: vec![station()],
            ..Default::default()
        })
        .unwrap();
    let axis = IntervalAxis {
        time_zone: "UTC".into(),
        bounds: vec![TimeInterval {
            start: time("2026-01-01T00:00:00"),
            end: time("2026-01-01T01:00:00"),
        }],
    };
    let selection = ObservationSelection {
        provider_codes: vec!["ana".into()],
        station_ids: vec![],
    };
    assert!(
        store
            .read_selection(&selection, &axis, &["missing".into()])
            .is_err()
    );
    use mgb_ops::{ObservationReplacement, ReplacementKind};
    store
        .write_batch(&ObservationBatch {
            replacements: vec![ObservationReplacement {
                replacement_id: "r".into(),
                station_id: "s".into(),
                variable: "rain".into(),
                kind: ReplacementKind::Point,
                start_at: axis.bounds[0].end,
                end_at: None,
                value: Some(0.0),
                reason: "review".into(),
            }],
            ..Default::default()
        })
        .unwrap();
    assert!(
        store
            .read_selection(&selection, &axis, &["r".into(), "r".into()])
            .is_err()
    );
}

#[test]
fn canonical_ingest_retries_and_inventory_changes_preserve_identity() {
    use mgb_ops::{ObservationReplacement, ReplacementKind, StationOffset};
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("observations.sqlite");
    let mut store = ObservationStore::create(&path, "UTC").unwrap();
    let at = time("2026-01-01T01:00:00");
    let batch = ObservationBatch {
        stations: vec![station()],
        series: vec![series("r", "rain", "mm", Some(3600))],
        samples: vec![ObservationSample {
            series_id: "r".into(),
            observed_at: at,
            value: None,
        }],
        replacements: vec![ObservationReplacement {
            replacement_id: "rain-edit".into(),
            station_id: "s".into(),
            variable: "rain".into(),
            kind: ReplacementKind::Point,
            start_at: at,
            end_at: None,
            value: Some(0.0),
            reason: "review".into(),
        }],
        offsets: vec![StationOffset {
            station_id: "s".into(),
            mini_id: 2,
            offset_m: -1.5,
        }],
    };
    store.ingest_batch(&batch).unwrap();
    store.ingest_batch(&batch).unwrap();

    let mut bad = batch.clone();
    bad.stations.push(Station {
        station_id: "new".into(),
        station_code: "2".into(),
        ..station()
    });
    bad.samples[0].value = Some(ObservationValue::RainMm(0.0));
    assert!(store.ingest_batch(&bad).is_err());
    assert_eq!(store.station_locations().unwrap().len(), 1);

    let changed = Station {
        name: "New name".into(),
        latitude: Some(10.0),
        ..station()
    };
    let bad_identity = Station {
        provider_code: "other".into(),
        ..station()
    };
    assert!(
        store
            .update_station_inventory(&[changed.clone(), bad_identity.clone()])
            .is_err()
    );
    assert_eq!(store.station_locations().unwrap()[0].station, station());
    assert!(store.update_station_inventory(&[bad_identity]).is_err());
    store
        .update_station_inventory(std::slice::from_ref(&changed))
        .unwrap();
    store
        .update_station_inventory(std::slice::from_ref(&changed))
        .unwrap();
    assert_eq!(store.station_locations().unwrap()[0].station, changed);
    assert_eq!(
        store.read_station_offsets(&["s".into()]).unwrap(),
        batch.offsets
    );
    assert_eq!(
        store.load_replacements(&["rain-edit".into()]).unwrap(),
        batch.replacements
    );
    assert_eq!(store.series_choices().unwrap()[0].0, "r");
}

#[test]
fn fresh_schema_supports_flow_edits_and_references() {
    use mgb_ops::store::LevelReference;
    use mgb_ops::{ObservationReplacement, ReplacementKind};
    for write_references in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("observations.sqlite");
        let mut store = ObservationStore::create(&path, "UTC").unwrap();
        let at = time("2026-01-01T01:00:00");
        let rain = ObservationReplacement {
            replacement_id: "rain-edit".into(),
            station_id: "s".into(),
            variable: "rain".into(),
            kind: ReplacementKind::Point,
            start_at: at,
            end_at: None,
            value: Some(0.0),
            reason: "review".into(),
        };
        let flow = ObservationReplacement {
            replacement_id: "flow-edit".into(),
            variable: "flow".into(),
            ..rain.clone()
        };
        store
            .ingest_batch(&ObservationBatch {
                stations: vec![station()],
                replacements: vec![rain.clone()],
                ..Default::default()
            })
            .unwrap();
        let references = if write_references {
            let references = vec![LevelReference {
                station_id: "s".into(),
                kind: "flood".into(),
                date: None,
                level_cm: 30.0,
            }];
            store.replace_level_references(&references).unwrap();
            references
        } else {
            Vec::new()
        };
        store.register_replacements(vec![flow.clone()]).unwrap();
        let read = ObservationStore::open_read_only(&path).unwrap();
        assert_eq!(
            read.load_replacements(&["flow-edit".into(), "rain-edit".into()])
                .unwrap(),
            vec![flow, rain]
        );
        assert_eq!(read.read_level_references().unwrap(), references);
        store.replace_level_references(&references).unwrap();
    }
}
