use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::fs::{self, File, OpenOptions};
use std::path::Path;
use std::path::PathBuf;

use chrono::{Datelike, NaiveDateTime, TimeZone, Timelike};
use serde::{Deserialize, Serialize};

use super::parquet;
use parquet::Kind::{Float, Int32, Int64, Text};
#[cfg(test)]
const TIME_FORMAT: &str = "%Y-%m-%dT%H:%M:%S";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservationError(pub String);
impl std::fmt::Display for ObservationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for ObservationError {}
fn error(message: impl Into<String>) -> ObservationError {
    ObservationError(message.into())
}
#[cfg(test)]
fn timestamp(value: &str, context: &str) -> Result<NaiveDateTime, ObservationError> {
    let result = NaiveDateTime::parse_from_str(value, TIME_FORMAT)
        .map_err(|_| error(format!("{context}: invalid local timestamp {value}")))?;
    if result.format(TIME_FORMAT).to_string() != value {
        return Err(error(format!("{context}: invalid local timestamp {value}")));
    }
    Ok(result)
}
fn validate_time(value: NaiveDateTime, context: &str) -> Result<(), ObservationError> {
    if value.nanosecond() != 0 || value.year() < 0 || value.year() > 9999 {
        return Err(error(format!(
            "{context}: timestamp must have whole seconds and a four-digit year"
        )));
    }
    Ok(())
}
fn nonempty(value: &str, context: &str) -> Result<(), ObservationError> {
    if value.trim().is_empty() {
        Err(error(format!("{context}: empty value")))
    } else {
        Ok(())
    }
}
fn finite(value: f64, context: &str) -> Result<(), ObservationError> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(error(format!("{context}: nonfinite value")))
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Station {
    pub station_id: String,
    pub provider_code: String,
    pub station_code: String,
    pub name: String,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ObservationSeries {
    pub series_id: String,
    pub station_id: String,
    pub variable: String,
    pub state: String,
    pub unit: String,
    pub interval_seconds: Option<i64>,
    pub created_at: NaiveDateTime,
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum ObservationValue {
    RainMm(f64),
    FlowM3s(f64),
    LevelCm(i32),
}
pub(crate) type ProviderSamples = Vec<(NaiveDateTime, ObservationValue)>;
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ObservationSample {
    pub series_id: String,
    pub observed_at: NaiveDateTime,
    pub value: Option<ObservationValue>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReplacementKind {
    Point,
    Period,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObservationReplacement {
    pub replacement_id: String,
    pub station_id: String,
    pub variable: String,
    pub kind: ReplacementKind,
    pub start_at: NaiveDateTime,
    pub end_at: Option<NaiveDateTime>,
    pub value: Option<f64>,
    #[serde(default)]
    pub reason: String,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StationOffset {
    pub station_id: String,
    pub mini_id: i64,
    pub offset_m: f64,
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ObservationInputs {
    pub stations: Vec<Station>,
    pub series: Vec<ObservationSeries>,
    pub samples: Vec<ObservationSample>,
    pub replacements: Vec<ObservationReplacement>,
    #[serde(default, skip_serializing_if = "std::collections::BTreeSet::is_empty")]
    pub excluded_rain: std::collections::BTreeSet<(String, NaiveDateTime)>,
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ObservationBatch {
    pub stations: Vec<Station>,
    pub series: Vec<ObservationSeries>,
    pub samples: Vec<ObservationSample>,
    pub replacements: Vec<ObservationReplacement>,
    pub offsets: Vec<StationOffset>,
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ObservationSelection {
    pub provider_codes: Vec<String>,
    pub station_ids: Vec<String>,
}

fn validate_station(s: &Station) -> Result<(), ObservationError> {
    for value in [&s.station_id, &s.provider_code, &s.station_code, &s.name] {
        nonempty(value, "station")?;
    }
    match (s.latitude, s.longitude) {
        (None, None) => Ok(()),
        (Some(lat), Some(lon))
            if lat.is_finite()
                && lon.is_finite()
                && (-90.0..=90.0).contains(&lat)
                && (-180.0..=180.0).contains(&lon) =>
        {
            Ok(())
        }
        _ => Err(error(format!(
            "station {}: invalid coordinates",
            s.station_id
        ))),
    }
}
fn validate_series(s: &ObservationSeries) -> Result<(), ObservationError> {
    validate_time(s.created_at, &s.series_id)?;
    nonempty(&s.series_id, "series ID")?;
    nonempty(&s.station_id, "series station")?;
    if !matches!(s.state.as_str(), "approved" | "curated" | "raw")
        || !matches!(
            (s.variable.as_str(), s.unit.as_str(), s.interval_seconds),
            ("rain", "mm", Some(3600)) | ("level", "cm", None) | ("flow", "m3/s", None)
        )
    {
        return Err(error(format!("series {}: invalid metadata", s.series_id)));
    }
    Ok(())
}
fn validate_sample(s: &ObservationSample, variable: &str) -> Result<(), ObservationError> {
    validate_time(s.observed_at, &s.series_id)?;
    nonempty(&s.series_id, "sample series")?;
    match (&s.value, variable) {
        (None, _) | (Some(ObservationValue::LevelCm(_)), "level") => Ok(()),
        (Some(ObservationValue::RainMm(v)), "rain")
        | (Some(ObservationValue::FlowM3s(v)), "flow") => {
            finite(*v, &format!("sample {} {}", s.series_id, s.observed_at))
        }
        _ => Err(error(format!(
            "sample {} {}: value disagrees with series variable",
            s.series_id, s.observed_at
        ))),
    }
}
pub fn validate_replacement(r: &ObservationReplacement) -> Result<(), ObservationError> {
    validate_time(r.start_at, &r.replacement_id)?;
    if let Some(end) = r.end_at {
        validate_time(end, &r.replacement_id)?;
    }
    nonempty(&r.replacement_id, "replacement ID")?;
    nonempty(&r.station_id, "replacement station")?;
    if !matches!(r.variable.as_str(), "rain" | "flow")
        || (r.kind == ReplacementKind::Point && r.end_at.is_some())
        || r.end_at.is_some_and(|end| end <= r.start_at)
    {
        return Err(error(format!(
            "replacement {}: invalid bounds or variable",
            r.replacement_id
        )));
    }
    if let Some(v) = r.value {
        finite(v, &format!("replacement {}", r.replacement_id))?;
    }
    Ok(())
}
fn validate_offset(o: &StationOffset) -> Result<(), ObservationError> {
    nonempty(&o.station_id, "offset station")?;
    finite(o.offset_m, "offset")?;
    if o.mini_id <= 0 {
        return Err(error(format!("offset {}: invalid mini ID", o.station_id)));
    }
    Ok(())
}

/// A reference line, in centimetres; it is independent of model offsets.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LevelReference {
    pub station_id: String,
    pub kind: String,
    pub date: Option<chrono::NaiveDate>,
    pub level_cm: f64,
}

/// A station and its explicitly supplied model offset, if present.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StationLocation {
    pub station: Station,
    pub offset: Option<StationOffset>,
}

fn validate_level_references(references: &[LevelReference]) -> Result<(), ObservationError> {
    let mut keys = HashSet::new();
    for r in references {
        nonempty(&r.station_id, "reference station")?;
        finite(r.level_cm, "reference level")?;
        let level_key = if r.kind == "historical" && r.level_cm != 0.0 {
            r.level_cm.to_bits()
        } else {
            0
        };
        if !keys.insert((r.station_id.clone(), r.kind.clone(), r.date, level_key)) {
            return Err(error("duplicate reference key"));
        }
        if !matches!(
            (r.kind.as_str(), r.date),
            ("attention" | "alert" | "flood" | "severe", None) | ("historical", Some(_))
        ) {
            return Err(error("invalid reference kind/date"));
        }
    }
    Ok(())
}

const MAX_DELTA_SEGMENTS: usize = 8;
const MAX_DELTA_ROWS: usize = 4096;

#[derive(Clone, Default, Serialize, Deserialize)]
struct Manifest {
    time_zone: String,
    generation: String,
    partitions: BTreeMap<String, String>,
    deltas: BTreeMap<String, Vec<String>>,
    tables: BTreeMap<String, String>,
}
#[derive(Clone, PartialEq, Serialize, Deserialize)]
struct InventoryVariable {
    station_id: String,
    variable: String,
}
#[derive(Clone, PartialEq, Serialize, Deserialize)]
struct Coverage {
    station_id: String,
    variables_key: String,
    day: chrono::NaiveDate,
}
#[derive(Clone, PartialEq, Serialize, Deserialize)]
struct Flag {
    series_id: String,
    observed_at: NaiveDateTime,
    reason: String,
}
#[cfg(test)]
#[derive(Clone, Serialize, Deserialize)]
struct StoredSample {
    series_id: String,
    observed_at: NaiveDateTime,
    rain: Option<f64>,
    flow: Option<f64>,
    level: Option<i32>,
}
const STATIONS: &[(&str, parquet::Kind)] = &[
    ("station_id", Text),
    ("provider_code", Text),
    ("station_code", Text),
    ("name", Text),
    ("latitude", Float),
    ("longitude", Float),
];
const SERIES: &[(&str, parquet::Kind)] = &[
    ("series_id", Text),
    ("station_id", Text),
    ("variable", Text),
    ("state", Text),
    ("unit", Text),
    ("interval_seconds", Int64),
    ("created_at", Text),
];
const SAMPLES: &[(&str, parquet::Kind)] = &[
    ("series_id", Text),
    ("observed_at", Text),
    ("rain", Float),
    ("flow", Float),
    ("level", Int32),
];
const REPLACEMENTS: &[(&str, parquet::Kind)] = &[
    ("replacement_id", Text),
    ("station_id", Text),
    ("variable", Text),
    ("kind", Text),
    ("start_at", Text),
    ("end_at", Text),
    ("value", Float),
    ("reason", Text),
];
const OFFSETS: &[(&str, parquet::Kind)] = &[
    ("station_id", Text),
    ("mini_id", Int64),
    ("offset_m", Float),
];
const REFERENCES: &[(&str, parquet::Kind)] = &[
    ("station_id", Text),
    ("kind", Text),
    ("date", Text),
    ("level_cm", Float),
];
const VARIABLES: &[(&str, parquet::Kind)] = &[("station_id", Text), ("variable", Text)];
const COVERAGE: &[(&str, parquet::Kind)] =
    &[("station_id", Text), ("variables_key", Text), ("day", Text)];
const FLAGS: &[(&str, parquet::Kind)] =
    &[("series_id", Text), ("observed_at", Text), ("reason", Text)];
#[derive(Clone, Default, PartialEq)]
struct Metadata {
    stations: Vec<Station>,
    series: Vec<ObservationSeries>,
    replacements: Vec<ObservationReplacement>,
    offsets: Vec<StationOffset>,
    references: Vec<LevelReference>,
    variables: Vec<InventoryVariable>,
    flags: Vec<Flag>,
    coverage: Vec<Coverage>,
}

fn partition_key(id: &str, _at: NaiveDateTime) -> String {
    format!("{:x}", Sha256::digest(id.as_bytes()))
}
fn io_error(e: impl std::fmt::Display) -> ObservationError {
    error(e.to_string())
}
fn write_table<T: Serialize>(
    path: &Path,
    name: &str,
    schema: &[(&str, parquet::Kind)],
    values: &[T],
) -> Result<(), ObservationError> {
    parquet::write_staged(&path.join(format!("{name}.parquet")), schema, values).map_err(io_error)
}
fn metadata_batches(
    root: &Path,
    m: &Manifest,
    name: &str,
    schema: &[(&str, parquet::Kind)],
) -> Result<Vec<arrow_array::RecordBatch>, ObservationError> {
    let path = m
        .tables
        .get(name)
        .ok_or_else(|| error(format!("missing table {name}")))?;
    parquet::read_batches(&root.join(path), schema).map_err(io_error)
}
fn text_column<'a>(
    batch: &'a arrow_array::RecordBatch,
    name: &str,
) -> Result<&'a arrow_array::StringArray, ObservationError> {
    batch
        .column_by_name(name)
        .and_then(|v| v.as_any().downcast_ref::<arrow_array::StringArray>())
        .ok_or_else(|| error(format!("invalid column {name}")))
}
fn float_column<'a>(
    batch: &'a arrow_array::RecordBatch,
    name: &str,
) -> Result<&'a arrow_array::Float64Array, ObservationError> {
    batch
        .column_by_name(name)
        .and_then(|v| v.as_any().downcast_ref::<arrow_array::Float64Array>())
        .ok_or_else(|| error(format!("invalid column {name}")))
}
fn int_column<'a>(
    batch: &'a arrow_array::RecordBatch,
    name: &str,
) -> Result<&'a arrow_array::Int64Array, ObservationError> {
    batch
        .column_by_name(name)
        .and_then(|v| v.as_any().downcast_ref::<arrow_array::Int64Array>())
        .ok_or_else(|| error(format!("invalid column {name}")))
}
fn required_str(array: &arrow_array::StringArray, row: usize) -> Result<&str, ObservationError> {
    use arrow_array::Array;
    if array.is_null(row) {
        return Err(error("null required text"));
    }
    Ok(array.value(row))
}
fn optional_float(
    array: &arrow_array::Float64Array,
    row: usize,
) -> Result<Option<f64>, ObservationError> {
    use arrow_array::Array;
    if array.is_null(row) {
        return Ok(None);
    }
    let value = array.value(row);
    if !value.is_finite() {
        return Err(error("nonfinite float"));
    }
    Ok(Some(value))
}
fn read_stations(
    root: &Path,
    m: &Manifest,
    selected: Option<&str>,
) -> Result<Vec<Station>, ObservationError> {
    let mut rows = Vec::new();
    for batch in metadata_batches(root, m, "stations", STATIONS)? {
        let id = text_column(&batch, "station_id")?;
        let provider = text_column(&batch, "provider_code")?;
        let code = text_column(&batch, "station_code")?;
        let name = text_column(&batch, "name")?;
        let lat = float_column(&batch, "latitude")?;
        let lon = float_column(&batch, "longitude")?;
        for row in 0..batch.num_rows() {
            let id = required_str(id, row)?;
            let provider = required_str(provider, row)?;
            let code = required_str(code, row)?;
            let name = required_str(name, row)?;
            let latitude = optional_float(lat, row)?;
            let longitude = optional_float(lon, row)?;
            if selected.is_none_or(|s| s == id) {
                rows.push(Station {
                    station_id: id.to_owned(),
                    provider_code: provider.to_owned(),
                    station_code: code.to_owned(),
                    name: name.to_owned(),
                    latitude,
                    longitude,
                });
            }
        }
    }
    Ok(rows)
}
fn read_series_metadata(
    root: &Path,
    m: &Manifest,
    selected: Option<&str>,
) -> Result<Vec<ObservationSeries>, ObservationError> {
    use arrow_array::Array;
    let mut rows = Vec::new();
    for batch in metadata_batches(root, m, "series", SERIES)? {
        let id = text_column(&batch, "series_id")?;
        let station = text_column(&batch, "station_id")?;
        let variable = text_column(&batch, "variable")?;
        let state = text_column(&batch, "state")?;
        let unit = text_column(&batch, "unit")?;
        let interval = int_column(&batch, "interval_seconds")?;
        let created = text_column(&batch, "created_at")?;
        for row in 0..batch.num_rows() {
            let id = required_str(id, row)?;
            let station = required_str(station, row)?;
            let variable = required_str(variable, row)?;
            let state = required_str(state, row)?;
            let unit = required_str(unit, row)?;
            let created_at = required_str(created, row)?
                .parse::<NaiveDateTime>()
                .map_err(io_error)?;
            let interval_seconds = if interval.is_null(row) {
                None
            } else {
                Some(interval.value(row))
            };
            if selected.is_none_or(|s| s == station) {
                rows.push(ObservationSeries {
                    series_id: id.to_owned(),
                    station_id: station.to_owned(),
                    variable: variable.to_owned(),
                    state: state.to_owned(),
                    unit: unit.to_owned(),
                    interval_seconds,
                    created_at,
                });
            }
        }
    }
    Ok(rows)
}
fn read_inventory_variables(
    root: &Path,
    m: &Manifest,
    collect: bool,
) -> Result<Vec<InventoryVariable>, ObservationError> {
    let mut rows = Vec::new();
    for batch in metadata_batches(root, m, "variables", VARIABLES)? {
        let station = text_column(&batch, "station_id")?;
        let variable = text_column(&batch, "variable")?;
        for row in 0..batch.num_rows() {
            let station = required_str(station, row)?;
            let variable = required_str(variable, row)?;
            if collect {
                rows.push(InventoryVariable {
                    station_id: station.to_owned(),
                    variable: variable.to_owned(),
                });
            }
        }
    }
    Ok(rows)
}

