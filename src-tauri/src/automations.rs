use crate::db::Db;
use serde_json::Value;

/// Automation rule.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AutomationRule {
    pub id: String,
    pub name: String,
    pub description: String,
    pub trigger_type: String,
    pub trigger_config: Value,
    pub action_type: String,
    pub action_config: Value,
    pub target_repo_ids: Vec<String>,
    pub target_root_ids: Vec<String>,
    pub max_risk_level: String,
    pub auto_apply: bool,
    pub enabled: bool,
    pub last_triggered_at: Option<String>,
    pub last_run_job_id: Option<String>,
    pub run_count: i64,
}

/// Notification.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Notification {
    pub id: String,
    pub rule_id: Option<String>,
    pub job_id: Option<String>,
    pub notification_type: String,
    pub title: String,
    pub message: String,
    pub read: bool,
    pub action_url: Option<String>,
    pub created_at: String,
}

fn validate_rule(rule: &AutomationRule) -> Result<(), String> {
    if rule.name.trim().is_empty() {
        return Err("Automation rule name cannot be empty".into());
    }
    if rule.trigger_type != "schedule" {
        return Err("Only schedule triggers are implemented".into());
    }
    if rule.action_type != "notify" {
        return Err("Only notification actions are implemented".into());
    }
    if rule.auto_apply {
        return Err("Auto-apply is not available for notification rules".into());
    }
    let interval = rule
        .trigger_config
        .get("intervalMinutes")
        .and_then(Value::as_u64)
        .ok_or_else(|| "Schedule rules require intervalMinutes".to_string())?;
    if !(1..=10_080).contains(&interval) {
        return Err("intervalMinutes must be between 1 and 10080".into());
    }
    Ok(())
}

/// Create an automation rule.
pub fn create_rule(rule: &AutomationRule, db: &Db) -> Result<(), String> {
    validate_rule(rule)?;
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO automation_rule (id, name, description, trigger_type, trigger_config, action_type, action_config, target_repo_ids, target_root_ids, max_risk_level, auto_apply, enabled, last_triggered_at, last_run_job_id, run_count, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)",
        rusqlite::params![
            rule.id,
            rule.name,
            rule.description,
            rule.trigger_type,
            serde_json::to_string(&rule.trigger_config).unwrap_or_default(),
            rule.action_type,
            serde_json::to_string(&rule.action_config).unwrap_or_default(),
            serde_json::to_string(&rule.target_repo_ids).unwrap_or_default(),
            serde_json::to_string(&rule.target_root_ids).unwrap_or_default(),
            rule.max_risk_level,
            rule.auto_apply as i32,
            rule.enabled as i32,
            rule.last_triggered_at,
            rule.last_run_job_id,
            rule.run_count,
            now,
            now,
        ],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// Update an automation rule.
pub fn update_rule(rule: &AutomationRule, db: &Db) -> Result<(), String> {
    validate_rule(rule)?;
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let now = chrono::Utc::now().to_rfc3339();
    let updated = conn.execute(
        "UPDATE automation_rule SET name = ?1, description = ?2, trigger_type = ?3, trigger_config = ?4, action_type = ?5, action_config = ?6, target_repo_ids = ?7, target_root_ids = ?8, max_risk_level = ?9, auto_apply = ?10, enabled = ?11, updated_at = ?12 WHERE id = ?13",
        rusqlite::params![
            rule.name,
            rule.description,
            rule.trigger_type,
            serde_json::to_string(&rule.trigger_config).unwrap_or_default(),
            rule.action_type,
            serde_json::to_string(&rule.action_config).unwrap_or_default(),
            serde_json::to_string(&rule.target_repo_ids).unwrap_or_default(),
            serde_json::to_string(&rule.target_root_ids).unwrap_or_default(),
            rule.max_risk_level,
            rule.auto_apply as i32,
            rule.enabled as i32,
            now,
            rule.id,
        ],
    )
    .map_err(|e| e.to_string())?;
    if updated == 0 {
        return Err(format!("Automation rule not found: {}", rule.id));
    }
    Ok(())
}

/// Delete an automation rule.
pub fn delete_rule(id: &str, db: &Db) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let deleted = conn.execute(
        "DELETE FROM automation_rule WHERE id = ?1",
        rusqlite::params![id],
    )
    .map_err(|e| e.to_string())?;
    if deleted == 0 {
        return Err(format!("Automation rule not found: {}", id));
    }
    Ok(())
}

