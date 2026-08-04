use super::{list_workspace_roots, write_audit, AppState};
use crate::{job_engine, permissions, security, verification};
use tauri::State;
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
    let contexts = commands
        .iter()
        .map(|command| permissions::verification_context_hash(&cwd, command))
        .collect::<Result<Vec<_>, _>>()?;
    let approvals = commands
        .iter()
        .zip(&approval_ids)
        .zip(&contexts)
        .map(|((_, approval_id), context_hash)| {
            (
                approval_id.as_str(),
                "shell.verify",
                Some(resolved_repo_id.as_str()),
                context_hash.as_str(),
            )
        })
        .collect::<Vec<_>>();
    permissions::consume_requests(&approvals, &state.db)?;
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
