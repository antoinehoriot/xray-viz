use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::graph::types::{NodeId, XrayGraph};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cycle {
    /// Ordered node IDs forming the cycle (e.g., [a, b, c] means a->b->c->a)
    pub nodes: Vec<NodeId>,
}

/// Detect all import cycles in the graph using Tarjan's SCC algorithm.
/// Returns cycles sorted by length (shortest first).
/// Only considers resolved edges (edge.resolved == true).
pub fn detect_cycles(graph: &XrayGraph) -> Vec<Cycle> {
    let node_ids: Vec<&str> = graph.nodes.iter().map(|n| n.id.as_str()).collect();
    let n = node_ids.len();

    if n == 0 {
        return vec![];
    }

    // Map node ID → index for adjacency list
    let mut id_to_idx: HashMap<&str, usize> = HashMap::with_capacity(n);
    for (i, &id) in node_ids.iter().enumerate() {
        id_to_idx.insert(id, i);
    }

    // Build forward adjacency list (resolved edges only, skip self-loops)
    let mut adj: Vec<Vec<usize>> = vec![vec![]; n];
    for edge in &graph.edges {
        if !edge.resolved {
            continue;
        }
        let src = match id_to_idx.get(edge.source.as_str()) {
            Some(&i) => i,
            None => continue,
        };
        let tgt = match id_to_idx.get(edge.target.as_str()) {
            Some(&i) => i,
            None => continue,
        };
        if src != tgt {
            adj[src].push(tgt);
        }
    }

    // Tarjan's SCC state
    let mut index_counter: usize = 0;
    let mut index_map: Vec<Option<usize>> = vec![None; n];
    let mut lowlink: Vec<usize> = vec![0; n];
    let mut on_stack: Vec<bool> = vec![false; n];
    let mut tarjan_stack: Vec<usize> = Vec::new();
    let mut cycles: Vec<Cycle> = Vec::new();

    for start in 0..n {
        if index_map[start].is_none() {
            strongconnect(
                start,
                &adj,
                &mut index_counter,
                &mut index_map,
                &mut lowlink,
                &mut on_stack,
                &mut tarjan_stack,
                &mut cycles,
                &node_ids,
            );
        }
    }

    cycles.sort_by_key(|c| c.nodes.len());
    cycles
}

