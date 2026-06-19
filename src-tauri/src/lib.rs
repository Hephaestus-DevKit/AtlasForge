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
mod profiler;
mod scanner;
mod security;
mod tool_broker;
mod verification;

use commands::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let db_path = get_db_path();
    let db = db::Db::new(&db_path).expect("Failed to initialize database");
    log::info!("Database initialized at {:?}", db_path);

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState { db })
        .invoke_handler(tauri::generate_handler![
            commands::greet,
            commands::list_workspace_roots,
            commands::add_workspace_root,
            commands::remove_workspace_root,
            commands::update_workspace_root,
            commands::list_repositories,
            commands::list_project_assets,
            commands::start_scan,
            commands::list_jobs,
            commands::list_jobs_by_type_cmd,
            commands::get_job_events,
            commands::get_repo_profile,
            commands::refresh_profiles,
            commands::list_repo_profiles_cmd,
            // Job engine
            commands::cancel_job_cmd,
            commands::retry_job_cmd,
            commands::get_job_detail,
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
            commands::check_gh_auth_cmd,
            commands::resolve_github_repo_cmd,
            commands::sync_github_cmd,
            commands::create_pr_cmd,
            commands::create_release_cmd,
            commands::rerun_workflow_cmd,
            // Verification
            commands::detect_commands_cmd,
            commands::run_verification_cmd,
            commands::run_batch_verification_cmd,
            commands::list_verification_runs_cmd,
            // Automation
            commands::list_automation_rules_cmd,
            commands::create_automation_rule_cmd,
            commands::update_automation_rule_cmd,
            commands::delete_automation_rule_cmd,
            commands::list_notifications_cmd,
            commands::mark_notification_read_cmd,
            commands::mark_all_notifications_read_cmd,
            commands::tick_scheduler_cmd,
            commands::list_scan_errors_cmd,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn get_db_path() -> std::path::PathBuf {
    let app_dir = dirs_next();
    std::path::PathBuf::from(app_dir).join("atlasforge.db")
}

fn dirs_next() -> String {
    // Use a simple approach: store in the user's home directory under .atlasforge
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".into());
    let dir = std::path::PathBuf::from(home).join(".atlasforge");
    std::fs::create_dir_all(&dir).ok();
    dir.to_string_lossy().to_string()
}
