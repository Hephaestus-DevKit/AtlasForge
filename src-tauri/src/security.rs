use crate::models::WorkspaceRoot;
use std::path::Path;

#[cfg(windows)]
fn normalized_windows_path(path: &Path) -> String {
    path.to_string_lossy()
        .replace('/', "\\")
        .trim_end_matches('\\')
        .to_lowercase()
}

fn path_is_within(path: &Path, root: &Path) -> bool {
    #[cfg(windows)]
    {
        let path = normalized_windows_path(path);
        let root = normalized_windows_path(root);
        path == root
            || path
                .strip_prefix(&root)
                .is_some_and(|suffix| suffix.starts_with('\\'))
    }

    #[cfg(not(windows))]
    {
        path.starts_with(root)
    }
}

/// Compare two existing paths after resolving symlinks and platform-specific casing.
pub fn same_path(left: &Path, right: &Path) -> bool {
    let Ok(left) = left.canonicalize() else {
        return false;
    };
    let Ok(right) = right.canonicalize() else {
        return false;
    };

    #[cfg(windows)]
    {
        normalized_windows_path(&left) == normalized_windows_path(&right)
    }

    #[cfg(not(windows))]
    {
        left == right
    }
}

/// Check if a given path is within any authorized workspace root.
/// Returns the matching root if authorized, None otherwise.
pub fn authorize_path<'a>(path: &Path, roots: &'a [WorkspaceRoot]) -> Option<&'a WorkspaceRoot> {
    let canonical = match path.canonicalize() {
        Ok(c) => c,
        Err(_) => return None,
    };
    for root in roots {
        // Since root.path is already canonicalized when saved to the database,
        // we can directly convert it to a Path without performing redundant I/O.
        let root_path = Path::new(&root.path);
        if path_is_within(&canonical, root_path) {
            return Some(root);
        }
    }
    None
}

/// Check if a write operation is allowed for a given path.
/// Returns Ok(()) if allowed, Err with reason if not.
pub fn authorize_write(path: &Path, roots: &[WorkspaceRoot]) -> Result<(), String> {
    match authorize_path(path, roots) {
        Some(root) => {
            if root.access_mode == "read_only" {
                Err(format!(
                    "Path {:?} is in a read-only root: {}",
                    path, root.label
                ))
            } else {
                Ok(())
            }
        }
        None => Err(format!("Path {:?} is not within any authorized root", path)),
    }
}

/// Check if a path should be excluded based on root exclude globs.
#[allow(dead_code)]
pub fn is_excluded(path: &Path, root: &WorkspaceRoot) -> bool {

    let compiled: Vec<glob::Pattern> = root
        .exclude_globs
        .iter()
        .filter_map(|pattern| glob::Pattern::new(pattern).ok())
        .collect();
    is_excluded_fast(path, &root.path, &root.exclude_globs, &compiled)
}

/// Optimized version of is_excluded using pre-compiled glob patterns.
pub fn is_excluded_fast(
    path: &Path,
    root_path: &str,
    exclude_globs: &[String],
    compiled_patterns: &[glob::Pattern],
) -> bool {
    let path_str = path.to_string_lossy();

    #[cfg(windows)]
    let (p_norm, r_norm) = {
        let p = path_str.replace('/', "\\").to_lowercase();
        let r = root_path.replace('/', "\\").to_lowercase();
        (p, r)
    };

    #[cfg(not(windows))]
    let (p_norm, r_norm) = (path_str.to_string(), root_path.to_string());

    let relative = if p_norm.starts_with(&r_norm) {
        &path_str[r_norm.len()..]
    } else {
        &path_str
    };
    let relative = relative.trim_start_matches('/').trim_start_matches('\\');
    let relative_slashes = relative.replace('\\', "/");

    for glob_pattern in compiled_patterns {
        if glob_pattern.matches(&relative_slashes) {
            return true;
        }
    }

    for pattern in exclude_globs {
        // Also match as a path prefix for directory patterns like "node_modules"
        if relative_slashes.starts_with(pattern)
            || relative_slashes.contains(&format!("/{}", pattern))
        {
            return true;
        }
    }
    false
}



