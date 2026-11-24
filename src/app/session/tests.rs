use super::*;
use std::{fs, sync::mpsc, thread, time::Duration};

fn fixture() -> tempfile::TempDir {
    fs::create_dir_all("runs").unwrap();
    tempfile::tempdir_in("runs").unwrap()
}

fn request(root: &Path, config: Option<PathBuf>) -> InspectionRequest {
    InspectionRequest {
        workspace: "area".into(),
        config,
        invocation_dir: root.canonicalize().unwrap(),
        gpkg_path: None,
        engine_executable: None,
        engine_input_dir: None,
    }
}

#[test]
fn explicit_paths_and_findings_match_one_shot_inspection() {
    let root = fixture();
    let root = root.path().canonicalize().unwrap();
    let workspace = root.join("area");
    fs::create_dir(&workspace).unwrap();
    let external = root.join("region.toml");
    let inspect = |config: Option<PathBuf>| {
        let request = request(&root, config);
        let direct = super::super::inspect(request.clone());
        let session = Session::new(request);
        let selected = session.workspace().inspect();
        assert_eq!(
            serde_json::to_value(&direct).unwrap(),
            serde_json::to_value(&selected).unwrap()
        );
        assert_eq!(selected.workspace, workspace);
        selected
    };
    let report = inspect(None);
    assert_eq!(report.config, workspace.join("config/workspace.toml"));
    assert_eq!(report.findings[1].code, super::super::FindingCode::Missing);

    fs::write(&external, "[spatial]\ngpkg_path = \"area.gpkg\"\n").unwrap();
    let report = inspect(Some("region.toml".into()));
    assert_eq!(report.config, external);
    assert_eq!(
        report.findings[2].path.as_deref(),
        Some(workspace.join("area.gpkg").as_path())
    );

    let mut with_overrides = request(&root, Some("region.toml".into()));
    with_overrides.gpkg_path = Some("override.gpkg".into());
    with_overrides.engine_input_dir = Some("override-inputs".into());
    let session = Session::new(with_overrides);
    let draft = session.workspace().new_draft().unwrap();
    let clean = !draft.is_dirty();
    assert!(clean);
    assert_eq!(
        draft.document.operation.engine.unwrap().input_dir,
        Some("override-inputs".into())
    );
    assert_eq!(
        session.workspace().inspect().findings[2].path.as_deref(),
        Some(workspace.join("override.gpkg").as_path())
    );

    #[cfg(unix)]
    {
        let alias = root.join("alias.toml");
        std::os::unix::fs::symlink(&external, &alias).unwrap();
        let report = inspect(Some("alias.toml".into()));
        assert_eq!(report.config, alias);
        assert_eq!(report.findings[1].status, super::super::FindingStatus::Pass);
    }
}

#[test]
fn the_saved_draft_workspace_does_not_change_the_selection() {
    let root = fixture();
    let root = root.path().canonicalize().unwrap();
    let other = root.join("other");
    let mut draft = Draft::new(other, &root);
    draft.save(Path::new("recipe.json"), false).unwrap();
    let session = Session::new(request(&root, Some("elsewhere.toml".into())));
    let opened = session
        .workspace()
        .open_draft(Path::new("recipe.json"))
        .unwrap();
    assert!(
        opened
            .findings
            .iter()
            .any(|finding| finding.contains("saved workspace differs"))
    );
    session.set_draft(opened).unwrap();
    let working = session.draft().unwrap();
    assert_eq!(working.draft.workspace, root.join("area"));
    assert_eq!(working.draft.document.workspace, root.join("other"));
    assert_eq!(session.workspace().config(), root.join("elsewhere.toml"));
}

