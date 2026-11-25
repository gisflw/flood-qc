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
    assert_level_int32(&path, "l");
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
    corrupt_level(&path, "l", 1.5);
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
    let manifest = current_manifest(&path);
    std::fs::remove_file(path.join(manifest["tables"]["references"].as_str().unwrap())).unwrap();
    assert!(ObservationStore::open_read_only(&path).is_err());
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
    assert_level_int32(&path, "l");
    corrupt_level(&path, "l", 2147483648.0);
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

fn partition(path: &std::path::Path, id: &str) -> std::path::PathBuf {
    use sha2::{Digest, Sha256};
    let manifest = current_manifest(path);
    let prefix = format!("{:x}", Sha256::digest(id.as_bytes()));
    let entry = manifest["partitions"]
        .as_object()
        .unwrap()
        .iter()
        .find(|(key, _)| key.starts_with(&prefix))
        .unwrap()
        .1
        .as_str()
        .unwrap();
    path.join(entry)
}
fn assert_level_int32(path: &std::path::Path, id: &str) {
    use parquet::{
        basic::Type,
        file::reader::{FileReader, SerializedFileReader},
    };
    let reader =
        SerializedFileReader::new(std::fs::File::open(partition(path, id)).unwrap()).unwrap();
    assert_eq!(
        reader.metadata().row_group(0).column(4).column_type(),
        Type::INT32
    );
}
fn corrupt_level(path: &std::path::Path, id: &str, value: f64) {
    use arrow_array::{ArrayRef, Float64Array, RecordBatch, StringArray};
    use arrow_schema::{DataType, Field, Schema};
    use parquet::arrow::ArrowWriter;
    use std::sync::Arc;
    let schema = Arc::new(Schema::new(vec![
        Field::new("series_id", DataType::Utf8, true),
        Field::new("observed_at", DataType::Utf8, true),
        Field::new("rain", DataType::Float64, true),
        Field::new("flow", DataType::Float64, true),
        Field::new("level", DataType::Float64, true),
    ]));
    let arrays: Vec<ArrayRef> = vec![
        Arc::new(StringArray::from(vec![id])),
        Arc::new(StringArray::from(vec!["2026-01-01T01:00:00"])),
        Arc::new(Float64Array::from(vec![None::<f64>])),
        Arc::new(Float64Array::from(vec![None::<f64>])),
        Arc::new(Float64Array::from(vec![Some(value)])),
    ];
    let batch = RecordBatch::try_new(schema.clone(), arrays).unwrap();
    let mut writer = ArrowWriter::try_new(
        std::fs::File::create(partition(path, id)).unwrap(),
        schema,
        None,
    )
    .unwrap();
    writer.write(&batch).unwrap();
    writer.close().unwrap();
}

fn current_manifest(root: &std::path::Path) -> serde_json::Value {
    serde_json::from_slice(&std::fs::read(root.join("current.json")).unwrap()).unwrap()
}
