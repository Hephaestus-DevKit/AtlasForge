-- Automations: Rules, scheduler, triggers, notifications

CREATE TABLE IF NOT EXISTS automation_rule (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    trigger_type TEXT NOT NULL CHECK(trigger_type IN (
        'schedule', 'ci_failure', 'new_commit', 'drift_detected', 'manual'
    )),
    trigger_config TEXT NOT NULL DEFAULT '{}',
    action_type TEXT NOT NULL CHECK(action_type IN (
        'scan', 'audit', 'fix', 'notify', 'github_sync'
    )),
    action_config TEXT NOT NULL DEFAULT '{}',
    target_repo_ids TEXT NOT NULL DEFAULT '[]',
    target_root_ids TEXT NOT NULL DEFAULT '[]',
    max_risk_level TEXT NOT NULL DEFAULT 'medium' CHECK(max_risk_level IN ('none', 'low', 'medium', 'high', 'critical')),
    auto_apply INTEGER NOT NULL DEFAULT 0,
    enabled INTEGER NOT NULL DEFAULT 1,
    last_triggered_at TEXT,
    last_run_job_id TEXT REFERENCES job(id) ON DELETE SET NULL,
    run_count INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_automation_trigger ON automation_rule(trigger_type);
CREATE INDEX IF NOT EXISTS idx_automation_enabled ON automation_rule(enabled);

CREATE TABLE IF NOT EXISTS notification (
    id TEXT PRIMARY KEY NOT NULL,
    rule_id TEXT REFERENCES automation_rule(id) ON DELETE SET NULL,
    job_id TEXT REFERENCES job(id) ON DELETE SET NULL,
    type TEXT NOT NULL CHECK(type IN ('info', 'warning', 'error', 'success')),
    title TEXT NOT NULL,
    message TEXT NOT NULL DEFAULT '',
    read INTEGER NOT NULL DEFAULT 0,
    action_url TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_notification_read ON notification(read);
CREATE INDEX IF NOT EXISTS idx_notification_created ON notification(created_at);
