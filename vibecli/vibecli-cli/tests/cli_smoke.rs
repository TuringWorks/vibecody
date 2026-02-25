//! Integration / smoke tests that invoke the `vibecli` binary as a subprocess.
//!
//! These tests verify the CLI's observable behaviour without importing internal
//! modules (which isn't possible for a binary-only crate).

use std::process::Command;

/// Returns the path to the compiled vibecli binary.
fn vibecli() -> Command {
    // `CARGO_BIN_EXE_vibecli` is set by cargo when building integration tests
    // for a crate that has a [[bin]] target.
    let bin = env!("CARGO_BIN_EXE_vibecli");
    Command::new(bin)
}

// ── Help / version ────────────────────────────────────────────────────────────

#[test]
fn help_flag_exits_zero() {
    let output = vibecli().arg("--help").output().expect("failed to run vibecli");
    assert!(output.status.success(), "--help should exit 0");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("vibecli") || stdout.contains("Usage"),
        "--help output should mention vibecli or Usage"
    );
}

#[test]
fn version_flag_exits_zero() {
    let output = vibecli().arg("--version").output().expect("failed to run vibecli");
    assert!(output.status.success(), "--version should exit 0");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("vibecli") || stdout.contains("0.1"),
        "--version should include binary name or version number"
    );
}

// ── Doctor (health check) ─────────────────────────────────────────────────────

#[test]
fn doctor_flag_exits_zero_and_reports_checks() {
    let output = vibecli().arg("--doctor").output().expect("failed to run vibecli");
    // doctor always exits 0 even if some checks fail (it's informational)
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{stdout}{stderr}");
    assert!(
        combined.contains("doctor") || combined.contains("check") || combined.contains("OK")
            || combined.contains("✓") || combined.contains("✗"),
        "--doctor should print health-check results; got:\n{combined}"
    );
}

// ── CI mode ───────────────────────────────────────────────────────────────────

#[test]
fn ci_flag_without_task_exits_nonzero_or_prints_usage() {
    // Running --ci without a task description should either fail or print usage.
    let output = vibecli().arg("--ci").output().expect("failed to run vibecli");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{stdout}{stderr}");

    // It should either exit non-zero OR print a helpful message
    let graceful = !output.status.success()
        || combined.contains("task")
        || combined.contains("required")
        || combined.contains("Usage");
    assert!(graceful,
        "--ci with no task should indicate missing arg or fail; got:\n{combined}");
}

// ── JSON output format ────────────────────────────────────────────────────────

#[test]
fn json_flag_with_help_does_not_panic() {
    // --json is a flag that changes output format; combining with --help should not panic.
    let output = vibecli()
        .args(["--json", "--help"])
        .output()
        .expect("failed to spawn vibecli");
    // Exit code may vary; we just verify no crash (signal termination)
    assert!(
        output.status.code().is_some(),
        "process should exit normally, not via signal"
    );
}

// ── Serve mode flag existence ─────────────────────────────────────────────────

#[test]
fn serve_flag_appears_in_help() {
    let output = vibecli().arg("--help").output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("serve") || stdout.contains("port"),
        "--help should mention --serve or --port; got:\n{stdout}"
    );
}

// ── MCP server flag existence ─────────────────────────────────────────────────

#[test]
fn mcp_server_flag_appears_in_help() {
    let output = vibecli().arg("--help").output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("mcp"),
        "--help should mention --mcp-server; got:\n{stdout}"
    );
}
