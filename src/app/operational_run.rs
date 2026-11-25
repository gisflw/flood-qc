use std::collections::HashSet;
use std::path::PathBuf;

use chrono::{DateTime, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};

use super::acquisition::AcquisitionDocument;
use super::operation::{lock_workspace, lock_workspace_job, run_preparation_locked};
use super::prepare::{TypedPreparationRequest, prepare_workspace};
use super::{
    AcquisitionReport, CalibrationDocument, ExecutionReport, ForcingDocument, HotStartDocument,
    OperationControl, OperationError, PreparationDocument, RoutingMode, ScenarioDocument,
};
use crate::store::{ForecastAssetChoice, ForecastCatalog, ForecastCorrection};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct RunParameters {
    pub reference_time: Option<String>,
    pub lookback_days: Option<u32>,
    pub forecast_days: Option<u32>,
    /// None selects zero and the latest usable raw cycle of each product.
    pub scenarios: Option<Vec<RunScenario>>,
    pub forcing: Option<ForcingDocument>,
    pub calibration: Option<CalibrationDocument>,
    pub geometry: Option<PathBuf>,
    pub hot_start: Option<HotStartDocument>,
    pub routing: Option<RoutingMode>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum RunScenario {
    Zero,
    Raw {
        forecast: String,
    },
    Corrected {
        forecast: String,
        corrections: Vec<ForecastAdjustment>,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ForecastAdjustment {
    pub t0_hour: i64,
    pub t1_hour: i64,
    #[serde(default)]
    pub shift_lat_pixels: f64,
    #[serde(default)]
    pub shift_lon_pixels: f64,
    #[serde(default)]
    pub rotation_deg: f64,
    #[serde(default = "unit_multiplier")]
    pub multiplication_factor: f64,
}

fn unit_multiplier() -> f64 {
    1.0
}

impl Default for ForecastAdjustment {
    fn default() -> Self {
        Self {
            t0_hour: 0,
            t1_hour: 0,
            shift_lat_pixels: 0.0,
            shift_lon_pixels: 0.0,
            rotation_deg: 0.0,
            multiplication_factor: unit_multiplier(),
        }
    }
}

impl std::str::FromStr for RunScenario {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::parse(value)
    }
}

impl RunScenario {
    pub fn parse(value: &str) -> Result<Self, String> {
        if value.starts_with('{') {
            let scenario: Self = serde_json::from_str(value).map_err(|error| error.to_string())?;
            scenario.validate()?;
            return Ok(scenario);
        }
        if value == "zero" {
            return Ok(Self::Zero);
        }
        let scenario = Self::Raw {
            forecast: value.into(),
        };
        scenario.validate()?;
        Ok(scenario)
    }

    fn validate(&self) -> Result<(), String> {
        match self {
            Self::Zero => Ok(()),
            Self::Raw { forecast } => Selector::parse(forecast).map(|_| ()),
            Self::Corrected {
                forecast,
                corrections,
            } => {
                let selector = Selector::parse(forecast)?;
                if selector.cycle.is_none() || corrections.is_empty() {
                    return Err(
                        "A corrected scenario requires an exact cycle and at least one correction"
                            .into(),
                    );
                }
                correction_values("validation", corrections).map(|_| ())
            }
        }
    }
}

struct Selector<'a> {
    provider: &'a str,
    product: Option<(&'a str, &'a str)>,
    cycle: Option<String>,
}

impl<'a> Selector<'a> {
    fn parse(value: &'a str) -> Result<Self, String> {
        let (product, cycle) = value
            .split_once('@')
            .map_or((value, None), |(name, cycle)| (name, Some(cycle)));
        let parts: Vec<_> = product.split('/').collect();
        if parts.iter().any(|part| {
            part.is_empty()
                || !part
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || b"_-.".contains(&byte))
        }) {
            return Err("Use provider[/model/product][@UTC-cycle]".into());
        }
        let (provider, product) = match parts.as_slice() {
            [provider] => (*provider, None),
            [provider, model, product] => (*provider, Some((*model, *product))),
            _ => return Err("Use provider[/model/product][@UTC-cycle]".into()),
        };
        let cycle = cycle
            .map(|cycle| {
                DateTime::parse_from_rfc3339(cycle)
                    .map(|time| time.to_utc().to_rfc3339_opts(SecondsFormat::Secs, true))
                    .map_err(|error| format!("Invalid forecast cycle: {error}"))
            })
            .transpose()?;
        Ok(Self {
            provider,
            product,
            cycle,
        })
    }

    fn matches(&self, asset: &ForecastAssetChoice) -> bool {
        asset.provider == self.provider
            && self
                .product
                .is_none_or(|(model, product)| asset.model == model && asset.product == product)
            && self
                .cycle
                .as_ref()
                .is_none_or(|cycle| *cycle == asset.cycle_utc)
    }
}

