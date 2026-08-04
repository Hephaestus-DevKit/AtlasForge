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
#[cfg(test)]
pub fn apply_patch(proposal_id: &str, db: &Db) -> Result<PatchProposal, String> {
    let cancellation = std::sync::atomic::AtomicBool::new(false);
    apply_patch_cancellable(proposal_id, db, &cancellation)
}

/// Apply a patch while honoring cancellation until the user worktree mutation begins.
pub fn apply_patch_cancellable(
    proposal_id: &str,
    db: &Db,
    cancellation: &std::sync::atomic::AtomicBool,
) -> Result<PatchProposal, String> {
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

    let repo_path = get_repo_worktree_path(&proposal.repo_id, db)?;
    ensure_repo_write_allowed(&proposal.repo_id, db)?;
    let patch_content = clean_patch_content(&proposal.patch_content);
    let patch_path = validate_patch_paths(&patch_content, None)?;
    if patch_path != proposal.file_path.replace('\\', "/") {
        return Err("Patch metadata does not match the patch target file".into());
    }
    let target_path = resolve_existing_repo_file(&repo_path, &patch_path)?;

    let base_head = run_git_output(&repo_path, &["rev-parse", "HEAD"])?;
    let status = run_git_output(&repo_path, &["status", "--porcelain"])?;
    if !status.is_empty() {
        return Err(
            "Patch application requires a clean working tree. Commit, stash, or move existing changes first."
                .into(),
        );
    }
    let base_file_hash = crate::permissions::hash_text_file(&target_path)?;
    let backup_content = std::fs::read_to_string(&target_path)
        .map_err(|error| format!("Cannot create patch backup: {error}"))?;
    let approval_context_hash = crate::permissions::hash_text(&format!(
        "{}\n{}\n{}\n{}",
        repo_path,
        base_head,
        status,
        crate::permissions::hash_text(&patch_content)
    ));

    let sandbox = std::env::temp_dir().join(format!("atlasforge-patch-{}", uuid::Uuid::new_v4()));
    let sandbox_path = sandbox.to_string_lossy().to_string();
    run_git(
        &repo_path,
        &["worktree", "add", "--detach", &sandbox_path, &base_head],
    )
    .map_err(|error| format!("Cannot create isolated patch worktree: {}", error))?;

    let isolated_result = (|| -> Result<(Option<String>, String), String> {
        link_dependency_cache(&repo_path, &sandbox_path)?;
        run_git_apply(&sandbox_path, &patch_content, &["--check"])?;
        run_git_apply(&sandbox_path, &patch_content, &[])?;
        let verification = run_post_apply_verification(&proposal.repo_id, &sandbox_path, db);
        if verification
            .as_deref()
            .and_then(|value| serde_json::from_str::<serde_json::Value>(value).ok())
            .and_then(|value| value.get("allPassed").and_then(|passed| passed.as_bool()))
            == Some(false)
        {
            return Err("Patch verification failed in the isolated worktree; the user worktree was not modified".into());
        }
        let sandbox_target = resolve_existing_repo_file(&sandbox_path, &patch_path)?;
        let expected_applied_hash = crate::permissions::hash_text_file(&sandbox_target)?;
        Ok((verification, expected_applied_hash))
    })();
    let cleanup_result = run_git(
        &repo_path,
        &["worktree", "remove", "--force", &sandbox_path],
    );
    let _ = std::fs::remove_dir_all(&sandbox);
    if let Err(error) = cleanup_result {
        log::warn!(
            "Failed to remove patch worktree '{}': {}",
            sandbox_path,
            error
        );
    }
    let (verification_json, expected_applied_hash) = isolated_result?;

    if cancellation.load(std::sync::atomic::Ordering::SeqCst) {
        return Err("Patch application cancelled before the user worktree was modified".into());
    }

    let current_head = run_git_output(&repo_path, &["rev-parse", "HEAD"])?;
    let current_status = run_git_output(&repo_path, &["status", "--porcelain"])?;
    let current_file_hash = crate::permissions::hash_text_file(&target_path)?;
    if current_head != base_head
        || !current_status.is_empty()
        || current_file_hash != base_file_hash
    {
        return Err(
            "Repository changed after approval. Request a new approval against the current baseline."
                .into(),
        );
    }

    // Persist recovery metadata and the mutation intent before touching the user
    // worktree. Startup recovery can now distinguish not-applied, applied, and
    // drifted states after a crash.
    {
        let mut conn = db.conn.lock().map_err(|e| e.to_string())?;
        let tx = conn.transaction().map_err(|e| e.to_string())?;
        tx.execute(
            "UPDATE patch_proposal
             SET verification_result = ?1, base_head_sha = ?2, base_file_hash = ?3,
                 applied_file_hash = ?4, backup_content = ?5, approval_context_hash = ?6
             WHERE id = ?7 AND status IN ('proposed', 'approved')",
            rusqlite::params![
                verification_json,
                base_head,
                base_file_hash,
                expected_applied_hash,
                backup_content,
                approval_context_hash,
                proposal_id,
            ],
        )
        .map_err(|e| e.to_string())?;
        tx.execute(
            "INSERT INTO audit_log (id, action, subject, scope, capability, risk_level, detail)
             VALUES (?1, 'patch_apply_prepared', ?2, ?3, 'fs.write_patch', 'high', ?4)",
            rusqlite::params![
                uuid::Uuid::new_v4().to_string(),
                format!("repo:{}", proposal.repo_id),
                proposal.file_path,
                serde_json::json!({"proposalId": proposal_id, "baseHead": base_head}).to_string(),
            ],
        )
        .map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())?;
    }

    if cancellation.load(std::sync::atomic::Ordering::SeqCst) {
        return Err("Patch application cancelled before the user worktree was modified".into());
    }
    run_git_apply(&repo_path, &patch_content, &["--check"])
        .map_err(|error| format!("Patch no longer applies cleanly: {}", error))?;
    run_git_apply(&repo_path, &patch_content, &[])
        .map_err(|error| format!("Patch apply failed: {}", error))?;
    let applied_file_hash = crate::permissions::hash_text_file(&target_path)?;
    if applied_file_hash != expected_applied_hash {
        let _ = run_git_apply(&repo_path, &patch_content, &["-R"]);
        return Err("Applied patch hash did not match the isolated verification result; a compensating rollback was attempted".into());
    }
    let now = chrono::Utc::now().to_rfc3339();

    let mut conn = db.conn.lock().map_err(|e| e.to_string())?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    let changed = tx
        .execute(
            "UPDATE patch_proposal
         SET status = 'applied', applied_at = ?1
         WHERE id = ?2 AND status IN ('proposed', 'approved') AND applied_file_hash = ?3",
            rusqlite::params![now, proposal_id, applied_file_hash,],
        )
        .map_err(|e| e.to_string())?;
    if changed != 1 {
        drop(tx);
        drop(conn);
        let compensated = run_git_apply(&repo_path, &patch_content, &["-R"]).is_ok()
            && crate::permissions::hash_text_file(&target_path)
                .ok()
                .as_deref()
                == Some(base_file_hash.as_str());
        return Err(format!(
            "Patch state could not be finalized after the file changed; compensating rollback {}",
            if compensated {
                "succeeded"
            } else {
                "failed and startup recovery is required"
            }
        ));
    }
    tx.execute(
        "INSERT INTO audit_log (id, action, subject, scope, capability, risk_level, detail)
         VALUES (?1, 'patch_applied', ?2, ?3, 'fs.write_patch', 'high', ?4)",
        rusqlite::params![
            uuid::Uuid::new_v4().to_string(),
            format!("repo:{}", proposal.repo_id),
            proposal.file_path,
            serde_json::json!({
            "proposalId": proposal_id,
            "filePath": proposal.file_path,
            "baseHead": base_head,
            "baseFileHash": base_file_hash,
            "appliedFileHash": applied_file_hash,
            "isolatedVerification": verification_json.is_some(),
            })
            .to_string(),
        ],
    )
    .map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;

    let mut applied = proposal;
    applied.status = "applied".into();
    applied.applied_at = Some(now);
    applied.verification_result = verification_json;
    Ok(applied)
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
#[cfg(test)]
pub fn rollback_patch(proposal_id: &str, db: &Db) -> Result<(), String> {
    let cancellation = std::sync::atomic::AtomicBool::new(false);
    rollback_patch_cancellable(proposal_id, db, &cancellation)
}

