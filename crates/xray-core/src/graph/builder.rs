use std::collections::HashMap;

use petgraph::stable_graph::{NodeIndex, StableGraph};
use petgraph::Directed;

use crate::graph::types::{Edge, EdgeKind, GraphStats, Node, NodeId, NodeKind, NodeMetadata, XrayGraph};
use crate::scanner::{FileAst, ImportKind};

/// Generate a stable NodeId from a file path: blake3 hash, first 16 hex chars.
/// Gated on native because blake3 is a native-only dep.
#[cfg(not(target_arch = "wasm32"))]
pub fn node_id_from_path(path: &str) -> NodeId {
    let hash = blake3::hash(path.as_bytes());
    hash.to_hex()[..16].to_string()
}

/// Heuristic entry-point detection by filename.
fn is_entry_point(path: &str) -> bool {
    let name = std::path::Path::new(path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("");
    matches!(
        name,
        "main.rs"
            | "lib.rs"
            | "mod.rs"
            | "index.ts"
            | "index.tsx"
            | "index.js"
            | "index.jsx"
            | "main.ts"
            | "main.js"
            | "main.py"
            | "__main__.py"
            | "app.ts"
            | "app.js"
    )
}

/// UTC timestamp as ISO 8601 string, computed from SystemTime without external deps.
fn utc_now_iso8601() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let s = secs % 60;
    let m = (secs / 60) % 60;
    let h = (secs / 3600) % 24;
    let days = secs / 86400;
    let (year, month, day) = days_to_ymd(days);
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year, month, day, h, m, s
    )
}

/// Convert days-since-Unix-epoch to (year, month, day).
/// Uses the algorithm from https://howardhinnant.github.io/date_algorithms.html
fn days_to_ymd(days: u64) -> (u32, u32, u32) {
    let z = days as i64 + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y as u32, m as u32, d as u32)
}

/// Builds an XrayGraph using a petgraph StableGraph internally.
pub struct GraphBuilder {
    root: String,
}

impl GraphBuilder {
    pub fn new(root: impl Into<String>) -> Self {
        Self { root: root.into() }
    }

