-- Tool Broker: tool registry, permission decisions, and tool invocation audit
CREATE TABLE IF NOT EXISTS tool (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL UNIQUE,
    category TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    input_schema TEXT NOT NULL DEFAULT '{}',
    output_schema TEXT NOT NULL DEFAULT '{}',
    risk_level TEXT NOT NULL DEFAULT 'low' CHECK(risk_level IN ('none', 'low', 'medium', 'high', 'critical')),
    requires_permission INTEGER NOT NULL DEFAULT 0,
    dry_run_supported INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS tool_invocation (
    id TEXT PRIMARY KEY NOT NULL,
    job_id TEXT REFERENCES job(id) ON DELETE CASCADE,
    tool_name TEXT NOT NULL,
    input TEXT NOT NULL DEFAULT '{}',
    output TEXT,
    status TEXT NOT NULL DEFAULT 'pending' CHECK(status IN ('pending', 'running', 'completed', 'failed', 'denied', 'dry_run')),
    risk_level TEXT NOT NULL DEFAULT 'low',
    permission_decision TEXT CHECK(permission_decision IN ('auto_approved', 'user_approved', 'denied', 'dry_run')),
    error_message TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    completed_at TEXT
);

CREATE INDEX IF NOT EXISTS idx_tool_invocation_job ON tool_invocation(job_id);
CREATE INDEX IF NOT EXISTS idx_tool_invocation_tool ON tool_invocation(tool_name);
CREATE INDEX IF NOT EXISTS idx_tool_category ON tool(category);

-- Pre-seed built-in tools
INSERT OR IGNORE INTO tool (id, name, category, description, risk_level, requires_permission, dry_run_supported) VALUES
    ('tool_fs_list', 'fs.list', 'fs', 'List files in a directory', 'none', 0, 0),
    ('tool_fs_read', 'fs.read', 'fs', 'Read file contents', 'low', 0, 0),
    ('tool_fs_write_patch', 'fs.write_patch', 'fs', 'Write a patch/diff to a file', 'high', 1, 1),
    ('tool_git_status', 'git.status', 'git', 'Get git status', 'none', 0, 0),
    ('tool_git_diff', 'git.diff', 'git', 'Get git diff', 'low', 0, 0),
    ('tool_git_commit', 'git.commit', 'git', 'Create a git commit', 'high', 1, 1),
    ('tool_git_tag', 'git.tag', 'git', 'Create a git tag', 'high', 1, 1),
    ('tool_shell_verify', 'shell.verify', 'shell', 'Run read-only verification command', 'medium', 1, 1),
    ('tool_shell_mutate', 'shell.mutate', 'shell', 'Run a mutating shell command', 'critical', 1, 1),
    ('tool_github_read', 'github.read', 'github', 'Read GitHub API data', 'low', 0, 0),
    ('tool_github_create_pr', 'github.create_pr', 'github', 'Create a pull request', 'high', 1, 1),
    ('tool_github_create_release', 'github.create_release', 'github', 'Create a GitHub release', 'critical', 1, 1);