/// Iterative Tarjan's strongconnect starting at `root`.
#[allow(clippy::too_many_arguments, clippy::ptr_arg)]
fn strongconnect(
    root: usize,
    adj: &[Vec<usize>],
    index_counter: &mut usize,
    index_map: &mut Vec<Option<usize>>,
    lowlink: &mut Vec<usize>,
    on_stack: &mut Vec<bool>,
    tarjan_stack: &mut Vec<usize>,
    cycles: &mut Vec<Cycle>,
    node_ids: &[&str],
) {
    // Work stack: (node_idx, neighbor_iterator_index)
    let mut work: Vec<(usize, usize)> = Vec::new();

    index_map[root] = Some(*index_counter);
    lowlink[root] = *index_counter;
    *index_counter += 1;
    on_stack[root] = true;
    tarjan_stack.push(root);
    work.push((root, 0));

    while !work.is_empty() {
        let (v, ni) = *work.last().unwrap();

        if ni < adj[v].len() {
            let w = adj[v][ni];
            work.last_mut().unwrap().1 += 1;

            if index_map[w].is_none() {
                // Tree edge: descend into w
                index_map[w] = Some(*index_counter);
                lowlink[w] = *index_counter;
                *index_counter += 1;
                on_stack[w] = true;
                tarjan_stack.push(w);
                work.push((w, 0));
            } else if on_stack[w] {
                // Back edge: w is already on the stack
                let w_index = index_map[w].unwrap();
                lowlink[v] = lowlink[v].min(w_index);
            }
        } else {
            // Finished all neighbors of v
            work.pop();

            // Propagate lowlink to parent
            if let Some(&(parent, _)) = work.last() {
                lowlink[parent] = lowlink[parent].min(lowlink[v]);
            }

            // If v is the root of an SCC, pop and record it
            if lowlink[v] == index_map[v].unwrap() {
                let mut scc: Vec<usize> = Vec::new();
                loop {
                    let w = tarjan_stack.pop().unwrap();
                    on_stack[w] = false;
                    scc.push(w);
                    if w == v {
                        break;
                    }
                }

                // Only report SCCs with more than 1 node (actual cycles)
                if scc.len() > 1 {
                    let nodes: Vec<NodeId> = scc.iter().map(|&i| node_ids[i].to_string()).collect();
                    cycles.push(Cycle { nodes });
                }
            }
        }
    }
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
    fn test_no_cycles() {
        // Linear graph: a -> b -> c
        let g = make_graph(&["a", "b", "c"], vec![edge("a", "b"), edge("b", "c")]);
        let cycles = detect_cycles(&g);
        assert!(cycles.is_empty(), "Linear graph should have no cycles");
    }

    #[test]
    fn test_simple_cycle() {
        // a -> b -> a
        let g = make_graph(&["a", "b"], vec![edge("a", "b"), edge("b", "a")]);
        let cycles = detect_cycles(&g);
        assert_eq!(cycles.len(), 1, "Should detect exactly one cycle");
        let nodes = &cycles[0].nodes;
        assert!(
            nodes.contains(&"a".to_string()) && nodes.contains(&"b".to_string()),
            "Cycle should contain both a and b"
        );
    }

    #[test]
    fn test_triangle_cycle() {
        // a -> b -> c -> a
        let g = make_graph(
            &["a", "b", "c"],
            vec![edge("a", "b"), edge("b", "c"), edge("c", "a")],
        );
        let cycles = detect_cycles(&g);
        assert_eq!(cycles.len(), 1, "Should detect exactly one cycle");
        let nodes = &cycles[0].nodes;
        assert_eq!(nodes.len(), 3, "Triangle cycle should have 3 nodes");
        assert!(nodes.contains(&"a".to_string()));
        assert!(nodes.contains(&"b".to_string()));
        assert!(nodes.contains(&"c".to_string()));
    }

    #[test]
    fn test_multiple_cycles() {
        // Two independent cycles: a->b->a and c->d->c
        let g = make_graph(
            &["a", "b", "c", "d"],
            vec![
                edge("a", "b"),
                edge("b", "a"),
                edge("c", "d"),
                edge("d", "c"),
            ],
        );
        let cycles = detect_cycles(&g);
        assert_eq!(cycles.len(), 2, "Should detect two independent cycles");
    }

    #[test]
    fn test_self_loop() {
        // a -> a (self-loop — should be skipped since src == tgt)
        let g = make_graph(&["a"], vec![edge("a", "a")]);
        let cycles = detect_cycles(&g);
        assert!(
            cycles.is_empty(),
            "Self-loop should not be reported as a cycle"
        );
    }

    #[test]
    fn test_unresolved_edges_ignored() {
        // a -unresolved-> b -> a (but the back edge is unresolved)
        let g = make_graph(
            &["a", "b"],
            vec![unresolved_edge("a", "b"), unresolved_edge("b", "a")],
        );
        let cycles = detect_cycles(&g);
        assert!(cycles.is_empty(), "Unresolved edges should not form cycles");
    }

    #[test]
    fn test_sorted_by_length() {
        // Ensure shorter cycles come first
        // Cycle 1: a->b->a (length 2)
        // Cycle 2: c->d->e->c (length 3)
        let g = make_graph(
            &["a", "b", "c", "d", "e"],
            vec![
                edge("a", "b"),
                edge("b", "a"),
                edge("c", "d"),
                edge("d", "e"),
                edge("e", "c"),
            ],
        );
        let cycles = detect_cycles(&g);
        assert_eq!(cycles.len(), 2);
        assert!(
            cycles[0].nodes.len() <= cycles[1].nodes.len(),
            "Cycles should be sorted by length"
        );
    }

    #[test]
    fn test_empty_graph() {
        let g = make_graph(&[], vec![]);
        let cycles = detect_cycles(&g);
        assert!(cycles.is_empty());
    }
}