/// Default exclude patterns applied to all roots.
pub const DEFAULT_EXCLUDE_GLOBS: &[&str] = &[
    "node_modules",
    ".git/objects",
    "dist",
    "build",
    ".env",
    "__pycache__",
    ".next",
    ".cache",
    "target",
    "*.pyc",
];

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn test_root(path: &Path, access_mode: &str) -> WorkspaceRoot {
        let path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        WorkspaceRoot {
            id: "test".into(),
            path: path.to_string_lossy().into_owned(),
            label: "Test".into(),
            access_mode: access_mode.into(),
            scan_enabled: true,
            include_globs: vec![],
            exclude_globs: DEFAULT_EXCLUDE_GLOBS
                .iter()
                .map(|s| s.to_string())
                .collect(),
            created_at: "".into(),
            last_scanned_at: None,
        }
    }

    #[test]
    fn test_excluded_node_modules() {
        let root_path = Path::new("/tmp/test_root");
        let root = test_root(root_path, "read_write");
        let path = root_path.join("project/node_modules/react/index.js");
        assert!(is_excluded(&path, &root));
    }

    #[test]
    fn test_excluded_env() {
        let root_path = Path::new("/tmp/test_root");
        let root = test_root(root_path, "read_write");
        let path = root_path.join("project/.env");
        assert!(is_excluded(&path, &root));
    }

    #[test]
    fn test_not_excluded_src() {
        let root_path = Path::new("/tmp/test_root");
        let root = test_root(root_path, "read_write");
        let path = root_path.join("project/src/main.ts");
        assert!(!is_excluded(&path, &root));
    }

    #[test]
    fn authorize_path_rejects_sibling_with_shared_prefix() {
        let temp = tempfile::tempdir().unwrap();
        let root_path = temp.path().join("repo");
        let sibling_path = temp.path().join("repo-copy");
        fs::create_dir_all(&root_path).unwrap();
        fs::create_dir_all(&sibling_path).unwrap();
        let root = test_root(&root_path, "read_write");

        assert!(authorize_path(&root_path, std::slice::from_ref(&root)).is_some());
        assert!(authorize_path(&sibling_path, &[root]).is_none());
    }

    #[test]
    fn authorize_path_resolves_nested_paths() {
        let temp = tempfile::tempdir().unwrap();
        let root_path = temp.path().join("repo");
        let nested_path = root_path.join("src");
        fs::create_dir_all(&nested_path).unwrap();
        let root = test_root(&root_path, "read_write");

        assert!(authorize_path(&nested_path, &[root]).is_some());
        assert!(same_path(&root_path, &root_path));
    }

    #[test]
    fn test_is_excluded_casing_and_separators() {
        let root = WorkspaceRoot {
            id: "test".into(),
            path: "C:\\Users\\User\\Projects".into(),
            label: "Test".into(),
            access_mode: "read_write".into(),
            scan_enabled: true,
            include_globs: vec![],
            exclude_globs: vec!["node_modules".into(), "dist/output.json".into()],
            created_at: "".into(),
            last_scanned_at: None,
        };

        // Standard matching casing
        assert!(is_excluded(Path::new("C:\\Users\\User\\Projects\\node_modules\\react\\index.js"), &root));
        // Mismatched casing on root
        assert!(is_excluded(Path::new("c:\\users\\user\\projects\\node_modules\\react\\index.js"), &root));
        // Forward slashes in path
        assert!(is_excluded(Path::new("C:/Users/User/Projects/node_modules/react/index.js"), &root));
        // Subpath match (e.g. dist/output.json glob with forward slash)
        assert!(is_excluded(Path::new("C:\\Users\\User\\Projects\\dist\\output.json"), &root));
        assert!(is_excluded(Path::new("C:/Users/User/Projects/dist/output.json"), &root));
    }
}

