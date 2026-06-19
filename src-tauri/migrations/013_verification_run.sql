-- Wave 5: Verification Engine v1
-- Store verification run results linked to repo and job

CREATE TABLE IF NOT EXISTS verification_run (
    id TEXT PRIMARY KEY NOT NULL,
    repo_id TEXT NOT NULL REFERENCES repository(id),
    job_id TEXT REFERENCES job(id),
    command TEXT NOT NULL,
    cwd TEXT NOT NULL,
    category TEXT NOT NULL DEFAULT 'check',
    risk_level TEXT NOT NULL DEFAULT 'low' CHECK(risk_level IN ('none', 'low', 'medium', 'high', 'critical')),
    success INTEGER NOT NULL DEFAULT 0,
    exit_code INTEGER,
    duration_ms INTEGER NOT NULL DEFAULT 0,
    timed_out INTEGER NOT NULL DEFAULT 0,
    stdout_tail TEXT NOT NULL DEFAULT '',
    stderr_tail TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_verification_run_repo_id ON verification_run(repo_id);
CREATE INDEX IF NOT EXISTS idx_verification_run_job_id ON verification_run(job_id);
CREATE INDEX IF NOT EXISTS idx_verification_run_created_at ON verification_run(created_at);
