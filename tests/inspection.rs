use mgb_ops::{InspectionReport, InspectionRequest, inspect};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

const INPUTS: &[&str] = &[
    "PARHIG.hig",
    "Previsao.meta",
    "MINI.gtp",
    "albiaf.hig",
    "paruso.hig",
    "Cell.hig",
    "COTA_AREA.FLP",
    "face.con",
    "Azimute_Mini.txt",
    "medias.cli",
    "FCORPLU.txt",
    "curvas_chave_parametros.txt",
];
fn put(path: &Path, bytes: &[u8]) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, bytes).unwrap();
}
fn fixture() -> TempDir {
    let temp = tempfile::tempdir().unwrap();
    let w = temp.path();
    put(
        &w.join("config.toml"),
        b"[spatial]\ngpkg_path = 'geometry.gpkg'\n",
    );
    put(&w.join("geometry.gpkg"), b"geometry");
    put(&w.join("model/engine"), b"#!/bin/sh\ntouch MARKER\n");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(w.join("model/engine"), fs::Permissions::from_mode(0o755)).unwrap();
    }
    #[cfg(windows)]
    put(&w.join("model/engine.exe"), b"engine");
    for name in INPUTS {
        put(&w.join("model/Input").join(name), b"data");
    }
    temp
}
fn request(w: &Path) -> InspectionRequest {
    InspectionRequest {
        workspace: w.to_owned(),
        invocation_dir: std::env::current_dir().unwrap(),
        config: None,
        gpkg_path: None,
        engine_executable: None,
        engine_input_dir: None,
    }
}
fn code(report: &InspectionReport, check: &str) -> mgb_ops::FindingCode {
    report
        .findings
        .iter()
        .find(|f| f.check == check)
        .unwrap()
        .code
}
#[test]
fn example_and_configuration_contract() {
    let example = mgb_ops::example_config();
    assert!(
        example.contains("Example only")
            && example.contains("Windows default")
            && example.contains("Linux default")
    );
    assert!(toml::from_str::<toml::Value>(&example).is_ok());
    let t = fixture();
    let w = t.path();
    assert!(inspect(request(w)).ok);
    for invalid in [
        "[spatial]\ngpkg_path = 3",
        "[spatial]\nunknown = 1",
        "[spatial]\ngpkg_path = 'x'\ngpkg_path = 'y'",
        "spatial: x",
        "[spatial]\ngpkg_path = '   '",
    ] {
        put(&w.join("config.toml"), invalid.as_bytes());
        let mut r = request(w);
        if !invalid.contains("'   '") {
            r.gpkg_path = Some("geometry.gpkg".into());
        }
        assert_eq!(
            code(&inspect(r), "config"),
            mgb_ops::FindingCode::InvalidConfig,
            "{invalid}"
        );
    }
    put(
        &w.join("config.toml"),
        b"[spatial]\ngpkg_path = ''\n[engine]\nexecutable = ''\ninput_dir = ''\n",
    );
    let mut r = request(w);
    r.gpkg_path = Some("geometry.gpkg".into());
    r.engine_executable = Some("model/engine".into());
    r.engine_input_dir = Some("model/Input".into());
    assert!(inspect(r).ok);
    fs::remove_file(w.join("config.toml")).unwrap();
    let mut r = request(w);
    r.gpkg_path = Some("geometry.gpkg".into());
    assert_eq!(code(&inspect(r), "config"), mgb_ops::FindingCode::Missing);
}
#[test]
fn aggregates_failures_and_suppresses_children() {
    let t = fixture();
    let w = t.path();
    fs::remove_file(w.join("geometry.gpkg")).unwrap();
    put(&w.join("model/Input/MINI.gtp"), b"");
    fs::remove_file(w.join("model/Input/Cell.hig")).unwrap();
    fs::create_dir(w.join("model/Input/Cell.hig")).unwrap();
    let r = inspect(request(w));
    assert!(!r.ok);
    assert_eq!(code(&r, "geometry"), mgb_ops::FindingCode::Missing);
    assert_eq!(
        code(&r, "engine.input.MINI.gtp"),
        mgb_ops::FindingCode::Empty
    );
    assert_eq!(
        code(&r, "engine.input.Cell.hig"),
        mgb_ops::FindingCode::WrongKind
    );
    assert_eq!(code(&r, "optional.database"), mgb_ops::FindingCode::Absent);
    assert_eq!(
        code(&r, "scope.execution"),
        mgb_ops::FindingCode::OutsideScope
    );
    let mut req = request(w);
    req.engine_input_dir = Some("missing".into());
    let r = inspect(req);
    assert_eq!(
        code(&r, "engine.inputs"),
        mgb_ops::FindingCode::PrerequisiteFailed
    );
    assert!(
        !r.findings
            .iter()
            .any(|f| f.check == "engine.input.MINI.gtp")
    );
}
#[cfg(unix)]
#[test]
fn permissions_and_symlinks() {
    use std::os::unix::fs::{PermissionsExt, symlink};
    let t = fixture();
    let w = t.path();
    let engine = w.join("model/engine");
    fs::set_permissions(&engine, fs::Permissions::from_mode(0o644)).unwrap();
    assert_eq!(
        code(&inspect(request(w)), "engine.executable"),
        mgb_ops::FindingCode::NotExecutable
    );
    fs::set_permissions(&engine, fs::Permissions::from_mode(0o755)).unwrap();
    let geom = w.join("geometry.gpkg");
    let target = w.join("target with spaces");
    fs::rename(&geom, &target).unwrap();
    symlink(&target, &geom).unwrap();
    assert!(inspect(request(w)).ok);
    fs::remove_file(target).unwrap();
    assert_eq!(
        code(&inspect(request(w)), "geometry"),
        mgb_ops::FindingCode::Missing
    );
    let input = w.join("model/Input/MINI.gtp");
    fs::set_permissions(&input, fs::Permissions::from_mode(0o000)).unwrap();
    if fs::File::open(&input).is_err() {
        assert_eq!(
            code(&inspect(request(w)), "engine.input.MINI.gtp"),
            mgb_ops::FindingCode::Unreadable
        );
    }
    fs::set_permissions(&input, fs::Permissions::from_mode(0o644)).unwrap();
}
fn inventory(path: &Path, root: &Path, result: &mut Vec<String>) {
    let m = fs::symlink_metadata(path).unwrap();
    let mut line = format!(
        "{:?} {:?} {:?}",
        path.strip_prefix(root).unwrap(),
        m.file_type(),
        m.modified().unwrap()
    );
    if m.is_file() {
        line.push_str(&format!(" {:?}", fs::read(path).unwrap()));
    }
    if m.is_symlink() {
        line.push_str(&format!(" {:?}", fs::read_link(path).unwrap()));
    }
    result.push(line);
    if m.is_dir() {
        let mut children: Vec<PathBuf> = fs::read_dir(path)
            .unwrap()
            .map(|e| e.unwrap().path())
            .collect();
        children.sort();
        for c in children {
            inventory(&c, root, result);
        }
    }
}
#[test]
fn cli_paths_output_and_preservation() {
    let t = fixture();
    let w = t.path();
    let other = tempfile::tempdir().unwrap();
    let external_config = other.path().join("config.toml");
    put(
        &external_config,
        b"[spatial]\ngpkg_path = 'geometry.gpkg'\n",
    );
    for name in [
        "assets/registry.sqlite",
        "assets/registry.sqlite-wal",
        "assets/registry.sqlite-shm",
    ] {
        put(&w.join(name), b"sentinel");
    }
    let mut before = Vec::new();
    inventory(w, w, &mut before);
    let run = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_mgb-ops"))
            .current_dir(other.path())
            .args(args)
            .output()
            .unwrap()
    };
    let ws = w.to_str().unwrap();
    let cfg = external_config.to_str().unwrap();
    let text = run(&["workspace", "inspect", "--workspace", ws, "--config", cfg]);
    assert!(text.status.success());
    assert!(
        String::from_utf8(text.stdout)
            .unwrap()
            .contains("bounded prerequisite checks")
    );
    let json = run(&[
        "workspace",
        "inspect",
        "--workspace",
        ws,
        "--config",
        cfg,
        "--json",
    ]);
    assert!(json.status.success());
    let report: serde_json::Value = serde_json::from_slice(&json.stdout).unwrap();
    assert_eq!(report["ok"], true);
    let fail = run(&[
        "workspace",
        "inspect",
        "--workspace",
        ws,
        "--config",
        cfg,
        "--gpkg-path",
        "missing",
        "--json",
    ]);
    assert_eq!(fail.status.code(), Some(1));
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&fail.stdout).unwrap()["ok"],
        false
    );
    assert_eq!(run(&["workspace", "inspect"]).status.code(), Some(2));
    assert_eq!(
        run(&["workspace", "inspect", "--workspace", ws, "--workspace", ws])
            .status
            .code(),
        Some(2)
    );
    assert!(run(&["config", "example"]).status.success());
    let mut after = Vec::new();
    inventory(w, w, &mut after);
    assert_eq!(before, after);
    assert!(!w.join("MARKER").exists());
}

