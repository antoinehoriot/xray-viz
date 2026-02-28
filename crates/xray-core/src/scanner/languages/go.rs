use crate::scanner::FileAst;

#[cfg(not(target_arch = "wasm32"))]
use tree_sitter::{Parser, Query, QueryCursor};

#[cfg(not(target_arch = "wasm32"))]
use crate::scanner::{ClassDecl, FunctionDecl, ImportDecl, ImportKind};

#[cfg(not(target_arch = "wasm32"))]
const QUERY_SRC: &str = include_str!("../../../../../grammars/go.scm");

/// Strip surrounding double-quotes from a Go interpreted string literal.
#[cfg(not(target_arch = "wasm32"))]
fn strip_quotes(s: &str) -> &str {
    let s = s.trim();
    if s.len() >= 2 && s.starts_with('"') && s.ends_with('"') {
        return &s[1..s.len() - 1];
    }
    s
}

/// Parse Go source into a FileAst using tree-sitter.
#[cfg(not(target_arch = "wasm32"))]
pub fn parse(source: &str, path: &str) -> FileAst {
    let language = tree_sitter_go::language();

    let mut parser = Parser::new();
    if parser.set_language(&language).is_err() {
        return FileAst {
            path: path.to_string(),
            language: "go".to_string(),
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
                language: "go".to_string(),
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
                language: "go".to_string(),
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
    // Go types (structs, interfaces) modelled as classes in the IR
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
                    // Go import paths are quoted string literals: import "fmt"
                    let specifier = strip_quotes(text);
                    imports.push(ImportDecl {
                        specifier: specifier.to_string(),
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
                "method.name" => {
                    // Methods are treated as functions in the IR
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
                "type.name" => {
                    // Go type declarations (struct, interface) modelled as classes
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
        language: "go".to_string(),
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
        language: "go".to_string(),
        imports: vec![],
        exports: vec![],
        functions: vec![],
        classes: vec![],
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;

    const SIMPLE_GO: &str = include_str!("../../../../../tests/fixtures/go/simple.go");

    #[test]
    fn test_imports() {
        let ast = parse(SIMPLE_GO, "simple.go");
        let specifiers: Vec<&str> = ast.imports.iter().map(|i| i.specifier.as_str()).collect();
        assert!(
            specifiers.contains(&"fmt"),
            "expected 'fmt' import, got: {specifiers:?}"
        );
        assert!(
            specifiers.contains(&"strings"),
            "expected 'strings' import, got: {specifiers:?}"
        );
        // Specifiers must not have surrounding quotes
        for s in &specifiers {
            assert!(
                !s.starts_with('"') && !s.starts_with('\''),
                "specifier should not contain quotes: {s}"
            );
        }
    }

    #[test]
    fn test_functions() {
        let ast = parse(SIMPLE_GO, "simple.go");
        let names: Vec<&str> = ast.functions.iter().map(|f| f.name.as_str()).collect();
        assert!(
            names.contains(&"main"),
            "expected 'main' function, got: {names:?}"
        );
        assert!(
            names.contains(&"greet"),
            "expected 'greet' function, got: {names:?}"
        );
    }

    #[test]
    fn test_methods() {
        let ast = parse(SIMPLE_GO, "simple.go");
        let names: Vec<&str> = ast.functions.iter().map(|f| f.name.as_str()).collect();
        assert!(
            names.contains(&"String"),
            "expected 'String' method, got: {names:?}"
        );
    }

    #[test]
    fn test_types_as_classes() {
        let ast = parse(SIMPLE_GO, "simple.go");
        let names: Vec<&str> = ast.classes.iter().map(|c| c.name.as_str()).collect();
        assert!(
            names.contains(&"Greeter"),
            "expected 'Greeter' type, got: {names:?}"
        );
    }

    #[test]
    fn test_empty_source() {
        let ast = parse("", "empty.go");
        assert!(ast.imports.is_empty());
        assert!(ast.functions.is_empty());
    }

    #[test]
    fn test_language_field() {
        let ast = parse("package main", "main.go");
        assert_eq!(ast.language, "go");
    }
}
