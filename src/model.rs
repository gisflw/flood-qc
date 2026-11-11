#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PrerequisiteCode {
    Ok,
    Missing,
    WrongKind,
    Unreadable,
    Empty,
    NotExecutable,
    PrerequisiteFailed,
}

use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};

pub(crate) const REQUIRED_INPUTS: &[&str] = &[
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

#[derive(Debug)]
pub(crate) struct Prerequisite {
    pub check: String,
    pub code: PrerequisiteCode,
    pub path: PathBuf,
    pub message: String,
}

fn directory(check: &str, path: &Path) -> Prerequisite {
    let result = match fs::metadata(path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            (PrerequisiteCode::Missing, "Create or select the directory")
        }
        Err(_) => (
            PrerequisiteCode::Unreadable,
            "Make the directory accessible",
        ),
        Ok(m) if !m.is_dir() => (PrerequisiteCode::WrongKind, "Select a directory"),
        Ok(_) => match fs::read_dir(path) {
            Ok(_) => (PrerequisiteCode::Ok, "Directory is accessible"),
            Err(_) => (
                PrerequisiteCode::Unreadable,
                "Make the directory accessible",
            ),
        },
    };
    Prerequisite {
        check: check.into(),
        code: result.0,
        path: path.into(),
        message: result.1.into(),
    }
}

fn file(check: &str, path: &Path, executable: bool) -> Prerequisite {
    let result = match fs::metadata(path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => (
            PrerequisiteCode::Missing,
            "Create or select the required file",
        ),
        Err(_) => (PrerequisiteCode::Unreadable, "Make the file accessible"),
        Ok(m) if !m.is_file() => (PrerequisiteCode::WrongKind, "Select a regular file"),
        Ok(m) => match File::open(path).and_then(|mut f| f.read(&mut [0u8; 1])) {
            Err(_) => (PrerequisiteCode::Unreadable, "Make the file readable"),
            Ok(0) => (PrerequisiteCode::Empty, "Provide a nonempty file"),
            Ok(_) if executable && !executable_bits(&m) => (
                PrerequisiteCode::NotExecutable,
                "Set an executable permission bit",
            ),
            Ok(_) => (PrerequisiteCode::Ok, "File is readable and nonempty"),
        },
    };
    Prerequisite {
        check: check.into(),
        code: result.0,
        path: path.into(),
        message: result.1.into(),
    }
}

#[cfg(target_os = "linux")]
fn executable_bits(metadata: &fs::Metadata) -> bool {
    use std::os::unix::fs::PermissionsExt;
    metadata.permissions().mode() & 0o111 != 0
}
#[cfg(not(target_os = "linux"))]
fn executable_bits(_: &fs::Metadata) -> bool {
    true
}

pub(crate) fn inspect(executable: &Path, input_dir: &Path) -> Vec<Prerequisite> {
    let mut findings = vec![
        file("engine.executable", executable, true),
        directory("engine.input_dir", input_dir),
    ];
    if findings[1].code == PrerequisiteCode::Ok {
        findings.extend(REQUIRED_INPUTS.iter().map(|name| {
            file(
                &format!("engine.input.{name}"),
                &input_dir.join(name),
                false,
            )
        }));
    } else {
        findings.push(Prerequisite {
            check: "engine.inputs".into(),
            code: PrerequisiteCode::PrerequisiteFailed,
            path: input_dir.into(),
            message: "Fix the input directory before checking its files".into(),
        });
    }
    findings
}
