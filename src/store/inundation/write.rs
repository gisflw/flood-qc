use super::{
    CALENDAR, CHUNK, FILL, INUNDATION_TILE, InundationLevel, InundationMask, InundationMetadata,
    LEVELS, SCHEMA_VERSION, TIME_UNITS, format_error, schema,
};
use crate::store::{StoreError, sync_directory};
use chrono::TimeZone;
use netcdf::FileMut;
use std::{
    fs::{self, File, OpenOptions, TryLockError},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};
use terrio::Window;

pub(crate) struct InundationStage {
    _lock: File,
    directory: Option<PathBuf>,
}

impl InundationStage {
    pub(crate) fn lock(results: &Path, scenario: &str) -> Result<Self, StoreError> {
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(results.join(format!(".inundation-{scenario}.lock")))?;
        lock.try_lock().map_err(|error| match error {
            TryLockError::WouldBlock => StoreError::Integrity(format!(
                "inundation processing is already active for scenario {scenario}"
            )),
            TryLockError::Error(error) => StoreError::Io(error),
        })?;
        Ok(Self {
            _lock: lock,
            directory: None,
        })
    }

    pub(crate) fn temporary_file(&mut self, parent: &Path) -> Result<PathBuf, StoreError> {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let dir = parent.join(format!(
            ".inundation-{}-{}-{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&dir)?;
        let file = dir.join(super::FLOOD_MASK_FILE);
        self.directory = Some(dir);
        Ok(file)
    }

    pub(crate) fn publish(
        &self,
        staged: &Path,
        destination: &Path,
        replace: bool,
        check_cancelled: impl FnOnce() -> Result<(), StoreError>,
    ) -> Result<(), StoreError> {
        let parent = destination
            .parent()
            .ok_or_else(|| schema("flood mask has no parent"))?;
        File::open(staged)?.sync_all()?;
        // Keep the original inode until directory synchronization succeeds.
        let original = staged.with_file_name("previous_flood_mask.nc");
        if replace {
            fs::hard_link(destination, &original)?;
        }
        check_cancelled()?;
        if replace {
            replace_file(staged, destination)?;
        } else {
            fs::hard_link(staged, destination)?;
        }
        if let Err(error) = sync_directory(parent) {
            if replace {
                replace_file(&original, destination)?;
            } else {
                fs::remove_file(destination)?;
            }
            return Err(format_error(error));
        }
        Ok(())
    }
}

fn replace_file(from: &Path, to: &Path) -> Result<(), StoreError> {
    #[cfg(windows)]
    atomicwrites::replace_atomic(from, to)?;
    #[cfg(not(windows))]
    fs::rename(from, to)?;
    Ok(())
}

impl Drop for InundationStage {
    fn drop(&mut self) {
        if let Some(directory) = &self.directory {
            let _ = fs::remove_dir_all(directory);
        }
    }
}

pub(crate) struct InundationWriter(FileMut);

impl InundationWriter {
    pub(crate) fn create(path: &Path, meta: &InundationMetadata) -> Result<Self, StoreError> {
        meta.validate()?;
        build_file(path, meta).map(Self).map_err(format_error)
    }

    pub(crate) fn write(
        &mut self,
        level: InundationLevel,
        time: usize,
        window: Window,
        data: &[u8],
    ) -> Result<(), StoreError> {
        let (name, _, _) = level.names();
        self.0
            .variable_mut(name)
            .ok_or_else(|| schema(format!("missing {name}")))?
            .put_values(
                data,
                ([time, window.y, window.x], [1, window.height, window.width]),
            )
            .map_err(format_error)
    }

    pub(crate) fn close(self) -> Result<(), StoreError> {
        self.0.close().map_err(format_error)
    }
}

impl InundationMask {
    pub(crate) fn validate_payload(
        &self,
        mut cancelled: impl FnMut() -> Result<(), StoreError>,
    ) -> Result<(), StoreError> {
        let times = self.metadata.timeline.bounds.len();
        let (width, height) = self.metadata.dimensions(InundationLevel::Full);
        for y in (0..height).step_by(INUNDATION_TILE) {
            let rows = INUNDATION_TILE.min(height - y);
            for x in (0..width).step_by(INUNDATION_TILE) {
                let columns = INUNDATION_TILE.min(width - x);
                for time in 0..times {
                    cancelled()?;
                    for level in LEVELS {
                        let factor = level.factor();
                        self.read_window(
                            level,
                            time,
                            x / factor,
                            y / factor,
                            columns.div_ceil(factor),
                            rows.div_ceil(factor),
                        )?;
                    }
                }
            }
        }
        Ok(())
    }
}

fn build_file(
    path: &Path,
    meta: &InundationMetadata,
) -> Result<FileMut, Box<dyn std::error::Error>> {
    let mut file = netcdf::create(path)?;
    file.add_attribute("Conventions", "CF-1.8")?;
    file.add_attribute("schema_version", SCHEMA_VERSION)?;
    file.add_attribute("artifact_kind", "flood_mask")?;
    file.add_attribute("processing_json", serde_json::to_string(&meta.provenance)?)?;
    file.add_attribute("operation_timezone", meta.timeline.time_zone.as_str())?;
    file.add_attribute("historical_count", i64::try_from(meta.historical_count)?)?;
    file.add_attribute("origin_x", meta.origin_x)?;
    file.add_attribute("origin_y", meta.origin_y)?;
    file.add_attribute("pixel_width", meta.pixel_width)?;
    file.add_attribute("pixel_height", meta.pixel_height)?;
    file.add_dimension("time", meta.timeline.bounds.len())?;
    file.add_dimension("bounds", 2)?;
    let mut crs = file.add_variable::<i32>("crs", &[])?;
    crs.put_attribute("grid_mapping_name", "latitude_longitude")?;
    crs.put_attribute("longitude_of_prime_meridian", 0.0_f64)?;
    crs.put_attribute("epsg_code", meta.epsg)?;
    let srs = terrio::crs::SpatialRef::from_epsg(meta.epsg as u32)?;
    let wkt = srs.to_wkt().ok_or("missing source CRS WKT")?;
    crs.put_attribute("crs_wkt", wkt.as_str())?;
    let (semi_major, inverse_flattening) = match meta.epsg {
        4326 => (6_378_137.0_f64, 298.257_223_563_f64),
        4618 => (6_378_160.0_f64, 298.25_f64),
        _ => return Err("unsupported geographic CRS".into()),
    };
    crs.put_attribute("semi_major_axis", semi_major)?;
    crs.put_attribute("inverse_flattening", inverse_flattening)?;
    crs.put_value(0_i32, ())?;
    write_time(&mut file, meta)?;
    for level in LEVELS {
        let (name, y, x) = level.names();
        let (width, height) = meta.dimensions(level);
        file.add_dimension(y, height)?;
        file.add_dimension(x, width)?;
        let mut mask = file.add_variable::<u8>(name, &["time", y, x])?;
        mask.set_fill_value(FILL)?;
        mask.set_chunking(&level.chunk(width, height))?;
        mask.set_compression(4, false)?;
        mask.put_attribute("grid_mapping", "crs")?;
        mask.put_attribute("coordinates", "forecast_reference_time")?;
        mask.put_attribute("valid_range", vec![0_u8, level.maximum()])?;
        if level == InundationLevel::Full {
            mask.put_attribute("long_name", "inundation extent")?;
            mask.put_attribute("units", "1")?;
            mask.put_attribute("flag_values", vec![0_u8, 1_u8])?;
            mask.put_attribute("flag_meanings", "dry wet")?;
        } else {
            mask.put_attribute("long_name", "percentage of block pixels classified as wet")?;
            mask.put_attribute("units", "%")?;
        }
        for (coord, east, len) in [(x, true, width), (y, false, height)] {
            let mut variable = file.add_variable::<f64>(coord, &[coord])?;
            let (axis, standard_name, units) = if east {
                ("X", "longitude", "degrees_east")
            } else {
                ("Y", "latitude", "degrees_north")
            };
            variable.put_attribute("axis", axis)?;
            variable.put_attribute("standard_name", standard_name)?;
            variable.put_attribute("units", units)?;
            for start in (0..len).step_by(CHUNK) {
                let end = (start + CHUNK).min(len);
                let values: Vec<_> = (start..end)
                    .map(|i| meta.coordinate(level, east, i))
                    .collect();
                variable.put_values(&values, start..end)?;
            }
        }
    }
    Ok(file)
}

fn write_time(
    file: &mut FileMut,
    meta: &InundationMetadata,
) -> Result<(), Box<dyn std::error::Error>> {
    let zone: chrono_tz::Tz = meta.timeline.time_zone.parse()?;
    let mut bounds = Vec::with_capacity(meta.timeline.bounds.len() * 2);
    let mut values = Vec::with_capacity(meta.timeline.bounds.len());
    for interval in &meta.timeline.bounds {
        let start = zone
            .from_local_datetime(&interval.start)
            .single()
            .ok_or("invalid local time")?;
        let end = zone
            .from_local_datetime(&interval.end)
            .single()
            .ok_or("invalid local time")?;
        bounds.extend([start.timestamp(), end.timestamp()]);
        values.push(end.timestamp());
    }
    let mut time = file.add_variable::<i64>("time", &["time"])?;
    time.put_attribute("standard_name", "time")?;
    time.put_attribute("units", TIME_UNITS)?;
    time.put_attribute("calendar", CALENDAR)?;
    time.put_attribute("bounds", "time_bounds")?;
    time.put_attribute("axis", "T")?;
    time.put_values(&values, ..)?;
    let mut time_bounds = file.add_variable::<i64>("time_bounds", &["time", "bounds"])?;
    time_bounds.put_values(&bounds, ..)?;
    let mut reference = file.add_variable::<i64>("forecast_reference_time", &[])?;
    reference.put_attribute("standard_name", "forecast_reference_time")?;
    reference.put_attribute("units", TIME_UNITS)?;
    reference.put_attribute("calendar", CALENDAR)?;
    reference.put_value(values[meta.historical_count - 1], ())?;
    Ok(())
}
