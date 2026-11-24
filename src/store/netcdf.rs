use std::path::Path;

use chrono::{Duration, NaiveDateTime};
use serde_json::Value;
use sha2::{Digest, Sha256};
use terrio::{
    DataType,
    multidim::{Array, ArrayInfo, Dataset, Dimension, Group},
};

use super::{
    BasinForcing, BasinGeometry, GridFrame, GridSeries, IntervalAxis, Provenance, RegularGridSpec,
    ResultSeries, StationResults, TimeInterval,
};

#[derive(Debug)]
pub enum StoreError {
    Io(std::io::Error),
    Sqlite(rusqlite::Error),
    Format(String),
    Schema(String),
    Integrity(String),
    InvalidPath(String),
    InvalidValue(String),
}
impl std::fmt::Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for StoreError {}
impl From<std::io::Error> for StoreError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}
impl From<rusqlite::Error> for StoreError {
    fn from(e: rusqlite::Error) -> Self {
        Self::Sqlite(e)
    }
}
pub(super) fn attributes(values: Vec<(&str, Value)>) -> Value {
    Value::Object(values.into_iter().map(|(k, v)| (k.to_owned(), v)).collect())
}
fn format_error(e: impl std::fmt::Display) -> StoreError {
    StoreError::Format(e.to_string())
}
fn invalid(e: impl std::fmt::Display) -> StoreError {
    StoreError::InvalidValue(e.to_string())
}
fn path(p: &Path) -> Result<&str, StoreError> {
    p.to_str()
        .ok_or_else(|| StoreError::InvalidPath("path is not UTF-8".into()))
}
fn attr<'a>(v: &'a Value, k: &str) -> Result<&'a str, StoreError> {
    v.get(k)
        .and_then(Value::as_str)
        .ok_or_else(|| StoreError::Schema(format!("missing attribute {k}")))
}
pub(super) fn dimension(name: &str, size: usize, index: bool) -> Dimension {
    Dimension {
        name: name.into(),
        path: format!("/{name}"),
        size,
        unlimited: false,
        indexing_variable: index.then(|| format!("/{name}")),
    }
}
pub(super) fn add(
    root: &mut Group,
    name: &str,
    datatype: DataType,
    dims: &[&str],
    bytes: Vec<u8>,
    attributes: Value,
) -> Result<(), StoreError> {
    let dimensions = dims
        .iter()
        .map(|name| {
            root.dimensions
                .iter()
                .find(|d| d.name == *name)
                .cloned()
                .ok_or_else(|| StoreError::Schema(format!("missing dimension {name}")))
        })
        .collect::<Result<Vec<_>, _>>()?;
    root.arrays.push(
        Array::from_bytes(
            ArrayInfo {
                name: name.into(),
                path: format!("/{name}"),
                data_type: datatype,
                dimensions,
                chunks: None,
                attributes,
                coordinates: vec![],
            },
            bytes,
        )
        .map_err(format_error)?,
    );
    Ok(())
}
pub(super) fn i64_bytes(values: impl IntoIterator<Item = i64>) -> Vec<u8> {
    values.into_iter().flat_map(i64::to_le_bytes).collect()
}
fn f64_bytes(values: impl IntoIterator<Item = f64>) -> Vec<u8> {
    values.into_iter().flat_map(f64::to_le_bytes).collect()
}
pub(super) fn base(
    axis: &IntervalAxis,
    kind: &str,
    provenance: &Provenance,
    units: &str,
) -> Result<Group, StoreError> {
    axis.validate().map_err(invalid)?;
    let origin = axis.bounds[0].start;
    let mut root = Group::new("/");
    root.attributes = attributes(vec![
        ("schema_version", 1.into()),
        ("artifact_kind", kind.into()),
        ("units", units.into()),
        ("operation_timezone", axis.time_zone.clone().into()),
        (
            "processing_json",
            serde_json::to_string(provenance)
                .map_err(format_error)?
                .into(),
        ),
        (
            "source_id",
            provenance
                .source_ids
                .first()
                .cloned()
                .unwrap_or_default()
                .into(),
        ),
        ("crs", "EPSG:4326".into()),
    ]);
    root.dimensions = vec![
        dimension("time", axis.bounds.len(), true),
        dimension("bounds", 2, false),
    ];
    let units = format!("seconds since {}", origin.format("%Y-%m-%d %H:%M:%S"));
    add(
        &mut root,
        "time",
        DataType::I64,
        &["time"],
        i64_bytes(axis.bounds.iter().map(|i| (i.end - origin).num_seconds())),
        attributes(vec![
            ("units", units.into()),
            ("calendar", "proleptic_gregorian".into()),
            ("bounds", "time_bounds".into()),
        ]),
    )?;
    add(
        &mut root,
        "time_bounds",
        DataType::I64,
        &["time", "bounds"],
        i64_bytes(axis.bounds.iter().flat_map(|i| {
            [
                (i.start - origin).num_seconds(),
                (i.end - origin).num_seconds(),
            ]
        })),
        attributes(vec![]),
    )?;
    Ok(root)
}
fn precipitation(
    root: &mut Group,
    dimensions: &[&str],
    values: &[f64],
    valid: &[bool],
) -> Result<(), StoreError> {
    let vals = values
        .iter()
        .zip(valid)
        .map(|(v, m)| if *m { *v } else { f64::NAN });
    add(
        root,
        "precipitation",
        DataType::F64,
        dimensions,
        f64_bytes(vals),
        attributes(vec![("units", "mm".into()), ("_FillValue", "NaN".into())]),
    )?;
    add(
        root,
        "valid",
        DataType::U8,
        dimensions,
        valid.iter().map(|v| u8::from(*v)).collect(),
        attributes(vec![]),
    )
}
pub(super) fn write(root: Group, p: &Path) -> Result<(), StoreError> {
    Dataset {
        driver: "Memory".into(),
        root,
    }
    .write(path(p)?)
    .map_err(format_error)
}
fn write_grid(p: &Path, series: &GridSeries, kind: &str) -> Result<(), StoreError> {
    series.validate().map_err(invalid)?;
    let g = &series.grid;
    let mut root = base(&series.timeline, kind, &series.provenance, "mm")?;
    root.attributes["requested_bbox"] =
        Value::from(serde_json::to_string(&g.requested_bbox).map_err(format_error)?);
    root.attributes["effective_bbox"] =
        Value::from(serde_json::to_string(&g.effective_bbox).map_err(format_error)?);
    root.attributes["latitude_orientation"] = Value::from("increasing");
    root.attributes["longitude_orientation"] = Value::from("increasing");
    root.attributes["latitude_resolution_degrees"] = Value::from(g.resolution_degrees);
    root.attributes["longitude_resolution_degrees"] = Value::from(g.resolution_degrees);
    root.dimensions.extend([
        dimension("latitude", g.latitudes.len(), true),
        dimension("longitude", g.longitudes.len(), true),
    ]);
    add(
        &mut root,
        "latitude",
        DataType::F64,
        &["latitude"],
        f64_bytes(g.latitudes.iter().copied()),
        attributes(vec![
            ("units", "degrees_north".into()),
            ("axis", "Y".into()),
        ]),
    )?;
    add(
        &mut root,
        "longitude",
        DataType::F64,
        &["longitude"],
        f64_bytes(g.longitudes.iter().copied()),
        attributes(vec![("units", "degrees_east".into()), ("axis", "X".into())]),
    )?;
    precipitation(
        &mut root,
        &["time", "latitude", "longitude"],
        &series.values,
        &series.valid,
    )?;
    write(root, p)
}
fn add_geometry(root: &mut Group, b: &BasinGeometry) -> Result<(), StoreError> {
    let geometry = serde_json::to_string(b).map_err(format_error)?;
    root.attributes["geometry_sha256"] =
        Value::from(format!("{:x}", Sha256::digest(geometry.as_bytes())));
    root.attributes["geometry_source_json"] =
        Value::from(serde_json::to_string(&b.source).map_err(format_error)?);
    root.dimensions
        .push(dimension("basin", b.mini_ids.len(), false));
    add(
        root,
        "mini_id",
        DataType::I64,
        &["basin"],
        i64_bytes(b.mini_ids.iter().copied()),
        attributes(vec![]),
    )?;
    add(
        root,
        "basin_latitude",
        DataType::F64,
        &["basin"],
        f64_bytes(b.latitudes.iter().copied()),
        attributes(vec![("units", "degrees_north".into())]),
    )?;
    add(
        root,
        "basin_longitude",
        DataType::F64,
        &["basin"],
        f64_bytes(b.longitudes.iter().copied()),
        attributes(vec![("units", "degrees_east".into())]),
    )?;
    Ok(())
}
pub fn write_basin_forcing(p: &Path, forcing: &BasinForcing) -> Result<(), StoreError> {
    forcing.validate().map_err(invalid)?;
    let b = &forcing.basins;
    let mut root = base(&forcing.timeline, "basin", &forcing.provenance, "mm")?;
    add_geometry(&mut root, b)?;
    precipitation(
        &mut root,
        &["time", "basin"],
        &forcing.values,
        &forcing.valid,
    )?;
    write(root, p)
}
pub(super) fn array(
    ds: &Dataset,
    name: &str,
    datatype: DataType,
    dims: &[&str],
) -> Result<Vec<u8>, StoreError> {
    let a = ds.array(name).map_err(format_error)?;
    if a.info.dimensions.iter().any(|dim| {
        ds.root
            .dimensions
            .iter()
            .find(|root| root.name == dim.name)
            .is_none_or(|root| root.size != dim.size)
    }) {
        return Err(StoreError::Schema(format!(
            "invalid {name} dimension sizes"
        )));
    }
    if a.info.data_type != datatype
        || a.info
            .dimensions
            .iter()
            .map(|d| d.name.as_str())
            .ne(dims.iter().copied())
    {
        return Err(StoreError::Schema(format!(
            "invalid {name} type/dimensions"
        )));
    }
    Ok(a.read().map_err(format_error)?.bytes)
}
pub(super) fn ints(bytes: Vec<u8>) -> Result<Vec<i64>, StoreError> {
    if !bytes.len().is_multiple_of(8) {
        return Err(StoreError::Schema("invalid integer bytes".into()));
    }
    Ok(bytes
        .as_chunks::<8>()
        .0
        .iter()
        .map(|v| i64::from_le_bytes(*v))
        .collect())
}
fn floats(bytes: Vec<u8>) -> Result<Vec<f64>, StoreError> {
    if !bytes.len().is_multiple_of(8) {
        return Err(StoreError::Schema("invalid float bytes".into()));
    }
    Ok(bytes
        .as_chunks::<8>()
        .0
        .iter()
        .map(|v| f64::from_le_bytes(*v))
        .collect())
}
pub(super) fn open(
    p: &Path,
    kind: &str,
    units: &str,
) -> Result<(Dataset, IntervalAxis, Provenance), StoreError> {
    let ds = terrio::multidim::open(path(p)?).map_err(format_error)?;
    let attrs = &ds.root.attributes;
    if attrs.get("schema_version").and_then(Value::as_i64) != Some(1)
        || attr(attrs, "artifact_kind")? != kind
        || attr(attrs, "units")? != units
        || attr(attrs, "crs")? != "EPSG:4326"
    {
        return Err(StoreError::Schema("unsupported artifact metadata".into()));
    }
    let time = ds.array("time").map_err(format_error)?;
    let units = attr(&time.info.attributes, "units")?
        .strip_prefix("seconds since ")
        .ok_or_else(|| StoreError::Schema("invalid time units".into()))?;
    if attr(&time.info.attributes, "calendar")? != "proleptic_gregorian"
        || attr(&time.info.attributes, "bounds")? != "time_bounds"
    {
        return Err(StoreError::Schema("invalid time metadata".into()));
    }
    let origin = NaiveDateTime::parse_from_str(units, "%Y-%m-%d %H:%M:%S").map_err(format_error)?;
    let times = ints(array(&ds, "time", DataType::I64, &["time"])?)?;
    let bounds = ints(array(
        &ds,
        "time_bounds",
        DataType::I64,
        &["time", "bounds"],
    )?)?;
    if times.len().checked_mul(2) != Some(bounds.len()) {
        return Err(StoreError::Schema("time bounds shape mismatch".into()));
    }
    let mut intervals = Vec::with_capacity(times.len());
    for (t, b) in times.iter().zip(bounds.as_chunks::<2>().0.iter()) {
        if *t != b[1] {
            return Err(StoreError::Integrity(
                "time differs from interval end".into(),
            ));
        }
        let start = origin
            .checked_add_signed(
                Duration::try_seconds(b[0]).ok_or_else(|| invalid("time overflow"))?,
            )
            .ok_or_else(|| invalid("time overflow"))?;
        let end = origin
            .checked_add_signed(
                Duration::try_seconds(b[1]).ok_or_else(|| invalid("time overflow"))?,
            )
            .ok_or_else(|| invalid("time overflow"))?;
        intervals.push(TimeInterval { start, end });
    }
    let axis = IntervalAxis {
        time_zone: attr(attrs, "operation_timezone")?.into(),
        bounds: intervals,
    };
    axis.validate().map_err(invalid)?;
    let provenance: Provenance =
        serde_json::from_str(attr(attrs, "processing_json")?).map_err(format_error)?;
    if attr(attrs, "source_id")? != provenance.source_ids.first().map_or("", String::as_str) {
        return Err(StoreError::Integrity("source identity mismatch".into()));
    }
    Ok((ds, axis, provenance))
}
pub(super) fn check_coordinate(ds: &Dataset, name: &str, units: &str) -> Result<(), StoreError> {
    let a = ds.array(name).map_err(format_error)?;
    if attr(&a.info.attributes, "units")? != units {
        return Err(StoreError::Schema(format!("invalid {name} units")));
    }
    Ok(())
}
fn read_values(ds: &Dataset, dims: &[&str]) -> Result<(Vec<f64>, Vec<bool>), StoreError> {
    let a = ds.array("precipitation").map_err(format_error)?;
    if attr(&a.info.attributes, "units")? != "mm"
        || attr(&a.info.attributes, "_FillValue")? != "NaN"
    {
        return Err(StoreError::Schema(
            "invalid precipitation attributes".into(),
        ));
    }
    let values = floats(array(ds, "precipitation", DataType::F64, dims)?)?;
    let mask = array(ds, "valid", DataType::U8, dims)?;
    if values.len() != mask.len()
        || values.iter().zip(&mask).any(|(v, m)| {
            !matches!(*m, 0 | 1) || (*m == 1 && !v.is_finite()) || (*m == 0 && !v.is_nan())
        })
    {
        return Err(StoreError::Integrity(
            "precipitation mask/value disagreement".into(),
        ));
    }
    Ok((values, mask.into_iter().map(|v| v == 1).collect()))
}
fn read_grid(p: &Path, kind: &str) -> Result<GridSeries, StoreError> {
    let (ds, timeline, provenance) = open(p, kind, "mm")?;
    let attrs = &ds.root.attributes;
    check_coordinate(&ds, "latitude", "degrees_north")?;
    check_coordinate(&ds, "longitude", "degrees_east")?;
    let latitudes = floats(array(&ds, "latitude", DataType::F64, &["latitude"])?)?;
    let longitudes = floats(array(&ds, "longitude", DataType::F64, &["longitude"])?)?;
    let requested_bbox =
        serde_json::from_str(attr(attrs, "requested_bbox")?).map_err(format_error)?;
    let effective_bbox =
        serde_json::from_str(attr(attrs, "effective_bbox")?).map_err(format_error)?;
    let resolution_degrees = attrs
        .get("latitude_resolution_degrees")
        .and_then(Value::as_f64)
        .ok_or_else(|| StoreError::Schema("missing grid resolution".into()))?;
    if attr(attrs, "latitude_orientation")? != "increasing"
        || attr(attrs, "longitude_orientation")? != "increasing"
        || attrs
            .get("longitude_resolution_degrees")
            .and_then(Value::as_f64)
            != Some(resolution_degrees)
    {
        return Err(StoreError::Integrity("grid metadata mismatch".into()));
    }
    let grid = RegularGridSpec {
        requested_bbox,
        resolution_degrees,
        latitudes,
        longitudes,
        effective_bbox,
    };
    let (values, valid) = read_values(&ds, &["time", "latitude", "longitude"])?;
    let out = GridSeries {
        timeline,
        grid,
        values,
        valid,
        provenance,
    };
    out.validate().map_err(invalid)?;
    Ok(out)
}
fn read_geometry(ds: &Dataset) -> Result<BasinGeometry, StoreError> {
    check_coordinate(ds, "basin_latitude", "degrees_north")?;
    check_coordinate(ds, "basin_longitude", "degrees_east")?;
    let basins = BasinGeometry {
        mini_ids: ints(array(ds, "mini_id", DataType::I64, &["basin"])?)?,
        latitudes: floats(array(ds, "basin_latitude", DataType::F64, &["basin"])?)?,
        longitudes: floats(array(ds, "basin_longitude", DataType::F64, &["basin"])?)?,
        source: serde_json::from_str(attr(&ds.root.attributes, "geometry_source_json")?)
            .map_err(format_error)?,
    };
    let geometry = serde_json::to_string(&basins).map_err(format_error)?;
    if attr(&ds.root.attributes, "geometry_sha256")?
        != format!("{:x}", Sha256::digest(geometry.as_bytes()))
    {
        return Err(StoreError::Integrity("geometry hash mismatch".into()));
    }
    basins.validate().map_err(invalid)?;
    Ok(basins)
}
pub fn read_basin_forcing(p: &Path) -> Result<BasinForcing, StoreError> {
    let (ds, timeline, provenance) = open(p, "basin", "mm")?;
    let basins = read_geometry(&ds)?;
    let (values, valid) = read_values(&ds, &["time", "basin"])?;
    let out = BasinForcing {
        timeline,
        basins,
        values,
        valid,
        provenance,
    };
    out.validate().map_err(invalid)?;
    Ok(out)
}

