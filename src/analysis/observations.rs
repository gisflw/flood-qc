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
    pub longitude: Option<f64>,
    pub latitude: Option<f64>,
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
    pub longitude: Option<f64>,
    pub latitude: Option<f64>,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewVariable {
    Rain,
    Flow,
    Level,
}
impl ReviewVariable {
    fn name(self) -> &'static str {
        match self {
            Self::Rain => "rain",
            Self::Flow => "flow",
            Self::Level => "level",
        }
    }

    fn unit_and_interval(self) -> (&'static str, Option<i64>) {
        match self {
            Self::Rain => ("mm", Some(3600)),
            Self::Flow => ("m3/s", None),
            Self::Level => ("cm", None),
        }
    }

    fn sample(self, value: ObservationValue) -> Option<f64> {
        match (self, value) {
            (Self::Rain, ObservationValue::RainMm(v))
            | (Self::Flow, ObservationValue::FlowM3s(v))
                if v.is_finite() =>
            {
                Some(v)
            }
            (Self::Level, ObservationValue::LevelCm(v)) => Some(f64::from(v)),
            _ => None,
        }
    }
}

pub fn select_observation_series(
    candidates: &[ObservationSeries],
    explicit_series_ids: &[String],
) -> Result<Vec<ObservationSeries>, ObservationError> {
    select_series(candidates, explicit_series_ids, ReviewVariable::Rain)
}

