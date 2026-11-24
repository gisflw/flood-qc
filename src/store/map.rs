use serde::Serialize;
use std::path::Path;

use terrio::Geometry;
use terrio::crs::{SpatialRef, Transformer};
use wbvector::Coord;

/// Geographic line and polygon boundaries for a named input layer.
#[derive(Debug)]
pub struct MapLayer {
    pub name: String,
    pub paths: Vec<Vec<(f64, f64)>>,
    pub polygons: Vec<MapPolygon>,
}

/// Read vector boundaries and transform each layer to longitude/latitude WGS 84.
pub fn read_map_layers(path: &Path) -> Result<Vec<MapLayer>, String> {
    let path = path.to_str().ok_or("geometry path is not UTF-8")?;
    let data = terrio::open_vector(path, &Default::default()).map_err(|e| e.to_string())?;
    let destination = SpatialRef::from_epsg(4326).map_err(|e| e.to_string())?;
    let mut layers = Vec::new();
    for layer in data.layers {
        let source = SpatialRef::from_epsg(layer.crs_epsg().ok_or("layer has no EPSG CRS")?)
            .map_err(|e| e.to_string())?;
        let transform = Transformer::new(&source, &destination).map_err(|e| e.to_string())?;
        let mut paths = Vec::new();
        let mut polygons = Vec::new();
        for feature in layer.features {
            let parts = geometry_paths(
                feature.geometry.ok_or("feature has no geometry")?,
                &transform,
            )?;
            paths.extend(parts.paths);
            polygons.extend(parts.polygons);
        }
        layers.push(MapLayer {
            name: layer.name,
            paths,
            polygons,
        });
    }
    Ok(layers)
}

/// One polygon part, with holes kept separate from its exterior.
#[derive(Clone, Debug)]
pub struct MapPolygon {
    pub exterior: Vec<(f64, f64)>,
    pub interiors: Vec<Vec<(f64, f64)>>,
}

#[derive(Default)]
struct GeometryParts {
    paths: Vec<Vec<(f64, f64)>>,
    polygons: Vec<MapPolygon>,
}

fn geometry_paths(geometry: Geometry, transform: &Transformer) -> Result<GeometryParts, String> {
    let mut parts = GeometryParts::default();
    let mut polygon_ranges = Vec::new();
    let mut push = |coords: Vec<Coord>, ring: bool| -> Result<(), String> {
        if coords.len() < if ring { 3 } else { 2 } {
            return Err("empty or short geometry".into());
        }
        if coords.iter().any(|c| !c.x.is_finite() || !c.y.is_finite()) {
            return Err("nonfinite source coordinate".into());
        }
        let points = coords
            .into_iter()
            .map(|c| {
                let (x, y) = transform.transform(c.x, c.y).map_err(|e| e.to_string())?;
                if !x.is_finite()
                    || !y.is_finite()
                    || !(-180.0..=180.0).contains(&x)
                    || !(-90.0..=90.0).contains(&y)
                {
                    return Err("invalid geographic coordinate".into());
                }
                Ok((x, y))
            })
            .collect::<Result<Vec<_>, String>>()?;
        let mut points = points;
        if ring {
            let first = points[0];
            let second = points
                .iter()
                .copied()
                .find(|point| *point != first)
                .ok_or("degenerate polygon ring")?;
            if !points.iter().any(|point| {
                (second.0 - first.0) * (point.1 - first.1)
                    != (second.1 - first.1) * (point.0 - first.0)
            }) {
                return Err("degenerate polygon ring".into());
            }
            // wbvector stores ring closure implicitly.
            points.push(first);
        }
        parts.paths.push(points);
        Ok(())
    };
    match geometry {
        Geometry::LineString(c) => push(c, false)?,
        Geometry::MultiLineString(lines) => {
            for line in lines {
                push(line, false)?;
            }
        }
        Geometry::Polygon {
            exterior,
            interiors,
        } => {
            push(exterior.0, true)?;
            let first = polygon_ranges
                .last()
                .map_or(0, |r: &std::ops::Range<usize>| r.end);
            let count = interiors.len();
            for ring in interiors {
                push(ring.0, true)?;
            }
            polygon_ranges.push(first..first + count + 1);
        }
        Geometry::MultiPolygon(polygons) => {
            for (exterior, interiors) in polygons {
                let first = polygon_ranges
                    .last()
                    .map_or(0, |r: &std::ops::Range<usize>| r.end);
                let count = interiors.len();
                push(exterior.0, true)?;
                for ring in interiors {
                    push(ring.0, true)?;
                }
                polygon_ranges.push(first..first + count + 1);
            }
        }
        _ => return Err("unsupported geometry type".into()),
    }
    for range in polygon_ranges {
        parts.polygons.push(MapPolygon {
            exterior: parts.paths[range.start].clone(),
            interiors: parts.paths[range.start + 1..range.end].to_vec(),
        });
    }
    if parts.paths.is_empty() {
        return Err("empty geometry".into());
    }
    Ok(parts)
}

