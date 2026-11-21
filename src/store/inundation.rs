use super::{IntervalAxis, StoreError, TimeInterval};
use chrono::{Offset, Utc};
use serde::{Deserialize, Serialize};
use std::path::Path;
use terrio::{
    DataType,
    multidim::{Array, Dataset},
};

#[cfg(feature = "inundation-write")]
mod raster;
#[cfg(feature = "inundation-write")]
mod write;
#[cfg(feature = "inundation-write")]
pub(crate) use self::raster::InundationRasters;
#[cfg(feature = "inundation-write")]
pub(crate) use self::write::{InundationStage, InundationWriter};
#[cfg(feature = "inundation-write")]
pub(crate) const INUNDATION_TILE: usize = 1024;

pub const FLOOD_MASK_FILE: &str = "flood_mask.nc";
const SCHEMA_VERSION: i32 = 1;
const FILL: u8 = 255;
const CHUNK: usize = 256;
const TIME_UNITS: &str = "seconds since 1970-01-01 00:00:00 +00:00";
const CALENDAR: &str = "proleptic_gregorian";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InundationLevel {
    Full,
    Overview4,
    Overview16,
}

impl std::hash::Hash for InundationLevel {
    fn hash<H: std::hash::Hasher>(&self, hasher: &mut H) {
        hasher.write_usize(self.factor());
    }
}

impl InundationLevel {
    pub const fn factor(self) -> usize {
        match self {
            Self::Full => 1,
            Self::Overview4 => 4,
            Self::Overview16 => 16,
        }
    }

    const fn names(self) -> (&'static str, &'static str, &'static str) {
        match self {
            Self::Full => ("inundation_mask", "y", "x"),
            Self::Overview4 => ("inundation_mask_4x", "y_4x", "x_4x"),
            Self::Overview16 => ("inundation_mask_16x", "y_16x", "x_16x"),
        }
    }

    const fn maximum(self) -> u8 {
        match self {
            Self::Full => 1,
            Self::Overview4 | Self::Overview16 => 100,
        }
    }

    fn chunk(self, width: usize, height: usize) -> [usize; 3] {
        let span = match self {
            Self::Full | Self::Overview4 => CHUNK,
            Self::Overview16 => CHUNK / 4,
        };
        [1, height.min(span), width.min(span)]
    }
}

const LEVELS: [InundationLevel; 3] = [
    InundationLevel::Full,
    InundationLevel::Overview4,
    InundationLevel::Overview16,
];

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InundationProvenance {
    pub run_manifest_sha256: String,
    pub scenario: String,
    pub hand_sha256: String,
    pub catchments_sha256: String,
    pub flood_source_sha256: String,
    pub hand_sidecars: Vec<(String, String)>,
    pub catchments_sidecars: Vec<(String, String)>,
    pub vertical_datum: String,
    pub hand_vertical_shift_m: i32,
}

