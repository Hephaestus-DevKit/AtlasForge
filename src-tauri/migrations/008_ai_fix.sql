-- AI Fix v1: Patch proposals, artifacts, verification results

CREATE TABLE IF NOT EXISTS artifact (
    id TEXT PRIMARY KEY NOT NULL,
    job_id TEXT NOT NULL REFERENCES job(id) ON DELETE CASCADE,
    type TEXT NOT NULL CHECK(type IN (
        'patch_proposal', 'diff', 'verification_log', 'ai_plan', 'ai_report', 'command_output'
    )),
    title TEXT NOT NULL,
    content TEXT NOT NULL DEFAULT '',
    file_path TEXT,
    metadata TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_artifact_job ON artifact(job_id);
CREATE INDEX IF NOT EXISTS idx_artifact_type ON artifact(type);

CREATE TABLE IF NOT EXISTS patch_proposal (
    id TEXT PRIMARY KEY NOT NULL,
    job_id TEXT NOT NULL REFERENCES job(id) ON DELETE CASCADE,
    artifact_id TEXT REFERENCES artifact(id) ON DELETE SET NULL,
    repo_id TEXT NOT NULL REFERENCES repository(id) ON DELETE CASCADE,
    file_path TEXT NOT NULL,
    patch_content TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    status TEXT NOT NULL DEFAULT 'proposed' CHECK(status IN (
        'proposed', 'approved', 'applied', 'rejected', 'conflict', 'rolled_back'
    )),
    applied_at TEXT,
    rolled_back_at TEXT,
    verification_result TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_patch_proposal_job ON patch_proposal(job_id);
CREATE INDEX IF NOT EXISTS idx_patch_proposal_repo ON patch_proposal(repo_id);
CREATE INDEX IF NOT EXISTS idx_patch_proposal_status ON patch_proposal(status);
