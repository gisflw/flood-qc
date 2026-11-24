use super::super::{Shared, commands::dispatch};
use super::*;
use crate::app::{
    Draft, DraftCorrection, DraftForecasts, DraftScenario, DraftVisualization, InspectionRequest,
};
use crate::store::{
    CanonicalForecastMetadata, ForecastCorrection, GridSeries, IntervalAxis, Provenance,
    RegularGridSpec, TimeInterval, write_grid_series,
};
use std::{
    fs,
    path::Path,
    sync::{Arc, atomic::AtomicBool},
};

fn fixture() -> (tempfile::TempDir, Session, Draft) {
    fs::create_dir_all("runs").unwrap();
    let root = tempfile::tempdir_in("runs").unwrap();
    let workspace = root.path().canonicalize().unwrap();
    let session = Session::new(InspectionRequest {
        invocation_dir: workspace.clone(),
        workspace: workspace.clone(),
        config: None,
        gpkg_path: None,
        engine_executable: None,
        engine_input_dir: None,
    });
    let draft = Draft::new(workspace.clone(), &workspace);
    (root, session, draft)
}

fn args(session: &Session, edit: Value) -> Value {
    let mut edit = edit;
    let version = session.draft().unwrap().version;
    edit["version"] = serde_json::to_value(version).unwrap();
    edit
}

fn candidate(session: &Session, edit: Value) -> Result<Candidate, CommandFinding> {
    validate(session, parse(args(session, edit))?)
}

fn stage_edit(
    state: &mut ProposalReviewState,
    session: &Session,
    turn: u64,
    edit: Value,
) -> ProposalReview {
    state
        .stage(turn, candidate(session, edit).unwrap())
        .unwrap();
    state.current().unwrap()
}

#[test]
fn incomplete_draft_reviews_all_changed_fields_and_only_a_save_writes() {
    let (_root, session, mut draft) = fixture();
    draft.document.visualization = Some(DraftVisualization {
        selected_location: Some("mini:4".into()),
        ..DraftVisualization::default()
    });
    let path = draft.workspace.join("saved.json");
    draft.save(&path, true).unwrap();
    let saved = fs::read(&path).unwrap();
    let original = draft.document.clone();
    session.set_draft(draft).unwrap();

    let mut state = ProposalReviewState::default();
    let edit = json!({"observed_horizon_days": 0, "forecast_horizon_days": 5});
    let review = stage_edit(&mut state, &session, 1, edit);
    assert_eq!(
        review.changes,
        [
            DraftEdit::ObservedHorizonDays {
                current: None,
                proposed: 0
            },
            DraftEdit::ForecastHorizonDays {
                current: None,
                proposed: 5
            },
        ]
    );
    assert!(
        review
            .readiness
            .iter()
            .any(|message| message.contains("Set the draft reference time"))
    );
    assert_eq!(session.draft().unwrap().draft.document, original);
    assert_eq!(state.apply(&session, &review), ProposalOutcome::Applied);
    state.cancel_turn(1);
    let snapshot = session.draft().unwrap();
    let mut expected = original;
    expected.operation.observed_horizon_days = Some(0);
    expected.operation.forecast_horizon_days = Some(5);
    assert_eq!(snapshot.draft.document, expected);
    assert!(snapshot.draft.is_dirty());
    assert_eq!(fs::read(&path).unwrap(), saved);

    let job = session.begin_draft_job().unwrap();
    let mut saved_draft = job.input().unwrap().draft.clone();
    saved_draft.save(&path, true).unwrap();
    job.replace(&session, saved_draft).unwrap();
    let reopened = session.workspace().open_draft(&path).unwrap();
    assert_eq!(reopened.document, expected);
    let clean = !session.draft().unwrap().draft.is_dirty();
    assert!(clean);
    assert_eq!(fs::read_dir(path.parent().unwrap()).unwrap().count(), 1);
}

