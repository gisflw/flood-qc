use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::fs::{self, File, OpenOptions};
use std::path::Path;
use std::path::PathBuf;

use chrono::{Datelike, NaiveDateTime, TimeZone, Timelike};
use serde::{Deserialize, Serialize};

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
/// Retained source evidence associated with an observation registration.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObservationProvenance {
    pub provider: String,
    pub station_id: String,
    pub variables: Vec<String>,
    pub request_start: Option<chrono::NaiveDate>,
    pub request_end: Option<chrono::NaiveDate>,
    pub reference_time: String,
    pub source_kind: String,
    pub source_path: PathBuf,
    pub sha256: String,
    pub byte_length: u64,
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

#[derive(Clone, Serialize, Deserialize)]
struct Manifest {
    time_zone: String,
    revision: i64,
    metadata_key: String,
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
#[derive(Clone, Default, PartialEq, Serialize, Deserialize)]
struct Metadata {
    stations: Vec<Station>,
    series: Vec<ObservationSeries>,
    replacements: Vec<ObservationReplacement>,
    offsets: Vec<StationOffset>,
    references: Vec<LevelReference>,
    variables: Vec<InventoryVariable>,
    flags: Vec<Flag>,
    coverage: Vec<Coverage>,
    #[serde(default)]
    partial_coverage: Option<serde_json::Value>,
    #[serde(default)]
    provenance: Vec<ObservationProvenance>,
}
fn partition_key(id: &str, _at: NaiveDateTime) -> String {
    id.to_owned()
}
fn io_error(e: impl std::fmt::Display) -> ObservationError {
    error(e.to_string())
}

struct PendingPartition {
    original: Option<Vec<ObservationSample>>,
    rows: Vec<ObservationSample>,
}

// Each operation holds the external process lock until every database handle is dropped.
pub struct ObservationStore {
    path: PathBuf,
    writable: bool,
    selected_revision: Option<i64>,
}

struct RegistryLock(File);
impl Drop for RegistryLock {
    fn drop(&mut self) {
        let _ = self.0.unlock();
    }
}

struct Db {
    conn: turso::Connection,
    _db: turso::Database,
}
impl Db {
    fn open(path: &Path, writable: bool) -> Result<Self, ObservationError> {
        pollster::block_on(async {
            let db = turso::Builder::new_local(
                path.to_str()
                    .ok_or_else(|| error("database path is not UTF-8"))?,
            )
            .read_only(!writable)
            .build()
            .await
            .map_err(io_error)?;
            let conn = db.connect().map_err(io_error)?;
            conn.execute("PRAGMA foreign_keys=ON", ())
                .await
                .map_err(io_error)?;
            if writable {
                conn.execute("PRAGMA synchronous=FULL", ())
                    .await
                    .map_err(io_error)?;
            }
            Ok(Self { conn, _db: db })
        })
    }
}
impl ObservationStore {
    fn lock_path(&self) -> PathBuf {
        let mut name = self.path.as_os_str().to_os_string();
        name.push(".lock");
        name.into()
    }
    fn read_lock(&self) -> Result<RegistryLock, ObservationError> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(self.lock_path())
            .map_err(io_error)?;
        file.lock().map_err(io_error)?;
        Ok(RegistryLock(file))
    }
    fn lock(&self) -> Result<RegistryLock, ObservationError> {
        if !self.writable || self.selected_revision.is_some() {
            return Err(error("observation store is read-only"));
        }
        self.read_lock()
    }
    pub fn create(path: &Path, time_zone: &str) -> Result<Self, ObservationError> {
        time_zone.parse::<chrono_tz::Tz>().map_err(io_error)?;
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            fs::create_dir_all(parent).map_err(io_error)?;
        }
        let store = Self {
            path: path.into(),
            writable: true,
            selected_revision: None,
        };
        let guard = store.lock()?;
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .map_err(io_error)?;
        let result = (|| {
            let db = Db::open(path, true)?;
            pollster::block_on(async {
                db.conn.execute_batch("CREATE TABLE registry_identity (id TEXT PRIMARY KEY); CREATE TABLE observation_metadata (key TEXT PRIMARY KEY, payload TEXT NOT NULL); CREATE TABLE observation_revision (id INTEGER PRIMARY KEY, time_zone TEXT NOT NULL, metadata_key TEXT NOT NULL REFERENCES observation_metadata(key)); CREATE TABLE observation_sample (series_id TEXT NOT NULL, observed_at TEXT NOT NULL, revision INTEGER NOT NULL REFERENCES observation_revision(id), payload TEXT NOT NULL, payload_sha256 TEXT NOT NULL, PRIMARY KEY(series_id, observed_at, revision));").await.map_err(io_error)?;
                let mut random = [0u8; 32];
                getrandom::fill(&mut random).map_err(io_error)?;
                let identity = format!("{:x}", Sha256::digest(random));
                db.conn
                    .execute("INSERT INTO registry_identity VALUES (?1)", [identity])
                    .await
                    .map_err(io_error)?;
                let payload = serde_json::to_string(&Metadata::default()).map_err(io_error)?;
                let key = format!("{:x}", Sha256::digest(payload.as_bytes()));
                db.conn
                    .execute(
                        "INSERT INTO observation_metadata VALUES (?1,?2)",
                        (key.clone(), payload),
                    )
                    .await
                    .map_err(io_error)?;
                db.conn
                    .execute(
                        "INSERT INTO observation_revision VALUES (1,?1,?2)",
                        (time_zone.to_owned(), key),
                    )
                    .await
                    .map_err(io_error)?;
                Ok::<_, ObservationError>(())
            })
        })();
        if result.is_err() {
            let _ = fs::remove_file(path);
        }
        result?;
        drop(guard);
        Ok(store)
    }
    pub fn open_read_only(path: &Path) -> Result<Self, ObservationError> {
        Self::open(path, false, None)
    }
    pub fn open_writable(path: &Path) -> Result<Self, ObservationError> {
        Self::open(path, true, None)
    }
    pub fn open_revision(path: &Path, revision: i64) -> Result<Self, ObservationError> {
        Self::open(path, false, Some(revision))
    }
    fn open(
        path: &Path,
        writable: bool,
        selected_revision: Option<i64>,
    ) -> Result<Self, ObservationError> {
        if !path.is_file() {
            return Err(error("observation database does not exist"));
        }
        let store = Self {
            path: path.into(),
            writable,
            selected_revision,
        };
        let guard = store.read_lock()?;
        let m = store.manifest()?;
        store.metadata(&m)?;
        drop(guard);
        Ok(store)
    }
    /// Stable identity shared by this registry and its independent snapshots.
    pub fn registry_id(&self) -> Result<String, ObservationError> {
        let _guard = self.read_lock()?;
        self.registry_id_locked()
    }
    fn registry_id_locked(&self) -> Result<String, ObservationError> {
        let db = Db::open(&self.path, false)?;
        pollster::block_on(async {
            let mut rows = db
                .conn
                .query("SELECT id FROM registry_identity", ())
                .await
                .map_err(io_error)?;
            let row = rows
                .next()
                .await
                .map_err(io_error)?
                .ok_or_else(|| error("missing registry identity"))?;
            let id = match row.get_value(0).map_err(io_error)? {
                turso::Value::Text(id) => id,
                _ => return Err(error("invalid registry identity")),
            };
            if id.len() != 64
                || !id
                    .bytes()
                    .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
                || rows.next().await.map_err(io_error)?.is_some()
            {
                return Err(error("invalid registry identity"));
            }
            Ok(id)
        })
    }
    pub fn revision(&self) -> Result<i64, ObservationError> {
        let _guard = self.read_lock()?;
        Ok(self.manifest()?.revision)
    }
    pub fn time_zone(&self) -> Result<String, ObservationError> {
        let _guard = self.read_lock()?;
        Ok(self.manifest()?.time_zone.clone())
    }
    fn manifest(&self) -> Result<std::sync::Arc<Manifest>, ObservationError> {
        let db = Db::open(&self.path, false)?;
        pollster::block_on(async {
            let mut rows = match self.selected_revision {
                Some(id) => db.conn.query("SELECT id,time_zone,metadata_key FROM observation_revision WHERE id=?1", [id]).await,
                None => db.conn.query("SELECT id,time_zone,metadata_key FROM observation_revision ORDER BY id DESC LIMIT 1", ()).await,
            }.map_err(io_error)?;
            let row = rows
                .next()
                .await
                .map_err(io_error)?
                .ok_or_else(|| error("observation revision not found"))?;
            let revision = match row.get_value(0).map_err(io_error)? {
                turso::Value::Integer(v) => v,
                _ => return Err(error("invalid revision")),
            };
            let time_zone = match row.get_value(1).map_err(io_error)? {
                turso::Value::Text(v) => v,
                _ => return Err(error("invalid time zone")),
            };
            let metadata_key = match row.get_value(2).map_err(io_error)? {
                turso::Value::Text(v) => v,
                _ => return Err(error("invalid metadata key")),
            };
            time_zone.parse::<chrono_tz::Tz>().map_err(io_error)?;
            Ok(std::sync::Arc::new(Manifest {
                revision,
                time_zone,
                metadata_key,
            }))
        })
    }
    fn metadata(&self, m: &Manifest) -> Result<std::sync::Arc<Metadata>, ObservationError> {
        let db = Db::open(&self.path, false)?;
        pollster::block_on(async {
            let mut rows = db
                .conn
                .query(
                    "SELECT payload FROM observation_metadata WHERE key=?1",
                    [m.metadata_key.as_str()],
                )
                .await
                .map_err(io_error)?;
            let row = rows
                .next()
                .await
                .map_err(io_error)?
                .ok_or_else(|| error("missing metadata"))?;
            let payload = match row.get_value(0).map_err(io_error)? {
                turso::Value::Text(v) => v,
                _ => return Err(error("invalid metadata")),
            };
            if format!("{:x}", Sha256::digest(payload.as_bytes())) != m.metadata_key {
                return Err(error("metadata checksum mismatch"));
            }
            Ok(std::sync::Arc::new(
                serde_json::from_str(&payload).map_err(io_error)?,
            ))
        })
    }
    fn samples_range(
        &self,
        m: &Manifest,
        key: &str,
        start: NaiveDateTime,
        end: NaiveDateTime,
    ) -> Result<Vec<ObservationSample>, ObservationError> {
        let db = Db::open(&self.path, false)?;
        pollster::block_on(async {
            let mut rows = db.conn.query("SELECT observed_at,payload,payload_sha256 FROM observation_sample WHERE series_id=?1 AND observed_at>=?2 AND observed_at<=?3 AND revision<=?4 ORDER BY observed_at,revision", (key.to_owned(), start.format("%Y-%m-%dT%H:%M:%S").to_string(), end.format("%Y-%m-%dT%H:%M:%S").to_string(), m.revision)).await.map_err(io_error)?;
            let mut result = BTreeMap::new();
            while let Some(row) = rows.next().await.map_err(io_error)? {
                let at = match row.get_value(0).map_err(io_error)? {
                    turso::Value::Text(v) => v,
                    _ => return Err(error("invalid sample timestamp")),
                };
                let payload = match row.get_value(1).map_err(io_error)? {
                    turso::Value::Text(v) => v,
                    _ => return Err(error("invalid sample")),
                };
                let digest = match row.get_value(2).map_err(io_error)? {
                    turso::Value::Text(v) => v,
                    _ => return Err(error("invalid sample checksum")),
                };
                if format!("{:x}", Sha256::digest(payload.as_bytes())) != digest {
                    return Err(error("sample checksum mismatch"));
                }
                let sample: ObservationSample = serde_json::from_str(&payload).map_err(io_error)?;
                if sample.series_id != key
                    || sample.observed_at.format("%Y-%m-%dT%H:%M:%S").to_string() != at
                {
                    return Err(error("sample identity mismatch"));
                }
                result.insert(sample.observed_at, sample);
            }
            Ok(result.into_values().collect())
        })
    }
    fn samples(&self, m: &Manifest, key: &str) -> Result<Vec<ObservationSample>, ObservationError> {
        let start = chrono::NaiveDate::from_ymd_opt(0, 1, 1)
            .expect("valid")
            .and_hms_opt(0, 0, 0)
            .expect("valid");
        let end = chrono::NaiveDate::from_ymd_opt(9999, 12, 31)
            .expect("valid")
            .and_hms_opt(23, 59, 59)
            .expect("valid");
        self.samples_range(m, key, start, end)
    }
    fn publish(
        &self,
        m: &mut Manifest,
        meta: &Metadata,
        _old: Option<&Metadata>,
        changed: BTreeMap<String, PendingPartition>,
        deltas: BTreeMap<String, Vec<ObservationSample>>,
    ) -> Result<(), ObservationError> {
        let payload = serde_json::to_string(meta).map_err(io_error)?;
        let key = format!("{:x}", Sha256::digest(payload.as_bytes()));
        let revision = m
            .revision
            .checked_add(1)
            .ok_or_else(|| error("revision overflow"))?;
        let db = Db::open(&self.path, true)?;
        pollster::block_on(async {
            db.conn
                .execute("BEGIN IMMEDIATE", ())
                .await
                .map_err(io_error)?;
            let result = async {
                db.conn
                    .execute(
                        "INSERT OR IGNORE INTO observation_metadata VALUES (?1,?2)",
                        (key.clone(), payload),
                    )
                    .await
                    .map_err(io_error)?;
                db.conn
                    .execute(
                        "INSERT INTO observation_revision VALUES (?1,?2,?3)",
                        (revision, m.time_zone.clone(), key.clone()),
                    )
                    .await
                    .map_err(io_error)?;
                for (id, pending) in changed {
                    let prior: BTreeMap<_, _> = pending
                        .original
                        .unwrap_or_default()
                        .into_iter()
                        .map(|s| (s.observed_at, s))
                        .collect();
                    for sample in pending.rows {
                        if prior.get(&sample.observed_at) == Some(&sample) {
                            continue;
                        }
                        let at = sample.observed_at.format("%Y-%m-%dT%H:%M:%S").to_string();
                        let value = serde_json::to_string(&sample).map_err(io_error)?;
                        let checksum = format!("{:x}", Sha256::digest(value.as_bytes()));
                        db.conn
                            .execute(
                                "INSERT INTO observation_sample VALUES (?1,?2,?3,?4,?5)",
                                (id.clone(), at, revision, value, checksum),
                            )
                            .await
                            .map_err(io_error)?;
                    }
                }
                for (id, rows) in deltas {
                    for sample in rows {
                        let at = sample.observed_at.format("%Y-%m-%dT%H:%M:%S").to_string();
                        let value = serde_json::to_string(&sample).map_err(io_error)?;
                        let checksum = format!("{:x}", Sha256::digest(value.as_bytes()));
                        db.conn
                            .execute(
                                "INSERT INTO observation_sample VALUES (?1,?2,?3,?4,?5)",
                                (id.clone(), at, revision, value, checksum),
                            )
                            .await
                            .map_err(io_error)?;
                    }
                }
                db.conn.execute("COMMIT", ()).await.map_err(io_error)?;
                Ok::<_, ObservationError>(())
            }
            .await;
            if result.is_err() {
                let _ = db.conn.execute("ROLLBACK", ()).await;
            }
            result
        })?;
        m.revision = revision;
        m.metadata_key = key;
        Ok(())
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
        self.mutate_revision(f).map(|(value, _)| value)
    }
    fn mutate_revision<T>(
        &mut self,
        f: impl FnOnce(
            &Self,
            &Manifest,
            &mut Metadata,
            &mut BTreeMap<String, PendingPartition>,
        ) -> Result<T, ObservationError>,
    ) -> Result<(T, i64), ObservationError> {
        let _lock = self.lock()?;
        let mut m = (*self.manifest()?).clone();
        let mut meta = (*self.metadata(&m)?).clone();
        let old = meta.clone();
        let mut changed = BTreeMap::new();
        let result = f(self, &m, &mut meta, &mut changed)?;
        changed.retain(|_, p| p.original.as_ref() != Some(&p.rows));
        if meta != old || !changed.is_empty() {
            self.publish(&mut m, &meta, Some(&old), changed, BTreeMap::new())?;
        }
        Ok((result, m.revision))
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
            changed.insert(
                key.clone(),
                PendingPartition {
                    original: Some(rows.clone()),
                    rows,
                },
            );
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
        let store = Self::open_read_only(path)?;
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
        let _ = projected;
        interval.validate().map_err(io_error)?;
        let m = self.manifest()?;
        let meta = self.metadata(&m)?;
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
    #[cfg(test)]
    pub(crate) fn upsert_ana_raw(
        &mut self,
        station: &str,
        created_at: NaiveDateTime,
        variables: &[(&str, ProviderSamples)],
    ) -> Result<usize, ObservationError> {
        self.upsert_ana_raw_with_coverage(station, created_at, variables, &[], &[], None, &[])
    }
    /// Source records visible at this selected revision; legacy data may have none.
    pub fn acquisition_provenance(&self) -> Result<Vec<ObservationProvenance>, ObservationError> {
        let _guard = self.read_lock()?;
        let manifest = self.manifest()?;
        Ok(self.metadata(&manifest)?.provenance.clone())
    }
    pub(crate) fn ana_partial_coverage(
        &self,
    ) -> Result<super::AnaPartialCoverage, ObservationError> {
        let _guard = self.read_lock()?;
        let manifest = self.manifest()?;
        let meta = self.metadata(&manifest)?;
        meta.partial_coverage
            .clone()
            .map(serde_json::from_value)
            .transpose()
            .map_err(io_error)
            .map(Option::unwrap_or_default)
    }
    #[allow(clippy::too_many_arguments)] // The registration groups data, coverage and retained source evidence.
    pub(crate) fn upsert_ana_raw_with_coverage(
        &mut self,
        station: &str,
        created_at: NaiveDateTime,
        variables: &[(&str, ProviderSamples)],
        coverage_variables: &[String],
        full_days: &[chrono::NaiveDate],
        partial: Option<(chrono::NaiveDate, i64)>,
        provenance: &[ObservationProvenance],
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
        for source in provenance {
            if source.provider != "ana"
                || source.station_id != station
                || source.variables.is_empty()
                || coverage_key(&source.variables).is_err()
                || !matches!(source.source_kind.as_str(), "ana_xml" | "ana_csv")
                || !source.source_path.starts_with("downloads/ana")
                || source
                    .source_path
                    .components()
                    .any(|part| !matches!(part, std::path::Component::Normal(_)))
                || source.sha256.len() != 64
                || !source
                    .sha256
                    .bytes()
                    .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
                || chrono::DateTime::parse_from_rfc3339(&source.reference_time).is_err()
                || source
                    .request_start
                    .zip(source.request_end)
                    .is_some_and(|(a, b)| a > b)
                || source.request_start.is_some() != source.request_end.is_some()
            {
                return Err(error("invalid acquisition provenance"));
            }
            if !meta.provenance.iter().any(|existing| {
                existing.provider == source.provider
                    && existing.station_id == source.station_id
                    && existing.variables == source.variables
                    && existing.request_start == source.request_start
                    && existing.request_end == source.request_end
                    && existing.reference_time == source.reference_time
                    && existing.source_kind == source.source_kind
                    && existing.sha256 == source.sha256
                    && existing.byte_length == source.byte_length
            }) {
                meta.provenance.push(source.clone());
            }
        }
        if !full_days.is_empty() || partial.is_some() {
            let key = coverage_key(coverage_variables)?;
            for day in full_days {
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
            if let Some((day, through_utc)) = partial {
                let mut coverage: super::AnaPartialCoverage = meta
                    .partial_coverage
                    .clone()
                    .map(serde_json::from_value)
                    .transpose()
                    .map_err(io_error)?
                    .unwrap_or_default();
                coverage
                    .record(station, coverage_variables, day, through_utc)
                    .map_err(io_error)?;
                meta.partial_coverage = Some(serde_json::to_value(coverage).map_err(io_error)?);
            }
        }
        let deltas = updates
            .into_iter()
            .map(|(key, rows)| (key, rows.into_values().collect()))
            .collect::<BTreeMap<_, Vec<_>>>();
        if meta != old || !deltas.is_empty() {
            self.publish(&mut m, &meta, Some(&old), BTreeMap::new(), deltas)?;
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
    pub(crate) fn replace_precipitation_flags_for_revision(
        &mut self,
        expected_revision: i64,
        series_ids: &[String],
        window: &super::IntervalAxis,
        flags: &[(String, NaiveDateTime, String)],
        expected: &[ObservationSample],
    ) -> Result<(BTreeSet<(String, NaiveDateTime)>, i64), ObservationError> {
        self.replace_precipitation_flags_inner(
            Some(expected_revision),
            series_ids,
            window,
            flags,
            Some(expected),
        )
    }
    fn replace_precipitation_flags_inner(
        &mut self,
        expected_revision: Option<i64>,
        series_ids: &[String],
        window: &super::IntervalAxis,
        flags: &[(String, NaiveDateTime, String)],
        expected: Option<&[ObservationSample]>,
    ) -> Result<(BTreeSet<(String, NaiveDateTime)>, i64), ObservationError> {
        window.validate().map_err(io_error)?;
        let start = window.bounds.first().expect("valid window").start;
        let end = window.bounds.last().expect("valid window").end;
        self.mutate_revision(|store, m, meta, changed| {
            if m.time_zone != window.time_zone {
                return Err(error("observation and request time zones differ"));
            }
            if expected_revision.is_some_and(|revision| revision != m.revision) {
                return Err(error("observations changed while evaluating exclusions"));
            }
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
    #[cfg(test)]
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

#[derive(Deserialize)]
struct InterchangeTable {
    file: String,
    count: usize,
    sha256: String,
}
#[derive(Deserialize)]
struct InterchangeManifest {
    time_zone: String,
    tables: BTreeMap<String, InterchangeTable>,
    #[serde(default)]
    partial_coverage: Option<String>,
}
#[derive(Deserialize)]
struct InterchangeSample {
    series_id: String,
    observed_at: NaiveDateTime,
    rain: Option<f64>,
    flow: Option<f64>,
    level: Option<i32>,
}
fn interchange_path(root: &Path, name: &str) -> Result<PathBuf, ObservationError> {
    let path = Path::new(name);
    if path
        .components()
        .any(|part| !matches!(part, std::path::Component::Normal(_)))
    {
        return Err(error("invalid interchange path"));
    }
    Ok(root.join(path))
}
fn interchange_rows<T: for<'a> Deserialize<'a>>(
    root: &Path,
    manifest: &InterchangeManifest,
    name: &str,
) -> Result<Vec<T>, ObservationError> {
    let table = manifest
        .tables
        .get(name)
        .ok_or_else(|| error(format!("missing interchange table {name}")))?;
    let bytes = fs::read(interchange_path(root, &table.file)?).map_err(io_error)?;
    if format!("{:x}", Sha256::digest(&bytes)) != table.sha256 {
        return Err(error(format!("{name}: checksum mismatch")));
    }
    let rows = bytes
        .split(|b| *b == b'\n')
        .filter(|line| !line.is_empty())
        .map(|line| serde_json::from_slice(line).map_err(io_error))
        .collect::<Result<Vec<T>, _>>()?;
    if rows.len() != table.count {
        return Err(error(format!("{name}: row count mismatch")));
    }
    Ok(rows)
}
impl ObservationStore {
    /// Import an independently verified legacy interchange into a new SQLite file.
    pub fn import_interchange(path: &Path, root: &Path) -> Result<Self, ObservationError> {
        let manifest: InterchangeManifest =
            serde_json::from_slice(&fs::read(root.join("manifest.json")).map_err(io_error)?)
                .map_err(io_error)?;
        let zone = manifest
            .time_zone
            .parse::<chrono_tz::Tz>()
            .map_err(io_error)?;
        let mut meta = Metadata {
            stations: interchange_rows(root, &manifest, "stations")?,
            series: interchange_rows(root, &manifest, "series")?,
            replacements: interchange_rows(root, &manifest, "replacements")?,
            offsets: interchange_rows(root, &manifest, "offsets")?,
            references: interchange_rows(root, &manifest, "references")?,
            variables: interchange_rows(root, &manifest, "variables")?,
            flags: interchange_rows(root, &manifest, "flags")?,
            coverage: interchange_rows(root, &manifest, "coverage")?,
            partial_coverage: None,
            provenance: Vec::new(),
        };
        if let Some(file) = &manifest.partial_coverage {
            meta.partial_coverage = Some(
                serde_json::from_slice(&fs::read(interchange_path(root, file)?).map_err(io_error)?)
                    .map_err(io_error)?,
            );
        }
        if let Some(partial) = &meta.partial_coverage {
            let _: super::AnaPartialCoverage =
                serde_json::from_value(partial.clone()).map_err(io_error)?;
        }
        let mut station_ids = HashSet::new();
        let mut provider_ids = HashSet::new();
        for station in &meta.stations {
            validate_station(station)?;
            if !station_ids.insert(station.station_id.clone())
                || !provider_ids
                    .insert((station.provider_code.clone(), station.station_code.clone()))
            {
                return Err(error("duplicate station identity"));
            }
        }
        let mut series = BTreeMap::new();
        for item in &meta.series {
            validate_series(item)?;
            if !station_ids.contains(&item.station_id)
                || series
                    .insert(item.series_id.clone(), item.variable.clone())
                    .is_some()
            {
                return Err(error("invalid series identity"));
            }
        }
        let mut replacement_ids = HashSet::new();
        for item in &meta.replacements {
            if !replacement_ids.insert(&item.replacement_id) {
                return Err(error("duplicate replacement"));
            }
            validate_replacement(item)?;
            local_time(zone, item.start_at, &item.replacement_id)?;
            if let Some(end) = item.end_at {
                local_time(zone, end, &item.replacement_id)?;
            }
            if !station_ids.contains(&item.station_id) {
                return Err(error("unknown replacement station"));
            }
        }
        let mut offset_ids = HashSet::new();
        for item in &meta.offsets {
            if !offset_ids.insert(&item.station_id) {
                return Err(error("duplicate offset"));
            }
            validate_offset(item)?;
            if !station_ids.contains(&item.station_id) {
                return Err(error("unknown offset station"));
            }
        }
        validate_level_references(&meta.references)?;
        if meta
            .references
            .iter()
            .any(|v| !station_ids.contains(&v.station_id))
            || meta
                .variables
                .iter()
                .any(|v| !station_ids.contains(&v.station_id))
            || meta
                .coverage
                .iter()
                .any(|v| !station_ids.contains(&v.station_id))
        {
            return Err(error("unknown metadata station"));
        }
        let mut variables_seen = HashSet::new();
        for item in &meta.variables {
            if !matches!(item.variable.as_str(), "rain" | "level" | "flow")
                || !variables_seen.insert((&item.station_id, &item.variable))
            {
                return Err(error("invalid inventory variable"));
            }
        }
        let mut coverage_seen = HashSet::new();
        for item in &meta.coverage {
            let vars: Vec<String> = item.variables_key.split(',').map(str::to_owned).collect();
            if coverage_key(&vars)? != item.variables_key
                || !coverage_seen.insert((&item.station_id, &item.variables_key, item.day))
            {
                return Err(error("invalid coverage"));
            }
        }
        let mut flag_keys = HashSet::new();
        for flag in &meta.flags {
            if !matches!(flag.reason.as_str(), "threshold" | "sequence")
                || series.get(&flag.series_id).map(String::as_str) != Some("rain")
                || !flag_keys.insert((
                    flag.series_id.clone(),
                    flag.observed_at,
                    flag.reason.clone(),
                ))
            {
                return Err(error("invalid precipitation flag"));
            }
        }
        let mut flag_samples: HashSet<_> = meta
            .flags
            .iter()
            .map(|f| (f.series_id.clone(), f.observed_at))
            .collect();
        let mut samples = Vec::new();
        for item in interchange_rows::<InterchangeSample>(root, &manifest, "samples")? {
            let value = match (item.rain, item.flow, item.level) {
                (None, None, None) => None,
                (Some(v), None, None) => Some(ObservationValue::RainMm(v)),
                (None, Some(v), None) => Some(ObservationValue::FlowM3s(v)),
                (None, None, Some(v)) => Some(ObservationValue::LevelCm(v)),
                _ => return Err(error("invalid sample value columns")),
            };
            let sample = ObservationSample {
                series_id: item.series_id,
                observed_at: item.observed_at,
                value,
            };
            let variable = series
                .get(&sample.series_id)
                .ok_or_else(|| error("unknown sample series"))?;
            validate_sample(&sample, variable)?;
            // Legacy local labels may fall in historical DST overlaps. Preserve
            // the source label without assigning it an inferred UTC instant.
            if !flag_samples.is_empty() {
                flag_samples.remove(&(sample.series_id.clone(), sample.observed_at));
            }
            samples.push(sample);
        }
        if !flag_samples.is_empty() {
            return Err(error("flag references unknown sample"));
        }
        let parent = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        let staging = tempfile::Builder::new()
            .prefix(".observation-import-")
            .tempdir_in(parent)
            .map_err(io_error)?;
        let staged_path = staging.path().join("observations.sqlite");
        let store = Self::create(&staged_path, &manifest.time_zone)?;
        store.replace_initial(&meta, &samples, 1, None)?;
        store.finish_export()?;
        drop(store);
        let destination = Self {
            path: path.into(),
            writable: true,
            selected_revision: None,
        };
        let guard = destination.lock()?;
        fs::hard_link(&staged_path, path).map_err(io_error)?;
        super::sync_directory(parent).map_err(io_error)?;
        drop(guard);
        Self::open_writable(path)
    }
    fn replace_initial(
        &self,
        meta: &Metadata,
        samples: &[ObservationSample],
        revision: i64,
        identity: Option<&str>,
    ) -> Result<(), ObservationError> {
        let _guard = self.lock()?;
        let m = self.manifest()?;
        let db = Db::open(&self.path, true)?;
        let payload = serde_json::to_string(meta).map_err(io_error)?;
        let key = format!("{:x}", Sha256::digest(payload.as_bytes()));
        pollster::block_on(async {
            db.conn
                .execute("BEGIN IMMEDIATE", ())
                .await
                .map_err(io_error)?;
            let result = async {
                if let Some(identity) = identity {
                    db.conn
                        .execute("UPDATE registry_identity SET id=?1", [identity])
                        .await
                        .map_err(io_error)?;
                }
                db.conn
                    .execute(
                        "INSERT OR IGNORE INTO observation_metadata VALUES (?1,?2)",
                        (key.clone(), payload),
                    )
                    .await
                    .map_err(io_error)?;
                db.conn
                    .execute("DELETE FROM observation_revision", ())
                    .await
                    .map_err(io_error)?;
                db.conn
                    .execute(
                        "INSERT INTO observation_revision VALUES (?1,?2,?3)",
                        (revision, m.time_zone.clone(), key),
                    )
                    .await
                    .map_err(io_error)?;
                let mut stmt = db
                    .conn
                    .prepare("INSERT INTO observation_sample VALUES (?1,?2,?3,?4,?5)")
                    .await
                    .map_err(io_error)?;
                for sample in samples {
                    let payload = serde_json::to_string(sample).map_err(io_error)?;
                    let checksum = format!("{:x}", Sha256::digest(payload.as_bytes()));
                    stmt.execute((
                        sample.series_id.clone(),
                        sample.observed_at.format("%Y-%m-%dT%H:%M:%S").to_string(),
                        revision,
                        payload,
                        checksum,
                    ))
                    .await
                    .map_err(io_error)?;
                }
                drop(stmt);
                db.conn.execute("COMMIT", ()).await.map_err(io_error)?;
                Ok::<_, ObservationError>(())
            }
            .await;
            if result.is_err() {
                let _ = db.conn.execute("ROLLBACK", ()).await;
            }
            result
        })
    }
    fn finish_export(&self) -> Result<(), ObservationError> {
        let guard = self.lock()?;
        {
            let db = Db::open(&self.path, true)?;
            pollster::block_on(async {
                let mut rows = db
                    .conn
                    .query("PRAGMA wal_checkpoint(TRUNCATE)", ())
                    .await
                    .map_err(io_error)?;
                let row = rows
                    .next()
                    .await
                    .map_err(io_error)?
                    .ok_or_else(|| error("checkpoint returned no status"))?;
                if row.get_value(0).map_err(io_error)? != turso::Value::Integer(0) {
                    return Err(error("checkpoint was busy"));
                }
                Ok::<_, ObservationError>(())
            })?;
        }
        File::open(&self.path)
            .and_then(|file| file.sync_all())
            .map_err(io_error)?;
        for suffix in ["-wal", "-shm"] {
            let mut name = self.path.as_os_str().to_os_string();
            name.push(suffix);
            match fs::remove_file(PathBuf::from(name)) {
                Ok(()) => (),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
                Err(e) => return Err(io_error(e)),
            }
        }
        if let Some(parent) = self.path.parent() {
            super::sync_directory(parent).map_err(io_error)?;
        }
        drop(guard);
        fs::remove_file(self.lock_path()).map_err(io_error)?;
        Ok(())
    }
    /// Export all values visible at a pinned revision into an independent database.
    pub fn export_revision(&self, destination: &Path) -> Result<(), ObservationError> {
        let revision = self
            .selected_revision
            .ok_or_else(|| error("export requires a pinned revision"))?;
        let guard = self.read_lock()?;
        let manifest = self.manifest()?;
        let metadata = self.metadata(&manifest)?;
        let mut samples = Vec::new();
        for series in &metadata.series {
            samples.extend(self.samples(&manifest, &series.series_id)?);
        }
        let time_zone = manifest.time_zone.clone();
        let identity = self.registry_id_locked()?;
        drop(manifest);
        drop(guard);
        let target = Self::create(destination, &time_zone)?;
        target.replace_initial(&metadata, &samples, revision, Some(&identity))?;
        target.finish_export()
    }
    /// Materialize a pinned revision's selected values for a portable input set.
    pub fn snapshot_revision(
        &self,
        destination: &Path,
        selection: &ObservationSelection,
        interval: &super::IntervalAxis,
        replacement_ids: &[String],
        extra_stations: &[String],
    ) -> Result<(), ObservationError> {
        let revision = self
            .selected_revision
            .ok_or_else(|| error("snapshot requires a pinned revision"))?;
        let (inputs, offsets) = self.read_selection_with_stations(
            selection,
            interval,
            replacement_ids,
            extra_stations,
        )?;
        let _guard = self.read_lock()?;
        let m = self.manifest()?;
        let original = self.metadata(&m)?;
        let identity = self.registry_id_locked()?;
        let station_ids: HashSet<_> = inputs
            .stations
            .iter()
            .map(|s| s.station_id.as_str())
            .collect();
        let series_ids: HashSet<_> = inputs.series.iter().map(|s| s.series_id.as_str()).collect();
        let metadata = Metadata {
            stations: inputs.stations.clone(),
            series: inputs.series.clone(),
            replacements: inputs.replacements.clone(),
            offsets,
            references: original
                .references
                .iter()
                .filter(|r| station_ids.contains(r.station_id.as_str()))
                .cloned()
                .collect(),
            variables: original
                .variables
                .iter()
                .filter(|r| station_ids.contains(r.station_id.as_str()))
                .cloned()
                .collect(),
            flags: original
                .flags
                .iter()
                .filter(|r| {
                    series_ids.contains(r.series_id.as_str())
                        && inputs
                            .samples
                            .iter()
                            .any(|s| s.series_id == r.series_id && s.observed_at == r.observed_at)
                })
                .cloned()
                .collect(),
            coverage: original
                .coverage
                .iter()
                .filter(|r| station_ids.contains(r.station_id.as_str()))
                .cloned()
                .collect(),
            partial_coverage: original.partial_coverage.clone(),
            provenance: original
                .provenance
                .iter()
                .filter(|source| station_ids.contains(source.station_id.as_str()))
                .cloned()
                .collect(),
        };
        drop(original);
        drop(m);
        drop(_guard);
        let destination_store = Self::create(destination, &interval.time_zone)?;
        destination_store.replace_initial(&metadata, &inputs.samples, revision, Some(&identity))?;
        destination_store.finish_export()
    }
}

#[cfg(test)]
mod sqlite_tests {
    use super::*;
    #[test]
    #[ignore = "regional legacy interchange import; requires runs/legacy-registry-export"]
    fn regional_interchange_import() {
        let root = Path::new("runs/legacy-registry-export");
        let dir = tempfile::Builder::new()
            .prefix("sqlite-import-")
            .tempdir_in("runs")
            .unwrap();
        let store =
            ObservationStore::import_interchange(&dir.path().join("observations.sqlite"), root)
                .unwrap();
        assert_eq!(store.station_locations().unwrap().len(), 440);
        assert_eq!(store.series_choices().unwrap().len(), 414);
        assert_eq!(store.replacement_choices().unwrap().len(), 6);
        let guard = store.read_lock().unwrap();
        let manifest = store.manifest().unwrap();
        let metadata = store.metadata(&manifest).unwrap();
        let mut count = 0;
        for series in &metadata.series {
            count += store.samples(&manifest, &series.series_id).unwrap().len();
        }
        assert_eq!(count, 2_001_960);
        drop(guard);
    }
    #[test]
    fn flag_revision_compare_and_swap_is_atomic() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("observations.sqlite");
        let mut store = ObservationStore::create(&path, "UTC").unwrap();
        let at = chrono::NaiveDate::from_ymd_opt(2026, 1, 1)
            .unwrap()
            .and_hms_opt(1, 0, 0)
            .unwrap();
        let station = Station {
            station_id: "s".into(),
            provider_code: "ana".into(),
            station_code: "1".into(),
            name: "old".into(),
            latitude: None,
            longitude: None,
        };
        let sample = ObservationSample {
            series_id: "rain".into(),
            observed_at: at,
            value: Some(ObservationValue::RainMm(1.0)),
        };
        store
            .write_batch(&ObservationBatch {
                stations: vec![station.clone()],
                series: vec![ObservationSeries {
                    series_id: "rain".into(),
                    station_id: "s".into(),
                    variable: "rain".into(),
                    state: "raw".into(),
                    unit: "mm".into(),
                    interval_seconds: Some(3600),
                    created_at: at,
                }],
                samples: vec![sample.clone()],
                ..Default::default()
            })
            .unwrap();
        let old_revision = store.revision().unwrap();
        let mut changed = station;
        changed.name = "new".into();
        store.update_station_inventory(&[changed]).unwrap();
        let current = store.revision().unwrap();
        let window = super::super::IntervalAxis {
            time_zone: "UTC".into(),
            bounds: vec![super::super::TimeInterval {
                start: at - chrono::TimeDelta::hours(1),
                end: at,
            }],
        };
        let flags = vec![("rain".into(), at, "threshold".into())];
        assert!(
            store
                .replace_precipitation_flags_for_revision(
                    old_revision,
                    &["rain".into()],
                    &window,
                    &flags,
                    std::slice::from_ref(&sample)
                )
                .is_err()
        );
        assert_eq!(store.revision().unwrap(), current);
        let (excluded, revised) = store
            .replace_precipitation_flags_for_revision(
                current,
                &["rain".into()],
                &window,
                &flags,
                std::slice::from_ref(&sample),
            )
            .unwrap();
        assert_eq!(excluded, BTreeSet::from([("rain".into(), at)]));
        assert_eq!(store.revision().unwrap(), revised);
        assert!(revised > current);
        let prior = ObservationStore::open_revision(&path, current).unwrap();
        assert!(
            prior
                .read_precipitation_flags(&["rain".into()], &window)
                .unwrap()
                .is_empty()
        );
        let (unchanged, same) = store
            .replace_precipitation_flags_for_revision(
                revised,
                &["rain".into()],
                &window,
                &flags,
                &[sample],
            )
            .unwrap();
        assert_eq!(unchanged, excluded);
        assert_eq!(same, revised);
    }
    #[test]
    fn revisions_keep_prior_values_and_skip_identical_ingest() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("observations.sqlite");
        let mut store = ObservationStore::create(&path, "UTC").unwrap();
        let identity = store.registry_id().unwrap();
        let unrelated =
            ObservationStore::create(&dir.path().join("unrelated.sqlite"), "UTC").unwrap();
        assert_ne!(identity, unrelated.registry_id().unwrap());
        let at = chrono::NaiveDate::from_ymd_opt(2026, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        let station = Station {
            station_id: "ana:1".into(),
            provider_code: "ana".into(),
            station_code: "1".into(),
            name: "old".into(),
            latitude: None,
            longitude: None,
        };
        let sample = ObservationSample {
            series_id: "rain".into(),
            observed_at: at,
            value: Some(ObservationValue::RainMm(0.0)),
        };
        let batch = ObservationBatch {
            stations: vec![station.clone()],
            series: vec![ObservationSeries {
                series_id: "rain".into(),
                station_id: station.station_id.clone(),
                variable: "rain".into(),
                state: "raw".into(),
                unit: "mm".into(),
                interval_seconds: Some(3600),
                created_at: at,
            }],
            samples: vec![sample.clone()],
            ..Default::default()
        };
        store.ingest_batch(&batch).unwrap();
        let original = store.revision().unwrap();
        store.ingest_batch(&batch).unwrap();
        assert_eq!(store.revision().unwrap(), original);
        let mut changed = station;
        changed.name = "new".into();
        store.update_station_inventory(&[changed]).unwrap();
        store
            .upsert_ana_raw_with_coverage(
                "ana:1",
                at,
                &[("rain", vec![(at, ObservationValue::RainMm(7.0))])],
                &["rain".into()],
                &[at.date()],
                Some((at.date(), 42)),
                &[ObservationProvenance {
                    provider: "ana".into(),
                    station_id: "ana:1".into(),
                    variables: vec!["rain".into()],
                    request_start: Some(at.date()),
                    request_end: Some(at.date()),
                    reference_time: "2026-01-01T00:00:00Z".into(),
                    source_kind: "ana_xml".into(),
                    source_path: "downloads/ana/a/source.xml".into(),
                    sha256: "a".repeat(64),
                    byte_length: 42,
                }],
            )
            .unwrap();
        assert_eq!(
            store.samples(&store.manifest().unwrap(), "rain").unwrap()[0].value,
            Some(ObservationValue::RainMm(7.0))
        );
        assert!(
            store
                .ana_coverage("ana:1", &["rain".into()])
                .unwrap()
                .contains(&at.date())
        );
        assert!(
            store
                .ana_partial_coverage()
                .unwrap()
                .covers("ana:1", &["rain".into()], at.date(), 42)
                .unwrap()
        );
        assert_eq!(store.acquisition_provenance().unwrap().len(), 1);
        let old = ObservationStore::open_revision(&path, original).unwrap();
        assert!(old.acquisition_provenance().unwrap().is_empty());
        assert!(
            old.ana_coverage("ana:1", &["rain".into()])
                .unwrap()
                .is_empty()
        );
        let old_partial = old
            .ana_partial_coverage()
            .unwrap()
            .covers("ana:1", &["rain".into()], at.date(), 42)
            .unwrap();
        assert_eq!(old_partial, bool::default());
        assert_eq!(old.station_locations().unwrap()[0].station.name, "old");
        assert_eq!(store.station_locations().unwrap()[0].station.name, "new");
        let expected_samples = vec![sample];
        assert_eq!(
            old.samples(&old.manifest().unwrap(), "rain").unwrap(),
            expected_samples
        );
        let copy = dir.path().join("copy.sqlite");
        old.export_revision(&copy).unwrap();
        assert_eq!(
            ObservationStore::open_revision(&copy, original)
                .unwrap()
                .registry_id()
                .unwrap(),
            identity
        );
        assert_eq!(
            ObservationStore::open_revision(&copy, original)
                .unwrap()
                .station_locations()
                .unwrap()[0]
                .station
                .name,
            "old"
        );
    }
}
