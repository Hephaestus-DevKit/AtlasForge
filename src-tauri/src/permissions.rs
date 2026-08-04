use crate::db::Db;
use crate::verification::{self, VerificationCommand};
use sha2::{Digest, Sha256};
use std::path::Path;

const APPROVAL_TTL_MINUTES: i64 = 15;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionRequest {
    pub id: String,
    pub job_id: Option<String>,
    pub repo_id: Option<String>,
    pub capability: String,
    pub scope: String,
    pub risk_level: String,
    pub command: Option<String>,
    pub context_hash: String,
    pub details: serde_json::Value,
    pub status: String,
    pub created_at: String,
    pub expires_at: String,
    pub decided_at: Option<String>,
}

pub fn hash_text(value: &str) -> String {
    format!("{:x}", Sha256::digest(value.as_bytes()))
}

pub fn hash_file(path: &Path) -> Result<String, String> {
    let bytes = std::fs::read(path)
        .map_err(|error| format!("Cannot read '{}': {}", path.display(), error))?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

pub fn hash_text_file(path: &Path) -> Result<String, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|error| format!("Cannot read '{}': {}", path.display(), error))?;
    Ok(hash_text(&content.replace("\r\n", "\n")))
}

pub fn verification_context_hash(
    cwd: &str,
    command: &VerificationCommand,
) -> Result<String, String> {
    let root = Path::new(cwd)
        .canonicalize()
        .map_err(|error| format!("Cannot resolve verification directory: {}", error))?;
    let mut material = format!(
        "{}\n{}\n{}\n",
        root.display(),
        command.command,
        command.expanded_command
    );
    for filename in [
        "package.json",
        "package-lock.json",
        "pnpm-lock.yaml",
        "yarn.lock",
        "bun.lock",
        "bun.lockb",
        "Cargo.toml",
        "Cargo.lock",
        "build.rs",
        "go.mod",
        "go.sum",
        "pyproject.toml",
        "requirements.txt",
    ] {
        let path = root.join(filename);
        if path.is_file() {
            material.push_str(filename);
            material.push(':');
            material.push_str(&hash_file(&path)?);
            material.push('\n');
        }
    }
    Ok(hash_text(&material))
}

pub fn request_verification(
    repo_id: &str,
    cwd: &str,
    requested_command: &str,
    db: &Db,
) -> Result<PermissionRequest, String> {
    let command = verification::resolve_command(cwd, requested_command)?;
    let context_hash = verification_context_hash(cwd, &command)?;
    create_request(
        None,
        Some(repo_id),
        "shell.verify",
        cwd,
        &command.risk_level,
        Some(&command.command),
        &context_hash,
        serde_json::json!({
            "name": command.name,
            "category": command.category,
            "command": command.command,
            "expandedCommand": command.expanded_command,
            "cwd": cwd,
            "reason": command.risk_explanation,
            "timeoutSecs": command.timeout_secs,
        }),
        db,
    )
}

pub fn request_patch(proposal_id: &str, db: &Db) -> Result<PermissionRequest, String> {
    let (repo_id, repo_path, file_path, patch_content, description, head, status, context_hash) =
        patch_context(proposal_id, db)?;
    if !status.is_empty() {
        return Err(
            "Patch application requires a clean working tree. Commit, stash, or move existing changes to another worktree first."
                .into(),
        );
    }
    let verification_commands = verification::detect_commands(&repo_path)
        .into_iter()
        .filter(|command| command.category != "install")
        .map(|command| {
            serde_json::json!({
                "command": command.command,
                "expandedCommand": command.expanded_command,
                "risk": command.risk_level,
            })
        })
        .collect::<Vec<_>>();
    create_request(
        None,
        Some(&repo_id),
        "fs.write_patch",
        &repo_path,
        "high",
        None,
        &context_hash,
        serde_json::json!({
            "proposalId": proposal_id,
            "filePath": file_path,
            "description": description,
            "repoPath": repo_path,
            "headSha": head,
            "workingTreeClean": status.is_empty(),
            "patchHash": hash_text(&patch_content),
            "isolatedVerificationCommands": verification_commands,
        }),
        db,
    )
}

pub fn current_patch_context_hash(proposal_id: &str, db: &Db) -> Result<(String, String), String> {
    let (repo_id, _, _, _, _, _, _, context_hash) = patch_context(proposal_id, db)?;
    Ok((repo_id, context_hash))
}