#[test]
fn proposal_is_typed_reports_no_change_and_preserves_explicit_legacy_periods() {
    let (_root, session, mut draft) = fixture();
    draft.document.operation.forecast_horizon_days = Some(5);
    session.set_draft(draft.clone()).unwrap();
    for edit in [
        json!({"observed_horizon_days": -1}),
        json!({"forecast_horizon_days": 3.5}),
        json!({"forecast_horizon_days": null}),
        json!({"workspace": "/elsewhere"}),
        json!({"forecast_asset":{"scenario_index":0,"asset_id":"next","corrections":[]}}),
    ] {
        let rejected = match parse::<ProposalArgs>(args(&session, edit)) {
            Ok(_) => panic!("argument should fail parsing"),
            Err(error) => error,
        };
        assert_eq!(rejected.code, "invalid_arguments");
    }
    for edit in [
        json!({"forecast_horizon_days": 0}),
        json!({"observed_horizon_days": u32::MAX}),
        json!({"forecast_asset":{"scenario_index":0,"asset_id":"next"}}),
    ] {
        let rejected = match candidate(&session, edit) {
            Ok(_) => panic!("argument should fail validation"),
            Err(error) => error,
        };
        assert_eq!(rejected.code, "invalid_proposal");
    }
    let mut state = ProposalReviewState::default();
    let edit = json!({"forecast_horizon_days": 5});
    let result = state.stage(1, candidate(&session, edit).unwrap()).unwrap();
    assert_eq!(result["status"], "unchanged");
    assert!(state.current().is_none());

    draft.document.schema_version = 1;
    draft.document.operation.forecast_horizon_days = None;
    draft.document.operation.historical_first_end = Some("2026-02-01T00:00:00".into());
    draft.document.operation.historical_last_end = Some("2026-02-02T00:00:00".into());
    draft.document.operation.forecast_first_end = Some("2026-02-02T01:00:00".into());
    draft.document.operation.forecast_last_end = Some("2026-02-03T00:00:00".into());
    let original = draft.document.clone();
    session.set_draft(draft).unwrap();
    let edit = json!({"observed_horizon_days": 1});
    let rejected = match candidate(&session, edit) {
        Ok(_) => panic!("legacy bounds are fixed"),
        Err(error) => error,
    };
    assert!(rejected.message.contains("Explicit period bounds"));
    assert_eq!(session.draft().unwrap().draft.document, original);
}

#[test]
fn review_is_bound_to_its_origin_and_busy_or_stale_drafts_are_guarded() {
    let (_root, session, draft) = fixture();
    session.set_draft(draft.clone()).unwrap();
    let mut state = ProposalReviewState::default();
    let edit = json!({"forecast_horizon_days": 5});
    let review = stage_edit(&mut state, &session, 1, edit.clone());
    let duplicate = candidate(&session, edit.clone()).unwrap();
    assert_eq!(
        state.stage(2, duplicate).unwrap_err().code,
        "proposal_pending"
    );
    state.cancel_turn(2);
    assert_eq!(state.current(), Some(review.clone()));

    let job = session.begin_draft_job().unwrap();
    assert_eq!(state.apply(&session, &review), ProposalOutcome::Busy);
    assert_eq!(state.current(), Some(review.clone()));
    drop(job);
    assert_eq!(state.apply(&session, &review), ProposalOutcome::Applied);

    session.set_draft(draft.clone()).unwrap();
    let review = stage_edit(&mut state, &session, 2, edit.clone());
    let snapshot = session.draft().unwrap();
    let mut edited = snapshot.draft.document;
    edited.operation.observed_horizon_days = Some(2);
    session
        .commit_form_edit(snapshot.version, edited.clone())
        .unwrap();
    assert_eq!(state.apply(&session, &review), ProposalOutcome::Stale);
    assert_eq!(session.draft().unwrap().draft.document, edited);

    let review = stage_edit(&mut state, &session, 3, edit.clone());
    session.set_draft(draft).unwrap();
    assert_eq!(state.apply(&session, &review), ProposalOutcome::Stale);
    let withdrawn = stage_edit(&mut state, &session, 4, edit.clone());
    state.cancel_turn(4);
    let review = stage_edit(&mut state, &session, 5, edit);
    assert_eq!(
        state.apply(&session, &withdrawn),
        ProposalOutcome::NoPending
    );
    assert_eq!(state.reject(&withdrawn), ProposalOutcome::NoPending);
    assert_eq!(state.reject(&review), ProposalOutcome::Rejected);
    assert!(state.current().is_none());
}