impl Metadata {
    fn read(root: &Path, m: &Manifest) -> Result<Self, ObservationError> {
        Self::read_projected(root, m, None)
    }
    fn read_projected(
        root: &Path,
        m: &Manifest,
        selected: Option<&str>,
    ) -> Result<Self, ObservationError> {
        fn table<T: serde::de::DeserializeOwned>(
            root: &Path,
            m: &Manifest,
            name: &str,
            schema: &[(&str, parquet::Kind)],
        ) -> Result<Vec<T>, ObservationError> {
            let path = m
                .tables
                .get(name)
                .ok_or_else(|| error(format!("missing table {name}")))?;
            parquet::read(&root.join(path), schema).map_err(io_error)
        }
        Ok(Self {
            stations: read_stations(root, m, selected)?,
            series: read_series_metadata(root, m, selected)?,
            replacements: table(root, m, "replacements", REPLACEMENTS)?,
            offsets: table(root, m, "offsets", OFFSETS)?,
            references: table(root, m, "references", REFERENCES)?,
            variables: read_inventory_variables(root, m, selected.is_none())?,
            flags: table(root, m, "flags", FLAGS)?,
            coverage: table(root, m, "coverage", COVERAGE)?,
        })
    }
    fn write_changed(
        &self,
        old: Option<&Self>,
        path: &Path,
        id: &str,
        m: &mut Manifest,
    ) -> Result<(), ObservationError> {
        write_changed_table(
            path,
            id,
            m,
            "stations",
            STATIONS,
            &self.stations,
            old.map(|o| o.stations.as_slice()),
        )?;
        write_changed_table(
            path,
            id,
            m,
            "series",
            SERIES,
            &self.series,
            old.map(|o| o.series.as_slice()),
        )?;
        write_changed_table(
            path,
            id,
            m,
            "replacements",
            REPLACEMENTS,
            &self.replacements,
            old.map(|o| o.replacements.as_slice()),
        )?;
        write_changed_table(
            path,
            id,
            m,
            "offsets",
            OFFSETS,
            &self.offsets,
            old.map(|o| o.offsets.as_slice()),
        )?;
        write_changed_table(
            path,
            id,
            m,
            "references",
            REFERENCES,
            &self.references,
            old.map(|o| o.references.as_slice()),
        )?;
        write_changed_table(
            path,
            id,
            m,
            "variables",
            VARIABLES,
            &self.variables,
            old.map(|o| o.variables.as_slice()),
        )?;
        write_changed_table(
            path,
            id,
            m,
            "flags",
            FLAGS,
            &self.flags,
            old.map(|o| o.flags.as_slice()),
        )?;
        write_changed_table(
            path,
            id,
            m,
            "coverage",
            COVERAGE,
            &self.coverage,
            old.map(|o| o.coverage.as_slice()),
        )?;
        Ok(())
    }
}

fn write_changed_table<T: Serialize + PartialEq>(
    path: &Path,
    id: &str,
    m: &mut Manifest,
    name: &str,
    schema: &[(&str, parquet::Kind)],
    rows: &[T],
    old: Option<&[T]>,
) -> Result<(), ObservationError> {
    if old != Some(rows) {
        write_table(path, name, schema, rows)?;
        m.tables
            .insert(name.into(), format!("generations/{id}/{name}.parquet"));
    }
    Ok(())
}