fn product_key(asset: &ForecastAssetChoice) -> (&str, &str, &str) {
    (&asset.provider, &asset.model, &asset.product)
}

fn forecast_label(asset: &ForecastAssetChoice) -> String {
    format!(
        "{}/{}/{}@{}",
        asset.provider, asset.model, asset.product, asset.cycle_utc
    )
}

fn correction_values(
    asset: &str,
    edits: &[ForecastAdjustment],
) -> Result<Vec<ForecastCorrection>, String> {
    edits
        .iter()
        .enumerate()
        .map(|(index, edit)| {
            let value = ForecastCorrection {
                correction_id: format!("{asset}:{}", index + 1),
                asset_id: asset.into(),
                t0_hour: edit.t0_hour,
                t1_hour: edit.t1_hour,
                shift_lat_pixels: edit.shift_lat_pixels,
                shift_lon_pixels: edit.shift_lon_pixels,
                rotation_deg: edit.rotation_deg,
                multiplication_factor: edit.multiplication_factor,
                editor: None,
                reason: String::new(),
                created_at: "1970-01-01T00:00:00Z".into(),
            };
            crate::store::check_correction(&value)
                .map_err(|error| format!("Correction {}: {error}", index + 1))?;
            Ok(value)
        })
        .collect()
}

pub struct ForecastPreview {
    pub raw: Vec<crate::store::GridFrame>,
    pub corrected: Vec<crate::store::GridFrame>,
}

#[derive(Debug, Serialize)]
pub struct OperationalRunReport {
    pub acquisition: Option<AcquisitionReport>,
    pub execution: Option<ExecutionReport>,
    pub error: Option<OperationError>,
}

/// Run parameters and workspace settings captured for a preview or an execution.
#[derive(Clone, Debug)]
pub struct OperationalRun {
    workspace: PathBuf,
    settings: super::WorkspaceConfig,
    parameters: RunParameters,
    reference: String,
    opened_at: DateTime<Utc>,
}

type Corrections = Vec<Option<Vec<ForecastCorrection>>>;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ResolvedScenario {
    pub forecast: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub corrections: Vec<ForecastAdjustment>,
}

pub(super) type ResolvedScenarios = Vec<ResolvedScenario>;

impl OperationalRun {
    pub fn open_workspace(workspace: PathBuf, now: DateTime<Utc>) -> Result<Self, String> {
        let workspace = workspace
            .canonicalize()
            .map_err(|error| error.to_string())?;
        let settings = super::read_workspace_config(&super::workspace_config_path(&workspace))?;
        let reference =
            super::workspace::reference(&settings.run.clone().unwrap_or_default(), None, now)?;
        Ok(Self {
            workspace,
            settings,
            parameters: RunParameters::default(),
            reference: reference.format("%Y-%m-%dT%H:%M:%S").to_string(),
            opened_at: now,
        })
    }

    pub fn parameters(&self) -> &RunParameters {
        &self.parameters
    }

    pub fn set_parameters(&mut self, parameters: RunParameters) {
        self.parameters = parameters;
    }

    fn effective_settings(&self) -> super::WorkspaceConfig {
        let mut settings = self.settings.clone();
        let run = settings.run.get_or_insert_default();
        if let Some(days) = self.parameters.lookback_days {
            run.lookback_days = days;
        }
        if let Some(days) = self.parameters.forecast_days {
            run.forecast_days = days;
        }
        settings
    }

