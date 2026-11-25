use std::collections::BTreeMap;

use chrono::{DateTime, TimeDelta, Utc};
use reqwest::Url;
use serde::Deserialize;
use terrio::{
    DataType,
    multidim::{self, Array},
};

use crate::store::RegularGridSpec;

const RESOLUTION: f64 = 0.1;

#[derive(Deserialize)]
struct Search {
    feed: Feed,
}

#[derive(Deserialize)]
struct Feed {
    entry: Vec<Entry>,
}

#[derive(Deserialize)]
struct Entry {
    producer_granule_id: String,
    time_start: DateTime<Utc>,
    time_end: DateTime<Utc>,
    links: Vec<Link>,
}

#[derive(Deserialize)]
struct Link {
    href: String,
    rel: Option<String>,
    #[serde(default)]
    inherited: bool,
}

pub(crate) struct ImergGranule {
    pub(crate) id: String,
    pub(crate) url: Url,
}

pub(crate) fn imerg_catalog(
    bytes: &[u8],
    start: DateTime<Utc>,
    end: DateTime<Utc>,
) -> Result<BTreeMap<DateTime<Utc>, ImergGranule>, String> {
    let search: Search = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    let halfway = TimeDelta::minutes(30);
    let mut granules = BTreeMap::new();
    for entry in search.feed.entry {
        if !(start..end).contains(&entry.time_start) {
            continue;
        }
        let id = entry.producer_granule_id;
        if !id.starts_with("3B-HHR-E.MS.MRG.3IMERG.")
            || !id.ends_with(".HDF5")
            || !id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-'))
        {
            return Err(format!("invalid IMERG Early granule ID {id}"));
        }
        let duration = entry.time_end - entry.time_start;
        if duration < halfway - TimeDelta::seconds(1)
            || duration > halfway
            || (entry.time_start - start).num_milliseconds() % halfway.num_milliseconds() != 0
        {
            return Err(format!("invalid IMERG half-hour interval for {id}"));
        }
        let url = entry
            .links
            .iter()
            .filter(|link| {
                !link.inherited
                    && link
                        .rel
                        .as_deref()
                        .is_some_and(|rel| rel.ends_with("/data#"))
            })
            .filter_map(|link| Url::parse(&link.href).ok())
            .find(|url| {
                url.scheme() == "https"
                    && url.host_str() == Some("data.gesdisc.earthdata.nasa.gov")
                    && url.port().is_none()
                    && url.username().is_empty()
                    && url.password().is_none()
                    && url.query().is_none()
                    && url.fragment().is_none()
                    && url.path().starts_with("/data/GPM_L3/GPM_3IMERGHHE.07/")
                    && url.path_segments().and_then(Iterator::last) == Some(id.as_str())
            })
            .ok_or_else(|| format!("no GES DISC download link for {id}"))?;
        if granules
            .insert(entry.time_start, ImergGranule { id, url })
            .is_some()
        {
            return Err(format!(
                "multiple IMERG Early granules at {}",
                entry.time_start
            ));
        }
    }
    let mut at = start;
    while at < end {
        if !granules.contains_key(&at) {
            return Err(format!("IMERG Early has no granule at {at}"));
        }
        at += halfway;
    }
    Ok(granules)
}

fn coordinate(array: &Array, origin: f64, max_size: usize) -> Result<Vec<f64>, String> {
    let [dimension] = array.info.dimensions.as_slice() else {
        return Err("IMERG coordinate must be one-dimensional".into());
    };
    if dimension.size != max_size {
        return Err(format!(
            "expected {max_size} native IMERG {} coordinates",
            array.info.name
        ));
    }
    let buffer = array.read().map_err(|e| e.to_string())?;
    let values = match array.info.data_type {
        DataType::F32 => buffer
            .bytes
            .as_chunks::<4>()
            .0
            .iter()
            .map(|b| f64::from(f32::from_le_bytes(*b)))
            .collect::<Vec<_>>(),
        DataType::F64 => buffer
            .bytes
            .as_chunks::<8>()
            .0
            .iter()
            .map(|b| f64::from_le_bytes(*b))
            .collect(),
        _ => return Err("expected floating-point IMERG coordinates".into()),
    };
    let native: Vec<_> = (0..max_size)
        .map(|i| origin + i as f64 * RESOLUTION)
        .collect();
    if values.len() != max_size
        || values
            .iter()
            .zip(&native)
            .any(|(raw, expected)| !raw.is_finite() || (raw - expected).abs() > 0.00002)
    {
        return Err(format!(
            "unexpected IMERG {} coordinate grid",
            array.info.name
        ));
    }
    Ok(native)
}

