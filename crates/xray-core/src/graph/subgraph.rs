use crate::graph::types::{Edge, Node, NodeId, XrayGraph};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};

/// Output of subgraph extraction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XrayExport {
    pub root: String,
    pub query: ExportQuery,
    pub entry_node: Option<Node>,
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    /// Node IDs in topological order (Kahn's algorithm).
    pub dependency_order: Vec<NodeId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportQuery {
    pub from: String,
    pub depth: usize,
    pub direction: String,
}

/// Find a node in the graph by exact path match or suffix match.
pub fn find_node<'a>(graph: &'a XrayGraph, from: &str) -> Option<&'a Node> {
    // Try exact match first.
    if let Some(n) = graph.nodes.iter().find(|n| n.path == from) {
        return Some(n);
    }
    // Suffix match.
    graph
        .nodes
        .iter()
        .find(|n| n.path.ends_with(from) || n.label == from)
}

/// Extract a subgraph via BFS from the starting node with depth limit.
///
/// - `direction`: "downstream" (follow forward edges), "upstream" (follow reverse edges), "both"
/// - `max_nodes`: cap on total exported nodes
pub fn extract_subgraph(
    graph: &XrayGraph,
    from: &str,
    depth: usize,
    direction: &str,
    max_nodes: usize,
) -> XrayExport {
    let query = ExportQuery {
        from: from.to_string(),
        depth,
        direction: direction.to_string(),
    };

    // Find starting node.
    let start_node = match find_node(graph, from) {
        Some(n) => n,
        None => {
            return XrayExport {
                root: graph.root.clone(),
                query,
                entry_node: None,
                nodes: vec![],
                edges: vec![],
                dependency_order: vec![],
            }
        }
    };
    let start_id = start_node.id.clone();

    // Build adjacency maps.
    let forward: HashMap<&str, Vec<&str>> = {
        let mut m: HashMap<&str, Vec<&str>> = HashMap::new();
        for e in &graph.edges {
            m.entry(e.source.as_str())
                .or_default()
                .push(e.target.as_str());
        }
        m
    };
    let reverse: HashMap<&str, Vec<&str>> = {
        let mut m: HashMap<&str, Vec<&str>> = HashMap::new();
        for e in &graph.edges {
            m.entry(e.target.as_str())
                .or_default()
                .push(e.source.as_str());
        }
        m
    };

    // BFS with depth tracking.
    let mut visited: HashSet<String> = HashSet::new();
    let mut queue: VecDeque<(String, usize)> = VecDeque::new();
    queue.push_back((start_id.clone(), 0));
    visited.insert(start_id.clone());

    while let Some((current, d)) = queue.pop_front() {
        if visited.len() >= max_nodes {
            break;
        }
        if d >= depth {
            continue;
        }

        let neighbors: Vec<&str> = match direction {
            "downstream" => forward.get(current.as_str()).cloned().unwrap_or_default(),
            "upstream" => reverse.get(current.as_str()).cloned().unwrap_or_default(),
            _ => {
                // "both"
                let mut n = forward.get(current.as_str()).cloned().unwrap_or_default();
                n.extend(reverse.get(current.as_str()).cloned().unwrap_or_default());
                n
            }
        };

        for neighbor in neighbors {
            if visited.len() >= max_nodes {
                break;
            }
            if visited.insert(neighbor.to_string()) {
                queue.push_back((neighbor.to_string(), d + 1));
            }
        }
    }

    // Collect nodes and edges for the subgraph.
    let nodes: Vec<Node> = graph
        .nodes
        .iter()
        .filter(|n| visited.contains(&n.id))
        .cloned()
        .collect();

    let edges: Vec<Edge> = graph
        .edges
        .iter()
        .filter(|e| visited.contains(&e.source) && visited.contains(&e.target))
        .cloned()
        .collect();

    let dependency_order = topological_sort(&nodes, &edges);

    XrayExport {
        root: graph.root.clone(),
        query,
        entry_node: Some(start_node.clone()),
        nodes,
        edges,
        dependency_order,
    }
}