fn write_sample_batch(path: &Path, rows: &[ObservationSample]) -> Result<(), ObservationError> {
    use arrow_array::{ArrayRef, Float64Array, Int32Array, RecordBatch, StringArray};
    use arrow_schema::{DataType, Field, Schema};
    use std::sync::Arc;
    let schema = Arc::new(Schema::new(vec![
        Field::new("series_id", DataType::Utf8, true),
        Field::new("observed_at", DataType::Utf8, true),
        Field::new("rain", DataType::Float64, true),
        Field::new("flow", DataType::Float64, true),
        Field::new("level", DataType::Int32, true),
    ]));
    let timestamps = rows
        .iter()
        .map(|r| r.observed_at.format("%Y-%m-%dT%H:%M:%S").to_string())
        .collect::<Vec<_>>();
    let arrays: Vec<ArrayRef> = vec![
        Arc::new(StringArray::from(
            rows.iter()
                .map(|r| Some(r.series_id.as_str()))
                .collect::<Vec<_>>(),
        )),
        Arc::new(StringArray::from(
            timestamps
                .iter()
                .map(|t| Some(t.as_str()))
                .collect::<Vec<_>>(),
        )),
        Arc::new(Float64Array::from(
            rows.iter()
                .map(|r| match r.value {
                    Some(ObservationValue::RainMm(v)) => Some(v),
                    _ => None,
                })
                .collect::<Vec<_>>(),
        )),
        Arc::new(Float64Array::from(
            rows.iter()
                .map(|r| match r.value {
                    Some(ObservationValue::FlowM3s(v)) => Some(v),
                    _ => None,
                })
                .collect::<Vec<_>>(),
        )),
        Arc::new(Int32Array::from(
            rows.iter()
                .map(|r| match r.value {
                    Some(ObservationValue::LevelCm(v)) => Some(v),
                    _ => None,
                })
                .collect::<Vec<_>>(),
        )),
    ];
    let batch = RecordBatch::try_new(schema, arrays).map_err(io_error)?;
    parquet::write_batch_staged(path, &batch, 1024).map_err(io_error)
}

fn sync_generation(path: &Path) -> Result<(), ObservationError> {
    let mut files = Vec::new();
    for entry in fs::read_dir(path).map_err(io_error)? {
        let entry = entry.map_err(io_error)?;
        if !entry.file_type().map_err(io_error)?.is_file() {
            return Err(error("unexpected staged generation entry"));
        }
        files.push(entry.path());
    }
    // All generation files are immutable before publication. Waiting for each
    // bounded group permits the filesystem to combine their durability work.
    for chunk in files.chunks(8) {
        std::thread::scope(|scope| -> Result<(), ObservationError> {
            let workers = chunk
                .iter()
                .map(|file| scope.spawn(move || File::open(file).and_then(|f| f.sync_data())))
                .collect::<Vec<_>>();
            for worker in workers {
                worker
                    .join()
                    .map_err(|_| error("generation sync worker panicked"))?
                    .map_err(io_error)?;
            }
            Ok(())
        })?;
    }
    super::sync_directory(path).map_err(io_error)
}

struct PendingPartition {
    original: Option<Vec<ObservationSample>>,
    rows: Vec<ObservationSample>,
}

pub struct ObservationStore {
    path: PathBuf,
    writable: bool,
    metadata_cache: std::sync::Mutex<Option<(String, std::sync::Arc<Metadata>)>>,
    manifest_cache: std::sync::Mutex<Option<(same_file::Handle, std::sync::Arc<Manifest>)>>,
}
impl ObservationStore {
    pub fn create(path: &Path, time_zone: &str) -> Result<Self, ObservationError> {
        time_zone
            .parse::<chrono_tz::Tz>()
            .map_err(|_| error(format!("invalid time zone {time_zone}")))?;
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            fs::create_dir_all(parent).map_err(io_error)?;
        }
        fs::create_dir(path).map_err(io_error)?;
        let result = (|| {
            fs::create_dir(path.join("generations")).map_err(io_error)?;
            File::create(path.join("access.lock")).map_err(io_error)?;
            let store = Self {
                path: path.into(),
                writable: true,
                metadata_cache: std::sync::Mutex::new(None),
                manifest_cache: std::sync::Mutex::new(None),
            };
            let mut manifest = Manifest {
                time_zone: time_zone.into(),
                ..Default::default()
            };
            store.publish(
                &mut manifest,
                &Metadata::default(),
                None,
                BTreeMap::new(),
                BTreeMap::new(),
            )?;
            Ok(store)
        })();
        if result.is_err() {
            let _ = fs::remove_dir_all(path);
        }
        result
    }
    pub fn open_read_only(path: &Path) -> Result<Self, ObservationError> {
        Self::open(path, false)
    }
    pub fn open_writable(path: &Path) -> Result<Self, ObservationError> {
        Self::open(path, true)
    }
    fn open(path: &Path, writable: bool) -> Result<Self, ObservationError> {
        let store = Self {
            path: path.into(),
            writable,
            metadata_cache: std::sync::Mutex::new(None),
            manifest_cache: std::sync::Mutex::new(None),
        };
        let guard = store.read_lock()?;
        let m = store.manifest()?;
        store.metadata(&m)?;
        drop(guard);
        Ok(store)
    }
    fn manifest(&self) -> Result<std::sync::Arc<Manifest>, ObservationError> {
        use std::io::Read;
        let handle =
            same_file::Handle::from_path(self.path.join("current.json")).map_err(io_error)?;
        let mut cache = self
            .manifest_cache
            .lock()
            .map_err(|_| error("observation manifest cache poisoned"))?;
        if let Some((old, manifest)) = &*cache
            && *old == handle
        {
            return Ok(manifest.clone());
        }
        let mut bytes = Vec::new();
        handle.as_file().read_to_end(&mut bytes).map_err(io_error)?;
        let m: Manifest = serde_json::from_slice(&bytes).map_err(io_error)?;
        if m.generation.is_empty()
            || !m
                .generation
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-')
        {
            return Err(error("invalid generation"));
        }
        m.time_zone
            .parse::<chrono_tz::Tz>()
            .map_err(|_| error("invalid time zone"))?;
        for path in m
            .partitions
            .values()
            .chain(m.tables.values())
            .chain(m.deltas.values().flatten())
        {
            if Path::new(path)
                .components()
                .any(|c| !matches!(c, std::path::Component::Normal(_)))
            {
                return Err(error("invalid partition path"));
            }
        }
        let m = std::sync::Arc::new(m);
        *cache = Some((handle, m.clone()));
        Ok(m)
    }
    pub fn time_zone(&self) -> Result<String, ObservationError> {
        let _guard = self.read_lock()?;
        Ok(self.manifest()?.time_zone.clone())
    }
    fn metadata(&self, m: &Manifest) -> Result<std::sync::Arc<Metadata>, ObservationError> {
        let mut cache = self
            .metadata_cache
            .lock()
            .map_err(|_| error("observation metadata cache poisoned"))?;
        if let Some((generation, metadata)) = &*cache
            && *generation == m.generation
        {
            return Ok(metadata.clone());
        }
        let metadata = std::sync::Arc::new(Metadata::read(&self.path, m)?);
        *cache = Some((m.generation.clone(), metadata.clone()));
        Ok(metadata)
    }
    fn read_lock(&self) -> Result<File, ObservationError> {
        let file = File::open(self.path.join("access.lock")).map_err(io_error)?;
        file.lock_shared().map_err(io_error)?;
        Ok(file)
    }
    fn lock(&self) -> Result<File, ObservationError> {
        if !self.writable {
            return Err(error("observation store is read-only"));
        }
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .open(self.path.join("access.lock"))
            .map_err(io_error)?;
        lock.lock().map_err(io_error)?;
        Ok(lock)
    }
    fn samples(&self, m: &Manifest, key: &str) -> Result<Vec<ObservationSample>, ObservationError> {
        let start = chrono::NaiveDate::from_ymd_opt(0, 1, 1)
            .expect("valid year")
            .and_hms_opt(0, 0, 0)
            .expect("valid time");
        let end = chrono::NaiveDate::from_ymd_opt(9999, 12, 31)
            .expect("valid year")
            .and_hms_opt(23, 59, 59)
            .expect("valid time");
        self.samples_range(m, key, start, end)
    }
    fn samples_range(
        &self,
        m: &Manifest,
        key: &str,
        start: NaiveDateTime,
        end: NaiveDateTime,
    ) -> Result<Vec<ObservationSample>, ObservationError> {
        use arrow_array::{Array, Float64Array, Int32Array, StringArray};
        let paths = m
            .partitions
            .get(key)
            .into_iter()
            .chain(m.deltas.get(key).into_iter().flatten());
        let from = start.format("%Y-%m-%dT%H:%M:%S").to_string();
        let to = end.format("%Y-%m-%dT%H:%M:%S").to_string();
        let mut out = BTreeMap::new();
        for path in paths {
            for batch in parquet::sample_batches(&self.path.join(path), SAMPLES, &from, &to)
                .map_err(io_error)?
            {
                let text = |name: &str| {
                    batch
                        .column_by_name(name)
                        .and_then(|c| c.as_any().downcast_ref::<StringArray>())
                        .ok_or_else(|| error(format!("invalid column {name}")))
                };
                let number = |name: &str| {
                    batch
                        .column_by_name(name)
                        .and_then(|c| c.as_any().downcast_ref::<Float64Array>())
                        .ok_or_else(|| error(format!("invalid column {name}")))
                };
                let id = text("series_id")?;
                let times = text("observed_at")?;
                let rain = number("rain")?;
                let flow = number("flow")?;
                let level = batch
                    .column_by_name("level")
                    .and_then(|c| c.as_any().downcast_ref::<Int32Array>())
                    .ok_or_else(|| error("invalid level column"))?;
                for i in 0..batch.num_rows() {
                    if times.is_null(i) || id.is_null(i) {
                        return Err(error("null sample identity"));
                    }
                    let at = times.value(i);
                    if at < from.as_str() || at > to.as_str() {
                        continue;
                    }
                    let observed_at =
                        NaiveDateTime::parse_from_str(at, "%Y-%m-%dT%H:%M:%S").map_err(io_error)?;
                    let value = match (!rain.is_null(i), !flow.is_null(i), !level.is_null(i)) {
                        (false, false, false) => None,
                        (true, false, false) => Some(ObservationValue::RainMm(rain.value(i))),
                        (false, true, false) => Some(ObservationValue::FlowM3s(flow.value(i))),
                        (false, false, true) => Some(ObservationValue::LevelCm(level.value(i))),
                        _ => return Err(error("invalid sample columns")),
                    };
                    out.insert(
                        observed_at,
                        ObservationSample {
                            series_id: id.value(i).into(),
                            observed_at,
                            value,
                        },
                    );
                }
            }
        }
        Ok(out.into_values().collect())
    }
    fn publish(
        &self,
        m: &mut Manifest,
        meta: &Metadata,
        old: Option<&Metadata>,
        changed: BTreeMap<String, PendingPartition>,
        deltas: BTreeMap<String, Vec<ObservationSample>>,
    ) -> Result<(), ObservationError> {
        let dir = tempfile::Builder::new()
            .prefix("g-")
            .tempdir_in(self.path.join("generations"))
            .map_err(io_error)?;
        let id = dir
            .path()
            .file_name()
            .ok_or_else(|| error("invalid generation"))?
            .to_string_lossy()
            .into_owned();
        meta.write_changed(old, dir.path(), &id, m)?;
        for (key, pending) in changed {
            let mut values = pending.rows;
            values.sort_by_key(|s| s.observed_at);
            let filename = format!("{}.parquet", key.replace('/', "-"));
            write_sample_batch(&dir.path().join(&filename), &values)?;
            m.deltas.remove(&key);
            m.partitions
                .insert(key, format!("generations/{id}/{filename}"));
        }
        for (key, mut values) in deltas {
            values.sort_by_key(|s| s.observed_at);
            let filename = format!("{key}-delta.parquet");
            write_sample_batch(&dir.path().join(&filename), &values)?;
            m.deltas
                .entry(key)
                .or_default()
                .push(format!("generations/{id}/{filename}"));
        }
        m.generation = id.clone();
        let bytes = serde_json::to_vec(m).map_err(io_error)?;
        fs::write(dir.path().join("manifest.json"), bytes).map_err(io_error)?;
        sync_generation(dir.path())?;
        let generation = dir.keep();
        super::sync_directory(&self.path.join("generations")).map_err(io_error)?;
        let temporary = tempfile::Builder::new()
            .prefix(".current-")
            .tempfile_in(&self.path)
            .map_err(io_error)?;
        let temporary_path = temporary.path().to_path_buf();
        temporary.close().map_err(io_error)?;
        let result = (|| {
            fs::hard_link(generation.join("manifest.json"), &temporary_path).map_err(io_error)?;
            fs::rename(&temporary_path, self.path.join("current.json")).map_err(io_error)
        })();
        if result.is_err() {
            let _ = fs::remove_file(&temporary_path);
        }
        result?;
        super::sync_directory(&self.path).map_err(io_error)?;
        self.cleanup_generations(m);
        Ok(())
    }
    fn cleanup_generations(&self, m: &Manifest) {
        let mut referenced = m
            .tables
            .values()
            .chain(m.partitions.values())
            .chain(m.deltas.values().flatten())
            .map(|v| self.path.join(v))
            .collect::<HashSet<_>>();
        referenced.insert(
            self.path
                .join("generations")
                .join(&m.generation)
                .join("manifest.json"),
        );
        let Ok(dirs) = fs::read_dir(self.path.join("generations")) else {
            return;
        };
        for dir in dirs.flatten() {
            let path = dir.path();
            if !dir.file_type().is_ok_and(|t| t.is_dir()) {
                continue;
            }
            let Ok(files) = fs::read_dir(&path) else {
                continue;
            };
            for file in files.flatten() {
                let p = file.path();
                if !referenced.contains(&p) && file.file_type().is_ok_and(|t| t.is_file()) {
                    let _ = fs::remove_file(p);
                }
            }
            let _ = fs::remove_dir(path);
        }
    }
    fn mutate<T>(
        &mut self,
        f: impl FnOnce(
            &Self,
            &Manifest,
            &mut Metadata,
            &mut BTreeMap<String, PendingPartition>,
        ) -> Result<T, ObservationError>,
    ) -> Result<T, ObservationError> {
        let _lock = self.lock()?;
        let mut m = (*self.manifest()?).clone();
        let mut metadata = (*self.metadata(&m)?).clone();
        let old = metadata.clone();
        let mut changed = BTreeMap::new();
        let value = f(self, &m, &mut metadata, &mut changed)?;
        changed.retain(|_, pending| pending.original.as_ref() != Some(&pending.rows));
        if metadata != old || !changed.is_empty() {
            self.publish(&mut m, &metadata, Some(&old), changed, BTreeMap::new())?;
        }
        Ok(value)
    }
    fn partition<'a>(
        &self,
        m: &Manifest,
        changed: &'a mut BTreeMap<String, PendingPartition>,
        id: &str,
        at: NaiveDateTime,
    ) -> Result<&'a mut Vec<ObservationSample>, ObservationError> {
        let key = partition_key(id, at);
        if !changed.contains_key(&key) {
            let rows = self.samples(m, &key)?;
            let original = m.partitions.contains_key(&key).then(|| rows.clone());
            changed.insert(key.clone(), PendingPartition { original, rows });
        }
        Ok(&mut changed.get_mut(&key).expect("inserted partition").rows)
    }
}

