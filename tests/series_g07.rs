use chrono::{Duration, NaiveDate};
use mgb_ops::store::{
    BasinGeometry, IntervalAxis, MiniTopology, Provenance, ResultSeries, TimeInterval,
};
use mgb_ops::{HourlyWindow, accumulated_rain};
fn fixture() -> (ResultSeries, Vec<MiniTopology>, HourlyWindow) {
    let start = NaiveDate::from_ymd_opt(2026, 1, 1)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap();
    let provenance = Provenance {
        source_ids: vec![],
        processing: serde_json::Value::Null,
    };
    let result = ResultSeries {
        timeline: IntervalAxis {
            time_zone: "UTC".into(),
            bounds: (0..2)
                .map(|h| TimeInterval {
                    start: start + Duration::hours(h),
                    end: start + Duration::hours(h + 1),
                })
                .collect(),
        },
        basins: BasinGeometry {
            mini_ids: vec![1, 2],
            longitudes: vec![0., 1.],
            latitudes: vec![0., 1.],
            source: provenance.clone(),
        },
        time_segment: vec![0, 1],
        precipitation: vec![2., 10., 4., 20.],
        precipitation_valid: vec![true; 4],
        flow: vec![0.; 4],
        flow_valid: vec![true; 4],
        raw_level_cm: vec![0; 4],
        raw_level_valid: vec![true; 4],
        provenance,
    };
    let topology = vec![
        MiniTopology {
            mini_id: 1,
            downstream: Some(2),
            area_km2: 2.,
        },
        MiniTopology {
            mini_id: 2,
            downstream: None,
            area_km2: 5.,
        },
    ];
    (
        result,
        topology,
        HourlyWindow::Relative {
            end: start + Duration::hours(2),
            hours: 2,
        },
    )
}
#[test]
fn weighted_accumulation_and_missing_coverage() {
    let (mut result, topology, window) = fixture();
    let summary = accumulated_rain(&result, &topology, 2, true, &window).unwrap();
    assert_eq!(summary.mean_mm, Some(162. / 7.));
    assert_eq!(summary.covered_area_hours, 14.);
    result.precipitation_valid[3] = false;
    let summary = accumulated_rain(&result, &topology, 2, true, &window).unwrap();
    assert_eq!(summary.mean_mm, None);
    assert_eq!(summary.complete_area_km2, 2.);
    assert_eq!(summary.covered_area_hours, 9.);
}

