use crate::store::{BasinForcing, BasinGeometry, IntervalAxis, Provenance};
use serde::{Deserialize, Serialize};
use std::{error::Error, fmt};

#[derive(Debug)]
pub enum ForecastPreparationError {
    InvalidInput(String),
    SourceMismatch(String),
    InvalidCorrection(String),
    AmbiguousInterval(String),
    MissingTimeCoverage(String),
    MissingSpatialCoverage(String),
    InvalidGeometry(String),
}
impl fmt::Display for ForecastPreparationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl Error for ForecastPreparationError {}

pub fn prepare_zero_forecast(
    timeline: &IntervalAxis,
    basins: &BasinGeometry,
) -> Result<BasinForcing, ForecastPreparationError> {
    timeline
        .validate()
        .map_err(|e| ForecastPreparationError::InvalidInput(e.to_string()))?;
    basins
        .validate()
        .map_err(|e| ForecastPreparationError::InvalidGeometry(e.to_string()))?;
    let count = timeline
        .bounds
        .len()
        .checked_mul(basins.mini_ids.len())
        .ok_or_else(|| ForecastPreparationError::InvalidInput("forcing shape overflow".into()))?;
    let mut values = Vec::new();
    values
        .try_reserve_exact(count)
        .map_err(|e| ForecastPreparationError::InvalidInput(e.to_string()))?;
    values.resize(count, 0.0);
    let mut valid = Vec::new();
    valid
        .try_reserve_exact(count)
        .map_err(|e| ForecastPreparationError::InvalidInput(e.to_string()))?;
    valid.resize(count, true);
    Ok(BasinForcing {
        timeline: timeline.clone(),
        basins: basins.clone(),
        values,
        valid,
        provenance: Provenance {
            source_ids: vec![],
            processing: serde_json::Value::String("zero forecast".into()),
        },
    })
}

#[derive(Clone, Debug)]
pub struct ForecastPreparationRequest {
    pub timeline: IntervalAxis,
    pub working_grid: crate::store::RegularGridSpec,
    pub basins: BasinGeometry,
    pub interpolation: crate::store::IdwSettings,
}

