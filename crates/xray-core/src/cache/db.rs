//! SQLite incremental cache (.xray/cache.db).
//! Schema: file_hash TEXT PRIMARY KEY -> deps_json BLOB
//! M1 implementation: use rusqlite to cache blake3(file) -> parsed deps.

#[cfg(not(target_arch = "wasm32"))]
pub fn open(_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    // M1: rusqlite::Connection::open(path)?;
    Ok(())
}
