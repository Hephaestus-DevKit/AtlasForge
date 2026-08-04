use std::io::Read;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

const MAX_CAPTURE_BYTES: usize = 1_000_000;

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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerificationCommand {
    pub name: String,
    pub command: String,
    pub timeout_secs: u64,
    pub category: String,
    pub risk_level: String,
    pub requires_approval: bool,
    pub expanded_command: String,
    pub risk_explanation: String,
}

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

pub fn detect_commands(worktree_path: &str) -> Vec<VerificationCommand> {
    let path = Path::new(worktree_path);
    let mut commands = Vec::new();

    if path.join("package.json").exists() {
        let (install_name, install_command) =
            if path.join("bun.lockb").exists() || path.join("bun.lock").exists() {
                ("bun install", "bun install --frozen-lockfile")
            } else if path.join("pnpm-lock.yaml").exists() {
                ("pnpm install", "pnpm install --frozen-lockfile")
            } else if path.join("yarn.lock").exists() {
                ("yarn install", "yarn install --frozen-lockfile")
            } else {
                ("npm install", "npm ci")
            };
        commands.push(VerificationCommand {
            name: install_name.into(),
            command: install_command.into(),
            timeout_secs: 120,
            category: "install".into(),
            risk_level: "high".into(),
            requires_approval: true,
            expanded_command: format!(
                "{} (may execute dependency lifecycle scripts)",
                install_command
            ),
            risk_explanation: "Installs dependencies and may execute lifecycle scripts supplied by the repository or its dependencies.".into(),
        });

        if let Ok(content) = std::fs::read_to_string(path.join("package.json")) {
            if let Ok(package) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(scripts) = package.get("scripts").and_then(|value| value.as_object()) {
                    for (script, command, timeout, category) in [
                        ("test", "npm test", 120, "test"),
                        ("lint", "npm run lint", 60, "lint"),
                        ("build", "npm run build", 120, "build"),
                    ] {
                        if scripts.contains_key(script) {
                            commands.push(node_script_command(
                                scripts, script, command, timeout, category,
                            ));
                        }
                    }
                    if scripts.contains_key("typecheck") {
                        commands.push(node_script_command(
                            scripts,
                            "typecheck",
                            "npm run typecheck",
                            60,
                            "typecheck",
                        ));
                    } else if scripts.contains_key("type-check") {
                        commands.push(node_script_command(
                            scripts,
                            "type-check",
                            "npm run type-check",
                            60,
                            "typecheck",
                        ));
                    }
                }
            }
        }
    }

    if path.join("Cargo.toml").exists() {
        for (name, command, timeout, category, reason) in [
            (
                "cargo check",
                "cargo check",
                180,
                "check",
                "Cargo may execute repository build scripts and procedural macros while checking.",
            ),
            (
                "cargo test",
                "cargo test",
                180,
                "test",
                "Compiles repository code and executes repository-controlled test binaries.",
            ),
            (
                "cargo clippy",
                "cargo clippy -- -D warnings",
                120,
                "lint",
                "Cargo may execute repository build scripts and procedural macros while linting.",
            ),
        ] {
            commands.push(VerificationCommand {
                name: name.into(),
                command: command.into(),
                timeout_secs: timeout,
                category: category.into(),
                risk_level: "medium".into(),
                requires_approval: true,
                expanded_command: command.into(),
                risk_explanation: reason.into(),
            });
        }
    }

    if path.join("pyproject.toml").exists()
        || path.join("setup.py").exists()
        || path.join("requirements.txt").exists()
    {
        commands.push(VerificationCommand {
            name: "python compile".into(),
            command: "python -m compileall .".into(),
            timeout_secs: 60,
            category: "check".into(),
            risk_level: "medium".into(),
            requires_approval: true,
            expanded_command: "python -m compileall .".into(),
            risk_explanation:
                "Processes repository-controlled source files using the selected Python interpreter."
                    .into(),
        });
    }

    if path.join("go.mod").exists() {
        for (name, command, category, reason) in [
            (
                "go build",
                "go build ./...",
                "build",
                "Builds repository-controlled packages using the configured Go toolchain.",
            ),
            (
                "go test",
                "go test ./...",
                "test",
                "Compiles repository code and executes repository-controlled tests.",
            ),
        ] {
            commands.push(VerificationCommand {
                name: name.into(),
                command: command.into(),
                timeout_secs: 120,
                category: category.into(),
                risk_level: "medium".into(),
                requires_approval: true,
                expanded_command: command.into(),
                risk_explanation: reason.into(),
            });
        }
    }

    commands
}

