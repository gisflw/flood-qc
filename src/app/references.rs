use crate::adapters::{normalize_station_code, read_reference_csvs};
use crate::store::{LevelReference, ObservationStore};
use serde::Serialize;
use std::{
    collections::{BTreeSet, HashMap},
    path::Path,
};

#[derive(Debug, Serialize)]
pub struct ReferenceImportReport {
    pub imported: usize,
    pub unmatched_codes: Vec<String>,
}

pub fn import_level_references(
    database: &Path,
    provider: &str,
    thresholds: &Path,
    history: &Path,
) -> Result<ReferenceImportReport, String> {
    if provider.trim().is_empty() {
        return Err("provider is required".into());
    }
    let parsed = read_reference_csvs(thresholds, history)?;
    let mut store = ObservationStore::open_writable(database).map_err(|e| e.to_string())?;
    let mut stations = HashMap::new();
    for location in store.station_locations().map_err(|e| e.to_string())? {
        if location.station.provider_code == provider {
            let code = normalize_station_code(&location.station.station_code)?;
            if stations
                .insert(code.clone(), location.station.station_id)
                .is_some()
            {
                return Err(format!("ambiguous station code {code}"));
            }
        }
    }
    if stations.is_empty() {
        return Err(format!("unknown provider {provider}"));
    }
    let mut unmatched: BTreeSet<_> = parsed
        .codes
        .into_iter()
        .filter(|code| !stations.contains_key(code))
        .collect();
    let mut references = Vec::new();
    for row in parsed.rows {
        if let Some(station_id) = stations.get(&row.code) {
            references.push(LevelReference {
                station_id: station_id.clone(),
                kind: row.kind,
                date: row.date,
                level_cm: row.level_cm,
            });
        } else {
            unmatched.insert(row.code);
        }
    }
    store
        .replace_level_references(&references)
        .map_err(|e| e.to_string())?;
    Ok(ReferenceImportReport {
        imported: references.len(),
        unmatched_codes: unmatched.into_iter().collect(),
    })
}

/// Explicitly upgrade the observation database for level references.
pub fn upgrade_level_references(database: &Path) -> Result<(), String> {
    let mut store = ObservationStore::open_writable(database).map_err(|e| e.to_string())?;
    store.upgrade_references().map_err(|e| e.to_string())
}