fn insert_unique<T: Clone + PartialEq>(
    items: &mut Vec<T>,
    item: &T,
    idempotent: bool,
    key: impl Fn(&T) -> bool,
    label: &str,
) -> Result<(), ObservationError> {
    if let Some(existing) = items.iter().find(|v| key(v)) {
        if idempotent && existing == item {
            return Ok(());
        }
        return Err(error(format!("conflicting {label}")));
    }
    items.push(item.clone());
    Ok(())
}
fn local_time(
    zone: chrono_tz::Tz,
    at: NaiveDateTime,
    context: &str,
) -> Result<(), ObservationError> {
    validate_time(at, context)?;
    if zone.from_local_datetime(&at).single().is_none() {
        return Err(error(format!(
            "{context}: ambiguous or nonexistent local time {at}"
        )));
    }
    Ok(())
}
fn selected_replacements(
    meta: &Metadata,
    ids: &[String],
) -> Result<Vec<ObservationReplacement>, ObservationError> {
    let mut result = Vec::new();
    let mut seen = HashSet::new();
    for id in ids {
        if !seen.insert(id) {
            return Err(error(format!("duplicate replacement {id}")));
        }
        result.push(
            meta.replacements
                .iter()
                .find(|r| r.replacement_id == *id)
                .ok_or_else(|| error(format!("unknown replacement {id}")))?
                .clone(),
        );
    }
    Ok(result)
}
impl ObservationStore {
    pub fn write_batch(&mut self, batch: &ObservationBatch) -> Result<(), ObservationError> {
        self.write_batch_inner(batch, false, false)
    }
    pub fn ingest_batch(&mut self, batch: &ObservationBatch) -> Result<(), ObservationError> {
        self.write_batch_inner(batch, true, false)
    }
    pub fn register_replacements(
        &mut self,
        replacements: Vec<ObservationReplacement>,
    ) -> Result<(), ObservationError> {
        self.write_batch_inner(
            &ObservationBatch {
                replacements,
                ..Default::default()
            },
            true,
            false,
        )
    }
    pub(crate) fn replace_replacements(
        &mut self,
        replacements: Vec<ObservationReplacement>,
    ) -> Result<(), ObservationError> {
        self.write_batch_inner(
            &ObservationBatch {
                replacements,
                ..Default::default()
            },
            false,
            true,
        )
    }
    fn write_batch_inner(
        &mut self,
        batch: &ObservationBatch,
        idempotent: bool,
        replace: bool,
    ) -> Result<(), ObservationError> {
        self.mutate(|store, m, meta, changed| {
            let zone = m.time_zone.parse::<chrono_tz::Tz>().map_err(io_error)?;
            if replace {
                meta.replacements.clear();
            }
            for s in &batch.stations {
                validate_station(s)?;
                if meta.stations.iter().any(|v| {
                    v.provider_code == s.provider_code
                        && v.station_code == s.station_code
                        && v.station_id != s.station_id
                }) {
                    return Err(error("duplicate provider station"));
                }
                insert_unique(
                    &mut meta.stations,
                    s,
                    idempotent,
                    |v| v.station_id == s.station_id,
                    "station",
                )?;
            }
            for s in &batch.series {
                validate_series(s)?;
                if !meta.stations.iter().any(|v| v.station_id == s.station_id) {
                    return Err(error("unknown series station"));
                }
                insert_unique(
                    &mut meta.series,
                    s,
                    idempotent,
                    |v| v.series_id == s.series_id,
                    "series",
                )?;
            }
            for s in &batch.samples {
                let series = meta
                    .series
                    .iter()
                    .find(|v| v.series_id == s.series_id)
                    .ok_or_else(|| error("unknown sample series"))?;
                validate_sample(s, &series.variable)?;
                local_time(zone, s.observed_at, &s.series_id)?;
                let part = store.partition(m, changed, &s.series_id, s.observed_at)?;
                match part.binary_search_by_key(&s.observed_at, |v| v.observed_at) {
                    Ok(i) if idempotent && part[i] == *s => {}
                    Ok(_) => return Err(error("conflicting sample")),
                    Err(i) => part.insert(i, s.clone()),
                }
            }
            for r in &batch.replacements {
                validate_replacement(r)?;
                local_time(zone, r.start_at, &r.replacement_id)?;
                if let Some(end) = r.end_at {
                    local_time(zone, end, &r.replacement_id)?;
                }
                if !meta.stations.iter().any(|s| s.station_id == r.station_id) {
                    return Err(error("unknown replacement station"));
                }
                insert_unique(
                    &mut meta.replacements,
                    r,
                    idempotent,
                    |v| v.replacement_id == r.replacement_id,
                    "replacement",
                )?;
            }
            for o in &batch.offsets {
                validate_offset(o)?;
                if !meta.stations.iter().any(|s| s.station_id == o.station_id) {
                    return Err(error("unknown offset station"));
                }
                insert_unique(
                    &mut meta.offsets,
                    o,
                    idempotent,
                    |v| v.station_id == o.station_id,
                    "offset",
                )?;
            }
            Ok(())
        })
    }
    pub fn read_station_offsets(
        &self,
        ids: &[String],
    ) -> Result<Vec<StationOffset>, ObservationError> {
        let _guard = self.read_lock()?;
        let meta = self.metadata(self.manifest()?.as_ref())?;
        Ok(ids
            .iter()
            .filter_map(|id| meta.offsets.iter().find(|v| v.station_id == *id).cloned())
            .collect())
    }
    pub fn load_replacements(
        &self,
        ids: &[String],
    ) -> Result<Vec<ObservationReplacement>, ObservationError> {
        let _guard = self.read_lock()?;
        selected_replacements(self.metadata(self.manifest()?.as_ref())?.as_ref(), ids)
    }
    pub(crate) fn read_station_selection(
        path: &Path,
        station: &str,
        interval: &super::IntervalAxis,
        ids: &[String],
    ) -> Result<(ObservationInputs, Vec<StationOffset>), ObservationError> {
        let store = Self {
            path: path.into(),
            writable: false,
            metadata_cache: std::sync::Mutex::new(None),
            manifest_cache: std::sync::Mutex::new(None),
        };
        let selection = ObservationSelection {
            provider_codes: Vec::new(),
            station_ids: vec![station.to_owned()],
        };
        store.read_selection_inner(&selection, interval, ids, &[], Some(station))
    }
    pub fn read_selection(
        &self,
        selection: &ObservationSelection,
        interval: &super::IntervalAxis,
        ids: &[String],
    ) -> Result<(ObservationInputs, Vec<StationOffset>), ObservationError> {
        self.read_selection_with_stations(selection, interval, ids, &[])
    }
    pub fn read_selection_with_stations(
        &self,
        selection: &ObservationSelection,
        interval: &super::IntervalAxis,
        ids: &[String],
        extra: &[String],
    ) -> Result<(ObservationInputs, Vec<StationOffset>), ObservationError> {
        self.read_selection_inner(selection, interval, ids, extra, None)
    }
    fn read_selection_inner(
        &self,
        selection: &ObservationSelection,
        interval: &super::IntervalAxis,
        ids: &[String],
        extra: &[String],
        projected: Option<&str>,
    ) -> Result<(ObservationInputs, Vec<StationOffset>), ObservationError> {
        let _guard = self.read_lock()?;
        if selection.provider_codes.is_empty() && selection.station_ids.is_empty() {
            return Err(error("selection requires providers or stations"));
        }
        let initial = if let Some(station) = projected {
            let m = self.manifest()?;
            let metadata =
                std::sync::Arc::new(Metadata::read_projected(&self.path, &m, Some(station))?);
            Some((m, metadata))
        } else {
            None
        };
        interval.validate().map_err(io_error)?;
        let (m, meta) = if let Some(initial) = initial {
            initial
        } else {
            let m = self.manifest()?;
            let metadata = self.metadata(&m)?;
            (m, metadata)
        };
        if m.time_zone != interval.time_zone {
            return Err(error("observation and request time zones differ"));
        }
        let mut stations = meta
            .stations
            .iter()
            .filter(|s| {
                selection.provider_codes.contains(&s.provider_code)
                    || selection.station_ids.contains(&s.station_id)
                    || extra.contains(&s.station_id)
            })
            .cloned()
            .collect::<Vec<_>>();
        stations.sort_by(|a, b| a.station_id.cmp(&b.station_id));
        for id in selection.station_ids.iter().chain(extra) {
            if !stations.iter().any(|s| s.station_id == *id) {
                return Err(error(format!("unknown station {id}")));
            }
        }
        for id in &selection.provider_codes {
            if !stations.iter().any(|s| s.provider_code == *id) {
                return Err(error(format!("unknown provider {id}")));
            }
        }
        let mut series = meta
            .series
            .iter()
            .filter(|v| stations.iter().any(|s| s.station_id == v.station_id))
            .cloned()
            .collect::<Vec<_>>();
        series.sort_by(|a, b| a.series_id.cmp(&b.series_id));
        let start = interval.bounds.first().expect("valid interval").start;
        let end = interval.bounds.last().expect("valid interval").end;
        let mut samples = Vec::new();
        let mut excluded_rain = BTreeSet::new();
        let flags = meta
            .flags
            .iter()
            .map(|f| (f.series_id.as_str(), f.observed_at))
            .collect::<HashSet<_>>();
        for s in &series {
            validate_series(s)?;
            for sample in self.samples_range(&m, &partition_key(&s.series_id, start), start, end)? {
                validate_sample(&sample, &s.variable)?;
                if sample.observed_at >= start
                    && sample.observed_at <= end
                    && !(s.variable == "rain" && sample.observed_at == start)
                {
                    if sample.value.is_some()
                        && flags.contains(&(s.series_id.as_str(), sample.observed_at))
                    {
                        excluded_rain.insert((s.series_id.clone(), sample.observed_at));
                    }
                    samples.push(sample);
                }
            }
        }
        let offsets = stations
            .iter()
            .filter_map(|s| {
                meta.offsets
                    .iter()
                    .find(|o| o.station_id == s.station_id)
                    .cloned()
            })
            .collect();
        Ok((
            ObservationInputs {
                stations,
                series,
                samples,
                replacements: selected_replacements(&meta, ids)?,
                excluded_rain,
            },
            offsets,
        ))
    }
    pub fn update_station_inventory(
        &mut self,
        stations: &[Station],
    ) -> Result<(), ObservationError> {
        self.mutate(|_, _, meta, _| {
            let mut seen = HashSet::new();
            for s in stations {
                validate_station(s)?;
                if !seen.insert(&s.station_id) {
                    return Err(error("duplicate station"));
                }
                let old = meta
                    .stations
                    .iter_mut()
                    .find(|v| {
                        v.station_id == s.station_id
                            && v.provider_code == s.provider_code
                            && v.station_code == s.station_code
                    })
                    .ok_or_else(|| error("unknown or changed station identity"))?;
                *old = s.clone();
            }
            Ok(())
        })
    }
    pub fn station_locations(&self) -> Result<Vec<StationLocation>, ObservationError> {
        let _guard = self.read_lock()?;
        let meta = self.metadata(self.manifest()?.as_ref())?;
        let mut result = meta
            .stations
            .iter()
            .map(|s| StationLocation {
                station: s.clone(),
                offset: meta
                    .offsets
                    .iter()
                    .find(|o| o.station_id == s.station_id)
                    .cloned(),
            })
            .collect::<Vec<_>>();
        result.sort_by(|a, b| a.station.station_id.cmp(&b.station.station_id));
        Ok(result)
    }
    pub fn series_choices(&self) -> Result<Vec<(String, String, String)>, ObservationError> {
        let _guard = self.read_lock()?;
        let meta = self.metadata(self.manifest()?.as_ref())?;
        let mut v = meta
            .series
            .iter()
            .map(|s| {
                (
                    s.series_id.clone(),
                    s.station_id.clone(),
                    s.variable.clone(),
                )
            })
            .collect::<Vec<_>>();
        v.sort();
        Ok(v)
    }
    pub fn replacement_choices(&self) -> Result<Vec<String>, ObservationError> {
        let _guard = self.read_lock()?;
        Ok(self
            .metadata(self.manifest()?.as_ref())?
            .replacements
            .iter()
            .map(|r| r.replacement_id.clone())
            .collect())
    }
    pub fn read_level_references(&self) -> Result<Vec<LevelReference>, ObservationError> {
        let _guard = self.read_lock()?;
        let mut refs = self.metadata(self.manifest()?.as_ref())?.references.clone();
        validate_level_references(&refs)?;
        refs.sort_by(|a, b| {
            (&a.station_id, &a.kind, a.date)
                .cmp(&(&b.station_id, &b.kind, b.date))
                .then_with(|| a.level_cm.total_cmp(&b.level_cm))
        });
        Ok(refs)
    }
    pub fn replace_level_references(
        &mut self,
        refs: &[LevelReference],
    ) -> Result<(), ObservationError> {
        validate_level_references(refs)?;
        self.mutate(|_, _, meta, _| {
            if refs
                .iter()
                .any(|r| !meta.stations.iter().any(|s| s.station_id == r.station_id))
            {
                return Err(error("unknown reference station"));
            }
            meta.references = refs.to_vec();
            Ok(())
        })
    }
}

