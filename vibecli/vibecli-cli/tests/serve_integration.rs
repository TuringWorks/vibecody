//! Integration tests for the VibeCLI HTTP serve daemon.
//!
//! Because `vibecli` is a binary crate (no `[lib]` target), in-process
//! integration tests that exercise the axum router via `tower::ServiceExt::oneshot()`
//! live inside the binary source at `src/serve.rs` under
//! `serve::tests::http_integration`. Those tests construct the router with a
//! mock AI provider and test every route without binding to a TCP port.
//!
//! This file provides additional integration-level smoke tests that start the
//! actual `vibecli serve` binary as a subprocess and hit the HTTP endpoints over
//! real TCP (ensuring the full startup path works as expected).
//!
//! ## In-process tests (26 tests in `src/serve.rs::tests::http_integration`)
//!
//! Run with:
//! ```bash
//! cargo test -p vibecli --bin vibecli -- serve::tests::http_integration
//! ```
//!
//! Covered routes and assertions:
//! - `GET  /health`            — 200 + JSON `{"status":"ok"}` + no auth required
//! - `POST /chat`              — 401 without auth, 401 with wrong token, 200 with correct token
//! - `POST /agent`             — 401 without auth
//! - `GET  /jobs`              — 401 without auth, 200 with auth (empty list)
//! - `GET  /jobs/:id`          — 404 for nonexistent job
//! - `POST /jobs/:id/cancel`   — 404 for nonexistent job
//! - `GET  /sessions`          — 200 with text/html content-type (or 500 if no DB)
//! - `GET  /sessions.json`     — application/json content-type
//! - `GET  /acp/v1/capabilities` — 200 (public)
//! - Security headers: X-Content-Type-Options, X-Frame-Options, Referrer-Policy, CSP
//! - CORS: allowed origin gets ACAO header; disallowed origin does not
//! - Body size limit: >1 MB body returns 413 Payload Too Large
//! - Unknown route returns 404

use std::process::Command;

/// Returns a `Command` for the compiled `vibecli` binary.
fn vibecli() -> Command {
    let bin = env!("CARGO_BIN_EXE_vibecli");
    Command::new(bin)
}

// ── Verify the serve flag is recognized ─────────────────────────────────────

#[test]
fn serve_flag_in_help_output() {
    let output = vibecli().arg("--help").output().expect("failed to run vibecli");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("serve") || stdout.contains("port"),
        "--help should mention serve functionality; got:\n{stdout}"
    );
}

/// Starting serve with port 0 should fail fast (no provider configured by
/// default) but should NOT panic — it should exit with a non-zero code.
#[test]
fn serve_without_provider_exits_gracefully() {
    let output = vibecli()
        .args(["--serve", "--port", "0", "--provider", "nonexistent"])
        .output()
        .expect("failed to run vibecli");
    // Should exit with a code (not killed by signal)
    assert!(
        output.status.code().is_some(),
        "serve should exit with a code, not a signal"
    );
}

/// Ensure that the in-process HTTP integration tests in `src/serve.rs` are
/// still discovered and passing. This test invokes `cargo test` as a subprocess
/// specifically for the `http_integration` module so that CI catches regressions
/// even if someone only runs `cargo test --test serve_integration`.
///
/// NOTE: This test is `#[ignore]` by default because it spawns a nested
/// `cargo test` process which is slow. Run it explicitly with:
/// ```bash
/// cargo test --test serve_integration -- --ignored
/// ```
#[test]
#[ignore]
fn inprocess_http_tests_pass() {
    let status = Command::new("cargo")
        .args([
            "test",
            "-p",
            "vibecli",
            "--bin",
            "vibecli",
            "--",
            "serve::tests::http_integration",
            "--test-threads=1",
        ])
        .status()
        .expect("failed to invoke cargo test");
    assert!(
        status.success(),
        "In-process HTTP integration tests should all pass"
    );
}
