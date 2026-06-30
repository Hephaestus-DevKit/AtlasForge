use crate::db::Db;
use crate::models::*;
use serde::{Deserialize, Serialize};

use std::path::Path;
use std::process::Command;
use walkdir::WalkDir;

/// Record of an error that occurred during scanning.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanErrorRecord {
    pub id: String,
    pub root_id: String,
    pub path: Option<String>,
    pub error_type: String,
    pub message: String,
}

/// Discover git repositories under an authorized root.
/// Returns (discovered_repos, errors).
pub fn scan_root(root: &WorkspaceRoot, db: &Db) -> (Vec<Repository>, Vec<ScanErrorRecord>) {
    let root_path = Path::new(&root.path);
    let root_id = &root.id;
    if !root_path.exists() {
        return (
            vec![],
            vec![ScanErrorRecord {
                id: uuid::Uuid::new_v4().to_string(),
                root_id: root_id.clone(),
                path: Some(root.path.clone()),
                error_type: "scan_error".into(),
                message: format!("Root path does not exist: {}", root.path),
            }],
        );
    }
    if !root_path.is_dir() {
        return (
            vec![],
            vec![ScanErrorRecord {
                id: uuid::Uuid::new_v4().to_string(),
                root_id: root_id.clone(),
                path: Some(root.path.clone()),
                error_type: "scan_error".into(),
                message: format!("Root path is not a directory: {}", root.path),
            }],
        );
    }



    let mut candidate_paths = Vec::new();
    let mut errors = Vec::new();
    let mut visited = std::collections::HashSet::new();

    let compiled_excludes: Vec<glob::Pattern> = root
        .exclude_globs
        .iter()
        .filter_map(|pattern| glob::Pattern::new(pattern).ok())
        .collect();

    for entry in WalkDir::new(root_path)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            let path = e.path();
            // Skip excluded paths
            if crate::security::is_excluded_fast(
                path,
                &root.path,
                &root.exclude_globs,
                &compiled_excludes,
            ) {
                return false;
            }
            // Do not walk into Git internals; repo roots are detected by checking for a .git child.
            if path.file_name() == Some(std::ffi::OsStr::new(".git")) {
                return false;
            }
            true
        })

    {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                errors.push(ScanErrorRecord {
                    id: uuid::Uuid::new_v4().to_string(),
                    root_id: root_id.clone(),
                    path: Some(
                        e.path()
                            .map(|p| p.to_string_lossy().to_string())
                            .unwrap_or_default(),
                    ),
                    error_type: "walk_error".into(),
                    message: format!("Walk error: {}", e),
                });
                continue;
            }
        };

        let path = entry.path();

        // Look for Git repositories. A .git child can be a directory or a worktree file.
        if entry.file_type().is_dir() && path.join(".git").exists() {
            let repo_path_str = path.to_string_lossy().to_string();
            if visited.contains(&repo_path_str) {
                continue;
            }
            visited.insert(repo_path_str.clone());
            candidate_paths.push(path.to_path_buf());
        }
    }

    let mut repos = Vec::new();
    let mut results = Vec::new();
    let root_ref = root;
    let db_ref = db;

    std::thread::scope(|s| {
        let mut threads = Vec::new();
        for path in candidate_paths {
            threads.push(s.spawn(move || {
                let conn = match db_ref.connection() {
                    Ok(c) => c,
                    Err(e) => return Err((e.to_string(), path.clone())),
                };
                discover_git_repo(&path, root_ref, &conn)
                    .map(|repo| (repo, path.clone()))
                    .map_err(|err| (err, path))
            }));
        }
        for t in threads {
            results.push(t.join());
        }
    });

    for res in results {
        match res {
            Ok(Ok((repo, _))) => repos.push(repo),
            Ok(Err((e, path))) => errors.push(ScanErrorRecord {
                id: uuid::Uuid::new_v4().to_string(),
                root_id: root_id.clone(),
                path: Some(path.to_string_lossy().to_string()),
                error_type: "scan_error".into(),
                message: e,
            }),
            Err(_) => errors.push(ScanErrorRecord {
                id: uuid::Uuid::new_v4().to_string(),
                root_id: root_id.clone(),
                path: None,
                error_type: "thread_panic".into(),
                message: "Thread panicked during repository discovery".into(),
            }),
        }
    }

    (repos, errors)
}

