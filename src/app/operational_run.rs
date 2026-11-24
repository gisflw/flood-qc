use super::acquisition::AcquisitionDocument;
use super::operation::{lock_workspace, run_preparation_locked};
use super::request::{coverage, decode_request_value, request_value, timestamp};
use super::{
    AcquisitionReport, Draft, DraftSources, ExecutionReport, OperationControl, OperationError,
    PreparationDocument,
};
use crate::store::ForecastProvider;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct AcquisitionRecipe {
    request: PathBuf,
}

#[derive(Debug, Serialize)]
pub struct OperationalRunReport {
    pub acquisition: Option<AcquisitionReport>,
    pub execution: Option<ExecutionReport>,
    pub error: Option<OperationError>,
}

/// A saved operation recipe and explicitly accepted sources for the current reference.
#[derive(Clone, Debug)]
pub struct OperationalRun {
    workspace: PathBuf,
    recipe_path: PathBuf,
    recipe: serde_json::Value,
    time_zone: String,
    scenarios: Vec<String>,
    reference: String,
    accepted: Option<DraftSources>,
    acquisition: Option<AcquisitionRecipe>,
    forecast_selectors: Vec<(usize, ForecastProvider)>,
}
impl OperationalRun {
    /// Load the selected TOML once. Resource paths in the recipe are workspace-relative.
    pub fn open(
        workspace: PathBuf,
        recipe_path: &Path,
        now: DateTime<Utc>,
    ) -> Result<Self, String> {
        let recipe_path = super::absolute(&workspace, recipe_path);
        let bytes = std::fs::read(&recipe_path).map_err(|e| e.to_string())?;
        let mut recipe = request_value(&bytes).map_err(|e| e.to_string())?;
        let root = recipe
            .as_object_mut()
            .ok_or("expected an operation recipe table")?;
        let acquisition: Option<AcquisitionRecipe> = root
            .remove("acquisition")
            .map(serde_json::from_value)
            .transpose()
            .map_err(|e| e.to_string())?;
        let mut forecast_selectors = Vec::new();
        if let Some(scenarios) = root
            .get_mut("scenarios")
            .and_then(serde_json::Value::as_array_mut)
        {
            for (index, scenario) in scenarios.iter_mut().enumerate() {
                if let Some(provider) = scenario
                    .as_object_mut()
                    .and_then(|table| table.remove("forecast_provider"))
                {
                    if acquisition.is_none()
                        || scenario["kind"] != "raw"
                        || scenario.get("asset_id").is_some()
                    {
                        return Err("forecast_provider requires acquisition and a raw scenario with no asset_id".into());
                    }
                    forecast_selectors.push((
                        index,
                        serde_json::from_value(provider).map_err(|e| e.to_string())?,
                    ));
                }
            }
        }
        let document = decode_request_value(recipe.clone()).map_err(|e| e.to_string())?;
        if document.schema_version != 1 || document.step_seconds != 3600 {
            return Err("Expected a version 1 or 2 hourly operation recipe".into());
        }
        let time_zone: chrono_tz::Tz = document
            .time_zone
            .parse()
            .map_err(|_| "Expected an IANA timezone in the operation recipe")?;
        let reference = now
            .with_timezone(&time_zone)
            .format("%Y-%m-%dT%H:00:00")
            .to_string();
        let scenarios = document.scenarios.into_iter().map(|item| item.id).collect();
        Ok(Self {
            workspace,
            recipe_path,
            recipe,
            time_zone: document.time_zone,
            scenarios,
            reference,
            accepted: None,
            acquisition,
            forecast_selectors,
        })
    }

    pub fn recipe_path(&self) -> &Path {
        &self.recipe_path
    }

    pub fn time_zone(&self) -> &str {
        // Preserve the supplied IANA identifier, including aliases used by canonical files.
        &self.time_zone
    }

    pub fn scenarios(&self) -> &[String] {
        &self.scenarios
    }

    pub fn reference(&self) -> &str {
        &self.reference
    }

