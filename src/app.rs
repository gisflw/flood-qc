mod project;
pub use self::project::discover_rainfall_grids;
mod series;
pub use self::series::{
    ChartSeries, Location, QueryFinding, SeriesPoint, SeriesQuery, SeriesReport, SeriesSelection,
    Variable, export_comparison_csv, export_series_csv, query_comparison, query_series,
};
mod references;
pub use self::references::{
    ReferenceImportReport, import_level_references, upgrade_level_references,
};
mod operation;
mod request;
pub use self::operation::{
    BatchStatus, ExecutionReport, ExecutionRequest, OperationControl, OperationError,
    OperationProgress, OperationStage, PreparationReport, PreparationRequest, ScenarioExecution,
    ScenarioProgress, TrialCache, discover_trials, execute_prepared, execute_prepared_controlled,
    read_trial, run_draft, tail_log,
};
pub use self::request::{
    EngineDocument, ForecastsDocument, GeometryDocument, GridDocument, InterpolationDocument,
    LevelsDocument, ObservedDocument, PreparationDocument, ScenarioDocument, retained_day_horizons,
};
mod forecast;
pub use self::forecast::{
    NormalizeForecastDocument, NormalizeForecastError, NormalizeForecastReport,
    NormalizeForecastRequest, normalize_forecast, normalize_forecast_document,
};

mod prepare;
pub use self::prepare::{
    ObservedInputs, ObservedRequest, TypedPreparationRequest, prepare, prepare_controlled,
    prepare_observations, prepare_typed,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FindingCode {
    Ok,
    Missing,
    WrongKind,
    Unreadable,
    Empty,
    InvalidConfig,
    NotExecutable,
    Present,
    Absent,
    MetadataUnavailable,
    PrerequisiteFailed,
    OutsideScope,
}

impl std::fmt::Display for FindingCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Ok => "ok",
            Self::Missing => "missing",
            Self::WrongKind => "wrong_kind",
            Self::Unreadable => "unreadable",
            Self::Empty => "empty",
            Self::InvalidConfig => "invalid_config",
            Self::NotExecutable => "not_executable",
            Self::Present => "present",
            Self::Absent => "absent",
            Self::MetadataUnavailable => "metadata_unavailable",
            Self::PrerequisiteFailed => "prerequisite_failed",
            Self::OutsideScope => "outside_scope",
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FindingStatus {
    Pass,
    Fail,
    Info,
    NotChecked,
}

impl std::fmt::Display for FindingStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Pass => "pass",
            Self::Fail => "fail",
            Self::Info => "info",
            Self::NotChecked => "not_checked",
        })
    }
}

use serde::{Deserialize, Serialize, Serializer};
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct InspectionRequest {
    pub workspace: PathBuf,
    pub invocation_dir: PathBuf,
    pub config: Option<PathBuf>,
    pub gpkg_path: Option<PathBuf>,
    pub engine_executable: Option<PathBuf>,
    pub engine_input_dir: Option<PathBuf>,
}

#[derive(Debug, Serialize)]
pub struct InspectionReport {
    pub ok: bool,
    #[serde(serialize_with = "serialize_path")]
    pub workspace: PathBuf,
    #[serde(serialize_with = "serialize_path")]
    pub config: PathBuf,
    pub findings: Vec<Finding>,
}

#[derive(Debug, Serialize)]
pub struct Finding {
    pub check: String,
    pub status: FindingStatus,
    pub code: FindingCode,
    #[serde(serialize_with = "serialize_optional_path")]
    pub path: Option<PathBuf>,
    pub message: String,
}