/// List all automation rules.
pub fn list_rules(db: &Db) -> Result<Vec<AutomationRule>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT id, name, description, trigger_type, trigger_config, action_type, action_config, target_repo_ids, target_root_ids, max_risk_level, auto_apply, enabled, last_triggered_at, last_run_job_id, run_count FROM automation_rule ORDER BY created_at")
        .map_err(|e| e.to_string())?;

    let rules = stmt
        .query_map([], |row| {
            let trigger_config_str: String = row.get(4)?;
            let action_config_str: String = row.get(6)?;
            let target_repo_ids_str: String = row.get(7)?;
            let target_root_ids_str: String = row.get(8)?;

            Ok(AutomationRule {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                trigger_type: row.get(3)?,
                trigger_config: serde_json::from_str(&trigger_config_str)
                    .unwrap_or(Value::Object(Default::default())),
                action_type: row.get(5)?,
                action_config: serde_json::from_str(&action_config_str)
                    .unwrap_or(Value::Object(Default::default())),
                target_repo_ids: serde_json::from_str(&target_repo_ids_str).unwrap_or_default(),
                target_root_ids: serde_json::from_str(&target_root_ids_str).unwrap_or_default(),
                max_risk_level: row.get(9)?,
                auto_apply: row.get::<_, i32>(10)? != 0,
                enabled: row.get::<_, i32>(11)? != 0,
                last_triggered_at: row.get(12)?,
                last_run_job_id: row.get(13)?,
                run_count: row.get(14)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(rules)
}

/// List notifications (optionally unread only).
pub fn list_notifications(
    unread_only: bool,
    limit: i64,
    db: &Db,
) -> Result<Vec<Notification>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let sql = if unread_only {
        "SELECT id, rule_id, job_id, type, title, message, read, action_url, created_at FROM notification WHERE read = 0 ORDER BY created_at DESC LIMIT ?1"
    } else {
        "SELECT id, rule_id, job_id, type, title, message, read, action_url, created_at FROM notification ORDER BY created_at DESC LIMIT ?1"
    };

    let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;

    let notifications = stmt
        .query_map(rusqlite::params![limit], |row| {
            Ok(Notification {
                id: row.get(0)?,
                rule_id: row.get(1)?,
                job_id: row.get(2)?,
                notification_type: row.get(3)?,
                title: row.get(4)?,
                message: row.get(5)?,
                read: row.get::<_, i32>(6)? != 0,
                action_url: row.get(7)?,
                created_at: row.get(8)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(notifications)
}

/// Mark a notification as read.
pub fn mark_notification_read(id: &str, db: &Db) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE notification SET read = 1 WHERE id = ?1",
        rusqlite::params![id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// Mark all notifications as read.
pub fn mark_all_notifications_read(db: &Db) -> Result<usize, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let count = conn
        .execute("UPDATE notification SET read = 1 WHERE read = 0", [])
        .map_err(|e| e.to_string())?;
    Ok(count)
}

/// Check and fire due automation rules (scheduler tick).
pub fn tick_scheduler(db: &Db) -> Result<Vec<String>, String> {
    let rules = list_rules(db)?;
    let now = chrono::Utc::now();
    let mut triggered = Vec::new();

    for rule in &rules {
        if !rule.enabled {
            continue;
        }

        if validate_rule(rule).is_err() {
            continue;
        }
        let interval_mins = rule
            .trigger_config
            .get("intervalMinutes")
            .and_then(Value::as_u64)
            .unwrap_or(60);
        let should_fire = rule
            .last_triggered_at
            .as_deref()
            .and_then(|last| chrono::DateTime::parse_from_rfc3339(last).ok())
            .map(|last| now.signed_duration_since(last).num_minutes() >= interval_mins as i64)
            .unwrap_or(true);

        if should_fire {
            let notification = Notification {
                id: uuid::Uuid::new_v4().to_string(),
                rule_id: Some(rule.id.clone()),
                job_id: None,
                notification_type: "info".to_string(),
                title: format!("Automation triggered: {}", rule.name),
                message: format!(
                    "Action: {} on {} repos",
                    rule.action_type,
                    rule.target_repo_ids.len()
                ),
                read: false,
                action_url: None,
                created_at: now.to_rfc3339(),
            };

            let mut conn = db.conn.lock().map_err(|e| e.to_string())?;
            let tx = conn.transaction().map_err(|e| e.to_string())?;
            let current_last: Option<String> = tx
                .query_row(
                    "SELECT last_triggered_at FROM automation_rule WHERE id = ?1 AND enabled = 1",
                    rusqlite::params![rule.id],
                    |row| row.get(0),
                )
                .map_err(|e| e.to_string())?;
            let still_due = current_last
                .as_deref()
                .and_then(|last| chrono::DateTime::parse_from_rfc3339(last).ok())
                .map(|last| {
                    now.signed_duration_since(last).num_minutes() >= interval_mins as i64
                })
                .unwrap_or(true);
            if !still_due {
                continue;
            }
            tx.execute(
                "UPDATE automation_rule SET last_triggered_at = ?1, run_count = run_count + 1, updated_at = ?1 WHERE id = ?2",
                rusqlite::params![notification.created_at, rule.id],
            )
            .map_err(|e| e.to_string())?;
            tx.execute(
                "INSERT INTO notification (id, rule_id, job_id, type, title, message, read, action_url, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                rusqlite::params![
                    notification.id,
                    notification.rule_id,
                    notification.job_id,
                    notification.notification_type,
                    notification.title,
                    notification.message,
                    notification.read as i32,
                    notification.action_url,
                    notification.created_at,
                ],
            )
            .map_err(|e| e.to_string())?;
            tx.commit().map_err(|e| e.to_string())?;
            triggered.push(rule.id.clone());
        }
    }

    Ok(triggered)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn test_db() -> Db {
        Db::new(&PathBuf::from(":memory:")).unwrap()
    }

    fn schedule_rule() -> AutomationRule {
        AutomationRule {
            id: "rule-1".into(),
            name: "Reminder".into(),
            description: String::new(),
            trigger_type: "schedule".into(),
            trigger_config: serde_json::json!({"intervalMinutes": 60}),
            action_type: "notify".into(),
            action_config: serde_json::json!({}),
            target_repo_ids: vec![],
            target_root_ids: vec![],
            max_risk_level: "low".into(),
            auto_apply: false,
            enabled: true,
            last_triggered_at: None,
            last_run_job_id: None,
            run_count: 0,
        }
    }

    #[test]
    fn rejects_unimplemented_automation_actions() {
        let mut rule = schedule_rule();
        rule.action_type = "fix".into();
        assert!(validate_rule(&rule).is_err());
    }

    #[test]
    fn scheduler_does_not_duplicate_notifications_within_interval() {
        let db = test_db();
        create_rule(&schedule_rule(), &db).unwrap();
        assert_eq!(tick_scheduler(&db).unwrap(), vec!["rule-1".to_string()]);
        assert!(tick_scheduler(&db).unwrap().is_empty());
        assert_eq!(list_notifications(false, 10, &db).unwrap().len(), 1);
    }
}
