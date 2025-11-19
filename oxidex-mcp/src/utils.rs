use anyhow::{Context, Result};
use std::path::{Component, Path, PathBuf};

/// Expand a glob pattern to a list of files
pub fn expand_glob(pattern: &str) -> Result<Vec<PathBuf>> {
    let paths: Result<Vec<PathBuf>> = glob::glob(pattern)
        .context("Invalid glob pattern")?
        .map(|result| result.context("Failed to read glob entry"))
        .collect();

    paths
}

/// Validate a path to prevent directory traversal
pub fn validate_path(path: &str) -> Result<()> {
    let path = Path::new(path);

    // Check for directory traversal components
    for component in path.components() {
        if component == Component::ParentDir {
            anyhow::bail!(
                "Path contains parent directory reference (directory traversal not allowed)"
            );
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_path_rejects_traversal() {
        assert!(validate_path("../etc/passwd").is_err());
        assert!(validate_path("photos/../../../etc").is_err());
        assert!(validate_path("photos/../etc").is_err());
    }

    #[test]
    fn test_validate_path_allows_dotdot_in_filename() {
        assert!(validate_path("my..photo.jpg").is_ok());
        assert!(validate_path("photos/my..vacation..file.jpg").is_ok());
        assert!(validate_path("file..with..dots.jpg").is_ok());
    }

    #[test]
    fn test_validate_path_accepts_safe_paths() {
        assert!(validate_path("photo.jpg").is_ok());
        assert!(validate_path("photos/vacation/img.jpg").is_ok());
        assert!(validate_path("/absolute/path/file.jpg").is_ok());
    }
}