/// A mini's geographic line or polygon paths, retaining multipart features.
#[derive(Debug)]
pub struct MiniMapFeature {
    pub mini_id: i64,
    pub paths: Vec<Vec<(f64, f64)>>,
    pub polygons: Vec<MapPolygon>,
}

/// Read one configured layer and require its IDs to match the supplied MINI topology.
pub fn read_mini_map_layer(
    path: &Path,
    name: &str,
    topology: &[super::MiniTopology],
) -> Result<Vec<MiniMapFeature>, String> {
    use std::collections::{HashMap, HashSet};
    let path = path.to_str().ok_or("geometry path is not UTF-8")?;
    let data = terrio::open_vector(path, &Default::default()).map_err(|e| e.to_string())?;
    let layer = data
        .layers
        .into_iter()
        .find(|l| l.name == name)
        .ok_or_else(|| format!("missing geometry layer {name}"))?;
    let index = layer
        .schema
        .field_index("mini_id")
        .ok_or("layer missing mini_id")?;
    let source = SpatialRef::from_epsg(layer.crs_epsg().ok_or("layer has no EPSG CRS")?)
        .map_err(|e| e.to_string())?;
    let destination = SpatialRef::from_epsg(4326).map_err(|e| e.to_string())?;
    let transform = Transformer::new(&source, &destination).map_err(|e| e.to_string())?;
    let expected: HashSet<_> = topology.iter().map(|t| t.mini_id).collect();
    if expected.len() != topology.len() || expected.is_empty() {
        return Err("invalid topology IDs".into());
    }
    let mut grouped: HashMap<i64, GeometryParts> = HashMap::new();
    for feature in layer.features {
        let id = match feature.attributes.get(index) {
            Some(wbvector::FieldValue::Integer(id)) if *id > 0 => *id,
            _ => return Err("invalid mini_id".into()),
        };
        if !expected.contains(&id) {
            return Err(format!("unknown geometry MINI {id}"));
        }
        let geometry = feature.geometry.ok_or("feature has no geometry")?;
        match name {
            "mini_segments"
                if !matches!(
                    geometry,
                    Geometry::LineString(_) | Geometry::MultiLineString(_)
                ) =>
            {
                return Err("mini_segments requires line geometry".into());
            }
            "mini_catchments"
                if !matches!(
                    geometry,
                    Geometry::Polygon { .. } | Geometry::MultiPolygon(_)
                ) =>
            {
                return Err("mini_catchments requires polygon geometry".into());
            }
            _ => {}
        }
        let parts = geometry_paths(geometry, &transform)?;
        let entry = grouped.entry(id).or_default();
        entry.paths.extend(parts.paths);
        entry.polygons.extend(parts.polygons);
    }
    if grouped.len() != expected.len() {
        return Err("geometry MINI IDs do not match topology".into());
    }
    let mut result: Vec<_> = grouped
        .into_iter()
        .map(|(mini_id, parts)| MiniMapFeature {
            mini_id,
            paths: parts.paths,
            polygons: parts.polygons,
        })
        .collect();
    result.sort_by_key(|f| f.mini_id);
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::read_map_layers;

    #[test]
    fn polygon_parts_keep_their_own_holes_and_closed_boundaries() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mini_catchments.geojson");
        std::fs::write(&path, r#"{"type":"FeatureCollection","features":[{"type":"Feature","properties":{"mini_id":1},"geometry":{"type":"MultiPolygon","coordinates":[[[[0,0],[4,0],[4,4],[0,4],[0,0]],[[1,1],[3,1],[3,3],[1,3],[1,1]]],[[[5,0],[6,0],[6,1],[5,1],[5,0]]]]}}]}"#).unwrap();
        let layer = read_map_layers(&path).unwrap().remove(0);
        assert_eq!(layer.polygons.len(), 2);
        assert_eq!(layer.polygons[0].interiors.len(), 1);
        assert!(layer.polygons[1].interiors.is_empty());
        assert_eq!(layer.paths.len(), 3);
        for path in &layer.paths {
            assert_eq!(path.first(), path.last());
        }
        let topology = [crate::store::MiniTopology {
            mini_id: 1,
            downstream: None,
            area_km2: 1.,
        }];
        let features = super::read_mini_map_layer(&path, &layer.name, &topology).unwrap();
        assert_eq!(features.len(), 1);
        assert_eq!(features[0].mini_id, 1);
        assert_eq!(features[0].polygons[0].interiors.len(), 1);
        assert!(features[0].polygons[1].interiors.is_empty());
    }
    #[test]
    fn transforms_projected_geometry_and_rejects_unsupported_geometry() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("geometry.geojson");
        let document = r#"{"type":"FeatureCollection","crs":{"type":"name","properties":{"name":"EPSG:31982"}},"features":[{"type":"Feature","properties":{},"geometry":{"type":"LineString","coordinates":[[373760.788471764,6482931.67578882],[373760.788471764,6482931.67578882]]}}]}"#;
        std::fs::write(&path, document).unwrap();
        let layers = read_map_layers(&path).unwrap();
        let (lon, lat) = layers[0].paths[0][0];
        assert!((lon + 52.3333).abs() < 1e-7);
        assert!((lat + 31.7822).abs() < 1e-7);
        std::fs::write(
            &path,
            document
                .replace(
                    "LineString\",\"coordinates\":[[",
                    "Point\",\"coordinates\":[",
                )
                .replace("],[373760.788471764,6482931.67578882]]", "]"),
        )
        .unwrap();
        assert!(read_map_layers(&path).is_err());
    }
}

