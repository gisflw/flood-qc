use super::DraftCommand;
use std::path::PathBuf;

pub(super) fn draft_action(
    command: DraftCommand,
    invocation: &std::path::Path,
) -> Result<serde_json::Value, String> {
    fn value<T: serde::Serialize>(data: T) -> Result<serde_json::Value, String> {
        serde_json::to_value(data).map_err(|e| e.to_string())
    }
    fn path(invocation: &std::path::Path, path: &std::path::Path) -> PathBuf {
        if path.is_absolute() {
            path.into()
        } else {
            invocation.join(path)
        }
    }
    match command {
        DraftCommand::New {
            workspace,
            output,
            reference,
            time_zone,
        } => {
            let mut draft = mgb_ops::Draft::new(workspace, invocation);
            if let (Some(reference), Some(zone)) = (reference, time_zone) {
                draft.initialize_anchor(&reference, &zone)?;
            }
            draft.save(&path(invocation, &output), true)?;
            value(draft.document)
        }
        DraftCommand::Open { workspace, draft } => {
            let draft = mgb_ops::Draft::open(workspace, invocation, &draft)?;
            let mut result = serde_json::Map::new();
            result.insert("document".into(), value(draft.document)?);
            result.insert("findings".into(), value(draft.findings)?);
            Ok(result.into())
        }
        DraftCommand::Save {
            workspace,
            draft,
            output,
            omit_visualization,
        } => {
            let mut current = mgb_ops::Draft::open(workspace, invocation, &draft)?;
            current.save(&output, !omit_visualization)?;
            let saved = mgb_ops::Draft::open(current.workspace.clone(), invocation, &output)?;
            value(saved.document)
        }
        DraftCommand::ImportToml {
            workspace,
            request,
            output,
        } => {
            let mut draft = mgb_ops::Draft::import_toml(workspace, invocation, &request)?;
            draft.save(&path(invocation, &output), true)?;
            value(draft.document)
        }
        DraftCommand::ImportManifest {
            workspace,
            manifest,
            output,
        } => {
            let mut draft = mgb_ops::Draft::import_manifest(workspace, invocation, &manifest)?;
            draft.save(&path(invocation, &output), true)?;
            value(draft.document)
        }
        DraftCommand::Edit {
            workspace,
            draft,
            document,
        } => {
            let mut replacement = mgb_ops::Draft::open(workspace.clone(), invocation, &document)?;
            if replacement
                .findings
                .iter()
                .any(|v| v == "saved workspace differs from selected workspace")
            {
                return Err("saved workspace differs from selected workspace".into());
            }
            let mut current = mgb_ops::Draft::open(workspace, invocation, &draft)?;
            let existing = &current.document.operation;
            let proposed = &replacement.document.operation;
            if (existing.reference_time.is_none() || existing.time_zone.is_none())
                && let (Some(reference), Some(zone)) =
                    (&proposed.reference_time, &proposed.time_zone)
            {
                current.initialize_anchor(reference, zone)?;
                replacement.document.sources = Default::default();
            }
            current.document = replacement.document;
            current.save(&path(invocation, &draft), true)?;
            value(current.document)
        }
        DraftCommand::Validate {
            workspace,
            draft,
            action,
            scenario,
        } => {
            let draft = mgb_ops::Draft::open(workspace, invocation, &draft)?;
            let action = match action.as_str() {
                "preparation" => mgb_ops::DraftAction::Preparation,
                "observations" => mgb_ops::DraftAction::ObservationPreview,
                "forecast" => mgb_ops::DraftAction::ForecastPreview {
                    scenario: scenario.ok_or("forecast validation requires --scenario")?,
                },
                _ => return Err("action must be preparation, observations or forecast".into()),
            };
            value(draft.validate(action))
        }
        DraftCommand::Refresh {
            workspace,
            draft,
            source,
            scenario,
        } => {
            let mut current = mgb_ops::Draft::open(workspace, invocation, &draft)?;
            match source.as_str() {
                "all" => current.refresh_inputs()?,
                "observations" => current.refresh_observations()?,
                "forecast" => current
                    .refresh_forecast(scenario.ok_or("forecast refresh requires --scenario")?)?,
                _ => return Err("source must be all, observations or forecast".into()),
            }
            current.save(&path(invocation, &draft), true)?;
            value(current.document)
        }
        DraftCommand::PreviewObservations { workspace, draft } => {
            value(mgb_ops::Draft::open(workspace, invocation, &draft)?.preview_observations()?)
        }
        DraftCommand::PreviewForecast {
            workspace,
            draft,
            scenario,
        } => {
            value(mgb_ops::Draft::open(workspace, invocation, &draft)?.preview_forecast(scenario)?)
        }
        DraftCommand::Materialize {
            workspace,
            draft,
            source,
            scenario,
        } => {
            let mut current = mgb_ops::Draft::open(workspace, invocation, &draft)?;
            match source.as_str() {
                "observations" => current.materialize_replacements()?,
                "forecast" => current.materialize_corrections(
                    scenario.ok_or("forecast materialization requires --scenario")?,
                )?,
                _ => return Err("source must be observations or forecast".into()),
            }
            current.save(&path(invocation, &draft), true)?;
            value(current.document)
        }
        DraftCommand::CopyRegisteredForEdit {
            workspace,
            draft,
            mapping,
        } => {
            let mut current = mgb_ops::Draft::open(workspace, invocation, &draft)?;
            let mapping: mgb_ops::RegistrationMapping = serde_json::from_slice(
                &std::fs::read(path(invocation, &mapping)).map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())?;
            current.copy_registered_for_edit(&mapping)?;
            current.save(&path(invocation, &draft), true)?;
            value(current.document)
        }
        DraftCommand::Register {
            workspace,
            draft,
            selection,
        } => {
            let current = mgb_ops::Draft::open(workspace, invocation, &draft)?;
            let selection: mgb_ops::RegistrationSelection = serde_json::from_slice(
                &std::fs::read(path(invocation, &selection)).map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())?;
            value(current.register_selected(&selection))
        }
        DraftCommand::Run { .. } | DraftCommand::Prepare { .. } => {
            unreachable!("handled as controlled operations")
        }
    }
}
