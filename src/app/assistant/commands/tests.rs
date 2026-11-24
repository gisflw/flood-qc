use super::{CommandContext, LocationKind, LocationSearch, MAX_POINTS, dispatch, location_search};
use crate::app::{Draft, InspectionRequest, Session};
use crate::store::{
    BasinGeometry, IntervalAxis, ObservationBatch, ObservationStore, OperationManifest, Provenance,
    ResultSeries, ScenarioRecord, Station, StationResults, TimeInterval, new_run_stage_for,
    publish_run, write_result_series, write_run_manifest, write_station_results,
};
use chrono::{Duration, NaiveDateTime};
use serde_json::{Value, json};
use std::{
    fs,
    path::Path,
    sync::{Arc, atomic::AtomicBool},
};

fn fixture() -> (tempfile::TempDir, CommandContext) {
    fs::create_dir_all("runs").unwrap();
    let root = tempfile::tempdir_in("runs").unwrap();
    let directory = root.path().canonicalize().unwrap();
    let shared = Arc::new(super::Shared::new());
    shared.begin(1);
    let session = Session::new(InspectionRequest {
        workspace: directory.join("workspace"),
        invocation_dir: directory,
        config: None,
        gpkg_path: None,
        engine_executable: None,
        engine_input_dir: None,
    });
    fs::create_dir(session.workspace().workspace()).unwrap();
    (
        root,
        CommandContext::new(session, shared, 1, Arc::new(AtomicBool::default())),
    )
}

fn invoke(context: &CommandContext, id: &str, args: Value) -> super::CommandResult {
    dispatch(context, "run_command", json!({"id": id, "arguments": args}))
}

#[test]
fn catalog_and_typed_dispatch_agree_on_identifiers_and_arguments() {
    let (_root, context) = fixture();
    let all = dispatch(&context, "search_commands", json!({"query": ""})).unwrap();
    assert_eq!(all["commands"].as_array().unwrap().len(), 6);
    let matches = dispatch(
        &context,
        "search_commands",
        json!({"query": "working DRAFT identity"}),
    )
    .unwrap();
    assert_eq!(matches["commands"][0]["id"], "draft.read");
    assert_eq!(
        matches["commands"][0]["arguments_schema"]["additionalProperties"],
        false
    );

    for (id, args, expected) in [
        ("unknown", json!({}), "unknown_command"),
        (
            "workspace.inspect",
            json!({"path": "/other"}),
            "invalid_arguments",
        ),
        (
            "series.query",
            json!({"root": "/other"}),
            "invalid_arguments",
        ),
        (
            "location.search",
            json!({"query": "", "kind": null}),
            "invalid_arguments",
        ),
        ("draft.read", json!({}), "no_open_draft"),
        ("run.summary", json!({}), "no_selected_run"),
    ] {
        assert_eq!(invoke(&context, id, args).unwrap_err().code, expected);
    }

    let report = invoke(&context, "workspace.inspect", json!({})).unwrap();
    assert_eq!(report["findings"][1]["check"], "config");
    assert_eq!(report["findings"][1]["code"], "missing");
    let workspace = context.session.workspace();
    let mut draft = Draft::new(workspace.workspace().into(), workspace.workspace());
    draft.document.operation.forecast_horizon_days = Some(3);
    let version = context.session.set_draft(draft).unwrap();
    let report = invoke(&context, "draft.read", json!({})).unwrap();
    let expected_version = json!(version);
    assert_eq!(report["version"], expected_version);
    assert_eq!(report["schema_version"], 2);
    assert_eq!(report["period"]["forecast_horizon_days"], 3);
    let absence = report.get("sources").is_none();
    assert!(absence);
}

