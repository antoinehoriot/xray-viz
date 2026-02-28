use crate::scanner::FileAst;

/// Parse Go source into a FileAst.
pub fn parse(_source: &str, _path: &str) -> FileAst {
    FileAst {
        path: _path.to_string(),
        language: "go".to_string(),
        imports: vec![],
        exports: vec![],
        functions: vec![],
        classes: vec![],
    }
}
