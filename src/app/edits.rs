use std::path::Path;

use chrono::{NaiveDateTime, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};

use crate::store::{
    ForecastCatalog, ForecastCorrection, ObservationReplacement, ObservationStore, ReplacementKind,
};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ObservationRow {
    station_id: String,
    variable: String,
    kind: ReplacementKind,
    start_at: NaiveDateTime,
    end_at: Option<NaiveDateTime>,
    value: Option<f64>,
    #[serde(default)]
    reason: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ForecastRow {
    t0_hour: i64,
    t1_hour: i64,
    shift_lat_pixels: f64,
    shift_lon_pixels: f64,
    rotation_deg: f64,
    multiplication_factor: f64,
    editor: Option<String>,
    #[serde(default)]
    reason: String,
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
const FORECAST_HEADERS: &[&str] = &[
    "t0_hour",
    "t1_hour",
    "shift_lat_pixels",
    "shift_lon_pixels",
    "rotation_deg",
    "multiplication_factor",
    "editor",
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

fn forecast_asset(catalog: &ForecastCatalog, cycle: &str) -> Result<String, String> {
    let (provider, time) = cycle.split_once('@').ok_or("Use provider@UTC-cycle")?;
    let time = chrono::DateTime::parse_from_rfc3339(time)
        .map_err(|e| e.to_string())?
        .to_utc()
        .to_rfc3339_opts(SecondsFormat::Secs, true);
    let choices: Vec<_> = catalog
        .asset_choices()
        .map_err(|e| e.to_string())?
        .into_iter()
        .filter(|asset| asset.provider == provider && asset.cycle_utc == time)
        .collect();
    let [asset] = choices.as_slice() else {
        return Err(format!("{provider}: expected one forecast for {time}"));
    };
    Ok(asset.asset_id.clone())
}

/// Replace the saved edit list for observations or for one concrete forecast cycle.
pub fn import_workspace_edits(
    workspace: &Path,
    input: &Path,
    forecast: Option<&str>,
) -> Result<usize, String> {
    let workspace = workspace.canonicalize().map_err(|e| e.to_string())?;
    let path = super::operation_database(&workspace);
    let _lock = super::operation::lock_workspace_job(&workspace).map_err(|e| e.to_string())?;
    if let Some(cycle) = forecast {
        let catalog =
            ForecastCatalog::open_writable(&workspace, &path).map_err(|e| e.to_string())?;
        let asset = forecast_asset(&catalog, cycle)?;
        let now = Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);
        let rows: Vec<_> = parse::<ForecastRow>(input, FORECAST_HEADERS)?
            .into_iter()
            .enumerate()
            .map(|(row, value)| ForecastCorrection {
                correction_id: format!("csv:{asset}:{}", row + 1),
                asset_id: asset.clone(),
                t0_hour: value.t0_hour,
                t1_hour: value.t1_hour,
                shift_lat_pixels: value.shift_lat_pixels,
                shift_lon_pixels: value.shift_lon_pixels,
                rotation_deg: value.rotation_deg,
                multiplication_factor: value.multiplication_factor,
                editor: value.editor,
                reason: value.reason,
                created_at: now.clone(),
            })
            .collect();
        for (index, row) in rows.iter().enumerate() {
            crate::store::check_correction(row)
                .map_err(|error| format!("{} row {}: {error}", input.display(), index + 2))?;
        }
        catalog
            .replace_corrections(&asset, &rows)
            .map_err(|e| e.to_string())?;
        Ok(rows.len())
    } else {
        let mut store = ObservationStore::open_writable(&path).map_err(|e| e.to_string())?;
        let rows: Vec<_> = parse::<ObservationRow>(input, OBSERVATION_HEADERS)?
            .into_iter()
            .enumerate()
            .map(|(row, value)| ObservationReplacement {
                replacement_id: format!("csv-row-{}", row + 2),
                station_id: value.station_id,
                variable: value.variable,
                kind: value.kind,
                start_at: value.start_at,
                end_at: value.end_at,
                value: value.value,
                reason: value.reason,
            })
            .collect();
        let count = rows.len();
        store
            .replace_replacements(rows)
            .map_err(|e| format!("{}: {e}", input.display()))?;
        Ok(count)
    }
}

pub fn export_workspace_edits(
    workspace: &Path,
    output: &Path,
    forecast: Option<&str>,
) -> Result<usize, String> {
    let workspace = workspace.canonicalize().map_err(|e| e.to_string())?;
    let path = super::operation_database(&workspace);
    if let Some(cycle) = forecast {
        let catalog =
            ForecastCatalog::open_read_only(&workspace, &path).map_err(|e| e.to_string())?;
        let asset = forecast_asset(&catalog, cycle)?;
        let ids: Vec<_> = catalog
            .correction_choices()
            .map_err(|e| e.to_string())?
            .into_iter()
            .filter(|(_, owner)| owner == &asset)
            .map(|(id, _)| id)
            .collect();
        let rows = catalog
            .load_corrections(&asset, &ids)
            .map_err(|e| e.to_string())?
            .into_iter()
            .map(|value| ForecastRow {
                t0_hour: value.t0_hour,
                t1_hour: value.t1_hour,
                shift_lat_pixels: value.shift_lat_pixels,
                shift_lon_pixels: value.shift_lon_pixels,
                rotation_deg: value.rotation_deg,
                multiplication_factor: value.multiplication_factor,
                editor: value.editor,
                reason: value.reason,
            })
            .collect();
        write(output, FORECAST_HEADERS, rows)
    } else {
        let store = ObservationStore::open_read_only(&path).map_err(|e| e.to_string())?;
        let ids = store.replacement_choices().map_err(|e| e.to_string())?;
        let rows = store
            .load_replacements(&ids)
            .map_err(|e| e.to_string())?
            .into_iter()
            .map(|value| ObservationRow {
                station_id: value.station_id,
                variable: value.variable,
                kind: value.kind,
                start_at: value.start_at,
                end_at: value.end_at,
                value: value.value,
                reason: value.reason,
            })
            .collect();
        write(output, OBSERVATION_HEADERS, rows)
    }
}
