-- Trusted execution foundation: explicit approvals and recoverable patch metadata.

CREATE TABLE IF NOT EXISTS permission_request (
    id TEXT PRIMARY KEY NOT NULL,
    job_id TEXT REFERENCES job(id) ON DELETE SET NULL,
    repo_id TEXT REFERENCES repository(id) ON DELETE CASCADE,
    capability TEXT NOT NULL,
    scope TEXT NOT NULL,
    risk_level TEXT NOT NULL CHECK(risk_level IN ('none', 'low', 'medium', 'high', 'critical')),
    command TEXT,
    context_hash TEXT NOT NULL,
    details TEXT NOT NULL DEFAULT '{}',
    status TEXT NOT NULL DEFAULT 'pending' CHECK(status IN ('pending', 'approved', 'denied', 'consumed', 'expired')),
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    expires_at TEXT NOT NULL,
    decided_at TEXT,
    consumed_at TEXT
);

CREATE INDEX IF NOT EXISTS idx_permission_request_status
    ON permission_request(status, expires_at);
CREATE INDEX IF NOT EXISTS idx_permission_request_repo
    ON permission_request(repo_id, created_at DESC);

ALTER TABLE patch_proposal ADD COLUMN base_head_sha TEXT;
ALTER TABLE patch_proposal ADD COLUMN base_file_hash TEXT;
ALTER TABLE patch_proposal ADD COLUMN applied_file_hash TEXT;
ALTER TABLE patch_proposal ADD COLUMN backup_content TEXT;
ALTER TABLE patch_proposal ADD COLUMN approval_context_hash TEXT;
