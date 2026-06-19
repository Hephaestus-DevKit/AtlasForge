-- Wave 2: Repository scan reliability improvements
-- Add UNIQUE constraint on repository.worktree_path for stable identity
-- Add scan_error table for structured scan error records

-- Add unique index on worktree_path if not already present
CREATE UNIQUE INDEX IF NOT EXISTS idx_repository_worktree_path_unique ON repository(worktree_path);

-- Scan error records: structured errors from scan operations
CREATE TABLE IF NOT EXISTS scan_error (
    id TEXT PRIMARY KEY NOT NULL,
    root_id TEXT NOT NULL REFERENCES workspace_root(id),
    job_id TEXT REFERENCES job(id),
    path TEXT,
    error_type TEXT NOT NULL DEFAULT 'scan_error',
    message TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_scan_error_root_id ON scan_error(root_id);
CREATE INDEX IF NOT EXISTS idx_scan_error_job_id ON scan_error(job_id);
