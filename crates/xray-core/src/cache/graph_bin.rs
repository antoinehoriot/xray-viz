//! Binary graph cache (.xray/graph.bin) using bincode + serde.
//! Serializes XrayGraph for instant re-serve without re-parsing.

#[cfg(not(target_arch = "wasm32"))]
mod native {
    use crate::graph::types::XrayGraph;
    use std::path::Path;

    pub fn save(graph: &XrayGraph, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let encoded =
            bincode::serde::encode_to_vec(graph, bincode::config::standard())?;
        std::fs::write(path, encoded)?;
        Ok(())
    }

    pub fn load(path: &Path) -> Result<Option<XrayGraph>, Box<dyn std::error::Error>> {
        if !path.exists() {
            return Ok(None);
        }
        let data = std::fs::read(path)?;
        let (graph, _) =
            bincode::serde::decode_from_slice(&data, bincode::config::standard())?;
        Ok(Some(graph))
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub use native::{load, save};

#[cfg(test)]
#[cfg(not(target_arch = "wasm32"))]
mod tests {
    use super::*;
    use crate::graph::types::XrayGraph;

    #[test]
    fn test_graph_bin_roundtrip() {
        let dir = std::env::temp_dir();
        let path = dir.join("xray_test_graph_bin.bin");
        let mut graph = XrayGraph::new("/repo");
        graph.stats.file_count = 42;
        save(&graph, &path).unwrap();
        let loaded = load(&path).unwrap().unwrap();
        assert_eq!(loaded.root, "/repo");
        assert_eq!(loaded.stats.file_count, 42);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_graph_bin_missing_returns_none() {
        let path = std::path::Path::new("/tmp/xray_nonexistent_12345.bin");
        let result = load(path).unwrap();
        assert!(result.is_none());
    }
}
