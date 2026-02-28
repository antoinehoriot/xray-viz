use crate::graph::types::XrayGraph;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::types::XrayGraph;

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
}