#[test]
fn config_precedence_and_invalid_values() {
    let t = fixture();
    let w = t.path();
    let cfg = w.join("config.toml");
    put(&cfg, b"[spatial]\ngpkg_path='geometry.gpkg'\n[engine]\nexecutable='no-engine'\ninput_dir='no-input'\n");
    let report = inspect(request(w));
    assert_eq!(
        code(&report, "engine.executable"),
        mgb_ops::FindingCode::Missing
    );
    assert_eq!(
        code(&report, "engine.input_dir"),
        mgb_ops::FindingCode::Missing
    );
    let mut req = request(w);
    req.engine_executable = Some("model/engine".into());
    req.engine_input_dir = Some("model/Input".into());
    assert!(inspect(req).ok);
    for source in ["", "[spatial]\n", "[engine]\n"] {
        put(&cfg, source.as_bytes());
        assert_eq!(
            code(&inspect(request(w)), "config"),
            mgb_ops::FindingCode::InvalidConfig
        );
        let mut req = request(w);
        req.gpkg_path = Some("geometry.gpkg".into());
        assert!(inspect(req).ok);
    }
    for source in [
        "unknown = 1",
        "[engine]\nunknown = 1",
        "[engine]\nexecutable = 42",
        "[engine]\ninput_dir = 42",
    ] {
        put(&cfg, source.as_bytes());
        let mut req = request(w);
        req.gpkg_path = Some("geometry.gpkg".into());
        req.engine_executable = Some("model/engine".into());
        req.engine_input_dir = Some("model/Input".into());
        let report = inspect(req);
        assert_eq!(
            code(&report, "config"),
            mgb_ops::FindingCode::InvalidConfig,
            "{source}"
        );
        if source.contains("unknown") {
            assert!(
                report
                    .findings
                    .iter()
                    .find(|f| f.check == "config")
                    .unwrap()
                    .message
                    .contains("unknown")
            );
        }
    }
    put(&cfg, &[0xff]);
    assert_eq!(
        code(&inspect(request(w)), "config"),
        mgb_ops::FindingCode::InvalidConfig
    );
    fs::remove_file(&cfg).unwrap();
    fs::create_dir(&cfg).unwrap();
    assert_eq!(
        code(&inspect(request(w)), "config"),
        mgb_ops::FindingCode::WrongKind
    );
}

