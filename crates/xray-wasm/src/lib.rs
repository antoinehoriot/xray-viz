use std::collections::HashMap;
use wasm_bindgen::prelude::*;
use xray_core::graph::types::XrayGraph;
use xray_core::layout::force::ForceState;

/// XrayEngine — WASM-bindgen entry point.
///
/// Handles computation only: layout, blast radius, search, LOD filtering.
/// Sigma.js v3 handles all WebGL rendering.
///
/// `get_node_positions()` returns a `Float32Array`-compatible `Vec<f32>` for
/// direct GPU upload — no JSON serialization overhead.
#[wasm_bindgen]
pub struct XrayEngine {
    graph: XrayGraph,
    /// Flat [x0, y0, x1, y1, ...] node positions (indexed by node Vec order).
    positions: Vec<f32>,
    /// Force simulation state — None when running hierarchical layout.
    force_state: Option<ForceState>,
    /// NodeId → Vec index for O(1) lookups (avoids O(N) linear scans per edge).
    node_idx: HashMap<String, u32>,
}

#[wasm_bindgen]
impl XrayEngine {
    /// Construct from graph JSON string (as produced by `xray scan`).
    #[wasm_bindgen(constructor)]
    pub fn new(graph_json: &str) -> Result<XrayEngine, JsValue> {
        let graph: XrayGraph =
            serde_json::from_str(graph_json).map_err(|e| JsValue::from_str(&e.to_string()))?;
        let n = graph.nodes.len();
        let node_idx = graph
            .nodes
            .iter()
            .enumerate()
            .map(|(i, node)| (node.id.clone(), i as u32))
            .collect();
        Ok(XrayEngine {
            graph,
            positions: vec![0.0f32; n * 2],
            force_state: None,
            node_idx,
        })
    }

    /// Run Sugiyama hierarchical layout (default).
    ///
    /// Assigns layers via longest-path from sources, then minimises crossings
    /// with the barycenter heuristic. Typical time: ~100–500 ms for 10 K nodes.
    pub fn layout_hierarchical(&mut self) {
        self.positions = xray_core::layout::sugiyama::layout(&self.graph);
        self.force_state = None; // reset so next force call restarts from scratch
    }

    /// Tick the force-directed layout (Fruchterman-Reingold).
    ///
    /// Call repeatedly from JS requestAnimationFrame until it returns `true`
    /// (converged). Positions are updated in place each tick.
    pub fn layout_force_directed(&mut self) -> bool {
        let n = self.graph.nodes.len();
        let state = self.force_state.get_or_insert_with(|| ForceState::new(n));
        xray_core::layout::force::tick(&self.graph, &mut self.positions, state)
    }

    /// Returns flat `[x0, y0, x1, y1, ...]` for direct GPU upload via Sigma.js.
    pub fn get_node_positions(&self) -> Vec<f32> {
        self.positions.clone()
    }

    /// Returns flat `[src0, tgt0, src1, tgt1, ...]` edge index pairs for Sigma.
    pub fn get_edges(&self) -> Vec<u32> {
        let mut out = Vec::with_capacity(self.graph.edges.len() * 2);
        for edge in &self.graph.edges {
            let src = self.node_idx.get(&edge.source).copied().unwrap_or(0);
            let tgt = self.node_idx.get(&edge.target).copied().unwrap_or(0);
            out.push(src);
            out.push(tgt);
        }
        out
    }

    /// Compute transitive downstream dependencies of the node at `node_idx`.
    ///
    /// Returns indices of all nodes reachable by following forward edges.
    pub fn blast_radius(&self, node_idx: u32) -> Vec<u32> {
        let Some(node) = self.graph.nodes.get(node_idx as usize) else {
            return vec![];
        };
        let dep_ids = xray_core::graph::blast_radius::blast_radius(&self.graph, &node.id);
        dep_ids
            .iter()
            .filter_map(|id| self.node_idx.get(id).copied())
            .collect()
    }

    /// Compute transitive upstream dependents of the node at `node_idx`.
    ///
    /// Returns indices of all nodes that (transitively) import this node.
    pub fn reverse_deps(&self, node_idx: u32) -> Vec<u32> {
        let Some(node) = self.graph.nodes.get(node_idx as usize) else {
            return vec![];
        };
        let rev_ids = xray_core::graph::blast_radius::reverse_deps(&self.graph, &node.id);
        rev_ids
            .iter()
            .filter_map(|id| self.node_idx.get(id).copied())
            .collect()
    }

    /// Search nodes by label or path. Returns matching node indices (case-insensitive).
    pub fn search(&self, query: &str) -> Vec<u32> {
        let q = query.to_lowercase();
        self.graph
            .nodes
            .iter()
            .enumerate()
            .filter(|(_, n)| {
                n.label.to_lowercase().contains(&q) || n.path.to_lowercase().contains(&q)
            })
            .map(|(i, _)| i as u32)
            .collect()
    }

    /// LOD viewport filtering: return node indices visible at the current zoom level.
    ///
    /// `viewport` must be `[x, y, width, height, zoom]` in graph-space coordinates.
    ///
    /// LOD thresholds (M2, file-level graphs):
    /// - zoom < 0.1 : return at most 500 nodes (entry points first, then sampled)
    /// - zoom 0.1–0.5 : return all non-Function nodes within the viewport
    /// - zoom ≥ 0.5 : return all nodes within the viewport
    ///
    /// If `viewport` is empty or positions are not yet computed, returns all indices.
    pub fn get_visible_nodes(&self, viewport: &[f32]) -> Vec<u32> {
        let n = self.graph.nodes.len();

        if viewport.len() < 5 || self.positions.is_empty() {
            return (0..n as u32).collect();
        }

        let vx = viewport[0];
        let vy = viewport[1];
        let vw = viewport[2];
        let vh = viewport[3];
        let zoom = viewport[4];

        // Collect nodes visible within the viewport bounds
        let mut visible: Vec<u32> = (0..n)
            .filter(|&i| {
                let x = self.positions[i * 2];
                let y = self.positions[i * 2 + 1];
                x >= vx && x <= vx + vw && y >= vy && y <= vy + vh
            })
            .map(|i| i as u32)
            .collect();

        if zoom >= 0.5 {
            // Full detail: return everything in viewport
            return visible;
        }

        if zoom >= 0.1 {
            // Medium zoom: exclude Function-kind nodes (file-level focus)
            use xray_core::graph::types::NodeKind;
            visible.retain(|&i| !matches!(self.graph.nodes[i as usize].kind, NodeKind::Function));
            return visible;
        }

        // Low zoom: cap at 500 nodes, prioritising entry points
        if visible.len() <= 500 {
            return visible;
        }
        let mut result: Vec<u32> = visible
            .iter()
            .filter(|&&i| self.graph.nodes[i as usize].metadata.is_entry_point)
            .copied()
            .collect();
        // Fill remaining capacity with evenly-sampled nodes
        let remaining = 500usize.saturating_sub(result.len());
        if remaining > 0 && !visible.is_empty() {
            let step = (visible.len() / remaining).max(1);
            for (count, &i) in visible.iter().step_by(step).enumerate() {
                if count >= remaining {
                    break;
                }
                if !self.graph.nodes[i as usize].metadata.is_entry_point {
                    result.push(i);
                }
            }
        }
        result
    }
}
