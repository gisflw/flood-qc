use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use chrono::{Duration, NaiveDateTime};
use mgb_ops::analysis::{
    HydrographAdjustment, HydrographPoint, HydrographThreshold, ObservationBias, RatingPoint,
    analyze_model_hydrograph, rating_value,
};
use mgb_ops::app::{
    BulletinRequest, ReturnPeriodRequest, export_return_period, generate_bulletin, preview_bulletin,
};
use mgb_ops::store::{
    BasinGeometry, IntervalAxis, LevelReference, ObservationBatch, ObservationSample,
    ObservationSeries, ObservationStore, ObservationValue, OperationManifest, Provenance,
    ResultSeries, ScenarioRecord, Station, StationOffset, StationResults, TimeInterval,
    read_map_layers, write_result_series, write_station_results,
};

fn reference() -> NaiveDateTime {
    "2026-01-02T06:00:00".parse().unwrap()
}

#[test]
fn ar1_and_recent_observation_bias_preserve_raw_and_missing_values() {
    let reference = reference();
    let point = |hour, raw, segment| HydrographPoint {
        timestamp: reference + Duration::hours(hour),
        raw,
        value: None,
        lower: None,
        upper: None,
        segment: Some(segment),
    };
    let points = vec![
        point(-2, Some(10.0), 0),
        point(0, Some(10.0), 0),
        point(1, None, 1),
        point(2, Some(30.0), 1),
    ];
    let obs = BTreeMap::from([
        (reference - Duration::hours(2), 20.0),
        (reference, 15.0),
        (reference + Duration::hours(2), 1234.0),
    ]);
    let adjustment: HydrographAdjustment =
        serde_json::from_str(r#"{"bias":{"kind":"ar1"},"offset":1.0,"band_percent":10.0}"#)
            .unwrap();
    let thresholds = [HydrographThreshold {
        name: "Alerta".into(),
        value: 35.0,
    }];
    let (adjusted, peak, anchor) = analyze_model_hydrograph(
        points.clone(),
        &obs,
        reference,
        true,
        &adjustment,
        &thresholds,
    )
    .unwrap();
    let expected = 31.0 + (10.0 * 0.97_f64.powi(2) + 0.03 * 5.0) * 0.97_f64.powi(2);
    assert!((adjusted[3].value.unwrap() - expected).abs() < 1e-12);
    assert!((adjusted[3].lower.unwrap() - expected * 0.9).abs() < 1e-12);
    assert_eq!(adjusted[3].raw, Some(30.0));
    assert_eq!(adjusted[2].value, None);
    assert_eq!(anchor, Some(reference));
    assert_eq!(peak.unwrap().threshold.as_deref(), Some("Alerta"));
    let (recent, ..) = analyze_model_hydrograph(
        points,
        &obs,
        reference,
        true,
        &HydrographAdjustment {
            bias: ObservationBias::Recent { max_age_hours: 1 },
            ..Default::default()
        },
        &[],
    )
    .unwrap();
    assert_eq!(recent[3].value, Some(35.0));
    let curve = [
        RatingPoint {
            level_cm: -20.0,
            flow_m3s: 0.0,
        },
        RatingPoint {
            level_cm: 80.0,
            flow_m3s: 40.0,
        },
    ];
    assert_eq!(rating_value(10.0, true, &curve), Some(5.0));
    assert_eq!(rating_value(5.0, false, &curve), Some(10.0));
    assert_eq!(rating_value(81.0, false, &curve), None);
}

fn trial(root: &Path) {
    let scenario = root.join("results/forecast");
    fs::create_dir_all(&scenario).unwrap();
    let reference = reference();
    let mut provenance = Provenance {
        source_ids: vec!["fixture".into()],
        processing: serde_json::json!({
            "reference_time": reference.format("%Y-%m-%dT%H:%M:%S").to_string(),
            "engine_sha256": "a".repeat(64),
            "snapshot_sha256": "b".repeat(64),
        }),
    };
    let timeline = IntervalAxis {
        time_zone: "UTC".into(),
        bounds: (-24..24)
            .map(|offset| TimeInterval {
                start: reference + Duration::hours(offset),
                end: reference + Duration::hours(offset + 1),
            })
            .collect(),
    };
    let time_segment = (0..48).map(|i| u8::from(i >= 24)).collect::<Vec<_>>();
    let level_cm = (0..48).map(|i| 97 + i).collect::<Vec<_>>();
    let basin = ResultSeries {
        timeline: timeline.clone(),
        basins: BasinGeometry {
            mini_ids: vec![1],
            longitudes: vec![-52.0],
            latitudes: vec![-30.0],
            source: provenance.clone(),
        },
        time_segment: time_segment.clone(),
        precipitation: vec![1.0; 48],
        precipitation_valid: vec![true; 48],
        flow: (1..=48).map(|hour| f64::from(hour - 24)).collect(),
        flow_valid: vec![true; 48],
        raw_level_cm: level_cm.clone(),
        raw_level_valid: vec![true; 48],
        provenance: provenance.clone(),
    };
    write_result_series(&scenario.join("basin.nc"), &basin).unwrap();
    provenance.source_ids = vec!["fixture".into()];
    let mut station = StationResults {
        timeline,
        time_segment,
        station_ids: vec!["s".into()],
        mini_ids: vec![Some(1)],
        offsets_m: vec![Some(0.0)],
        level_cm,
        level_valid: vec![true; 48],
        provenance,
    };
    write_station_results(&scenario.join("station.nc"), &station).unwrap();
    let upper = root.join("results/upper");
    fs::create_dir(&upper).unwrap();
    let mut basin = basin;
    for level in &mut basin.raw_level_cm[24..] {
        *level += 10;
    }
    for level in &mut station.level_cm[24..] {
        *level += 10;
    }
    station.level_valid[35] = false;
    station.level_cm[35] = 0;
    write_result_series(&upper.join("basin.nc"), &basin).unwrap();
    write_station_results(&upper.join("station.nc"), &station).unwrap();
    let manifest = OperationManifest {
        preparation_sha256: None,
        state: "completed".into(),
        recipe: serde_json::Value::Null,
        inputs: vec![],
        scenarios: ["forecast", "upper"]
            .into_iter()
            .map(|id| ScenarioRecord {
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
            })
            .collect(),
        created_at: String::new(),
        finished_at: None,
        error: None,
    };
    mgb_ops::store::write_draft(&root.join("manifest.toml"), &manifest).unwrap();
}

fn observation_db(path: &Path) {
    let reference = reference();
    let mut store = ObservationStore::create(path, "UTC").unwrap();
    let series = |id: &str, variable: &str, unit: &str, interval_seconds| ObservationSeries {
        series_id: id.into(),
        station_id: "s".into(),
        variable: variable.into(),
        state: "raw".into(),
        unit: unit.into(),
        interval_seconds,
        created_at: reference - Duration::days(2),
    };
    let mut samples = (-23..=0)
        .map(|hour| ObservationSample {
            series_id: "rain".into(),
            observed_at: reference + Duration::hours(hour),
            value: Some(ObservationValue::RainMm(1.0)),
        })
        .collect::<Vec<_>>();
    samples.extend(
        [(-24, 100), (-1, 120), (0, 130)].map(|(hour, value)| ObservationSample {
            series_id: "level".into(),
            observed_at: reference + Duration::hours(hour),
            value: Some(ObservationValue::LevelCm(value)),
        }),
    );
    store
        .write_batch(&ObservationBatch {
            stations: vec![Station {
                station_id: "s".into(),
                provider_code: "ana".into(),
                station_code: "1".into(),
                name: "Estação São João".into(),
                latitude: Some(-30.0),
                longitude: Some(-52.0),
            }],
            series: vec![
                series("rain", "rain", "mm", Some(3600)),
                series("level", "level", "cm", None),
            ],
            samples,
            offsets: vec![StationOffset {
                station_id: "s".into(),
                mini_id: 1,
                offset_m: 0.0,
            }],
            ..Default::default()
        })
        .unwrap();

    store
        .replace_level_references(&[LevelReference {
            station_id: "s".into(),
            kind: "flood".into(),
            date: None,
            level_cm: 125.0,
        }])
        .unwrap();
}

#[test]
fn retained_bulletin_reuses_values_across_preview_documents_csv_and_gis() {
    let scratch = tempfile::tempdir().unwrap();
    let root = scratch.path();
    let geometry = root.join("MINI.gtp");
    fs::write(
        &geometry,
        "Mini Xcen Ycen Area_(km2) MiniJus\n1 -52 -30 3 -1\n",
    )
    .unwrap();
    let polygons = root.join("catchments.geojson");
    fs::write(&polygons, r#"{"type":"FeatureCollection","features":[{"type":"Feature","properties":{"mini_id":1},"geometry":{"type":"Polygon","coordinates":[[[-53,-31],[-51,-31],[-51,-29],[-53,-29],[-53,-31]]]}}]}"#).unwrap();
    let layer = read_map_layers(&polygons).unwrap().remove(0).name;
    let profile = root.join("profile.toml");
    fs::write(
        &profile,
        format!(
            r#"
title = "Boletim de São João"
basin = "Bacia de teste"
outlet_mini_id = 1
history_hours = 24
forecast_lead_hours = [6, 12]
rainfall_windows = [-24, 24]
catchment_layer = "{layer}"

[[stations]]
station_id = "s"
level_series = "level"
rainfall_series = "rain"
river = "Rio São João"
municipality = "Açúcar"

[stations.adjustment.bias]
kind = "recent"
max_age_hours = 24
"#
        ),
    )
    .unwrap();
    let request = BulletinRequest {
        trial: root.join("trial"),
        scenario_id: "forecast".into(),
        observations: root.join("observations"),
        geometry: geometry.clone(),
        map_geometry: Some(polygons.clone()),
        profile,
        output: root.join("bulletin"),
    };
    trial(&request.trial);
    observation_db(&request.observations);

    let preview = preview_bulletin(&request).unwrap();
    assert!(!request.output.exists());
    assert_eq!(preview.content.tables[0].rows[0].get(3).unwrap(), "24.0");
    assert_eq!(preview.content.tables[0].rows[0].get(6).unwrap(), "30.0");
    assert_eq!(
        preview.content.tables[0].rows[0].get(8).unwrap(),
        "Inundação"
    );
    assert_eq!(preview.content.tables[0].rows[0].get(10).unwrap(), "136.0");
    assert_eq!(preview.content.tables[1].rows[0].last().unwrap(), "24.0");
    let exported = generate_bulletin(&request).unwrap();
    assert!(fs::read(&exported.pdf).unwrap().starts_with(b"%PDF-"));
    assert!(
        fs::read_to_string(&exported.html)
            .unwrap()
            .contains("Estação São João")
    );
    assert!(
        fs::read_to_string(&exported.markdown)
            .unwrap()
            .contains("](chuva-2.png)")
    );
    assert!(
        fs::read_to_string(request.output.join("estacoes.csv"))
            .unwrap()
            .contains("136.0")
    );
    assert!(
        exported
            .figures
            .iter()
            .all(|path| fs::read(path).unwrap().starts_with(b"\x89PNG"))
    );
    assert!(generate_bulletin(&request).is_err());
    let primary_profile = fs::read_to_string(&request.profile).unwrap();
    fs::write(
        &request.profile,
        primary_profile.replace(
            "[[stations]]",
            "comparison_scenarios = [\"upper\"]\n\n[[stations]]",
        ),
    )
    .unwrap();
    let comparison = preview_bulletin(&request).unwrap();
    let summary = &comparison.content.tables[0];
    let cell = |label| {
        let index = summary.columns.iter().position(|col| col == label).unwrap();
        summary.rows[0][index].as_str()
    };
    assert_eq!(cell("Nível +6 h (cm)"), "136.0");
    assert_eq!(cell("Faixa +6 h (cm)"), "136.0 a 146.0 (2/2)");
    assert_eq!(cell("Faixa +12 h (cm)"), "142.0 a 142.0 (1/2)");
    assert_eq!(
        cell("Tendência +12 h"),
        "Subindo (+12.0 cm) · observado 02/01 06:00 até 02/01 18:00"
    );
    assert_eq!(
        comparison.content.figures.len(),
        preview.content.figures.len()
    );

    assert!(
        mgb_ops::analysis::bulletin_pdf(&comparison.content)
            .unwrap()
            .starts_with(b"%PDF-")
    );
    let thresholds = root.join("thresholds.toml");
    fs::write(&thresholds, "[[mini]]\nmini_id=1\n[[mini.thresholds]]\nyears=2.0\nflow_m3s=4.0\n[[mini.thresholds]]\nyears=10.0\nflow_m3s=24.0\n").unwrap();
    let mut gis = ReturnPeriodRequest {
        trial: request.trial,
        scenario_id: request.scenario_id,
        geometry: polygons,
        topology: geometry,
        layer,
        thresholds,
        destination: root.join("return-period.geojson"),
        forecast_day: 1,
    };
    let classified = export_return_period(&gis).unwrap();
    assert_eq!(classified.features[0].hours, 24);
    assert_eq!(classified.features[0].peak_m3s, Some(24.0));
    assert_eq!(classified.features[0].return_period_years, Some(10.0));
    let exported = wbvector::geojson::read(&gis.destination).unwrap();
    let field = exported.schema.field_index("return_period_years").unwrap();
    assert_eq!(exported.features[0].attributes[field].as_f64(), Some(10.0));
    gis.forecast_day = 2;
    let partial = export_return_period(&gis).unwrap();
    assert_eq!(partial.features[0].status, "incomplete");
    assert_eq!(partial.features[0].return_period_years, None);
}
