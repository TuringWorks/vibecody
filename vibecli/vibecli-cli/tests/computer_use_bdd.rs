/*!
 * BDD tests for the desktop action model using Cucumber.
 * Run with: cargo test --test computer_use_bdd
 */
use cucumber::{given, then, when, World};
use vibecli_cli::computer_use::{Action, MouseButton, ScreenBounds};

#[derive(Debug, Default, World)]
pub struct CuWorld {
    action: Option<Action>,
    bounds: Option<ScreenBounds>,
    validation: Option<Result<(), String>>,
}

// ── Given ─────────────────────────────────────────────────────────────────────

#[given(expr = "an action of type Type with text {string}")]
fn type_action(world: &mut CuWorld, text: String) {
    world.action = Some(Action::Type { text });
}

#[given("a Screenshot action")]
fn screenshot_action(world: &mut CuWorld) {
    world.action = Some(Action::Screenshot);
}

#[given(expr = "screen bounds of {int} x {int}")]
fn set_bounds(world: &mut CuWorld, width: u32, height: u32) {
    world.bounds = Some(ScreenBounds::new(width, height));
}

// ── When ──────────────────────────────────────────────────────────────────────

#[when(expr = "I validate a click at {int} {int}")]
fn validate_click(world: &mut CuWorld, x: i32, y: i32) {
    let action = Action::Click { x, y, button: MouseButton::Left };
    let result = world.bounds.as_ref().unwrap().validate_action(&action);
    world.validation = Some(result);
}

// ── Then ──────────────────────────────────────────────────────────────────────

#[then("the action should be destructive")]
fn assert_destructive(world: &mut CuWorld) {
    assert!(
        world.action.as_ref().unwrap().is_destructive(),
        "expected action to be destructive"
    );
}

#[then("the action should not be destructive")]
fn assert_not_destructive(world: &mut CuWorld) {
    assert!(
        !world.action.as_ref().unwrap().is_destructive(),
        "expected action to NOT be destructive"
    );
}

#[then("the validation should succeed")]
fn assert_validation_ok(world: &mut CuWorld) {
    let r = world.validation.as_ref().unwrap();
    assert!(r.is_ok(), "expected validation to succeed, got: {:?}", r);
}

#[then("the validation should fail")]
fn assert_validation_fail(world: &mut CuWorld) {
    let r = world.validation.as_ref().unwrap();
    assert!(r.is_err(), "expected validation to fail but it succeeded");
}

fn main() {
    futures::executor::block_on(CuWorld::run("tests/features/computer_use.feature"));
}