#[test]
fn edits_reversals_discard_and_identical_open_advance_the_working_version() {
    let root = fixture();
    let session = Session::new(request(root.path(), None));
    let selection = session.workspace();
    let draft = Draft::new(selection.workspace().into(), root.path());
    let first = session.set_draft(draft.clone()).unwrap();
    let reopened = session.set_draft(draft).unwrap();
    assert_ne!(first.draft, reopened.draft);
    assert!(reopened.revision > first.revision);

    let original = session.draft().unwrap().draft.document;
    let mut edited = original.clone();
    edited.operation.forecast_horizon_days = Some(8);
    let changed = session.commit_form_edit(reopened, edited.clone()).unwrap();
    let reverted = session.commit_form_edit(changed, original.clone()).unwrap();
    assert!(reverted.revision > changed.revision);
    assert!(changed.revision > reopened.revision);
    assert_eq!(
        session.apply_draft_update(reopened, edited.clone()),
        Err(DraftConflict::Stale)
    );

    let applied = session.apply_draft_update(reverted, edited).unwrap();
    session.discard_draft_changes().unwrap();
    let discarded = session.draft().unwrap();
    assert!(discarded.version.revision > applied.revision);
    assert_eq!(discarded.draft.document, original);

    let mut presentation = discarded.draft.document.clone();
    presentation.visualization = Some(super::super::DraftVisualization::default());
    let view = session
        .commit_form_edit(discarded.version, presentation)
        .unwrap();
    assert_eq!(view, discarded.version);
}

#[test]
fn save_fixes_the_anchor_and_job_admission_keeps_rejected_updates_in_memory_only() {
    let root = fixture();
    let session = Session::new(request(root.path(), None));
    session
        .set_draft(Draft::new(
            session.workspace().workspace().into(),
            root.path(),
        ))
        .unwrap();
    let snapshot = session.draft().unwrap();
    let mut proposal = snapshot.draft.document.clone();
    proposal.operation.reference_time = Some("2026-10-03T00:00:00".into());
    proposal.operation.time_zone = Some("UTC".into());
    let mut invalid = proposal.clone();
    invalid.operation.time_zone = Some("Unavailable/Timezone".into());
    let conflict_1 = matches!(
        session.validate_draft_update(snapshot.version, &invalid),
        Err(DraftConflict::Invalid(_))
    );
    assert!(conflict_1);
    // This incomplete draft is editable even though it still cannot be prepared.
    session
        .validate_draft_update(snapshot.version, &proposal)
        .unwrap();
    let before_save = session
        .apply_draft_update(snapshot.version, proposal)
        .unwrap();

    let job = session.begin_draft_job().unwrap();
    let mut working = job.input().unwrap().draft.clone();
    let path = root.path().canonicalize().unwrap().join("saved.json");
    working.save(&path, false).unwrap();
    let saved = fs::read(&path).unwrap();
    let mut proposed_edit = working.document.clone();
    proposed_edit.operation.forecast_horizon_days = Some(6);
    thread::scope(|scope| {
        let outcome =
            scope.spawn(|| session.apply_draft_update(before_save, proposed_edit.clone()));
        assert_eq!(outcome.join().unwrap(), Err(DraftConflict::JobPending));
    });
    assert_eq!(session.draft().unwrap().draft.document, working.document);
    assert_eq!(fs::read(&path).unwrap(), saved);

    let saved_version = job.replace(&session, working).unwrap();
    assert!(saved_version.revision > before_save.revision);
    let mut reanchor = proposed_edit.clone();
    reanchor.operation.reference_time = Some("2026-10-03T01:00:00".into());
    let conflict_2 = matches!(
        session.validate_draft_update(saved_version, &reanchor),
        Err(DraftConflict::Invalid(_))
    );
    assert!(conflict_2);
    assert_eq!(
        session.apply_draft_update(before_save, proposed_edit.clone()),
        Err(DraftConflict::Stale)
    );
    assert!(
        session
            .apply_draft_update(saved_version, proposed_edit)
            .is_ok()
    );
    assert_eq!(fs::read(&path).unwrap(), saved);
}

#[test]
fn results_from_another_session_cannot_replace_a_new_workspace_draft() {
    let root = fixture();
    let old = Session::new(request(root.path(), None));
    let old_draft = Draft::new(old.workspace().workspace().into(), root.path());
    let version = old.set_draft(old_draft.clone()).unwrap();
    let job = old.begin_draft_job().unwrap();
    let mut next_request = request(root.path(), None);
    next_request.workspace = "next".into();
    let new = Session::new(next_request);
    assert_eq!(
        new.apply_draft_update(version, old_draft.document.clone()),
        Err(DraftConflict::Stale)
    );
    let new_draft = Draft::new(new.workspace().workspace().into(), root.path());
    let next_version = new.set_draft(new_draft.clone()).unwrap();
    assert_eq!(
        job.replace(&new, old_draft.clone()),
        Err(DraftConflict::Stale)
    );
    assert_eq!(new.draft().unwrap().version, next_version);
    assert_eq!(new.draft().unwrap().draft.document, new_draft.document);
    assert!(old.begin_draft_job().is_ok());
}

