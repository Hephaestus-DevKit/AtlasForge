use super::{ensure_github_write_enabled, write_audit, AppState};
use crate::{github, job_engine};
use tauri::State;
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