fn corrected_frame(
    mut frame: crate::store::GridFrame,
    c: &crate::store::ForecastCorrection,
) -> Result<crate::store::GridFrame, ForecastPreparationError> {
    use crate::store::RegularGridSpec;
    let fail = |s: &str| ForecastPreparationError::InvalidCorrection(s.into());
    let r = frame.grid.resolution_degrees;
    let dx = c.shift_lon_pixels.round_ties_even() * r;
    let dy = c.shift_lat_pixels.round_ties_even() * r;
    if !dx.is_finite() || !dy.is_finite() {
        return Err(fail("translation overflow"));
    }
    if c.rotation_deg.abs() < 1e-12 {
        for x in &mut frame.grid.longitudes {
            *x += dx;
        }
        for y in &mut frame.grid.latitudes {
            *y += dy;
        }
        for (i, v) in frame.grid.effective_bbox.iter_mut().enumerate() {
            *v += if i % 2 == 0 { dx } else { dy };
        }
        frame.grid.requested_bbox = frame.grid.effective_bbox;
        for (v, m) in frame.values.iter_mut().zip(&frame.valid) {
            if *m {
                *v = (*v * c.multiplication_factor).max(0.);
                if !v.is_finite() {
                    return Err(fail("rainfall multiplication overflow"));
                }
            }
        }
        frame.validate().map_err(|e| fail(&e.to_string()))?;
        return Ok(frame);
    }
    let [w, s, e, n] = frame.grid.effective_bbox;
    let cx = (w + e) / 2.;
    let cy = (s + n) / 2.;
    let theta = c.rotation_deg.to_radians();
    let (sin, cos) = theta.sin_cos();
    let corners = [(w, s), (w, n), (e, s), (e, n)];
    let mut west = f64::INFINITY;
    let mut east = f64::NEG_INFINITY;
    let mut south = f64::INFINITY;
    let mut north = f64::NEG_INFINITY;
    for (x, y) in corners {
        let x = x + dx - cx;
        let y = y + dy - cy;
        let xx = cx + x * cos - y * sin;
        let yy = cy + x * sin + y * cos;
        west = west.min(xx);
        east = east.max(xx);
        south = south.min(yy);
        north = north.max(yy);
    }
    west = (west / r).floor() * r;
    east = (east / r).ceil() * r;
    south = (south / r).floor() * r;
    north = (north / r).ceil() * r;
    if ![west, east, south, north].iter().all(|x| x.is_finite())
        || west < -180.
        || east > 180.
        || south < -90.
        || north > 90.
    {
        return Err(fail("rotated footprint outside supported geometry"));
    }
    let nx = ((east - west) / r).round();
    let ny = ((north - south) / r).round();
    if nx < 1. || ny < 1. || nx >= usize::MAX as f64 || ny >= usize::MAX as f64 {
        return Err(fail("invalid rotated grid size"));
    }
    let nx = nx as usize;
    let ny = ny as usize;
    let len = nx
        .checked_mul(ny)
        .ok_or_else(|| fail("rotated grid size overflow"))?;
    let mut values = Vec::new();
    values
        .try_reserve_exact(len)
        .map_err(|_| fail("rotated grid allocation failed"))?;
    let mut valid = Vec::new();
    valid
        .try_reserve_exact(len)
        .map_err(|_| fail("rotated mask allocation failed"))?;
    let longitudes: Vec<f64> = (0..nx).map(|i| west + r / 2. + i as f64 * r).collect();
    let latitudes: Vec<f64> = (0..ny).map(|i| south + r / 2. + i as f64 * r).collect();
    let old_width = frame.grid.longitudes.len();
    for y in &latitudes {
        for x in &longitudes {
            let xx = x - cx;
            let yy = y - cy;
            let sx = cx + xx * cos + yy * sin - dx;
            let sy = cy - xx * sin + yy * cos - dy;
            let col = ((sx - frame.grid.longitudes[0]) / r).round_ties_even();
            let row = ((sy - frame.grid.latitudes[0]) / r).round_ties_even();
            if row >= 0.
                && col >= 0.
                && row < frame.grid.latitudes.len() as f64
                && col < old_width as f64
            {
                let i = row as usize * old_width + col as usize;
                if frame.valid[i] {
                    let v = (frame.values[i] * c.multiplication_factor).max(0.);
                    if !v.is_finite() {
                        return Err(fail("rainfall multiplication overflow"));
                    }
                    values.push(v);
                    valid.push(true);
                    continue;
                }
            }
            values.push(f64::NAN);
            valid.push(false);
        }
    }
    frame.grid = RegularGridSpec {
        requested_bbox: [west, south, east, north],
        resolution_degrees: r,
        latitudes,
        longitudes,
        effective_bbox: [west, south, east, north],
    };
    frame.values = values;
    frame.valid = valid;
    frame.validate().map_err(|e| fail(&e.to_string()))?;
    Ok(frame)
}

pub fn corrected_forecast_frames(
    native: &crate::store::GridSeries,
    corrections: &[crate::store::ForecastCorrection],
    used: &[usize],
) -> Result<
    (
        Vec<crate::store::GridFrame>,
        Vec<crate::store::CorrectionCoverage>,
    ),
    ForecastPreparationError,
