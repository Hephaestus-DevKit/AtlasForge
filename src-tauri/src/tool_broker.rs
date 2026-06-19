use crate::db::Db;
use crate::models::*;
use std::path::Path;

/// Tool metadata from the registry.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolInfo {
    pub id: String,
    pub name: String,
    pub category: String,
    pub description: String,
    pub risk_level: String,
    pub requires_permission: bool,
    pub dry_run_supported: bool,
}

/// Result of a tool invocation.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolResult {
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
    pub was_dry_run: bool,
}

/// List all registered tools.
pub fn list_tools(db: &Db) -> Result<Vec<ToolInfo>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT id, name, category, description, risk_level, requires_permission, dry_run_supported FROM tool ORDER BY category, name")
        .map_err(|e| e.to_string())?;

    let tools = stmt
        .query_map([], |row| {
            Ok(ToolInfo {
                id: row.get(0)?,
                name: row.get(1)?,
                category: row.get(2)?,
                description: row.get(3)?,
                risk_level: row.get(4)?,
                requires_permission: row.get::<_, i32>(5)? != 0,
                dry_run_supported: row.get::<_, i32>(6)? != 0,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(tools)
}

/// Check if a tool invocation is permitted based on workspace roots and risk level.
pub fn check_permission(
    tool_name: &str,
    input: &serde_json::Value,
    roots: &[WorkspaceRoot],
    auto_policy: &str, // "observe", "suggest", "assisted", "autonomous_local", "autonomous_publish"
) -> Result<String, String> {
    let risk = get_tool_risk(tool_name);

    // Check path-based authorization for fs/git tools
    let path_candidate = input
        .get("path")
        .or_else(|| input.get("cwd"))
        .and_then(|v| v.as_str());

    let requires_path = tool_name.starts_with("fs.")
        || tool_name.starts_with("git.")
        || tool_name.starts_with("shell.");
    if requires_path && path_candidate.is_none() {
        return Err(format!(
            "Tool '{}' requires an authorized path or working directory",
            tool_name
        ));
    }

    if let Some(path_str) = path_candidate {
        let path = Path::new(path_str);
        let authorized = crate::security::authorize_path(path, roots);

        if authorized.is_none() {
            return Err(format!(
                "Path '{}' is not within any authorized workspace root",
                path_str
            ));
        }

        // Check write permission
        if tool_name.contains("write")
            || tool_name.contains("commit")
            || tool_name.contains("mutate")
        {
            if let Err(e) = crate::security::authorize_write(path, roots) {
                return Err(e);
            }
        }
    }

    if tool_name == "shell.verify" {
        let command = input
            .get("command")
            .and_then(|value| value.as_str())
            .ok_or_else(|| "shell.verify requires a command".to_string())?;
        let cwd = input
            .get("cwd")
            .and_then(|value| value.as_str())
            .ok_or_else(|| "shell.verify requires a working directory".to_string())?;
        let command = crate::verification::resolve_command(cwd, command)?;
        crate::verification::ensure_automatic_command_allowed(&command)?;
    }

    // Check risk level vs auto policy
    match auto_policy {
        "observe" => {
            if risk != "none" {
                return Err(format!(
                    "Auto policy 'observe' does not allow tool '{}' with risk level '{}'",
                    tool_name, risk
                ));
            }
        }
        "suggest" => {
            if risk == "critical" {
                return Err(format!(
                    "Auto policy 'suggest' does not allow critical tool '{}'",
                    tool_name
                ));
            }
        }
        "assisted" => {
            if matches!(risk, "medium" | "high" | "critical") {
                return Err(format!(
                    "Tool '{}' requires user approval (risk: {})",
                    tool_name, risk
                ));
            }
        }
        "autonomous_local" => {
            if risk == "critical" {
                return Err(format!(
                    "Auto policy 'autonomous_local' does not allow critical tool '{}'",
                    tool_name
                ));
            }
        }
        "autonomous_publish" => {
            // All tools allowed, but audit everything
        }
        _ => return Err(format!("Unknown auto policy: {}", auto_policy)),
    }

    Ok("auto_approved".to_string())
}

/// Invoke a tool, checking permissions and recording audit.
pub fn invoke_tool(
    job_id: &str,
    tool_name: &str,
    input: &serde_json::Value,
    dry_run: bool,
    db: &Db,
) -> Result<ToolResult, String> {
    let risk = get_tool_risk(tool_name);
    let serialized_input = serde_json::to_string(input).map_err(|e| e.to_string())?;
    if !crate::ai_provider::scan_for_secrets(&serialized_input).is_empty() {
        return Err("Tool input contains potential secret material".into());
    }
    let stored_input = crate::ai_provider::redact_secrets(&serialized_input);

    // Record the invocation
    let invocation_id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    {
        let conn = db.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT INTO tool_invocation (id, job_id, tool_name, input, status, risk_level, permission_decision, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![
                invocation_id,
                job_id,
                tool_name,
                stored_input,
                if dry_run { Some("dry_run") } else { None },
                risk,
                if dry_run { "dry_run" } else { "pending" },
                now,
            ],
        )
        .map_err(|e| e.to_string())?;
    }

    if dry_run {
        let preview = dry_run_preview(tool_name, input);
        let conn = db.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "UPDATE tool_invocation SET status = 'dry_run', output = ?1, completed_at = datetime('now') WHERE id = ?2",
            rusqlite::params![serde_json::to_string(&preview).unwrap_or_default(), invocation_id],
        )
        .map_err(|e| e.to_string())?;

        return Ok(ToolResult {
            success: true,
            output: serde_json::to_string(&preview).unwrap_or_default(),
            error: None,
            was_dry_run: true,
        });
    }

    let roots = load_workspace_roots(db)?;
    let permission_decision = match check_permission(tool_name, input, &roots, "assisted") {
        Ok(decision) => decision,
        Err(e) => {
            let conn = db.conn.lock().map_err(|lock_err| lock_err.to_string())?;
            conn.execute(
                "UPDATE tool_invocation SET status = 'failed', permission_decision = 'denied', error_message = ?1, completed_at = datetime('now') WHERE id = ?2",
                rusqlite::params![e.clone(), invocation_id],
            )
            .map_err(|db_err| db_err.to_string())?;
            return Err(e);
        }
    };

    {
        let conn = db.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "UPDATE tool_invocation SET permission_decision = ?1 WHERE id = ?2",
            rusqlite::params![permission_decision, invocation_id],
        )
        .map_err(|e| e.to_string())?;
    }

    // Execute the tool
    let result = execute_tool(tool_name, input);

    // Update invocation record
    {
        let stored_output = crate::ai_provider::redact_secrets(&result.output);
        let stored_error = result
            .error
            .as_deref()
            .map(crate::ai_provider::redact_secrets);
        let conn = db.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "UPDATE tool_invocation SET status = ?1, output = ?2, error_message = ?3, completed_at = datetime('now') WHERE id = ?4",
            rusqlite::params![
                if result.success { "completed" } else { "failed" },
                stored_output,
                stored_error,
                invocation_id,
            ],
        )
        .map_err(|e| e.to_string())?;
    }

    write_audit_log(
        db,
        "tool_invocation",
        &format!("job:{}", job_id),
        tool_name,
        tool_name,
        risk,
        &serde_json::json!({
            "invocation_id": invocation_id,
            "tool": tool_name,
            "dry_run": false,
            "success": result.success,
        })
        .to_string(),
    )?;

    Ok(result)
}

