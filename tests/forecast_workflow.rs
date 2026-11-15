use mgb_ops::{NormalizeForecastRequest, normalize_forecast};

fn document() -> String {
    r#"schema_version = 1
provider = "ecmwf"
model = "ifs"
product = "fc"
cycle_utc = "2026-07-20T00:00:00Z"
operation_timezone = "America/Sao_Paulo"
source = "missing.grib2"
crop_bbox = [-60.0,-35.0,-50.0,-25.0]
catalog = "data/forecasts.sqlite"
artifact_root = "data/forecasts"
"#
    .into()
}
#[test]
fn invalid_requests_do_not_create_catalog_or_artifacts() {
    let dir = tempfile::tempdir().unwrap();
    let request_file = dir.path().join("normalize.toml");
    for contents in [
        document() + "unexpected = 'secret-value'\n",
        document().replace("schema_version = 1", "schema_version = 2"),
        document().replace("2026-07-20T00:00:00Z", "2026-07-20T00:00:00-03:00"),
        document().replace("2026-07-20T00:00:00Z", "2026-07-20T00:00:00.001Z"),
        document().replace("America/Sao_Paulo", "Invalid/Zone"),
        document().replace("source = \"missing.grib2\"", "source = \" \""),
        document().replace("product = \"fc\"", "product = \"unsupported\""),
    ] {
        std::fs::write(&request_file, contents).unwrap();
        let error = normalize_forecast(NormalizeForecastRequest {
            invocation_dir: dir.path().to_path_buf(),
            workspace: ".".into(),
            request_file: "normalize.toml".into(),
        })
        .unwrap_err();
        assert!(!error.to_string().contains("secret-value"));
        assert!(!dir.path().join("data").exists());
    }
}

#[test]
fn library_and_cli_normalize_the_same_canonical_values() {
    use std::process::Command;
    let dir = tempfile::tempdir().unwrap();
    let fixture =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/forecast/noaa.grib2");
    let request_text = document()
        .replace("provider = \"ecmwf\"", "provider = \"noaa\"")
        .replace("model = \"ifs\"", "model = \"gfs\"")
        .replace("missing.grib2", "sources/noaa.grib2")
        .replace("[-60.0,-35.0,-50.0,-25.0]", "[-180.0,-90.0,180.0,90.0]");
    std::fs::create_dir(dir.path().join("requests")).unwrap();
    std::fs::write(dir.path().join("requests/normalize.toml"), &request_text).unwrap();
    for name in ["library", "cli"] {
        let sources = dir.path().join(name).join("sources");
        std::fs::create_dir_all(&sources).unwrap();
        std::fs::copy(&fixture, sources.join("noaa.grib2")).unwrap();
    }
    let report = normalize_forecast(NormalizeForecastRequest {
        invocation_dir: dir.path().to_path_buf(),
        workspace: "library".into(),
        request_file: "requests/normalize.toml".into(),
    })
    .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_mgb-ops"))
        .current_dir(dir.path())
        .args([
            "forecast",
            "normalize",
            "--workspace",
            "cli",
            "--request",
            "requests/normalize.toml",
            "--json",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let cli: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(cli["asset_id"], report.asset_id);
    assert_eq!(cli["source_id"], report.source_id);
    let a = mgb_ops::store::read_grid_series(&report.native_artifact).unwrap();
    let b = mgb_ops::store::read_grid_series(std::path::Path::new(
        cli["native_artifact"].as_str().unwrap(),
    ))
    .unwrap();
    assert_eq!(a, b);
    assert_eq!(
        std::fs::read(&report.retained_source).unwrap(),
        std::fs::read(&fixture).unwrap()
    );
    let source_path = dir.path().join("library/sources/noaa.grib2");
    std::fs::write(&source_path, b"changed external source").unwrap();
    assert_eq!(
        mgb_ops::store::read_grid_series(&report.native_artifact).unwrap(),
        a
    );
    assert_eq!(
        std::fs::read(&report.retained_source).unwrap(),
        std::fs::read(&fixture).unwrap()
    );
    assert!(!dir.path().join("library/data/observations.sqlite").exists());
}
