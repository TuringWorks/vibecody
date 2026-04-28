//! Entry point for the new `vibe-sandbox` stack.
//!
//! Slice **N4-light**: this module exposes a small, opt-in helper that
//! routes a single command through the new tier-0 native sandbox. The
//! existing `tool_executor.rs::with_no_network` path is unchanged; call
//! sites adopt this entry point one at a time so the migration is
//! reviewable.
//!
//! Wiring details for each tier live in `docs/design/sandbox-tiers/`.

use std::path::Path;
use std::process::Output;

use vibe_sandbox::{NetPolicy, ResourceLimits, SandboxTier, SelectOptions, select};

#[derive(Debug, thiserror::Error)]
pub enum SandboxRunError {
    #[error("sandbox setup: {0}")]
    Setup(String),
    #[error("sandbox spawn: {0}")]
    Spawn(String),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("native sandbox not available on this platform")]
    Unavailable,
}

#[derive(Debug, Clone)]
pub struct SandboxRunOptions<'a> {
    pub host_dir: &'a Path,
    pub guest_dir: &'a Path,
    pub net: NetPolicy,
    pub limits: ResourceLimits,
    pub policy_id: &'a str,
}

impl<'a> SandboxRunOptions<'a> {
    pub fn new(host_dir: &'a Path, policy_id: &'a str) -> Self {
        SandboxRunOptions {
            host_dir,
            guest_dir: host_dir,
            net: NetPolicy::None,
            limits: ResourceLimits::default(),
            policy_id,
        }
    }
}

/// Run a single command inside the native Tier-0 sandbox with the host
/// directory bound rw at `guest_dir`. Returns the captured `Output` (stdout,
/// stderr, exit status). Fails cleanly on platforms without a native
/// implementation.
///
/// This is intentionally narrow — call sites that need streaming stdio,
/// signal handling, or cross-tier behaviour go through the full
/// `vibe_sandbox::Sandbox` trait directly.
pub fn run_in_sandbox(
    cmd: &str,
    args: &[&str],
    opts: SandboxRunOptions<'_>,
) -> Result<Output, SandboxRunError> {
    let mut sandbox = select(SandboxTier::Native, &SelectOptions::default())
        .map_err(|e| SandboxRunError::Setup(e.to_string()))?
        .into_sandbox();
    sandbox
        .bind_rw(opts.host_dir, opts.guest_dir)
        .map_err(|e| SandboxRunError::Setup(e.to_string()))?;
    sandbox.network(opts.net.clone());
    sandbox.limits(opts.limits.clone());

    // Native macOS impl provides a typed `run_capture` helper that returns
    // a captured Output without exposing tokio. Linux impl is wired the
    // same way in slice N1.4. For now we go through the trait's `spawn` +
    // wait, which works on every platform.
    let cmd_os: std::ffi::OsString = cmd.into();
    let arg_owned: Vec<std::ffi::OsString> = args.iter().map(|a| (*a).into()).collect();
    let arg_refs: Vec<&std::ffi::OsStr> = arg_owned.iter().map(|s| s.as_os_str()).collect();
    let child = sandbox
        .spawn(&cmd_os, &arg_refs)
        .map_err(|e| SandboxRunError::Spawn(e.to_string()))?;
    let output = child.wait_with_output()?;
    let _ = opts.policy_id; // hooked into broker audit in slice B6.
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[cfg(target_os = "macos")]
    #[test]
    fn run_echo_in_sandbox_returns_output() {
        let tmp = tempfile::tempdir().unwrap();
        let host = tmp.path().to_owned();
        let guest = PathBuf::from("/work");
        let opts = SandboxRunOptions {
            host_dir: &host,
            guest_dir: &guest,
            net: NetPolicy::None,
            limits: ResourceLimits::default(),
            policy_id: "test:echo",
        };
        let out = run_in_sandbox("/bin/sh", &["-c", "echo hello"], opts).unwrap();
        assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));
        assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "hello");
    }
}
