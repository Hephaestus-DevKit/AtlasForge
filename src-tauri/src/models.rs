use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceRoot {
    pub id: String,
    pub path: String,
    pub label: String,
    pub access_mode: String,
    pub scan_enabled: bool,
    pub include_globs: Vec<String>,
    pub exclude_globs: Vec<String>,
    pub created_at: String,
    pub last_scanned_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectAsset {
    pub id: String,
    pub root_id: String,
    pub path: String,
    pub kind: String,
    pub name: String,
    pub primary_language: Option<String>,
    pub last_observed_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Repository {
    pub id: String,
    pub asset_id: String,
    pub worktree_path: String,
    pub git_dir_path: String,
    pub is_bare: bool,
    pub is_worktree: bool,
    pub default_branch: Option<String>,
    pub current_branch: Option<String>,
    pub head_sha: Option<String>,
    pub remote_origin_url: Option<String>,
    pub dirty_state: bool,
    pub ahead_behind: Option<AheadBehind>,
    pub last_commit_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepositorySummary {
    pub repository: Repository,
    pub profile: Option<RepoProfile>,
    pub health_score: Option<i32>,
    pub last_verification_success: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AheadBehind {
    pub ahead: i64,
    pub behind: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Job {
    pub id: String,
    #[serde(rename = "type")]
    pub job_type: String,
    pub status: String,
    pub input: String,
    pub created_at: String,
    pub updated_at: String,
    pub completed_at: Option<String>,
    pub error_message: Option<String>,
    pub progress: i32,
    pub progress_total: i32,
    pub parent_job_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobEvent {
    pub id: String,
    pub job_id: String,
    pub seq: i64,
    #[serde(rename = "type")]
    pub event_type: String,
    pub payload: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditEntry {
    pub id: String,
    pub action: String,
    pub subject: String,
    pub scope: String,
    pub capability: String,
    pub risk_level: String,
    pub detail: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddRootInput {
    pub path: String,
    pub label: String,
    pub access_mode: String,
    pub scan_enabled: bool,
    pub include_globs: Vec<String>,
    pub exclude_globs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanResult {
    pub job_id: String,
    pub repos_discovered: usize,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepoProfile {
    pub id: String,
    pub repo_id: String,
    pub languages: Vec<String>,
    pub frameworks: Vec<String>,
    pub package_managers: Vec<String>,
    pub scripts: serde_json::Value,
    pub ci_systems: Vec<String>,
    pub has_readme: bool,
    pub has_license: bool,
    pub license_type: Option<String>,
    pub detected_at: String,
}
