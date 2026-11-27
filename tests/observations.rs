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
    let path = dir.path().join("observations");
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
    assert!(
        !store
            .station_locations()
            .unwrap()
            .iter()
            .any(|s| s.station.station_id == "new")
    );
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
    let path = dir.path().join("observations");
    let mut store = ObservationStore::create(&path, "UTC").unwrap();
    store
        .write_batch(&ObservationBatch {
            stations: vec![station()],
            series: vec![series("l", "level", "cm", None)],
            samples: vec![ObservationSample {
                series_id: "l".into(),
                observed_at: time("2026-01-01T01:00:00"),
                value: Some(ObservationValue::LevelCm(1)),
            }],
            ..Default::default()
        })
        .unwrap();
    drop(store);
    corrupt_level(&path, "l", 1.5, true);
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
    damage_metadata(&path);
    assert!(ObservationStore::open_read_only(&path).is_err());
}
#[test]
fn changed_valid_sample_payload_fails_checksum() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("observations.sqlite");
    let mut store = ObservationStore::create(&path, "UTC").unwrap();
    let at = time("2026-01-01T01:00:00");
    store
        .write_batch(&ObservationBatch {
            stations: vec![station()],
            series: vec![series("l", "level", "cm", None)],
            samples: vec![ObservationSample {
                series_id: "l".into(),
                observed_at: at,
                value: Some(ObservationValue::LevelCm(1)),
            }],
            ..Default::default()
        })
        .unwrap();
    let revision = store.revision().unwrap();
    drop(store);
    corrupt_level(&path, "l", 2.0, false);
    let pinned = ObservationStore::open_revision(&path, revision).unwrap();
    let result = pinned.read_selection(
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
    );
    assert!(result.unwrap_err().to_string().contains("checksum"));
}
#[test]
fn invalid_batches_and_existing_destination_preserve_data() {
    use mgb_ops::StationOffset;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("observations");
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
    let path = dir.path().join("observations");
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
    corrupt_level(&path, "l", 2147483648.0, true);
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
    let path = dir.path().join("observations");
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
    let path = dir.path().join("observations");
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
        let path = dir.path().join("observations");
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

fn corrupt_level(path: &std::path::Path, id: &str, value: f64, refresh_checksum: bool) {
    use sha2::{Digest, Sha256};
    pollster::block_on(async {
        let db = turso::Builder::new_local(path.to_str().unwrap())
            .build()
            .await
            .unwrap();
        let conn = db.connect().unwrap();
        let level = if value.fract() == 0.0 {
            serde_json::json!(value as i64)
        } else {
            serde_json::json!(value)
        };
        let payload = serde_json::json!({
            "series_id": id,
            "observed_at": "2026-01-01T01:00:00",
            "value": {"LevelCm": level},
        })
        .to_string();
        if refresh_checksum {
            let checksum = format!("{:x}", Sha256::digest(payload.as_bytes()));
            conn.execute(
                "UPDATE observation_sample SET payload=?1,payload_sha256=?2 WHERE series_id=?3",
                (payload, checksum, id),
            )
            .await
            .unwrap();
        } else {
            conn.execute(
                "UPDATE observation_sample SET payload=?1 WHERE series_id=?2",
                (payload, id),
            )
            .await
            .unwrap();
        }
    });
}
fn damage_metadata(path: &std::path::Path) {
    pollster::block_on(async {
        let db = turso::Builder::new_local(path.to_str().unwrap())
            .build()
            .await
            .unwrap();
        let conn = db.connect().unwrap();
        conn.execute("UPDATE observation_metadata SET payload='{}'", ())
            .await
            .unwrap();
    });
}

#[test]
fn selected_snapshot_reopens_without_original_or_wal() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("observations.sqlite");
    let mut store = ObservationStore::create(&path, "UTC").unwrap();
    let at = time("2026-01-01T01:00:00");
    let sample = ObservationSample {
        series_id: "r".into(),
        observed_at: at,
        value: Some(ObservationValue::RainMm(0.0)),
    };
    store
        .write_batch(&ObservationBatch {
            stations: vec![station()],
            series: vec![series("r", "rain", "mm", Some(3600))],
            samples: vec![sample.clone()],
            ..Default::default()
        })
        .unwrap();
    let revision = store.revision().unwrap();
    let pinned = ObservationStore::open_revision(&path, revision).unwrap();
    let snapshot = dir.path().join("saved.sqlite");
    let axis = IntervalAxis {
        time_zone: "UTC".into(),
        bounds: vec![TimeInterval {
            start: time("2026-01-01T00:00:00"),
            end: at,
        }],
    };
    let selection = ObservationSelection {
        provider_codes: vec!["ana".into()],
        station_ids: vec![],
    };
    pinned
        .snapshot_revision(&snapshot, &selection, &axis, &[], &[])
        .unwrap();
    assert!(!dir.path().join("saved.sqlite-wal").exists());
    assert!(!dir.path().join("saved.sqlite-shm").exists());
    assert!(!dir.path().join("saved.sqlite.lock").exists());
    drop(pinned);
    drop(store);
    std::fs::remove_file(path).unwrap();
    let independent = ObservationStore::open_revision(&snapshot, revision).unwrap();
    assert_eq!(
        independent
            .read_selection(&selection, &axis, &[])
            .unwrap()
            .0
            .samples,
        vec![sample]
    );
}

