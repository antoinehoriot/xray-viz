use std::collections::HashMap;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use crate::scanner::FileAst;
use crate::graph::types::Node;

mod utils;
mod config;

pub struct FileScanner {
    root: PathBuf,
    extensions: Vec<String>,
}

impl FileScanner {
    pub fn new(root: &Path) -> Self {
        Self {
            root: root.to_path_buf(),
            extensions: vec!["rs".to_string(), "ts".to_string()],
        }
    }

    pub fn scan(&self) -> Vec<PathBuf> {
        let mut result = Vec::new();
        self.walk(&self.root, &mut result);
        result
    }

    fn walk(&self, dir: &Path, out: &mut Vec<PathBuf>) {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    self.walk(&path, out);
                } else if let Some(ext) = path.extension() {
                    if self.extensions.contains(&ext.to_string_lossy().to_string()) {
                        out.push(path);
                    }
                }
            }
        }
    }
}

pub fn parse_file(path: &Path) -> Option<FileAst> {
    let content = std::fs::read_to_string(path).ok()?;
    Some(FileAst {
        path: path.to_string_lossy().to_string(),
        language: "rust".to_string(),
        imports: vec![],
        exports: vec![],
        functions: vec![],
        classes: vec![],
    })
}

pub fn index(paths: &[PathBuf]) -> HashMap<String, Node> {
    let mut map = HashMap::new();
    for p in paths {
        if let Some(_ast) = parse_file(p) {
            // placeholder
        }
    }
    map
}
