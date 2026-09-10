//! Fruchterman-Reingold force-directed layout.
//!
//! Per-tick API: the caller (XrayEngine) stores `ForceState` between animation
//! frames and passes it into each `tick()` call. This keeps xray-core pure and
//! WASM-compatible (no global / thread-local state).

use crate::graph::types::XrayGraph;
use std::collections::HashMap;

const MIN_DIST: f32 = 0.01;

/// Persistent simulation state stored by XrayEngine between ticks.
pub struct ForceState {
    /// Current temperature (controls max displacement per tick; cools each tick).
    pub temperature: f32,
    /// Number of ticks elapsed (0 = before first tick; used to detect first call).
    pub tick: u32,
}

impl ForceState {
    /// Create initial state for a graph with `n` nodes.
    pub fn new(n: usize) -> Self {
        let temperature = if n == 0 {
            1.0
        } else {
            (n as f32).sqrt() * 50.0
        };
        Self {
            temperature,
            tick: 0,
        }
    }
}

/// Place nodes in a sunflower spiral to avoid cold-start clustering.
///
/// Uses the approximate golden angle (~137.5°) for uniform angular distribution.
#[allow(clippy::ptr_arg)]
pub fn init_spiral(n: usize, positions: &mut Vec<f32>) {
    for i in 0..n {
        let theta = i as f32 * 2.3999f32; // golden-angle approx in radians
        let r = (i as f32 + 1.0).sqrt() * 40.0;
        positions[i * 2] = r * theta.cos();
        positions[i * 2 + 1] = r * theta.sin();
    }
}

