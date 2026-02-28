use std::collections::HashSet;

use crate::scanner::FileAst;

/// Post-parse resolution pass: fills in `resolved_path` on each `ImportDecl`
/// whose specifier can be matched to a known file in the project.
///
/// `file_asts` — mutable slice of parsed files (paths are project-relative)
/// `known_paths` — set of all project-relative file paths that exist
#[allow(clippy::ptr_arg)]
pub fn resolve_imports(file_asts: &mut Vec<FileAst>, known_paths: &HashSet<String>) {
    for ast in file_asts.iter_mut() {
        let file_dir = parent_dir(&ast.path);
        let language = ast.language.clone();

        for import in ast.imports.iter_mut() {
            if import.resolved_path.is_some() {
                continue;
            }
            import.resolved_path = match language.as_str() {
                "typescript" | "javascript" => {
                    resolve_ts(&import.specifier, &file_dir, known_paths)
                }
                "rust" => resolve_rust(&import.specifier, &file_dir, known_paths),
                "python" => resolve_python(&import.specifier, &file_dir, known_paths),
                "go" => resolve_go(&import.specifier, &file_dir, known_paths),
                "java" => resolve_java(&import.specifier, known_paths),
                _ => None,
            };
        }
    }
}

// ── Path helpers ──────────────────────────────────────────────────────────────

fn parent_dir(path: &str) -> String {
    match path.rfind('/') {
        Some(idx) => path[..idx].to_string(),
        None => String::new(),
    }
}

fn join_path(dir: &str, name: &str) -> String {
    if dir.is_empty() {
        name.to_string()
    } else {
        format!("{dir}/{name}")
    }
}

/// Collapse `.` and `..` segments in a `/`-separated path.
fn normalize_path(path: &str) -> String {
    let mut parts: Vec<&str> = Vec::new();
    for seg in path.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            s => parts.push(s),
        }
    }
    parts.join("/")
}

// ── TypeScript / JavaScript ────────────────────────────────────────────────────

/// Resolve `./foo` or `../foo` TypeScript specifiers.
/// Bare specifiers (npm packages) return None.
fn resolve_ts(specifier: &str, file_dir: &str, known_paths: &HashSet<String>) -> Option<String> {
    if !specifier.starts_with("./") && !specifier.starts_with("../") {
        return None; // npm package or alias — skip
    }

    let base = normalize_path(&join_path(file_dir, specifier));

    // Direct match (specifier already has extension, e.g. `./foo.js`)
    if known_paths.contains(&base) {
        return Some(base);
    }

    // Try adding extensions
    for ext in &[".ts", ".tsx", ".js", ".jsx", ".mjs", ".cjs"] {
        let candidate = format!("{base}{ext}");
        if known_paths.contains(&candidate) {
            return Some(candidate);
        }
    }

    // Try as directory with index file
    for idx in &["index.ts", "index.tsx", "index.js", "index.jsx"] {
        let candidate = format!("{base}/{idx}");
        if known_paths.contains(&candidate) {
            return Some(candidate);
        }
    }

    None
}

// ── Rust ───────────────────────────────────────────────────────────────────────

/// Resolve Rust specifiers:
/// - `./foo`      — mod declaration (mod foo;)
/// - `crate::…`  — absolute path from crate root
/// - `super::…`  — relative path from parent module
/// - `self::…`   — relative path from current module
/// - everything else is an external crate — skip
fn resolve_rust(specifier: &str, file_dir: &str, known_paths: &HashSet<String>) -> Option<String> {
    if let Some(name) = specifier.strip_prefix("./") {
        // mod declaration
        let candidate_file = join_path(file_dir, &format!("{name}.rs"));
        if known_paths.contains(&candidate_file) {
            return Some(candidate_file);
        }
        let candidate_mod = join_path(file_dir, &format!("{name}/mod.rs"));
        if known_paths.contains(&candidate_mod) {
            return Some(candidate_mod);
        }
        return None;
    }

    if let Some(rest) = specifier.strip_prefix("crate::") {
        let crate_src = find_crate_src(file_dir, known_paths)?;
        return resolve_rust_module_path(rest, &crate_src, known_paths);
    }

    if let Some(rest) = specifier.strip_prefix("super::") {
        // super = one level up from current module's directory
        let parent = parent_dir(file_dir);
        return resolve_rust_module_path(rest, &parent, known_paths);
    }

    if let Some(rest) = specifier.strip_prefix("self::") {
        return resolve_rust_module_path(rest, file_dir, known_paths);
    }

    None // external crate (std, serde, axum, …)
}

/// Try to resolve a `::`-delimited module path under `base_dir`.
///
/// Tries progressively shorter prefixes to handle trailing type/symbol names
/// (e.g. `scanner::FileAst` → tries `scanner/FileAst.rs` then `scanner.rs`).
fn resolve_rust_module_path(
    module_path: &str,
    base_dir: &str,
    known_paths: &HashSet<String>,
) -> Option<String> {
    let segments: Vec<&str> = module_path.split("::").collect();

    for n in (1..=segments.len()).rev() {
        let rel = segments[..n].join("/");

        let as_file = join_path(base_dir, &format!("{rel}.rs"));
        if known_paths.contains(&as_file) {
            return Some(as_file);
        }

        let as_mod = join_path(base_dir, &format!("{rel}/mod.rs"));
        if known_paths.contains(&as_mod) {
            return Some(as_mod);
        }
    }

    None
}