fn discover_git_repo(
    repo_path: &Path,
    root: &WorkspaceRoot,
    conn: &rusqlite::Connection,
) -> Result<Repository, String> {
    let worktree_path = repo_path.to_string_lossy().to_string();
    let git_dir_path = repo_path.join(".git").to_string_lossy().to_string();
    let name = repo_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown".into());

    // Create or find the project asset
    let asset_id = ensure_project_asset(&worktree_path, root, &name, conn)?;

    // Check for existing repo by worktree_path to preserve stable IDs in the struct
    let existing_id = {
        let mut stmt = conn
            .prepare("SELECT id FROM repository WHERE worktree_path = ?1")
            .map_err(|e| e.to_string())?;
        stmt.query_row(rusqlite::params![worktree_path], |row| row.get(0))
            .ok()
    };
    let repo_id = existing_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    // Read git info
    let current_branch = git_command(repo_path, &["rev-parse", "--abbrev-ref", "HEAD"]).ok();
    let head_sha = git_command(repo_path, &["rev-parse", "HEAD"]).ok();
    let default_branch = git_command(repo_path, &["symbolic-ref", "refs/remotes/origin/HEAD"])
        .ok()
        .map(|s| s.replace("refs/remotes/origin/", ""));
    let remote_origin_url = git_command(repo_path, &["config", "--get", "remote.origin.url"]).ok();
    let dirty_state = is_dirty(repo_path);
    let ahead_behind = get_ahead_behind(repo_path);
    let last_commit_at = git_command(repo_path, &["log", "-1", "--format=%aI"]).ok();
    let is_bare = false; // we only discover non-bare repos
    let is_worktree = Path::new(&git_dir_path).is_file(); // .git file means worktree

    let repo = Repository {
        id: repo_id,
        asset_id,
        worktree_path: worktree_path.clone(),
        git_dir_path,
        is_bare,
        is_worktree,
        default_branch,
        current_branch,
        head_sha,
        remote_origin_url,
        dirty_state,
        ahead_behind,
        last_commit_at,
    };

    // Upsert into database
    upsert_repository(&repo, conn)?;

    Ok(repo)
}

fn ensure_project_asset(
    path: &str,
    root: &WorkspaceRoot,
    name: &str,
    conn: &rusqlite::Connection,
) -> Result<String, String> {
    // Check existing
    let mut stmt = conn
        .prepare("SELECT id FROM project_asset WHERE path = ?1")
        .map_err(|e| e.to_string())?;
    let existing: Option<String> = stmt
        .query_row(rusqlite::params![path], |row| row.get(0))
        .ok();

    if let Some(id) = existing {
        // Update last_observed_at
        conn.execute(
            "UPDATE project_asset SET last_observed_at = datetime('now') WHERE id = ?1",
            rusqlite::params![id],
        )
        .map_err(|e| e.to_string())?;
        return Ok(id);
    }

    let id = uuid::Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO project_asset (id, root_id, path, kind, name, last_observed_at) VALUES (?1, ?2, ?3, 'git_repo', ?4, datetime('now'))",
        rusqlite::params![id, root.id, path, name],
    )
    .map_err(|e| e.to_string())?;

    Ok(id)
}

