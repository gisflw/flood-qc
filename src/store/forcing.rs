use super::{IntervalAxis, ValueError};
use chrono::{Duration, NaiveDateTime, TimeZone, Timelike};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashSet},
    path::Path,
};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ForcingSelections {
    pub qobs: Vec<ObservedFlowMapping>,
    pub reservoirs: Vec<ReservoirMapping>,
    pub wind: Vec<WindMapping>,
    pub disable_reservoirs: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObservedFlowMapping {
    pub mini_id: i64,
    pub series_id: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReservoirMapping {
    pub mini_id: i64,
    pub observed_series_id: Option<String>,
    pub schedule_id: Option<String>,
    pub extrapolation: Option<ReleaseExtrapolation>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WindMapping {
    pub block: usize,
    pub series_id: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReleaseExtrapolationMethod {
    Linear,
    Logarithmic,
    Exponential,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReleaseExtrapolation {
    pub method: ReleaseExtrapolationMethod,
    pub start_at: NaiveDateTime,
    pub end_at: NaiveDateTime,
    pub start_m3s: f64,
    pub end_m3s: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReservoirSchedules {
    pub schema_version: u32,
    pub time_zone: String,
    pub samples: Vec<ReservoirScheduleSample>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReservoirScheduleSample {
    pub series_id: String,
    pub at: NaiveDateTime,
    pub flow_m3s: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WindObservations {
    pub schema_version: u32,
    pub time_zone: String,
    pub samples: Vec<WindSample>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WindSample {
    pub series_id: String,
    pub at: NaiveDateTime,
    pub wind: Option<WindVector>,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WindVector {
    pub speed_mps: f64,
    pub direction_degrees: f64,
}

impl WindVector {
    pub fn validate(self) -> Result<(), ValueError> {
        if valid_flow(self.speed_mps) && (0.0..=360.0).contains(&self.direction_degrees) {
            Ok(())
        } else {
            Err(ValueError("invalid wind speed or direction".into()))
        }
    }
}

pub(crate) fn valid_flow(value: f64) -> bool {
    value >= 0.0 && (value as f32).is_finite()
}

pub(crate) fn whole_hour(time: NaiveDateTime) -> bool {
    time.minute() == 0 && time.second() == 0 && time.nanosecond() == 0
}

fn saved_samples<'a>(
    version: u32,
    time_zone: &str,
    samples: impl Iterator<Item = (&'a str, NaiveDateTime)>,
) -> Result<(), ValueError> {
    if version != 1 {
        return Err(ValueError("unsupported forcing schema version".into()));
    }
    let zone = time_zone
        .parse::<chrono_tz::Tz>()
        .map_err(|_| ValueError("invalid forcing timezone".into()))?;
    let mut seen = HashSet::new();
    for (id, at) in samples {
        if id.trim().is_empty()
            || !whole_hour(at)
            || zone.from_local_datetime(&at).single().is_none()
            || !seen.insert((id, at))
        {
            return Err(ValueError(format!(
                "invalid or duplicate forcing sample {id} {at}"
            )));
        }
    }
    Ok(())
}

impl ReservoirSchedules {
    pub fn validate(&self) -> Result<(), ValueError> {
        saved_samples(
            self.schema_version,
            &self.time_zone,
            self.samples
                .iter()
                .map(|row| (row.series_id.as_str(), row.at)),
        )?;
        if self
            .samples
            .iter()
            .flat_map(|row| row.flow_m3s)
            .any(|value| !valid_flow(value))
        {
            return Err(ValueError("invalid scheduled reservoir release".into()));
        }
        Ok(())
    }
}

impl WindObservations {
    pub fn validate(&self) -> Result<(), ValueError> {
        saved_samples(
            self.schema_version,
            &self.time_zone,
            self.samples
                .iter()
                .map(|row| (row.series_id.as_str(), row.at)),
        )?;
        for wind in self.samples.iter().flat_map(|row| row.wind) {
            wind.validate()?;
        }
        Ok(())
    }
}

#[derive(Serialize, Deserialize)]
struct ScheduleRow {
    series_id: String,
    at: NaiveDateTime,
    flow_m3s: Option<f64>,
}
#[derive(Serialize, Deserialize)]
struct WindRow {
    series_id: String,
    at: NaiveDateTime,
    speed_mps: Option<f64>,
    direction_degrees: Option<f64>,
}
fn forcing_metadata(kind: &str, zone: &str) -> std::collections::HashMap<String, String> {
    [
        ("kind".into(), kind.into()),
        ("version".into(), "1".into()),
        ("time_zone".into(), zone.into()),
    ]
    .into()
}
fn forcing_zone(path: &Path, kind: &str) -> Result<String, ValueError> {
    let metadata = super::parquet::read_metadata(path).map_err(ValueError)?;
    if metadata.get("version").map(String::as_str) != Some("1")
        || metadata.get("kind").map(String::as_str) != Some(kind)
    {
        return Err(ValueError("unsupported forcing Parquet schema".into()));
    }
    metadata
        .get("time_zone")
        .cloned()
        .ok_or_else(|| ValueError("missing forcing timezone".into()))
}
fn schedule_columns() -> [(&'static str, super::parquet::Kind); 3] {
    use super::parquet::Kind::{Float, Text};
    [("series_id", Text), ("at", Text), ("flow_m3s", Float)]
}
fn wind_columns() -> [(&'static str, super::parquet::Kind); 4] {
    use super::parquet::Kind::{Float, Text};
    [
        ("series_id", Text),
        ("at", Text),
        ("speed_mps", Float),
        ("direction_degrees", Float),
    ]
}
pub fn write_reservoir_schedules(
    path: &Path,
    schedules: &ReservoirSchedules,
) -> Result<(), ValueError> {
    schedules.validate()?;
    let rows = schedules
        .samples
        .iter()
        .map(|s| ScheduleRow {
            series_id: s.series_id.clone(),
            at: s.at,
            flow_m3s: s.flow_m3s,
        })
        .collect::<Vec<_>>();
    super::parquet::write_with_metadata(
        path,
        &schedule_columns(),
        &rows,
        forcing_metadata("reservoir_schedules", &schedules.time_zone),
    )
    .map_err(ValueError)
}
pub fn write_wind_observations(path: &Path, wind: &WindObservations) -> Result<(), ValueError> {
    wind.validate()?;
    let rows = wind
        .samples
        .iter()
        .map(|s| WindRow {
            series_id: s.series_id.clone(),
            at: s.at,
            speed_mps: s.wind.map(|w| w.speed_mps),
            direction_degrees: s.wind.map(|w| w.direction_degrees),
        })
        .collect::<Vec<_>>();
    super::parquet::write_with_metadata(
        path,
        &wind_columns(),
        &rows,
        forcing_metadata("wind_observations", &wind.time_zone),
    )
    .map_err(ValueError)
}
pub fn read_reservoir_schedules(path: &Path) -> Result<ReservoirSchedules, ValueError> {
    let time_zone = forcing_zone(path, "reservoir_schedules")?;
    let rows: Vec<ScheduleRow> =
        super::parquet::read(path, &schedule_columns()).map_err(ValueError)?;
    let schedules = ReservoirSchedules {
        schema_version: 1,
        time_zone,
        samples: rows
            .into_iter()
            .map(|s| ReservoirScheduleSample {
                series_id: s.series_id,
                at: s.at,
                flow_m3s: s.flow_m3s,
            })
            .collect(),
    };
    schedules.validate()?;
    Ok(schedules)
}
pub fn read_wind_observations(path: &Path) -> Result<WindObservations, ValueError> {
    let time_zone = forcing_zone(path, "wind_observations")?;
    let rows: Vec<WindRow> = super::parquet::read(path, &wind_columns()).map_err(ValueError)?;
    let samples = rows
        .into_iter()
        .map(|s| {
            let wind = match (s.speed_mps, s.direction_degrees) {
                (None, None) => None,
                (Some(speed_mps), Some(direction_degrees)) => Some(WindVector {
                    speed_mps,
                    direction_degrees,
                }),
                _ => return Err(ValueError("unpaired wind components".into())),
            };
            Ok(WindSample {
                series_id: s.series_id,
                at: s.at,
                wind,
            })
        })
        .collect::<Result<Vec<_>, ValueError>>()?;
    let wind = WindObservations {
        schema_version: 1,
        time_zone,
        samples,
    };
    wind.validate()?;
    Ok(wind)
}

impl ForcingSelections {
    pub fn validate(&self) -> Result<(), ValueError> {
        let mut flow_minis = HashSet::new();
        for mapping in &self.qobs {
            if mapping.mini_id <= 0
                || !flow_minis.insert(mapping.mini_id)
                || mapping.series_id.trim().is_empty()
            {
                return Err(ValueError("invalid or duplicate QOBS mapping".into()));
            }
        }
        let mut reservoir_minis = HashSet::new();
        for mapping in &self.reservoirs {
            if mapping.mini_id <= 0
                || !reservoir_minis.insert(mapping.mini_id)
                || [&mapping.observed_series_id, &mapping.schedule_id]
                    .into_iter()
                    .filter_map(Option::as_deref)
                    .any(|id| id.trim().is_empty())
                || (mapping.observed_series_id.is_none()
                    && mapping.schedule_id.is_none()
                    && mapping.extrapolation.is_none())
            {
                return Err(ValueError("invalid or duplicate reservoir mapping".into()));
            }
            if let Some(extrapolation) = &mapping.extrapolation {
                extrapolation.validate()?;
            }
        }
        if self.disable_reservoirs && !self.reservoirs.is_empty() {
            return Err(ValueError(
                "disabled reservoirs cannot have mappings".into(),
            ));
        }
        if self.wind.len() > 1
            || self
                .wind
                .first()
                .is_some_and(|mapping| mapping.block != 1 || mapping.series_id.trim().is_empty())
        {
            return Err(ValueError(
                "the qualified RS engine supports one wind series at block 1".into(),
            ));
        }
        Ok(())
    }

    pub fn is_empty(&self) -> bool {
        self.qobs.is_empty()
            && self.reservoirs.is_empty()
            && self.wind.is_empty()
            && !self.disable_reservoirs
    }
}

impl ReleaseExtrapolation {
    pub fn validate(&self) -> Result<(), ValueError> {
        if !whole_hour(self.start_at)
            || !whole_hour(self.end_at)
            || self.start_at > self.end_at
            || !valid_flow(self.start_m3s)
            || !valid_flow(self.end_m3s)
            || (self.method == ReleaseExtrapolationMethod::Exponential
                && (self.start_m3s == 0.0 || self.end_m3s == 0.0))
            || (self.method == ReleaseExtrapolationMethod::Logarithmic
                && self.start_at == self.end_at
                && self.start_m3s != self.end_m3s)
        {
            return Err(ValueError(
                "invalid reservoir extrapolation interval or flows".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreparedAdditionalForcing {
    pub timeline: IntervalAxis,
    pub qobs: BTreeMap<i64, Vec<Option<f64>>>,
    pub reservoirs: BTreeMap<i64, Vec<Option<f64>>>,
    pub wind: Option<Vec<Option<WindVector>>>,
    pub disable_reservoirs: bool,
}

impl PreparedAdditionalForcing {
    pub fn validate(&self) -> Result<(), ValueError> {
        self.timeline.validate()?;
        if self.timeline.bounds.iter().any(|interval| {
            !whole_hour(interval.end) || interval.end - interval.start != Duration::hours(1)
        }) || self
            .timeline
            .bounds
            .windows(2)
            .any(|pair| pair[0].end != pair[1].start)
        {
            return Err(ValueError(
                "forcing requires contiguous whole-hour intervals".into(),
            ));
        }
        let len = self.timeline.bounds.len();
        if self
            .qobs
            .iter()
            .chain(&self.reservoirs)
            .any(|(&mini, values)| {
                mini <= 0
                    || values.len() != len
                    || values.iter().flatten().any(|&value| !valid_flow(value))
            })
            || (self.disable_reservoirs && !self.reservoirs.is_empty())
        {
            return Err(ValueError("invalid prepared flow forcing".into()));
        }
        if let Some(wind) = &self.wind {
            if wind.len() != len {
                return Err(ValueError("invalid prepared wind length".into()));
            }
            for value in wind.iter().flatten() {
                value.validate()?;
            }
        }
        Ok(())
    }

    pub fn prefix(&self, len: usize) -> Result<Self, ValueError> {
        self.validate()?;
        if len == 0 || len > self.timeline.bounds.len() {
            return Err(ValueError("invalid prepared forcing prefix".into()));
        }
        let mut prefix = self.clone();
        prefix.timeline.bounds.truncate(len);
        for values in prefix
            .qobs
            .values_mut()
            .chain(prefix.reservoirs.values_mut())
        {
            values.truncate(len);
        }
        if let Some(wind) = &mut prefix.wind {
            wind.truncate(len);
        }
        Ok(prefix)
    }
}
