//! BDD tests for the cross-platform Sandbox trait + tier selection.
//! Run with: cargo test -p vibe-sandbox --test sandbox_trait_bdd

use cucumber::{World, given, then, when};
use std::str::FromStr;
use vibe_sandbox::{
    BindMode, NetPolicy, ResourceLimits, Sandbox, SandboxTier, SelectOptions, select,
};

#[derive(Default, World)]
pub struct SbWorld {
    requested_tier: Option<SandboxTier>,
    sandbox: Option<Box<dyn Sandbox>>,
    parsed_tier: Option<SandboxTier>,
    net_policy: Option<NetPolicy>,
    limits: Option<ResourceLimits>,
    bind_mode: Option<BindMode>,
    host_supports_firecracker: bool,
    downgrade_recorded: bool,
}

impl std::fmt::Debug for SbWorld {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SbWorld")
            .field("requested_tier", &self.requested_tier)
            .field("has_sandbox", &self.sandbox.is_some())
            .field("parsed_tier", &self.parsed_tier)
            .field("downgrade_recorded", &self.downgrade_recorded)
            .finish()
    }
}

#[given(expr = "a request for the {word} tier")]
fn request_tier(world: &mut SbWorld, name: String) {
    world.requested_tier = Some(SandboxTier::from_str(&name).expect("known tier"));
}

#[when("I call select on it")]
fn call_select_on_requested(world: &mut SbWorld) {
    let tier = world.requested_tier.expect("tier requested");
    let opts = SelectOptions::default();
    world.sandbox = Some(select(tier, &opts).expect("select succeeds").into_sandbox());
}

#[then(expr = "I get a sandbox whose tier is {string}")]
fn sandbox_tier_is(world: &mut SbWorld, expected: String) {
    let actual = world.sandbox.as_ref().expect("sandbox built").tier();
    assert_eq!(actual.to_string(), expected);
}

#[given(expr = "a tier name {string}")]
fn tier_name(world: &mut SbWorld, name: String) {
    world.parsed_tier = Some(SandboxTier::from_str(&name).expect("parses"));
}

#[when("I parse it via FromStr")]
fn parse_via_fromstr(_: &mut SbWorld) {
    // No-op: parsing happened in the Given step.
}

#[then(expr = "I get a tier whose Display is {string}")]
fn display_is(world: &mut SbWorld, expected: String) {
    assert_eq!(world.parsed_tier.unwrap().to_string(), expected);
}

#[given("a fresh NetPolicy default")]
fn fresh_net_policy(world: &mut SbWorld) {
    world.net_policy = Some(NetPolicy::default());
}

#[then(expr = "the policy variant is {string}")]
fn policy_variant_is(world: &mut SbWorld, expected: String) {
    let p = world.net_policy.as_ref().unwrap();
    let variant = match p {
        NetPolicy::None => "None",
        NetPolicy::Brokered { .. } => "Brokered",
        NetPolicy::Direct => "Direct",
    };
    assert_eq!(variant, expected);
}

#[given("a fresh ResourceLimits default")]
fn fresh_limits(world: &mut SbWorld) {
    world.limits = Some(ResourceLimits::default());
}

#[then("memory_bytes is unset")]
fn memory_unset(world: &mut SbWorld) {
    assert!(world.limits.as_ref().unwrap().memory_bytes.is_none());
}

#[then("cpu_quota_ms_per_sec is unset")]
fn cpu_unset(world: &mut SbWorld) {
    assert!(world.limits.as_ref().unwrap().cpu_quota_ms_per_sec.is_none());
}

#[then("wall_clock is unset")]
fn wall_unset(world: &mut SbWorld) {
    assert!(world.limits.as_ref().unwrap().wall_clock.is_none());
}

#[given("a host that does not support Firecracker")]
fn host_no_fc(world: &mut SbWorld) {
    world.host_supports_firecracker = false;
}

#[when(expr = "I call select on the {word} tier")]
fn call_select_on(world: &mut SbWorld, name: String) {
    let tier = SandboxTier::from_str(&name).unwrap();
    let mut opts = SelectOptions::default();
    opts.host_supports_firecracker = world.host_supports_firecracker;
    opts.host_supports_hyperlight = false;
    opts.on_downgrade = Some(Box::new(|| {
        // Captured below via shared flag — here we just no-op since the test
        // checks via a thread-local recorded inside select itself.
    }));
    let out = select(tier, &opts).expect("select succeeds");
    world.downgrade_recorded = out.downgraded();
    world.sandbox = Some(out.into_sandbox());
}

#[then(expr = "the returned tier is {string}")]
fn returned_tier_is(world: &mut SbWorld, expected: String) {
    assert_eq!(
        world.sandbox.as_ref().unwrap().tier().to_string(),
        expected
    );
}

#[then("a downgrade event was recorded")]
fn downgrade_recorded(world: &mut SbWorld) {
    assert!(world.downgrade_recorded, "expected a downgrade event");
}

#[given(expr = "a BindMode {string}")]
fn bind_mode(world: &mut SbWorld, name: String) {
    world.bind_mode = Some(match name.as_str() {
        "Rw" => BindMode::Rw,
        "Ro" => BindMode::Ro,
        other => panic!("unknown BindMode: {other}"),
    });
}

#[then("the mode allows writes")]
fn mode_allows_writes(world: &mut SbWorld) {
    assert!(world.bind_mode.unwrap().allows_writes());
}

#[then("the mode does not allow writes")]
fn mode_no_writes(world: &mut SbWorld) {
    assert!(!world.bind_mode.unwrap().allows_writes());
}

#[then("the mode allows reads")]
fn mode_allows_reads(world: &mut SbWorld) {
    assert!(world.bind_mode.unwrap().allows_reads());
}

fn main() {
    futures::executor::block_on(SbWorld::run("tests/features/sandbox_trait.feature"));
}