fn raw_series(
    meta: &Metadata,
    station: &str,
    variable: &str,
) -> Result<Option<String>, ObservationError> {
    let matches = meta
        .series
        .iter()
        .filter(|s| s.station_id == station && s.variable == variable && s.state == "raw")
        .collect::<Vec<_>>();
    if matches.len() > 1 {
        return Err(error(format!(
            "multiple raw {variable} series for ANA station {station}"
        )));
    }
    Ok(matches.first().map(|s| s.series_id.clone()))
}
impl ObservationStore {
    pub(crate) fn station_inventory(
        &self,
    ) -> Result<Vec<(Station, Vec<String>)>, ObservationError> {
        let _guard = self.read_lock()?;
        let m = self.manifest()?;
        let metadata = self.metadata(&m)?;
        let mut indexed: BTreeMap<&str, Vec<String>> = BTreeMap::new();
        for item in &metadata.variables {
            indexed
                .entry(&item.station_id)
                .or_default()
                .push(item.variable.clone());
        }
        let mut rows = metadata
            .stations
            .iter()
            .map(|station| {
                let mut variables = indexed
                    .remove(station.station_id.as_str())
                    .unwrap_or_default();
                variables.sort();
                (station.clone(), variables)
            })
            .collect::<Vec<_>>();
        rows.sort_by(|a, b| a.0.station_id.cmp(&b.0.station_id));
        Ok(rows)
    }
    pub fn observed_variables(&self, station_id: &str) -> Result<Vec<String>, ObservationError> {
        let _guard = self.read_lock()?;
        let meta = self.metadata(self.manifest()?.as_ref())?;
        let mut result = meta
            .variables
            .iter()
            .filter(|v| v.station_id == station_id)
            .map(|v| v.variable.clone())
            .collect::<Vec<_>>();
        result.sort();
        Ok(result)
    }
    pub(crate) fn import_ana_inventory(
        &mut self,
        stations: &[Station],
        entries: &[(String, Vec<String>)],
    ) -> Result<(), ObservationError> {
        self.mutate(|_, _, meta, _| {
            for station in stations {
                validate_station(station)?;
                if station.provider_code != "ana" {
                    return Err(error("invalid ANA provider"));
                }
                if meta.stations.iter().any(|s| {
                    s.provider_code == station.provider_code
                        && s.station_code == station.station_code
                        && s.station_id != station.station_id
                }) {
                    return Err(error("conflicting ANA station identity"));
                }
                if let Some(s) = meta
                    .stations
                    .iter_mut()
                    .find(|s| s.station_id == station.station_id)
                {
                    if s.provider_code != station.provider_code
                        || s.station_code != station.station_code
                    {
                        return Err(error("conflicting ANA station identity"));
                    }
                    *s = station.clone();
                } else {
                    meta.stations.push(station.clone());
                }
            }
            for (station, variables) in entries {
                if !meta.stations.iter().any(|s| s.station_id == *station) {
                    return Err(error("unknown inventory station"));
                }
                meta.variables.retain(|v| v.station_id != *station);
                let mut seen = HashSet::new();
                for variable in variables {
                    if !matches!(variable.as_str(), "rain" | "level" | "flow")
                        || !seen.insert(variable)
                    {
                        return Err(error("invalid inventory variable"));
                    }
                    meta.variables.push(InventoryVariable {
                        station_id: station.clone(),
                        variable: variable.clone(),
                    });
                }
            }
            Ok(())
        })
    }
    #[cfg(test)]
    pub(crate) fn ana_raw_times(
        &self,
        station: &str,
        variable: &str,
        start: NaiveDateTime,
        end: NaiveDateTime,
    ) -> Result<Vec<NaiveDateTime>, ObservationError> {
        let _guard = self.read_lock()?;
        let m = self.manifest()?;
        let meta = self.metadata(&m)?;
        let Some(id) = raw_series(&meta, station, variable)? else {
            return Ok(Vec::new());
        };
        Ok(self
            .samples_range(&m, &partition_key(&id, start), start, end)?
            .into_iter()
            .filter(|s| s.observed_at >= start && s.observed_at < end && s.value.is_some())
            .map(|s| s.observed_at)
            .collect())
    }
    pub(crate) fn upsert_ana_raw(
        &mut self,
        station: &str,
        created_at: NaiveDateTime,
        variables: &[(&str, ProviderSamples)],
    ) -> Result<usize, ObservationError> {
        let _lock = self.lock()?;
        let mut m = (*self.manifest()?).clone();
        let mut meta = (*self.metadata(&m)?).clone();
        let old = meta.clone();
        let zone = m.time_zone.parse::<chrono_tz::Tz>().map_err(io_error)?;
        if !meta
            .stations
            .iter()
            .any(|s| s.station_id == station && s.provider_code == "ana")
        {
            return Err(error(format!("unknown ANA station {station}")));
        }
        let mut written = 0;
        let mut updates: BTreeMap<String, BTreeMap<NaiveDateTime, ObservationSample>> =
            BTreeMap::new();
        for (variable, values) in variables {
            let (unit, interval) = match *variable {
                "rain" => ("mm", Some(3600)),
                "level" => ("cm", None),
                "flow" => ("m3/s", None),
                _ => return Err(error("unsupported ANA raw variable")),
            };
            let id = if let Some(id) = raw_series(&meta, station, variable)? {
                id
            } else {
                let series = ObservationSeries {
                    series_id: format!("{station}.{variable}.raw"),
                    station_id: station.into(),
                    variable: (*variable).into(),
                    state: "raw".into(),
                    unit: unit.into(),
                    interval_seconds: interval,
                    created_at,
                };
                validate_series(&series)?;
                if meta.series.iter().any(|s| s.series_id == series.series_id) {
                    return Err(error("conflicting ANA series"));
                }
                let id = series.series_id.clone();
                meta.series.push(series);
                id
            };
            if values.is_empty() {
                continue;
            }
            let start = values.iter().map(|(at, _)| *at).min().expect("not empty");
            let end = values.iter().map(|(at, _)| *at).max().expect("not empty");
            let key = partition_key(&id, start);
            let mut current = self
                .samples_range(&m, &key, start, end)?
                .into_iter()
                .map(|s| (s.observed_at, s))
                .collect::<BTreeMap<_, _>>();
            if let Some(prior) = updates.get(&key) {
                current.extend(prior.clone());
            }
            for (at, value) in values {
                local_time(zone, *at, &id)?;
                let sample = ObservationSample {
                    series_id: id.clone(),
                    observed_at: *at,
                    value: Some(*value),
                };
                validate_sample(&sample, variable)?;
                if current.get(at) != Some(&sample) {
                    current.insert(*at, sample.clone());
                    updates.entry(key.clone()).or_default().insert(*at, sample);
                    written += 1;
                    if *variable == "rain" {
                        meta.flags
                            .retain(|f| f.series_id != id || f.observed_at != *at);
                    }
                }
            }
        }
        let mut bases = BTreeMap::new();
        let mut deltas = BTreeMap::new();
        for (key, rows) in updates {
            let prior = m.deltas.get(&key).map(Vec::as_slice).unwrap_or(&[]);
            let mut total = rows.len();
            for path in prior {
                total = total
                    .checked_add(
                        parquet::sample_row_count(&self.path.join(path)).map_err(io_error)?,
                    )
                    .ok_or_else(|| error("delta row count overflow"))?;
            }
            if total >= MAX_DELTA_ROWS {
                let mut merged = self
                    .samples(&m, &key)?
                    .into_iter()
                    .map(|s| (s.observed_at, s))
                    .collect::<BTreeMap<_, _>>();
                merged.extend(rows);
                bases.insert(
                    key,
                    PendingPartition {
                        original: None,
                        rows: merged.into_values().collect(),
                    },
                );
            } else if prior.len() >= MAX_DELTA_SEGMENTS {
                // Consolidate only the revisions. The base is immutable until
                // the bounded revision count requires a full merge.
                let mut delta_only = m.clone();
                delta_only.partitions.remove(&key);
                let mut merged = self
                    .samples(&delta_only, &key)?
                    .into_iter()
                    .map(|s| (s.observed_at, s))
                    .collect::<BTreeMap<_, _>>();
                merged.extend(rows);
                m.deltas.remove(&key);
                deltas.insert(key, merged.into_values().collect());
            } else {
                deltas.insert(key, rows.into_values().collect());
            }
        }
        if meta != old || !bases.is_empty() || !deltas.is_empty() {
            self.publish(&mut m, &meta, Some(&old), bases, deltas)?;
        }
        Ok(written)
    }
    pub(crate) fn read_precipitation_flags(
        &self,
        series_ids: &[String],
        window: &super::IntervalAxis,
    ) -> Result<Vec<(String, NaiveDateTime, String)>, ObservationError> {
        let _guard = self.read_lock()?;
        window.validate().map_err(io_error)?;
        let manifest = self.manifest()?;
        if manifest.time_zone != window.time_zone {
            return Err(error("observation and request time zones differ"));
        }
        let start = window.bounds.first().expect("valid window").start;
        let end = window.bounds.last().expect("valid window").end;
        let metadata = self.metadata(&manifest)?;
        Ok(metadata
            .flags
            .iter()
            .filter(|f| {
                series_ids.contains(&f.series_id) && f.observed_at > start && f.observed_at <= end
            })
            .map(|f| (f.series_id.clone(), f.observed_at, f.reason.clone()))
            .collect())
    }
    pub(crate) fn replace_precipitation_flags(
        &mut self,
        series_ids: &[String],
        window: &super::IntervalAxis,
        flags: &[(String, NaiveDateTime, String)],
        expected: Option<&[ObservationSample]>,
    ) -> Result<BTreeSet<(String, NaiveDateTime)>, ObservationError> {
        window.validate().map_err(io_error)?;
        let start = window.bounds.first().expect("valid window").start;
        let end = window.bounds.last().expect("valid window").end;
        self.mutate(|store, m, meta, changed| {
            if let Some(expected) = expected {
                for id in series_ids {
                    let mut current = BTreeMap::new();
                    let key = partition_key(id, start);
                    let samples = match changed.get(&key) {
                        Some(v) => v.rows.clone(),
                        None => store.samples_range(m, &key, start, end)?,
                    };
                    for sample in samples {
                        if sample.observed_at > start && sample.observed_at <= end {
                            current.insert(sample.observed_at, sample.value);
                        }
                    }
                    let expected = expected
                        .iter()
                        .filter(|s| s.series_id == *id)
                        .map(|s| (s.observed_at, s.value))
                        .collect::<BTreeMap<_, _>>();
                    if current != expected {
                        return Err(error(format!(
                            "{id}: rain changed while evaluating exclusions"
                        )));
                    }
                }
            }
            meta.flags.retain(|f| {
                !series_ids.contains(&f.series_id) || f.observed_at <= start || f.observed_at > end
            });
            let mut excluded = BTreeSet::new();
            for (id, at, reason) in flags {
                if !matches!(reason.as_str(), "threshold" | "sequence") {
                    return Err(error("invalid precipitation flag reason"));
                }
                let key = partition_key(id, *at);
                let samples = match changed.get(&key) {
                    Some(v) => v.rows.clone(),
                    None => store.samples_range(m, &key, *at, *at)?,
                };
                let sample = samples
                    .iter()
                    .find(|s| s.observed_at == *at)
                    .ok_or_else(|| error("flag references unknown sample"))?;
                if meta
                    .flags
                    .iter()
                    .any(|f| f.series_id == *id && f.observed_at == *at && f.reason == *reason)
                {
                    return Err(error("duplicate precipitation flag"));
                }
                if sample.value.is_some() {
                    excluded.insert((id.clone(), *at));
                }
                meta.flags.push(Flag {
                    series_id: id.clone(),
                    observed_at: *at,
                    reason: reason.clone(),
                });
            }
            Ok(excluded)
        })
    }
}

