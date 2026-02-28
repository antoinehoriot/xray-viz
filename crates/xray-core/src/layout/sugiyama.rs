//! Sugiyama hierarchical layout (default).
//! Produces a layered DAG layout showing dependency direction.
//! M2 implementation: full Sugiyama algorithm with crossing minimization.

use crate::graph::types::XrayGraph;

/// Compute node positions using Sugiyama hierarchical layout.
/// Returns flat [x0, y0, x1, y1, ...] positions indexed by node order.
pub fn layout(_graph: &XrayGraph) -> Vec<f32> {
    vec![]
}