/// Roll back a patch while honoring cancellation until restoration begins.
pub fn rollback_patch_cancellable(
    proposal_id: &str,
    db: &Db,
    cancellation: &std::sync::atomic::AtomicBool,
) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let (repo_id, file_path, patch_content, base_file_hash, applied_file_hash, backup_content): (
        String,
        String,
        String,
        Option<String>,
        Option<String>,
        Option<String>,
    ) = conn
        .query_row(
            "SELECT repo_id, file_path, patch_content, base_file_hash, applied_file_hash, backup_content
         FROM patch_proposal WHERE id = ?1 AND status = 'applied'",
            rusqlite::params![proposal_id],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                ))
            },
        )
        .map_err(|e| format!("Cannot rollback: {}", e))?;
    drop(conn);

    let patch_content = clean_patch_content(&patch_content);

    let repo_path = get_repo_worktree_path(&repo_id, db)?;
    ensure_repo_write_allowed(&repo_id, db)?;
    let expected_applied_hash = applied_file_hash
        .ok_or_else(|| "Applied file hash is missing; automatic rollback is unsafe".to_string())?;
    let expected_base_hash = base_file_hash
        .ok_or_else(|| "Base file hash is missing; automatic rollback is unsafe".to_string())?;
    let target_path = resolve_existing_repo_file(&repo_path, &file_path)?;
    let current_hash = crate::permissions::hash_text_file(&target_path)?;
    if current_hash != expected_applied_hash {
        return Err(
            "The patched file changed after application. Automatic rollback was refused to preserve newer work."
                .into(),
        );
    }
    validate_patch_paths(&patch_content, Some(&file_path))?;
    write_audit_log(
        db,
        "patch_rollback_prepared",
        &format!("repo:{}", repo_id),
        "fs.write_patch",
        "high",
        &serde_json::json!({"proposalId": proposal_id, "filePath": file_path}).to_string(),
    )?;
    if cancellation.load(std::sync::atomic::Ordering::SeqCst) {
        return Err("Patch rollback cancelled before the user worktree was modified".into());
    }
    run_git_apply(&repo_path, &patch_content, &["--check", "-R"])
        .map_err(|error| format!("Rollback validation failed: {}", error))?;
    if let Err(reverse_error) = run_git_apply(&repo_path, &patch_content, &["-R"]) {
        let backup = backup_content.as_deref().ok_or_else(|| {
            format!("Rollback failed and no backup is available: {reverse_error}")
        })?;
        std::fs::write(&target_path, backup).map_err(|error| {
            format!("Rollback and backup restoration failed: {reverse_error}; {error}")
        })?;
    }
    let restored_hash = crate::permissions::hash_text_file(&target_path)?;
    if restored_hash != expected_base_hash {
        return Err("Rollback integrity check failed after restoring the backup".into());
    }

    let now = chrono::Utc::now().to_rfc3339();
    let mut conn = db.conn.lock().map_err(|e| e.to_string())?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    let changed = tx.execute(
        "UPDATE patch_proposal SET status = 'rolled_back', rolled_back_at = ?1 WHERE id = ?2 AND status = 'applied'",
        rusqlite::params![now, proposal_id],
    )
    .map_err(|e| e.to_string())?;
    if changed != 1 {
        return Err("Rollback restored the file but its proposal state could not be finalized; startup recovery will reconcile it".into());
    }
    tx.execute(
        "INSERT INTO audit_log (id, action, subject, scope, capability, risk_level, detail)
         VALUES (?1, 'patch_rolled_back', ?2, ?3, 'fs.write_patch', 'high', ?4)",
        rusqlite::params![
            uuid::Uuid::new_v4().to_string(),
            format!("repo:{}", repo_id),
            file_path,
            serde_json::json!({
            "proposalId": proposal_id,
            "filePath": file_path,
            "restoredHash": restored_hash,
            })
            .to_string(),
        ],
    )
    .map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())
}

