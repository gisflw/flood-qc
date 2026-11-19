use super::operation::{lock_workspace, run_preparation_locked};
use super::request::{coverage, decode_request_value, request_value, timestamp};
use super::{Draft, DraftSources, ExecutionReport, OperationControl, OperationError};
use chrono::{DateTime, Utc};
use std::path::{Path, PathBuf};

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
        let recipe = request_value(&bytes).map_err(|e| e.to_string())?;
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

    pub fn validate(&self) -> Vec<String> {
        match self.selection() {
            Ok(selection) => selection.validate_preparation(),
            Err(error) => vec![error],
        }
    }

    pub fn accept_inputs(&mut self) -> Result<(), String> {
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

    /// Prepare and execute under one workspace lock and clean up the temporary preparation.
    pub fn run_all(&self, control: &OperationControl) -> Result<ExecutionReport, OperationError> {
        control.check()?;
        let selection = self.accepted_selection()?;
        let workspace = self.canonical_workspace()?;
        let lock = lock_workspace(&workspace)?;
        run_preparation_locked(control, &workspace, &lock, |workspace, output, control| {
            selection.prepare_selected(workspace.to_path_buf(), output, true, control)
        })
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