/// Walk up from `file_dir` to find the `src/` directory that contains
/// `lib.rs` or `main.rs` — that is the crate root source directory.
fn find_crate_src(file_dir: &str, known_paths: &HashSet<String>) -> Option<String> {
    let parts: Vec<&str> = file_dir.split('/').filter(|s| !s.is_empty()).collect();

    // Walk from current directory up to the project root
    for n in (0..=parts.len()).rev() {
        let dir = parts[..n].join("/");
        let lib = join_path(&dir, "lib.rs");
        let main = join_path(&dir, "main.rs");

        if known_paths.contains(&lib) || known_paths.contains(&main) {
            return Some(dir);
        }
    }

    None
}

// ── Python ─────────────────────────────────────────────────────────────────────

/// Resolve Python relative imports (specifiers starting with `.`).
/// Absolute imports (external packages) return None.
fn resolve_python(specifier: &str, file_dir: &str, known_paths: &HashSet<String>) -> Option<String> {
    if !specifier.starts_with('.') {
        return None; // stdlib or third-party package
    }

    // Count leading dots: `.foo` = 1, `..foo` = 2
    let dot_count = specifier.chars().take_while(|c| *c == '.').count();
    let module = &specifier[dot_count..];

    // Start at file_dir and go up (dot_count - 1) levels
    let mut parts: Vec<&str> = file_dir.split('/').filter(|s| !s.is_empty()).collect();
    for _ in 0..dot_count.saturating_sub(1) {
        parts.pop();
    }
    let base_dir = parts.join("/");

    if module.is_empty() {
        // `from . import x` → resolve to the package's __init__.py
        let candidate = join_path(&base_dir, "__init__.py");
        return known_paths.contains(&candidate).then_some(candidate);
    }

    // Convert dotted module name to path (e.g. `foo.bar` → `foo/bar`)
    let module_path = module.replace('.', "/");
    let base = join_path(&base_dir, &module_path);

    let as_file = format!("{base}.py");
    if known_paths.contains(&as_file) {
        return Some(as_file);
    }

    let as_pkg = format!("{base}/__init__.py");
    if known_paths.contains(&as_pkg) {
        return Some(as_pkg);
    }

    None
}

// ── Go ─────────────────────────────────────────────────────────────────────────

/// Go uses package-level imports. Without module resolution context we can only
/// attempt to match relative-looking paths (unusual in Go but handled for
/// completeness). Most Go imports are external packages — return None.
fn resolve_go(specifier: &str, file_dir: &str, known_paths: &HashSet<String>) -> Option<String> {
    if !specifier.starts_with("./") && !specifier.starts_with("../") {
        return None; // external module path
    }

    let base = normalize_path(&join_path(file_dir, specifier));

    // Go file match
    #[allow(clippy::single_element_loop)]
    for ext in &[".go"] {
        let candidate = format!("{base}{ext}");
        if known_paths.contains(&candidate) {
            return Some(candidate);
        }
    }

    None
}

// ── Java ───────────────────────────────────────────────────────────────────────