> {
    use crate::store::{CorrectionCoverage, GridFrame};
    let invalid = |s: &str| ForecastPreparationError::InvalidInput(s.into());
    native.validate().map_err(|e| invalid(&e.to_string()))?;
    if native
        .timeline
        .bounds
        .windows(2)
        .any(|v| v[1].start < v[0].end)
    {
        return Err(ForecastPreparationError::AmbiguousInterval(
            "overlapping native intervals".into(),
        ));
    }
    if native
        .values
        .iter()
        .zip(&native.valid)
        .any(|(v, valid)| *valid && *v < 0.0)
    {
        return Err(invalid("negative forecast rainfall"));
    }
    if used.iter().any(|i| *i >= native.timeline.bounds.len()) {
        return Err(invalid("native interval index out of range"));
    }
    for &index in used {
        let interval = &native.timeline.bounds[index];
        let seconds = (interval.end - interval.start).num_seconds();
        if seconds <= 0 || seconds % 3600 != 0 {
            return Err(invalid("native duration must be divisible by an hour"));
        }
    }
    let cycle = if corrections.is_empty() {
        None
    } else {
        let value = native
            .provenance
            .processing
            .get("normalization")
            .and_then(|v| v.get("cycle_utc"))
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| {
                ForecastPreparationError::SourceMismatch("missing forecast cycle".into())
            })?;
        let utc = chrono::DateTime::parse_from_rfc3339(value)
            .map_err(|e| ForecastPreparationError::SourceMismatch(e.to_string()))?;
        let zone = native
            .timeline
            .time_zone
            .parse::<chrono_tz::Tz>()
            .map_err(|e| ForecastPreparationError::SourceMismatch(e.to_string()))?;
        Some(utc.with_timezone(&zone).naive_local())
    };
    let expected_asset = native
        .provenance
        .processing
        .get("asset_id")
        .and_then(serde_json::Value::as_str);
    let mut ids = std::collections::HashSet::new();
    for c in corrections {
        if c.correction_id.is_empty()
            || !ids.insert(&c.correction_id)
            || expected_asset != Some(c.asset_id.as_str())
            || c.t0_hour < 0
            || c.t1_hour < c.t0_hour
            || ![
                c.shift_lat_pixels,
                c.shift_lon_pixels,
                c.rotation_deg,
                c.multiplication_factor,
            ]
            .iter()
            .all(|x| x.is_finite())
            || c.multiplication_factor <= 0.
        {
            return Err(ForecastPreparationError::InvalidCorrection(format!(
                "invalid correction {}",
                c.correction_id
            )));
        }
    }
    let mut coverage: Vec<CorrectionCoverage> = corrections
        .iter()
        .enumerate()
        .map(|(position, c)| CorrectionCoverage {
            correction_id: c.correction_id.clone(),
            position,
            native_interval_indices: Vec::new(),
        })
        .collect();
    let cells = native
        .grid
        .latitudes
        .len()
        .checked_mul(native.grid.longitudes.len())
        .ok_or_else(|| invalid("native shape overflow"))?;
    let mut transformed = Vec::new();
    for &i in used {
        let b = &native.timeline.bounds[i];
        let mut frame = GridFrame {
            interval: b.clone(),
            time_zone: native.timeline.time_zone.clone(),
            grid: native.grid.clone(),
            values: native.values[i * cells..(i + 1) * cells].to_vec(),
            valid: native.valid[i * cells..(i + 1) * cells].to_vec(),
            provenance: native.provenance.clone(),
        };
        let mut applied = 0;
        for (j, c) in corrections.iter().enumerate() {
            let start = cycle
                .unwrap()
                .checked_add_signed(
                    chrono::Duration::try_hours(c.t0_hour)
                        .ok_or_else(|| invalid("correction hour overflow"))?,
                )
                .ok_or_else(|| invalid("correction hour overflow"))?;
            let end = cycle
                .unwrap()
                .checked_add_signed(
                    chrono::Duration::try_hours(c.t1_hour)
                        .ok_or_else(|| invalid("correction hour overflow"))?,
                )
                .ok_or_else(|| invalid("correction hour overflow"))?;
            if b.start >= start && b.end <= end {
                frame = corrected_frame(frame, c)?;
                coverage[j].native_interval_indices.push(i);
                applied += 1;
            }
        }
        if let Some(map) = frame.provenance.processing.as_object_mut() {
            map.insert("correction_count".into(), applied.into());
        }
        transformed.push(frame);
    }
    Ok((transformed, coverage))
}