#[test]
fn canonical_comparisons_are_isolated_and_csv_is_atomic() {
    use mgb_ops::store::{
        OperationManifest, ScenarioRecord, StationResults, write_result_series,
        write_station_results,
    };
    use mgb_ops::{
        Location, SeriesQuery, SeriesSelection, Variable, export_series_csv, query_series,
    };
    use std::fs;
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("run");
    fs::create_dir_all(root.join("results/good")).unwrap();
    let (mut basin, _, window) = fixture();
    let provenance = Provenance {
        source_ids: vec!["input".into()],
        processing: serde_json::json!({"reference_time":"2026-01-01T01:00:00", "engine_sha256":"a".repeat(64), "snapshot_sha256":"b".repeat(64)}),
    };
    basin.provenance = provenance.clone();
    basin.raw_level_cm[0] = -2;
    let station = StationResults {
        timeline: basin.timeline.clone(),
        time_segment: basin.time_segment.clone(),
        station_ids: vec!["A,é".into(), "002".into()],
        mini_ids: vec![Some(1), Some(2)],
        offsets_m: vec![Some(0.0), Some(0.1)],
        level_cm: vec![-1, 20, 0, 21],
        level_valid: vec![true; 4],
        provenance,
    };
    write_result_series(&root.join("results/good/basin.nc"), &basin).unwrap();
    write_station_results(&root.join("results/good/station.nc"), &station).unwrap();
    for id in ["reference", "offset", "mini", "coordinates"] {
        fs::create_dir_all(root.join("results").join(id)).unwrap();
        let mut other_basin = basin.clone();
        let mut other_station = station.clone();
        match id {
            "reference" => {
                other_basin.time_segment = vec![0, 0];
                other_station.time_segment = vec![0, 0];
                other_basin.provenance.processing["reference_time"] = "2026-01-01T02:00:00".into();
                other_station.provenance.processing["reference_time"] =
                    "2026-01-01T02:00:00".into();
            }
            "offset" => other_station.offsets_m[0] = Some(1.),
            "mini" => other_station.mini_ids[0] = Some(2),
            "coordinates" => other_basin.basins.longitudes[0] = 5.,
            _ => unreachable!(),
        }
        write_result_series(
            &root.join("results").join(id).join("basin.nc"),
            &other_basin,
        )
        .unwrap();
        write_station_results(
            &root.join("results").join(id).join("station.nc"),
            &other_station,
        )
        .unwrap();
    }
    let record = |id: &str| ScenarioRecord {
        id: id.into(),
        state: "completed".into(),
        started_at: None,
        finished_at: None,
        exit_code: None,
        signal: None,
        error: None,
        stdout: None,
        stderr: None,
        basin_result: None,
        station_result: None,
    };
    let manifest = OperationManifest {
        preparation_sha256: None,
        state: "completed".into(),
        recipe: serde_json::Value::Null,
        inputs: vec![],
        scenarios: ["good", "bad", "reference", "offset", "mini", "coordinates"]
            .iter()
            .map(|id| record(id))
            .collect(),
        created_at: "".into(),
        finished_at: None,
        error: None,
    };
    mgb_ops::store::write_draft(&root.join("manifest.toml"), &manifest).unwrap();
    let selection = |scenario: &str| SeriesSelection::Model {
        root: root.clone(),
        scenario_id: scenario.into(),
        variable: Variable::Level,
    };
    let mut request = SeriesQuery {
        location: Location::Station("A,é".into()),
        time_zone: "UTC".into(),
        window,
        selections: vec![selection("good"), selection("bad")],
    };
    let report = query_series(&request).unwrap();
    assert_eq!(report.series.len(), 1);
    assert_eq!(report.findings.len(), 1);
    assert_eq!(report.series[0].points[0].value, Some(-1.));
    request.selections = ["good", "reference", "offset", "mini", "coordinates", "bad"]
        .iter()
        .map(|id| selection(id))
        .collect();
    let report = query_series(&request).unwrap();
    assert_eq!(report.series.len(), 2);
    assert_eq!(
        report
            .findings
            .iter()
            .map(|f| f.selection_index)
            .collect::<Vec<_>>(),
        [2, 3, 4, 5]
    );
    assert_eq!(
        report.series[0].forecast_reference_time.as_deref(),
        Some("2026-01-01T01:00:00")
    );
    assert_eq!(
        report.series[1].forecast_reference_time.as_deref(),
        Some("2026-01-01T02:00:00")
    );
    assert_eq!(
        report.series[0].points[0].timestamp,
        report.series[1].points[0].timestamp
    );
    let mini_request = SeriesQuery {
        location: Location::Mini(1),
        selections: vec![selection("good")],
        ..request.clone()
    };
    let mini_report = query_series(&mini_request).unwrap();
    assert_eq!(mini_report.series[0].series_kind, "model_raw_level");
    assert_eq!(mini_report.series[0].points[0].value, Some(-2.));
    assert_eq!(mini_report.series[0].points[1].value, Some(0.));
    let destination = dir.path().join("out.csv");
    fs::write(&destination, b"preserve").unwrap();
    assert!(export_series_csv(&request, &destination).is_err());
    assert_eq!(fs::read(&destination).unwrap(), b"preserve");
    request.selections = vec![selection("good")];
    export_series_csv(&request, &destination).unwrap();
    let mut csv = csv::Reader::from_path(&destination).unwrap();
    let rows: Vec<_> = csv.records().map(Result::unwrap).collect();
    assert_eq!(&rows[0][3], "A,é");
    assert_eq!(&rows[0][10], "-1");
    assert_eq!(&rows[1][10], "0");

    use mgb_ops::app::{export_comparison_csv, query_comparison};
    use mgb_ops::store::{
        ObservationBatch, ObservationSample, ObservationSeries, ObservationStore, ObservationValue,
        Station,
    };
    let path = dir.path().join("observations");
    let start = basin.timeline.bounds[0].start;
    let mut database = ObservationStore::create(&path, "UTC").unwrap();
    for (id, value) in [("A,é", 10), ("002", 19)] {
        let series_id = format!("{id}-level");
        database
            .write_batch(&ObservationBatch {
                stations: vec![Station {
                    station_id: id.into(),
                    station_code: id.into(),
                    name: id.into(),
                    provider_code: "test".into(),
                    latitude: None,
                    longitude: None,
                }],
                series: vec![ObservationSeries {
                    series_id: series_id.clone(),
                    station_id: id.into(),
                    variable: "level".into(),
                    unit: "cm".into(),
                    state: "raw".into(),
                    interval_seconds: None,
                    created_at: start,
                }],
                samples: vec![ObservationSample {
                    series_id,
                    observed_at: start,
                    value: Some(ObservationValue::LevelCm(value)),
                }],
                ..Default::default()
            })
            .unwrap();
    }
    let model_rain = SeriesSelection::Model {
        root: root.clone(),
        scenario_id: "good".into(),
        variable: Variable::Rain,
    };
    let observed = SeriesSelection::Observation {
        database: path,
        series_id: "002-level".into(),
        variable: Variable::Level,
        effective: false,
        replacement_ids: vec![],
        replacements: None,
    };
    let mut comparison = SeriesQuery {
        location: Location::Mini(2),
        selections: vec![model_rain, selection("good"), observed],
        ..mini_request
    };
    let report = query_comparison(&comparison, "002").unwrap();
    assert!(report.findings.is_empty());
    assert_eq!(
        report
            .series
            .iter()
            .map(|s| s.points[0].value)
            .collect::<Vec<_>>(),
        [Some(10.), Some(20.), Some(19.)],
    );
    assert_eq!(report.series[1].series_kind, "model_station_level");
    export_comparison_csv(&report, Some("002"), &destination).unwrap();
    let csv = csv::Reader::from_path(&destination)
        .unwrap()
        .into_records()
        .map(Result::unwrap)
        .collect::<Vec<_>>();
    assert!(
        csv.iter()
            .filter(|row| &row[8] == "rain")
            .all(|row| &row[2] == "mini" && &row[3] == "2")
    );
    assert!(
        csv.iter()
            .filter(|row| &row[8] == "level")
            .all(|row| &row[2] == "station" && &row[3] == "002")
    );

    comparison.location = Location::Mini(1);
    comparison.selections.push(selection("bad"));
    let report = query_comparison(&comparison, "002").unwrap();
    assert_eq!(report.findings[0].selection_index, 3);
    assert_eq!(report.series[0].points[0].value, Some(2.));
    assert_eq!(report.series[1].series_kind, "model_raw_level");
    assert_eq!(report.series[1].points[0].value, Some(-2.));
    assert_eq!(report.series[2].points[0].value, Some(19.));

    // A corrupt station result must leave the model rainfall and recorded station observations usable.
    fs::write(root.join("results/good/station.nc"), "corrupt").unwrap();
    let report = query_comparison(&comparison, "002").unwrap();
    assert_eq!(
        report
            .findings
            .iter()
            .map(|f| f.selection_index)
            .collect::<Vec<_>>(),
        [1, 3]
    );
    assert_eq!(
        report
            .series
            .iter()
            .map(|s| s.selection_index)
            .collect::<Vec<_>>(),
        [0, 2]
    );
}