#[cfg(unix)]
#[test]
fn native_paths_and_output_errors() {
    use std::os::unix::ffi::OsStringExt;
    use std::os::unix::fs::{PermissionsExt, symlink};
    use std::process::Stdio;
    let t = fixture();
    let w = t.path();
    let outside = tempfile::tempdir().unwrap();
    fs::create_dir(outside.path().join("sub")).unwrap();
    put(&outside.path().join("geometry.gpkg"), b"outside");
    symlink(outside.path().join("sub"), w.join("link")).unwrap();
    let mut req = request(w);
    req.gpkg_path = Some("link/../geometry.gpkg".into());
    let report = inspect(req);
    assert!(report.ok);
    let geom = report
        .findings
        .iter()
        .find(|f| f.check == "geometry")
        .unwrap()
        .path
        .as_ref()
        .unwrap();
    assert!(geom.ends_with("link/../geometry.gpkg"));
    fs::remove_file(outside.path().join("geometry.gpkg")).unwrap();
    let mut req = request(w);
    req.gpkg_path = Some("link/../geometry.gpkg".into());
    assert_eq!(
        code(&inspect(req), "geometry"),
        mgb_ops::FindingCode::Missing
    );
    let native = std::ffi::OsString::from_vec(b"nonunicode-\xff".to_vec());
    let mut req = request(w);
    req.gpkg_path = Some(native.into());
    let json = serde_json::to_value(inspect(req)).unwrap();
    assert!(
        json["findings"]
            .as_array()
            .unwrap()
            .iter()
            .find(|f| f["check"] == "geometry")
            .unwrap()["path"]
            .as_str()
            .unwrap()
            .contains('\u{fffd}')
    );
    let cfg = w.join("config.toml");
    fs::set_permissions(&cfg, fs::Permissions::from_mode(0o000)).unwrap();
    if fs::File::open(&cfg).is_err() {
        assert_eq!(
            code(&inspect(request(w)), "config"),
            mgb_ops::FindingCode::Unreadable
        );
    }
    fs::set_permissions(&cfg, fs::Permissions::from_mode(0o644)).unwrap();
    let full = fs::OpenOptions::new()
        .write(true)
        .open("/dev/full")
        .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_mgb-ops"))
        .args(["config", "example"])
        .stdout(Stdio::from(full))
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(!output.stderr.is_empty());
}

