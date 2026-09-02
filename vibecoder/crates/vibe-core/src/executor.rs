//! Command execution with safety checks and optional OS-level sandboxing.
//!
//! Commands arrive as POSIX shell strings -- from the agent, from a detected
//! build system, from a user -- so they run under [`crate::shell`] on every
//! platform, Windows included. This used to branch to `cmd /C` on Windows,
//! which parses those strings but does not mean the same thing by them: no
//! `>/dev/null`, no `$(...)`, different quoting. That fails quietly, with a
//! wrong result rather than an error.

use anyhow::Result;
use std::path::Path;
use std::process::Output;

/// A failure to *start* a command, reported so that "this machine has no
/// shell" reads differently from "that program does not exist". Both arrive
/// from the OS as the same `NotFound`, and only one of them is fixed by
/// installing anything. The original error stays as the source.
fn spawn_failed(error: std::io::Error) -> anyhow::Error {
    let explanation = crate::shell::explain(&error);
    anyhow::Error::new(error).context(explanation)
}

pub struct CommandExecutor;

impl CommandExecutor {
    /// Execute a shell command, returning stdout + stderr.
    pub fn execute(command: &str) -> Result<Output> {
        crate::shell::sh(command).output().map_err(spawn_failed)
    }

    /// Execute a shell command with an optional working directory.
    pub fn execute_in(command: &str, cwd: &Path) -> Result<Output> {
        crate::shell::sh(command)
            .current_dir(cwd)
            .output()
            .map_err(spawn_failed)
    }

