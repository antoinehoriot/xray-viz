use wasm_bindgen::prelude::*;
use xray_core::graph::types::XrayGraph;

/// XrayEngine — WASM-bindgen entry point.
/// Handles computation only: layout, blast radius, search, LOD filtering.
/// Sigma.js v3 handles all WebGL rendering.
#[wasm_bindgen]
pub struct XrayEngine {
    graph: XrayGraph,
    /// Flat [x0, y0, x1, y1, ...] node positions
    positions: Vec<f32>,
}

#[wasm_bindgen]
impl XrayEngine {
    /// Construct from graph JSON string.
    #[wasm_bindgen(constructor)]
    pub fn new(graph_json: &str) -> Result<XrayEngine, JsValue> {
        let graph: XrayGraph = serde_json::from_str(graph_json)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        let n = graph.nodes.len();
        Ok(XrayEngine {
            graph,
            positions: vec![0.0f32; n * 2],
        })
    }

    /// Run Sugiyama hierarchical layout (default).
    /// ~100-500ms for 10K nodes in WASM.
    pub fn layout_hierarchical(&mut self) {
        self.positions = xray_core::layout::sugiyama::layout(&self.graph);
    }

    /// Tick force-directed layout (Fruchterman-Reingold + Barnes-Hut).
    /// Returns true when converged.
    pub fn layout_force_directed(&mut self) -> bool {
        xray_core::layout::force::tick(&self.graph, &mut self.positions)
    }

    /// Returns flat [x0, y0, x1, y1, ...] for direct GPU upload via Sigma.js.
    pub fn get_node_positions(&self) -> Vec<f32> {
        self.positions.clone()
    }

    /// Returns flat [src0, tgt0, src1, tgt1, ...] edge index list.
    pub fn get_edges(&self) -> Vec<u32> {
        let mut out = Vec::with_capacity(self.graph.edges.len() * 2);
        for edge in &self.graph.edges {
            let src = self.graph.nodes.iter().position(|n| n.id == edge.source).unwrap_or(0) as u32;
            let tgt = self.graph.nodes.iter().position(|n| n.id == edge.target).unwrap_or(0) as u32;
            out.push(src);
            out.push(tgt);
        }
        out
    }

    /// Compute transitive downstream dependencies of node at index.
    pub fn blast_radius(&self, node_idx: u32) -> Vec<u32> {
        let Some(node) = self.graph.nodes.get(node_idx as usize) else {
            return vec![];
        };
        let dep_ids = xray_core::graph::blast_radius::blast_radius(&self.graph, &node.id);
        dep_ids.iter()
            .filter_map(|id| self.graph.nodes.iter().position(|n| &n.id == id))
            .map(|i| i as u32)
            .collect()
    }

    /// Compute transitive upstream dependents of node at index.
    pub fn reverse_deps(&self, node_idx: u32) -> Vec<u32> {
        let Some(node) = self.graph.nodes.get(node_idx as usize) else {
            return vec![];
        };
        let rev_ids = xray_core::graph::blast_radius::reverse_deps(&self.graph, &node.id);
        rev_ids.iter()
            .filter_map(|id| self.graph.nodes.iter().position(|n| &n.id == id))
            .map(|i| i as u32)
            .collect()
    }

    /// Search nodes by label/path query. Returns matching node indices.
    pub fn search(&self, query: &str) -> Vec<u32> {
        let q = query.to_lowercase();
        self.graph.nodes.iter()
            .enumerate()
            .filter(|(_, n)| n.label.to_lowercase().contains(&q) || n.path.to_lowercase().contains(&q))
            .map(|(i, _)| i as u32)
            .collect()
    }

    /// LOD viewport filtering: return node indices visible at current zoom.
    /// viewport: [x, y, width, height, zoom]
    pub fn get_visible_nodes(&self, viewport: &[f32]) -> Vec<u32> {
        // M2: implement proper LOD based on zoom level
        // For now return all node indices
        let _ = viewport;
        (0..self.graph.nodes.len() as u32).collect()
    }
}
