use mgb_ops::{
    InspectionRequest,
    app::{RunParameters, Session},
};

#[test]
fn selected_toml_defaults_are_revisioned_and_job_admission_is_atomic() {
    std::fs::create_dir_all("runs").unwrap();
    let dir = tempfile::tempdir_in("runs").unwrap();
    let root = dir.path().canonicalize().unwrap();
    let custom = root.join("regional.toml");
    let text = "[run]\ntime_zone='UTC'\nreference_time='2026-07-20T00:00:00'\nlookback_days=2\nforecast_days=1\n[spatial]\nbounds=[0.0,0.0,1.0,1.0]\n";
    std::fs::write(&custom, text).unwrap();
    let session = Session::new(InspectionRequest {
        workspace: root.clone(),
        invocation_dir: root.clone(),
        config: Some(custom.clone()),
        gpkg_path: None,
        engine_executable: None,
        engine_input_dir: None,
    });
    assert_eq!(session.config(), custom);
    let before = session.run_setup().unwrap();
    std::fs::write(&custom, text.replace("lookback_days=2", "lookback_days=7")).unwrap();
    let (job, snapshot) = session.start_job_with_setup().unwrap();
    assert_eq!(
        snapshot.unwrap().operation.coverage().unwrap()[0],
        "2026-07-18T00:00:00"
    );
    assert!(
        session
            .update_run_setup(&before.revision, RunParameters::default())
            .is_err()
    );
    assert!(session.open_run_setup(before.operation.clone()).is_err());
    assert!(session.reload_configuration().is_err());
    drop(job);
    session.reload_configuration().unwrap();
    assert!(
        session
            .update_run_setup(&before.revision, RunParameters::default())
            .is_err()
    );
    assert_eq!(
        session.run_setup().unwrap().operation.coverage().unwrap()[0],
        "2026-07-13T00:00:00"
    );
}
