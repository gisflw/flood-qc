use crate::analysis::upstream_minis;
use crate::store::{IntervalAxis, MiniTopology, ResultSeries, TimeInterval};
use chrono::{Duration, NaiveDateTime, Timelike};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum HourlyWindow {
    Explicit {
        start: NaiveDateTime,
        end: NaiveDateTime,
    },
    Relative {
        end: NaiveDateTime,
        hours: u32,
    },
}

pub fn resolve_hourly_window(zone: &str, window: &HourlyWindow) -> Result<TimeInterval, String> {
    let (start, end) = match *window {
        HourlyWindow::Explicit { start, end } => (start, end),
        HourlyWindow::Relative { end, hours } => (
            end.checked_sub_signed(Duration::hours(i64::from(hours)))
                .ok_or("window overflow")?,
            end,
        ),
    };
    if start >= end
        || [start, end]
            .iter()
            .any(|t| t.minute() != 0 || t.second() != 0 || t.nanosecond() != 0)
    {
        return Err("window must have ordered whole-hour bounds".into());
    }
    let interval = TimeInterval { start, end };
    IntervalAxis {
        time_zone: zone.into(),
        bounds: vec![interval.clone()],
    }
    .validate()
    .map_err(|e| e.to_string())?;
    Ok(interval)
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct RainSummary {
    pub window: TimeInterval,
    pub total_area_km2: f64,
    pub complete_area_km2: f64,
    pub covered_area_hours: f64,
    pub expected_area_hours: f64,
    pub mean_mm: Option<f64>,
}

pub fn accumulated_rain(
    results: &ResultSeries,
    topology: &[MiniTopology],
    outlet: i64,
    upstream: bool,
    window: &HourlyWindow,
) -> Result<RainSummary, String> {
    results.validate().map_err(|e| e.to_string())?;
    let window = resolve_hourly_window(&results.timeline.time_zone, window)?;
    let ids = if upstream {
        upstream_minis(topology, outlet)?
    } else {
        crate::store::validate_mini_topology(topology)?;
        vec![outlet]
    };
    let hours = (window.end - window.start).num_hours() as f64;
    let mut area = 0.0;
    let mut complete_area = 0.0;
    let mut covered = 0.0;
    let mut weighted = 0.0;
    let mut complete = true;
    for id in ids {
        let node = topology
            .iter()
            .find(|n| n.mini_id == id)
            .ok_or_else(|| format!("unknown MINI {id}"))?;
        area += node.area_km2;
        let Some(index) = results.basins.mini_ids.iter().position(|v| *v == id) else {
            complete = false;
            continue;
        };
        let mut sum = 0.0;
        let mut count = 0;
        for (t, interval) in results.timeline.bounds.iter().enumerate() {
            if interval.end <= window.start || interval.start >= window.end {
                continue;
            }
            if interval.start < window.start
                || interval.end > window.end
                || interval.end - interval.start != Duration::hours(1)
                || interval.start.minute() != 0
                || interval.start.second() != 0
            {
                return Err("rainfall contains a partial or non-hourly native interval".into());
            }
            let cell = t * results.basins.mini_ids.len() + index;
            if results.precipitation_valid[cell] {
                sum += results.precipitation[cell];
                count += 1;
            }
        }
        if !sum.is_finite() {
            return Err("rainfall accumulation overflow".into());
        }
        covered += node.area_km2 * f64::from(count);
        if f64::from(count) == hours {
            complete_area += node.area_km2;
            weighted += node.area_km2 * sum;
        } else {
            complete = false;
        }
    }
    if !area.is_finite()
        || area <= 0.0
        || !complete_area.is_finite()
        || !covered.is_finite()
        || !(area * hours).is_finite()
        || !weighted.is_finite()
    {
        return Err("rainfall accumulation overflow".into());
    }
    Ok(RainSummary {
        window,
        total_area_km2: area,
        complete_area_km2: complete_area,
        covered_area_hours: covered,
        expected_area_hours: area * hours,
        mean_mm: complete.then_some(weighted / area),
    })
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct GridAccumulation {
    pub window: TimeInterval,
    pub values: Vec<Option<f64>>,
    pub covered_hours: Vec<f64>,
    pub expected_hours: f64,
}

pub fn accumulated_grid_rain(
    grid: &crate::store::GridSeries,
    window: &HourlyWindow,
) -> Result<GridAccumulation, String> {
    grid.validate().map_err(|e| e.to_string())?;
    let window = resolve_hourly_window(&grid.timeline.time_zone, window)?;
    let cells = grid.grid.latitudes.len() * grid.grid.longitudes.len();
    let mut sums = vec![0.0; cells];
    let mut coverage = vec![0.0; cells];
    for (t, interval) in grid.timeline.bounds.iter().enumerate() {
        if interval.end <= window.start || interval.start >= window.end {
            continue;
        }
        if interval.start.minute() != 0
            || interval.start.second() != 0
            || interval.end.minute() != 0
            || interval.end.second() != 0
        {
            return Err("native rainfall interval has non-hourly bounds".into());
        }
        if interval.start < window.start || interval.end > window.end {
            continue;
        }
        let hours = (interval.end - interval.start).num_hours() as f64;
        for i in 0..cells {
            let j = t * cells + i;
            if grid.valid[j] {
                sums[i] += grid.values[j];
                coverage[i] += hours;
            }
        }
    }
    let expected = (window.end - window.start).num_hours() as f64;
    if sums.iter().chain(&coverage).any(|v| !v.is_finite()) {
        return Err("rainfall accumulation overflow".into());
    }
    let values = sums
        .into_iter()
        .zip(&coverage)
        .map(|(sum, covered)| (*covered == expected).then_some(sum))
        .collect();
    Ok(GridAccumulation {
        window,
        values,
        covered_hours: coverage,
        expected_hours: expected,
    })
}

pub fn accumulated_grid_frame(
    frame: &crate::store::GridFrame,
    window: &HourlyWindow,
) -> Result<GridAccumulation, String> {
    frame.validate().map_err(|e| e.to_string())?;
    accumulated_grid_rain(
        &crate::store::GridSeries {
            timeline: IntervalAxis {
                time_zone: frame.time_zone.clone(),
                bounds: vec![frame.interval.clone()],
            },
            grid: frame.grid.clone(),
            values: frame.values.clone(),
            valid: frame.valid.clone(),
            provenance: frame.provenance.clone(),
        },
        window,
    )
}
