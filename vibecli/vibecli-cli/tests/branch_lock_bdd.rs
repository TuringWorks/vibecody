/*!
 * BDD tests for branch_lock using Cucumber.
 * Run with: cargo test --test branch_lock_bdd
 */
use cucumber::{World, given, then, when};
use vibecli_cli::branch_lock::{CollisionRegistry, LockIntent};

#[derive(Debug, Default, World)]
pub struct BlWorld {
    registry: CollisionRegistry,
    last_acquire_ok: bool,
    last_collisions: usize,
}

fn intent_from(s: &str) -> LockIntent {
    match s {
        "read" => LockIntent::Read,
        "write" => LockIntent::Write,
        "exclusive" => LockIntent::Exclusive,
        _ => LockIntent::Write,
    }
}

#[given(expr = "branch {string} is locked by lane {string} for {word}")]
fn lock_branch(world: &mut BlWorld, branch: String, lane: String, intent_str: String) {
    world
        .registry
        .acquire(&branch, &lane, intent_from(&intent_str))
        .unwrap();
}

#[when(expr = "lane {string} tries to lock {string} for {word}")]
fn try_lock(world: &mut BlWorld, lane: String, branch: String, intent_str: String) {
    match world.registry.acquire(&branch, &lane, intent_from(&intent_str)) {
        Ok(_) => {
            world.last_acquire_ok = true;
            world.last_collisions = 0;
        }
        Err(c) => {
            world.last_acquire_ok = false;
            world.last_collisions = c.len();
        }
    }
}

#[when(expr = "lane {string} releases {string}")]
fn release_branch(world: &mut BlWorld, lane: String, branch: String) {
    world.registry.release(&branch, &lane);
}

#[then("the acquisition should fail with a collision")]
fn check_fail(world: &mut BlWorld) {
    assert!(!world.last_acquire_ok, "expected failure but got success");
    assert!(world.last_collisions > 0);
}

#[then("the acquisition should succeed")]
fn check_success(world: &mut BlWorld) {
    assert!(
        world.last_acquire_ok,
        "expected success but got failure with {} collisions",
        world.last_collisions
    );
}

fn main() {
    futures::executor::block_on(BlWorld::run("tests/features/branch_lock.feature"));
}