/// List tool invocations for a job.
pub fn list_invocations(job_id: &str, db: &Db) -> Result<Vec<serde_json::Value>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT id, tool_name, input, output, status, risk_level, permission_decision, error_message, created_at, completed_at FROM tool_invocation WHERE job_id = ?1 ORDER BY created_at")
        .map_err(|e| e.to_string())?;

    let invocations = stmt
        .query_map(rusqlite::params![job_id], |row| {
            Ok(serde_json::json!({
                "id": row.get::<_, String>(0)?,
                "toolName": row.get::<_, String>(1)?,
                "input": row.get::<_, String>(2)?,
                "output": row.get::<_, Option<String>>(3)?,
                "status": row.get::<_, String>(4)?,
                "riskLevel": row.get::<_, String>(5)?,
                "permissionDecision": row.get::<_, Option<String>>(6)?,
                "errorMessage": row.get::<_, Option<String>>(7)?,
                "createdAt": row.get::<_, String>(8)?,
                "completedAt": row.get::<_, Option<String>>(9)?,
            }))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(invocations)
}

fn get_tool_risk(tool_name: &str) -> &'static str {
    match tool_name {
        "fs.list" | "fs.read" | "git.status" | "git.diff" | "github.read" => "none",
        "shell.verify" => "medium",
        "fs.write_patch" | "git.commit" | "git.tag" | "github.create_pr" => "high",
        "shell.mutate"
        | "github.create_release"
        | "github.delete_release"
        | "git.push"
        | "git.force_push" => "critical",
        _ => "low",
    }
}

