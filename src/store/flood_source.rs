use super::netcdf::{
    add, array, attributes, base, check_coordinate, dimension, i64_bytes, ints, open, write,
};
use super::{IntervalAxis, Provenance, StoreError};
use std::{collections::HashSet, fs, path::Path};
use terrio::DataType;

pub(crate) const FLOOD_SOURCE_FILE: &str = "flood_source.nc";
pub(crate) const FLOOD_SOURCE_DIAGNOSTIC: &str = "flood_source_unavailable.txt";

#[derive(Debug, PartialEq)]
pub(crate) struct FloodSource {
    pub(crate) timeline: IntervalAxis,
    pub(crate) mini_ids: Vec<i64>,
    pub(crate) depth_m: Vec<f32>,
    pub(crate) channel_depth_m: Vec<f32>,
    pub(crate) historical_count: usize,
    pub(crate) provenance: Provenance,
}

impl FloodSource {
    fn validate(&self) -> Result<(), StoreError> {
        self.timeline
            .validate()
            .map_err(|error| StoreError::InvalidValue(error.to_string()))?;
        let times = self.timeline.bounds.len();
        let minis = self.mini_ids.len();
        let mut seen = HashSet::new();
        if self.historical_count == 0
            || self.historical_count >= times
            || minis == 0
            || times.checked_mul(minis) != Some(self.depth_m.len())
            || self.channel_depth_m.len() != minis
            || self.mini_ids.iter().any(|id| *id <= 0 || !seen.insert(id))
            || self
                .depth_m
                .iter()
                .any(|value| !value.is_finite() || *value < 0.0)
            || self
                .channel_depth_m
                .iter()
                .any(|value| !value.is_finite() || *value <= 0.0)
        {
            return Err(StoreError::InvalidValue(
                "invalid flood source shape, IDs, or depth".into(),
            ));
        }
        Ok(())
    }
}

fn write_source(path: &Path, source: &FloodSource) -> Result<(), StoreError> {
    source.validate()?;
    let mut root = base(&source.timeline, "flood_source", &source.provenance, "m")?;
    root.attributes["historical_count"] = (source.historical_count as i64).into();
    root.dimensions
        .push(dimension("basin", source.mini_ids.len(), false));
    add(
        &mut root,
        "mini_id",
        DataType::I64,
        &["basin"],
        i64_bytes(source.mini_ids.iter().copied()),
        attributes(vec![]),
    )?;
    for (name, dims, values) in [
        ("depth", &["time", "basin"][..], &source.depth_m),
        ("channel_depth", &["basin"][..], &source.channel_depth_m),
    ] {
        add(
            &mut root,
            name,
            DataType::F32,
            dims,
            values.iter().copied().flat_map(f32::to_le_bytes).collect(),
            attributes(vec![("units", "m".into())]),
        )?;
    }
    write(root, path)
}

pub(crate) fn write_flood_source(path: &Path, source: &FloodSource) -> Result<(), StoreError> {
    let temporary = path.with_file_name(".flood_source.tmp.nc");
    let result = (|| {
        write_source(&temporary, source)?;
        if super::read_flood_source(&temporary)? != *source {
            return Err(StoreError::Integrity(
                "flood source differs after writing".into(),
            ));
        }
        fs::File::open(&temporary)?.sync_all()?;
        fs::rename(&temporary, path)?;
        if let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            && let Err(error) = super::sync_directory(parent)
        {
            let _ = fs::remove_file(path);
            return Err(StoreError::Format(error));
        }
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

fn float32(bytes: Vec<u8>) -> Result<Vec<f32>, StoreError> {
    let (words, remainder) = bytes.as_chunks::<4>();
    if !remainder.is_empty() {
        return Err(StoreError::Schema("invalid float32 bytes".into()));
    }
    Ok(words.iter().map(|word| f32::from_le_bytes(*word)).collect())
}

pub(crate) fn read_flood_source(path: &Path) -> Result<FloodSource, StoreError> {
    let (dataset, timeline, provenance) = open(path, "flood_source", "m")?;
    let historical_count = dataset.root.attributes["historical_count"]
        .as_u64()
        .and_then(|value| usize::try_from(value).ok())
        .ok_or_else(|| StoreError::Schema("invalid flood source historical count".into()))?;
    check_coordinate(&dataset, "depth", "m")?;
    check_coordinate(&dataset, "channel_depth", "m")?;
    let source = FloodSource {
        timeline,
        mini_ids: ints(array(&dataset, "mini_id", DataType::I64, &["basin"])?)?,
        depth_m: float32(array(&dataset, "depth", DataType::F32, &["time", "basin"])?)?,
        channel_depth_m: float32(array(&dataset, "channel_depth", DataType::F32, &["basin"])?)?,
        historical_count,
        provenance,
    };
    source.validate()?;
    Ok(source)
}
