use crate::graph::types::{XrayGraph, Node, Edge};

/// Builds an XrayGraph from nodes and edges.
/// In M1, this will use petgraph::StableGraph internally.
pub struct GraphBuilder {
    root: String,
}

impl GraphBuilder {
    pub fn new(root: impl Into<String>) -> Self {
        Self { root: root.into() }
    }

    pub fn build(self, nodes: Vec<Node>, edges: Vec<Edge>) -> XrayGraph {
        let mut graph = XrayGraph::new(self.root);
        graph.stats.file_count = nodes.iter().filter(|n| matches!(n.kind, crate::graph::types::NodeKind::File)).count() as u64;
        graph.stats.edge_count = edges.len() as u64;
        graph.nodes = nodes;
        graph.edges = edges;
        graph
    }
}
