use mgb_ops::analysis::upstream_minis;
use mgb_ops::store::{read_mini_map_layer, read_mini_topology};
use mgb_ops::{
    ObservationBatch, ObservationSample, ObservationSeries, ObservationStore, ObservationValue,
    Station, StationOffset, import_level_references,
};
use std::fs;

#[test]
fn topology_validates_and_multipart_geometry_matches_ids() {
    let d = tempfile::tempdir().unwrap();
    let mini = d.path().join("MINI.gtp");
    fs::write(
        &mini,
        "CatID Mini Xcen Ycen Area_(km2) MiniJus\n228 1 -52 -30 2 2\n9 2 -51 -30 5 -1\n",
    )
    .unwrap();
    let (_, topology) = read_mini_topology(&mini).unwrap();
    assert_eq!(upstream_minis(&topology, 2).unwrap(), vec![1, 2]);
    assert_eq!(topology[0].area_km2, 2.0);
    let geo = d.path().join("mini.geojson");
    fs::write(&geo,r#"{"type":"FeatureCollection","features":[{"type":"Feature","properties":{"mini_id":1},"geometry":{"type":"MultiLineString","coordinates":[[[-52,-30],[-51,-30]],[[-52,-30],[-51,-30]]]}},{"type":"Feature","properties":{"mini_id":2},"geometry":{"type":"LineString","coordinates":[[-51,-30],[-50,-30]]}}]}"#).unwrap();
    let layers = mgb_ops::store::read_map_layers(&geo).unwrap();
    let features = read_mini_map_layer(&geo, &layers[0].name, &topology).unwrap();
    assert_eq!(features.len(), 2);
    assert_eq!(features[0].paths.len(), 2);
    for bad in ["2 1", "2 99", "0 2", "NaN 2"] {
        fs::write(
            &mini,
            format!("Mini Xcen Ycen Area_(km2) MiniJus\n1 -52 -30 {bad}\n2 -51 -30 5 -1\n"),
        )
        .unwrap();
        assert!(read_mini_topology(&mini).is_err());
    }
}

#[test]
fn reference_import_preserves_station_and_offset_on_failure() {
    let d = tempfile::tempdir().unwrap();
    let db = d.path().join("observations");
    let mut store = ObservationStore::create(&db, "UTC").unwrap();
    store
        .write_batch(&ObservationBatch {
            stations: vec![Station {
                station_id: "a".into(),
                provider_code: "ana".into(),
                station_code: "00123".into(),
                name: "A".into(),
                latitude: Some(-30.0),
                longitude: Some(-52.0),
            }],
            series: vec![ObservationSeries {
                series_id: "series".into(),
                station_id: "a".into(),
                variable: "level".into(),
                state: "raw".into(),
                unit: "cm".into(),
                interval_seconds: None,
                created_at: chrono::NaiveDate::from_ymd_opt(2020, 1, 1)
                    .unwrap()
                    .and_hms_opt(0, 0, 0)
                    .unwrap(),
            }],
            samples: vec![ObservationSample {
                series_id: "series".into(),
                observed_at: chrono::NaiveDate::from_ymd_opt(2020, 1, 2)
                    .unwrap()
                    .and_hms_opt(0, 0, 0)
                    .unwrap(),
                value: Some(ObservationValue::LevelCm(-42)),
            }],
            offsets: vec![StationOffset {
                station_id: "a".into(),
                mini_id: 1,
                offset_m: 0.25,
            }],
            ..Default::default()
        })
        .unwrap();
    drop(store);
    let sample = || {
        let at = chrono::NaiveDate::from_ymd_opt(2020, 1, 2)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        ObservationStore::open_read_only(&db)
            .unwrap()
            .read_selection(
                &mgb_ops::ObservationSelection {
                    provider_codes: vec![],
                    station_ids: vec!["a".into()],
                },
                &mgb_ops::IntervalAxis {
                    time_zone: "UTC".into(),
                    bounds: vec![mgb_ops::TimeInterval {
                        start: at - chrono::Duration::hours(1),
                        end: at,
                    }],
                },
                &[],
            )
            .unwrap()
            .0
    };
    let original_sample = sample();
    assert!(
        ObservationStore::open_read_only(&db)
            .unwrap()
            .read_level_references()
            .unwrap()
            .is_empty()
    );
    let thresholds = d.path().join("thresholds.csv");
    let history = d.path().join("history.csv");
    fs::write(&thresholds,"\u{feff}station_code,station_name,level_atention,level_alert,level_flood,level_severe\r\n123,A,0,-1.25,,\r\n999,Missing,,,,\r\n").unwrap();
    fs::write(
        &history,
        "station_code,level_registered,date\r\n123,5.125,2020-01-02\r\n",
    )
    .unwrap();
    let report = import_level_references(&db, "ana", &thresholds, &history).unwrap();
    assert_eq!(sample(), original_sample);
    assert_eq!(report.imported, 3);
    assert_eq!(report.unmatched_codes, vec!["999"]);
    let store = ObservationStore::open_read_only(&db).unwrap();
    let before = store.read_level_references().unwrap();
    assert!(before.iter().any(|r| r.level_cm == -1.25));
    let locations = store.station_locations().unwrap();
    drop(store);
    fs::write(
        &history,
        "station_code,level_registered,date\n123,3,invalid\n",
    )
    .unwrap();
    assert!(import_level_references(&db, "ana", &thresholds, &history).is_err());
    let store = ObservationStore::open_read_only(&db).unwrap();
    assert_eq!(store.read_level_references().unwrap(), before);
    assert_eq!(store.station_locations().unwrap(), locations);
    drop(store);
    assert_eq!(sample(), original_sample);
    let mut store = ObservationStore::open_writable(&db).unwrap();
    let mut replacement = before.clone();
    replacement[1].station_id = "unknown".into();
    assert!(store.replace_level_references(&replacement).is_err());
    assert_eq!(store.read_level_references().unwrap(), before);
    assert_eq!(store.station_locations().unwrap(), locations);
    drop(store);
    assert_eq!(sample(), original_sample);
}

#[test]
#[ignore = "requires external RS source data"]
fn rs_geometry_and_reference_coverage() {
    use std::path::Path;
    let root = Path::new("/workspace/rs_hydro_forecast");
    fs::create_dir_all("runs").unwrap();
    let d = tempfile::tempdir_in("runs").unwrap();
    let mini = d.path().join("MINI.gtp");
    let gpkg = d.path().join("geometry.gpkg");
    fs::copy(root.join("mgb_runner/Input/MINI.gtp"), &mini).unwrap();
    fs::copy(root.join("source/rs_hydro.gpkg"), &gpkg).unwrap();
    let (_, topology) = read_mini_topology(&mini).unwrap();
    assert_eq!(topology.len(), 8649);
    for layer in ["mini_segments", "mini_catchments"] {
        assert_eq!(
            read_mini_map_layer(&gpkg, layer, &topology).unwrap().len(),
            8649
        );
    }
    let ids = upstream_minis(&topology, 6393).unwrap();
    assert_eq!(ids.len(), 9);
    let sum: f64 = topology
        .iter()
        .filter(|t| ids.contains(&t.mini_id))
        .map(|t| t.area_km2)
        .sum();
    assert!((sum - 643.33966).abs() < 1e-6);
    let db = d.path().join("observations");
    let mut store = ObservationStore::create(&db, "UTC").unwrap();
    let mut reader =
        csv::Reader::from_path(root.join("source/history_station_inventory.csv")).unwrap();
    let mut stations = Vec::new();
    for row in reader.records() {
        let row = row.unwrap();
        if &row[0] == "ana" {
            stations.push(Station {
                station_id: row[1].into(),
                provider_code: "ana".into(),
                station_code: row[1].into(),
                name: row[2].into(),
                latitude: None,
                longitude: None,
            });
        }
    }
    store
        .write_batch(&ObservationBatch {
            stations,
            ..Default::default()
        })
        .unwrap();

    drop(store);
    let report = import_level_references(
        &db,
        "ana",
        &root.join("source/station_level_reference.csv"),
        &root.join("source/registered_floods.csv"),
    )
    .unwrap();
    assert_eq!(report.imported, 154);
    assert!(report.unmatched_codes.is_empty());
    assert_eq!(
        ObservationStore::open_read_only(&db)
            .unwrap()
            .read_level_references()
            .unwrap()
            .len(),
        154
    );
}

#[test]
fn duplicate_keys_and_cli_share_the_import() {
    use std::process::Command;
    let d = tempfile::tempdir().unwrap();
    let db = d.path().join("observations");
    let mut store = ObservationStore::create(&db, "UTC").unwrap();
    store
        .write_batch(&ObservationBatch {
            stations: vec![Station {
                station_id: "a".into(),
                provider_code: "ana".into(),
                station_code: "1".into(),
                name: "A".into(),
                latitude: None,
                longitude: None,
            }],
            ..Default::default()
        })
        .unwrap();

    drop(store);
    let thresholds = d.path().join("t.csv");
    let history = d.path().join("h.csv");
    fs::write(&history, "station_code,level_registered,date\n").unwrap();
    fs::write(&thresholds,"station_code,station_name,level_atention,level_alert,level_flood,level_severe\n1,A,1,,,\n001,A,2,,,\n").unwrap();
    assert!(import_level_references(&db, "ana", &thresholds, &history).is_err());
    fs::write(&thresholds,"station_code,station_name,level_atention,level_alert,level_flood,level_severe\n1,A,1,,,\n001,A,,2,,\n").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_mgb-ops"))
        .args(["references", "import", "--assets"])
        .arg(&db)
        .args(["--provider", "ana", "--thresholds"])
        .arg(&thresholds)
        .arg("--history")
        .arg(&history)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["imported"], 2);
    let before = ObservationStore::open_read_only(&db)
        .unwrap()
        .read_level_references()
        .unwrap();
    assert!(import_level_references(&db, "anaa", &thresholds, &history).is_err());
    let failed = Command::new(env!("CARGO_BIN_EXE_mgb-ops"))
        .args(["references", "import", "--assets"])
        .arg(&db)
        .args(["--provider", "anaa", "--thresholds"])
        .arg(&thresholds)
        .arg("--history")
        .arg(&history)
        .output()
        .unwrap();
    assert!(!failed.status.success());
    assert_eq!(
        ObservationStore::open_read_only(&db)
            .unwrap()
            .read_level_references()
            .unwrap(),
        before
    );
    assert_eq!(
        ObservationStore::open_read_only(&db)
            .unwrap()
            .read_level_references()
            .unwrap()
            .len(),
        2
    );
}

