pub mod cache;
pub mod graph;
pub mod layout;
pub mod scanner;

pub use graph::types::{Edge, EdgeKind, GraphStats, Node, NodeKind, NodeMetadata, XrayGraph};

/// Detect language from file extension.
/// Returns None for unsupported extensions.
pub fn detect_language(path: &std::path::Path) -> Option<&'static str> {
    match path.extension()?.to_str()? {
        "ts" | "tsx" | "js" | "jsx" | "mjs" | "cjs" => Some("typescript"),
        "py" => Some("python"),
        "rs" => Some("rust"),
        "go" => Some("go"),
        "java" => Some("java"),
        _ => None,
    }
}
