//! Safe path validation and normalization.
//!
//! All file paths derived from user input or external sources MUST be
//! resolved through this module before use. This prevents path traversal
//! attacks, symlink escapes, and accidental writes outside workspace
//! boundaries.

use std::path::{Path, PathBuf};

/// Errors that can occur during path validation.
#[derive(Debug, thiserror::Error)]
pub enum PathError {
    /// The resolved path escapes the workspace boundary.
    #[error("path traversal denied: resolved path is outside workspace")]
    TraversalDenied,

    /// The path could not be canonicalized (e.g., does not exist).
    #[error("failed to resolve path: {0}")]
    ResolutionFailed(#[from] std::io::Error),

    /// The provided path was empty.
    #[error("path must not be empty")]
    EmptyPath,

    /// The filename contains invalid characters.
    #[error("invalid filename: {0}")]
    InvalidFilename(String),
}

/// Resolve a user-supplied path relative to a workspace root, ensuring the
/// result stays within the workspace boundary.
///
/// # Errors
///
/// Returns `PathError::TraversalDenied` if the resolved path escapes the
/// workspace. Returns `PathError::ResolutionFailed` if canonicalization fails.
///
/// # Examples
///
/// ```no_run
/// use kryon_security::path::safe_resolve;
/// use std::path::Path;
///
/// let root = Path::new("/workspace");
/// let resolved = safe_resolve(root, "src/main.rs").unwrap();
/// assert!(resolved.starts_with("/workspace"));
/// ```
pub fn safe_resolve(workspace_root: &Path, user_path: &str) -> Result<PathBuf, PathError> {
    if user_path.is_empty() {
        return Err(PathError::EmptyPath);
    }

    let canonical_root = workspace_root.canonicalize()?;
    let candidate = canonical_root.join(user_path);

    // Attempt to canonicalize; if the file doesn't exist yet, normalize
    // the path manually to catch traversal sequences.
    let resolved = match candidate.canonicalize() {
        Ok(p) => p,
        Err(_) => normalize_path(&candidate),
    };

    if !resolved.starts_with(&canonical_root) {
        tracing::warn!(
            workspace = %canonical_root.display(),
            attempted = %resolved.display(),
            "path traversal attempt blocked"
        );
        return Err(PathError::TraversalDenied);
    }

    Ok(resolved)
}

/// Check whether a resolved path is within the workspace boundary.
///
/// Both paths should already be canonicalized for accurate comparison.
#[must_use]
pub fn is_within_workspace(workspace_root: &Path, target: &Path) -> bool {
    match (workspace_root.canonicalize(), target.canonicalize()) {
        (Ok(root), Ok(resolved)) => resolved.starts_with(&root),
        _ => false,
    }
}

/// Validate that a filename does not contain dangerous characters or sequences.
///
/// Rejects:
/// - Empty names
/// - Names containing path separators (`/`, `\`)
/// - Names containing null bytes
/// - Names that are `.` or `..`
/// - Names starting with `-` (could be interpreted as CLI flags)
///
/// # Errors
///
/// Returns `PathError::EmptyPath` if the name is empty.
/// Returns `PathError::InvalidFilename` if the name contains forbidden characters.
pub fn validate_filename(name: &str) -> Result<(), PathError> {
    if name.is_empty() {
        return Err(PathError::EmptyPath);
    }

    if name == "." || name == ".." {
        return Err(PathError::InvalidFilename(
            "filename cannot be '.' or '..'".to_string(),
        ));
    }

    if name.contains('/') || name.contains('\\') || name.contains('\0') {
        return Err(PathError::InvalidFilename(
            "filename contains invalid characters".to_string(),
        ));
    }

    if name.starts_with('-') {
        return Err(PathError::InvalidFilename(
            "filename must not start with '-'".to_string(),
        ));
    }

    Ok(())
}

/// Normalize a path by resolving `.` and `..` components without touching
/// the filesystem. This is used as a fallback when the target file does
/// not yet exist and `canonicalize()` would fail.
fn normalize_path(path: &Path) -> PathBuf {
    let mut components = Vec::new();
    for component in path.components() {
        match component {
            std::path::Component::ParentDir => {
                components.pop();
            }
            std::path::Component::CurDir => {}
            other => components.push(other),
        }
    }
    components.iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_empty_path_rejected() {
        let root = std::env::temp_dir();
        let result = safe_resolve(&root, "");
        assert!(matches!(result, Err(PathError::EmptyPath)));
    }

    #[test]
    fn test_traversal_denied() {
        let root = std::env::temp_dir().join("kryon_test_root");
        fs::create_dir_all(&root).unwrap();

        let result = safe_resolve(&root, "../../../etc/passwd");
        assert!(matches!(result, Err(PathError::TraversalDenied)));

        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn test_valid_path_within_workspace() {
        let root = std::env::temp_dir().join("kryon_test_valid");
        let sub = root.join("src");
        fs::create_dir_all(&sub).unwrap();

        let file = sub.join("main.rs");
        fs::write(&file, "fn main() {}").unwrap();

        let result = safe_resolve(&root, "src/main.rs");
        assert!(result.is_ok());

        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn test_validate_filename_rejects_traversal() {
        assert!(validate_filename("..").is_err());
        assert!(validate_filename(".").is_err());
        assert!(validate_filename("foo/bar").is_err());
        assert!(validate_filename("foo\\bar").is_err());
        assert!(validate_filename("-flag").is_err());
        assert!(validate_filename("").is_err());
    }

    #[test]
    fn test_validate_filename_accepts_valid() {
        assert!(validate_filename("main.rs").is_ok());
        assert!(validate_filename(".gitignore").is_ok());
        assert!(validate_filename("README.md").is_ok());
        assert!(validate_filename("my_module").is_ok());
    }

    #[test]
    fn test_normalize_resolves_parent_dirs() {
        let path = PathBuf::from("/workspace/src/../config/./theme.toml");
        let normalized = normalize_path(&path);
        assert_eq!(normalized, PathBuf::from("/workspace/config/theme.toml"));
    }
}