#[test]
fn finding_values_keep_their_wire_strings() {
    for (value, expected) in [
        (mgb_ops::FindingStatus::Pass, "pass"),
        (mgb_ops::FindingStatus::Fail, "fail"),
        (mgb_ops::FindingStatus::Info, "info"),
        (mgb_ops::FindingStatus::NotChecked, "not_checked"),
    ] {
        assert_eq!(serde_json::to_value(value).unwrap(), expected);
        assert_eq!(value.to_string(), expected);
    }
    for (value, expected) in [
        (mgb_ops::FindingCode::Ok, "ok"),
        (mgb_ops::FindingCode::Missing, "missing"),
        (mgb_ops::FindingCode::WrongKind, "wrong_kind"),
        (mgb_ops::FindingCode::Unreadable, "unreadable"),
        (mgb_ops::FindingCode::Empty, "empty"),
        (mgb_ops::FindingCode::InvalidConfig, "invalid_config"),
        (mgb_ops::FindingCode::NotExecutable, "not_executable"),
        (mgb_ops::FindingCode::Present, "present"),
        (mgb_ops::FindingCode::Absent, "absent"),
        (
            mgb_ops::FindingCode::MetadataUnavailable,
            "metadata_unavailable",
        ),
        (
            mgb_ops::FindingCode::PrerequisiteFailed,
            "prerequisite_failed",
        ),
        (mgb_ops::FindingCode::OutsideScope, "outside_scope"),
    ] {
        assert_eq!(serde_json::to_value(value).unwrap(), expected);
        assert_eq!(value.to_string(), expected);
    }
}