#[test]
fn discovered_command_persists_review_on_completion_and_withdraws_on_cancellation() {
    let (_root, session, draft) = fixture();
    session.set_draft(draft).unwrap();
    let shared = Arc::new(Shared::new());
    let cancelled = Arc::new(AtomicBool::new(false));
    let command_context =
        |turn| CommandContext::new(session.clone(), shared.clone(), turn, cancelled.clone());
    assert!(shared.begin(1));
    let first = command_context(1);
    let search = json!({"query": "propose_edit"});
    let discovered = dispatch(&first, "search_commands", search).unwrap();
    assert_eq!(discovered["commands"][0]["id"], "draft.propose_edit");
    assert_eq!(
        discovered["commands"][0]["arguments_schema"]["additionalProperties"],
        false
    );
    let read = json!({"id":"draft.read","arguments":{}});
    let draft = dispatch(&first, "run_command", read).unwrap();
    let proposal = json!({
        "id": "draft.propose_edit",
        "arguments": {"version": draft["version"], "forecast_horizon_days": 6}
    });
    let response = dispatch(&first, "run_command", proposal.clone()).unwrap();
    assert_eq!(response["status"], "staged");
    shared.finish(1, true);
    let review = shared.state.lock().unwrap().review.current().unwrap();

    assert!(shared.begin(2));
    let second = command_context(2);
    let error = dispatch(&second, "run_command", proposal.clone()).unwrap_err();
    assert_eq!(error.code, "proposal_pending");
    shared.finish(2, false);
    assert_eq!(
        shared.state.lock().unwrap().review.current(),
        Some(review.clone())
    );
    assert_eq!(
        shared.state.lock().unwrap().review.reject(&review),
        ProposalOutcome::Rejected
    );

    assert!(shared.begin(3));
    let third = command_context(3);
    dispatch(&third, "run_command", proposal.clone()).unwrap();
    shared.finish(3, false);
    assert_eq!(shared.state.lock().unwrap().review.current(), None);

    assert!(shared.begin(4));
    let fourth = command_context(4);
    dispatch(&fourth, "run_command", proposal.clone()).unwrap();
    shared.close();
    assert_eq!(shared.state.lock().unwrap().review.current(), None);
    assert_eq!(
        dispatch(&fourth, "run_command", proposal).unwrap_err().code,
        "cancelled"
    );
}

fn forecast_assets(workspace: &Path) -> (String, String, ForecastCorrection) {
    let start = chrono::NaiveDate::from_ymd_opt(2026, 2, 2)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap();
    let native = GridSeries {
        timeline: IntervalAxis {
            time_zone: "UTC".into(),
            bounds: vec![TimeInterval {
                start,
                end: start + chrono::Duration::hours(1),
            }],
        },
        grid: RegularGridSpec {
            requested_bbox: [0., 0., 1., 1.],
            effective_bbox: [0., 0., 1., 1.],
            resolution_degrees: 1.,
            latitudes: vec![0.5],
            longitudes: vec![0.5],
        },
        values: vec![2.],
        valid: vec![true],
        provenance: Provenance {
            source_ids: vec!["synthetic".into()],
            processing: Value::Null,
        },
    };
    let path = workspace.join("source.nc");
    write_grid_series(&path, &native).unwrap();
    let mut catalog =
        ForecastCatalog::open(workspace, Path::new("forecasts.sqlite"), Path::new("files"))
            .unwrap();
    let mut metadata = CanonicalForecastMetadata {
        provider: "local".into(),
        model: "forecast".into(),
        product: "rain".into(),
        cycle_utc: start.and_utc(),
    };
    let first = catalog
        .register_canonical(&path, &metadata)
        .unwrap()
        .asset_id;
    metadata.cycle_utc += chrono::Duration::hours(1);
    let second = catalog
        .register_canonical(&path, &metadata)
        .unwrap()
        .asset_id;
    let correction = ForecastCorrection {
        correction_id: "scale".into(),
        asset_id: first.clone(),
        t0_hour: 0,
        t1_hour: 1,
        shift_lat_pixels: 0.,
        shift_lon_pixels: 0.,
        rotation_deg: 0.,
        multiplication_factor: 2.,
        editor: None,
        reason: "synthetic".into(),
        created_at: "2026-02-02T00:00:00Z".into(),
    };
    catalog.save_correction(&correction).unwrap();
    (first, second, correction)
}

