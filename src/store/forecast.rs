use super::{
    ForecastCorrection, ForecastSource, GridSeries, StoreError, read_grid_series, write_grid_series,
};
use chrono::{DateTime, SecondsFormat, Timelike, Utc};
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::HashSet,
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Component, Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT_STAGE: AtomicU64 = AtomicU64::new(0);

pub(super) const SCHEMA: &str = "CREATE TABLE asset (asset_id TEXT PRIMARY KEY NOT NULL,source_id TEXT NOT NULL,provider TEXT NOT NULL,model TEXT NOT NULL,product TEXT NOT NULL,cycle_utc TEXT NOT NULL,source_path TEXT NOT NULL,source_sha256 TEXT NOT NULL,source_byte_length INTEGER NOT NULL CHECK(source_byte_length>=0),native_path TEXT UNIQUE NOT NULL,native_sha256 TEXT NOT NULL,native_byte_length INTEGER NOT NULL CHECK(native_byte_length>=0),source_format TEXT NOT NULL CHECK(source_format IN ('grib2','netcdf')),normalization_json TEXT NOT NULL,operation_timezone TEXT NOT NULL,valid_from TEXT NOT NULL,valid_to TEXT NOT NULL,CHECK(valid_from<valid_to));";
pub(super) const CORRECTION_SCHEMA: &str = "CREATE TABLE correction (correction_id TEXT PRIMARY KEY NOT NULL,asset_id TEXT NOT NULL REFERENCES asset(asset_id),t0_hour INTEGER NOT NULL CHECK(t0_hour>=0),t1_hour INTEGER NOT NULL CHECK(t1_hour>=t0_hour),shift_lat_pixels REAL NOT NULL,shift_lon_pixels REAL NOT NULL,rotation_deg REAL NOT NULL,multiplication_factor REAL NOT NULL CHECK(multiplication_factor>0),editor TEXT,reason TEXT NOT NULL,created_at TEXT NOT NULL);";
#[derive(Debug, Clone)]
pub struct ForecastAssetChoice {
    pub asset_id: String,
    pub provider: String,
    pub model: String,
    pub product: String,
    pub cycle_utc: String,
}
#[derive(Debug, Clone)]
pub struct ForecastAsset {
    pub asset_id: String,
    pub source_id: String,
    pub retained_source: PathBuf,
    pub native_artifact: PathBuf,
}
impl ForecastAsset {
    pub fn snapshot_source_path(&self) -> PathBuf {
        let extension = if self
            .retained_source
            .extension()
            .is_some_and(|ext| ext == "nc")
        {
            "nc"
        } else {
            "grib2"
        };
        PathBuf::from(format!("sources/original/{}.{extension}", self.asset_id))
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanonicalForecastMetadata {
    pub provider: String,
    pub model: String,
    pub product: String,
    pub cycle_utc: DateTime<Utc>,
}
impl CanonicalForecastMetadata {
    fn cycle(&self) -> String {
        self.cycle_utc.to_rfc3339_opts(SecondsFormat::Secs, true)
    }

    fn validate(&self, native: &GridSeries) -> Result<(), StoreError> {
        if [&self.provider, &self.model, &self.product]
            .iter()
            .any(|text| text.trim().is_empty())
            || self.cycle_utc.nanosecond() != 0
        {
            return Err(invalid(
                "forecast metadata requires nonempty provider, model, product and a whole-second UTC cycle",
            ));
        }
        if native
            .values
            .iter()
            .zip(&native.valid)
            .any(|(value, valid)| *valid && *value < 0.0)
        {
            return Err(invalid("negative forecast rainfall"));
        }
        if native
            .timeline
            .bounds
            .iter()
            .any(|interval| (interval.end - interval.start).num_seconds() % 3600 != 0)
        {
            return Err(invalid(
                "canonical forecast intervals must span whole hours",
            ));
        }
        Ok(())
    }
}
fn canonical_source_id(hash: &str, settings: &str) -> String {
    let mut identity = Sha256::new();
    identity.update(b"canonical-forecast:");
    identity.update(hash.as_bytes());
    identity.update(settings.as_bytes());
    format!("{:x}", identity.finalize())
}
fn forecast_asset_id(source_id: &str) -> String {
    let key = format!("forecast-asset:{source_id}");
    format!("{:x}", Sha256::digest(key.as_bytes()))
}
pub struct ForecastSnapshotSelection {
    pub asset_id: String,
    pub correction_ids: Vec<String>,
    pub corrections: Option<Vec<ForecastCorrection>>,
}
pub struct ForecastCatalog {
    workspace: PathBuf,
    root: PathBuf,
    connection: Connection,
}
fn invalid(s: impl Into<String>) -> StoreError {
    StoreError::InvalidValue(s.into())
}
fn relative(p: &Path) -> Result<(), StoreError> {
    if p.as_os_str().is_empty() || p.components().any(|c| !matches!(c, Component::Normal(_))) {
        Err(StoreError::InvalidPath(format!(
            "invalid workspace-relative path {}",
            p.display()
        )))
    } else {
        Ok(())
    }
}
fn resolve(workspace: &Path, p: &Path) -> Result<PathBuf, StoreError> {
    relative(p)?;
    let joined = workspace.join(p);
    let canonical = joined.canonicalize()?;
    if !canonical.starts_with(workspace) {
        return Err(StoreError::InvalidPath("path escapes workspace".into()));
    }
    Ok(canonical)
}
pub(crate) fn create_workspace_dir(
    workspace: &Path,
    relative_path: &Path,
) -> Result<(), StoreError> {
    relative(relative_path)?;
    let mut path = PathBuf::new();
    for component in relative_path.components() {
        path.push(component.as_os_str());
        let full = workspace.join(&path);
        if let Err(err) = fs::create_dir(&full)
            && err.kind() != std::io::ErrorKind::AlreadyExists
        {
            return Err(err.into());
        }
        let verified = resolve(workspace, &path)?;
        if !verified.is_dir() {
            return Err(StoreError::InvalidPath(
                "artifact component is not a directory".into(),
            ));
        }
    }
    Ok(())
}
fn text(p: &Path) -> Result<&str, StoreError> {
    p.to_str()
        .ok_or_else(|| StoreError::InvalidPath("path is not UTF-8".into()))
}
fn digest(path: &Path) -> Result<(String, u64), StoreError> {
    if !fs::metadata(path)?.is_file() {
        return Err(invalid("forecast artifact must be a regular file"));
    }
    let mut file = File::open(path)?;
    let mut hash = Sha256::new();
    let count = std::io::copy(&mut file, &mut hash)?;
    Ok((format!("{:x}", hash.finalize()), count))
}
fn copy_source(source: &Path, dest: &Path) -> Result<(), StoreError> {
    let mut input = File::open(source)?;
    let mut out = OpenOptions::new().write(true).create_new(true).open(dest)?;
    std::io::copy(&mut input, &mut out)?;
    out.flush()?;
    out.sync_all()?;
    Ok(())
}
fn check_source(source: &ForecastSource, native: &GridSeries) -> Result<(), StoreError> {
    native.validate().map_err(|e| invalid(e.to_string()))?;
    if native.provenance.source_ids.as_slice() != [source.source_id.as_str()] {
        return Err(StoreError::Integrity(
            "native source identity mismatch".into(),
        ));
    }
    if source.source_id.len() != 64
        || !source
            .source_id
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
    {
        return Err(invalid("invalid source identity"));
    }
    let settings_bytes =
        serde_json::to_vec(&source.normalization_json).map_err(|e| invalid(e.to_string()))?;
    let mut identity = Sha256::new();
    identity.update(source.sha256.as_bytes());
    identity.update(settings_bytes);
    if source.source_id != format!("{:x}", identity.finalize())
        || native.provenance.processing.get("normalization") != Some(&source.normalization_json)
    {
        return Err(StoreError::Integrity(
            "source identity/settings mismatch".into(),
        ));
    }
    if native
        .values
        .iter()
        .zip(&native.valid)
        .any(|(value, valid)| *valid && *value < 0.0)
    {
        return Err(invalid("negative forecast rainfall"));
    }
    let settings = &source.normalization_json;
    let provider = match source.provider {
        super::ForecastProvider::Ecmwf => "ecmwf",
        super::ForecastProvider::Noaa => "noaa",
    };
    let cycle = source
        .cycle_utc
        .to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let bbox: [f64; 4] = serde_json::from_value(
        settings
            .get("crop_bbox")
            .cloned()
            .ok_or_else(|| invalid("missing crop bbox"))?,
    )
    .map_err(|e| invalid(e.to_string()))?;
    if settings.get("provider").and_then(serde_json::Value::as_str) != Some(provider)
        || settings.get("model").and_then(serde_json::Value::as_str) != Some(source.model.as_str())
        || settings.get("product").and_then(serde_json::Value::as_str)
            != Some(source.product.as_str())
        || settings
            .get("cycle_utc")
            .and_then(serde_json::Value::as_str)
            != Some(cycle.as_str())
        || settings
            .get("operation_timezone")
            .and_then(serde_json::Value::as_str)
            != Some(native.timeline.time_zone.as_str())
        || bbox != native.grid.requested_bbox
        || !matches!(
            (provider, source.model.as_str(), source.product.as_str()),
            ("ecmwf", "ifs", "fc") | ("noaa", "gfs", "fc")
        )
    {
        return Err(StoreError::Integrity(
            "normalization metadata mismatch".into(),
        ));
    }
    Ok(())
}
fn bounds(native: &GridSeries) -> (String, String) {
    let first = native.timeline.bounds.first().unwrap().start;
    let last = native.timeline.bounds.last().unwrap().end;
    (
        first.format("%Y-%m-%dT%H:%M:%S").to_string(),
        last.format("%Y-%m-%dT%H:%M:%S").to_string(),
    )
}
pub fn check_correction(c: &ForecastCorrection) -> Result<(), StoreError> {
    if c.correction_id.is_empty()
        || c.asset_id.is_empty()
        || chrono::DateTime::parse_from_rfc3339(&c.created_at).is_err()
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
        return Err(invalid("invalid correction"));
    }
    Ok(())
}
impl ForecastCatalog {
    pub fn asset_choices(&self) -> Result<Vec<ForecastAssetChoice>, StoreError> {
        let mut statement = self.connection.prepare("SELECT asset_id,provider,model,product,cycle_utc FROM asset ORDER BY cycle_utc DESC,asset_id")?;
        let rows = statement.query_map([], |row| {
            Ok(ForecastAssetChoice {
                asset_id: row.get(0)?,
                provider: row.get(1)?,
                model: row.get(2)?,
                product: row.get(3)?,
                cycle_utc: row.get(4)?,
            })
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    }
    pub(crate) fn covering_assets(
        &self,
        cycle_at_or_before: DateTime<chrono::Utc>,
        start: chrono::NaiveDateTime,
        end: chrono::NaiveDateTime,
        zone: &str,
    ) -> Result<Vec<ForecastAssetChoice>, StoreError> {
        let mut statement = self.connection.prepare(
            "SELECT asset_id,provider,model,product,cycle_utc FROM asset
             WHERE cycle_utc<=?1 AND valid_from<=?2 AND valid_to>=?3 AND operation_timezone=?4
             ORDER BY provider,model,product,cycle_utc DESC,asset_id",
        )?;
        let rows = statement.query_map(
            params![
                cycle_at_or_before.to_rfc3339_opts(SecondsFormat::Secs, true),
                start.format("%Y-%m-%dT%H:%M:%S").to_string(),
                end.format("%Y-%m-%dT%H:%M:%S").to_string(),
                zone
            ],
            |row| {
                Ok(ForecastAssetChoice {
                    asset_id: row.get(0)?,
                    provider: row.get(1)?,
                    model: row.get(2)?,
                    product: row.get(3)?,
                    cycle_utc: row.get(4)?,
                })
            },
        )?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    pub fn correction_choices(&self) -> Result<Vec<(String, String)>, StoreError> {
        let mut statement = self
            .connection
            .prepare("SELECT correction_id,asset_id FROM correction ORDER BY rowid")?;
        let rows = statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?;
        Ok(rows.collect::<Result<_, _>>()?)
    }
    pub fn open_read_only(workspace: &Path, catalog: &Path) -> Result<Self, StoreError> {
        Self::open_existing(workspace, catalog, false)
    }
    pub fn open_writable(workspace: &Path, catalog: &Path) -> Result<Self, StoreError> {
        Self::open_existing(workspace, catalog, true)
    }
    fn open_existing(workspace: &Path, catalog: &Path, writable: bool) -> Result<Self, StoreError> {
        let workspace = workspace.canonicalize()?;
        let path = if catalog.is_absolute() {
            catalog.to_path_buf()
        } else {
            workspace.join(catalog)
        };
        let connection = Connection::open_with_flags(
            path,
            if writable {
                rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE
            } else {
                rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY
            },
        )?;
        connection.pragma_update(None, "foreign_keys", true)?;
        connection.prepare("SELECT source_format, native_path FROM asset LIMIT 0")?;
        Ok(Self {
            workspace,
            root: PathBuf::new(),
            connection,
        })
    }
    pub(crate) fn check_native_available(&self, asset_id: &str) -> Result<(), StoreError> {
        let (native, checksum, length): (String, String, i64) = self.connection.query_row(
            "SELECT native_path,native_sha256,native_byte_length FROM asset WHERE asset_id=?1",
            [asset_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )?;
        let path = resolve(&self.workspace, Path::new(&native))?;
        let length =
            u64::try_from(length).map_err(|_| invalid("invalid native forecast length"))?;
        let metadata = fs::metadata(&path)?;
        if !metadata.is_file()
            || length == 0
            || metadata.len() != length
            || digest(&path)? != (checksum, length)
        {
            return Err(StoreError::Integrity(format!(
                "unavailable or corrupt forecast file {}",
                path.display()
            )));
        }
        Ok(())
    }
    pub fn open(
        workspace: &Path,
        catalog: &Path,
        artifact_root: &Path,
    ) -> Result<Self, StoreError> {
        let workspace = workspace.canonicalize()?;
        let cat = if catalog.is_absolute() {
            catalog.to_path_buf()
        } else {
            workspace.join(catalog)
        };
        let parent = cat
            .parent()
            .ok_or_else(|| StoreError::InvalidPath("catalog must have parent".into()))?;
        fs::create_dir_all(parent)?;
        let root = if artifact_root.is_absolute() {
            artifact_root.strip_prefix(&workspace).map_err(|_| {
                StoreError::InvalidPath("retained artifacts must be inside workspace".into())
            })?
        } else {
            artifact_root
        };
        relative(root)?;
        create_workspace_dir(&workspace, root)?;
        let mut connection = Connection::open(cat)?;
        connection.pragma_update(None, "foreign_keys", true)?;
        let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let count: i64 = tx.query_row(
            "SELECT count(*) FROM sqlite_schema WHERE type='table' AND name NOT LIKE 'sqlite_%'",
            [],
            |row| row.get(0),
        )?;
        if count == 0 {
            tx.execute_batch(SCHEMA)?;
            tx.execute_batch(CORRECTION_SCHEMA)?;
        } else {
            tx.prepare("SELECT source_format, native_path FROM asset LIMIT 0")?;
        }
        tx.commit()?;
        Ok(Self {
            workspace,
            root: root.to_path_buf(),
            connection,
        })
    }
    /// Register a canonical NetCDF without re-encoding its retained source or native grid.
    pub fn register_canonical(
        &mut self,
        source: &Path,
        metadata: &CanonicalForecastMetadata,
    ) -> Result<ForecastAsset, StoreError> {
        if self.root.as_os_str().is_empty() {
            return Err(invalid("canonical registration requires an artifact root"));
        }
        let (sha, length) = digest(source)?;
        let length_i64 =
            i64::try_from(length).map_err(|_| invalid("canonical forecast too large"))?;
        let settings = serde_json::to_string(metadata).map_err(|e| invalid(e.to_string()))?;
        let source_id = canonical_source_id(&sha, &settings);
        let asset_id = forecast_asset_id(&source_id);
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let exists: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM asset WHERE asset_id=?1)",
            [&asset_id],
            |r| r.get(0),
        )?;
        if exists {
            tx.commit()?;
            return self.load_native(&asset_id).map(|(asset, _)| asset);
        }
        let rel_dir = self.root.join(&asset_id);
        let dir = self.workspace.join(&rel_dir);
        let retained_source = rel_dir.join("source.nc");
        let native_artifact = rel_dir.join("native.nc");
        let root = resolve(&self.workspace, &self.root)?;
        let staged_series = if !dir.exists() {
            let stage = root.join(format!(
                ".canonical-{}-{}",
                std::process::id(),
                NEXT_STAGE.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&stage)?;
            let result = (|| {
                let retained = stage.join("source.nc");
                copy_source(source, &retained)?;
                if digest(&retained)? != (sha.clone(), length) {
                    return Err(StoreError::Integrity(
                        "canonical forecast source changed".into(),
                    ));
                }
                let series = read_grid_series(&retained)?;
                metadata.validate(&series)?;
                copy_source(&retained, &stage.join("native.nc"))?;
                super::sync_directory(&stage).map_err(std::io::Error::other)?;
                match fs::rename(&stage, &dir) {
                    Ok(()) => Ok(series),
                    Err(_) if dir.is_dir() => Ok(series),
                    Err(err) => Err(StoreError::Io(err)),
                }
            })();
            if stage.is_dir() {
                let _ = fs::remove_dir_all(&stage);
            }
            Some(result?)
        } else {
            None
        };
        let retained = resolve(&self.workspace, &retained_source)?;
        let native = resolve(&self.workspace, &native_artifact)?;
        for file in [&retained, &native] {
            if digest(file)? != (sha.clone(), length) {
                return Err(StoreError::Integrity(
                    "canonical forecast retained artifact differs".into(),
                ));
            }
        }
        let series = match staged_series {
            Some(series) => series,
            None => {
                let series = read_grid_series(&native)?;
                metadata.validate(&series)?;
                series
            }
        };
        let (first, last) = bounds(&series);
        super::sync_directory(&root).map_err(std::io::Error::other)?;
        tx.execute(
            "INSERT INTO asset VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)",
            params![
                asset_id,
                source_id,
                metadata.provider,
                metadata.model,
                metadata.product,
                metadata.cycle(),
                text(&retained_source)?,
                sha,
                length_i64,
                text(&native_artifact)?,
                sha,
                length_i64,
                "netcdf",
                settings,
                series.timeline.time_zone,
                first,
                last,
            ],
        )?;
        tx.commit()?;
        Ok(ForecastAsset {
            asset_id,
            source_id,
            retained_source,
            native_artifact,
        })
    }

    pub fn register_native(
        &mut self,
        source_path: &Path,
        source: &ForecastSource,
        native: &GridSeries,
    ) -> Result<ForecastAsset, StoreError> {
        check_source(source, native)?;
        if digest(source_path)? != (source.sha256.clone(), source.byte_length) {
            return Err(StoreError::Integrity(
                "source changed during normalization".into(),
            ));
        }
        if self.root.as_os_str().is_empty() {
            return Err(invalid("native registration requires an artifact root"));
        }
        let asset_id = forecast_asset_id(&source.source_id);
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let exists: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM asset WHERE asset_id=?1)",
            [&asset_id],
            |row| row.get(0),
        )?;
        if exists {
            tx.commit()?;
            return self.load_native(&asset_id).map(|(asset, _)| asset);
        }
        let root = resolve(&self.workspace, &self.root)?;
        let (retained_source, native_artifact, created) = if self.root == Path::new("assets") {
            let provider = match source.provider {
                super::ForecastProvider::Ecmwf => "ecmwf",
                super::ForecastProvider::Noaa => "noaa",
            };
            let stem = format!(
                "{}_{}_{}",
                source.model,
                source.product,
                source.cycle_utc.format("%Y%m%dT%H%MZ")
            );
            let source_dir = Path::new("downloads").join(provider);
            let native_dir = self.root.join(provider);
            create_workspace_dir(&self.workspace, &source_dir)?;
            create_workspace_dir(&self.workspace, &native_dir)?;
            let retained = source_dir.join(format!("{stem}.grib2"));
            let native_path = native_dir.join(format!("{stem}-{}.nc", &asset_id[..12]));
            let original = self.workspace.join(&retained);
            if !original.exists() {
                let staged = tempfile::Builder::new()
                    .prefix(".download-")
                    .tempdir_in(self.workspace.join(&source_dir))?;
                let raw = staged.path().join("source.grib2");
                copy_source(source_path, &raw)?;
                if digest(&raw)? != (source.sha256.clone(), source.byte_length) {
                    return Err(StoreError::Integrity(
                        "retained source hash mismatch".into(),
                    ));
                }
                fs::rename(raw, original)?;
                super::sync_directory(&self.workspace.join(&source_dir))
                    .map_err(std::io::Error::other)?;
            }
            let original = resolve(&self.workspace, &retained)?;
            if digest(&original)? != (source.sha256.clone(), source.byte_length) {
                return Err(StoreError::Integrity(
                    "download for the same forecast cycle has different bytes".into(),
                ));
            }
            let target = self.workspace.join(&native_path);
            let created = !target.exists();
            if created {
                let staged = tempfile::Builder::new()
                    .prefix(".native-")
                    .tempdir_in(self.workspace.join(&native_dir))?;
                let normalized = staged.path().join("native.nc");
                write_grid_series(&normalized, native)?;
                File::open(&normalized)?.sync_all()?;
                fs::rename(normalized, target)?;
                super::sync_directory(&self.workspace.join(&native_dir))
                    .map_err(std::io::Error::other)?;
            }
            (retained, native_path, created)
        } else {
            let rel_dir = self.root.join(&asset_id);
            let dir = self.workspace.join(&rel_dir);
            let retained_source = rel_dir.join("source.grib2");
            let native_artifact = rel_dir.join("native.nc");
            let created = if !dir.exists() {
                let stage = tempfile::Builder::new()
                    .prefix(".native-")
                    .tempdir_in(&root)?;
                let staged_source = stage.path().join("source.grib2");
                copy_source(source_path, &staged_source)?;
                if digest(&staged_source)? != (source.sha256.clone(), source.byte_length) {
                    return Err(StoreError::Integrity(
                        "retained source hash mismatch".into(),
                    ));
                }
                let staged_native = stage.path().join("native.nc");
                write_grid_series(&staged_native, native)?;
                File::open(&staged_native)?.sync_all()?;
                super::sync_directory(stage.path()).map_err(std::io::Error::other)?;
                match fs::rename(stage.path(), &dir) {
                    Ok(()) => true,
                    Err(_) if dir.is_dir() => false,
                    Err(error) => return Err(error.into()),
                }
            } else {
                false
            };
            (retained_source, native_artifact, created)
        };
        let native_file = resolve(&self.workspace, &native_artifact)?;
        let source_file = resolve(&self.workspace, &retained_source)?;
        if digest(&source_file)? != (source.sha256.clone(), source.byte_length) {
            return Err(StoreError::Integrity(
                "retained source hash mismatch".into(),
            ));
        }
        if !created {
            let prior = read_grid_series(&native_file)?;
            if prior.timeline != native.timeline
                || prior.grid != native.grid
                || prior.provenance != native.provenance
                || prior.valid != native.valid
                || prior
                    .values
                    .iter()
                    .zip(&native.values)
                    .zip(&native.valid)
                    .any(|((old, next), valid)| *valid && old != next)
            {
                return Err(StoreError::Integrity("retained native grid differs".into()));
            }
        }
        super::sync_directory(&root).map_err(std::io::Error::other)?;
        let (native_sha, native_len) = digest(&native_file)?;
        let first = &native
            .timeline
            .bounds
            .first()
            .ok_or_else(|| invalid("empty axis"))?
            .start;
        let last = &native
            .timeline
            .bounds
            .last()
            .ok_or_else(|| invalid("empty axis"))?
            .end;
        let provider = match source.provider {
            super::ForecastProvider::Ecmwf => "ecmwf",
            super::ForecastProvider::Noaa => "noaa",
        };
        let settings = serde_json::to_string(&source.normalization_json)
            .map_err(|e| invalid(e.to_string()))?;
        let source_len =
            i64::try_from(source.byte_length).map_err(|_| invalid("source too large"))?;
        let native_len = i64::try_from(native_len).map_err(|_| invalid("native too large"))?;
        tx.execute(
            "INSERT INTO asset VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)",
            params![
                asset_id,
                source.source_id,
                provider,
                source.model,
                source.product,
                source
                    .cycle_utc
                    .to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
                text(&retained_source)?,
                source.sha256,
                source_len,
                text(&native_artifact)?,
                native_sha,
                native_len,
                "grib2",
                settings,
                native.timeline.time_zone,
                first.format("%Y-%m-%dT%H:%M:%S").to_string(),
                last.format("%Y-%m-%dT%H:%M:%S").to_string()
            ],
        )?;
        tx.commit()?;
        Ok(ForecastAsset {
            asset_id,
            source_id: source.source_id.clone(),
            retained_source,
            native_artifact,
        })
    }
    pub fn load_native(&self, asset_id: &str) -> Result<(ForecastAsset, GridSeries), StoreError> {
        self.read_native(asset_id, true)
    }
    fn read_native(
        &self,
        asset_id: &str,
        verify_original: bool,
    ) -> Result<(ForecastAsset, GridSeries), StoreError> {
        let row=self.connection.query_row("SELECT source_id,source_path,source_sha256,source_byte_length,native_path,native_sha256,native_byte_length,source_format,operation_timezone,valid_from,valid_to FROM asset WHERE asset_id=?",[asset_id],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?,r.get::<_,i64>(3)?,r.get::<_,String>(4)?,r.get::<_,String>(5)?,r.get::<_,i64>(6)?,r.get::<_,String>(7)?,r.get::<_,String>(8)?,r.get::<_,String>(9)?,r.get::<_,String>(10)?))).optional()?.ok_or_else(||invalid("unknown asset"))?;
        let (
            source_id,
            src,
            src_sha,
            src_len,
            native,
            native_sha,
            native_len,
            source_format,
            zone,
            start,
            end,
        ) = row;
        let retained_source = PathBuf::from(src);
        let native_artifact = PathBuf::from(native);
        let native_file = resolve(&self.workspace, &native_artifact)?;
        if !matches!(source_format.as_str(), "grib2" | "netcdf")
            || src_len < 0
            || native_len < 0
            || (verify_original
                && digest(&resolve(&self.workspace, &retained_source)?)?
                    != (src_sha.clone(), src_len as u64))
            || digest(&native_file)? != (native_sha.clone(), native_len as u64)
        {
            return Err(StoreError::Integrity(
                "asset content/schema mismatch".into(),
            ));
        }
        let series = read_grid_series(&native_file)?;
        let (provider,model,product,cycle,settings):(String,String,String,String,String)=self.connection.query_row("SELECT provider,model,product,cycle_utc,normalization_json FROM asset WHERE asset_id=?",[asset_id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?)))?;
        if source_format == "netcdf" {
            let metadata: CanonicalForecastMetadata = serde_json::from_str(&settings)
                .map_err(|e| StoreError::Integrity(e.to_string()))?;
            if metadata.provider != provider
                || metadata.model != model
                || metadata.product != product
                || metadata.cycle() != cycle
                || retained_source.extension().is_none_or(|ext| ext != "nc")
                || src_sha != native_sha
                || src_len != native_len
                || canonical_source_id(&src_sha, &settings) != source_id
            {
                return Err(StoreError::Integrity(
                    "canonical forecast metadata mismatch".into(),
                ));
            }
            metadata
                .validate(&series)
                .map_err(|e| StoreError::Integrity(e.to_string()))?;
        } else {
            let provider = match provider.as_str() {
                "ecmwf" => super::ForecastProvider::Ecmwf,
                "noaa" => super::ForecastProvider::Noaa,
                _ => return Err(StoreError::Integrity("unknown provider".into())),
            };
            let cycle_utc = DateTime::parse_from_rfc3339(&cycle)
                .map_err(|e| StoreError::Integrity(e.to_string()))?
                .to_utc();
            let normalization_json = serde_json::from_str(&settings)
                .map_err(|e| StoreError::Integrity(e.to_string()))?;
            let source = ForecastSource {
                source_id: source_id.clone(),
                provider,
                model,
                product,
                cycle_utc,
                sha256: src_sha,
                byte_length: src_len as u64,
                normalization_json,
            };
            check_source(&source, &series).map_err(|e| StoreError::Integrity(e.to_string()))?;
        }
        if asset_id != forecast_asset_id(&source_id) {
            return Err(StoreError::Integrity("asset identity mismatch".into()));
        }
        if series.timeline.time_zone != zone || bounds(&series) != (start, end) {
            return Err(StoreError::Integrity("asset metadata mismatch".into()));
        }
        Ok((
            ForecastAsset {
                asset_id: asset_id.into(),
                source_id,
                retained_source,
                native_artifact,
            },
            series,
        ))
    }
    pub(crate) fn load_for_run(
        &self,
        asset_id: &str,
    ) -> Result<(ForecastAsset, GridSeries), StoreError> {
        let (asset, mut native) = self.read_native(asset_id, false)?;
        self.selected_provenance(&asset, &mut native, false)?;
        Ok((asset, native))
    }

    pub fn save_correction(&self, c: &ForecastCorrection) -> Result<(), StoreError> {
        check_correction(c)?;
        self.connection.execute(
            "INSERT INTO correction VALUES (?,?,?,?,?,?,?,?,?,?,?)",
            params![
                c.correction_id,
                c.asset_id,
                c.t0_hour,
                c.t1_hour,
                c.shift_lat_pixels,
                c.shift_lon_pixels,
                c.rotation_deg,
                c.multiplication_factor,
                c.editor,
                c.reason,
                c.created_at
            ],
        )?;
        Ok(())
    }
    pub fn register_corrections(
        &self,
        corrections: &[ForecastCorrection],
    ) -> Result<(), StoreError> {
        self.write_corrections(corrections)
    }

    fn write_corrections(&self, corrections: &[ForecastCorrection]) -> Result<(), StoreError> {
        let tx = self.connection.unchecked_transaction()?;
        for c in corrections {
            check_correction(c)?;
            let existing: Option<bool> = tx.query_row(
                "SELECT asset_id IS ?2 AND t0_hour IS ?3 AND t1_hour IS ?4 AND shift_lat_pixels IS ?5 AND shift_lon_pixels IS ?6 AND rotation_deg IS ?7 AND multiplication_factor IS ?8 AND editor IS ?9 AND reason IS ?10 AND created_at IS ?11 FROM correction WHERE correction_id=?1",
                params![c.correction_id,c.asset_id,c.t0_hour,c.t1_hour,c.shift_lat_pixels,c.shift_lon_pixels,c.rotation_deg,c.multiplication_factor,c.editor,c.reason,c.created_at], |row| row.get(0)
            ).optional()?;
            if let Some(matches) = existing {
                if !matches {
                    return Err(invalid(format!(
                        "conflicting correction {}",
                        c.correction_id
                    )));
                }
                continue;
            }
            tx.execute(
                "INSERT INTO correction VALUES (?,?,?,?,?,?,?,?,?,?,?)",
                params![
                    c.correction_id,
                    c.asset_id,
                    c.t0_hour,
                    c.t1_hour,
                    c.shift_lat_pixels,
                    c.shift_lon_pixels,
                    c.rotation_deg,
                    c.multiplication_factor,
                    c.editor,
                    c.reason,
                    c.created_at
                ],
            )?;
        }
        tx.commit()?;
        Ok(())
    }
    pub fn load_selected(
        &self,
        asset_id: &str,
        ids: &[String],
    ) -> Result<(ForecastAsset, GridSeries, Vec<ForecastCorrection>), StoreError> {
        let tx = self.connection.unchecked_transaction()?;
        let (asset, mut native) = self.load_native(asset_id)?;
        let corrections = self.load_corrections(asset_id, ids)?;
        self.selected_provenance(&asset, &mut native, !ids.is_empty())?;
        tx.commit()?;
        Ok((asset, native, corrections))
    }

    fn selected_provenance(
        &self,
        asset: &ForecastAsset,
        native: &mut GridSeries,
        has_corrections: bool,
    ) -> Result<(), StoreError> {
        let processing = &mut native.provenance.processing;
        if asset
            .retained_source
            .extension()
            .is_some_and(|ext| ext == "nc")
        {
            let settings: String = self.connection.query_row(
                "SELECT normalization_json FROM asset WHERE asset_id=?1",
                [&asset.asset_id],
                |row| row.get(0),
            )?;
            let normalization: serde_json::Value = serde_json::from_str(&settings)
                .map_err(|e| StoreError::Integrity(e.to_string()))?;
            *processing = serde_json::Value::Object(
                [
                    ("parent".into(), std::mem::take(processing)),
                    ("normalization".into(), normalization),
                    ("asset_id".into(), asset.asset_id.clone().into()),
                ]
                .into_iter()
                .collect(),
            );
        } else if let Some(map) = processing.as_object_mut() {
            map.insert("asset_id".into(), asset.asset_id.clone().into());
        } else if has_corrections {
            return Err(invalid("native processing metadata is not an object"));
        }
        Ok(())
    }
    /// Materialize selected catalog rows while reading their native grids in one transaction.
    pub fn snapshot_selected(
        &self,
        destination: &Path,
        selections: &[(String, Vec<String>)],
    ) -> Result<Vec<(ForecastAsset, GridSeries, Vec<ForecastCorrection>)>, StoreError> {
        let selections: Vec<_> = selections
            .iter()
            .map(|(id, ids)| ForecastSnapshotSelection {
                asset_id: id.clone(),
                correction_ids: ids.clone(),
                corrections: None,
            })
            .collect();
        self.snapshot_selected_values(destination, &selections)
    }

    pub fn snapshot_selected_values(
        &self,
        destination: &Path,
        selections: &[ForecastSnapshotSelection],
    ) -> Result<Vec<(ForecastAsset, GridSeries, Vec<ForecastCorrection>)>, StoreError> {
        let tx = self.connection.unchecked_transaction()?;
        let mut destination_db = Connection::open(destination)?;
        destination_db.execute_batch(SCHEMA)?;
        destination_db.execute_batch(CORRECTION_SCHEMA)?;
        let destination_tx = destination_db.transaction()?;
        let mut selected = Vec::new();
        let mut asset_ids = HashSet::new();
        let mut correction_ids = std::collections::HashMap::new();
        for selection in selections {
            let id = &selection.asset_id;
            let ids = &selection.correction_ids;
            let supplied = &selection.corrections;
            let (asset, mut native) = self.load_native(id)?;
            let corrections = if let Some(values) = supplied {
                if values.iter().map(|c| &c.correction_id).ne(ids.iter()) {
                    return Err(invalid("correction IDs do not match supplied values"));
                }
                let mut seen = HashSet::new();
                for c in values {
                    check_correction(c)?;
                    if c.asset_id != *id || !seen.insert(&c.correction_id) {
                        return Err(invalid(
                            "duplicate correction ID or correction belongs to another asset",
                        ));
                    }
                }
                values.clone()
            } else {
                self.load_corrections(id, ids)?
            };
            self.selected_provenance(&asset, &mut native, !ids.is_empty())?;
            if asset_ids.insert(id.clone()) {
                let mut query = tx.prepare("SELECT * FROM asset WHERE asset_id=?1")?;
                let values = query.query_row([id], |row| {
                    (0..17)
                        .map(|index| row.get::<_, rusqlite::types::Value>(index))
                        .collect::<Result<Vec<_>, _>>()
                })?;
                destination_tx.execute("INSERT INTO asset VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17)", rusqlite::params_from_iter(values))?;
                destination_tx.execute(
                    "UPDATE asset SET source_path=?1,native_path=?2 WHERE asset_id=?3",
                    params![
                        text(&asset.snapshot_source_path())?,
                        format!("sources/native/{id}.nc"),
                        id
                    ],
                )?;
            }
            for correction in &corrections {
                if let Some(previous) = correction_ids.get(&correction.correction_id) {
                    if previous != correction {
                        return Err(invalid("conflicting correction ID"));
                    }
                } else {
                    correction_ids.insert(correction.correction_id.clone(), correction.clone());
                    destination_tx.execute(
                        "INSERT INTO correction VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
                        params![
                            correction.correction_id,
                            correction.asset_id,
                            correction.t0_hour,
                            correction.t1_hour,
                            correction.shift_lat_pixels,
                            correction.shift_lon_pixels,
                            correction.rotation_deg,
                            correction.multiplication_factor,
                            correction.editor,
                            correction.reason,
                            correction.created_at
                        ],
                    )?;
                }
            }
            selected.push((asset, native, corrections));
        }
        destination_tx.commit()?;
        tx.commit()?;
        Ok(selected)
    }
    pub fn load_corrections(
        &self,
        asset_id: &str,
        ids: &[String],
    ) -> Result<Vec<ForecastCorrection>, StoreError> {
        let exists: bool = self.connection.query_row(
            "SELECT EXISTS(SELECT 1 FROM asset WHERE asset_id=?)",
            [asset_id],
            |row| row.get(0),
        )?;
        if !exists {
            return Err(invalid("unknown asset"));
        }
        let mut seen = HashSet::new();
        let mut out = Vec::new();
        for id in ids {
            if !seen.insert(id) {
                return Err(invalid("duplicate correction ID"));
            }
            let c=self.connection.query_row("SELECT correction_id,asset_id,t0_hour,t1_hour,shift_lat_pixels,shift_lon_pixels,rotation_deg,multiplication_factor,editor,reason,created_at FROM correction WHERE correction_id=?",[id],|r|Ok(ForecastCorrection{correction_id:r.get(0)?,asset_id:r.get(1)?,t0_hour:r.get(2)?,t1_hour:r.get(3)?,shift_lat_pixels:r.get(4)?,shift_lon_pixels:r.get(5)?,rotation_deg:r.get(6)?,multiplication_factor:r.get(7)?,editor:r.get(8)?,reason:r.get(9)?,created_at:r.get(10)?})).optional()?.ok_or_else(||invalid("unknown correction ID"))?;
            check_correction(&c).map_err(|e| StoreError::Integrity(e.to_string()))?;
            if c.asset_id != asset_id {
                return Err(invalid("correction belongs to another asset"));
            }
            out.push(c);
        }
        Ok(out)
    }
}