    fn document(
        &self,
        scenarios: Vec<ScenarioDocument>,
        resolve_sources: bool,
    ) -> Result<PreparationDocument, String> {
        let settings = self.effective_settings();
        let reference = super::workspace::reference(
            settings.run.as_ref().unwrap(),
            Some(self.reference()),
            self.opened_at,
        )?;
        let mut document = super::workspace::operational_document(
            &self.workspace,
            &settings,
            reference,
            resolve_sources,
            scenarios,
        )?;
        document.forcing = self.parameters.forcing.clone();
        document.calibration = self.parameters.calibration.clone();
        document.hot_start = self.parameters.hot_start.clone();
        document.routing = self.parameters.routing;
        if let Some(geometry) = &self.parameters.geometry {
            document.geometry.mini_gtp = geometry.clone();
        }
        Ok(document)
    }

    pub fn reference(&self) -> &str {
        self.parameters
            .reference_time
            .as_deref()
            .unwrap_or(&self.reference)
    }

    pub fn set_reference(&mut self, reference: String) {
        self.parameters.reference_time = Some(reference);
    }

    pub fn time_zone(&self) -> &str {
        self.settings
            .run
            .as_ref()
            .and_then(|run| run.time_zone)
            .unwrap()
            .name()
    }

    pub fn recipe_path(&self) -> PathBuf {
        super::workspace_config_path(&self.workspace)
    }

    pub fn set_scenarios(&mut self, values: &[String]) -> Result<(), String> {
        self.parameters.scenarios = Some(
            values
                .iter()
                .map(|value| RunScenario::parse(value))
                .collect::<Result<Vec<_>, _>>()?,
        );
        Ok(())
    }

    pub fn coverage(&self) -> Result<[String; 4], String> {
        let document = self.document(Vec::new(), false)?;
        Ok([
            document.historical_first_end,
            document.historical_last_end,
            document.forecast_first_end,
            document.forecast_last_end,
        ])
    }

    fn catalog(&self) -> Result<ForecastCatalog, String> {
        ForecastCatalog::open_read_only(
            &self.workspace,
            &super::operation_database(&self.workspace),
        )
        .map_err(|error| error.to_string())
    }

    fn covering_assets(
        &self,
        catalog: &ForecastCatalog,
        document: &PreparationDocument,
    ) -> Result<Vec<ForecastAssetChoice>, String> {
        let time = |value: &str| {
            super::request::timestamp("reference_time", value).map_err(|error| error.to_string())
        };
        let reference = super::workspace::reference(
            self.settings.run.as_ref().unwrap(),
            Some(self.reference()),
            self.opened_at,
        )?;
        catalog
            .covering_assets(
                reference.to_utc(),
                time(&document.reference_time)?,
                time(&document.forecast_last_end)?,
                self.time_zone(),
            )
            .map_err(|error| error.to_string())
    }

    pub fn available_forecasts(&self) -> Result<Vec<String>, String> {
        if !super::operation_database(&self.workspace).exists() {
            return Ok(Vec::new());
        }
        let catalog = self.catalog()?;
        let document = self.document(Vec::new(), false)?;
        Ok(self
            .covering_assets(&catalog, &document)?
            .iter()
            .filter(|asset| catalog.check_native_available(&asset.asset_id).is_ok())
            .map(forecast_label)
            .collect())
    }

    pub fn default_scenarios(&self) -> Result<Vec<RunScenario>, String> {
        let mut chosen = Vec::new();
        let mut seen = HashSet::new();
        for label in self.available_forecasts()? {
            let (product, _) = label
                .split_once('@')
                .expect("forecast labels contain a cycle");
            if seen.insert(product.to_owned()) {
                chosen.push(RunScenario::Raw { forecast: label });
            }
        }
        Ok(std::iter::once(RunScenario::Zero).chain(chosen).collect())
    }

    pub fn scenarios(&self) -> Result<Vec<RunScenario>, String> {
        match &self.parameters.scenarios {
            Some(scenarios) => Ok(scenarios.clone()),
            None => self.default_scenarios(),
        }
    }

