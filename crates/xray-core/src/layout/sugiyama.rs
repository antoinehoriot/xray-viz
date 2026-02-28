//! Sugiyama hierarchical layout (default).
//! Produces a layered DAG layout showing dependency direction.
//! Uses Kahn's longest-path layer assignment + barycenter crossing minimization.

use crate::graph::types::XrayGraph;
use std::collections::{HashMap, VecDeque};

const LAYER_SEP: f32 = 150.0;
const NODE_SEP: f32 = 100.0;

/// Assign each node a layer using Kahn's topological sort (longest-path).
/// Nodes in cycles (never dequeued) get assigned to max_layer + 1.
fn assign_layers(n: usize, adj: &[Vec<usize>], in_degree: &[usize]) -> Vec<i32> {
    let mut layer = vec![-1i32; n];
    let mut deg = in_degree.to_vec();
    let mut queue: VecDeque<usize> = VecDeque::new();

    for i in 0..n {
        if deg[i] == 0 {
            layer[i] = 0;
            queue.push_back(i);
        }
    }

    while let Some(u) = queue.pop_front() {
        for &v in &adj[u] {
            // Longest-path: layer[v] = max(layer[v], layer[u] + 1)
            if layer[v] < layer[u] + 1 {
                layer[v] = layer[u] + 1;
            }
            if deg[v] > 0 {
                deg[v] -= 1;
                if deg[v] == 0 {
                    queue.push_back(v);
                }
            }
        }
    }

    // Fallback: cycle members with no non-cycle predecessor remain at -1
    let max_layer = layer.iter().filter(|&&l| l >= 0).max().copied().unwrap_or(0);
    for l in &mut layer {
        if *l < 0 {
            *l = max_layer + 1;
        }
    }
    layer
}

/// Barycenter crossing minimization: 2*passes sweeps over adjacent layer pairs.
/// Forward sweep: sort layer li by barycenters of predecessors in li-1.
/// Backward sweep: sort layer li by barycenters of successors in li+1.
fn barycenter_sort(
    order: &mut Vec<Vec<usize>>,
    adj: &[Vec<usize>],
    radj: &[Vec<usize>],
    passes: usize,
    n: usize,
) {
    if order.len() < 2 {
        return;
    }

    // node → its layer index
    let mut node_layer = vec![0usize; n];
    for (li, layer) in order.iter().enumerate() {
        for &node in layer {
            node_layer[node] = li;
        }
    }

    // node → its position within its layer (float for averaging)
    let mut pos = vec![0.0f32; n];
    for layer in order.iter() {
        for (i, &node) in layer.iter().enumerate() {
            pos[node] = i as f32;
        }
    }

    let layers = order.len();
    for pass in 0..passes {
        let forward = pass % 2 == 0;
        let range: Vec<usize> = if forward {
            (1..layers).collect()
        } else {
            (0..layers - 1).rev().collect()
        };

        for li in range {
            let neighbor_li = if forward { li - 1 } else { li + 1 };
            let neighbor_adj: &[Vec<usize>] = if forward { radj } else { adj };

            let mut bary: Vec<(usize, f32)> = order[li]
                .iter()
                .map(|&node| {
                    // Collect positions of this node's neighbors in the adjacent layer
                    let nb_pos: Vec<f32> = neighbor_adj[node]
                        .iter()
                        .filter(|&&nb| node_layer[nb] == neighbor_li)
                        .map(|&nb| pos[nb])
                        .collect();
                    let bc = if nb_pos.is_empty() {
                        pos[node]
                    } else {
                        nb_pos.iter().sum::<f32>() / nb_pos.len() as f32
                    };
                    (node, bc)
                })
                .collect();

            bary.sort_by(|a, b| {
                a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal)
            });

            // Update positions for this layer
            for (i, &(node, _)) in bary.iter().enumerate() {
                pos[node] = i as f32;
            }
            order[li] = bary.into_iter().map(|(node, _)| node).collect();
        }
    }
}

