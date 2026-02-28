use crate::scanner::FileAst;

/// Parse Java source into a FileAst.
pub fn parse(_source: &str, _path: &str) -> FileAst {
    FileAst {
        path: _path.to_string(),
        language: "java".to_string(),
        imports: vec![],
        exports: vec![],
        functions: vec![],
        classes: vec![],
    }
}
