-- GitHub Integration: Read and Write state

CREATE TABLE IF NOT EXISTS github_integration (
    id TEXT PRIMARY KEY NOT NULL,
    repo_id TEXT NOT NULL REFERENCES repository(id) ON DELETE CASCADE,
    github_owner TEXT NOT NULL,
    github_repo TEXT NOT NULL,
    is_fork INTEGER NOT NULL DEFAULT 0,
    default_branch TEXT,
    visibility TEXT CHECK(visibility IN ('public', 'private', 'internal')),
    last_synced_at TEXT,
    sync_errors TEXT NOT NULL DEFAULT '[]',
    UNIQUE(repo_id)
);

CREATE INDEX IF NOT EXISTS idx_github_integration_repo ON github_integration(repo_id);

CREATE TABLE IF NOT EXISTS github_workflow_run (
    id TEXT PRIMARY KEY NOT NULL,
    integration_id TEXT NOT NULL REFERENCES github_integration(id) ON DELETE CASCADE,
    run_id TEXT NOT NULL,
    workflow_name TEXT NOT NULL,
    branch TEXT,
    status TEXT NOT NULL,
    conclusion TEXT,
    triggered_at TEXT,
    completed_at TEXT,
    url TEXT,
    UNIQUE(integration_id, run_id)
);

CREATE INDEX IF NOT EXISTS idx_workflow_run_integration ON github_workflow_run(integration_id);

CREATE TABLE IF NOT EXISTS github_pr (
    id TEXT PRIMARY KEY NOT NULL,
    integration_id TEXT NOT NULL REFERENCES github_integration(id) ON DELETE CASCADE,
    pr_number INTEGER NOT NULL,
    title TEXT NOT NULL,
    state TEXT NOT NULL CHECK(state IN ('open', 'closed', 'merged')),
    author TEXT,
    branch TEXT,
    url TEXT,
    created_at_gh TEXT,
    updated_at_gh TEXT,
    UNIQUE(integration_id, pr_number)
);

CREATE INDEX IF NOT EXISTS idx_pr_integration ON github_pr(integration_id);

CREATE TABLE IF NOT EXISTS github_release (
    id TEXT PRIMARY KEY NOT NULL,
    integration_id TEXT NOT NULL REFERENCES github_integration(id) ON DELETE CASCADE,
    release_id TEXT NOT NULL,
    tag_name TEXT NOT NULL,
    name TEXT,
    is_draft INTEGER NOT NULL DEFAULT 0,
    is_prerelease INTEGER NOT NULL DEFAULT 0,
    published_at TEXT,
    url TEXT,
    UNIQUE(integration_id, release_id)
);

CREATE INDEX IF NOT EXISTS idx_release_integration ON github_release(integration_id);