fn serialize_path<S: Serializer>(path: &Path, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(&path.to_string_lossy())
}
fn serialize_optional_path<S: Serializer>(
    path: &Option<PathBuf>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    match path {
        Some(path) => serializer.serialize_some(&path.to_string_lossy()),
        None => serializer.serialize_none(),
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceConfig {
    pub spatial: Option<WorkspaceSpatial>,
    pub engine: Option<WorkspaceEngine>,
}
type Config = WorkspaceConfig;
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceSpatial {
    pub gpkg_path: Option<String>,
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceEngine {
    pub executable: Option<String>,
    pub input_dir: Option<String>,
}

pub fn read_workspace_config(path: &Path) -> Result<WorkspaceConfig, String> {
    let contents = fs::read_to_string(path).map_err(|e| e.to_string())?;
    toml::from_str(&contents).map_err(|e| e.to_string())
}

pub fn write_workspace_config(path: &Path, config: &WorkspaceConfig) -> Result<(), String> {
    if config
        .spatial
        .as_ref()
        .and_then(|s| s.gpkg_path.as_deref())
        .is_none_or(|p| p.trim().is_empty())
        || config.engine.as_ref().is_some_and(|e| {
            [e.executable.as_deref(), e.input_dir.as_deref()]
                .into_iter()
                .flatten()
                .any(|p| p.trim().is_empty())
        })
    {
        return Err("configuration paths must be nonempty".into());
    }
    let contents = toml::to_string_pretty(config).map_err(|e| e.to_string())?;
    crate::store::write_atomic(path, contents.as_bytes())
}

const LINUX_ENGINE: &str = "mgb_runner/MGB_Inercial_Prev_RSH";
const WINDOWS_ENGINE: &str = "mgb_runner/MGB_Inercial_PrevRS_FORTRAN.exe";
fn default_engine() -> &'static str {
    if cfg!(windows) {
        WINDOWS_ENGINE
    } else {
        LINUX_ENGINE
    }
}

pub fn example_config() -> String {
    format!(
        "[spatial]\n# Example only: replace with your regional geometry file.\ngpkg_path = \"data/source/your_region.gpkg\"\n\n[engine]\n# Linux default: {LINUX_ENGINE}\n# Windows default: {WINDOWS_ENGINE}\nexecutable = \"{}\"\ninput_dir = \"mgb_runner/Input\"\n",
        default_engine()
    )
}
fn absolute(base: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.into()
    } else {
        base.join(path)
    }
}
fn finding(
    check: &str,
    status: FindingStatus,
    code: FindingCode,
    path: Option<PathBuf>,
    message: impl Into<String>,
) -> Finding {
    Finding {
        check: check.into(),
        status,
        code,
        path,
        message: message.into(),
    }
}
fn scope(findings: &mut Vec<Finding>) {
    for (check, message) in [
        (
            "scope.contents",
            "Geometry and input content validity are outside this inspection",
        ),
        ("scope.forcing", "Forcing is outside this inspection"),
        ("scope.state", "Saved state is outside this inspection"),
        (
            "scope.runtime",
            "Runtime compatibility is outside this inspection",
        ),
        (
            "scope.execution",
            "Real execution is outside this inspection",
        ),
    ] {
        findings.push(finding(
            check,
            FindingStatus::NotChecked,
            FindingCode::OutsideScope,
            None,
            message,
        ));
    }
}
fn finish(workspace: PathBuf, config: PathBuf, mut findings: Vec<Finding>) -> InspectionReport {
    scope(&mut findings);
    InspectionReport {
        ok: !findings.iter().any(|f| f.status == FindingStatus::Fail),
        workspace,
        config,
        findings,
    }
}
fn failure(check: &str, code: FindingCode, path: PathBuf, message: &str) -> Finding {
    finding(check, FindingStatus::Fail, code, Some(path), message)
}
fn dependent(findings: &mut Vec<Finding>) {
    findings.push(finding(
        "dependent_checks",
        FindingStatus::NotChecked,
        FindingCode::PrerequisiteFailed,
        None,
        "Fix the workspace and configuration before inspecting dependent resources",
    ));
}
fn geometry(path: &Path) -> Finding {
    let result = match fs::metadata(path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            (FindingCode::Missing, "Create or select the geometry file")
        }
        Err(_) => (FindingCode::Unreadable, "Make the geometry file accessible"),
        Ok(m) if !m.is_file() => (FindingCode::WrongKind, "Select a regular geometry file"),
        Ok(_) => match File::open(path).and_then(|mut f| f.read(&mut [0u8; 1])) {
            Err(_) => (FindingCode::Unreadable, "Make the geometry file readable"),
            Ok(0) => (FindingCode::Empty, "Provide a nonempty geometry file"),
            Ok(_) => (FindingCode::Ok, "Geometry file is readable and nonempty"),
        },
    };
    finding(
        "geometry",
        if result.0 == FindingCode::Ok {
            FindingStatus::Pass
        } else {
            FindingStatus::Fail
        },
        result.0,
        Some(path.into()),
        result.1,
    )
}
fn optional_path(
    override_value: Option<PathBuf>,
    config_value: Option<String>,
    default: Option<&str>,
) -> Option<PathBuf> {
    override_value
        .or_else(|| config_value.map(PathBuf::from))
        .or_else(|| default.map(PathBuf::from))
}
fn valid_path(path: &Option<PathBuf>) -> bool {
    path.as_ref()
        .is_some_and(|p| !p.as_os_str().to_string_lossy().trim().is_empty())
}

