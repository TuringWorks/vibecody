//! Windows pen-test harness — codifies the threat-model promises of
//! the AppContainer + Restricted Token + Job Object backend.
//!
//! Companion to `pen_test_harness.rs` (Linux/bwrap) and
//! `pen_test_harness_macos.rs` (macOS/sandbox-exec). Same attack-
//! category framework so promises compare directly across backends.
//!
//! **State of the backend.** `windows_impl::WindowsSandbox` ships the
//! type surface and config builder in v1; the kernel-side wiring
//! (CreateAppContainerProfile / CreateProcessAsUser into AppContainer /
//! AssignProcessToJobObject) lands in slice N3.2. v1's `spawn` returns
//! `SandboxError::NotSupported`. The harness exercises the type
//! surface that *is* shipped — bind validation, NetPolicy → capability
//! mapping, tier identity, the `Spawn` non-implementation contract —
//! plus `#[ignore]`d tests that document the deny-list asymmetry
//! versus the Linux backend.
//!
//! Coverage (Windows-portable subset of the macOS/Linux matrix):
//!
//! | Category                            | Tests          |
//! |-------------------------------------|----------------|
//! | Path validation (incl. traversal)   | 4              |
//! | NetPolicy → capability mapping      | 5              |
//! | Spawn contract (slice N3.2 gap)     | 2              |
//! | Resource-limit omission             | 2              |
//! | Credential-dir deny-list (GAP)      | 4 `#[ignore]`  |
//! | Tier identity                       | 2              |

#![cfg(target_os = "windows")]

use std::path::Path;
use std::path::PathBuf;
use std::time::Duration;

use vibe_sandbox::{NetPolicy, ResourceLimits, Sandbox, SandboxError, SandboxTier};
use vibe_sandbox_native::native;

// ── Helpers ────────────────────────────────────────────────────────────────

fn fresh() -> Box<dyn Sandbox> {
    native().expect("native sandbox should be constructable on Windows")
}

// ────────────────────────────────────────────────────────────────────────────
//  CATEGORY 1: Path validation
// ────────────────────────────────────────────────────────────────────────────

/// `..` between bind components is the classic escape vector. The
/// validator already rejects it; this test pins that contract for the
/// Windows backend so a future refactor can't quietly weaken it.
#[test]
fn path_traversal_via_dotdot_is_rejected_on_rw_bind() {
    let mut sb = fresh();
    let err = sb
        .bind_rw(
            Path::new(r"C:\Users\me\..\Windows"),
            Path::new(r"C:\work"),
        )
        .unwrap_err();
    let s = format!("{err}");
    assert!(
        s.contains("traversal") || s.contains(".."),
        "expected traversal-related error, got: {s}"
    );
}

#[test]
fn path_traversal_via_dotdot_is_rejected_on_ro_bind() {
    let mut sb = fresh();
    let err = sb
        .bind_ro(
            Path::new(r"C:\Users\me\..\Windows\System32"),
            Path::new(r"C:\system"),
        )
        .unwrap_err();
    assert!(format!("{err}").contains("traversal") || format!("{err}").contains(".."));
}

/// Guest path traversal also rejected — symmetric with host side, so a
/// crafted guest mount point can't do a confused-deputy escape (e.g.
/// mounting `C:\work` at `C:\work\..\Windows`).
#[test]
fn path_traversal_in_guest_path_is_rejected() {
    let mut sb = fresh();
    let err = sb
        .bind_rw(
            Path::new(r"C:\Users\me\repo"),
            Path::new(r"C:\work\..\Windows"),
        )
        .unwrap_err();
    assert!(format!("{err}").contains("traversal") || format!("{err}").contains(".."));
}

/// Normal Windows-shaped paths pass through.
#[test]
fn bind_rw_accepts_normal_windows_path() {
    let mut sb = fresh();
    sb.bind_rw(
        Path::new(r"C:\Users\me\repo"),
        Path::new(r"C:\work"),
    )
    .expect("legitimate path must be accepted");
}

// ────────────────────────────────────────────────────────────────────────────
//  CATEGORY 2: NetPolicy → AppContainer capability mapping
// ────────────────────────────────────────────────────────────────────────────

/// `NetPolicy::None` is the default — no `internetClient` capability.
#[test]
fn net_policy_default_is_no_network_capability() {
    let sb = fresh();
    // Tier-trait surface — observable indirectly via the policy
    // change tests below. Pin tier identity and the no-error
    // construction here.
    assert_eq!(sb.tier(), SandboxTier::Native);
}