#[test]
fn observed_endpoints_and_supplied_rain_edits() {
    use mgb_ops::{
        Location, ObservationBatch, ObservationReplacement, ObservationSample, ObservationSeries,
        ObservationStore, ObservationValue, ReplacementKind, SeriesQuery, SeriesSelection, Station,
        Variable, query_series,
    };
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("observations");
    let (_, _, window) = fixture();
    let start = NaiveDate::from_ymd_opt(2026, 1, 1)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap();
    let series = |id: &str, variable: &str, unit: &str, interval_seconds| ObservationSeries {
        series_id: id.into(),
        station_id: "s".into(),
        variable: variable.into(),
        state: "raw".into(),
        unit: unit.into(),
        interval_seconds,
        created_at: start,
    };
    let mut store = ObservationStore::create(&path, "UTC").unwrap();
    store
        .write_batch(&ObservationBatch {
            stations: vec![Station {
                station_id: "s".into(),
                provider_code: "p".into(),
                station_code: "1".into(),
                name: "S".into(),
                latitude: None,
                longitude: None,
            }],
            series: vec![
                series("l", "level", "cm", None),
                series("f", "flow", "m3/s", None),
                series("r", "rain", "mm", Some(3600)),
            ],
            samples: vec![
                ObservationSample {
                    series_id: "l".into(),
                    observed_at: start,
                    value: Some(ObservationValue::LevelCm(-3)),
                },
                ObservationSample {
                    series_id: "l".into(),
                    observed_at: start + Duration::hours(2),
                    value: Some(ObservationValue::LevelCm(0)),
                },
                ObservationSample {
                    series_id: "f".into(),
                    observed_at: start,
                    value: Some(ObservationValue::FlowM3s(1.234567890123)),
                },
                ObservationSample {
                    series_id: "f".into(),
                    observed_at: start + Duration::hours(2),
                    value: Some(ObservationValue::FlowM3s(0.)),
                },
                ObservationSample {
                    series_id: "r".into(),
                    observed_at: start + Duration::hours(1),
                    value: Some(ObservationValue::RainMm(0.)),
                },
            ],
            ..Default::default()
        })
        .unwrap();
    drop(store);
    let make = |id: &str, variable, effective, replacements| SeriesSelection::Observation {
        database: path.clone(),
        series_id: id.into(),
        variable,
        effective,
        replacement_ids: vec![],
        replacements,
    };
    let edit = ObservationReplacement {
        replacement_id: "draft".into(),
        station_id: "s".into(),
        variable: "rain".into(),
        kind: ReplacementKind::Point,
        start_at: start + Duration::hours(1),
        end_at: None,
        value: Some(4.),
        reason: "".into(),
    };
    let request = SeriesQuery {
        location: Location::Station("s".into()),
        time_zone: "UTC".into(),
        window,
        selections: vec![
            make("l", Variable::Level, false, None),
            make("f", Variable::Flow, false, None),
            make("r", Variable::Rain, false, None),
            make("r", Variable::Rain, true, Some(vec![edit])),
        ],
    };
    let report = query_series(&request).unwrap();
    assert!(report.findings.is_empty(), "{:?}", report.findings);
    assert_eq!(
        report.series[0]
            .points
            .iter()
            .map(|p| p.value)
            .collect::<Vec<_>>(),
        [Some(-3.), Some(0.)]
    );
    assert_eq!(
        report.series[1]
            .points
            .iter()
            .map(|p| p.value)
            .collect::<Vec<_>>(),
        [Some(1.234567890123), Some(0.)]
    );
    assert_eq!(
        report.series[2]
            .points
            .iter()
            .map(|p| p.value)
            .collect::<Vec<_>>(),
        [Some(0.), None]
    );
    assert_eq!(
        report.series[3]
            .points
            .iter()
            .map(|p| p.value)
            .collect::<Vec<_>>(),
        [Some(4.), None]
    );
    let request_path = dir.path().join("request.json");
    std::fs::write(&request_path, serde_json::to_vec(&request).unwrap()).unwrap();
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_mgb-ops"))
        .args(["series", "query", "--request"])
        .arg(&request_path)
        .arg("--json")
        .output()
        .unwrap();
    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["result"], serde_json::to_value(&report).unwrap());
    let api_path = dir.path().join("api.csv");
    let cli_path = dir.path().join("cli.csv");
    mgb_ops::export_series_csv(&request, &api_path).unwrap();
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_mgb-ops"))
        .args(["series", "export", "--request"])
        .arg(&request_path)
        .arg("--output")
        .arg(&cli_path)
        .arg("--json")
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&output.stdout).unwrap()["ok"],
        true
    );
    assert_eq!(
        std::fs::read(&api_path).unwrap(),
        std::fs::read(&cli_path).unwrap()
    );
    let mut reader = csv::Reader::from_path(&api_path).unwrap();
    assert_eq!(
        reader.headers().unwrap().iter().collect::<Vec<_>>(),
        [
            "timestamp",
            "time_zone",
            "location_kind",
            "location_id",
            "series_kind",
            "run_id",
            "scenario_id",
            "forecast_reference_time",
            "variable",
            "unit",
            "value"
        ]
    );
    let rows = reader.records().map(Result::unwrap).collect::<Vec<_>>();
    assert_eq!(
        rows.iter()
            .map(|r| (&r[0], &r[8], &r[10]))
            .collect::<Vec<_>>(),
        [
            ("2026-01-01T00:00:00", "level", "-3"),
            ("2026-01-01T00:00:00", "flow", "1.234567890123"),
            ("2026-01-01T01:00:00", "rain", "0"),
            ("2026-01-01T01:00:00", "rain", "4"),
            ("2026-01-01T02:00:00", "level", "0"),
            ("2026-01-01T02:00:00", "flow", "0"),
            ("2026-01-01T02:00:00", "rain", ""),
            ("2026-01-01T02:00:00", "rain", "")
        ]
    );
    assert!(
        rows.iter()
            .all(|r| r.len() == 11 && r[5].is_empty() && r[6].is_empty() && r[7].is_empty())
    );
    let previous = std::fs::read(&cli_path).unwrap();
    let mut bad_request = request.clone();
    bad_request.selections.push(SeriesSelection::Observation {
        database: dir.path().join("absent.sqlite"),
        series_id: "missing".into(),
        variable: Variable::Rain,
        effective: false,
        replacement_ids: vec![],
        replacements: None,
    });
    std::fs::write(&request_path, serde_json::to_vec(&bad_request).unwrap()).unwrap();
    let failed = std::process::Command::new(env!("CARGO_BIN_EXE_mgb-ops"))
        .args(["series", "export", "--request"])
        .arg(&request_path)
        .arg("--output")
        .arg(&cli_path)
        .arg("--json")
        .output()
        .unwrap();
    assert_eq!(failed.status.code(), Some(1));
    assert!(failed.stderr.is_empty());
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&failed.stdout).unwrap()["ok"],
        false
    );
    assert_eq!(std::fs::read(&cli_path).unwrap(), previous);
    assert!(
        ObservationStore::open_read_only(&path)
            .unwrap()
            .replacement_choices()
            .unwrap()
            .is_empty()
    );
}

