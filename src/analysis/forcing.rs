use crate::store::{
    ForcingSelections, IntervalAxis, ObservationInputs, ObservationValue,
    PreparedAdditionalForcing, ReleaseExtrapolation, ReleaseExtrapolationMethod,
    ReservoirSchedules, ValueError, WindObservations,
};
use chrono::NaiveDateTime;
use std::collections::BTreeMap;

fn extrapolate(config: &ReleaseExtrapolation, at: NaiveDateTime) -> Option<f64> {
    if !(config.start_at..=config.end_at).contains(&at) {
        return None;
    }
    let span = (config.end_at - config.start_at).num_hours() + 1;
    let step = (at - config.start_at).num_hours() + 1;
    let start = config.start_m3s;
    let end = config.end_m3s;
    if step == span {
        return Some(end);
    }
    // Form5 advances the linear and exponential curves at the first hour.
    let fraction = step as f64 / span as f64;
    Some(match config.method {
        ReleaseExtrapolationMethod::Linear => start + (end - start) * fraction,
        ReleaseExtrapolationMethod::Logarithmic => {
            start + (end - start) * ((step as f64).ln() / (span as f64).ln())
        }
        ReleaseExtrapolationMethod::Exponential => {
            (start.ln() + (end.ln() - start.ln()) * fraction).exp()
        }
    })
}

