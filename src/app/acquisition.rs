mod ana;
mod forecast;

use std::fs;
use std::path::{Path, PathBuf};

use chrono::{DateTime, NaiveDateTime, TimeZone, Timelike, Utc};
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};

use crate::adapters::{AnaAggregation, ProviderClient};
use crate::store::ForecastProvider;

use super::OperationControl;
use super::operation::lock_workspace_job;

#[derive(Clone, Copy, Debug)]
pub enum AcquisitionKind {
    Observations,
    Forecasts,
}

#[derive(Clone, Debug)]
pub struct AcquisitionRequest {
    pub workspace: PathBuf,
    pub request_file: PathBuf,
    pub reference_time: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct AcquisitionReport {
    pub reference_time: NaiveDateTime,
    pub ana: Option<AnaAcquisitionReport>,
    pub forecasts: Vec<ForecastAcquisitionOutcome>,
    pub cancelled: bool,
}

impl AcquisitionReport {
    pub fn is_complete(&self) -> bool {
        !self.cancelled
            && self.ana.as_ref().is_none_or(|ana| {
                ana.error.is_none() && ana.stations.iter().all(|station| station.error.is_none())
            })
            && self
                .forecasts
                .iter()
                .all(|forecast| forecast.error.is_none())
    }

    pub(super) fn run_blocker(&self) -> Option<&str> {
        self.ana
            .as_ref()
            .and_then(|ana| ana.error.as_deref())
            .or_else(|| {
                self.forecasts
                    .iter()
                    .find_map(|forecast| forecast.error.as_deref())
            })
    }
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct AnaAcquisitionReport {
    pub stations: Vec<AnaStationOutcome>,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct AnaStationOutcome {
    pub station_code: String,
    pub requested_days: usize,
    pub skipped_days: usize,
    pub raw_xml: Vec<PathBuf>,
    pub csv: Option<PathBuf>,
    pub normalized_rows: usize,
    pub changed_samples: usize,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ForecastAcquisitionOutcome {
    pub provider: ForecastProvider,
    pub attempted_cycles: Vec<DateTime<Utc>>,
    pub cycle_utc: Option<DateTime<Utc>>,
    pub reused: bool,
    pub asset_id: Option<String>,
    pub retained_source: Option<PathBuf>,
    pub native_artifact: Option<PathBuf>,
    pub valid_from: Option<NaiveDateTime>,
    pub valid_to: Option<NaiveDateTime>,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct AcquisitionDocument {
    schema_version: u32,
    pub(super) time_zone: Tz,
    pub(super) ana: Option<AnaDocument>,
    pub(super) forecasts: Option<ForecastDocument>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(super) struct AnaDocument {
    pub(super) database: PathBuf,
    inventory: PathBuf,
    request_days: u32,
    fetch_window_days: u32,
    timeout_seconds: u64,
    station_codes: Vec<String>,
    aggregation: AnaAggregation,
}

impl Default for AnaDocument {
    fn default() -> Self {
        Self {
            database: "data/observations.sqlite".into(),
            inventory: "data/source/history_station_inventory.csv".into(),
            request_days: 90,
            fetch_window_days: 30,
            timeout_seconds: 15,
            station_codes: Vec::new(),
            aggregation: AnaAggregation::default(),
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ForecastDocument {
    #[serde(default = "default_catalog")]
    pub(super) catalog: PathBuf,
    #[serde(default = "default_artifact_root")]
    artifact_root: PathBuf,
    #[serde(default = "default_providers")]
    pub(super) providers: Vec<ForecastProvider>,
    pub(super) model_bbox: [f64; 4],
    #[serde(default = "default_lookback")]
    lookback_cycles: u32,
    #[serde(default = "default_buffer")]
    buffer_fraction: f64,
}

fn default_catalog() -> PathBuf {
    "data/forecasts.sqlite".into()
}
fn default_artifact_root() -> PathBuf {
    "data/forecasts".into()
}
fn default_providers() -> Vec<ForecastProvider> {
    vec![ForecastProvider::Ecmwf, ForecastProvider::Noaa]
}
fn default_lookback() -> u32 {
    12
}
fn default_buffer() -> f64 {
    2.0
}

impl ForecastDocument {
    fn buffered_bbox(&self) -> Result<[f64; 4], String> {
        let [west, south, east, north] = self.model_bbox;
        if !self.model_bbox.iter().all(|value| value.is_finite())
            || west >= east
            || south >= north
            || !self.buffer_fraction.is_finite()
            || self.buffer_fraction < 0.0
        {
            return Err("invalid model bounds or forecast buffer".into());
        }
        let x = (east - west) * self.buffer_fraction;
        let y = (north - south) * self.buffer_fraction;
        let bbox = [west - x, south - y, east + x, north + y];
        if bbox[0] < -180.0 || bbox[1] < -90.0 || bbox[2] > 180.0 || bbox[3] > 90.0 {
            return Err("buffered forecast bounds exceed geographic limits".into());
        }
        Ok(bbox)
    }
}

impl AcquisitionDocument {
    pub(super) fn read(workspace: &Path, request: &Path) -> Result<Self, String> {
        let path = workspace.join(request);
        let content = fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let document: Self = toml::from_str(&content).map_err(|e| e.message().to_owned())?;
        if document.schema_version != 1 || (document.ana.is_none() && document.forecasts.is_none())
        {
            return Err(
                "acquisition requires schema_version = 1 and at least one provider section".into(),
            );
        }
        if let Some(ana) = &document.ana {
            if ana.request_days == 0
                || !(1..=30).contains(&ana.fetch_window_days)
                || !(1..=120).contains(&ana.timeout_seconds)
            {
                return Err("ANA requires positive request_days, fetch_window_days of 1–30, and timeout_seconds of 1–120".into());
            }
            require_path(&ana.database)?;
            require_path(&ana.inventory)?;
        }
        if let Some(forecasts) = &document.forecasts {
            if forecasts.lookback_cycles == 0
                || forecasts.providers.is_empty()
                || forecasts
                    .providers
                    .iter()
                    .enumerate()
                    .any(|(index, provider)| forecasts.providers[..index].contains(provider))
            {
                return Err("forecasts require a positive lookback and unique providers".into());
            }
            require_path(&forecasts.catalog)?;
            require_path(&forecasts.artifact_root)?;
            forecasts.buffered_bbox()?;
        }
        Ok(document)
    }

    pub(super) fn reference(
        &self,
        reference: Option<&str>,
        now: DateTime<Utc>,
    ) -> Result<DateTime<Tz>, String> {
        let local = match reference {
            Some(value) => {
                super::request::timestamp("reference_time", value).map_err(|e| e.to_string())?
            }
            None => now
                .with_timezone(&self.time_zone)
                .naive_local()
                .with_minute(0)
                .unwrap()
                .with_second(0)
                .unwrap()
                .with_nanosecond(0)
                .unwrap(),
        };
        self.time_zone
            .from_local_datetime(&local)
            .single()
            .ok_or_else(|| format!("ambiguous or nonexistent reference time {local}"))
    }

    pub(super) fn acquire_locked(
        &self,
        workspace: &Path,
        reference: DateTime<Tz>,
        kind: Option<AcquisitionKind>,
        control: &OperationControl,
    ) -> Result<AcquisitionReport, String> {
        let client = ProviderClient::new().map_err(|e| e.to_string())?;
        let mut report = AcquisitionReport {
            reference_time: reference.naive_local(),
            ana: None,
            forecasts: Vec::new(),
            cancelled: false,
        };
        if !matches!(kind, Some(AcquisitionKind::Forecasts))
            && let Some(ana) = &self.ana
        {
            report.ana = Some(ana::acquire(workspace, ana, reference, &client, control));
        }
        if !control.is_cancelled()
            && !matches!(kind, Some(AcquisitionKind::Observations))
            && let Some(forecasts) = &self.forecasts
        {
            report.forecasts = forecast::acquire(workspace, forecasts, reference, &client, control);
        }
        report.cancelled = control.is_cancelled();
        Ok(report)
    }
}

fn require_path(path: &Path) -> Result<(), String> {
    if path.to_string_lossy().trim().is_empty() {
        Err("acquisition paths must be nonempty".into())
    } else {
        Ok(())
    }
}

pub fn acquire_inputs(
    request: &AcquisitionRequest,
    kind: AcquisitionKind,
    control: &OperationControl,
) -> Result<AcquisitionReport, String> {
    let workspace = request
        .workspace
        .canonicalize()
        .map_err(|e| e.to_string())?;
    let document = AcquisitionDocument::read(&workspace, &request.request_file)?;
    if match kind {
        AcquisitionKind::Observations => document.ana.is_none(),
        AcquisitionKind::Forecasts => document.forecasts.is_none(),
    } {
        return Err("selected acquisition section is missing".into());
    }
    let reference = document.reference(request.reference_time.as_deref(), Utc::now())?;
    let _lock = lock_workspace_job(&workspace).map_err(|e| e.to_string())?;
    document.acquire_locked(&workspace, reference, Some(kind), control)
}

pub fn import_ana_csv(
    request: &AcquisitionRequest,
    paths: &[PathBuf],
    control: &OperationControl,
) -> Result<AcquisitionReport, String> {
    let workspace = request
        .workspace
        .canonicalize()
        .map_err(|e| e.to_string())?;
    let document = AcquisitionDocument::read(&workspace, &request.request_file)?;
    let reference = document.reference(request.reference_time.as_deref(), Utc::now())?;
    let ana = document
        .ana
        .as_ref()
        .ok_or("selected acquisition section is missing")?;
    let _lock = lock_workspace_job(&workspace).map_err(|e| e.to_string())?;
    let outcome = ana::replay(&workspace, ana, reference, paths, control)?;
    Ok(AcquisitionReport {
        reference_time: reference.naive_local(),
        ana: Some(outcome),
        forecasts: Vec::new(),
        cancelled: control.is_cancelled(),
    })
}