#[test]
fn upstream_rejects_invalid_direct_topology_even_at_outlet() {
    use mgb_ops::store::MiniTopology;
    let mut topology = vec![
        MiniTopology {
            mini_id: 1,
            downstream: Some(2),
            area_km2: 1.0,
        },
        MiniTopology {
            mini_id: 2,
            downstream: Some(1),
            area_km2: 1.0,
        },
    ];
    assert!(upstream_minis(&topology, 1).is_err());
    topology[1].downstream = Some(3);
    assert!(upstream_minis(&topology, 1).is_err());
    topology[1].downstream = None;
    topology[0].area_km2 = f64::NAN;
    assert!(upstream_minis(&topology, 1).is_err());
}

#[test]
fn geometry_rejects_invalid_rings_wrong_layers_and_ids() {
    use mgb_ops::store::MiniTopology;
    let d = tempfile::tempdir().unwrap();
    let topology = [MiniTopology {
        mini_id: 1,
        downstream: None,
        area_km2: 1.0,
    }];
    let polygon = |ring: &str| {
        format!(
            r#"{{"type":"FeatureCollection","features":[{{"type":"Feature","properties":{{"mini_id":1}},"geometry":{{"type":"Polygon","coordinates":[{ring}]}}}}]}}"#
        )
    };
    let catchments = d.path().join("mini_catchments.geojson");
    for ring in ["[[0,0],[1,1]]", "[[0,0],[1,0],[2,0],[0,0]]"] {
        fs::write(&catchments, polygon(ring)).unwrap();
        assert!(mgb_ops::store::read_map_layers(&catchments).is_err());
        assert!(read_mini_map_layer(&catchments, "mini_catchments", &topology).is_err());
    }
    fs::write(&catchments, polygon("[[0,0],[1,0],[1,1],[0,0]]")).unwrap();
    let valid = read_mini_map_layer(&catchments, "mini_catchments", &topology).unwrap();
    assert_eq!(valid[0].paths[0].first(), valid[0].paths[0].last());
    assert!(read_mini_map_layer(&catchments, "missing", &topology).is_err());
    let wrong = [MiniTopology {
        mini_id: 2,
        downstream: None,
        area_km2: 1.0,
    }];
    assert!(read_mini_map_layer(&catchments, "mini_catchments", &wrong).is_err());
    let segments = d.path().join("mini_segments.geojson");
    fs::write(&segments, polygon("[[0,0],[1,0],[1,1],[0,0]]")).unwrap();
    assert!(read_mini_map_layer(&segments, "mini_segments", &topology).is_err());
    fs::write(&catchments,r#"{"type":"FeatureCollection","features":[{"type":"Feature","properties":{"mini_id":1},"geometry":{"type":"LineString","coordinates":[[0,0],[1,1]]}}]}"#).unwrap();
    assert!(read_mini_map_layer(&catchments, "mini_catchments", &topology).is_err());
}

#[test]
fn csv_rejects_duplicate_headers_and_invalid_unmatched_rows() {
    use mgb_ops::adapters::read_reference_csvs;
    let d = tempfile::tempdir().unwrap();
    let t = d.path().join("thresholds.csv");
    let h = d.path().join("history.csv");
    fs::write(&t,"station_code,station_name,level_atention,level_alert,level_flood,level_severe\n1,\"Name, quoted\",0,,,\n").unwrap();
    for history in [
        "station_code,level_registered,date\n1,5,2020-01-01\n001,5,2020-01-01\n",
        "station_code,level_registered,date\n999,NaN,2020-01-01\n",
        "station_code,level_registered,level_registered,date\n1,5,NaN,2020-01-01\n",
    ] {
        fs::write(&h, history).unwrap();
        assert!(read_reference_csvs(&t, &h).is_err());
    }
    fs::write(
        &h,
        "station_code,level_registered,date,extra\n1,5,2020-01-01,ok\n",
    )
    .unwrap();
    assert_eq!(read_reference_csvs(&t, &h).unwrap().rows.len(), 2);
}

#[test]
fn detached_reference_reader_rejects_duplicate_and_invalid_records() {
    let d = tempfile::tempdir().unwrap();
    let db = d.path().join("observations");
    let mut store = ObservationStore::create(&db, "UTC").unwrap();
    store
        .write_batch(&ObservationBatch {
            stations: vec![Station {
                station_id: "a".into(),
                provider_code: "ana".into(),
                station_code: "1".into(),
                name: "A".into(),
                latitude: None,
                longitude: None,
            }],
            ..Default::default()
        })
        .unwrap();

    drop(store);
    corrupt_references(&db, false);
    assert!(
        ObservationStore::open_read_only(&db)
            .unwrap()
            .read_level_references()
            .is_err()
    );
    corrupt_references(&db, true);
    assert!(
        ObservationStore::open_read_only(&db)
            .unwrap()
            .read_level_references()
            .is_err()
    );
}

#[test]
fn reference_json_cli_reports_success_and_operational_failures() {
    use std::process::Command;
    let d = tempfile::tempdir().unwrap();
    let db = d.path().join("observations");
    let mut store = ObservationStore::create(&db, "UTC").unwrap();
    store
        .write_batch(&ObservationBatch {
            stations: vec![Station {
                station_id: "a".into(),
                provider_code: "ana".into(),
                station_code: "1".into(),
                name: "A".into(),
                latitude: None,
                longitude: None,
            }],
            ..Default::default()
        })
        .unwrap();
    drop(store);
    let t = d.path().join("thresholds.csv");
    let h = d.path().join("history.csv");
    fs::write(
        &t,
        "station_code,station_name,level_atention,level_alert,level_flood,level_severe\n1,A,1,,,\n",
    )
    .unwrap();
    fs::write(&h, "station_code,level_registered,date\n").unwrap();
    let run = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_mgb-ops"))
            .args(args)
            .output()
            .unwrap()
    };
    let dbs = db.to_str().unwrap();
    let ts = t.to_str().unwrap();
    let hs = h.to_str().unwrap();
    let imported = run(&[
        "references",
        "import",
        "--assets",
        dbs,
        "--provider",
        "ana",
        "--thresholds",
        ts,
        "--history",
        hs,
        "--json",
    ]);
    assert!(imported.status.success());
    let report: serde_json::Value = serde_json::from_slice(&imported.stdout).unwrap();
    assert_eq!(report["result"]["imported"], 1);
    assert!(imported.stderr.is_empty());
    let before = ObservationStore::open_read_only(&db)
        .unwrap()
        .read_level_references()
        .unwrap();
    let unknown = run(&[
        "references",
        "import",
        "--assets",
        dbs,
        "--provider",
        "anaa",
        "--thresholds",
        ts,
        "--history",
        hs,
        "--json",
    ]);
    let missing = run(&[
        "references",
        "import",
        "--assets",
        d.path().join("missing").to_str().unwrap(),
        "--provider",
        "ana",
        "--thresholds",
        ts,
        "--history",
        hs,
        "--json",
    ]);
    fs::write(&h, "station_code,level_registered,date\n1,3,invalid\n").unwrap();
    let malformed = run(&[
        "references",
        "import",
        "--assets",
        dbs,
        "--provider",
        "ana",
        "--thresholds",
        ts,
        "--history",
        hs,
        "--json",
    ]);
    for output in [unknown, missing, malformed] {
        assert_eq!(output.status.code(), Some(1));
        let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(value["ok"], false);
        assert!(
            value["error"]
                .as_str()
                .is_some_and(|error| !error.is_empty())
        );
        assert!(output.stderr.is_empty());
    }
    assert_eq!(
        ObservationStore::open_read_only(&db)
            .unwrap()
            .read_level_references()
            .unwrap(),
        before
    );
    let invalid_args = run(&["references", "import", "--json"]);
    assert_eq!(invalid_args.status.code(), Some(2));
}