pub fn prepare_additional_forcing(
    timeline: &IntervalAxis,
    selections: &ForcingSelections,
    observations: &ObservationInputs,
    schedules: Option<&ReservoirSchedules>,
    wind: Option<&WindObservations>,
) -> Result<PreparedAdditionalForcing, ValueError> {
    selections.validate()?;
    if let Some(schedules) = schedules {
        schedules.validate()?;
        if schedules.time_zone != timeline.time_zone {
            return Err(ValueError(
                "reservoir schedule and operation time zones differ".into(),
            ));
        }
    }
    if let Some(wind) = wind {
        wind.validate()?;
        if wind.time_zone != timeline.time_zone {
            return Err(ValueError("wind and operation time zones differ".into()));
        }
    }

    let mut observed = BTreeMap::new();
    let ids = selections
        .qobs
        .iter()
        .map(|qobs| qobs.series_id.as_str())
        .chain(
            selections
                .reservoirs
                .iter()
                .filter_map(|reservoir| reservoir.observed_series_id.as_deref()),
        );
    for id in ids {
        if observed.contains_key(id) {
            continue;
        }
        if !observations.series.iter().any(|series| {
            series.series_id == id && series.variable == "flow" && series.unit == "m3/s"
        }) {
            return Err(ValueError(format!("unknown observed flow series {id}")));
        }
        let mut values = BTreeMap::new();
        for sample in observations
            .samples
            .iter()
            .filter(|sample| sample.series_id == id)
        {
            let value = match sample.value {
                Some(ObservationValue::FlowM3s(value)) => Some(value),
                None => None,
                Some(_) => return Err(ValueError(format!("invalid observed flow sample {id}"))),
            };
            if values.insert(sample.observed_at, value).is_some() {
                return Err(ValueError(format!(
                    "duplicate observed flow sample {id} {}",
                    sample.observed_at
                )));
            }
        }
        observed.insert(id, values);
    }

    let mut schedule_values = BTreeMap::<_, BTreeMap<_, _>>::new();
    for sample in schedules.into_iter().flat_map(|source| &source.samples) {
        schedule_values
            .entry(sample.series_id.as_str())
            .or_default()
            .insert(sample.at, sample.flow_m3s);
    }
    let qobs = selections
        .qobs
        .iter()
        .map(|mapping| {
            let values = &observed[mapping.series_id.as_str()];
            (
                mapping.mini_id,
                timeline
                    .bounds
                    .iter()
                    .map(|interval| values.get(&interval.end).copied().flatten())
                    .collect(),
            )
        })
        .collect();

    let mut reservoirs = BTreeMap::new();
    for mapping in &selections.reservoirs {
        let observed = mapping
            .observed_series_id
            .as_deref()
            .map(|id| &observed[id]);
        let scheduled = mapping
            .schedule_id
            .as_deref()
            .map(|id| {
                schedule_values
                    .get(id)
                    .ok_or_else(|| ValueError(format!("unknown reservoir schedule {id}")))
            })
            .transpose()?;
        let values = timeline
            .bounds
            .iter()
            .map(|interval| {
                let at = interval.end;
                observed
                    .and_then(|values| values.get(&at).copied().flatten())
                    .or_else(|| scheduled.and_then(|values| values.get(&at).copied().flatten()))
                    .or_else(|| {
                        mapping
                            .extrapolation
                            .as_ref()
                            .and_then(|config| extrapolate(config, at))
                    })
            })
            .collect();
        reservoirs.insert(mapping.mini_id, values);
    }

    let wind = selections
        .wind
        .first()
        .map(|mapping| {
            let values: BTreeMap<_, _> = wind
                .into_iter()
                .flat_map(|source| &source.samples)
                .filter(|sample| sample.series_id == mapping.series_id)
                .map(|sample| (sample.at, sample.wind))
                .collect();
            if values.is_empty() {
                return Err(ValueError(format!(
                    "unknown wind series {}",
                    mapping.series_id
                )));
            }
            Ok(timeline
                .bounds
                .iter()
                .map(|interval| values.get(&interval.end).copied().flatten())
                .collect())
        })
        .transpose()?;

    let prepared = PreparedAdditionalForcing {
        timeline: timeline.clone(),
        qobs,
        reservoirs,
        wind,
        disable_reservoirs: selections.disable_reservoirs,
    };
    prepared.validate()?;
    Ok(prepared)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::{
        ObservationSample, ObservationSeries, ReservoirMapping, ReservoirScheduleSample,
        TimeInterval,
    };
    use chrono::Duration;

    #[test]
    fn observed_zero_wins_then_schedule_then_configured_release() {
        let start: NaiveDateTime = "2026-07-10T00:00:00".parse().unwrap();
        let hour = |n| start + Duration::hours(n);
        let timeline = IntervalAxis {
            time_zone: "UTC".into(),
            bounds: (0..5)
                .map(|n| TimeInterval {
                    start: hour(n),
                    end: hour(n + 1),
                })
                .collect(),
        };
        let mut curve = ReleaseExtrapolation {
            method: ReleaseExtrapolationMethod::Linear,
            start_at: hour(1),
            end_at: hour(4),
            start_m3s: 0.0,
            end_m3s: 4.0,
        };
        let selections = ForcingSelections {
            reservoirs: vec![ReservoirMapping {
                mini_id: 12,
                observed_series_id: Some("flow".into()),
                schedule_id: Some("schedule".into()),
                extrapolation: Some(curve.clone()),
            }],
            ..ForcingSelections::default()
        };
        let observations = ObservationInputs {
            series: vec![ObservationSeries {
                series_id: "flow".into(),
                station_id: "station".into(),
                variable: "flow".into(),
                state: "raw".into(),
                unit: "m3/s".into(),
                interval_seconds: None,
                created_at: start,
            }],
            samples: vec![
                ObservationSample {
                    series_id: "flow".into(),
                    observed_at: hour(1),
                    value: Some(ObservationValue::FlowM3s(0.0)),
                },
                ObservationSample {
                    series_id: "flow".into(),
                    observed_at: hour(2),
                    value: None,
                },
            ],
            ..ObservationInputs::default()
        };
        let schedules = ReservoirSchedules {
            schema_version: 1,
            time_zone: "UTC".into(),
            samples: vec![
                ReservoirScheduleSample {
                    series_id: "schedule".into(),
                    at: hour(1),
                    flow_m3s: Some(99.0),
                },
                ReservoirScheduleSample {
                    series_id: "schedule".into(),
                    at: hour(2),
                    flow_m3s: Some(9.0),
                },
            ],
        };
        let prepared = prepare_additional_forcing(
            &timeline,
            &selections,
            &observations,
            Some(&schedules),
            None,
        )
        .unwrap();
        assert_eq!(
            prepared.reservoirs[&12],
            [Some(0.0), Some(9.0), Some(3.0), Some(4.0), None]
        );
        assert_eq!(
            prepared.prefix(2).unwrap().reservoirs[&12],
            [Some(0.0), Some(9.0)]
        );
        curve.method = ReleaseExtrapolationMethod::Logarithmic;
        assert_eq!(extrapolate(&curve, hour(1)), Some(0.0));
        assert_eq!(extrapolate(&curve, hour(2)), Some(2.0));
        curve.method = ReleaseExtrapolationMethod::Exponential;
        curve.start_m3s = 1.0;
        curve.end_m3s = 16.0;
        assert_eq!(extrapolate(&curve, hour(2)), Some(4.0));
    }
}
