use crate::ai_fix;
use crate::ai_provider;
use crate::auditor;
use crate::automations;
use crate::db::Db;
use crate::github;
use crate::indexer;
use crate::job_engine;
use crate::models::*;
use crate::permissions;
use crate::profiler;
use crate::scanner::{self, scan_root};
use crate::security;
use crate::tool_broker;
use crate::verification;
use std::sync::Arc;
use tauri::State;

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Db>,
    pub jobs: Arc<job_engine::JobRuntime>,
}

const MAX_ROOT_GLOBS: usize = 128;
const MAX_GLOB_LENGTH: usize = 512;

fn validate_root_settings(input: &AddRootInput) -> Result<(), String> {
    if !matches!(input.access_mode.as_str(), "read_only" | "read_write") {
        return Err("Access mode must be 'read_only' or 'read_write'".into());
    }

    for (kind, patterns) in [
        ("include", &input.include_globs),
        ("exclude", &input.exclude_globs),
    ] {
        if patterns.len() > MAX_ROOT_GLOBS {
            return Err(format!(
                "Too many {} globs: maximum is {}",
                kind, MAX_ROOT_GLOBS
            ));
        }
        for pattern in patterns {
            if pattern.trim().is_empty() {
                return Err(format!("{} glob must not be empty", kind));
            }
            if pattern.len() > MAX_GLOB_LENGTH {
                return Err(format!(
                    "{} glob exceeds {} characters",
                    kind, MAX_GLOB_LENGTH
                ));
            }
            glob::Pattern::new(&pattern.replace('\\', "/"))
                .map_err(|err| format!("Invalid {} glob '{}': {}", kind, pattern, err))?;
        }
    }
    Ok(())
}

fn effective_exclude_globs(input: &AddRootInput) -> Vec<String> {
    if input.exclude_globs.is_empty() {
        security::DEFAULT_EXCLUDE_GLOBS
            .iter()
            .map(|pattern| pattern.to_string())
            .collect()
    } else {
        input.exclude_globs.clone()
    }
}

// --- Greeting (test IPC) ---

#[tauri::command]
pub fn greet(name: &str) -> String {
    format!("Hello, {}! Welcome to AtlasForge.", name)
}

// --- Workspace Root commands ---

#[tauri::command]
pub fn list_workspace_roots(state: State<AppState>) -> Result<Vec<WorkspaceRoot>, String> {
    load_workspace_roots(&state.db)
}

