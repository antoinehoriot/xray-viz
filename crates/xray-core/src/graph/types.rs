use serde::{Deserialize, Serialize};

/// Stable node identifier (blake3 hash of canonical path, hex, first 16 chars)
pub type NodeId = String;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub enum NodeKind {
    File,
    Module,
    Function,
    Class,
    Interface,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub enum EdgeKind {
    Import,
    DynamicImport,
    Call,
    Inheritance,
    ReExport,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeMetadata {
    pub exports: Vec<String>,
    pub is_entry_point: bool,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub id: NodeId,
    pub kind: NodeKind,
    pub label: String,
    pub path: String,
    pub language: String,
    pub line_start: Option<u32>,
    pub line_end: Option<u32>,
    pub loc: Option<u32>,
    pub metadata: NodeMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    pub id: String,
    pub source: NodeId,
    pub target: NodeId,
    pub kind: EdgeKind,
    pub symbol: Option<String>,
    pub resolved: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GraphStats {
    pub file_count: u64,
    pub function_count: u64,
    pub edge_count: u64,
    pub parse_duration_ms: u64,
    pub cache_hits: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XrayGraph {
    pub version: u8,
    pub root: String,
    pub scanned_at: String,
    pub languages: Vec<String>,
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    pub stats: GraphStats,
}

impl XrayGraph {
    pub fn new(root: impl Into<String>) -> Self {
        Self {
            version: 1,
            root: root.into(),
            scanned_at: String::new(),
            languages: Vec::new(),
            nodes: Vec::new(),
            edges: Vec::new(),
            stats: GraphStats::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_xray_graph_new() {
        let g = XrayGraph::new("/path/to/repo");
        assert_eq!(g.version, 1);
        assert_eq!(g.root, "/path/to/repo");
        assert!(g.nodes.is_empty());
        assert!(g.edges.is_empty());
    }

    #[test]
    fn test_serialization_round_trip() {
        let g = XrayGraph::new("/repo");
        let json = serde_json::to_string(&g).expect("serialize");
        let g2: XrayGraph = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(g.root, g2.root);
        assert_eq!(g.version, g2.version);
    }
}
