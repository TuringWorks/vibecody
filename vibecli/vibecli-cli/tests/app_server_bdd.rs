/*!
 * BDD tests for the JSON-RPC 2.0 AppServer dispatcher using Cucumber.
 * Run with: cargo test --test app_server_bdd
 */
use cucumber::{given, then, when, World};
use serde_json::json;
use vibecli_cli::app_server::{AppServer, RpcId, RpcRequest};

// AppServer holds a HashMap<String, HandlerFn> where HandlerFn = Box<dyn Fn...>
// which is not Debug. Wrap in Option so the World derive is satisfied via Option's Debug.
#[derive(Default, World)]
pub struct AsWorld {
    server: Option<AppServer>,
    last_response: Option<vibecli_cli::app_server::RpcResponse>,
    last_raw: Option<String>,
}

impl std::fmt::Debug for AsWorld {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AsWorld")
            .field("server", &self.server.is_some())
            .field("last_response", &self.last_response)
            .field("last_raw", &self.last_raw)
            .finish()
    }
}

// ── Given ─────────────────────────────────────────────────────────────────────

#[given(expr = "an app server with an {string} handler")]
fn server_with_echo(world: &mut AsWorld, method: String) {
    let mut server = AppServer::new();
    server.register(
        &method,
        Box::new(|params| params.unwrap_or(json!("no params"))),
    );
    world.server = Some(server);
}

// ── When ──────────────────────────────────────────────────────────────────────

#[when(expr = "I dispatch a request for method {string} with params {string}")]
fn dispatch_with_params(world: &mut AsWorld, method: String, params: String) {
    let req = RpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(RpcId::Number(1)),
        method,
        params: Some(json!(params)),
    };
    world.last_response = Some(world.server.as_ref().unwrap().dispatch(&req));
}

#[when(expr = "I dispatch a request for method {string}")]
fn dispatch_no_params(world: &mut AsWorld, method: String) {
    let req = RpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(RpcId::Number(1)),
        method,
        params: None,
    };
    world.last_response = Some(world.server.as_ref().unwrap().dispatch(&req));
}

#[when(expr = "I handle raw JSON {string}")]
fn handle_raw(world: &mut AsWorld, raw: String) {
    world.last_raw = Some(world.server.as_ref().unwrap().handle_raw(&raw));
}

// ── Then ──────────────────────────────────────────────────────────────────────

#[then("the response should have no error")]
fn assert_no_error(world: &mut AsWorld) {
    let r = world.last_response.as_ref().unwrap();
    assert!(r.error.is_none(), "expected no error, got: {:?}", r.error);
}

#[then(expr = "the result should equal {string}")]
fn assert_result_eq(world: &mut AsWorld, expected: String) {
    let r = world.last_response.as_ref().unwrap();
    assert_eq!(r.result, Some(json!(expected)), "result mismatch");
}

#[then(expr = "the response should have an error with code {int}")]
fn assert_error_code(world: &mut AsWorld, code: i32) {
    let r = world.last_response.as_ref().unwrap();
    let err = r.error.as_ref().expect("expected an error");
    assert_eq!(err.code, code, "error code mismatch");
}

#[then(expr = "the raw response should contain {string}")]
fn assert_raw_contains(world: &mut AsWorld, needle: String) {
    let raw = world.last_raw.as_ref().unwrap();
    assert!(raw.contains(&needle), "expected {:?} in raw response: {}", needle, raw);
}

fn main() {
    futures::executor::block_on(AsWorld::run("tests/features/app_server.feature"));
}
