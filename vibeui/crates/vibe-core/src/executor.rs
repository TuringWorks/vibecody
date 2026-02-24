//! Command execution with safety checks and optional OS-level sandboxing.

use anyhow::Result;
use std::path::Path;
use std::process::{Command, Output};

pub struct CommandExecutor;

impl CommandExecutor {
    /// Execute a shell command, returning stdout + stderr.
    pub fn execute(command: &str) -> Result<Output> {
        let output = if cfg!(target_os = "windows") {
            Command::new("cmd").args(["/C", command]).output()?
        } else {
            Command::new("sh").arg("-c").arg(command).output()?
        };
        Ok(output)
    }

    /// Execute a shell command with an optional working directory.
    pub fn execute_in(command: &str, cwd: &Path) -> Result<Output> {
        let output = if cfg!(target_os = "windows") {
            Command::new("cmd")
                .args(["/C", command])
                .current_dir(cwd)
                .output()?
        } else {
            Command::new("sh")
                .arg("-c")
                .arg(command)
                .current_dir(cwd)
                .output()?
        };
        Ok(output)
    }

    /// Execute inside an OS-level sandbox when possible.
    ///
    /// - **macOS**: uses `sandbox-exec` with a restrictive profile that denies
    ///   network access and limits filesystem writes to the provided `workspace_root`.
    /// - **Linux**: wraps in `bwrap` (bubblewrap) if available.
    /// - **Other / unavailable**: falls back to `execute_in` with a warning.
    pub fn execute_sandboxed(command: &str, cwd: &Path, workspace_root: &Path) -> Result<Output> {
        Self::execute_sandboxed_impl(command, cwd, workspace_root)
    }

    #[cfg(target_os = "macos")]
    fn execute_sandboxed_impl(command: &str, cwd: &Path, workspace_root: &Path) -> Result<Output> {
        let profile = format!(
            "(version 1)\n\
             (deny default)\n\
             (allow process*)\n\
             (allow file-read*)\n\
             (allow file-write* (subpath \"{workspace}\"))\n\
             (allow file-write* (literal \"/dev/null\"))\n\
             (deny network*)\n",
            workspace = workspace_root.display()
        );
        let profile_path = std::env::temp_dir().join("vibecli_sandbox.sb");
        std::fs::write(&profile_path, profile)?;
        let out = Command::new("sandbox-exec")
            .arg("-f").arg(&profile_path)
            .arg("sh").arg("-c").arg(command)
            .current_dir(cwd)
            .output();
        let _ = std::fs::remove_file(&profile_path);
        Ok(out?)
    }

    #[cfg(target_os = "linux")]
    fn execute_sandboxed_impl(command: &str, cwd: &Path, workspace_root: &Path) -> Result<Output> {
        let bwrap_ok = Command::new("bwrap").arg("--version").output().is_ok();
        if bwrap_ok {
            let ws = workspace_root.display().to_string();
            return Ok(Command::new("bwrap")
                .args(["--bind", "/", "/", "--dev", "/dev", "--bind", &ws, &ws, "--unshare-net", "sh", "-c", command])
                .current_dir(cwd)
                .output()?);
        }
        tracing::warn!("bwrap not available — running without sandbox: {}", command);
        Self::execute_in(command, cwd)
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    fn execute_sandboxed_impl(command: &str, cwd: &Path, _workspace_root: &Path) -> Result<Output> {
        Self::execute_in(command, cwd)
    }

    /// Returns true if the command appears safe (basic blocklist).
    pub fn is_safe_command(command: &str) -> bool {
        let dangerous = [
            "rm -rf /",
            "rm -rf ~",
            "del /f /s",
            "format c:",
            "mkfs",
            "dd if=",
            ":(){ :|:& };:",
            "> /dev/sda",
        ];
        !dangerous.iter().any(|d| command.contains(d))
    }

    /// Execute with an optional approval gate. Returns an error if the command
    /// matches the dangerous pattern and `auto_approve` is false.
    pub fn execute_with_approval(command: &str, auto_approve: bool) -> Result<Output> {
        if !Self::is_safe_command(command) && !auto_approve {
            anyhow::bail!("Command requires manual approval: {}", command);
        }
        Self::execute(command)
    }

    /// Combine stdout and stderr from an `Output` into a single string.
    pub fn output_to_string(output: &Output) -> String {
        let mut result = String::new();
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        if !stdout.is_empty() {
            result.push_str(&stdout);
        }
        if !stderr.is_empty() {
            if !result.is_empty() {
                result.push_str("\n--- stderr ---\n");
            }
            result.push_str(&stderr);
        }
        if result.is_empty() {
            result.push_str("[no output]");
        }
        result
    }
}