/// Run one Fruchterman-Reingold simulation tick.
///
/// Mutates `positions` (flat `[x0, y0, x1, y1, ...]`) in place.
/// Mutates `state` to advance temperature and tick counter.
///
/// Returns `true` when the layout has converged (temperature exhausted or
/// total displacement below threshold).
///
/// # First tick
/// When `state.tick == 0`, positions are initialised via [`init_spiral`]
/// before running the first force iteration.
pub fn tick(graph: &XrayGraph, positions: &mut Vec<f32>, state: &mut ForceState) -> bool {
    let n = graph.nodes.len();
    if n == 0 {
        return true;
    }

    // Initialise positions on the very first tick
    if state.tick == 0 {
        init_spiral(n, positions);
    }
    state.tick += 1;

    // Fruchterman-Reingold parameters
    // Area proportional to N so spacing stays constant as graph grows
    let k = 100.0f32; // optimal pair-wise distance
    let k2 = k * k;
    let t = state.temperature;

    let mut dx = vec![0.0f32; n];
    let mut dy = vec![0.0f32; n];

    // --- Repulsive forces (all pairs, O(N²)) ---
    // For N ≤ ~2 000 this is fast enough for interactive use.
    // Barnes-Hut quadtree can replace this in a future optimisation pass.
    for u in 0..n {
        let ux = positions[u * 2];
        let uy = positions[u * 2 + 1];
        for v in (u + 1)..n {
            let vx = positions[v * 2];
            let vy = positions[v * 2 + 1];
            let delta_x = ux - vx;
            let delta_y = uy - vy;
            let dist = (delta_x * delta_x + delta_y * delta_y).sqrt().max(MIN_DIST);
            let force = k2 / dist;
            let fx = delta_x / dist * force;
            let fy = delta_y / dist * force;
            dx[u] += fx;
            dy[u] += fy;
            dx[v] -= fx;
            dy[v] -= fy;
        }
    }

    // Build NodeId → Vec-index map for edge traversal
    let id_to_idx: HashMap<&str, usize> = graph
        .nodes
        .iter()
        .enumerate()
        .map(|(i, node)| (node.id.as_str(), i))
        .collect();

    // --- Attractive forces along edges ---
    for edge in &graph.edges {
        let (Some(&u), Some(&v)) = (
            id_to_idx.get(edge.source.as_str()),
            id_to_idx.get(edge.target.as_str()),
        ) else {
            continue;
        };
        if u == v {
            continue;
        }
        let delta_x = positions[u * 2] - positions[v * 2];
        let delta_y = positions[u * 2 + 1] - positions[v * 2 + 1];
        let dist = (delta_x * delta_x + delta_y * delta_y).sqrt().max(MIN_DIST);
        let force = dist * dist / k;
        let fx = delta_x / dist * force;
        let fy = delta_y / dist * force;
        dx[u] -= fx;
        dy[u] -= fy;
        dx[v] += fx;
        dy[v] += fy;
    }

    // --- Apply displacements, clamped by temperature ---
    let mut total_disp = 0.0f32;
    for i in 0..n {
        let disp = (dx[i] * dx[i] + dy[i] * dy[i]).sqrt().max(MIN_DIST);
        let clamped = disp.min(t);
        positions[i * 2] += dx[i] / disp * clamped;
        positions[i * 2 + 1] += dy[i] / disp * clamped;
        total_disp += clamped;
    }

    // Geometric cooling (5% per tick)
    state.temperature = (t * 0.95).max(0.01);

    // Converged when temperature is exhausted or displacements are negligible
    state.temperature < 0.1 || total_disp < 0.5 * n as f32
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
    fn test_empty_graph_converges_immediately() {
        let g = XrayGraph::new("/repo");
        let mut positions = vec![];
        let mut state = ForceState::new(0);
        assert!(tick(&g, &mut positions, &mut state));
    }

    #[test]
    fn test_single_node_converges() {
        let g = make_graph(&["a"], &[]);
        let mut positions = vec![0.0f32; 2];
        let mut state = ForceState::new(1);
        // Single node: no pair interactions. Converges quickly.
        for _ in 0..300 {
            if tick(&g, &mut positions, &mut state) {
                return;
            }
        }
        panic!("Single-node layout did not converge in 300 ticks");
    }

    #[test]
    fn test_positions_initialised_after_first_tick() {
        let g = make_graph(&["a", "b", "c"], &[("a", "b")]);
        let mut positions = vec![0.0f32; 6];
        let mut state = ForceState::new(3);
        tick(&g, &mut positions, &mut state);
        // After tick 1 the spiral init runs, so positions are no longer all-zero
        assert!(
            !positions.iter().all(|&p| p == 0.0),
            "positions should be initialised after first tick"
        );
    }

    #[test]
    fn test_repulsion_separates_nodes() {
        // Two unconnected nodes: pure repulsion should push them apart
        let g = make_graph(&["a", "b"], &[]);
        let mut positions = vec![0.0f32; 4];
        let mut state = ForceState::new(2);
        for _ in 0..60 {
            tick(&g, &mut positions, &mut state);
        }
        let dx = positions[0] - positions[2];
        let dy = positions[1] - positions[3];
        let dist = (dx * dx + dy * dy).sqrt();
        assert!(dist > 10.0, "nodes should repel: dist={dist}");
    }

    #[test]
    fn test_temperature_decreases_each_tick() {
        let g = make_graph(&["a", "b"], &[("a", "b")]);
        let mut positions = vec![0.0f32; 4];
        let mut state = ForceState::new(2);
        let t0 = state.temperature;
        tick(&g, &mut positions, &mut state);
        assert!(
            state.temperature < t0,
            "temperature should decrease: {t0} → {}",
            state.temperature
        );
    }

    #[test]
    fn test_tick_count_increments() {
        let g = make_graph(&["a"], &[]);
        let mut positions = vec![0.0f32; 2];
        let mut state = ForceState::new(1);
        assert_eq!(state.tick, 0);
        tick(&g, &mut positions, &mut state);
        assert_eq!(state.tick, 1);
        tick(&g, &mut positions, &mut state);
        assert_eq!(state.tick, 2);
    }
}