#[test]
fn grid_native_intervals_are_not_prorated() {
    use mgb_ops::{
        accumulated_grid_rain,
        store::{GridSeries, RegularGridSpec},
    };
    let start = NaiveDate::from_ymd_opt(2026, 1, 1)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap();
    let grid = RegularGridSpec::new([0., 0., 1., 1.], 1.).unwrap();
    let n = grid.latitudes.len() * grid.longitudes.len();
    let series = GridSeries {
        timeline: IntervalAxis {
            time_zone: "UTC".into(),
            bounds: vec![
                TimeInterval {
                    start,
                    end: start + Duration::hours(3),
                },
                TimeInterval {
                    start: start + Duration::hours(3),
                    end: start + Duration::hours(6),
                },
            ],
        },
        grid,
        values: vec![2.; n * 2],
        valid: vec![true; n * 2],
        provenance: Provenance {
            source_ids: vec![],
            processing: serde_json::Value::Null,
        },
    };
    let whole = accumulated_grid_rain(
        &series,
        &HourlyWindow::Relative {
            end: start + Duration::hours(6),
            hours: 6,
        },
    )
    .unwrap();
    assert_eq!(whole.values[0], Some(4.));
    let partial = accumulated_grid_rain(
        &series,
        &HourlyWindow::Relative {
            end: start + Duration::hours(6),
            hours: 5,
        },
    )
    .unwrap();
    assert_eq!(partial.values[0], None);
    assert_eq!(partial.covered_hours[0], 3.);
    let mut invalid = series.clone();
    invalid.timeline.bounds[0].start += Duration::minutes(30);
    assert!(
        accumulated_grid_rain(
            &invalid,
            &HourlyWindow::Relative {
                end: start + Duration::hours(6),
                hours: 5
            }
        )
        .is_err()
    );
}

