use crate::db::Db;
use std::path::{Component, Path, PathBuf};

/// Artifact types.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Artifact {
    pub id: String,
    pub job_id: String,
    pub artifact_type: String,
    pub title: String,
    pub content: String,
    pub file_path: Option<String>,
    pub metadata: serde_json::Value,
}

/// Patch proposal.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PatchProposal {
    pub id: String,
    pub job_id: String,
    pub artifact_id: Option<String>,
    pub repo_id: String,
    pub file_path: String,
    pub patch_content: String,
    pub description: String,
    pub status: String,
    pub applied_at: Option<String>,
    pub rolled_back_at: Option<String>,
    pub verification_result: Option<String>,
}

/// Create an artifact.
pub fn create_artifact(artifact: &Artifact, db: &Db) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO artifact (id, job_id, type, title, content, file_path, metadata, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        rusqlite::params![
            artifact.id,
            artifact.job_id,
            artifact.artifact_type,
            artifact.title,
            artifact.content,
            artifact.file_path,
            serde_json::to_string(&artifact.metadata).unwrap_or_default(),
            now,
        ],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// List artifacts for a job.
pub fn list_artifacts(job_id: &str, db: &Db) -> Result<Vec<Artifact>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT id, job_id, type, title, content, file_path, metadata FROM artifact WHERE job_id = ?1 ORDER BY created_at")
        .map_err(|e| e.to_string())?;

    let artifacts = stmt
        .query_map(rusqlite::params![job_id], |row| {
            let metadata_str: String = row.get(6)?;
            Ok(Artifact {
                id: row.get(0)?,
                job_id: row.get(1)?,
                artifact_type: row.get(2)?,
                title: row.get(3)?,
                content: row.get(4)?,
                file_path: row.get(5)?,
                metadata: serde_json::from_str(&metadata_str).unwrap_or(serde_json::Value::Null),
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(artifacts)
}

/// Create a patch proposal.
pub fn create_patch_proposal(proposal: &PatchProposal, db: &Db) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO patch_proposal (id, job_id, artifact_id, repo_id, file_path, patch_content, description, status, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        rusqlite::params![
            proposal.id,
            proposal.job_id,
            proposal.artifact_id,
            proposal.repo_id,
            proposal.file_path,
            proposal.patch_content,
            proposal.description,
            proposal.status,
            now,
        ],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// Approve and apply a patch proposal.
pub fn apply_patch(proposal_id: &str, db: &Db) -> Result<PatchProposal, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    // Load proposal
    let proposal: PatchProposal = conn.query_row(
        "SELECT id, job_id, artifact_id, repo_id, file_path, patch_content, description, status, applied_at, rolled_back_at, verification_result FROM patch_proposal WHERE id = ?1",
        rusqlite::params![proposal_id],
        |row| {
            Ok(PatchProposal {
                id: row.get(0)?,
                job_id: row.get(1)?,
                artifact_id: row.get(2)?,
                repo_id: row.get(3)?,
                file_path: row.get(4)?,
                patch_content: row.get(5)?,
                description: row.get(6)?,
                status: row.get(7)?,
                applied_at: row.get(8)?,
                rolled_back_at: row.get(9)?,
                verification_result: row.get(10)?,
            })
        },
    )
    .map_err(|e| e.to_string())?;

    if proposal.status != "proposed" && proposal.status != "approved" {
        return Err(format!(
            "Cannot apply patch with status '{}'",
            proposal.status
        ));
    }
    drop(conn);

    // Apply the patch using git apply
    let repo_path = get_repo_worktree_path(&proposal.repo_id, db)?;
    ensure_repo_write_allowed(&proposal.repo_id, db)?;
    validate_patch_paths(&proposal.patch_content, None)?;
    match run_git_apply(&repo_path, &proposal.patch_content, &["--check"]) {
        Ok(()) => {
            // Actually apply
            let apply_result = run_git_apply(&repo_path, &proposal.patch_content, &[]);
            match apply_result {
                Ok(()) => {
                    let now = chrono::Utc::now().to_rfc3339();

                    // Run post-apply verification
                    let verification_json =
                        run_post_apply_verification(&proposal.repo_id, &repo_path, db);

                    let conn = db.conn.lock().map_err(|e| e.to_string())?;
                    conn.execute(
                        "UPDATE patch_proposal SET status = 'applied', applied_at = ?1, verification_result = ?2 WHERE id = ?3",
                        rusqlite::params![now, verification_json, proposal_id],
                    )
                    .map_err(|e| e.to_string())?;
                    drop(conn);

                    write_audit_log(
                        db,
                        "patch_applied",
                        &format!("repo:{}", proposal.repo_id),
                        "fs.write_patch",
                        "high",
                        &serde_json::json!({
                            "proposal_id": proposal_id,
                            "file_path": proposal.file_path,
                            "verification_passed": verification_json.as_ref().and_then(|v| serde_json::from_str::<serde_json::Value>(v).ok()).and_then(|v| v.get("allPassed").and_then(|p| p.as_bool())).unwrap_or(false),
                        }).to_string(),
                    )?;

                    let mut applied = proposal.clone();
                    applied.status = "applied".to_string();
                    applied.applied_at = Some(now);
                    applied.verification_result = verification_json;
                    Ok(applied)
                }
                Err(e) => {
                    let conn = db.conn.lock().map_err(|lock_err| lock_err.to_string())?;
                    conn.execute(
                        "UPDATE patch_proposal SET status = 'conflict' WHERE id = ?1",
                        rusqlite::params![proposal_id],
                    )
                    .ok();
                    Err(format!("Patch apply failed: {}", e))
                }
            }
        }
        Err(e) => {
            let conn = db.conn.lock().map_err(|lock_err| lock_err.to_string())?;
            conn.execute(
                "UPDATE patch_proposal SET status = 'conflict' WHERE id = ?1",
                rusqlite::params![proposal_id],
            )
            .ok();
            Err(format!(
                "Patch has conflicts and cannot be applied cleanly: {}",
                e
            ))
        }
    }
}

/// Reject a patch proposal.
pub fn reject_patch(proposal_id: &str, reason: &str, db: &Db) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE patch_proposal SET status = 'rejected' WHERE id = ?1",
        rusqlite::params![proposal_id],
    )
    .map_err(|e| e.to_string())?;

    drop(conn);
    write_audit_log(
        db,
        "patch_rejected",
        proposal_id,
        "fs.write_patch",
        "high",
        &serde_json::json!({ "reason": reason }).to_string(),
    )?;

    Ok(())
}

/// Roll back an applied patch.
pub fn rollback_patch(proposal_id: &str, db: &Db) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let (repo_id, patch_content): (String, String) = conn.query_row(
        "SELECT repo_id, patch_content FROM patch_proposal WHERE id = ?1 AND status = 'applied'",
        rusqlite::params![proposal_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )
    .map_err(|e| format!("Cannot rollback: {}", e))?;
    drop(conn);

    let repo_path = get_repo_worktree_path(&repo_id, db)?;
    ensure_repo_write_allowed(&repo_id, db)?;
    validate_patch_paths(&patch_content, None)?;

    // Reverse apply using git apply -R. Do not try to manually invert patch text.
    match run_git_apply(&repo_path, &patch_content, &["-R"]) {
        Ok(()) => {
            let now = chrono::Utc::now().to_rfc3339();
            let conn = db.conn.lock().map_err(|e| e.to_string())?;
            conn.execute(
                "UPDATE patch_proposal SET status = 'rolled_back', rolled_back_at = ?1 WHERE id = ?2",
                rusqlite::params![now, proposal_id],
            )
            .map_err(|e| e.to_string())?;
            drop(conn);

            write_audit_log(
                db,
                "patch_rolled_back",
                &format!("repo:{}", repo_id),
                "fs.write_patch",
                "high",
                &serde_json::json!({ "proposal_id": proposal_id }).to_string(),
            )?;
            Ok(())
        }
        Err(e) => Err(format!(
            "Rollback failed: {}. Review the working tree and revert the affected files manually.",
            e
        )),
    }
}

/// List patch proposals for a repo.
pub fn list_patch_proposals(repo_id: &str, db: &Db) -> Result<Vec<PatchProposal>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT id, job_id, artifact_id, repo_id, file_path, patch_content, description, status, applied_at, rolled_back_at, verification_result FROM patch_proposal WHERE repo_id = ?1 ORDER BY created_at DESC")
        .map_err(|e| e.to_string())?;

    let proposals = stmt
        .query_map(rusqlite::params![repo_id], |row| {
            Ok(PatchProposal {
                id: row.get(0)?,
                job_id: row.get(1)?,
                artifact_id: row.get(2)?,
                repo_id: row.get(3)?,
                file_path: row.get(4)?,
                patch_content: row.get(5)?,
                description: row.get(6)?,
                status: row.get(7)?,
                applied_at: row.get(8)?,
                rolled_back_at: row.get(9)?,
                verification_result: row.get(10)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(proposals)
}

// --- Internal helpers ---

fn run_git_apply(repo_path: &str, patch_content: &str, args: &[&str]) -> Result<(), String> {
    let mut child = std::process::Command::new("git")
        .arg("apply")
        .args(args)
        .current_dir(repo_path)
        .stdin(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to run git apply: {}", e))?;

    if let Some(mut stdin) = child.stdin.take() {
        use std::io::Write;
        stdin
            .write_all(patch_content.as_bytes())
            .map_err(|e| format!("Failed to write patch: {}", e))?;
    }

    let output = child
        .wait_with_output()
        .map_err(|e| format!("Failed to wait for git apply: {}", e))?;
    if !output.status.success() {
        return Err(format!(
            "git apply failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    Ok(())
}

fn get_repo_worktree_path(repo_id: &str, db: &Db) -> Result<String, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.query_row(
        "SELECT worktree_path FROM repository WHERE id = ?1",
        rusqlite::params![repo_id],
        |row| row.get(0),
    )
    .map_err(|e| format!("Repository not found: {}", e))
}

fn ensure_repo_write_allowed(repo_id: &str, db: &Db) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let (access_mode, root_label): (String, String) = conn
        .query_row(
            "SELECT workspace_root.access_mode, workspace_root.label
             FROM repository
             JOIN project_asset ON project_asset.id = repository.asset_id
             JOIN workspace_root ON workspace_root.id = project_asset.root_id
             WHERE repository.id = ?1",
            rusqlite::params![repo_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|e| format!("Repository workspace root not found: {}", e))?;
    if access_mode == "read_only" {
        return Err(format!(
            "Repository is under read-only workspace root '{}'",
            root_label
        ));
    }
    Ok(())
}

fn validate_relative_path(path: &str) -> Result<PathBuf, String> {
    let path = Path::new(path);
    if path.as_os_str().is_empty() || path.is_absolute() {
        return Err("Patch paths must be non-empty repository-relative paths".into());
    }
    if path.components().any(|component| {
        matches!(
            component,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    }) {
        return Err(format!(
            "Patch path escapes the repository boundary: {}",
            path.display()
        ));
    }
    Ok(path.to_path_buf())
}

fn resolve_existing_repo_file(repo_path: &str, relative_path: &str) -> Result<PathBuf, String> {
    let relative = validate_relative_path(relative_path)?;
    let repo = Path::new(repo_path)
        .canonicalize()
        .map_err(|e| format!("Cannot resolve repository path: {}", e))?;
    let resolved = repo
        .join(&relative)
        .canonicalize()
        .map_err(|e| format!("Cannot resolve target file '{}': {}", relative_path, e))?;
    if !resolved.starts_with(&repo) {
        return Err(format!(
            "Target file is outside the repository: {}",
            relative_path
        ));
    }
    if !resolved.is_file() {
        return Err(format!("Target path is not a file: {}", relative_path));
    }
    Ok(resolved)
}

fn diff_header_path(line: &str) -> Option<&str> {
    line.strip_prefix("--- ")
        .or_else(|| line.strip_prefix("+++ "))
        .and_then(|value| value.split_whitespace().next())
}

fn validate_patch_paths(
    patch_content: &str,
    expected_file: Option<&str>,
) -> Result<String, String> {
    if !patch_content.lines().any(|line| line.starts_with("@@")) {
        return Err("Patch does not contain a unified diff hunk".into());
    }

    let mut paths = Vec::new();
    for line in patch_content.lines() {
        let Some(header_path) = diff_header_path(line) else {
            continue;
        };
        if header_path == "/dev/null" {
            continue;
        }
        let relative = header_path
            .strip_prefix("a/")
            .or_else(|| header_path.strip_prefix("b/"))
            .ok_or_else(|| {
                format!(
                    "Patch header must use a/ or b/ repository-relative paths: {}",
                    header_path
                )
            })?;
        let normalized = validate_relative_path(relative)?
            .to_string_lossy()
            .replace('\\', "/");
        if !paths.contains(&normalized) {
            paths.push(normalized);
        }
    }

    if paths.len() != 1 {
        return Err(format!(
            "A patch proposal must target exactly one repository file; found {}",
            paths.len()
        ));
    }

    if let Some(expected_file) = expected_file {
        let expected = validate_relative_path(expected_file)?
            .to_string_lossy()
            .replace('\\', "/");
        if paths[0] != expected {
            return Err(format!(
                "Patch targets '{}' but the requested file was '{}'",
                paths[0], expected
            ));
        }
    }

    Ok(paths.remove(0))
}

/// Run post-apply verification after a patch is applied.
/// Detects available verification commands and runs them, returning a JSON summary.
fn run_post_apply_verification(repo_id: &str, repo_path: &str, db: &Db) -> Option<String> {
    let commands: Vec<_> = crate::verification::detect_commands(repo_path)
        .into_iter()
        .filter(|cmd| cmd.category != "install")
        .filter(|cmd| cmd.risk_level != "high" && cmd.risk_level != "critical")
        .collect();
    if commands.is_empty() {
        return None;
    }

    let mut results = Vec::new();
    let mut all_passed = true;

    for cmd in &commands {
        let result =
            crate::verification::run_verification(&cmd.command, repo_path, cmd.timeout_secs);
        if !result.success {
            all_passed = false;
        }
        // Log each verification
        let _ = write_audit_log(
            db,
            "post_apply_verification",
            &format!("repo:{}", repo_id),
            "verification",
            if result.success { "low" } else { "high" },
            &serde_json::json!({
                "command": cmd.command,
                "success": result.success,
                "exitCode": result.exit_code,
                "durationMs": result.duration_ms,
            })
            .to_string(),
        );
        results.push(serde_json::json!({
            "command": cmd.command,
            "success": result.success,
            "exitCode": result.exit_code,
            "durationMs": result.duration_ms,
            "category": cmd.category,
        }));
    }

    Some(
        serde_json::json!({
            "allPassed": all_passed,
            "commandCount": commands.len(),
            "results": results,
        })
        .to_string(),
    )
}

/// Fix plan generated from AI analysis of audit findings.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FixPlan {
    pub id: String,
    pub job_id: String,
    pub repo_id: String,
    pub snapshot_id: String,
    pub provider_id: String,
    pub model: String,
    pub plan_content: String,
    pub context_summary: String,
    pub tokens_in: usize,
    pub tokens_out: usize,
    pub created_at: String,
}

/// Preview the context that would be sent to AI for a fix plan, without actually calling AI.
/// Returns the ContextPack summary and the redacted prompt preview.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextPreview {
    pub purpose: String,
    pub sections: Vec<ContextPreviewSection>,
    pub total_tokens_estimate: usize,
    pub max_tokens: usize,
    pub prompt_preview: String,
    pub secrets_found: Vec<crate::ai_provider::SecretMatch>,
    pub secret_count_after_redaction: usize,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextPreviewSection {
    pub label: String,
    pub source: String,
    pub tokens_estimate: usize,
    pub content_preview: String, // first 500 chars
}

pub fn preview_fix_plan_context(
    repo_id: &str,
    snapshot_id: &str,
    db: &Db,
) -> Result<ContextPreview, String> {
    use crate::ai_provider::{self, ContextPack};
    use crate::auditor;

    // 1. Load findings
    let findings = auditor::load_findings(snapshot_id, db)?;
    if findings.is_empty() {
        return Err("No findings to analyze. Run an audit first.".into());
    }

    // 2. Load snapshot for context
    let snapshot = auditor::load_snapshot(snapshot_id, repo_id, db)?;

    // 3. Build context pack (same logic as generate_fix_plan)
    let findings_summary: Vec<String> = findings
        .iter()
        .map(|f| {
            format!(
                "- [{}] {}: {} (severity: {})",
                f.category, f.title, f.description, f.severity
            )
        })
        .collect();

    let findings_detail: Vec<String> = findings
        .iter()
        .map(|f| {
            let mut s = format!(
                "## Finding: {}\nCategory: {}\nSeverity: {}",
                f.title, f.category, f.severity
            );
            if !f.description.is_empty() {
                s.push_str(&format!("\nDescription: {}", f.description));
            }
            if !f.evidence.is_empty() {
                s.push_str(&format!("\nEvidence: {}", f.evidence));
            }
            if let Some(ref fp) = f.file_path {
                s.push_str(&format!("\nFile: {}", fp));
            }
            if let Some(ref fix) = f.suggested_fix {
                s.push_str(&format!("\nSuggested fix: {}", fix));
            }
            s
        })
        .collect();

    let scores_summary: Vec<String> =
        serde_json::from_str::<serde_json::Value>(&snapshot.category_scores)
            .ok()
            .and_then(|v| {
                v.as_object().map(|obj| {
                    obj.iter()
                        .filter_map(|(cat, val)| {
                            let score = val.get("score")?.as_i64()?;
                            let max = val.get("maxScore")?.as_i64().unwrap_or(100);
                            Some(format!("- {}: {}/{}", cat, score, max))
                        })
                        .collect::<Vec<_>>()
                })
            })
            .unwrap_or_default();

    let pack = ContextPack::new("fix_plan", 8000)
        .with_system_prompt(
            "You are an expert software engineer reviewing repository health findings. \
             Analyze the findings and generate a prioritized fix plan. \
             For each finding, suggest concrete steps to resolve it. \
             Order by severity (critical > error > warning > info). \
             Format as a clear numbered list with action items.",
        )
        .add_section(
            "Repository Health Score",
            &format!("repo:{}", repo_id),
            &format!(
                "Overall score: {}/100\n\nCategory breakdown:\n{}",
                snapshot.score,
                scores_summary.join("\n")
            ),
        )
        .add_section(
            "Findings Summary",
            &format!("snapshot:{}", snapshot_id),
            &findings_summary.join("\n"),
        )
        .add_section(
            "Findings Detail",
            &format!("snapshot:{}", snapshot_id),
            &findings_detail.join("\n\n"),
        );

    let raw_prompt = pack.build();

    // 4. Scan for secrets before redaction
    let secrets_before = ai_provider::scan_for_secrets(&raw_prompt);

    // 5. Redact secrets
    let redacted_prompt = ai_provider::redact_secrets(&raw_prompt);

    // 6. Re-scan after redaction
    let secrets_after = ai_provider::scan_for_secrets(&redacted_prompt);

    // 7. Build preview
    let sections: Vec<ContextPreviewSection> = pack
        .sections
        .iter()
        .map(|s| ContextPreviewSection {
            label: s.label.clone(),
            source: s.source.clone(),
            tokens_estimate: s.tokens_estimate,
            content_preview: ai_provider::redact_secrets(&s.content)
                .chars()
                .take(500)
                .collect(),
        })
        .collect();
    let total_tokens_estimate: usize = sections.iter().map(|s| s.tokens_estimate).sum();

    Ok(ContextPreview {
        purpose: pack.purpose,
        sections,
        total_tokens_estimate,
        max_tokens: pack.max_tokens,
        prompt_preview: redacted_prompt.chars().take(2000).collect(),
        secrets_found: secrets_before,
        secret_count_after_redaction: secrets_after.len(),
    })
}

/// Generate a fix plan from audit findings using AI.
/// Builds a ContextPack from findings, redacts secrets, calls AI, and stores the result as an artifact.
pub async fn generate_fix_plan(
    repo_id: &str,
    snapshot_id: &str,
    provider_id: &str,
    model: Option<&str>,
    db: &Db,
) -> Result<FixPlan, String> {
    use crate::ai_provider::{self, ContextPack};
    use crate::auditor;

    // 1. Load findings
    let findings = auditor::load_findings(snapshot_id, db)?;
    if findings.is_empty() {
        return Err("No findings to analyze. Run an audit first.".into());
    }

    // 2. Load snapshot for context
    let snapshot = auditor::load_snapshot(snapshot_id, repo_id, db)?;

    // 3. Build context pack from findings
    let findings_summary: Vec<String> = findings
        .iter()
        .map(|f| {
            format!(
                "- [{}] {}: {} (severity: {})",
                f.category, f.title, f.description, f.severity
            )
        })
        .collect();

    let findings_detail: Vec<String> = findings
        .iter()
        .map(|f| {
            let mut s = format!(
                "## Finding: {}\nCategory: {}\nSeverity: {}",
                f.title, f.category, f.severity
            );
            if !f.description.is_empty() {
                s.push_str(&format!("\nDescription: {}", f.description));
            }
            if !f.evidence.is_empty() {
                s.push_str(&format!("\nEvidence: {}", f.evidence));
            }
            if let Some(ref fp) = f.file_path {
                s.push_str(&format!("\nFile: {}", fp));
            }
            if let Some(ref fix) = f.suggested_fix {
                s.push_str(&format!("\nSuggested fix: {}", fix));
            }
            s
        })
        .collect();

    let scores_summary: Vec<String> =
        serde_json::from_str::<serde_json::Value>(&snapshot.category_scores)
            .ok()
            .and_then(|v| {
                v.as_object().map(|obj| {
                    obj.iter()
                        .filter_map(|(cat, val)| {
                            let score = val.get("score")?.as_i64()?;
                            let max = val.get("maxScore")?.as_i64().unwrap_or(100);
                            Some(format!("- {}: {}/{}", cat, score, max))
                        })
                        .collect::<Vec<_>>()
                })
            })
            .unwrap_or_default();

    let pack = ContextPack::new("fix_plan", 8000)
        .with_system_prompt(
            "You are an expert software engineer reviewing repository health findings. \
             Analyze the findings and generate a prioritized fix plan. \
             For each finding, suggest concrete steps to resolve it. \
             Order by severity (critical > error > warning > info). \
             Format as a clear numbered list with action items.",
        )
        .add_section(
            "Repository Health Score",
            &format!("repo:{}", repo_id),
            &format!(
                "Overall score: {}/100\n\nCategory breakdown:\n{}",
                snapshot.score,
                scores_summary.join("\n")
            ),
        )
        .add_section(
            "Findings Summary",
            &format!("snapshot:{}", snapshot_id),
            &findings_summary.join("\n"),
        )
        .add_section(
            "Findings Detail",
            &format!("snapshot:{}", snapshot_id),
            &findings_detail.join("\n\n"),
        );

    let prompt = pack.build();

    // 4. Redact secrets from prompt
    let prompt = ai_provider::redact_secrets(&prompt);

    // 5. Re-scan for secrets after redaction (paranoid check)
    let secrets = ai_provider::scan_for_secrets(&prompt);
    if !secrets.is_empty() {
        return Err(format!(
            "Context still contains {} potential secret(s) after redaction. Review the repository files for sensitive content.",
            secrets.len()
        ));
    }

    // 6. Get provider
    let providers = ai_provider::list_providers(db)?;
    let provider = providers
        .into_iter()
        .find(|p| p.id == provider_id)
        .ok_or_else(|| format!("Provider not found: {}", provider_id))?;

    if !provider.enabled {
        return Err(format!(
            "Provider '{}' is disabled. Enable it in Settings first.",
            provider.name
        ));
    }

    // 7. Track the external call before creating dependent artifacts.
    let job_id = crate::job_engine::create_job(
        "ai_fix_plan",
        &serde_json::json!({
            "repoId": repo_id,
            "snapshotId": snapshot_id,
            "providerId": provider_id,
        })
        .to_string(),
        db,
    )?;
    crate::job_engine::append_job_event(
        &job_id,
        "ai_fix_plan_started",
        &serde_json::json!({"repoId": repo_id, "snapshotId": snapshot_id}).to_string(),
        db,
    )?;

    let outcome = async {
        let response = ai_provider::call_ai(&provider, &prompt, model)
            .await
            .map_err(|e| format!("AI call failed: {}", e))?;
        if response.content.trim().is_empty() {
            return Err("AI returned an empty response. The model may not support this query format or may be misconfigured.".into());
        }
        if !ai_provider::scan_for_secrets(&response.content).is_empty() {
            return Err("AI response contains potential secret material and was not stored".into());
        }

        let now = chrono::Utc::now().to_rfc3339();
        let plan_id = uuid::Uuid::new_v4().to_string();
        let context_summary = format!(
            "Analyzed {} findings from health snapshot (score: {}/100). Provider: {}, Model: {}.",
            findings.len(),
            snapshot.score,
            provider.name,
            response.model
        );
        let artifact = Artifact {
            id: plan_id.clone(),
            job_id: job_id.clone(),
            artifact_type: "ai_plan".into(),
            title: format!("Fix Plan for repo {}", repo_id),
            content: response.content.clone(),
            file_path: None,
            metadata: serde_json::json!({
                "planId": plan_id,
                "repoId": repo_id,
                "snapshotId": snapshot_id,
                "providerId": provider_id,
                "model": response.model,
                "tokensIn": response.tokens_in,
                "tokensOut": response.tokens_out,
                "findingsCount": findings.len(),
            }),
        };
        create_artifact(&artifact, db)?;
        write_audit_log(
            db,
            "fix_plan_generated",
            &format!("repo:{}", repo_id),
            "ai.read",
            "low",
            &serde_json::json!({
                "planId": plan_id,
                "findingsCount": findings.len(),
            })
            .to_string(),
        )?;

        Ok::<FixPlan, String>(FixPlan {
            id: plan_id,
            job_id: job_id.clone(),
            repo_id: repo_id.into(),
            snapshot_id: snapshot_id.into(),
            provider_id: provider_id.into(),
            model: response.model,
            plan_content: response.content,
            context_summary,
            tokens_in: response.tokens_in,
            tokens_out: response.tokens_out,
            created_at: now,
        })
    }
    .await;

    match outcome {
        Ok(plan) => {
            crate::job_engine::complete_job(&job_id, db)?;
            crate::job_engine::append_job_event(
                &job_id,
                "ai_fix_plan_completed",
                &serde_json::json!({"planId": plan.id}).to_string(),
                db,
            )?;
            Ok(plan)
        }
        Err(error) => {
            crate::job_engine::fail_job(&job_id, &error, db)?;
            Err(error)
        }
    }
}

/// Propose a unified diff patch from an AI fix plan.
/// Validates the patch format before saving.
pub async fn propose_fix(
    repo_id: &str,
    provider_id: &str,
    model: Option<&str>,
    fix_instruction: &str,
    target_file: Option<&str>,
    db: &Db,
) -> Result<PatchProposal, String> {
    use crate::ai_provider::{self, ContextPack};

    // 1. Get repo worktree path
    let repo_path = get_repo_worktree_path(repo_id, db)?;

    // 2. Build context pack for diff generation
    let mut pack = ContextPack::new("propose_fix", 8000)
        .with_system_prompt(
            "You are an expert software engineer generating a unified diff patch. \
             Output ONLY a valid unified diff format patch. Do not include explanations outside the diff. \
             The diff must start with --- and +++ lines and include @@ hunks. \
             Make minimal, targeted changes to address the specific issue."
        )
        .add_section("Fix Instruction",
            "user",
            fix_instruction
        );

    // 3. If target file specified, include its content as context
    if let Some(tf) = target_file {
        let full_path = resolve_existing_repo_file(&repo_path, tf)?;
        let content = std::fs::read_to_string(&full_path)
            .map_err(|e| format!("Cannot read target file '{}': {}", tf, e))?;
        let content = ai_provider::redact_secrets(&content);
        let secrets = ai_provider::scan_for_secrets(&content);
        if !secrets.is_empty() {
            return Err(format!(
                "Target file '{}' contains {} potential secret(s). Fix the file manually or redact sensitive content first.",
                tf, secrets.len()
            ));
        }
        pack = pack.add_section("Current File Content", tf, &content);
    }

    let prompt = pack.build();
    let prompt = ai_provider::redact_secrets(&prompt);

    // Re-scan for secrets
    let secrets = ai_provider::scan_for_secrets(&prompt);
    if !secrets.is_empty() {
        return Err(format!(
            "Prompt contains {} potential secret(s) after redaction. Cannot send to AI.",
            secrets.len()
        ));
    }

    // 4. Get provider
    let providers = ai_provider::list_providers(db)?;
    let provider = providers
        .into_iter()
        .find(|p| p.id == provider_id)
        .ok_or_else(|| format!("Provider not found: {}", provider_id))?;

    if !provider.enabled {
        return Err(format!("Provider '{}' is disabled.", provider.name));
    }

    // 5. Track the external call before creating dependent records.
    let job_id = crate::job_engine::create_job(
        "ai_propose_fix",
        &serde_json::json!({
            "repoId": repo_id,
            "providerId": provider_id,
            "targetFile": target_file,
        })
        .to_string(),
        db,
    )?;
    crate::job_engine::append_job_event(
        &job_id,
        "ai_propose_fix_started",
        &serde_json::json!({"repoId": repo_id, "targetFile": target_file}).to_string(),
        db,
    )?;

    let outcome = async {
        let response = ai_provider::call_ai(&provider, &prompt, model)
            .await
            .map_err(|e| format!("AI call failed: {}", e))?;
        if response.content.trim().is_empty() {
            return Err("AI returned an empty response. Cannot create a patch proposal.".into());
        }

        let patch_content = response.content.trim().to_string();
        if !ai_provider::scan_for_secrets(&patch_content).is_empty() {
            return Err("AI patch contains potential secret material and was not stored".into());
        }
        let file_path = validate_patch_paths(&patch_content, target_file)?;
        let redacted_instruction = ai_provider::redact_secrets(fix_instruction);
        let artifact = Artifact {
            id: uuid::Uuid::new_v4().to_string(),
            job_id: job_id.clone(),
            artifact_type: "patch_proposal".into(),
            title: format!(
                "Patch proposal: {}",
                fix_instruction.chars().take(80).collect::<String>()
            ),
            content: patch_content.clone(),
            file_path: Some(file_path.clone()),
            metadata: serde_json::json!({
                "repoId": repo_id,
                "providerId": provider_id,
                "model": response.model,
                "tokensIn": response.tokens_in,
                "tokensOut": response.tokens_out,
                "instruction": redacted_instruction,
            }),
        };
        create_artifact(&artifact, db)?;

        let proposal = PatchProposal {
            id: uuid::Uuid::new_v4().to_string(),
            job_id: job_id.clone(),
            artifact_id: Some(artifact.id),
            repo_id: repo_id.into(),
            file_path: file_path.clone(),
            patch_content,
            description: redacted_instruction.chars().take(200).collect(),
            status: "proposed".into(),
            applied_at: None,
            rolled_back_at: None,
            verification_result: None,
        };
        create_patch_proposal(&proposal, db)?;
        write_audit_log(
            db,
            "patch_proposed",
            &format!("repo:{}", repo_id),
            "ai.read",
            "medium",
            &serde_json::json!({
                "proposalId": proposal.id,
                "filePath": file_path,
            })
            .to_string(),
        )?;
        Ok::<PatchProposal, String>(proposal)
    }
    .await;

    match outcome {
        Ok(proposal) => {
            crate::job_engine::complete_job(&job_id, db)?;
            crate::job_engine::append_job_event(
                &job_id,
                "ai_propose_fix_completed",
                &serde_json::json!({"proposalId": proposal.id}).to_string(),
                db,
            )?;
            Ok(proposal)
        }
        Err(error) => {
            crate::job_engine::fail_job(&job_id, &error, db)?;
            Err(error)
        }
    }
}

/// List fix plans for a repo (stored as artifacts of type ai_plan).
pub fn list_fix_plans(repo_id: &str, db: &Db) -> Result<Vec<Artifact>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT id, job_id, type, title, content, file_path, metadata FROM artifact WHERE type = 'ai_plan' AND json_extract(metadata, '$.repoId') = ?1 ORDER BY rowid DESC")
        .map_err(|e| e.to_string())?;

    let artifacts = stmt
        .query_map(rusqlite::params![repo_id], |row| {
            let metadata_str: String = row.get(6)?;
            Ok(Artifact {
                id: row.get(0)?,
                job_id: row.get(1)?,
                artifact_type: row.get(2)?,
                title: row.get(3)?,
                content: row.get(4)?,
                file_path: row.get(5)?,
                metadata: serde_json::from_str(&metadata_str).unwrap_or(serde_json::Value::Null),
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(artifacts)
}

fn write_audit_log(
    db: &Db,
    action: &str,
    subject: &str,
    capability: &str,
    risk_level: &str,
    detail: &str,
) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO audit_log (id, action, subject, scope, capability, risk_level, detail) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        rusqlite::params![
            uuid::Uuid::new_v4().to_string(),
            action,
            subject,
            subject,
            capability,
            risk_level,
            detail,
        ],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_single_file_unified_diff() {
        let patch = "--- a/src/main.rs\n+++ b/src/main.rs\n@@ -1 +1 @@\n-old\n+new\n";
        assert_eq!(
            validate_patch_paths(patch, Some("src/main.rs")).unwrap(),
            "src/main.rs"
        );
    }

    #[test]
    fn rejects_patch_path_traversal_and_multi_file_patches() {
        let traversal = "--- a/../secret.txt\n+++ b/../secret.txt\n@@ -1 +1 @@\n-a\n+b\n";
        assert!(validate_patch_paths(traversal, None).is_err());

        let multi = "--- a/a.txt\n+++ b/a.txt\n@@ -1 +1 @@\n-a\n+b\n--- a/b.txt\n+++ b/b.txt\n@@ -1 +1 @@\n-a\n+b\n";
        assert!(validate_patch_paths(multi, None).is_err());
    }

    #[test]
    fn rejects_patch_that_does_not_match_requested_file() {
        let patch = "--- a/src/main.rs\n+++ b/src/main.rs\n@@ -1 +1 @@\n-old\n+new\n";
        assert!(validate_patch_paths(patch, Some("src/lib.rs")).is_err());
    }
}