fn node_script_command(
    scripts: &serde_json::Map<String, serde_json::Value>,
    script_name: &str,
    command: &str,
    timeout_secs: u64,
    category: &str,
) -> VerificationCommand {
    let mut lifecycle = Vec::new();
    for name in [
        format!("pre{}", script_name),
        script_name.to_string(),
        format!("post{}", script_name),
    ] {
        if let Some(body) = scripts.get(&name).and_then(|value| value.as_str()) {
            lifecycle.push(format!("{}: {}", name, body));
        }
    }
    VerificationCommand {
        name: command.into(),
        command: command.into(),
        timeout_secs,
        category: category.into(),
        risk_level: "medium".into(),
        requires_approval: true,
        expanded_command: lifecycle.join("\n"),
        risk_explanation:
            "Runs repository-controlled package scripts, including matching pre/post lifecycle hooks."
                .into(),
    }
}

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

#[cfg(test)]
pub fn ensure_automatic_command_allowed(command: &VerificationCommand) -> Result<(), String> {
    if command.requires_approval {
        return Err(format!(
            "Verification command '{}' requires explicit approval",
            command.command
        ));
    }
    Ok(())
}

pub fn tail_truncate(value: &str, max_bytes: usize) -> String {
    if value.len() <= max_bytes {
        return value.to_string();
    }
    let mut cut = value.len() - max_bytes;
    while cut < value.len() && !value.is_char_boundary(cut) {
        cut += 1;
    }
    format!("...[truncated]{}", &value[cut..])
}

pub fn run_verification(command: &str, cwd: &str, timeout_secs: u64) -> VerificationResult {
    run_verification_with_control(command, cwd, timeout_secs, Arc::new(AtomicBool::new(false)))
}

pub fn run_verification_with_control(
    command: &str,
    cwd: &str,
    timeout_secs: u64,
    cancellation: Arc<AtomicBool>,
) -> VerificationResult {
    let started = Instant::now();
    let child = shell_command(command)
        .current_dir(cwd)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn();
    let mut child = match child {
        Ok(child) => child,
        Err(error) => {
            return VerificationResult {
                success: false,
                command: command.into(),
                exit_code: None,
                stdout: String::new(),
                stderr: format!("Failed to start command: {}", error),
                duration_ms: 0,
                timed_out: false,
            };
        }
    };
    #[cfg(windows)]
    let process_tree = WindowsJob::attach(&child).ok();

    let stdout_reader = child
        .stdout
        .take()
        .map(|stdout| std::thread::spawn(move || read_bounded(stdout, MAX_CAPTURE_BYTES)));
    let stderr_reader = child
        .stderr
        .take()
        .map(|stderr| std::thread::spawn(move || read_bounded(stderr, MAX_CAPTURE_BYTES)));
    let timeout = Duration::from_secs(timeout_secs);

    loop {
        if cancellation.load(Ordering::SeqCst) {
            terminate_process_tree(
                &mut child,
                #[cfg(windows)]
                process_tree.as_ref(),
            );
            return build_result(
                command,
                false,
                None,
                join_reader(stdout_reader),
                append_message(join_reader(stderr_reader), "Command cancelled by user"),
                started.elapsed(),
                false,
            );
        }
        if started.elapsed() >= timeout {
            terminate_process_tree(
                &mut child,
                #[cfg(windows)]
                process_tree.as_ref(),
            );
            return build_result(
                command,
                false,
                None,
                join_reader(stdout_reader),
                append_message(
                    join_reader(stderr_reader),
                    &format!("Command timed out after {}s", timeout_secs),
                ),
                started.elapsed(),
                true,
            );
        }
        match child.try_wait() {
            Ok(Some(status)) => {
                return build_result(
                    command,
                    status.success(),
                    status.code(),
                    join_reader(stdout_reader),
                    join_reader(stderr_reader),
                    started.elapsed(),
                    false,
                );
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(50)),
            Err(error) => {
                terminate_process_tree(
                    &mut child,
                    #[cfg(windows)]
                    process_tree.as_ref(),
                );
                return build_result(
                    command,
                    false,
                    None,
                    join_reader(stdout_reader),
                    append_message(
                        join_reader(stderr_reader),
                        &format!("Failed to wait for process: {}", error),
                    ),
                    started.elapsed(),
                    false,
                );
            }
        }
    }
}