pub fn write_grid_series(p: &Path, series: &GridSeries) -> Result<(), StoreError> {
    write_grid(p, series, "grid")
}
pub fn read_grid_series(p: &Path) -> Result<GridSeries, StoreError> {
    read_grid(p, "grid")
}
pub fn write_grid_frame(p: &Path, frame: &GridFrame) -> Result<(), StoreError> {
    frame.validate().map_err(invalid)?;
    write_grid(
        p,
        &GridSeries {
            timeline: IntervalAxis {
                time_zone: frame.time_zone.clone(),
                bounds: vec![frame.interval.clone()],
            },
            grid: frame.grid.clone(),
            values: frame.values.clone(),
            valid: frame.valid.clone(),
            provenance: frame.provenance.clone(),
        },
        "transformed_frame",
    )
}
pub fn read_grid_frame(p: &Path) -> Result<GridFrame, StoreError> {
    let series = read_grid(p, "transformed_frame")?;
    if series.timeline.bounds.len() != 1 {
        return Err(StoreError::Schema("frame must contain one interval".into()));
    }
    let frame = GridFrame {
        interval: series.timeline.bounds[0].clone(),
        time_zone: series.timeline.time_zone,
        grid: series.grid,
        values: series.values,
        valid: series.valid,
        provenance: series.provenance,
    };
    frame.validate().map_err(invalid)?;
    Ok(frame)
}