    fn find_asset<'a>(
        &self,
        forecast: &str,
        assets: &'a [ForecastAssetChoice],
        catalog: &ForecastCatalog,
    ) -> Result<&'a ForecastAssetChoice, String> {
        let selector = Selector::parse(forecast)?;
        let mut candidates = assets.iter().filter(|asset| {
            selector.matches(asset) && catalog.check_native_available(&asset.asset_id).is_ok()
        });
        let asset = candidates
            .next()
            .ok_or_else(|| format!("{forecast}: no usable cycle covers the operation window"))?;
        for other in candidates {
            if product_key(asset) != product_key(other) || asset.cycle_utc == other.cycle_utc {
                return Err(format!(
                    "{forecast}: ambiguous forecast; select a provider, model, product and cycle"
                ));
            }
        }
        Ok(asset)
    }

    fn resolve(&self) -> Result<(PreparationDocument, Corrections, ResolvedScenarios), String> {
        let scenarios = self.scenarios()?;
        if scenarios.is_empty() {
            return Err("Select at least one scenario".into());
        }
        let mut documents = Vec::with_capacity(scenarios.len());
        let mut corrections = Vec::with_capacity(scenarios.len());
        let mut seen = HashSet::new();
        let mut selections = HashSet::new();
        let mut resolved = Vec::with_capacity(scenarios.len());
        let document = self.document(Vec::new(), false)?;
        let catalog = scenarios
            .iter()
            .any(|scenario| *scenario != RunScenario::Zero)
            .then(|| self.catalog())
            .transpose()?;
        let assets = catalog
            .as_ref()
            .map(|catalog| self.covering_assets(catalog, &document))
            .transpose()?
            .unwrap_or_default();
        for scenario in &scenarios {
            scenario.validate()?;
            let (id, kind, asset_id, values, inputs) = match scenario {
                RunScenario::Zero => (
                    "zero".into(),
                    "zero",
                    None,
                    None,
                    ResolvedScenario {
                        forecast: None,
                        corrections: Vec::new(),
                    },
                ),
                RunScenario::Raw { forecast } | RunScenario::Corrected { forecast, .. } => {
                    let asset = self.find_asset(forecast, &assets, catalog.as_ref().unwrap())?;
                    let (kind, values) =
                        if let RunScenario::Corrected { corrections, .. } = scenario {
                            (
                                "corrected",
                                Some(correction_values(&asset.asset_id, corrections)?),
                            )
                        } else {
                            ("raw", None)
                        };
                    let cycle = DateTime::parse_from_rfc3339(&asset.cycle_utc)
                        .map_err(|error| error.to_string())?
                        .format("%Y%m%dT%H%MZ");
                    if !selections.insert((asset.asset_id.as_str(), kind)) {
                        return Err(format!(
                            "Duplicate {kind} scenario for {}",
                            forecast_label(asset)
                        ));
                    }
                    let product = format!("{}-{}-{}", asset.provider, asset.model, asset.product)
                        .replace('.', "-");
                    let id = if self.parameters.scenarios.is_none() || !forecast.contains('@') {
                        format!("{product}-{kind}")
                    } else {
                        format!("{product}-{cycle}-{kind}")
                    };
                    let edits = match scenario {
                        RunScenario::Corrected { corrections, .. } => corrections.clone(),
                        _ => Vec::new(),
                    };
                    (
                        id,
                        kind,
                        Some(asset.asset_id.clone()),
                        values,
                        ResolvedScenario {
                            forecast: Some(forecast_label(asset)),
                            corrections: edits,
                        },
                    )
                }
            };
            if !seen.insert(id.clone()) {
                return Err(format!("Duplicate scenario {id}"));
            }
            documents.push(ScenarioDocument {
                id,
                kind: kind.into(),
                asset_id,
                correction_ids: None,
            });
            corrections.push(values);
            resolved.push(inputs);
        }
        Ok((self.document(documents, true)?, corrections, resolved))
    }

    pub fn acquires_inputs(&self) -> bool {
        self.settings
            .acquisition
            .as_ref()
            .is_some_and(|acquisition| {
                !acquisition.observation_providers.is_empty()
                    || !acquisition.forecast_providers.is_empty()
            })
    }

    pub fn check_parameters(&self) -> Result<(), String> {
        self.coverage()?;
        if let Some(scenarios) = &self.parameters.scenarios {
            if scenarios.is_empty() {
                return Err("Select at least one scenario".into());
            }
            for scenario in scenarios {
                scenario.validate()?;
            }
        }
        Ok(())
    }

    pub fn validate(&self) -> Vec<String> {
        if let Err(error) = self.check_parameters() {
            return vec![error];
        }
        let result = if self.acquires_inputs() {
            AcquisitionDocument::from_workspace(&self.settings)
                .and_then(|_| self.document(Vec::new(), false).map(|_| ()))
        } else {
            self.resolve().and_then(|(document, _, _)| {
                super::request::validate_document(document, Vec::new())
                    .map(|_| ())
                    .map_err(|e| e.to_string())
            })
        };
        result.err().into_iter().collect()
    }

    pub fn preview_forecast(
        &self,
        scenario: &RunScenario,
    ) -> Result<super::ForecastPreview, String> {
        let RunScenario::Corrected {
            forecast,
            corrections,
        } = scenario
        else {
            return Err("Select a corrected forecast scenario".into());
        };
        scenario.validate()?;
        let catalog = self.catalog()?;
        let document = self.document(Vec::new(), false)?;
        let assets = self.covering_assets(&catalog, &document)?;
        let asset = self.find_asset(forecast, &assets, &catalog)?;
        let (_, native) = catalog
            .load_for_run(&asset.asset_id)
            .map_err(|error| error.to_string())?;
        let values = correction_values(&asset.asset_id, corrections)?;
        let time =
            |label| super::request::timestamp("forecast", label).map_err(|error| error.to_string());
        let timeline = super::request::axis(
            time(&document.forecast_first_end)?,
            time(&document.forecast_last_end)?,
            &document.time_zone,
            "forecast",
        )
        .map_err(|error| error.to_string())?;
        let mut used = crate::analysis::forecast_interval_selection(&native.timeline, &timeline)
            .map_err(|error| error.to_string())?;
        used.dedup();
        let (raw, _) = crate::analysis::corrected_forecast_frames(&native, &[], &used)
            .map_err(|error| error.to_string())?;
        let (corrected, _) = crate::analysis::corrected_forecast_frames(&native, &values, &used)
            .map_err(|error| error.to_string())?;
        Ok(ForecastPreview { raw, corrected })
    }

    pub fn prepare_saved(
        &self,
        invocation_dir: PathBuf,
        output: PathBuf,
        control: &OperationControl,
    ) -> Result<super::PreparationReport, OperationError> {
        let _lock = lock_workspace_job(&self.workspace)?;
        self.prepare_locked(invocation_dir, output, control)
    }

    pub(super) fn prepare_locked(
        &self,
        invocation_dir: PathBuf,
        output: PathBuf,
        control: &OperationControl,
    ) -> Result<super::PreparationReport, OperationError> {
        let (document, corrections, scenarios) = self.resolve().map_err(invalid)?;
        prepare_workspace(
            TypedPreparationRequest {
                workspace: self.workspace.clone(),
                invocation_dir,
                output,
                document,
                replacements: None,
                corrections,
            },
            scenarios,
            control,
        )
    }

    pub fn run_all_report(&self, control: &OperationControl) -> OperationalRunReport {
        let mut acquisition = None;
        match lock_workspace(&self.workspace)
            .and_then(|lock| self.run_locked(control, &lock, &mut acquisition))
        {
            Ok(execution) => OperationalRunReport {
                acquisition,
                execution: Some(execution),
                error: None,
            },
            Err(error) => OperationalRunReport {
                acquisition,
                execution: None,
                error: Some(error),
            },
        }
    }

    pub(super) fn run_locked(
        &self,
        control: &OperationControl,
        lock: &std::fs::File,
        report: &mut Option<AcquisitionReport>,
    ) -> Result<ExecutionReport, OperationError> {
        control.check()?;
        self.check_parameters().map_err(invalid)?;
        if self.acquires_inputs() {
            let acquisition =
                AcquisitionDocument::from_workspace(&self.settings).map_err(invalid)?;
            let reference = super::workspace::reference(
                self.settings.run.as_ref().unwrap(),
                Some(self.reference()),
                self.opened_at,
            )
            .map_err(invalid)?;
            super::workspace::initialize_database(&self.workspace, reference.timezone())
                .map_err(invalid)?;
            let acquired = report.insert(
                acquisition
                    .acquire_locked(&self.workspace, reference, None, control)
                    .map_err(invalid)?,
            );
            control.check()?;
            if let Some(message) = acquired.ana.as_ref().and_then(|ana| ana.error.as_ref()) {
                return Err(invalid(message.clone()));
            }
        }
        run_preparation_locked(
            control,
            &self.workspace,
            lock,
            |workspace, output, control| {
                self.prepare_locked(workspace.to_path_buf(), output, control)
            },
        )
    }
}

fn invalid(message: String) -> OperationError {
    OperationError::InvalidRequest {
        field: "run".into(),
        message,
    }
}
