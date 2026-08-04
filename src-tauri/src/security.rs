use crate::models::WorkspaceRoot;
use std::path::Path;

const SENSITIVE_DIRECTORY_NAMES: &[&str] = &[
    ".aws", ".azure", ".gnupg", ".ssh", "credentials", "secrets",
];

const SENSITIVE_FILE_NAMES: &[&str] = &[
    ".aws_credentials", ".netrc", ".npmrc", ".pypirc", "authorized_keys",
    "credentials.json", "id_dsa", "id_ecdsa", "id_ed25519", "id_rsa",
    "known_hosts", "service-account.json", "serviceaccount.json",
];

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

    // Derive the suffix from the same normalized representation used for the
    // prefix comparison. Slicing the original UTF-8 path by a lower-cased byte
    // length can panic for non-ASCII Windows paths.
    let relative = p_norm.strip_prefix(&r_norm).unwrap_or(&p_norm);
    let relative = relative.trim_start_matches('/').trim_start_matches('\\');
    let relative_slashes = relative.replace('\\', "/");
    #[cfg(windows)]
    let relative_for_matching = relative_slashes.to_lowercase();
    #[cfg(not(windows))]
    let relative_for_matching = relative_slashes.clone();

    for glob_pattern in compiled_patterns {
        let matches = glob_pattern.matches(&relative_slashes);
        if matches {
            return true;
        }
    }

    #[cfg(windows)]
    for pattern in exclude_globs {
        if let Ok(pattern) = glob::Pattern::new(&pattern.to_lowercase()) {
            if pattern.matches(&relative_for_matching) {
                return true;
            }
        }
    }

    for pattern in exclude_globs {
        #[cfg(windows)]
        let pattern = pattern.to_lowercase();
        #[cfg(not(windows))]
        let pattern = pattern.as_str();
        // Also match as a path prefix for directory patterns like "node_modules"
        if relative_for_matching.starts_with(&pattern)
            || relative_for_matching.contains(&format!("/{}", pattern))
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
    ".aws",
    ".azure",
    ".gnupg",
    ".ssh",
    "credentials",
    "secrets",
    ".npmrc",
    ".pypirc",
    ".netrc",
    "*.pem",
    "*.p12",
    "*.pfx",
    "*.key",
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
    fn sensitive_paths_are_case_insensitive() {
        assert!(is_sensitive_path(Path::new("repo/Secrets/token.txt")));
        assert!(is_sensitive_path(Path::new("repo/.ENV.Local")));
        assert!(is_sensitive_path(Path::new("repo/Credentials.JSON")));
        assert!(!is_sensitive_path(Path::new("repo/src/config.ts")));
    }

    #[test]
    fn authorize_read_blocks_sensitive_and_custom_excluded_paths() {
        let temp = tempfile::tempdir().unwrap();
        let root_path = temp.path().join("repo");
        fs::create_dir_all(root_path.join("private")).unwrap();
        fs::write(root_path.join("private/note.txt"), "private").unwrap();
        fs::write(root_path.join(".env"), "TOKEN=short").unwrap();
        fs::write(root_path.join("readme.txt"), "safe").unwrap();
        let mut root = test_root(&root_path, "read_write");
        root.exclude_globs.push("private".into());

        assert!(authorize_read(&root_path.join("readme.txt"), std::slice::from_ref(&root)).is_ok());
        assert!(authorize_read(&root_path.join(".env"), std::slice::from_ref(&root)).is_err());
        assert!(authorize_read(&root_path.join("private/note.txt"), &[root]).is_err());
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

    #[cfg(windows)]
    #[test]
    fn test_is_excluded_case_insensitive_directory_names() {
        let root = test_root(Path::new("C:\\Projects"), "read_write");
        assert!(is_excluded(
            Path::new("C:\\Projects\\Node_Modules\\pkg\\index.js"),
            &root
        ));
    }
}

/// Return whether a path is sensitive regardless of configurable root globs.
/// Matching is case-insensitive on every platform so repository behavior stays
/// consistent when it moves between Windows and case-sensitive filesystems.
pub fn is_sensitive_path(path: &Path) -> bool {
    for component in path.components() {
        let std::path::Component::Normal(value) = component else {
            continue;
        };
        let Some(value) = value.to_str() else {
            continue;
        };
        let value = value.to_lowercase();
        if SENSITIVE_DIRECTORY_NAMES.contains(&value.as_str()) {
            return true;
        }
    }

    path.file_name()
        .and_then(|value| value.to_str())
        .map(|value| {
            let value = value.to_lowercase();
            value == ".env"
                || value.starts_with(".env.")
                || SENSITIVE_FILE_NAMES.contains(&value.as_str())
                || value.ends_with(".pem")
                || value.ends_with(".p12")
                || value.ends_with(".pfx")
                || value.ends_with(".key")
        })
        .unwrap_or(false)
}

/// Authorize a filesystem read and enforce user exclusions plus the global
/// sensitive-path policy. The matching root can be reused to filter children.
pub fn authorize_read<'a>(
    path: &Path,
    roots: &'a [WorkspaceRoot],
) -> Result<&'a WorkspaceRoot, String> {
    let root = authorize_path(path, roots)
        .ok_or_else(|| format!("Path {:?} is not within any authorized root", path))?;
    if is_sensitive_path(path) {
        return Err(format!("Path {:?} is blocked by the sensitive-path policy", path));
    }
    if is_excluded(path, root) {
        return Err(format!("Path {:?} is excluded by workspace policy", path));
    }
    Ok(root)
}

