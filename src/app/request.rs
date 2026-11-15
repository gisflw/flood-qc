use crate::analysis::PrecipitationQcPolicy;
use crate::store::{IdwSettings, IntervalAxis, RegularGridSpec, TimeInterval};
use chrono::{Duration, NaiveDateTime, Timelike};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub(super) struct RequestError {
    pub field: String,
    pub message: String,
}
impl std::fmt::Display for RequestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.field, self.message)
    }
}
impl std::error::Error for RequestError {}
fn invalid(field: &str, message: impl Into<String>) -> RequestError {
    RequestError {
        field: field.into(),
        message: message.into(),
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct PreparationDocument {
    pub schema_version: u32,
    pub time_zone: String,
    pub reference_time: String,
    pub step_seconds: u32,
    pub historical_first_end: String,
    pub historical_last_end: String,
    pub forecast_first_end: String,
    pub forecast_last_end: String,
    pub observed: ObservedDocument,
    pub geometry: GeometryDocument,
    pub grid: GridDocument,
    pub interpolation: InterpolationDocument,
    pub qc: PrecipitationQcPolicy,
    pub levels: LevelsDocument,
    pub engine: EngineDocument,
    pub forecasts: ForecastsDocument,
    pub scenarios: Vec<ScenarioDocument>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ObservedDocument {
    pub path: PathBuf,
    pub provider_codes: Vec<String>,
    pub station_ids: Vec<String>,
    #[serde(default)]
    pub series_ids: Vec<String>,
    pub replacement_ids: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct GeometryDocument {
    pub mini_gtp: PathBuf,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct GridDocument {
    pub bbox: [f64; 4],
    pub resolution_degrees: f64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct InterpolationDocument {
    pub nearest_stations: usize,
    pub power: f64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct LevelsDocument {
    pub station_ids: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct EngineDocument {
    pub executable: PathBuf,
    pub input_dir: PathBuf,
    pub omp_threads: usize,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ForecastsDocument {
    pub catalog: PathBuf,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ScenarioDocument {
    pub id: String,
    pub kind: String,
    pub asset_id: Option<String>,
    pub correction_ids: Option<Vec<String>>,
}
pub(super) struct ValidatedRequest {
    pub document: PreparationDocument,
    pub historical: IntervalAxis,
    pub forecast: IntervalAxis,
    pub request_bytes: Vec<u8>,
    pub grid: RegularGridSpec,
    pub interpolation: IdwSettings,
    pub qc: PrecipitationQcPolicy,
}
fn timestamp(field: &str, value: &str) -> Result<NaiveDateTime, RequestError> {
    let time = NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M:%S")
        .map_err(|_| invalid(field, "expected local YYYY-MM-DDTHH:MM:SS"))?;
    if value.len() != 19
        || time.format("%Y-%m-%dT%H:%M:%S").to_string() != value
        || time.minute() != 0
        || time.second() != 0
        || time.nanosecond() != 0
    {
        return Err(invalid(field, "expected hour-aligned local timestamp"));
    }
    Ok(time)
}
fn axis(
    first: NaiveDateTime,
    last: NaiveDateTime,
    zone: &str,
    field: &str,
) -> Result<IntervalAxis, RequestError> {
    if first > last {
        return Err(invalid(field, "first endpoint is after last"));
    }
    let count = usize::try_from((last - first).num_hours())
        .ok()
        .and_then(|v| v.checked_add(1))
        .ok_or_else(|| invalid(field, "timeline size overflow"))?;
    let mut bounds = Vec::new();
    bounds
        .try_reserve_exact(count)
        .map_err(|_| invalid(field, "timeline allocation failed"))?;
    for index in 0..count {
        let end = first
            .checked_add_signed(Duration::hours(index as i64))
            .ok_or_else(|| invalid(field, "timestamp overflow"))?;
        let start = end
            .checked_sub_signed(Duration::hours(1))
            .ok_or_else(|| invalid(field, "timestamp overflow"))?;
        bounds.push(TimeInterval { start, end });
    }
    let axis = IntervalAxis {
        time_zone: zone.into(),
        bounds,
    };
    axis.validate().map_err(|e| invalid(field, e.to_string()))?;
    Ok(axis)
}
fn identifiers(field: &str, ids: &[String]) -> Result<(), RequestError> {
    let mut seen = std::collections::HashSet::new();
    if ids
        .iter()
        .any(|id| id.trim().is_empty() || !seen.insert(id))
    {
        return Err(invalid(field, "IDs must be nonempty and unique"));
    }
    Ok(())
}
fn resolve_file(
    workspace: &Path,
    field: &str,
    path: &mut PathBuf,
    directory: bool,
) -> Result<(), RequestError> {
    if path.as_os_str().is_empty() || path.to_string_lossy().trim().is_empty() {
        return Err(invalid(field, "path must be nonempty"));
    }
    *path = super::absolute(workspace, path);
    let metadata = std::fs::metadata(&*path)
        .map_err(|e| invalid(field, format!("{}: {e}", path.display())))?;
    if if directory {
        !metadata.is_dir()
    } else {
        !metadata.is_file()
    } {
        return Err(invalid(field, "wrong source resource kind"));
    }
    Ok(())
}
pub(super) fn parse_request(request_bytes: Vec<u8>) -> Result<ValidatedRequest, RequestError> {
    let text = std::str::from_utf8(&request_bytes)
        .map_err(|_| invalid("request", "expected UTF-8 TOML"))?;
    let document: PreparationDocument =
        toml::from_str(text).map_err(|e| invalid("request", e.message()))?;
    if document.schema_version != 1 {
        return Err(invalid("schema_version", "unsupported schema version"));
    }
    if document.step_seconds != 3600 {
        return Err(invalid("step_seconds", "only hourly steps are qualified"));
    }
    document
        .time_zone
        .parse::<chrono_tz::Tz>()
        .map_err(|_| invalid("time_zone", "unknown IANA timezone"))?;
    let reference = timestamp("reference_time", &document.reference_time)?;
    let hfirst = timestamp("historical_first_end", &document.historical_first_end)?;
    let hlast = timestamp("historical_last_end", &document.historical_last_end)?;
    let ffirst = timestamp("forecast_first_end", &document.forecast_first_end)?;
    let flast = timestamp("forecast_last_end", &document.forecast_last_end)?;
    if reference != hlast || hlast.checked_add_signed(Duration::hours(1)) != Some(ffirst) {
        return Err(invalid(
            "timeline",
            "reference must end history and forecast must immediately follow",
        ));
    }
    let historical = axis(hfirst, hlast, &document.time_zone, "historical")?;
    let forecast = axis(ffirst, flast, &document.time_zone, "forecast")?;
    let mut combined = historical.clone();
    combined.bounds.extend(forecast.bounds.iter().cloned());
    combined
        .validate()
        .map_err(|e| invalid("timeline", e.to_string()))?;
    let grid = RegularGridSpec::new(document.grid.bbox, document.grid.resolution_degrees)
        .map_err(|e| invalid("grid", e.to_string()))?;
    let interpolation = IdwSettings {
        nearest_stations: document.interpolation.nearest_stations,
        power: document.interpolation.power,
    };
    interpolation
        .validate()
        .map_err(|e| invalid("interpolation", e.to_string()))?;
    let qc = document.qc.clone();
    if ![
        qc.high_threshold_mm,
        qc.sequence_lower_mm,
        qc.sequence_upper_mm,
    ]
    .iter()
    .all(|v| v.is_finite())
        || qc.high_threshold_mm < 0.
        || qc.sequence_lower_mm < 0.
        || qc.sequence_lower_mm >= qc.sequence_upper_mm
        || qc.sequence_min_length == 0
    {
        return Err(invalid(
            "qc",
            "invalid thresholds or minimum sequence length",
        ));
    }
    for (field, ids) in [
        ("observed.provider_codes", &document.observed.provider_codes),
        ("observed.station_ids", &document.observed.station_ids),
        ("observed.series_ids", &document.observed.series_ids),
        (
            "observed.replacement_ids",
            &document.observed.replacement_ids,
        ),
        ("levels.station_ids", &document.levels.station_ids),
    ] {
        identifiers(field, ids)?;
    }
    if document.observed.provider_codes.is_empty() && document.observed.station_ids.is_empty() {
        return Err(invalid(
            "observed",
            "select at least one provider or station",
        ));
    }
    if document.engine.omp_threads == 0 {
        return Err(invalid(
            "engine.omp_threads",
            "thread count must be positive",
        ));
    }
    if document.scenarios.is_empty() {
        return Err(invalid("scenarios", "select at least one scenario"));
    }
    let mut scenario_ids = std::collections::HashSet::new();
    for scenario in &document.scenarios {
        let id = scenario.id.as_bytes();
        if id.is_empty()
            || id.len() > 64
            || !id[0].is_ascii_alphanumeric()
            || id
                .iter()
                .any(|c| !c.is_ascii_alphanumeric() && *c != b'_' && *c != b'-')
            || !scenario_ids.insert(&scenario.id)
        {
            return Err(invalid(
                "scenarios.id",
                "IDs must be unique and match [A-Za-z0-9][A-Za-z0-9_-]{0,63}",
            ));
        }
        let asset = scenario
            .asset_id
            .as_ref()
            .is_some_and(|id| !id.trim().is_empty());
        let corrections = scenario.correction_ids.as_deref().unwrap_or(&[]);
        identifiers("scenarios.correction_ids", corrections)?;
        let valid = match scenario.kind.as_str() {
            "zero" => scenario.asset_id.is_none() && scenario.correction_ids.is_none(),
            "raw" => asset && corrections.is_empty(),
            "corrected" => asset && !corrections.is_empty(),
            _ => false,
        };
        if !valid {
            return Err(invalid(
                "scenarios",
                "invalid kind/asset/correction selection",
            ));
        }
    }
    Ok(ValidatedRequest {
        document,
        historical,
        forecast,
        request_bytes,
        grid,
        interpolation,
        qc,
    })
}

pub(super) fn read_request(
    workspace: &Path,
    request_file: &Path,
) -> Result<ValidatedRequest, RequestError> {
    let request_bytes =
        std::fs::read(request_file).map_err(|e| invalid("request", e.to_string()))?;
    let mut accepted = parse_request(request_bytes)?;
    let document = &mut accepted.document;
    resolve_file(
        workspace,
        "observed.path",
        &mut document.observed.path,
        false,
    )?;
    resolve_file(
        workspace,
        "geometry.mini_gtp",
        &mut document.geometry.mini_gtp,
        false,
    )?;
    resolve_file(
        workspace,
        "engine.executable",
        &mut document.engine.executable,
        false,
    )?;
    resolve_file(
        workspace,
        "engine.input_dir",
        &mut document.engine.input_dir,
        true,
    )?;
    if document
        .scenarios
        .iter()
        .any(|scenario| scenario.kind != "zero")
    {
        resolve_file(
            workspace,
            "forecasts.catalog",
            &mut document.forecasts.catalog,
            false,
        )?;
    } else {
        if document.forecasts.catalog.as_os_str().is_empty()
            || document
                .forecasts
                .catalog
                .to_string_lossy()
                .trim()
                .is_empty()
        {
            return Err(invalid("forecasts.catalog", "path must be nonempty"));
        }
        document.forecasts.catalog = super::absolute(workspace, &document.forecasts.catalog);
    }
    Ok(accepted)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn request_text() -> String {
        r#"schema_version = 1
time_zone = "America/Sao_Paulo"
reference_time = "2026-07-20T00:00:00"
step_seconds = 3600
historical_first_end = "2026-07-10T00:00:00"
historical_last_end = "2026-07-20T00:00:00"
forecast_first_end = "2026-07-20T01:00:00"
forecast_last_end = "2026-07-25T01:00:00"
[observed]
path = "observations.sqlite"
provider_codes = ["ana"]
station_ids = []
replacement_ids = []
[geometry]
mini_gtp = "MINI.gtp"
[grid]
bbox = [-58.95016,-34.40217,-48.85759,-26.29721]
resolution_degrees = 0.1
[interpolation]
nearest_stations = 5
power = 2.0
[qc]
high_threshold_mm = 50.0
sequence_lower_mm = 0.0
sequence_upper_mm = 1.0
sequence_min_length = 5
[levels]
station_ids = []
[engine]
executable = "engine"
input_dir = "Input"
omp_threads = 1
[forecasts]
catalog = "missing-forecast.sqlite"
[[scenarios]]
id = "zero"
kind = "zero"
"#
        .into()
    }
    #[test]
    fn saved_axis_and_paths_are_explicit_and_zero_needs_no_forecast_catalog() {
        let dir = tempfile::tempdir().unwrap();
        for file in ["observations.sqlite", "MINI.gtp", "engine"] {
            std::fs::write(dir.path().join(file), b"fixture").unwrap();
        }
        std::fs::create_dir(dir.path().join("Input")).unwrap();
        let path = dir.path().join("request.toml");
        std::fs::write(&path, request_text()).unwrap();
        let valid = read_request(dir.path(), &path).unwrap();
        assert_eq!(valid.historical.bounds.len(), 241);
        assert_eq!(valid.forecast.bounds.len(), 121);
        assert_eq!(
            valid.historical.bounds.last().unwrap().end,
            valid.forecast.bounds[0].start
        );
        assert_eq!(valid.document.engine.executable, dir.path().join("engine"));
        if valid.document.forecasts.catalog.exists() {
            panic!("zero forecast created a catalog");
        }
        assert!(valid.document.observed.series_ids.is_empty());
        for text in [
            request_text().replace("schema_version = 1", "schema_version = 2"),
            request_text().replace("step_seconds = 3600", "step_seconds = 1800"),
            request_text().replace(
                "reference_time = \"2026-07-20T00:00:00\"",
                "reference_time = \"2026-07-20T01:00:00\"",
            ),
            request_text().replace(
                "forecast_first_end = \"2026-07-20T01:00:00\"",
                "forecast_first_end = \"2026-07-20T02:00:00\"",
            ),
            request_text().replace(
                "resolution_degrees = 0.1",
                "resolution_degrees = 0.1\nfuture_key = 1",
            ),
            request_text().replace("replacement_ids = []\n", ""),
            request_text().replace("replacement_ids = []", "replacement_ids = [\"r\",\"r\"]"),
            request_text().replace("id = \"zero\"", "id = \"../escape\""),
            request_text().replace("kind = \"zero\"", "kind = \"zero\"\nasset_id = \"x\""),
            request_text().replace("kind = \"zero\"", "kind = \"raw\""),
            request_text().replace(
                "kind = \"zero\"",
                "kind = \"corrected\"\nasset_id = \"x\"\ncorrection_ids = []",
            ),
            request_text().replace("sequence_upper_mm = 1.0", "sequence_upper_mm = 0.0"),
            request_text().replace("omp_threads = 1", "omp_threads = 0"),
            request_text().replace("power = 2.0", "power = nan"),
        ] {
            std::fs::write(&path, text).unwrap();
            assert!(read_request(dir.path(), &path).is_err());
        }
        if dir.path().join("missing-forecast.sqlite").exists() {
            panic!("invalid requests created a catalog");
        }
    }
}
