use std::collections::{BTreeMap, HashSet};

use chrono::Duration;
use serde::{Deserialize, Serialize};

use crate::store::{MiniFlowExceedance, ResultSeries, TimeInterval};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FlowReturnPeriod {
    pub years: f64,
    pub flow_m3s: f64,
}

pub fn forecast_day_exceedance(
    results: &ResultSeries,
    window: &TimeInterval,
    thresholds: &BTreeMap<i64, Vec<FlowReturnPeriod>>,
) -> Result<Vec<MiniFlowExceedance>, String> {
    results.validate().map_err(|e| e.to_string())?;
    if window.end - window.start != Duration::hours(24) {
        return Err("return-period export requires a 24-hour forecast window".into());
    }
    let ids: HashSet<_> = results.basins.mini_ids.iter().copied().collect();
    for (mini, thresholds) in thresholds {
        if !ids.contains(mini)
            || thresholds.is_empty()
            || thresholds.iter().any(|v| {
                !v.years.is_finite()
                    || v.years <= 0.0
                    || !v.flow_m3s.is_finite()
                    || v.flow_m3s < 0.0
            })
            || thresholds
                .windows(2)
                .any(|v| v[0].years >= v[1].years || v[0].flow_m3s >= v[1].flow_m3s)
        {
            return Err(format!(
                "MINI {mini}: invalid or unordered return-period thresholds"
            ));
        }
    }
    let mut report: Vec<_> = results
        .basins
        .mini_ids
        .iter()
        .map(|mini_id| MiniFlowExceedance {
            mini_id: *mini_id,
            peak_m3s: None,
            peak_at: None,
            hours: 0,
            return_period_years: None,
            status: String::new(),
        })
        .collect();
    for (i, interval) in results.timeline.bounds.iter().enumerate() {
        if interval.end <= window.start || interval.start >= window.end {
            continue;
        }
        if interval.start < window.start
            || interval.end > window.end
            || interval.end - interval.start != Duration::hours(1)
            || results.time_segment[i] != 1
            || (interval.start - window.start).num_seconds() % 3600 != 0
        {
            return Err("return-period window contains an incompatible retained interval".into());
        }
        let row = i * report.len();
        for (j, mini) in report.iter_mut().enumerate() {
            if results.flow_valid[row + j] {
                let value = results.flow[row + j];
                mini.hours += 1;
                if mini.peak_m3s.is_none_or(|old| value > old) {
                    mini.peak_m3s = Some(value);
                    mini.peak_at = Some(interval.end);
                }
            }
        }
    }
    for mini in &mut report {
        let status = if mini.hours < 24 {
            "incomplete"
        } else if let Some(thresholds) = thresholds.get(&mini.mini_id) {
            mini.return_period_years = thresholds
                .iter()
                .rev()
                .find(|threshold| mini.peak_m3s.is_some_and(|peak| peak >= threshold.flow_m3s))
                .map(|threshold| threshold.years);
            if mini.return_period_years.is_some() {
                "reached"
            } else {
                "below_lowest"
            }
        } else {
            "unconfigured"
        };
        mini.status = status.into();
    }
    Ok(report)
}
