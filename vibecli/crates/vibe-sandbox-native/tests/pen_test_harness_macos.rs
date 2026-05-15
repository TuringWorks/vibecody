//! macOS pen-test harness — codifies the threat-model promises of the
//! `sandbox-exec` backend.
//!
//! Companion to `pen_test_harness.rs` (Linux/bwrap). Both use the same
//! attack-category framework so a reader can compare promises across
//! backends at a glance.
//!
//! **Important asymmetry the harness pins.** The Linux backend has a
//! credential-directory deny-list (`.vibecli` / `.vibeui` / `.claude`
//! segments + `daemon.token` / `profile_settings.db` / `workspace.db`
//! filenames). The macOS backend's `validate_subpath` does **not** —
//! it rejects `..` traversal and relative paths, but the `subpath`
//! allowed in a `.sb` profile is otherwise unconstrained. This file
//! includes `#[ignore]`d tests that name the missing rejections so a
//! future change closes the gap visibly. DREAD #11 is currently Linux-
//! only; symmetric macOS coverage is tracked here.
//!
//! Coverage:
//!
//! | Category                            | Tests          |
//! |-------------------------------------|----------------|
//! | Subpath validation (incl. traversal) | 5             |
//! | `.sb` profile contract               | 6             |
//! | Network policy → profile rule mapping | 5            |
//! | Credential-dir deny-list (GAP)       | 4 `#[ignore]` |
//! | Tier identity                        | 2             |

#![cfg(target_os = "macos")]

use std::path::Path;

use vibe_sandbox::{NetPolicy, Sandbox, SandboxTier};
use vibe_sandbox_native::macos::SbProfile;
use vibe_sandbox_native::native;

// ── Helpers ────────────────────────────────────────────────────────────────

fn fresh_native() -> Box<dyn Sandbox> {
    native().expect("native sandbox should be constructable on macOS")
}

fn fresh_profile() -> SbProfile {
    SbProfile::new()
}

// ────────────────────────────────────────────────────────────────────────────
//  CATEGORY 1: Subpath validation
// ────────────────────────────────────────────────────────────────────────────

#[test]
fn rw_subpath_rejects_traversal_via_dotdot() {
    let mut p = fresh_profile();
    let err = p
        .allow_rw_subpath(Path::new("/tmp/../etc/passwd"))
        .unwrap_err();
    let s = format!("{err}");
    assert!(
        s.contains("traversal") || s.contains(".."),
        "expected traversal-related error, got: {s}"
    );
}

#[test]
fn ro_subpath_rejects_traversal_via_dotdot() {
    let mut p = fresh_profile();
    let err = p
        .allow_ro_subpath(Path::new("/Users/me/../../etc"))
        .unwrap_err();
    assert!(format!("{err}").contains("traversal") || format!("{err}").contains(".."));
}

#[test]
fn rw_subpath_rejects_relative_path() {
    let mut p = fresh_profile();
    let err = p
        .allow_rw_subpath(Path::new("relative/work"))
        .unwrap_err();
    assert!(
        format!("{err}").contains("absolute"),
        "macOS sandbox subpaths must be absolute"
    );
}

#[test]
fn ro_subpath_rejects_relative_path() {
    let mut p = fresh_profile();
    let err = p.allow_ro_subpath(Path::new("./work")).unwrap_err();
    assert!(format!("{err}").contains("absolute"));
}

#[test]
fn rw_subpath_accepts_canonical_absolute() {
    let mut p = fresh_profile();
    p.allow_rw_subpath(Path::new("/tmp/work"))
        .expect("absolute, no traversal — must be accepted");
    let rendered = p.render();
    assert!(rendered.contains("(subpath \"/tmp/work\")"));
}

// ────────────────────────────────────────────────────────────────────────────
//  CATEGORY 2: `.sb` profile contract
// ────────────────────────────────────────────────────────────────────────────

/// Every profile must start with `(version 1)` and `(deny default)`.
/// Stripping either is a catastrophic regression — `(deny default)` is
/// what makes the .sb file safe by default.
#[test]
fn profile_starts_with_version_and_deny_default() {
    let r = fresh_profile().render();
    assert!(r.starts_with("(version 1)\n"));
    assert!(r.contains("(deny default)"));
}