fn build_result(
    command: &str,
    success: bool,
    exit_code: Option<i32>,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    duration: Duration,
    timed_out: bool,
) -> VerificationResult {
    VerificationResult {
        success,
        command: command.into(),
        exit_code,
        stdout: crate::ai_provider::redact_secrets(&String::from_utf8_lossy(&stdout)),
        stderr: crate::ai_provider::redact_secrets(&String::from_utf8_lossy(&stderr)),
        duration_ms: duration.as_millis() as u64,
        timed_out,
    }
}

fn read_bounded<R: Read>(mut reader: R, max_bytes: usize) -> Vec<u8> {
    let mut retained = Vec::new();
    let mut buffer = [0_u8; 8192];
    while let Ok(read) = reader.read(&mut buffer) {
        if read == 0 {
            break;
        }
        retained.extend_from_slice(&buffer[..read]);
        if retained.len() > max_bytes {
            let overflow = retained.len() - max_bytes;
            retained.drain(..overflow);
        }
    }
    retained
}

fn join_reader(reader: Option<std::thread::JoinHandle<Vec<u8>>>) -> Vec<u8> {
    reader
        .and_then(|handle| handle.join().ok())
        .unwrap_or_default()
}

fn append_message(mut output: Vec<u8>, message: &str) -> Vec<u8> {
    if !output.is_empty() && !output.ends_with(b"\n") {
        output.push(b'\n');
    }
    output.extend_from_slice(message.as_bytes());
    output
}

fn terminate_process_tree(
    child: &mut std::process::Child,
    #[cfg(windows)] job: Option<&WindowsJob>,
) {
    #[cfg(windows)]
    {
        if let Some(job) = job {
            job.terminate();
        } else {
            let _ = std::process::Command::new("taskkill")
                .args(["/PID", &child.id().to_string(), "/T", "/F"])
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status();
        }
    }
    let _ = child.kill();
    let _ = child.wait();
}

#[cfg(windows)]
struct WindowsJob {
    handle: windows_sys::Win32::Foundation::HANDLE,
}

#[cfg(windows)]
impl WindowsJob {
    fn attach(child: &std::process::Child) -> Result<Self, String> {
        use std::os::windows::io::AsRawHandle;
        use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
        use windows_sys::Win32::System::JobObjects::{
            AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
            SetInformationJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
            JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
        };

        unsafe {
            let handle = CreateJobObjectW(std::ptr::null(), std::ptr::null());
            if handle.is_null() || handle == INVALID_HANDLE_VALUE {
                return Err("CreateJobObjectW failed".into());
            }
            let mut limits: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
            limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            if SetInformationJobObject(
                handle,
                JobObjectExtendedLimitInformation,
                &limits as *const _ as *const std::ffi::c_void,
                std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            ) == 0
            {
                CloseHandle(handle);
                return Err("SetInformationJobObject failed".into());
            }
            let process_handle = child.as_raw_handle() as windows_sys::Win32::Foundation::HANDLE;
            if AssignProcessToJobObject(handle, process_handle) == 0 {
                CloseHandle(handle);
                return Err("AssignProcessToJobObject failed".into());
            }
            Ok(Self { handle })
        }
    }

    fn terminate(&self) {
        unsafe {
            windows_sys::Win32::System::JobObjects::TerminateJobObject(self.handle, 1);
        }
    }
}

#[cfg(windows)]
impl Drop for WindowsJob {
    fn drop(&mut self) {
        unsafe {
            windows_sys::Win32::Foundation::CloseHandle(self.handle);
        }
    }
}

