-- WorkspaceRoot: user-authorized root directories
CREATE TABLE IF NOT EXISTS workspace_root (
    id TEXT PRIMARY KEY NOT NULL,
    path TEXT NOT NULL,
    label TEXT NOT NULL DEFAULT '',
    access_mode TEXT NOT NULL DEFAULT 'read_write' CHECK(access_mode IN ('read_only', 'read_write')),
    scan_enabled INTEGER NOT NULL DEFAULT 1,
    include_globs TEXT NOT NULL DEFAULT '[]',
    exclude_globs TEXT NOT NULL DEFAULT '["node_modules",".git/objects","dist","build",".env","__pycache__",".next",".cache","target"]',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    last_scanned_at TEXT
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_workspace_root_path ON workspace_root(path);

-- ProjectAsset: discovered projects under roots
CREATE TABLE IF NOT EXISTS project_asset (
    id TEXT PRIMARY KEY NOT NULL,
    root_id TEXT NOT NULL REFERENCES workspace_root(id),
    path TEXT NOT NULL,
    kind TEXT NOT NULL DEFAULT 'git_repo' CHECK(kind IN ('git_repo', 'directory_project', 'document_collection', 'artifact_bundle')),
    name TEXT NOT NULL,
    primary_language TEXT,
    last_observed_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(path)
);

-- Repository: git repo details
CREATE TABLE IF NOT EXISTS repository (
    id TEXT PRIMARY KEY NOT NULL,
    asset_id TEXT NOT NULL REFERENCES project_asset(id),
    worktree_path TEXT NOT NULL,
    git_dir_path TEXT NOT NULL,
    is_bare INTEGER NOT NULL DEFAULT 0,
    is_worktree INTEGER NOT NULL DEFAULT 0,
    default_branch TEXT,
    current_branch TEXT,
    head_sha TEXT,
    remote_origin_url TEXT,
    dirty_state INTEGER NOT NULL DEFAULT 0,
    ahead_behind TEXT,
    last_commit_at TEXT
);

CREATE INDEX IF NOT EXISTS idx_repository_asset_id ON repository(asset_id);
CREATE INDEX IF NOT EXISTS idx_repository_worktree_path ON repository(worktree_path);

-- Job: long-running tasks
CREATE TABLE IF NOT EXISTS job (
    id TEXT PRIMARY KEY NOT NULL,
    type TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending' CHECK(status IN ('pending', 'running', 'completed', 'failed', 'cancelled')),
    input TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    completed_at TEXT
);

CREATE INDEX IF NOT EXISTS idx_job_status ON job(status);

-- JobEvent: task evidence chain
CREATE TABLE IF NOT EXISTS job_event (
    id TEXT PRIMARY KEY NOT NULL,
    job_id TEXT NOT NULL REFERENCES job(id),
    seq INTEGER NOT NULL,
    type TEXT NOT NULL,
    payload TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(job_id, seq)
);

CREATE INDEX IF NOT EXISTS idx_job_event_job_id ON job_event(job_id);

-- Audit log: immutable record of all security-relevant actions
CREATE TABLE IF NOT EXISTS audit_log (
    id TEXT PRIMARY KEY NOT NULL,
    action TEXT NOT NULL,
    subject TEXT NOT NULL,
    scope TEXT NOT NULL,
    capability TEXT NOT NULL,
    risk_level TEXT NOT NULL DEFAULT 'low' CHECK(risk_level IN ('none', 'low', 'medium', 'high', 'critical')),
    detail TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_audit_log_created_at ON audit_log(created_at);
