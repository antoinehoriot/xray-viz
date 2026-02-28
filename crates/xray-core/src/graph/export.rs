use crate::graph::subgraph::XrayExport;
use crate::graph::types::{Node, NodeId, XrayGraph};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Serialize graph to compact JSON string.
pub fn to_json(graph: &XrayGraph) -> Result<String, serde_json::Error> {
    serde_json::to_string(graph)
}

/// Serialize graph to pretty-printed JSON string.
pub fn to_json_pretty(graph: &XrayGraph) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(graph)
}

/// Serialize graph to Graphviz DOT format.
pub fn to_dot(graph: &XrayGraph) -> String {
    let mut out = String::from("digraph xray {\n  rankdir=LR;\n  node [shape=box];\n");
    for node in &graph.nodes {
        out.push_str(&format!(
            "  \"{}\" [label=\"{}\"];\n",
            node.id, node.label
        ));
    }
    for edge in &graph.edges {
        out.push_str(&format!(
            "  \"{}\" -> \"{}\";\n",
            edge.source, edge.target
        ));
    }
    out.push('}');
    out
}

// ─── AI Context Export ──────────────────────────────────────────────────────

/// Top-level AI export JSON payload (SPEC.md §7).
#[derive(Debug, Serialize, Deserialize)]
pub struct AiExportJson {
    pub xray_export_version: u8,
    pub exported_at: String,
    pub root: String,
    pub query: AiExportQuery,
    pub summary: AiExportSummary,
    pub nodes: Vec<Node>,
    pub edges: Vec<crate::graph::types::Edge>,
    pub dependency_order: Vec<NodeId>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AiExportQuery {
    pub from: String,
    pub depth: usize,
    pub direction: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AiExportSummary {
    pub node_count: usize,
    pub edge_count: usize,
    pub languages: Vec<String>,
    pub entry_node: AiEntryNode,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AiEntryNode {
    pub id: String,
    pub path: String,
    pub exports: Vec<String>,
}

/// Serialize an `XrayExport` to AI context JSON format (SPEC.md §7).
pub fn to_ai_json(export: &XrayExport, exported_at: &str) -> Result<String, serde_json::Error> {
    let languages: Vec<String> = {
        let mut set: HashSet<String> = HashSet::new();
        for n in &export.nodes {
            set.insert(n.language.clone());
        }
        let mut v: Vec<String> = set.into_iter().collect();
        v.sort();
        v
    };

    let entry = export.entry_node.as_ref();
    let entry_node = AiEntryNode {
        id: entry.map(|n| n.id.clone()).unwrap_or_default(),
        path: entry.map(|n| n.path.clone()).unwrap_or_default(),
        exports: entry.map(|n| n.metadata.exports.clone()).unwrap_or_default(),
    };

    let payload = AiExportJson {
        xray_export_version: 1,
        exported_at: exported_at.to_string(),
        root: export.root.clone(),
        query: AiExportQuery {
            from: export.query.from.clone(),
            depth: export.query.depth,
            direction: export.query.direction.clone(),
        },
        summary: AiExportSummary {
            node_count: export.nodes.len(),
            edge_count: export.edges.len(),
            languages,
            entry_node,
        },
        nodes: export.nodes.clone(),
        edges: export.edges.clone(),
        dependency_order: export.dependency_order.clone(),
    };

    serde_json::to_string_pretty(&payload)
}

/// Render an `XrayExport` as AI-readable Markdown (SPEC.md §7).
pub fn to_ai_markdown(export: &XrayExport, exported_at: &str) -> String {
    let mut out = String::new();

    // Header
    out.push_str("# Xray Context Export\n\n");
    out.push_str(&format!("**Repository:** {}\n", export.root));
    out.push_str(&format!("**Exported:** {}\n", exported_at));
    out.push_str(&format!(
        "**Starting node:** {} (depth {}, {} directions)\n\n",
        export.query.from, export.query.depth, export.query.direction
    ));

    // Architecture Summary
    let languages: Vec<String> = {
        let mut set: HashSet<String> = HashSet::new();
        for n in &export.nodes {
            set.insert(n.language.clone());
        }
        let mut v: Vec<String> = set.into_iter().collect();
        v.sort();
        v
    };
    let lang_names: Vec<String> = languages
        .iter()
        .map(|l| {
            let mut s = l.clone();
            if let Some(c) = s.get_mut(0..1) {
                c.make_ascii_uppercase();
            }
            s
        })
        .collect();
    out.push_str("## Architecture Summary\n\n");
    out.push_str(&format!(
        "{} files, {} dependencies across {} language{}",
        export.nodes.len(),
        export.edges.len(),
        languages.len(),
        if languages.len() == 1 { "" } else { "s" }
    ));
    if !lang_names.is_empty() {
        out.push_str(&format!(" ({}).\n\n", lang_names.join(", ")));
    } else {
        out.push_str(".\n\n");
    }

    // Entry Node Details
    if let Some(entry) = &export.entry_node {
        out.push_str(&format!("## Entry Node: {}\n\n", entry.path));

        if !entry.metadata.exports.is_empty() {
            out.push_str(&format!(
                "- **Exports:** {}\n",
                entry.metadata.exports.join(", ")
            ));
        }

        // Direct dependencies (entry → others)
        let direct_deps: Vec<&str> = export
            .edges
            .iter()
            .filter(|e| e.source == entry.id)
            .filter_map(|e| {
                export.nodes.iter().find(|n| n.id == e.target).map(|n| n.path.as_str())
            })
            .collect();
        if !direct_deps.is_empty() {
            out.push_str(&format!(
                "- **Direct dependencies:** {}\n",
                direct_deps.join(", ")
            ));
        }

        // Direct dependents (others → entry)
        let direct_deps_on: Vec<&str> = export
            .edges
            .iter()
            .filter(|e| e.target == entry.id)
            .filter_map(|e| {
                export.nodes.iter().find(|n| n.id == e.source).map(|n| n.path.as_str())
            })
            .collect();
        if !direct_deps_on.is_empty() {
            out.push_str(&format!(
                "- **Direct dependents:** {}\n",
                direct_deps_on.join(", ")
            ));
        }
        out.push('\n');

        // Dependency Tree (ASCII art)
        out.push_str("## Dependency Tree\n\n```\n");
        let tree = build_tree(&entry.id, &export.nodes, &export.edges, 0, &mut HashSet::new());
        out.push_str(&tree);
        out.push_str("```\n\n");
    }

    // File Index
    out.push_str("## File Index\n\n");
    out.push_str("| File | Language | Exports |\n");
    out.push_str("|---|---|---|\n");
    for node in &export.nodes {
        let lang = {
            let mut s = node.language.clone();
            if let Some(c) = s.get_mut(0..1) {
                c.make_ascii_uppercase();
            }
            s
        };
        out.push_str(&format!(
            "| {} | {} | {} |\n",
            node.path,
            lang,
            node.metadata.exports.join(", ")
        ));
    }
    out.push('\n');

    // Dependency Matrix
    out.push_str("## Dependency Matrix\n\n");
    out.push_str("| From | To | Kind |\n");
    out.push_str("|---|---|---|\n");
    let node_by_id: HashMap<&str, &str> = export
        .nodes
        .iter()
        .map(|n| (n.id.as_str(), n.path.as_str()))
        .collect();
    for edge in &export.edges {
        let src = node_by_id.get(edge.source.as_str()).copied().unwrap_or(&edge.source);
        let tgt = node_by_id.get(edge.target.as_str()).copied().unwrap_or(&edge.target);
        let kind = format!("{:?}", edge.kind);
        out.push_str(&format!("| {} | {} | {} |\n", src, tgt, kind));
    }

    out
}

/// Recursively build an ASCII dependency tree for a node.
fn build_tree(
    node_id: &str,
    nodes: &[Node],
    edges: &[crate::graph::types::Edge],
    depth: usize,
    visited: &mut HashSet<String>,
) -> String {
    let node = match nodes.iter().find(|n| n.id == node_id) {
        Some(n) => n,
        None => return String::new(),
    };

    if visited.contains(node_id) {
        // Cycle — show path without recursing.
        let prefix = if depth == 0 { "" } else { "    " };
        return format!("{}└── {} (cycle)\n", "│   ".repeat(depth.saturating_sub(1)), node.path)
            .replace(&"│   ".repeat(depth.saturating_sub(1)), prefix);
    }
    visited.insert(node_id.to_string());

    let mut out = String::new();
    if depth == 0 {
        out.push_str(&node.path);
        out.push('\n');
    }

    let children: Vec<&str> = edges
        .iter()
        .filter(|e| e.source == node_id)
        .filter_map(|e| nodes.iter().find(|n| n.id == e.target).map(|n| n.id.as_str()))
        .collect();

    for (i, child_id) in children.iter().enumerate() {
        let is_last = i == children.len() - 1;
        let connector = if is_last { "└── " } else { "├── " };
        let indent = "│   ".repeat(depth);
        let child_node = match nodes.iter().find(|n| n.id == *child_id) {
            Some(n) => n,
            None => continue,
        };
        out.push_str(&format!("{}{}{}\n", indent, connector, child_node.path));

        if !visited.contains(*child_id) {
            let child_tree = build_tree(child_id, nodes, edges, depth + 1, visited);
            out.push_str(&child_tree);
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::subgraph::{ExportQuery, XrayExport};
    use crate::graph::types::{Edge, EdgeKind, Node, NodeKind, NodeMetadata};

    fn make_node(id: &str, path: &str, exports: Vec<&str>) -> Node {
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
                exports: exports.iter().map(|s| s.to_string()).collect(),
                is_entry_point: false,
                tags: vec![],
            },
        }
    }

    fn make_edge(src: &str, tgt: &str) -> Edge {
        Edge {
            id: format!("{}→{}", src, tgt),
            source: src.to_string(),
            target: tgt.to_string(),
            kind: EdgeKind::Import,
            symbol: None,
            resolved: true,
        }
    }

    fn sample_export() -> XrayExport {
        let entry = make_node("a1", "src/auth/index.ts", vec!["login", "logout"]);
        let dep = make_node("b2", "src/db/client.ts", vec!["query"]);
        let edge = make_edge("a1", "b2");
        XrayExport {
            root: "/repo".to_string(),
            query: ExportQuery {
                from: "src/auth/index.ts".to_string(),
                depth: 2,
                direction: "both".to_string(),
            },
            entry_node: Some(entry.clone()),
            nodes: vec![entry, dep],
            edges: vec![edge],
            dependency_order: vec!["a1".to_string(), "b2".to_string()],
        }
    }

    #[test]
    fn test_json_roundtrip() {
        let g = XrayGraph::new("/repo");
        let json = to_json(&g).unwrap();
        let g2: XrayGraph = serde_json::from_str(&json).unwrap();
        assert_eq!(g.root, g2.root);
        assert_eq!(g.version, g2.version);
    }

    #[test]
    fn test_json_pretty_is_multiline() {
        let g = XrayGraph::new("/repo");
        let json = to_json_pretty(&g).unwrap();
        assert!(json.contains('\n'));
    }

    #[test]
    fn test_dot_output() {
        let g = XrayGraph::new("/repo");
        let dot = to_dot(&g);
        assert!(dot.starts_with("digraph xray {"));
        assert!(dot.ends_with('}'));
    }

    #[test]
    fn test_ai_json_has_required_fields() {
        let export = sample_export();
        let json = to_ai_json(&export, "2026-02-28T09:00:00Z").unwrap();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["xray_export_version"], 1);
        assert_eq!(v["exported_at"], "2026-02-28T09:00:00Z");
        assert_eq!(v["root"], "/repo");
        assert_eq!(v["query"]["from"], "src/auth/index.ts");
        assert_eq!(v["query"]["depth"], 2);
        assert_eq!(v["summary"]["node_count"], 2);
        assert_eq!(v["summary"]["edge_count"], 1);
        assert!(v["dependency_order"].is_array());
        assert!(v["nodes"].is_array());
        assert!(v["edges"].is_array());
    }

    #[test]
    fn test_ai_json_entry_node_exports() {
        let export = sample_export();
        let json = to_ai_json(&export, "2026-02-28T09:00:00Z").unwrap();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        let exports = v["summary"]["entry_node"]["exports"].as_array().unwrap();
        assert_eq!(exports.len(), 2);
    }

    #[test]
    fn test_ai_markdown_header() {
        let export = sample_export();
        let md = to_ai_markdown(&export, "2026-02-28T09:00:00Z");
        assert!(md.contains("# Xray Context Export"));
        assert!(md.contains("**Repository:** /repo"));
        assert!(md.contains("**Exported:** 2026-02-28T09:00:00Z"));
    }

    #[test]
    fn test_ai_markdown_file_index() {
        let export = sample_export();
        let md = to_ai_markdown(&export, "2026-02-28T09:00:00Z");
        assert!(md.contains("## File Index"));
        assert!(md.contains("src/auth/index.ts"));
        assert!(md.contains("login, logout"));
    }

    #[test]
    fn test_ai_markdown_dependency_matrix() {
        let export = sample_export();
        let md = to_ai_markdown(&export, "2026-02-28T09:00:00Z");
        assert!(md.contains("## Dependency Matrix"));
        assert!(md.contains("src/auth/index.ts"));
        assert!(md.contains("src/db/client.ts"));
    }
}
