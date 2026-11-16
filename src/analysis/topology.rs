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