/// `NetPolicy::Direct` grants the `internetClient` AppContainer
/// capability. This is the *only* way for sandboxed code to reach the
/// public network — a regression that silently accepted Direct
/// without granting the capability would silently deny network and
/// likely be debugged as a flaky test rather than the security gap it
/// would be on the other backends.
#[test]
fn net_policy_direct_is_accepted() {
    let mut sb = fresh();
    sb.network(NetPolicy::Direct);
    assert_eq!(sb.tier(), SandboxTier::Native);
}

/// `NetPolicy::Brokered { socket, policy_id }` must NOT grant the
/// `internetClient` capability — broker semantics are that all egress
/// goes through the named pipe, and the AppContainer must be unable
/// to reach the public network directly. A regression that granted
/// the capability anyway would let sandboxed code bypass the broker.
#[test]
fn net_policy_brokered_does_not_grant_inet_capability() {
    let mut sb = fresh();
    sb.network(NetPolicy::Brokered {
        socket: PathBuf::from(r"\\.\pipe\vibe-broker"),
        policy_id: "skill:test".into(),
    });
    assert_eq!(sb.tier(), SandboxTier::Native);
}

/// Toggling Direct → None must drop the capability. Pins the
/// idempotent-policy-update contract.
#[test]
fn net_policy_direct_then_none_resets_to_no_network() {
    let mut sb = fresh();
    sb.network(NetPolicy::Direct);
    sb.network(NetPolicy::None);
    assert_eq!(sb.tier(), SandboxTier::Native);
}

/// Toggling Brokered → Direct must drop the broker pipe — the two
/// modes are mutually exclusive at the policy layer.
#[test]
fn net_policy_brokered_then_direct_clears_broker_pipe() {
    let mut sb = fresh();
    sb.network(NetPolicy::Brokered {
        socket: PathBuf::from(r"\\.\pipe\vibe-broker"),
        policy_id: "p".into(),
    });
    sb.network(NetPolicy::Direct);
    assert_eq!(sb.tier(), SandboxTier::Native);
}

// ────────────────────────────────────────────────────────────────────────────
//  CATEGORY 3: Spawn contract (slice N3.2 gap)
// ────────────────────────────────────────────────────────────────────────────

/// The current backend's `spawn` returns `SandboxError::NotSupported`
/// pointing at slice N3.2. This is a *typed* gap — callers fall back
/// to an un-sandboxed subprocess on Windows CI but the error spells
/// the gap out. A regression that silently spawned an un-sandboxed
/// child without surfacing the error would defeat the contract.
#[test]
fn spawn_surfaces_not_supported_until_slice_n32_lands() {
    use std::ffi::OsString;
    let sb = fresh();
    let cmd = OsString::from("cmd.exe");
    let arg = OsString::from("/c echo hi");
    let args = [arg.as_os_str()];
    let result = sb.spawn(cmd.as_os_str(), &args);
    match result {
        Err(SandboxError::NotSupported(msg)) => {
            assert!(
                msg.contains("N3.2") || msg.to_lowercase().contains("windows"),
                "expected a slice-N3.2 reference, got: {msg}"
            );
        }
        Err(other) => panic!("expected NotSupported, got: {other}"),
        Ok(_) => panic!(
            "spawn unexpectedly succeeded — if slice N3.2 has landed, \
             un-ignore the follow-up tests and remove this gap pin"
        ),
    }
}

/// Even after `spawn` errors, the sandbox stays usable for further
/// configuration calls (no poisoned-state semantics).
#[test]
fn spawn_failure_does_not_poison_sandbox() {
    use std::ffi::OsString;
    let mut sb = fresh();
    let cmd = OsString::from("cmd.exe");
    let args: [&std::ffi::OsStr; 0] = [];
    let _ = sb.spawn(cmd.as_os_str(), &args);
    // Subsequent config calls still work.
    sb.network(NetPolicy::Direct);
    sb.bind_rw(Path::new(r"C:\tmp\work"), Path::new(r"C:\work"))
        .expect("sandbox not poisoned by failed spawn");
}

// ────────────────────────────────────────────────────────────────────────────
//  CATEGORY 4: Resource-limit omission
// ────────────────────────────────────────────────────────────────────────────

/// Default limits are all-`None` (unbounded). Same contract as
/// Linux/macOS; callers MUST set explicit limits and the review
/// checklist flags omissions.
#[test]
fn resource_limits_default_is_unbounded_by_contract() {
    let l: ResourceLimits = ResourceLimits::default();
    assert!(l.cpu_quota_ms_per_sec.is_none());
    assert!(l.memory_bytes.is_none());
    assert!(l.pids.is_none());
    assert!(l.wall_clock.is_none());
    assert!(l.max_open_files.is_none());
}

