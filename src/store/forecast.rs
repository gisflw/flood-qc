use super::{
    ForecastCorrection, ForecastSource, GridSeries, StoreError, read_grid_series, write_grid_series,
};
use rusqlite::{Connection, OptionalExtension, params};
use sha2::{Digest, Sha256};
use std::{
    collections::HashSet,
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Component, Path, PathBuf},
};

const SCHEMA: &str = "CREATE TABLE asset (asset_id TEXT PRIMARY KEY NOT NULL,source_id TEXT NOT NULL,provider TEXT NOT NULL,model TEXT NOT NULL,product TEXT NOT NULL,cycle_utc TEXT NOT NULL,source_path TEXT UNIQUE NOT NULL,source_sha256 TEXT NOT NULL,source_byte_length INTEGER NOT NULL CHECK(source_byte_length>=0),native_path TEXT UNIQUE NOT NULL,native_sha256 TEXT NOT NULL,native_byte_length INTEGER NOT NULL CHECK(native_byte_length>=0),schema_version INTEGER NOT NULL,normalization_json TEXT NOT NULL,operation_timezone TEXT NOT NULL,valid_from TEXT NOT NULL,valid_to TEXT NOT NULL,CHECK(valid_from<valid_to));CREATE TABLE correction (correction_id TEXT PRIMARY KEY NOT NULL,asset_id TEXT NOT NULL REFERENCES asset(asset_id),t0_hour INTEGER NOT NULL CHECK(t0_hour>=0),t1_hour INTEGER NOT NULL CHECK(t1_hour>=t0_hour),shift_lat_pixels REAL NOT NULL,shift_lon_pixels REAL NOT NULL,rotation_deg REAL NOT NULL,multiplication_factor REAL NOT NULL CHECK(multiplication_factor>0),editor TEXT,reason TEXT NOT NULL,created_at TEXT NOT NULL);";
#[derive(Debug, Clone)]
pub struct ForecastAsset {
    pub asset_id: String,
    pub source_id: String,
    pub retained_source: PathBuf,
    pub native_artifact: PathBuf,
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
fn create_workspace_dir(workspace: &Path, relative_path: &Path) -> Result<(), StoreError> {
    relative(relative_path)?;
    let mut path = PathBuf::new();
    for component in relative_path.components() {
        path.push(component.as_os_str());
        let full = workspace.join(&path);
        if !full.exists() {
            fs::create_dir(&full)?;
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
fn check_correction(c: &ForecastCorrection) -> Result<(), StoreError> {
    if c.correction_id.is_empty()
        || c.asset_id.is_empty()
        || c.reason.trim().is_empty()
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
    pub fn open_read_only(workspace: &Path, catalog: &Path) -> Result<Self, StoreError> {
        let workspace = workspace.canonicalize()?;
        let path = if catalog.is_absolute() {
            catalog.to_path_buf()
        } else {
            workspace.join(catalog)
        };
        let connection =
            Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;
        let version: i64 = connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
        if version != 1 {
            return Err(StoreError::Schema(format!(
                "unsupported forecast catalog version {version}"
            )));
        }
        Ok(Self {
            workspace,
            root: PathBuf::new(),
            connection,
        })
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
        let version: i64 = connection.pragma_query_value(None, "user_version", |r| r.get(0))?;
        if version == 0 {
            let tx = connection.transaction()?;
            let tables:i64=tx.query_row("SELECT count(*) FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'",[],|r|r.get(0))?;
            if tables != 0 {
                return Err(StoreError::Schema(
                    "unversioned nonempty forecast catalog".into(),
                ));
            }
            tx.execute_batch(SCHEMA)?;
            tx.pragma_update(None, "user_version", 1)?;
            tx.commit()?;
        } else if version != 1 {
            return Err(StoreError::Schema(format!(
                "unsupported forecast catalog version {version}"
            )));
        }
        Ok(Self {
            workspace,
            root: root.to_path_buf(),
            connection,
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
        let asset_key = format!("forecast-asset:{}", source.source_id);
        let asset_id = format!("{:x}", Sha256::digest(asset_key.as_bytes()));
        let rel_dir = self.root.join(&asset_id);
        let dir = self.workspace.join(&rel_dir);
        fs::create_dir(&dir)?;
        resolve(&self.workspace, &rel_dir)?;
        let retained_source = rel_dir.join("source.grib2");
        let native_artifact = rel_dir.join("native.nc");
        let source_file = self.workspace.join(&retained_source);
        let native_file = self.workspace.join(&native_artifact);
        let source_partial = dir.join("source.partial.grib2");
        copy_source(source_path, &source_partial)?;
        if digest(&source_partial)? != (source.sha256.clone(), source.byte_length) {
            return Err(StoreError::Integrity(
                "retained source hash mismatch".into(),
            ));
        }
        fs::rename(&source_partial, &source_file)?;
        let temp = dir.join("native.partial.nc");
        write_grid_series(&temp, native)?;
        File::open(&temp)?.sync_all()?;
        fs::rename(&temp, &native_file)?;
        File::open(&dir)?.sync_all()?;
        File::open(self.workspace.join(&self.root))?.sync_all()?;
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
        self.connection.execute(
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
                1,
                settings,
                native.timeline.time_zone,
                first.format("%Y-%m-%dT%H:%M:%S").to_string(),
                last.format("%Y-%m-%dT%H:%M:%S").to_string()
            ],
        )?;
        Ok(ForecastAsset {
            asset_id,
            source_id: source.source_id.clone(),
            retained_source,
            native_artifact,
        })
    }
    pub fn load_native(&self, asset_id: &str) -> Result<(ForecastAsset, GridSeries), StoreError> {
        let row=self.connection.query_row("SELECT source_id,source_path,source_sha256,source_byte_length,native_path,native_sha256,native_byte_length,schema_version,operation_timezone,valid_from,valid_to FROM asset WHERE asset_id=?",[asset_id],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?,r.get::<_,i64>(3)?,r.get::<_,String>(4)?,r.get::<_,String>(5)?,r.get::<_,i64>(6)?,r.get::<_,i64>(7)?,r.get::<_,String>(8)?,r.get::<_,String>(9)?,r.get::<_,String>(10)?))).optional()?.ok_or_else(||invalid("unknown asset"))?;
        let (
            source_id,
            src,
            src_sha,
            src_len,
            native,
            native_sha,
            native_len,
            version,
            zone,
            start,
            end,
        ) = row;
        let retained_source = PathBuf::from(src);
        let native_artifact = PathBuf::from(native);
        let source_file = resolve(&self.workspace, &retained_source)?;
        let native_file = resolve(&self.workspace, &native_artifact)?;
        if version != 1
            || src_len < 0
            || native_len < 0
            || digest(&source_file)? != (src_sha.clone(), src_len as u64)
            || digest(&native_file)? != (native_sha, native_len as u64)
        {
            return Err(StoreError::Integrity(
                "asset content/schema mismatch".into(),
            ));
        }
        let series = read_grid_series(&native_file)?;
        let (provider,model,product,cycle,settings):(String,String,String,String,String)=self.connection.query_row("SELECT provider,model,product,cycle_utc,normalization_json FROM asset WHERE asset_id=?",[asset_id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?)))?;
        let provider = match provider.as_str() {
            "ecmwf" => super::ForecastProvider::Ecmwf,
            "noaa" => super::ForecastProvider::Noaa,
            _ => return Err(StoreError::Integrity("unknown provider".into())),
        };
        let cycle_utc = chrono::DateTime::parse_from_rfc3339(&cycle)
            .map_err(|e| StoreError::Integrity(e.to_string()))?
            .to_utc();
        let normalization_json =
            serde_json::from_str(&settings).map_err(|e| StoreError::Integrity(e.to_string()))?;
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
        let asset_key = format!("forecast-asset:{source_id}");
        if asset_id != format!("{:x}", Sha256::digest(asset_key.as_bytes())) {
            return Err(StoreError::Integrity("asset identity mismatch".into()));
        }
        let first = series
            .timeline
            .bounds
            .first()
            .ok_or_else(|| invalid("empty axis"))?;
        let last = series
            .timeline
            .bounds
            .last()
            .ok_or_else(|| invalid("empty axis"))?;
        if series.provenance.source_ids.as_slice() != [source_id.as_str()]
            || series.timeline.time_zone != zone
            || first.start.format("%Y-%m-%dT%H:%M:%S").to_string() != start
            || last.end.format("%Y-%m-%dT%H:%M:%S").to_string() != end
        {
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
    pub fn load_selected(
        &self,
        asset_id: &str,
        ids: &[String],
    ) -> Result<(ForecastAsset, GridSeries, Vec<ForecastCorrection>), StoreError> {
        let tx = self.connection.unchecked_transaction()?;
        let (asset, mut native) = self.load_native(asset_id)?;
        let corrections = self.load_corrections(asset_id, ids)?;
        tx.commit()?;
        if let Some(settings) = native.provenance.processing.as_object_mut() {
            settings.insert("asset_id".into(), asset.asset_id.clone().into());
        } else if !ids.is_empty() {
            return Err(invalid("native processing metadata is not an object"));
        }
        Ok((asset, native, corrections))
    }
    /// Materialize selected catalog rows while reading their native grids in one transaction.
    pub fn snapshot_selected(
        &self,
        destination: &Path,
        selections: &[(String, Vec<String>)],
    ) -> Result<Vec<(ForecastAsset, GridSeries, Vec<ForecastCorrection>)>, StoreError> {
        let tx = self.connection.unchecked_transaction()?;
        let mut destination_db = Connection::open(destination)?;
        destination_db.execute_batch(SCHEMA)?;
        destination_db.pragma_update(None, "user_version", 1)?;
        let destination_tx = destination_db.transaction()?;
        let mut selected = Vec::new();
        let mut asset_ids = HashSet::new();
        let mut correction_ids = HashSet::new();
        for (id, ids) in selections {
            let (asset, mut native) = self.load_native(id)?;
            let corrections = self.load_corrections(id, ids)?;
            if let Some(processing) = native.provenance.processing.as_object_mut() {
                processing.insert("asset_id".into(), id.clone().into());
            } else if !ids.is_empty() {
                return Err(invalid("native processing metadata is not an object"));
            }
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
                        format!("sources/original/{id}.grib2"),
                        format!("sources/native/{id}.nc"),
                        id
                    ],
                )?;
            }
            for correction in &corrections {
                if correction_ids.insert(correction.correction_id.clone()) {
                    let mut query =
                        tx.prepare("SELECT * FROM correction WHERE correction_id=?1")?;
                    let values = query.query_row([&correction.correction_id], |row| {
                        (0..11)
                            .map(|index| row.get::<_, rusqlite::types::Value>(index))
                            .collect::<Result<Vec<_>, _>>()
                    })?;
                    destination_tx.execute(
                        "INSERT INTO correction VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
                        rusqlite::params_from_iter(values),
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