pub fn inspect(request: InspectionRequest) -> InspectionReport {
    let workspace = absolute(&request.invocation_dir, &request.workspace);
    let config_path = request.config.as_ref().map_or_else(
        || workspace.join("config/workspace.toml"),
        |p| absolute(&request.invocation_dir, p),
    );
    let mut findings = Vec::new();
    let workspace_check = match fs::metadata(&workspace) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Some((
            FindingCode::Missing,
            "Create or select the workspace directory",
        )),
        Err(_) => Some((
            FindingCode::Unreadable,
            "Make the workspace directory accessible",
        )),
        Ok(m) if !m.is_dir() => Some((FindingCode::WrongKind, "Select a workspace directory")),
        Ok(_) if fs::read_dir(&workspace).is_err() => Some((
            FindingCode::Unreadable,
            "Make the workspace directory accessible",
        )),
        Ok(_) => None,
    };
    if let Some((code, message)) = workspace_check {
        findings.push(failure("workspace", code, workspace.clone(), message));
        dependent(&mut findings);
        return finish(workspace, config_path, findings);
    }
    findings.push(finding(
        "workspace",
        FindingStatus::Pass,
        FindingCode::Ok,
        Some(workspace.clone()),
        "Workspace directory is accessible",
    ));
    let config_contents = match fs::metadata(&config_path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Err((
            FindingCode::Missing,
            "Create or select a TOML configuration file",
        )),
        Err(_) => Err((
            FindingCode::Unreadable,
            "Make the configuration file accessible",
        )),
        Ok(m) if !m.is_file() => Err((
            FindingCode::WrongKind,
            "Select a regular configuration file",
        )),
        Ok(_) => fs::read_to_string(&config_path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::InvalidData {
                (
                    FindingCode::InvalidConfig,
                    "Save the configuration as UTF-8 TOML",
                )
            } else {
                (
                    FindingCode::Unreadable,
                    "Make the configuration file readable",
                )
            }
        }),
    };
    let contents = match config_contents {
        Ok(c) => c,
        Err((code, message)) => {
            findings.push(failure("config", code, config_path.clone(), message));
            dependent(&mut findings);
            return finish(workspace, config_path, findings);
        }
    };
    let parsed: Config = match toml::from_str(&contents) {
        Ok(c) => c,
        Err(e) => {
            let location = e
                .span()
                .map(|s| {
                    let prefix = &contents[..s.start.min(contents.len())];
                    format!(
                        " at line {}, column {}",
                        prefix.bytes().filter(|b| *b == b'\n').count() + 1,
                        prefix.rsplit('\n').next().unwrap_or("").chars().count() + 1
                    )
                })
                .unwrap_or_default();
            let detail = e
                .message()
                .strip_prefix("unknown field `")
                .and_then(|rest| rest.split('`').next())
                .filter(|key| {
                    key.chars()
                        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
                })
                .map(|key| format!(" unknown key `{key}`;"))
                .unwrap_or_default();
            findings.push(failure("config", FindingCode::InvalidConfig, config_path.clone(), &format!("Invalid TOML configuration{location};{detail} correct its syntax, keys, or value types")));
            dependent(&mut findings);
            return finish(workspace, config_path, findings);
        }
    };
    let spatial = parsed.spatial.unwrap_or_default();
    let engine = parsed.engine.unwrap_or_default();
    let gpkg = optional_path(request.gpkg_path, spatial.gpkg_path, None);
    let executable = optional_path(
        request.engine_executable,
        engine.executable,
        Some(default_engine()),
    );
    let input_dir = optional_path(
        request.engine_input_dir,
        engine.input_dir,
        Some("mgb_runner/Input"),
    );
    if let Some(key) = [
        ("spatial.gpkg_path", &gpkg),
        ("engine.executable", &executable),
        ("engine.input_dir", &input_dir),
    ]
    .iter()
    .find_map(|(k, p)| (!valid_path(p)).then_some(k))
    {
        findings.push(failure(
            "config",
            FindingCode::InvalidConfig,
            config_path.clone(),
            &format!("Set {key} to a nonempty path"),
        ));
        dependent(&mut findings);
        return finish(workspace, config_path, findings);
    }
    findings.push(finding(
        "config",
        FindingStatus::Pass,
        FindingCode::Ok,
        Some(config_path.clone()),
        "Configuration is valid",
    ));
    let gpkg = absolute(&workspace, gpkg.as_deref().unwrap());
    let executable = absolute(&workspace, executable.as_deref().unwrap());
    let input_dir = absolute(&workspace, input_dir.as_deref().unwrap());
    findings.push(geometry(&gpkg));
    findings.extend(
        crate::model::inspect(&executable, &input_dir)
            .into_iter()
            .map(|p| {
                finding(
                    &p.check,
                    if p.code == crate::model::PrerequisiteCode::Ok {
                        FindingStatus::Pass
                    } else if p.code == crate::model::PrerequisiteCode::PrerequisiteFailed {
                        FindingStatus::NotChecked
                    } else {
                        FindingStatus::Fail
                    },
                    match p.code {
                        crate::model::PrerequisiteCode::Ok => FindingCode::Ok,
                        crate::model::PrerequisiteCode::Missing => FindingCode::Missing,
                        crate::model::PrerequisiteCode::WrongKind => FindingCode::WrongKind,
                        crate::model::PrerequisiteCode::Unreadable => FindingCode::Unreadable,
                        crate::model::PrerequisiteCode::Empty => FindingCode::Empty,
                        crate::model::PrerequisiteCode::NotExecutable => FindingCode::NotExecutable,
                        crate::model::PrerequisiteCode::PrerequisiteFailed => {
                            FindingCode::PrerequisiteFailed
                        }
                    },
                    Some(p.path),
                    p.message,
                )
            }),
    );
    for (name, relative) in [
        ("history", "data/history.sqlite"),
        ("cache", "data/cache"),
        ("runs", "data/runs"),
    ] {
        let path = workspace.join(relative);
        let (code, message) = match fs::metadata(&path) {
            Ok(_) => (FindingCode::Present, "Optional resource is present"),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                (FindingCode::Absent, "Optional resource is absent")
            }
            Err(_) => (
                FindingCode::MetadataUnavailable,
                "Optional resource metadata is unavailable",
            ),
        };
        findings.push(finding(
            &format!("optional.{name}"),
            FindingStatus::Info,
            code,
            Some(path),
            message,
        ));
    }
    finish(workspace, config_path, findings)
}

mod draft;
pub use self::draft::{
    Draft, DraftAction, DraftCorrection, DraftDocument, DraftEngine, DraftFinding, DraftForecasts,
    DraftGeometry, DraftGrid, DraftInstructions, DraftInterpolation, DraftLevels, DraftObserved,
    DraftOperation, DraftQc, DraftReplacement, DraftScenario, DraftSources, DraftVisualization,
    ForecastPreview, ObservationPreview, RegistrationCorrectionSelection, RegistrationMapping,
    RegistrationOutcome, RegistrationReport, RegistrationSelection,
};
