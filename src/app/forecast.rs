use std::path::PathBuf;

use chrono::{DateTime, Timelike, Utc};
use serde::{Deserialize, Serialize};

use crate::adapters::{NormalizationError, NormalizationRequest, normalize_saved_forecast};
use crate::store::{ForecastCatalog, ForecastProvider, IntervalAxis, StoreError};

#[derive(Debug)]
pub enum NormalizeForecastError {
    Configuration(String),
    Normalization(NormalizationError),
    Persistence(StoreError),
}
impl std::fmt::Display for NormalizeForecastError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Configuration(message) => write!(f, "forecast request: {message}"),
            Self::Normalization(error) => write!(f, "forecast normalization: {error}"),
            Self::Persistence(error) => write!(f, "forecast persistence: {error}"),
        }
    }
}
impl std::error::Error for NormalizeForecastError {}
#[derive(Clone, Debug)]
pub struct NormalizeForecastRequest {
    pub invocation_dir: PathBuf,
    pub workspace: PathBuf,
    pub request_file: PathBuf,
}
#[derive(Clone, Debug, Serialize)]
pub struct NormalizeForecastReport {
    pub source_id: String,
    pub asset_id: String,
    pub retained_source: PathBuf,
    pub native_artifact: PathBuf,
    pub timeline: IntervalAxis,
    pub native_shape: [usize; 3],
    pub effective_bbox: [f64; 4],
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    schema_version: u32,
    provider: ForecastProvider,
    model: String,
    product: String,
    cycle_utc: String,
    operation_timezone: String,
    source: PathBuf,
    crop_bbox: [f64; 4],
    catalog: PathBuf,
    artifact_root: PathBuf,
}
pub fn normalize_forecast(
    request: NormalizeForecastRequest,
) -> Result<NormalizeForecastReport, NormalizeForecastError> {
    let invalid = NormalizeForecastError::Configuration;
    let workspace = super::absolute(&request.invocation_dir, &request.workspace);
    let request_file = super::absolute(&request.invocation_dir, &request.request_file);
    if !workspace.is_dir() {
        return Err(invalid(format!(
            "workspace is not a directory: {}",
            workspace.display()
        )));
    }
    let bytes = std::fs::read_to_string(&request_file)
        .map_err(|e| invalid(format!("cannot read {}: {e}", request_file.display())))?;
    let config: Document = toml::from_str(&bytes).map_err(|e| invalid(e.message().into()))?;
    if config.schema_version != 1 {
        return Err(invalid("unsupported schema_version".into()));
    }
    for (name, path) in [
        ("source", &config.source),
        ("catalog", &config.catalog),
        ("artifact_root", &config.artifact_root),
    ] {
        if path.as_os_str().is_empty() || path.to_string_lossy().trim().is_empty() {
            return Err(invalid(format!("{name} must be a nonempty path")));
        }
    }
    let cycle = DateTime::parse_from_rfc3339(&config.cycle_utc)
        .map_err(|_| invalid("cycle_utc must be RFC3339 UTC".into()))?;
    if cycle.offset().local_minus_utc() != 0 || cycle.nanosecond() != 0 {
        return Err(invalid("cycle_utc must use UTC and whole seconds".into()));
    }
    let operation_timezone = config
        .operation_timezone
        .parse::<chrono_tz::Tz>()
        .map_err(|_| invalid("invalid operation_timezone".into()))?;
    let source = super::absolute(&workspace, &config.source);
    let normalized = normalize_saved_forecast(
        &source,
        &NormalizationRequest {
            provider: config.provider,
            model: config.model,
            product: config.product,
            cycle_utc: cycle.with_timezone(&Utc),
            operation_timezone,
            crop_bbox: config.crop_bbox,
        },
    )
    .map_err(NormalizeForecastError::Normalization)?;
    let mut catalog = ForecastCatalog::open(&workspace, &config.catalog, &config.artifact_root)
        .map_err(NormalizeForecastError::Persistence)?;
    let asset = catalog
        .register_native(&source, &normalized.source, &normalized.native)
        .map_err(NormalizeForecastError::Persistence)?;
    Ok(NormalizeForecastReport {
        source_id: asset.source_id,
        asset_id: asset.asset_id,
        retained_source: workspace.join(asset.retained_source),
        native_artifact: workspace.join(asset.native_artifact),
        native_shape: [
            normalized.native.timeline.bounds.len(),
            normalized.native.grid.latitudes.len(),
            normalized.native.grid.longitudes.len(),
        ],
        effective_bbox: normalized.native.grid.effective_bbox,
        timeline: normalized.native.timeline,
    })
}
