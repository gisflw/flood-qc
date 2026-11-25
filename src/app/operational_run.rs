use std::collections::HashSet;
use std::path::PathBuf;

use chrono::{DateTime, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};

use super::acquisition::AcquisitionDocument;
use super::operation::{lock_workspace, lock_workspace_job, run_captured_locked};
use super::prepare::{TypedPreparationRequest, prepare_workspace};
use super::{
    AcquisitionReport, CalibrationDocument, ExecutionReport, ForcingDocument, HotStartDocument,
    OperationControl, OperationError, PreparationDocument, RoutingMode, ScenarioDocument,
};
use crate::store::{ForecastAssetChoice, ForecastCatalog};

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
#[serde(deny_unknown_fields)]
pub struct RunScenario {
    pub id: String,
    pub base: ScenarioBase,
    #[serde(default)]
    pub edits: Vec<crate::analysis::ScenarioGridEdit>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ScenarioBase {
    Zero,
    Forecast { forecast: String },
    Scenario { id: String },
}

pub type ForecastAdjustment = crate::analysis::ScenarioGridEdit;

impl std::str::FromStr for RunScenario {
    type Err = String;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::parse(value)
    }
}
impl RunScenario {
    pub fn parse(value: &str) -> Result<Self, String> {
        let scenario = if value.starts_with('{') {
            serde_json::from_str(value).map_err(|error: serde_json::Error| error.to_string())?
        } else if value == "zero" {
            Self {
                id: "zero".into(),
                base: ScenarioBase::Zero,
                edits: vec![],
            }
        } else {
            let id: String = value
                .chars()
                .map(|c| {
                    if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                        c
                    } else {
                        '-'
                    }
                })
                .take(64)
                .collect();
            Self {
                id,
                base: ScenarioBase::Forecast {
                    forecast: value.into(),
                },
                edits: vec![],
            }
        };
        scenario.validate()?;
        Ok(scenario)
    }
    fn validate(&self) -> Result<(), String> {
        let id = self.id.as_bytes();
        if id.is_empty()
            || id.len() > 64
            || !id[0].is_ascii_alphanumeric()
            || id
                .iter()
                .any(|c| !c.is_ascii_alphanumeric() && *c != b'_' && *c != b'-')
        {
            return Err("Scenario IDs must match [A-Za-z0-9][A-Za-z0-9_-]{0,63}".into());
        }
        match &self.base {
            ScenarioBase::Forecast { forecast } => {
                Selector::parse(forecast)?;
            }
            ScenarioBase::Scenario { id } if id.is_empty() => {
                return Err("Empty base scenario ID".into());
            }
            _ => {}
        }
        for edit in &self.edits {
            edit.validate()?;
        }
        Ok(())
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

#[derive(Clone, Debug)]
pub struct OperationalRun {
    workspace: PathBuf,
    config_path: PathBuf,
    settings: super::WorkspaceConfig,
    parameters: RunParameters,
    reference: String,
    opened_at: DateTime<Utc>,
    preview_cache: std::sync::Arc<
        std::sync::Mutex<
            std::collections::HashMap<String, std::sync::Arc<crate::store::GridSeries>>,
        >,
    >,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ResolvedScenario {
    pub forecast: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base: Option<String>,
    pub edits: Vec<ForecastAdjustment>,
}

pub(super) type ResolvedScenarios = Vec<ResolvedScenario>;

impl OperationalRun {
    pub fn open_workspace(workspace: PathBuf, now: DateTime<Utc>) -> Result<Self, String> {
        let config_path = super::workspace_config_path(&workspace);
        let settings = super::read_workspace_config(&config_path)?;
        Self::from_config(workspace, settings, config_path, now)
    }

    pub fn from_config(
        workspace: PathBuf,
        settings: super::WorkspaceConfig,
        config_path: PathBuf,
        now: DateTime<Utc>,
    ) -> Result<Self, String> {
        let workspace = workspace
            .canonicalize()
            .map_err(|error| error.to_string())?;
        let reference =
            super::workspace::reference(&settings.run.clone().unwrap_or_default(), None, now)?;
        Ok(Self {
            workspace,
            config_path,
            settings,
            parameters: RunParameters::default(),
            reference: reference.format("%Y-%m-%dT%H:%M:%S").to_string(),
            opened_at: now,
            preview_cache: Default::default(),
        })
    }

    pub(super) fn workspace(&self) -> &std::path::Path {
        &self.workspace
    }

    pub fn parameters(&self) -> &RunParameters {
        &self.parameters
    }

    pub fn set_parameters(&mut self, parameters: RunParameters) {
        self.parameters = parameters;
    }

    pub(super) fn at_admission(&self, now: DateTime<Utc>) -> Result<Self, String> {
        let mut operation = self.clone();
        if operation.parameters.reference_time.is_none() {
            operation.reference = super::workspace::reference(
                &operation.settings.run.clone().unwrap_or_default(),
                None,
                now,
            )?
            .format("%Y-%m-%dT%H:%M:%S")
            .to_string();
        }
        operation.opened_at = now;
        Ok(operation)
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
        document.forcing = self
            .parameters
            .forcing
            .clone()
            .or_else(|| self.settings.forcing.clone());
        document.calibration = self
            .parameters
            .calibration
            .clone()
            .or_else(|| self.settings.calibration.clone());
        document.hot_start = self
            .parameters
            .hot_start
            .clone()
            .or_else(|| self.settings.hot_start.clone());
        document.routing = self.parameters.routing.or(self.settings.routing);
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
        self.config_path.clone()
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
        ForecastCatalog::open_read_only(&self.workspace, &super::forecast_assets(&self.workspace))
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
        if !super::forecast_assets(&self.workspace).exists() {
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
        let mut chosen = vec![RunScenario::parse("zero")?];
        let mut seen = HashSet::new();
        for label in self.available_forecasts()? {
            let (product, _) = label
                .split_once('@')
                .expect("forecast labels contain a cycle");
            if seen.insert(product.to_owned()) {
                let mut scenario = RunScenario::parse(&label)?;
                scenario.id = product
                    .chars()
                    .map(|c| {
                        if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                            c
                        } else {
                            '-'
                        }
                    })
                    .take(64)
                    .collect();
                chosen.push(scenario);
            }
        }
        Ok(chosen)
    }

    pub fn scenarios(&self) -> Result<Vec<RunScenario>, String> {
        match &self.parameters.scenarios {
            Some(scenarios) => Ok(scenarios.clone()),
            None => match &self.settings.scenarios {
                Some(scenarios) => Ok(scenarios.clone()),
                None => self.default_scenarios(),
            },
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

    fn resolve(&self) -> Result<(PreparationDocument, ResolvedScenarios), String> {
        let scenarios = self.scenarios()?;
        if scenarios.is_empty() {
            return Err("Select at least one scenario".into());
        }
        let document = self.document(Vec::new(), false)?;
        let catalog = scenarios
            .iter()
            .any(|s| matches!(s.base, ScenarioBase::Forecast { .. }))
            .then(|| self.catalog())
            .transpose()?;
        let assets = catalog
            .as_ref()
            .map(|c| self.covering_assets(c, &document))
            .transpose()?
            .unwrap_or_default();
        let mut documents: Vec<ScenarioDocument> = Vec::new();
        let mut resolved: Vec<ResolvedScenario> = Vec::new();
        let mut seen = HashSet::new();
        for scenario in &scenarios {
            scenario.validate()?;
            if !seen.insert(scenario.id.clone()) {
                return Err(format!("Duplicate scenario {}", scenario.id));
            }
            let (asset_id, forecast, base) = match &scenario.base {
                ScenarioBase::Zero => (None, None, None),
                ScenarioBase::Forecast { forecast } => {
                    let asset = self.find_asset(forecast, &assets, catalog.as_ref().unwrap())?;
                    (
                        Some(asset.asset_id.clone()),
                        Some(forecast_label(asset)),
                        None,
                    )
                }
                ScenarioBase::Scenario { id } => {
                    let index = documents
                        .iter()
                        .position(|item| &item.id == id)
                        .ok_or_else(|| format!("Base scenario {id} must appear earlier"))?;
                    (None, resolved[index].forecast.clone(), Some(id.clone()))
                }
            };
            documents.push(ScenarioDocument {
                id: scenario.id.clone(),
                asset_id,
                base: base.clone(),
                edits: scenario.edits.clone(),
            });
            resolved.push(ResolvedScenario {
                forecast,
                base,
                edits: scenario.edits.clone(),
            });
        }
        Ok((self.document(documents, true)?, resolved))
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
            self.resolve().and_then(|(document, _)| {
                super::request::validate_document(document, Vec::new())
                    .map(|_| ())
                    .map_err(|e| e.to_string())
            })
        };
        result.err().into_iter().collect()
    }

    fn cached_preview_grid(
        &self,
        key: String,
        build: impl FnOnce() -> Result<crate::store::GridSeries, String>,
    ) -> Result<std::sync::Arc<crate::store::GridSeries>, String> {
        if let Some(grid) = self
            .preview_cache
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(&key)
            .cloned()
        {
            return Ok(grid);
        }
        let grid = std::sync::Arc::new(build()?);
        let mut cache = self.preview_cache.lock().unwrap_or_else(|e| e.into_inner());
        if cache.len() >= 8 {
            cache.clear();
        }
        cache.insert(key, grid.clone());
        Ok(grid)
    }

    pub fn preview_forecast(
        &self,
        scenario: &RunScenario,
    ) -> Result<super::ForecastPreview, String> {
        let mut draft = self.clone();
        let mut scenarios = draft.scenarios()?;
        if let Some(existing) = scenarios.iter_mut().find(|item| item.id == scenario.id) {
            *existing = scenario.clone();
        } else {
            scenarios.push(scenario.clone());
        }
        draft.parameters.scenarios = Some(scenarios);
        let (document, resolved) = draft.resolve()?;
        let time =
            |label| super::request::timestamp("forecast", label).map_err(|error| error.to_string());
        let timeline = super::request::axis(
            time(&document.forecast_first_end)?,
            time(&document.forecast_last_end)?,
            &document.time_zone,
            "forecast",
        )
        .map_err(|error| error.to_string())?;
        let grid = crate::store::RegularGridSpec::new(
            document.grid.bbox,
            document.grid.resolution_degrees,
        )
        .map_err(|e| e.to_string())?;
        let catalog = if document.scenarios.iter().any(|s| s.asset_id.is_some()) {
            Some(draft.catalog()?)
        } else {
            None
        };
        let mut previous: std::collections::HashMap<
            String,
            (String, std::sync::Arc<crate::store::GridSeries>),
        > = std::collections::HashMap::new();
        let grid_key = serde_json::to_string(&(&timeline, &grid)).map_err(|e| e.to_string())?;
        for (doc, input) in document.scenarios.iter().zip(&resolved) {
            let (base_key, raw) = if let Some(base) = &input.base {
                previous
                    .get(base)
                    .cloned()
                    .ok_or("Base scenario must precede variant")?
            } else if let Some(asset_id) = &doc.asset_id {
                let catalog = catalog.as_ref().unwrap();
                let identity = catalog
                    .native_identity(asset_id)
                    .map_err(|e| e.to_string())?;
                let key = serde_json::to_string(&(asset_id, identity, &grid_key))
                    .map_err(|e| e.to_string())?;
                let raw = self.cached_preview_grid(key.clone(), || {
                    let (_, native) = catalog.load_for_run(asset_id).map_err(|e| e.to_string())?;
                    crate::analysis::forecast_working_grid(&native, &timeline, &grid)
                        .map_err(|e| e.to_string())
                })?;
                (key, raw)
            } else {
                let key = format!("zero:{grid_key}");
                let raw = self.cached_preview_grid(key.clone(), || {
                    let size = timeline.bounds.len() * grid.latitudes.len() * grid.longitudes.len();
                    Ok(crate::store::GridSeries {
                        timeline: timeline.clone(),
                        grid: grid.clone(),
                        values: vec![0.; size],
                        valid: vec![true; size],
                        provenance: crate::store::Provenance {
                            source_ids: vec![],
                            processing: serde_json::json!({"kind":"zero"}),
                        },
                    })
                })?;
                (key, raw)
            };
            let mut key = base_key;
            let mut corrected = raw.clone();
            for edit in &input.edits {
                key = serde_json::to_string(&(&key, edit)).map_err(|e| e.to_string())?;
                corrected = self.cached_preview_grid(key.clone(), || {
                    crate::analysis::apply_scenario_grid_edits(
                        &corrected,
                        std::slice::from_ref(edit),
                    )
                    .map_err(|e| e.to_string())
                })?;
            }
            if doc.id == scenario.id {
                let frames = |series: &crate::store::GridSeries| {
                    let cells = series.grid.latitudes.len() * series.grid.longitudes.len();
                    series
                        .timeline
                        .bounds
                        .iter()
                        .enumerate()
                        .map(|(i, interval)| crate::store::GridFrame {
                            interval: interval.clone(),
                            time_zone: series.timeline.time_zone.clone(),
                            grid: series.grid.clone(),
                            values: series.values[i * cells..(i + 1) * cells].to_vec(),
                            valid: series.valid[i * cells..(i + 1) * cells].to_vec(),
                            provenance: series.provenance.clone(),
                        })
                        .collect()
                };
                return Ok(ForecastPreview {
                    raw: frames(&raw),
                    corrected: frames(&corrected),
                });
            }
            previous.insert(doc.id.clone(), (key, corrected));
        }
        Err("Scenario not found".into())
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
        let (document, scenarios) = self.resolve().map_err(invalid)?;
        prepare_workspace(
            TypedPreparationRequest {
                workspace: self.workspace.clone(),
                invocation_dir,
                output,
                document,
                replacements: None,
            },
            scenarios,
            control,
        )
    }

    pub(super) fn capture_locked(
        &self,
        invocation_dir: PathBuf,
        output: PathBuf,
        control: &OperationControl,
    ) -> Result<super::PreparationReport, OperationError> {
        let (document, scenarios) = self.resolve().map_err(invalid)?;
        super::prepare::capture_workspace(
            TypedPreparationRequest {
                workspace: self.workspace.clone(),
                invocation_dir,
                output,
                document,
                replacements: None,
            },
            scenarios,
            control,
        )
    }

    pub fn run_all_report(&self, control: &OperationControl) -> OperationalRunReport {
        let mut acquisition = None;
        match self
            .at_admission(Utc::now())
            .map_err(invalid)
            .and_then(|operation| {
                lock_workspace(&operation.workspace)
                    .and_then(|lock| operation.run_locked(control, &lock, &mut acquisition))
            }) {
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

    pub(super) fn acquire_locked(
        &self,
        control: &OperationControl,
    ) -> Result<Option<AcquisitionReport>, OperationError> {
        control.check()?;
        self.check_parameters().map_err(invalid)?;
        if !self.acquires_inputs() {
            return Ok(None);
        }
        let acquisition = AcquisitionDocument::from_workspace(&self.settings).map_err(invalid)?;
        let reference = super::workspace::reference(
            self.settings.run.as_ref().unwrap(),
            Some(self.reference()),
            self.opened_at,
        )
        .map_err(invalid)?;
        super::workspace::initialize_assets(&self.workspace, reference.timezone())
            .map_err(invalid)?;
        let acquired = acquisition
            .acquire_locked(&self.workspace, reference, None, control)
            .map_err(invalid)?;
        control.check()?;
        if let Some(message) = acquired.ana.as_ref().and_then(|ana| ana.error.as_ref()) {
            return Err(invalid(message.clone()));
        }
        Ok(Some(acquired))
    }

    pub(super) fn run_locked(
        &self,
        control: &OperationControl,
        lock: &std::fs::File,
        report: &mut Option<AcquisitionReport>,
    ) -> Result<ExecutionReport, OperationError> {
        super::input_sets::resolve_input_replacement(&self.workspace, control)?;
        *report = self.acquire_locked(control)?;
        let engine = self
            .document(Vec::new(), false)
            .map_err(invalid)?
            .engine
            .executable;
        let engine = super::absolute(&self.workspace, &engine);
        let _ = lock;
        run_captured_locked(
            control,
            &self.workspace,
            &engine,
            |workspace, output, control| self.capture_locked(workspace.into(), output, control),
        )
    }
}

fn invalid(message: String) -> OperationError {
    OperationError::InvalidRequest {
        field: "run".into(),
        message,
    }
}
