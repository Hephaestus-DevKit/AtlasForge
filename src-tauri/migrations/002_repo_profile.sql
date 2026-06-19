-- RepoProfile: detected tech stack and project metadata
CREATE TABLE IF NOT EXISTS repo_profile (
    id TEXT PRIMARY KEY NOT NULL,
    repo_id TEXT NOT NULL REFERENCES repository(id) ON DELETE CASCADE,
    languages TEXT NOT NULL DEFAULT '[]',
    frameworks TEXT NOT NULL DEFAULT '[]',
    package_managers TEXT NOT NULL DEFAULT '[]',
    scripts TEXT NOT NULL DEFAULT '{}',
    ci_systems TEXT NOT NULL DEFAULT '[]',
    has_readme INTEGER NOT NULL DEFAULT 0,
    has_license INTEGER NOT NULL DEFAULT 0,
    license_type TEXT,
    detected_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(repo_id)
);

CREATE INDEX IF NOT EXISTS idx_repo_profile_repo_id ON repo_profile(repo_id);