/// Resolve Java import declarations to file paths.
///
/// Java imports are fully-qualified class names like `com.example.Foo`.
/// We convert the dotted path to a slash path and append `.java`.
/// Wildcard imports (`com.example.*`) are not resolvable to a single file.
fn resolve_java(specifier: &str, known_paths: &HashSet<String>) -> Option<String> {
    // Wildcard or star imports cannot be resolved to a single file
    if specifier.ends_with('*') {
        return None;
    }

    // Convert `com.example.Foo` → `com/example/Foo.java`
    let path = format!("{}.java", specifier.replace('.', "/"));
    if known_paths.contains(&path) {
        return Some(path);
    }

    // Also try without the last component (in case specifier is a package)
    None
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scanner::{ImportDecl, ImportKind};

    fn make_import(specifier: &str) -> ImportDecl {
        ImportDecl {
            specifier: specifier.to_string(),
            resolved_path: None,
            kind: ImportKind::Static,
            symbols: vec![],
            line: 0,
        }
    }

    fn known(paths: &[&str]) -> HashSet<String> {
        paths.iter().map(|s| s.to_string()).collect()
    }

    // ── TypeScript ────────────────────────────────────────────────────────────

    #[test]
    fn ts_resolves_relative_with_extension() {
        let paths = known(&["src/utils.ts"]);
        let result = resolve_ts("./utils", "src", &paths);
        assert_eq!(result, Some("src/utils.ts".to_string()));
    }

    #[test]
    fn ts_resolves_tsx() {
        let paths = known(&["src/components/Button.tsx"]);
        let result = resolve_ts("./Button", "src/components", &paths);
        assert_eq!(result, Some("src/components/Button.tsx".to_string()));
    }

    #[test]
    fn ts_resolves_index() {
        let paths = known(&["src/components/index.ts"]);
        let result = resolve_ts("./components", "src", &paths);
        assert_eq!(result, Some("src/components/index.ts".to_string()));
    }

    #[test]
    fn ts_ignores_bare_specifier() {
        let paths = known(&["node_modules/react/index.js"]);
        let result = resolve_ts("react", "src", &paths);
        assert_eq!(result, None);
    }

    #[test]
    fn ts_resolves_parent_dir() {
        let paths = known(&["src/utils.ts"]);
        let result = resolve_ts("../utils", "src/components", &paths);
        assert_eq!(result, Some("src/utils.ts".to_string()));
    }

    // ── Rust mod declarations ─────────────────────────────────────────────────

    #[test]
    fn rust_resolves_mod_as_file() {
        let paths = known(&["src/utils.rs"]);
        let result = resolve_rust("./utils", "src", &paths);
        assert_eq!(result, Some("src/utils.rs".to_string()));
    }

    #[test]
    fn rust_resolves_mod_as_dir() {
        let paths = known(&["src/scanner/mod.rs"]);
        let result = resolve_rust("./scanner", "src", &paths);
        assert_eq!(result, Some("src/scanner/mod.rs".to_string()));
    }

    #[test]
    fn rust_resolves_crate_path() {
        let paths = known(&[
            "crates/xray-core/src/lib.rs",
            "crates/xray-core/src/scanner/mod.rs",
        ]);
        let result = resolve_rust(
            "crate::scanner::FileAst",
            "crates/xray-core/src/graph",
            &paths,
        );
        assert_eq!(
            result,
            Some("crates/xray-core/src/scanner/mod.rs".to_string())
        );
    }

    #[test]
    fn rust_ignores_external_crate() {
        let paths = known(&[]);
        let result = resolve_rust("std::collections::HashMap", "src", &paths);
        assert_eq!(result, None);
    }

    // ── Python ────────────────────────────────────────────────────────────────

    #[test]
    fn python_resolves_relative_sibling() {
        let paths = known(&["mypackage/utils.py"]);
        let result = resolve_python(".utils", "mypackage", &paths);
        assert_eq!(result, Some("mypackage/utils.py".to_string()));
    }

    #[test]
    fn python_resolves_parent_package() {
        let paths = known(&["mypackage/utils.py"]);
        let result = resolve_python("..utils", "mypackage/sub", &paths);
        assert_eq!(result, Some("mypackage/utils.py".to_string()));
    }

    #[test]
    fn python_ignores_absolute() {
        let paths = known(&["os.py"]);
        let result = resolve_python("os", "mypackage", &paths);
        assert_eq!(result, None);
    }

    // ── Java ──────────────────────────────────────────────────────────────────

    #[test]
    fn java_resolves_qualified_class() {
        let paths = known(&["com/example/Foo.java"]);
        let result = resolve_java("com.example.Foo", &paths);
        assert_eq!(result, Some("com/example/Foo.java".to_string()));
    }

    #[test]
    fn java_ignores_wildcard_import() {
        let paths = known(&["com/example/Foo.java"]);
        let result = resolve_java("com.example.*", &paths);
        assert_eq!(result, None);
    }

    #[test]
    fn java_ignores_missing_class() {
        let paths = known(&["com/example/Bar.java"]);
        let result = resolve_java("com.example.Foo", &paths);
        assert_eq!(result, None);
    }

    // ── resolve_imports integration ───────────────────────────────────────────

    #[test]
    fn resolve_imports_fills_resolved_path() {
        let mut asts = vec![
            FileAst {
                path: "src/main.ts".to_string(),
                language: "typescript".to_string(),
                imports: vec![make_import("./utils")],
                exports: vec![],
                functions: vec![],
                classes: vec![],
            },
            FileAst {
                path: "src/utils.ts".to_string(),
                language: "typescript".to_string(),
                imports: vec![],
                exports: vec![],
                functions: vec![],
                classes: vec![],
            },
        ];

        let known: HashSet<String> = asts.iter().map(|a| a.path.clone()).collect();
        resolve_imports(&mut asts, &known);

        assert_eq!(
            asts[0].imports[0].resolved_path,
            Some("src/utils.ts".to_string())
        );
    }

    #[test]
    fn resolve_imports_skips_already_resolved() {
        let mut asts = vec![FileAst {
            path: "src/main.ts".to_string(),
            language: "typescript".to_string(),
            imports: vec![ImportDecl {
                specifier: "./utils".to_string(),
                resolved_path: Some("custom/path.ts".to_string()),
                kind: ImportKind::Static,
                symbols: vec![],
                line: 0,
            }],
            exports: vec![],
            functions: vec![],
            classes: vec![],
        }];

        let known: HashSet<String> = asts.iter().map(|a| a.path.clone()).collect();
        resolve_imports(&mut asts, &known);

        // Should not be overwritten
        assert_eq!(
            asts[0].imports[0].resolved_path,
            Some("custom/path.ts".to_string())
        );
    }
}
