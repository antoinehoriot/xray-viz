use crate::scanner::FileAst;

#[cfg(not(target_arch = "wasm32"))]
use tree_sitter::{Parser, Query, QueryCursor};

#[cfg(not(target_arch = "wasm32"))]
use crate::scanner::{ClassDecl, FunctionDecl, ImportDecl, ImportKind};

#[cfg(not(target_arch = "wasm32"))]
const QUERY_SRC: &str = include_str!("../../../../../grammars/go.scm");

/// Strip surrounding double-quotes from a Go import path literal.
/// e.g. `"fmt"` → `fmt`, `"github.com/foo/bar"` → `github.com/foo/bar`
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
#[allow(clippy::collapsible_match)]
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
                    // Go import paths come as `"fmt"` or `"github.com/foo/bar"` — strip quotes
                    let specifier = strip_quotes(text).to_string();
                    if !imports
                        .iter()
                        .any(|i: &ImportDecl| i.specifier == specifier)
                    {
                        imports.push(ImportDecl {
                            specifier,
                            resolved_path: None,
                            kind: ImportKind::Static,
                            symbols: vec![],
                            line,
                        });
                    }
                }
                "function.name" | "method.name" => {
                    if !functions
                        .iter()
                        .any(|f: &FunctionDecl| f.name == text && f.line_start == line)
                    {
                        functions.push(FunctionDecl {
                            name: text.to_string(),
                            line_start: line,
                            line_end: cap.node.end_position().row as u32,
                            calls: vec![],
                            is_exported: text.starts_with(|c: char| c.is_uppercase()),
                            is_async: false,
                        });
                    }
                }
                "type.name" => {
                    // Go type declarations modelled as classes in the IR
                    if !classes
                        .iter()
                        .any(|c: &ClassDecl| c.name == text && c.line_start == line)
                    {
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

/// WASM stub — Go parser is native-only.
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
            specifiers.contains(&"os"),
            "expected 'os' import, got: {specifiers:?}"
        );
    }

    #[test]
    fn test_no_quotes_in_specifiers() {
        let ast = parse(SIMPLE_GO, "simple.go");
        for imp in &ast.imports {
            assert!(
                !imp.specifier.starts_with('"'),
                "specifier should not contain quotes: {}",
                imp.specifier
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
            names.contains(&"Greet"),
            "expected 'Greet' function, got: {names:?}"
        );
    }

    #[test]
    fn test_exported_function_detection() {
        let ast = parse(SIMPLE_GO, "simple.go");
        let greet = ast.functions.iter().find(|f| f.name == "Greet");
        assert!(greet.is_some(), "Greet function not found");
        assert!(
            greet.unwrap().is_exported,
            "Greet should be exported (uppercase)"
        );

        let main_fn = ast.functions.iter().find(|f| f.name == "main");
        assert!(main_fn.is_some(), "main function not found");
        assert!(
            !main_fn.unwrap().is_exported,
            "main should not be exported (lowercase)"
        );
    }

    #[test]
    fn test_types_as_classes() {
        let ast = parse(SIMPLE_GO, "simple.go");
        let names: Vec<&str> = ast.classes.iter().map(|c| c.name.as_str()).collect();
        assert!(
            names.contains(&"Person"),
            "expected 'Person' type, got: {names:?}"
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
