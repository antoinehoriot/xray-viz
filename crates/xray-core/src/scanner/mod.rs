pub mod languages;
pub mod resolve;

#[cfg(not(target_arch = "wasm32"))]
pub mod walker;

/// AST intermediate representation for a single file.
#[derive(Debug, Clone)]
pub struct FileAst {
    pub path: String,
    pub language: String,
    pub imports: Vec<ImportDecl>,
    pub exports: Vec<ExportDecl>,
    pub functions: Vec<FunctionDecl>,
    pub classes: Vec<ClassDecl>,
}

#[derive(Debug, Clone)]
pub struct ImportDecl {
    pub specifier: String,
    pub resolved_path: Option<String>,
    pub kind: ImportKind,
    pub symbols: Vec<String>,
    pub line: u32,
}

#[derive(Debug, Clone)]
pub enum ImportKind {
    Static,
    Dynamic,
    Reexport,
}

#[derive(Debug, Clone)]
pub struct ExportDecl {
    pub symbol: String,
    pub kind: ExportKind,
    pub line: u32,
}

#[derive(Debug, Clone)]
pub enum ExportKind {
    Named,
    Default,
    Star,
}

#[derive(Debug, Clone)]
pub struct FunctionDecl {
    pub name: String,
    pub line_start: u32,
    pub line_end: u32,
    pub calls: Vec<CallSite>,
    pub is_exported: bool,
    pub is_async: bool,
}

#[derive(Debug, Clone)]
pub struct ClassDecl {
    pub name: String,
    pub line_start: u32,
    pub line_end: u32,
}

#[derive(Debug, Clone)]
pub struct CallSite {
    pub callee: String,
    pub resolved_node_id: Option<String>,
    pub line: u32,
}
