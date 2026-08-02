use crate::db::Db;
use rusqlite::OptionalExtension;
use serde_json::Value;

/// GitHub integration state for a repo.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitHubIntegration {
    pub id: String,
    pub repo_id: String,
    pub github_owner: String,
    pub github_repo: String,
    pub is_fork: bool,
    pub default_branch: Option<String>,
    pub visibility: Option<String>,
    pub last_synced_at: Option<String>,
}

/// Workflow run from GitHub Actions.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowRun {
    pub id: String,
    pub integration_id: String,
    pub run_id: String,
    pub workflow_name: String,
    pub branch: Option<String>,
    pub status: String,
    pub conclusion: Option<String>,
    pub triggered_at: Option<String>,
    pub completed_at: Option<String>,
    pub url: Option<String>,
}
/// Pull request from GitHub.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitHubPR {
    pub id: String,
    pub integration_id: String,
    pub pr_number: i64,
    pub title: String,
    pub state: String,
    pub author: Option<String>,
    pub branch: Option<String>,
    pub url: Option<String>,
}

/// GitHub release.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitHubRelease {
    pub id: String,
    pub integration_id: String,
    pub release_id: String,
    pub tag_name: String,
    pub name: Option<String>,
    pub is_draft: bool,
    pub is_prerelease: bool,
    pub published_at: Option<String>,
    pub url: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitHubEvidence {
    pub workflow_runs: Vec<WorkflowRun>,
    pub pull_requests: Vec<GitHubPR>,
    pub releases: Vec<GitHubRelease>,
    pub sync_errors: Vec<String>,
}

pub fn load_evidence(repo_id: &str, db: &Db) -> Result<GitHubEvidence, String> {
    let conn = db.conn.lock().map_err(|error| error.to_string())?;
    let (integration_id, sync_errors): (String, String) = conn
        .query_row(
            "SELECT id, sync_errors FROM github_integration WHERE repo_id = ?1",
            rusqlite::params![repo_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|error| format!("GitHub integration not found: {}", error))?;

    let workflow_runs = {
        let mut stmt = conn
            .prepare(
                "SELECT id, integration_id, run_id, workflow_name, branch, status, conclusion,
                        triggered_at, completed_at, url
                 FROM github_workflow_run
                 WHERE integration_id = ?1
                 ORDER BY triggered_at DESC LIMIT 20",
            )
            .map_err(|error| error.to_string())?;
        let rows = stmt
            .query_map(rusqlite::params![&integration_id], |row| {
                Ok(WorkflowRun {
                    id: row.get(0)?,
                    integration_id: row.get(1)?,
                    run_id: row.get(2)?,
                    workflow_name: row.get(3)?,
                    branch: row.get(4)?,
                    status: row.get(5)?,
                    conclusion: row.get(6)?,
                    triggered_at: row.get(7)?,
                    completed_at: row.get(8)?,
                    url: row.get(9)?,
                })
            })
            .map_err(|error| error.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|error| error.to_string())?
    };
    let pull_requests = {
        let mut stmt = conn
            .prepare(
                "SELECT id, integration_id, pr_number, title, state, author, branch, url
                 FROM github_pr WHERE integration_id = ?1
                 ORDER BY updated_at_gh DESC LIMIT 20",
            )
            .map_err(|error| error.to_string())?;
        let rows = stmt
            .query_map(rusqlite::params![&integration_id], |row| {
                Ok(GitHubPR {
                    id: row.get(0)?,
                    integration_id: row.get(1)?,
                    pr_number: row.get(2)?,
                    title: row.get(3)?,
                    state: row.get(4)?,
                    author: row.get(5)?,
                    branch: row.get(6)?,
                    url: row.get(7)?,
                })
            })
            .map_err(|error| error.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|error| error.to_string())?
    };
    let releases = {
        let mut stmt = conn
            .prepare(
                "SELECT id, integration_id, release_id, tag_name, name, is_draft,
                        is_prerelease, published_at, url
                 FROM github_release WHERE integration_id = ?1
                 ORDER BY published_at DESC LIMIT 20",
            )
            .map_err(|error| error.to_string())?;
        let rows = stmt
            .query_map(rusqlite::params![&integration_id], |row| {
                Ok(GitHubRelease {
                    id: row.get(0)?,
                    integration_id: row.get(1)?,
                    release_id: row.get(2)?,
                    tag_name: row.get(3)?,
                    name: row.get(4)?,
                    is_draft: row.get::<_, i32>(5)? != 0,
                    is_prerelease: row.get::<_, i32>(6)? != 0,
                    published_at: row.get(7)?,
                    url: row.get(8)?,
                })
            })
            .map_err(|error| error.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|error| error.to_string())?
    };
    Ok(GitHubEvidence {
        workflow_runs,
        pull_requests,
        releases,
        sync_errors: serde_json::from_str(&sync_errors).unwrap_or_default(),
    })
}

pub fn set_sync_errors(integration_id: &str, errors: &[String], db: &Db) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|error| error.to_string())?;
    conn.execute(
        "UPDATE github_integration
         SET sync_errors = ?1, last_synced_at = ?2
         WHERE id = ?3",
        rusqlite::params![
            serde_json::to_string(errors).unwrap_or_else(|_| "[]".into()),
            chrono::Utc::now().to_rfc3339(),
            integration_id,
        ],
    )
    .map_err(|error| error.to_string())?;
    Ok(())
}

