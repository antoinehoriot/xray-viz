//! SQLite incremental cache (.xray/cache.db).
//! Tables:
//!   file_cache     (file_hash TEXT PK, path TEXT, language TEXT, deps_json TEXT, parsed_at INTEGER)
//!   file_functions (file_hash TEXT PK, functions_json TEXT, classes_json TEXT)

#[cfg(not(target_arch = "wasm32"))]
mod native {
    use rusqlite::{Connection, params};
    use serde::{Deserialize, Serialize};
    use std::path::Path;
    use crate::scanner::{ClassDecl, FunctionDecl, ImportDecl, ImportKind};

    #[derive(Debug, Serialize, Deserialize)]
    struct CachedImport {
        specifier: String,
        resolved_path: Option<String>,
        kind: String,
        symbols: Vec<String>,
        line: u32,
    }

    impl From<&ImportDecl> for CachedImport {
        fn from(d: &ImportDecl) -> Self {
            Self {
                specifier: d.specifier.clone(),
                resolved_path: d.resolved_path.clone(),
                kind: match d.kind {
                    ImportKind::Static => "static",
                    ImportKind::Dynamic => "dynamic",
                    ImportKind::Reexport => "reexport",
                }
                .to_string(),
                symbols: d.symbols.clone(),
                line: d.line,
            }
        }
    }

    impl From<CachedImport> for ImportDecl {
        fn from(c: CachedImport) -> Self {
            Self {
                specifier: c.specifier,
                resolved_path: c.resolved_path,
                kind: match c.kind.as_str() {
                    "dynamic" => ImportKind::Dynamic,
                    "reexport" => ImportKind::Reexport,
                    _ => ImportKind::Static,
                },
                symbols: c.symbols,
                line: c.line,
            }
        }
    }

    pub struct CacheDb {
        conn: Connection,
    }

    impl CacheDb {
        pub fn open(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let conn = Connection::open(path)?;
            conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS file_cache (
                    file_hash TEXT PRIMARY KEY,
                    path      TEXT NOT NULL,
                    language  TEXT NOT NULL,
                    deps_json TEXT NOT NULL,
                    parsed_at INTEGER NOT NULL
                );
                CREATE TABLE IF NOT EXISTS file_functions (
                    file_hash      TEXT PRIMARY KEY,
                    functions_json TEXT NOT NULL,
                    classes_json   TEXT NOT NULL
                );",
            )?;
            Ok(Self { conn })
        }

        pub fn get_deps(
            &self,
            file_hash: &str,
        ) -> Result<Option<Vec<ImportDecl>>, Box<dyn std::error::Error>> {
            let mut stmt = self
                .conn
                .prepare("SELECT deps_json FROM file_cache WHERE file_hash = ?1")?;
            let mut rows = stmt.query(params![file_hash])?;
            if let Some(row) = rows.next()? {
                let json: String = row.get(0)?;
                let cached: Vec<CachedImport> = serde_json::from_str(&json)?;
                Ok(Some(cached.into_iter().map(ImportDecl::from).collect()))
            } else {
                Ok(None)
            }
        }

        pub fn put_deps(
            &self,
            file_hash: &str,
            path: &str,
            language: &str,
            deps: &[ImportDecl],
        ) -> Result<(), Box<dyn std::error::Error>> {
            let cached: Vec<CachedImport> = deps.iter().map(CachedImport::from).collect();
            let json = serde_json::to_string(&cached)?;
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64;
            self.conn.execute(
                "INSERT OR REPLACE INTO file_cache (file_hash, path, language, deps_json, parsed_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![file_hash, path, language, json, now],
            )?;
            Ok(())
        }