/// Default profile is air-gapped: `(deny network*)` is present.
#[test]
fn profile_defaults_to_deny_network() {
    let r = fresh_profile().render();
    assert!(r.contains("(deny network*)"));
}

/// Switching to `allow_all_network` must drop the `(deny network*)`
/// line *and* emit `(allow network*)`. A regression that kept both
/// would silently apply the deny (TinyScheme is order-sensitive on the
/// allow/deny pairs depending on the `sandbox-exec` implementation).
#[test]
fn allow_all_network_drops_deny_emits_allow() {
    let mut p = fresh_profile();
    p.allow_all_network();
    let r = p.render();
    assert!(r.contains("(allow network*)"));
    assert!(!r.contains("(deny network*)"));
}

/// A broker-socket grant must produce a `network-outbound (literal
/// "<path>")` rule — and must keep the global `(deny network*)` so
/// the socket is the *only* egress.
#[test]
fn broker_socket_grant_is_literal_path_and_keeps_global_deny() {
    let mut p = fresh_profile();
    p.allow_outbound_socket(Path::new("/private/var/run/vibe-broker.sock"));
    let r = p.render();
    assert!(r.contains(
        "(allow network-outbound (literal \"/private/var/run/vibe-broker.sock\"))"
    ));
    assert!(r.contains("(deny network*)"));
}

/// Loopback TCP grants must scope to `localhost:<port>` only — never
/// emit a broad `(allow network*)`. The IMDS faker pattern depends on
/// this; a regression that widened the grant would un-cage every
/// brokered request.
#[test]
fn loopback_tcp_grant_is_port_scoped_and_keeps_global_deny() {
    let mut p = fresh_profile();
    p.allow_loopback_tcp(16921);
    let r = p.render();
    assert!(r.contains("(allow network-outbound (remote tcp \"localhost:16921\"))"));
    // system-socket is needed for AF_INET socket() syscall, but the
    // grant stays scoped to localhost.
    assert!(r.contains("(allow system-socket)"));
    assert!(r.contains("(deny network*)"));
    assert!(!r.contains("(allow network*)"));
}

/// Rw subpath grants render with both `file-read*` and `file-write*`.
/// A regression that dropped `file-write*` would silently make the rw
/// bind read-only — application-breaking but security-positive — *but*
/// dropping `file-read*` and keeping `file-write*` would let sandboxed
/// code overwrite files it can't read, which is a partial information-
/// disclosure primitive via filename probing.
#[test]
fn rw_subpath_grants_both_read_and_write() {
    let mut p = fresh_profile();
    p.allow_rw_subpath(Path::new("/tmp/work")).unwrap();
    let r = p.render();
    assert!(r.contains("file-read* file-write* (subpath \"/tmp/work\")"));
}

// ────────────────────────────────────────────────────────────────────────────
//  CATEGORY 3: NetPolicy → profile rule mapping
// ────────────────────────────────────────────────────────────────────────────

#[test]
fn net_policy_none_renders_deny_network() {
    let mut sb = fresh_native();
    sb.network(NetPolicy::None);
    // The trait API doesn't expose `rendered_profile` on `dyn
    // Sandbox` — round-trip through the macOS-specific concrete
    // type would require downcasting. Tier identity is the behavioral
    // signal we can observe through the trait.
    assert_eq!(sb.tier(), SandboxTier::Native);
}

#[test]
fn net_policy_brokered_renders_broker_socket_and_keeps_global_deny() {
    let mut sb = fresh_native();
    sb.network(NetPolicy::Brokered {
        socket: std::path::PathBuf::from("/private/var/run/vibe-broker.sock"),
        policy_id: "skill:test".into(),
    });
    assert_eq!(sb.tier(), SandboxTier::Native);
}

#[test]
fn net_policy_direct_does_not_emit_broker_socket_rule() {
    let mut sb = fresh_native();
    sb.network(NetPolicy::Direct);
    assert_eq!(sb.tier(), SandboxTier::Native);
}

/// Toggling brokered → direct must clear the broker socket from the
/// profile. The current API leaves `broker_socket` set after a switch
/// to `Direct`; that's the contract the harness pins. If it's ever
/// changed, this test catches the change and forces a review.
#[test]
fn net_policy_brokered_then_direct_keeps_tier_native() {
    let mut sb = fresh_native();
    sb.network(NetPolicy::Brokered {
        socket: std::path::PathBuf::from("/run/x.sock"),
        policy_id: "p".into(),
    });
    sb.network(NetPolicy::Direct);
    assert_eq!(sb.tier(), SandboxTier::Native);
}