fn coverage_key(variables: &[String]) -> Result<String, ObservationError> {
    let mut values = variables.to_vec();
    values.sort();
    values.dedup();
    if values.is_empty()
        || values
            .iter()
            .any(|v| !matches!(v.as_str(), "rain" | "level" | "flow"))
    {
        return Err(error("invalid ANA coverage variables"));
    }
    Ok(values.join(","))
}
impl ObservationStore {
    pub(crate) fn ana_coverage(
        &self,
        station: &str,
        variables: &[String],
    ) -> Result<BTreeSet<chrono::NaiveDate>, ObservationError> {
        let _guard = self.read_lock()?;
        let key = coverage_key(variables)?;
        let meta = self.metadata(self.manifest()?.as_ref())?;
        Ok(meta
            .coverage
            .iter()
            .filter(|c| c.station_id == station && c.variables_key == key)
            .map(|c| c.day)
            .collect())
    }
    pub(crate) fn mark_ana_coverage(
        &mut self,
        station: &str,
        variables: &[String],
        days: &[chrono::NaiveDate],
    ) -> Result<(), ObservationError> {
        let key = coverage_key(variables)?;
        self.mutate(|_, _, meta, _| {
            if !meta
                .stations
                .iter()
                .any(|s| s.station_id == station && s.provider_code == "ana")
            {
                return Err(error("unknown ANA station"));
            }
            for day in days {
                if !meta
                    .coverage
                    .iter()
                    .any(|c| c.station_id == station && c.variables_key == key && c.day == *day)
                {
                    meta.coverage.push(Coverage {
                        station_id: station.into(),
                        variables_key: key.clone(),
                        day: *day,
                    });
                }
            }
            Ok(())
        })
    }
}