#[allow(clippy::type_complexity)]
fn patch_context(
    proposal_id: &str,
    db: &Db,
) -> Result<
    (
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
    ),
    String,
> {
    let conn = db.conn.lock().map_err(|error| error.to_string())?;
    let (repo_id, repo_path, file_path, patch_content, description): (
        String,
        String,
        String,
        String,
        String,
    ) = conn
        .query_row(
            "SELECT p.repo_id, r.worktree_path, p.file_path, p.patch_content, p.description
             FROM patch_proposal p
             JOIN repository r ON r.id = p.repo_id
             WHERE p.id = ?1",
            rusqlite::params![proposal_id],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            },
        )
        .map_err(|error| format!("Patch proposal not found: {}", error))?;
    drop(conn);

    let head = git_output(&repo_path, &["rev-parse", "HEAD"])?;
    let status = git_output(&repo_path, &["status", "--porcelain"])?;
    let context_hash = hash_text(&format!(
        "{}\n{}\n{}\n{}",
        repo_path,
        head,
        status,
        hash_text(&patch_content)
    ));
    Ok((
        repo_id,
        repo_path,
        file_path,
        patch_content,
        description,
        head,
        status,
        context_hash,
    ))
}

#[allow(clippy::too_many_arguments)]
pub fn create_request(
    job_id: Option<&str>,
    repo_id: Option<&str>,
    capability: &str,
    scope: &str,
    risk_level: &str,
    command: Option<&str>,
    context_hash: &str,
    details: serde_json::Value,
    db: &Db,
) -> Result<PermissionRequest, String> {
    let id = uuid::Uuid::new_v4().to_string();
    let created_at = chrono::Utc::now();
    let expires_at = created_at + chrono::Duration::minutes(APPROVAL_TTL_MINUTES);
    let details_json = serde_json::to_string(&details).map_err(|error| error.to_string())?;
    let conn = db.conn.lock().map_err(|error| error.to_string())?;
    conn.execute(
        "INSERT INTO permission_request
         (id, job_id, repo_id, capability, scope, risk_level, command, context_hash, details, status, created_at, expires_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 'pending', ?10, ?11)",
        rusqlite::params![
            id,
            job_id,
            repo_id,
            capability,
            scope,
            risk_level,
            command,
            context_hash,
            details_json,
            created_at.to_rfc3339(),
            expires_at.to_rfc3339(),
        ],
    )
    .map_err(|error| error.to_string())?;
    drop(conn);
    load_request(&id, db)
}

pub fn decide_request(id: &str, approved: bool, db: &Db) -> Result<PermissionRequest, String> {
    expire_requests(db)?;
    let status = if approved { "approved" } else { "denied" };
    let conn = db.conn.lock().map_err(|error| error.to_string())?;
    let changed = conn
        .execute(
            "UPDATE permission_request
             SET status = ?1, decided_at = ?2
             WHERE id = ?3 AND status = 'pending' AND expires_at > ?2",
            rusqlite::params![status, chrono::Utc::now().to_rfc3339(), id],
        )
        .map_err(|error| error.to_string())?;
    if changed == 0 {
        return Err("Approval request is no longer pending or has expired".into());
    }
    drop(conn);
    write_audit(id, status, db)?;
    load_request(id, db)
}

pub fn consume_request(
    id: &str,
    capability: &str,
    repo_id: Option<&str>,
    expected_context_hash: &str,
    db: &Db,
) -> Result<(), String> {
    consume_requests(
        &[(id, capability, repo_id, expected_context_hash)],
        db,
    )
}

pub fn request_rollback(proposal_id: &str, db: &Db) -> Result<PermissionRequest, String> {
    let (repo_id, repo_path, file_path, head, git_status, current_hash, context_hash) =
        rollback_context(proposal_id, db)?;
    create_request(
        None,
        Some(&repo_id),
        "fs.rollback_patch",
        &repo_path,
        "high",
        None,
        &context_hash,
        serde_json::json!({
            "proposalId": proposal_id,
            "filePath": file_path,
            "repoPath": repo_path,
            "headSha": head,
            "workingTreeState": git_status,
            "currentFileHash": current_hash,
            "effect": "Restore the exact pre-patch file content",
        }),
        db,
    )
}

pub fn current_rollback_context_hash(
    proposal_id: &str,
    db: &Db,
) -> Result<(String, String), String> {
    let (repo_id, _, _, _, _, _, context_hash) = rollback_context(proposal_id, db)?;
    Ok((repo_id, context_hash))
}

