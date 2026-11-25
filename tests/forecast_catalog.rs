use chrono::{NaiveDate, TimeZone, Utc};
use mgb_ops::store::{
    ForecastCatalog, ForecastProvider, ForecastSnapshotSelection, ForecastSource, GridSeries,
    IntervalAxis, Provenance, RegularGridSpec, TimeInterval,
};
use sha2::{Digest, Sha256};
#[test]
fn forced_replacement_waits_for_readers_and_retires_old_files() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("download.grib2");
    std::fs::write(&src, b"source").unwrap();
    let hash = format!("{:x}", Sha256::digest(b"source"));
    let settings = serde_json::json!({"provider":"noaa","model":"gfs","product":"fc","cycle_utc":"2026-07-20T00:00:00Z","operation_timezone":"UTC","crop_bbox":[0.,0.,1.,1.]});
    let mut identity = Sha256::new();
    identity.update(hash.as_bytes());
    identity.update(serde_json::to_vec(&settings).unwrap());
    let id = format!("{:x}", identity.finalize());
    let source = ForecastSource {
        source_id: id.clone(),
        provider: ForecastProvider::Noaa,
        model: "gfs".into(),
        product: "fc".into(),
        cycle_utc: Utc.with_ymd_and_hms(2026, 7, 20, 0, 0, 0).unwrap(),
        sha256: hash,
        byte_length: 6,
        normalization_json: settings.clone(),
    };
    let start = NaiveDate::from_ymd_opt(2026, 7, 20)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap();
    let native = GridSeries {
        timeline: IntervalAxis {
            time_zone: "UTC".into(),
            bounds: vec![TimeInterval {
                start,
                end: start + chrono::Duration::hours(3),
            }],
        },
        grid: RegularGridSpec {
            requested_bbox: [0., 0., 1., 1.],
            resolution_degrees: 1.,
            latitudes: vec![0.5],
            longitudes: vec![0.5],
            effective_bbox: [0., 0., 1., 1.],
        },
        values: vec![3.],
        valid: vec![true],
        provenance: Provenance {
            source_ids: vec![id.clone()],
            processing: serde_json::json!({"normalization":settings}),
        },
    };
    let root = std::path::Path::new("assets/forecasts");
    let mut c = ForecastCatalog::open(dir.path(), root, root).unwrap();
    let mut bad_source = source.clone();
    bad_source.byte_length = 7;
    assert!(c.register_native(&src, &bad_source, &native).is_err());
    assert!(c.asset_choices().unwrap().is_empty());
    let first = c.register_native(&src, &source, &native).unwrap();
    assert_eq!(c.asset_choices().unwrap()[0].asset_id, first.asset_id);
    let canonical_path = dir.path().join("canonical.nc");
    let mut canonical = native.clone();
    canonical.grid.requested_bbox[0] -= 1e-14;
    canonical.provenance.processing = serde_json::json!({"different": "provenance"});
    mgb_ops::store::write_grid_series(&canonical_path, &canonical).unwrap();
    let metadata = mgb_ops::store::CanonicalForecastMetadata {
        provider: "noaa".into(),
        model: "gfs".into(),
        product: "fc".into(),
        cycle_utc: source.cycle_utc,
    };
    let reused = c.register_canonical(&canonical_path, &metadata).unwrap();
    assert_eq!(reused.asset_id, first.asset_id);
    assert_eq!(reused.source_id, first.source_id);
    assert_eq!(
        c.load_native(&first.asset_id)
            .unwrap()
            .1
            .grid
            .requested_bbox,
        native.grid.requested_bbox
    );
    assert_eq!(c.asset_choices().unwrap().len(), 1);
    canonical.values[0] = 8.;
    let different_path = dir.path().join("different.nc");
    mgb_ops::store::write_grid_series(&different_path, &canonical).unwrap();
    assert!(c.register_canonical(&different_path, &metadata).is_err());
    let original_identity = c.native_identity(&first.asset_id).unwrap();
    let mut updated = native.clone();
    updated.values[0] = 4.;
    assert!(c.register_native(&src, &source, &updated).is_err());
    assert_eq!(c.load_native(&first.asset_id).unwrap().1.values, [3.]);
    assert!(
        c.register_native_force(&src, &bad_source, &updated)
            .is_err()
    );
    assert_eq!(c.load_native(&first.asset_id).unwrap().1.values, [3.]);
    std::fs::write(&src, b"revised source").unwrap();
    let mut revised_source = source.clone();
    revised_source.sha256 = format!("{:x}", Sha256::digest(b"revised source"));
    revised_source.byte_length = 14;
    let mut identity = Sha256::new();
    identity.update(revised_source.sha256.as_bytes());
    identity.update(serde_json::to_vec(&revised_source.normalization_json).unwrap());
    revised_source.source_id = format!("{:x}", identity.finalize());
    updated.provenance.source_ids = vec![revised_source.source_id.clone()];
    assert!(c.register_native(&src, &revised_source, &updated).is_err());
    assert_eq!(c.load_native(&first.asset_id).unwrap().1.values, [3.]);
    let guard = std::fs::File::open(dir.path().join(root).join("access.lock")).unwrap();
    guard.lock_shared().unwrap();
    let (started_tx, started_rx) = std::sync::mpsc::channel();
    let (done_tx, done_rx) = std::sync::mpsc::channel();
    let workspace = dir.path().to_path_buf();
    let writer_source = src.clone();
    let writer_record = revised_source.clone();
    let writer_grid = updated.clone();
    let writer = std::thread::spawn(move || {
        let mut catalog = ForecastCatalog::open_writable(&workspace, root).unwrap();
        started_tx.send(()).unwrap();
        let result = catalog.register_native_force(&writer_source, &writer_record, &writer_grid);
        done_tx.send(result).unwrap();
    });
    started_rx.recv().unwrap();
    assert!(matches!(
        done_rx.recv_timeout(std::time::Duration::from_millis(100)),
        Err(std::sync::mpsc::RecvTimeoutError::Timeout)
    ));
    assert!(dir.path().join(&first.native_artifact).is_file());
    drop(guard);
    let second = done_rx.recv().unwrap().unwrap();
    writer.join().unwrap();
    assert_eq!(first.asset_id, second.asset_id);
    assert_ne!(first.source_id, second.source_id);
    assert_eq!(c.asset_choices().unwrap().len(), 1);
    assert_ne!(first.native_artifact, second.native_artifact);
    assert_ne!(
        original_identity,
        c.native_identity(&second.asset_id).unwrap()
    );
    assert!(!dir.path().join(&first.native_artifact).exists());
    assert!(!dir.path().join(&first.retained_source).exists());
    assert_eq!(c.load_native(&second.asset_id).unwrap().1.values, [4.]);
    let canonical_replacement = c
        .register_canonical_force(&different_path, &metadata)
        .unwrap();
    assert_eq!(canonical_replacement.asset_id, first.asset_id);
    assert_eq!(c.asset_choices().unwrap().len(), 1);
    assert!(!dir.path().join(&second.native_artifact).exists());
    assert_eq!(
        c.load_native(&canonical_replacement.asset_id)
            .unwrap()
            .1
            .values,
        [8.]
    );
    let second = canonical_replacement;
    assert_eq!(
        c.load_native(&second.asset_id)
            .unwrap()
            .1
            .grid
            .requested_bbox,
        canonical.grid.requested_bbox
    );
    std::fs::remove_file(dir.path().join(&second.retained_source)).unwrap();
    std::fs::remove_file(&src).unwrap();
    assert_eq!(c.load_native(&second.asset_id).unwrap().1.values, [8.]);
    std::fs::write(dir.path().join(&second.native_artifact), b"corrupt").unwrap();
    assert!(c.load_native(&second.asset_id).is_err());
}

