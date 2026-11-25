use std::path::Path;

use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

use crate::store::{ObservationReplacement, ObservationStore, ReplacementKind};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObservationEdit {
    pub station_id: String,
    pub variable: String,
    pub kind: ReplacementKind,
    pub start_at: NaiveDateTime,
    pub end_at: Option<NaiveDateTime>,
    pub value: Option<f64>,
    #[serde(default)]
    pub reason: String,
}

const OBSERVATION_HEADERS: &[&str] = &[
    "station_id",
    "variable",
    "kind",
    "start_at",
    "end_at",
    "value",
    "reason",
];
fn parse<T: serde::de::DeserializeOwned>(input: &Path, columns: &[&str]) -> Result<Vec<T>, String> {
    let describe = |error: csv::Error| format!("{}: {error}", input.display());
    let mut reader = csv::Reader::from_path(input).map_err(describe)?;
    let headers = reader.headers().map_err(describe)?;
    if headers.iter().ne(columns.iter().copied()) {
        return Err(format!(
            "{}: expected CSV header {}",
            input.display(),
            columns.join(",")
        ));
    }
    reader
        .deserialize()
        .map(|row| row.map_err(describe))
        .collect()
}

fn write<T: Serialize>(output: &Path, columns: &[&str], rows: Vec<T>) -> Result<usize, String> {
    let count = rows.len();
    let mut writer = csv::WriterBuilder::new()
        .has_headers(false)
        .from_writer(Vec::new());
    writer.write_record(columns).map_err(|e| e.to_string())?;
    for row in rows {
        writer.serialize(row).map_err(|e| e.to_string())?;
    }
    let bytes = writer.into_inner().map_err(|e| e.to_string())?;
    crate::store::write_new_atomic(output, &bytes)?;
    Ok(count)
}

/// Replace the workspace's ordered observation edits atomically.
pub fn import_workspace_edits(workspace: &Path, input: &Path) -> Result<usize, String> {
    let rows = parse(input, OBSERVATION_HEADERS)?;
    save_workspace_observation_edits(workspace, rows)
        .map_err(|error| format!("{}: {error}", input.display()))
}

pub fn save_workspace_observation_edits(
    workspace: &Path,
    rows: Vec<ObservationEdit>,
) -> Result<usize, String> {
    let workspace = workspace
        .canonicalize()
        .map_err(|error| error.to_string())?;
    let path = super::observation_assets(&workspace);
    let _lock =
        super::operation::lock_workspace_job(&workspace).map_err(|error| error.to_string())?;
    let mut store = ObservationStore::open_writable(&path).map_err(|error| error.to_string())?;
    let rows: Vec<_> = rows
        .into_iter()
        .enumerate()
        .map(|(index, row)| ObservationReplacement {
            replacement_id: format!("edit-{}", index + 1),
            station_id: row.station_id,
            variable: row.variable,
            kind: row.kind,
            start_at: row.start_at,
            end_at: row.end_at,
            value: row.value,
            reason: row.reason,
        })
        .collect();
    let count = rows.len();
    store
        .replace_replacements(rows)
        .map_err(|error| error.to_string())?;
    Ok(count)
}

pub fn workspace_observation_edits(workspace: &Path) -> Result<Vec<ObservationEdit>, String> {
    let path = super::observation_assets(workspace);
    let store = ObservationStore::open_read_only(&path).map_err(|error| error.to_string())?;
    let ids = store
        .replacement_choices()
        .map_err(|error| error.to_string())?;
    let rows = store
        .load_replacements(&ids)
        .map_err(|error| error.to_string())?
        .into_iter()
        .map(|row| ObservationEdit {
            station_id: row.station_id,
            variable: row.variable,
            kind: row.kind,
            start_at: row.start_at,
            end_at: row.end_at,
            value: row.value,
            reason: row.reason,
        })
        .collect();
    Ok(rows)
}

pub fn export_workspace_edits(workspace: &Path, output: &Path) -> Result<usize, String> {
    write(
        output,
        OBSERVATION_HEADERS,
        workspace_observation_edits(workspace)?,
    )
}
