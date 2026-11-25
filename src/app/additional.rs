use super::request::{
    CalibrationDocument, ForcingDocument, PreparationDocument, RoutingMode, ValidatedRequest,
};
use crate::store::{
    IntervalAxis, ObservationInputs, ObservationReplacement, ObservationSample, ObservationStore,
    ObservationValue, PreparedAdditionalForcing, ReservoirSchedules, WindObservations, digest_file,
};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy)]
pub enum CanonicalForcingKind {
    Reservoirs,
    Wind,
}

pub fn save_canonical_forcing(
    source: &Path,
    destination: &Path,
    kind: CanonicalForcingKind,
) -> Result<(), String> {
    let bytes = fs::read(source).map_err(|error| error.to_string())?;
    match kind {
        CanonicalForcingKind::Reservoirs => parse_reservoirs(&bytes).map(|_| ())?,
        CanonicalForcingKind::Wind => parse_wind(&bytes).map(|_| ())?,
    }
    if destination.try_exists().map_err(|e| e.to_string())? {
        if digest_file(destination)?
            == (format!("{:x}", Sha256::digest(&bytes)), bytes.len() as u64)
        {
            return Ok(());
        }
        return Err(format!(
            "forcing destination already exists: {}",
            destination.display()
        ));
    }
    crate::store::write_new_atomic(destination, &bytes)
}

fn parse_reservoirs(bytes: &[u8]) -> Result<ReservoirSchedules, String> {
    let data: ReservoirSchedules = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    data.validate().map_err(|e| e.to_string())?;
    Ok(data)
}

fn parse_wind(bytes: &[u8]) -> Result<WindObservations, String> {
    let data: WindObservations = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    data.validate().map_err(|e| e.to_string())?;
    Ok(data)
}

fn flow_ids(doc: Option<&ForcingDocument>) -> Vec<&str> {
    doc.into_iter()
        .flat_map(|doc| {
            doc.mappings
                .qobs
                .iter()
                .map(|item| item.series_id.as_str())
                .chain(
                    doc.mappings
                        .reservoirs
                        .iter()
                        .filter_map(|item| item.observed_series_id.as_deref()),
                )
        })
        .collect()
}

pub(super) fn extra_stations(
    store: &ObservationStore,
    doc: &PreparationDocument,
) -> Result<Vec<String>, String> {
    let mut stations = doc.levels.station_ids.clone();
    let flow = flow_ids(doc.forcing.as_ref());
    if !flow.is_empty() {
        let source = store.series_choices().map_err(|e| e.to_string())?;
        for id in flow {
            let (_, station, _) = source
                .iter()
                .find(|(series, _, variable)| series == id && variable == "flow")
                .ok_or_else(|| format!("unknown observed flow series {id}"))?;
            if !stations.contains(station) {
                stations.push(station.clone());
            }
        }
    }
    Ok(stations)
}

pub(super) fn validate_engine(doc: &PreparationDocument, sha256: &str) -> Result<(), String> {
    if doc
        .hot_start
        .as_ref()
        .is_some_and(|state| state.use_bundle.is_some())
    {
        return Err("selected MGB build has no qualified hot-start reader".into());
    }
    if doc.routing == Some(RoutingMode::MuskingumCunge) {
        return Err("selected MGB build has no qualified Muskingum–Cunge routing mode".into());
    }
    let needs_qualified_engine = doc.routing.is_some()
        || doc
            .hot_start
            .as_ref()
            .is_some_and(|state| state.save_at.is_some())
        || doc
            .forcing
            .as_ref()
            .is_some_and(|forcing| !forcing.mappings.is_empty());
    if needs_qualified_engine && sha256 != crate::model::QUALIFIED_RS_ENGINE_SHA256 {
        return Err("selected MGB executable is not qualified for explicit routing, hot-start capture or generated forcing".into());
    }
    Ok(())
}

pub(super) fn check_engine(doc: &PreparationDocument, executable: &Path) -> Result<(), String> {
    if doc.routing.is_none() && doc.hot_start.is_none() && doc.forcing.is_none() {
        return Ok(());
    }
    validate_engine(doc, &digest_file(executable)?.0)
}