#[allow(clippy::type_complexity)]
fn rollback_context(
    proposal_id: &str,
    db: &Db,
) -> Result<(String, String, String, String, String, String, String), String> {
    let conn = db.conn.lock().map_err(|error| error.to_string())?;
    let (repo_id, repo_path, file_path, patch_content, proposal_status): (
        String,
        String,
        String,
        String,
        String,
    ) = conn
        .query_row(
            "SELECT p.repo_id, r.worktree_path, p.file_path, p.patch_content, p.status
             FROM patch_proposal p JOIN repository r ON r.id = p.repo_id
             WHERE p.id = ?1",
            rusqlite::params![proposal_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
        )
        .map_err(|error| format!("Patch proposal not found: {error}"))?;
    drop(conn);
    if proposal_status != "applied" {
        return Err("Only an applied patch can be rolled back".into());
    }
    let repo_root = std::path::Path::new(&repo_path)
        .canonicalize()
        .map_err(|error| format!("Cannot resolve repository: {error}"))?;
    let target = repo_root
        .join(&file_path)
        .canonicalize()
        .map_err(|error| format!("Cannot resolve rollback target: {error}"))?;
    if !target.starts_with(&repo_root) {
        return Err("Rollback target escapes the repository".into());
    }
    let head = git_output(&repo_path, &["rev-parse", "HEAD"])?;
    let git_status = git_output(&repo_path, &["status", "--porcelain"])?;
    let current_hash = hash_text_file(&target)?;
    let context_hash = hash_text(&format!(
        "rollback\n{}\n{}\n{}\n{}\n{}",
        repo_path,
        head,
        git_status,
        current_hash,
        hash_text(&patch_content),
    ));
    Ok((repo_id, repo_path, file_path, head, git_status, current_hash, context_hash))
}

/// Validate and consume a group of approvals atomically. No request is consumed
/// when any member is stale, mismatched, expired, or already used.
pub fn consume_requests(
    requests: &[(&str, &str, Option<&str>, &str)],
    db: &Db,
) -> Result<(), String> {
    expire_requests(db)?;
    let mut conn = db.conn.lock().map_err(|error| error.to_string())?;
    let tx = conn.transaction().map_err(|error| error.to_string())?;
    for (id, capability, repo_id, context_hash) in requests {
        let actual: (String, String, Option<String>, String) = tx
            .query_row(
                "SELECT status, capability, repo_id, context_hash
                 FROM permission_request WHERE id = ?1",
                rusqlite::params![*id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .map_err(|error| format!("Approval request not found: {error}"))?;
        if actual.0 != "approved" {
            return Err(format!("Approval request is '{}', not approved", actual.0));
        }
        if actual.1 != *capability
            || actual.2.as_deref() != *repo_id
            || actual.3 != *context_hash
        {
            return Err("Approval does not match the current operation context".into());
        }
    }
    let consumed_at = chrono::Utc::now().to_rfc3339();
    for (id, _, _, _) in requests {
        let changed = tx
            .execute(
                "UPDATE permission_request SET status = 'consumed', consumed_at = ?1
                 WHERE id = ?2 AND status = 'approved'",
                rusqlite::params![consumed_at, *id],
            )
            .map_err(|error| error.to_string())?;
        if changed != 1 {
            return Err("Approval set changed while it was being consumed".into());
        }
        tx.execute(
            "INSERT INTO audit_log (id, action, subject, scope, capability, risk_level, detail)
             VALUES (?1, 'permission_consumed', ?2, 'permission', 'permission', 'high', '{}')",
            rusqlite::params![uuid::Uuid::new_v4().to_string(), *id],
        )
        .map_err(|error| error.to_string())?;
    }
    tx.commit().map_err(|error| error.to_string())
}

pub fn list_requests(status: Option<&str>, db: &Db) -> Result<Vec<PermissionRequest>, String> {
    expire_requests(db)?;
    let conn = db.conn.lock().map_err(|error| error.to_string())?;
    let sql = if status.is_some() {
        "SELECT id FROM permission_request WHERE status = ?1 ORDER BY created_at DESC"
    } else {
        "SELECT id FROM permission_request ORDER BY created_at DESC LIMIT 200"
    };
    let mut stmt = conn.prepare(sql).map_err(|error| error.to_string())?;
    let ids = if let Some(status) = status {
        stmt.query_map(rusqlite::params![status], |row| row.get::<_, String>(0))
            .map_err(|error| error.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| error.to_string())?
    } else {
        stmt.query_map([], |row| row.get::<_, String>(0))
            .map_err(|error| error.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| error.to_string())?
    };
    drop(stmt);
    drop(conn);
    ids.into_iter().map(|id| load_request(&id, db)).collect()
}

pub fn load_request(id: &str, db: &Db) -> Result<PermissionRequest, String> {
    let conn = db.conn.lock().map_err(|error| error.to_string())?;
    conn.query_row(
        "SELECT id, job_id, repo_id, capability, scope, risk_level, command, context_hash,
                details, status, created_at, expires_at, decided_at
         FROM permission_request WHERE id = ?1",
        rusqlite::params![id],
        |row| {
            let details: String = row.get(8)?;
            Ok(PermissionRequest {
                id: row.get(0)?,
                job_id: row.get(1)?,
                repo_id: row.get(2)?,
                capability: row.get(3)?,
                scope: row.get(4)?,
                risk_level: row.get(5)?,
                command: row.get(6)?,
                context_hash: row.get(7)?,
                details: serde_json::from_str(&details).unwrap_or_default(),
                status: row.get(9)?,
                created_at: row.get(10)?,
                expires_at: row.get(11)?,
                decided_at: row.get(12)?,
            })
        },
    )
    .map_err(|error| format!("Approval request not found: {}", error))
}

fn expire_requests(db: &Db) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|error| error.to_string())?;
    conn.execute(
        "UPDATE permission_request
         SET status = 'expired', decided_at = ?1
         WHERE status IN ('pending', 'approved') AND expires_at <= ?1",
        rusqlite::params![chrono::Utc::now().to_rfc3339()],
    )
    .map_err(|error| error.to_string())?;
    Ok(())
}

fn write_audit(id: &str, decision: &str, db: &Db) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|error| error.to_string())?;
    conn.execute(
        "INSERT INTO audit_log (id, action, subject, scope, capability, risk_level, detail)
         VALUES (?1, 'permission_decision', ?2, 'approval', 'permission', 'high', ?3)",
        rusqlite::params![
            uuid::Uuid::new_v4().to_string(),
            id,
            serde_json::json!({"decision": decision}).to_string(),
        ],
    )
    .map_err(|error| error.to_string())?;
    Ok(())
}

