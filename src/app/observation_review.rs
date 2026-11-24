use std::collections::BTreeMap;
use std::path::PathBuf;

use chrono::{Datelike, Duration, NaiveDateTime};
use serde::{Deserialize, Serialize};

use crate::analysis::{
    AvailabilityState, EffectiveObservation, HourlyWindow, ReviewVariable, SpatialQcStation,
    effective_review_observations, resolve_hourly_window, spatial_precipitation_qc,
};
use crate::store::{
    IdwSettings, IntervalAxis, ObservationError, ObservationReplacement, ObservationSelection,
    ObservationStore, Station, TimeInterval, validate_replacement,
};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ReviewWindow {
    Recent72Hours,
    Month,
    Explicit { start: NaiveDateTime },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObservationReviewRequest {
    pub database: PathBuf,
    pub selection: ObservationSelection,
    #[serde(default)]
    pub series_ids: Vec<String>,
    pub variable: ReviewVariable,
    pub end: NaiveDateTime,
    pub window: ReviewWindow,
    #[serde(default)]
    pub replacement_ids: Vec<String>,
    pub replacements: Option<Vec<ObservationReplacement>>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct AvailabilityHour {
    pub observed_at: NaiveDateTime,
    pub state: AvailabilityState,
    pub raw_present: bool,
    pub raw_value: Option<f64>,
    pub effective_value: Option<f64>,
    pub applied_instruction_ids: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct StationAvailability {
    pub station: Station,
    pub series_id: Option<String>,
    pub absent: usize,
    pub null: usize,
    pub excluded: usize,
    pub valid: usize,
    pub hours: Vec<AvailabilityHour>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ObservationReviewReport {
    pub time_zone: String,
    pub window: TimeInterval,
    pub variable: ReviewVariable,
    pub stations: Vec<StationAvailability>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SpatialObservationQcReport {
    pub time_zone: String,
    pub window: TimeInterval,
    pub stations: Vec<SpatialQcStation>,
}

fn review_window(request: &ObservationReviewRequest) -> Result<HourlyWindow, ObservationError> {
    Ok(match request.window {
        ReviewWindow::Recent72Hours => HourlyWindow::Relative {
            end: request.end,
            hours: 72,
        },
        ReviewWindow::Month => {
            let bounds = (|| {
                let first = request.end.date().with_day(1)?;
                let first = first.and_hms_opt(0, 0, 0)?;
                let next = first.checked_add_months(chrono::Months::new(1))?;
                Some((
                    first.checked_sub_signed(Duration::hours(1))?,
                    next.checked_sub_signed(Duration::hours(1))?,
                ))
            })()
            .ok_or_else(|| ObservationError("month bounds overflow".into()))?;
            HourlyWindow::Explicit {
                start: bounds.0,
                end: bounds.1,
            }
        }
        ReviewWindow::Explicit { start } => HourlyWindow::Explicit {
            start,
            end: request.end,
        },
    })
}

struct ReviewSelection {
    axis: IntervalAxis,
    window: TimeInterval,
    stations: Vec<Station>,
    rows: Vec<EffectiveObservation>,
}

fn review_selection(
    request: &ObservationReviewRequest,
) -> Result<ReviewSelection, ObservationError> {
    let store = ObservationStore::open_read_only(&request.database)?;
    let time_zone = store.time_zone()?;
    let window =
        resolve_hourly_window(&time_zone, &review_window(request)?).map_err(ObservationError)?;
    let axis = IntervalAxis {
        time_zone,
        bounds: (0..(window.end - window.start).num_hours())
            .map(|hour| TimeInterval {
                start: window.start + Duration::hours(hour),
                end: window.start + Duration::hours(hour + 1),
            })
            .collect(),
    };
    let ids = if request.replacements.is_some() {
        if !request.replacement_ids.is_empty() {
            return Err(ObservationError(
                "select replacement IDs or supply replacement values".into(),
            ));
        }
        &[][..]
    } else {
        &request.replacement_ids
    };
    let (mut inputs, _) = store.read_selection(&request.selection, &axis, ids)?;
    if let Some(replacements) = &request.replacements {
        for replacement in replacements {
            validate_replacement(replacement)?;
        }
        inputs.replacements.clone_from(replacements);
    }
    let rows =
        effective_review_observations(&inputs, &axis, request.variable, &request.series_ids)?;
    Ok(ReviewSelection {
        axis,
        window,
        stations: inputs.stations,
        rows,
    })
}

pub fn review_observations(
    request: &ObservationReviewRequest,
) -> Result<ObservationReviewReport, ObservationError> {
    let ReviewSelection {
        axis,
        window,
        stations,
        rows,
    } = review_selection(request)?;
    let mut by_station = BTreeMap::<String, Vec<EffectiveObservation>>::new();
    for row in rows {
        by_station
            .entry(row.station_id.clone())
            .or_default()
            .push(row);
    }
    let stations = stations
        .into_iter()
        .map(|station| {
            let selected = by_station.remove(&station.station_id).unwrap_or_default();
            let mut report = StationAvailability {
                station,
                series_id: selected.first().map(|row| row.series_id.clone()),
                absent: 0,
                null: 0,
                excluded: 0,
                valid: 0,
                hours: Vec::with_capacity(axis.bounds.len()),
            };
            if selected.is_empty() {
                report.absent = axis.bounds.len();
                report
                    .hours
                    .extend(axis.bounds.iter().map(|interval| AvailabilityHour {
                        observed_at: interval.end,
                        state: AvailabilityState::Absent,
                        raw_present: false,
                        raw_value: None,
                        effective_value: None,
                        applied_instruction_ids: Vec::new(),
                    }));
            }
            for row in selected {
                let state = row.availability();
                match state {
                    AvailabilityState::Absent => report.absent += 1,
                    AvailabilityState::Null => report.null += 1,
                    AvailabilityState::Excluded => report.excluded += 1,
                    AvailabilityState::Valid => report.valid += 1,
                }
                report.hours.push(AvailabilityHour {
                    observed_at: row.observed_at,
                    state,
                    raw_present: row.raw_present,
                    raw_value: row.raw_value,
                    effective_value: row.effective_value,
                    applied_instruction_ids: row.applied_instruction_ids,
                });
            }
            report
        })
        .collect();
    Ok(ObservationReviewReport {
        time_zone: axis.time_zone,
        window,
        variable: request.variable,
        stations,
    })
}

pub fn spatial_observation_qc(
    request: &ObservationReviewRequest,
    target: Option<&str>,
    settings: &IdwSettings,
) -> Result<SpatialObservationQcReport, ObservationError> {
    if request.variable != ReviewVariable::Rain {
        return Err(ObservationError(
            "spatial precipitation QC requires rain".into(),
        ));
    }
    let ReviewSelection {
        axis, window, rows, ..
    } = review_selection(request)?;
    let stations = spatial_precipitation_qc(&rows, target, settings)?;
    Ok(SpatialObservationQcReport {
        time_zone: axis.time_zone,
        window,
        stations,
    })
}
