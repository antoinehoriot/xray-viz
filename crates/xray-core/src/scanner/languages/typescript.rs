use crate::scanner::FileAst;

#[cfg(not(target_arch = "wasm32"))]
use tree_sitter::{Parser, Query, QueryCursor};

#[cfg(not(target_arch = "wasm32"))]
use crate::scanner::{ClassDecl, ExportDecl, ExportKind, FunctionDecl, ImportDecl, ImportKind};

#[cfg(not(target_arch = "wasm32"))]
const QUERY_SRC: &str = include_str!("../../../../../grammars/typescript.scm");

#[cfg(not(target_arch = "wasm32"))]
fn strip_quotes(s: &str) -> &str {
    let s = s.trim();
    if s.len() >= 2 {
        let first = s.as_bytes()[0];
        let last = s.as_bytes()[s.len() - 1];
        if (first == b'"' && last == b'"')
            || (first == b'\'' && last == b'\'')
            || (first == b'`' && last == b'`')
        {
            return &s[1..s.len() - 1];
        }
    }
    s
}

/// Parse TypeScript/JavaScript source into a FileAst using tree-sitter.
#[cfg(not(target_arch = "wasm32"))]
pub fn parse(source: &str, path: &str) -> FileAst {
    let language = tree_sitter_typescript::language_typescript();

    let mut parser = Parser::new();
    if parser.set_language(&language).is_err() {
        return FileAst {
            path: path.to_string(),
            language: "typescript".to_string(),
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
                language: "typescript".to_string(),
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
                language: "typescript".to_string(),
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
    let mut exports: Vec<ExportDecl> = Vec::new();
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
                    imports.push(ImportDecl {
                        specifier: strip_quotes(text).to_string(),
                        resolved_path: None,
                        kind: ImportKind::Static,
                        symbols: vec![],
                        line,
                    });
                }
                "import.dynamic.specifier" => {
                    imports.push(ImportDecl {
                        specifier: strip_quotes(text).to_string(),
                        resolved_path: None,
                        kind: ImportKind::Dynamic,
                        symbols: vec![],
                        line,
                    });
                }
                "export.source" => {
                    exports.push(ExportDecl {
                        symbol: strip_quotes(text).to_string(),
                        kind: ExportKind::Star,
                        line,
                    });
                }
                "export.name" => {
                    exports.push(ExportDecl {
                        symbol: text.to_string(),
                        kind: ExportKind::Named,
                        line,
                    });
                }
                "function.name" => {
                    if !functions
                        .iter()
                        .any(|f| f.name == text && f.line_start == line)
                    {
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
                "class.name" => {
                    classes.push(ClassDecl {
                        name: text.to_string(),
                        line_start: line,
                        line_end: cap.node.end_position().row as u32,
                    });
                }
                // symbol/name captures are informational, not creating separate entries
                _ => {}
            }
        }
    }

    FileAst {
        path: path.to_string(),
        language: "typescript".to_string(),
        imports,
        exports,
        functions,
        classes,
    }
}

/// WASM stub — tree-sitter not available in WASM build.
#[cfg(target_arch = "wasm32")]
pub fn parse(_source: &str, path: &str) -> FileAst {
    FileAst {
        path: path.to_string(),
        language: "typescript".to_string(),
        imports: vec![],
        exports: vec![],
        functions: vec![],
        classes: vec![],
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;

    const SIMPLE_TS: &str = include_str!("../../../../../tests/fixtures/typescript/simple.ts");
    const EXPORTS_TS: &str =
        include_str!("../../../../../tests/fixtures/typescript/with_exports.ts");

    #[test]
    fn test_static_imports() {
        let ast = parse(SIMPLE_TS, "simple.ts");
        let specifiers: Vec<&str> = ast.imports.iter().map(|i| i.specifier.as_str()).collect();
        assert!(
            specifiers.contains(&"react"),
            "expected 'react' in imports, got: {specifiers:?}"
        );
        assert!(
            specifiers.contains(&"./utils"),
            "expected './utils' in imports, got: {specifiers:?}"
        );
        for s in &specifiers {
            assert!(
                !s.starts_with('"') && !s.starts_with('\''),
                "specifier should not contain quotes: {s}"
            );
        }
    }

    #[test]
    fn test_dynamic_imports() {
        let ast = parse(SIMPLE_TS, "simple.ts");
        let dynamic: Vec<&ImportDecl> = ast
            .imports
            .iter()
            .filter(|i| matches!(i.kind, ImportKind::Dynamic))
            .collect();
        assert!(!dynamic.is_empty(), "expected at least one dynamic import");
        assert_eq!(dynamic[0].specifier, "./lazy-module");
    }

    #[test]
    fn test_exports() {
        let ast = parse(EXPORTS_TS, "with_exports.ts");
        assert!(!ast.exports.is_empty(), "expected exports");
        let names: Vec<&str> = ast.exports.iter().map(|e| e.symbol.as_str()).collect();
        assert!(names.contains(&"greet"), "expected 'greet' export");
    }

    #[test]
    fn test_functions() {
        let ast = parse(EXPORTS_TS, "with_exports.ts");
        assert!(!ast.functions.is_empty(), "expected function declarations");
        let names: Vec<&str> = ast.functions.iter().map(|f| f.name.as_str()).collect();
        assert!(names.contains(&"greet"), "expected 'greet' function");
    }

    #[test]
    fn test_classes() {
        let ast = parse(EXPORTS_TS, "with_exports.ts");
        assert!(!ast.classes.is_empty(), "expected class declarations");
        let names: Vec<&str> = ast.classes.iter().map(|c| c.name.as_str()).collect();
        assert!(names.contains(&"MyService"), "expected 'MyService' class");
    }

    #[test]
    fn test_empty_source() {
        let ast = parse("", "empty.ts");
        assert!(ast.imports.is_empty());
        assert!(ast.exports.is_empty());
        assert!(ast.functions.is_empty());
    }

    #[test]
    fn test_language_field() {
        let ast = parse("const x = 1;", "foo.ts");
        assert_eq!(ast.language, "typescript");
        assert_eq!(ast.path, "foo.ts");
    }
}