    pub fn set_reference(&mut self, reference: String) {
        if self.reference != reference {
            self.reference = reference;
            self.accepted = None;
        }
    }

    fn document(&self) -> Result<super::PreparationDocument, String> {
        let mut recipe = self.recipe.clone();
        let reference = timestamp("reference_time", &self.reference).map_err(|e| e.to_string())?;
        if recipe["schema_version"].as_u64() == Some(1) {
            // Legacy explicit periods keep their exact offsets from the reference.
            let original = timestamp(
                "reference_time",
                recipe["reference_time"]
                    .as_str()
                    .ok_or("Reference required")?,
            )
            .map_err(|e| e.to_string())?;
            let shift = reference - original;
            for key in [
                "historical_first_end",
                "historical_last_end",
                "forecast_first_end",
                "forecast_last_end",
            ] {
                let position = timestamp(key, recipe[key].as_str().ok_or("Boundary required")?)
                    .map_err(|e| e.to_string())?;
                let shifted = position
                    .checked_add_signed(shift)
                    .ok_or("Shifted coverage overflows the timestamp range")?;
                recipe[key] = shifted.format("%Y-%m-%dT%H:%M:%S").to_string().into();
            }
        }
        recipe["reference_time"] = self.reference.clone().into();
        decode_request_value(recipe).map_err(|e| e.to_string())
    }

    /// Return first and last interval-end labels for observations, then for forecasts.
    pub fn coverage(&self) -> Result<[String; 4], String> {
        coverage(&serde_json::to_value(self.document()?).map_err(|e| e.to_string())?)
    }

    // Reuse the typed selection and source acceptance checks used by draft experiments.
    // This transient adapter holds only registered TOML selections.
    fn selection(&self) -> Result<Draft, String> {
        let mut selection =
            Draft::from_preparation(self.workspace.clone(), &self.workspace, self.document()?)?;
        if let Some(accepted) = &self.accepted {
            selection.document.sources = accepted.clone();
        }
        Ok(selection)
    }

    pub fn has_accepted_inputs(&self) -> bool {
        self.accepted.is_some()
    }

    pub fn acquires_inputs(&self) -> bool {
        self.acquisition.is_some()
    }

    pub fn validate(&self) -> Vec<String> {
        if self.acquires_inputs() {
            return self.acquisition_setup().err().into_iter().collect();
        }
        match self.selection() {
            Ok(selection) => selection.validate_preparation(),
            Err(error) => vec![error],
        }
    }

    pub fn accept_inputs(&mut self) -> Result<(), String> {
        if self.acquires_inputs() {
            return Err("Run All resolves and accepts the acquired inputs".into());
        }
        let mut selection = self.selection()?;
        selection.refresh_inputs()?;
        let findings = selection.validate_preparation();
        if let Some(error) = findings.first() {
            return Err(error.clone());
        }
        self.accepted = Some(selection.document.sources);
        Ok(())
    }

    fn accepted_selection(&self) -> Result<Draft, OperationError> {
        if self.accepted.is_none() {
            return Err(OperationError::InvalidRequest {
                field: "inputs".into(),
                message: "Explicitly accept selected inputs for this reference".into(),
            });
        }
        self.selection()
            .map_err(|message| OperationError::InvalidRequest {
                field: "request".into(),
                message,
            })
    }

