use std::path::Path;
use std::time::{Duration, Instant};

/// Verification result.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerificationResult {
    pub success: bool,
    pub command: String,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub duration_ms: u64,
    pub timed_out: bool,
}

/// Detect available verification commands for a repo.
pub fn detect_commands(worktree_path: &str) -> Vec<VerificationCommand> {
    let path = Path::new(worktree_path);
    let mut commands = Vec::new();

    // Node.js / Bun
    if path.join("package.json").exists() {
        // Detect package manager and install commands
        if path.join("bun.lockb").exists() || path.join("bun.lock").exists() {
            commands.push(VerificationCommand {
                name: "bun install".into(),
                command: "bun install --frozen-lockfile".into(),
                timeout_secs: 120,
                category: "install".into(),
                risk_level: "high".into(),
            });
        } else if path.join("pnpm-lock.yaml").exists() {
            commands.push(VerificationCommand {
                name: "pnpm install".into(),
                command: "pnpm install --frozen-lockfile".into(),
                timeout_secs: 120,
                category: "install".into(),
                risk_level: "high".into(),
            });
        } else if path.join("yarn.lock").exists() {
            commands.push(VerificationCommand {
                name: "yarn install".into(),
                command: "yarn install --frozen-lockfile".into(),
                timeout_secs: 120,
                category: "install".into(),
                risk_level: "high".into(),
            });
        } else {
            commands.push(VerificationCommand {
                name: "npm install".into(),
                command: "npm ci".into(),
                timeout_secs: 120,
                category: "install".into(),
                risk_level: "high".into(),
            });
        }

        // Check for test/lint/build scripts
        if let Ok(content) = std::fs::read_to_string(path.join("package.json")) {
            if let Ok(pkg) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(scripts) = pkg.get("scripts") {
                    if scripts.get("test").is_some() {
                        commands.push(VerificationCommand {
                            name: "npm test".into(),
                            command: "npm test".into(),
                            timeout_secs: 120,
                            category: "test".into(),
                            risk_level: "low".into(),
                        });
                    }
                    if scripts.get("lint").is_some() {
                        commands.push(VerificationCommand {
                            name: "npm run lint".into(),
                            command: "npm run lint".into(),
                            timeout_secs: 60,
                            category: "lint".into(),
                            risk_level: "low".into(),
                        });
                    }
                    if scripts.get("build").is_some() {
                        commands.push(VerificationCommand {
                            name: "npm run build".into(),
                            command: "npm run build".into(),
                            timeout_secs: 120,
                            category: "build".into(),
                            risk_level: "medium".into(),
                        });
                    }
                    if scripts.get("typecheck").is_some() || scripts.get("type-check").is_some() {
                        let cmd = if scripts.get("typecheck").is_some() {
                            "npm run typecheck"
                        } else {
                            "npm run type-check"
                        };
                        commands.push(VerificationCommand {
                            name: cmd.into(),
                            command: cmd.into(),
                            timeout_secs: 60,
                            category: "typecheck".into(),
                            risk_level: "low".into(),
                        });
                    }
                }
            }
        }
    }

    // Rust
    if path.join("Cargo.toml").exists() {
        commands.push(VerificationCommand {
            name: "cargo check".into(),
            command: "cargo check".into(),
            timeout_secs: 180,
            category: "check".into(),
            risk_level: "low".into(),
        });
        commands.push(VerificationCommand {
            name: "cargo test".into(),
            command: "cargo test".into(),
            timeout_secs: 180,
            category: "test".into(),
            risk_level: "low".into(),
        });
        commands.push(VerificationCommand {
            name: "cargo clippy".into(),
            command: "cargo clippy -- -D warnings".into(),
            timeout_secs: 120,
            category: "lint".into(),
            risk_level: "low".into(),
        });
    }

    // Python
    if path.join("pyproject.toml").exists()
        || path.join("setup.py").exists()
        || path.join("requirements.txt").exists()
    {
        commands.push(VerificationCommand {
            name: "python check".into(),
            command: "python -m py_compile .".into(),
            timeout_secs: 60,
            category: "check".into(),
            risk_level: "low".into(),
        });
    }

    // Go
    if path.join("go.mod").exists() {
        commands.push(VerificationCommand {
            name: "go build".into(),
            command: "go build ./...".into(),
            timeout_secs: 120,
            category: "build".into(),
            risk_level: "medium".into(),
        });
        commands.push(VerificationCommand {
            name: "go test".into(),
            command: "go test ./...".into(),
            timeout_secs: 120,
            category: "test".into(),
            risk_level: "low".into(),
        });
    }

    commands
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerificationCommand {
    pub name: String,
    pub command: String,
    pub timeout_secs: u64,
    pub category: String,
    pub risk_level: String,
}

