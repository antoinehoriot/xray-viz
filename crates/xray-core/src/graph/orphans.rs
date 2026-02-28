use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::graph::types::{EdgeKind, NodeId, NodeKind, XrayGraph};

/// A file node with zero resolved importers — a dead code candidate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrphanFile {
    /// Stable NodeId (blake3 hash of path, 16 hex chars)
    pub id: NodeId,
    /// Relative file path from scan root
    pub path: String,
    /// Programming language
    pub language: String,
    /// True when the file is a recognised entry point (main.rs, index.ts, …)
    /// Entry-point orphans are reported but callers may choose to suppress them.
    pub is_entry_point: bool,
}

/// Detect files with zero resolved importers (in-degree 0 for import edges).
///
/// Only `File` nodes are considered. Function/Class nodes introduced by
/// function-level scanning are skipped.
///
/// Only edges of kind `Import`, `DynamicImport`, or `ReExport` that are
/// `resolved == true` count as "importers". Contains edges are excluded so
/// that file→function containment does not artificially raise a file's in-degree.
///
/// Entry-point files (is_entry_point == true) are included in the result so
/// callers can decide whether to display or suppress them.
pub fn detect_orphans(graph: &XrayGraph) -> Vec<OrphanFile> {
    // Build the set of node IDs that have at least one resolved import-kind in-edge.
    let imported: HashSet<&str> = graph
        .edges
        .iter()
        .filter(|e| {
            e.resolved
                && matches!(
                    e.kind,
                    EdgeKind::Import | EdgeKind::DynamicImport | EdgeKind::ReExport
                )
        })
        .map(|e| e.target.as_str())
        .collect();

    // Every File node not in `imported` is an orphan.
    let mut orphans: Vec<OrphanFile> = graph
        .nodes
        .iter()
        .filter(|n| matches!(n.kind, NodeKind::File) && !imported.contains(n.id.as_str()))
        .map(|n| OrphanFile {
            id: n.id.clone(),
            path: n.path.clone(),
            language: n.language.clone(),
            is_entry_point: n.metadata.is_entry_point,
        })
        .collect();

    // Stable sort by path for deterministic output.
    orphans.sort_by(|a, b| a.path.cmp(&b.path));
    orphans
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::types::*;

    fn file_node(id: &str, path: &str, is_entry_point: bool) -> Node {
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
                is_entry_point,
                tags: vec![],
            },
        }
    }

    fn import_edge(src: &str, tgt: &str) -> Edge {
        Edge {
            id: format!("{}→{}:Import", src, tgt),
            source: src.to_string(),
            target: tgt.to_string(),
            kind: EdgeKind::Import,
            symbol: None,
            resolved: true,
        }
    }

    fn unresolved_edge(src: &str, tgt: &str) -> Edge {
        Edge {
            id: format!("{}→{}:Import", src, tgt),
            source: src.to_string(),
            target: tgt.to_string(),
            kind: EdgeKind::Import,
            symbol: None,
            resolved: false,
        }
    }

    fn contains_edge(src: &str, tgt: &str) -> Edge {
        Edge {
            id: format!("{}→{}:Contains", src, tgt),
            source: src.to_string(),
            target: tgt.to_string(),
            kind: EdgeKind::Contains,
            symbol: None,
            resolved: true,
        }
    }

    fn make_graph(nodes: Vec<Node>, edges: Vec<Edge>) -> XrayGraph {
        let mut g = XrayGraph::new("/repo");
        g.nodes = nodes;
        g.edges = edges;
        g
    }

    #[test]
    fn test_all_orphans_empty_graph() {
        let g = make_graph(vec![], vec![]);
        assert!(detect_orphans(&g).is_empty());
    }

    #[test]
    fn test_isolated_node_is_orphan() {
        // Single file, no edges → orphan
        let g = make_graph(vec![file_node("a", "a.ts", false)], vec![]);
        let orphans = detect_orphans(&g);
        assert_eq!(orphans.len(), 1);
        assert_eq!(orphans[0].id, "a");
    }

    #[test]
    fn test_imported_file_is_not_orphan() {
        // a imports b → b is NOT an orphan; a IS an orphan (nothing imports it)
        let g = make_graph(
            vec![file_node("a", "a.ts", false), file_node("b", "b.ts", false)],
            vec![import_edge("a", "b")],
        );
        let orphans = detect_orphans(&g);
        assert_eq!(orphans.len(), 1);
        assert_eq!(orphans[0].id, "a");
    }

    #[test]
    fn test_unresolved_edge_does_not_lift_orphan() {
        // a -unresolved-> b → b still has in-degree 0 for resolved edges
        let g = make_graph(
            vec![file_node("a", "a.ts", false), file_node("b", "b.ts", false)],
            vec![unresolved_edge("a", "b")],
        );
        let orphans = detect_orphans(&g);
        // Both a and b are orphans (unresolved edges don't count)
        let ids: Vec<&str> = orphans.iter().map(|o| o.id.as_str()).collect();
        assert!(ids.contains(&"a"));
        assert!(ids.contains(&"b"));
    }

    #[test]
    fn test_contains_edge_does_not_lift_orphan() {
        // file→function Contains edge: the file should still be an orphan
        let mut func_node = file_node("fn_a", "fn_a.ts", false);
        func_node.kind = NodeKind::Function;
        let g = make_graph(
            vec![file_node("a", "a.ts", false), func_node],
            vec![contains_edge("a", "fn_a")],
        );
        let orphans = detect_orphans(&g);
        // Only File nodes are checked; "a" is still an orphan
        let ids: Vec<&str> = orphans.iter().map(|o| o.id.as_str()).collect();
        assert!(ids.contains(&"a"));
        // fn_a is not a File node — should not appear as orphan
        assert!(!ids.contains(&"fn_a"));
    }

    #[test]
    fn test_entry_point_included() {
        // Entry points have no importers by design; detect_orphans still reports them
        let g = make_graph(vec![file_node("main", "src/main.rs", true)], vec![]);
        let orphans = detect_orphans(&g);
        assert_eq!(orphans.len(), 1);
        assert!(orphans[0].is_entry_point);
    }

    #[test]
    fn test_diamond_graph() {
        // root → a, root → b, a → leaf, b → leaf
        // Only root is an orphan (nothing imports it)
        let g = make_graph(
            vec![
                file_node("root", "root.ts", false),
                file_node("a", "a.ts", false),
                file_node("b", "b.ts", false),
                file_node("leaf", "leaf.ts", false),
            ],
            vec![
                import_edge("root", "a"),
                import_edge("root", "b"),
                import_edge("a", "leaf"),
                import_edge("b", "leaf"),
            ],
        );
        let orphans = detect_orphans(&g);
        assert_eq!(orphans.len(), 1);
        assert_eq!(orphans[0].id, "root");
    }

    #[test]
    fn test_sorted_by_path() {
        let g = make_graph(
            vec![
                file_node("z", "z.ts", false),
                file_node("a", "a.ts", false),
                file_node("m", "m.ts", false),
            ],
            vec![],
        );
        let orphans = detect_orphans(&g);
        assert_eq!(orphans.len(), 3);
        assert_eq!(orphans[0].path, "a.ts");
        assert_eq!(orphans[1].path, "m.ts");
        assert_eq!(orphans[2].path, "z.ts");
    }

    #[test]
    fn test_dynamic_import_counts() {
        // a -dynamic-> b → b is NOT an orphan
        let g = make_graph(
            vec![file_node("a", "a.ts", false), file_node("b", "b.ts", false)],
            vec![Edge {
                id: "a→b:DynamicImport".to_string(),
                source: "a".to_string(),
                target: "b".to_string(),
                kind: EdgeKind::DynamicImport,
                symbol: None,
                resolved: true,
            }],
        );
        let orphans = detect_orphans(&g);
        assert_eq!(orphans.len(), 1);
        assert_eq!(orphans[0].id, "a");
    }

    #[test]
    fn test_reexport_counts() {
        // a -reexport-> b → b is NOT an orphan
        let g = make_graph(
            vec![file_node("a", "a.ts", false), file_node("b", "b.ts", false)],
            vec![Edge {
                id: "a→b:ReExport".to_string(),
                source: "a".to_string(),
                target: "b".to_string(),
                kind: EdgeKind::ReExport,
                symbol: None,
                resolved: true,
            }],
        );
        let orphans = detect_orphans(&g);
        assert_eq!(orphans.len(), 1);
        assert_eq!(orphans[0].id, "a");
    }
}