fn result_attributes(root: &mut Group, provenance: &Provenance) -> Result<(), StoreError> {
    for name in ["reference_time", "engine_sha256", "snapshot_sha256"] {
        let value = attr(&provenance.processing, name)?;
        if name == "reference_time" {
            let time =
                NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M:%S").map_err(format_error)?;
            if time.format("%Y-%m-%dT%H:%M:%S").to_string() != value {
                return Err(invalid("invalid result reference time"));
            }
        } else if value.len() != 64
            || !value
                .bytes()
                .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
        {
            return Err(invalid(format!("invalid {name}")));
        }
        root.attributes[name] = value.into();
    }
    Ok(())
}
fn check_result_attributes(ds: &Dataset, provenance: &Provenance) -> Result<(), StoreError> {
    let mut expected = Group::new("/");
    result_attributes(&mut expected, provenance)?;
    for name in ["reference_time", "engine_sha256", "snapshot_sha256"] {
        if ds.root.attributes.get(name) != expected.attributes.get(name) {
            return Err(StoreError::Integrity(format!("result {name} mismatch")));
        }
    }
    Ok(())
}
fn add_segments(root: &mut Group, segments: &[u8]) -> Result<(), StoreError> {
    add(
        root,
        "time_segment",
        DataType::U8,
        &["time"],
        segments.to_vec(),
        attributes(vec![("flag_meanings", "historical forecast".into())]),
    )
}
fn add_float_result(
    root: &mut Group,
    name: &str,
    dims: &[&str],
    values: &[f64],
    valid: &[bool],
    units: &str,
    meaning: &str,
) -> Result<(), StoreError> {
    add(
        root,
        name,
        DataType::F64,
        dims,
        f64_bytes(
            values
                .iter()
                .zip(valid)
                .map(|(v, m)| if *m { *v } else { f64::NAN }),
        ),
        attributes(vec![
            ("units", units.into()),
            ("_FillValue", "NaN".into()),
            ("sampling", meaning.into()),
        ]),
    )?;
    add(
        root,
        &format!("{name}_valid"),
        DataType::U8,
        dims,
        valid.iter().map(|v| u8::from(*v)).collect(),
        attributes(vec![]),
    )
}
fn add_level_result(
    root: &mut Group,
    name: &str,
    dims: &[&str],
    levels: &[i32],
    valid: &[bool],
) -> Result<(), StoreError> {
    add(
        root,
        name,
        DataType::I32,
        dims,
        levels.iter().flat_map(|v| v.to_le_bytes()).collect(),
        attributes(vec![
            ("units", "cm".into()),
            ("sampling", "instantaneous".into()),
        ]),
    )?;
    add(
        root,
        &format!("{name}_valid"),
        DataType::U8,
        dims,
        valid.iter().map(|v| u8::from(*v)).collect(),
        attributes(vec![]),
    )
}
fn result_mask(ds: &Dataset, name: &str, dims: &[&str]) -> Result<Vec<bool>, StoreError> {
    let bytes = array(ds, name, DataType::U8, dims)?;
    if bytes.iter().any(|v| *v > 1) {
        return Err(StoreError::Integrity(format!("invalid {name} mask")));
    }
    Ok(bytes.into_iter().map(|v| v == 1).collect())
}
fn read_float_result(
    ds: &Dataset,
    name: &str,
    dims: &[&str],
    units: &str,
    meaning: &str,
) -> Result<(Vec<f64>, Vec<bool>), StoreError> {
    let a = ds.array(name).map_err(format_error)?;
    if attr(&a.info.attributes, "units")? != units
        || attr(&a.info.attributes, "_FillValue")? != "NaN"
        || attr(&a.info.attributes, "sampling")? != meaning
    {
        return Err(StoreError::Schema(format!("invalid {name} metadata")));
    }
    let values = floats(array(ds, name, DataType::F64, dims)?)?;
    let valid = result_mask(ds, &format!("{name}_valid"), dims)?;
    if values.len() != valid.len()
        || values
            .iter()
            .zip(&valid)
            .any(|(v, m)| if *m { !v.is_finite() } else { !v.is_nan() })
    {
        return Err(StoreError::Integrity(format!(
            "invalid {name} payload/mask"
        )));
    }
    Ok((values, valid))
}
fn read_level_result(
    ds: &Dataset,
    name: &str,
    dims: &[&str],
) -> Result<(Vec<i32>, Vec<bool>), StoreError> {
    let a = ds.array(name).map_err(format_error)?;
    if attr(&a.info.attributes, "units")? != "cm"
        || attr(&a.info.attributes, "sampling")? != "instantaneous"
    {
        return Err(StoreError::Schema(format!("invalid {name} metadata")));
    }
    let bytes = array(ds, name, DataType::I32, dims)?;
    if !bytes.len().is_multiple_of(4) {
        return Err(StoreError::Schema("invalid level byte length".into()));
    }
    let values: Vec<_> = bytes
        .as_chunks::<4>()
        .0
        .iter()
        .map(|v| i32::from_le_bytes(*v))
        .collect();
    let valid = result_mask(ds, &format!("{name}_valid"), dims)?;
    if values.len() != valid.len() || values.iter().zip(&valid).any(|(v, m)| !*m && *v != 0) {
        return Err(StoreError::Integrity(
            "invalid missing level payload".into(),
        ));
    }
    Ok((values, valid))
}
pub fn write_result_series(path: &Path, results: &ResultSeries) -> Result<(), StoreError> {
    results.validate().map_err(invalid)?;
    let mut root = base(
        &results.timeline,
        "basin_results",
        &results.provenance,
        "mm",
    )?;
    result_attributes(&mut root, &results.provenance)?;
    add_geometry(&mut root, &results.basins)?;
    add_segments(&mut root, &results.time_segment)?;
    add_float_result(
        &mut root,
        "precipitation",
        &["time", "basin"],
        &results.precipitation,
        &results.precipitation_valid,
        "mm",
        "interval_total_(start,end]",
    )?;
    add_float_result(
        &mut root,
        "flow",
        &["time", "basin"],
        &results.flow,
        &results.flow_valid,
        "m3 s-1",
        "instantaneous",
    )?;
    add_level_result(
        &mut root,
        "raw_level",
        &["time", "basin"],
        &results.raw_level_cm,
        &results.raw_level_valid,
    )?;
    write(root, path)
}
pub(crate) fn read_result_timeline(path: &Path) -> Result<IntervalAxis, StoreError> {
    let (dataset, timeline, provenance) = open(path, "basin_results", "mm")?;
    check_result_attributes(&dataset, &provenance)?;
    Ok(timeline)
}
pub fn read_result_series(path: &Path) -> Result<ResultSeries, StoreError> {
    let (ds, timeline, provenance) = open(path, "basin_results", "mm")?;
    check_result_attributes(&ds, &provenance)?;
    let basins = read_geometry(&ds)?;
    let time_segment = array(&ds, "time_segment", DataType::U8, &["time"])?;
    let (precipitation, precipitation_valid) = read_float_result(
        &ds,
        "precipitation",
        &["time", "basin"],
        "mm",
        "interval_total_(start,end]",
    )?;
    let (flow, flow_valid) =
        read_float_result(&ds, "flow", &["time", "basin"], "m3 s-1", "instantaneous")?;
    let (raw_level_cm, raw_level_valid) = read_level_result(&ds, "raw_level", &["time", "basin"])?;
    let results = ResultSeries {
        timeline,
        basins,
        time_segment,
        precipitation,
        precipitation_valid,
        flow,
        flow_valid,
        raw_level_cm,
        raw_level_valid,
        provenance,
    };
    results.validate().map_err(invalid)?;
    Ok(results)
}
pub fn write_station_results(path: &Path, results: &StationResults) -> Result<(), StoreError> {
    results.validate().map_err(invalid)?;
    let mut root = base(
        &results.timeline,
        "station_results",
        &results.provenance,
        "cm",
    )?;
    result_attributes(&mut root, &results.provenance)?;
    let count = results.station_ids.len();
    let width = results
        .station_ids
        .iter()
        .map(String::len)
        .max()
        .unwrap_or(1);
    let size = count
        .checked_mul(width)
        .ok_or_else(|| invalid("station ID shape overflow"))?;
    let mut bytes = vec![0; size];
    for (i, id) in results.station_ids.iter().enumerate() {
        bytes[i * width..i * width + id.len()].copy_from_slice(id.as_bytes());
    }
    root.dimensions.extend([
        dimension("station", count, false),
        dimension("station_strlen", width, false),
    ]);
    add(
        &mut root,
        "station_id",
        DataType::U8,
        &["station", "station_strlen"],
        bytes,
        attributes(vec![("encoding", "UTF-8".into())]),
    )?;
    let lengths = results
        .station_ids
        .iter()
        .map(|id| i64::try_from(id.len()).map_err(|_| invalid("station ID too long")))
        .collect::<Result<Vec<_>, _>>()?;
    add(
        &mut root,
        "station_id_length",
        DataType::I64,
        &["station"],
        i64_bytes(lengths),
        attributes(vec![]),
    )?;
    add(
        &mut root,
        "mini_id",
        DataType::I64,
        &["station"],
        i64_bytes(results.mini_ids.iter().map(|v| v.unwrap_or(0))),
        attributes(vec![]),
    )?;
    add(
        &mut root,
        "mini_valid",
        DataType::U8,
        &["station"],
        results
            .mini_ids
            .iter()
            .map(|v| u8::from(v.is_some()))
            .collect(),
        attributes(vec![]),
    )?;
    add(
        &mut root,
        "offset_m",
        DataType::F64,
        &["station"],
        f64_bytes(results.offsets_m.iter().map(|v| v.unwrap_or(0.))),
        attributes(vec![("units", "m".into())]),
    )?;
    add(
        &mut root,
        "offset_valid",
        DataType::U8,
        &["station"],
        results
            .offsets_m
            .iter()
            .map(|v| u8::from(v.is_some()))
            .collect(),
        attributes(vec![]),
    )?;
    add_segments(&mut root, &results.time_segment)?;
    add_level_result(
        &mut root,
        "level",
        &["time", "station"],
        &results.level_cm,
        &results.level_valid,
    )?;
    write(root, path)
}
pub fn read_station_results(path: &Path) -> Result<StationResults, StoreError> {
    let (ds, timeline, provenance) = open(path, "station_results", "cm")?;
    check_result_attributes(&ds, &provenance)?;
    let ids = ds.array("station_id").map_err(format_error)?;
    if attr(&ids.info.attributes, "encoding")? != "UTF-8" {
        return Err(StoreError::Schema("invalid station ID encoding".into()));
    }
    let bytes = array(
        &ds,
        "station_id",
        DataType::U8,
        &["station", "station_strlen"],
    )?;
    let lengths = ints(array(
        &ds,
        "station_id_length",
        DataType::I64,
        &["station"],
    )?)?;
    let width = ds
        .root
        .dimensions
        .iter()
        .find(|d| d.name == "station_strlen")
        .ok_or_else(|| invalid("missing station string dimension"))?
        .size;
    if width == 0 || lengths.len().checked_mul(width) != Some(bytes.len()) {
        return Err(invalid("invalid station string shape"));
    }
    let mut station_ids = Vec::with_capacity(lengths.len());
    for (i, length) in lengths.iter().enumerate() {
        let length =
            usize::try_from(*length).map_err(|_| invalid("invalid station string length"))?;
        if length == 0
            || length > width
            || bytes[i * width + length..(i + 1) * width]
                .iter()
                .any(|v| *v != 0)
        {
            return Err(invalid("invalid station string payload"));
        }
        station_ids.push(
            String::from_utf8(bytes[i * width..i * width + length].to_vec())
                .map_err(format_error)?,
        );
    }
    let mini = ints(array(&ds, "mini_id", DataType::I64, &["station"])?)?;
    let mini_valid = result_mask(&ds, "mini_valid", &["station"])?;
    let offsets = floats(array(&ds, "offset_m", DataType::F64, &["station"])?)?;
    check_coordinate(&ds, "offset_m", "m")?;
    let offset_valid = result_mask(&ds, "offset_valid", &["station"])?;
    if [
        mini.len(),
        mini_valid.len(),
        offsets.len(),
        offset_valid.len(),
    ]
    .iter()
    .any(|v| *v != station_ids.len())
    {
        return Err(invalid("invalid station mapping shape"));
    }
    let mut mini_ids = Vec::with_capacity(mini.len());
    let mut offsets_m = Vec::with_capacity(mini.len());
    for i in 0..mini.len() {
        if mini_valid[i] != offset_valid[i]
            || !offsets[i].is_finite()
            || (mini_valid[i] && mini[i] <= 0)
            || (!mini_valid[i] && (mini[i] != 0 || offsets[i] != 0.))
        {
            return Err(StoreError::Integrity(
                "station mapping mask/payload mismatch".into(),
            ));
        }
        mini_ids.push(mini_valid[i].then_some(mini[i]));
        offsets_m.push(offset_valid[i].then_some(offsets[i]));
    }
    let time_segment = array(&ds, "time_segment", DataType::U8, &["time"])?;
    let (level_cm, level_valid) = read_level_result(&ds, "level", &["time", "station"])?;
    let results = StationResults {
        timeline,
        time_segment,
        station_ids,
        mini_ids,
        offsets_m,
        level_cm,
        level_valid,
        provenance,
    };
    results.validate().map_err(invalid)?;
    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_same_total_shape_with_mismatched_dimension_lengths() {
        let mut root = Group::new("/");
        root.dimensions = vec![dimension("time", 2, false), dimension("basin", 3, false)];
        let dims = vec![dimension("time", 3, false), dimension("basin", 2, false)];
        root.arrays.push(
            Array::from_bytes(
                ArrayInfo {
                    name: "rain".into(),
                    path: "/rain".into(),
                    data_type: DataType::F64,
                    dimensions: dims,
                    chunks: None,
                    attributes: attributes(vec![]),
                    coordinates: vec![],
                },
                vec![0; 48],
            )
            .unwrap(),
        );
        let ds = Dataset {
            driver: "Memory".into(),
            root,
        };
        assert!(array(&ds, "rain", DataType::F64, &["time", "basin"]).is_err());
    }
}