fn dry_run_preview(tool_name: &str, input: &serde_json::Value) -> serde_json::Value {
    match tool_name {
        "fs.write_patch" => serde_json::json!({
            "preview": "Would apply patch to file",
            "file": input.get("path").unwrap_or(&serde_json::Value::Null),
            "diffPreview": input.get("diff").and_then(|d| d.as_str()).map(|s| {
                let lines: Vec<&str> = s.lines().take(5).collect();
                if lines.len() < s.lines().count() {
                    format!("{}\n... ({} more lines)", lines.join("\n"), s.lines().count() - 5)
                } else {
                    lines.join("\n")
                }
            }),
        }),
        "git.commit" => serde_json::json!({
            "preview": "Would create git commit",
            "message": input.get("message").unwrap_or(&serde_json::Value::Null),
            "filesStaged": input.get("files").unwrap_or(&serde_json::Value::Null),
        }),
        "shell.verify" | "shell.mutate" => serde_json::json!({
            "preview": "Would execute command",
            "command": input.get("command").unwrap_or(&serde_json::Value::Null),
        }),
        "github.create_pr" => serde_json::json!({
            "preview": "Would create pull request",
            "title": input.get("title").unwrap_or(&serde_json::Value::Null),
            "branch": input.get("branch").unwrap_or(&serde_json::Value::Null),
        }),
        "github.create_release" => serde_json::json!({
            "preview": "Would create GitHub release",
            "tag": input.get("tag").unwrap_or(&serde_json::Value::Null),
            "name": input.get("name").unwrap_or(&serde_json::Value::Null),
        }),
        _ => serde_json::json!({
            "preview": format!("Would execute tool: {}", tool_name),
            "input": input,
        }),
    }
}