/// Reconcile patch operations interrupted between filesystem mutation and the
/// final database transaction. Recovery only changes state when the current
/// file hash exactly matches a stored baseline or isolated applied hash.
pub fn recover_interrupted_patch_operations(db: &Db) -> Result<usize, String> {
    type RecoveryCandidate = (
        String,
        String,
        String,
        String,
        Option<String>,
        Option<String>,
    );
    let candidates: Vec<RecoveryCandidate> = {
        let conn = db.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare(
                "SELECT p.id, p.status, r.worktree_path, p.file_path,
                        p.base_file_hash, p.applied_file_hash
                 FROM patch_proposal p
                 JOIN repository r ON r.id = p.repo_id
                 WHERE (p.status IN ('proposed', 'approved') AND p.applied_file_hash IS NOT NULL)
                    OR p.status = 'applied'",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                ))
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?
    };

    let mut recovered = 0;
    for (id, status, repo_path, file_path, base_hash, applied_hash) in candidates {
        let target = match resolve_existing_repo_file(&repo_path, &file_path) {
            Ok(target) => target,
            Err(_) => continue,
        };
        let current_hash = match crate::permissions::hash_text_file(&target) {
            Ok(hash) => hash,
            Err(_) => continue,
        };
        let next_status = if matches!(status.as_str(), "proposed" | "approved") {
            if applied_hash.as_deref() == Some(current_hash.as_str()) {
                Some("applied")
            } else if base_hash.as_deref() == Some(current_hash.as_str()) {
                None
            } else {
                Some("conflict")
            }
        } else if status == "applied" && base_hash.as_deref() == Some(current_hash.as_str()) {
            Some("rolled_back")
        } else {
            None
        };
        let Some(next_status) = next_status else {
            continue;
        };
        let conn = db.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "UPDATE patch_proposal
             SET status = ?1,
                 applied_at = CASE WHEN ?1 = 'applied' THEN COALESCE(applied_at, datetime('now')) ELSE applied_at END,
                 rolled_back_at = CASE WHEN ?1 = 'rolled_back' THEN COALESCE(rolled_back_at, datetime('now')) ELSE rolled_back_at END
             WHERE id = ?2 AND status = ?3",
            rusqlite::params![next_status, id, status],
        )
        .map_err(|e| e.to_string())?;
        recovered += 1;
    }
    Ok(recovered)
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
    let mut git_args = vec!["apply"];
    git_args.extend_from_slice(args);
    let output = crate::process_runner::run_with_input(
        "git",
        &git_args,
        Some(Path::new(repo_path)),
        Some(patch_content.as_bytes()),
        crate::process_runner::DEFAULT_TIMEOUT,
        crate::process_runner::DEFAULT_OUTPUT_LIMIT,
    )
    .map_err(|e| format!("Failed to run git apply: {}", e))?;
    if !output.success {
        return Err(format!("git apply failed: {}", output.stderr));
    }

    Ok(())
}

