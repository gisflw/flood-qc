use std::collections::BTreeMap;
use std::path::PathBuf;

use chrono::{Duration, NaiveDateTime};
use serde::{Deserialize, Serialize};

use crate::analysis::{FlowReturnPeriod, forecast_day_exceedance};
use crate::store::{
    MiniFlowExceedance, TimeInterval, read_mini_map_layer, read_mini_topology,
    write_return_period_geojson,
};

use super::bulletin::{retained_result, validate_geometry};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReturnPeriodRequest {
    pub trial: PathBuf,
    pub scenario_id: String,
    pub geometry: PathBuf,
    pub topology: PathBuf,
    pub layer: String,
    pub thresholds: PathBuf,
    pub destination: PathBuf,
    pub forecast_day: u32,
}

#[derive(Clone, Debug, Serialize)]
pub struct ReturnPeriodExport {
    pub destination: PathBuf,
    pub start: NaiveDateTime,
    pub end: NaiveDateTime,
    pub time_zone: String,
    pub features: Vec<MiniFlowExceedance>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ThresholdFile {
    mini: Vec<MiniThresholds>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MiniThresholds {
    mini_id: i64,
    thresholds: Vec<FlowReturnPeriod>,
}

pub fn export_return_period(request: &ReturnPeriodRequest) -> Result<ReturnPeriodExport, String> {
    let day = request
        .forecast_day
        .checked_sub(1)
        .ok_or("forecast_day must be 1 or greater")?;
    let input: ThresholdFile =
        toml::from_str(&std::fs::read_to_string(&request.thresholds).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    let mut thresholds = BTreeMap::new();
    for mini in input.mini {
        if thresholds.insert(mini.mini_id, mini.thresholds).is_some() {
            return Err(format!("duplicate thresholds for MINI {}", mini.mini_id));
        }
    }
    if thresholds.is_empty() {
        return Err("select thresholds for at least one MINI".into());
    }
    let (model, reference) = retained_result(&request.trial, &request.scenario_id)?;
    let results = &model.basin;
    let start = reference
        .checked_add_signed(Duration::days(i64::from(day)))
        .ok_or("forecast day overflow")?;
    let end = start
        .checked_add_signed(Duration::days(1))
        .ok_or("forecast day overflow")?;
    let window = TimeInterval { start, end };
    let (geometry, topology) = read_mini_topology(&request.topology)?;
    validate_geometry(results, &geometry)?;
    let geometry = read_mini_map_layer(&request.geometry, &request.layer, &topology)?;
    let features = forecast_day_exceedance(results, &window, &thresholds)?;
    write_return_period_geojson(
        &request.destination,
        &geometry,
        &features,
        &window,
        &results.timeline.time_zone,
    )?;
    Ok(ReturnPeriodExport {
        destination: request.destination.clone(),
        start,
        end,
        time_zone: results.timeline.time_zone.clone(),
        features,
    })
}