    /// Execute a shell command, killing it once it goes quiet or outlives the
    /// hard cap.
    ///
    /// `execute_in` waits forever, so an agent that starts a server — `python3
    /// server.py`, `npm run dev` — pins the whole run: the tool call never
    /// returns, the loop never takes another turn, and every watchdog that
    /// checks between turns becomes unreachable. Observed directly: greenfield
    /// runs that had already built a complete, working application sat until
    /// the harness killed them, four times in five.
    ///
    /// The bound is on **idle time, not total time**, because that is what
    /// actually separates the two cases. A build or a test suite emits output
    /// continuously; a server prints its startup line and then goes silent. A
    /// flat total limit has to choose between killing real builds (this
    /// workspace's `cargo build` takes nine minutes) and leaving a hang to
    /// burn most of a run. Idle time needs no such compromise. `hard_cap`
    /// remains as a backstop for a command that is both endless and chatty.
    pub fn execute_in_bounded(
        command: &str,
        cwd: &Path,
        idle_limit: std::time::Duration,
        hard_cap: std::time::Duration,
    ) -> Result<Output> {
        use std::io::Read;
        use std::sync::atomic::{AtomicU64, Ordering};
        use std::sync::{Arc, Mutex};

        let mut child = crate::shell::sh(command)
            .current_dir(cwd)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(spawn_failed)?;

        let started = std::time::Instant::now();
        // Milliseconds since `started` at which output was last seen.
        let last_output_ms = Arc::new(AtomicU64::new(0));
        let stdout_buf = Arc::new(Mutex::new(Vec::new()));
        let stderr_buf = Arc::new(Mutex::new(Vec::new()));

        let mut readers = Vec::new();
        for (pipe, buf) in [
            (
                child
                    .stdout
                    .take()
                    .map(|p| Box::new(p) as Box<dyn Read + Send>),
                Arc::clone(&stdout_buf),
            ),
            (
                child
                    .stderr
                    .take()
                    .map(|p| Box::new(p) as Box<dyn Read + Send>),
                Arc::clone(&stderr_buf),
            ),
        ] {
            let Some(mut pipe) = pipe else { continue };
            let stamp = Arc::clone(&last_output_ms);
            readers.push(std::thread::spawn(move || {
                let mut chunk = [0u8; 8192];
                loop {
                    match pipe.read(&mut chunk) {
                        Ok(0) | Err(_) => return,
                        Ok(n) => {
                            stamp.store(started.elapsed().as_millis() as u64, Ordering::Relaxed);
                            if let Ok(mut b) = buf.lock() {
                                b.extend_from_slice(&chunk[..n]);
                            }
                        }
                    }
                }
            }));
        }

        let collect = |stdout_buf: &Arc<Mutex<Vec<u8>>>, stderr_buf: &Arc<Mutex<Vec<u8>>>| {
            (
                stdout_buf.lock().map(|b| b.clone()).unwrap_or_default(),
                stderr_buf.lock().map(|b| b.clone()).unwrap_or_default(),
            )
        };

        loop {
            if let Some(status) = child.try_wait()? {
                for r in readers {
                    let _ = r.join();
                }
                let (stdout, stderr) = collect(&stdout_buf, &stderr_buf);
                return Ok(Output {
                    status,
                    stdout,
                    stderr,
                });
            }

            let idle = started
                .elapsed()
                .saturating_sub(std::time::Duration::from_millis(
                    last_output_ms.load(Ordering::Relaxed),
                ));
            let expired = idle >= idle_limit || started.elapsed() >= hard_cap;
            if expired {
                let _ = child.kill();
                let _ = child.wait();
                let (stdout, mut stderr) = collect(&stdout_buf, &stderr_buf);
                stderr.extend_from_slice(
                    format!(
                        "\n[command produced no output for {}s and was terminated after {}s. \
                         If this is a server or another long-running process, start it in the \
                         background instead — `<command> &` — so the session can continue.]",
                        idle.as_secs(),
                        started.elapsed().as_secs()
                    )
                    .as_bytes(),
                );
                // Non-zero: reporting success here would tell the agent its
                // server had exited cleanly.
                return Ok(Output {
                    status: failed_status(),
                    stdout,
                    stderr,
                });
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
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
        let out = vibe_no_window::std_command("sandbox-exec")
            .arg("-f")
            .arg(&profile_path)
            .arg("sh")
            .arg("-c")
            .arg(command)
            .current_dir(cwd)
            .output();
        let _ = std::fs::remove_file(&profile_path);
        Ok(out?)
    }

    #[cfg(target_os = "linux")]
    fn execute_sandboxed_impl(command: &str, cwd: &Path, workspace_root: &Path) -> Result<Output> {
        let bwrap_ok = vibe_no_window::std_command("bwrap")
            .arg("--version")
            .output()
            .is_ok();
        if bwrap_ok {
            let ws = workspace_root.display().to_string();
            // Read-only bind of system dirs + read-write bind of workspace only
            return Ok(vibe_no_window::std_command("bwrap")
                .args(["--ro-bind", "/usr", "/usr"])
                .args(["--ro-bind", "/lib", "/lib"])
                .args(["--ro-bind", "/lib64", "/lib64"])
                .args(["--ro-bind", "/bin", "/bin"])
                .args(["--ro-bind", "/etc/resolv.conf", "/etc/resolv.conf"])
                .args(["--bind", &ws, &ws]) // workspace: read-write
                .args(["--dev", "/dev"])
                .args(["--tmpfs", "/tmp"])
                .args(["--unshare-net"]) // no network access
                .args(["--unshare-pid"]) // PID namespace isolation
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
                // Shutdown/reboot
                r"(?i)\b(poweroff|reboot|halt|shutdown)\b",
                // Reverse shells and exfiltration
                r"/dev/tcp/",
                r"(?i)\bnc\s+.*-e\b",
                r"(?i)\bncat\s+.*-e\b",
                // Encoded execution
                r"base64\s+-d\s*\|\s*sh",
                r"base64\s+-d\s*\|\s*bash",
                // crontab persistence
                r"(?i)\bcrontab\b",
                // Firewall manipulation
                r"(?i)\biptables\b",
                r"(?i)\bufw\b",
                // curl/wget POST exfiltration
                r"curl\s+.*-d\s",
                r"wget\s+.*--post-data",
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

    /// Combine stdout and stderr from an `Output` into a single lossless string.
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

#[cfg(test)]
mod tests {
    use super::*;

    /// An `ExitStatus` carrying `code`, for the output-formatting tests.
    ///
    /// The raw value is platform-encoded -- a wait status on Unix, where the
    /// exit code sits in the high byte, and the exit code itself on Windows.
    /// The two constructors take different types and different numbers, so a
    /// shared literal is not possible; this crate's tests used the Unix one
    /// unconditionally and therefore did not compile on Windows at all.
    fn exit_status(code: i32) -> std::process::ExitStatus {
        #[cfg(unix)]
        {
            use std::os::unix::process::ExitStatusExt;
            std::process::ExitStatus::from_raw(code << 8)
        }
        #[cfg(windows)]
        {
            use std::os::windows::process::ExitStatusExt;
            std::process::ExitStatus::from_raw(code as u32)
        }
    }

    // ── is_safe_command ──────────────────────────────────────────────────

    #[test]
    fn safe_command_allows_normal_commands() {
        assert!(CommandExecutor::is_safe_command("ls -la"));
        assert!(CommandExecutor::is_safe_command("cargo test"));
        assert!(CommandExecutor::is_safe_command("cat /etc/hosts"));
        assert!(CommandExecutor::is_safe_command("git status"));
        assert!(CommandExecutor::is_safe_command("echo hello"));
    }

    #[test]
    fn safe_command_blocks_rm_rf_root() {
        assert!(!CommandExecutor::is_safe_command("rm -rf /"));
        assert!(!CommandExecutor::is_safe_command("rm -rf ~/"));
        assert!(!CommandExecutor::is_safe_command("rm  -rf  /")); // extra spaces
    }

    #[test]
    fn safe_command_blocks_rm_fr_root() {
        // flag reordering: -fr instead of -rf
        assert!(!CommandExecutor::is_safe_command("rm -fr /"));
    }

    #[test]
    fn safe_command_blocks_fork_bomb() {
        assert!(!CommandExecutor::is_safe_command(":(){ :|:& };:"));
    }

    #[test]
    fn safe_command_blocks_mkfs() {
        assert!(!CommandExecutor::is_safe_command("mkfs.ext4 /dev/sda1"));
    }

    #[test]
    fn safe_command_blocks_dd() {
        assert!(!CommandExecutor::is_safe_command(
            "dd if=/dev/zero of=/dev/sda"
        ));
    }

    #[test]
    fn safe_command_blocks_write_to_device() {
        assert!(!CommandExecutor::is_safe_command("echo bad > /dev/sda"));
    }

    #[test]
    fn safe_command_blocks_chmod_777_root() {
        assert!(!CommandExecutor::is_safe_command("chmod -R 777 /"));
    }

    #[test]
    fn safe_command_blocks_shred() {
        assert!(!CommandExecutor::is_safe_command("shred -vfz /important"));
    }

    #[test]
    fn safe_command_allows_rm_single_file() {
        // Non-recursive rm on a specific file is OK
        assert!(CommandExecutor::is_safe_command("rm file.txt"));
        assert!(CommandExecutor::is_safe_command("rm -f file.txt"));
    }

    // ── execute ──────────────────────────────────────────────────────────

    #[test]
    fn execute_simple_command() {
        let output = CommandExecutor::execute("echo hello").unwrap();
        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("hello"));
    }

    #[test]
    fn execute_in_specific_dir() {
        // Not a hardcoded `/tmp`: `current_dir` is set through the OS, not the
        // shell, so a POSIX path is not a valid argument on Windows and the
        // spawn fails outright. Nor the temp dir itself -- MSYS reports that as
        // `/tmp` whatever Windows calls it. A directory we create ourselves has
        // a name that survives the translation.
        let parent = tempfile::tempdir().expect("temp dir");
        let dir = parent.path().join("vibecody-cwd-probe");
        std::fs::create_dir(&dir).expect("create probe dir");
        let output = CommandExecutor::execute_in("pwd", &dir).unwrap();
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("vibecody-cwd-probe"),
            "pwd printed {stdout:?}"
        );
    }

    // ── execute_with_approval ────────────────────────────────────────────

    #[test]
    fn execute_with_approval_blocks_dangerous() {
        let result = CommandExecutor::execute_with_approval("rm -rf /", false);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("approval"));
    }

    #[test]
    fn execute_with_approval_allows_dangerous_when_approved() {
        // With auto_approve=true, even dangerous commands pass the gate
        // (but rm -rf / would still fail at the OS level)
        // We test with a safe command to verify the approval path
        let result = CommandExecutor::execute_with_approval("echo ok", true);
        assert!(result.is_ok());
    }

    // ── output_to_string ─────────────────────────────────────────────────

    #[test]
    fn output_to_string_stdout_only() {
        let output = Output {
            status: exit_status(0),
            stdout: b"hello\n".to_vec(),
            stderr: vec![],
        };
        let s = CommandExecutor::output_to_string(&output);
        assert_eq!(s, "hello\n");
    }

    #[test]
    fn output_to_string_stderr_only() {
        let output = Output {
            status: exit_status(1),
            stdout: vec![],
            stderr: b"error\n".to_vec(),
        };
        let s = CommandExecutor::output_to_string(&output);
        assert_eq!(s, "error\n");
    }

    #[test]
    fn output_to_string_both() {
        let output = Output {
            status: exit_status(0),
            stdout: b"out\n".to_vec(),
            stderr: b"err\n".to_vec(),
        };
        let s = CommandExecutor::output_to_string(&output);
        assert!(s.contains("out\n"));
        assert!(s.contains("--- stderr ---"));
        assert!(s.contains("err\n"));
    }

    #[test]
    fn output_to_string_empty() {
        let output = Output {
            status: exit_status(0),
            stdout: vec![],
            stderr: vec![],
        };
        let s = CommandExecutor::output_to_string(&output);
        assert_eq!(s, "[no output]");
    }
}

/// A non-zero `ExitStatus` for commands we terminated ourselves.
fn failed_status() -> std::process::ExitStatus {
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        std::process::ExitStatus::from_raw(9)
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::ExitStatusExt;
        // Any non-zero code; the message in stderr carries the detail. This
        // used to run `cmd /C exit 1` to manufacture one, which spawned a
        // whole process -- and, from a GUI, a console window -- for a value
        // that can simply be constructed.
        std::process::ExitStatus::from_raw(1)
    }
    #[cfg(not(any(unix, windows)))]
    {
        // No portable constructor. Success is the wrong answer here, but it
        // is the one the old `unwrap_or_default()` also gave, and no target
        // we ship reaches this arm.
        std::process::ExitStatus::default()
    }
}

#[cfg(test)]
mod bounded_tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn a_command_that_finishes_returns_its_output() {
        let out = CommandExecutor::execute_in_bounded(
            "echo hello",
            std::path::Path::new("."),
            Duration::from_secs(10),
            Duration::from_secs(30),
        )
        .expect("should run");
        assert!(out.status.success());
        assert!(String::from_utf8_lossy(&out.stdout).contains("hello"));
    }

    #[test]
    fn a_silent_command_is_killed_and_reported_as_failed() {
        // The shape that pinned entire agent runs: a server started in the
        // foreground never returns, so the tool call never returned either.
        let start = std::time::Instant::now();
        let out = CommandExecutor::execute_in_bounded(
            "sleep 30",
            std::path::Path::new("."),
            Duration::from_secs(1),
            Duration::from_secs(60),
        )
        .expect("should return rather than hang");
        assert!(
            start.elapsed() < Duration::from_secs(10),
            "must not wait for the child"
        );
        assert!(
            !out.status.success(),
            "an abandoned command is not a success"
        );
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(stderr.contains("no output"), "{stderr}");
        assert!(
            stderr.contains("background"),
            "should tell the agent what to do instead"
        );
    }

    #[test]
    fn a_slow_but_talking_command_is_not_killed() {
        // The whole reason the bound is on idle time rather than total time: a
        // build or test suite runs long *and* keeps printing. A flat total
        // limit would have to choose between killing those and letting a hang
        // burn most of a run.
        let out = CommandExecutor::execute_in_bounded(
            // Integer seconds: fractional `sleep` is a GNU extension, and the
            // `sleep` that Windows resolves rejects it outright. Three ticks a
            // second apart still outlive the two-second idle bound below.
            "for i in 1 2 3; do echo tick; sleep 1; done",
            std::path::Path::new("."),
            Duration::from_secs(2),
            Duration::from_secs(60),
        )
        .expect("should run");
        assert!(
            out.status.success(),
            "a command that keeps producing output must survive an idle bound: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert_eq!(
            String::from_utf8_lossy(&out.stdout).matches("tick").count(),
            3
        );
    }

    #[test]
    fn an_endless_but_chatty_command_still_hits_the_hard_cap() {
        let start = std::time::Instant::now();
        let out = CommandExecutor::execute_in_bounded(
            "while true; do echo noise; sleep 0.1; done",
            std::path::Path::new("."),
            Duration::from_secs(30),
            Duration::from_secs(2),
        )
        .expect("should return");
        assert!(start.elapsed() < Duration::from_secs(20));
        assert!(!out.status.success());
    }
}