#[cfg(test)]
mod parquet_tests {
    use super::*;
    use ::parquet::{
        basic::{Compression, Type},
        column::page::Page,
        file::reader::{FileReader, SerializedFileReader},
    };

    fn time(s: &str) -> NaiveDateTime {
        timestamp(s, "test").unwrap()
    }
    #[test]
    fn projected_selection_rejects_corruption_in_unrelated_metadata() {
        use arrow_array::{ArrayRef, Float64Array, Int64Array, RecordBatch, StringArray};
        use arrow_schema::{DataType, Field, Schema};
        use std::sync::Arc;
        fs::create_dir_all("runs").unwrap();
        let dir = tempfile::tempdir_in("runs").unwrap();
        let mut store = ObservationStore::create(&dir.path().join("obs"), "UTC").unwrap();
        let at = time("2026-01-01T01:00:00");
        let selected = Station {
            station_id: "selected".into(),
            provider_code: "ana".into(),
            station_code: "1".into(),
            name: "Selected".into(),
            latitude: Some(0.0),
            longitude: Some(0.0),
        };
        let other = Station {
            station_id: "other".into(),
            provider_code: "ana".into(),
            station_code: "2".into(),
            name: "Other".into(),
            latitude: Some(0.0),
            longitude: Some(0.0),
        };
        store
            .write_batch(&ObservationBatch {
                stations: vec![selected.clone(), other.clone()],
                series: vec![ObservationSeries {
                    series_id: "rain".into(),
                    station_id: "selected".into(),
                    variable: "rain".into(),
                    state: "raw".into(),
                    unit: "mm".into(),
                    interval_seconds: Some(3600),
                    created_at: at,
                }],
                ..Default::default()
            })
            .unwrap();
        store
            .import_ana_inventory(
                &[selected, other],
                &[
                    ("selected".into(), vec!["rain".into()]),
                    ("other".into(), vec!["rain".into()]),
                ],
            )
            .unwrap();
        let manifest = store.manifest().unwrap();
        let axis = super::super::IntervalAxis {
            time_zone: "UTC".into(),
            bounds: vec![super::super::TimeInterval {
                start: at - chrono::Duration::hours(1),
                end: at,
            }],
        };
        let rejected = |expected: &str| {
            let projected =
                ObservationStore::read_station_selection(&store.path, "selected", &axis, &[])
                    .unwrap_err();
            let full = ObservationStore::open_read_only(&store.path).err().unwrap();
            assert!(projected.0.contains(expected), "{}", projected.0);
            assert!(full.0.contains(expected), "{}", full.0);
        };
        let station_path = store.path.join(manifest.tables.get("stations").unwrap());
        let original = fs::read(&station_path).unwrap();
        let station_schema = Arc::new(Schema::new(vec![
            Field::new("station_id", DataType::Utf8, true),
            Field::new("provider_code", DataType::Utf8, true),
            Field::new("station_code", DataType::Utf8, true),
            Field::new("name", DataType::Utf8, true),
            Field::new("latitude", DataType::Float64, true),
            Field::new("longitude", DataType::Float64, true),
        ]));
        for (name, latitude, expected) in [
            (None, Some(0.0), "null required text"),
            (Some("Other"), Some(f64::NAN), "nonfinite float"),
        ] {
            let arrays: Vec<ArrayRef> = vec![
                Arc::new(StringArray::from(Vec::from([
                    Some("selected"),
                    Some("other"),
                ]))),
                Arc::new(StringArray::from(Vec::from([Some("ana"), Some("ana")]))),
                Arc::new(StringArray::from(Vec::from([Some("1"), Some("2")]))),
                Arc::new(StringArray::from(Vec::from([Some("Selected"), name]))),
                Arc::new(Float64Array::from(Vec::from([Some(0.0), latitude]))),
                Arc::new(Float64Array::from(Vec::from([Some(0.0), Some(0.0)]))),
            ];
            let batch = RecordBatch::try_new(station_schema.clone(), arrays).unwrap();
            parquet::write_batch_staged(&station_path, &batch, 1024).unwrap();
            rejected(expected);
        }
        fs::write(&station_path, original).unwrap();
        let series_path = store.path.join(manifest.tables.get("series").unwrap());
        let original = fs::read(&series_path).unwrap();
        let series_schema = Arc::new(Schema::new(vec![
            Field::new("series_id", DataType::Utf8, true),
            Field::new("station_id", DataType::Utf8, true),
            Field::new("variable", DataType::Utf8, true),
            Field::new("state", DataType::Utf8, true),
            Field::new("unit", DataType::Utf8, true),
            Field::new("interval_seconds", DataType::Int64, true),
            Field::new("created_at", DataType::Utf8, true),
        ]));
        let arrays: Vec<ArrayRef> = vec![
            Arc::new(StringArray::from(Vec::from([
                Some("rain"),
                Some("other-rain"),
            ]))),
            Arc::new(StringArray::from(Vec::from([
                Some("selected"),
                Some("other"),
            ]))),
            Arc::new(StringArray::from(Vec::from([Some("rain"), Some("rain")]))),
            Arc::new(StringArray::from(Vec::from([Some("raw"), Some("raw")]))),
            Arc::new(StringArray::from(Vec::from([Some("mm"), Some("mm")]))),
            Arc::new(Int64Array::from(Vec::from([Some(3600), Some(3600)]))),
            Arc::new(StringArray::from(Vec::from([
                Some("2026-01-01T01:00:00"),
                Some("bad-time"),
            ]))),
        ];
        let batch = RecordBatch::try_new(series_schema, arrays).unwrap();
        parquet::write_batch_staged(&series_path, &batch, 1024).unwrap();
        assert!(
            ObservationStore::read_station_selection(&store.path, "selected", &axis, &[]).is_err()
        );
        assert!(ObservationStore::open_read_only(&store.path).is_err());
        fs::write(&series_path, original).unwrap();
        let variable_path = store.path.join(manifest.tables.get("variables").unwrap());
        let original = fs::read(&variable_path).unwrap();
        let variable_schema = Arc::new(Schema::new(vec![
            Field::new("station_id", DataType::Utf8, true),
            Field::new("variable", DataType::Utf8, true),
        ]));
        let arrays: Vec<ArrayRef> = vec![
            Arc::new(StringArray::from(Vec::from([
                Some("selected"),
                Some("other"),
            ]))),
            Arc::new(StringArray::from(Vec::from([Some("rain"), None]))),
        ];
        let batch = RecordBatch::try_new(variable_schema, arrays).unwrap();
        parquet::write_batch_staged(&variable_path, &batch, 1024).unwrap();
        rejected("null required text");
        fs::write(&variable_path, original).unwrap();
        let reference_path = store.path.join(manifest.tables.get("references").unwrap());
        fs::remove_file(reference_path).unwrap();
        assert!(
            ObservationStore::read_station_selection(&store.path, "selected", &axis, &[]).is_err()
        );
        assert!(ObservationStore::open_read_only(&store.path).is_err());
    }
    #[test]
    fn typed_station_decoder_rejects_null_required_fields_and_nonfinite_coordinates() {
        use arrow_array::{ArrayRef, Float64Array, RecordBatch, StringArray};
        use arrow_schema::{DataType, Field, Schema};
        use std::sync::Arc;
        fs::create_dir_all("runs").unwrap();
        let dir = tempfile::tempdir_in("runs").unwrap();
        let store = ObservationStore::create(&dir.path().join("obs"), "UTC").unwrap();
        let manifest = store.manifest().unwrap();
        let path = store.path.join(manifest.tables.get("stations").unwrap());
        let schema = Arc::new(Schema::new(vec![
            Field::new("station_id", DataType::Utf8, true),
            Field::new("provider_code", DataType::Utf8, true),
            Field::new("station_code", DataType::Utf8, true),
            Field::new("name", DataType::Utf8, true),
            Field::new("latitude", DataType::Float64, true),
            Field::new("longitude", DataType::Float64, true),
        ]));
        for (name, latitude) in [(None, Some(0.0)), (Some("Station"), Some(f64::NAN))] {
            let arrays: Vec<ArrayRef> = vec![
                Arc::new(StringArray::from(Vec::from([Some("s")]))),
                Arc::new(StringArray::from(Vec::from([Some("ana")]))),
                Arc::new(StringArray::from(Vec::from([Some("1")]))),
                Arc::new(StringArray::from(Vec::from([name]))),
                Arc::new(Float64Array::from(Vec::from([latitude]))),
                Arc::new(Float64Array::from(Vec::from([Some(0.0)]))),
            ];
            let batch = RecordBatch::try_new(schema.clone(), arrays).unwrap();
            parquet::write_batch_staged(&path, &batch, 1024).unwrap();
            assert!(read_stations(&store.path, &manifest, None).is_err());
            let legacy_decode = parquet::read::<Station>(&path, STATIONS);
            assert!(legacy_decode.is_err());
        }
    }
    #[test]
    fn ana_updates_overlay_and_compact_without_losing_values() {
        fs::create_dir_all("runs").unwrap();
        let dir = tempfile::tempdir_in("runs").unwrap();
        let mut store = ObservationStore::create(&dir.path().join("obs"), "UTC").unwrap();
        let at = time("2026-01-01T01:00:00");
        store
            .write_batch(&ObservationBatch {
                stations: vec![Station {
                    station_id: "s".into(),
                    provider_code: "ana".into(),
                    station_code: "1".into(),
                    name: "S".into(),
                    latitude: None,
                    longitude: None,
                }],
                series: vec![ObservationSeries {
                    series_id: "rain".into(),
                    station_id: "s".into(),
                    variable: "rain".into(),
                    state: "raw".into(),
                    unit: "mm".into(),
                    interval_seconds: Some(3600),
                    created_at: at,
                }],
                samples: vec![
                    ObservationSample {
                        series_id: "rain".into(),
                        observed_at: at,
                        value: Some(ObservationValue::RainMm(0.0)),
                    },
                    ObservationSample {
                        series_id: "rain".into(),
                        observed_at: at - chrono::Duration::hours(1),
                        value: None,
                    },
                ],
                ..Default::default()
            })
            .unwrap();
        let initial_manifest = store.manifest().unwrap();
        let base_key = partition_key("rain", at);
        let original_base = initial_manifest.partitions.get(&base_key).unwrap().clone();
        let other_at = at - chrono::Duration::hours(2);
        let other_values = vec![(other_at, ObservationValue::RainMm(7.0))];
        assert_eq!(
            store
                .upsert_ana_raw("s", at, &[("rain", other_values)])
                .unwrap(),
            1
        );
        for i in 1..=19 {
            let values = vec![(at, ObservationValue::RainMm(f64::from(i)))];
            assert_eq!(
                store
                    .upsert_ana_raw("s", at, &[("rain", values.clone())])
                    .unwrap(),
                1
            );
            assert_eq!(
                store.upsert_ana_raw("s", at, &[("rain", values)]).unwrap(),
                0
            );
            let m = store.manifest().unwrap();
            let key = partition_key("rain", at);
            assert!(
                m.deltas
                    .get(&key)
                    .is_none_or(|v| v.len() <= MAX_DELTA_SEGMENTS)
            );
            assert_eq!(m.partitions.get(&key), Some(&original_base));
            let samples = store.samples(&m, &key).unwrap();
            assert_eq!(samples.len(), 3);
            assert_eq!(samples[0].value, Some(ObservationValue::RainMm(7.0)));
            assert_eq!(samples[1].value, None);
            assert_eq!(
                samples[2].value,
                Some(ObservationValue::RainMm(f64::from(i)))
            );
        }
        let bulk = (0..MAX_DELTA_ROWS)
            .map(|i| {
                (
                    at + chrono::Duration::hours(i as i64 + 1),
                    ObservationValue::RainMm(i as f64),
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(
            store.upsert_ana_raw("s", at, &[("rain", bulk)]).unwrap(),
            MAX_DELTA_ROWS
        );
        let reopened = ObservationStore::open_read_only(&store.path).unwrap();
        let m = reopened.manifest().unwrap();
        assert_ne!(m.partitions.get(&base_key), Some(&original_base));
        let no_deltas = !m.deltas.contains_key(&base_key);
        assert!(no_deltas);
        let values = reopened.samples(&m, &base_key).unwrap();
        assert_eq!(values.len(), MAX_DELTA_ROWS + 3);
        assert_eq!(values[0].value, Some(ObservationValue::RainMm(7.0)));
        assert_eq!(values[1].value, None);
        assert_eq!(values[2].value, Some(ObservationValue::RainMm(19.0)));
        assert_eq!(
            values.last().unwrap().value,
            Some(ObservationValue::RainMm((MAX_DELTA_ROWS - 1) as f64))
        );
    }
    #[test]
    fn readers_pin_generations_until_writer_finishes() {
        use std::{sync::mpsc, time::Duration};
        fs::create_dir_all("runs").unwrap();
        let dir = tempfile::tempdir_in("runs").unwrap();
        let path = dir.path().join("assets");
        let reader = ObservationStore::create(&path, "UTC").unwrap();
        let guard = reader.read_lock().unwrap();
        let (tx, rx) = mpsc::channel();
        let target = path.clone();
        let writer = std::thread::spawn(move || {
            let mut store = ObservationStore::open_writable(&target).unwrap();
            tx.send(()).unwrap();
            store
                .write_batch(&ObservationBatch {
                    stations: vec![Station {
                        station_id: "s".into(),
                        provider_code: "ana".into(),
                        station_code: "1".into(),
                        name: "S".into(),
                        latitude: None,
                        longitude: None,
                    }],
                    ..Default::default()
                })
                .unwrap();
            tx.send(()).unwrap();
        });
        rx.recv_timeout(Duration::from_secs(5)).unwrap();
        assert!(rx.recv_timeout(Duration::from_millis(50)).is_err());
        drop(guard);
        rx.recv_timeout(Duration::from_secs(5)).unwrap();
        writer.join().unwrap();
        assert_eq!(reader.station_locations().unwrap().len(), 1);
        let m = reader.manifest().unwrap();
        let mut references = m
            .tables
            .values()
            .chain(m.partitions.values())
            .chain(m.deltas.values().flatten())
            .cloned()
            .collect::<HashSet<_>>();
        references.insert(format!("generations/{}/manifest.json", m.generation));
        let mut total = 0;
        for dir in fs::read_dir(path.join("generations")).unwrap() {
            for file in fs::read_dir(dir.unwrap().path()).unwrap() {
                let file = file.unwrap().path();
                let rel = file
                    .strip_prefix(&path)
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .replace('\\', "/");
                assert!(references.contains(&rel));
                total += 1;
            }
        }
        assert_eq!(total, references.len());
    }
    #[test]
    fn typed_samples_and_atomic_publication() {
        fs::create_dir_all("runs").unwrap();
        let temp = tempfile::tempdir_in("runs").unwrap();
        let root = temp.path().join("observations");
        let mut store = ObservationStore::create(&root, "UTC").unwrap();
        let station = Station {
            station_id: "s".into(),
            provider_code: "ana".into(),
            station_code: "1".into(),
            name: "Station".into(),
            latitude: Some(0.0),
            longitude: Some(0.0),
        };
        let series = ObservationSeries {
            series_id: "rain".into(),
            station_id: "s".into(),
            variable: "rain".into(),
            state: "raw".into(),
            unit: "mm".into(),
            interval_seconds: Some(3600),
            created_at: time("2026-01-01T00:00:00"),
        };
        let samples = vec![
            ObservationSample {
                series_id: "rain".into(),
                observed_at: time("2026-01-01T01:00:00"),
                value: Some(ObservationValue::RainMm(0.0)),
            },
            ObservationSample {
                series_id: "rain".into(),
                observed_at: time("2026-01-01T02:00:00"),
                value: None,
            },
            ObservationSample {
                series_id: "rain".into(),
                observed_at: time("2026-02-01T01:00:00"),
                value: Some(ObservationValue::RainMm(1.23456789012345)),
            },
        ];
        let batch = ObservationBatch {
            stations: vec![station],
            series: vec![series],
            samples: samples.clone(),
            ..Default::default()
        };
        store.write_batch(&batch).unwrap();
        store.ingest_batch(&batch).unwrap();
        let before = store.manifest().unwrap();
        let mut conflict = batch.clone();
        conflict.samples[0].value = Some(ObservationValue::RainMm(1.0));
        assert!(store.ingest_batch(&conflict).is_err());
        assert_eq!(store.manifest().unwrap().generation, before.generation);
        let axis = super::super::IntervalAxis {
            time_zone: "UTC".into(),
            bounds: vec![super::super::TimeInterval {
                start: time("2026-01-01T00:00:00"),
                end: time("2026-02-01T01:00:00"),
            }],
        };
        let selection = ObservationSelection {
            station_ids: vec!["s".into()],
            provider_codes: vec![],
        };
        assert_eq!(
            store
                .read_selection(&selection, &axis, &[])
                .unwrap()
                .0
                .samples,
            samples
        );
        let path = root.join(
            before
                .partitions
                .get(&partition_key("rain", samples[0].observed_at))
                .unwrap(),
        );
        let reader = SerializedFileReader::new(File::open(&path).unwrap()).unwrap();
        assert_eq!(reader.metadata().file_metadata().version(), 2);
        let row_group = reader.metadata().row_group(0);
        let compressed = matches!(row_group.column(2).compression(), Compression::ZSTD(_));
        assert!(compressed);
        assert_eq!(row_group.column(2).column_type(), Type::DOUBLE);
        assert_eq!(row_group.column(4).column_type(), Type::INT32);
        let group = reader.get_row_group(0).unwrap();
        let mut pages = group.get_column_page_reader(2).unwrap();
        let mut v2 = false;
        while let Some(page) = pages.get_next_page().unwrap() {
            if matches!(page, Page::DataPageV2 { .. }) {
                v2 = true;
            }
        }
        assert!(v2);
        let rows = parquet::read::<StoredSample>(&path, SAMPLES).unwrap();
        assert_eq!(rows[0].rain, Some(0.0));
        assert_eq!(rows[1].rain, None);
        let flags = [(
            "rain".to_string(),
            samples[0].observed_at,
            "threshold".to_string(),
        )];
        let window = super::super::IntervalAxis {
            time_zone: "UTC".into(),
            bounds: vec![super::super::TimeInterval {
                start: time("2026-01-01T00:00:00"),
                end: time("2026-01-01T02:00:00"),
            }],
        };
        assert_eq!(
            store
                .replace_precipitation_flags(&["rain".into()], &window, &flags, Some(&samples[..2]))
                .unwrap()
                .len(),
            1
        );
        assert!(
            store
                .replace_precipitation_flags(&["rain".into()], &window, &[], Some(&[]))
                .is_err()
        );
        assert_eq!(
            store
                .read_precipitation_flags(&["rain".into()], &window)
                .unwrap(),
            flags
        );
        let selected = store.read_selection(&selection, &axis, &[]).unwrap().0;
        assert_eq!(selected.samples, samples);
        assert!(
            selected
                .excluded_rain
                .contains(&("rain".into(), samples[0].observed_at))
        );
    }
}
