//! Graph node annotations: read/write `.xray/annotations.yml`.
//!
//! Annotations attach human-readable metadata (team, domain, status) to graph
//! nodes (identified by their relative file path). The YAML file format is:
//!
//! ```yaml
//! src/main.ts:
//!   team: platform
//!   domain: core
//!   status: stable
//! src/utils/helpers.ts:
//!   domain: shared
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Metadata tags for a single graph node.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct NodeAnnotation {
    /// Owning team (e.g. "platform", "infra", "payments").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub team: Option<String>,

    /// Logical domain (e.g. "core", "auth", "billing").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,

    /// Status label (e.g. "stable", "wip", "deprecated").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
}

/// Map from relative node path to its annotation.
pub type Annotations = HashMap<String, NodeAnnotation>;

/// Load annotations from `<root>/.xray/annotations.yml`.
///
/// Returns an empty map when the file does not exist.
/// Returns an error if the file exists but cannot be parsed.
pub fn load(root: &Path) -> Result<Annotations, Box<dyn std::error::Error>> {
    let path = root.join(".xray").join("annotations.yml");
    if !path.exists() {
        return Ok(HashMap::new());
    }
    let content = std::fs::read_to_string(&path)?;
    let annotations: Annotations = serde_yaml::from_str(&content)?;
    Ok(annotations)
}

/// Persist `annotations` to `<root>/.xray/annotations.yml`.
///
/// Creates the `.xray/` directory if it does not exist.
pub fn save(root: &Path, annotations: &Annotations) -> Result<(), Box<dyn std::error::Error>> {
    let dir = root.join(".xray");
    std::fs::create_dir_all(&dir)?;
    let path = dir.join("annotations.yml");
    let content = serde_yaml::to_string(annotations)?;
    std::fs::write(&path, content)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp_root(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("xray_ann_test_{name}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn load_returns_empty_when_no_file() {
        let root = temp_root("no_file");
        let ann = load(&root).unwrap();
        assert!(ann.is_empty());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn save_and_load_roundtrip() {
        let root = temp_root("roundtrip");
        let mut ann: Annotations = HashMap::new();
        ann.insert(
            "src/main.ts".to_string(),
            NodeAnnotation {
                team: Some("platform".to_string()),
                domain: Some("core".to_string()),
                status: Some("stable".to_string()),
            },
        );
        ann.insert(
            "src/utils.ts".to_string(),
            NodeAnnotation {
                team: None,
                domain: Some("shared".to_string()),
                status: None,
            },
        );
        save(&root, &ann).unwrap();
        let loaded = load(&root).unwrap();
        assert_eq!(loaded.len(), 2);
        let main_ann = loaded.get("src/main.ts").unwrap();
        assert_eq!(main_ann.team.as_deref(), Some("platform"));
        assert_eq!(main_ann.domain.as_deref(), Some("core"));
        assert_eq!(main_ann.status.as_deref(), Some("stable"));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn save_creates_xray_dir_if_missing() {
        let root = temp_root("mkdir");
        // .xray dir should not exist yet
        assert!(!root.join(".xray").exists());
        let ann: Annotations = HashMap::new();
        save(&root, &ann).unwrap();
        assert!(root.join(".xray").join("annotations.yml").exists());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn node_annotation_default_has_all_none() {
        let ann = NodeAnnotation::default();
        assert!(ann.team.is_none());
        assert!(ann.domain.is_none());
        assert!(ann.status.is_none());
    }
}