/// Resolve an exact command against the backend-detected allowlist for a repository.
pub fn resolve_command(
    worktree_path: &str,
    requested_command: &str,
) -> Result<VerificationCommand, String> {
    detect_commands(worktree_path)
        .into_iter()
        .find(|candidate| candidate.command == requested_command)
        .ok_or_else(|| {
            format!(
                "Verification command is not allowed for this repository: {}",
                requested_command
            )
        })
}

/// High-risk verification commands mutate dependencies or other repository state.
pub fn ensure_automatic_command_allowed(command: &VerificationCommand) -> Result<(), String> {
    if matches!(command.risk_level.as_str(), "high" | "critical") {
        return Err(format!(
            "Verification command '{}' requires an approval flow that is not implemented yet",
            command.command
        ));
    }
    Ok(())
}

/// A stored verification run record (from DB).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerificationRun {
    pub id: String,
    pub repo_id: String,
    pub job_id: Option<String>,
    pub command: String,
    pub cwd: String,
    pub category: String,
    pub risk_level: String,
    pub success: bool,
    pub exit_code: Option<i32>,
    pub duration_ms: u64,
    pub timed_out: bool,
    pub stdout_tail: String,
    pub stderr_tail: String,
    pub created_at: String,
}

/// Truncate output to the last `max_bytes` bytes, keeping the tail.
pub fn tail_truncate(s: &str, max_bytes: usize) -> String {
    if s.len() <= max_bytes {
        return s.to_string();
    }
    let start = s.len() - max_bytes;
    // Find the next UTF-8 char boundary to avoid panics
    let mut cut = start;
    while cut < s.len() && !s.is_char_boundary(cut) {
        cut += 1;
    }
    format!("...[truncated]{}", &s[cut..])
}

