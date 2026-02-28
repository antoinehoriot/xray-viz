use crate::scanner::FileAst;

#[cfg(not(target_arch = "wasm32"))]
use tree_sitter::{Parser, Query, QueryCursor};

#[cfg(not(target_arch = "wasm32"))]
use crate::scanner::{ClassDecl, FunctionDecl, ImportDecl, ImportKind};

#[cfg(not(target_arch = "wasm32"))]
const QUERY_SRC: &str = include_str!("../../../../../grammars/rust.scm");

/// Parse Rust source into a FileAst using tree-sitter.
#[cfg(not(target_arch = "wasm32"))]
pub fn parse(source: &str, path: &str) -> FileAst {
    let language = tree_sitter_rust::language();

    let mut parser = Parser::new();
    if parser.set_language(&language).is_err() {
        return FileAst {
            path: path.to_string(),
            language: "rust".to_string(),
            imports: vec![],
            exports: vec![],
            functions: vec![],
            classes: vec![],
        };
    }

    let tree = match parser.parse(source, None) {
        Some(t) => t,
        None => {
            return FileAst {
                path: path.to_string(),
                language: "rust".to_string(),
                imports: vec![],
                exports: vec![],
                functions: vec![],
                classes: vec![],
            }
        }
    };

    let query = match Query::new(&language, QUERY_SRC) {
        Ok(q) => q,
        Err(_) => {
            return FileAst {
                path: path.to_string(),
                language: "rust".to_string(),
                imports: vec![],
                exports: vec![],
                functions: vec![],
                classes: vec![],
            }
        }
    };

    let source_bytes = source.as_bytes();
    let capture_names = query.capture_names().to_vec();

    let mut imports: Vec<ImportDecl> = Vec::new();
    let mut functions: Vec<FunctionDecl> = Vec::new();
    // Rust has structs/enums/traits — model as "classes" for the IR
    let mut classes: Vec<ClassDecl> = Vec::new();

    let mut cursor = QueryCursor::new();
    for m in cursor.matches(&query, tree.root_node(), source_bytes) {
        for cap in m.captures {
            let name = capture_names[cap.index as usize];
            let text = match cap.node.utf8_text(source_bytes) {
                Ok(t) => t,
                Err(_) => continue,
            };
            let line = cap.node.start_position().row as u32;

            match name {
                "import.specifier" => {
                    // use std::collections::HashMap → specifier is the use tree text
                    imports.push(ImportDecl {
                        specifier: text.to_string(),
                        resolved_path: None,
                        kind: ImportKind::Static,
                        symbols: vec![],
                        line,
                    });
                }
                "mod.name" => {
                    // mod foo; treated as an import of a sibling module
                    imports.push(ImportDecl {
                        specifier: format!("./{text}"),
                        resolved_path: None,
                        kind: ImportKind::Static,
                        symbols: vec![],
                        line,
                    });
                }
                "extern.crate" => {
                    // extern crate serde; treated as external import
                    imports.push(ImportDecl {
                        specifier: text.to_string(),
                        resolved_path: None,
                        kind: ImportKind::Static,
                        symbols: vec![],
                        line,
                    });
                }
                "function.name" => {
                    if !functions.iter().any(|f| f.name == text && f.line_start == line) {
                        functions.push(FunctionDecl {
                            name: text.to_string(),
                            line_start: line,
                            line_end: cap.node.end_position().row as u32,
                            calls: vec![],
                            is_exported: false,
                            is_async: false,
                        });
                    }
                }
                "struct.name" | "enum.name" | "trait.name" => {
                    // Rust types modelled as classes in the IR
                    if !classes.iter().any(|c| c.name == text && c.line_start == line) {
                        classes.push(ClassDecl {
                            name: text.to_string(),
                            line_start: line,
                            line_end: cap.node.end_position().row as u32,
                        });
                    }
                }
                _ => {}
            }
        }
    }

    FileAst {
        path: path.to_string(),
        language: "rust".to_string(),
        imports,
        exports: vec![],
        functions,
        classes,
    }
}

/// WASM stub.
#[cfg(target_arch = "wasm32")]
pub fn parse(_source: &str, path: &str) -> FileAst {
    FileAst {
        path: path.to_string(),
        language: "rust".to_string(),
        imports: vec![],
        exports: vec![],
        functions: vec![],
        classes: vec![],
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;

    const SIMPLE_RS: &str = include_str!("../../../../../tests/fixtures/rust/simple.rs");

    #[test]
    fn test_use_imports() {
        let ast = parse(SIMPLE_RS, "simple.rs");
        let specifiers: Vec<&str> = ast.imports.iter().map(|i| i.specifier.as_str()).collect();
        assert!(
            specifiers.iter().any(|s| s.contains("std")),
            "expected std imports, got: {specifiers:?}"
        );
    }

    #[test]
    fn test_mod_imports() {
        let ast = parse(SIMPLE_RS, "simple.rs");
        let specifiers: Vec<&str> = ast.imports.iter().map(|i| i.specifier.as_str()).collect();
        assert!(
            specifiers.contains(&"./utils"),
            "expected './utils' mod import, got: {specifiers:?}"
        );
        assert!(
            specifiers.contains(&"./config"),
            "expected './config' mod import, got: {specifiers:?}"
        );
    }

    #[test]
    fn test_structs_as_classes() {
        let ast = parse(SIMPLE_RS, "simple.rs");
        let names: Vec<&str> = ast.classes.iter().map(|c| c.name.as_str()).collect();
        assert!(
            names.contains(&"FileScanner"),
            "expected 'FileScanner' struct, got: {names:?}"
        );
    }

    #[test]
    fn test_functions() {
        let ast = parse(SIMPLE_RS, "simple.rs");
        let names: Vec<&str> = ast.functions.iter().map(|f| f.name.as_str()).collect();
        assert!(
            names.contains(&"parse_file"),
            "expected 'parse_file' function, got: {names:?}"
        );
    }

    #[test]
    fn test_empty_source() {
        let ast = parse("", "empty.rs");
        assert!(ast.imports.is_empty());
        assert!(ast.functions.is_empty());
    }

    #[test]
    fn test_language_field() {
        let ast = parse("fn main() {}", "main.rs");
        assert_eq!(ast.language, "rust");
    }
}