pub(crate) fn forecast_interval_selection(
    native: &IntervalAxis,
    target_axis: &IntervalAxis,
) -> Result<Vec<usize>, ForecastPreparationError> {
    let invalid = |s: &str| ForecastPreparationError::InvalidInput(s.into());
    if native.bounds.windows(2).any(|v| v[1].start < v[0].end) {
        return Err(ForecastPreparationError::AmbiguousInterval(
            "overlapping native intervals".into(),
        ));
    }
    if native.time_zone != target_axis.time_zone {
        return Err(ForecastPreparationError::SourceMismatch(
            "operation timezone mismatch".into(),
        ));
    }
    let mut selected = Vec::new();
    for target in &target_axis.bounds {
        if (target.end - target.start) != chrono::Duration::hours(1) {
            return Err(invalid("target intervals must be hourly"));
        }
        let matching = native
            .bounds
            .iter()
            .position(|b| b.start <= target.start && b.end >= target.end);
        let i = matching.ok_or_else(|| {
            ForecastPreparationError::MissingTimeCoverage(format!(
                "target interval ending at {}",
                target.end
            ))
        })?;
        let seconds = (native.bounds[i].end - native.bounds[i].start).num_seconds();
        if seconds <= 0 || seconds % 3600 != 0 {
            return Err(invalid("native duration must be divisible by an hour"));
        }
        selected.push(i);
    }
    Ok(selected)
}

pub fn prepare_forecast(
    native: &crate::store::GridSeries,
    corrections: &[crate::store::ForecastCorrection],
    request: &ForecastPreparationRequest,
) -> Result<crate::store::ForecastPreparation, ForecastPreparationError> {
    use crate::store::{ForecastPreparation, GridSeries};
    let invalid = |s: &str| ForecastPreparationError::InvalidInput(s.into());
    let selected = forecast_interval_selection(&native.timeline, &request.timeline)?;
    native.validate().map_err(|e| invalid(&e.to_string()))?;
    request
        .timeline
        .validate()
        .map_err(|e| invalid(&e.to_string()))?;
    request
        .working_grid
        .validate()
        .map_err(|e| invalid(&e.to_string()))?;
    request
        .basins
        .validate()
        .map_err(|e| ForecastPreparationError::InvalidGeometry(e.to_string()))?;
    request
        .interpolation
        .validate()
        .map_err(|e| invalid(&e.to_string()))?;
    if native
        .values
        .iter()
        .zip(&native.valid)
        .any(|(v, m)| *m && *v < 0.)
    {
        return Err(invalid("negative forecast rainfall"));
    }
    let mut used = Vec::new();
    for &index in &selected {
        if !used.contains(&index) {
            used.push(index);
        }
    }
    let (transformed, coverage) = corrected_forecast_frames(native, corrections, &used)?;
    let mut values = Vec::new();
    let mut valid = Vec::new();
    for (target_index, &source_index) in selected.iter().enumerate() {
        let pos = used
            .iter()
            .position(|i| *i == source_index)
            .ok_or_else(|| invalid("missing transformed frame"))?;
        let frame = &transformed[pos];
        let hours = (frame.interval.end - frame.interval.start).num_seconds() as f64 / 3600.;
        let scaled: Vec<f64> = frame.values.iter().map(|v| v / hours).collect();
        let (v, m) = super::spatial::bilinear_resample(
            &scaled,
            &frame.valid,
            &frame.grid.latitudes,
            &frame.grid.longitudes,
            &request.working_grid.latitudes,
            &request.working_grid.longitudes,
        )
        .map_err(|e| ForecastPreparationError::MissingSpatialCoverage(e.to_string()))?;
        if m.iter().any(|v| !*v) {
            return Err(ForecastPreparationError::MissingSpatialCoverage(format!(
                "target interval {target_index}"
            )));
        }
        values.extend(v);
        valid.extend(m);
    }
    use sha2::{Digest, Sha256};
    let mut provenance = native.provenance.clone();
    let mut recipe = serde_json::Map::new();
    recipe.insert("parent".into(), provenance.processing.clone());
    recipe.insert("correction_count".into(), corrections.len().into());
    recipe.insert(
        "target_axis".into(),
        serde_json::to_value(&request.timeline).map_err(|e| invalid(&e.to_string()))?,
    );
    recipe.insert(
        "requested_bbox".into(),
        serde_json::to_value(request.working_grid.requested_bbox)
            .map_err(|e| invalid(&e.to_string()))?,
    );
    recipe.insert(
        "effective_bbox".into(),
        serde_json::to_value(request.working_grid.effective_bbox)
            .map_err(|e| invalid(&e.to_string()))?,
    );
    recipe.insert(
        "resolution_degrees".into(),
        request.working_grid.resolution_degrees.into(),
    );
    recipe.insert(
        "idw_settings".into(),
        serde_json::to_value(&request.interpolation).map_err(|e| invalid(&e.to_string()))?,
    );
    let geometry = serde_json::to_vec(&request.basins).map_err(|e| invalid(&e.to_string()))?;
    recipe.insert(
        "geometry_sha256".into(),
        format!("{:x}", Sha256::digest(&geometry)).into(),
    );
    provenance.processing = recipe.into();
    let working = GridSeries {
        timeline: request.timeline.clone(),
        grid: request.working_grid.clone(),
        values,
        valid,
        provenance,
    };
    working.validate().map_err(|e| invalid(&e.to_string()))?;
    let basin = super::spatial::interpolate_grid_to_basins(
        &working,
        &request.basins,
        &request.interpolation,
    )
    .map_err(|e| ForecastPreparationError::MissingSpatialCoverage(e.to_string()))?;
    Ok(ForecastPreparation {
        transformed,
        working,
        basin,
        correction_coverage: coverage,
    })
}