fn execute_tool(tool_name: &str, input: &serde_json::Value) -> ToolResult {
    match tool_name {
        "fs.list" => {
            let path = input.get("path").and_then(|v| v.as_str()).unwrap_or(".");
            let p = Path::new(path);
            if !p.exists() {
                return ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(format!("Path does not exist: {}", path)),
                    was_dry_run: false,
                };
            }
            match std::fs::read_dir(p) {
                Ok(read_dir) => {
                    let entries: Vec<String> = read_dir
                        .filter_map(|e| e.ok())
                        .map(|e| e.file_name().to_string_lossy().to_string())
                        .collect();
                    ToolResult {
                        success: true,
                        output: serde_json::to_string(&entries).unwrap_or_default(),
                        error: None,
                        was_dry_run: false,
                    }
                }
                Err(e) => ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(e.to_string()),
                    was_dry_run: false,
                },
            }
        }
        "fs.read" => {
            let path = input.get("path").and_then(|v| v.as_str()).unwrap_or("");
            match std::fs::read_to_string(path) {
                Ok(content) => {
                    let max_len = 100_000;
                    let truncated = content.len() > max_len;
                    let s = if truncated {
                        &content[..max_len]
                    } else {
                        &content
                    };
                    ToolResult {
                        success: true,
                        output: s.to_string(),
                        error: if truncated {
                            Some("Output truncated at 100KB".into())
                        } else {
                            None
                        },
                        was_dry_run: false,
                    }
                }
                Err(e) => ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(e.to_string()),
                    was_dry_run: false,
                },
            }
        }
        "git.status" | "git.diff" => {
            let path = input.get("path").and_then(|v| v.as_str()).unwrap_or(".");
            let args: Vec<&str> = if tool_name == "git.status" {
                vec!["status", "--porcelain"]
            } else {
                vec!["diff"]
            };
            match std::process::Command::new("git")
                .args(&args)
                .current_dir(path)
                .output()
            {
                Ok(output) => {
                    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                    if output.status.success() {
                        ToolResult {
                            success: true,
                            output: stdout,
                            error: None,
                            was_dry_run: false,
                        }
                    } else {
                        ToolResult {
                            success: false,
                            output: stdout,
                            error: Some(String::from_utf8_lossy(&output.stderr).to_string()),
                            was_dry_run: false,
                        }
                    }
                }
                Err(e) => ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(e.to_string()),
                    was_dry_run: false,
                },
            }
        }
        "shell.verify" => {
            let cmd = input.get("command").and_then(|v| v.as_str()).unwrap_or("");
            let cwd = input.get("cwd").and_then(|v| v.as_str()).unwrap_or(".");
            let resolved = match crate::verification::resolve_command(cwd, cmd) {
                Ok(resolved) => resolved,
                Err(error) => {
                    return ToolResult {
                        success: false,
                        output: String::new(),
                        error: Some(error),
                        was_dry_run: false,
                    };
                }
            };
            let result = crate::verification::run_verification(
                &resolved.command,
                cwd,
                resolved.timeout_secs,
            );
            ToolResult {
                success: result.success,
                output: serde_json::to_string(&result).unwrap_or_default(),
                error: if result.success {
                    None
                } else {
                    Some(result.stderr.clone())
                },
                was_dry_run: false,
            }
        }
        _ => ToolResult {
            success: false,
            output: String::new(),
            error: Some(format!("Tool '{}' not implemented yet", tool_name)),
            was_dry_run: false,
        },
    }
}

fn load_workspace_roots(db: &Db) -> Result<Vec<WorkspaceRoot>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT id, path, label, access_mode, scan_enabled, include_globs, exclude_globs, created_at, last_scanned_at FROM workspace_root")
        .map_err(|e| e.to_string())?;

    let roots = stmt
        .query_map([], |row| {
            let include_globs_str: String = row.get(5)?;
            let exclude_globs_str: String = row.get(6)?;
            Ok(WorkspaceRoot {
                id: row.get(0)?,
                path: row.get(1)?,
                label: row.get(2)?,
                access_mode: row.get(3)?,
                scan_enabled: row.get::<_, i32>(4)? != 0,
                include_globs: serde_json::from_str(&include_globs_str).unwrap_or_default(),
                exclude_globs: serde_json::from_str(&exclude_globs_str).unwrap_or_default(),
                created_at: row.get(7)?,
                last_scanned_at: row.get(8)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(roots)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn root_for(path: &Path) -> WorkspaceRoot {
        WorkspaceRoot {
            id: "root".into(),
            path: path.to_string_lossy().into_owned(),
            label: "Root".into(),
            access_mode: "read_write".into(),
            scan_enabled: true,
            include_globs: vec![],
            exclude_globs: vec![],
            created_at: String::new(),
            last_scanned_at: None,
        }
    }

    #[test]
    fn path_based_tools_require_explicit_authorized_path() {
        assert!(check_permission(
            "git.status",
            &serde_json::json!({}),
            &[],
            "assisted"
        )
        .is_err());
    }

    #[test]
    fn assisted_policy_blocks_medium_shell_execution_without_approval() {
        let temp = tempfile::tempdir().unwrap();
        fs::write(
            temp.path().join("package.json"),
            r#"{"scripts":{"test":"vitest"}}"#,
        )
        .unwrap();
        let roots = vec![root_for(temp.path())];
        let input = serde_json::json!({
            "cwd": temp.path().to_string_lossy(),
            "command": "npm test",
        });
        assert!(check_permission("shell.verify", &input, &roots, "assisted").is_err());
    }
}

fn write_audit_log(
    db: &Db,
    action: &str,
    subject: &str,
    scope: &str,
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
            scope,
            capability,
            risk_level,
            detail,
        ],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}