fn run_git(repo_path: &str, args: &[&str]) -> Result<(), String> {
    let output = crate::process_runner::run_default("git", args, Some(Path::new(repo_path)))
        .map_err(|error| format!("Failed to run git: {}", error))?;
    if !output.success {
        return Err(output.stderr.trim().to_string());
    }
    Ok(())
}

fn run_git_output(repo_path: &str, args: &[&str]) -> Result<String, String> {
    let output = crate::process_runner::run_default("git", args, Some(Path::new(repo_path)))
        .map_err(|error| format!("Failed to run git: {}", error))?;
    if !output.success {
        return Err(output.stderr.trim().to_string());
    }
    Ok(output.stdout.trim().to_string())
}

fn link_dependency_cache(repo_path: &str, sandbox_path: &str) -> Result<(), String> {
    let source = Path::new(repo_path).join("node_modules");
    let destination = Path::new(sandbox_path).join("node_modules");
    if !source.is_dir() || destination.exists() {
        return Ok(());
    }
    #[cfg(windows)]
    {
        let status = std::process::Command::new("cmd")
            .args([
                "/C",
                "mklink",
                "/J",
                &destination.to_string_lossy(),
                &source.to_string_lossy(),
            ])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped())
            .status()
            .map_err(|error| format!("Cannot link dependency cache: {}", error))?;
        if !status.success() {
            return Err("Cannot create isolated node_modules junction".into());
        }
    }
    #[cfg(not(windows))]
    std::os::unix::fs::symlink(&source, &destination)
        .map_err(|error| format!("Cannot link dependency cache: {}", error))?;
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

