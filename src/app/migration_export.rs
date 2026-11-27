use super::{AutomationPostAction, BulletinRequest, HydrographRequest};
use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HydrographBatchItem {
    pub name: String,
    pub request: HydrographRequest,
}

pub fn export_hydrograph_batch(
    items: &[HydrographBatchItem],
    destination: &Path,
) -> Result<Vec<PathBuf>, String> {
    if items.is_empty() {
        return Err("select at least one hydrograph".into());
    }
    let mut names = std::collections::HashSet::new();
    for item in items {
        let name = item.name.as_bytes();
        if name
            .first()
            .is_none_or(|byte| !byte.is_ascii_alphanumeric())
            || name.len() > 64
            || !name
                .iter()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
            || !names.insert(item.name.to_ascii_lowercase())
        {
            return Err(format!(
                "invalid or duplicate hydrograph filename {}",
                item.name
            ));
        }
    }
    crate::store::publish_directory(destination, |stage| {
        let mut artifacts = Vec::new();
        for item in items {
            let report = super::analyze_hydrograph(&item.request)?;
            if report.curves.is_empty() {
                return Err(format!("hydrograph {} has no available series", item.name));
            }
            for extension in ["csv", "png"] {
                let filename = format!("{}.{extension}", item.name);
                super::export_hydrograph(&report, &stage.join(&filename))?;
                artifacts.push(destination.join(filename));
            }
        }
        Ok(artifacts)
    })
}

pub fn publish_automation_artifacts(
    action: AutomationPostAction,
    run: &Path,
    output: &Path,
) -> Result<Vec<PathBuf>, String> {
    match action {
        AutomationPostAction::Report {
            profile,
            observations,
            geometry,
            map_geometry,
            scenario_id,
        } => {
            let report = super::generate_bulletin(&BulletinRequest {
                trial: run.into(),
                scenario_id,
                observations,
                geometry,
                map_geometry,
                profile,
                output: output.into(),
            })?;
            let mut attachments = vec![report.pdf];
            attachments.extend(report.csv);
            Ok(attachments)
        }
        AutomationPostAction::Export => {
            let report = super::export_run(run, None, output)?;
            Ok(vec![report.root.join("manifest.toml")])
        }
    }
}