#[test]
fn ambiguous_station_matches_are_ordered_and_report_truncation() {
    let (_root, context) = fixture();
    let data = context.session.workspace().workspace().join("data");
    fs::create_dir(&data).unwrap();
    let mut store = ObservationStore::create(&data.join("history.sqlite"), "UTC").unwrap();
    store
        .write_batch(&ObservationBatch {
            stations: ["z", "a", "y"]
                .into_iter()
                .map(|id| Station {
                    station_id: id.into(),
                    provider_code: "example".into(),
                    station_code: id.into(),
                    name: "Rio Claro".into(),
                    latitude: None,
                    longitude: None,
                })
                .collect(),
            ..ObservationBatch::default()
        })
        .unwrap();
    let report = location_search(
        &context.session,
        LocationSearch {
            query: "riO".into(),
            kind: Some(LocationKind::Station),
            limit: 2,
        },
    )
    .unwrap();
    assert_eq!(report["matches"][0]["location"]["station"], "a");
    assert_eq!(report["matches"][1]["location"]["station"], "y");
    assert_eq!(report["more"], true);
    let id_match = invoke(
        &context,
        "location.search",
        json!({"query": "z", "kind": "station"}),
    )
    .unwrap();
    assert_eq!(id_match["matches"][0]["location"]["station"], "z");
    assert_eq!(id_match["more"], false);
}

fn scenario(id: &str) -> ScenarioRecord {
    ScenarioRecord {
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
    }
}

fn write_run(root: &Path, step_minutes: i64) {
    let reference = "2026-10-03T00:00:00".parse::<NaiveDateTime>().unwrap();
    let len = MAX_POINTS + 4;
    let timeline = IntervalAxis {
        time_zone: "America/Sao_Paulo".into(),
        bounds: (0..len)
            .map(|index| {
                let start = reference + Duration::minutes(index as i64 * step_minutes);
                TimeInterval {
                    start,
                    end: start + Duration::minutes(step_minutes),
                }
            })
            .collect(),
    };
    let provenance = Provenance {
        source_ids: vec!["test".into()],
        processing: json!({
            "reference_time": "2026-10-03T00:00:00",
            "engine_sha256": "a".repeat(64), "snapshot_sha256": "b".repeat(64),
        }),
    };
    let good = root.join("results/healthy");
    fs::create_dir_all(&good).unwrap();
    let mut rain_mask = vec![true; len];
    rain_mask[1] = false;
    write_result_series(
        &good.join("basin.nc"),
        &ResultSeries {
            timeline: timeline.clone(),
            basins: BasinGeometry {
                mini_ids: vec![1],
                longitudes: vec![-51.0],
                latitudes: vec![-30.0],
                source: Provenance {
                    source_ids: vec!["geometry".into()],
                    processing: Value::Null,
                },
            },
            time_segment: vec![1; len],
            precipitation: vec![0.0; len],
            precipitation_valid: rain_mask,
            flow: vec![0.0; len],
            flow_valid: vec![true; len],
            raw_level_cm: vec![-12; len],
            raw_level_valid: vec![true; len],
            provenance: provenance.clone(),
        },
    )
    .unwrap();
    write_station_results(
        &good.join("station.nc"),
        &StationResults {
            timeline,
            time_segment: vec![1; len],
            station_ids: vec!["station".into()],
            mini_ids: vec![Some(1)],
            offsets_m: vec![Some(0.5)],
            level_cm: vec![-62; len],
            level_valid: vec![true; len],
            provenance,
        },
    )
    .unwrap();
    write_run_manifest(
        root,
        &OperationManifest {
            schema_version: 1,
            preparation_sha256: None,
            state: "completed".into(),
            recipe: Value::Null,
            inputs: Vec::new(),
            scenarios: vec![scenario("broken"), scenario("healthy")],
            created_at: "2026-10-03T00:00:00".into(),
            finished_at: None,
            error: None,
        },
    )
    .unwrap();
}

fn series_query(location: Value, variable: &str, end: &str, expected: Option<Value>) -> Value {
    let mut args = json!({
        "scenario_id": "healthy", "location": location, "variable": variable,
        "start": "2026-10-03T00:00:00", "end": end
    });
    if let Some(expected) = expected {
        args["expected_run"] = expected;
    }
    args
}