fn clean_patch_content(content: &str) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let mut start_idx = None;
    for (i, line) in lines.iter().enumerate() {
        if line.starts_with("diff --git") || line.starts_with("--- ") || line.starts_with("+++ ") {
            start_idx = Some(i);
            break;
        }
    }

    let cleaned_lines = if let Some(idx) = start_idx {
        lines[idx..].to_vec()
    } else {
        lines
    };

    let mut final_lines = Vec::new();
    for line in cleaned_lines {
        let trimmed = line.trim();
        if trimmed == "```" || trimmed.starts_with("```") {
            break;
        }
        final_lines.push(line);
    }

    let mut joined = final_lines.join("\n");
    if !joined.ends_with('\n') && !joined.is_empty() {
        joined.push('\n');
    }
    joined
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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct FixPlanDocument {
    summary: String,
    items: Vec<FixPlanItem>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct FixPlanItem {
    priority: String,
    title: String,
    rationale: String,
    steps: Vec<String>,
    verification: Vec<String>,
    affected_files: Vec<String>,
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
             Return JSON only with this shape: \
             {\"summary\":\"...\",\"items\":[{\"priority\":\"critical|high|medium|low\",\
             \"title\":\"...\",\"rationale\":\"...\",\"steps\":[\"...\"],\
             \"verification\":[\"...\"],\"affectedFiles\":[\"...\"]}]}. \
             Order items by severity and include only concrete, verifiable work.",
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
    runtime: &crate::job_engine::JobRuntime,
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
             Return JSON only with this shape: \
             {\"summary\":\"...\",\"items\":[{\"priority\":\"critical|high|medium|low\",\
             \"title\":\"...\",\"rationale\":\"...\",\"steps\":[\"...\"],\
             \"verification\":[\"...\"],\"affectedFiles\":[\"...\"]}]}. \
             Order items by severity and include only concrete, verifiable work.",
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

    let system_prompt = pack.system_prompt.clone();
    let prompt = pack.build_body();

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
    let cancellation = crate::job_engine::begin_job(&job_id, db, runtime)?;

    let outcome = async {
        let response = ai_provider::call_ai(&provider, &prompt, system_prompt.as_deref(), model)
            .await
            .map_err(|e| format!("AI call failed: {}", e))?;
        if response.content.trim().is_empty() {
            return Err("AI returned an empty response. The model may not support this query format or may be misconfigured.".into());
        }
        if !ai_provider::scan_for_secrets(&response.content).is_empty() {
            return Err("AI response contains potential secret material and was not stored".into());
        }
        if cancellation.load(std::sync::atomic::Ordering::SeqCst) {
            return Err("AI fix plan cancelled by user".into());
        }
        let plan_document = parse_fix_plan_document(&response.content)?;
        let plan_content = serde_json::to_string_pretty(&plan_document)
            .map_err(|error| format!("Cannot serialize validated fix plan: {}", error))?;

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
            content: plan_content.clone(),
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
                "itemCount": plan_document.items.len(),
                "schemaVersion": 1,
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
            plan_content,
            context_summary,
            tokens_in: response.tokens_in,
            tokens_out: response.tokens_out,
            created_at: now,
        })
    }
    .await;

    match outcome {
        Ok(plan) => {
            if cancellation.load(std::sync::atomic::Ordering::SeqCst) {
                runtime.finish(&job_id);
                return Err("AI fix plan cancelled by user".into());
            }
            crate::job_engine::complete_job(&job_id, db)?;
            runtime.finish(&job_id);
            crate::job_engine::append_job_event(
                &job_id,
                "ai_fix_plan_completed",
                &serde_json::json!({"planId": plan.id}).to_string(),
                db,
            )?;
            Ok(plan)
        }
        Err(error) => {
            if cancellation.load(std::sync::atomic::Ordering::SeqCst) {
                runtime.finish(&job_id);
                return Err("AI fix plan cancelled by user".into());
            }
            crate::job_engine::fail_job(&job_id, &error, db)?;
            runtime.finish(&job_id);
            Err(error)
        }
    }
}

fn parse_fix_plan_document(content: &str) -> Result<FixPlanDocument, String> {
    let trimmed = content.trim();
    let json = if trimmed.starts_with("```") {
        let without_opening = trimmed
            .split_once('\n')
            .map(|(_, rest)| rest)
            .ok_or_else(|| "AI fix plan code fence is incomplete".to_string())?;
        without_opening
            .strip_suffix("```")
            .map(str::trim)
            .ok_or_else(|| "AI fix plan code fence is incomplete".to_string())?
    } else {
        trimmed
    };
    let document: FixPlanDocument = serde_json::from_str(json)
        .map_err(|error| format!("AI fix plan is not valid structured JSON: {}", error))?;
    if document.summary.trim().is_empty() || document.items.is_empty() {
        return Err("AI fix plan must include a summary and at least one item".into());
    }
    let valid_priorities = ["critical", "high", "medium", "low"];
    for (index, item) in document.items.iter().enumerate() {
        if !valid_priorities.contains(&item.priority.as_str()) {
            return Err(format!(
                "Fix plan item {} has an invalid priority",
                index + 1
            ));
        }
        if item.title.trim().is_empty()
            || item.rationale.trim().is_empty()
            || item.steps.is_empty()
            || item.verification.is_empty()
            || item.steps.iter().any(|step| step.trim().is_empty())
            || item.verification.iter().any(|step| step.trim().is_empty())
        {
            return Err(format!(
                "Fix plan item {} must include title, rationale, steps, and verification",
                index + 1
            ));
        }
    }
    Ok(document)
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
    runtime: &crate::job_engine::JobRuntime,
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

    let system_prompt = pack.system_prompt.clone();
    let prompt = pack.build_body();
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
    let cancellation = crate::job_engine::begin_job(&job_id, db, runtime)?;

    let outcome = async {
        let response = ai_provider::call_ai(&provider, &prompt, system_prompt.as_deref(), model)
            .await
            .map_err(|e| format!("AI call failed: {}", e))?;
        if response.content.trim().is_empty() {
            return Err("AI returned an empty response. Cannot create a patch proposal.".into());
        }

        let patch_content = clean_patch_content(&response.content);
        if !ai_provider::scan_for_secrets(&patch_content).is_empty() {
            return Err("AI patch contains potential secret material and was not stored".into());
        }
        if cancellation.load(std::sync::atomic::Ordering::SeqCst) {
            return Err("AI fix proposal cancelled by user".into());
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
            if cancellation.load(std::sync::atomic::Ordering::SeqCst) {
                runtime.finish(&job_id);
                return Err("AI fix proposal cancelled by user".into());
            }
            crate::job_engine::complete_job(&job_id, db)?;
            runtime.finish(&job_id);
            crate::job_engine::append_job_event(
                &job_id,
                "ai_propose_fix_completed",
                &serde_json::json!({"proposalId": proposal.id}).to_string(),
                db,
            )?;
            Ok(proposal)
        }
        Err(error) => {
            if cancellation.load(std::sync::atomic::Ordering::SeqCst) {
                runtime.finish(&job_id);
                return Err("AI fix proposal cancelled by user".into());
            }
            crate::job_engine::fail_job(&job_id, &error, db)?;
            runtime.finish(&job_id);
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
    use crate::db::Db;
    use std::fs;

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

    #[test]
    fn isolated_patch_apply_and_hash_guarded_rollback() {
        let temp = tempfile::tempdir().unwrap();
        let repo = temp.path().join("repo");
        fs::create_dir_all(&repo).unwrap();
        fs::write(repo.join("note.txt"), "old\n").unwrap();
        for args in [
            vec!["init", "-b", "main"],
            vec!["config", "user.name", "AtlasForge Test"],
            vec!["config", "user.email", "atlasforge@example.invalid"],
            vec!["add", "note.txt"],
            vec!["commit", "-m", "init"],
        ] {
            let status = std::process::Command::new("git")
                .args(args)
                .current_dir(&repo)
                .status()
                .unwrap();
            assert!(status.success());
        }

        let db = Db::new(&std::path::PathBuf::from(":memory:")).unwrap();
        let job_id = crate::job_engine::create_job("patch", "{}", &db).unwrap();
        {
            let conn = db.conn.lock().unwrap();
            conn.execute(
                "INSERT INTO workspace_root (id, path, label, access_mode)
                 VALUES ('root', ?1, 'Root', 'read_write')",
                rusqlite::params![repo.to_string_lossy()],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO project_asset (id, root_id, path, name)
                 VALUES ('asset', 'root', ?1, 'Repo')",
                rusqlite::params![repo.to_string_lossy()],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO repository (id, asset_id, worktree_path, git_dir_path)
                 VALUES ('repo', 'asset', ?1, ?2)",
                rusqlite::params![repo.to_string_lossy(), repo.join(".git").to_string_lossy()],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO patch_proposal
                 (id, job_id, repo_id, file_path, patch_content, description)
                 VALUES ('patch', ?1, 'repo', 'note.txt', ?2, 'Update note')",
                rusqlite::params![
                    job_id,
                    "--- a/note.txt\n+++ b/note.txt\n@@ -1 +1 @@\n-old\n+new\n"
                ],
            )
            .unwrap();
        }

        let applied = apply_patch("patch", &db).unwrap();
        assert_eq!(applied.status, "applied");
        let applied_content = fs::read_to_string(repo.join("note.txt")).unwrap();
        assert_eq!(applied_content.replace("\r\n", "\n"), "new\n");

        fs::write(repo.join("note.txt"), "newer user work\n").unwrap();
        assert!(rollback_patch("patch", &db).is_err());
        fs::write(repo.join("note.txt"), applied_content).unwrap();
        rollback_patch("patch", &db).unwrap();
        assert_eq!(
            fs::read_to_string(repo.join("note.txt"))
                .unwrap()
                .replace("\r\n", "\n"),
            "old\n"
        );
    }

    #[test]
    fn parses_and_validates_structured_fix_plans() {
        let document = parse_fix_plan_document(
            "```json\n{\"summary\":\"Fix safety issues\",\"items\":[{\"priority\":\"high\",\"title\":\"Add guard\",\"rationale\":\"Prevents data loss\",\"steps\":[\"Add validation\"],\"verification\":[\"Run tests\"],\"affectedFiles\":[\"src/lib.rs\"]}]}\n```",
        )
        .unwrap();
        assert_eq!(document.items.len(), 1);
        assert!(parse_fix_plan_document("{\"summary\":\"\",\"items\":[]}").is_err());
    }
}