/// Kahn's topological sort. Returns node IDs in dependency order.
/// Cycles are handled by falling back to remaining nodes appended at the end.
fn topological_sort(nodes: &[Node], edges: &[Edge]) -> Vec<NodeId> {
    let node_ids: HashSet<&str> = nodes.iter().map(|n| n.id.as_str()).collect();

    // In-degree map.
    let mut in_degree: HashMap<&str, usize> = HashMap::new();
    for n in nodes {
        in_degree.entry(n.id.as_str()).or_insert(0);
    }
    let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();
    for e in edges {
        if node_ids.contains(e.source.as_str()) && node_ids.contains(e.target.as_str()) {
            adj.entry(e.source.as_str())
                .or_default()
                .push(e.target.as_str());
            *in_degree.entry(e.target.as_str()).or_insert(0) += 1;
        }
    }

    let mut queue: VecDeque<&str> = in_degree
        .iter()
        .filter(|(_, &deg)| deg == 0)
        .map(|(&id, _)| id)
        .collect();

    let mut result: Vec<NodeId> = Vec::with_capacity(nodes.len());
    let mut remaining: HashSet<&str> = node_ids.clone();

    while let Some(current) = queue.pop_front() {
        result.push(current.to_string());
        remaining.remove(current);
        if let Some(neighbors) = adj.get(current) {
            for &next in neighbors {
                if let Some(deg) = in_degree.get_mut(next) {
                    *deg = deg.saturating_sub(1);
                    if *deg == 0 {
                        queue.push_back(next);
                    }
                }
            }
        }
    }

    // Append any remaining (cycle nodes) in stable order.
    for n in nodes {
        if remaining.contains(n.id.as_str()) {
            result.push(n.id.clone());
        }
    }

    result
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

    fn make_graph(nodes: Vec<Node>, edges: Vec<Edge>) -> XrayGraph {
        let mut g = XrayGraph::new("/repo");
        g.nodes = nodes;
        g.edges = edges;
        g
    }

    #[test]
    fn test_find_node_exact() {
        let g = make_graph(vec![make_node("a1", "src/auth/index.ts")], vec![]);
        let n = find_node(&g, "src/auth/index.ts");
        assert!(n.is_some());
        assert_eq!(n.unwrap().id, "a1");
    }

    #[test]
    fn test_find_node_suffix() {
        let g = make_graph(vec![make_node("a1", "src/auth/index.ts")], vec![]);
        let n = find_node(&g, "auth/index.ts");
        assert!(n.is_some());
    }

    #[test]
    fn test_find_node_not_found() {
        let g = make_graph(vec![make_node("a1", "src/auth/index.ts")], vec![]);
        let n = find_node(&g, "missing.ts");
        assert!(n.is_none());
    }

    #[test]
    fn test_extract_downstream_depth1() {
        // a → b → c; from=a depth=1 downstream → {a, b}
        let g = make_graph(
            vec![
                make_node("a", "a.ts"),
                make_node("b", "b.ts"),
                make_node("c", "c.ts"),
            ],
            vec![make_edge("a", "b"), make_edge("b", "c")],
        );
        let export = extract_subgraph(&g, "a.ts", 1, "downstream", 200);
        let ids: Vec<&str> = export.nodes.iter().map(|n| n.id.as_str()).collect();
        assert!(ids.contains(&"a"));
        assert!(ids.contains(&"b"));
        assert!(!ids.contains(&"c")); // depth 2, excluded
    }

    #[test]
    fn test_extract_upstream() {
        // a → b; from=b direction=upstream → {a, b}
        let g = make_graph(
            vec![make_node("a", "a.ts"), make_node("b", "b.ts")],
            vec![make_edge("a", "b")],
        );
        let export = extract_subgraph(&g, "b.ts", 1, "upstream", 200);
        let ids: Vec<&str> = export.nodes.iter().map(|n| n.id.as_str()).collect();
        assert!(ids.contains(&"a"));
        assert!(ids.contains(&"b"));
    }

    #[test]
    fn test_extract_both() {
        // a → b → c; from=b both → {a, b, c}
        let g = make_graph(
            vec![
                make_node("a", "a.ts"),
                make_node("b", "b.ts"),
                make_node("c", "c.ts"),
            ],
            vec![make_edge("a", "b"), make_edge("b", "c")],
        );
        let export = extract_subgraph(&g, "b.ts", 2, "both", 200);
        let ids: Vec<&str> = export.nodes.iter().map(|n| n.id.as_str()).collect();
        assert!(ids.contains(&"a"));
        assert!(ids.contains(&"b"));
        assert!(ids.contains(&"c"));
    }

    #[test]
    fn test_max_nodes_cap() {
        let nodes: Vec<Node> = (0..10)
            .map(|i| make_node(&format!("n{i}"), &format!("n{i}.ts")))
            .collect();
        let edges: Vec<Edge> = (0..9)
            .map(|i| make_edge(&format!("n{i}"), &format!("n{}", i + 1)))
            .collect();
        let g = make_graph(nodes, edges);
        let export = extract_subgraph(&g, "n0.ts", 100, "downstream", 3);
        assert!(export.nodes.len() <= 3);
    }

    #[test]
    fn test_not_found_returns_empty() {
        let g = make_graph(vec![], vec![]);
        let export = extract_subgraph(&g, "missing.ts", 2, "both", 200);
        assert!(export.nodes.is_empty());
        assert!(export.entry_node.is_none());
    }

    #[test]
    fn test_dependency_order_linear() {
        // a → b → c; topo order should be [a, b, c]
        let nodes = vec![
            make_node("a", "a.ts"),
            make_node("b", "b.ts"),
            make_node("c", "c.ts"),
        ];
        let edges = vec![make_edge("a", "b"), make_edge("b", "c")];
        let order = topological_sort(&nodes, &edges);
        let a_pos = order.iter().position(|x| x == "a").unwrap();
        let b_pos = order.iter().position(|x| x == "b").unwrap();
        let c_pos = order.iter().position(|x| x == "c").unwrap();
        assert!(a_pos < b_pos);
        assert!(b_pos < c_pos);
    }

    #[test]
    fn test_dependency_order_cycle() {
        // a ↔ b (cycle); should not panic
        let nodes = vec![make_node("a", "a.ts"), make_node("b", "b.ts")];
        let edges = vec![make_edge("a", "b"), make_edge("b", "a")];
        let order = topological_sort(&nodes, &edges);
        assert_eq!(order.len(), 2);
    }

    #[test]
    fn test_edges_included_in_subgraph() {
        // a → b; extract both → edges should contain a→b
        let g = make_graph(
            vec![make_node("a", "a.ts"), make_node("b", "b.ts")],
            vec![make_edge("a", "b")],
        );
        let export = extract_subgraph(&g, "a.ts", 2, "both", 200);
        assert_eq!(export.edges.len(), 1);
        assert_eq!(export.edges[0].source, "a");
        assert_eq!(export.edges[0].target, "b");
    }
}