fn load_workspace_roots(db: &Db) -> Result<Vec<WorkspaceRoot>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT id, path, label, access_mode, scan_enabled, include_globs, exclude_globs, created_at, last_scanned_at FROM workspace_root ORDER BY created_at")
        .map_err(|e| e.to_string())?;

    let roots = stmt
        .query_map([], |row| {
            let include_globs_str: String = row.get(5)?;
            let exclude_globs_str: String = row.get(6)?;
            let include_globs: Vec<String> =
                serde_json::from_str(&include_globs_str).unwrap_or_default();
            let exclude_globs: Vec<String> =
                serde_json::from_str(&exclude_globs_str).unwrap_or_default();
            Ok(WorkspaceRoot {
                id: row.get(0)?,
                path: row.get(1)?,
                label: row.get(2)?,
                access_mode: row.get(3)?,
                scan_enabled: row.get::<_, i32>(4)? != 0,
                include_globs,
                exclude_globs,
                created_at: row.get(7)?,
                last_scanned_at: row.get(8)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(roots)
}

#[tauri::command]
pub fn add_workspace_root(
    state: State<AppState>,
    input: AddRootInput,
) -> Result<WorkspaceRoot, String> {
    validate_root_settings(&input)?;
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
    let exclude_globs = effective_exclude_globs(&input);
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
    validate_root_settings(&updates)?;
    let exclude_globs = effective_exclude_globs(&updates);
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
        .prepare("SELECT id, asset_id, worktree_path, git_dir_path, is_bare, is_worktree, default_branch, current_branch, head_sha, remote_origin_url, dirty_state, ahead_behind, last_commit_at FROM repository ORDER BY worktree_path")
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
        .prepare("SELECT id, root_id, path, kind, name, primary_language, last_observed_at FROM project_asset ORDER BY last_observed_at DESC LIMIT ?1")
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

        let root_for_scan = root.clone();
        let db_for_scan = state.db.clone();
        let scan_result = tauri::async_runtime::spawn_blocking(move || {
            let result = scan_root(&root_for_scan, &db_for_scan);
            for repo in &result.0 {
                if let Err(e) = profiler::profile_repo(&repo.id, &repo.worktree_path, &db_for_scan) {
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

        // Persist scan errors to DB
        if !scan_errors.is_empty() {
            if let Err(e) = scanner::persist_scan_errors(&scan_errors, Some(&job_id), &state.db) {
                log::warn!("Failed to persist scan errors for root {}: {}", root.id, e);
            }
        }

        if scan_errors.is_empty() {
            job_engine::append_job_event(
                &job_id,
                "root_scanned",
                &serde_json::json!({
                    "rootId": root.id,
                    "label": root.label,
                    "reposFound": repos.len(),
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
    let mut refreshed = 0;

    for repo in &repos {
        match profiler::profile_repo(&repo.id, &repo.worktree_path, &state.db) {
            Ok(_) => refreshed += 1,
            Err(e) => log::warn!("Failed to profile repo {}: {}", repo.worktree_path, e),
        }
    }

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
            let roots = load_workspace_roots(&state.db)?;
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
                    let (_, errors) = scan_root(root, &state.db);
                    if !errors.is_empty() {
                        scanner::persist_scan_errors(&errors, Some(&job.id), &state.db)?;
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
            let path = repository_path(&repo_id, &state.db)?;
            indexer::index_repo(&repo_id, &path, &state.db).map(|_| ())
        }
        "audit" => {
            let repo_id = job_input_string(&job.input, "repoId")?;
            let path = repository_path(&repo_id, &state.db)?;
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

fn repository_path(repo_id: &str, db: &Db) -> Result<String, String> {
    let conn = db.conn.lock().map_err(|error| error.to_string())?;
    conn.query_row(
        "SELECT worktree_path FROM repository WHERE id = ?1",
        rusqlite::params![repo_id],
        |row| row.get(0),
    )
    .map_err(|error| format!("Repository not found: {}", error))
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
    let conn = state.db.conn.lock().map_err(|e| e.to_string())?;
    let worktree_path: String = conn
        .query_row(
            "SELECT worktree_path FROM repository WHERE id = ?1",
            rusqlite::params![repo_id],
            |row| row.get(0),
        )
        .map_err(|e| format!("Repository not found: {}", e))?;
    drop(conn);

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
    let conn = state.db.conn.lock().map_err(|e| e.to_string())?;
    let worktree_path: String = conn
        .query_row(
            "SELECT worktree_path FROM repository WHERE id = ?1",
            rusqlite::params![repo_id],
            |row| row.get(0),
        )
        .map_err(|e| format!("Repository not found: {}", e))?;
    drop(conn);

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
    let _cancellation = job_engine::begin_job(&job_id, &state.db, &state.jobs)?;

    job_engine::append_job_event(
        &job_id,
        "ai_call_started",
        &serde_json::json!({"providerId": provider_id}).to_string(),
        &state.db,
    )?;

    match ai_provider::call_ai(&provider, &prompt, None, model.as_deref()).await {
        Ok(response) => {
            job_engine::complete_job(&job_id, &state.db)?;
            state.jobs.finish(&job_id);
            job_engine::append_job_event(&job_id, "ai_call_completed", &serde_json::json!({"tokensIn": response.tokens_in, "tokensOut": response.tokens_out}).to_string(), &state.db)?;
            Ok(response)
        }
        Err(e) => {
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
    let _cancellation = job_engine::begin_job(&job_id, &state.db, &state.jobs)?;

    job_engine::append_job_event(
        &job_id,
        "patch_apply_started",
        &serde_json::json!({"proposalId": proposal_id}).to_string(),
        &state.db,
    )?;

    match ai_fix::apply_patch(&proposal_id, &state.db) {
        Ok(proposal) => {
            job_engine::complete_job(&job_id, &state.db)?;
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
pub fn rollback_patch_cmd(state: State<AppState>, proposal_id: String) -> Result<(), String> {
    ai_fix::rollback_patch(&proposal_id, &state.db)
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

// --- GitHub commands ---

#[tauri::command]
pub fn check_gh_auth_cmd() -> Result<github::GhAuthStatus, String> {
    github::check_gh_auth()
}

#[tauri::command]
pub fn resolve_github_repo_cmd(
    state: State<AppState>,
    repo_id: String,
) -> Result<github::GitHubIntegration, String> {
    let conn = state.db.conn.lock().map_err(|e| e.to_string())?;
    let worktree_path: String = conn
        .query_row(
            "SELECT worktree_path FROM repository WHERE id = ?1",
            rusqlite::params![repo_id],
            |row| row.get(0),
        )
        .map_err(|e| format!("Repository not found: {}", e))?;
    drop(conn);

    let integration = github::resolve_github_repo(&repo_id, &worktree_path, &state.db)?;

    write_audit(
        &state,
        "resolve_github",
        &repo_id,
        &worktree_path,
        "github.read",
        "none",
        &format!(
            "Resolved GitHub: {}/{}",
            integration.github_owner, integration.github_repo
        ),
    )?;

    Ok(integration)
}

#[tauri::command]
pub fn get_github_evidence_cmd(
    state: State<AppState>,
    repo_id: String,
) -> Result<github::GitHubEvidence, String> {
    github::load_evidence(&repo_id, &state.db)
}

#[tauri::command]
pub fn get_github_integration_cmd(
    state: State<AppState>,
    repo_id: String,
) -> Result<Option<github::GitHubIntegration>, String> {
    github::load_integration(&repo_id, &state.db)
}

#[tauri::command]
pub async fn sync_github_cmd(
    state: State<'_, AppState>,
    repo_id: String,
) -> Result<serde_json::Value, String> {
    let integration = github::load_integration(&repo_id, &state.db)?
        .ok_or_else(|| "No GitHub integration found. Run resolve_github_repo first.".to_string())?;

    // Create a job
    let job_id = job_engine::create_job(
        "github_sync",
        &serde_json::json!({"repoId": repo_id}).to_string(),
        &state.db,
    )?;
    let cancellation = job_engine::begin_job(&job_id, &state.db, &state.jobs)?;

    job_engine::append_job_event(
        &job_id,
        "github_sync_started",
        &serde_json::json!({"repoId": repo_id}).to_string(),
        &state.db,
    )?;

    let integration_for_sync = integration.clone();
    let db_for_sync = state.db.clone();
    let cancellation_for_sync = cancellation.clone();
    let sync_result = tauri::async_runtime::spawn_blocking(move || {
        let workflows = github::sync_workflow_runs(&integration_for_sync, &db_for_sync)?;
        if cancellation_for_sync.load(std::sync::atomic::Ordering::SeqCst) {
            return Err("GitHub sync cancelled by user".into());
        }
        let prs = github::sync_prs(&integration_for_sync, &db_for_sync)?;
        if cancellation_for_sync.load(std::sync::atomic::Ordering::SeqCst) {
            return Err("GitHub sync cancelled by user".into());
        }
        let releases = github::sync_releases(&integration_for_sync, &db_for_sync)?;
        Ok::<_, String>((workflows.len(), prs.len(), releases.len()))
    })
    .await
    .unwrap_or_else(|e| Err(format!("GitHub sync worker failed: {}", e)));

    if cancellation.load(std::sync::atomic::Ordering::SeqCst) {
        state.jobs.finish(&job_id);
        return Err("GitHub sync cancelled by user".into());
    }

    match sync_result {
        Ok((wf_count, pr_count, rel_count)) => {
            github::set_sync_errors(&integration.id, &[], &state.db)?;
            job_engine::complete_job(&job_id, &state.db)?;
            state.jobs.finish(&job_id);
            job_engine::append_job_event(
                &job_id,
                "github_sync_completed",
                &serde_json::json!({"workflows": wf_count, "prs": pr_count, "releases": rel_count})
                    .to_string(),
                &state.db,
            )?;

            write_audit(
                &state,
                "sync_github",
                &repo_id,
                &format!("{}/{}", integration.github_owner, integration.github_repo),
                "github.read",
                "low",
                &format!(
                    "Synced {} workflows, {} PRs, {} releases",
                    wf_count, pr_count, rel_count
                ),
            )?;

            Ok(serde_json::json!({
                "workflows": wf_count,
                "prs": pr_count,
                "releases": rel_count,
            }))
        }
        Err(e) => {
            let _ = github::set_sync_errors(&integration.id, std::slice::from_ref(&e), &state.db);
            job_engine::fail_job(&job_id, &e, &state.db)?;
            state.jobs.finish(&job_id);
            Err(e)
        }
    }
}

#[tauri::command]
pub fn create_pr_cmd(
    state: State<AppState>,
    repo_id: String,
    title: String,
    body: String,
    head: String,
    base: String,
    draft: Option<bool>,
) -> Result<github::GitHubPR, String> {
    ensure_github_write_enabled()?;
    let integration = github::load_integration(&repo_id, &state.db)?
        .ok_or_else(|| "No GitHub integration found".to_string())?;

    let pr = github::create_pr(
        &integration,
        &title,
        &body,
        &head,
        &base,
        draft.unwrap_or(false),
    )?;

    write_audit(
        &state,
        "create_pr",
        &repo_id,
        &format!("pr:{}", pr.pr_number),
        "github.create_pr",
        "high",
        &format!("Created PR #{}: {}", pr.pr_number, title),
    )?;

    Ok(pr)
}

#[tauri::command]
pub fn create_release_cmd(
    state: State<AppState>,
    repo_id: String,
    tag: String,
    name: String,
    body: String,
    draft: Option<bool>,
    prerelease: Option<bool>,
) -> Result<github::GitHubRelease, String> {
    ensure_github_write_enabled()?;
    let integration = github::load_integration(&repo_id, &state.db)?
        .ok_or_else(|| "No GitHub integration found".to_string())?;

    let release = github::create_release(
        &integration,
        &tag,
        &name,
        &body,
        draft.unwrap_or(false),
        prerelease.unwrap_or(false),
    )?;

    write_audit(
        &state,
        "create_release",
        &repo_id,
        &tag,
        "github.create_release",
        "critical",
        &format!("Created release: {}", tag),
    )?;

    Ok(release)
}

#[tauri::command]
pub fn rerun_workflow_cmd(
    state: State<AppState>,
    repo_id: String,
    run_id: String,
) -> Result<(), String> {
    ensure_github_write_enabled()?;
    let integration = github::load_integration(&repo_id, &state.db)?
        .ok_or_else(|| "No GitHub integration found".to_string())?;

    github::rerun_workflow(&integration, &run_id)?;

    write_audit(
        &state,
        "rerun_workflow",
        &repo_id,
        &run_id,
        "github.write",
        "medium",
        &format!("Reran workflow: {}", run_id),
    )?;

    Ok(())
}

// --- Verification commands ---

#[tauri::command]
pub fn detect_commands_cmd(
    state: State<AppState>,
    worktree_path: String,
) -> Result<Vec<verification::VerificationCommand>, String> {
    let roots = list_workspace_roots(state)?;
    let path = std::path::Path::new(&worktree_path);
    security::authorize_path(path, &roots).ok_or_else(|| {
        format!(
            "Path is not within any authorized workspace root: {}",
            worktree_path
        )
    })?;
    Ok(verification::detect_commands(&worktree_path))
}

/// Resolve a worktree_path to a repo_id. Returns (repo_id, worktree_path).
fn resolve_repo_id(
    state: &State<AppState>,
    worktree_or_repo_id: &str,
) -> Result<(String, String), String> {
    let conn = state.db.conn.lock().map_err(|e| e.to_string())?;
    // Try as repo_id first
    let mut stmt = conn
        .prepare("SELECT id, worktree_path FROM repository WHERE id = ?1")
        .map_err(|e| e.to_string())?;
    let mut rows = stmt
        .query(rusqlite::params![worktree_or_repo_id])
        .map_err(|e| e.to_string())?;
    if let Some(row) = rows.next().map_err(|e| e.to_string())? {
        let id: String = row.get(0).map_err(|e| e.to_string())?;
        let wt: String = row.get(1).map_err(|e| e.to_string())?;
        return Ok((id, wt));
    }
    drop(rows);
    drop(stmt);

    // Try as worktree_path
    let mut stmt = conn
        .prepare("SELECT id, worktree_path FROM repository WHERE worktree_path = ?1")
        .map_err(|e| e.to_string())?;
    let mut rows = stmt
        .query(rusqlite::params![worktree_or_repo_id])
        .map_err(|e| e.to_string())?;
    if let Some(row) = rows.next().map_err(|e| e.to_string())? {
        let id: String = row.get(0).map_err(|e| e.to_string())?;
        let wt: String = row.get(1).map_err(|e| e.to_string())?;
        return Ok((id, wt));
    }

    Err(format!(
        "No repository found for id or path: {}",
        worktree_or_repo_id
    ))
}

fn resolve_verification_repo(
    state: &State<AppState>,
    cwd: &str,
    repo_id: Option<&str>,
) -> Result<Option<String>, String> {
    if let Some(repo_id) = repo_id {
        let (resolved_id, worktree_path) = resolve_repo_id(state, repo_id)?;
        if !security::same_path(
            std::path::Path::new(cwd),
            std::path::Path::new(&worktree_path),
        ) {
            return Err(format!(
                "Repository '{}' does not match verification directory '{}'",
                resolved_id, cwd
            ));
        }
        return Ok(Some(resolved_id));
    }

    let conn = state.db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT id, worktree_path FROM repository")
        .map_err(|e| e.to_string())?;
    let repositories = stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(repositories
        .into_iter()
        .find(|(_, worktree_path)| {
            security::same_path(
                std::path::Path::new(cwd),
                std::path::Path::new(worktree_path),
            )
        })
        .map(|(id, _)| id))
}

#[tauri::command]
pub async fn run_verification_cmd(
    state: State<'_, AppState>,
    command: String,
    cwd: String,
    repo_id: Option<String>,
    approval_id: String,
) -> Result<verification::VerificationResult, String> {
    let roots = list_workspace_roots(state.clone())?;
    let cwd_path = std::path::Path::new(&cwd);
    security::authorize_path(cwd_path, &roots).ok_or_else(|| {
        format!(
            "Working directory is not within any authorized workspace root: {}",
            cwd
        )
    })?;

    if !cwd_path.is_dir() {
        return Err(format!(
            "Verification directory is not a directory: {}",
            cwd
        ));
    }

    let command_meta = verification::resolve_command(&cwd, &command)?;
    let resolved_repo_id = resolve_verification_repo(&state, &cwd, repo_id.as_deref())?;
    let resolved_repo_id = resolved_repo_id
        .ok_or_else(|| "Verification requires a registered repository".to_string())?;
    let context_hash = permissions::verification_context_hash(&cwd, &command_meta)?;
    permissions::consume_request(
        &approval_id,
        "shell.verify",
        Some(&resolved_repo_id),
        &context_hash,
        &state.db,
    )?;

    // Create a job
    let job_id = job_engine::create_job(
        "verification",
        &serde_json::json!({"command": command, "cwd": cwd}).to_string(),
        &state.db,
    )?;
    let cancellation = job_engine::begin_job(&job_id, &state.db, &state.jobs)?;

    job_engine::append_job_event(
        &job_id,
        "verification_started",
        &serde_json::json!({"command": command, "cwd": cwd}).to_string(),
        &state.db,
    )?;

    let category = command_meta.category.as_str();
    let risk_level = command_meta.risk_level.as_str();
    let result = verification::run_verification_with_control(
        &command_meta.command,
        &cwd,
        command_meta.timeout_secs,
        cancellation,
    );
    let summary = verification::summarize_output(&result);

    if result.success {
        job_engine::complete_job(&job_id, &state.db)?;
        job_engine::append_job_event(&job_id, "verification_completed",
            &serde_json::json!({"success": true, "durationMs": result.duration_ms, "summary": summary}).to_string(), &state.db)?;
    } else {
        let error_msg = format!(
            "Command '{}' failed (exit code: {})",
            command,
            result.exit_code.map_or("N/A".into(), |c| c.to_string())
        );
        job_engine::fail_job(&job_id, &error_msg, &state.db)?;
        job_engine::append_job_event(&job_id, "verification_failed",
            &serde_json::json!({"success": false, "exitCode": result.exit_code, "durationMs": result.duration_ms, "summary": summary}).to_string(), &state.db)?;
    }
    state.jobs.finish(&job_id);

    // Persist verification_run record
    {
        let run_id = uuid::Uuid::new_v4().to_string();
        let stdout_tail = verification::tail_truncate(&result.stdout, 4000);
        let stderr_tail = verification::tail_truncate(&result.stderr, 4000);
        let now2 = chrono::Utc::now().to_rfc3339();
        let conn = state.db.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT INTO verification_run (id, repo_id, job_id, command, cwd, category, risk_level, success, exit_code, duration_ms, timed_out, stdout_tail, stderr_tail, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            rusqlite::params![
                run_id,
                &resolved_repo_id,
                job_id,
                command,
                cwd,
                category,
                risk_level,
                result.success as i32,
                result.exit_code,
                result.duration_ms as i64,
                result.timed_out as i32,
                stdout_tail,
                stderr_tail,
                now2,
            ],
        ).map_err(|e| e.to_string())?;
    }

    write_audit(
        &state,
        "run_verification",
        &job_id,
        &cwd,
        "fs.read",
        risk_level,
        &format!(
            "Ran verification: {} ({})",
            command,
            if result.success { "passed" } else { "failed" }
        ),
    )?;

    Ok(result)
}

/// Run multiple verification commands sequentially.
#[tauri::command]
pub async fn run_batch_verification_cmd(
    state: State<'_, AppState>,
    command_names: Vec<String>,
    cwd: String,
    repo_id: Option<String>,
    approval_ids: Vec<String>,
) -> Result<Vec<verification::VerificationResult>, String> {
    let roots = list_workspace_roots(state.clone())?;
    let cwd_path = std::path::Path::new(&cwd);
    security::authorize_path(cwd_path, &roots).ok_or_else(|| {
        format!(
            "Working directory is not within any authorized workspace root: {}",
            cwd
        )
    })?;

    if !cwd_path.is_dir() {
        return Err(format!(
            "Verification directory is not a directory: {}",
            cwd
        ));
    }
    if command_names.is_empty() {
        return Err("No verification commands were selected".into());
    }

    let mut seen = std::collections::HashSet::new();
    let mut commands = Vec::with_capacity(command_names.len());
    for command in command_names {
        if !seen.insert(command.clone()) {
            return Err(format!("Duplicate verification command: {}", command));
        }
        let resolved = verification::resolve_command(&cwd, &command)?;
        commands.push(resolved);
    }

    let resolved_repo_id = resolve_verification_repo(&state, &cwd, repo_id.as_deref())?;
    let resolved_repo_id = resolved_repo_id
        .ok_or_else(|| "Batch verification requires a registered repository".to_string())?;
    if approval_ids.len() != commands.len() {
        return Err("Each verification command requires its own approval".into());
    }
    for (command, approval_id) in commands.iter().zip(&approval_ids) {
        let context_hash = permissions::verification_context_hash(&cwd, command)?;
        permissions::consume_request(
            approval_id,
            "shell.verify",
            Some(&resolved_repo_id),
            &context_hash,
            &state.db,
        )?;
    }
    let job_id = job_engine::create_job(
        "verification_batch",
        &serde_json::json!({
            "commands": commands.iter().map(|cmd| &cmd.command).collect::<Vec<_>>(),
            "cwd": cwd,
        })
        .to_string(),
        &state.db,
    )?;
    let cancellation = job_engine::begin_job(&job_id, &state.db, &state.jobs)?;
    job_engine::update_progress(&job_id, 0, commands.len() as i32, &state.db)?;
    job_engine::append_job_event(
        &job_id,
        "verification_batch_started",
        &serde_json::json!({"commandCount": commands.len(), "cwd": cwd}).to_string(),
        &state.db,
    )?;

    let mut results = Vec::new();

    for (index, cmd) in commands.iter().enumerate() {
        if cancellation.load(std::sync::atomic::Ordering::SeqCst) {
            break;
        }
        let result = verification::run_verification_with_control(
            &cmd.command,
            &cwd,
            cmd.timeout_secs,
            cancellation.clone(),
        );
        results.push(result.clone());

        // Persist each run
        {
            let run_id = uuid::Uuid::new_v4().to_string();
            let stdout_tail = verification::tail_truncate(&result.stdout, 4000);
            let stderr_tail = verification::tail_truncate(&result.stderr, 4000);
            let now = chrono::Utc::now().to_rfc3339();
            let conn = state.db.conn.lock().map_err(|e| e.to_string())?;
            conn.execute(
                "INSERT INTO verification_run (id, repo_id, job_id, command, cwd, category, risk_level, success, exit_code, duration_ms, timed_out, stdout_tail, stderr_tail, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
                rusqlite::params![
                    run_id,
                    &resolved_repo_id,
                    job_id,
                    cmd.command,
                    cwd,
                    cmd.category,
                    cmd.risk_level,
                    result.success as i32,
                    result.exit_code,
                    result.duration_ms as i64,
                    result.timed_out as i32,
                    stdout_tail,
                    stderr_tail,
                    now,
                ],
            ).map_err(|e| e.to_string())?;
        }

        write_audit(
            &state,
            "run_verification",
            &cmd.command,
            &cwd,
            "fs.read",
            &cmd.risk_level,
            &format!(
                "Ran verification: {} ({})",
                cmd.command,
                if result.success { "passed" } else { "failed" }
            ),
        )?;
        job_engine::update_progress(
            &job_id,
            (index + 1) as i32,
            commands.len() as i32,
            &state.db,
        )?;
    }

    if cancellation.load(std::sync::atomic::Ordering::SeqCst) {
        state.jobs.finish(&job_id);
        return Ok(results);
    }
    let failed_count = results.iter().filter(|result| !result.success).count();
    if failed_count == 0 {
        job_engine::complete_job(&job_id, &state.db)?;
        job_engine::append_job_event(
            &job_id,
            "verification_batch_completed",
            &serde_json::json!({"commandCount": results.len()}).to_string(),
            &state.db,
        )?;
    } else {
        let error = format!(
            "{} of {} verification commands failed",
            failed_count,
            results.len()
        );
        job_engine::fail_job(&job_id, &error, &state.db)?;
        job_engine::append_job_event(
            &job_id,
            "verification_batch_failed",
            &serde_json::json!({
                "commandCount": results.len(),
                "failedCount": failed_count,
            })
            .to_string(),
            &state.db,
        )?;
    }
    state.jobs.finish(&job_id);

    Ok(results)
}

/// List stored verification runs for a repo.
#[tauri::command]
pub fn list_verification_runs_cmd(
    state: State<AppState>,
    repo_id: String,
    limit: Option<i64>,
) -> Result<Vec<verification::VerificationRun>, String> {
    let limit = limit.unwrap_or(50).clamp(1, 500);
    let conn = state.db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare(
        "SELECT id, repo_id, job_id, command, cwd, category, risk_level, success, exit_code, duration_ms, timed_out, stdout_tail, stderr_tail, created_at FROM verification_run WHERE repo_id = ?1 ORDER BY created_at DESC LIMIT ?2"
    ).map_err(|e| e.to_string())?;
    let runs = stmt
        .query_map(rusqlite::params![repo_id, limit], |row| {
            Ok(verification::VerificationRun {
                id: row.get(0)?,
                repo_id: row.get(1)?,
                job_id: row.get(2)?,
                command: row.get(3)?,
                cwd: row.get(4)?,
                category: row.get(5)?,
                risk_level: row.get(6)?,
                success: row.get::<_, i32>(7)? != 0,
                exit_code: row.get(8)?,
                duration_ms: row.get::<_, i64>(9)? as u64,
                timed_out: row.get::<_, i32>(10)? != 0,
                stdout_tail: row.get(11)?,
                stderr_tail: row.get(12)?,
                created_at: row.get(13)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    Ok(runs)
}

// --- Automation commands ---

#[tauri::command]
pub fn list_automation_rules_cmd(
    state: State<AppState>,
) -> Result<Vec<automations::AutomationRule>, String> {
    automations::list_rules(&state.db)
}

#[tauri::command]
pub fn create_automation_rule_cmd(
    state: State<AppState>,
    rule: automations::AutomationRule,
) -> Result<(), String> {
    automations::create_rule(&rule, &state.db)?;

    write_audit(
        &state,
        "create_automation_rule",
        &rule.id,
        &rule.name,
        "automation",
        "low",
        &format!(
            "Created rule: {} (trigger: {}, action: {})",
            rule.name, rule.trigger_type, rule.action_type
        ),
    )?;

    Ok(())
}

#[tauri::command]
pub fn update_automation_rule_cmd(
    state: State<AppState>,
    rule: automations::AutomationRule,
) -> Result<(), String> {
    automations::update_rule(&rule, &state.db)
}

#[tauri::command]
pub fn delete_automation_rule_cmd(state: State<AppState>, id: String) -> Result<(), String> {
    automations::delete_rule(&id, &state.db)?;

    write_audit(
        &state,
        "delete_automation_rule",
        &id,
        "system",
        "automation",
        "low",
        &format!("Deleted rule: {}", id),
    )?;

    Ok(())
}

#[tauri::command]
pub fn list_notifications_cmd(
    state: State<AppState>,
    unread_only: Option<bool>,
    limit: Option<i64>,
) -> Result<Vec<automations::Notification>, String> {
    automations::list_notifications(
        unread_only.unwrap_or(false),
        limit.unwrap_or(50).clamp(1, 500),
        &state.db,
    )
}

#[tauri::command]
pub fn mark_notification_read_cmd(state: State<AppState>, id: String) -> Result<(), String> {
    automations::mark_notification_read(&id, &state.db)
}

#[tauri::command]
pub fn mark_all_notifications_read_cmd(state: State<AppState>) -> Result<usize, String> {
    automations::mark_all_notifications_read(&state.db)
}

#[tauri::command]
pub fn tick_scheduler_cmd(state: State<AppState>) -> Result<Vec<String>, String> {
    let triggered = automations::tick_scheduler(&state.db)?;

    write_audit(
        &state,
        "tick_scheduler",
        "scheduler",
        "automation",
        "automation",
        "low",
        &format!("Scheduler tick triggered {} rule(s)", triggered.len()),
    )?;

    Ok(triggered)
}

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

#[cfg(test)]
mod tests {
    use super::*;

    fn root_input(access_mode: &str, include_globs: Vec<String>) -> AddRootInput {
        AddRootInput {
            path: "C:/workspace".into(),
            label: "Workspace".into(),
            access_mode: access_mode.into(),
            scan_enabled: true,
            include_globs,
            exclude_globs: vec!["node_modules".into()],
        }
    }

    #[test]
    fn workspace_settings_reject_invalid_modes_and_globs() {
        assert!(validate_root_settings(&root_input("admin", vec![])).is_err());
        assert!(validate_root_settings(&root_input(
            "read_only",
            vec!["[unterminated".into()]
        ))
        .is_err());
        assert!(validate_root_settings(&root_input(
            "read_write",
            vec!["clients/**".into()]
        ))
        .is_ok());
    }
}