/// Peak flow and configured threshold class for one MINI's 24-hour forecast window.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct MiniFlowExceedance {
    pub mini_id: i64,
    pub peak_m3s: Option<f64>,
    pub peak_at: Option<chrono::NaiveDateTime>,
    pub hours: u8,
    pub return_period_years: Option<f64>,
    pub status: String,
}

pub fn write_return_period_geojson(
    destination: &Path,
    features: &[MiniMapFeature],
    report: &[MiniFlowExceedance],
    window: &super::TimeInterval,
    time_zone: &str,
) -> Result<(), String> {
    use std::collections::HashMap;
    use wbvector::{
        Geometry, Ring,
        feature::{FieldDef, FieldType, FieldValue, Layer},
    };
    if destination
        .extension()
        .is_none_or(|value| value != "geojson")
    {
        return Err("return-period destination must end in .geojson".into());
    }
    let summary: HashMap<_, _> = report.iter().map(|row| (row.mini_id, row)).collect();
    if summary.len() != report.len() || summary.len() != features.len() {
        return Err("return-period report and geometry MINI counts differ".into());
    }
    let mut layer = Layer::new("return_period").with_crs_epsg(4326);
    for (field, kind) in [
        ("mini_id", FieldType::Integer),
        ("peak_m3s", FieldType::Float),
        ("peak_at", FieldType::Text),
        ("hours", FieldType::Integer),
        ("return_period_years", FieldType::Float),
        ("status", FieldType::Text),
        ("window_start", FieldType::Text),
        ("window_end", FieldType::Text),
        ("time_zone", FieldType::Text),
    ] {
        layer.add_field(FieldDef::new(field, kind));
    }
    let coordinates = |points: &[(f64, f64)]| -> Vec<Coord> {
        points.iter().map(|&(x, y)| Coord::xy(x, y)).collect()
    };
    for feature in features {
        let row = summary
            .get(&feature.mini_id)
            .ok_or("return-period MINI geometry mismatch")?;
        let geometry = if !feature.polygons.is_empty() {
            Geometry::MultiPolygon(
                feature
                    .polygons
                    .iter()
                    .map(|polygon| {
                        (
                            Ring::new(coordinates(&polygon.exterior)),
                            polygon
                                .interiors
                                .iter()
                                .map(|ring| Ring::new(coordinates(ring)))
                                .collect(),
                        )
                    })
                    .collect(),
            )
        } else if !feature.paths.is_empty() {
            Geometry::MultiLineString(feature.paths.iter().map(|line| coordinates(line)).collect())
        } else {
            return Err(format!("MINI {} has no lines or polygons", feature.mini_id));
        };
        layer
            .add_feature(
                Some(geometry),
                &[
                    ("mini_id", row.mini_id.into()),
                    (
                        "peak_m3s",
                        row.peak_m3s.map_or(FieldValue::Null, FieldValue::Float),
                    ),
                    (
                        "peak_at",
                        row.peak_at
                            .map_or(FieldValue::Null, |v| v.to_string().into()),
                    ),
                    ("hours", i64::from(row.hours).into()),
                    (
                        "return_period_years",
                        row.return_period_years
                            .map_or(FieldValue::Null, FieldValue::Float),
                    ),
                    ("status", row.status.clone().into()),
                    ("window_start", window.start.to_string().into()),
                    ("window_end", window.end.to_string().into()),
                    ("time_zone", time_zone.into()),
                ],
            )
            .map_err(|e| e.to_string())?;
    }
    super::write_atomic(destination, wbvector::geojson::to_string(&layer).as_bytes())
}