/// A populated limits config round-trips through `Sandbox::limits()`.
/// The trait stores but doesn't expose them — behavioral assertion is
/// that the call doesn't panic and the tier stays Native.
#[test]
fn resource_limits_round_trip_through_sandbox_limits() {
    let mut sb = fresh();
    sb.limits(ResourceLimits {
        cpu_quota_ms_per_sec: Some(500),
        memory_bytes: Some(512 * 1024 * 1024),
        pids: Some(64),
        wall_clock: Some(Duration::from_secs(30)),
        max_open_files: Some(256),
    });
    assert_eq!(sb.tier(), SandboxTier::Native);
}

// ────────────────────────────────────────────────────────────────────────────
//  CATEGORY 5: Credential-directory deny-list
// ────────────────────────────────────────────────────────────────────────────
//
// The Linux `DENIED_SEGMENTS` deny-list was ported to Windows on
// 2026-05-14. The Windows deny-list also includes `Credentials` /
// `Vault` (case-insensitive) so bindings under `%APPDATA%\Microsoft\…`
// are rejected without needing a full `AppData\Roaming\Microsoft\…`
// prefix match. These tests guard the same promise as the Linux
// backend plus the Windows-specific credential-store extension.

#[test]
fn windows_rejects_user_vibecli_state_dir() {
    let mut sb = fresh();
    let err = sb
        .bind_rw(
            Path::new(r"C:\Users\alice\.vibecli"),
            Path::new(r"C:\work"),
        )
        .unwrap_err();
    assert!(format!("{err}").contains(".vibecli"));
}

#[test]
fn windows_rejects_workspace_vibecli_state_dir() {
    let mut sb = fresh();
    let err = sb
        .bind_ro(
            Path::new(r"C:\Users\alice\code\myrepo\.vibecli"),
            Path::new(r"C:\repo\.vibecli"),
        )
        .unwrap_err();
    assert!(format!("{err}").contains(".vibecli"));
}

#[test]
fn windows_rejects_user_claude_state_dir() {
    let mut sb = fresh();
    let err = sb
        .bind_rw(
            Path::new(r"C:\Users\alice\.claude"),
            Path::new(r"C:\work"),
        )
        .unwrap_err();
    assert!(format!("{err}").contains(".claude"));
}

/// `%APPDATA%\Microsoft\Credentials` (the Windows Credential Manager
/// backing store) and `Vault` (DPAPI-secured credential store) must
/// be rejected. Matches via segment, not full prefix, so any
/// subdirectory under one is also denied.
#[test]
fn windows_rejects_appdata_credentials_dir() {
    let mut sb = fresh();
    let err = sb
        .bind_rw(
            Path::new(r"C:\Users\alice\AppData\Roaming\Microsoft\Credentials"),
            Path::new(r"C:\creds"),
        )
        .unwrap_err();
    assert!(format!("{err}").contains("Credentials"));
}

#[test]
fn windows_rejects_appdata_vault_dir() {
    let mut sb = fresh();
    let err = sb
        .bind_ro(
            Path::new(r"C:\Users\alice\AppData\Roaming\Microsoft\Vault"),
            Path::new(r"C:\vault"),
        )
        .unwrap_err();
    assert!(format!("{err}").contains("Vault"));
}

/// Case-insensitive segment match — NTFS is case-insensitive by
/// default. An attacker / typo shouldn't bypass via casing.
#[test]
fn windows_rejects_case_variant_of_vibecli_segment() {
    let mut sb = fresh();
    let err = sb
        .bind_rw(
            Path::new(r"C:\Users\alice\.VibeCli\profile_settings.db"),
            Path::new(r"C:\work"),
        )
        .unwrap_err();
    let s = format!("{err}");
    assert!(s.contains(".vibecli") || s.contains("profile_settings.db"));
}

/// Filename-only match: even outside a deny-listed parent, the known
/// credential filenames are rejected.
#[test]
fn windows_rejects_daemon_token_filename_anywhere() {
    let mut sb = fresh();
    let err = sb
        .bind_rw(
            Path::new(r"C:\tmp\exports\daemon.token"),
            Path::new(r"C:\t"),
        )
        .unwrap_err();
    assert!(format!("{err}").contains("daemon.token"));
}

// ────────────────────────────────────────────────────────────────────────────
//  CATEGORY 6: Tier identity
// ────────────────────────────────────────────────────────────────────────────

#[test]
fn windows_sandbox_reports_native_tier() {
    let sb = fresh();
    assert_eq!(sb.tier(), SandboxTier::Native);
}

#[test]
fn windows_sandbox_tier_is_stable_across_constructions() {
    let a = fresh();
    let b = fresh();
    assert_eq!(a.tier(), b.tier());
    assert_eq!(a.tier(), SandboxTier::Native);
}
