use super::{BasinGeometry, Provenance};
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::Path,
};

/// Local area and downstream relationship from MINI.gtp.
#[derive(Clone, Debug, PartialEq)]
pub struct MiniTopology {
    pub mini_id: i64,
    pub downstream: Option<i64>,
    pub area_km2: f64,
}

fn parse_mini(
    content: &str,
    with_topology: bool,
) -> Result<(BasinGeometry, Vec<MiniTopology>), String> {
    let mut lines = content.lines();
    let header: Vec<_> = lines
        .next()
        .ok_or("missing MINI header")?
        .split_whitespace()
        .collect();
    let column = |name| {
        header
            .iter()
            .position(|value| *value == name)
            .ok_or_else(|| format!("missing MINI {name} column"))
    };
    let (mini, x, y) = (column("Mini")?, column("Xcen")?, column("Ycen")?);
    let topology_columns = if with_topology {
        Some((column("MiniJus")?, column("Area_(km2)")?))
    } else {
        None
    };
    let mut geometry = BasinGeometry {
        mini_ids: Vec::new(),
        longitudes: Vec::new(),
        latitudes: Vec::new(),
        source: Provenance {
            source_ids: Vec::new(),
            processing: serde_json::Value::Null,
        },
    };
    let mut topology = Vec::new();
    for line in lines.filter(|line| !line.trim().is_empty()) {
        let fields: Vec<_> = line.split_whitespace().collect();
        let field = |index| fields.get(index).ok_or("short MINI row");
        let mini_id: i64 = field(mini)?.parse().map_err(|_| "invalid MINI ID")?;
        geometry.mini_ids.push(mini_id);
        geometry
            .longitudes
            .push(field(x)?.parse().map_err(|_| "invalid MINI longitude")?);
        geometry
            .latitudes
            .push(field(y)?.parse().map_err(|_| "invalid MINI latitude")?);
        if let Some((downstream, area)) = topology_columns {
            let next: i64 = field(downstream)?.parse().map_err(|_| "invalid MiniJus")?;
            let area_km2: f64 = field(area)?.parse().map_err(|_| "invalid MINI area")?;
            if !area_km2.is_finite() || area_km2 <= 0.0 || next == 0 || next < -1 {
                return Err(format!("invalid topology for MINI {mini_id}"));
            }
            topology.push(MiniTopology {
                mini_id,
                downstream: (next != -1).then_some(next),
                area_km2,
            });
        }
    }
    geometry.validate().map_err(|error| error.to_string())?;
    if with_topology {
        validate_mini_topology(&topology)?;
    }
    Ok((geometry, topology))
}

pub fn read_basin_geometry(path: &Path) -> Result<BasinGeometry, String> {
    let content = fs::read_to_string(path).map_err(|error| error.to_string())?;
    parse_basin_geometry(&content)
}
/// Read MINI centers and topology, using local Area_(km2), never cumulative AreaM.
pub fn read_mini_topology(path: &Path) -> Result<(BasinGeometry, Vec<MiniTopology>), String> {
    let content = fs::read_to_string(path).map_err(|error| error.to_string())?;
    parse_mini(&content, true)
}

pub(crate) fn parse_basin_geometry(content: &str) -> Result<BasinGeometry, String> {
    Ok(parse_mini(content, false)?.0)
}

/// Validate IDs, positive local areas, references and acyclic downstream paths.
pub fn validate_mini_topology(topology: &[MiniTopology]) -> Result<(), String> {
    let mut nodes = HashMap::new();
    for node in topology {
        if node.mini_id <= 0
            || !node.area_km2.is_finite()
            || node.area_km2 <= 0.0
            || node.downstream.is_some_and(|id| id <= 0)
            || nodes.insert(node.mini_id, node.downstream).is_some()
        {
            return Err("invalid or duplicate MINI topology".into());
        }
    }
    if nodes.is_empty() {
        return Err("empty MINI topology".into());
    }
    let mut complete = HashSet::new();
    for node in topology {
        let mut seen = HashSet::new();
        let mut current = Some(node.mini_id);
        while let Some(id) = current {
            if complete.contains(&id) {
                break;
            }
            if !seen.insert(id) {
                return Err(format!("MINI cycle at {id}"));
            }
            current = *nodes
                .get(&id)
                .ok_or_else(|| format!("unknown downstream MINI {id}"))?;
        }
        complete.extend(seen);
    }
    Ok(())
}
