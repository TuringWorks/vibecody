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
            r#"(version 1)
(deny default)
(allow process-exec
    (literal "/bin/sh")
    (literal "/bin/bash")
    (literal "/usr/bin/env")
    (subpath "/usr/bin")
    (subpath "/usr/local/bin")
    (subpath "/opt/homebrew/bin"))
(allow process-fork)
(allow process-signal (target self))
(allow file-read*
    (subpath "/usr")
    (subpath "/opt")
    (subpath "/Library/Developer")
    (subpath "/private/tmp")
    (subpath "/tmp"))
(allow file-read* (subpath "{workspace}"))
(allow file-write* (subpath "{workspace}"))
(allow file-write* (literal "/dev/null"))
(allow file-write* (subpath "/tmp"))
(allow file-write* (subpath "/private/tmp"))
(deny network*)
"#,
            workspace = workspace_root.display()
        );
        let profile_path = std::env::temp_dir().join(format!(
            "vibecli_sandbox_{}_{:016x}.sb",
            std::process::id(),
            rand::random::<u64>()
        ));
        std::fs::write(&profile_path, &profile)?;
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
            // Read-only bind of system dirs + read-write bind of workspace only
            return Ok(Command::new("bwrap")
                .args(["--ro-bind", "/usr", "/usr"])
                .args(["--ro-bind", "/lib", "/lib"])
                .args(["--ro-bind", "/lib64", "/lib64"])
                .args(["--ro-bind", "/bin", "/bin"])
                .args(["--ro-bind", "/etc/resolv.conf", "/etc/resolv.conf"])
                .args(["--bind", &ws, &ws])       // workspace: read-write
                .args(["--dev", "/dev"])
                .args(["--tmpfs", "/tmp"])
                .args(["--unshare-net"])           // no network access
                .args(["--unshare-pid"])           // PID namespace isolation
                .args(["--", "sh", "-c", command])
                .current_dir(cwd)
                .output()?);
        }
        tracing::warn!(command = %command, "bwrap not available — running without sandbox");
        Self::execute_in(command, cwd)
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    fn execute_sandboxed_impl(command: &str, cwd: &Path, _workspace_root: &Path) -> Result<Output> {
        Self::execute_in(command, cwd)
    }

    /// Returns true if the command appears safe (blocklist check).
    ///
    /// Normalizes whitespace and checks for dangerous patterns using regex
    /// to resist bypass via extra spaces, flag reordering, or quoting.
    pub fn is_safe_command(command: &str) -> bool {
        use std::sync::OnceLock;

        static DANGEROUS: OnceLock<Vec<regex::Regex>> = OnceLock::new();
        let patterns = DANGEROUS.get_or_init(|| {
            [
                // rm with recursive+force on root, home, or all
                r"rm\s+(-[a-zA-Z]*r[a-zA-Z]*f[a-zA-Z]*|-[a-zA-Z]*f[a-zA-Z]*r[a-zA-Z]*|--recursive\s+--force|--force\s+--recursive)\s+[/~]",
                // Windows del with force+recursive
                r"(?i)del\s+/[fFs]\s+/[sS]",
                // Disk format / mkfs
                r"(?i)(format\s+[a-z]:|mkfs[\s.])",
                // dd writing to disk devices
                r"dd\s+.*\bif=",
                // Fork bomb patterns
                r":\(\)\s*\{[^}]*\|\s*:.*\};?\s*:",
                // Direct write to block devices
                r">\s*/dev/(sd[a-z]|nvme|vd[a-z]|hd[a-z]|disk)",
                // chmod 777 on root
                r"chmod\s+(-[a-zA-Z]*R[a-zA-Z]*\s+)?777\s+/\s*$",
                // Wiping commands
                r"shred\s+.*\s+/",
            ]
            .iter()
            .filter_map(|p| regex::Regex::new(p).ok())
            .collect()
        });

        // Normalize whitespace (tabs, multiple spaces → single space)
        let normalized: String = command.split_whitespace().collect::<Vec<_>>().join(" ");
        !patterns.iter().any(|re| re.is_match(&normalized))
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