fn crop(axis: &[f64], lower: f64, upper: f64) -> std::ops::Range<usize> {
    let margin = RESOLUTION / 2.0;
    axis.partition_point(|&v| v + margin <= lower + 1e-10)
        ..axis.partition_point(|&v| v - margin < upper - 1e-10)
}

pub(crate) struct ImergField {
    pub(crate) grid: RegularGridSpec,
    rates: Vec<Option<f64>>,
}

pub(crate) fn read_imerg_granule(bytes: Vec<u8>, bbox: [f64; 4]) -> Result<ImergField, String> {
    let ds = multidim::open_bytes_owned(bytes).map_err(|e| e.to_string())?;
    let [west, south, east, north] = bbox;
    let latitudes = coordinate(
        &ds.array("/Grid/lat").map_err(|e| e.to_string())?,
        -89.95,
        1800,
    )?;
    let longitudes = coordinate(
        &ds.array("/Grid/lon").map_err(|e| e.to_string())?,
        -179.95,
        3600,
    )?;
    let field = ds.array("/Grid/precipitation").map_err(|e| e.to_string())?;
    let [time, x, y] = field.info.dimensions.as_slice() else {
        return Err("IMERG precipitation must have three dimensions".into());
    };
    if (time.size, x.size, y.size) != (1, longitudes.len(), latitudes.len())
        || field.info.data_type != DataType::F32
    {
        return Err("expected native IMERG float32 precipitation (time, lon, lat)".into());
    }
    let attrs = &field.info.attributes;
    if !attrs
        .get("Units")
        .or_else(|| attrs.get("units"))
        .and_then(serde_json::Value::as_str)
        .is_some_and(|units| {
            matches!(
                units.trim().to_ascii_lowercase().as_str(),
                "mm/hr" | "mm/hour" | "mm h-1" | "mm / hr"
            )
        })
    {
        return Err("expected IMERG precipitation rate units (mm/hr)".into());
    }
    let fill = attrs
        .get("_FillValue")
        .and_then(serde_json::Value::as_f64)
        .map(|v| v as f32);
    if attrs.get("scale_factor").is_some() || attrs.get("add_offset").is_some() {
        return Err("unexpected packed IMERG precipitation rates".into());
    }
    let y_indices = crop(&latitudes, south, north);
    let x_indices = crop(&longitudes, west, east);
    let dy = latitudes.len();
    let buffer = field.read().map_err(|e| e.to_string())?;
    if buffer.bytes.len() != 4 * dy * longitudes.len() {
        return Err("invalid IMERG precipitation payload length".into());
    }
    let rates = y_indices
        .clone()
        .flat_map(|iy| {
            let bytes = &buffer.bytes;
            x_indices.clone().map(move |ix| {
                let start = 4 * (ix * dy + iy);
                let raw = f32::from_le_bytes(bytes[start..start + 4].try_into().unwrap());
                (raw.is_finite() && raw >= 0.0 && Some(raw) != fill).then_some(f64::from(raw))
            })
        })
        .collect();
    let latitudes = latitudes[y_indices].to_vec();
    let longitudes = longitudes[x_indices].to_vec();
    let margin = RESOLUTION / 2.0;
    let grid = RegularGridSpec {
        requested_bbox: bbox,
        resolution_degrees: RESOLUTION,
        effective_bbox: [
            longitudes.first().ok_or("empty IMERG longitude crop")? - margin,
            latitudes.first().ok_or("empty IMERG latitude crop")? - margin,
            longitudes.last().ok_or("empty IMERG longitude crop")? + margin,
            latitudes.last().ok_or("empty IMERG latitude crop")? + margin,
        ],
        latitudes,
        longitudes,
    };
    grid.validate().map_err(|e| e.to_string())?;
    Ok(ImergField { grid, rates })
}