#[test]
#[ignore = "regional legacy import requires runs/legacy-registry-export"]
fn regional_legacy_interchange_imports_all_samples() {
    let dir = tempfile::Builder::new()
        .prefix("sqlite-import-")
        .tempdir_in("runs")
        .unwrap();
    let path = dir.path().join("observations.sqlite");
    let store = ObservationStore::import_interchange(
        &path,
        std::path::Path::new("runs/legacy-registry-export"),
    )
    .unwrap();
    assert_eq!(store.station_locations().unwrap().len(), 440);
    assert_eq!(store.series_choices().unwrap().len(), 414);
    assert_eq!(store.replacement_choices().unwrap().len(), 6);
    drop(store);
    pollster::block_on(async {
        let db = turso::Builder::new_local(path.to_str().unwrap())
            .read_only(true)
            .build()
            .await
            .unwrap();
        let conn = db.connect().unwrap();
        let mut rows = conn
            .query("SELECT COUNT(*) FROM observation_sample", ())
            .await
            .unwrap();
        assert_eq!(
            rows.next().await.unwrap().unwrap().get_value(0).unwrap(),
            turso::Value::Integer(2_001_960)
        );
    });
}

#[test]
fn interchange_import_preserves_historical_dst_label_and_rejects_bad_digest() {
    use sha2::{Digest, Sha256};
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("input");
    std::fs::create_dir(&input).unwrap();
    let mut tables = serde_json::Map::new();
    let station = serde_json::json!({"station_id":"s","provider_code":"ana","station_code":"1","name":"station","latitude":null,"longitude":null});
    let series = serde_json::json!({"series_id":"r","station_id":"s","variable":"rain","state":"raw","unit":"mm","interval_seconds":3600,"created_at":"2016-01-01T00:00:00"});
    let sample = serde_json::json!({"series_id":"r","observed_at":"2016-02-20T23:00:00","rain":0.0,"flow":null,"level":null});
    for (name, rows) in [
        ("stations", vec![station]),
        ("series", vec![series]),
        ("samples", vec![sample]),
        ("replacements", vec![]),
        ("offsets", vec![]),
        ("references", vec![]),
        ("variables", vec![]),
        ("flags", vec![]),
        ("coverage", vec![]),
    ] {
        let bytes = rows
            .iter()
            .map(|row| format!("{}\n", row))
            .collect::<String>();
        let filename = format!("{name}.jsonl");
        std::fs::write(input.join(&filename), &bytes).unwrap();
        tables.insert(name.into(),serde_json::json!({"file":filename,"count":rows.len(),"sha256":format!("{:x}",Sha256::digest(bytes.as_bytes()))}));
    }
    std::fs::write(
        input.join("manifest.json"),
        serde_json::to_vec(&serde_json::json!({"time_zone":"America/Sao_Paulo","tables":tables}))
            .unwrap(),
    )
    .unwrap();
    let path = dir.path().join("observations.sqlite");
    let store = ObservationStore::import_interchange(&path, &input).unwrap();
    assert_eq!(store.series_choices().unwrap().len(), 1);
    pollster::block_on(async {
        let db = turso::Builder::new_local(path.to_str().unwrap())
            .read_only(true)
            .build()
            .await
            .unwrap();
        let conn = db.connect().unwrap();
        let mut rows = conn
            .query("SELECT observed_at,payload FROM observation_sample", ())
            .await
            .unwrap();
        let row = rows.next().await.unwrap().unwrap();
        assert_eq!(
            row.get_value(0).unwrap(),
            turso::Value::Text("2016-02-20T23:00:00".into())
        );
        let turso::Value::Text(payload) = row.get_value(1).unwrap() else {
            panic!("sample payload is not text")
        };
        let value: serde_json::Value = serde_json::from_str(&payload).unwrap();
        assert_eq!(value["value"]["RainMm"], 0.0);
    });
    assert!(path.exists());
    assert!(!dir.path().join("observations.sqlite-wal").exists());
    std::fs::write(input.join("samples.jsonl"), b"damaged\n").unwrap();
    let bad = dir.path().join("bad.sqlite");
    assert!(ObservationStore::import_interchange(&bad, &input).is_err());
    assert!(!bad.exists());
    assert!(ObservationStore::import_interchange(&path, &input).is_err());
    assert_eq!(store.series_choices().unwrap().len(), 1);
}

