use crate::ai_fix;
use crate::ai_provider;
use crate::auditor;
use crate::db::Db;
use crate::github;
use crate::indexer;
use crate::job_engine;
use crate::models::*;
use crate::permissions;
use crate::profiler;
use crate::scanner::{self, scan_root};
use crate::tool_broker;
use crate::workspace;
use std::sync::Arc;
use tauri::State;

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Db>,
    pub jobs: Arc<job_engine::JobRuntime>,
}

// --- Greeting (test IPC) ---

#[tauri::command]
pub fn greet(name: &str) -> String {
    format!("Hello, {}! Welcome to AtlasForge.", name)
}

// --- Workspace Root commands ---

#[tauri::command]
pub fn list_workspace_roots(state: State<AppState>) -> Result<Vec<WorkspaceRoot>, String> {
    workspace::load_workspace_roots(&state.db)
}

#[tauri::command]
pub fn add_workspace_root(
    state: State<AppState>,
    input: AddRootInput,
) -> Result<WorkspaceRoot, String> {
    workspace::validate_root_settings(&input)?;
    let path = std::path::Path::new(&input.path);
    if !path.exists() {
        return Err(format!("Path does not exist: {}", input.path));
    }
    if !path.is_dir() {
        return Err(format!("Path is not a directory: {}", input.path));
    }

    let canonical = path
        .canonicalize()
        .map_err(|e| format!("Cannot resolve path: {}", e))?
        .to_string_lossy()
        .to_string();

    // Check for duplicate root (by canonical path)
    {
        let conn = state.db.conn.lock().map_err(|e| e.to_string())?;
        let existing: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM workspace_root WHERE path = ?1",
                rusqlite::params![canonical],
                |row| row.get(0),
            )
            .unwrap_or(0);
        drop(conn);
        if existing > 0 {
            return Err(format!(
                "A workspace root already exists for path: {}",
                canonical
            ));
        }
    }

    let id = uuid::Uuid::new_v4().to_string();
    let include_globs = input.include_globs.clone();
    let exclude_globs = workspace::effective_exclude_globs(&input);
    let include_globs_json = serde_json::to_string(&include_globs).map_err(|e| e.to_string())?;
    let exclude_globs_json = serde_json::to_string(&exclude_globs).map_err(|e| e.to_string())?;
    let label = if input.label.is_empty() {
        path.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| canonical.clone())
    } else {
        input.label
    };

    let conn = state.db.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO workspace_root (id, path, label, access_mode, scan_enabled, include_globs, exclude_globs) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        rusqlite::params![
            id, canonical, label, input.access_mode,
            input.scan_enabled as i32, include_globs_json, exclude_globs_json,
        ],
    )
    .map_err(|e| e.to_string())?;

    drop(conn);
    write_audit(
        &state,
        "add_workspace_root",
        &id,
        &canonical,
        "fs.read",
        "low",
        &format!("Added root: {} ({})", label, canonical),
    )?;

    Ok(WorkspaceRoot {
        id,
        path: canonical,
        label,
        access_mode: input.access_mode,
        scan_enabled: input.scan_enabled,
        include_globs,
        exclude_globs,
        created_at: chrono::Utc::now().to_rfc3339(),
        last_scanned_at: None,
    })
}

#[tauri::command]
pub fn remove_workspace_root(state: State<AppState>, id: String) -> Result<(), String> {
    let mut conn = state.db.conn.lock().map_err(|e| e.to_string())?;
    let path: String = conn
        .query_row(
            "SELECT path FROM workspace_root WHERE id = ?1",
            rusqlite::params![id],
            |row| row.get(0),
        )
        .map_err(|e| format!("Root not found: {}", e))?;

    let tx = conn.transaction().map_err(|e| e.to_string())?;
    tx.execute(
        "DELETE FROM verification_run WHERE repo_id IN (
            SELECT repository.id
            FROM repository
            JOIN project_asset ON project_asset.id = repository.asset_id
            WHERE project_asset.root_id = ?1
        )",
        rusqlite::params![id],
    )
    .map_err(|e| e.to_string())?;
    tx.execute(
        "DELETE FROM repository WHERE asset_id IN (
            SELECT id FROM project_asset WHERE root_id = ?1
        )",
        rusqlite::params![id],
    )
    .map_err(|e| e.to_string())?;
    tx.execute(
        "DELETE FROM project_asset WHERE root_id = ?1",
        rusqlite::params![id],
    )
    .map_err(|e| e.to_string())?;
    tx.execute(
        "DELETE FROM scan_error WHERE root_id = ?1",
        rusqlite::params![id],
    )
    .map_err(|e| e.to_string())?;
    tx.execute(
        "DELETE FROM workspace_root WHERE id = ?1",
        rusqlite::params![id],
    )
    .map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;

    drop(conn);
    write_audit(
        &state,
        "remove_workspace_root",
        &id,
        &path,
        "fs.read",
        "low",
        &format!("Removed root: {}", path),
    )?;
    Ok(())
}

#[tauri::command]
pub fn update_workspace_root(
    state: State<AppState>,
    id: String,
    updates: AddRootInput,
) -> Result<WorkspaceRoot, String> {
    workspace::validate_root_settings(&updates)?;
    let exclude_globs = workspace::effective_exclude_globs(&updates);
    let include_globs_json =
        serde_json::to_string(&updates.include_globs).map_err(|e| e.to_string())?;
    let exclude_globs_json = serde_json::to_string(&exclude_globs).map_err(|e| e.to_string())?;

    let conn = state.db.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE workspace_root SET label = ?1, access_mode = ?2, scan_enabled = ?3, include_globs = ?4, exclude_globs = ?5 WHERE id = ?6",
        rusqlite::params![
            updates.label, updates.access_mode, updates.scan_enabled as i32,
            include_globs_json, exclude_globs_json, id,
        ],
    )
    .map_err(|e| e.to_string())?;

    drop(conn);
    let roots = list_workspace_roots(state)?;
    roots
        .into_iter()
        .find(|r| r.id == id)
        .ok_or_else(|| "Root not found after update".into())
}

