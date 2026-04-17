/*!
 * BDD tests for workspace_fingerprint using Cucumber.
 * Run with: cargo test --test workspace_fingerprint_bdd
 */
use cucumber::{World, given, then, when};
use vibecli_cli::workspace_fingerprint::workspace_fingerprint;

#[derive(Debug, Default, World)]
pub struct WfWorld {
    path_a: String,
    path_b: String,
    fp_a: String,
    fp_b: String,
}

// Given steps
#[given(expr = "a workspace path {string}")]
fn single_path(world: &mut WfWorld, path: String) { world.path_a = path; }

#[given(expr = "workspace paths {string} and {string}")]
fn two_paths(world: &mut WfWorld, a: String, b: String) { world.path_a = a; world.path_b = b; }

// When steps
#[when("I compute the fingerprint")]
fn compute_one(world: &mut WfWorld) { world.fp_a = workspace_fingerprint(&world.path_a); }

#[when("I compute both fingerprints")]
fn compute_both(world: &mut WfWorld) {
    world.fp_a = workspace_fingerprint(&world.path_a);
    world.fp_b = workspace_fingerprint(&world.path_b);
}

// Then steps
#[then("it should be exactly 16 characters of hex digits")]
fn check_hex(world: &mut WfWorld) {
    assert_eq!(world.fp_a.len(), 16);
    assert!(world.fp_a.chars().all(|c| c.is_ascii_hexdigit()));
}

#[then("they should be identical")]
fn check_identical(world: &mut WfWorld) { assert_eq!(world.fp_a, world.fp_b); }

#[then("they should differ")]
fn check_differ(world: &mut WfWorld) { assert_ne!(world.fp_a, world.fp_b); }

fn main() {
    futures::executor::block_on(WfWorld::run("tests/features/workspace_fingerprint.feature"));
}