/// Resample native accumulations into hourly values on the workspace grid.
pub fn forecast_working_grid(
    native: &crate::store::GridSeries,
    timeline: &IntervalAxis,
    grid: &crate::store::RegularGridSpec,
) -> Result<crate::store::GridSeries, ForecastPreparationError> {
    let selected = forecast_interval_selection(&native.timeline, timeline)?;
    let mut used = Vec::new();
    for &index in &selected {
        if !used.contains(&index) {
            used.push(index);
        }
    }
    let (frames, _) = corrected_forecast_frames(native, &[], &used)?;
    let mut values = Vec::new();
    let mut valid = Vec::new();
    let mut prepared_frames = std::collections::HashMap::new();
    for index in used.iter().copied() {
        let frame = &frames[used.iter().position(|v| *v == index).unwrap()];
        let hours = (frame.interval.end - frame.interval.start).num_seconds() as f64 / 3600.;
        let scaled: Vec<_> = frame.values.iter().map(|v| v / hours).collect();
        let (v, m) = super::spatial::bilinear_resample(
            &scaled,
            &frame.valid,
            &frame.grid.latitudes,
            &frame.grid.longitudes,
            &grid.latitudes,
            &grid.longitudes,
        )
        .map_err(|e| ForecastPreparationError::MissingSpatialCoverage(e.to_string()))?;
        if m.iter().any(|v| !*v) {
            return Err(ForecastPreparationError::MissingSpatialCoverage(
                "working grid".into(),
            ));
        }
        prepared_frames.insert(index, (v, m));
    }
    for index in selected {
        let (v, m) = prepared_frames.get(&index).expect("selected native frame");
        values.extend_from_slice(v);
        valid.extend_from_slice(m);
    }
    let result = crate::store::GridSeries {
        timeline: timeline.clone(),
        grid: grid.clone(),
        values,
        valid,
        provenance: native.provenance.clone(),
    };
    result
        .validate()
        .map_err(|e| ForecastPreparationError::InvalidInput(e.to_string()))?;
    Ok(result)
}

