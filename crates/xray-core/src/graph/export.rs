use crate::graph::types::XrayGraph;

/// Serialize graph to JSON string.
pub fn to_json(graph: &XrayGraph) -> Result<String, serde_json::Error> {
    serde_json::to_string(graph)
}

/// Pretty-print graph as JSON string.
pub fn to_json_pretty(graph: &XrayGraph) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(graph)
}
