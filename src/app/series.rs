use crate::analysis::{
    HourlyWindow, effective_flow_observations, effective_observations, resolve_hourly_window,
};
use crate::store::{
    IntervalAxis, ObservationSelection, ObservationStore, ObservationValue, ResultSeries,
    StationResults, TimeInterval, read_result_series, read_retained_manifest, read_station_results,
};
use chrono::{Duration, NaiveDateTime, Timelike};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Location {
    Station(String),
    Mini(i64),
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Variable {
    Rain,
    Flow,
    Level,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SeriesSelection {
    Model {
        root: PathBuf,
        scenario_id: String,
        variable: Variable,
    },
    Observation {
        database: PathBuf,
        series_id: String,
        variable: Variable,
        effective: bool,
        replacement_ids: Vec<String>,
        #[serde(default)]
        replacements: Option<Vec<crate::store::ObservationReplacement>>,
    },
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SeriesQuery {
    pub location: Location,
    pub time_zone: String,
    pub window: HourlyWindow,
    pub selections: Vec<SeriesSelection>,
}
#[derive(Clone, Debug, Serialize)]
pub struct SeriesPoint {
    pub timestamp: NaiveDateTime,
    pub value: Option<f64>,
    pub time_segment: Option<u8>,
}
#[derive(Clone, Debug, Serialize)]
pub struct ChartSeries {
    pub selection_index: usize,
    pub series_kind: String,
    pub run_id: Option<String>,
    pub scenario_id: Option<String>,
    pub forecast_reference_time: Option<String>,
    pub variable: String,
    pub unit: String,
    pub points: Vec<SeriesPoint>,
}
#[derive(Clone, Debug, Serialize)]
pub struct QueryFinding {
    pub selection_index: usize,
    pub message: String,
}
#[derive(Clone, Debug, Serialize)]
pub struct SeriesReport {
    pub location: Location,
    pub time_zone: String,
    pub window: TimeInterval,
    pub series: Vec<ChartSeries>,
    pub findings: Vec<QueryFinding>,
}
fn names(v: &Variable) -> (&'static str, &'static str) {
    match v {
        Variable::Rain => ("rain", "mm"),
        Variable::Flow => ("flow", "m3/s"),
        Variable::Level => ("level", "cm"),
    }
}
pub(super) fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-')
}
type ModelMapping = Option<(i64, f64, f64, f64)>;

pub(super) struct ModelSource {
    pub(super) basin: ResultSeries,
    station: Result<Option<StationResults>, String>,
    pub(super) reference: String,
    run_id: String,
}

pub(super) fn read_model(
    root: &Path,
    scenario: &str,
    zone: Option<&str>,
    comparison: bool,
) -> Result<ModelSource, String> {
    if !valid_id(scenario) {
        return Err("invalid scenario ID".into());
    }
    let manifest = read_retained_manifest(root)?;
    if !manifest
        .scenarios
        .iter()
        .any(|s| s.id == scenario && s.state == "completed")
    {
        return Err(format!("scenario {scenario} is absent or incomplete"));
    }
    if manifest.preparation_sha256.is_some() {
        for name in [
            "QTUDO_Inercial_Atual.MGB",
            "QTUDO_Inercial_Prev.MGB",
            "YTUDO.MGB",
            "YTUDO_prev.MGB",
        ] {
            if !root
                .join("scenarios")
                .join(scenario)
                .join("Output")
                .join(name)
                .is_file()
            {
                return Err(format!("scenario {scenario}: missing engine stream {name}"));
            }
        }
    }
    let basin = read_result_series(&root.join("results").join(scenario).join("basin.nc"))
        .map_err(|e| e.to_string())?;
    let station_path = root.join("results").join(scenario).join("station.nc");
    let station = if comparison && matches!(station_path.try_exists(), Ok(false)) {
        Ok(None)
    } else {
        read_station_results(&station_path)
            .map_err(|e| e.to_string())
            .and_then(|station| {
                if basin.timeline != station.timeline
                    || basin.time_segment != station.time_segment
                    || basin.provenance.processing.get("reference_time")
                        != station.provenance.processing.get("reference_time")
                {
                    return Err(
                        "result timeline, timezone, segment or reference metadata disagree".into(),
                    );
                }
                Ok(Some(station))
            })
    };
    if !comparison {
        station.as_ref().map_err(Clone::clone)?;
    }
    if zone.is_some_and(|zone| basin.timeline.time_zone != zone) {
        return Err("result timeline, timezone, segment or reference metadata disagree".into());
    }
    let reference = basin
        .provenance
        .processing
        .get("reference_time")
        .and_then(|v| v.as_str())
        .ok_or("missing forecast reference time")?
        .to_owned();
    let parsed_reference = NaiveDateTime::parse_from_str(&reference, "%Y-%m-%dT%H:%M:%S")
        .map_err(|_| "invalid forecast reference time")?;
    if parsed_reference.format("%Y-%m-%dT%H:%M:%S").to_string() != reference
        || basin
            .time_segment
            .iter()
            .zip(&basin.timeline.bounds)
            .any(|(segment, interval)| {
                (*segment == 0 && interval.end > parsed_reference)
                    || (*segment == 1 && interval.start < parsed_reference)
            })
    {
        return Err("forecast reference time conflicts with result segments".into());
    }
    IntervalAxis {
        time_zone: basin.timeline.time_zone.clone(),
        bounds: vec![TimeInterval {
            start: parsed_reference,
            end: parsed_reference
                .checked_add_signed(Duration::hours(1))
                .ok_or("reference time overflow")?,
        }],
    }
    .validate()
    .map_err(|e| e.to_string())?;
    Ok(ModelSource {
        basin,
        station,
        reference,
        run_id: root
            .file_name()
            .ok_or("run root needs a name")?
            .to_string_lossy()
            .into_owned(),
    })
}

fn model(
    source: &ModelSource,
    scenario: &str,
    variable: &Variable,
    location: &Location,
    comparison_station: Option<&str>,
    window: &TimeInterval,
) -> Result<(ChartSeries, ModelMapping), String> {
    let basin = &source.basin;
    let station = source.station.as_ref().ok().and_then(|s| s.as_ref());
    let mapping = match location {
        Location::Mini(id) => Some((*id, 0.0)),
        Location::Station(id) => {
            let station = station.ok_or("missing station results")?;
            let index = station
                .station_ids
                .iter()
                .position(|v| v == id)
                .ok_or_else(|| format!("station {id} missing from results"))?;
            station.mini_ids[index].zip(station.offsets_m[index])
        }
    };
    let mini = mapping
        .map(|v| v.0)
        .ok_or("station has no explicit mini/offset association")?;
    let bi = basin
        .basins
        .mini_ids
        .iter()
        .position(|v| *v == mini)
        .ok_or_else(|| format!("MINI {mini} missing from basin results"))?;
    let mapping = mapping.map(|(id, offset)| {
        (
            id,
            offset,
            basin.basins.longitudes[bi],
            basin.basins.latitudes[bi],
        )
    });
    let station_level = if matches!(variable, Variable::Level) {
        source.station.as_ref().map_err(Clone::clone)?;
        station.and_then(|station| {
            let id = match location {
                Location::Station(id) => id,
                Location::Mini(_) => comparison_station?,
            };
            let index = station.station_ids.iter().position(|value| value == id)?;
            (station.mini_ids[index] == Some(mini) && station.offsets_m[index].is_some())
                .then_some((station, index))
        })
    } else {
        None
    };
    let mut points = Vec::new();
    for (t, interval) in basin.timeline.bounds.iter().enumerate() {
        if matches!(variable, Variable::Rain)
            && interval.end > window.start
            && interval.start < window.end
            && (interval.start < window.start
                || interval.end > window.end
                || interval.end - interval.start != Duration::hours(1))
        {
            return Err("partial or non-hourly rainfall interval".into());
        }
        let include = if matches!(variable, Variable::Rain) {
            interval.start >= window.start && interval.end <= window.end
        } else {
            interval.end >= window.start && interval.end <= window.end
        };
        if !include {
            continue;
        }
        let i = t * basin.basins.mini_ids.len() + bi;
        let value = match variable {
            Variable::Rain => basin.precipitation_valid[i].then_some(basin.precipitation[i]),
            Variable::Flow => basin.flow_valid[i].then_some(basin.flow[i]),
            Variable::Level => {
                if let Some((station, si)) = station_level {
                    let j = t * station.station_ids.len() + si;
                    station.level_valid[j].then_some(f64::from(station.level_cm[j]))
                } else {
                    basin.raw_level_valid[i].then_some(f64::from(basin.raw_level_cm[i]))
                }
            }
        };
        points.push(SeriesPoint {
            timestamp: interval.end,
            value,
            time_segment: Some(basin.time_segment[t]),
        });
    }
    let (name, unit) = names(variable);
    Ok((
        ChartSeries {
            selection_index: 0,
            series_kind: match (variable, station_level, comparison_station) {
                (Variable::Level, None, _) => "model_raw_level",
                (Variable::Level, Some(_), Some(_)) => "model_station_level",
                _ => "model",
            }
            .into(),
            run_id: Some(source.run_id.clone()),
            scenario_id: Some(scenario.into()),
            forecast_reference_time: Some(source.reference.clone()),
            variable: name.into(),
            unit: unit.into(),
            points,
        },
        mapping,
    ))
}
fn observation(
    selection: &SeriesSelection,
    station: &Location,
    zone: &str,
    window: &TimeInterval,
) -> Result<ChartSeries, String> {
    let SeriesSelection::Observation {
        database,
        series_id,
        variable,
        effective,
        replacement_ids,
        replacements,
    } = selection
    else {
        return Err("expected observation selection".into());
    };
    let effective = *effective;
    let supplied = replacements;
    let replacements = if supplied.is_some() {
        &[][..]
    } else {
        replacement_ids.as_slice()
    };
    let Location::Station(id) = station else {
        return Err("observations require a station".into());
    };
    if effective && matches!(variable, Variable::Level) {
        return Err("effective observations are supported for rainfall and flow".into());
    }
    let bounds = if matches!(variable, Variable::Rain) || effective {
        let mut bounds = Vec::new();
        let mut start = if matches!(variable, Variable::Flow) {
            window
                .start
                .checked_sub_signed(Duration::hours(1))
                .ok_or("window overflow")?
        } else {
            window.start
        };
        while start < window.end {
            let end = start
                .checked_add_signed(Duration::hours(1))
                .ok_or("window overflow")?;
            bounds.push(TimeInterval { start, end });
            start = end;
        }
        bounds
    } else {
        vec![window.clone()]
    };
    let axis = IntervalAxis {
        time_zone: zone.into(),
        bounds,
    };
    let store = ObservationStore::open_read_only(database).map_err(|e| e.to_string())?;
    let (mut inputs, _) = store
        .read_selection(
            &ObservationSelection {
                station_ids: vec![id.clone()],
                provider_codes: vec![],
            },
            &axis,
            replacements,
        )
        .map_err(|e| e.to_string())?;
    if let Some(values) = supplied {
        for value in values {
            crate::store::validate_replacement(value).map_err(|e| e.to_string())?;
        }
        inputs.replacements = values.clone();
    }
    let (name, unit) = names(variable);
    let selected = inputs
        .series
        .iter()
        .find(|s| s.series_id == *series_id && s.station_id == *id && s.variable == name)
        .ok_or("selected observation series is absent or incompatible")?;
    if matches!(variable, Variable::Rain) && selected.interval_seconds != Some(3600) {
        return Err("rain series must have hourly native intervals".into());
    }
    let points = if effective {
        inputs.series.retain(|s| s.series_id == *series_id);
        inputs.samples.retain(|s| s.series_id == *series_id);
        inputs.replacements.retain(|item| item.station_id == *id);
        let mut points: Vec<_> = if matches!(variable, Variable::Flow) {
            effective_flow_observations(&inputs, &axis)
        } else {
            effective_observations(&inputs, &axis)
        }
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(|r| SeriesPoint {
            timestamp: r.observed_at,
            value: r.effective_value,
            time_segment: None,
        })
        .collect();
        if matches!(variable, Variable::Flow) {
            points.extend(
                inputs
                    .samples
                    .iter()
                    .filter(|sample| {
                        let time = sample.observed_at;
                        time >= window.start
                            && time <= window.end
                            && (time.minute() != 0 || time.second() != 0 || time.nanosecond() != 0)
                    })
                    .map(|sample| SeriesPoint {
                        timestamp: sample.observed_at,
                        value: sample.value.and_then(|value| match value {
                            ObservationValue::FlowM3s(value) => Some(value),
                            _ => None,
                        }),
                        time_segment: None,
                    }),
            );
            points.sort_by_key(|point| point.timestamp);
        }
        points
    } else if matches!(variable, Variable::Rain) {
        let mut samples = inputs
            .samples
            .iter()
            .filter(|s| s.series_id == *series_id)
            .peekable();
        let mut points = Vec::with_capacity(axis.bounds.len());
        for interval in &axis.bounds {
            if samples.peek().is_some_and(|s| s.observed_at < interval.end) {
                return Err("rain sample is off the hourly grid".into());
            }
            let value = samples
                .next_if(|s| s.observed_at == interval.end)
                .and_then(|s| s.value)
                .map(|value| match value {
                    ObservationValue::RainMm(v) | ObservationValue::FlowM3s(v) => v,
                    ObservationValue::LevelCm(v) => f64::from(v),
                });
            points.push(SeriesPoint {
                timestamp: interval.end,
                value,
                time_segment: None,
            });
        }
        points
    } else {
        inputs
            .samples
            .iter()
            .filter(|s| {
                s.series_id == *series_id
                    && if matches!(variable, Variable::Rain) {
                        s.observed_at > window.start && s.observed_at <= window.end
                    } else {
                        s.observed_at >= window.start && s.observed_at <= window.end
                    }
            })
            .map(|s| SeriesPoint {
                timestamp: s.observed_at,
                value: s.value.map(|v| match v {
                    ObservationValue::RainMm(v) | ObservationValue::FlowM3s(v) => v,
                    ObservationValue::LevelCm(v) => f64::from(v),
                }),
                time_segment: None,
            })
            .collect()
    };
    Ok(ChartSeries {
        selection_index: 0,
        series_kind: if effective {
            "observed_effective"
        } else {
            "observed_raw"
        }
        .into(),
        run_id: None,
        scenario_id: None,
        forecast_reference_time: None,
        variable: name.into(),
        unit: unit.into(),
        points,
    })
}
pub fn query_series(request: &SeriesQuery) -> Result<SeriesReport, String> {
    query(request, None, None)
}

/// Compare model series for one MINI with observations from one station.
pub fn query_comparison(request: &SeriesQuery, station: &str) -> Result<SeriesReport, String> {
    query(request, Some(station), None)
}

pub(super) fn query(
    request: &SeriesQuery,
    comparison_station: Option<&str>,
    retained: Option<(&Path, &str, &ModelSource)>,
) -> Result<SeriesReport, String> {
    if comparison_station.is_some_and(|station| {
        !matches!(request.location, Location::Mini(_)) || station.trim().is_empty()
    }) {
        return Err("comparison requires a MINI and station ID".into());
    }
    match &request.location {
        Location::Station(id) if id.trim().is_empty() => {
            return Err("station ID must be nonempty".into());
        }
        Location::Mini(id) if *id <= 0 => return Err("MINI ID must be positive".into()),
        _ => {}
    }
    let window = resolve_hourly_window(&request.time_zone, &request.window)?;
    let mut report = SeriesReport {
        location: request.location.clone(),
        time_zone: request.time_zone.clone(),
        window: window.clone(),
        series: vec![],
        findings: vec![],
    };
    let mut mapping = None;
    let mut model_source = None;
    let observation_location = comparison_station.map(|id| Location::Station(id.into()));
    for (i, selection) in request.selections.iter().enumerate() {
        let result = match selection {
            SeriesSelection::Model {
                root,
                scenario_id,
                variable,
            } => {
                let source = if let Some((_, _, source)) =
                    retained.filter(|(path, id, _)| *path == root && *id == scenario_id)
                {
                    Ok(source)
                } else {
                    if model_source
                        .as_ref()
                        .is_none_or(|(path, scenario, _)| *path != root || *scenario != scenario_id)
                    {
                        drop(model_source.take());
                        model_source = Some((
                            root,
                            scenario_id,
                            read_model(
                                root,
                                scenario_id,
                                Some(&request.time_zone),
                                comparison_station.is_some(),
                            ),
                        ));
                    }
                    let (_, _, source) = model_source.as_ref().unwrap();
                    source.as_ref().map_err(Clone::clone)
                };
                source.and_then(|source| {
                    model(
                        source,
                        scenario_id,
                        variable,
                        &request.location,
                        comparison_station,
                        &window,
                    )
                })
            }
            SeriesSelection::Observation { .. } => observation(
                selection,
                observation_location.as_ref().unwrap_or(&request.location),
                &request.time_zone,
                &window,
            )
            .map(|s| (s, None)),
        };
        match result {
            Ok((mut series, new_mapping)) => {
                if new_mapping.is_some() && mapping.is_some() && mapping != new_mapping {
                    report.findings.push(QueryFinding {
                        selection_index: i,
                        message:
                            "station mini/offset or mini coordinates differ from earlier comparison"
                                .into(),
                    });
                    continue;
                }
                if new_mapping.is_some() {
                    mapping = new_mapping;
                }
                series.selection_index = i;
                report.series.push(series);
            }
            Err(message) => report.findings.push(QueryFinding {
                selection_index: i,
                message,
            }),
        }
    }
    Ok(report)
}

pub fn export_series_csv(
    request: &SeriesQuery,
    destination: &Path,
) -> Result<SeriesReport, String> {
    let report = query_series(request)?;
    write_series_csv(&report, None, destination)?;
    Ok(report)
}

/// Export the comparison snapshot currently displayed by the desktop.
pub fn export_comparison_csv(
    report: &SeriesReport,
    station: Option<&str>,
    destination: &Path,
) -> Result<(), String> {
    write_series_csv(report, station, destination)
}

fn write_series_csv(
    report: &SeriesReport,
    station: Option<&str>,
    destination: &Path,
) -> Result<(), String> {
    if !report.findings.is_empty() {
        let details = report
            .findings
            .iter()
            .map(|f| format!("selection {}: {}", f.selection_index, f.message))
            .collect::<Vec<_>>()
            .join("; ");
        return Err(format!("cannot export: {details}"));
    }
    let mut rows = Vec::new();
    for series in &report.series {
        for point in &series.points {
            rows.push((point.timestamp, series.selection_index, series, point));
        }
    }
    rows.sort_by_key(|r| (r.0, r.1));
    let mut csv = csv::Writer::from_writer(Vec::new());
    csv.write_record([
        "timestamp",
        "time_zone",
        "location_kind",
        "location_id",
        "series_kind",
        "run_id",
        "scenario_id",
        "forecast_reference_time",
        "variable",
        "unit",
        "value",
    ])
    .map_err(|e| e.to_string())?;
    let (kind, id) = match &report.location {
        Location::Station(id) => ("station", id.clone()),
        Location::Mini(id) => ("mini", id.to_string()),
    };
    for (_, _, s, p) in rows {
        let (kind, id) = match station {
            Some(station) if s.scenario_id.is_none() || s.series_kind == "model_station_level" => {
                ("station", station)
            }
            _ => (kind, id.as_str()),
        };
        csv.write_record([
            p.timestamp.format("%Y-%m-%dT%H:%M:%S").to_string(),
            report.time_zone.clone(),
            kind.into(),
            id.into(),
            s.series_kind.clone(),
            s.run_id.clone().unwrap_or_default(),
            s.scenario_id.clone().unwrap_or_default(),
            s.forecast_reference_time.clone().unwrap_or_default(),
            s.variable.clone(),
            s.unit.clone(),
            p.value.map(|v| v.to_string()).unwrap_or_default(),
        ])
        .map_err(|e| e.to_string())?;
    }
    let bytes = csv.into_inner().map_err(|e| e.to_string())?;
    crate::store::write_atomic(destination, &bytes)?;
    Ok(())
}