#[test]
fn read_commands_preserve_retained_identity_timezone_masks_and_limits() {
    let (root, context) = fixture();
    let workspace = context.session.workspace().workspace();
    let active = new_run_stage_for(workspace, "active").unwrap();
    write_run(&active, 60);
    publish_run(workspace, &active).unwrap();
    let report = invoke(&context, "run.summary", json!({})).unwrap();
    assert_eq!(report["reference_time"], "2026-10-03T00:00:00");
    assert_eq!(report["scenarios"][0]["basin_results"], false);
    let fault = report["scenarios"][0]["findings"][0].is_string();
    assert!(fault);
    assert_eq!(report["scenarios"][1]["basin_results"], true);
    assert_eq!(report["scenarios"][1]["station_results"], true);
    let identity = report["identity"].clone();

    let mini = json!({"mini": 1});
    let query = |location, variable, end, expected| {
        invoke(
            &context,
            "series.query",
            series_query(location, variable, end, expected),
        )
    };
    let rain = query(
        mini.clone(),
        "rain",
        "2026-10-10T00:00:00",
        Some(identity.clone()),
    )
    .unwrap();
    assert_eq!(rain["identity"], identity);
    assert_eq!(rain["report"]["scenario_id"], "healthy");
    assert_eq!(rain["report"]["time_zone"], "America/Sao_Paulo");
    let data = &rain["report"]["series"][0];
    assert_eq!(data["unit"], "mm");
    let points = data["points"].as_array().unwrap();
    assert_eq!(points.len(), MAX_POINTS);
    assert_eq!(points[0]["timestamp"], "2026-10-03T01:00:00");
    assert_eq!(points[0]["value"], 0.0);
    assert!(points[1]["value"].is_null());
    assert_eq!(
        query(mini.clone(), "rain", "2026-10-10T01:00:00", None)
            .unwrap_err()
            .code,
        "series_limit"
    );
    assert_eq!(
        query(mini.clone(), "flow", "2026-10-10T00:00:00", None)
            .unwrap_err()
            .code,
        "series_limit"
    );
    let raw = query(mini.clone(), "level", "2026-10-03T01:00:00", None).unwrap();
    let station = query(
        json!({"station": "station"}),
        "level",
        "2026-10-03T01:00:00",
        None,
    )
    .unwrap();
    assert_eq!(raw["report"]["series"][0]["series_kind"], "model_raw_level");
    assert_eq!(raw["report"]["series"][0]["points"][0]["value"], -12.0);
    assert_eq!(station["report"]["series"][0]["unit"], "cm");
    assert_eq!(station["report"]["series"][0]["points"][0]["value"], -62.0);

    let next = new_run_stage_for(workspace, "next").unwrap();
    write_run(&next, 30);
    publish_run(workspace, &next).unwrap();
    assert_eq!(
        query(mini.clone(), "flow", "2026-10-03T01:00:00", Some(identity))
            .unwrap_err()
            .code,
        "stale_run"
    );
    assert_eq!(
        query(mini.clone(), "flow", "2026-10-06T13:00:00", None)
            .unwrap_err()
            .code,
        "series_limit"
    );

    let active_identity = invoke(&context, "run.summary", json!({})).unwrap()["identity"].clone();
    let saved = root.path().canonicalize().unwrap().join("saved");
    write_run(&saved, 60);
    let candidate = context.session.run().select_saved(&saved).unwrap();
    context.session.select_run(candidate).unwrap();
    assert_eq!(
        query(
            mini.clone(),
            "flow",
            "2026-10-03T01:00:00",
            Some(active_identity)
        )
        .unwrap_err()
        .code,
        "stale_run"
    );
    let saved_identity = invoke(&context, "run.summary", json!({})).unwrap()["identity"].clone();
    fs::remove_dir_all(&saved).unwrap();
    let stale_assertion = query(
        mini.clone(),
        "flow",
        "2026-10-03T01:00:00",
        Some(json!({"Active": next})),
    )
    .unwrap_err();
    assert_eq!(stale_assertion.code, "stale_run");
    context
        .session
        .select_run(context.session.run().follow_active())
        .unwrap();
    fs::remove_file(workspace.join("data/cache/run/active.json")).unwrap();
    assert_eq!(
        query(
            mini.clone(),
            "flow",
            "2026-10-03T01:00:00",
            Some(saved_identity)
        )
        .unwrap_err()
        .code,
        "stale_run"
    );
}
