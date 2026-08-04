mod ai_fix;
mod ai_provider;
mod auditor;
mod automations;
mod commands;
mod db;
mod github;
mod indexer;
mod job_engine;
mod models;
mod permissions;
mod process_runner;
mod profiler;
mod scanner;
mod security;
mod tool_broker;
mod verification;
mod workspace;

use commands::AppState;
use std::sync::Arc;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let db_path = get_db_path(app.handle())?;
            migrate_legacy_database(&db_path)?;
            let db = db::Db::new(&db_path).map_err(std::io::Error::other)?;
            let recovered_patches = ai_fix::recover_interrupted_patch_operations(&db)
                .map_err(std::io::Error::other)?;
            if recovered_patches > 0 {
                log::warn!("Reconciled {} interrupted patch operations", recovered_patches);
            }
            let recovered =
                job_engine::recover_interrupted_jobs(&db).map_err(std::io::Error::other)?;
            if recovered > 0 {
                log::warn!("Marked {} interrupted jobs as failed", recovered);
            }
            log::info!("Database initialized at {:?}", db_path);
            let db = Arc::new(db);
            let scheduler_db = db.clone();
            tauri::async_runtime::spawn(async move {
                let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
                interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
                loop {
                    interval.tick().await;
                    if let Err(error) = automations::tick_scheduler(&scheduler_db) {
                        log::warn!("Automation scheduler tick failed: {}", error);
                    }
                }
            });
            app.manage(AppState {
                db,
                jobs: Arc::new(job_engine::JobRuntime::default()),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::greet,
            commands::list_workspace_roots,
            commands::add_workspace_root,
            commands::remove_workspace_root,
            commands::update_workspace_root,
            commands::list_repositories,
            commands::list_repository_summaries,
            commands::list_project_assets,
            commands::start_scan,
            commands::list_jobs,
            commands::list_jobs_by_type_cmd,
            commands::get_job_events,
            commands::list_audit_log_cmd,
            commands::get_repo_profile,
            commands::refresh_profiles,
            commands::list_repo_profiles_cmd,
            // Job engine
            commands::cancel_job_cmd,
            commands::retry_job_cmd,
            commands::get_job_detail,
            commands::request_verification_approval_cmd,
            commands::request_patch_approval_cmd,
            commands::request_rollback_approval_cmd,
            commands::decide_permission_request_cmd,
            commands::list_permission_requests_cmd,
            // Tool broker
            commands::list_tools_cmd,
            commands::invoke_tool_cmd,
            commands::list_invocations_cmd,
            // Indexer
            commands::search_index_cmd,
            commands::list_documents_cmd,
            commands::reindex_repo_cmd,
            // Auditor
            commands::audit_repo_cmd,
            commands::get_health_snapshot_cmd,
            commands::get_findings_cmd,
            // AI Provider
            commands::list_ai_providers_cmd,
            commands::detect_local_providers_cmd,
            commands::upsert_ai_provider_cmd,
            commands::delete_ai_provider_cmd,
            commands::probe_ai_provider_cmd,
            commands::call_ai_cmd,
            // AI Fix
            commands::list_artifacts_cmd,
            commands::list_patch_proposals_cmd,
            commands::apply_patch_cmd,
            commands::reject_patch_cmd,
            commands::rollback_patch_cmd,
            // AI Fix Plan
            commands::generate_fix_plan_cmd,
            commands::propose_fix_cmd,
            commands::list_fix_plans_cmd,
            commands::preview_fix_plan_context_cmd,
            // GitHub
            commands::github_commands::check_gh_auth_cmd,
            commands::github_commands::resolve_github_repo_cmd,
            commands::github_commands::get_github_evidence_cmd,
            commands::github_commands::get_github_integration_cmd,
            commands::github_commands::sync_github_cmd,
            commands::github_commands::create_pr_cmd,
            commands::github_commands::create_release_cmd,
            commands::github_commands::rerun_workflow_cmd,
            // Verification
            commands::verification_commands::detect_commands_cmd,
            commands::verification_commands::run_verification_cmd,
            commands::verification_commands::run_batch_verification_cmd,
            commands::verification_commands::list_verification_runs_cmd,
            // Automation
            commands::automation_commands::list_automation_rules_cmd,
            commands::automation_commands::create_automation_rule_cmd,
            commands::automation_commands::update_automation_rule_cmd,
            commands::automation_commands::delete_automation_rule_cmd,
            commands::automation_commands::list_notifications_cmd,
            commands::automation_commands::mark_notification_read_cmd,
            commands::automation_commands::mark_all_notifications_read_cmd,
            commands::automation_commands::tick_scheduler_cmd,
            commands::list_scan_errors_cmd,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn get_db_path(app: &tauri::AppHandle) -> Result<std::path::PathBuf, std::io::Error> {
    let app_dir = app.path().app_data_dir().map_err(std::io::Error::other)?;
    std::fs::create_dir_all(&app_dir)?;
    Ok(app_dir.join("atlasforge.db"))
}

fn migrate_legacy_database(target: &std::path::Path) -> Result<(), std::io::Error> {
    if target.exists() {
        return Ok(());
    }
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".into());
    let legacy = std::path::PathBuf::from(home)
        .join(".atlasforge")
        .join("atlasforge.db");
    if legacy.is_file() {
        std::fs::copy(&legacy, target)?;
        for suffix in ["-wal", "-shm"] {
            let source = std::path::PathBuf::from(format!("{}{}", legacy.display(), suffix));
            let destination = std::path::PathBuf::from(format!("{}{}", target.display(), suffix));
            if source.is_file() {
                std::fs::copy(source, destination)?;
            }
        }
        log::info!("Migrated legacy database from {:?}", legacy);
    }
    Ok(())
}
