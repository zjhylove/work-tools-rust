//! # Path safety utilities
//!
//! Reusable path validation functions for sandbox enforcement.
//! Used by commands (write_file, read_plugin_asset) and tests.

use std::path::Path;

/// Validates that `target` is inside or equal to `base_dir`.
///
/// Uses `canonicalize` on both paths to resolve symlinks, `.` and `..`
/// components, yielding the true on-disk location before comparison.
///
/// Returns `Ok(())` if the target is within the sandbox, or an error
/// message suitable for returning to the caller.
#[allow(dead_code)]
pub fn sandbox_check(target: &Path, base_dir: &Path) -> Result<(), String> {
    let canonical_target = std::fs::canonicalize(target)
        .map_err(|e| format!("解析路径失败: {}", e))?;
    let canonical_base = std::fs::canonicalize(base_dir)
        .map_err(|e| format!("解析应用目录失败: {}", e))?;

    if !canonical_target.starts_with(&canonical_base) {
        return Err(format!(
            "写入路径超出应用目录范围: {:?}",
            canonical_target
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sandbox_allows_path_inside_base() {
        let base = tempfile::tempdir().unwrap();
        let inner = base.path().join("subdir");
        std::fs::create_dir_all(&inner).unwrap();
        assert!(sandbox_check(&inner, base.path()).is_ok());
    }

    #[test]
    fn sandbox_allows_file_inside_base() {
        let base = tempfile::tempdir().unwrap();
        let file = base.path().join("file.txt");
        std::fs::write(&file, "hello").unwrap();
        assert!(sandbox_check(&file, base.path()).is_ok());
    }

    #[test]
    fn sandbox_rejects_path_outside_base() {
        let base = tempfile::tempdir().unwrap();
        let other = tempfile::tempdir().unwrap();
        assert!(sandbox_check(other.path(), base.path()).is_err());
    }

    #[test]
    fn sandbox_rejects_symlink_escape() {
        let base = tempfile::tempdir().unwrap();
        let other = tempfile::tempdir().unwrap();

        let symlink_path = base.path().join("escape");
        let symlink_result = {
            #[cfg(unix)]
            {
                std::os::unix::fs::symlink(other.path(), &symlink_path)
            }
            #[cfg(windows)]
            {
                std::os::windows::fs::symlink_dir(other.path(), &symlink_path)
            }
        };

        // Skip if symlink creation fails (e.g. no privilege on Windows)
        if symlink_result.is_err() {
            return;
        }

        let canonical = std::fs::canonicalize(&symlink_path).unwrap();
        let canonical_base = std::fs::canonicalize(base.path()).unwrap();
        if !canonical.starts_with(&canonical_base) {
            assert!(sandbox_check(&symlink_path, base.path()).is_err());
        }
    }

    #[test]
    fn sandbox_rejects_nonexistent_path() {
        let base = tempfile::tempdir().unwrap();
        let ghost = std::path::PathBuf::from("/this/absolutely/does/not/exist");
        assert!(sandbox_check(&ghost, base.path()).is_err());
    }
}
