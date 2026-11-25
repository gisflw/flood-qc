use chrono::{NaiveDate, TimeZone, Utc};
use mgb_ops::store::{
    ForecastCatalog, ForecastCorrection, ForecastProvider, ForecastSource, GridSeries,
    IntervalAxis, Provenance, RegularGridSpec, TimeInterval,
};
use sha2::{Digest, Sha256};
#[test]
fn retained_source_is_immutable_and_correction_order_is_explicit() {
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
    let mut c =
        ForecastCatalog::open(dir.path(), "operation.sqlite".as_ref(), "assets".as_ref()).unwrap();
    let mut bad_source = source.clone();
    bad_source.byte_length = 7;
    assert!(c.register_native(&src, &bad_source, &native).is_err());
    let db = rusqlite::Connection::open(dir.path().join("operation.sqlite")).unwrap();
    let count: i64 = db
        .query_row("SELECT count(*) FROM asset", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 0);
    let asset = c.register_native(&src, &source, &native).unwrap();
    assert_ne!(asset.asset_id, asset.source_id);
    let choices = c.asset_choices().unwrap();
    assert_eq!(choices.len(), 1);
    assert_eq!(choices[0].asset_id, asset.asset_id);
    assert_eq!(choices[0].provider, "noaa");
    std::fs::write(&src, b"changed").unwrap();
    let (_, got) = c.load_native(&asset.asset_id).unwrap();
    assert_eq!(got.values, [3.]);
    let correction = |name: &str| ForecastCorrection {
        correction_id: name.into(),
        asset_id: asset.asset_id.clone(),
        t0_hour: 0,
        t1_hour: 3,
        shift_lat_pixels: 0.,
        shift_lon_pixels: 0.,
        rotation_deg: 0.,
        multiplication_factor: 1.,
        editor: None,
        reason: "adjust".into(),
        created_at: "2026-07-20T00:00:00Z".into(),
    };
    c.save_correction(&correction("first")).unwrap();
    c.save_correction(&correction("second")).unwrap();
    assert_eq!(
        c.correction_choices().unwrap(),
        vec![
            ("first".into(), asset.asset_id.clone()),
            ("second".into(), asset.asset_id.clone())
        ]
    );
    let ids = vec!["second".into(), "first".into()];
    let (_, loaded, selected) = c.load_selected(&asset.asset_id, &ids).unwrap();
    assert_eq!(loaded.provenance.processing["asset_id"], asset.asset_id);
    assert_eq!(
        selected
            .iter()
            .map(|c| c.correction_id.as_str())
            .collect::<Vec<_>>(),
        ["second", "first"]
    );
    assert!(
        c.load_corrections(&asset.asset_id, &["first".into(), "first".into()])
            .is_err()
    );
    assert!(c.load_corrections("other", &["first".into()]).is_err());
    assert!(c.load_corrections("other", &[]).is_err());
    assert!(c.load_corrections(&asset.asset_id, &[]).unwrap().is_empty());
    db.execute(
        "UPDATE correction SET reason='' WHERE correction_id='first'",
        [],
    )
    .unwrap();
    assert_eq!(
        c.load_corrections(&asset.asset_id, &["first".into()])
            .unwrap()[0]
            .reason,
        ""
    );
    db.execute("UPDATE asset SET normalization_json='{'", [])
        .unwrap();
    assert!(c.load_native(&asset.asset_id).is_err());
    db.execute(
        "UPDATE asset SET normalization_json=?",
        [serde_json::to_string(&source.normalization_json).unwrap()],
    )
    .unwrap();
    std::fs::write(dir.path().join(asset.retained_source), b"corrupt").unwrap();
    assert!(c.load_native(&asset.asset_id).is_err());
}