// --- Repository commands ---

#[tauri::command]
pub fn list_repositories(state: State<AppState>) -> Result<Vec<Repository>, String> {
    let conn = state.db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT r.id, r.asset_id, r.worktree_path, r.git_dir_path, r.is_bare, r.is_worktree, r.default_branch, r.current_branch, r.head_sha, r.remote_origin_url, r.dirty_state, r.ahead_behind, r.last_commit_at FROM repository r JOIN project_asset a ON a.id = r.asset_id WHERE a.is_available = 1 ORDER BY r.worktree_path")
        .map_err(|e| e.to_string())?;

    let repos = stmt
        .query_map([], |row| {
            let ahead_behind_str: Option<String> = row.get(11)?;
            let ahead_behind: Option<AheadBehind> = ahead_behind_str
                .as_deref()
                .and_then(|s| serde_json::from_str(s).ok());
            Ok(Repository {
                id: row.get(0)?,
                asset_id: row.get(1)?,
                worktree_path: row.get(2)?,
                git_dir_path: row.get(3)?,
                is_bare: row.get::<_, i32>(4)? != 0,
                is_worktree: row.get::<_, i32>(5)? != 0,
                default_branch: row.get(6)?,
                current_branch: row.get(7)?,
                head_sha: row.get(8)?,
                remote_origin_url: row.get(9)?,
                dirty_state: row.get::<_, i32>(10)? != 0,
                ahead_behind,
                last_commit_at: row.get(12)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(repos)
}

#[tauri::command]
pub fn list_repository_summaries(state: State<AppState>) -> Result<Vec<RepositorySummary>, String> {
    let conn = state.db.conn.lock().map_err(|error| error.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT
                r.id, r.asset_id, r.worktree_path, r.git_dir_path, r.is_bare, r.is_worktree,
                r.default_branch, r.current_branch, r.head_sha, r.remote_origin_url,
                r.dirty_state, r.ahead_behind, r.last_commit_at,
                p.id, p.languages, p.frameworks, p.package_managers, p.scripts,
                p.ci_systems, p.has_readme, p.has_license, p.license_type, p.detected_at,
                (SELECT score FROM repo_health_snapshot h
                 WHERE h.repo_id = r.id ORDER BY h.created_at DESC LIMIT 1),
                (SELECT success FROM verification_run v
                 WHERE v.repo_id = r.id ORDER BY v.created_at DESC LIMIT 1)
             FROM repository r
             JOIN project_asset a ON a.id = r.asset_id AND a.is_available = 1
             LEFT JOIN repo_profile p ON p.repo_id = r.id
             ORDER BY r.worktree_path",
        )
        .map_err(|error| error.to_string())?;
    let summaries = stmt
        .query_map([], |row| {
            let ahead_behind = row
                .get::<_, Option<String>>(11)?
                .as_deref()
                .and_then(|value| serde_json::from_str(value).ok());
            let profile_id: Option<String> = row.get(13)?;
            let profile = profile_id.map(|id| RepoProfile {
                id,
                repo_id: row.get(0).unwrap_or_default(),
                languages: row
                    .get::<_, String>(14)
                    .ok()
                    .and_then(|value| serde_json::from_str(&value).ok())
                    .unwrap_or_default(),
                frameworks: row
                    .get::<_, String>(15)
                    .ok()
                    .and_then(|value| serde_json::from_str(&value).ok())
                    .unwrap_or_default(),
                package_managers: row
                    .get::<_, String>(16)
                    .ok()
                    .and_then(|value| serde_json::from_str(&value).ok())
                    .unwrap_or_default(),
                scripts: row
                    .get::<_, String>(17)
                    .ok()
                    .and_then(|value| serde_json::from_str(&value).ok())
                    .unwrap_or_default(),
                ci_systems: row
                    .get::<_, String>(18)
                    .ok()
                    .and_then(|value| serde_json::from_str(&value).ok())
                    .unwrap_or_default(),
                has_readme: row.get::<_, i32>(19).unwrap_or_default() != 0,
                has_license: row.get::<_, i32>(20).unwrap_or_default() != 0,
                license_type: row.get(21).ok().flatten(),
                detected_at: row.get(22).unwrap_or_default(),
            });
            Ok(RepositorySummary {
                repository: Repository {
                    id: row.get(0)?,
                    asset_id: row.get(1)?,
                    worktree_path: row.get(2)?,
                    git_dir_path: row.get(3)?,
                    is_bare: row.get::<_, i32>(4)? != 0,
                    is_worktree: row.get::<_, i32>(5)? != 0,
                    default_branch: row.get(6)?,
                    current_branch: row.get(7)?,
                    head_sha: row.get(8)?,
                    remote_origin_url: row.get(9)?,
                    dirty_state: row.get::<_, i32>(10)? != 0,
                    ahead_behind,
                    last_commit_at: row.get(12)?,
                },
                profile,
                health_score: row.get(23)?,
                last_verification_success: row.get::<_, Option<i32>>(24)?.map(|value| value != 0),
            })
        })
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    Ok(summaries)
}

#[tauri::command]
pub fn list_project_assets(
    state: State<AppState>,
    limit: Option<i64>,
) -> Result<Vec<ProjectAsset>, String> {
    let limit = limit.unwrap_or(200).clamp(1, 1_000);
    let conn = state.db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT id, root_id, path, kind, name, primary_language, last_observed_at FROM project_asset WHERE is_available = 1 ORDER BY last_observed_at DESC LIMIT ?1")
        .map_err(|e| e.to_string())?;

    let assets = stmt
        .query_map(rusqlite::params![limit], |row| {
            Ok(ProjectAsset {
                id: row.get(0)?,
                root_id: row.get(1)?,
                path: row.get(2)?,
                kind: row.get(3)?,
                name: row.get(4)?,
                primary_language: row.get(5)?,
                last_observed_at: row.get(6)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(assets)
}

// --- Job/Scan commands ---

#[tauri::command]
pub async fn start_scan(
    state: State<'_, AppState>,
    root_ids: Option<Vec<String>>,
) -> Result<ScanResult, String> {
    let roots = list_workspace_roots(state.clone())?;
    let targets: Vec<WorkspaceRoot> = if let Some(ids) = root_ids {
        roots.into_iter().filter(|r| ids.contains(&r.id)).collect()
    } else {
        roots
    };

    if targets.is_empty() {
        return Err("No workspace roots to scan".into());
    }

    // Create a job
    let input = serde_json::to_string(&targets.iter().map(|r| r.id.clone()).collect::<Vec<_>>())
        .unwrap_or_default();
    let job_id = job_engine::create_job("scan", &input, &state.db)?;
    let cancellation = job_engine::begin_job(&job_id, &state.db, &state.jobs)?;

    job_engine::update_progress(&job_id, 0, targets.len() as i32, &state.db)?;
    job_engine::append_job_event(
        &job_id,
        "scan_started",
        &format!("{{\"rootCount\":{}}}", targets.len()),
        &state.db,
    )?;

    let mut repos_discovered = 0;
    let mut roots_scanned = 0;
    let mut roots_skipped = 0;
    let mut errors = Vec::new();

    for root in &targets {
        if cancellation.load(std::sync::atomic::Ordering::SeqCst) {
            state.jobs.finish(&job_id);
            return Err("Scan cancelled by user".into());
        }
        if !root.scan_enabled {
            roots_skipped += 1;
            job_engine::append_job_event(
                &job_id,
                "root_skipped",
                &serde_json::json!({
                    "rootId": root.id,
                    "label": root.label,
                    "reason": "scan_disabled",
                })
                .to_string(),
                &state.db,
            )?;
            job_engine::update_progress(
                &job_id,
                roots_scanned + roots_skipped,
                targets.len() as i32,
                &state.db,
            )?;
            continue;
        }

        let scan_started_at: String = {
            let conn = state.db.conn.lock().map_err(|e| e.to_string())?;
            conn.query_row("SELECT datetime('now')", [], |row| row.get(0))
                .map_err(|e| e.to_string())?
        };
        let root_for_scan = root.clone();
        let db_for_scan = state.db.clone();
        let scan_result = tauri::async_runtime::spawn_blocking(move || {
            let result = scan_root(&root_for_scan, &db_for_scan);
            for repo in &result.0 {
                if let Err(e) = profiler::profile_repo(&repo.id, &repo.worktree_path, &db_for_scan)
                {
                    log::warn!("Failed to profile repo {}: {}", repo.worktree_path, e);
                }
            }
            result
        })
        .await;
        let (repos, scan_errors) = match scan_result {
            Ok(result) => result,
            Err(error) => {
                let message = format!("Scan worker failed: {}", error);
                state.jobs.finish(&job_id);
                job_engine::fail_job(&job_id, &message, &state.db)?;
                return Err(message);
            }
        };
        if cancellation.load(std::sync::atomic::Ordering::SeqCst) {
            state.jobs.finish(&job_id);
            return Err("Scan cancelled by user".into());
        }
        roots_scanned += 1;
        repos_discovered += repos.len();

        // Replace current root diagnostics; the job event stream retains history.
        scanner::clear_scan_errors(&root.id, &state.db)?;
        if !scan_errors.is_empty() {
            if let Err(e) = scanner::persist_scan_errors(&scan_errors, Some(&job_id), &state.db) {
                log::warn!("Failed to persist scan errors for root {}: {}", root.id, e);
            }
        }

        let unavailable_assets = if scan_errors.is_empty() {
            scanner::reconcile_root_assets(&root.id, &scan_started_at, &state.db)?
        } else {
            0
        };

        if scan_errors.is_empty() {
            job_engine::append_job_event(
                &job_id,
                "root_scanned",
                &serde_json::json!({
                    "rootId": root.id,
                    "label": root.label,
                    "reposFound": repos.len(),
                    "assetsMarkedUnavailable": unavailable_assets,
                })
                .to_string(),
                &state.db,
            )?;
        } else {
            errors.extend(scan_errors.iter().map(|e| e.message.clone()));
            job_engine::append_job_event(
                &job_id,
                "root_scan_error",
                &serde_json::json!({
                    "rootId": root.id,
                    "label": root.label,
                    "reposFound": repos.len(),
                    "errors": scan_errors.iter().map(|e| &e.message).collect::<Vec<_>>(),
                })
                .to_string(),
                &state.db,
            )?;
        }

        // Update last_scanned_at
        let conn = state.db.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "UPDATE workspace_root SET last_scanned_at = datetime('now') WHERE id = ?1",
            rusqlite::params![root.id],
        )
        .map_err(|e| e.to_string())?;
        drop(conn);

        job_engine::update_progress(
            &job_id,
            roots_scanned + roots_skipped,
            targets.len() as i32,
            &state.db,
        )?;
    }

    // Mark job as completed via job_engine
    job_engine::complete_job(&job_id, &state.db)?;
    state.jobs.finish(&job_id);

    job_engine::append_job_event(
        &job_id,
        "scan_summary",
        &serde_json::json!({
            "reposDiscovered": repos_discovered,
            "rootsScanned": roots_scanned,
            "rootsSkipped": roots_skipped,
            "errorCount": errors.len(),
        })
        .to_string(),
        &state.db,
    )?;

    write_audit(
        &state,
        "start_scan",
        &job_id,
        "system",
        "fs.read",
        "low",
        &format!(
            "Scan completed: {} repos found, {} roots scanned, {} roots skipped, {} errors",
            repos_discovered,
            roots_scanned,
            roots_skipped,
            errors.len()
        ),
    )?;

    Ok(ScanResult {
        job_id,
        repos_discovered,
        errors,
    })
}

#[tauri::command]
pub fn list_jobs(state: State<AppState>, limit: i64) -> Result<Vec<Job>, String> {
    let limit = limit.clamp(1, 500);
    let conn = state.db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT id, type, status, input, created_at, updated_at, completed_at, error_message, progress, progress_total, parent_job_id FROM job ORDER BY created_at DESC LIMIT ?1")
        .map_err(|e| e.to_string())?;

    let jobs = stmt
        .query_map(rusqlite::params![limit], |row| {
            Ok(Job {
                id: row.get(0)?,
                job_type: row.get(1)?,
                status: row.get(2)?,
                input: row.get(3)?,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
                completed_at: row.get(6)?,
                error_message: row.get(7)?,
                progress: row.get::<_, i32>(8)?,
                progress_total: row.get::<_, i32>(9)?,
                parent_job_id: row.get(10)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(jobs)
}

#[tauri::command]
pub fn list_jobs_by_type_cmd(
    state: State<AppState>,
    job_type: String,
    limit: Option<i64>,
) -> Result<Vec<Job>, String> {
    job_engine::list_jobs_by_type(&job_type, limit.unwrap_or(50).clamp(1, 500), &state.db)
}

#[tauri::command]
pub fn get_job_events(state: State<AppState>, job_id: String) -> Result<Vec<JobEvent>, String> {
    let conn = state.db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT id, job_id, seq, type, payload, created_at FROM job_event WHERE job_id = ?1 ORDER BY seq")
        .map_err(|e| e.to_string())?;

    let events = stmt
        .query_map(rusqlite::params![job_id], |row| {
            Ok(JobEvent {
                id: row.get(0)?,
                job_id: row.get(1)?,
                seq: row.get(2)?,
                event_type: row.get(3)?,
                payload: row.get(4)?,
                created_at: row.get(5)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(events)
}

#[tauri::command]
pub fn list_audit_log_cmd(
    state: State<AppState>,
    limit: Option<i64>,
) -> Result<Vec<AuditEntry>, String> {
    let limit = limit.unwrap_or(100).clamp(1, 500);
    let conn = state.db.conn.lock().map_err(|error| error.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT id, action, subject, scope, capability, risk_level, detail, created_at
             FROM audit_log ORDER BY created_at DESC LIMIT ?1",
        )
        .map_err(|error| error.to_string())?;
    let entries = stmt
        .query_map(rusqlite::params![limit], |row| {
            Ok(AuditEntry {
                id: row.get(0)?,
                action: row.get(1)?,
                subject: row.get(2)?,
                scope: row.get(3)?,
                capability: row.get(4)?,
                risk_level: row.get(5)?,
                detail: row.get(6)?,
                created_at: row.get(7)?,
            })
        })
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    Ok(entries)
}

// --- Repo Profile commands ---

#[tauri::command]
pub fn get_repo_profile(
    state: State<AppState>,
    repo_id: String,
) -> Result<Option<RepoProfile>, String> {
    profiler::load_profile(&repo_id, &state.db)
}

#[tauri::command]
pub async fn refresh_profiles(state: State<'_, AppState>) -> Result<usize, String> {
    let repos = list_repositories(state.clone())?;
    let db = state.db.clone();
    let refreshed = tauri::async_runtime::spawn_blocking(move || {
        let mut refreshed = 0;
        for repo in &repos {
            match profiler::profile_repo(&repo.id, &repo.worktree_path, &db) {
                Ok(_) => refreshed += 1,
                Err(e) => log::warn!("Failed to profile repo {}: {}", repo.worktree_path, e),
            }
        }
        refreshed
    })
    .await
    .map_err(|error| format!("Profile worker failed: {error}"))?;

    write_audit(
        &state,
        "refresh_profiles",
        "system",
        "all",
        "fs.read",
        "low",
        &format!("Refreshed {} profiles", refreshed),
    )?;

    Ok(refreshed)
}

#[tauri::command]
pub fn list_repo_profiles_cmd(state: State<AppState>) -> Result<Vec<RepoProfile>, String> {
    profiler::load_all_profiles(&state.db)
}

// --- Job Engine commands ---

#[tauri::command]
pub fn cancel_job_cmd(state: State<AppState>, job_id: String) -> Result<Job, String> {
    job_engine::cancel_job(&job_id, &state.db, &state.jobs)
}

#[tauri::command]
pub fn retry_job_cmd(state: State<AppState>, job_id: String) -> Result<Job, String> {
    let original = job_engine::load_job(&job_id, &state.db)?;
    if !matches!(
        original.job_type.as_str(),
        "scan" | "reindex" | "audit" | "github_sync"
    ) {
        return Err(format!(
            "Jobs of type '{}' require fresh input or approval and must be restarted from their feature page",
            original.job_type
        ));
    }
    let retried = job_engine::retry_job(&job_id, &state.db)?;
    let state = state.inner().clone();
    let queued_job = retried.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let _ = dispatch_queued_job(&queued_job, &state);
    });
    Ok(retried)
}

#[tauri::command]
pub fn get_job_detail(state: State<AppState>, job_id: String) -> Result<Job, String> {
    job_engine::load_job(&job_id, &state.db)
}

fn dispatch_queued_job(job: &Job, state: &AppState) -> Result<(), String> {
    let cancellation = job_engine::begin_job(&job.id, &state.db, &state.jobs)?;
    let result = match job.job_type.as_str() {
        "scan" => {
            let root_ids: Vec<String> =
                serde_json::from_str(&job.input).map_err(|error| error.to_string())?;
            let roots = workspace::load_workspace_roots(&state.db)?;
            let targets = roots
                .into_iter()
                .filter(|root| root_ids.contains(&root.id))
                .collect::<Vec<_>>();
            job_engine::update_progress(&job.id, 0, targets.len() as i32, &state.db)?;
            for (index, root) in targets.iter().enumerate() {
                if cancellation.load(std::sync::atomic::Ordering::SeqCst) {
                    return Ok(());
                }
                if root.scan_enabled {
                    let scan_started_at: String = {
                        let conn = state.db.conn.lock().map_err(|e| e.to_string())?;
                        conn.query_row("SELECT datetime('now')", [], |row| row.get(0))
                            .map_err(|e| e.to_string())?
                    };
                    let (_, errors) = scan_root(root, &state.db);
                    scanner::clear_scan_errors(&root.id, &state.db)?;
                    if !errors.is_empty() {
                        scanner::persist_scan_errors(&errors, Some(&job.id), &state.db)?;
                    } else {
                        scanner::reconcile_root_assets(&root.id, &scan_started_at, &state.db)?;
                    }
                }
                job_engine::update_progress(
                    &job.id,
                    (index + 1) as i32,
                    targets.len() as i32,
                    &state.db,
                )?;
            }
            Ok(())
        }
        "reindex" => {
            let repo_id = job_input_string(&job.input, "repoId")?;
            let path = workspace::repository_path(&repo_id, &state.db)?;
            indexer::index_repo(&repo_id, &path, &state.db).map(|_| ())
        }
        "audit" => {
            let repo_id = job_input_string(&job.input, "repoId")?;
            let path = workspace::repository_path(&repo_id, &state.db)?;
            auditor::audit_repo(&repo_id, &path, None, &state.db).map(|_| ())
        }
        "github_sync" => {
            let repo_id = job_input_string(&job.input, "repoId")?;
            let integration = github::load_integration(&repo_id, &state.db)?
                .ok_or_else(|| "GitHub integration is no longer available".to_string())?;
            github::sync_workflow_runs(&integration, &state.db)?;
            github::sync_prs(&integration, &state.db)?;
            github::sync_releases(&integration, &state.db)?;
            Ok(())
        }
        _ => Err(format!("Unsupported queued job type: {}", job.job_type)),
    };
    if cancellation.load(std::sync::atomic::Ordering::SeqCst) {
        state.jobs.finish(&job.id);
        return Ok(());
    }
    match result {
        Ok(()) => job_engine::complete_job(&job.id, &state.db)?,
        Err(ref error) => job_engine::fail_job(&job.id, error, &state.db)?,
    }
    state.jobs.finish(&job.id);
    result
}

fn job_input_string(input: &str, key: &str) -> Result<String, String> {
    serde_json::from_str::<serde_json::Value>(input)
        .map_err(|error| error.to_string())?
        .get(key)
        .and_then(|value| value.as_str())
        .map(ToOwned::to_owned)
        .ok_or_else(|| format!("Job input is missing '{}'", key))
}

#[tauri::command]
pub fn request_verification_approval_cmd(
    state: State<AppState>,
    repo_id: String,
    cwd: String,
    command: String,
) -> Result<permissions::PermissionRequest, String> {
    permissions::request_verification(&repo_id, &cwd, &command, &state.db)
}

#[tauri::command]
pub fn request_patch_approval_cmd(
    state: State<AppState>,
    proposal_id: String,
) -> Result<permissions::PermissionRequest, String> {
    permissions::request_patch(&proposal_id, &state.db)
}

#[tauri::command]
pub fn request_rollback_approval_cmd(
    state: State<AppState>,
    proposal_id: String,
) -> Result<permissions::PermissionRequest, String> {
    permissions::request_rollback(&proposal_id, &state.db)
}

#[tauri::command]
pub fn decide_permission_request_cmd(
    state: State<AppState>,
    request_id: String,
    approved: bool,
) -> Result<permissions::PermissionRequest, String> {
    permissions::decide_request(&request_id, approved, &state.db)
}

#[tauri::command]
pub fn list_permission_requests_cmd(
    state: State<AppState>,
    status: Option<String>,
) -> Result<Vec<permissions::PermissionRequest>, String> {
    permissions::list_requests(status.as_deref(), &state.db)
}

// --- Tool Broker commands ---

#[tauri::command]
pub fn list_tools_cmd(state: State<AppState>) -> Result<Vec<tool_broker::ToolInfo>, String> {
    tool_broker::list_tools(&state.db)
}

#[tauri::command]
pub fn invoke_tool_cmd(
    state: State<AppState>,
    job_id: String,
    tool_name: String,
    input: serde_json::Value,
    dry_run: Option<bool>,
) -> Result<tool_broker::ToolResult, String> {
    let dry_run = dry_run.unwrap_or(true); // Default to dry-run for safety
    tool_broker::invoke_tool(&job_id, &tool_name, &input, dry_run, &state.db)
}

#[tauri::command]
pub fn list_invocations_cmd(
    state: State<AppState>,
    job_id: String,
) -> Result<Vec<serde_json::Value>, String> {
    tool_broker::list_invocations(&job_id, &state.db)
}

// --- Indexer commands ---

#[tauri::command]
pub fn search_index_cmd(
    state: State<AppState>,
    query: String,
    limit: Option<i64>,
    repo_id: Option<String>,
) -> Result<Vec<indexer::SearchResult>, String> {
    let rid = repo_id.as_deref();
    indexer::search(&query, limit.unwrap_or(20), rid, &state.db)
}

#[tauri::command]
pub fn list_documents_cmd(
    state: State<AppState>,
    repo_id: String,
) -> Result<Vec<serde_json::Value>, String> {
    indexer::list_documents(&repo_id, &state.db)
}

#[tauri::command]
pub async fn reindex_repo_cmd(
    state: State<'_, AppState>,
    repo_id: String,
) -> Result<indexer::IndexStats, String> {
    // Get worktree path
    let worktree_path: String = {
        let conn = state.db.conn.lock().map_err(|e| e.to_string())?;
        conn.query_row(
            "SELECT worktree_path FROM repository WHERE id = ?1",
            rusqlite::params![repo_id],
            |row| row.get(0),
        )
        .map_err(|e| format!("Repository not found: {}", e))?
    };

    // Create a job
    let job_id = job_engine::create_job(
        "reindex",
        &serde_json::json!({"repoId": repo_id}).to_string(),
        &state.db,
    )?;
    let cancellation = job_engine::begin_job(&job_id, &state.db, &state.jobs)?;
    if cancellation.load(std::sync::atomic::Ordering::SeqCst) {
        state.jobs.finish(&job_id);
        return Err("Reindex cancelled by user".into());
    }

    job_engine::append_job_event(
        &job_id,
        "reindex_started",
        &serde_json::json!({"repoId": repo_id}).to_string(),
        &state.db,
    )?;

    let repo_for_index = repo_id.clone();
    let path_for_index = worktree_path.clone();
    let db_for_index = state.db.clone();
    let index_result = tauri::async_runtime::spawn_blocking(move || {
        indexer::index_repo(&repo_for_index, &path_for_index, &db_for_index)
    })
    .await
    .unwrap_or_else(|e| Err(format!("Index worker failed: {}", e)));

    if cancellation.load(std::sync::atomic::Ordering::SeqCst) {
        state.jobs.finish(&job_id);
        return Err("Reindex cancelled by user".into());
    }

    match index_result {
        Ok(stats) => {
            job_engine::complete_job(&job_id, &state.db)?;
            state.jobs.finish(&job_id);
            job_engine::append_job_event(
                &job_id,
                "reindex_completed",
                &serde_json::json!({"documents": stats.documents, "chunks": stats.chunks})
                    .to_string(),
                &state.db,
            )?;

            write_audit(
                &state,
                "reindex_repo",
                &repo_id,
                &worktree_path,
                "fs.read",
                "low",
                &format!(
                    "Indexed {} documents, {} chunks",
                    stats.documents, stats.chunks
                ),
            )?;

            Ok(stats)
        }
        Err(e) => {
            job_engine::fail_job(&job_id, &e, &state.db)?;
            state.jobs.finish(&job_id);
            Err(e)
        }
    }
}

// --- Auditor commands ---

#[tauri::command]
pub async fn audit_repo_cmd(
    state: State<'_, AppState>,
    repo_id: String,
) -> Result<auditor::HealthSnapshot, String> {
    let worktree_path: String = {
        let conn = state.db.conn.lock().map_err(|e| e.to_string())?;
        conn.query_row(
            "SELECT worktree_path FROM repository WHERE id = ?1",
            rusqlite::params![repo_id],
            |row| row.get(0),
        )
        .map_err(|e| format!("Repository not found: {}", e))?
    };

    // Create a job
    let job_id = job_engine::create_job(
        "audit",
        &serde_json::json!({"repoId": repo_id}).to_string(),
        &state.db,
    )?;
    let cancellation = job_engine::begin_job(&job_id, &state.db, &state.jobs)?;
    if cancellation.load(std::sync::atomic::Ordering::SeqCst) {
        state.jobs.finish(&job_id);
        return Err("Audit cancelled by user".into());
    }

    job_engine::append_job_event(
        &job_id,
        "audit_started",
        &serde_json::json!({"repoId": repo_id}).to_string(),
        &state.db,
    )?;

    let repo_for_audit = repo_id.clone();
    let path_for_audit = worktree_path.clone();
    let db_for_audit = state.db.clone();
    let audit_result = tauri::async_runtime::spawn_blocking(move || {
        auditor::audit_repo(&repo_for_audit, &path_for_audit, None, &db_for_audit)
    })
    .await
    .unwrap_or_else(|e| Err(format!("Audit worker failed: {}", e)));

    if cancellation.load(std::sync::atomic::Ordering::SeqCst) {
        state.jobs.finish(&job_id);
        return Err("Audit cancelled by user".into());
    }

    match audit_result {
        Ok(snapshot) => {
            job_engine::complete_job(&job_id, &state.db)?;
            state.jobs.finish(&job_id);
            job_engine::append_job_event(
                &job_id,
                "audit_completed",
                &serde_json::json!({"score": snapshot.score, "repoId": repo_id}).to_string(),
                &state.db,
            )?;

            write_audit(
                &state,
                "audit_repo",
                &repo_id,
                &worktree_path,
                "fs.read",
                "low",
                &format!("Audit score: {}", snapshot.score),
            )?;

            Ok(snapshot)
        }
        Err(e) => {
            job_engine::fail_job(&job_id, &e, &state.db)?;
            state.jobs.finish(&job_id);
            Err(e)
        }
    }
}

#[tauri::command]
pub fn get_health_snapshot_cmd(
    state: State<AppState>,
    repo_id: String,
) -> Result<Option<auditor::HealthSnapshot>, String> {
    auditor::load_latest_snapshot(&repo_id, &state.db)
}

#[tauri::command]
pub fn get_findings_cmd(
    state: State<AppState>,
    snapshot_id: String,
) -> Result<Vec<auditor::Finding>, String> {
    auditor::load_findings(&snapshot_id, &state.db)
}

// --- AI Provider commands ---

#[tauri::command]
pub fn list_ai_providers_cmd(
    state: State<AppState>,
) -> Result<Vec<ai_provider::AiProvider>, String> {
    ai_provider::list_providers(&state.db)
}

#[tauri::command]
pub fn detect_local_providers_cmd() -> Vec<ai_provider::AiProvider> {
    ai_provider::detect_local_providers()
}

#[tauri::command]
pub fn upsert_ai_provider_cmd(
    state: State<AppState>,
    provider: ai_provider::AiProvider,
) -> Result<(), String> {
    ai_provider::upsert_provider(&provider, &state.db)
}

#[tauri::command]
pub fn delete_ai_provider_cmd(state: State<AppState>, id: String) -> Result<(), String> {
    ai_provider::delete_provider(&id, &state.db)
}

#[tauri::command]
pub async fn probe_ai_provider_cmd(
    state: State<'_, AppState>,
    provider_id: String,
) -> Result<ai_provider::ProviderProbe, String> {
    let provider = ai_provider::list_providers(&state.db)?
        .into_iter()
        .find(|provider| provider.id == provider_id)
        .ok_or_else(|| format!("Provider not found: {}", provider_id))?;
    Ok(ai_provider::probe_provider(&provider).await)
}

#[tauri::command]
pub async fn call_ai_cmd(
    state: State<'_, AppState>,
    provider_id: String,
    prompt: String,
    model: Option<String>,
) -> Result<ai_provider::AiResponse, String> {
    let providers = ai_provider::list_providers(&state.db)?;
    let provider = providers
        .into_iter()
        .find(|p| p.id == provider_id)
        .ok_or_else(|| format!("Provider not found: {}", provider_id))?;
    if !provider.enabled {
        return Err(format!("Provider '{}' is disabled", provider.name));
    }

    // Scan for secrets before sending
    let secrets = ai_provider::scan_for_secrets(&prompt);
    if !secrets.is_empty() {
        return Err(format!(
            "Prompt contains potential secrets ({}). Redact before sending to AI.",
            secrets.len()
        ));
    }

    // Create a job
    let job_id = job_engine::create_job(
        "ai_call",
        &serde_json::json!({"providerId": provider_id}).to_string(),
        &state.db,
    )?;
    let cancellation = job_engine::begin_job(&job_id, &state.db, &state.jobs)?;

    job_engine::append_job_event(
        &job_id,
        "ai_call_started",
        &serde_json::json!({"providerId": provider_id}).to_string(),
        &state.db,
    )?;

    match ai_provider::call_ai(&provider, &prompt, None, model.as_deref()).await {
        Ok(response) => {
            if cancellation.load(std::sync::atomic::Ordering::SeqCst) {
                state.jobs.finish(&job_id);
                return Err("AI call cancelled by user".into());
            }
            job_engine::complete_job(&job_id, &state.db)?;
            state.jobs.finish(&job_id);
            job_engine::append_job_event(&job_id, "ai_call_completed", &serde_json::json!({"tokensIn": response.tokens_in, "tokensOut": response.tokens_out}).to_string(), &state.db)?;
            Ok(response)
        }
        Err(e) => {
            if cancellation.load(std::sync::atomic::Ordering::SeqCst) {
                state.jobs.finish(&job_id);
                return Err("AI call cancelled by user".into());
            }
            job_engine::fail_job(&job_id, &e, &state.db)?;
            state.jobs.finish(&job_id);
            Err(e)
        }
    }
}

// --- AI Fix commands ---

#[tauri::command]
pub fn list_artifacts_cmd(
    state: State<AppState>,
    job_id: String,
) -> Result<Vec<ai_fix::Artifact>, String> {
    ai_fix::list_artifacts(&job_id, &state.db)
}

#[tauri::command]
pub fn list_patch_proposals_cmd(
    state: State<AppState>,
    repo_id: String,
) -> Result<Vec<ai_fix::PatchProposal>, String> {
    ai_fix::list_patch_proposals(&repo_id, &state.db)
}

#[tauri::command]
pub async fn apply_patch_cmd(
    state: State<'_, AppState>,
    proposal_id: String,
    approval_id: String,
) -> Result<ai_fix::PatchProposal, String> {
    let (repo_id, context_hash) = permissions::current_patch_context_hash(&proposal_id, &state.db)?;
    permissions::consume_request(
        &approval_id,
        "fs.write_patch",
        Some(&repo_id),
        &context_hash,
        &state.db,
    )?;
    // Create a job
    let job_id = job_engine::create_job(
        "patch_apply",
        &serde_json::json!({"proposalId": proposal_id}).to_string(),
        &state.db,
    )?;
    let cancellation = job_engine::begin_job(&job_id, &state.db, &state.jobs)?;

    job_engine::append_job_event(
        &job_id,
        "patch_apply_started",
        &serde_json::json!({"proposalId": proposal_id}).to_string(),
        &state.db,
    )?;

    let db = state.db.clone();
    let proposal_for_worker = proposal_id.clone();
    let cancellation_for_worker = cancellation.clone();
    let apply_result = tauri::async_runtime::spawn_blocking(move || {
        ai_fix::apply_patch_cancellable(&proposal_for_worker, &db, &cancellation_for_worker)
    })
    .await
    .unwrap_or_else(|error| Err(format!("Patch worker failed: {error}")));
    match apply_result {
        Ok(proposal) => {
            job_engine::complete_committed_job(&job_id, &state.db)?;
            state.jobs.finish(&job_id);
            job_engine::append_job_event(
                &job_id,
                "patch_apply_completed",
                &serde_json::json!({"proposalId": proposal_id}).to_string(),
                &state.db,
            )?;
            Ok(proposal)
        }
        Err(e) => {
            if cancellation.load(std::sync::atomic::Ordering::SeqCst) {
                state.jobs.finish(&job_id);
                return Err("Patch application cancelled by user".into());
            }
            job_engine::fail_job(&job_id, &e, &state.db)?;
            state.jobs.finish(&job_id);
            Err(e)
        }
    }
}

#[tauri::command]
pub fn reject_patch_cmd(
    state: State<AppState>,
    proposal_id: String,
    reason: String,
) -> Result<(), String> {
    ai_fix::reject_patch(&proposal_id, &reason, &state.db)
}

#[tauri::command]
pub async fn rollback_patch_cmd(
    state: State<'_, AppState>,
    proposal_id: String,
    approval_id: String,
) -> Result<(), String> {
    let (repo_id, context_hash) =
        permissions::current_rollback_context_hash(&proposal_id, &state.db)?;
    permissions::consume_request(
        &approval_id,
        "fs.rollback_patch",
        Some(&repo_id),
        &context_hash,
        &state.db,
    )?;
    let job_id = job_engine::create_job(
        "patch_rollback",
        &serde_json::json!({"proposalId": proposal_id}).to_string(),
        &state.db,
    )?;
    let cancellation = job_engine::begin_job(&job_id, &state.db, &state.jobs)?;
    let db = state.db.clone();
    let proposal_for_worker = proposal_id.clone();
    let cancellation_for_worker = cancellation.clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        ai_fix::rollback_patch_cancellable(&proposal_for_worker, &db, &cancellation_for_worker)
    })
    .await
    .unwrap_or_else(|error| Err(format!("Rollback worker failed: {error}")));
    match result {
        Ok(()) => job_engine::complete_committed_job(&job_id, &state.db)?,
        Err(ref error) => {
            if cancellation.load(std::sync::atomic::Ordering::SeqCst) {
                state.jobs.finish(&job_id);
                return Err("Patch rollback cancelled by user".into());
            }
            job_engine::fail_job(&job_id, error, &state.db)?
        }
    }
    state.jobs.finish(&job_id);
    result
}

// --- AI Fix Plan / Propose commands ---

#[tauri::command]
pub async fn generate_fix_plan_cmd(
    state: State<'_, AppState>,
    repo_id: String,
    snapshot_id: String,
    provider_id: String,
    model: Option<String>,
) -> Result<ai_fix::FixPlan, String> {
    ai_fix::generate_fix_plan(
        &repo_id,
        &snapshot_id,
        &provider_id,
        model.as_deref(),
        &state.db,
        &state.jobs,
    )
    .await
}

#[tauri::command]
pub async fn propose_fix_cmd(
    state: State<'_, AppState>,
    repo_id: String,
    provider_id: String,
    model: Option<String>,
    fix_instruction: String,
    target_file: Option<String>,
) -> Result<ai_fix::PatchProposal, String> {
    ai_fix::propose_fix(
        &repo_id,
        &provider_id,
        model.as_deref(),
        &fix_instruction,
        target_file.as_deref(),
        &state.db,
        &state.jobs,
    )
    .await
}

#[tauri::command]
pub fn list_fix_plans_cmd(
    state: State<AppState>,
    repo_id: String,
) -> Result<Vec<ai_fix::Artifact>, String> {
    ai_fix::list_fix_plans(&repo_id, &state.db)
}

#[tauri::command]
pub fn preview_fix_plan_context_cmd(
    state: State<AppState>,
    repo_id: String,
    snapshot_id: String,
) -> Result<ai_fix::ContextPreview, String> {
    ai_fix::preview_fix_plan_context(&repo_id, &snapshot_id, &state.db)
}

pub mod automation_commands;
pub mod github_commands;
pub mod verification_commands;

// --- Scan Error commands ---

#[tauri::command]
pub fn list_scan_errors_cmd(
    state: State<AppState>,
    root_id: String,
) -> Result<Vec<scanner::ScanErrorRecord>, String> {
    scanner::list_scan_errors(&root_id, &state.db)
}

// --- Helpers ---

pub fn write_audit(
    state: &State<AppState>,
    action: &str,
    subject: &str,
    scope: &str,
    capability: &str,
    risk_level: &str,
    detail: &str,
) -> Result<(), String> {
    let conn = state.db.conn.lock().map_err(|e| e.to_string())?;
    let id = uuid::Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO audit_log (id, action, subject, scope, capability, risk_level, detail) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        rusqlite::params![id, action, subject, scope, capability, risk_level, detail],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

fn ensure_github_write_enabled() -> Result<(), String> {
    Err(
        "GitHub write operations remain disabled until their dedicated preview and approval UI is implemented."
            .into(),
    )
}
