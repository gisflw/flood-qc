use std::collections::BTreeMap;
use std::path::Path;

use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

use crate::analysis::{
    HydrographAdjustment, HydrographPeak, HydrographPoint, HydrographThreshold, ObservationBias,
    analyze_model_hydrograph, rating_value, validate_hydrograph,
};
use crate::store::{TimeInterval, write_atomic};

use super::series::{ModelSource, query};
use super::{Location, QueryFinding, SeriesQuery};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HydrographRequest {
    pub series: SeriesQuery,
    pub comparison_station: Option<String>,
    #[serde(default)]
    pub adjustment: HydrographAdjustment,
    #[serde(default)]
    pub thresholds: Vec<HydrographThreshold>,
}

#[derive(Clone, Debug, Serialize)]
pub struct HydrographCurve {
    pub kind: String,
    pub run_id: Option<String>,
    pub scenario_id: Option<String>,
    pub reference_time: Option<NaiveDateTime>,
    pub bias_anchor: Option<NaiveDateTime>,
    pub source_variable: String,
    pub source_unit: String,
    pub variable: String,
    pub unit: String,
    pub points: Vec<HydrographPoint>,
    pub peak: Option<HydrographPeak>,
}

#[derive(Clone, Debug, Serialize)]
pub struct HydrographReport {
    pub location: Location,
    pub comparison_station: Option<String>,
    pub time_zone: String,
    pub window: TimeInterval,
    pub adjustment: HydrographAdjustment,
    pub thresholds: Vec<HydrographThreshold>,
    pub curves: Vec<HydrographCurve>,
    pub findings: Vec<QueryFinding>,
}

pub fn analyze_hydrograph(request: &HydrographRequest) -> Result<HydrographReport, String> {
    analyze_hydrograph_from_model(request, None)
}

pub(super) fn analyze_hydrograph_from_model(
    request: &HydrographRequest,
    retained: Option<(&Path, &str, &ModelSource)>,
) -> Result<HydrographReport, String> {
    let adjustment = &request.adjustment;
    validate_hydrograph(adjustment, &request.thresholds)?;
    let report = query(
        &request.series,
        request.comparison_station.as_deref(),
        retained,
    )?;
    let mut observed = BTreeMap::new();
    if adjustment.bias != ObservationBias::None {
        for series in report.series.iter().filter(|s| s.run_id.is_none()) {
            let values = series
                .points
                .iter()
                .filter_map(|point| point.value.map(|value| (point.timestamp, value)))
                .collect();
            if observed.insert(series.variable.as_str(), values).is_some() {
                return Err(format!(
                    "select one observation series for {} bias",
                    series.variable
                ));
            }
        }
    }

    let mut curves = Vec::new();
    let mut findings = report.findings;
    for series in &report.series {
        let flow_to_level = series.variable == "flow";
        let convert = !adjustment.rating_curve.is_empty();
        let (variable, unit) = match (series.variable.as_str(), convert) {
            ("flow", true) => ("level", "cm"),
            ("level", true) => ("flow", "m3/s"),
            ("rain", _) => return Err("use rainfall analysis for rain series".into()),
            (variable, false) => (variable, series.unit.as_str()),
            _ => return Err("unsupported rating curve variable".into()),
        };
        let mut points: Vec<_> = series
            .points
            .iter()
            .map(|point| HydrographPoint {
                timestamp: point.timestamp,
                raw: point.value,
                value: None,
                lower: None,
                upper: None,
                segment: point.time_segment,
            })
            .collect();
        let reference = series
            .forecast_reference_time
            .as_ref()
            .map(|value| value.parse::<NaiveDateTime>().map_err(|e| e.to_string()))
            .transpose()?;
        let (points, peak, bias_anchor) = if let Some(reference) = reference {
            let empty = BTreeMap::new();
            let observations = observed.get(series.variable.as_str()).unwrap_or(&empty);
            let result = analyze_model_hydrograph(
                points,
                observations,
                reference,
                flow_to_level,
                adjustment,
                &request.thresholds,
            )?;
            if adjustment.bias != ObservationBias::None && result.2.is_none() {
                findings.push(QueryFinding {
                    selection_index: series.selection_index,
                    message: "No eligible observation/model overlap for requested bias".into(),
                });
            }
            result
        } else {
            for point in &mut points {
                point.value = point
                    .raw
                    .and_then(|value| rating_value(value, flow_to_level, &adjustment.rating_curve));
            }
            (points, None, None)
        };
        curves.push(HydrographCurve {
            kind: series.series_kind.clone(),
            run_id: series.run_id.clone(),
            scenario_id: series.scenario_id.clone(),
            reference_time: reference,
            bias_anchor,
            source_variable: series.variable.clone(),
            source_unit: series.unit.clone(),
            variable: variable.into(),
            unit: unit.into(),
            points,
            peak,
        });
    }
    if (!request.thresholds.is_empty() || !adjustment.rating_curve.is_empty())
        && let Some(first) = curves.first()
        && curves.iter().any(|curve| curve.variable != first.variable)
    {
        return Err("thresholds and rating curves require one output variable".into());
    }

    Ok(HydrographReport {
        location: report.location,
        comparison_station: request.comparison_station.clone(),
        time_zone: report.time_zone,
        window: report.window,
        adjustment: adjustment.clone(),
        thresholds: request.thresholds.clone(),
        curves,
        findings,
    })
}

pub fn export_hydrograph(report: &HydrographReport, path: &Path) -> Result<(), String> {
    match path.extension().and_then(|s| s.to_str()) {
        Some("csv") => write_atomic(path, &hydrograph_csv(report)?),
        Some("png") => write_atomic(path, &hydrograph_png(report)?),
        _ => Err("hydrograph export requires a .csv or .png destination".into()),
    }
}

pub(super) fn hydrograph_csv(report: &HydrographReport) -> Result<Vec<u8>, String> {
    let mut csv = csv::Writer::from_writer(Vec::new());
    csv.write_record([
        "series",
        "run",
        "scenario",
        "reference_time",
        "timestamp",
        "segment",
        "raw_variable",
        "raw_unit",
        "raw",
        "variable",
        "unit",
        "value",
        "lower",
        "upper",
    ])
    .map_err(|e| e.to_string())?;
    for curve in &report.curves {
        for point in &curve.points {
            csv.serialize((
                &curve.kind,
                &curve.run_id,
                &curve.scenario_id,
                curve.reference_time,
                point.timestamp,
                point.segment,
                &curve.source_variable,
                &curve.source_unit,
                point.raw,
                &curve.variable,
                &curve.unit,
                point.value,
                point.lower,
                point.upper,
            ))
            .map_err(|e| e.to_string())?;
        }
    }
    csv.into_inner().map_err(|e| e.to_string())
}

pub(super) fn hydrograph_png(report: &HydrographReport) -> Result<Vec<u8>, String> {
    let unit = report
        .curves
        .first()
        .ok_or("no series to plot")?
        .unit
        .as_str();
    if report.curves.iter().any(|curve| curve.unit != unit) {
        return Err("PNG export requires one unit per hydrograph".into());
    }
    let curves: Vec<_> = report
        .curves
        .iter()
        .map(|curve| {
            let label = match (&curve.run_id, &curve.scenario_id) {
                (Some(run), Some(scenario)) => format!("{run} · {scenario}"),
                _ => "Observado".into(),
            };
            (label, curve.points.as_slice())
        })
        .collect();
    crate::analysis::render_hydrograph_png(&curves, unit, &report.window, &report.thresholds)
}