/// Run a verification command with timeout.
pub fn run_verification(command: &str, cwd: &str, timeout_secs: u64) -> VerificationResult {
    let start = Instant::now();

    let child = shell_command(command)
        .current_dir(cwd)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn();

    let mut child = match child {
        Ok(c) => c,
        Err(e) => {
            return VerificationResult {
                success: false,
                command: command.to_string(),
                exit_code: None,
                stdout: String::new(),
                stderr: format!("Failed to start command: {}", e),
                duration_ms: 0,
                timed_out: false,
            };
        }
    };

    // Simple timeout: check if process has exited within timeout
    let timeout = Duration::from_secs(timeout_secs);
    let result = child.wait_timeout(timeout);

    match result {
        Ok(Some(status)) => {
            let output = child
                .wait_with_output()
                .unwrap_or_else(|_| std::process::Output {
                    status,
                    stdout: Vec::new(),
                    stderr: Vec::new(),
                });
            VerificationResult {
                success: status.success(),
                command: command.to_string(),
                exit_code: status.code(),
                stdout: crate::ai_provider::redact_secrets(
                    &String::from_utf8_lossy(&output.stdout),
                ),
                stderr: crate::ai_provider::redact_secrets(
                    &String::from_utf8_lossy(&output.stderr),
                ),
                duration_ms: start.elapsed().as_millis() as u64,
                timed_out: false,
            }
        }
        Ok(None) => {
            // Timed out - kill the process
            let _ = child.kill();
            let _ = child.wait();
            VerificationResult {
                success: false,
                command: command.to_string(),
                exit_code: None,
                stdout: String::new(),
                stderr: format!("Command timed out after {}s", timeout_secs),
                duration_ms: start.elapsed().as_millis() as u64,
                timed_out: true,
            }
        }
        Err(e) => VerificationResult {
            success: false,
            command: command.to_string(),
            exit_code: None,
            stdout: String::new(),
            stderr: format!("Failed to wait for process: {}", e),
            duration_ms: start.elapsed().as_millis() as u64,
            timed_out: false,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp_dir() -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!("atlasforge_verify_{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn tail_truncate_keeps_short_output() {
        assert_eq!(tail_truncate("short", 10), "short");
    }

    #[test]
    fn tail_truncate_keeps_tail_and_marker() {
        let truncated = tail_truncate("abcdefghijklmnopqrstuvwxyz", 8);
        assert_eq!(truncated, "...[truncated]stuvwxyz");
    }

    #[test]
    fn node_install_commands_are_high_risk() {
        let dir = temp_dir();
        fs::write(dir.join("package.json"), r#"{"scripts":{"test":"vitest"}}"#).unwrap();
        fs::write(dir.join("package-lock.json"), "{}").unwrap();

        let commands = detect_commands(&dir.to_string_lossy());
        let install = commands
            .iter()
            .find(|cmd| cmd.category == "install")
            .unwrap();
        assert_eq!(install.command, "npm ci");
        assert_eq!(install.risk_level, "high");

        let test = commands.iter().find(|cmd| cmd.category == "test").unwrap();
        assert_eq!(test.risk_level, "low");

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn bun_install_uses_frozen_lockfile() {
        let dir = temp_dir();
        fs::write(dir.join("package.json"), "{}").unwrap();
        fs::write(dir.join("bun.lock"), "").unwrap();

        let commands = detect_commands(&dir.to_string_lossy());
        let install = commands
            .iter()
            .find(|cmd| cmd.category == "install")
            .unwrap();
        assert_eq!(install.command, "bun install --frozen-lockfile");
        assert_eq!(install.risk_level, "high");

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn rust_commands_are_low_risk_checks() {
        let dir = temp_dir();
        fs::write(
            dir.join("Cargo.toml"),
            "[package]\nname='x'\nversion='0.1.0'\nedition='2021'\n",
        )
        .unwrap();

        let commands = detect_commands(&dir.to_string_lossy());
        assert!(commands
            .iter()
            .any(|cmd| cmd.command == "cargo check" && cmd.risk_level == "low"));
        assert!(commands
            .iter()
            .any(|cmd| cmd.command == "cargo test" && cmd.risk_level == "low"));
        assert!(commands
            .iter()
            .any(|cmd| cmd.command == "cargo clippy -- -D warnings" && cmd.risk_level == "low"));

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn resolve_command_rejects_arbitrary_shell_input() {
        let dir = temp_dir();
        fs::write(dir.join("package.json"), r#"{"scripts":{"test":"vitest"}}"#).unwrap();

        let allowed = resolve_command(&dir.to_string_lossy(), "npm test").unwrap();
        assert_eq!(allowed.category, "test");
        assert!(resolve_command(&dir.to_string_lossy(), "echo unsafe").is_err());

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn automatic_verification_rejects_install_commands() {
        let command = VerificationCommand {
            name: "npm install".into(),
            command: "npm ci".into(),
            timeout_secs: 120,
            category: "install".into(),
            risk_level: "high".into(),
        };
        assert!(ensure_automatic_command_allowed(&command).is_err());
    }
}

/// Summarize verification output for inclusion in reports.
pub fn summarize_output(result: &VerificationResult) -> String {
    let mut summary = Vec::new();

    summary.push(format!(
        "**{}** — {} ({})",
        result.command,
        if result.success {
            "✅ PASSED"
        } else {
            "❌ FAILED"
        },
        format_duration(result.duration_ms)
    ));

    if result.timed_out {
        summary.push("- ⏱️ Timed out".into());
    }

    if let Some(code) = result.exit_code {
        if code != 0 {
            summary.push(format!("- Exit code: {}", code));
        }
    }

    // Include last N lines of stderr if failed
    if !result.success && !result.stderr.is_empty() {
        let lines: Vec<&str> = result.stderr.lines().collect();
        let tail = lines.iter().rev().take(5).collect::<Vec<_>>();
        summary.push("- Last stderr lines:".into());
        for line in tail {
            summary.push(format!("  {}", line));
        }
    }

    summary.join("\n")
}

fn format_duration(ms: u64) -> String {
    if ms < 1000 {
        format!("{}ms", ms)
    } else {
        format!("{:.1}s", ms as f64 / 1000.0)
    }
}

fn shell_command(command: &str) -> std::process::Command {
    #[cfg(windows)]
    {
        let mut cmd = std::process::Command::new("cmd");
        cmd.arg("/C").arg(command);
        cmd
    }

    #[cfg(not(windows))]
    {
        let mut cmd = std::process::Command::new("sh");
        cmd.arg("-c").arg(command);
        cmd
    }
}

// Extension trait for child process wait with timeout
trait ChildWaitTimeout {
    fn wait_timeout(
        &mut self,
        timeout: Duration,
    ) -> Result<Option<std::process::ExitStatus>, String>;
}

impl ChildWaitTimeout for std::process::Child {
    fn wait_timeout(
        &mut self,
        timeout: Duration,
    ) -> Result<Option<std::process::ExitStatus>, String> {
        let start = Instant::now();
        loop {
            match self.try_wait() {
                Ok(Some(status)) => return Ok(Some(status)),
                Ok(None) => {
                    if start.elapsed() > timeout {
                        return Ok(None);
                    }
                    std::thread::sleep(Duration::from_millis(100));
                }
                Err(e) => return Err(e.to_string()),
            }
        }
    }
}