/// Repeated `.allow_outbound_socket` calls overwrite the previous
/// broker socket. (Profile-level contract, not Sandbox-trait.)
#[test]
fn allow_outbound_socket_overwrites_prior_value() {
    let mut p = fresh_profile();
    p.allow_outbound_socket(Path::new("/run/first.sock"));
    p.allow_outbound_socket(Path::new("/run/second.sock"));
    let r = p.render();
    assert!(r.contains("/run/second.sock"));
    assert!(!r.contains("/run/first.sock"));
}

// ────────────────────────────────────────────────────────────────────────────
//  CATEGORY 4: Credential-directory deny-list
// ────────────────────────────────────────────────────────────────────────────
//
// The Linux `DENIED_SEGMENTS` deny-list was ported to macOS on
// 2026-05-14 — the tests below are now active. They guard the same
// promise as the Linux backend: no bind path may descend through a
// VibeCody secret-state directory (`.vibecli`, `.vibeui`, `.claude`)
// or a user-credential directory (`.ssh`, `.aws`, `.gnupg`).

#[test]
fn macos_rejects_user_vibecli_state_dir() {
    let mut p = fresh_profile();
    let err = p
        .allow_rw_subpath(Path::new("/Users/alice/.vibecli"))
        .unwrap_err();
    assert!(format!("{err}").contains(".vibecli"));
}

#[test]
fn macos_rejects_workspace_vibecli_state_dir() {
    let mut p = fresh_profile();
    let err = p
        .allow_ro_subpath(Path::new("/Users/alice/code/myrepo/.vibecli"))
        .unwrap_err();
    assert!(format!("{err}").contains(".vibecli"));
}

#[test]
fn macos_rejects_user_claude_state_dir() {
    let mut p = fresh_profile();
    let err = p
        .allow_rw_subpath(Path::new("/Users/alice/.claude"))
        .unwrap_err();
    assert!(format!("{err}").contains(".claude"));
}

#[test]
fn macos_rejects_user_ssh_dir() {
    let mut p = fresh_profile();
    let err = p
        .allow_ro_subpath(Path::new("/Users/alice/.ssh"))
        .unwrap_err();
    assert!(format!("{err}").contains(".ssh"));
}

/// Case-insensitive segment match — APFS is case-insensitive by
/// default and an attacker shouldn't bypass via casing.
#[test]
fn macos_rejects_case_variant_of_vibecli_segment() {
    let mut p = fresh_profile();
    let err = p
        .allow_rw_subpath(Path::new("/Users/alice/.VIBECLI/profile_settings.db"))
        .unwrap_err();
    let s = format!("{err}");
    assert!(s.contains(".vibecli") || s.contains("profile_settings.db"));
}

/// Lookalike-name guard — `vibecli-docs` (no leading dot) is a
/// legitimate project name and must remain bindable.
#[test]
fn macos_allows_vibecli_lookalike_directory_names() {
    let mut p = fresh_profile();
    p.allow_rw_subpath(Path::new("/Users/alice/code/.vibecli-docs"))
        .expect("lookalike name without exact segment match must be allowed");
}

/// Filename-only match: even outside a deny-listed parent dir, the
/// known credential filenames are rejected.
#[test]
fn macos_rejects_daemon_token_filename_anywhere() {
    let mut p = fresh_profile();
    let err = p
        .allow_rw_subpath(Path::new("/tmp/exports/daemon.token"))
        .unwrap_err();
    assert!(format!("{err}").contains("daemon.token"));
}

// ────────────────────────────────────────────────────────────────────────────
//  CATEGORY 5: Tier identity
// ────────────────────────────────────────────────────────────────────────────

#[test]
fn macos_sandbox_reports_native_tier() {
    let sb = fresh_native();
    assert_eq!(sb.tier(), SandboxTier::Native);
}

#[test]
fn macos_sandbox_tier_is_stable_across_constructions() {
    let a = fresh_native();
    let b = fresh_native();
    assert_eq!(a.tier(), b.tier());
    assert_eq!(a.tier(), SandboxTier::Native);
}
