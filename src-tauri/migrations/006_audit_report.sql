-- Repo Audit v1: Health snapshots and findings

CREATE TABLE IF NOT EXISTS repo_health_snapshot (
    id TEXT PRIMARY KEY NOT NULL,
    repo_id TEXT NOT NULL REFERENCES repository(id) ON DELETE CASCADE,
    scan_id TEXT REFERENCES job(id) ON DELETE SET NULL,
    score INTEGER NOT NULL DEFAULT 0 CHECK(score >= 0 AND score <= 100),
    category_scores TEXT NOT NULL DEFAULT '{}',
    recommended_tasks TEXT NOT NULL DEFAULT '[]',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(repo_id, scan_id)
);

CREATE INDEX IF NOT EXISTS idx_health_snapshot_repo ON repo_health_snapshot(repo_id);

CREATE TABLE IF NOT EXISTS finding (
    id TEXT PRIMARY KEY NOT NULL,
    snapshot_id TEXT NOT NULL REFERENCES repo_health_snapshot(id) ON DELETE CASCADE,
    category TEXT NOT NULL CHECK(category IN (
        'runnable', 'tests', 'ci', 'dependencies', 'security',
        'docs', 'release', 'public_surface', 'git_hygiene', 'platform_compat'
    )),
    severity TEXT NOT NULL DEFAULT 'info' CHECK(severity IN ('info', 'warning', 'error', 'critical')),
    title TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    evidence TEXT NOT NULL DEFAULT '',
    file_path TEXT,
    line_range TEXT,
    suggested_fix TEXT,
    auto_fixable INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_finding_snapshot ON finding(snapshot_id);
CREATE INDEX IF NOT EXISTS idx_finding_category ON finding(category);
CREATE INDEX IF NOT EXISTS idx_finding_severity ON finding(severity);
