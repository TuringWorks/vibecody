/*!
 * BDD tests for the ngrok tunnel auto-detection module.
 * Run with: cargo test --test ngrok_bdd
 */
use cucumber::{World, given, then, when};

/// Local copy of TunnelConfig for BDD assertions — mirrors config::TunnelConfig.
#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
struct TunnelConfig {
    #[serde(default)]
    tailscale_funnel: bool,
    #[serde(default)]
    ngrok_auto_start: bool,
    #[serde(default)]
    ngrok_auth_token: Option<String>,
}

#[derive(Debug, Default, World)]
pub struct NgrokWorld {
    mock_api_body: String,
    mock_port: u16,
    extracted_url: Option<String>,
    tunnel_config: Option<TunnelConfig>,
    roundtripped_config: Option<TunnelConfig>,
}

// ── Parse helper — mirrors logic in ngrok::detect_tunnel without the TCP probe ──

fn parse_ngrok_api(body: &str, port: u16) -> Option<String> {
    let json: serde_json::Value = serde_json::from_str(body).ok()?;
    let tunnels = json["tunnels"].as_array()?;
    let port_str = port.to_string();
    for tunnel in tunnels {
        if tunnel["proto"].as_str() != Some("https") { continue; }
        let addr = tunnel["config"]["addr"].as_str().unwrap_or("");
        if addr.contains(&port_str) {
            if let Some(url) = tunnel["public_url"].as_str() {
                return Some(url.to_string());
            }
        }
    }
    None
}

// ── Given ─────────────────────────────────────────────────────────────────────

#[given("no process is listening on localhost port 4040")]
fn no_ngrok_running(_world: &mut NgrokWorld) {
    // No-op: in a normal test environment port 4040 is not in use.
}

#[given(expr = "a mock ngrok API response with an HTTPS tunnel on port {int}")]
fn ngrok_https_response(world: &mut NgrokWorld, port: u16) {
    world.mock_api_body = format!(
        r#"{{"tunnels":[{{"name":"cmd","proto":"https","public_url":"https://abc123.ngrok.io","config":{{"addr":"localhost:{port}"}}}}]}}"#
    );
    world.mock_port = port;
}

#[given(expr = "a mock ngrok API response with an HTTP-only tunnel on port {int}")]
fn ngrok_http_response(world: &mut NgrokWorld, port: u16) {
    world.mock_api_body = format!(
        r#"{{"tunnels":[{{"name":"cmd","proto":"http","public_url":"http://abc123.ngrok.io","config":{{"addr":"localhost:{port}"}}}}]}}"#
    );
}

#[given("a mock ngrok API response with no tunnels")]
fn ngrok_empty_response(world: &mut NgrokWorld) {
    world.mock_api_body = r#"{"tunnels":[]}"#.to_string();
}

#[given("a mock ngrok API response that is invalid JSON")]
fn ngrok_bad_json(world: &mut NgrokWorld) {
    world.mock_api_body = "not-json".to_string();
}

#[when(expr = "I create a TunnelConfig with ngrok_auto_start true and token {string}")]
fn tunnel_config_with_start(world: &mut NgrokWorld, token: String) {
    world.tunnel_config = Some(TunnelConfig {
        tailscale_funnel: false,
        ngrok_auto_start: true,
        ngrok_auth_token: Some(token),
    });
}

// ── When ──────────────────────────────────────────────────────────────────────

#[when(expr = "I call detect_tunnel for port {int}")]
fn call_detect_tunnel(world: &mut NgrokWorld, port: u16) {
    // Without a real ngrok process the TCP probe will fail → None.
    world.extracted_url = vibecli_cli::ngrok::detect_tunnel(port);
}

#[when(expr = "I parse the ngrok API response for port {int}")]
fn parse_response(world: &mut NgrokWorld, port: u16) {
    let body = world.mock_api_body.clone();
    world.extracted_url = parse_ngrok_api(&body, port);
}

#[when("I create a default TunnelConfig")]
fn create_default_config(world: &mut NgrokWorld) {
    world.tunnel_config = Some(TunnelConfig::default());
}

#[when("I serialise and deserialise the config")]
fn roundtrip_config(world: &mut NgrokWorld) {
    if let Some(cfg) = &world.tunnel_config {
        let json = serde_json::to_string(cfg).expect("serialise");
        world.roundtripped_config = serde_json::from_str(&json).ok();
    }
}

// ── Then ──────────────────────────────────────────────────────────────────────

#[then("the result should be None")]
fn result_is_none(world: &mut NgrokWorld) {
    assert!(world.extracted_url.is_none(),
        "expected None but got {:?}", world.extracted_url);
}

#[then(expr = "the extracted URL should be {string}")]
fn extracted_url_eq(world: &mut NgrokWorld, expected: String) {
    assert_eq!(world.extracted_url.as_deref(), Some(expected.as_str()),
        "URL mismatch");
}

#[then("the extracted URL should be empty")]
fn extracted_url_empty(world: &mut NgrokWorld) {
    assert!(world.extracted_url.is_none(),
        "expected no URL but got {:?}", world.extracted_url);
}

#[then("tailscale_funnel should be false")]
fn ts_funnel_false(world: &mut NgrokWorld) {
    assert!(!world.tunnel_config.as_ref().unwrap().tailscale_funnel);
}

#[then("ngrok_auto_start should be false")]
fn ngrok_start_false(world: &mut NgrokWorld) {
    assert!(!world.tunnel_config.as_ref().unwrap().ngrok_auto_start);
}

#[then("ngrok_auto_start should be true")]
fn ngrok_start_true(world: &mut NgrokWorld) {
    let cfg = world.roundtripped_config.as_ref()
        .or(world.tunnel_config.as_ref())
        .unwrap();
    assert!(cfg.ngrok_auto_start);
}

#[then("ngrok_auth_token should be None")]
fn ngrok_token_none(world: &mut NgrokWorld) {
    assert!(world.tunnel_config.as_ref().unwrap().ngrok_auth_token.is_none());
}

#[then(expr = "ngrok_auth_token should be {string}")]
fn ngrok_token_eq(world: &mut NgrokWorld, expected: String) {
    let cfg = world.roundtripped_config.as_ref()
        .or(world.tunnel_config.as_ref())
        .unwrap();
    assert_eq!(cfg.ngrok_auth_token.as_deref(), Some(expected.as_str()));
}

fn main() {
    futures::executor::block_on(
        NgrokWorld::run("tests/features/ngrok.feature"),
    );
}
