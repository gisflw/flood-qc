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

#[derive(Debug)]
pub enum DownloadEvent {
    Started(NaiveDateTime),
    AnaWindow {
        station: String,
        start: chrono::NaiveDate,
        end: chrono::NaiveDate,
        rows: usize,
        loading: bool,
    },
    AnaFinished {
        station: String,
        dates: Option<(chrono::NaiveDate, chrono::NaiveDate)>,
        rows: usize,
        changed_samples: usize,
        error: Option<String>,
        cancelled: bool,
    },
    AnaFailed(String),
    ForecastCycle {
        provider: ForecastProvider,
        cycle: DateTime<Utc>,
    },
    ForecastLead {
        provider: ForecastProvider,
        cycle: DateTime<Utc>,
        lead: i64,
        index: usize,
        total: usize,
    },
    ForecastFinished(ForecastAcquisitionOutcome),
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
    inventory: Option<PathBuf>,
    request_days: u32,
    fetch_window_days: u32,
    timeout_seconds: u64,
    station_codes: Vec<String>,
    aggregation: AnaAggregation,
}

impl Default for AnaDocument {
    fn default() -> Self {
        Self {
            database: "operation.sqlite".into(),
            inventory: Some("source/history_station_inventory.csv".into()),
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
    "operation.sqlite".into()
}
fn default_artifact_root() -> PathBuf {
    "assets".into()
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
    pub(super) fn from_workspace(config: &super::WorkspaceConfig) -> Result<Self, String> {
        let run = config.run.clone().unwrap_or_default();
        let options = config.acquisition.clone().unwrap_or_default();
        if options
            .observation_providers
            .iter()
            .any(|provider| provider != "ana")
            || options.observation_providers.len() > 1
            || options.request_days == 0
            || !(1..=30).contains(&options.fetch_window_days)
            || !(1..=120).contains(&options.timeout_seconds)
            || options
                .forecast_providers
                .iter()
                .enumerate()
                .any(|(index, provider)| options.forecast_providers[..index].contains(provider))
            || options.lookback_cycles == 0
        {
            return Err(
                "Invalid acquisition providers, request window, timeout, or forecast lookback"
                    .into(),
            );
        }
        let ana = (!options.observation_providers.is_empty()).then(|| AnaDocument {
            database: super::workspace::DATABASE_PATH.into(),
            inventory: None,
            request_days: options.request_days,
            fetch_window_days: options.fetch_window_days,
            timeout_seconds: options.timeout_seconds,
            ..AnaDocument::default()
        });
        let forecasts = if options.forecast_providers.is_empty() {
            None
        } else {
            let document = ForecastDocument {
                catalog: super::workspace::DATABASE_PATH.into(),
                artifact_root: "assets".into(),
                providers: options.forecast_providers,
                lookback_cycles: options.lookback_cycles,
                buffer_fraction: options.buffer_fraction,
                model_bbox: config
                    .spatial
                    .as_ref()
                    .and_then(|s| s.bounds)
                    .ok_or("Set spatial.bounds")?,
            };
            document.buffered_bbox()?;
            Some(document)
        };
        Ok(Self {
            schema_version: 1,
            time_zone: run.time_zone.ok_or("Set run.time_zone")?,
            ana,
            forecasts,
        })
    }

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
            if let Some(inventory) = &ana.inventory {
                require_path(inventory)?;
            }
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
        control.download(|| DownloadEvent::Started(reference.naive_local()));
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
    let settings = (request.request_file == Path::new(super::workspace::CONFIG_PATH))
        .then(|| super::read_workspace_config(&super::workspace_config_path(&workspace)))
        .transpose()?;
    let document = match &settings {
        Some(config) => AcquisitionDocument::from_workspace(config)?,
        None => AcquisitionDocument::read(&workspace, &request.request_file)?,
    };
    if match kind {
        AcquisitionKind::Observations => document.ana.is_none(),
        AcquisitionKind::Forecasts => document.forecasts.is_none(),
    } {
        return Err("selected acquisition section is missing".into());
    }
    let now = Utc::now();
    let reference = match &settings {
        Some(config) => super::workspace::reference(
            &config.run.clone().unwrap_or_default(),
            request.reference_time.as_deref(),
            now,
        )?,
        None => document.reference(request.reference_time.as_deref(), now)?,
    };
    let _lock = lock_workspace_job(&workspace).map_err(|e| e.to_string())?;
    if settings.is_some() {
        super::workspace::initialize_database(&workspace, document.time_zone)?;
    }
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
    let settings = (request.request_file == Path::new(super::workspace::CONFIG_PATH))
        .then(|| super::read_workspace_config(&super::workspace_config_path(&workspace)))
        .transpose()?;
    let document = match &settings {
        Some(config) => AcquisitionDocument::from_workspace(config)?,
        None => AcquisitionDocument::read(&workspace, &request.request_file)?,
    };
    let now = Utc::now();
    let reference = match &settings {
        Some(config) => super::workspace::reference(
            &config.run.clone().unwrap_or_default(),
            request.reference_time.as_deref(),
            now,
        )?,
        None => document.reference(request.reference_time.as_deref(), now)?,
    };
    let ana = document
        .ana
        .as_ref()
        .ok_or("selected acquisition section is missing")?;
    let _lock = lock_workspace_job(&workspace).map_err(|e| e.to_string())?;
    if settings.is_some() {
        super::workspace::initialize_database(&workspace, document.time_zone)?;
    }
    let outcome = ana::replay(&workspace, ana, reference, paths, control)?;
    Ok(AcquisitionReport {
        reference_time: reference.naive_local(),
        ana: Some(outcome),
        forecasts: Vec::new(),
        cancelled: control.is_cancelled(),
    })
}

pub fn import_workspace_inventory(workspace: &Path, inventory: &Path) -> Result<usize, String> {
    let workspace = workspace.canonicalize().map_err(|e| e.to_string())?;
    let settings = super::read_workspace_config(&super::workspace_config_path(&workspace))?;
    let zone = settings
        .run
        .and_then(|run| run.time_zone)
        .ok_or("Set run.time_zone")?;
    let input = fs::File::open(inventory).map_err(|e| e.to_string())?;
    let stations = crate::adapters::read_ana_inventory(input)?;
    let _lock = lock_workspace_job(&workspace).map_err(|e| e.to_string())?;
    super::workspace::initialize_database(&workspace, zone)?;
    let mut db =
        crate::store::ObservationStore::open_writable(&super::operation_database(&workspace))
            .map_err(|e| e.to_string())?;
    if db.time_zone().map_err(|e| e.to_string())? != zone.name() {
        return Err("observation database timezone differs from run.time_zone".into());
    }
    let existing = db.station_locations().map_err(|e| e.to_string())?;
    let mut entries = Vec::new();
    let mut variables = Vec::new();
    for mut entry in stations {
        if let Some(prior) = existing.iter().find(|item| {
            item.station.provider_code == "ana"
                && crate::adapters::ana_station_code(&item.station.station_code)
                    .is_ok_and(|code| code == entry.station.station_code)
        }) {
            entry
                .station
                .station_id
                .clone_from(&prior.station.station_id);
            entry
                .station
                .station_code
                .clone_from(&prior.station.station_code);
        }
        variables.push((
            entry.station.station_id.clone(),
            entry
                .variables
                .iter()
                .map(|value| value.code().to_owned())
                .collect(),
        ));
        entries.push(entry.station);
    }
    db.import_ana_inventory(&entries, &variables)
        .map_err(|e| e.to_string())?;
    Ok(entries.len())
}

pub fn export_workspace_inventory(workspace: &Path, output: &Path) -> Result<usize, String> {
    let db = crate::store::ObservationStore::open_read_only(&super::operation_database(workspace))
        .map_err(|error| error.to_string())?;
    let mut writer = csv::Writer::from_writer(Vec::new());
    writer
        .write_record([
            "provider_code",
            "station_code",
            "station_name",
            "latitude",
            "longitude",
            "observed_variables",
        ])
        .map_err(|error| error.to_string())?;
    let mut count = 0;
    for entry in db.station_locations().map_err(|error| error.to_string())? {
        let station = entry.station;
        if station.provider_code != "ana" {
            continue;
        }
        let variables = db
            .observed_variables(&station.station_id)
            .map_err(|error| error.to_string())?;
        let variables = if variables.is_empty() {
            "none".into()
        } else {
            variables.join(",")
        };
        writer
            .serialize((
                station.provider_code,
                station.station_code,
                station.name,
                station.latitude,
                station.longitude,
                variables,
            ))
            .map_err(|error| error.to_string())?;
        count += 1;
    }
    let data = writer.into_inner().map_err(|error| error.to_string())?;
    crate::store::write_new_atomic(output, &data)?;
    Ok(count)
}
