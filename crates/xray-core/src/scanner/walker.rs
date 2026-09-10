//! Directory walker using the `ignore` crate (respects .gitignore, parallel walking).
//! M1 implementation: walk directories and parse source files with tree-sitter.

use std::path::PathBuf;

use ignore::WalkBuilder;
use rayon::prelude::*;

use crate::detect_language;
use crate::scanner::{languages, FileAst};

/// Walk `root` directory, parse each supported source file, and return a `FileAst` per file.
///
/// - Respects `.gitignore` and other ignore files via the `ignore` crate.
/// - Parses files in parallel using `rayon`.
/// - Only processes files with a supported extension (TypeScript, Python, Rust, Go, Java).
pub fn walk(root: &str) -> Vec<FileAst> {
    // Phase 1: collect all parseable file paths (single-threaded walk)
    let paths: Vec<PathBuf> = WalkBuilder::new(root)
        .hidden(false) // include dot-prefixed files (still respects .gitignore)
        .build()
        .filter_map(|result| result.ok())
        .filter(|entry| entry.file_type().map(|ft| ft.is_file()).unwrap_or(false))
        .filter(|entry| detect_language(entry.path()).is_some())
        .map(|entry| entry.path().to_path_buf())
        .collect();

    // Phase 2: parse in parallel
    paths
        .par_iter()
        .filter_map(|path| {
            let lang = detect_language(path)?;
            let source = std::fs::read_to_string(path).ok()?;
            let path_str = path.to_str()?;
            Some(parse_source(&source, path_str, lang))
        })
        .collect()
}

fn parse_source(source: &str, path: &str, language: &str) -> FileAst {
    match language {
        "typescript" => languages::typescript::parse(source, path),
        "python" => languages::python::parse(source, path),
        "rust" => languages::rust::parse(source, path),
        "go" => languages::go::parse(source, path),
        "java" => languages::java::parse(source, path),
        _ => FileAst {
            path: path.to_string(),
            language: language.to_string(),
            imports: vec![],
            exports: vec![],
            functions: vec![],
            classes: vec![],
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_walk_fixtures() {
        let fixtures_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../tests/fixtures");
        let asts = walk(fixtures_dir);
        assert!(
            !asts.is_empty(),
            "expected at least one FileAst from fixtures"
        );

        let languages: Vec<&str> = asts.iter().map(|a| a.language.as_str()).collect();
        assert!(
            languages.contains(&"typescript"),
            "expected TypeScript files"
        );
        assert!(languages.contains(&"python"), "expected Python files");
        assert!(languages.contains(&"rust"), "expected Rust files");
    }

    #[test]
    fn test_walk_nonexistent() {
        let asts = walk("/nonexistent/path/that/does/not/exist");
        assert!(
            asts.is_empty(),
            "nonexistent path should yield empty result"
        );
    }
}