/// An ordered edit to hourly rainfall on the workspace grid.
/// The bounding box selects output cell centres; the transform samples the
/// previous grid, so successive edits compose in their given order.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScenarioGridEdit {
    pub start: chrono::NaiveDateTime,
    pub end: chrono::NaiveDateTime,
    pub bbox: Option<[f64; 4]>,
    #[serde(default = "default_scenario_multiplier")]
    pub multiplication_factor: f64,
    #[serde(default)]
    pub rotation_deg: f64,
    #[serde(default)]
    pub shift_lon_pixels: f64,
    #[serde(default)]
    pub shift_lat_pixels: f64,
}
fn default_scenario_multiplier() -> f64 {
    1.0
}

impl ScenarioGridEdit {
    pub fn validate(&self) -> Result<(), String> {
        use chrono::Timelike;
        if self.start >= self.end
            || [self.start, self.end]
                .iter()
                .any(|time| time.minute() != 0 || time.second() != 0 || time.nanosecond() != 0)
            || ![
                self.multiplication_factor,
                self.rotation_deg,
                self.shift_lon_pixels,
                self.shift_lat_pixels,
            ]
            .iter()
            .all(|v| v.is_finite())
            || self.multiplication_factor < 0.0
            || self.bbox.is_some_and(|b| {
                !b.iter().all(|v| v.is_finite())
                    || b[0] >= b[2]
                    || b[1] >= b[3]
                    || b[0] < -180.
                    || b[2] > 180.
                    || b[1] < -90.
                    || b[3] > 90.
            })
        {
            return Err("invalid scenario edit bounds or values".into());
        }
        Ok(())
    }
}

