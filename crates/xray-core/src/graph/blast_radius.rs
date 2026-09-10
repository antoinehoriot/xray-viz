use crate::graph::types::{NodeId, XrayGraph};
use std::collections::{HashMap, HashSet, VecDeque};

/// Build a forward adjacency map: source NodeId → Vec of target NodeIds.
fn forward_map(graph: &XrayGraph) -> HashMap<&str, Vec<&str>> {
    let mut map: HashMap<&str, Vec<&str>> = HashMap::new();
    for edge in &graph.edges {
        map.entry(edge.source.as_str())
            .or_default()
            .push(edge.target.as_str());
    }
    map
}

/// Build a reverse adjacency map: target NodeId → Vec of source NodeIds.
fn reverse_map(graph: &XrayGraph) -> HashMap<&str, Vec<&str>> {
    let mut map: HashMap<&str, Vec<&str>> = HashMap::new();
    for edge in &graph.edges {
        map.entry(edge.target.as_str())
            .or_default()
            .push(edge.source.as_str());
    }
    map
}

/// BFS traversal over an adjacency map starting from `start`.
/// Returns all reachable node IDs except `start` itself.
fn bfs<'a>(adj: &HashMap<&'a str, Vec<&'a str>>, start: &str) -> Vec<NodeId> {
    let mut visited: HashSet<&str> = HashSet::new();
    let mut queue: VecDeque<&str> = VecDeque::new();
    queue.push_back(start);
    visited.insert(start);

    while let Some(current) = queue.pop_front() {
        if let Some(neighbors) = adj.get(current) {
            for &neighbor in neighbors {
                if visited.insert(neighbor) {
                    queue.push_back(neighbor);
                }
            }
        }
    }

    visited
        .into_iter()
        .filter(|&id| id != start)
        .map(str::to_owned)
        .collect()
}

/// Compute transitive downstream dependencies of `start` (BFS over forward edges).
pub fn blast_radius(graph: &XrayGraph, start: &NodeId) -> Vec<NodeId> {
    let adj = forward_map(graph);
    bfs(&adj, start)
}

/// Compute transitive upstream dependents of `start` (BFS over reverse edges).
pub fn reverse_deps(graph: &XrayGraph, start: &NodeId) -> Vec<NodeId> {
    let adj = reverse_map(graph);
    bfs(&adj, start)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::types::*;

    fn edge(src: &str, tgt: &str) -> Edge {
        Edge {
            id: format!("{}→{}:Import", src, tgt),
            source: src.to_string(),
            target: tgt.to_string(),
            kind: EdgeKind::Import,
            symbol: None,
            resolved: true,
        }
    }

    fn file_node(id: &str) -> Node {
        Node {
            id: id.to_string(),
            kind: NodeKind::File,
            label: format!("{id}.ts"),
            path: format!("{id}.ts"),
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

    fn make_graph(node_ids: &[&str], edges: Vec<Edge>) -> XrayGraph {
        let mut g = XrayGraph::new("/repo");
        g.nodes = node_ids.iter().map(|&id| file_node(id)).collect();
        g.edges = edges;
        g
    }

    #[test]
    fn test_blast_radius_direct() {
        // a → b
        let g = make_graph(&["a", "b"], vec![edge("a", "b")]);
        let deps = blast_radius(&g, &"a".to_string());
        assert!(deps.contains(&"b".to_string()));
        assert!(!deps.contains(&"a".to_string()));
    }

    #[test]
    fn test_blast_radius_transitive() {
        // a → b → c
        let g = make_graph(&["a", "b", "c"], vec![edge("a", "b"), edge("b", "c")]);
        let deps = blast_radius(&g, &"a".to_string());
        assert!(deps.contains(&"b".to_string()));
        assert!(deps.contains(&"c".to_string()));
    }

    #[test]
    fn test_blast_radius_cycle() {
        // a → b → a (cycle — must not infinite loop)
        let g = make_graph(&["a", "b"], vec![edge("a", "b"), edge("b", "a")]);
        let deps = blast_radius(&g, &"a".to_string());
        assert!(deps.contains(&"b".to_string()));
    }

    #[test]
    fn test_reverse_deps() {
        // a → b
        let g = make_graph(&["a", "b"], vec![edge("a", "b")]);
        let rev = reverse_deps(&g, &"b".to_string());
        assert!(rev.contains(&"a".to_string()));
        assert!(!rev.contains(&"b".to_string()));
    }

    #[test]
    fn test_reverse_deps_transitive() {
        // a → b → c; reverse of c gives {a, b}
        let g = make_graph(&["a", "b", "c"], vec![edge("a", "b"), edge("b", "c")]);
        let rev = reverse_deps(&g, &"c".to_string());
        assert!(rev.contains(&"a".to_string()));
        assert!(rev.contains(&"b".to_string()));
    }

    #[test]
    fn test_isolated_node() {
        let g = make_graph(&["a"], vec![]);
        let deps = blast_radius(&g, &"a".to_string());
        assert!(deps.is_empty());
    }
}
