//! Offline Pro license validation utilities.
//!
//! License key format: `XRAY-XXXX-XXXX` where each X is an ASCII alphanumeric character.
//! Keys are stored in `~/.xray/license.key`.

use std::fs;
use std::path::PathBuf;

const LICENSE_PREFIX: &str = "XRAY-";

/// Returns the path to the stored license key: `~/.xray/license.key`.
fn license_path() -> Option<PathBuf> {
    let mut path = dirs::home_dir()?;
    path.push(".xray");
    path.push("license.key");
    Some(path)
}

/// Returns `true` if `key` matches the `XRAY-XXXX-XXXX` format
/// (prefix + two 4-character ASCII alphanumeric segments).
pub fn is_valid_key(key: &str) -> bool {
    let trimmed = key.trim();
    let Some(rest) = trimmed.strip_prefix(LICENSE_PREFIX) else {
        return false;
    };
    let parts: Vec<&str> = rest.split('-').collect();
    if parts.len() != 2 {
        return false;
    }
    parts
        .iter()
        .all(|p| p.len() == 4 && p.chars().all(|c| c.is_ascii_alphanumeric()))
}

/// Read the stored license key from `~/.xray/license.key`.
/// Returns `None` if no key is stored or the file cannot be read.
pub fn get_license() -> Option<String> {
    let path = license_path()?;
    let content = fs::read_to_string(path).ok()?;
    let key = content.trim().to_string();
    if key.is_empty() {
        None
    } else {
        Some(key)
    }
}

/// Persist `key` to `~/.xray/license.key`, creating the directory if needed.
pub fn save_license(key: &str) -> Result<(), Box<dyn std::error::Error>> {
    let path = license_path().ok_or("could not determine home directory")?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&path, key.trim())?;
    Ok(())
}

/// Remove the stored license key file.
/// No-op if the file does not exist.
pub fn remove_license() -> Result<(), Box<dyn std::error::Error>> {
    let path = license_path().ok_or("could not determine home directory")?;
    if path.exists() {
        fs::remove_file(&path)?;
    }
    Ok(())
}

/// Returns `true` when a valid Pro license key is stored.
pub fn is_pro() -> bool {
    get_license().map(|k| is_valid_key(&k)).unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Key format validation ──────────────────────────────────────────────────

    #[test]
    fn valid_key_accepts_uppercase() {
        assert!(is_valid_key("XRAY-AB12-CD34"));
    }

    #[test]
    fn valid_key_accepts_lowercase() {
        assert!(is_valid_key("XRAY-abcd-1234"));
    }

    #[test]
    fn valid_key_accepts_all_alpha() {
        assert!(is_valid_key("XRAY-ABCD-EFGH"));
    }

    #[test]
    fn valid_key_trims_whitespace() {
        assert!(is_valid_key("  XRAY-AB12-CD34  "));
    }

    #[test]
    fn invalid_key_empty() {
        assert!(!is_valid_key(""));
    }

    #[test]
    fn invalid_key_wrong_prefix() {
        assert!(!is_valid_key("NOTXRAY-AB12-CD34"));
    }

    #[test]
    fn invalid_key_missing_second_segment() {
        assert!(!is_valid_key("XRAY-AB12"));
    }

    #[test]
    fn invalid_key_extra_segment() {
        assert!(!is_valid_key("XRAY-AB12-CD34-EF56"));
    }

    #[test]
    fn invalid_key_short_segment() {
        assert!(!is_valid_key("XRAY-AB1-CD345"));
    }

    #[test]
    fn invalid_key_special_chars() {
        assert!(!is_valid_key("XRAY-AB!@-CD34"));
    }

    // ── Path-level save / load / remove (direct fs, no HOME manipulation) ───

    /// Writes a key to a given path under a temporary base directory.
    fn write_key_at(base: &std::path::Path, key: &str) {
        let dir = base.join(".xray");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("license.key"), key).unwrap();
    }

    /// Reads the key from the given base directory (mirrors get_license logic).
    fn read_key_at(base: &std::path::Path) -> Option<String> {
        let path = base.join(".xray").join("license.key");
        let content = fs::read_to_string(path).ok()?;
        let s = content.trim().to_string();
        if s.is_empty() {
            None
        } else {
            Some(s)
        }
    }

    #[test]
    fn is_pro_does_not_panic() {
        // Smoke test: is_pro() must not panic regardless of disk state.
        let _ = is_pro();
    }

    #[test]
    fn write_and_read_key_via_helpers() {
        let dir = std::env::temp_dir().join("xray_test_wr");
        let _ = fs::remove_dir_all(&dir);
        let key = "XRAY-AB12-CD34";
        write_key_at(&dir, key);
        let loaded = read_key_at(&dir).expect("key not found");
        assert_eq!(loaded, key);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn key_file_trimmed_on_read() {
        let dir = std::env::temp_dir().join("xray_test_trim");
        let _ = fs::remove_dir_all(&dir);
        write_key_at(&dir, "  XRAY-TR1M-KEY1  \n");
        let loaded = read_key_at(&dir).unwrap();
        assert_eq!(loaded, "XRAY-TR1M-KEY1");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn empty_file_returns_none() {
        let dir = std::env::temp_dir().join("xray_test_empty");
        let _ = fs::remove_dir_all(&dir);
        write_key_at(&dir, "   ");
        assert!(read_key_at(&dir).is_none());
        let _ = fs::remove_dir_all(&dir);
    }
}