fn git_output(repo_path: &str, args: &[&str]) -> Result<String, String> {
    let output = crate::process_runner::run_default(
        "git",
        args,
        Some(std::path::Path::new(repo_path)),
    )
        .map_err(|error| format!("Cannot run git: {}", error))?;
    if !output.success {
        return Err(output.stderr.trim().to_string());
    }
    Ok(output.stdout.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;
    use std::sync::Mutex;

    fn test_db() -> Db {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
        for sql in [
            crate::db::MIGRATION_001_SQL,
            crate::db::MIGRATION_002_SQL,
            crate::db::MIGRATION_003_SQL,
            crate::db::MIGRATION_004_SQL,
            crate::db::MIGRATION_005_SQL,
            crate::db::MIGRATION_006_SQL,
            crate::db::MIGRATION_007_SQL,
            crate::db::MIGRATION_008_SQL,
            crate::db::MIGRATION_009_SQL,
            crate::db::MIGRATION_010_SQL,
            crate::db::MIGRATION_011_SQL,
            crate::db::MIGRATION_012_SQL,
            crate::db::MIGRATION_013_SQL,
            crate::db::MIGRATION_014_SQL,
        ] {
            conn.execute_batch(sql).unwrap();
        }
        Db {
            conn: Mutex::new(conn),
            path: std::path::PathBuf::new(),
        }
    }

    #[test]
    fn approvals_are_single_use_and_context_bound() {
        let db = test_db();
        let request = create_request(
            None,
            None,
            "shell.verify",
            "repo",
            "medium",
            Some("test"),
            "context-a",
            serde_json::json!({}),
            &db,
        )
        .unwrap();
        decide_request(&request.id, true, &db).unwrap();
        assert!(consume_request(&request.id, "shell.verify", None, "context-b", &db).is_err());
        consume_request(&request.id, "shell.verify", None, "context-a", &db).unwrap();
        assert!(consume_request(&request.id, "shell.verify", None, "context-a", &db).is_err());
    }
}
