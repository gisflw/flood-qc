use std::path::Path;

use terrio::Geometry;
use terrio::crs::{SpatialRef, Transformer};
use wbvector::Coord;

/// Geographic line and polygon boundaries for a named input layer.
pub struct MapLayer {
    pub name: String,
    pub paths: Vec<Vec<(f64, f64)>>,
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
        for feature in layer.features {
            paths.extend(geometry_paths(
                feature.geometry.ok_or("feature has no geometry")?,
                &transform,
            )?);
        }
        layers.push(MapLayer {
            name: layer.name,
            paths,
        });
    }
    Ok(layers)
}

fn geometry_paths(
    geometry: Geometry,
    transform: &Transformer,
) -> Result<Vec<Vec<(f64, f64)>>, String> {
    let mut paths = Vec::new();
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
        paths.push(points);
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
            for ring in interiors {
                push(ring.0, true)?;
            }
        }
        Geometry::MultiPolygon(polygons) => {
            for (exterior, interiors) in polygons {
                push(exterior.0, true)?;
                for ring in interiors {
                    push(ring.0, true)?;
                }
            }
        }
        _ => return Err("unsupported geometry type".into()),
    }
    if paths.is_empty() {
        return Err("empty geometry".into());
    }
    Ok(paths)
}

/// A mini's geographic line or polygon paths, retaining multipart features.
#[derive(Debug)]
pub struct MiniMapFeature {
    pub mini_id: i64,
    pub paths: Vec<Vec<(f64, f64)>>,
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
    let mut grouped: HashMap<i64, Vec<Vec<(f64, f64)>>> = HashMap::new();
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
        grouped
            .entry(id)
            .or_default()
            .extend(geometry_paths(geometry, &transform)?);
    }
    if grouped.len() != expected.len() {
        return Err("geometry MINI IDs do not match topology".into());
    }
    let mut result: Vec<_> = grouped
        .into_iter()
        .map(|(mini_id, paths)| MiniMapFeature { mini_id, paths })
        .collect();
    result.sort_by_key(|f| f.mini_id);
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::read_map_layers;

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