#[test]
fn forecast_edit_checks_catalog_corrections_and_requires_fresh_acceptance() {
    let (_root, session, mut draft) = fixture();
    let (first, second, correction) = forecast_assets(&draft.workspace);
    draft.document.schema_version = 1;
    draft.document.operation.forecast_first_end = Some("2026-02-02T01:00:00".into());
    draft.document.operation.forecasts = Some(DraftForecasts {
        catalog: Some("forecasts.sqlite".into()),
    });
    draft.document.operation.scenarios.push(DraftScenario {
        id: Some("test".into()),
        kind: Some("raw".into()),
        asset_id: Some(first.clone()),
        correction_ids: None,
    });
    draft.refresh_forecast(0).unwrap();
    draft.preview_forecast(0).unwrap();
    draft
        .document
        .sources
        .accepted
        .insert("unrelated".into(), "existing".into());
    let edit = json!({"forecast_asset": {"scenario_index": 0, "asset_id": second}});
    let mut registered = draft.clone();
    let scenario = &mut registered.document.operation.scenarios[0];
    scenario.kind = Some("corrected".into());
    scenario.correction_ids = Some(vec![correction.correction_id.clone()]);
    let mut inline = draft.clone();
    inline
        .document
        .instructions
        .corrections
        .push(Some(vec![DraftCorrection {
            draft_id: "local".into(),
            registered_origin: None,
            value: correction,
        }]));
    for incompatible in [registered, inline] {
        let original = incompatible.document.clone();
        session.set_draft(incompatible).unwrap();
        let rejected = match candidate(&session, edit.clone()) {
            Ok(_) => panic!("incompatible corrections must be preserved"),
            Err(error) => error,
        };
        assert!(rejected.message.contains("another asset"));
        assert_eq!(session.draft().unwrap().draft.document, original);
    }
    session.set_draft(draft).unwrap();
    let unknown = json!({"forecast_asset": {"scenario_index": 0, "asset_id": "unknown"}});
    let rejected = match candidate(&session, unknown) {
        Ok(_) => panic!("asset must exist in the selected catalog"),
        Err(error) => error,
    };
    assert!(rejected.message.contains("unknown asset"));
    let mut state = ProposalReviewState::default();
    let review = stage_edit(&mut state, &session, 1, edit);
    assert_eq!(
        review.changes,
        [DraftEdit::ForecastAsset {
            scenario_index: 0,
            scenario_id: Some("test".into()),
            current: Some(first),
            proposed: second,
        }]
    );
    assert_eq!(state.apply(&session, &review), ProposalOutcome::Applied);
    let mut working = session.draft().unwrap().draft;
    let acceptance = &working.document.sources.accepted;
    assert_eq!(acceptance.get("forecast:0"), None);
    assert_eq!(acceptance.get("unrelated").unwrap(), "existing");
    assert_eq!(
        working.document.operation.forecast_first_end.as_deref(),
        Some("2026-02-02T01:00:00")
    );
    assert!(
        working
            .preview_forecast(0)
            .unwrap_err()
            .contains("has not been accepted")
    );
    working.refresh_forecast(0).unwrap();
    working.preview_forecast(0).unwrap();
}