#[test]
fn windows_validate_labels_dst_and_locations() {
    use mgb_ops::{Location, SeriesQuery, query_series, resolve_hourly_window};
    let start = NaiveDate::from_ymd_opt(2026, 1, 1)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap();
    let relative = HourlyWindow::Relative {
        end: start + Duration::hours(2),
        hours: 2,
    };
    let explicit = HourlyWindow::Explicit {
        start,
        end: start + Duration::hours(2),
    };
    assert_eq!(
        resolve_hourly_window("UTC", &relative).unwrap(),
        resolve_hourly_window("UTC", &explicit).unwrap()
    );
    assert!(
        resolve_hourly_window(
            "UTC",
            &HourlyWindow::Explicit {
                start,
                end: start + Duration::minutes(30)
            }
        )
        .is_err()
    );
    let dst = NaiveDate::from_ymd_opt(2026, 3, 8)
        .unwrap()
        .and_hms_opt(1, 0, 0)
        .unwrap();
    assert!(
        resolve_hourly_window(
            "America/New_York",
            &HourlyWindow::Explicit {
                start: dst,
                end: dst + Duration::hours(2)
            }
        )
        .is_err()
    );
    for location in [Location::Station(" ".into()), Location::Mini(0)] {
        assert!(
            query_series(&SeriesQuery {
                location,
                time_zone: "UTC".into(),
                window: relative.clone(),
                selections: vec![]
            })
            .is_err()
        );
    }
}

#[test]
fn spatial_missing_zero_and_overflow_are_distinct() {
    let (mut result, topology, window) = fixture();
    result.precipitation.fill(0.);
    assert_eq!(
        accumulated_rain(&result, &topology, 2, true, &window)
            .unwrap()
            .mean_mm,
        Some(0.)
    );
    result.basins.mini_ids.pop();
    result.basins.longitudes.pop();
    result.basins.latitudes.pop();
    result.precipitation = vec![0., 0.];
    result.precipitation_valid = vec![true; 2];
    result.flow = vec![0.; 2];
    result.flow_valid = vec![true; 2];
    result.raw_level_cm = vec![0; 2];
    result.raw_level_valid = vec![true; 2];
    let missing = accumulated_rain(&result, &topology, 2, true, &window).unwrap();
    assert_eq!(missing.mean_mm, None);
    assert_eq!(missing.complete_area_km2, 2.);
    assert_eq!(missing.covered_area_hours, 4.);
    result.precipitation = vec![f64::MAX, f64::MAX];
    assert!(accumulated_rain(&result, &topology, 2, true, &window).is_err());
}