        /// Retrieve cached function/class declarations for a file.
        #[allow(clippy::type_complexity)]
        pub fn get_functions(
            &self,
            file_hash: &str,
        ) -> Result<Option<(Vec<FunctionDecl>, Vec<ClassDecl>)>, Box<dyn std::error::Error>> {
            let mut stmt = self.conn.prepare(
                "SELECT functions_json, classes_json FROM file_functions WHERE file_hash = ?1",
            )?;
            let mut rows = stmt.query(params![file_hash])?;
            if let Some(row) = rows.next()? {
                let funcs_json: String = row.get(0)?;
                let classes_json: String = row.get(1)?;
                let funcs: Vec<FunctionDecl> = serde_json::from_str(&funcs_json)?;
                let classes: Vec<ClassDecl> = serde_json::from_str(&classes_json)?;
                Ok(Some((funcs, classes)))
            } else {
                Ok(None)
            }
        }

        /// Store function/class declarations for a file.
        pub fn put_functions(
            &self,
            file_hash: &str,
            functions: &[FunctionDecl],
            classes: &[ClassDecl],
        ) -> Result<(), Box<dyn std::error::Error>> {
            let funcs_json = serde_json::to_string(functions)?;
            let classes_json = serde_json::to_string(classes)?;
            self.conn.execute(
                "INSERT OR REPLACE INTO file_functions (file_hash, functions_json, classes_json)
                 VALUES (?1, ?2, ?3)",
                params![file_hash, funcs_json, classes_json],
            )?;
            Ok(())
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub use native::CacheDb;

#[cfg(test)]
#[cfg(not(target_arch = "wasm32"))]
mod tests {
    use super::CacheDb;
    use crate::scanner::{CallSite, ClassDecl, FunctionDecl, ImportDecl, ImportKind};

    fn make_import(specifier: &str) -> ImportDecl {
        ImportDecl {
            specifier: specifier.to_string(),
            resolved_path: None,
            kind: ImportKind::Static,
            symbols: vec!["foo".to_string()],
            line: 1,
        }
    }

    fn temp_db_path(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("xray_test_{name}.db"))
    }

    #[test]
    fn test_cache_roundtrip() {
        let db_path = temp_db_path("roundtrip");
        let _ = std::fs::remove_file(&db_path);
        let db = CacheDb::open(&db_path).unwrap();

        let imports = vec![make_import("../utils")];
        db.put_deps("abc123", "src/main.ts", "typescript", &imports)
            .unwrap();

        let result = db.get_deps("abc123").unwrap().unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].specifier, "../utils");
        let _ = std::fs::remove_file(&db_path);
    }

    #[test]
    fn test_cache_miss() {
        let db_path = temp_db_path("miss");
        let _ = std::fs::remove_file(&db_path);
        let db = CacheDb::open(&db_path).unwrap();
        let result = db.get_deps("notexist").unwrap();
        assert!(result.is_none());
        let _ = std::fs::remove_file(&db_path);
    }

    #[test]
    fn test_functions_roundtrip() {
        let db_path = temp_db_path("funcs");
        let _ = std::fs::remove_file(&db_path);
        let db = CacheDb::open(&db_path).unwrap();

        let functions = vec![FunctionDecl {
            name: "doWork".to_string(),
            line_start: 5,
            line_end: 10,
            calls: vec![CallSite {
                callee: "helper".to_string(),
                resolved_node_id: None,
                line: 7,
            }],
            is_exported: true,
            is_async: false,
        }];
        let classes = vec![ClassDecl {
            name: "MyClass".to_string(),
            line_start: 1,
            line_end: 4,
        }];

        db.put_functions("xyz789", &functions, &classes).unwrap();
        let (funcs, cls) = db.get_functions("xyz789").unwrap().unwrap();
        assert_eq!(funcs.len(), 1);
        assert_eq!(funcs[0].name, "doWork");
        assert_eq!(funcs[0].calls.len(), 1);
        assert_eq!(cls.len(), 1);
        assert_eq!(cls[0].name, "MyClass");
        let _ = std::fs::remove_file(&db_path);
    }

    #[test]
    fn test_functions_miss() {
        let db_path = temp_db_path("funcs_miss");
        let _ = std::fs::remove_file(&db_path);
        let db = CacheDb::open(&db_path).unwrap();
        let result = db.get_functions("notexist").unwrap();
        assert!(result.is_none());
        let _ = std::fs::remove_file(&db_path);
    }
}