/// Compute node positions using Sugiyama hierarchical layout.
///
/// Returns flat `[x0, y0, x1, y1, ...]` positions indexed by node order in
/// `graph.nodes`. Suitable for direct use as Sigma.js node coordinates.
///
/// Handles cycles by assigning cycle nodes to a fallback layer (max + 1).
/// Layer 0 is at the top (sources / entry points), deeper layers below.
pub fn layout(graph: &XrayGraph) -> Vec<f32> {
    let n = graph.nodes.len();
    if n == 0 {
        return vec![];
    }
    if n == 1 {
        return vec![0.0, 0.0];
    }

    // Build index lookup: NodeId → Vec index
    let id_to_idx: HashMap<&str, usize> = graph
        .nodes
        .iter()
        .enumerate()
        .map(|(i, node)| (node.id.as_str(), i))
        .collect();

    // Forward adjacency (adj) and reverse adjacency (radj), plus in-degree
    let mut adj: Vec<Vec<usize>> = vec![vec![]; n];
    let mut radj: Vec<Vec<usize>> = vec![vec![]; n];
    let mut in_degree = vec![0usize; n];

    for edge in &graph.edges {
        let (Some(&src), Some(&tgt)) = (
            id_to_idx.get(edge.source.as_str()),
            id_to_idx.get(edge.target.as_str()),
        ) else {
            continue;
        };
        if src == tgt {
            continue;
        }
        adj[src].push(tgt);
        radj[tgt].push(src);
        in_degree[tgt] += 1;
    }

    // Layer assignment (longest-path from sources)
    let layers_vec = assign_layers(n, &adj, &in_degree);
    let max_layer = layers_vec.iter().copied().max().unwrap_or(0) as usize;

    // Group nodes by layer
    let mut layer_groups: Vec<Vec<usize>> = vec![vec![]; max_layer + 1];
    for (i, &l) in layers_vec.iter().enumerate() {
        layer_groups[l as usize].push(i);
    }

    // Crossing minimization (4 alternating sweeps)
    barycenter_sort(&mut layer_groups, &adj, &radj, 4, n);

    // Coordinate assignment: center each layer horizontally
    let mut positions = vec![0.0f32; n * 2];
    for (layer_idx, group) in layer_groups.iter().enumerate() {
        let y = layer_idx as f32 * LAYER_SEP;
        let x_start = -(group.len() as f32 - 1.0) * NODE_SEP * 0.5;
        for (pos_in_layer, &node_idx) in group.iter().enumerate() {
            positions[node_idx * 2] = x_start + pos_in_layer as f32 * NODE_SEP;
            positions[node_idx * 2 + 1] = y;
        }
    }

    positions
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::types::*;

    fn make_graph(node_ids: &[&str], edge_pairs: &[(&str, &str)]) -> XrayGraph {
        let mut g = XrayGraph::new("/repo");
        g.nodes = node_ids
            .iter()
            .map(|&id| Node {
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
            })
            .collect();
        g.edges = edge_pairs
            .iter()
            .map(|&(src, tgt)| Edge {
                id: format!("{src}→{tgt}:Import"),
                source: src.to_string(),
                target: tgt.to_string(),
                kind: EdgeKind::Import,
                symbol: None,
                resolved: true,
            })
            .collect();
        g
    }

    #[test]
    fn test_empty_graph() {
        let g = XrayGraph::new("/repo");
        let positions = layout(&g);
        assert!(positions.is_empty());
    }

    #[test]
    fn test_single_node() {
        let g = make_graph(&["a"], &[]);
        let positions = layout(&g);
        assert_eq!(positions.len(), 2);
    }

    #[test]
    fn test_linear_chain_layers() {
        // a → b → c: layer[a]=0, layer[b]=1, layer[c]=2
        // y increases with layer, so y(a) < y(b) < y(c)
        let g = make_graph(&["a", "b", "c"], &[("a", "b"), ("b", "c")]);
        let positions = layout(&g);
        assert_eq!(positions.len(), 6);
        let ya = positions[0 * 2 + 1];
        let yb = positions[1 * 2 + 1];
        let yc = positions[2 * 2 + 1];
        assert!(ya < yb, "layer(a)={ya} should be above layer(b)={yb}");
        assert!(yb < yc, "layer(b)={yb} should be above layer(c)={yc}");
    }

    #[test]
    fn test_diamond_topology() {
        // a → b, a → c, b → d, c → d
        let g = make_graph(
            &["a", "b", "c", "d"],
            &[("a", "b"), ("a", "c"), ("b", "d"), ("c", "d")],
        );
        let positions = layout(&g);
        assert_eq!(positions.len(), 8);
        // a at layer 0, b/c at layer 1, d at layer 2
        let ya = positions[0 * 2 + 1];
        let yd = positions[3 * 2 + 1];
        assert!(ya < yd, "a should have smaller y than d");
    }

    #[test]
    fn test_cycle_does_not_panic() {
        // a → b → a (cycle)
        let g = make_graph(&["a", "b"], &[("a", "b"), ("b", "a")]);
        let positions = layout(&g);
        assert_eq!(positions.len(), 4);
    }

    #[test]
    fn test_isolated_nodes() {
        // Two disconnected nodes
        let g = make_graph(&["a", "b"], &[]);
        let positions = layout(&g);
        assert_eq!(positions.len(), 4);
    }

    #[test]
    fn test_positions_count_matches_nodes() {
        let g = make_graph(&["a", "b", "c", "d", "e"], &[("a", "b"), ("b", "c"), ("c", "d")]);
        let positions = layout(&g);
        assert_eq!(positions.len(), g.nodes.len() * 2);
    }
}
