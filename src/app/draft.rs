use super::operation::{OperationError, PreparationReport};
use super::prepare::{TypedPreparationRequest, prepare_typed_checked};
use super::request::{
    PreparationDocument, axis, compact_period, decode_request, expand_period, historical_start,
    resolve_request, timestamp, validate_document,
};
use crate::analysis::{
    EffectiveObservation, PrecipitationQcFinding, PrecipitationQcPolicy, SelectedObservation,
    corrected_forecast_frames, detect_suspect_precipitation, effective_observations,
    select_observation_series,
};
use crate::store::{
    ForecastCatalog, ForecastCorrection, GridFrame, GridSeries, IntervalAxis, ObservationInputs,
    ObservationReplacement, ObservationSelection, ObservationStore, OperationManifest,
    check_correction, collect_inputs, digest_file, read_draft, read_retained_manifest,
    validate_replacement, write_draft,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

fn unavailable(workspace: &Path, path: &Path, directory: bool) -> Option<String> {
    if path.as_os_str().is_empty() || path.to_string_lossy().trim().is_empty() {
        return Some("empty path".into());
    }
    let path = super::absolute(workspace, path);
    let result = std::fs::metadata(&path).and_then(|metadata| {
        if directory {
            if !metadata.is_dir() {
                return Err(std::io::Error::other("not a directory"));
            }
            std::fs::read_dir(&path).map(|_| ())
        } else {
            if !metadata.is_file() || metadata.len() == 0 {
                return Err(std::io::Error::other("not a nonempty file"));
            }
            std::fs::File::open(&path).map(|_| ())
        }
    });
    result
        .err()
        .map(|error| format!("{}: {error}", path.display()))
}

pub(super) fn identity(value: &impl Serialize) -> Result<String, String> {
    let bytes = serde_json::to_vec(value).map_err(|e| e.to_string())?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

pub(super) fn observation_identity(
    inputs: &ObservationInputs,
    timeline: &IntervalAxis,
    selection: &ObservationSelection,
) -> Result<String, String> {
    let mut inputs = inputs.clone();
    inputs.stations.retain(|s| {
        selection.provider_codes.contains(&s.provider_code)
            || selection.station_ids.contains(&s.station_id)
    });
    inputs
        .series
        .retain(|s| inputs.stations.iter().any(|v| v.station_id == s.station_id));
    inputs
        .samples
        .retain(|s| inputs.series.iter().any(|v| v.series_id == s.series_id));
    identity(&(inputs, timeline))
}

pub(super) fn read_recipe_manifest(path: &Path) -> Result<OperationManifest, String> {
    let manifest = if path.is_dir() {
        let manifest = read_retained_manifest(path)?;
        if manifest.preparation_sha256.is_some() {
            super::operation::read_trial(path).map_err(|e| e.to_string())?;
        }
        manifest
    } else {
        let value: serde_json::Value = read_draft(path)?;
        if value.get("attempt_id").is_some() {
            crate::store::read_run_attempt(path)?
                .operation
                .ok_or("attempt has no retained operation recipe")?
        } else {
            serde_json::from_value(value).map_err(|e| e.to_string())?
        }
    };
    if manifest.schema_version != 1
        || !["prepared", "completed", "failed"].contains(&manifest.state.as_str())
    {
        return Err("expected a retained operation recipe".into());
    }
    Ok(manifest)
}

pub(super) struct ManifestRecipe {
    pub preparation: PreparationDocument,
    pub replacements: Vec<ObservationReplacement>,
    pub corrections: Vec<Option<Vec<ForecastCorrection>>>,
}

impl ManifestRecipe {
    pub(super) fn decode(manifest: &OperationManifest) -> Result<Self, String> {
        let mut document = manifest
            .recipe
            .get("document")
            .cloned()
            .ok_or("manifest has no recipe document")?;
        let object = document.as_object_mut().ok_or("invalid manifest recipe")?;
        let replacements: Vec<ObservationReplacement> = serde_json::from_value(
            object
                .remove("replacement_values")
                .ok_or("missing replacement values")?,
        )
        .map_err(|e| e.to_string())?;
        let corrections: Vec<ForecastCorrection> = serde_json::from_value(
            object
                .remove("correction_values")
                .ok_or("missing correction values")?,
        )
        .map_err(|e| e.to_string())?;
        object.remove("selected_series");
        object.remove("selected_stations");
        let preparation: PreparationDocument =
            serde_json::from_value(document).map_err(|e| e.to_string())?;
        let corrections = preparation
            .scenarios
            .iter()
            .map(|scenario| {
                scenario
                    .asset_id
                    .as_ref()
                    .map(|asset| {
                        scenario
                            .correction_ids
                            .as_deref()
                            .unwrap_or(&[])
                            .iter()
                            .map(|id| {
                                let value = corrections
                                    .iter()
                                    .find(|value| {
                                        &value.correction_id == id && &value.asset_id == asset
                                    })
                                    .ok_or("manifest missing selected correction")?
                                    .clone();
                                Ok(value)
                            })
                            .collect::<Result<Vec<_>, String>>()
                    })
                    .transpose()
            })
            .collect::<Result<Vec<_>, String>>()?;
        Ok(Self {
            preparation,
            replacements,
            corrections,
        })
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct DraftObserved {
    pub path: Option<PathBuf>,
    pub provider_codes: Option<Vec<String>>,
    pub station_ids: Option<Vec<String>>,
    pub series_ids: Option<Vec<String>>,
    pub replacement_ids: Option<Vec<String>>,
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct DraftGeometry {
    pub mini_gtp: Option<PathBuf>,
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct DraftGrid {
    pub bbox: Option<[f64; 4]>,
    pub resolution_degrees: Option<f64>,
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct DraftInterpolation {
    pub nearest_stations: Option<usize>,
    pub power: Option<f64>,
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct DraftLevels {
    pub station_ids: Option<Vec<String>>,
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct DraftEngine {
    pub executable: Option<PathBuf>,
    pub input_dir: Option<PathBuf>,
    pub omp_threads: Option<usize>,
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct DraftForecasts {
    pub catalog: Option<PathBuf>,
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct DraftQc {
    pub high_threshold_mm: Option<f64>,
    pub sequence_lower_mm: Option<f64>,
    pub sequence_upper_mm: Option<f64>,
    pub sequence_min_length: Option<usize>,
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct DraftScenario {
    pub id: Option<String>,
    pub kind: Option<String>,
    pub asset_id: Option<String>,
    pub correction_ids: Option<Vec<String>>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct DraftOperation {
    pub time_zone: Option<String>,
    pub reference_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub observed_horizon_days: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub forecast_horizon_days: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step_seconds: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub historical_first_end: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub historical_last_end: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub forecast_first_end: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub forecast_last_end: Option<String>,
    pub observed: Option<DraftObserved>,
    pub geometry: Option<DraftGeometry>,
    pub grid: Option<DraftGrid>,
    pub interpolation: Option<DraftInterpolation>,
    pub qc: Option<DraftQc>,
    pub levels: Option<DraftLevels>,
    pub engine: Option<DraftEngine>,
    pub forecasts: Option<DraftForecasts>,
    pub scenarios: Vec<DraftScenario>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub forcing: Option<super::ForcingDocument>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub calibration: Option<super::CalibrationDocument>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hot_start: Option<super::HotStartDocument>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub routing: Option<super::RoutingMode>,
}

impl DraftOperation {
    /// Read-only inclusive hourly coverage using the shared authoring-period rules.
    pub fn coverage(&self) -> Result<String, String> {
        let mut value = serde_json::to_value(self).map_err(|e| e.to_string())?;
        expand_period(&mut value).map_err(|e| e.to_string())?;
        let [hfirst, hlast, ffirst, flast] = super::request::coverage(&value)?;
        Ok(format!("{hfirst} → {hlast} · {ffirst} → {flast} · 1 h"))
    }

    fn validate_period_format(&self, version: u32) -> Result<(), String> {
        let explicit = self.step_seconds.is_some()
            || self.historical_first_end.is_some()
            || self.historical_last_end.is_some()
            || self.forecast_first_end.is_some()
            || self.forecast_last_end.is_some();
        let horizons = self.observed_horizon_days.is_some() || self.forecast_horizon_days.is_some();
        if (version == 2 && explicit) || (version == 1 && horizons) {
            return Err("draft period fields disagree with schema version".into());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DraftReplacement {
    pub draft_id: String,
    pub registered_origin: Option<String>,
    pub value: ObservationReplacement,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DraftCorrection {
    pub draft_id: String,
    pub registered_origin: Option<String>,
    pub value: ForecastCorrection,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct DraftInstructions {
    pub replacements: Option<Vec<DraftReplacement>>,
    pub corrections: Vec<Option<Vec<DraftCorrection>>>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct DraftSources {
    pub accepted: std::collections::BTreeMap<String, String>,
    pub generation: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct DraftVisualization {
    pub map_position: Option<[f64; 3]>,
    pub selected_location: Option<String>,
    pub visible_runs: Vec<String>,
    pub visible_scenarios: Vec<String>,
    pub chart_range: Option<[String; 2]>,
}

impl DraftVisualization {
    fn valid(&self) -> bool {
        let map_valid = self.map_position.is_none_or(|v| {
            v.iter().all(|n| n.is_finite())
                && (-90.0..=90.0).contains(&v[0])
                && (-180.0..=180.0).contains(&v[1])
                && (0.0..=30.0).contains(&v[2])
        });
        let range_valid = self.chart_range.as_ref().is_none_or(|v| {
            let start = chrono::NaiveDateTime::parse_from_str(&v[0], "%Y-%m-%dT%H:%M:%S");
            let end = chrono::NaiveDateTime::parse_from_str(&v[1], "%Y-%m-%dT%H:%M:%S");
            matches!((start, end), (Ok(start), Ok(end)) if start < end)
        });
        map_valid && range_valid
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DraftDocument {
    pub schema_version: u32,
    pub workspace: PathBuf,
    pub sources: DraftSources,
    pub operation: DraftOperation,
    pub instructions: DraftInstructions,
    #[serde(default)]
    pub visualization: Option<DraftVisualization>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawDraftDocument {
    schema_version: u32,
    workspace: PathBuf,
    sources: DraftSources,
    operation: DraftOperation,
    instructions: DraftInstructions,
    #[serde(default)]
    visualization: Option<serde_json::Value>,
}

#[derive(Clone, Debug)]
pub enum DraftAction {
    ObservationPreview,
    ForecastPreview { scenario: usize },
    Preparation,
}
#[derive(Clone, Debug, Serialize)]
pub struct DraftFinding {
    pub action: String,
    pub message: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct ObservationPreview {
    pub generation: String,
    pub rows: Vec<EffectiveObservation>,
    pub qc: Vec<PrecipitationQcFinding>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ForecastPreview {
    pub generation: String,
    pub raw: Vec<GridFrame>,
    pub corrected: Vec<GridFrame>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct RegistrationSelection {
    pub replacements: Vec<String>,
    pub corrections: Vec<RegistrationCorrectionSelection>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegistrationCorrectionSelection {
    pub scenario: usize,
    pub draft_id: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegistrationMapping {
    pub draft_id: String,
    pub scenario: Option<usize>,
    pub catalog_id: String,
}
#[derive(Clone, Debug, Serialize)]
pub struct RegistrationOutcome {
    pub mappings: Vec<RegistrationMapping>,
    pub error: Option<String>,
}
#[derive(Clone, Debug, Serialize)]
pub struct RegistrationReport {
    pub observations: RegistrationOutcome,
    pub forecasts: RegistrationOutcome,
}
impl RegistrationReport {
    pub fn succeeded(&self) -> bool {
        self.observations.error.is_none() && self.forecasts.error.is_none()
    }
}
fn registration_id(
    draft_id: &str,
    origin: &Option<String>,
    current: &str,
    value: &impl Serialize,
) -> Result<String, String> {
    if draft_id.trim().is_empty() {
        return Err("draft ID must be nonempty".into());
    }
    match origin {
        Some(origin) if origin.trim().is_empty() => {
            Err("registered origin must be nonempty".into())
        }
        Some(origin) => Ok(format!("draft-{}", identity(&(draft_id, origin, value))?)),
        None => Ok(current.into()),
    }
}
#[derive(Clone, Debug)]
pub struct Draft {
    pub document: DraftDocument,
    baseline: DraftDocument,
    anchor: DraftAnchor,
    pub path: Option<PathBuf>,
    pub workspace: PathBuf,
    invocation_dir: PathBuf,
    pub findings: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq)]
struct DraftAnchor {
    reference: Option<String>,
    zone: Option<String>,
}

impl DraftAnchor {
    fn from_operation(operation: &DraftOperation) -> Self {
        Self {
            reference: operation.reference_time.clone(),
            zone: operation.time_zone.clone(),
        }
    }

    fn check_edit(&self, proposed: &Self) -> Result<(), String> {
        if self.reference.is_some() && self.reference != proposed.reference
            || self.zone.is_some() && self.zone != proposed.zone
        {
            return Err("Draft reference time and timezone are fixed".into());
        }
        Ok(())
    }

    fn validate_time(&self) -> Result<(), String> {
        let (Some(reference), Some(zone)) = (&self.reference, &self.zone) else {
            return Ok(());
        };
        let time = timestamp("reference_time", reference).map_err(|e| e.to_string())?;
        let timezone: chrono_tz::Tz = zone.parse().map_err(|_| "expected IANA timezone")?;
        use chrono::TimeZone;
        timezone
            .from_local_datetime(&time)
            .single()
            .ok_or("ambiguous or absent local reference")?;
        Ok(())
    }

    fn apply(&self, operation: &mut DraftOperation) {
        operation.reference_time.clone_from(&self.reference);
        operation.time_zone.clone_from(&self.zone);
    }
}

impl Draft {
    pub fn new(workspace: PathBuf, invocation_dir: &Path) -> Self {
        let workspace = super::absolute(invocation_dir, &workspace);
        let config = super::read_workspace_config(&workspace.join("config/workspace.toml")).ok();
        Self::new_with_workspace_config(workspace, invocation_dir, config.as_ref())
    }

    pub fn new_with_workspace_config(
        workspace: PathBuf,
        invocation_dir: &Path,
        config: Option<&super::WorkspaceConfig>,
    ) -> Self {
        let workspace = super::absolute(invocation_dir, &workspace);
        let mut document = DraftDocument {
            schema_version: 2,
            workspace: workspace.clone(),
            sources: DraftSources::default(),
            operation: DraftOperation::default(),
            instructions: DraftInstructions::default(),
            visualization: None,
        };
        if let Some(config) = config {
            let engine = config.engine.clone().unwrap_or_default();
            document.operation.engine = Some(DraftEngine {
                executable: Some(
                    engine
                        .executable
                        .map_or_else(|| super::default_engine().into(), PathBuf::from),
                ),
                input_dir: Some(
                    engine
                        .input_dir
                        .map_or_else(|| PathBuf::from("mgb_runner/Input"), PathBuf::from),
                ),
                omp_threads: None,
            });
        }
        Self {
            baseline: document.clone(),
            document,
            anchor: DraftAnchor::default(),
            path: None,
            workspace: workspace.clone(),
            invocation_dir: invocation_dir.into(),
            findings: Vec::new(),
        }
    }
    pub(super) fn with_engine_overrides(
        mut self,
        executable: Option<PathBuf>,
        input_dir: Option<PathBuf>,
    ) -> Self {
        if executable.is_some() || input_dir.is_some() {
            let engine = self.document.operation.engine.get_or_insert_default();
            engine.executable = executable.or_else(|| engine.executable.take());
            engine.input_dir = input_dir.or_else(|| engine.input_dir.take());
            self.baseline = self.document.clone();
        }
        self
    }

    /// Initialize a fresh fixed-reference run without selecting operational sources.
    pub fn new_run(
        workspace: PathBuf,
        invocation_dir: &Path,
        config: Option<&super::WorkspaceConfig>,
        reference: &str,
        zone: &str,
    ) -> Result<Self, String> {
        let mut draft = Self::new_with_workspace_config(workspace, invocation_dir, config);
        draft.initialize_anchor(reference, zone)?;
        for (path, kind) in [
            (PathBuf::from("data/history.sqlite"), 0),
            (PathBuf::from("data/forecasts.sqlite"), 1),
        ] {
            if super::absolute(&draft.workspace, &path).is_file() {
                if kind == 0 {
                    draft.document.operation.observed = Some(DraftObserved {
                        path: Some(path),
                        ..Default::default()
                    });
                } else {
                    draft.document.operation.forecasts = Some(DraftForecasts {
                        catalog: Some(path),
                    });
                }
            }
        }
        if let Some(input) = draft
            .document
            .operation
            .engine
            .as_ref()
            .and_then(|e| e.input_dir.as_ref())
        {
            let path = input.join("MINI.gtp");
            if super::absolute(&draft.workspace, &path).is_file() {
                draft.document.operation.geometry = Some(DraftGeometry {
                    mini_gtp: Some(path),
                });
            }
        }
        draft.baseline = draft.document.clone();
        Ok(draft)
    }

    /// Fill the unset parts of a draft's anchor once. Existing source acceptance
    /// belongs to the old selection and must be obtained again.
    pub fn initialize_anchor(&mut self, reference: &str, zone: &str) -> Result<(), String> {
        let proposed = DraftAnchor {
            reference: Some(reference.into()),
            zone: Some(zone.into()),
        };
        self.anchor.check_edit(&proposed)?;
        proposed.validate_time()?;
        if self.anchor != proposed {
            self.document.sources = DraftSources::default();
        }
        proposed.apply(&mut self.document.operation);
        self.anchor = proposed;
        Ok(())
    }

    /// Instantiate a new scheduled selection, leaving the saved draft untouched.
    pub(super) fn for_rolling_slot(&self, reference: &str, zone: &str) -> Result<Self, String> {
        self.validate_boundary()?;
        if !self.workspace_matches() {
            return Err("saved workspace differs from selected workspace".into());
        }
        let mut slot = self.clone();
        slot.path = None;
        slot.anchor = DraftAnchor::default();
        slot.initialize_anchor(reference, zone)?;
        slot.baseline = slot.document.clone();
        Ok(slot)
    }

    pub fn from_preparation(
        workspace: PathBuf,
        invocation_dir: &Path,
        document: PreparationDocument,
    ) -> Result<Self, String> {
        if document.schema_version != 1 {
            return Err("unsupported preparation schema version".into());
        }
        if document
            .grid
            .bbox
            .iter()
            .copied()
            .chain([
                document.grid.resolution_degrees,
                document.interpolation.power,
                document.qc.high_threshold_mm,
                document.qc.sequence_lower_mm,
                document.qc.sequence_upper_mm,
            ])
            .any(|v| !v.is_finite())
        {
            return Err("preparation contains a nonfinite operational value".into());
        }
        let mut draft = Self::new(workspace, invocation_dir);
        let mut value = serde_json::to_value(document).map_err(|e| e.to_string())?;
        value
            .as_object_mut()
            .ok_or("invalid preparation")?
            .remove("schema_version");
        draft.document.schema_version = if compact_period(&mut value) { 2 } else { 1 };
        draft.document.operation = serde_json::from_value(value).map_err(|e| e.to_string())?;
        draft.anchor = DraftAnchor::from_operation(&draft.document.operation);
        draft.baseline = draft.document.clone();
        Ok(draft)
    }
    pub fn import_toml(
        workspace: PathBuf,
        invocation_dir: &Path,
        path: &Path,
    ) -> Result<Self, String> {
        let path = super::absolute(invocation_dir, path);
        let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        let document = decode_request(text.as_bytes()).map_err(|e| e.to_string())?;
        Self::from_preparation(workspace, invocation_dir, document)
    }
    pub fn import_manifest(
        workspace: PathBuf,
        invocation_dir: &Path,
        path: &Path,
    ) -> Result<Self, String> {
        let path = super::absolute(invocation_dir, path);
        let manifest = read_recipe_manifest(&path)?;
        Self::from_manifest(workspace, invocation_dir, &manifest)
    }

    fn from_manifest(
        workspace: PathBuf,
        invocation_dir: &Path,
        manifest: &OperationManifest,
    ) -> Result<Self, String> {
        let ManifestRecipe {
            preparation,
            replacements,
            corrections,
        } = ManifestRecipe::decode(manifest)?;
        let mut draft = Self::from_preparation(workspace, invocation_dir, preparation)?;
        draft.document.instructions.replacements = Some(
            replacements
                .into_iter()
                .map(|value| DraftReplacement {
                    draft_id: value.replacement_id.clone(),
                    registered_origin: None,
                    value,
                })
                .collect(),
        );
        draft.document.instructions.corrections = corrections
            .into_iter()
            .map(|values| {
                values.map(|values| {
                    values
                        .into_iter()
                        .map(|value| DraftCorrection {
                            draft_id: value.correction_id.clone(),
                            registered_origin: None,
                            value,
                        })
                        .collect()
                })
            })
            .collect();
        draft.baseline = draft.document.clone();
        Ok(draft)
    }
    /// Convert an explicit-boundary recipe only when all inclusive hourly labels are unchanged.
    pub fn use_day_horizons(&mut self) -> Result<(), String> {
        self.validate_anchor(&self.document)?;
        if self.document.schema_version == 2 {
            return Ok(());
        }
        let mut value =
            serde_json::to_value(&self.document.operation).map_err(|e| e.to_string())?;
        if !compact_period(&mut value) {
            return Err("Legacy coverage cannot be represented by whole-day horizons without changing labels; original file preserved. Use the existing CLI workflow.".into());
        }
        self.document.operation = serde_json::from_value(value).map_err(|e| e.to_string())?;
        self.document.schema_version = 2;
        Ok(())
    }

    pub fn preparation_document(&self) -> Result<PreparationDocument, String> {
        self.validate_boundary()?;
        let mut value =
            serde_json::to_value(&self.document.operation).map_err(|e| e.to_string())?;
        value
            .as_object_mut()
            .ok_or("invalid operation")?
            .insert("schema_version".into(), 1.into());
        expand_period(&mut value).map_err(|e| e.to_string())?;
        serde_json::from_value(value).map_err(|e| e.to_string())
    }
    pub fn save_preparation_toml(&self, path: &Path) -> Result<(), String> {
        if self.document.instructions.replacements.is_some()
            || self
                .document
                .instructions
                .corrections
                .iter()
                .any(Option::is_some)
        {
            return Err("Draft instruction values cannot be saved in an operation TOML; save the draft instead".into());
        }
        let document = self.preparation_document()?;
        let mut value = serde_json::to_value(&document).map_err(|e| e.to_string())?;
        let mut output = toml::Value::try_from(&document).map_err(|e| e.to_string())?;
        if compact_period(&mut value) {
            let table = output.as_table_mut().ok_or("invalid preparation")?;
            for key in [
                "step_seconds",
                "historical_first_end",
                "historical_last_end",
                "forecast_first_end",
                "forecast_last_end",
            ] {
                table.remove(key);
            }
            table.insert("schema_version".into(), 2.into());
            for key in ["observed_horizon_days", "forecast_horizon_days"] {
                table.insert(
                    key.into(),
                    value[key].as_i64().ok_or("invalid horizon")?.into(),
                );
            }
        }
        let text = toml::to_string_pretty(&output).map_err(|e| e.to_string())?;
        crate::store::write_atomic(
            &super::absolute(&self.invocation_dir, path),
            text.as_bytes(),
        )
    }
    pub fn open(workspace: PathBuf, invocation_dir: &Path, path: &Path) -> Result<Self, String> {
        let workspace = super::absolute(invocation_dir, &workspace);
        let path = super::absolute(invocation_dir, path);
        let raw: RawDraftDocument = read_draft(&path)?;
        if !matches!(raw.schema_version, 1 | 2) {
            return Err("unsupported draft schema version".into());
        }
        raw.operation.validate_period_format(raw.schema_version)?;
        let mut findings = Vec::new();
        let visualization = match raw.visualization {
            Some(value) => match serde_json::from_value::<DraftVisualization>(value) {
                Ok(view) if view.valid() => Some(view),
                _ => {
                    findings.push("invalid visualization state was omitted".into());
                    None
                }
            },
            None => None,
        };
        let document = DraftDocument {
            schema_version: raw.schema_version,
            workspace: raw.workspace,
            sources: raw.sources,
            operation: raw.operation,
            instructions: raw.instructions,
            visualization,
        };
        let saved_workspace = super::absolute(invocation_dir, &document.workspace);
        let matches = match (workspace.canonicalize(), saved_workspace.canonicalize()) {
            (Ok(chosen), Ok(saved)) => chosen == saved,
            _ => workspace == saved_workspace,
        };
        if !matches {
            findings.push("saved workspace differs from selected workspace".into());
        }
        let mut draft = Self {
            baseline: document.clone(),
            anchor: DraftAnchor::from_operation(&document.operation),
            document,
            path: Some(path),
            workspace,
            invocation_dir: invocation_dir.into(),
            findings,
        };
        if matches {
            let operation = &draft.document.operation;
            let sources = [
                (
                    "observations",
                    operation.observed.as_ref().and_then(|v| v.path.as_deref()),
                    false,
                ),
                (
                    "forecast catalog",
                    operation
                        .forecasts
                        .as_ref()
                        .and_then(|v| v.catalog.as_deref()),
                    false,
                ),
                (
                    "geometry",
                    operation
                        .geometry
                        .as_ref()
                        .and_then(|v| v.mini_gtp.as_deref()),
                    false,
                ),
                (
                    "engine",
                    operation
                        .engine
                        .as_ref()
                        .and_then(|v| v.executable.as_deref()),
                    false,
                ),
                (
                    "engine inputs",
                    operation
                        .engine
                        .as_ref()
                        .and_then(|v| v.input_dir.as_deref()),
                    true,
                ),
            ];
            for (label, path, directory) in sources {
                if let Some(path) = path
                    && let Some(error) = unavailable(&draft.workspace, path, directory)
                {
                    draft.findings.push(format!("{label} unavailable: {error}"));
                }
            }
            if let Some(catalog_path) = operation
                .forecasts
                .as_ref()
                .and_then(|v| v.catalog.as_deref())
                && unavailable(&draft.workspace, catalog_path, false).is_none()
            {
                match ForecastCatalog::open_read_only(&draft.workspace, catalog_path) {
                    Err(error) => draft
                        .findings
                        .push(format!("forecast catalog unavailable: {error}")),
                    Ok(catalog) => {
                        for (index, scenario) in operation.scenarios.iter().enumerate() {
                            if let Some(asset) = &scenario.asset_id
                                && !draft
                                    .document
                                    .sources
                                    .accepted
                                    .contains_key(&format!("forecast:{index}"))
                                && let Err(error) = catalog.check_asset_available(asset)
                            {
                                draft
                                    .findings
                                    .push(format!("forecast:{index} unavailable: {error}"));
                            }
                        }
                    }
                }
            }
        }
        if matches {
            if draft.document.sources.accepted.contains_key("observations") {
                let mut check = draft.clone();
                match check.refresh_observations() {
                    Ok(())
                        if check.document.sources.accepted.get("observations")
                            == draft.document.sources.accepted.get("observations") => {}
                    Ok(()) => draft
                        .findings
                        .push("observations changed; refresh inputs".into()),
                    Err(error) => draft
                        .findings
                        .push(format!("observations unavailable: {error}")),
                }
            }
            for index in 0..draft.document.operation.scenarios.len() {
                let key = format!("forecast:{index}");
                if draft.document.sources.accepted.contains_key(&key) {
                    let mut check = draft.clone();
                    match check.refresh_forecast(index) {
                        Ok(())
                            if check.document.sources.accepted.get(&key)
                                == draft.document.sources.accepted.get(&key) => {}
                        Ok(()) => draft
                            .findings
                            .push(format!("{key} changed; refresh inputs")),
                        Err(error) => draft.findings.push(format!("{key} unavailable: {error}")),
                    }
                }
            }
        }
        if matches && draft.document.sources.accepted.contains_key("geometry") {
            for finding in draft.validate_preparation() {
                if !draft.findings.contains(&finding) {
                    draft.findings.push(finding);
                }
            }
        }
        if let Some(view) = &draft.document.visualization {
            if let Some(location) = &view.selected_location {
                if let Some(id) = location.strip_prefix("station:") {
                    let result = draft
                        .document
                        .operation
                        .observed
                        .as_ref()
                        .and_then(|o| o.path.as_ref())
                        .ok_or_else(|| "observation database not selected".to_owned())
                        .and_then(|path| {
                            crate::store::ObservationStore::open_read_only(&super::absolute(
                                &draft.workspace,
                                path,
                            ))
                            .map_err(|e| e.to_string())
                        })
                        .and_then(|store| store.station_locations().map_err(|e| e.to_string()));
                    match result {
                        Ok(stations) if stations.iter().any(|s| s.station.station_id == id) => {}
                        Ok(_) => draft
                            .findings
                            .push(format!("view station unavailable: {id}")),
                        Err(e) => draft
                            .findings
                            .push(format!("view station unavailable: {id}: {e}")),
                    }
                } else if let Some(id) = location.strip_prefix("mini:") {
                    let result = draft
                        .document
                        .operation
                        .geometry
                        .as_ref()
                        .and_then(|g| g.mini_gtp.as_ref())
                        .ok_or_else(|| "MINI topology not selected".to_owned())
                        .and_then(|path| {
                            crate::store::read_mini_topology(&super::absolute(
                                &draft.workspace,
                                path,
                            ))
                        });
                    match result {
                        Ok((_, nodes)) if nodes.iter().any(|n| n.mini_id.to_string() == id) => {}
                        Ok(_) => draft.findings.push(format!("view MINI unavailable: {id}")),
                        Err(e) => draft
                            .findings
                            .push(format!("view MINI unavailable: {id}: {e}")),
                    }
                } else {
                    draft
                        .findings
                        .push(format!("view location invalid: {location}"));
                }
            }
            let mut scenarios = std::collections::HashSet::new();
            for path in &view.visible_runs {
                let root = super::absolute(&draft.workspace, Path::new(path));
                match read_retained_manifest(&root) {
                    Ok(manifest) => {
                        for scenario in manifest
                            .scenarios
                            .into_iter()
                            .filter(|s| s.state == "completed")
                        {
                            if view.visible_scenarios.contains(&scenario.id) {
                                if scenario.id.is_empty()
                                    || !scenario.id.bytes().all(|b| {
                                        b.is_ascii_alphanumeric() || b == b'_' || b == b'-'
                                    })
                                {
                                    draft
                                        .findings
                                        .push(format!("view scenario invalid: {}", scenario.id));
                                    continue;
                                }
                                scenarios.insert(scenario.id.clone());
                                for name in ["basin.nc", "station.nc"] {
                                    if !root.join("results").join(&scenario.id).join(name).is_file()
                                    {
                                        draft.findings.push(format!(
                                            "view result unavailable: {path} / {} / {name}",
                                            scenario.id
                                        ));
                                    }
                                }
                            }
                        }
                    }
                    Err(error) => draft
                        .findings
                        .push(format!("view result unavailable: {path}: {error}")),
                }
            }
            for scenario in &view.visible_scenarios {
                if !scenarios.contains(scenario) {
                    draft
                        .findings
                        .push(format!("view scenario unavailable: {scenario}"));
                }
            }
        }
        Ok(draft)
    }
    fn validate_anchor(&self, document: &DraftDocument) -> Result<(), String> {
        let proposed = DraftAnchor::from_operation(&document.operation);
        self.anchor.check_edit(&proposed)?;
        if self.anchor != proposed
            && (!document.sources.accepted.is_empty() || !self.baseline.sources.accepted.is_empty())
        {
            return Err(
                "Initialize the draft anchor explicitly before accepting sources again".into(),
            );
        }
        Ok(())
    }

    pub(super) fn validate_working_document(
        &self,
        proposed: &DraftDocument,
        allow_clear_acceptance: bool,
    ) -> Result<(), String> {
        if proposed.workspace != self.document.workspace {
            return Err("The draft workspace cannot be changed by a working update".into());
        }
        let sources = &self.document.sources;
        if proposed.sources != *sources
            && !(allow_clear_acceptance
                && proposed.sources.generation == sources.generation
                && proposed
                    .sources
                    .accepted
                    .iter()
                    .all(|(key, value)| sources.accepted.get(key) == Some(value)))
        {
            return Err("Accept sources through the explicit source workflow".into());
        }
        let anchor = DraftAnchor::from_operation(&proposed.operation);
        if anchor != self.anchor {
            anchor.validate_time()?;
        }
        self.validate_document(proposed)
    }

    pub(super) fn same_edit_state(&self, other: &Self) -> bool {
        fn same_document(left: &DraftDocument, right: &DraftDocument) -> bool {
            left.schema_version == right.schema_version
                && left.workspace == right.workspace
                && left.sources == right.sources
                && left.operation == right.operation
                && left.instructions == right.instructions
        }
        same_document(&self.document, &other.document)
            && same_document(&self.baseline, &other.baseline)
            && self.anchor == other.anchor
            && self.path == other.path
    }

    fn validate_boundary(&self) -> Result<(), String> {
        self.validate_document(&self.document)
    }

    fn validate_document(&self, document: &DraftDocument) -> Result<(), String> {
        self.validate_anchor(document)?;
        if !matches!(document.schema_version, 1 | 2) {
            return Err("unsupported draft schema version".into());
        }
        let operation = &document.operation;
        operation.validate_period_format(document.schema_version)?;
        let mut numbers = Vec::new();
        if let Some(grid) = &operation.grid {
            numbers.extend(grid.bbox.into_iter().flatten());
            numbers.extend(grid.resolution_degrees);
        }
        if let Some(interpolation) = &operation.interpolation {
            numbers.extend(interpolation.power);
        }
        if let Some(qc) = &operation.qc {
            numbers.extend(
                [
                    qc.high_threshold_mm,
                    qc.sequence_lower_mm,
                    qc.sequence_upper_mm,
                ]
                .into_iter()
                .flatten(),
            );
        }
        if let Some(replacements) = &document.instructions.replacements {
            numbers.extend(replacements.iter().filter_map(|v| v.value.value));
        }
        for values in document.instructions.corrections.iter().flatten() {
            for item in values {
                numbers.extend([
                    item.value.shift_lat_pixels,
                    item.value.shift_lon_pixels,
                    item.value.rotation_deg,
                    item.value.multiplication_factor,
                ]);
            }
        }
        if numbers.iter().any(|v| !v.is_finite()) {
            return Err("draft contains a nonfinite operational value".into());
        }
        Ok(())
    }
    pub fn save(&mut self, path: &Path, include_visualization: bool) -> Result<(), String> {
        self.validate_boundary()?;
        let mut saved = self.document.clone();
        let invalid_view = saved
            .visualization
            .as_ref()
            .is_some_and(|view| !view.valid());
        if invalid_view {
            saved.visualization = None;
        }
        if !include_visualization {
            saved.visualization = None;
        }
        let path = super::absolute(&self.invocation_dir, path);
        write_draft(&path, &saved)?;
        if invalid_view {
            self.findings
                .push("invalid visualization state was omitted".into());
        }
        self.baseline = saved;
        self.anchor = DraftAnchor::from_operation(&self.document.operation);
        self.path = Some(path);
        Ok(())
    }
    fn workspace_matches(&self) -> bool {
        let saved = super::absolute(&self.invocation_dir, &self.document.workspace);
        match (self.workspace.canonicalize(), saved.canonicalize()) {
            (Ok(chosen), Ok(saved)) => chosen == saved,
            _ => self.workspace == saved,
        }
    }
    fn observation_inputs(&self) -> Result<(ObservationInputs, IntervalAxis), String> {
        self.validate_boundary()?;
        if self
            .document
            .operation
            .step_seconds
            .is_some_and(|v| v != 3600)
        {
            return Err("only hourly steps are qualified".into());
        }
        if !self.workspace_matches() {
            return Err("saved workspace differs from selected workspace".into());
        }
        let operation = &self.document.operation;
        let observed = operation
            .observed
            .as_ref()
            .ok_or("select observation source")?;
        let zone = operation
            .time_zone
            .as_deref()
            .ok_or("select operation timezone")?;
        let (first, last) = if self.document.schema_version == 2 {
            let reference = timestamp(
                "reference_time",
                operation
                    .reference_time
                    .as_deref()
                    .ok_or("select reference time")?,
            )
            .map_err(|e| e.to_string())?;
            let days = operation
                .observed_horizon_days
                .ok_or("select observed days")?;
            (
                historical_start(reference, chrono::Duration::days(i64::from(days)))
                    .map_err(|e| e.to_string())?,
                reference,
            )
        } else {
            let first = timestamp(
                "historical_first_end",
                operation
                    .historical_first_end
                    .as_deref()
                    .ok_or("select historical start")?,
            )
            .map_err(|e| e.to_string())?;
            let last = timestamp(
                "historical_last_end",
                operation
                    .historical_last_end
                    .as_deref()
                    .ok_or("select historical end")?,
            )
            .map_err(|e| e.to_string())?;
            (first, last)
        };
        let timeline = axis(first, last, zone, "historical").map_err(|e| e.to_string())?;
        let path = observed
            .path
            .as_deref()
            .ok_or("select observation source")?;
        let path = super::absolute(&self.workspace, path);
        let selection = ObservationSelection {
            provider_codes: observed.provider_codes.clone().unwrap_or_default(),
            station_ids: observed.station_ids.clone().unwrap_or_default(),
        };
        let ids = observed.replacement_ids.as_deref().unwrap_or(&[]);
        let store = ObservationStore::open_read_only(&path).map_err(|e| e.to_string())?;
        let (mut inputs, _) = store
            .read_selection(
                &selection,
                &timeline,
                if self.document.instructions.replacements.is_some() {
                    &[]
                } else {
                    ids
                },
            )
            .map_err(|e| e.to_string())?;
        let chosen = select_observation_series(
            &inputs.series,
            observed.series_ids.as_deref().unwrap_or(&[]),
        )
        .map_err(|e| e.to_string())?;
        if chosen.is_empty() {
            return Err("selection has no rainfall series".into());
        }
        inputs
            .series
            .retain(|s| s.variable != "rain" || chosen.iter().any(|v| v.series_id == s.series_id));
        inputs
            .samples
            .retain(|s| inputs.series.iter().any(|v| v.series_id == s.series_id));
        Ok((inputs, timeline))
    }
    pub fn refresh_observations(&mut self) -> Result<(), String> {
        let (inputs, timeline) = self.observation_inputs()?;
        let observed = self
            .document
            .operation
            .observed
            .as_ref()
            .ok_or("select observation source")?;
        let selection = ObservationSelection {
            provider_codes: observed.provider_codes.clone().unwrap_or_default(),
            station_ids: observed.station_ids.clone().unwrap_or_default(),
        };
        let identity = observation_identity(&inputs, &timeline, &selection)?;
        self.document.sources.generation = self
            .document
            .sources
            .generation
            .checked_add(1)
            .ok_or("source generation overflow")?;
        self.document
            .sources
            .accepted
            .insert("observations".into(), identity);
        self.anchor = DraftAnchor::from_operation(&self.document.operation);
        self.findings
            .retain(|finding| !finding.starts_with("observations "));
        Ok(())
    }
    pub fn preview_observations(&self) -> Result<ObservationPreview, String> {
        let (mut inputs, timeline) = self.observation_inputs()?;
        let observed = self
            .document
            .operation
            .observed
            .as_ref()
            .ok_or("select observation source")?;
        let selection = ObservationSelection {
            provider_codes: observed.provider_codes.clone().unwrap_or_default(),
            station_ids: observed.station_ids.clone().unwrap_or_default(),
        };
        let identity = observation_identity(&inputs, &timeline, &selection)?;
        if self.document.sources.accepted.get("observations") != Some(&identity) {
            return Err(
                "observation source changed or has not been accepted; refresh inputs".into(),
            );
        }
        if let Some(values) = &self.document.instructions.replacements {
            let mut seen = std::collections::HashSet::new();
            if values
                .iter()
                .any(|v| v.draft_id.trim().is_empty() || !seen.insert(&v.draft_id))
            {
                return Err("draft replacement IDs must be nonempty and unique".into());
            }
            for item in values {
                validate_replacement(&item.value).map_err(|e| e.to_string())?;
            }
            inputs.replacements = values.iter().map(|v| v.value.clone()).collect();
        }
        let rows = effective_observations(&inputs, &timeline).map_err(|e| e.to_string())?;
        let raw: Vec<_> = rows
            .iter()
            .filter(|row| row.raw_present)
            .map(|row| SelectedObservation {
                station_id: row.station_id.clone(),
                series_id: row.series_id.clone(),
                observed_at: row.observed_at,
                longitude: row.longitude,
                latitude: row.latitude,
                value: row.raw_value,
            })
            .collect();
        let policy = self
            .document
            .operation
            .qc
            .as_ref()
            .ok_or("select QC policy")?;
        let policy: PrecipitationQcPolicy =
            serde_json::from_value(serde_json::to_value(policy).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
        let qc = detect_suspect_precipitation(&raw, &policy).map_err(|e| e.to_string())?;
        Ok(ObservationPreview {
            generation: self.generation()?,
            rows,
            qc,
        })
    }
    fn forecast_inputs(
        &self,
        index: usize,
    ) -> Result<(GridSeries, Vec<ForecastCorrection>), String> {
        self.validate_boundary()?;
        if !self.workspace_matches() {
            return Err("saved workspace differs from selected workspace".into());
        }
        let operation = &self.document.operation;
        let scenario = operation.scenarios.get(index).ok_or("select a scenario")?;
        let asset = scenario
            .asset_id
            .as_deref()
            .ok_or("select a forecast asset")?;
        let catalog = operation
            .forecasts
            .as_ref()
            .and_then(|f| f.catalog.as_deref())
            .ok_or("select a forecast catalog")?;
        let catalog =
            ForecastCatalog::open_read_only(&self.workspace, catalog).map_err(|e| e.to_string())?;
        let supplied = self
            .document
            .instructions
            .corrections
            .get(index)
            .and_then(Option::as_ref);
        let ids = if supplied.is_some() {
            &[][..]
        } else {
            scenario.correction_ids.as_deref().unwrap_or(&[])
        };
        let (_, native, corrections) = catalog
            .load_selected(asset, ids)
            .map_err(|e| e.to_string())?;
        Ok((native, corrections))
    }
    pub fn refresh_forecast(&mut self, index: usize) -> Result<(), String> {
        let inputs = self.forecast_inputs(index)?;
        let identity = identity(&inputs)?;
        self.document.sources.generation = self
            .document
            .sources
            .generation
            .checked_add(1)
            .ok_or("source generation overflow")?;
        self.document
            .sources
            .accepted
            .insert(format!("forecast:{index}"), identity);
        self.anchor = DraftAnchor::from_operation(&self.document.operation);
        let prefix = format!("forecast:{index} ");
        self.findings
            .retain(|finding| !finding.starts_with(&prefix));
        Ok(())
    }
    pub fn preview_forecast(&self, index: usize) -> Result<ForecastPreview, String> {
        let (native, registered) = self.forecast_inputs(index)?;
        let identity = identity(&(&native, &registered))?;
        if self
            .document
            .sources
            .accepted
            .get(&format!("forecast:{index}"))
            != Some(&identity)
        {
            return Err("forecast source changed or has not been accepted; refresh inputs".into());
        }
        let supplied = self
            .document
            .instructions
            .corrections
            .get(index)
            .and_then(Option::as_ref);
        let corrections = if let Some(values) = supplied {
            let mut seen = std::collections::HashSet::new();
            if values
                .iter()
                .any(|v| v.draft_id.trim().is_empty() || !seen.insert(&v.draft_id))
            {
                return Err("draft correction IDs must be nonempty and unique".into());
            }
            for item in values {
                check_correction(&item.value).map_err(|e| e.to_string())?;
            }
            values.iter().map(|v| v.value.clone()).collect::<Vec<_>>()
        } else {
            registered
        };

        let used: Vec<_> = (0..native.timeline.bounds.len()).collect();
        let (raw, _) = corrected_forecast_frames(&native, &[], &used).map_err(|e| e.to_string())?;
        let (corrected, _) =
            corrected_forecast_frames(&native, &corrections, &used).map_err(|e| e.to_string())?;
        Ok(ForecastPreview {
            generation: self.generation()?,
            raw,
            corrected,
        })
    }
    fn effective_document(&self) -> Result<PreparationDocument, String> {
        self.validate_boundary()?;
        let mut document = self.preparation_document()?;
        if let Some(values) = &self.document.instructions.replacements {
            let mut ids = std::collections::HashSet::new();
            if values
                .iter()
                .any(|v| v.draft_id.trim().is_empty() || !ids.insert(&v.draft_id))
            {
                return Err("draft replacement IDs must be nonempty and unique".into());
            }
            for item in values {
                validate_replacement(&item.value).map_err(|e| e.to_string())?;
            }
            document.observed.replacement_ids = values
                .iter()
                .map(|v| v.value.replacement_id.clone())
                .collect();
        }
        if self.document.instructions.corrections.len() > document.scenarios.len() {
            return Err("correction selections exceed scenarios".into());
        }
        let mut corrections = std::collections::HashMap::new();
        for (index, values) in self.document.instructions.corrections.iter().enumerate() {
            if let Some(values) = values {
                let mut ids = std::collections::HashSet::new();
                if values
                    .iter()
                    .any(|v| v.draft_id.trim().is_empty() || !ids.insert(&v.draft_id))
                {
                    return Err("draft correction IDs must be nonempty and unique".into());
                }
                for item in values {
                    check_correction(&item.value).map_err(|e| e.to_string())?;
                    if let Some(previous) =
                        corrections.insert(&item.value.correction_id, &item.value)
                        && previous != &item.value
                    {
                        return Err("conflicting correction ID".into());
                    }
                    if document.scenarios[index].asset_id.as_deref()
                        != Some(item.value.asset_id.as_str())
                    {
                        return Err("correction belongs to another asset".into());
                    }
                }
                document.scenarios[index].correction_ids = Some(
                    values
                        .iter()
                        .map(|v| v.value.correction_id.clone())
                        .collect(),
                );
            }
        }
        Ok(document)
    }
    pub fn refresh_inputs(&mut self) -> Result<(), String> {
        self.refresh_all_inputs().map(|_| ())
    }

    fn refresh_all_inputs(&mut self) -> Result<IntervalAxis, String> {
        let mut refreshed = self.clone();
        refreshed.refresh_observations()?;
        let document = self.effective_document()?;
        let valid = validate_document(document.clone(), Vec::new()).map_err(|e| e.to_string())?;
        let valid = resolve_request(&self.workspace, valid).map_err(|e| e.to_string())?;
        let selection = ObservationSelection {
            provider_codes: document.observed.provider_codes.clone(),
            station_ids: document.observed.station_ids.clone(),
        };
        let store = ObservationStore::open_read_only(&valid.document.observed.path)
            .map_err(|e| e.to_string())?;
        let stations = super::additional::extra_stations(&store, &valid.document)?;
        let (mut inputs, offsets) = store
            .read_selection_with_stations(
                &selection,
                &valid.historical,
                if self.document.instructions.replacements.is_some() {
                    &[]
                } else {
                    &document.observed.replacement_ids
                },
                &stations,
            )
            .map_err(|e| e.to_string())?;
        let registered = inputs.replacements.clone();
        if let Some(values) = &self.document.instructions.replacements {
            inputs.replacements = values.iter().map(|item| item.value.clone()).collect();
        }
        if let Some(inputs) = super::additional::capture(&valid, &inputs, &registered)? {
            refreshed
                .document
                .sources
                .accepted
                .insert("additional_forcing".into(), inputs.accepted);
        } else {
            refreshed
                .document
                .sources
                .accepted
                .remove("additional_forcing");
        }
        super::additional::check_engine(&valid.document, &valid.document.engine.executable)?;
        if let Some(calibration) = &valid.document.calibration {
            let geometry = calibration
                .use_edited_geometry
                .then_some(valid.document.geometry.mini_gtp.as_path());
            let soil = calibration.soil.as_deref();
            crate::model::validate_calibration(&valid.document.engine.input_dir, geometry, soil)?;
        }
        if let Some(value) = super::additional::soil_identity(valid.document.calibration.as_ref())?
        {
            refreshed
                .document
                .sources
                .accepted
                .insert("soil".into(), value);
        } else {
            refreshed.document.sources.accepted.remove("soil");
        }
        refreshed
            .document
            .sources
            .accepted
            .insert("offsets".into(), identity(&offsets)?);
        for (key, path) in [
            ("geometry", &valid.document.geometry.mini_gtp),
            ("engine", &valid.document.engine.executable),
        ] {
            refreshed
                .document
                .sources
                .accepted
                .insert(key.into(), identity(&digest_file(path)?)?);
        }
        refreshed.document.sources.accepted.insert(
            "engine_inputs".into(),
            identity(&collect_inputs(&valid.document.engine.input_dir)?)?,
        );
        for (index, scenario) in document.scenarios.iter().enumerate() {
            if scenario.asset_id.is_some() {
                refreshed.refresh_forecast(index)?;
            }
        }
        self.document.sources = refreshed.document.sources;
        self.anchor = refreshed.anchor;
        self.findings.retain(|finding| {
            finding.starts_with("view ")
                || finding.starts_with("invalid visualization")
                || finding.starts_with("saved workspace")
        });
        Ok(valid.forecast)
    }
    pub fn materialize_replacements(&mut self) -> Result<(), String> {
        self.validate_anchor(&self.document)?;
        if !matches!(self.document.schema_version, 1 | 2) {
            return Err("unsupported draft schema version".into());
        }
        if !self.workspace_matches() {
            return Err("saved workspace differs from selected workspace".into());
        }
        if self.document.instructions.replacements.is_some() {
            return Err("draft replacements already supplied".into());
        }
        let observed = self
            .document
            .operation
            .observed
            .as_ref()
            .ok_or("select observations")?;
        let path = observed.path.as_deref().ok_or("select observations")?;
        let ids = observed.replacement_ids.as_deref().unwrap_or(&[]);
        let store = ObservationStore::open_read_only(&super::absolute(&self.workspace, path))
            .map_err(|e| e.to_string())?;
        let values = store.load_replacements(ids).map_err(|e| e.to_string())?;
        self.document.instructions.replacements = Some(
            values
                .into_iter()
                .map(|value| DraftReplacement {
                    draft_id: value.replacement_id.clone(),
                    registered_origin: Some(value.replacement_id.clone()),
                    value,
                })
                .collect(),
        );
        self.document.sources.accepted.remove("observations");
        Ok(())
    }
    pub fn materialize_corrections(&mut self, index: usize) -> Result<(), String> {
        self.validate_anchor(&self.document)?;
        if !matches!(self.document.schema_version, 1 | 2) {
            return Err("unsupported draft schema version".into());
        }
        if !self.workspace_matches() {
            return Err("saved workspace differs from selected workspace".into());
        }
        if self
            .document
            .instructions
            .corrections
            .get(index)
            .and_then(Option::as_ref)
            .is_some()
        {
            return Err("draft corrections already supplied".into());
        }
        let scenario = self
            .document
            .operation
            .scenarios
            .get(index)
            .ok_or("select a scenario")?;
        let asset = scenario
            .asset_id
            .as_deref()
            .ok_or("select a forecast asset")?;
        let path = self
            .document
            .operation
            .forecasts
            .as_ref()
            .and_then(|v| v.catalog.as_deref())
            .ok_or("select forecast catalog")?;
        let store =
            ForecastCatalog::open_read_only(&self.workspace, path).map_err(|e| e.to_string())?;
        let values = store
            .load_corrections(asset, scenario.correction_ids.as_deref().unwrap_or(&[]))
            .map_err(|e| e.to_string())?;
        if self.document.instructions.corrections.len() <= index {
            self.document
                .instructions
                .corrections
                .resize_with(index + 1, || None);
        }
        self.document.instructions.corrections[index] = Some(
            values
                .into_iter()
                .map(|value| DraftCorrection {
                    draft_id: value.correction_id.clone(),
                    registered_origin: Some(value.correction_id.clone()),
                    value,
                })
                .collect(),
        );
        self.document
            .sources
            .accepted
            .remove(&format!("forecast:{index}"));
        Ok(())
    }
    pub fn copy_registered_for_edit(
        &mut self,
        mapping: &RegistrationMapping,
    ) -> Result<(), String> {
        self.validate_anchor(&self.document)?;
        if !matches!(self.document.schema_version, 1 | 2) {
            return Err("unsupported draft schema version".into());
        }
        if !self.workspace_matches() {
            return Err("saved workspace differs from selected workspace".into());
        }
        if let Some(index) = mapping.scenario {
            let items = self
                .document
                .instructions
                .corrections
                .get(index)
                .and_then(Option::as_ref)
                .ok_or("no draft corrections for scenario")?;
            let matches: Vec<_> = items
                .iter()
                .enumerate()
                .filter(|(_, v)| v.draft_id == mapping.draft_id)
                .collect();
            if matches.len() != 1 {
                return Err("unknown or ambiguous draft correction".into());
            }
            let (position, item) = matches[0];
            let mut expected = item.value.clone();
            expected.correction_id = mapping.catalog_id.clone();
            let path = self
                .document
                .operation
                .forecasts
                .as_ref()
                .and_then(|v| v.catalog.as_deref())
                .ok_or("select forecast catalog")?;
            let catalog = ForecastCatalog::open_read_only(&self.workspace, path)
                .map_err(|e| e.to_string())?;
            let values = catalog
                .load_corrections(
                    &expected.asset_id,
                    std::slice::from_ref(&mapping.catalog_id),
                )
                .map_err(|e| e.to_string())?;
            if values[0] != expected {
                return Err("registered correction differs from draft".into());
            }
            self.document.instructions.corrections[index]
                .as_mut()
                .unwrap()[position]
                .registered_origin = Some(mapping.catalog_id.clone());
        } else {
            let items = self
                .document
                .instructions
                .replacements
                .as_ref()
                .ok_or("no draft replacements")?;
            let matches: Vec<_> = items
                .iter()
                .enumerate()
                .filter(|(_, v)| v.draft_id == mapping.draft_id)
                .collect();
            if matches.len() != 1 {
                return Err("unknown or ambiguous draft replacement".into());
            }
            let (position, item) = matches[0];
            let mut expected = item.value.clone();
            expected.replacement_id = mapping.catalog_id.clone();
            let path = self
                .document
                .operation
                .observed
                .as_ref()
                .and_then(|v| v.path.as_deref())
                .ok_or("select observations")?;
            let store = ObservationStore::open_read_only(&super::absolute(&self.workspace, path))
                .map_err(|e| e.to_string())?;
            let values = store
                .load_replacements(std::slice::from_ref(&mapping.catalog_id))
                .map_err(|e| e.to_string())?;
            if values[0] != expected {
                return Err("registered replacement differs from draft".into());
            }
            self.document.instructions.replacements.as_mut().unwrap()[position].registered_origin =
                Some(mapping.catalog_id.clone());
        }
        Ok(())
    }
    pub fn register_selected(&self, selection: &RegistrationSelection) -> RegistrationReport {
        let observations = (|| -> Result<Vec<RegistrationMapping>, String> {
            if !matches!(self.document.schema_version, 1 | 2) {
                return Err("unsupported draft schema version".into());
            }
            if selection.replacements.is_empty() {
                return Ok(Vec::new());
            }
            self.validate_anchor(&self.document)?;
            if !self.workspace_matches() {
                return Err("saved workspace differs from selected workspace".into());
            }
            let items = self
                .document
                .instructions
                .replacements
                .as_ref()
                .ok_or("no draft replacements")?;
            let path = self
                .document
                .operation
                .observed
                .as_ref()
                .and_then(|v| v.path.as_deref())
                .ok_or("select observations")?;
            let mut store =
                ObservationStore::open_writable(&super::absolute(&self.workspace, path))
                    .map_err(|e| e.to_string())?;
            let mut seen = std::collections::HashSet::new();
            let mut values = Vec::new();
            let mut mappings = Vec::new();
            for id in &selection.replacements {
                if !seen.insert(id) {
                    return Err("duplicate selected replacement".into());
                }
                let matches: Vec<_> = items.iter().filter(|v| &v.draft_id == id).collect();
                if matches.len() != 1 {
                    return Err(format!("unknown or ambiguous draft replacement {id}"));
                }
                let item = matches[0];
                let mut catalog_id = registration_id(
                    id,
                    &item.registered_origin,
                    &item.value.replacement_id,
                    &item.value,
                )?;
                let mut value = item.value.clone();
                if let Some(origin) = &item.registered_origin {
                    value.replacement_id.clone_from(origin);
                    if store
                        .load_replacements(std::slice::from_ref(origin))
                        .is_ok_and(|rows| rows.first() == Some(&value))
                    {
                        catalog_id.clone_from(origin);
                    }
                }
                value.replacement_id = catalog_id.clone();
                values.push(value);
                mappings.push(RegistrationMapping {
                    draft_id: id.clone(),
                    scenario: None,
                    catalog_id,
                });
            }
            store
                .register_replacements(values)
                .map_err(|e| e.to_string())?;
            Ok(mappings)
        })();
        let forecasts = (|| -> Result<Vec<RegistrationMapping>, String> {
            if !matches!(self.document.schema_version, 1 | 2) {
                return Err("unsupported draft schema version".into());
            }
            if selection.corrections.is_empty() {
                return Ok(Vec::new());
            }
            self.validate_anchor(&self.document)?;
            if !self.workspace_matches() {
                return Err("saved workspace differs from selected workspace".into());
            }
            let path = self
                .document
                .operation
                .forecasts
                .as_ref()
                .and_then(|v| v.catalog.as_deref())
                .ok_or("select forecast catalog")?;
            let store =
                ForecastCatalog::open_writable(&self.workspace, path).map_err(|e| e.to_string())?;
            let mut seen = std::collections::HashSet::new();
            let mut values = Vec::new();
            let mut mappings = Vec::new();
            for selected in &selection.corrections {
                if !seen.insert((selected.scenario, &selected.draft_id)) {
                    return Err("duplicate selected correction".into());
                }
                let scenario = self
                    .document
                    .operation
                    .scenarios
                    .get(selected.scenario)
                    .ok_or("unknown scenario")?;
                let items = self
                    .document
                    .instructions
                    .corrections
                    .get(selected.scenario)
                    .and_then(Option::as_ref)
                    .ok_or("no draft corrections for scenario")?;
                let matches: Vec<_> = items
                    .iter()
                    .filter(|v| v.draft_id == selected.draft_id)
                    .collect();
                if matches.len() != 1 {
                    return Err(format!(
                        "unknown or ambiguous draft correction {}",
                        selected.draft_id
                    ));
                }
                let item = matches[0];
                if scenario.asset_id.as_deref() != Some(&item.value.asset_id) {
                    return Err("correction belongs to another asset".into());
                }
                let mut catalog_id = registration_id(
                    &selected.draft_id,
                    &item.registered_origin,
                    &item.value.correction_id,
                    &item.value,
                )?;
                let mut value = item.value.clone();
                if let Some(origin) = &item.registered_origin {
                    value.correction_id.clone_from(origin);
                    if store
                        .load_corrections(&value.asset_id, std::slice::from_ref(origin))
                        .is_ok_and(|rows| rows.first() == Some(&value))
                    {
                        catalog_id.clone_from(origin);
                    }
                }
                value.correction_id = catalog_id.clone();
                values.push(value);
                mappings.push(RegistrationMapping {
                    draft_id: selected.draft_id.clone(),
                    scenario: Some(selected.scenario),
                    catalog_id,
                });
            }
            store
                .register_corrections(&values)
                .map_err(|e| e.to_string())?;
            Ok(mappings)
        })();
        fn outcome(result: Result<Vec<RegistrationMapping>, String>) -> RegistrationOutcome {
            match result {
                Ok(mappings) => RegistrationOutcome {
                    mappings,
                    error: None,
                },
                Err(error) => RegistrationOutcome {
                    mappings: Vec::new(),
                    error: Some(error),
                },
            }
        }
        RegistrationReport {
            observations: outcome(observations),
            forecasts: outcome(forecasts),
        }
    }
    pub fn validate(&self, action: DraftAction) -> Vec<DraftFinding> {
        let (name, errors) = match action {
            DraftAction::ObservationPreview => (
                "observation_preview",
                self.preview_observations().err().into_iter().collect(),
            ),
            DraftAction::ForecastPreview { scenario } => (
                "forecast_preview",
                self.preview_forecast(scenario).err().into_iter().collect(),
            ),
            DraftAction::Preparation => ("preparation", self.validate_preparation()),
        };
        errors
            .into_iter()
            .map(|message| DraftFinding {
                action: name.into(),
                message,
            })
            .collect()
    }
    pub fn validate_preparation(&self) -> Vec<String> {
        if !self.workspace_matches() {
            return vec!["saved workspace differs from selected workspace".into()];
        }
        let mut current = self.clone();
        let forecast = match current.refresh_all_inputs() {
            Ok(forecast) => forecast,
            Err(error) => return vec![error],
        };
        if let Err(error) = current.preview_observations() {
            return vec![error];
        }
        let mut corrections = std::collections::HashMap::new();
        for (index, scenario) in current.document.operation.scenarios.iter().enumerate() {
            if scenario.asset_id.is_none() {
                continue;
            }
            let (native, registered) = match current.forecast_inputs(index) {
                Ok(inputs) => inputs,
                Err(error) => return vec![error],
            };
            if let Err(error) =
                crate::analysis::forecast_interval_selection(&native.timeline, &forecast)
            {
                let scenario = scenario.id.as_deref().unwrap_or_default();
                let finding = format!("Scenario {scenario}: {error}");
                return vec![finding];
            }
            let values = current
                .document
                .instructions
                .corrections
                .get(index)
                .and_then(Option::as_ref)
                .map_or(registered, |items| {
                    items.iter().map(|item| item.value.clone()).collect()
                });
            for value in values {
                if let Some(previous) =
                    corrections.insert(value.correction_id.clone(), value.clone())
                    && previous != value
                {
                    return vec!["conflicting correction ID".into()];
                }
            }
        }
        current
            .document
            .sources
            .accepted
            .iter()
            .filter(|(key, value)| self.document.sources.accepted.get(*key) != Some(*value))
            .map(|(key, _)| format!("{key} changed or has not been accepted; refresh inputs"))
            .collect()
    }
    pub fn prepare(
        &self,
        invocation_dir: PathBuf,
        output: PathBuf,
    ) -> Result<PreparationReport, OperationError> {
        self.prepare_controlled(invocation_dir, output, &super::OperationControl::default())
    }
    pub fn prepare_controlled(
        &self,
        invocation_dir: PathBuf,
        output: PathBuf,
        control: &super::OperationControl,
    ) -> Result<PreparationReport, OperationError> {
        self.prepare_selected(invocation_dir, output, false, control)
    }

    pub(super) fn prepare_selected(
        &self,
        invocation_dir: PathBuf,
        output: PathBuf,
        operational: bool,
        control: &super::OperationControl,
    ) -> Result<PreparationReport, OperationError> {
        if !self.workspace_matches() {
            return Err(OperationError::InvalidRequest {
                field: "workspace".into(),
                message: "saved workspace differs from selected workspace".into(),
            });
        }
        let document =
            self.effective_document()
                .map_err(|message| OperationError::InvalidRequest {
                    field: "draft".into(),
                    message,
                })?;
        let replacements = self
            .document
            .instructions
            .replacements
            .as_ref()
            .map(|values| values.iter().map(|v| v.value.clone()).collect());
        let corrections = (0..document.scenarios.len())
            .map(|i| {
                self.document
                    .instructions
                    .corrections
                    .get(i)
                    .and_then(Option::as_ref)
                    .map(|values| values.iter().map(|v| v.value.clone()).collect())
            })
            .collect();
        prepare_typed_checked(
            TypedPreparationRequest {
                workspace: self.workspace.clone(),
                invocation_dir,
                output,
                document,
                replacements,
                corrections,
            },
            Some(self.document.sources.accepted.clone()),
            operational,
            control,
        )
    }
    pub fn generation(&self) -> Result<String, String> {
        self.validate_boundary()?;
        let mut document = self.document.clone();
        document.visualization = None;
        identity(&(
            document,
            self.workspace
                .canonicalize()
                .unwrap_or_else(|_| self.workspace.clone()),
        ))
    }
    pub fn preview_is_current(&self, generation: &str) -> bool {
        self.generation().is_ok_and(|current| current == generation)
    }
    pub fn discard(&mut self) {
        self.document = self.baseline.clone();
        if self.anchor != DraftAnchor::from_operation(&self.document.operation) {
            self.anchor.apply(&mut self.document.operation);
            self.document.sources = DraftSources::default();
        }
    }
    pub fn is_dirty(&self) -> bool {
        self.document.workspace != self.baseline.workspace
            || self.document.schema_version != self.baseline.schema_version
            || self.document.sources != self.baseline.sources
            || self.document.operation != self.baseline.operation
            || self.document.instructions != self.baseline.instructions
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn displayed_coverage_rejects_the_same_timezone_transitions_as_preparation() {
        let mut operation = DraftOperation {
            time_zone: Some("America/New_York".into()),
            reference_time: Some("2026-11-01T00:00:00".into()),
            observed_horizon_days: Some(0),
            forecast_horizon_days: Some(1),
            ..Default::default()
        };
        assert!(operation.coverage().is_err());
        operation.time_zone = Some("Not/AZone".into());
        assert!(operation.coverage().is_err());
        operation.time_zone = Some("UTC".into());
        assert!(operation.coverage().is_ok());
    }
    #[test]
    fn fresh_reference_coverage_and_legacy_conversion_preserve_labels() {
        let root = PathBuf::from("/workspace/synthetic");
        let mut draft = Draft::new_run(
            root.clone(),
            &root,
            None,
            "2026-07-20T00:00:00",
            "America/Sao_Paulo",
        )
        .unwrap();
        {
            let unchanged = !(draft.is_dirty());
            assert!(unchanged);
        };
        draft.document.operation.observed_horizon_days = Some(10);
        draft.document.operation.forecast_horizon_days = Some(5);
        let coverage = draft.document.operation.coverage().unwrap();
        assert_eq!(
            coverage,
            "2026-07-10T00:00:00 → 2026-07-20T00:00:00 · 2026-07-20T01:00:00 → 2026-07-25T01:00:00 · 1 h"
        );
        let mut explicit = serde_json::to_value(&draft.document.operation).unwrap();
        expand_period(&mut explicit).unwrap();
        draft.document.operation = serde_json::from_value(explicit.clone()).unwrap();
        draft.document.schema_version = 1;
        draft.use_day_horizons().unwrap();
        assert_eq!(draft.document.schema_version, 2);
        let mut converted = serde_json::to_value(&draft.document.operation).unwrap();
        expand_period(&mut converted).unwrap();
        assert_eq!(converted, explicit);
        draft.document.operation = serde_json::from_value(explicit).unwrap();
        draft.document.schema_version = 1;
        draft.document.operation.forecast_last_end = Some("2026-07-25T02:00:00".into());
        let original = draft.document.clone();
        assert!(draft.use_day_horizons().is_err());
        assert_eq!(draft.document, original);
        assert!(
            Draft::new_run(
                root.clone(),
                &root,
                None,
                "2026-07-20T00:00:00",
                "Not/AZone"
            )
            .is_err()
        );
    }

    #[test]
    fn anchored_actions_reject_changed_reference_or_timezone() {
        std::fs::create_dir_all("runs").unwrap();
        let dir = tempfile::tempdir_in("runs").unwrap();
        let root = dir.path().canonicalize().unwrap();
        let path = root.join("draft.json");
        let mut draft =
            Draft::new_run(root.clone(), &root, None, "2026-07-20T00:00:00", "UTC").unwrap();
        draft.save(&path, true).unwrap();
        let original = std::fs::read(&path).unwrap();
        for (reference, zone) in [
            (Some("2026-07-21T00:00:00"), Some("UTC")),
            (Some("2026-07-20T00:00:00"), Some("Etc/UTC")),
            (None, Some("UTC")),
        ] {
            let mut opened = Draft::open(root.clone(), &root, &path).unwrap();
            opened.document.operation.reference_time = reference.map(str::to_owned);
            opened.document.operation.time_zone = zone.map(str::to_owned);
            assert!(opened.save(&path, true).unwrap_err().contains("fixed"));
            assert!(opened.refresh_inputs().unwrap_err().contains("fixed"));
            assert!(opened.preview_observations().unwrap_err().contains("fixed"));
            assert!(opened.preview_forecast(0).unwrap_err().contains("fixed"));
            assert!(
                opened
                    .prepare(root.clone(), root.join("prep"))
                    .unwrap_err()
                    .to_string()
                    .contains("fixed")
            );
            let report = opened.register_selected(&RegistrationSelection {
                replacements: vec!["obs".into()],
                corrections: vec![RegistrationCorrectionSelection {
                    scenario: 0,
                    draft_id: "fcst".into(),
                }],
            });
            assert!(report.observations.error.unwrap().contains("fixed"));
            assert!(report.forecasts.error.unwrap().contains("fixed"));
            assert_eq!(std::fs::read(&path).unwrap(), original);
        }
    }

    #[test]
    fn incomplete_draft_can_initialize_explicitly_without_reusing_accepted_sources() {
        let root = PathBuf::from("/workspace/synthetic");
        let mut draft = Draft::new(root.clone(), &root);
        draft
            .document
            .sources
            .accepted
            .insert("forecast:0".into(), "old".into());
        draft.document.operation.reference_time = Some("2026-07-20T00:00:00".into());
        assert!(draft.generation().unwrap_err().contains("explicitly"));
        draft
            .initialize_anchor("2026-07-20T00:00:00", "UTC")
            .unwrap();
        assert!(draft.document.sources.accepted.is_empty());
        assert!(
            draft
                .initialize_anchor("2026-07-21T00:00:00", "UTC")
                .is_err()
        );
        draft.discard();
        assert_eq!(
            draft.document.operation.reference_time.as_deref(),
            Some("2026-07-20T00:00:00")
        );
    }

    #[test]
    fn selected_registration_rolls_back_and_retries_independently() {
        use crate::store::{ObservationBatch, ReplacementKind, Station};
        std::fs::create_dir_all("runs").unwrap();
        let dir = tempfile::tempdir_in("runs").unwrap();
        let root = dir.path().canonicalize().unwrap();
        let obs = root.join("observations.sqlite");
        let mut store = ObservationStore::create(&obs, "UTC").unwrap();
        store
            .write_batch(&ObservationBatch {
                stations: vec![Station {
                    station_id: "s".into(),
                    provider_code: "p".into(),
                    station_code: "s".into(),
                    name: "station".into(),
                    latitude: None,
                    longitude: None,
                }],
                ..Default::default()
            })
            .unwrap();
        let catalog =
            ForecastCatalog::open(&root, Path::new("forecast.sqlite"), Path::new("assets"))
                .unwrap();
        let db = rusqlite::Connection::open(root.join("forecast.sqlite")).unwrap();
        db.execute("INSERT INTO asset VALUES ('asset','source','p','m','p','cycle','source','hash',1,'native','hash',1,1,'{}','UTC','a','b')", []).unwrap();
        drop(catalog);
        let mut draft = Draft::new(root.clone(), &root);
        draft.document.operation.observed = Some(DraftObserved {
            path: Some(obs),
            ..Default::default()
        });
        draft.document.operation.forecasts = Some(DraftForecasts {
            catalog: Some("forecast.sqlite".into()),
        });
        draft.document.operation.scenarios.push(DraftScenario {
            asset_id: Some("asset".into()),
            ..Default::default()
        });
        let replacement = |id: &str, station: &str| DraftReplacement {
            draft_id: id.into(),
            registered_origin: None,
            value: ObservationReplacement {
                replacement_id: id.into(),
                station_id: station.into(),
                variable: "rain".into(),
                kind: ReplacementKind::Point,
                start_at: chrono::NaiveDate::from_ymd_opt(2026, 1, 1)
                    .unwrap()
                    .and_hms_opt(0, 0, 0)
                    .unwrap(),
                end_at: None,
                value: Some(2.0),
                reason: String::new(),
            },
        };
        draft.document.instructions.replacements = Some(vec![
            replacement("one", "s"),
            replacement("bad", "missing"),
            replacement("unselected", "s"),
        ]);
        draft.document.instructions.corrections = [Some(vec![DraftCorrection {
            draft_id: "c".into(),
            registered_origin: None,
            value: ForecastCorrection {
                correction_id: "c".into(),
                asset_id: "asset".into(),
                t0_hour: 0,
                t1_hour: 1,
                shift_lat_pixels: 0.,
                shift_lon_pixels: 0.,
                rotation_deg: 0.,
                multiplication_factor: 1.,
                editor: None,
                reason: String::new(),
                created_at: "2026-01-01T00:00:00Z".into(),
            },
        }])]
        .into();
        let select = RegistrationSelection {
            replacements: vec!["one".into(), "bad".into()],
            corrections: vec![RegistrationCorrectionSelection {
                scenario: 0,
                draft_id: "c".into(),
            }],
        };
        let original = draft.document.instructions.clone();
        let good = draft.document.instructions.corrections[0].as_ref().unwrap()[0]
            .value
            .clone();
        let mut bad = good.clone();
        bad.correction_id = "bad".into();
        bad.multiplication_factor = 0.;
        let catalog = ForecastCatalog::open_writable(&root, Path::new("forecast.sqlite")).unwrap();
        assert!(catalog.register_corrections(&[good, bad]).is_err());
        let count: i64 = db
            .query_row("SELECT count(*) FROM correction", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 0);
        let result = draft.register_selected(&select);
        assert!(result.observations.error.is_some());
        assert!(result.forecasts.error.is_none());
        let count: i64 = rusqlite::Connection::open(root.join("observations.sqlite"))
            .unwrap()
            .query_row("SELECT count(*) FROM observation_replacement", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(count, 0);
        draft.document.instructions.replacements.as_mut().unwrap()[1]
            .value
            .station_id = "s".into();
        let path = root.join("draft.json");
        draft.save(&path, true).unwrap();
        let draft = Draft::open(root.clone(), &root, &path).unwrap();
        let retry = draft.register_selected(&select);
        assert!(retry.succeeded());
        assert!(draft.register_selected(&select).succeeded());
        assert_eq!(
            draft.document.instructions.corrections,
            original.corrections
        );
        let mut scheduled = draft.clone();
        scheduled
            .initialize_anchor("2026-01-01T00:00:00", "UTC")
            .unwrap();
        scheduled
            .document
            .sources
            .accepted
            .insert("observations".into(), "old".into());
        scheduled.save(&path, true).unwrap();
        let original_bytes = std::fs::read(&path).unwrap();
        let before = scheduled.document.clone();
        let slot = scheduled
            .for_rolling_slot("2026-01-02T00:00:00", "UTC")
            .unwrap();
        assert_eq!(slot.document.instructions, before.instructions);
        assert_eq!(
            slot.document.operation.reference_time.as_deref(),
            Some("2026-01-02T00:00:00")
        );
        assert!(slot.document.sources.accepted.is_empty());
        assert!(slot.path.is_none());
        assert_eq!(scheduled.document, before);
        assert_eq!(std::fs::read(&path).unwrap(), original_bytes);
        let count: i64 = rusqlite::Connection::open(root.join("observations.sqlite"))
            .unwrap()
            .query_row("SELECT count(*) FROM observation_replacement", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(count, 2);
        let count: i64 = db
            .query_row("SELECT count(*) FROM correction", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 1);
        let mut new_copy = draft.clone();
        new_copy
            .copy_registered_for_edit(&retry.observations.mappings[0])
            .unwrap();
        new_copy
            .copy_registered_for_edit(&retry.forecasts.mappings[0])
            .unwrap();
        let unchanged_copy = new_copy.register_selected(&select);
        assert_eq!(
            unchanged_copy.observations.mappings[0].catalog_id,
            retry.observations.mappings[0].catalog_id
        );
        assert_eq!(
            unchanged_copy.forecasts.mappings[0].catalog_id,
            retry.forecasts.mappings[0].catalog_id
        );
        new_copy
            .document
            .instructions
            .replacements
            .as_mut()
            .unwrap()[0]
            .value
            .value = Some(5.);
        new_copy.document.instructions.corrections[0]
            .as_mut()
            .unwrap()[0]
            .value
            .multiplication_factor = 5.;
        let new_copy_selection = RegistrationSelection {
            replacements: vec!["one".into()],
            corrections: vec![RegistrationCorrectionSelection {
                scenario: 0,
                draft_id: "c".into(),
            }],
        };
        let new_copy_report = new_copy.register_selected(&new_copy_selection);
        assert!(new_copy_report.succeeded());
        assert_ne!(new_copy_report.observations.mappings[0].catalog_id, "one");
        assert_ne!(new_copy_report.forecasts.mappings[0].catalog_id, "c");
        let copy_path = root.join("new-copy.json");
        new_copy.save(&copy_path, true).unwrap();
        let reopened_copy = Draft::open(root.clone(), &root, &copy_path).unwrap();
        let new_copy_retry = reopened_copy.register_selected(&new_copy_selection);
        assert_eq!(
            new_copy_report.observations.mappings[0].catalog_id,
            new_copy_retry.observations.mappings[0].catalog_id
        );
        assert_eq!(
            new_copy_report.forecasts.mappings[0].catalog_id,
            new_copy_retry.forecasts.mappings[0].catalog_id
        );
        assert!(
            new_copy
                .copy_registered_for_edit(&retry.observations.mappings[0])
                .is_err()
        );
        let mut extra = draft.clone();
        let orphan = extra.document.instructions.corrections[0].clone();
        extra.document.instructions.corrections = vec![None, orphan.clone()];
        extra.document.operation.scenarios[0].correction_ids = Some(vec!["c".into()]);
        extra.materialize_corrections(0).unwrap();
        assert_eq!(extra.document.instructions.corrections.len(), 2);
        assert_eq!(extra.document.instructions.corrections[1], orphan);
        let mut materialized = draft.clone();
        materialized.document.instructions.replacements = None;
        materialized
            .document
            .operation
            .observed
            .as_mut()
            .unwrap()
            .replacement_ids = Some(vec!["bad".into(), "one".into()]);
        materialized.document.instructions.corrections = Vec::new();
        materialized.document.operation.scenarios[0].correction_ids = Some(vec!["c".into()]);
        materialized.materialize_replacements().unwrap();
        materialized.materialize_corrections(0).unwrap();
        let replacements = materialized
            .document
            .instructions
            .replacements
            .as_ref()
            .unwrap();
        assert_eq!(
            replacements
                .iter()
                .map(|v| v.value.replacement_id.as_str())
                .collect::<Vec<_>>(),
            ["bad", "one"]
        );
        assert_eq!(replacements[1].registered_origin.as_deref(), Some("one"));
        assert_eq!(
            materialized.document.instructions.corrections[0]
                .as_ref()
                .unwrap()[0]
                .value,
            original.corrections[0].as_ref().unwrap()[0].value
        );
        let materialized_path = root.join("materialized.json");
        materialized.save(&materialized_path, true).unwrap();
        let materialized = Draft::open(root.clone(), &root, &materialized_path).unwrap();
        let copied = materialized.register_selected(&RegistrationSelection {
            replacements: vec!["one".into()],
            corrections: vec![RegistrationCorrectionSelection {
                scenario: 0,
                draft_id: "c".into(),
            }],
        });
        assert!(copied.succeeded());
        let retry_copy = materialized.register_selected(&RegistrationSelection {
            replacements: vec!["one".into()],
            corrections: vec![RegistrationCorrectionSelection {
                scenario: 0,
                draft_id: "c".into(),
            }],
        });
        assert_eq!(
            copied.observations.mappings[0].catalog_id,
            retry_copy.observations.mappings[0].catalog_id
        );
        assert_eq!(
            copied.forecasts.mappings[0].catalog_id,
            retry_copy.forecasts.mappings[0].catalog_id
        );
        let mut edited_again = materialized.clone();
        edited_again
            .document
            .instructions
            .replacements
            .as_mut()
            .unwrap()[1]
            .value
            .value = Some(7.);
        edited_again.document.instructions.corrections[0]
            .as_mut()
            .unwrap()[0]
            .value
            .multiplication_factor = 3.;
        let edit_selection = RegistrationSelection {
            replacements: vec!["one".into()],
            corrections: vec![RegistrationCorrectionSelection {
                scenario: 0,
                draft_id: "c".into(),
            }],
        };
        let edited_report = edited_again.register_selected(&edit_selection);
        assert!(edited_report.succeeded());
        assert_ne!(
            edited_report.observations.mappings[0].catalog_id,
            copied.observations.mappings[0].catalog_id
        );
        assert_ne!(
            edited_report.forecasts.mappings[0].catalog_id,
            copied.forecasts.mappings[0].catalog_id
        );
        edited_again.save(&materialized_path, true).unwrap();
        edited_again = Draft::open(root.clone(), &root, &materialized_path).unwrap();
        let retry = edited_again.register_selected(&edit_selection);
        assert_eq!(
            edited_report.observations.mappings[0].catalog_id,
            retry.observations.mappings[0].catalog_id
        );
        assert_eq!(
            edited_report.forecasts.mappings[0].catalog_id,
            retry.forecasts.mappings[0].catalog_id
        );
        edited_again
            .document
            .instructions
            .replacements
            .as_mut()
            .unwrap()[1]
            .value
            .value = Some(8.);
        edited_again.document.instructions.corrections[0]
            .as_mut()
            .unwrap()[0]
            .value
            .multiplication_factor = 4.;
        let second_edit = edited_again.register_selected(&edit_selection);
        assert!(second_edit.succeeded());
        assert_ne!(
            edited_report.observations.mappings[0].catalog_id,
            second_edit.observations.mappings[0].catalog_id
        );
        assert_ne!(
            edited_report.forecasts.mappings[0].catalog_id,
            second_edit.forecasts.mappings[0].catalog_id
        );
        let original_correction: f64 = db
            .query_row(
                "SELECT multiplication_factor FROM correction WHERE correction_id='c'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(original_correction, 1.);
        let mut unsupported = edited_again.clone();
        unsupported.document.schema_version = 3;
        let rejected = unsupported.register_selected(&edit_selection);
        assert!(rejected.observations.error.unwrap().contains("schema"));
        assert!(rejected.forecasts.error.unwrap().contains("schema"));
        let mut changed = draft.clone();
        changed.document.instructions.corrections[0]
            .as_mut()
            .unwrap()[0]
            .value
            .multiplication_factor = 2.;
        assert!(
            changed
                .register_selected(&select)
                .forecasts
                .error
                .unwrap()
                .contains("conflicting")
        );
        changed.document.instructions.replacements.as_mut().unwrap()[0].draft_id = "edited".into();
        changed.document.instructions.replacements.as_mut().unwrap()[0].registered_origin =
            Some("one".into());
        changed.document.instructions.replacements.as_mut().unwrap()[0]
            .value
            .value = Some(3.);
        let copied = changed.register_selected(&RegistrationSelection {
            replacements: vec!["edited".into()],
            corrections: vec![],
        });
        assert!(copied.succeeded());
        assert_ne!(copied.observations.mappings[0].catalog_id, "one");
        let values: Vec<f64> = rusqlite::Connection::open(root.join("observations.sqlite"))
            .unwrap()
            .prepare("SELECT value FROM observation_replacement WHERE replacement_id='one'")
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .map(Result::unwrap)
            .collect();
        assert_eq!(values, [2.]);
        changed.document.instructions.corrections[0]
            .as_mut()
            .unwrap()[0]
            .value
            .multiplication_factor = 0.;
        let invalid = RegistrationSelection {
            replacements: Vec::new(),
            corrections: select.corrections,
        };
        assert!(
            changed
                .register_selected(&invalid)
                .forecasts
                .error
                .is_some()
        );
    }
    #[test]
    fn incomplete_roundtrip_discard_and_failed_save_preserve_baseline() {
        std::fs::create_dir_all("runs").unwrap();
        let dir = tempfile::tempdir_in("runs").unwrap();
        let root = dir.path().canonicalize().unwrap();
        let path = root.join("draft.json");
        let mut draft = Draft::new("workspace".into(), &root);
        draft.document.operation.time_zone = Some("UTC".into());
        draft.save(&path, true).unwrap();
        let bytes = std::fs::read(&path).unwrap();
        draft.document.operation.time_zone = None;
        assert!(draft.is_dirty());
        assert!(draft.save(&root.join("missing/draft.json"), true).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
        draft.discard();
        let clean = !draft.is_dirty();
        assert!(clean);
        assert_eq!(draft.document.operation.time_zone.as_deref(), Some("UTC"));
        let reopened = Draft::open("workspace".into(), &root, &path).unwrap();
        assert_eq!(
            reopened.document.operation.time_zone.as_deref(),
            Some("UTC")
        );
        let text = String::from_utf8(bytes)
            .unwrap()
            .replace("\"schema_version\": 2", "\"schema_version\": 3");
        std::fs::write(&path, text).unwrap();
        assert!(Draft::open("workspace".into(), &root, &path).is_err());
    }
    #[test]
    fn missing_saved_locations_remain_available_with_findings() {
        std::fs::create_dir_all("runs").unwrap();
        let dir = tempfile::tempdir_in("runs").unwrap();
        let root = dir.path().canonicalize().unwrap();
        let path = root.join("draft.json");
        for location in ["station:absent", "mini:42", "unknown"] {
            let mut draft = Draft::new(root.clone(), &root);
            draft.document.visualization = Some(DraftVisualization {
                selected_location: Some(location.into()),
                visible_runs: vec!["missing-run".into()],
                ..Default::default()
            });
            draft.save(&path, true).unwrap();
            let reopened = Draft::open(root.clone(), &root, &path).unwrap();
            assert_eq!(
                reopened
                    .document
                    .visualization
                    .as_ref()
                    .unwrap()
                    .selected_location
                    .as_deref(),
                Some(location)
            );
            assert!(reopened.findings.iter().any(
                |f| f.contains("view ") && f.contains(location.split(':').next_back().unwrap())
            ));
            assert!(reopened.findings.iter().any(|f| f.contains("missing-run")));
            let clean = !reopened.is_dirty();
            assert!(clean);
        }
    }
    #[test]
    fn strict_json_view_and_workspace_findings() {
        std::fs::create_dir_all("runs").unwrap();
        let dir = tempfile::tempdir_in("runs").unwrap();
        let root = dir.path().canonicalize().unwrap();
        let path = root.join("draft.json");
        let mut draft = Draft::new("chosen".into(), &root);
        draft.document.operation.grid = Some(DraftGrid {
            bbox: Some([0.0, 0.0, 1.0, 1.0]),
            resolution_degrees: None,
        });
        draft.save(&path, true).unwrap();
        let bytes = std::fs::read_to_string(&path).unwrap();
        let altered = bytes.replace(
            "\"visualization\": null",
            "\"visualization\": {\"map_position\": \"bad\"}",
        );
        std::fs::write(&path, altered).unwrap();
        let opened = Draft::open("chosen".into(), &root, &path).unwrap();
        assert!(opened.document.visualization.is_none());
        assert!(opened.findings.iter().any(|v| v.contains("visualization")));
        let mismatched = Draft::open("other".into(), &root, &path).unwrap();
        assert!(mismatched.preview_observations().is_err());
        let unknown = bytes.replace("\"time_zone\": null", "\"time_zone\": null, \"unknown\": 1");
        std::fs::write(&path, unknown).unwrap();
        assert!(Draft::open("chosen".into(), &root, &path).is_err());
        std::fs::write(&path, bytes).unwrap();
        draft.document.visualization = Some(DraftVisualization {
            map_position: Some([0.0, 0.0, 3.0]),
            ..DraftVisualization::default()
        });
        let clean = !draft.is_dirty();
        assert!(clean);
        draft.save(&path, false).unwrap();
        draft.discard();
        assert!(draft.document.visualization.is_none());
        draft.document.visualization = Some(DraftVisualization {
            map_position: Some([91.0, 0.0, 3.0]),
            ..DraftVisualization::default()
        });
        draft.save(&path, true).unwrap();
        let reopened = Draft::open("chosen".into(), &root, &path).unwrap();
        assert!(reopened.document.visualization.is_none());
        assert!(draft.findings.iter().any(|v| v.contains("visualization")));
        let original = std::fs::read(&path).unwrap();
        draft
            .document
            .operation
            .grid
            .as_mut()
            .unwrap()
            .resolution_degrees = Some(f64::NAN);
        assert!(draft.save(&path, true).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), original);
        draft.discard();
        assert!(
            draft
                .document
                .operation
                .grid
                .unwrap()
                .resolution_degrees
                .is_none()
        );
    }
    #[cfg(unix)]
    #[test]
    fn failed_save_as_preserves_existing_destination_and_baseline() {
        use std::os::unix::fs::PermissionsExt;
        std::fs::create_dir_all("runs").unwrap();
        let dir = tempfile::tempdir_in("runs").unwrap();
        let root = dir.path().canonicalize().unwrap();
        let mut draft = Draft::new("workspace".into(), &root);
        draft.document.operation.time_zone = Some("UTC".into());
        draft.save(Path::new("original.json"), true).unwrap();
        let destination_dir = root.join("readonly");
        std::fs::create_dir(&destination_dir).unwrap();
        let destination = destination_dir.join("draft.json");
        std::fs::write(&destination, b"existing destination").unwrap();
        std::fs::set_permissions(&destination_dir, std::fs::Permissions::from_mode(0o500)).unwrap();
        if std::fs::write(destination_dir.join("probe"), b"probe").is_ok() {
            std::fs::set_permissions(&destination_dir, std::fs::Permissions::from_mode(0o700))
                .unwrap();
            return;
        }
        draft.document.operation.time_zone = Some("America/Sao_Paulo".into());
        let result = draft.save(&destination, true);
        std::fs::set_permissions(&destination_dir, std::fs::Permissions::from_mode(0o700)).unwrap();
        assert!(result.is_err());
        assert_eq!(
            std::fs::read(&destination).unwrap(),
            b"existing destination"
        );
        assert_eq!(
            draft.path.as_deref(),
            Some(root.join("original.json").as_path())
        );
        assert!(draft.is_dirty());
        draft.discard();
        assert_eq!(draft.document.operation.time_zone.as_deref(), Some("UTC"));
    }
}
