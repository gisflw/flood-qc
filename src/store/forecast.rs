use super::{
    ForecastCorrection, ForecastSource, GridSeries, StoreError, read_grid_series, write_grid_series,
};
use chrono::{DateTime, SecondsFormat, Timelike, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Component, Path, PathBuf},
};

#[derive(Debug, Clone)]
pub struct ForecastAssetChoice {
    pub asset_id: String,
    pub provider: String,
    pub model: String,
    pub product: String,
    pub cycle_utc: String,
    pub registered_at: String,
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
fn forecast_asset_id(identity: &serde_json::Value, source_id: &str) -> Result<String, StoreError> {
    let mut digest = Sha256::new();
    digest.update(b"forecast-asset:");
    digest.update(serde_json::to_vec(identity).map_err(|e| invalid(e.to_string()))?);
    digest.update(source_id.as_bytes());
    let label = |key: &str| -> Result<String, StoreError> {
        let text = identity
            .get(key)
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| invalid("missing forecast identity field"))?;
        let label: String = text
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() {
                    c.to_ascii_lowercase()
                } else {
                    '-'
                }
            })
            .take(24)
            .collect();
        if label.is_empty() {
            return Err(invalid("empty forecast identity field"));
        }
        Ok(label)
    };
    let cycle = identity
        .get("cycle_utc")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| invalid("missing forecast cycle"))?;
    let cycle = DateTime::parse_from_rfc3339(cycle)
        .map_err(|e| invalid(e.to_string()))?
        .with_timezone(&Utc)
        .format("%Y%m%dT%H%M%SZ");
    Ok(format!(
        "{}-{}-{}-{}-{:x}",
        label("provider")?,
        label("model")?,
        label("product")?,
        cycle,
        digest.finalize()
    ))
}
fn valid_asset_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 160
        && id.bytes().all(|b| {
            b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || b == b'T' || b == b'Z'
        })
}
fn logical_identity(
    provider: &str,
    model: &str,
    product: &str,
    cycle: &str,
    native: &GridSeries,
) -> serde_json::Value {
    let grid = &native.grid;
    serde_json::json!({"provider": provider, "model": model, "product": product, "cycle_utc": cycle, "grid": {"resolution_degrees": grid.resolution_degrees, "latitudes": grid.latitudes, "longitudes": grid.longitudes}, "operation_timezone": native.timeline.time_zone})
}
fn same_physical_grid(left: &super::RegularGridSpec, right: &super::RegularGridSpec) -> bool {
    left.resolution_degrees == right.resolution_degrees
        && left.latitudes == right.latitudes
        && left.longitudes == right.longitudes
}
pub struct ForecastSnapshotSelection {
    pub asset_id: String,
}
pub struct ForecastCatalog {
    workspace: PathBuf,
    root: PathBuf,
    writable: bool,
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
// Keep the original JSON representation for identities supplied by adapters.
// TOML table ordering is not stable and would otherwise change their digest.
mod json_string {
    use serde::{Deserialize, Deserializer, Serializer};
    pub fn serialize<S: Serializer>(
        value: &serde_json::Value,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        let text = serde_json::to_string(value).map_err(serde::ser::Error::custom)?;
        serializer.serialize_str(&text)
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<serde_json::Value, D::Error> {
        let text = String::deserialize(deserializer)?;
        serde_json::from_str(&text).map_err(serde::de::Error::custom)
    }
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct AssetRecord {
    asset_id: String,
    source_id: String,
    #[serde(with = "json_string")]
    logical_identity: serde_json::Value,
    registered_at: String,
    provider: String,
    model: String,
    product: String,
    cycle_utc: String,
    retained_source: PathBuf,
    native_artifact: PathBuf,
    native_sha256: String,
    native_byte_length: u64,
    source_format: String,
    source_sha256: String,
    source_byte_length: u64,
    #[serde(with = "json_string")]
    normalization: serde_json::Value,
    operation_timezone: String,
    valid_from: String,
    valid_to: String,
}

impl ForecastCatalog {
    fn access_lock(&self, exclusive: bool) -> Result<File, StoreError> {
        let path = self.workspace.join(&self.root).join("access.lock");
        let file = if exclusive {
            OpenOptions::new().read(true).write(true).open(path)?
        } else {
            File::open(path)?
        };
        if exclusive {
            file.lock()?;
        } else {
            file.lock_shared()?;
        }
        Ok(file)
    }
    fn initialize_lock(workspace: &Path, root: &Path) -> Result<(), StoreError> {
        OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(workspace.join(root).join("access.lock"))?;
        Ok(())
    }
    fn record(&self, id: &str) -> Result<AssetRecord, StoreError> {
        if !valid_asset_id(id) {
            return Err(invalid("invalid asset ID"));
        }
        let relative_path = self.root.join(id).join("metadata.toml");
        let path = resolve(&self.workspace, &relative_path)?;
        let record: AssetRecord = super::read_draft(&path).map_err(StoreError::Integrity)?;
        if record.asset_id != id
            || forecast_asset_id(&record.logical_identity, &record.source_id)? != id
        {
            return Err(StoreError::Integrity(format!(
                "asset identity mismatch: {id} != {}",
                forecast_asset_id(&record.logical_identity, &record.source_id)?
            )));
        }
        Ok(record)
    }
    fn records(&self) -> Result<Vec<AssetRecord>, StoreError> {
        let mut records = Vec::new();
        for entry in fs::read_dir(self.workspace.join(&self.root))? {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                continue;
            }
            let name = entry.file_name();
            let Some(id) = name.to_str() else {
                continue;
            };
            if valid_asset_id(id) {
                records.push(self.record(id)?);
            }
        }
        Ok(records)
    }
    fn choice(r: &AssetRecord) -> ForecastAssetChoice {
        ForecastAssetChoice {
            asset_id: r.asset_id.clone(),
            provider: r.provider.clone(),
            model: r.model.clone(),
            product: r.product.clone(),
            cycle_utc: r.cycle_utc.clone(),
            registered_at: r.registered_at.clone(),
        }
    }
    pub fn asset_choices(&self) -> Result<Vec<ForecastAssetChoice>, StoreError> {
        let mut result: Vec<_> = self.records()?.iter().map(Self::choice).collect();
        result.sort_by(|a, b| {
            b.cycle_utc
                .cmp(&a.cycle_utc)
                .then(b.registered_at.cmp(&a.registered_at))
                .then(a.asset_id.cmp(&b.asset_id))
        });
        Ok(result)
    }
    pub(crate) fn covering_assets(
        &self,
        cycle_at_or_before: DateTime<Utc>,
        start: chrono::NaiveDateTime,
        end: chrono::NaiveDateTime,
        zone: &str,
    ) -> Result<Vec<ForecastAssetChoice>, StoreError> {
        let cycle = cycle_at_or_before.to_rfc3339_opts(SecondsFormat::Secs, true);
        let start = start.format("%Y-%m-%dT%H:%M:%S").to_string();
        let end = end.format("%Y-%m-%dT%H:%M:%S").to_string();
        let mut result: Vec<_> = self
            .records()?
            .iter()
            .filter(|r| {
                r.cycle_utc <= cycle
                    && r.valid_from <= start
                    && r.valid_to >= end
                    && r.operation_timezone == zone
            })
            .map(Self::choice)
            .collect();
        result.sort_by(|a, b| {
            (&a.provider, &a.model, &a.product)
                .cmp(&(&b.provider, &b.model, &b.product))
                .then(b.cycle_utc.cmp(&a.cycle_utc))
                .then(b.registered_at.cmp(&a.registered_at))
                .then(a.asset_id.cmp(&b.asset_id))
        });
        Ok(result)
    }
    pub fn open_read_only(workspace: &Path, catalog: &Path) -> Result<Self, StoreError> {
        Self::open_existing(workspace, catalog, false)
    }
    pub fn open_writable(workspace: &Path, catalog: &Path) -> Result<Self, StoreError> {
        Self::open_existing(workspace, catalog, true)
    }
    fn open_existing(workspace: &Path, catalog: &Path, writable: bool) -> Result<Self, StoreError> {
        let workspace = workspace.canonicalize()?;
        let root = if catalog.is_absolute() {
            catalog
                .strip_prefix(&workspace)
                .map_err(|_| invalid("catalog outside workspace"))?
                .to_path_buf()
        } else {
            catalog.to_path_buf()
        };
        if !resolve(&workspace, &root)?.is_dir() {
            return Err(invalid("forecast catalog must be a directory"));
        }
        if writable {
            Self::initialize_lock(&workspace, &root)?;
        } else {
            File::open(workspace.join(&root).join("access.lock"))?;
        }
        Ok(Self {
            workspace,
            root,
            writable,
        })
    }
    pub fn open(
        workspace: &Path,
        catalog: &Path,
        artifact_root: &Path,
    ) -> Result<Self, StoreError> {
        let workspace = workspace.canonicalize()?;
        let root = if artifact_root.is_absolute() {
            artifact_root
                .strip_prefix(&workspace)
                .map_err(|_| invalid("artifacts outside workspace"))?
        } else {
            artifact_root
        };
        let catalog = if catalog.is_absolute() {
            catalog
                .strip_prefix(&workspace)
                .map_err(|_| invalid("catalog outside workspace"))?
        } else {
            catalog
        };
        if root != catalog {
            return Err(invalid("forecast catalog and artifact root must match"));
        }
        create_workspace_dir(&workspace, root)?;
        Self::initialize_lock(&workspace, root)?;
        Ok(Self {
            workspace,
            root: root.to_path_buf(),
            writable: true,
        })
    }
    pub(crate) fn check_native_available(&self, id: &str) -> Result<(), StoreError> {
        let _guard = self.access_lock(false)?;
        let record = self.record(id)?;
        let path = resolve(&self.workspace, &record.native_artifact)?;
        if digest(&path)? != (record.native_sha256, record.native_byte_length) {
            return Err(StoreError::Integrity("native length mismatch".into()));
        }
        Ok(())
    }
    fn commit(
        &self,
        mut record: AssetRecord,
        source: &Path,
        series: &GridSeries,
        canonical: bool,
        _force: bool,
    ) -> Result<ForecastAsset, StoreError> {
        if !self.writable {
            return Err(invalid("forecast catalog is read-only"));
        }
        let _guard = self.access_lock(true)?;
        let root = resolve(&self.workspace, &self.root)?;
        let dir = root.join(&record.asset_id);
        if dir.exists() && !fs::symlink_metadata(&dir)?.is_dir() {
            return Err(StoreError::InvalidPath(
                "forecast asset is not a directory".into(),
            ));
        }
        if dir.exists() {
            let (asset, previous) = self.load_native_unlocked(&record.asset_id)?;
            if previous.timeline != series.timeline
                || !same_physical_grid(&previous.grid, &series.grid)
                || previous.valid != series.valid
                || previous
                    .values
                    .iter()
                    .zip(&series.values)
                    .zip(&series.valid)
                    .any(|((a, b), valid)| *valid && a != b)
            {
                return Err(StoreError::Integrity("retained native grid differs".into()));
            }
            return Ok(asset);
        }
        let stage = tempfile::Builder::new()
            .prefix(".forecast-")
            .tempdir_in(&root)?;
        let original = if canonical {
            "source.nc"
        } else {
            "source.grib2"
        };
        copy_source(source, &stage.path().join(original))?;
        if digest(&stage.path().join(original))?
            != (record.source_sha256.clone(), record.source_byte_length)
        {
            return Err(StoreError::Integrity(
                "source changed during registration".into(),
            ));
        }
        let native = stage.path().join("native.nc");
        if canonical {
            fs::rename(stage.path().join(original), &native)?;
        } else {
            write_grid_series(&native, series)?;
            File::open(&native)?.sync_all()?;
        }
        (record.native_sha256, record.native_byte_length) = digest(&native)?;
        let relative_dir = self.root.join(&record.asset_id);
        record.retained_source = relative_dir.join(if canonical { "native.nc" } else { original });
        record.native_artifact = relative_dir.join("native.nc");
        super::write_draft(&stage.path().join("metadata.toml"), &record)
            .map_err(std::io::Error::other)?;
        super::sync_directory(stage.path()).map_err(std::io::Error::other)?;
        match fs::rename(stage.path(), &dir) {
            Ok(()) => super::sync_directory(&root).map_err(std::io::Error::other)?,
            Err(_) if dir.is_dir() => {}
            Err(e) => return Err(e.into()),
        }
        let (asset, previous) = self.load_native_unlocked(&record.asset_id)?;
        if previous.timeline != series.timeline
            || !same_physical_grid(&previous.grid, &series.grid)
            || previous.valid != series.valid
            || previous
                .values
                .iter()
                .zip(&series.values)
                .zip(&series.valid)
                .any(|((a, b), valid)| *valid && a != b)
        {
            return Err(StoreError::Integrity("retained native grid differs".into()));
        }
        Ok(asset)
    }
    pub fn register_canonical(
        &mut self,
        source: &Path,
        metadata: &CanonicalForecastMetadata,
    ) -> Result<ForecastAsset, StoreError> {
        self.register_canonical_with_force(source, metadata, false)
    }
    pub fn register_canonical_force(
        &mut self,
        source: &Path,
        metadata: &CanonicalForecastMetadata,
    ) -> Result<ForecastAsset, StoreError> {
        self.register_canonical_with_force(source, metadata, true)
    }
    fn register_canonical_with_force(
        &mut self,
        source: &Path,
        metadata: &CanonicalForecastMetadata,
        force: bool,
    ) -> Result<ForecastAsset, StoreError> {
        let (sha, len) = digest(source)?;
        let series = read_grid_series(source)?;
        metadata.validate(&series)?;
        let settings = serde_json::to_value(metadata).map_err(|e| invalid(e.to_string()))?;
        let encoded = serde_json::to_string(metadata).map_err(|e| invalid(e.to_string()))?;
        let source_id = canonical_source_id(&sha, &encoded);
        let (start, end) = bounds(&series);
        let logical_identity = logical_identity(
            &metadata.provider,
            &metadata.model,
            &metadata.product,
            &metadata.cycle(),
            &series,
        );
        let record = AssetRecord {
            asset_id: forecast_asset_id(&logical_identity, &source_id)?,
            source_id,
            logical_identity,
            registered_at: Utc::now().to_rfc3339_opts(SecondsFormat::Nanos, true),
            provider: metadata.provider.clone(),
            model: metadata.model.clone(),
            product: metadata.product.clone(),
            cycle_utc: metadata.cycle(),
            retained_source: PathBuf::new(),
            native_artifact: PathBuf::new(),
            native_sha256: String::new(),
            native_byte_length: 0,
            source_format: "netcdf".into(),
            source_sha256: sha,
            source_byte_length: len,
            normalization: settings,
            operation_timezone: series.timeline.time_zone.clone(),
            valid_from: start,
            valid_to: end,
        };
        self.commit(record, source, &series, true, force)
    }
    pub fn register_native(
        &mut self,
        source_path: &Path,
        source: &ForecastSource,
        native: &GridSeries,
    ) -> Result<ForecastAsset, StoreError> {
        self.register_native_with_force(source_path, source, native, false)
    }
    pub fn register_native_force(
        &mut self,
        source_path: &Path,
        source: &ForecastSource,
        native: &GridSeries,
    ) -> Result<ForecastAsset, StoreError> {
        self.register_native_with_force(source_path, source, native, true)
    }
    fn register_native_with_force(
        &mut self,
        source_path: &Path,
        source: &ForecastSource,
        native: &GridSeries,
        force: bool,
    ) -> Result<ForecastAsset, StoreError> {
        check_source(source, native)?;
        if digest(source_path)? != (source.sha256.clone(), source.byte_length) {
            return Err(StoreError::Integrity(
                "source changed during normalization".into(),
            ));
        }
        let (start, end) = bounds(native);
        let provider = match source.provider {
            super::ForecastProvider::Ecmwf => "ecmwf",
            super::ForecastProvider::Noaa => "noaa",
        };
        let logical_identity = logical_identity(
            provider,
            &source.model,
            &source.product,
            &source.cycle_utc.to_rfc3339_opts(SecondsFormat::Secs, true),
            native,
        );
        let record = AssetRecord {
            asset_id: forecast_asset_id(&logical_identity, &source.source_id)?,
            source_id: source.source_id.clone(),
            logical_identity,
            registered_at: Utc::now().to_rfc3339_opts(SecondsFormat::Nanos, true),
            provider: provider.into(),
            model: source.model.clone(),
            product: source.product.clone(),
            cycle_utc: source.cycle_utc.to_rfc3339_opts(SecondsFormat::Secs, true),
            retained_source: PathBuf::new(),
            native_artifact: PathBuf::new(),
            native_sha256: String::new(),
            native_byte_length: 0,
            source_format: "grib2".into(),
            source_sha256: source.sha256.clone(),
            source_byte_length: source.byte_length,
            normalization: source.normalization_json.clone(),
            operation_timezone: native.timeline.time_zone.clone(),
            valid_from: start,
            valid_to: end,
        };
        self.commit(record, source_path, native, false, force)
    }
    /// Return the declared native content identity without reading the grid.
    /// Loading the native grid independently verifies these values against its bytes.
    pub fn native_identity(&self, asset_id: &str) -> Result<(String, u64), StoreError> {
        let _guard = self.access_lock(false)?;
        let record = self.record(asset_id)?;
        Ok((record.native_sha256, record.native_byte_length))
    }
    pub fn load_native(&self, asset_id: &str) -> Result<(ForecastAsset, GridSeries), StoreError> {
        let _guard = self.access_lock(false)?;
        self.load_native_unlocked(asset_id)
    }
    fn load_native_unlocked(
        &self,
        asset_id: &str,
    ) -> Result<(ForecastAsset, GridSeries), StoreError> {
        let r = self.record(asset_id)?;
        let expected_parent = self.root.join(asset_id);
        let valid_name = r
            .native_artifact
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| {
                name == "native.nc" || (name.starts_with("native-") && name.ends_with(".nc"))
            });
        if r.native_artifact.parent() != Some(expected_parent.as_path())
            || !valid_name
            || digest(&resolve(&self.workspace, &r.native_artifact)?)?
                != (r.native_sha256.clone(), r.native_byte_length)
        {
            return Err(StoreError::Integrity(
                "native forecast digest mismatch".into(),
            ));
        }
        let series = read_grid_series(&self.workspace.join(&r.native_artifact))?;
        if logical_identity(&r.provider, &r.model, &r.product, &r.cycle_utc, &series)
            != r.logical_identity
        {
            return Err(StoreError::Integrity(
                "forecast identity metadata mismatch".into(),
            ));
        }
        if series.timeline.time_zone != r.operation_timezone
            || bounds(&series) != (r.valid_from.clone(), r.valid_to.clone())
        {
            return Err(StoreError::Integrity("forecast metadata mismatch".into()));
        }
        if r.source_format == "netcdf" {
            let metadata: CanonicalForecastMetadata =
                serde_json::from_value(r.normalization.clone())
                    .map_err(|e| StoreError::Integrity(e.to_string()))?;
            let encoded = serde_json::to_string(&metadata).map_err(|e| invalid(e.to_string()))?;
            if canonical_source_id(&r.source_sha256, &encoded) != r.source_id
                || metadata.provider != r.provider
                || metadata.model != r.model
                || metadata.product != r.product
                || metadata.cycle() != r.cycle_utc
                || r.source_sha256 != r.native_sha256
                || r.source_byte_length != r.native_byte_length
            {
                return Err(StoreError::Integrity("forecast metadata mismatch".into()));
            }
            metadata.validate(&series)?;
        } else if r.source_format == "grib2" {
            let provider = match r.provider.as_str() {
                "ecmwf" => super::ForecastProvider::Ecmwf,
                "noaa" => super::ForecastProvider::Noaa,
                _ => return Err(invalid("unknown provider")),
            };
            let source = ForecastSource {
                source_id: r.source_id.clone(),
                provider,
                model: r.model.clone(),
                product: r.product.clone(),
                cycle_utc: DateTime::parse_from_rfc3339(&r.cycle_utc)
                    .map_err(|e| invalid(e.to_string()))?
                    .to_utc(),
                sha256: r.source_sha256.clone(),
                byte_length: r.source_byte_length,
                normalization_json: r.normalization.clone(),
            };
            check_source(&source, &series)?;
        } else {
            return Err(invalid("unknown forecast format"));
        }
        Ok((
            ForecastAsset {
                asset_id: r.asset_id,
                source_id: r.source_id,
                retained_source: r.retained_source,
                native_artifact: r.native_artifact,
            },
            series,
        ))
    }
    pub(crate) fn load_for_run(
        &self,
        asset_id: &str,
    ) -> Result<(ForecastAsset, GridSeries), StoreError> {
        let _guard = self.access_lock(false)?;
        let (asset, mut native) = self.load_native_unlocked(asset_id)?;
        self.selected_provenance(&asset, &mut native)?;
        Ok((asset, native))
    }
    fn selected_provenance(
        &self,
        asset: &ForecastAsset,
        native: &mut GridSeries,
    ) -> Result<(), StoreError> {
        let r = self.record(&asset.asset_id)?;
        let processing = &mut native.provenance.processing;
        if r.source_format == "netcdf" {
            *processing = serde_json::json!({"parent": std::mem::take(processing), "normalization": r.normalization, "asset_id": asset.asset_id});
        } else if let Some(map) = processing.as_object_mut() {
            map.insert("asset_id".into(), asset.asset_id.clone().into());
        }
        Ok(())
    }
    pub fn snapshot_selected_values(
        &self,
        destination: &Path,
        selections: &[ForecastSnapshotSelection],
    ) -> Result<Vec<(ForecastAsset, GridSeries)>, StoreError> {
        let _guard = self.access_lock(false)?;
        fs::create_dir_all(destination)?;
        Self::initialize_lock(destination, Path::new(""))?;
        let mut selected = Vec::new();
        let mut records = Vec::new();
        for selection in selections {
            let (asset, mut native) = self.load_native_unlocked(&selection.asset_id)?;
            self.selected_provenance(&asset, &mut native)?;
            let mut record = self.record(&selection.asset_id)?;
            if record.native_artifact != asset.native_artifact {
                return Err(StoreError::Integrity(
                    "forecast changed during snapshot".into(),
                ));
            }
            let directory = destination.join(&selection.asset_id);
            fs::create_dir_all(&directory)?;
            let target = directory.join("native.nc");
            if !target.exists() {
                copy_source(&resolve(&self.workspace, &asset.native_artifact)?, &target)?;
            }
            if digest(&target)? != (record.native_sha256.clone(), record.native_byte_length) {
                return Err(StoreError::Integrity(
                    "snapshot native digest mismatch".into(),
                ));
            }
            let relative = Path::new("sources/forecasts").join(&selection.asset_id);
            record.native_artifact = relative.join("native.nc");
            record.retained_source = relative.join(if record.source_format == "netcdf" {
                "source.nc"
            } else {
                "source.grib2"
            });
            records.push(record);
            selected.push((asset, native));
        }
        fs::create_dir_all(destination)?;
        for record in records {
            let directory = destination.join(&record.asset_id);
            fs::create_dir_all(&directory)?;
            super::write_draft(&directory.join("metadata.toml"), &record)
                .map_err(std::io::Error::other)?;
        }
        Ok(selected)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::{IntervalAxis, Provenance, RegularGridSpec, TimeInterval};

    #[test]
    fn canonical_asset_survives_original_removal_and_detects_grid_corruption() {
        fs::create_dir_all("runs").unwrap();
        let workspace = tempfile::tempdir_in("runs").unwrap();
        let workspace = workspace.path();
        let start = chrono::NaiveDate::from_ymd_opt(2026, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        let grid = GridSeries {
            timeline: IntervalAxis {
                time_zone: "UTC".into(),
                bounds: vec![TimeInterval {
                    start,
                    end: start + chrono::Duration::hours(1),
                }],
            },
            grid: RegularGridSpec {
                requested_bbox: [0., 0., 1., 1.],
                resolution_degrees: 1.,
                latitudes: vec![0.5],
                longitudes: vec![0.5],
                effective_bbox: [0., 0., 1., 1.],
            },
            values: vec![3.],
            valid: vec![true],
            provenance: Provenance {
                source_ids: vec!["fixture".into()],
                processing: serde_json::json!({}),
            },
        };
        let input = workspace.join("input.nc");
        write_grid_series(&input, &grid).unwrap();
        let metadata = CanonicalForecastMetadata {
            provider: "fixture".into(),
            model: "model".into(),
            product: "rain".into(),
            cycle_utc: DateTime::parse_from_rfc3339("2026-01-01T00:00:00Z")
                .unwrap()
                .to_utc(),
        };
        let mut catalog = ForecastCatalog::open(
            workspace,
            Path::new("assets/forecasts"),
            Path::new("assets/forecasts"),
        )
        .unwrap();
        let asset = catalog.register_canonical(&input, &metadata).unwrap();
        assert_eq!(asset.retained_source, asset.native_artifact);
        fs::remove_file(input).unwrap();
        assert_eq!(catalog.load_native(&asset.asset_id).unwrap().1.values, [3.]);
        fs::write(workspace.join(&asset.native_artifact), b"corrupt").unwrap();
        assert!(catalog.load_native(&asset.asset_id).is_err());
    }
}
