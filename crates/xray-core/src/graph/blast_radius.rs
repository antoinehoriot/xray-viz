use crate::graph::types::{XrayGraph, NodeId};
use std::collections::HashSet;

/// Compute transitive downstream dependencies of `start`.
pub fn blast_radius(graph: &XrayGraph, start: &NodeId) -> Vec<NodeId> {
    let mut visited = HashSet::new();
    let mut queue = vec![start.clone()];
    while let Some(current) = queue.pop() {
        if visited.contains(&current) {
            continue;
        }
        visited.insert(current.clone());
        for edge in &graph.edges {
            if &edge.source == &current {
                queue.push(edge.target.clone());
            }
        }
    }
    visited.into_iter().filter(|id| id != start).collect()
}

/// Compute transitive upstream dependents of `start`.
pub fn reverse_deps(graph: &XrayGraph, start: &NodeId) -> Vec<NodeId> {
    let mut visited = HashSet::new();
    let mut queue = vec![start.clone()];
    while let Some(current) = queue.pop() {
        if visited.contains(&current) {
            continue;
        }
        visited.insert(current.clone());
        for edge in &graph.edges {
            if &edge.target == &current {
                queue.push(edge.source.clone());
            }
        }
    }
    visited.into_iter().filter(|id| id != start).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::types::*;

    fn make_graph() -> XrayGraph {
        let mut g = XrayGraph::new("/repo");
        g.nodes.push(Node {
            id: "a".to_string(),
            kind: NodeKind::File,
            label: "a.ts".to_string(),
            path: "a.ts".to_string(),
            language: "typescript".to_string(),
            line_start: None,
            line_end: None,
            loc: None,
            metadata: NodeMetadata { exports: vec![], is_entry_point: false, tags: vec![] },
        });
        g.nodes.push(Node {
            id: "b".to_string(),
            kind: NodeKind::File,
            label: "b.ts".to_string(),
            path: "b.ts".to_string(),
            language: "typescript".to_string(),
            line_start: None,
            line_end: None,
            loc: None,
            metadata: NodeMetadata { exports: vec![], is_entry_point: false, tags: vec![] },
        });
        g.edges.push(Edge {
            id: "a→b:Import".to_string(),
            source: "a".to_string(),
            target: "b".to_string(),
            kind: EdgeKind::Import,
            symbol: None,
            resolved: true,
        });
        g
    }

    #[test]
    fn test_blast_radius() {
        let g = make_graph();
        let deps = blast_radius(&g, &"a".to_string());
        assert!(deps.contains(&"b".to_string()));
    }

    #[test]
    fn test_reverse_deps() {
        let g = make_graph();
        let rev = reverse_deps(&g, &"b".to_string());
        assert!(rev.contains(&"a".to_string()));
    }
}
