use chrono::{NaiveDate, TimeZone, Utc};
use mgb_ops::store::{
    ForecastCatalog, ForecastProvider, ForecastSnapshotSelection, ForecastSource, GridSeries,
    IntervalAxis, Provenance, RegularGridSpec, TimeInterval,
};
use sha2::{Digest, Sha256};
#[test]
fn revised_forecasts_keep_previous_versions() {
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
    assert_eq!(
        c.register_native(&src, &source, &native).unwrap().asset_id,
        first.asset_id
    );
    let mut updated = native.clone();
    updated.values[0] = 4.;
    assert!(c.register_native_force(&src, &source, &updated).is_err());
    std::fs::write(&src, b"revised source").unwrap();
    let mut revised = source.clone();
    revised.sha256 = format!("{:x}", Sha256::digest(b"revised source"));
    revised.byte_length = 14;
    let mut identity = Sha256::new();
    identity.update(revised.sha256.as_bytes());
    identity.update(serde_json::to_vec(&revised.normalization_json).unwrap());
    revised.source_id = format!("{:x}", identity.finalize());
    updated.provenance.source_ids = vec![revised.source_id.clone()];
    let second = c.register_native_force(&src, &revised, &updated).unwrap();
    assert_ne!(first.asset_id, second.asset_id);
    assert!(first.asset_id.starts_with("noaa-gfs-fc-20260720T000000Z-"));
    assert_eq!(c.asset_choices().unwrap().len(), 2);
    assert_eq!(c.asset_choices().unwrap()[0].asset_id, second.asset_id);
    assert_eq!(c.load_native(&first.asset_id).unwrap().1.values, [3.]);
    assert_eq!(c.load_native(&second.asset_id).unwrap().1.values, [4.]);
    assert!(dir.path().join(&first.native_artifact).exists());
    assert!(dir.path().join(&first.retained_source).exists());
    assert_eq!(
        c.register_native_force(&src, &revised, &updated)
            .unwrap()
            .asset_id,
        second.asset_id
    );
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
    let updated = catalog.register_canonical(&input, &metadata).unwrap();
    assert_ne!(updated.asset_id, asset.asset_id);
    assert_ne!(updated.source_id, asset.source_id);
    assert_eq!(catalog.asset_choices().unwrap().len(), 2);
    assert_eq!(catalog.load_native(&asset.asset_id).unwrap().1.values, [3.]);
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
