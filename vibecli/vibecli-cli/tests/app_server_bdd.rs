/*!
 * BDD tests for the embedded app server lifecycle using Cucumber.
 * Run with: cargo test --test app_server_bdd
 */
use cucumber::{given, then, when, World};
use vibecli_cli::app_server::{AppServer, AppServerConfig, AppServerError};

#[derive(Debug, Default, World)]
pub struct AsWorld {
    server: AppServer,
    start_err: Option<AppServerError>,
    stop_err: Option<AppServerError>,
    second_start_err: Option<AppServerError>,
    start_attempts: u32,
}

// ── Given ─────────────────────────────────────────────────────────────────────

#[given("a new app server")]
fn new_server(world: &mut AsWorld) {
    world.server = AppServer::new();
}

// ── When ──────────────────────────────────────────────────────────────────────

#[when(expr = "I start the server on port {int} serving {string}")]
fn start_server(world: &mut AsWorld, port: u16, root: String) {
    // Use was_running snapshot so we know whether this is the second attempt.
    let was_running = world.server.is_running();
    world.start_attempts += 1;
    if let Err(e) = world.server.start(AppServerConfig::new(port, root)) {
        if was_running {
            world.second_start_err = Some(e);
        } else {
            world.start_err = Some(e);
        }
    }
}

#[when("I stop the server")]
fn stop_server(world: &mut AsWorld) {
    world.stop_err = world.server.stop().err();
}

#[when("I stop the server without starting")]
fn stop_without_start(world: &mut AsWorld) {
    world.stop_err = world.server.stop().err();
}

// ── Then ──────────────────────────────────────────────────────────────────────

#[then("the server should be running")]
fn assert_running(world: &mut AsWorld) {
    assert!(world.server.is_running(), "expected server to be running");
}

#[then("the server should not be running")]
fn assert_not_running(world: &mut AsWorld) {
    assert!(!world.server.is_running(), "expected server to not be running");
}

#[then(expr = "the port should be {int}")]
fn assert_port(world: &mut AsWorld, expected: u16) {
    assert_eq!(world.server.port(), Some(expected), "port mismatch");
}

#[then("the second start should fail with AlreadyRunning")]
fn assert_already_running(world: &mut AsWorld) {
    assert_eq!(
        world.second_start_err.as_ref().unwrap(),
        &AppServerError::AlreadyRunning,
        "expected AlreadyRunning error"
    );
}

#[then("the stop should fail with NotRunning")]
fn assert_not_running_err(world: &mut AsWorld) {
    assert_eq!(
        world.stop_err.as_ref().unwrap(),
        &AppServerError::NotRunning,
        "expected NotRunning error"
    );
}

fn main() {
    futures::executor::block_on(AsWorld::run("tests/features/app_server.feature"));
}