#[test]
fn canonical_netcdf_intake_keeps_bytes_and_existing_correction_workflow() {
    use mgb_ops::analysis::corrected_forecast_frames;
    use mgb_ops::store::{CanonicalForecastMetadata, write_grid_series};

    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("external.nc");
    let cycle = Utc.with_ymd_and_hms(2026, 7, 20, 0, 0, 0).unwrap();
    let start = cycle.naive_utc();
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
            resolution_degrees: 1.,
            latitudes: vec![0.5],
            longitudes: vec![0.5],
            effective_bbox: [0., 0., 1., 1.],
        },
        values: vec![3.],
        valid: vec![true],
        provenance: Provenance {
            source_ids: vec!["external".into()],
            processing: "original canonical provenance".into(),
        },
    };
    write_grid_series(&input, &native).unwrap();
    let original_bytes = std::fs::read(&input).unwrap();
    let mut catalog =
        ForecastCatalog::open(dir.path(), "forecasts.sqlite".as_ref(), "files".as_ref()).unwrap();
    let metadata = CanonicalForecastMetadata {
        provider: "local".into(),
        model: "forecast model".into(),
        product: "rainfall".into(),
        cycle_utc: cycle,
    };
    let asset = catalog.register_canonical(&input, &metadata).unwrap();
    assert_eq!(
        catalog
            .register_canonical(&input, &metadata)
            .unwrap()
            .asset_id,
        asset.asset_id
    );
    assert_eq!(catalog.asset_choices().unwrap().len(), 1);
    assert_eq!(catalog.asset_choices().unwrap()[0].provider, "local");

    std::fs::write(&input, b"invalid new content").unwrap();
    assert!(catalog.register_canonical(&input, &metadata).is_err());
    assert_eq!(
        std::fs::read_dir(dir.path().join("files")).unwrap().count(),
        1
    );
    for path in [&asset.retained_source, &asset.native_artifact] {
        assert_eq!(
            std::fs::read(dir.path().join(path)).unwrap(),
            original_bytes
        );
    }
    assert_eq!(catalog.load_native(&asset.asset_id).unwrap().1, native);

    catalog
        .register_corrections(&[ForecastCorrection {
            correction_id: "scale".into(),
            asset_id: asset.asset_id.clone(),
            t0_hour: 0,
            t1_hour: 1,
            shift_lat_pixels: 0.,
            shift_lon_pixels: 0.,
            rotation_deg: 0.,
            multiplication_factor: 2.,
            editor: None,
            reason: "review".into(),
            created_at: "2026-07-20T00:00:00Z".into(),
        }])
        .unwrap();
    let ids = vec!["scale".into()];
    let (_, loaded, corrections) = catalog.load_selected(&asset.asset_id, &ids).unwrap();
    assert_eq!(
        loaded.provenance.processing["parent"],
        native.provenance.processing
    );
    assert_eq!(
        corrected_forecast_frames(&loaded, &corrections, &[0])
            .unwrap()
            .0[0]
            .values,
        [6.0]
    );

    let snapshot = dir.path().join("snapshot");
    std::fs::create_dir_all(snapshot.join("sources/original")).unwrap();
    std::fs::create_dir_all(snapshot.join("sources/native")).unwrap();
    let snapshot_source = snapshot.join(asset.snapshot_source_path());
    assert_eq!(snapshot_source.extension().unwrap(), "nc");
    std::fs::copy(dir.path().join(&asset.retained_source), &snapshot_source).unwrap();
    std::fs::copy(
        dir.path().join(&asset.native_artifact),
        snapshot.join(format!("sources/native/{}.nc", asset.asset_id)),
    )
    .unwrap();
    let snapshot_db = snapshot.join("sources/forecast.sqlite");
    catalog
        .snapshot_selected(&snapshot_db, &[(asset.asset_id.clone(), ids.clone())])
        .unwrap();
    let reopened = ForecastCatalog::open_read_only(&snapshot, &snapshot_db).unwrap();
    let (_, snapshot_native, snapshot_corrections) =
        reopened.load_selected(&asset.asset_id, &ids).unwrap();
    assert_eq!(snapshot_native, loaded);
    assert_eq!(snapshot_corrections, corrections);
    std::fs::write(&snapshot_source, b"not the retained bytes").unwrap();
    assert!(reopened.load_native(&asset.asset_id).is_err());
}
