use crate::scanner::FileAst;

/// Parse Rust source into a FileAst.
pub fn parse(_source: &str, _path: &str) -> FileAst {
    FileAst {
        path: _path.to_string(),
        language: "rust".to_string(),
        imports: vec![],
        exports: vec![],
        functions: vec![],
        classes: vec![],
    }
}