pub(super) fn soil_identity(
    selection: Option<&CalibrationDocument>,
) -> Result<Option<String>, String> {
    selection
        .and_then(|selection| selection.soil.as_ref())
        .map(|path| digest_file(path).and_then(|value| super::draft::identity(&value)))
        .transpose()
}

pub(super) struct AdditionalInputs {
    pub prepared: PreparedAdditionalForcing,
    pub accepted: String,
    pub raw_files: Vec<(&'static str, Vec<u8>)>,
}

pub(super) fn capture(
    request: &ValidatedRequest,
    observations: &ObservationInputs,
    registered: &[ObservationReplacement],
) -> Result<Option<AdditionalInputs>, String> {
    let Some(doc) = request
        .document
        .forcing
        .as_ref()
        .filter(|doc| !doc.mappings.is_empty())
    else {
        return Ok(None);
    };
    let read_bytes = |path: &Option<PathBuf>| -> Result<Option<Vec<u8>>, String> {
        path.as_deref()
            .map(|path| fs::read(path).map_err(|error| format!("{}: {error}", path.display())))
            .transpose()
    };
    let schedules = read_bytes(&doc.reservoir_schedules)?;
    let wind = read_bytes(&doc.wind_observations)?;
    let reservoirs = schedules.as_deref().map(parse_reservoirs).transpose()?;
    let wind_values = wind.as_deref().map(parse_wind).transpose()?;
    let selected_ids = flow_ids(Some(doc));
    let mut raw = observations.clone();
    raw.series
        .retain(|item| selected_ids.contains(&item.series_id.as_str()));
    raw.samples
        .retain(|item| selected_ids.contains(&item.series_id.as_str()));
    raw.stations.retain(|station| {
        raw.series
            .iter()
            .any(|item| item.station_id == station.station_id)
    });
    raw.replacements = registered
        .iter()
        .filter(|item| item.variable == "flow")
        .cloned()
        .collect();
    let accepted = super::draft::identity(&(
        &raw,
        schedules
            .as_deref()
            .map(|bytes| format!("{:x}", Sha256::digest(bytes))),
        wind.as_deref()
            .map(|bytes| format!("{:x}", Sha256::digest(bytes))),
    ))?;
    let mut hourly = ObservationInputs {
        series: raw.series.clone(),
        ..Default::default()
    };
    for series in raw.series {
        let selected = ObservationInputs {
            stations: raw
                .stations
                .iter()
                .filter(|station| station.station_id == series.station_id)
                .cloned()
                .collect(),
            samples: raw
                .samples
                .iter()
                .filter(|sample| sample.series_id == series.series_id)
                .cloned()
                .collect(),
            replacements: observations
                .replacements
                .iter()
                .filter(|item| item.station_id == series.station_id && item.variable == "flow")
                .cloned()
                .collect(),
            series: vec![series],
            excluded_rain: Default::default(),
        };
        let points = crate::analysis::effective_flow_observations(&selected, &request.historical)
            .map_err(|e| e.to_string())?;
        hourly
            .samples
            .extend(points.into_iter().map(|point| ObservationSample {
                series_id: point.series_id,
                observed_at: point.observed_at,
                value: point.effective_value.map(ObservationValue::FlowM3s),
            }));
    }
    let mut timeline: IntervalAxis = request.historical.clone();
    timeline
        .bounds
        .extend(request.forecast.bounds.iter().cloned());
    let prepared = crate::analysis::prepare_additional_forcing(
        &timeline,
        &doc.mappings,
        &hourly,
        reservoirs.as_ref(),
        wind_values.as_ref(),
    )
    .map_err(|e| e.to_string())?;
    let raw_files = [
        ("reservoir-schedules.json", schedules),
        ("wind-observations.json", wind),
    ]
    .into_iter()
    .filter_map(|(name, value)| value.map(|bytes| (name, bytes)))
    .collect();
    Ok(Some(AdditionalInputs {
        prepared,
        accepted,
        raw_files,
    }))
}
