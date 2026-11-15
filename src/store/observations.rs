use std::collections::{HashMap, HashSet};
use std::path::Path;

use chrono::{Datelike, NaiveDateTime, TimeZone, Timelike};
use rusqlite::{Connection, OpenFlags, OptionalExtension, params, types::ValueRef};
use serde::{Deserialize, Serialize};

const SCHEMA: &str = include_str!("sql/observations_v1.sql");
const TIME_FORMAT: &str = "%Y-%m-%dT%H:%M:%S";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservationError(pub String);
impl std::fmt::Display for ObservationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for ObservationError {}
impl From<rusqlite::Error> for ObservationError {
    fn from(value: rusqlite::Error) -> Self {
        Self(value.to_string())
    }
}
fn error(message: impl Into<String>) -> ObservationError {
    ObservationError(message.into())
}
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
fn label(value: NaiveDateTime) -> String {
    value.format(TIME_FORMAT).to_string()
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
pub struct ObservationReplacement {
    pub replacement_id: String,
    pub station_id: String,
    pub variable: String,
    pub kind: ReplacementKind,
    pub start_at: NaiveDateTime,
    pub end_at: Option<NaiveDateTime>,
    pub value: Option<f64>,
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
fn validate_replacement(r: &ObservationReplacement) -> Result<(), ObservationError> {
    validate_time(r.start_at, &r.replacement_id)?;
    if let Some(end) = r.end_at {
        validate_time(end, &r.replacement_id)?;
    }
    nonempty(&r.replacement_id, "replacement ID")?;
    nonempty(&r.station_id, "replacement station")?;
    nonempty(&r.reason, "replacement reason")?;
    if r.variable != "rain"
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

pub struct ObservationStore {
    connection: Connection,
}
impl ObservationStore {
    pub fn create(path: &Path, time_zone: &str) -> Result<Self, ObservationError> {
        time_zone
            .parse::<chrono_tz::Tz>()
            .map_err(|_| error(format!("invalid time zone {time_zone}")))?;
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .map_err(|e| error(format!("create observations {}: {e}", path.display())))?;
        let result = (|| {
            let connection = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_WRITE)?;
            connection.execute_batch("PRAGMA foreign_keys=ON;")?;
            let tx = connection.unchecked_transaction()?;
            tx.execute_batch(SCHEMA)?;
            tx.execute(
                "INSERT INTO metadata VALUES ('format','mgb-ops-observations')",
                [],
            )?;
            tx.execute("INSERT INTO metadata VALUES ('time_zone',?1)", [time_zone])?;
            tx.commit()?;
            Ok(Self { connection })
        })();
        if result.is_err() {
            let _ = std::fs::remove_file(path);
        }
        result
    }
    pub fn open_read_only(path: &Path) -> Result<Self, ObservationError> {
        let connection = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
        connection.execute_batch("PRAGMA foreign_keys=ON; PRAGMA query_only=ON;")?;
        Self::verify(&connection)?;
        Ok(Self { connection })
    }
    fn verify(connection: &Connection) -> Result<String, ObservationError> {
        let version: i64 = connection.query_row("PRAGMA user_version", [], |r| r.get(0))?;
        if version != 1 {
            return Err(error(format!(
                "unsupported observation schema version {version}"
            )));
        }
        let format: String =
            connection.query_row("SELECT value FROM metadata WHERE key='format'", [], |r| {
                r.get(0)
            })?;
        if format != "mgb-ops-observations" {
            return Err(error("invalid observations format"));
        }
        let zone: String = connection.query_row(
            "SELECT value FROM metadata WHERE key='time_zone'",
            [],
            |r| r.get(0),
        )?;
        zone.parse::<chrono_tz::Tz>()
            .map_err(|_| error(format!("invalid time zone {zone}")))?;
        Ok(zone)
    }
    pub fn time_zone(&self) -> Result<String, ObservationError> {
        Self::verify(&self.connection)
    }
    pub fn write_batch(&mut self, batch: &ObservationBatch) -> Result<(), ObservationError> {
        let tx = self.connection.transaction()?;
        let zone: chrono_tz::Tz = Self::verify(&tx)?
            .parse()
            .map_err(|_| error("invalid time zone"))?;
        let unique = |at: NaiveDateTime, context: &str| {
            if zone.from_local_datetime(&at).single().is_some() {
                Ok(())
            } else {
                Err(error(format!(
                    "{context}: ambiguous or nonexistent local timestamp {at}"
                )))
            }
        };
        for s in &batch.stations {
            validate_station(s)?;
            tx.execute(
                "INSERT INTO station VALUES (?1,?2,?3,?4,?5,?6)",
                params![
                    s.station_id,
                    s.provider_code,
                    s.station_code,
                    s.name,
                    s.latitude,
                    s.longitude
                ],
            )
            .map_err(|e| error(format!("station {}: {e}", s.station_id)))?;
        }
        for s in &batch.series {
            validate_series(s)?;
            tx.execute(
                "INSERT INTO observation_series VALUES (?1,?2,?3,?4,?5,?6,?7)",
                params![
                    s.series_id,
                    s.station_id,
                    s.variable,
                    s.state,
                    s.unit,
                    s.interval_seconds,
                    label(s.created_at)
                ],
            )
            .map_err(|e| error(format!("series {}: {e}", s.series_id)))?;
        }
        for s in &batch.samples {
            let variable: String = tx
                .query_row(
                    "SELECT variable FROM observation_series WHERE series_id=?1",
                    [&s.series_id],
                    |r| r.get(0),
                )
                .map_err(|e| error(format!("sample {}: {e}", s.series_id)))?;
            validate_sample(s, &variable)?;
            unique(s.observed_at, &s.series_id)?;
            let value = match s.value {
                None => rusqlite::types::Value::Null,
                Some(ObservationValue::LevelCm(v)) => rusqlite::types::Value::Integer(i64::from(v)),
                Some(ObservationValue::RainMm(v) | ObservationValue::FlowM3s(v)) => {
                    rusqlite::types::Value::Real(v)
                }
            };
            tx.execute(
                "INSERT INTO observation VALUES (?1,?2,?3)",
                params![s.series_id, label(s.observed_at), value],
            )
            .map_err(|e| error(format!("sample {} {}: {e}", s.series_id, s.observed_at)))?;
            if variable == "level" && s.value.is_some() {
                let storage: String = tx.query_row(
                    "SELECT typeof(value) FROM observation WHERE series_id=?1 AND observed_at=?2",
                    params![s.series_id, label(s.observed_at)],
                    |r| r.get(0),
                )?;
                if storage != "integer" {
                    return Err(error(format!(
                        "sample {}: level storage is not integer",
                        s.series_id
                    )));
                }
            }
        }
        for r in &batch.replacements {
            validate_replacement(r)?;
            unique(r.start_at, &r.replacement_id)?;
            if let Some(end) = r.end_at {
                unique(end, &r.replacement_id)?;
            }
            tx.execute(
                "INSERT INTO observation_replacement VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
                params![
                    r.replacement_id,
                    r.station_id,
                    r.variable,
                    if r.kind == ReplacementKind::Point {
                        "point"
                    } else {
                        "period"
                    },
                    label(r.start_at),
                    r.end_at.map(label),
                    r.value,
                    r.reason
                ],
            )
            .map_err(|e| error(format!("replacement {}: {e}", r.replacement_id)))?;
        }
        for o in &batch.offsets {
            validate_offset(o)?;
            tx.execute(
                "INSERT INTO station_offset VALUES (?1,?2,?3)",
                params![o.station_id, o.mini_id, o.offset_m],
            )
            .map_err(|e| error(format!("offset {}: {e}", o.station_id)))?;
        }
        tx.commit()?;
        Ok(())
    }
    pub fn read_station_offsets(
        &self,
        station_ids: &[String],
    ) -> Result<Vec<StationOffset>, ObservationError> {
        let tx = self.connection.unchecked_transaction()?;
        Self::verify(&tx)?;
        let result = offsets(&tx, station_ids)?;
        tx.commit()?;
        Ok(result)
    }
    pub fn read_selection(
        &self,
        selection: &ObservationSelection,
        interval: &super::IntervalAxis,
        replacement_ids: &[String],
    ) -> Result<(ObservationInputs, Vec<StationOffset>), ObservationError> {
        self.read_selection_with_stations(selection, interval, replacement_ids, &[])
    }

    pub fn read_selection_with_stations(
        &self,
        selection: &ObservationSelection,
        interval: &super::IntervalAxis,
        replacement_ids: &[String],
        extra_station_ids: &[String],
    ) -> Result<(ObservationInputs, Vec<StationOffset>), ObservationError> {
        if selection.provider_codes.is_empty() && selection.station_ids.is_empty() {
            return Err(error("selection requires providers or stations"));
        }
        interval.validate().map_err(|e| error(e.to_string()))?;
        let start = interval.bounds.first().unwrap().start;
        let end = interval.bounds.last().unwrap().end;
        let tx = self.connection.unchecked_transaction()?;
        if Self::verify(&tx)? != interval.time_zone {
            return Err(error("observation and request time zones differ"));
        }
        let mut inputs = ObservationInputs::default();
        {
            let mut stmt = tx.prepare("SELECT station_id,provider_code,station_code,name,latitude,longitude FROM station ORDER BY station_id")?;
            let mut rows = stmt.query([])?;
            while let Some(r) = rows.next()? {
                let s = Station {
                    station_id: r.get(0)?,
                    provider_code: r.get(1)?,
                    station_code: r.get(2)?,
                    name: r.get(3)?,
                    latitude: r.get(4)?,
                    longitude: r.get(5)?,
                };
                if selection.provider_codes.contains(&s.provider_code)
                    || selection.station_ids.contains(&s.station_id)
                    || extra_station_ids.contains(&s.station_id)
                {
                    validate_station(&s)?;
                    inputs.stations.push(s);
                }
            }
        }
        for id in &selection.station_ids {
            if !inputs.stations.iter().any(|s| &s.station_id == id) {
                return Err(error(format!("unknown station {id}")));
            }
        }
        for id in extra_station_ids {
            if !inputs
                .stations
                .iter()
                .any(|station| &station.station_id == id)
            {
                return Err(error(format!("unknown station {id}")));
            }
        }
        for id in &selection.provider_codes {
            if !inputs.stations.iter().any(|s| &s.provider_code == id) {
                return Err(error(format!("unknown provider {id}")));
            }
        }
        let ids: HashSet<_> = inputs
            .stations
            .iter()
            .map(|s| s.station_id.as_str())
            .collect();
        {
            let mut stmt = tx.prepare("SELECT series_id,station_id,variable,state,unit,interval_seconds,created_at FROM observation_series ORDER BY series_id")?;
            let mut rows = stmt.query([])?;
            while let Some(r) = rows.next()? {
                let station_id: String = r.get(1)?;
                if !ids.contains(station_id.as_str()) {
                    continue;
                }
                let series_id: String = r.get(0)?;
                let created: String = r.get(6)?;
                let s = ObservationSeries {
                    series_id,
                    station_id,
                    variable: r.get(2)?,
                    state: r.get(3)?,
                    unit: r.get(4)?,
                    interval_seconds: r.get(5)?,
                    created_at: timestamp(&created, "series created_at")?,
                };
                validate_series(&s)?;
                inputs.series.push(s);
            }
        }
        let variables: HashMap<_, _> = inputs
            .series
            .iter()
            .map(|s| (s.series_id.as_str(), s.variable.as_str()))
            .collect();
        {
            let mut stmt = tx.prepare("SELECT series_id,observed_at,value FROM observation WHERE observed_at>?1 AND observed_at<=?2 ORDER BY series_id,observed_at")?;
            let mut rows = stmt.query(params![label(start), label(end)])?;
            while let Some(r) = rows.next()? {
                let id: String = r.get(0)?;
                let Some(variable) = variables.get(id.as_str()) else {
                    continue;
                };
                let time: String = r.get(1)?;
                let at = timestamp(&time, &id)?;
                let value = match (r.get_ref(2)?, *variable) {
                    (ValueRef::Null, _) => None,
                    (ValueRef::Integer(v), "level") => Some(ObservationValue::LevelCm(
                        i32::try_from(v)
                            .map_err(|_| error(format!("sample {id} {time}: level outside i32")))?,
                    )),
                    (ValueRef::Integer(v), "rain") => Some(ObservationValue::RainMm(v as f64)),
                    (ValueRef::Real(v), "rain") => Some(ObservationValue::RainMm(v)),
                    (ValueRef::Integer(v), "flow") => Some(ObservationValue::FlowM3s(v as f64)),
                    (ValueRef::Real(v), "flow") => Some(ObservationValue::FlowM3s(v)),
                    _ => {
                        return Err(error(format!(
                            "sample {id} {time}: invalid numeric storage"
                        )));
                    }
                };
                let s = ObservationSample {
                    series_id: id,
                    observed_at: at,
                    value,
                };
                validate_sample(&s, variable)?;
                inputs.samples.push(s);
            }
        }
        let mut seen = HashSet::new();
        for id in replacement_ids {
            if !seen.insert(id) {
                return Err(error(format!("duplicate replacement {id}")));
            }
            let r = tx.query_row("SELECT station_id,variable,kind,start_at,end_at,value,reason FROM observation_replacement WHERE replacement_id=?1",[id],|r| Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?,r.get::<_,String>(3)?,r.get::<_,Option<String>>(4)?,r.get::<_,Option<f64>>(5)?,r.get::<_,String>(6)?))).optional()?.ok_or_else(|| error(format!("unknown replacement {id}")))?;
            let item = ObservationReplacement {
                replacement_id: id.clone(),
                station_id: r.0,
                variable: r.1,
                kind: match r.2.as_str() {
                    "point" => ReplacementKind::Point,
                    "period" => ReplacementKind::Period,
                    _ => return Err(error(format!("replacement {id}: invalid kind"))),
                },
                start_at: timestamp(&r.3, id)?,
                end_at: r.4.map(|v| timestamp(&v, id)).transpose()?,
                value: r.5,
                reason: r.6,
            };
            validate_replacement(&item)?;
            inputs.replacements.push(item);
        }
        let selected_ids: Vec<_> = inputs
            .stations
            .iter()
            .map(|s| s.station_id.clone())
            .collect();
        let offsets = offsets(&tx, &selected_ids)?;
        tx.commit()?;
        Ok((inputs, offsets))
    }
}
fn offsets(
    connection: &Connection,
    ids: &[String],
) -> Result<Vec<StationOffset>, ObservationError> {
    let mut result = Vec::new();
    for id in ids {
        if let Some((mini_id, offset_m)) = connection
            .query_row(
                "SELECT mini_id,offset_m FROM station_offset WHERE station_id=?1",
                [id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?
        {
            let o = StationOffset {
                station_id: id.clone(),
                mini_id,
                offset_m,
            };
            validate_offset(&o)?;
            result.push(o);
        }
    }
    Ok(result)
}