/// Apply half-open, ordered scenario edits to an already-hourly working grid.
pub fn apply_scenario_grid_edits(
    source: &crate::store::GridSeries,
    edits: &[ScenarioGridEdit],
) -> Result<crate::store::GridSeries, ForecastPreparationError> {
    let invalid = |message: &str| ForecastPreparationError::InvalidCorrection(message.into());
    source.validate().map_err(|e| invalid(&e.to_string()))?;
    if source
        .timeline
        .bounds
        .iter()
        .any(|b| b.end - b.start != chrono::Duration::hours(1))
    {
        return Err(invalid("scenario grid must be hourly"));
    }
    let grid = &source.grid;
    let width = grid.longitudes.len();
    let height = grid.latitudes.len();
    let cells = width
        .checked_mul(height)
        .ok_or_else(|| invalid("grid shape overflow"))?;
    let mut result = source.clone();
    for edit in edits {
        let bbox = edit.bbox.unwrap_or(grid.effective_bbox);
        edit.validate().map_err(|e| invalid(&e))?;
        if source.timeline.bounds.iter().any(|b| {
            (b.start < edit.start && edit.start < b.end) || (b.start < edit.end && edit.end < b.end)
        }) {
            return Err(invalid("edit boundaries must align with hourly intervals"));
        }
        let (sin, cos) = edit.rotation_deg.to_radians().sin_cos();
        let cx = (bbox[0] + bbox[2]) / 2.0;
        let cy = (bbox[1] + bbox[3]) / 2.0;
        let resolution = grid.resolution_degrees;
        let dx = edit.shift_lon_pixels * resolution;
        let dy = edit.shift_lat_pixels * resolution;
        if !dx.is_finite() || !dy.is_finite() {
            return Err(invalid("scenario translation overflow"));
        }
        for (time, interval) in source.timeline.bounds.iter().enumerate() {
            if interval.start < edit.start || interval.end > edit.end {
                continue;
            }
            let offset = time * cells;
            let previous_values = result.values[offset..offset + cells].to_vec();
            let previous_valid = result.valid[offset..offset + cells].to_vec();
            for (row, y) in grid.latitudes.iter().enumerate() {
                for (column, x) in grid.longitudes.iter().enumerate() {
                    if *x < bbox[0] || *x > bbox[2] || *y < bbox[1] || *y > bbox[3] {
                        continue;
                    }
                    // Inverse of translation followed by rotation about the fixed box centre.
                    let x = x - cx;
                    let y = y - cy;
                    let sx = cx + x * cos + y * sin - dx;
                    let sy = cy - x * sin + y * cos - dy;
                    let source_col = ((sx - grid.longitudes[0]) / resolution).round_ties_even();
                    let source_row = ((sy - grid.latitudes[0]) / resolution).round_ties_even();
                    let target = offset + row * width + column;
                    if source_col >= 0.0
                        && source_col < width as f64
                        && source_row >= 0.0
                        && source_row < height as f64
                    {
                        let index = source_row as usize * width + source_col as usize;
                        result.valid[target] = previous_valid[index];
                        if previous_valid[index] {
                            let value = previous_values[index] * edit.multiplication_factor;
                            if !value.is_finite() {
                                return Err(invalid("scenario multiplication overflow"));
                            }
                            result.values[target] = value;
                        } else {
                            result.values[target] = f64::NAN;
                        }
                    } else {
                        result.values[target] = f64::NAN;
                        result.valid[target] = false;
                    }
                }
            }
        }
    }
    result.validate().map_err(|e| invalid(&e.to_string()))?;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::{ForecastCorrection, GridFrame, RegularGridSpec, TimeInterval};
    fn sample() -> GridFrame {
        let start = chrono::NaiveDate::from_ymd_opt(2026, 7, 20)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        GridFrame {
            interval: TimeInterval {
                start,
                end: start + chrono::Duration::hours(3),
            },
            time_zone: "UTC".into(),
            grid: RegularGridSpec {
                requested_bbox: [0., 0., 0.5, 0.5],
                resolution_degrees: 0.25,
                latitudes: vec![0.125, 0.375],
                longitudes: vec![0.125, 0.375],
                effective_bbox: [0., 0., 0.5, 0.5],
            },
            values: vec![1., 2., 3., 4.],
            valid: vec![true; 4],
            provenance: Provenance {
                source_ids: vec![],
                processing: serde_json::Value::Null,
            },
        }
    }
    fn correction() -> ForecastCorrection {
        ForecastCorrection {
            correction_id: "a".into(),
            asset_id: "x".into(),
            t0_hour: 0,
            t1_hour: 3,
            shift_lat_pixels: 0.,
            shift_lon_pixels: 0.,
            rotation_deg: 0.,
            multiplication_factor: 1.,
            editor: None,
            reason: "test".into(),
            created_at: "2026-07-20T00:00:00Z".into(),
        }
    }
    #[test]
    fn scenario_edits_are_hourly_ordered_and_box_selects_output() {
        let frame = sample();
        let start = frame.interval.start;
        let source = crate::store::GridSeries {
            timeline: crate::store::IntervalAxis {
                time_zone: frame.time_zone,
                bounds: (0..2)
                    .map(|i| crate::store::TimeInterval {
                        start: start + chrono::Duration::hours(i),
                        end: start + chrono::Duration::hours(i + 1),
                    })
                    .collect(),
            },
            grid: frame.grid,
            values: vec![1., 2., 3., 4., 1., 2., 3., 4.],
            valid: vec![true; 8],
            provenance: frame.provenance,
        };
        let edit = ScenarioGridEdit {
            start,
            end: start + chrono::Duration::hours(1),
            bbox: Some([0., 0., 0.25, 0.5]),
            multiplication_factor: 2.,
            rotation_deg: 0.,
            shift_lon_pixels: 0.,
            shift_lat_pixels: 0.,
        };
        let mut second = edit.clone();
        second.multiplication_factor = 3.;
        let result = apply_scenario_grid_edits(&source, &[edit, second]).unwrap();
        assert_eq!(result.values, [6., 2., 18., 4., 1., 2., 3., 4.]);
        assert_eq!(source.values[0], 1.);
        let rotation = ScenarioGridEdit {
            start,
            end: start + chrono::Duration::hours(1),
            bbox: None,
            multiplication_factor: 1.,
            rotation_deg: 90.,
            shift_lon_pixels: 0.,
            shift_lat_pixels: 0.,
        };
        let rotated = apply_scenario_grid_edits(&source, std::slice::from_ref(&rotation)).unwrap();
        let mut combined = rotation;
        combined.shift_lon_pixels = 1.0;
        let moved = apply_scenario_grid_edits(&source, &[combined]).unwrap();
        assert_eq!(&moved.valid[..4], &[false, false, true, true]);
        assert_eq!(&moved.values[2..4], &[3.0, 1.0]);
        assert_eq!(&rotated.values[..4], &[3., 1., 4., 2.]);
        assert_eq!(&rotated.values[4..], &[1., 2., 3., 4.]);
        let invalid = ScenarioGridEdit {
            start,
            end: start + chrono::Duration::minutes(30),
            bbox: None,
            multiplication_factor: 1.,
            rotation_deg: 0.,
            shift_lon_pixels: 0.,
            shift_lat_pixels: 0.,
        };
        assert!(apply_scenario_grid_edits(&source, &[invalid]).is_err());
    }

    #[test]
    fn preview_rejects_invalid_native_values_and_intervals() {
        let frame = sample();
        let mut native = crate::store::GridSeries {
            timeline: crate::store::IntervalAxis {
                time_zone: frame.time_zone,
                bounds: vec![frame.interval],
            },
            grid: frame.grid,
            values: frame.values,
            valid: frame.valid,
            provenance: frame.provenance,
        };
        native.values[0] = -1.0;
        let negative = corrected_forecast_frames(&native, &[], &[0]).is_err();
        assert!(negative);
        native.values[0] = 1.0;
        native.timeline.bounds[0].end =
            native.timeline.bounds[0].start + chrono::Duration::minutes(30);
        let interval = corrected_forecast_frames(&native, &[], &[0]).is_err();
        assert!(interval);
    }
    #[test]
    fn ties_even_translation_and_rotation() {
        let mut c = correction();
        c.shift_lat_pixels = 2.5;
        c.shift_lon_pixels = 3.5;
        let shifted = corrected_frame(sample(), &c).unwrap();
        assert_eq!(shifted.grid.latitudes, [0.625, 0.875]);
        assert_eq!(shifted.grid.longitudes, [1.125, 1.375]);
        assert_eq!(shifted.values, [1., 2., 3., 4.]);
        let mut c = correction();
        c.rotation_deg = 90.;
        let rotated = corrected_frame(sample(), &c).unwrap();
        assert!(rotated.grid.latitudes.len() >= 2);
        assert!(rotated.grid.longitudes.len() >= 2);
        let present: Vec<f64> = rotated
            .values
            .iter()
            .zip(&rotated.valid)
            .filter_map(|(v, m)| m.then_some(*v))
            .collect();
        assert_eq!(present, [3., 1., 4., 2.]);
        let mut shift = correction();
        shift.shift_lon_pixels = 1.;
        let mut rotate = correction();
        rotate.rotation_deg = 90.;
        let first = corrected_frame(corrected_frame(sample(), &shift).unwrap(), &rotate).unwrap();
        let second = corrected_frame(corrected_frame(sample(), &rotate).unwrap(), &shift).unwrap();
        assert_ne!(first.grid.effective_bbox, second.grid.effective_bbox);
    }
}