/// Check if gh CLI is authenticated.
pub fn check_gh_auth() -> Result<GhAuthStatus, String> {
    let output = std::process::Command::new("gh")
        .args(["auth", "status"])
        .output()
        .map_err(|e| format!("gh CLI not found: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let combined = format!("{}\n{}", stdout, stderr);

    let authenticated = combined.contains("Logged in");
    let username = combined
        .lines()
        .find(|l| l.contains("Logged in to"))
        .and_then(|l| {
            // Extract username from "Logged in to github.com as USERNAME"
            let parts: Vec<&str> = l.split(" as ").collect();
            parts
                .get(1)
                .map(|s| s.split_whitespace().next().unwrap_or("").to_string())
        });

    Ok(GhAuthStatus {
        authenticated,
        username,
        message: if authenticated {
            "GitHub CLI authenticated".into()
        } else {
            "Not authenticated. Run `gh auth login`".into()
        },
    })
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GhAuthStatus {
    pub authenticated: bool,
    pub username: Option<String>,
    pub message: String,
}

/// Resolve a repo's GitHub remote and create integration record.
pub fn resolve_github_repo(
    repo_id: &str,
    worktree_path: &str,
    db: &Db,
) -> Result<GitHubIntegration, String> {
    // Get remote URL
    let remote_url = std::process::Command::new("git")
        .args(["config", "--get", "remote.origin.url"])
        .current_dir(worktree_path)
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .map_err(|e| format!("Cannot get git remote: {}", e))?;

    if remote_url.is_empty() {
        return Err("No remote.origin.url configured".into());
    }

    let (owner, repo_name) = parse_github_url(&remote_url)?;

    // Fetch repo info via gh CLI
    let repo_info = std::process::Command::new("gh")
        .args([
            "repo",
            "view",
            &format!("{}/{}", owner, repo_name),
            "--json",
            "isFork,defaultBranchRef,visibility",
        ])
        .output()
        .map_err(|e| format!("gh CLI not available: {}", e))?;

    let (is_fork, default_branch, visibility) = if repo_info.status.success() {
        let info: Value = serde_json::from_str(&String::from_utf8_lossy(&repo_info.stdout))
            .unwrap_or(Value::Null);
        (
            info.get("isFork")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
            info.get("defaultBranchRef")
                .and_then(|v| v.get("name"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            info.get("visibility")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
        )
    } else {
        (false, None, None)
    };

    // Keep a stable integration id when resolving the same repository again.
    // Replacing a row with a new id would cascade-delete all previously synced
    // workflow, PR, and release evidence.
    let existing_id = load_integration(repo_id, db)?
        .filter(|value| {
            value.github_owner.eq_ignore_ascii_case(&owner)
                && value.github_repo.eq_ignore_ascii_case(&repo_name)
        })
        .map(|value| value.id);
    let integration = GitHubIntegration {
        id: existing_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
        repo_id: repo_id.to_string(),
        github_owner: owner,
        github_repo: repo_name,
        is_fork,
        default_branch,
        visibility,
        last_synced_at: Some(chrono::Utc::now().to_rfc3339()),
    };

    // Save to database
    save_integration(&integration, db)?;

    Ok(integration)
}

/// Sync workflow runs from GitHub.
pub fn sync_workflow_runs(
    integration: &GitHubIntegration,
    db: &Db,
) -> Result<Vec<WorkflowRun>, String> {
    let output = std::process::Command::new("gh")
        .args([
            "run",
            "list",
            "--repo",
            &format!("{}/{}", integration.github_owner, integration.github_repo),
            "--limit",
            "20",
            "--json",
            "databaseId,name,headBranch,status,conclusion,createdAt,updatedAt,url",
        ])
        .output()
        .map_err(|e| format!("gh CLI not available: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "Failed to fetch workflow runs: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let runs: Vec<Value> = serde_json::from_str(&String::from_utf8_lossy(&output.stdout))
        .map_err(|e| format!("Invalid response: {}", e))?;

    let mut workflow_runs = Vec::new();
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    for run in runs {
        let run_id = run
            .get("databaseId")
            .and_then(|v| v.as_i64())
            .ok_or_else(|| "Workflow run response is missing databaseId".to_string())?
            .to_string();
        let stable_id = conn
            .query_row(
                "SELECT id FROM github_workflow_run WHERE integration_id = ?1 AND run_id = ?2",
                rusqlite::params![integration.id, run_id],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|error| format!("Failed to resolve workflow run identity: {}", error))?
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        let wr = WorkflowRun {
            id: stable_id,
            integration_id: integration.id.clone(),
            run_id: run_id.clone(),
            workflow_name: run
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string(),
            branch: run
                .get("headBranch")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            status: run
                .get("status")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string(),
            conclusion: run
                .get("conclusion")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            triggered_at: run
                .get("createdAt")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            completed_at: run
                .get("updatedAt")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            url: run
                .get("url")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
        };

        // Upsert
        conn.execute(
            "INSERT INTO github_workflow_run (id, integration_id, run_id, workflow_name, branch, status, conclusion, triggered_at, completed_at, url)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
             ON CONFLICT(integration_id, run_id) DO UPDATE SET
               workflow_name = excluded.workflow_name,
               branch = excluded.branch,
               status = excluded.status,
               conclusion = excluded.conclusion,
               triggered_at = excluded.triggered_at,
               completed_at = excluded.completed_at,
               url = excluded.url",
            rusqlite::params![
                wr.id, wr.integration_id, wr.run_id, wr.workflow_name, wr.branch,
                wr.status, wr.conclusion, wr.triggered_at, wr.completed_at, wr.url,
            ],
        ).map_err(|error| format!("Failed to store workflow run: {}", error))?;

        workflow_runs.push(wr);
    }

    Ok(workflow_runs)
}

/// Sync PRs from GitHub.
pub fn sync_prs(integration: &GitHubIntegration, db: &Db) -> Result<Vec<GitHubPR>, String> {
    let output = std::process::Command::new("gh")
        .args([
            "pr",
            "list",
            "--repo",
            &format!("{}/{}", integration.github_owner, integration.github_repo),
            "--state",
            "all",
            "--limit",
            "20",
            "--json",
            "number,title,state,author,headRefName,url",
        ])
        .output()
        .map_err(|e| format!("gh CLI not available: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "Failed to fetch PRs: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let prs: Vec<Value> = serde_json::from_str(&String::from_utf8_lossy(&output.stdout))
        .map_err(|e| format!("Invalid response: {}", e))?;

    let mut result = Vec::new();
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    for pr in prs {
        let pr_number = pr
            .get("number")
            .and_then(|v| v.as_i64())
            .ok_or_else(|| "Pull request response is missing number".to_string())?;
        let state = normalize_pr_state(
            pr.get("state")
                .and_then(|value| value.as_str())
                .ok_or_else(|| "Pull request response is missing state".to_string())?,
        )?;
        let stable_id = conn
            .query_row(
                "SELECT id FROM github_pr WHERE integration_id = ?1 AND pr_number = ?2",
                rusqlite::params![integration.id, pr_number],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|error| format!("Failed to resolve pull request identity: {}", error))?
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        let gh_pr = GitHubPR {
            id: stable_id,
            integration_id: integration.id.clone(),
            pr_number,
            title: pr
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            state,
            author: pr
                .get("author")
                .and_then(|a| a.get("login"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            branch: pr
                .get("headRefName")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            url: pr
                .get("url")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
        };

        conn.execute(
            "INSERT INTO github_pr (id, integration_id, pr_number, title, state, author, branch, url, created_at_gh, updated_at_gh)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, datetime('now'), datetime('now'))
             ON CONFLICT(integration_id, pr_number) DO UPDATE SET
               title = excluded.title,
               state = excluded.state,
               author = excluded.author,
               branch = excluded.branch,
               url = excluded.url,
               updated_at_gh = excluded.updated_at_gh",
            rusqlite::params![
                gh_pr.id, gh_pr.integration_id, gh_pr.pr_number, gh_pr.title,
                gh_pr.state, gh_pr.author, gh_pr.branch, gh_pr.url,
            ],
        ).map_err(|error| format!("Failed to store pull request: {}", error))?;

        result.push(gh_pr);
    }

    Ok(result)
}

/// Sync releases from GitHub.
pub fn sync_releases(
    integration: &GitHubIntegration,
    db: &Db,
) -> Result<Vec<GitHubRelease>, String> {
    let output = std::process::Command::new("gh")
        .args([
            "release",
            "list",
            "--repo",
            &format!("{}/{}", integration.github_owner, integration.github_repo),
            "--limit",
            "20",
            "--json",
            "databaseId,tagName,name,isDraft,isPrerelease,publishedAt,url",
        ])
        .output()
        .map_err(|e| format!("gh CLI not available: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "Failed to fetch releases: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let releases: Vec<Value> = serde_json::from_str(&String::from_utf8_lossy(&output.stdout))
        .map_err(|e| format!("Invalid response: {}", e))?;

    let mut result = Vec::new();
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    for rel in releases {
        let release_id = rel
            .get("databaseId")
            .and_then(|v| v.as_i64())
            .ok_or_else(|| "Release response is missing databaseId".to_string())?
            .to_string();
        let stable_id = conn
            .query_row(
                "SELECT id FROM github_release WHERE integration_id = ?1 AND release_id = ?2",
                rusqlite::params![integration.id, release_id],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|error| format!("Failed to resolve release identity: {}", error))?
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        let gh_rel = GitHubRelease {
            id: stable_id,
            integration_id: integration.id.clone(),
            release_id,
            tag_name: rel
                .get("tagName")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            name: rel
                .get("name")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            is_draft: rel
                .get("isDraft")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
            is_prerelease: rel
                .get("isPrerelease")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
            published_at: rel
                .get("publishedAt")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            url: rel
                .get("url")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
        };

        conn.execute(
            "INSERT INTO github_release (id, integration_id, release_id, tag_name, name, is_draft, is_prerelease, published_at, url)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
             ON CONFLICT(integration_id, release_id) DO UPDATE SET
               tag_name = excluded.tag_name,
               name = excluded.name,
               is_draft = excluded.is_draft,
               is_prerelease = excluded.is_prerelease,
               published_at = excluded.published_at,
               url = excluded.url",
            rusqlite::params![
                gh_rel.id, gh_rel.integration_id, gh_rel.release_id, gh_rel.tag_name,
                gh_rel.name, gh_rel.is_draft as i32, gh_rel.is_prerelease as i32,
                gh_rel.published_at, gh_rel.url,
            ],
        ).map_err(|error| format!("Failed to store release: {}", error))?;

        result.push(gh_rel);
    }

    Ok(result)
}

/// Create a pull request (requires permission).
pub fn create_pr(
    integration: &GitHubIntegration,
    title: &str,
    body: &str,
    head: &str,
    base: &str,
    draft: bool,
) -> Result<GitHubPR, String> {
    let repo = format!("{}/{}", integration.github_owner, integration.github_repo);
    let mut args = vec![
        "pr",
        "create",
        "--repo",
        repo.as_str(),
        "--title",
        title,
        "--body",
        body,
        "--head",
        head,
        "--base",
        base,
    ];

    if draft {
        args.push("--draft");
    }

    let output = std::process::Command::new("gh")
        .args(&args)
        .output()
        .map_err(|e| format!("gh CLI not available: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "Failed to create PR: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let url = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let pr_number: i64 = url
        .split('/')
        .next_back()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    Ok(GitHubPR {
        id: uuid::Uuid::new_v4().to_string(),
        integration_id: integration.id.clone(),
        pr_number,
        title: title.to_string(),
        state: "open".to_string(),
        author: None,
        branch: Some(head.to_string()),
        url: Some(url),
    })
}

/// Create a GitHub release (requires permission).
pub fn create_release(
    integration: &GitHubIntegration,
    tag: &str,
    name: &str,
    body: &str,
    draft: bool,
    prerelease: bool,
) -> Result<GitHubRelease, String> {
    let repo = format!("{}/{}", integration.github_owner, integration.github_repo);
    let mut args = vec![
        "release",
        "create",
        tag,
        "--repo",
        repo.as_str(),
        "--title",
        name,
        "--notes",
        body,
    ];

    if draft {
        args.push("--draft");
    }
    if prerelease {
        args.push("--prerelease");
    }

    let output = std::process::Command::new("gh")
        .args(&args)
        .output()
        .map_err(|e| format!("gh CLI not available: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "Failed to create release: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    Ok(GitHubRelease {
        id: uuid::Uuid::new_v4().to_string(),
        integration_id: integration.id.clone(),
        release_id: "".to_string(),
        tag_name: tag.to_string(),
        name: Some(name.to_string()),
        is_draft: draft,
        is_prerelease: prerelease,
        published_at: Some(chrono::Utc::now().to_rfc3339()),
        url: Some(String::from_utf8_lossy(&output.stdout).trim().to_string()),
    })
}

/// Rerun a failed workflow.
pub fn rerun_workflow(integration: &GitHubIntegration, run_id: &str) -> Result<(), String> {
    let output = std::process::Command::new("gh")
        .args([
            "run",
            "rerun",
            run_id,
            "--repo",
            &format!("{}/{}", integration.github_owner, integration.github_repo),
        ])
        .output()
        .map_err(|e| format!("gh CLI not available: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "Failed to rerun workflow: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    Ok(())
}

// --- Internal ---

fn normalize_pr_state(state: &str) -> Result<String, String> {
    let normalized = state.to_ascii_lowercase();
    if matches!(normalized.as_str(), "open" | "closed" | "merged") {
        Ok(normalized)
    } else {
        Err(format!("Unsupported pull request state: {}", state))
    }
}

fn parse_github_url(url: &str) -> Result<(String, String), String> {
    // Handle https://github.com/owner/repo.git
    // Handle git@github.com:owner/repo.git
    let url = url.trim().trim_end_matches(".git");

    if url.starts_with("https://github.com/") || url.starts_with("http://github.com/") {
        let parts: Vec<&str> = url.split('/').collect();
        if parts.len() >= 5 {
            return Ok((parts[3].to_string(), parts[4].to_string()));
        }
    } else if let Some(rest) = url.strip_prefix("git@github.com:") {
        let parts: Vec<&str> = rest.split('/').collect();
        if parts.len() >= 2 {
            return Ok((parts[0].to_string(), parts[1].to_string()));
        }
    }

    Err(format!("Cannot parse GitHub URL: {}", url))
}

fn save_integration(integration: &GitHubIntegration, db: &Db) -> Result<(), String> {
    let mut conn = db.conn.lock().map_err(|e| e.to_string())?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    let existing_remote = tx
        .query_row(
            "SELECT github_owner, github_repo FROM github_integration WHERE repo_id = ?1",
            rusqlite::params![integration.repo_id],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    if existing_remote.is_some_and(|(owner, repo)| {
        !owner.eq_ignore_ascii_case(&integration.github_owner)
            || !repo.eq_ignore_ascii_case(&integration.github_repo)
    }) {
        // A local repository may be repointed to another GitHub remote. In
        // that case old workflow/PR/release evidence belongs to a different
        // remote and must be removed via the integration cascade.
        tx.execute(
            "DELETE FROM github_integration WHERE repo_id = ?1",
            rusqlite::params![integration.repo_id],
        )
        .map_err(|e| e.to_string())?;
    }
    tx.execute(
        "INSERT INTO github_integration (id, repo_id, github_owner, github_repo, is_fork, default_branch, visibility, last_synced_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
         ON CONFLICT(repo_id) DO UPDATE SET
           github_owner = excluded.github_owner,
           github_repo = excluded.github_repo,
           is_fork = excluded.is_fork,
           default_branch = excluded.default_branch,
           visibility = excluded.visibility,
           last_synced_at = excluded.last_synced_at",
        rusqlite::params![
            integration.id,
            integration.repo_id,
            integration.github_owner,
            integration.github_repo,
            integration.is_fork as i32,
            integration.default_branch,
            integration.visibility,
            integration.last_synced_at,
        ],
    )
    .map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

/// Load GitHub integration for a repo.
pub fn load_integration(repo_id: &str, db: &Db) -> Result<Option<GitHubIntegration>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let result = conn.query_row(
        "SELECT id, repo_id, github_owner, github_repo, is_fork, default_branch, visibility, last_synced_at FROM github_integration WHERE repo_id = ?1",
        rusqlite::params![repo_id],
        |row| {
            Ok(GitHubIntegration {
                id: row.get(0)?,
                repo_id: row.get(1)?,
                github_owner: row.get(2)?,
                github_repo: row.get(3)?,
                is_fork: row.get::<_, i32>(4)? != 0,
                default_branch: row.get(5)?,
                visibility: row.get(6)?,
                last_synced_at: row.get(7)?,
            })
        },
    );

    match result {
        Ok(integration) => Ok(Some(integration)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn integration(id: &str, owner: &str, repo: &str) -> GitHubIntegration {
        GitHubIntegration {
            id: id.into(),
            repo_id: "repo-id".into(),
            github_owner: owner.into(),
            github_repo: repo.into(),
            is_fork: false,
            default_branch: Some("main".into()),
            visibility: Some("private".into()),
            last_synced_at: None,
        }
    }

    #[test]
    fn github_pr_states_are_normalized_for_the_database_constraint() {
        assert_eq!(normalize_pr_state("OPEN").unwrap(), "open");
        assert_eq!(normalize_pr_state("MERGED").unwrap(), "merged");
        assert!(normalize_pr_state("UNKNOWN").is_err());
    }

    #[test]
    fn parses_https_and_ssh_github_remotes() {
        assert_eq!(
            parse_github_url("https://github.com/acme/widget.git").unwrap(),
            ("acme".into(), "widget".into())
        );
        assert_eq!(
            parse_github_url("git@github.com:acme/widget.git").unwrap(),
            ("acme".into(), "widget".into())
        );
    }

    #[test]
    fn changing_remote_replaces_integration_and_cascades_stale_evidence() {
        let temp = tempfile::tempdir().unwrap();
        let db = Db::new(&temp.path().join("github.db")).unwrap();
        {
            let conn = db.conn.lock().unwrap();
            conn.execute(
                "INSERT INTO workspace_root(id, path, label) VALUES('root', 'C:/root', 'Root')",
                [],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO project_asset(id, root_id, path, name) VALUES('asset', 'root', 'C:/root/repo', 'Repo')",
                [],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO repository(id, asset_id, worktree_path, git_dir_path) VALUES('repo-id', 'asset', 'C:/root/repo', 'C:/root/repo/.git')",
                [],
            )
            .unwrap();
        }

        save_integration(&integration("old", "acme", "one"), &db).unwrap();
        db.conn
            .lock()
            .unwrap()
            .execute(
                "INSERT INTO github_workflow_run(id, integration_id, run_id, workflow_name, status) VALUES('run', 'old', '1', 'CI', 'completed')",
                [],
            )
            .unwrap();

        save_integration(&integration("new", "acme", "two"), &db).unwrap();
        let conn = db.conn.lock().unwrap();
        let integration_id: String = conn
            .query_row(
                "SELECT id FROM github_integration WHERE repo_id = 'repo-id'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let evidence_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM github_workflow_run", [], |row| row.get(0))
            .unwrap();
        assert_eq!(integration_id, "new");
        assert_eq!(evidence_count, 0);
    }
}
