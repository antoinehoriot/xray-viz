use crate::scanner::FileAst;

#[cfg(not(target_arch = "wasm32"))]
use tree_sitter::{Parser, Query, QueryCursor};

#[cfg(not(target_arch = "wasm32"))]
use crate::scanner::{ClassDecl, FunctionDecl, ImportDecl, ImportKind};

#[cfg(not(target_arch = "wasm32"))]
const QUERY_SRC: &str = include_str!("../../../../../grammars/java.scm");

/// Parse Java source into a FileAst using tree-sitter.
#[cfg(not(target_arch = "wasm32"))]
pub fn parse(source: &str, path: &str) -> FileAst {
    let language = tree_sitter_java::language();

    let mut parser = Parser::new();
    if parser.set_language(&language).is_err() {
        return FileAst {
            path: path.to_string(),
            language: "java".to_string(),
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
                language: "java".to_string(),
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
                language: "java".to_string(),
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
                    // Java: `import com.example.Foo;` → specifier is `com.example.Foo`
                    let specifier = text.to_string();
                    if !imports.iter().any(|i: &ImportDecl| i.specifier == specifier) {
                        imports.push(ImportDecl {
                            specifier,
                            resolved_path: None,
                            kind: ImportKind::Static,
                            symbols: vec![],
                            line,
                        });
                    }
                }
                "class.name" | "interface.name" => {
                    // Java classes and interfaces both modelled as ClassDecl
                    if !classes.iter().any(|c: &ClassDecl| c.name == text && c.line_start == line) {
                        classes.push(ClassDecl {
                            name: text.to_string(),
                            line_start: line,
                            line_end: cap.node.end_position().row as u32,
                        });
                    }
                }
                "method.name" => {
                    if !functions.iter().any(|f: &FunctionDecl| f.name == text && f.line_start == line) {
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
                _ => {}
            }
        }
    }

    FileAst {
        path: path.to_string(),
        language: "java".to_string(),
        imports,
        exports: vec![],
        functions,
        classes,
    }
}

/// WASM stub — Java parser is native-only.
#[cfg(target_arch = "wasm32")]
pub fn parse(_source: &str, path: &str) -> FileAst {
    FileAst {
        path: path.to_string(),
        language: "java".to_string(),
        imports: vec![],
        exports: vec![],
        functions: vec![],
        classes: vec![],
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;

    const SIMPLE_JAVA: &str = include_str!("../../../../../tests/fixtures/java/Simple.java");

    #[test]
    fn test_imports() {
        let ast = parse(SIMPLE_JAVA, "Simple.java");
        let specifiers: Vec<&str> = ast.imports.iter().map(|i| i.specifier.as_str()).collect();
        assert!(
            specifiers.iter().any(|s| s.contains("java.util")),
            "expected java.util import, got: {specifiers:?}"
        );
        assert!(
            specifiers.iter().any(|s| s.contains("java.io")),
            "expected java.io import, got: {specifiers:?}"
        );
    }

    #[test]
    fn test_classes() {
        let ast = parse(SIMPLE_JAVA, "Simple.java");
        let names: Vec<&str> = ast.classes.iter().map(|c| c.name.as_str()).collect();
        assert!(
            names.contains(&"Simple"),
            "expected 'Simple' class, got: {names:?}"
        );
    }

    #[test]
    fn test_methods() {
        let ast = parse(SIMPLE_JAVA, "Simple.java");
        let names: Vec<&str> = ast.functions.iter().map(|f| f.name.as_str()).collect();
        assert!(
            names.contains(&"greet"),
            "expected 'greet' method, got: {names:?}"
        );
        assert!(
            names.contains(&"main"),
            "expected 'main' method, got: {names:?}"
        );
    }

    #[test]
    fn test_empty_source() {
        let ast = parse("", "Empty.java");
        assert!(ast.imports.is_empty());
        assert!(ast.functions.is_empty());
    }

    #[test]
    fn test_language_field() {
        let ast = parse("class Foo {}", "Foo.java");
        assert_eq!(ast.language, "java");
    }
}
