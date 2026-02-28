use crate::scanner::FileAst;

/// Parse TypeScript/JavaScript source into a FileAst.
/// M1 implementation: use tree-sitter to extract imports/exports/functions.
pub fn parse(_source: &str, _path: &str) -> FileAst {
    FileAst {
        path: _path.to_string(),
        language: "typescript".to_string(),
        imports: vec![],
        exports: vec![],
        functions: vec![],
        classes: vec![],
    }
}