pub(crate) fn imerg_hour_depth(
    first: &ImergField,
    second: &ImergField,
) -> Result<impl Iterator<Item = Option<f64>>, String> {
    if first.grid != second.grid {
        return Err("IMERG half-hour grids disagree".into());
    }
    Ok(first
        .rates
        .iter()
        .copied()
        .zip(second.rates.iter().copied())
        .map(|(a, b)| a.zip(b).map(|(a, b)| a * 0.5 + b * 0.5)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Map, Value};
    use terrio::multidim::{ArrayInfo, Dataset, Dimension, Group};

    #[test]
    fn discovery_requires_both_halves_and_a_trusted_data_link() {
        let start: DateTime<Utc> = "2026-11-22T00:00:00Z".parse().unwrap();
        let end = start + TimeDelta::hours(1);
        let entry = |minute, end_minute, offset| {
            let id = format!(
                "3B-HHR-E.MS.MRG.3IMERG.20261122-S00{minute}00-E00{end_minute}59.{offset}.V07C.HDF5"
            );
            let link = |href: String| {
                BTreeMap::from([
                    (
                        "rel",
                        String::from("http://esipfed.org/ns/fedsearch/1.1/data#"),
                    ),
                    ("href", href),
                ])
            };
            let links = [
                link(format!("https://example.org/{id}")),
                link(format!(
                    "https://data.gesdisc.earthdata.nasa.gov/data/GPM_L3/GPM_3IMERGHHE.07/2026/326/{id}"
                )),
            ];
            Value::Object(Map::from_iter([
                ("producer_granule_id".into(), Value::from(id)),
                (
                    "time_start".into(),
                    Value::from(format!("2026-11-22T00:{minute}:00.000Z")),
                ),
                (
                    "time_end".into(),
                    Value::from(format!("2026-11-22T00:{end_minute}:59.999Z")),
                ),
                ("links".into(), serde_json::to_value(links).unwrap()),
            ]))
        };
        let first = entry("00", "29", "0000");
        let second = entry("30", "59", "0030");
        let catalog = |entries| {
            serde_json::to_vec(&BTreeMap::from([(
                "feed",
                BTreeMap::from([("entry", entries)]),
            )]))
            .unwrap()
        };
        let granules =
            imerg_catalog(&catalog(vec![second.clone(), first.clone()]), start, end).unwrap();
        assert_eq!(
            granules[&start].url.host_str(),
            Some("data.gesdisc.earthdata.nasa.gov")
        );
        let partial = catalog(vec![first.clone()]);
        assert!(imerg_catalog(&partial, start, end).is_err());
        let mut untrusted = first;
        untrusted["links"].as_array_mut().unwrap().pop();
        let unexpected_host = catalog(vec![untrusted, second]);
        assert!(imerg_catalog(&unexpected_host, start, end).is_err());
    }

    fn native_imerg(cells: [f32; 4]) -> Vec<u8> {
        let mut group = Group::new("/Grid");
        for (name, size) in [("time", 1), ("lon", 3600), ("lat", 1800)] {
            group.dimensions.push(Dimension {
                name: name.into(),
                path: format!("/Grid/{name}"),
                size,
                unlimited: false,
                indexing_variable: (name != "time").then(|| format!("/Grid/{name}")),
            });
        }
        let array = |name: &str, dims: Vec<Dimension>, attributes: Value, values: Vec<u8>| {
            Array::from_bytes(
                ArrayInfo {
                    name: name.into(),
                    path: format!("/Grid/{name}"),
                    data_type: DataType::F32,
                    dimensions: dims,
                    chunks: None,
                    attributes,
                    coordinates: vec![],
                },
                values,
            )
            .unwrap()
        };
        for (d, origin) in [(1, -179.95), (2, -89.95)] {
            let axis = &group.dimensions[d];
            group.arrays.push(array(
                &axis.name,
                vec![axis.clone()],
                Value::Object(Map::new()),
                (0..axis.size)
                    .flat_map(|i| ((origin + i as f64 * 0.1) as f32).to_le_bytes())
                    .collect(),
            ));
        }
        let mut rates = vec![0; 4 * 3600 * 1800];
        for (index, value) in [0, 1800, 1, 1801].into_iter().zip(cells) {
            rates[index * 4..index * 4 + 4].copy_from_slice(&value.to_le_bytes());
        }
        group.arrays.push(array(
            "precipitation",
            group.dimensions.clone(),
            Value::Object(Map::from_iter([
                ("Units".into(), Value::from("mm/hr")),
                ("_FillValue".into(), Value::from(-9999.9)),
            ])),
            rates,
        ));
        let mut root = Group::new("/");
        root.groups.push(group);
        Dataset {
            driver: "Memory".into(),
            root,
        }
        .to_netcdf_bytes()
        .unwrap()
    }

    #[test]
    fn native_grid_crop_transposes_and_requires_two_valid_rates() {
        let bbox = [-180.0, -90.0, -179.8, -89.8];
        let first = read_imerg_granule(native_imerg([0.0, 4.0, -9999.9, 16.0]), bbox).unwrap();
        let second = read_imerg_granule(native_imerg([0.0, 8.0, 32.0, -9999.9]), bbox).unwrap();
        assert_eq!(first.grid.latitudes.len(), 2);
        assert_eq!(first.grid.longitudes.len(), 2);
        assert_eq!(first.rates, [Some(0.0), Some(4.0), None, Some(16.0)]);
        assert_eq!(
            imerg_hour_depth(&first, &second)
                .unwrap()
                .collect::<Vec<_>>(),
            [Some(0.0), Some(6.0), None, None]
        );
    }
}
