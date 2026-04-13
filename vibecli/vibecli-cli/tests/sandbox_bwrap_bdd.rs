/*!
 * BDD tests for sandbox_bwrap using Cucumber 0.20.
 * Run with: cargo test --test sandbox_bwrap_bdd
 */
use cucumber::{World, given, then, when};
use vibecli_cli::sandbox_bwrap::BwrapProfile;

#[derive(Debug, World)]
pub struct BwrapWorld {
    profile: BwrapProfile,
    initial_ro_count: usize,
    validation_error: Option<String>,
}

impl Default for BwrapWorld {
    fn default() -> Self {
        Self {
            profile: BwrapProfile::new(),
            initial_ro_count: 0,
            validation_error: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Given steps
// ---------------------------------------------------------------------------

#[given("a minimal bwrap profile")]
fn set_minimal_profile(world: &mut BwrapWorld) {
    world.profile = BwrapProfile::minimal();
    world.initial_ro_count = world.profile.ro_count();
}

#[given(expr = "a minimal bwrap profile with an extra ro bind to {string}")]
fn set_minimal_with_extra_ro(world: &mut BwrapWorld, dst: String) {
    world.profile = BwrapProfile::minimal().add_ro(dst.clone(), dst);
    world.initial_ro_count = world.profile.ro_count();
}

// ---------------------------------------------------------------------------
// When steps
// ---------------------------------------------------------------------------

#[when(expr = "I add a read-only bind from {string} to {string}")]
fn add_ro_bind(world: &mut BwrapWorld, src: String, dst: String) {
    // Clone profile, add bind, and reassign (builder pattern returns owned Self)
    let profile = world.profile.clone().add_ro(src, dst);
    world.profile = profile;
}

#[when("I enable network access")]
fn enable_network(world: &mut BwrapWorld) {
    let profile = world.profile.clone().with_network();
    world.profile = profile;
}

// ---------------------------------------------------------------------------
// Then steps
// ---------------------------------------------------------------------------

#[then("it should unshare network")]
fn check_unshares_net(world: &mut BwrapWorld) {
    assert!(
        world.profile.unshares_network(),
        "Expected profile to unshare network"
    );
}

#[then("it should unshare pid")]
fn check_unshares_pid(world: &mut BwrapWorld) {
    assert!(
        world.profile.unshares_pid(),
        "Expected profile to unshare PID"
    );
}

#[then("it should not unshare network")]
fn check_not_unshares_net(world: &mut BwrapWorld) {
    assert!(
        !world.profile.unshares_network(),
        "Expected profile NOT to unshare network after with_network()"
    );
}

#[then("it should still unshare pid")]
fn check_still_unshares_pid(world: &mut BwrapWorld) {
    assert!(
        world.profile.unshares_pid(),
        "Expected profile to still unshare PID"
    );
}

#[then("the ro_count should increase by 1")]
fn check_ro_count_increased(world: &mut BwrapWorld) {
    let current = world.profile.ro_count();
    assert_eq!(
        current,
        world.initial_ro_count + 1,
        "Expected ro_count = {} but got {}",
        world.initial_ro_count + 1,
        current
    );
}

#[then("validation should fail with a duplicate destination error")]
fn check_validation_fails(world: &mut BwrapWorld) {
    match world.profile.validate() {
        Ok(()) => panic!("Expected validation to fail but it passed"),
        Err(e) => {
            assert!(
                e.message.contains("Duplicate") || e.message.contains("duplicate"),
                "Expected duplicate-destination error but got: {}",
                e.message
            );
            world.validation_error = Some(e.message);
        }
    }
}

fn main() {
    futures::executor::block_on(BwrapWorld::run("tests/features/sandbox_bwrap.feature"));
}
