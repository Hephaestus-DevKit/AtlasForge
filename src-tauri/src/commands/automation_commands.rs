use super::{write_audit, AppState};
use crate::automations;
use tauri::State;
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
