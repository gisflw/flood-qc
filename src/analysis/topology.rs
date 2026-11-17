use crate::store::{MiniTopology, validate_mini_topology};
use std::collections::{HashMap, HashSet};

/// Return all upstream mini IDs including the outlet, in input order.
pub fn upstream_minis(topology: &[MiniTopology], outlet: i64) -> Result<Vec<i64>, String> {
    validate_mini_topology(topology)?;
    let nodes: HashMap<_, _> = topology.iter().map(|v| (v.mini_id, v.downstream)).collect();
    if nodes.len() != topology.len() || !nodes.contains_key(&outlet) {
        return Err("duplicate or unknown MINI ID".into());
    }
    let mut result = Vec::new();
    for node in topology {
        let mut seen = HashSet::new();
        let mut current = Some(node.mini_id);
        while let Some(id) = current {
            if !seen.insert(id) {
                return Err(format!("MINI cycle at {id}"));
            }
            if id == outlet {
                result.push(node.mini_id);
                break;
            }
            current = *nodes
                .get(&id)
                .ok_or_else(|| format!("unknown downstream MINI {id}"))?;
        }
    }
    Ok(result)
}

/// Sum authoritative local areas for an explicit, nonempty, unique mini selection.
pub fn mini_area(topology: &[MiniTopology], mini_ids: &[i64]) -> Result<f64, String> {
    validate_mini_topology(topology)?;
    let selected: HashSet<_> = mini_ids.iter().copied().collect();
    if selected.is_empty() || selected.len() != mini_ids.len() {
        return Err("area requires nonempty unique MINI IDs".into());
    }
    let mut area = 0.0;
    let mut count = 0;
    for mini in topology
        .iter()
        .filter(|mini| selected.contains(&mini.mini_id))
    {
        area += mini.area_km2;
        count += 1;
    }
    if count != selected.len() {
        return Err("unknown MINI in area selection".into());
    }
    if !area.is_finite() {
        return Err("selected MINI area overflow".into());
    }
    Ok(area)
}

#[cfg(test)]
mod tests {
    use super::mini_area;
    use crate::store::MiniTopology;

    #[test]
    fn local_area_rejects_unknown_duplicate_and_overflowing_selections() {
        let mut topology = [
            MiniTopology {
                mini_id: 1,
                downstream: Some(2),
                area_km2: 2.,
            },
            MiniTopology {
                mini_id: 2,
                downstream: None,
                area_km2: 3.,
            },
        ];
        assert_eq!(mini_area(&topology, &[2, 1]).unwrap(), 5.);
        assert_eq!(mini_area(&topology, &[2]).unwrap(), 3.);
        assert!(mini_area(&topology, &[]).is_err());
        assert!(mini_area(&topology, &[1, 1]).is_err());
        assert!(mini_area(&topology, &[3]).is_err());
        for mini in &mut topology {
            mini.area_km2 = f64::MAX;
        }
        assert!(mini_area(&topology, &[1, 2]).is_err());
    }
}