pub fn summarize_output(result: &VerificationResult) -> String {
    let mut summary = vec![format!(
        "**{}** - {} ({})",
        result.command,
        if result.success { "PASSED" } else { "FAILED" },
        format_duration(result.duration_ms)
    )];
    if result.timed_out {
        summary.push("- Timed out".into());
    }
    if let Some(code) = result.exit_code {
        if code != 0 {
            summary.push(format!("- Exit code: {}", code));
        }
    }
    if !result.success && !result.stderr.is_empty() {
        summary.push("- Last stderr lines:".into());
        for line in result
            .stderr
            .lines()
            .rev()
            .take(5)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
        {
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
        assert_eq!(
            tail_truncate("abcdefghijklmnopqrstuvwxyz", 8),
            "...[truncated]stuvwxyz"
        );
    }

    #[test]
    fn node_scripts_show_lifecycle_hooks_and_require_approval() {
        let dir = temp_dir();
        fs::write(
            dir.join("package.json"),
            r#"{"scripts":{"pretest":"prepare","test":"vitest","posttest":"cleanup"}}"#,
        )
        .unwrap();
        fs::write(dir.join("package-lock.json"), "{}").unwrap();
        let commands = detect_commands(&dir.to_string_lossy());
        let test = commands.iter().find(|cmd| cmd.category == "test").unwrap();
        assert_eq!(test.risk_level, "medium");
        assert!(test.requires_approval);
        assert!(test.expanded_command.contains("pretest: prepare"));
        assert!(test.expanded_command.contains("posttest: cleanup"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn rust_commands_require_approval() {
        let dir = temp_dir();
        fs::write(
            dir.join("Cargo.toml"),
            "[package]\nname='x'\nversion='0.1.0'\nedition='2021'\n",
        )
        .unwrap();
        let commands = detect_commands(&dir.to_string_lossy());
        assert!(commands.iter().all(|command| command.requires_approval));
        assert!(commands
            .iter()
            .all(|command| command.risk_level == "medium"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn resolve_command_rejects_arbitrary_shell_input() {
        let dir = temp_dir();
        fs::write(dir.join("package.json"), r#"{"scripts":{"test":"vitest"}}"#).unwrap();
        assert!(resolve_command(&dir.to_string_lossy(), "npm test").is_ok());
        assert!(resolve_command(&dir.to_string_lossy(), "echo unsafe").is_err());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn automatic_verification_rejects_repository_code() {
        let command = VerificationCommand {
            name: "npm test".into(),
            command: "npm test".into(),
            timeout_secs: 120,
            category: "test".into(),
            risk_level: "medium".into(),
            requires_approval: true,
            expanded_command: "test: vitest".into(),
            risk_explanation: "Repository script".into(),
        };
        assert!(ensure_automatic_command_allowed(&command).is_err());
    }

    #[test]
    fn captures_large_output_without_pipe_deadlock() {
        #[cfg(windows)]
        let command =
            "powershell -NoProfile -NonInteractive -Command \"1..20000 | ForEach-Object { 'abcdefghij' }\"";
        #[cfg(not(windows))]
        let command = "yes abcdefghij | head -n 20000";
        let result = run_verification(command, ".", 30);
        assert!(result.success, "{}", result.stderr);
        assert!(!result.stdout.is_empty());
        assert!(result.stdout.len() <= MAX_CAPTURE_BYTES);
    }

    #[test]
    fn cancellation_stops_a_running_command() {
        #[cfg(windows)]
        let command = "powershell -NoProfile -Command \"Start-Sleep -Seconds 10\"";
        #[cfg(not(windows))]
        let command = "sleep 10";
        let cancellation = Arc::new(AtomicBool::new(false));
        let signal = cancellation.clone();
        let thread =
            std::thread::spawn(move || run_verification_with_control(command, ".", 30, signal));
        std::thread::sleep(Duration::from_millis(250));
        cancellation.store(true, Ordering::SeqCst);
        let result = thread.join().unwrap();
        assert!(!result.success);
        assert!(result.stderr.contains("cancelled"));
        assert!(result.duration_ms < 5_000);
    }
}
