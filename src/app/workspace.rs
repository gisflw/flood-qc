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

use crate::analysis::PrecipitationQcPolicy;
use crate::store::ForecastProvider;
use serde::{Deserialize, Serialize, Serializer};
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
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
    #[serde(rename = "model", alias = "engine")]
    pub engine: Option<WorkspaceEngine>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run: Option<WorkspaceRun>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub acquisition: Option<WorkspaceAcquisition>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub precipitation_interpolation: Option<super::InterpolationDocument>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub precipitation_exclude: Option<PrecipitationQcPolicy>,
}
type Config = WorkspaceConfig;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct WorkspaceSpatial {
    #[serde(rename = "mini_layers", alias = "gpkg_path")]
    pub gpkg_path: Option<String>,
    pub hand_raster: Option<PathBuf>,
    pub catchment_raster: Option<PathBuf>,
    pub bounds: Option<[f64; 4]>,
    pub resolution_degrees: Option<f64>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct WorkspaceEngine {
    pub executable: Option<String>,
    pub input_dir: Option<String>,
    pub omp_threads: Option<usize>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct WorkspaceRun {
    pub time_zone: Option<chrono_tz::Tz>,
    pub reference_time: String,
    pub lookback_days: u32,
    pub forecast_days: u32,
    pub timestep_hours: u32,
}
impl Default for WorkspaceRun {
    fn default() -> Self {
        Self {
            time_zone: None,
            reference_time: "now".into(),
            lookback_days: 5,
            forecast_days: 5,
            timestep_hours: 1,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct WorkspaceAcquisition {
    pub observation_providers: Vec<String>,
    pub forecast_providers: Vec<ForecastProvider>,
    pub request_days: u32,
    pub fetch_window_days: u32,
    pub timeout_seconds: u64,
    pub lookback_cycles: u32,
    pub buffer_fraction: f64,
}
impl Default for WorkspaceAcquisition {
    fn default() -> Self {
        Self {
            observation_providers: Vec::new(),
            forecast_providers: Vec::new(),
            request_days: 90,
            fetch_window_days: 30,
            timeout_seconds: 15,
            lookback_cycles: 4,
            buffer_fraction: 2.0,
        }
    }
}

pub(super) const CONFIG_PATH: &str = "config.toml";
pub(super) const DATABASE_PATH: &str = "operation.sqlite";

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

const LINUX_ENGINE: &str = "model/engine";
const WINDOWS_ENGINE: &str = "model/engine.exe";
pub(super) fn default_engine() -> &'static str {
    if cfg!(windows) {
        WINDOWS_ENGINE
    } else {
        LINUX_ENGINE
    }
}

pub fn example_config() -> String {
    format!(
        r#"[run]
# Required for operations. No default timezone.
# time_zone = "America/Sao_Paulo"
# Defaults
# reference_time = "now"
# lookback_days = 5
# forecast_days = 5
# timestep_hours = 1

[spatial]
# Example only. Set this path to the regional geometry.
mini_layers = "source/geometry.gpkg"
# No default HAND raster.
# hand_raster = "source/HAND_Flood.tif"
# The default is to derive the catchment raster on the HAND grid.
# catchment_raster = "source/catchments.tif"
# Required for operations. Set west, south, east and north.
# bounds = [-59.0, -35.0, -48.0, -26.0]
# Default in geographic degrees
# resolution_degrees = 0.1

[model]
# Linux default is {LINUX_ENGINE}
# Windows default is {WINDOWS_ENGINE}
# executable = "{}"
# Defaults
# input_dir = "model/Input"
# omp_threads = 4

[acquisition]
# Defaults
# observation_providers = []
# forecast_providers = []
# request_days = 90
# fetch_window_days = 30
# timeout_seconds = 15
# lookback_cycles = 4
# buffer_fraction = 2.0

[precipitation_interpolation]
# Defaults
# nearest_stations = 5
# power = 2.0

[precipitation_exclude]
# Defaults in hourly mm and consecutive samples
# high_threshold_mm = 50.0
# sequence_lower_mm = 0.0
# sequence_upper_mm = 1.0
# sequence_min_length = 5
"#,
        default_engine()
    )
}

pub(super) fn absolute(base: &Path, path: &Path) -> PathBuf {
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
        || workspace.join(CONFIG_PATH),
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
        Some("model/Input"),
    );
    if let Some(key) = [
        ("spatial.mini_layers", &gpkg),
        ("model.executable", &executable),
        ("model.input_dir", &input_dir),
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
        ("database", DATABASE_PATH),
        ("cache", "cache"),
        ("runs", "cache/run"),
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

pub fn workspace_config_path(workspace: &Path) -> PathBuf {
    workspace.join(CONFIG_PATH)
}

pub fn operation_database(workspace: &Path) -> PathBuf {
    workspace.join(DATABASE_PATH)
}

pub fn initialize_workspace(workspace: &Path) -> Result<PathBuf, String> {
    let workspace = workspace
        .canonicalize()
        .map_err(|error| error.to_string())?;
    let config = read_workspace_config(&workspace_config_path(&workspace))?;
    let zone = config
        .run
        .and_then(|run| run.time_zone)
        .ok_or("Set run.time_zone")?;
    let _lock =
        super::operation::lock_workspace_job(&workspace).map_err(|error| error.to_string())?;
    initialize_database(&workspace, zone)?;
    let path = operation_database(&workspace);
    let observations =
        crate::store::ObservationStore::open_read_only(&path).map_err(|error| error.to_string())?;
    if observations
        .time_zone()
        .map_err(|error| error.to_string())?
        != zone.name()
    {
        return Err("Database timezone differs from run.time_zone".into());
    }
    crate::store::ForecastCatalog::open_read_only(&workspace, &path)
        .map_err(|error| error.to_string())?;
    Ok(path)
}

pub(super) fn initialize_database(workspace: &Path, zone: chrono_tz::Tz) -> Result<(), String> {
    let path = operation_database(workspace);
    if !path.exists() {
        crate::store::ObservationStore::create_operation(&path, zone.name())
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub(super) fn reference(
    run: &WorkspaceRun,
    value: Option<&str>,
    now: chrono::DateTime<chrono::Utc>,
) -> Result<chrono::DateTime<chrono_tz::Tz>, String> {
    use chrono::{TimeZone, Timelike};
    let zone = run.time_zone.ok_or("Set run.time_zone")?;
    let local = match value.unwrap_or(&run.reference_time) {
        "now" => now
            .with_timezone(&zone)
            .naive_local()
            .with_minute(0)
            .unwrap()
            .with_second(0)
            .unwrap()
            .with_nanosecond(0)
            .unwrap(),
        value => {
            super::request::timestamp("run.reference_time", value).map_err(|e| e.to_string())?
        }
    };
    if local.minute() != 0 || local.second() != 0 || local.nanosecond() != 0 {
        return Err("run.reference_time must be hour-aligned".into());
    }
    zone.from_local_datetime(&local)
        .single()
        .ok_or_else(|| format!("ambiguous or nonexistent reference time {local}"))
}

pub(super) fn operational_document(
    workspace: &Path,
    config: &WorkspaceConfig,
    reference: chrono::DateTime<chrono_tz::Tz>,
    resolve_sources: bool,
    scenarios: Vec<super::ScenarioDocument>,
) -> Result<super::PreparationDocument, String> {
    use super::{
        EngineDocument, ForecastsDocument, GeometryDocument, GridDocument, LevelsDocument,
        ObservedDocument, PreparationDocument,
    };
    let run = config.run.clone().unwrap_or_default();
    if run.timestep_hours != 1 || run.forecast_days == 0 {
        return Err("Use run.timestep_hours = 1 and a positive run.forecast_days".into());
    }
    let engine = config.engine.clone().unwrap_or_default();
    let spatial = config.spatial.clone().unwrap_or_default();
    let acquisition = config.acquisition.clone().unwrap_or_default();
    let database = operation_database(workspace);
    let input_dir = PathBuf::from(engine.input_dir.unwrap_or_else(|| "model/Input".into()));
    let local = reference.naive_local();
    let first =
        super::request::historical_start(local, chrono::Duration::days(run.lookback_days.into()))
            .map_err(|e| e.to_string())?;
    let forecast_first = local
        .checked_add_signed(chrono::Duration::hours(1))
        .ok_or("forecast start is out of range")?;
    let forecast_last = forecast_first
        .checked_add_signed(chrono::Duration::days(run.forecast_days.into()))
        .ok_or("forecast end is out of range")?;
    let label = |value: chrono::NaiveDateTime| value.format("%Y-%m-%dT%H:%M:%S").to_string();
    let mut providers = acquisition.observation_providers;
    let mut levels = Vec::new();
    let mut replacements = Vec::new();
    if resolve_sources {
        let observations =
            crate::store::ObservationStore::open_read_only(&database).map_err(|e| e.to_string())?;
        let stations = observations
            .station_locations()
            .map_err(|e| e.to_string())?;
        if providers.is_empty() {
            providers.extend(
                stations
                    .iter()
                    .map(|entry| entry.station.provider_code.clone()),
            );
            providers.sort();
            providers.dedup();
        }
        levels.extend(
            stations
                .into_iter()
                .filter(|entry| {
                    entry.offset.is_some() && providers.contains(&entry.station.provider_code)
                })
                .map(|entry| entry.station.station_id),
        );
        replacements = observations
            .replacement_choices()
            .map_err(|e| e.to_string())?;
    }
    Ok(PreparationDocument {
        schema_version: 1,
        time_zone: reference.timezone().name().into(),
        reference_time: label(local),
        step_seconds: 3600,
        historical_first_end: label(first),
        historical_last_end: label(local),
        forecast_first_end: label(forecast_first),
        forecast_last_end: label(forecast_last),
        observed: ObservedDocument {
            path: database.clone(),
            provider_codes: providers,
            station_ids: Vec::new(),
            series_ids: Vec::new(),
            replacement_ids: replacements,
        },
        geometry: GeometryDocument {
            mini_gtp: input_dir.join("MINI.gtp"),
        },
        grid: GridDocument {
            bbox: spatial.bounds.ok_or("Set spatial.bounds")?,
            resolution_degrees: spatial.resolution_degrees.unwrap_or(0.1),
        },
        interpolation: config
            .precipitation_interpolation
            .clone()
            .unwrap_or_default(),
        qc: config.precipitation_exclude.clone().unwrap_or_default(),
        levels: LevelsDocument {
            station_ids: levels,
        },
        engine: EngineDocument {
            executable: engine
                .executable
                .map_or_else(|| default_engine().into(), PathBuf::from),
            input_dir,
            omp_threads: engine.omp_threads.unwrap_or(4),
        },
        forecasts: ForecastsDocument { catalog: database },
        scenarios,
        forcing: None,
        calibration: None,
        hot_start: None,
        routing: None,
    })
}