#[test]
fn canonical_snapshot_contains_native_and_inline_selection() {
    use mgb_ops::store::{CanonicalForecastMetadata, write_grid_series};
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("external.nc");
    let cycle = Utc.with_ymd_and_hms(2026, 7, 20, 0, 0, 0).unwrap();
    let start = cycle.naive_utc();
    let mut native = GridSeries {
        timeline: IntervalAxis {
            time_zone: "UTC".into(),
            bounds: vec![TimeInterval {
                start,
                end: start + chrono::Duration::hours(1),
            }],
        },
        grid: RegularGridSpec {
            requested_bbox: [0., 0., 1., 1.],
            resolution_degrees: 1.,
            latitudes: vec![0.5],
            longitudes: vec![0.5],
            effective_bbox: [0., 0., 1., 1.],
        },
        values: vec![3.],
        valid: vec![true],
        provenance: Provenance {
            source_ids: vec!["external".into()],
            processing: "original provenance".into(),
        },
    };
    write_grid_series(&input, &native).unwrap();
    let root = std::path::Path::new("assets/forecasts");
    let mut catalog = ForecastCatalog::open(dir.path(), root, root).unwrap();
    let metadata = CanonicalForecastMetadata {
        provider: "local".into(),
        model: "model".into(),
        product: "rainfall".into(),
        cycle_utc: cycle,
    };
    let asset = catalog.register_canonical(&input, &metadata).unwrap();
    assert_eq!(
        catalog
            .register_canonical_force(&input, &metadata)
            .unwrap()
            .asset_id,
        asset.asset_id
    );
    native.values[0] = 5.;
    std::fs::remove_file(&input).unwrap();
    write_grid_series(&input, &native).unwrap();
    assert!(catalog.register_canonical(&input, &metadata).is_err());
    let updated = catalog.register_canonical_force(&input, &metadata).unwrap();
    assert_eq!(updated.asset_id, asset.asset_id);
    assert_ne!(updated.source_id, asset.source_id);
    assert_eq!(catalog.asset_choices().unwrap().len(), 1);
    let asset = updated;
    let snapshot = dir.path().join("snapshot");
    let snapshot_catalog = snapshot.join("sources/forecasts");
    let selected = catalog
        .snapshot_selected_values(
            &snapshot_catalog,
            &[ForecastSnapshotSelection {
                asset_id: asset.asset_id.clone(),
            }],
        )
        .unwrap();
    assert_eq!(selected[0].0.asset_id, asset.asset_id);
    std::fs::copy(
        dir.path().join(&asset.native_artifact),
        snapshot_catalog.join(&asset.asset_id).join("native.nc"),
    )
    .unwrap();
    let reopened =
        ForecastCatalog::open_read_only(&snapshot, std::path::Path::new("sources/forecasts"))
            .unwrap();
    assert_eq!(reopened.load_native(&asset.asset_id).unwrap().1, native);
}