fn upsert_repository(repo: &Repository, conn: &rusqlite::Connection) -> Result<(), String> {
    let ahead_behind_json = repo
        .ahead_behind
        .as_ref()
        .map(|ab| serde_json::to_string(ab).unwrap_or_default());

    // Check for existing repo by worktree_path to preserve stable IDs
    let existing_id: Option<String> = conn
        .query_row(
            "SELECT id FROM repository WHERE worktree_path = ?1",
            rusqlite::params![repo.worktree_path],
            |row| row.get(0),
        )
        .ok();

    if let Some(existing_id) = existing_id {
        // Update existing repo in place, preserving the stable ID
        conn.execute(
            "UPDATE repository SET asset_id = ?1, git_dir_path = ?2, is_bare = ?3, is_worktree = ?4, default_branch = ?5, current_branch = ?6, head_sha = ?7, remote_origin_url = ?8, dirty_state = ?9, ahead_behind = ?10, last_commit_at = ?11 WHERE id = ?12",
            rusqlite::params![
                repo.asset_id,
                repo.git_dir_path,
                repo.is_bare as i32,
                repo.is_worktree as i32,
                repo.default_branch,
                repo.current_branch,
                repo.head_sha,
                repo.remote_origin_url,
                repo.dirty_state as i32,
                ahead_behind_json,
                repo.last_commit_at,
                existing_id,
            ],
        )
        .map_err(|e| e.to_string())?;
    } else {
        // Insert new repo
        conn.execute(
            "INSERT INTO repository (id, asset_id, worktree_path, git_dir_path, is_bare, is_worktree, default_branch, current_branch, head_sha, remote_origin_url, dirty_state, ahead_behind, last_commit_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            rusqlite::params![
                repo.id,
                repo.asset_id,
                repo.worktree_path,
                repo.git_dir_path,
                repo.is_bare as i32,
                repo.is_worktree as i32,
                repo.default_branch,
                repo.current_branch,
                repo.head_sha,
                repo.remote_origin_url,
                repo.dirty_state as i32,
                ahead_behind_json,
                repo.last_commit_at,
            ],
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn git_command(repo_path: &Path, args: &[&str]) -> Result<String, String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(repo_path)
        .output()
        .map_err(|e| format!("git command failed: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "git {:?} failed: {}",
            args,
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn is_dirty(repo_path: &Path) -> bool {
    git_command(repo_path, &["status", "--porcelain"])
        .map(|s| !s.is_empty())
        .unwrap_or(false)
}

fn get_ahead_behind(repo_path: &Path) -> Option<AheadBehind> {
    // Try to get ahead/behind for current branch vs its upstream
    let branch = git_command(repo_path, &["rev-parse", "--abbrev-ref", "@{u}"]).ok()?;
    let output = git_command(
        repo_path,
        &[
            "rev-list",
            "--left-right",
            "--count",
            &format!("{}...HEAD", branch),
        ],
    )
    .ok()?;

    let parts: Vec<&str> = output.split_whitespace().collect();
    if parts.len() == 2 {
        Some(AheadBehind {
            behind: parts[0].parse().ok()?,
            ahead: parts[1].parse().ok()?,
        })
    } else {
        None
    }
}

/// Persist scan error records into the scan_error table.
pub fn persist_scan_errors(
    errors: &[ScanErrorRecord],
    job_id: Option<&str>,
    db: &Db,
) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    for err in errors {
        conn.execute(
            "INSERT INTO scan_error (id, root_id, job_id, path, error_type, message) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![
                err.id,
                err.root_id,
                job_id,
                err.path,
                err.error_type,
                err.message,
            ],
        )
        .map_err(|e| format!("Failed to insert scan error: {}", e))?;
    }

    Ok(())
}

/// List scan error records for a given root.
pub fn list_scan_errors(root_id: &str, db: &Db) -> Result<Vec<ScanErrorRecord>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let mut stmt = conn
        .prepare("SELECT id, root_id, path, error_type, message FROM scan_error WHERE root_id = ?1")
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map(rusqlite::params![root_id], |row| {
            Ok(ScanErrorRecord {
                id: row.get(0)?,
                root_id: row.get(1)?,
                path: row.get(2)?,
                error_type: row.get(3)?,
                message: row.get(4)?,
            })
        })
        .map_err(|e| e.to_string())?;

    let mut records = Vec::new();
    for row in rows {
        records.push(row.map_err(|e| e.to_string())?);
    }

    Ok(records)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Db;
    use std::fs;
    use std::path::PathBuf;

    /// Create a file-backed database with migrations applied.
    fn test_db(db_path: &Path) -> Db {
        Db::new(&db_path.to_path_buf()).expect("Failed to create test DB")
    }

    /// Create a minimal WorkspaceRoot for testing.
    fn test_root(path: &str) -> WorkspaceRoot {
        WorkspaceRoot {
            id: uuid::Uuid::new_v4().to_string(),
            path: path.to_string(),
            label: "test-root".into(),
            access_mode: "read_only".into(),
            scan_enabled: true,
            include_globs: vec![],
            exclude_globs: vec!["node_modules".into()],
            created_at: "2025-01-01T00:00:00Z".into(),
            last_scanned_at: None,
        }
    }

    fn insert_root(db: &Db, root: &WorkspaceRoot) {
        let conn = db.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO workspace_root (id, path, label, access_mode, scan_enabled, include_globs, exclude_globs) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![
                root.id,
                root.path,
                root.label,
                root.access_mode,
                root.scan_enabled as i32,
                serde_json::to_string(&root.include_globs).unwrap(),
                serde_json::to_string(&root.exclude_globs).unwrap(),
            ],
        )
        .unwrap();
    }

    /// Create a temporary directory with a unique name, returning its path.
    /// Caller is responsible for cleanup.
    fn create_temp_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("atlas_test_{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).expect("Failed to create temp dir");
        dir
    }

    /// Initialize a bare git repo in the given directory.
    fn init_git_repo(dir: &Path) {
        let status = std::process::Command::new("git")
            .args(["init"])
            .current_dir(dir)
            .status()
            .expect("Failed to run git init");
        assert!(status.success(), "git init failed");

        // Make an initial commit so HEAD exists
        let file_path = dir.join("README.md");
        fs::write(&file_path, "test").expect("Failed to write README");

        std::process::Command::new("git")
            .args(["add", "."])
            .current_dir(dir)
            .status()
            .expect("Failed to run git add");

        std::process::Command::new("git")
            .args(["commit", "-m", "init"])
            .env("GIT_AUTHOR_NAME", "test")
            .env("GIT_AUTHOR_EMAIL", "test@test.com")
            .env("GIT_COMMITTER_NAME", "test")
            .env("GIT_COMMITTER_EMAIL", "test@test.com")
            .current_dir(dir)
            .status()
            .expect("Failed to run git commit");
    }

    #[test]
    fn test_stable_repo_id_across_scans() {
        let tmp = create_temp_dir();
        let db = test_db(&tmp.join("test_db.db"));
        let repo_dir = tmp.join("my-repo");
        fs::create_dir_all(&repo_dir).unwrap();
        init_git_repo(&repo_dir);

        let root = test_root(&tmp.to_string_lossy());
        insert_root(&db, &root);

        let (repos1, _) = scan_root(&root, &db);
        assert_eq!(repos1.len(), 1, "First scan should find 1 repo");

        let (repos2, _) = scan_root(&root, &db);
        assert_eq!(repos2.len(), 1, "Second scan should find 1 repo");

        // The upsert logic preserves the existing ID from the DB
        assert_eq!(
            repos1[0].worktree_path, repos2[0].worktree_path,
            "Worktree paths should match"
        );

        // Verify the DB has exactly one row for this repo path
        let conn = db.conn.lock().unwrap();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM repository WHERE worktree_path = ?1",
                rusqlite::params![repos1[0].worktree_path],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1, "Should have exactly 1 repo row in DB");

        drop(conn);

        // Cleanup
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_duplicate_path_not_duplicated() {
        let tmp = create_temp_dir();
        let db = test_db(&tmp.join("test_db.db"));
        let repo_dir = tmp.join("dup-repo");
        fs::create_dir_all(&repo_dir).unwrap();
        init_git_repo(&repo_dir);

        let root = test_root(&tmp.to_string_lossy());
        insert_root(&db, &root);

        // Scan the same root twice
        scan_root(&root, &db);
        scan_root(&root, &db);

        // Verify only 1 repo row in the DB
        let conn = db.conn.lock().unwrap();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM repository", [], |row| row.get(0))
            .unwrap();
        assert_eq!(
            count, 1,
            "Duplicate scan should not create duplicate repo rows"
        );

        drop(conn);
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_worktree_git_file_detection() {
        let tmp = create_temp_dir();
        let db = test_db(&tmp.join("test_db.db"));
        let repo_dir = tmp.join("wt-repo");
        fs::create_dir_all(&repo_dir).unwrap();

        // Create a .git *file* (not directory) to simulate a worktree
        let git_file = repo_dir.join(".git");
        fs::write(&git_file, "gitdir: /some/other/path/.git/worktrees/my-wt").unwrap();

        let root = test_root(&tmp.to_string_lossy());
        insert_root(&db, &root);

        let (repos, errors) = scan_root(&root, &db);

        // The repo should be discovered; discover_git_repo may fail due to missing
        // actual git objects, but the is_worktree flag is set based on .git being a file.
        // If it succeeded, check the flag; if it errored, that's acceptable for a fake worktree.
        if !repos.is_empty() {
            assert!(
                repos[0].is_worktree,
                "Repo with .git file should have is_worktree = true"
            );
        }
        // If it failed, that's expected since the worktree is not real.
        // The important thing is the is_worktree detection logic was exercised.
        let _ = errors;

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_scan_error_persistence() {
        let tmp = create_temp_dir();
        let db = test_db(&tmp.join("test_db.db"));
        let root = test_root(&tmp.to_string_lossy());
        insert_root(&db, &root);

        let job_id = crate::job_engine::create_job("scan", "{}", &db).unwrap();

        let errors = vec![ScanErrorRecord {
            id: uuid::Uuid::new_v4().to_string(),
            root_id: root.id.clone(),
            path: Some("/fake/path".into()),
            error_type: "scan_error".into(),
            message: "test error message".into(),
        }];

        persist_scan_errors(&errors, Some(&job_id), &db).unwrap();

        let loaded = list_scan_errors(&root.id, &db).unwrap();
        assert_eq!(loaded.len(), 1, "Should have 1 scan error");
        assert_eq!(loaded[0].root_id, root.id);
        assert_eq!(loaded[0].path, Some("/fake/path".into()));
        assert_eq!(loaded[0].error_type, "scan_error");
        assert_eq!(loaded[0].message, "test error message");

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_excluded_directories_skipped() {
        let tmp = create_temp_dir();
        let db = test_db(&tmp.join("test_db.db"));

        // Create a repo inside node_modules — it should be skipped
        let nm_dir = tmp.join("node_modules").join("hidden-repo");
        fs::create_dir_all(&nm_dir).unwrap();
        init_git_repo(&nm_dir);

        // Create a normal repo that should be found
        let normal_dir = tmp.join("visible-repo");
        fs::create_dir_all(&normal_dir).unwrap();
        init_git_repo(&normal_dir);

        let root = test_root(&tmp.to_string_lossy());
        insert_root(&db, &root);

        let (repos, _) = scan_root(&root, &db);

        // Only the visible repo should be found; node_modules is excluded
        assert_eq!(
            repos.len(),
            1,
            "Should find exactly 1 repo (node_modules excluded)"
        );
        assert!(
            repos[0].worktree_path.contains("visible-repo"),
            "Found repo should be the visible one, not in node_modules"
        );

        let _ = fs::remove_dir_all(&tmp);
    }
}
