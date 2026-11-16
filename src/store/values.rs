use chrono::{Duration, NaiveDateTime, Offset, TimeZone, Timelike};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValueError(pub String);
impl std::fmt::Display for ValueError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for ValueError {}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Provenance {
    pub source_ids: Vec<String>,
    pub processing: serde_json::Value,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeInterval {
    pub start: NaiveDateTime,
    pub end: NaiveDateTime,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IntervalAxis {
    pub time_zone: String,
    pub bounds: Vec<TimeInterval>,
}
impl IntervalAxis {
    pub fn validate(&self) -> Result<(), ValueError> {
        let zone = self
            .time_zone
            .parse::<chrono_tz::Tz>()
            .map_err(|_| ValueError("invalid operation timezone".into()))?;
        if self.bounds.is_empty() {
            return Err(ValueError("empty interval axis".into()));
        }
        let mut previous = None;
        let mut offset = None;
        for interval in &self.bounds {
            if interval.start >= interval.end || previous.is_some_and(|end| interval.start < end) {
                return Err(ValueError("unordered or overlapping intervals".into()));
            }
            let mut time = interval.start;
            loop {
                if time.nanosecond() != 0 {
                    return Err(ValueError("subsecond local timestamp".into()));
                }
                let local = zone.from_local_datetime(&time).single().ok_or_else(|| {
                    ValueError(format!("ambiguous/nonexistent local timestamp {time}"))
                })?;
                let seconds = local.offset().fix().local_minus_utc();
                if offset.is_some_and(|old| old != seconds) {
                    return Err(ValueError(
                        "timezone-offset transitions are unqualified".into(),
                    ));
                }
                offset = Some(seconds);
                if time == interval.end {
                    break;
                }
                time = time
                    .checked_add_signed(Duration::hours(1))
                    .map_or(interval.end, |next| next.min(interval.end));
            }
            previous = Some(interval.end);
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RegularGridSpec {
    pub requested_bbox: [f64; 4],
    pub resolution_degrees: f64,
    pub latitudes: Vec<f64>,
    pub longitudes: Vec<f64>,
    pub effective_bbox: [f64; 4],
}
fn bbox_valid(b: &[f64; 4]) -> bool {
    b.iter().all(|v| v.is_finite())
        && b[0] < b[2]
        && b[1] < b[3]
        && b[0] >= -180.0
        && b[2] <= 180.0
        && b[1] >= -90.0
        && b[3] <= 90.0
}
impl RegularGridSpec {
    pub fn new(requested_bbox: [f64; 4], resolution_degrees: f64) -> Result<Self, ValueError> {
        if !bbox_valid(&requested_bbox)
            || !resolution_degrees.is_finite()
            || resolution_degrees <= 0.0
        {
            return Err(ValueError("invalid grid bounds/resolution".into()));
        }
        let axis = |lower: f64, upper: f64| -> Result<Vec<f64>, ValueError> {
            let count = ((upper - lower) / resolution_degrees).floor() + 2.0;
            if !count.is_finite() || count >= usize::MAX as f64 {
                return Err(ValueError("grid dimension overflow".into()));
            }
            let mut values = Vec::new();
            values
                .try_reserve_exact(count as usize)
                .map_err(|_| ValueError("grid allocation failed".into()))?;
            for i in 0..count as usize {
                values.push(lower - resolution_degrees / 2.0 + i as f64 * resolution_degrees);
            }
            Ok(values)
        };
        let longitudes = axis(requested_bbox[0], requested_bbox[2])?;
        let latitudes = axis(requested_bbox[1], requested_bbox[3])?;
        let effective_bbox = [
            longitudes[0] - resolution_degrees / 2.0,
            latitudes[0] - resolution_degrees / 2.0,
            longitudes[longitudes.len() - 1] + resolution_degrees / 2.0,
            latitudes[latitudes.len() - 1] + resolution_degrees / 2.0,
        ];
        let grid = Self {
            requested_bbox,
            resolution_degrees,
            latitudes,
            longitudes,
            effective_bbox,
        };
        grid.validate()?;
        Ok(grid)
    }
    pub fn validate(&self) -> Result<(), ValueError> {
        let r = self.resolution_degrees;
        if !bbox_valid(&self.requested_bbox)
            || !r.is_finite()
            || r <= 0.0
            || !self.effective_bbox.iter().all(|v| v.is_finite())
        {
            return Err(ValueError("invalid grid metadata".into()));
        }
        for axis in [&self.latitudes, &self.longitudes] {
            if axis.is_empty()
                || !axis.iter().all(|v| v.is_finite())
                || axis
                    .windows(2)
                    .any(|p| p[1] <= p[0] || ((p[1] - p[0]) - r).abs() > r * 1e-9)
            {
                return Err(ValueError("invalid regular grid coordinates".into()));
            }
        }
        let expected = [
            self.longitudes[0] - r / 2.0,
            self.latitudes[0] - r / 2.0,
            self.longitudes[self.longitudes.len() - 1] + r / 2.0,
            self.latitudes[self.latitudes.len() - 1] + r / 2.0,
        ];
        if expected
            .iter()
            .zip(self.effective_bbox)
            .any(|(a, b)| (a - b).abs() > r * 1e-9)
        {
            return Err(ValueError(
                "grid footprint disagrees with coordinates".into(),
            ));
        }
        self.latitudes
            .len()
            .checked_mul(self.longitudes.len())
            .ok_or_else(|| ValueError("grid shape overflow".into()))?;
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct IdwSettings {
    pub nearest_stations: usize,
    pub power: f64,
}
impl IdwSettings {
    pub fn validate(&self) -> Result<(), ValueError> {
        if self.nearest_stations == 0 || !self.power.is_finite() || self.power <= 0.0 {
            Err(ValueError("invalid IDW settings".into()))
        } else {
            Ok(())
        }
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BasinGeometry {
    pub mini_ids: Vec<i64>,
    pub longitudes: Vec<f64>,
    pub latitudes: Vec<f64>,
    pub source: Provenance,
}
impl BasinGeometry {
    pub fn validate(&self) -> Result<(), ValueError> {
        let mut ids = HashSet::new();
        if self.mini_ids.is_empty()
            || self.mini_ids.len() != self.longitudes.len()
            || self.mini_ids.len() != self.latitudes.len()
            || self.mini_ids.iter().any(|id| *id <= 0 || !ids.insert(*id))
            || self
                .longitudes
                .iter()
                .any(|v| !v.is_finite() || !(-180.0..=180.0).contains(v))
            || self
                .latitudes
                .iter()
                .any(|v| !v.is_finite() || !(-90.0..=90.0).contains(v))
        {
            Err(ValueError("invalid basin IDs/coordinates".into()))
        } else {
            Ok(())
        }
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GridSeries {
    pub timeline: IntervalAxis,
    pub grid: RegularGridSpec,
    pub values: Vec<f64>,
    pub valid: Vec<bool>,
    pub provenance: Provenance,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BasinForcing {
    pub timeline: IntervalAxis,
    pub basins: BasinGeometry,
    pub values: Vec<f64>,
    pub valid: Vec<bool>,
    pub provenance: Provenance,
}
fn validate_values(
    values: &[f64],
    valid: &[bool],
    length: Option<usize>,
) -> Result<(), ValueError> {
    if length != Some(values.len())
        || values.len() != valid.len()
        || values
            .iter()
            .zip(valid)
            .any(|(v, m)| v.is_infinite() || (*m && !v.is_finite()))
    {
        Err(ValueError("invalid precipitation shape/mask/value".into()))
    } else {
        Ok(())
    }
}
impl GridSeries {
    pub fn validate(&self) -> Result<(), ValueError> {
        self.timeline.validate()?;
        self.grid.validate()?;
        validate_values(
            &self.values,
            &self.valid,
            self.timeline
                .bounds
                .len()
                .checked_mul(self.grid.latitudes.len())
                .and_then(|v| v.checked_mul(self.grid.longitudes.len())),
        )
    }
}
impl BasinForcing {
    pub fn validate(&self) -> Result<(), ValueError> {
        self.timeline.validate()?;
        self.basins.validate()?;
        validate_values(
            &self.values,
            &self.valid,
            self.timeline
                .bounds
                .len()
                .checked_mul(self.basins.mini_ids.len()),
        )
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GridFrame {
    pub interval: TimeInterval,
    pub time_zone: String,
    pub grid: RegularGridSpec,
    pub values: Vec<f64>,
    pub valid: Vec<bool>,
    pub provenance: Provenance,
}
impl GridFrame {
    pub fn validate(&self) -> Result<(), ValueError> {
        IntervalAxis {
            time_zone: self.time_zone.clone(),
            bounds: vec![self.interval.clone()],
        }
        .validate()?;
        self.grid.validate()?;
        validate_values(
            &self.values,
            &self.valid,
            self.grid
                .latitudes
                .len()
                .checked_mul(self.grid.longitudes.len()),
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ForecastProvider {
    Ecmwf,
    Noaa,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ForecastSource {
    pub source_id: String,
    pub provider: ForecastProvider,
    pub model: String,
    pub product: String,
    pub cycle_utc: chrono::DateTime<chrono::Utc>,
    pub sha256: String,
    pub byte_length: u64,
    pub normalization_json: serde_json::Value,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ForecastCorrection {
    pub correction_id: String,
    pub asset_id: String,
    pub t0_hour: i64,
    pub t1_hour: i64,
    pub shift_lat_pixels: f64,
    pub shift_lon_pixels: f64,
    pub rotation_deg: f64,
    pub multiplication_factor: f64,
    pub editor: Option<String>,
    #[serde(default)]
    pub reason: String,
    pub created_at: String,
}
#[derive(Clone, Debug, PartialEq)]
pub struct CorrectionCoverage {
    pub correction_id: String,
    pub position: usize,
    pub native_interval_indices: Vec<usize>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct ForecastPreparation {
    pub transformed: Vec<GridFrame>,
    pub working: GridSeries,
    pub basin: BasinForcing,
    pub correction_coverage: Vec<CorrectionCoverage>,
}

/// Canonical basin results, ordered by time and then by basin.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ResultSeries {
    pub timeline: IntervalAxis,
    pub basins: BasinGeometry,
    pub time_segment: Vec<u8>,
    pub precipitation: Vec<f64>,
    pub precipitation_valid: Vec<bool>,
    pub flow: Vec<f64>,
    pub flow_valid: Vec<bool>,
    pub raw_level_cm: Vec<i32>,
    pub raw_level_valid: Vec<bool>,
    pub provenance: Provenance,
}

impl ResultSeries {
    pub fn validate(&self) -> Result<(), ValueError> {
        self.timeline.validate()?;
        self.basins.validate()?;
        let times = self.timeline.bounds.len();
        let count = times.checked_mul(self.basins.mini_ids.len());
        validate_values(&self.precipitation, &self.precipitation_valid, count)?;
        validate_values(&self.flow, &self.flow_valid, count)?;
        if self
            .precipitation
            .iter()
            .zip(&self.precipitation_valid)
            .any(|(value, valid)| *valid && *value < 0.0)
        {
            return Err(ValueError("negative result precipitation".into()));
        }
        if count != Some(self.raw_level_cm.len())
            || self.raw_level_valid.len() != self.raw_level_cm.len()
            || self
                .raw_level_cm
                .iter()
                .zip(&self.raw_level_valid)
                .any(|(v, valid)| !valid && *v != 0)
            || self.time_segment.len() != times
            || self.time_segment.iter().any(|segment| *segment > 1)
            || self.time_segment.windows(2).any(|pair| pair[0] > pair[1])
        {
            return Err(ValueError(
                "invalid result shape, segment or level mask".into(),
            ));
        }
        Ok(())
    }
}

/// Station levels retain requested order and unknown reference mappings.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StationResults {
    pub timeline: IntervalAxis,
    pub time_segment: Vec<u8>,
    pub station_ids: Vec<String>,
    pub mini_ids: Vec<Option<i64>>,
    pub offsets_m: Vec<Option<f64>>,
    pub level_cm: Vec<i32>,
    pub level_valid: Vec<bool>,
    pub provenance: Provenance,
}

impl StationResults {
    pub fn validate(&self) -> Result<(), ValueError> {
        self.timeline.validate()?;
        let times = self.timeline.bounds.len();
        let stations = self.station_ids.len();
        let mut ids = HashSet::new();
        if self.time_segment.len() != times
            || self.time_segment.iter().any(|segment| *segment > 1)
            || self.time_segment.windows(2).any(|pair| pair[0] > pair[1])
            || self
                .station_ids
                .iter()
                .any(|id| id.is_empty() || !ids.insert(id))
            || self.mini_ids.len() != stations
            || self.offsets_m.len() != stations
            || times.checked_mul(stations) != Some(self.level_cm.len())
            || self.level_valid.len() != self.level_cm.len()
        {
            return Err(ValueError(
                "invalid station result shape or identities".into(),
            ));
        }
        for (index, (mini, offset)) in self.mini_ids.iter().zip(&self.offsets_m).enumerate() {
            match (mini, offset) {
                (Some(id), Some(value)) if *id > 0 && value.is_finite() => {}
                (None, None) => {
                    if (0..times).any(|t| self.level_valid[t * stations + index]) {
                        return Err(ValueError("unmapped station has valid level".into()));
                    }
                }
                _ => return Err(ValueError("invalid station offset mapping".into())),
            }
        }
        if self
            .level_cm
            .iter()
            .zip(&self.level_valid)
            .any(|(v, valid)| !valid && *v != 0)
        {
            return Err(ValueError("invalid station level payload".into()));
        }
        Ok(())
    }
}