    /// Build XrayGraph from FileAsts produced by the scanner.
    /// Uses petgraph::StableGraph for deduplication and internal graph ops,
    /// then exports to the flat XrayGraph format for serialization.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn build_from_asts(
        self,
        asts: Vec<FileAst>,
        parse_duration_ms: u64,
        cache_hits: u64,
    ) -> XrayGraph {
        let mut inner: StableGraph<Node, (), Directed> = StableGraph::new();
        let mut node_map: HashMap<NodeId, NodeIndex> = HashMap::new();

        // First pass: add all file nodes
        for ast in &asts {
            let node_id = node_id_from_path(&ast.path);
            if node_map.contains_key(&node_id) {
                continue;
            }
            let label = std::path::Path::new(&ast.path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(&ast.path)
                .to_string();
            let exports: Vec<String> = ast.exports.iter().map(|e| e.symbol.clone()).collect();
            let node = Node {
                id: node_id.clone(),
                kind: NodeKind::File,
                label,
                path: ast.path.clone(),
                language: ast.language.clone(),
                line_start: None,
                line_end: None,
                loc: None,
                metadata: NodeMetadata {
                    exports,
                    is_entry_point: is_entry_point(&ast.path),
                    tags: vec![],
                },
            };
            let idx = inner.add_node(node);
            node_map.insert(node_id, idx);
        }

        // Second pass: build edges from resolved imports
        let mut edges: Vec<Edge> = Vec::new();
        for ast in &asts {
            let source_id = node_id_from_path(&ast.path);
            for import in &ast.imports {
                let kind = match import.kind {
                    ImportKind::Static => EdgeKind::Import,
                    ImportKind::Dynamic => EdgeKind::DynamicImport,
                    ImportKind::Reexport => EdgeKind::ReExport,
                };
                let kind_str = format!("{:?}", kind);
                let symbol = import.symbols.first().cloned();

                if let Some(resolved) = &import.resolved_path {
                    let target_id = node_id_from_path(resolved);
                    let resolved_in_graph = node_map.contains_key(&target_id);
                    edges.push(Edge {
                        id: format!("{}→{}:{}", source_id, target_id, kind_str),
                        source: source_id.clone(),
                        target: target_id,
                        kind,
                        symbol,
                        resolved: resolved_in_graph,
                    });
                } else {
                    // Unresolvable import — record with resolved: false
                    let specifier_id = format!("unresolved:{}", import.specifier);
                    edges.push(Edge {
                        id: format!("{}→{}:{}", source_id, import.specifier, kind_str),
                        source: source_id.clone(),
                        target: specifier_id,
                        kind,
                        symbol,
                        resolved: false,
                    });
                }
            }
        }

        // Extract nodes from petgraph (preserves stable ordering)
        let nodes: Vec<Node> = inner.node_weights().cloned().collect();

        let mut languages: Vec<String> = nodes.iter().map(|n| n.language.clone()).collect();
        languages.sort();
        languages.dedup();

        let file_count = nodes
            .iter()
            .filter(|n| matches!(n.kind, NodeKind::File))
            .count() as u64;

        XrayGraph {
            version: 1,
            root: self.root,
            scanned_at: utc_now_iso8601(),
            languages,
            stats: GraphStats {
                file_count,
                function_count: 0,
                edge_count: edges.len() as u64,
                parse_duration_ms,
                cache_hits,
            },
            nodes,
            edges,
        }
    }

    /// Simple build from pre-constructed nodes and edges (for testing / manual use).
    pub fn build(self, nodes: Vec<Node>, edges: Vec<Edge>) -> XrayGraph {
        let file_count = nodes
            .iter()
            .filter(|n| matches!(n.kind, NodeKind::File))
            .count() as u64;
        let edge_count = edges.len() as u64;
        let mut languages: Vec<String> = nodes.iter().map(|n| n.language.clone()).collect();
        languages.sort();
        languages.dedup();
        XrayGraph {
            version: 1,
            root: self.root,
            scanned_at: utc_now_iso8601(),
            languages,
            stats: GraphStats {
                file_count,
                function_count: 0,
                edge_count,
                parse_duration_ms: 0,
                cache_hits: 0,
            },
            nodes,
            edges,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::types::*;

    fn make_node(id: &str, path: &str) -> Node {
        Node {
            id: id.to_string(),
            kind: NodeKind::File,
            label: path.to_string(),
            path: path.to_string(),
            language: "typescript".to_string(),
            line_start: None,
            line_end: None,
            loc: None,
            metadata: NodeMetadata {
                exports: vec![],
                is_entry_point: false,
                tags: vec![],
            },
        }
    }

    #[test]
    fn test_build_basic() {
        let node_a = make_node("a", "a.ts");
        let node_b = make_node("b", "b.ts");
        let edge = Edge {
            id: "a→b:Import".to_string(),
            source: "a".to_string(),
            target: "b".to_string(),
            kind: EdgeKind::Import,
            symbol: None,
            resolved: true,
        };
        let builder = GraphBuilder::new("/repo");
        let graph = builder.build(vec![node_a, node_b], vec![edge]);
        assert_eq!(graph.stats.file_count, 2);
        assert_eq!(graph.stats.edge_count, 1);
        assert_eq!(graph.version, 1);
    }

    #[test]
    fn test_utc_now_iso8601() {
        let ts = utc_now_iso8601();
        assert!(ts.ends_with('Z'));
        assert!(ts.contains('T'));
        assert_eq!(ts.len(), 20); // "YYYY-MM-DDTHH:MM:SSZ"
    }

    #[test]
    fn test_days_to_ymd_epoch() {
        let (y, m, d) = days_to_ymd(0);
        assert_eq!((y, m, d), (1970, 1, 1));
    }

    #[test]
    fn test_is_entry_point() {
        assert!(is_entry_point("src/main.rs"));
        assert!(is_entry_point("src/index.ts"));
        assert!(!is_entry_point("src/utils.ts"));
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn test_node_id_from_path() {
        let id = node_id_from_path("src/index.ts");
        assert_eq!(id.len(), 16);
        // Same path → same ID
        assert_eq!(id, node_id_from_path("src/index.ts"));
        // Different path → different ID
        assert_ne!(id, node_id_from_path("src/other.ts"));
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn test_build_from_asts_empty() {
        let builder = GraphBuilder::new("/repo");
        let graph = builder.build_from_asts(vec![], 0, 0);
        assert_eq!(graph.nodes.len(), 0);
        assert_eq!(graph.edges.len(), 0);
        assert_eq!(graph.version, 1);
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn test_build_from_asts_deduplication() {
        use crate::scanner::{ExportDecl, ExportKind, FileAst};
        // Same file AST added twice — should deduplicate nodes
        let ast = FileAst {
            path: "src/index.ts".to_string(),
            language: "typescript".to_string(),
            imports: vec![],
            exports: vec![ExportDecl {
                symbol: "main".to_string(),
                kind: ExportKind::Named,
                line: 1,
            }],
            functions: vec![],
            classes: vec![],
        };
        let builder = GraphBuilder::new("/repo");
        let graph = builder.build_from_asts(vec![ast.clone(), ast], 0, 0);
        assert_eq!(graph.nodes.len(), 1);
        assert!(graph.nodes[0].metadata.is_entry_point);
        assert_eq!(graph.nodes[0].metadata.exports, vec!["main"]);
    }
}
