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
pub struct PreparationDocument {
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
pub struct ObservedDocument {
    pub path: PathBuf,
    pub provider_codes: Vec<String>,
    pub station_ids: Vec<String>,
    #[serde(default)]
    pub series_ids: Vec<String>,
    pub replacement_ids: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GeometryDocument {
    pub mini_gtp: PathBuf,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GridDocument {
    pub bbox: [f64; 4],
    pub resolution_degrees: f64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InterpolationDocument {
    pub nearest_stations: usize,
    pub power: f64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LevelsDocument {
    pub station_ids: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EngineDocument {
    pub executable: PathBuf,
    pub input_dir: PathBuf,
    pub omp_threads: usize,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ForecastsDocument {
    pub catalog: PathBuf,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScenarioDocument {
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
pub(super) fn timestamp(field: &str, value: &str) -> Result<NaiveDateTime, RequestError> {
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
pub(super) fn axis(
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
pub(super) fn compact_period(value: &mut serde_json::Value) -> bool {
    let Some(reference) = value
        .get("reference_time")
        .and_then(|v| v.as_str())
        .and_then(|v| timestamp("reference_time", v).ok())
    else {
        return false;
    };
    let Some(first) = value
        .get("historical_first_end")
        .and_then(|v| v.as_str())
        .and_then(|v| timestamp("historical_first_end", v).ok())
    else {
        return false;
    };
    let Some(last) = value
        .get("forecast_last_end")
        .and_then(|v| v.as_str())
        .and_then(|v| timestamp("forecast_last_end", v).ok())
    else {
        return false;
    };
    let history = reference
        .date()
        .signed_duration_since(first.date())
        .num_days();
    let forecast = (last - reference - Duration::hours(1)).num_seconds();
    if first.time() != chrono::NaiveTime::MIN
        || history < 0
        || forecast <= 0
        || forecast % 86400 != 0
    {
        return false;
    }
    let (Ok(history), Ok(forecast)) = (u32::try_from(history), u32::try_from(forecast / 86400))
    else {
        return false;
    };
    let mut compact = value.clone();
    let object = compact.as_object_mut().unwrap();
    for key in [
        "step_seconds",
        "historical_first_end",
        "historical_last_end",
        "forecast_first_end",
        "forecast_last_end",
    ] {
        object.remove(key);
    }
    object.insert("observed_horizon_days".into(), history.into());
    object.insert("forecast_horizon_days".into(), forecast.into());
    let mut expanded = compact.clone();
    if expand_period(&mut expanded).is_err() || expanded != *value {
        return false;
    }
    *value = compact;
    true
}

pub(super) fn historical_start(
    reference: NaiveDateTime,
    days: Duration,
) -> Result<NaiveDateTime, RequestError> {
    reference
        .date()
        .and_hms_opt(0, 0, 0)
        .unwrap()
        .checked_sub_signed(days)
        .ok_or_else(|| invalid("observed_horizon_days", "timestamp overflow"))
}

// Authoring periods expand once; retained recipes keep the qualified v1 representation.
pub(super) fn expand_period(value: &mut serde_json::Value) -> Result<(), RequestError> {
    let object = value
        .as_object_mut()
        .ok_or_else(|| invalid("request", "expected object"))?;
    if !object.contains_key("observed_horizon_days")
        && !object.contains_key("forecast_horizon_days")
    {
        return Ok(());
    }
    for key in [
        "step_seconds",
        "historical_first_end",
        "historical_last_end",
        "forecast_first_end",
        "forecast_last_end",
    ] {
        if object.get(key).is_some_and(|v| !v.is_null()) {
            return Err(invalid(
                key,
                "cannot combine explicit boundaries with day horizons",
            ));
        }
    }
    let reference = timestamp(
        "reference_time",
        object
            .get("reference_time")
            .and_then(|v| v.as_str())
            .ok_or_else(|| invalid("reference_time", "required"))?,
    )?;
    let days = |key: &str| -> Result<Duration, RequestError> {
        let count = object
            .get(key)
            .and_then(|v| v.as_u64())
            .and_then(|v| i64::try_from(v).ok())
            .ok_or_else(|| invalid(key, "expected nonnegative integer days"))?;
        Duration::try_days(count).ok_or_else(|| invalid(key, "day horizon overflow"))
    };
    let history = days("observed_horizon_days")?;
    let forecast = days("forecast_horizon_days")?;
    if forecast == Duration::zero() {
        return Err(invalid(
            "forecast_horizon_days",
            "expected positive integer days",
        ));
    }
    let hfirst = historical_start(reference, history)?;
    let ffirst = reference
        .checked_add_signed(Duration::hours(1))
        .ok_or_else(|| invalid("reference_time", "timestamp overflow"))?;
    let flast = ffirst
        .checked_add_signed(forecast)
        .ok_or_else(|| invalid("forecast_horizon_days", "timestamp overflow"))?;
    object.remove("observed_horizon_days");
    object.remove("forecast_horizon_days");
    object.insert("step_seconds".into(), 3600.into());
    for (key, time) in [
        ("historical_first_end", hfirst),
        ("historical_last_end", reference),
        ("forecast_first_end", ffirst),
        ("forecast_last_end", flast),
    ] {
        object.insert(
            key.into(),
            time.format("%Y-%m-%dT%H:%M:%S").to_string().into(),
        );
    }
    Ok(())
}

pub(super) fn parse_request(request_bytes: Vec<u8>) -> Result<ValidatedRequest, RequestError> {
    let document = decode_request(&request_bytes)?;
    validate_document(document, request_bytes)
}

pub(super) fn decode_request(request_bytes: &[u8]) -> Result<PreparationDocument, RequestError> {
    let text = std::str::from_utf8(request_bytes)
        .map_err(|_| invalid("request", "expected UTF-8 TOML"))?;
    let raw: toml::Value = toml::from_str(text).map_err(|e| invalid("request", e.message()))?;
    let mut value = serde_json::to_value(raw).map_err(|e| invalid("request", e.to_string()))?;
    if value.get("schema_version").and_then(|v| v.as_u64()) == Some(2) {
        if value.get("observed_horizon_days").is_none()
            || value.get("forecast_horizon_days").is_none()
        {
            return Err(invalid("timeline", "version 2 requires both day horizons"));
        }
        expand_period(&mut value)?;
        value["schema_version"] = 1.into();
    }
    serde_json::from_value(value).map_err(|e| invalid("request", e.to_string()))
}

pub(super) fn validate_document(
    document: PreparationDocument,
    request_bytes: Vec<u8>,
) -> Result<ValidatedRequest, RequestError> {
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
    resolve_request(workspace, parse_request(request_bytes)?)
}

pub(super) fn resolve_request(
    workspace: &Path,
    mut accepted: ValidatedRequest,
) -> Result<ValidatedRequest, RequestError> {
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
    fn v2_text() -> String {
        let mut text = request_text().replace("schema_version = 1", "schema_version = 2");
        for line in request_text().lines().filter(|line| {
            line.starts_with("step_seconds")
                || line.starts_with("historical_")
                || line.starts_with("forecast_")
        }) {
            text = text.replace(&format!("{line}\n"), "");
        }
        text.replacen(
            "[observed]",
            "observed_horizon_days = 10\nforecast_horizon_days = 5\n[observed]",
            1,
        )
    }
    #[test]
    fn reference_horizons_preserve_legacy_labels_and_reject_invalid_periods() {
        let valid = parse_request(v2_text().into_bytes()).unwrap();
        let legacy = parse_request(request_text().into_bytes()).unwrap();
        assert_eq!(valid.historical, legacy.historical);
        assert_eq!(valid.forecast, legacy.forecast);
        assert_eq!(
            serde_json::to_value(&valid.document).unwrap(),
            serde_json::to_value(&legacy.document).unwrap()
        );
        assert_eq!(valid.historical.bounds.len(), 241);
        assert_eq!(valid.forecast.bounds.len(), 121);
        let nonmidnight = parse_request(
            v2_text()
                .replace("2026-07-20T00:00:00", "2026-07-20T06:00:00")
                .into_bytes(),
        )
        .unwrap();
        assert_eq!(nonmidnight.historical.bounds.len(), 247);
        assert_eq!(nonmidnight.forecast.bounds.len(), 121);
        let zero_history = parse_request(
            v2_text()
                .replace("observed_horizon_days = 10", "observed_horizon_days = 0")
                .into_bytes(),
        )
        .unwrap();
        assert_eq!(zero_history.historical.bounds.len(), 1);
        for text in [
            v2_text().replace("forecast_horizon_days = 5", "forecast_horizon_days = 0"),
            v2_text().replace("observed_horizon_days = 10", "observed_horizon_days = -1"),
            v2_text().replace(
                "observed_horizon_days = 10",
                "observed_horizon_days = 9223372036854775807",
            ),
            v2_text().replace(
                "forecast_horizon_days = 5",
                "forecast_horizon_days = 9223372036854775807",
            ),
            v2_text().replace("[observed]", "step_seconds = 3600\n[observed]"),
            v2_text().replace(
                "[observed]",
                "historical_first_end = \"2026-07-10T00:00:00\"\n[observed]",
            ),
        ] {
            assert!(parse_request(text.into_bytes()).is_err());
        }
        for reference in [
            "2026-03-08T02:00:00",
            "2026-11-01T01:00:00",
            "2026-03-09T00:00:00",
        ] {
            let text = v2_text()
                .replace("America/Sao_Paulo", "America/New_York")
                .replace("2026-07-20T00:00:00", reference);
            assert!(parse_request(text.into_bytes()).is_err());
        }
        let mut value = serde_json::to_value(valid.document).unwrap();
        assert!(compact_period(&mut value));
        assert_eq!(value["observed_horizon_days"], 10);
        let mut arbitrary = serde_json::to_value(nonmidnight.document).unwrap();
        arbitrary["historical_first_end"] = "2026-07-10T01:00:00".into();
        let original = arbitrary.clone();
        let remained_explicit = !compact_period(&mut arbitrary);
        assert!(remained_explicit);
        assert_eq!(arbitrary, original);
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