fn select_series(
    candidates: &[ObservationSeries],
    explicit_series_ids: &[String],
    variable: ReviewVariable,
) -> Result<Vec<ObservationSeries>, ObservationError> {
    let name = variable.name();
    let mut explicit = HashSet::new();
    for id in explicit_series_ids {
        if !explicit.insert(id.as_str()) {
            return Err(error(format!("duplicate explicit series {id}")));
        }
        if !candidates
            .iter()
            .any(|s| &s.series_id == id && s.variable == name)
        {
            return Err(error(format!("unknown {name} series {id}")));
        }
    }
    let mut grouped: BTreeMap<&str, Vec<&ObservationSeries>> = BTreeMap::new();
    for s in candidates.iter().filter(|s| s.variable == name) {
        if !matches!(s.state.as_str(), "approved" | "curated" | "raw")
            || (s.unit.as_str(), s.interval_seconds) != variable.unit_and_interval()
        {
            return Err(error(format!(
                "series {}: invalid {name} metadata",
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
    effective_review_observations(inputs, timeline, ReviewVariable::Rain, &[])
}

pub fn effective_flow_observations(
    inputs: &ObservationInputs,
    timeline: &IntervalAxis,
) -> Result<Vec<EffectiveObservation>, ObservationError> {
    effective_review_observations(inputs, timeline, ReviewVariable::Flow, &[])
}

pub fn effective_review_observations(
    inputs: &ObservationInputs,
    timeline: &IntervalAxis,
    variable: ReviewVariable,
    explicit_series_ids: &[String],
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
    let selected = select_series(&inputs.series, explicit_series_ids, variable)?;
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
        if !matches!(r.variable.as_str(), "rain" | "flow")
            || (r.kind == ReplacementKind::Point && r.end_at.is_some())
            || r.end_at.is_some_and(|end| end <= r.start_at)
            || r.value.is_some_and(|v| !v.is_finite())
        {
            return Err(error(format!("invalid replacement {}", r.replacement_id)));
        }
        if r.variable != variable.name() {
            continue;
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
            if variable != ReviewVariable::Rain {
                continue;
            }
            return Err(error(format!(
                "series {}: off-grid sample {}",
                s.series_id, s.observed_at
            )));
        }
        let value = match s.value {
            None => None,
            Some(v) => Some(variable.sample(v).ok_or_else(|| {
                error(format!(
                    "series {}: invalid {} value",
                    s.series_id,
                    variable.name()
                ))
            })?),
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
                (Some(a), Some(b))
            }
            (None, None) => (None, None),
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
                if r.variable == variable.name()
                    && r.station_id == s.station_id
                    && matches_replacement(r, interval.end)
                {
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
    if let Some(row) = effective
        .iter()
        .find(|r| r.longitude.is_none() || r.latitude.is_none())
    {
        return Err(error(format!(
            "station {}: coordinates required for interpolation",
            row.station_id
        )));
    }
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
        let lons: Vec<_> = points.iter().map(|r| r.longitude.unwrap()).collect();
        let lats: Vec<_> = points.iter().map(|r| r.latitude.unwrap()).collect();
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AvailabilityState {
    Absent,
    Null,
    Excluded,
    Valid,
}

impl EffectiveObservation {
    pub fn availability(&self) -> AvailabilityState {
        match (self.effective_value, &self.replacement, self.raw_present) {
            (Some(_), _, _) => AvailabilityState::Valid,
            (None, AppliedReplacement::Null, _) => AvailabilityState::Excluded,
            (None, _, true) => AvailabilityState::Null,
            (None, _, false) => AvailabilityState::Absent,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SpatialQcHour {
    pub observed_at: NaiveDateTime,
    pub raw_mm: f64,
    pub predicted_mm: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SpatialQcStation {
    pub station_id: String,
    pub expected_hours: usize,
    pub raw_hours: usize,
    pub matched_hours: usize,
    pub raw_total_mm: Option<f64>,
    pub predicted_total_mm: Option<f64>,
    pub residual_mm: Option<f64>,
    pub hours: Vec<SpatialQcHour>,
}

/// Compare raw target rain with IDW from other stations' effective rain at the same hour.
pub fn spatial_precipitation_qc(
    rows: &[EffectiveObservation],
    target: Option<&str>,
    settings: &crate::store::IdwSettings,
) -> Result<Vec<SpatialQcStation>, ObservationError> {
    settings.validate().map_err(|e| error(e.to_string()))?;
    let mut stations = BTreeMap::<&str, Vec<&EffectiveObservation>>::new();
    let mut neighbors = BTreeMap::<NaiveDateTime, Vec<_>>::new();
    for row in rows {
        stations
            .entry(row.station_id.as_str())
            .or_default()
            .push(row);
        if let (Some(lon), Some(lat), Some(value)) =
            (row.longitude, row.latitude, row.effective_value)
        {
            neighbors
                .entry(row.observed_at)
                .or_default()
                .push((&row.station_id, lon, lat, value));
        }
    }
    if let Some(target) = target
        && !stations.contains_key(target)
    {
        return Err(error(format!(
            "no selected rainfall series for station {target}"
        )));
    }
    let mut result = Vec::new();
    for (station, rows) in stations {
        if target.is_some_and(|id| id != station) {
            continue;
        }
        let mut report = SpatialQcStation {
            station_id: station.into(),
            expected_hours: rows.len(),
            raw_hours: rows.iter().filter(|row| row.raw_value.is_some()).count(),
            matched_hours: 0,
            raw_total_mm: None,
            predicted_total_mm: None,
            residual_mm: None,
            hours: Vec::new(),
        };
        for row in rows {
            let (Some(raw), Some(lon), Some(lat), Some(candidates)) = (
                row.raw_value,
                row.longitude,
                row.latitude,
                neighbors.get(&row.observed_at),
            ) else {
                continue;
            };
            let (mut ids, mut lons, mut lats, mut values) =
                (Vec::new(), Vec::new(), Vec::new(), Vec::new());
            for &(id, x, y, rain) in candidates {
                if id != station {
                    ids.push(id.clone());
                    lons.push(x);
                    lats.push(y);
                    values.push(rain);
                }
            }
            if ids.is_empty() {
                continue;
            }
            let predicted = crate::analysis::interpolate_stations(
                &ids,
                &lons,
                &lats,
                &values,
                &[lon],
                &[lat],
                settings,
            )
            .map_err(|e| error(format!("station {station} {}: {e}", row.observed_at)))?[0];
            report.hours.push(SpatialQcHour {
                observed_at: row.observed_at,
                raw_mm: raw,
                predicted_mm: predicted,
            });
        }
        report.matched_hours = report.hours.len();
        if !report.hours.is_empty() {
            let raw = report.hours.iter().map(|row| row.raw_mm).sum::<f64>();
            let predicted = report.hours.iter().map(|row| row.predicted_mm).sum::<f64>();
            let residual = predicted - raw;
            if [raw, predicted, residual].iter().any(|v| !v.is_finite()) {
                return Err(error(format!(
                    "station {station}: spatial rainfall QC overflow"
                )));
            }
            report.raw_total_mm = Some(raw);
            report.predicted_total_mm = Some(predicted);
            report.residual_mm = Some(residual);
        }
        result.push(report);
    }
    Ok(result)
}