    fn acquisition_setup(&self) -> Result<(AcquisitionDocument, PreparationDocument), String> {
        let recipe = self
            .acquisition
            .as_ref()
            .ok_or("acquisition is not configured")?;
        if recipe.request.to_string_lossy().trim().is_empty() {
            return Err("acquisition.request must be nonempty".into());
        }
        let acquisition = AcquisitionDocument::read(&self.workspace, &recipe.request)?;
        let mut operation = self.document()?;
        let same_path = |left: &Path, right: &Path| {
            let path = |path: &Path| {
                let path = self.workspace.join(path);
                path.canonicalize().unwrap_or(path)
            };
            path(left) == path(right)
        };
        if acquisition.time_zone.name() != operation.time_zone
            || acquisition
                .ana
                .as_ref()
                .is_some_and(|ana| !same_path(&ana.database, &operation.observed.path))
            || acquisition.forecasts.as_ref().is_some_and(|forecast| {
                !same_path(&forecast.catalog, &operation.forecasts.catalog)
                    || forecast.model_bbox != operation.grid.bbox
            })
        {
            return Err("acquisition timezone, catalog paths and forecast bounds must match the operation recipe".into());
        }
        for &(index, provider) in &self.forecast_selectors {
            if acquisition
                .forecasts
                .as_ref()
                .is_none_or(|forecast| !forecast.providers.contains(&provider))
            {
                return Err(format!("acquisition does not enable {provider:?}"));
            }
            // Validate ordinary recipe fields before starting provider requests.
            operation.scenarios[index].asset_id = Some("pending-acquisition".into());
        }
        super::request::validate_document(operation.clone(), Vec::new())
            .map_err(|e| e.to_string())?;
        Ok((acquisition, operation))
    }

    /// Prepare and execute under one workspace lock and clean up the temporary preparation.
    pub fn run_all(&self, control: &OperationControl) -> Result<ExecutionReport, OperationError> {
        self.run_locked(control, &mut None)
    }

    pub fn run_all_report(&self, control: &OperationControl) -> OperationalRunReport {
        let mut acquisition = None;
        match self.run_locked(control, &mut acquisition) {
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

    fn run_locked(
        &self,
        control: &OperationControl,
        report: &mut Option<AcquisitionReport>,
    ) -> Result<ExecutionReport, OperationError> {
        control.check()?;
        let workspace = self.canonical_workspace()?;
        let saved = (!self.acquires_inputs())
            .then(|| self.accepted_selection())
            .transpose()?;
        let lock = lock_workspace(&workspace)?;
        let selection = match saved {
            Some(saved) => saved,
            None => self.acquire_selection(&workspace, control, report)?,
        };
        run_preparation_locked(control, &workspace, &lock, |workspace, output, control| {
            selection.prepare_selected(workspace.to_path_buf(), output, true, control)
        })
    }

    fn acquire_selection(
        &self,
        workspace: &Path,
        control: &OperationControl,
        report: &mut Option<AcquisitionReport>,
    ) -> Result<Draft, OperationError> {
        let error = |message| OperationError::InvalidRequest {
            field: "acquisition".into(),
            message,
        };
        let (document, mut recipe) = self.acquisition_setup().map_err(error)?;
        let reference = document
            .reference(Some(&self.reference), Utc::now())
            .map_err(error)?;
        let acquired = report.insert(
            document
                .acquire_locked(workspace, reference, None, control)
                .map_err(error)?,
        );
        control.check()?;
        if let Some(message) = acquired.run_blocker() {
            return Err(error(message.into()));
        }
        let forecast_end = timestamp("forecast_last_end", &recipe.forecast_last_end)
            .map_err(|e| error(e.to_string()))?;
        for &(index, provider) in &self.forecast_selectors {
            let forecast = acquired
                .forecasts
                .iter()
                .find(|forecast| forecast.provider == provider)
                .ok_or_else(|| error(format!("no acquired forecast for {provider:?}")))?;
            if forecast
                .valid_from
                .is_none_or(|start| start > reference.naive_local())
                || forecast.valid_to.is_none_or(|end| end < forecast_end)
            {
                return Err(error(format!(
                    "{provider:?} cycle does not cover the full operation forecast"
                )));
            }
            recipe.scenarios[index].asset_id = forecast.asset_id.clone();
        }
        let mut selection =
            Draft::from_preparation(workspace.to_path_buf(), workspace, recipe).map_err(error)?;
        selection.refresh_inputs().map_err(error)?;
        if let Some(finding) = selection.validate_preparation().into_iter().next() {
            return Err(error(finding));
        }
        Ok(selection)
    }

    fn canonical_workspace(&self) -> Result<PathBuf, OperationError> {
        self.workspace
            .canonicalize()
            .map_err(|e| OperationError::Source {
                path: self.workspace.clone(),
                message: e.to_string(),
            })
    }
}
