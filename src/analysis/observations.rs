use crate::store::{
    IntervalAxis, ObservationError, ObservationInputs, ObservationSeries, ObservationValue,
    ReplacementKind,
};
use chrono::{Duration, NaiveDateTime, Timelike};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};

fn error(message: impl Into<String>) -> ObservationError {
    ObservationError(message.into())
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SelectedObservation {
    pub station_id: String,
    pub series_id: String,
    pub observed_at: NaiveDateTime,
    pub longitude: f64,
    pub latitude: f64,
    pub value: Option<f64>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum AppliedReplacement {
    None,
    Null,
    Value(f64),
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EffectiveObservation {
    pub station_id: String,
    pub series_id: String,
    pub observed_at: NaiveDateTime,
    pub longitude: f64,
    pub latitude: f64,
    pub raw_present: bool,
    pub raw_value: Option<f64>,
    pub effective_value: Option<f64>,
    pub replacement: AppliedReplacement,
    pub applied_instruction_ids: Vec<String>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrecipitationQcPolicy {
    pub high_threshold_mm: f64,
    pub sequence_lower_mm: f64,
    pub sequence_upper_mm: f64,
    pub sequence_min_length: usize,
}
impl Default for PrecipitationQcPolicy {
    fn default() -> Self {
        Self {
            high_threshold_mm: 50.0,
            sequence_lower_mm: 0.0,
            sequence_upper_mm: 1.0,
            sequence_min_length: 5,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PrecipitationQcFinding {
    pub station_id: String,
    pub observed_at: NaiveDateTime,
    pub value: f64,
    pub match_type: String,
    pub sequence_id: Option<String>,
}

pub fn select_observation_series(
    candidates: &[ObservationSeries],
    explicit_series_ids: &[String],
) -> Result<Vec<ObservationSeries>, ObservationError> {
    let mut explicit = HashSet::new();
    for id in explicit_series_ids {
        if !explicit.insert(id.as_str()) {
            return Err(error(format!("duplicate explicit series {id}")));
        }
        if !candidates
            .iter()
            .any(|s| &s.series_id == id && s.variable == "rain")
        {
            return Err(error(format!("unknown rain series {id}")));
        }
    }
    let mut grouped: BTreeMap<&str, Vec<&ObservationSeries>> = BTreeMap::new();
    for s in candidates.iter().filter(|s| s.variable == "rain") {
        if !matches!(
            (s.state.as_str(), s.unit.as_str(), s.interval_seconds),
            ("approved" | "curated" | "raw", "mm", Some(3600))
        ) {
            return Err(error(format!(
                "series {}: invalid rainfall metadata",
                s.series_id
            )));
        }
        grouped.entry(&s.station_id).or_default().push(s);
    }
    let mut result = Vec::new();
    for (station, mut series) in grouped {
        let overrides: Vec<_> = series
            .iter()
            .filter(|s| explicit.contains(s.series_id.as_str()))
            .collect();
        if overrides.len() > 1 {
            return Err(error(format!(
                "station {station}: multiple explicit series"
            )));
        }
        if let Some(s) = overrides.first() {
            result.push((***s).clone());
            continue;
        }
        let rank = |s: &ObservationSeries| match s.state.as_str() {
            "approved" => 0,
            "curated" => 1,
            _ => 2,
        };
        series.sort_by(|a, b| {
            rank(a)
                .cmp(&rank(b))
                .then_with(|| b.created_at.cmp(&a.created_at))
                .then_with(|| a.series_id.cmp(&b.series_id))
        });
        if series.len() > 1
            && rank(series[0]) == rank(series[1])
            && series[0].created_at == series[1].created_at
        {
            return Err(error(format!(
                "station {station}: ambiguous series {} and {}",
                series[0].series_id, series[1].series_id
            )));
        }
        result.push(series[0].clone());
    }
    Ok(result)
}

pub fn detect_suspect_precipitation(
    samples: &[SelectedObservation],
    policy: &PrecipitationQcPolicy,
) -> Result<Vec<PrecipitationQcFinding>, ObservationError> {
    if !policy.high_threshold_mm.is_finite()
        || !policy.sequence_lower_mm.is_finite()
        || !policy.sequence_upper_mm.is_finite()
        || policy.high_threshold_mm < 0.0
        || policy.sequence_lower_mm < 0.0
        || policy.sequence_lower_mm >= policy.sequence_upper_mm
        || policy.sequence_min_length == 0
    {
        return Err(error("invalid precipitation QC policy"));
    }
    let mut sorted: Vec<_> = samples.iter().collect();
    sorted.sort_by(|a, b| {
        a.station_id
            .cmp(&b.station_id)
            .then(a.observed_at.cmp(&b.observed_at))
    });
    let mut findings = Vec::new();
    let mut run: Vec<&SelectedObservation> = Vec::new();
    let mut ordinal = 0;
    let mut previous_station = "";
    fn flush(
        run: &mut Vec<&SelectedObservation>,
        ordinal: &mut usize,
        min: usize,
        out: &mut Vec<PrecipitationQcFinding>,
    ) {
        if run.len() >= min {
            *ordinal += 1;
            let id = format!(
                "{}:{}:{}",
                run[0].station_id,
                run[0].observed_at.format("%Y-%m-%dT%H:%M:%S"),
                ordinal
            );
            for row in run.iter() {
                out.push(PrecipitationQcFinding {
                    station_id: row.station_id.clone(),
                    observed_at: row.observed_at,
                    value: row.value.unwrap(),
                    match_type: "sequence".into(),
                    sequence_id: Some(id.clone()),
                });
            }
        }
        run.clear();
    }
    for s in sorted {
        if let Some(v) = s.value {
            if !v.is_finite() {
                return Err(error(format!(
                    "station {} {}: nonfinite rainfall",
                    s.station_id, s.observed_at
                )));
            }
            if v > policy.high_threshold_mm {
                findings.push(PrecipitationQcFinding {
                    station_id: s.station_id.clone(),
                    observed_at: s.observed_at,
                    value: v,
                    match_type: "threshold".into(),
                    sequence_id: None,
                });
            }
        }
        if s.station_id != previous_station {
            flush(
                &mut run,
                &mut ordinal,
                policy.sequence_min_length,
                &mut findings,
            );
            ordinal = 0;
            previous_station = &s.station_id;
        }
        let qualifies = s
            .value
            .is_some_and(|v| v > policy.sequence_lower_mm && v < policy.sequence_upper_mm);
        if !qualifies
            || run
                .last()
                .is_some_and(|last| s.observed_at - last.observed_at != Duration::hours(1))
        {
            flush(
                &mut run,
                &mut ordinal,
                policy.sequence_min_length,
                &mut findings,
            );
        }
        if qualifies {
            run.push(s);
        }
    }
    flush(
        &mut run,
        &mut ordinal,
        policy.sequence_min_length,
        &mut findings,
    );
    findings.sort_by(|a, b| {
        a.station_id
            .cmp(&b.station_id)
            .then(a.observed_at.cmp(&b.observed_at))
            .then(a.match_type.cmp(&b.match_type))
    });
    Ok(findings)
}

pub fn effective_observations(
    inputs: &ObservationInputs,
    timeline: &IntervalAxis,
) -> Result<Vec<EffectiveObservation>, ObservationError> {
    timeline.validate().map_err(|e| error(e.to_string()))?;
    let mut stations = HashMap::new();
    for station in &inputs.stations {
        if station.station_id.trim().is_empty()
            || stations
                .insert(station.station_id.as_str(), station)
                .is_some()
        {
            return Err(error(format!(
                "duplicate or empty station {}",
                station.station_id
            )));
        }
    }
    let mut series_by_id = HashMap::new();
    for series in &inputs.series {
        if series.series_id.trim().is_empty()
            || !stations.contains_key(series.station_id.as_str())
            || series_by_id
                .insert(series.series_id.as_str(), series)
                .is_some()
        {
            return Err(error(format!(
                "invalid or duplicate series {}",
                series.series_id
            )));
        }
    }
    let mut sample_keys = HashSet::new();
    for sample in &inputs.samples {
        let series = series_by_id
            .get(sample.series_id.as_str())
            .ok_or_else(|| error(format!("unknown sample series {}", sample.series_id)))?;
        if !sample_keys.insert((sample.series_id.as_str(), sample.observed_at)) {
            return Err(error(format!(
                "duplicate sample {} {}",
                sample.series_id, sample.observed_at
            )));
        }
        match (series.variable.as_str(), sample.value) {
            ("rain", None)
            | ("flow", None)
            | ("level", None)
            | ("level", Some(ObservationValue::LevelCm(_))) => (),
            ("rain", Some(ObservationValue::RainMm(v)))
            | ("flow", Some(ObservationValue::FlowM3s(v)))
                if v.is_finite() => {}
            _ => {
                return Err(error(format!(
                    "invalid sample {} {}",
                    sample.series_id, sample.observed_at
                )));
            }
        }
    }
    let selected = select_observation_series(&inputs.series, &[])?;
    let mut times = HashSet::new();
    let mut previous = None;
    for interval in &timeline.bounds {
        if interval.end - interval.start != Duration::hours(1)
            || interval.start.minute() != 0
            || interval.start.second() != 0
            || interval.start.nanosecond() != 0
            || previous.is_some_and(|end| end != interval.start)
            || !times.insert(interval.end)
        {
            return Err(error(
                "timeline must have consecutive hour-aligned intervals",
            ));
        }
        previous = Some(interval.end);
    }
    let mut replacements = HashSet::new();
    for r in &inputs.replacements {
        if !replacements.insert(&r.replacement_id) {
            return Err(error(format!("duplicate replacement {}", r.replacement_id)));
        }
        if r.variable != "rain"
            || (r.kind == ReplacementKind::Point && r.end_at.is_some())
            || r.end_at.is_some_and(|end| end <= r.start_at)
            || r.value.is_some_and(|v| !v.is_finite())
        {
            return Err(error(format!("invalid replacement {}", r.replacement_id)));
        }
        let applicable = timeline
            .bounds
            .iter()
            .any(|t| matches_replacement(r, t.end));
        if applicable && !selected.iter().any(|s| s.station_id == r.station_id) {
            return Err(error(format!(
                "replacement {}: unselected station {}",
                r.replacement_id, r.station_id
            )));
        }
    }
    let selected_ids: HashSet<_> = selected.iter().map(|s| s.series_id.as_str()).collect();
    let mut samples = HashMap::new();
    for s in &inputs.samples {
        if !selected_ids.contains(s.series_id.as_str()) {
            continue;
        }
        if !times.contains(&s.observed_at) {
            return Err(error(format!(
                "series {}: off-grid sample {}",
                s.series_id, s.observed_at
            )));
        }
        let value = match s.value {
            None => None,
            Some(ObservationValue::RainMm(v)) if v.is_finite() => Some(v),
            _ => {
                return Err(error(format!(
                    "series {}: invalid rainfall value",
                    s.series_id
                )));
            }
        };
        if samples
            .insert((s.series_id.as_str(), s.observed_at), value)
            .is_some()
        {
            return Err(error(format!(
                "series {}: duplicate sample {}",
                s.series_id, s.observed_at
            )));
        }
    }
    let mut rows = Vec::new();
    for s in &selected {
        let station = stations
            .get(s.station_id.as_str())
            .ok_or_else(|| error(format!("unknown station {}", s.station_id)))?;
        let (lat, lon) = match (station.latitude, station.longitude) {
            (Some(a), Some(b))
                if a.is_finite()
                    && b.is_finite()
                    && (-90.0..=90.0).contains(&a)
                    && (-180.0..=180.0).contains(&b) =>
            {
                (a, b)
            }
            _ => {
                return Err(error(format!(
                    "station {}: invalid coordinates",
                    s.station_id
                )));
            }
        };
        for interval in &timeline.bounds {
            let raw = samples.get(&(s.series_id.as_str(), interval.end));
            let mut row = EffectiveObservation {
                station_id: s.station_id.clone(),
                series_id: s.series_id.clone(),
                observed_at: interval.end,
                longitude: lon,
                latitude: lat,
                raw_present: raw.is_some(),
                raw_value: raw.copied().flatten(),
                effective_value: raw.copied().flatten(),
                replacement: AppliedReplacement::None,
                applied_instruction_ids: Vec::new(),
            };
            for r in &inputs.replacements {
                if r.station_id == s.station_id && matches_replacement(r, interval.end) {
                    row.effective_value = r.value;
                    row.replacement = r
                        .value
                        .map_or(AppliedReplacement::Null, AppliedReplacement::Value);
                    row.applied_instruction_ids.push(r.replacement_id.clone());
                }
            }
            rows.push(row);
        }
    }
    Ok(rows)
}
fn matches_replacement(r: &crate::store::ObservationReplacement, at: NaiveDateTime) -> bool {
    match r.kind {
        ReplacementKind::Point => at == r.start_at,
        ReplacementKind::Period => at >= r.start_at && r.end_at.is_none_or(|end| at < end),
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ReplacementDiagnostic {
    pub replacement_id: String,
    pub applied: usize,
    pub superseded: usize,
    pub inserted: usize,
    pub inactive: bool,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ObservedPreparation {
    pub effective: Vec<EffectiveObservation>,
    pub qc: Vec<PrecipitationQcFinding>,
    pub working: crate::store::GridSeries,
    pub basin: crate::store::BasinForcing,
    pub provenance: crate::store::Provenance,
    pub replacement_diagnostics: Vec<ReplacementDiagnostic>,
}

pub fn prepare_observed_forcing(
    inputs: &ObservationInputs,
    timeline: &IntervalAxis,
    grid: &crate::store::RegularGridSpec,
    basins: &crate::store::BasinGeometry,
    interpolation: &crate::store::IdwSettings,
    qc: &PrecipitationQcPolicy,
) -> Result<ObservedPreparation, ObservationError> {
    timeline.validate().map_err(|e| error(e.to_string()))?;
    grid.validate().map_err(|e| error(e.to_string()))?;
    basins.validate().map_err(|e| error(e.to_string()))?;
    let effective = effective_observations(inputs, timeline)?;
    let raw: Vec<_> = effective
        .iter()
        .filter(|r| r.raw_present)
        .map(|r| SelectedObservation {
            station_id: r.station_id.clone(),
            series_id: r.series_id.clone(),
            observed_at: r.observed_at,
            longitude: r.longitude,
            latitude: r.latitude,
            value: r.raw_value,
        })
        .collect();
    let findings = detect_suspect_precipitation(&raw, qc)?;
    let diagnostics = inputs
        .replacements
        .iter()
        .map(|r| {
            let matches: Vec<_> = effective
                .iter()
                .filter(|row| row.applied_instruction_ids.contains(&r.replacement_id))
                .collect();
            ReplacementDiagnostic {
                replacement_id: r.replacement_id.clone(),
                applied: matches.len(),
                superseded: matches
                    .iter()
                    .filter(|row| row.applied_instruction_ids.last() != Some(&r.replacement_id))
                    .count(),
                inserted: matches
                    .iter()
                    .filter(|row| {
                        !row.raw_present
                            && row.applied_instruction_ids.last() == Some(&r.replacement_id)
                            && row.effective_value.is_some()
                    })
                    .count(),
                inactive: matches.is_empty(),
            }
        })
        .collect::<Vec<_>>();
    #[derive(Serialize)]
    struct Processing<'a> {
        time_zone: &'a str,
        selected_series: Vec<&'a ObservationSeries>,
        replacements: &'a [crate::store::ObservationReplacement],
        diagnostics: &'a [ReplacementDiagnostic],
        grid: &'a crate::store::RegularGridSpec,
        interpolation: &'a crate::store::IdwSettings,
        qc: &'a PrecipitationQcPolicy,
        station_algorithm: &'static str,
        basin_algorithm: &'static str,
    }
    let ids: HashSet<_> = effective.iter().map(|r| r.series_id.as_str()).collect();
    let processing = Processing {
        time_zone: &timeline.time_zone,
        selected_series: inputs
            .series
            .iter()
            .filter(|s| ids.contains(s.series_id.as_str()))
            .collect(),
        replacements: &inputs.replacements,
        diagnostics: &diagnostics,
        grid,
        interpolation,
        qc,
        station_algorithm: "degree_idw_station_coincident_mean_distance_station_id",
        basin_algorithm: "degree_idw_grid_zero_1e-12_distance_index",
    };
    let mut source_ids: Vec<_> = ids.into_iter().map(str::to_owned).collect();
    source_ids.sort();
    let provenance = crate::store::Provenance {
        source_ids,
        processing: serde_json::to_value(processing).map_err(|e| error(e.to_string()))?,
    };
    let target_lons: Vec<_> = grid
        .latitudes
        .iter()
        .flat_map(|_| grid.longitudes.iter().copied())
        .collect();
    let target_lats: Vec<_> = grid
        .latitudes
        .iter()
        .flat_map(|lat| grid.longitudes.iter().map(move |_| *lat))
        .collect();
    let mut values = Vec::new();
    for interval in &timeline.bounds {
        let points: Vec<_> = effective
            .iter()
            .filter(|r| r.observed_at == interval.end && r.effective_value.is_some())
            .collect();
        if points.is_empty() {
            return Err(error(format!("no usable rainfall at {}", interval.end)));
        }
        let ids: Vec<_> = points.iter().map(|r| r.station_id.clone()).collect();
        let lons: Vec<_> = points.iter().map(|r| r.longitude).collect();
        let lats: Vec<_> = points.iter().map(|r| r.latitude).collect();
        let rain: Vec<_> = points.iter().map(|r| r.effective_value.unwrap()).collect();
        values.extend(
            crate::analysis::interpolate_stations(
                &ids,
                &lons,
                &lats,
                &rain,
                &target_lons,
                &target_lats,
                interpolation,
            )
            .map_err(|e| error(format!("{}: {e}", interval.end)))?,
        );
    }
    let working = crate::store::GridSeries {
        timeline: timeline.clone(),
        grid: grid.clone(),
        valid: vec![true; values.len()],
        values,
        provenance: provenance.clone(),
    };
    let basin = crate::analysis::interpolate_grid_to_basins(&working, basins, interpolation)
        .map_err(|e| error(e.to_string()))?;
    Ok(ObservedPreparation {
        effective,
        qc: findings,
        working,
        basin,
        provenance,
        replacement_diagnostics: diagnostics,
    })
}
