/*!
 * BDD tests for the Windows-style ACL sandbox policy using Cucumber.
 * Run with: cargo test --test sandbox_windows_bdd
 */
use cucumber::{given, then, when, World};
use vibecli_cli::sandbox_windows::{
    NetworkPolicy, SandboxVerdict, WindowsSandbox, WindowsSandboxConfig,
};

#[derive(Debug, Default, World)]
pub struct SwWorld {
    sandbox: Option<WindowsSandbox>,
    path_verdict: Option<SandboxVerdict>,
    net_verdict: Option<SandboxVerdict>,
}

// ── Given ─────────────────────────────────────────────────────────────────────

#[given(expr = "a sandbox with allowed path {string}")]
fn sandbox_allow_path(world: &mut SwWorld, path: String) {
    let cfg = WindowsSandboxConfig::default_restricted().allow_path(&path);
    world.sandbox = Some(WindowsSandbox::new(cfg));
}

#[given(expr = "a sandbox with denied path {string}")]
fn sandbox_deny_path(world: &mut SwWorld, path: String) {
    let cfg = WindowsSandboxConfig::default_restricted().deny_path(&path);
    world.sandbox = Some(WindowsSandbox::new(cfg));
}

#[given(expr = "a sandbox with allowed path {string} and denied path {string}")]
fn sandbox_allow_and_deny(world: &mut SwWorld, allow: String, deny: String) {
    let cfg = WindowsSandboxConfig::default_restricted()
        .allow_path(&allow)
        .deny_path(&deny);
    world.sandbox = Some(WindowsSandbox::new(cfg));
}

#[given(expr = "a sandbox with no internet and allowed host {string}")]
fn sandbox_no_internet_with_host(world: &mut SwWorld, host: String) {
    let mut cfg = WindowsSandboxConfig::default_restricted();
    cfg.network = NetworkPolicy { allow_internet: false, allowed_hosts: vec![host] };
    world.sandbox = Some(WindowsSandbox::new(cfg));
}

// ── When ──────────────────────────────────────────────────────────────────────

#[when(expr = "I check access to path {string}")]
fn check_path(world: &mut SwWorld, path: String) {
    if let Some(sb) = &world.sandbox {
        world.path_verdict = Some(sb.check_path(&path));
    }
}

#[when(expr = "I check network access to {string}")]
fn check_network(world: &mut SwWorld, host: String) {
    if let Some(sb) = &world.sandbox {
        world.net_verdict = Some(sb.check_network(&host));
    }
}

// ── Then ──────────────────────────────────────────────────────────────────────

#[then("the path verdict should be allowed")]
fn path_allowed(world: &mut SwWorld) {
    let v = world.path_verdict.as_ref().unwrap();
    assert!(v.allowed, "expected path allowed, reason: {}", v.reason);
}

#[then("the path verdict should be denied")]
fn path_denied(world: &mut SwWorld) {
    let v = world.path_verdict.as_ref().unwrap();
    assert!(!v.allowed, "expected path denied, reason: {}", v.reason);
}

#[then("the network verdict should be allowed")]
fn net_allowed(world: &mut SwWorld) {
    let v = world.net_verdict.as_ref().unwrap();
    assert!(v.allowed, "expected network allowed, reason: {}", v.reason);
}

#[then("the network verdict should be denied")]
fn net_denied(world: &mut SwWorld) {
    let v = world.net_verdict.as_ref().unwrap();
    assert!(!v.allowed, "expected network denied, reason: {}", v.reason);
}

fn main() {
    futures::executor::block_on(SwWorld::run("tests/features/sandbox_windows.feature"));
}