impl InundationProvenance {
    fn validate(&self) -> Result<(), StoreError> {
        let digest = |s: &String| {
            s.len() == 64
                && s.bytes()
                    .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
        };
        if self.vertical_datum != "Z0"
            || self.hand_vertical_shift_m != 0
            || self.scenario.is_empty()
            || ![
                &self.run_manifest_sha256,
                &self.hand_sha256,
                &self.catchments_sha256,
                &self.flood_source_sha256,
            ]
            .into_iter()
            .chain(
                self.hand_sidecars
                    .iter()
                    .chain(&self.catchments_sidecars)
                    .map(|(_, sha)| sha),
            )
            .all(digest)
        {
            return Err(schema("invalid flood-mask provenance"));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct InundationMetadata {
    pub width: usize,
    pub height: usize,
    pub epsg: i32,
    /// Northwest corner of the full-resolution grid.
    pub origin_x: f64,
    pub origin_y: f64,
    pub pixel_width: f64,
    /// Signed north-up pixel height.
    pub pixel_height: f64,
    pub timeline: IntervalAxis,
    pub historical_count: usize,
    pub provenance: InundationProvenance,
}

impl InundationMetadata {
    pub fn dimensions(&self, level: InundationLevel) -> (usize, usize) {
        let scale = level.factor();
        (self.width.div_ceil(scale), self.height.div_ceil(scale))
    }

    fn validate(&self) -> Result<(), StoreError> {
        validate_grid(
            self.width,
            self.height,
            self.epsg,
            [
                self.origin_x,
                self.pixel_width,
                0.,
                self.origin_y,
                0.,
                self.pixel_height,
            ],
        )?;
        if self.historical_count == 0 || self.historical_count >= self.timeline.bounds.len() {
            return Err(schema("invalid flood-mask history/forecast split"));
        }
        self.timeline.validate().map_err(format_error)?;
        self.provenance.validate()
    }

    // The outermost overview cell can cover less than its nominal footprint.
    fn coordinate(&self, level: InundationLevel, east: bool, index: usize) -> f64 {
        let factor = level.factor();
        let (origin, step, size) = if east {
            (self.origin_x, self.pixel_width, self.width)
        } else {
            (self.origin_y, self.pixel_height, self.height)
        };
        let start = index * factor;
        let len = factor.min(size - start);
        origin + (start as f64 + len as f64 / 2.0) * step
    }
}

fn validate_grid(
    width: usize,
    height: usize,
    epsg: i32,
    transform: [f64; 6],
) -> Result<(), StoreError> {
    let [west, dx, rotate_x, north, rotate_y, dy] = transform;
    let east = west + dx * width as f64;
    let south = north + dy * height as f64;
    if width == 0
        || height == 0
        || !matches!(epsg, 4326 | 4618)
        || !transform.into_iter().all(f64::is_finite)
        || rotate_x != 0.
        || rotate_y != 0.
        || dx <= 0.
        || dy >= 0.
        || west < -180.
        || east > 180.
        || east <= west
        || south < -90.
        || north > 90.
        || south >= north
    {
        return Err(schema(
            "expected a north-up geographic grid in EPSG:4618 or EPSG:4326",
        ));
    }
    Ok(())
}

/// Lazy reader for the retained scenario flood mask.
#[derive(Debug)]
pub struct InundationMask {
    dataset: Dataset,
    metadata: InundationMetadata,
}

impl InundationMask {
    pub fn open(path: &Path) -> Result<Self, StoreError> {
        let dataset = terrio::multidim::open(path_str(path)?).map_err(format_error)?;
        let attrs = &dataset.root.attributes;
        if attrs["schema_version"].as_i64() != Some(i64::from(SCHEMA_VERSION))
            || attrs["artifact_kind"].as_str() != Some("flood_mask")
            || attrs["Conventions"].as_str() != Some("CF-1.8")
        {
            return Err(schema("unsupported flood-mask schema"));
        }
        let full = dataset.array("inundation_mask").map_err(format_error)?;
        let [_, height, width] = full
            .shape()
            .try_into()
            .map_err(|_| schema("invalid flood-mask shape"))?;
        let projection = dataset.array("crs").map_err(format_error)?;
        if projection.info.attributes["grid_mapping_name"].as_str() != Some("latitude_longitude") {
            return Err(schema("invalid flood-mask grid mapping"));
        }
        let epsg = projection.info.attributes["epsg_code"]
            .as_i64()
            .and_then(|n| i32::try_from(n).ok())
            .ok_or_else(|| schema("missing flood-mask EPSG code"))?;
        let pixel = |key: &str| {
            attrs[key]
                .as_f64()
                .ok_or_else(|| schema(format!("missing flood-mask {key}")))
        };
        let metadata = InundationMetadata {
            width,
            height,
            epsg,
            origin_x: pixel("origin_x")?,
            origin_y: pixel("origin_y")?,
            pixel_width: pixel("pixel_width")?,
            pixel_height: pixel("pixel_height")?,
            timeline: read_timeline(&dataset)?,
            historical_count: attrs["historical_count"]
                .as_u64()
                .and_then(|n| usize::try_from(n).ok())
                .ok_or_else(|| schema("missing flood-mask history count"))?,
            provenance: serde_json::from_str(
                attrs["processing_json"]
                    .as_str()
                    .ok_or_else(|| schema("missing flood-mask provenance"))?,
            )
            .map_err(format_error)?,
        };
        metadata.validate()?;
        check_forecast_reference(&dataset, &metadata)?;
        for level in LEVELS {
            let (name, y, x) = level.names();
            let (width, height) = metadata.dimensions(level);
            let array = checked_array(
                &dataset,
                name,
                DataType::U8,
                &[
                    ("time", metadata.timeline.bounds.len()),
                    (y, height),
                    (x, width),
                ],
            )?;
            if array.info.attributes["_FillValue"].as_u64() != Some(u64::from(FILL))
                || array.info.attributes["grid_mapping"].as_str() != Some("crs")
                || array.info.chunks.as_deref() != Some(&level.chunk(width, height))
            {
                return Err(schema(format!("invalid {name} attributes or chunking")));
            }
            for (east, (coord, len)) in [(true, (x, width)), (false, (y, height))] {
                let coordinate = checked_array(&dataset, coord, DataType::F64, &[(coord, len)])?;
                for start in (0..len).step_by(CHUNK) {
                    let values = coordinate
                        .view()
                        .slice_axis(0, start, CHUNK.min(len - start), 1)
                        .and_then(|view| view.read())
                        .map_err(format_error)?
                        .bytes;
                    if values
                        .as_chunks::<8>()
                        .0
                        .iter()
                        .enumerate()
                        .any(|(i, bytes)| {
                            f64::from_le_bytes(*bytes)
                                != metadata.coordinate(level, east, start + i)
                        })
                    {
                        return Err(schema(format!("invalid {coord} coordinates")));
                    }
                }
            }
        }
        Ok(Self { dataset, metadata })
    }

    pub fn metadata(&self) -> &InundationMetadata {
        &self.metadata
    }

    /// Read one time step at the selected level, as row-major pixels.
    pub fn read_window(
        &self,
        level: InundationLevel,
        time: usize,
        x: usize,
        y: usize,
        width: usize,
        height: usize,
    ) -> Result<Vec<u8>, StoreError> {
        let (cols, rows) = self.metadata.dimensions(level);
        if !terrio::Window::new(x, y, width, height).fits(cols, rows) {
            return Err(StoreError::InvalidValue(
                "flood-mask window is outside the grid".into(),
            ));
        }
        let (name, _, _) = level.names();
        let bytes = self
            .dataset
            .array(name)
            .and_then(|array| array.view().isel_axis(0, time))
            .and_then(|view| view.slice_axis(0, y, height, 1))
            .and_then(|view| view.slice_axis(1, x, width, 1))
            .and_then(|view| view.read())
            .map_err(format_error)?
            .bytes;
        if bytes.iter().any(|&n| n != FILL && n > level.maximum()) {
            return Err(StoreError::Integrity(format!("invalid values in {name}")));
        }
        Ok(bytes)
    }
}

fn read_timeline(dataset: &Dataset) -> Result<IntervalAxis, StoreError> {
    let time = dataset.array("time").map_err(format_error)?;
    let [len] = time
        .shape()
        .try_into()
        .map_err(|_| schema("invalid flood-mask time shape"))?;
    let time = checked_array(dataset, "time", DataType::I64, &[("time", len)])?;
    if time.info.attributes["units"].as_str() != Some(TIME_UNITS)
        || time.info.attributes["calendar"].as_str() != Some(CALENDAR)
        || time.info.attributes["bounds"].as_str() != Some("time_bounds")
    {
        return Err(schema("invalid flood-mask time metadata"));
    }
    let bounds = checked_array(
        dataset,
        "time_bounds",
        DataType::I64,
        &[("time", len), ("bounds", 2)],
    )?
    .read()
    .map_err(format_error)?
    .bytes;
    let ends = time.read().map_err(format_error)?.bytes;
    let time_zone = dataset.root.attributes["operation_timezone"]
        .as_str()
        .ok_or_else(|| schema("missing flood-mask operation timezone"))?;
    let zone: chrono_tz::Tz = time_zone.parse().map_err(format_error)?;
    let local = |n| {
        let timestamp = chrono::DateTime::<Utc>::from_timestamp(n, 0)
            .ok_or_else(|| schema("flood-mask timestamp is out of range"))?
            .with_timezone(&zone);
        timestamp
            .naive_utc()
            .checked_add_offset(timestamp.offset().fix())
            .ok_or_else(|| schema("flood-mask local timestamp is out of range"))
    };
    let bounds = bounds
        .as_chunks::<16>()
        .0
        .iter()
        .zip(ends.as_chunks::<8>().0)
        .map(|(bytes, label)| {
            let [start, end] = bytes.as_chunks::<8>().0 else {
                unreachable!()
            };
            if label != end {
                return Err(schema("flood-mask time label differs from interval end"));
            }
            Ok(TimeInterval {
                start: local(i64::from_le_bytes(*start))?,
                end: local(i64::from_le_bytes(*end))?,
            })
        })
        .collect::<Result<_, StoreError>>()?;
    Ok(IntervalAxis {
        time_zone: time_zone.into(),
        bounds,
    })
}

fn check_forecast_reference(
    dataset: &Dataset,
    metadata: &InundationMetadata,
) -> Result<(), StoreError> {
    use chrono::TimeZone;

    let time = checked_array(dataset, "forecast_reference_time", DataType::I64, &[])?;
    if time.info.attributes["units"].as_str() != Some(TIME_UNITS)
        || time.info.attributes["calendar"].as_str() != Some(CALENDAR)
        || time.info.attributes["standard_name"].as_str() != Some("forecast_reference_time")
    {
        return Err(schema("invalid flood-mask forecast reference metadata"));
    }
    let bytes = time.read().map_err(format_error)?.bytes;
    let reference = i64::from_le_bytes(
        bytes
            .try_into()
            .map_err(|_| schema("invalid forecast reference size"))?,
    );
    let zone: chrono_tz::Tz = metadata.timeline.time_zone.parse().map_err(format_error)?;
    let expected = zone
        .from_local_datetime(&metadata.timeline.bounds[metadata.historical_count - 1].end)
        .single()
        .ok_or_else(|| schema("invalid forecast reference local time"))?
        .timestamp();
    if reference != expected {
        return Err(schema(
            "flood-mask forecast reference differs from the last historical interval",
        ));
    }
    Ok(())
}

fn checked_array(
    dataset: &Dataset,
    name: &str,
    kind: DataType,
    shape: &[(&str, usize)],
) -> Result<Array, StoreError> {
    let array = dataset.array(name).map_err(format_error)?;
    if array.info.data_type != kind
        || array
            .info
            .dimensions
            .iter()
            .map(|d| (d.name.as_str(), d.size))
            .ne(shape.iter().copied())
    {
        return Err(schema(format!("invalid {name} type or dimensions")));
    }
    Ok(array)
}

fn schema(message: impl Into<String>) -> StoreError {
    StoreError::Schema(message.into())
}

fn format_error(error: impl std::fmt::Display) -> StoreError {
    StoreError::Format(error.to_string())
}

fn path_str(path: &Path) -> Result<&str, StoreError> {
    path.to_str()
        .ok_or_else(|| StoreError::InvalidPath(format!("path is not UTF-8: {}", path.display())))
}
