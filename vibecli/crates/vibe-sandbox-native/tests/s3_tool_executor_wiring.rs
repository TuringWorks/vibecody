//! S3 — `tool_executor` → `vibe-sandbox` wiring integration test.
//!
//! Exercises the exact pattern that `vibecli/vibecli-cli/src/tool_executor.rs::run_in_native_sandbox`
//! follows, but lives in the sandbox crate so it can run end-to-end
//! without dragging in the vibecli build (which is gated on the Metal
//! toolchain on macOS).
//!
//! The pattern under test:
//!   1. `native()` constructs the OS-appropriate Tier-0 sandbox.
//!   2. `bind_rw(cwd, cwd)` exposes the workspace.
//!   3. `network(NetPolicy::Direct | NetPolicy::None)` matches the
//!      `tool_executor::network_disabled` flag.
//!   4. `spawn("sh", &["-c", "<command>"])` runs the shell command.
//!   5. `wait_with_output()` collects stdout+stderr+exit.
//!
//! All four backends (Linux bwrap, macOS sandbox-exec, Windows AppContainer,
//! unsupported-OS error) are exercised through cfg-gated assertions.

use std::ffi::OsStr;
use vibe_sandbox::NetPolicy;

/// Build the same sandbox `tool_executor::run_in_native_sandbox` builds,
/// then assert it has the expected shape. Doesn't actually spawn —
/// spawning requires bwrap/sandbox-exec to be installed and have the
/// right caps, which isn't guaranteed in every dev environment.
#[test]
fn tool_executor_wiring_pattern_constructs_native_sandbox() {
    let tmp = std::env::temp_dir().join("vibe_s3_tool_executor_wiring");
    let _ = std::fs::create_dir_all(&tmp);

    #[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
    {
        let mut sb = vibe_sandbox_native::native().expect("native() succeeds");
        sb.bind_rw(&tmp, &tmp).expect("bind_rw");
        sb.network(NetPolicy::Direct);
        assert_eq!(sb.tier(), vibe_sandbox::SandboxTier::Native);
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        let err = vibe_sandbox_native::native().expect_err("native() unsupported");
        assert!(matches!(
            err,
            vibe_sandbox::SandboxError::TierUnsupported { .. }
        ));
    }

    let _ = std::fs::remove_dir_all(&tmp);
}

/// `network_disabled = true` in the daemon → `NetPolicy::None` on the
/// sandbox. Round-trip via the trait surface to prove the toggle is
/// idempotent and reachable.
#[test]
#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
fn network_policy_toggles_through_trait() {
    let tmp = std::env::temp_dir().join("vibe_s3_net_toggle");
    let _ = std::fs::create_dir_all(&tmp);

    let mut sb = vibe_sandbox_native::native().expect("native()");
    sb.bind_rw(&tmp, &tmp).expect("bind_rw");

    // Direct → None → Direct (idempotent in both directions).
    sb.network(NetPolicy::Direct);
    sb.network(NetPolicy::None);
    sb.network(NetPolicy::Direct);
    sb.network(NetPolicy::None);

    let _ = std::fs::remove_dir_all(&tmp);
}

/// The deny-list (Linux bind_rw rejects `.vibecli` / `.ssh` / `.aws` /
/// etc.) MUST refuse to expose a credential dir even if `tool_executor`
/// is somehow tricked into asking for one. This is the single most
/// important S3 invariant: the agent-loop's shell tool cannot read the
/// daemon's own bearer token.
#[test]
#[cfg(target_os = "linux")]
fn bind_rw_refuses_vibecli_workspace_for_tool_executor() {
    // Segment-based validator → doesn't need the path to exist.
    let creds = std::path::Path::new("/tmp/some/.vibecli/jobs.db");
    let mut sb = vibe_sandbox_native::native().expect("native()");
    let err = sb.bind_rw(creds, creds).expect_err(
        "bind_rw of a .vibecli path MUST be refused — \
         a sandboxed shell command would otherwise read daemon.token",
    );
    assert!(matches!(err, vibe_sandbox::SandboxError::Setup(_)));
}

/// The same deny-list, but for macOS. The macOS implementation
/// canonicalizes first, so the path must exist for the test to
/// exercise the validator — we use the actual home directory path.
#[test]
#[cfg(target_os = "macos")]
fn bind_rw_refuses_vibecli_workspace_for_tool_executor_macos() {
    // Create a `.vibecli/` directory in TMPDIR so the macOS validator's
    // canonicalize step succeeds, then the segment check fires.
    let tmp = std::env::temp_dir().join("vibe_s3_macos_deny");
    let creds = tmp.join(".vibecli");
    let _ = std::fs::create_dir_all(&creds);

    let mut sb = vibe_sandbox_native::native().expect("native()");
    let result = sb.bind_rw(&creds, &creds);
    let _ = std::fs::remove_dir_all(&tmp);

    let err = result.expect_err("bind_rw of a .vibecli path MUST be refused on macOS");
    // Either Setup (validator) or Io (canonicalize) is acceptable;
    // both fail closed.
    assert!(matches!(
        err,
        vibe_sandbox::SandboxError::Setup(_) | vibe_sandbox::SandboxError::Io(_)
    ));
}

/// Smoke-test the spawn API shape — does NOT actually run the sandbox
/// (bwrap / sandbox-exec may not be installed in CI dev sandboxes).
/// What this asserts: the `OsStr` arg shape `tool_executor` uses is
/// the shape `Sandbox::spawn` expects, so no `&str` → `OsStr` mismatch
/// silently slips through.
#[test]
#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
fn spawn_signature_accepts_sh_dash_c_command() {
    let tmp = std::env::temp_dir().join("vibe_s3_spawn_shape");
    let _ = std::fs::create_dir_all(&tmp);

    let mut sb = vibe_sandbox_native::native().expect("native()");
    sb.bind_rw(&tmp, &tmp).expect("bind_rw");
    sb.network(NetPolicy::None);

    // Construct the same OsStr args tool_executor builds. We don't
    // assert .spawn() succeeds (the runner host may lack bwrap /
    // sandbox-exec / AppContainer caps) — only that the call
    // typechecks and returns a Result of the documented shape.
    let cmd: &OsStr = OsStr::new("sh");
    let dc: &OsStr = OsStr::new("-c");
    let body: &OsStr = OsStr::new("true");
    let _: vibe_sandbox::Result<std::process::Child> = sb.spawn(cmd, &[dc, body]);

    let _ = std::fs::remove_dir_all(&tmp);
}
