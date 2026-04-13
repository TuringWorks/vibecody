/*!
 * BDD tests for plugin bundle validation using Cucumber.
 * Run with: cargo test --test plugin_bundle_bdd
 */
use cucumber::{given, then, when, World};
use vibecli_cli::plugin_bundle::{PluginBundle, PluginMeta};

#[derive(Debug, Default, World)]
pub struct PbWorld {
    bundle: PluginBundle,
    report: Option<vibecli_cli::plugin_bundle::BundleReport>,
}

// ── Given ─────────────────────────────────────────────────────────────────────

#[given(expr = "a bundle with plugin {string} version {string}")]
fn add_simple_plugin(world: &mut PbWorld, id: String, version: String) {
    world.bundle.add(PluginMeta::new(id, version));
}

#[given(expr = "a plugin {string} version {string} that requires {string}")]
fn add_plugin_with_dep(world: &mut PbWorld, id: String, version: String, dep: String) {
    world.bundle.add(PluginMeta::new(id, version).require(dep));
}

#[given(expr = "a bundle with plugin {string} version {string} that requires {string}")]
fn add_bundle_plugin_with_dep(world: &mut PbWorld, id: String, version: String, dep: String) {
    world.bundle.add(PluginMeta::new(id, version).require(dep));
}

// ── When ──────────────────────────────────────────────────────────────────────

#[when("I validate the bundle")]
fn validate(world: &mut PbWorld) {
    world.report = Some(world.bundle.validate());
}

// ── Then ──────────────────────────────────────────────────────────────────────

#[then("the bundle should be valid")]
fn assert_valid(world: &mut PbWorld) {
    let r = world.report.as_ref().unwrap();
    assert!(r.valid, "expected valid bundle, got: {:?}", r);
}

#[then("the bundle should be invalid")]
fn assert_invalid(world: &mut PbWorld) {
    let r = world.report.as_ref().unwrap();
    assert!(!r.valid, "expected invalid bundle but it was valid");
}

#[then(expr = "there should be {int} missing dependency")]
fn assert_missing_deps(world: &mut PbWorld, count: usize) {
    let r = world.report.as_ref().unwrap();
    assert_eq!(r.missing_deps.len(), count, "missing_deps count mismatch: {:?}", r.missing_deps);
}

#[then(expr = "there should be {int} duplicate id")]
fn assert_duplicate_ids(world: &mut PbWorld, count: usize) {
    let r = world.report.as_ref().unwrap();
    assert_eq!(r.duplicate_ids.len(), count, "duplicate_ids count mismatch: {:?}", r.duplicate_ids);
}

fn main() {
    futures::executor::block_on(PbWorld::run("tests/features/plugin_bundle.feature"));
}
