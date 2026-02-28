//! Directory walker using the `ignore` crate (respects .gitignore, parallel walking).
//! M1 implementation: integrate with language parsers and produce FileAst per file.

use crate::scanner::FileAst;

/// Walk `root` directory and return placeholder FileAst for each source file found.
/// Full implementation in M1 using `ignore::WalkBuilder`.
pub fn walk(_root: &str) -> Vec<FileAst> {
    vec![]
}