#[test]
fn sqlite_preserves_float_bits_through_revision_and_portable_snapshot() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("observations.sqlite");
    let mut store = ObservationStore::create(&path, "UTC").unwrap();
    let start = time("2026-01-01T00:00:00");
    let values = [
        0.0,
        -0.0,
        0.1,
        f64::MIN_POSITIVE,
        f64::from_bits(1),
        f64::MAX,
    ];
    let samples: Vec<_> = values
        .iter()
        .enumerate()
        .map(|(index, value)| ObservationSample {
            series_id: "r".into(),
            observed_at: start + chrono::TimeDelta::hours(index as i64 + 1),
            value: Some(ObservationValue::RainMm(*value)),
        })
        .collect();
    let mut metadata = station();
    metadata.latitude = Some(-0.0);
    metadata.longitude = Some(f64::MIN_POSITIVE);
    store
        .write_batch(&ObservationBatch {
            stations: vec![metadata],
            series: vec![series("r", "rain", "mm", Some(3600))],
            samples,
            ..Default::default()
        })
        .unwrap();
    let revision = store.revision().unwrap();
    let pinned = ObservationStore::open_revision(&path, revision).unwrap();
    let axis = IntervalAxis {
        time_zone: "UTC".into(),
        bounds: (0..values.len())
            .map(|index| TimeInterval {
                start: start + chrono::TimeDelta::hours(index as i64),
                end: start + chrono::TimeDelta::hours(index as i64 + 1),
            })
            .collect(),
    };
    let selection = ObservationSelection {
        provider_codes: vec!["ana".into()],
        station_ids: vec![],
    };
    let snapshot = dir.path().join("portable.sqlite");
    pinned
        .snapshot_revision(&snapshot, &selection, &axis, &[], &[])
        .unwrap();
    for db in [&path, &snapshot] {
        let read = ObservationStore::open_revision(db, revision).unwrap();
        let (data, _) = read.read_selection(&selection, &axis, &[]).unwrap();
        assert_eq!(
            data.stations[0].latitude.unwrap().to_bits(),
            (-0.0f64).to_bits()
        );
        assert_eq!(
            data.stations[0].longitude.unwrap().to_bits(),
            f64::MIN_POSITIVE.to_bits()
        );
        let actual: Vec<_> = data
            .samples
            .iter()
            .map(|sample| match sample.value {
                Some(ObservationValue::RainMm(v)) => v.to_bits(),
                _ => panic!("unexpected sample"),
            })
            .collect();
        assert_eq!(actual, values.map(f64::to_bits));
    }
}