fn corrupt_references(root: &std::path::Path, invalid_kind: bool) {
    use sha2::{Digest, Sha256};
    pollster::block_on(async {
        let db = turso::Builder::new_local(root.to_str().unwrap())
            .build()
            .await
            .unwrap();
        let conn = db.connect().unwrap();
        let mut rows=conn.query("SELECT payload FROM observation_metadata JOIN observation_revision ON observation_metadata.key=observation_revision.metadata_key ORDER BY id DESC LIMIT 1",()).await.unwrap();
        let row = rows.next().await.unwrap().unwrap();
        let turso::Value::Text(payload) = row.get_value(0).unwrap() else {
            panic!("metadata text");
        };
        drop(rows);
        let mut metadata: serde_json::Value = serde_json::from_str(&payload).unwrap();
        metadata["references"] = if invalid_kind {
            serde_json::json!([{"station_id":"a","kind":"historical","date":null,"level_cm":1.0}])
        } else {
            serde_json::json!([{"station_id":"a","kind":"alert","date":null,"level_cm":1.0},{"station_id":"a","kind":"alert","date":null,"level_cm":2.0}])
        };
        let payload = serde_json::to_string(&metadata).unwrap();
        let key = format!("{:x}", Sha256::digest(payload.as_bytes()));
        conn.execute(
            "INSERT INTO observation_metadata VALUES (?1,?2)",
            (key.clone(), payload),
        )
        .await
        .unwrap();
        conn.execute("UPDATE observation_revision SET metadata_key=?1 WHERE id=(SELECT MAX(id) FROM observation_revision)",[key]).await.unwrap();
    });
}