fn stage(session: &Session, name: &str) -> PathBuf {
    let root = crate::store::new_run_stage_for(session.workspace().workspace(), name).unwrap();
    fs::write(root.join("manifest.json"), name).unwrap();
    fs::write(root.join("result.txt"), name).unwrap();
    root
}

#[test]
fn active_artifacts_stay_in_one_generation_during_publication() {
    let root = fixture();
    let session = Session::new(request(root.path(), None));
    let workspace = session.workspace().workspace();
    assert!(session.run().summary(None).unwrap().is_none());
    let first = stage(&session, "first");
    crate::store::publish_run(workspace, &first).unwrap();
    let next = stage(&session, "next");
    let snapshot = session.run();
    let expected = RunIdentity::Active(first.clone());

    thread::scope(|scope| {
        let (reading, read_started) = mpsc::channel();
        let (release, wait_for_release) = mpsc::channel();
        let reader = &snapshot;
        let generation = &expected;
        let query = scope.spawn(move || {
            reader.read(Some(generation), |root| {
                let manifest = fs::read_to_string(root.join("manifest.json")).unwrap();
                reading.send(()).unwrap();
                wait_for_release.recv().unwrap();
                let result = fs::read_to_string(root.join("result.txt")).unwrap();
                Ok((manifest, result))
            })
        });
        read_started.recv().unwrap();
        let (publishing, publish_started) = mpsc::channel();
        let (published, publish_finished) = mpsc::channel();
        let next_root = &next;
        let publisher = scope.spawn(move || {
            publishing.send(()).unwrap();
            crate::store::publish_run(workspace, next_root).unwrap();
            published.send(()).unwrap();
        });
        publish_started.recv().unwrap();
        let blocked = publish_finished
            .recv_timeout(Duration::from_millis(75))
            .is_err();
        release.send(()).unwrap();
        let result = query.join().unwrap().unwrap().unwrap();
        publisher.join().unwrap();
        assert!(blocked);
        assert_eq!(result.identity, expected);
        assert_eq!(result.data, ("first".into(), "first".into()));
    });

    let retired = !first.exists();
    assert!(retired);
    assert!(snapshot.read(Some(&expected), |_| Ok(())).is_err());
    let result = snapshot
        .read(None, |path| {
            fs::read_to_string(path.join("result.txt")).map_err(|e| e.to_string())
        })
        .unwrap()
        .unwrap();
    assert_eq!(result.identity, RunIdentity::Active(next));
    assert_eq!(result.data, "next");
}

#[test]
fn saved_selection_stays_fixed_and_rejects_managed_directories_and_aliases() {
    let root = fixture();
    let session = Session::new(request(root.path(), None));
    let workspace = session.workspace().workspace();
    let active = stage(&session, "active");
    crate::store::publish_run(workspace, &active).unwrap();
    let saved = root.path().canonicalize().unwrap().join("retained");
    fs::create_dir(&saved).unwrap();
    fs::write(saved.join("result.txt"), "retained").unwrap();
    assert!(session.run().select_saved(&active).is_err());
    #[cfg(unix)]
    {
        let alias = saved.with_file_name("alias");
        std::os::unix::fs::symlink(&active, &alias).unwrap();
        assert!(session.run().select_saved(&alias).is_err());
    }
    let active_snapshot = session.run();
    session
        .select_run(session.run().select_saved(&saved).unwrap())
        .unwrap();
    let snapshot = session.run();
    let replaced = !session.is_current_run(&active_snapshot);
    assert!(replaced);
    let expected = RunIdentity::Saved(saved.clone());
    let next = stage(&session, "published");
    crate::store::publish_run(workspace, &next).unwrap();
    let result = snapshot
        .read(Some(&expected), |path| {
            fs::read_to_string(path.join("result.txt")).map_err(|e| e.to_string())
        })
        .unwrap()
        .unwrap();
    assert_eq!(result.identity, expected);
    assert_eq!(result.data, "retained");
    assert!(session.is_current_run(&snapshot));
    assert!(session.select_run(active_snapshot).is_err());
    session.select_run(snapshot.follow_active()).unwrap();
    assert!(session.run().read(Some(&expected), |_| Ok(())).is_err());
    let retired = !active.exists();
    assert!(retired);
}
