use std::io::{Read, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(60);
pub const DEFAULT_OUTPUT_LIMIT: usize = 2 * 1024 * 1024;

#[derive(Debug)]
pub struct ProcessOutput {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
    pub truncated: bool,
}

fn drain_bounded<R: Read + Send + 'static>(mut reader: R, limit: usize) -> thread::JoinHandle<(Vec<u8>, bool)> {
    thread::spawn(move || {
        let mut captured = Vec::with_capacity(limit.min(64 * 1024));
        let mut buffer = [0_u8; 8192];
        let mut truncated = false;
        loop {
            match reader.read(&mut buffer) {
                Ok(0) | Err(_) => break,
                Ok(read) => {
                    let remaining = limit.saturating_sub(captured.len());
                    let retained = remaining.min(read);
                    captured.extend_from_slice(&buffer[..retained]);
                    truncated |= retained < read;
                }
            }
        }
        (captured, truncated)
    })
}

#[cfg(windows)]
fn terminate_process_tree(process_id: u32) {
    let _ = Command::new("taskkill")
        .args(["/PID", &process_id.to_string(), "/T", "/F"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

#[cfg(not(windows))]
fn terminate_process_tree(_process_id: u32) {}

pub fn run(
    program: &str,
    args: &[&str],
    cwd: Option<&Path>,
    timeout: Duration,
    output_limit: usize,
) -> Result<ProcessOutput, String> {
    run_with_input(program, args, cwd, None, timeout, output_limit)
}

pub fn run_with_input(
    program: &str,
    args: &[&str],
    cwd: Option<&Path>,
    input: Option<&[u8]>,
    timeout: Duration,
    output_limit: usize,
) -> Result<ProcessOutput, String> {
    let mut command = Command::new(program);
    command
        .args(args)
        .stdin(if input.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(path) = cwd {
        command.current_dir(path);
    }

    let mut child = command
        .spawn()
        .map_err(|error| format!("Cannot start {program}: {error}"))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| format!("Cannot capture {program} stdout"))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| format!("Cannot capture {program} stderr"))?;
    let stdout_reader = drain_bounded(stdout, output_limit);
    let stderr_reader = drain_bounded(stderr, output_limit);
    let mut input_writer = input.map(|content| {
        let content = content.to_vec();
        let mut stdin = child.stdin.take();
        thread::spawn(move || -> Result<(), String> {
            let mut stdin = stdin
                .take()
                .ok_or_else(|| "Cannot open process stdin".to_string())?;
            stdin
                .write_all(&content)
                .map_err(|error| format!("Cannot write to process stdin: {error}"))
        })
    });
    let started = Instant::now();

    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if started.elapsed() < timeout => thread::sleep(Duration::from_millis(20)),
            Ok(None) => {
                terminate_process_tree(child.id());
                let _ = child.kill();
                let _ = child.wait();
                if let Some(writer) = input_writer.take() {
                    let _ = writer.join();
                }
                let _ = stdout_reader.join();
                let _ = stderr_reader.join();
                return Err(format!(
                    "{program} timed out after {} seconds",
                    timeout.as_secs()
                ));
            }
            Err(error) => {
                terminate_process_tree(child.id());
                let _ = child.kill();
                let _ = child.wait();
                if let Some(writer) = input_writer.take() {
                    let _ = writer.join();
                }
                let _ = stdout_reader.join();
                let _ = stderr_reader.join();
                return Err(format!("Cannot wait for {program}: {error}"));
            }
        }
    };

    if let Some(writer) = input_writer.take() {
        writer
            .join()
            .map_err(|_| format!("{program} stdin writer failed"))??;
    }

    let (stdout, stdout_truncated) = stdout_reader
        .join()
        .map_err(|_| format!("{program} stdout reader failed"))?;
    let (stderr, stderr_truncated) = stderr_reader
        .join()
        .map_err(|_| format!("{program} stderr reader failed"))?;
    Ok(ProcessOutput {
        success: status.success(),
        stdout: String::from_utf8_lossy(&stdout).into_owned(),
        stderr: String::from_utf8_lossy(&stderr).into_owned(),
        truncated: stdout_truncated || stderr_truncated,
    })
}

pub fn run_default(program: &str, args: &[&str], cwd: Option<&Path>) -> Result<ProcessOutput, String> {
    run(
        program,
        args,
        cwd,
        DEFAULT_TIMEOUT,
        DEFAULT_OUTPUT_LIMIT,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn captures_successful_output() {
        #[cfg(windows)]
        let output = run_default("cmd", &["/C", "echo", "atlasforge"], None).unwrap();
        #[cfg(not(windows))]
        let output = run_default("printf", &["atlasforge"], None).unwrap();
        assert!(output.success);
        assert!(output.stdout.contains("atlasforge"));
        assert!(!output.truncated);
    }

    #[test]
    fn bounds_captured_output() {
        #[cfg(windows)]
        let output = run(
            "cmd",
            &["/C", "for /L %i in (1,1,100) do @echo x"],
            None,
            DEFAULT_TIMEOUT,
            16,
        )
        .unwrap();
        #[cfg(not(windows))]
        let output = run(
            "sh",
            &["-c", "yes x | head -n 100"],
            None,
            DEFAULT_TIMEOUT,
            16,
        )
        .unwrap();
        assert!(output.success);
        assert!(output.stdout.len() <= 16);
        assert!(output.truncated);
    }
}
