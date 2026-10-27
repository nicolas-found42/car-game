use super::*;
use sha2::{Digest, Sha256};

impl Assembly {
    pub fn validate_visuals(
        &self,
        spec: &crate::roster::CarSpec,
        bytes: &[u8],
    ) -> Result<(), String> {
        if bytes.get(..4) != Some(b"glTF") || bytes.len() < 20 {
            return Err("invalid-prepared-glb".into());
        }
        let length = u32::from_le_bytes(bytes[12..16].try_into().unwrap()) as usize;
        let json = bytes.get(20..20 + length).ok_or("invalid-glb-json")?;
        let document: serde_json::Value =
            serde_json::from_slice(json).map_err(|e| e.to_string())?;
        let nodes = document["nodes"].as_array().ok_or("missing-nodes")?;
        let roots = document["scenes"][0]["nodes"]
            .as_array()
            .ok_or("missing-scene")?;
        let mut scene = BTreeSet::new();
        for root in roots {
            visit(
                nodes,
                root.as_u64().ok_or("invalid-node")? as usize,
                &mut scene,
            )?;
        }
        let hash = format!("{:x}", Sha256::digest(bytes));
        let mut owned = BTreeSet::new();
        for part in &self.parts {
            let binding = &self.definitions[&part.definition].visual;
            if binding.asset != spec.path || binding.scene != 0 || binding.prepared_sha256 != hash {
                return Err("visual-source-mismatch".into());
            }
            bind_nodes(binding, nodes, &scene, &mut owned)?;
        }
        if owned != scene {
            return Err("incomplete-visual-ownership".into());
        }
        Ok(())
    }
}

fn bind_nodes(
    binding: &VisualBinding,
    nodes: &[serde_json::Value],
    scene: &BTreeSet<usize>,
    owned: &mut BTreeSet<usize>,
) -> Result<(), String> {
    for (index, name) in &binding.nodes {
        if !scene.contains(index) || nodes[*index]["name"].as_str() != Some(name) {
            return Err("visual-node-mismatch".into());
        }
        if nodes
            .iter()
            .filter(|node| node["name"].as_str() == Some(name))
            .count()
            != 1
        {
            return Err("ambiguous-visual-node".into());
        }
        let mut subtree = BTreeSet::new();
        visit(nodes, *index, &mut subtree)?;
        for node in subtree {
            if !owned.insert(node) {
                return Err("duplicate-visual-ownership".into());
            }
        }
    }
    Ok(())
}

fn visit(
    nodes: &[serde_json::Value],
    index: usize,
    visited: &mut BTreeSet<usize>,
) -> Result<(), String> {
    let node = nodes.get(index).ok_or("invalid-node")?;
    if !visited.insert(index) {
        return Err("duplicate-scene-node".into());
    }
    if let Some(children) = node["children"].as_array() {
        for child in children {
            visit(
                nodes,
                child.as_u64().ok_or("invalid-node")? as usize,
                visited,
            )?;
        }
    }
    Ok(())
}
