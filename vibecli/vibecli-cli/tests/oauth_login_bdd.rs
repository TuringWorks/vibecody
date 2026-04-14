/*!
 * BDD tests for oauth_login using Cucumber.
 * Run with: cargo test --test oauth_login_bdd
 *
 * The module is compiled directly via #[path] because the lib.rs declaration
 * is managed separately; this keeps the BDD harness self-contained.
 */
#[path = "../src/oauth_login.rs"]
mod oauth_login;

use cucumber::{World, given, then, when};
use std::time::{SystemTime, UNIX_EPOCH};
use oauth_login::{
    NoopCallbacks, OAuthCredentials, OAuthFlowResult, OAuthManager, OAuthProvider,
};

// ---------------------------------------------------------------------------
// World
// ---------------------------------------------------------------------------

#[derive(Debug, Default, World)]
pub struct OAuthWorld {
    manager: Option<OAuthManager>,
    /// Last token-expiry check result (true = expired).
    is_expired: bool,
    /// The token string returned by `access_token_if_valid`, if any.
    valid_token: Option<String>,
    /// Providers listed as logged-in.
    logged_in: Vec<String>,
    /// Result of the most recent device-flow simulation.
    flow_result: Option<String>, // "success" | "cancelled" | "error"
    /// Auth header built in the last step.
    auth_header: Option<String>,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn parse_provider(s: &str) -> OAuthProvider {
    OAuthProvider::from_str(s)
        .unwrap_or_else(|| panic!("unknown provider slug: {}", s))
}

fn provider_slug(p: &OAuthProvider) -> &'static str {
    match p {
        OAuthProvider::AnthropicClaude => "anthropic_claude",
        OAuthProvider::GitHubCopilot => "github_copilot",
        OAuthProvider::GoogleGeminiCli => "google_gemini_cli",
        OAuthProvider::OpenAICodex => "openai_codex",
    }
}

// ---------------------------------------------------------------------------
// Given
// ---------------------------------------------------------------------------

#[given(expr = "a fresh OAuth token for provider {string} that expires in {int} seconds")]
fn given_fresh_token(world: &mut OAuthWorld, provider_str: String, expires_in: u64) {
    let provider = parse_provider(&provider_str);
    let expires_at_ms = now_ms() + expires_in * 1_000;

    let creds = OAuthCredentials {
        provider,
        access_token: "tok_fresh_abc".to_string(),
        refresh_token: Some("refresh_fresh".to_string()),
        expires_at_ms: Some(expires_at_ms),
        scopes: vec!["read".to_string()],
        account_email: Some("user@example.com".to_string()),
    };

    let mgr = world.manager.get_or_insert_with(OAuthManager::new);
    mgr.store_credentials(creds);
}

#[given(expr = "an expired OAuth token for provider {string}")]
fn given_expired_token(world: &mut OAuthWorld, provider_str: String) {
    let provider = parse_provider(&provider_str);

    let creds = OAuthCredentials {
        provider,
        access_token: "tok_expired_xyz".to_string(),
        refresh_token: None,
        expires_at_ms: Some(now_ms() - 1_000), // 1 second in the past
        scopes: vec![],
        account_email: None,
    };

    let mgr = world.manager.get_or_insert_with(OAuthManager::new);
    mgr.store_credentials(creds);
}

#[given("an empty OAuth manager")]
fn given_empty_manager(world: &mut OAuthWorld) {
    world.manager = Some(OAuthManager::new());
}

// ---------------------------------------------------------------------------
// When
// ---------------------------------------------------------------------------

#[when(expr = "I check whether the token is expired")]
fn when_check_expired(world: &mut OAuthWorld) {
    // We inspect the first provider that has credentials stored.
    // In our scenarios only one provider is always set up at a time in
    // the single-provider steps; for multi-provider steps this when-step
    // is not invoked.
    let mgr = world.manager.as_ref().expect("manager not initialised");

    // Try each provider in a deterministic order.
    for provider in &[
        OAuthProvider::AnthropicClaude,
        OAuthProvider::GitHubCopilot,
        OAuthProvider::GoogleGeminiCli,
        OAuthProvider::OpenAICodex,
    ] {
        if let Some(creds) = mgr.get_credentials(provider) {
            world.is_expired = creds.is_expired();
            world.valid_token = creds.access_token_if_valid().map(str::to_string);
            return;
        }
    }
    world.is_expired = true;
    world.valid_token = None;
}

#[when("I list providers that are logged in")]
fn when_list_logged_in(world: &mut OAuthWorld) {
    let mgr = world.manager.as_ref().expect("manager not initialised");
    world.logged_in = mgr
        .list_logged_in()
        .into_iter()
        .map(|p| provider_slug(p).to_string())
        .collect();
}

#[when(expr = "I simulate a device flow for provider {string} with mock token {string}")]
fn when_simulate_device_flow(
    world: &mut OAuthWorld,
    provider_str: String,
    mock_token: String,
) {
    let provider = parse_provider(&provider_str);
    let mgr = world.manager.as_mut().expect("manager not initialised");

    let result = mgr.simulate_device_flow(provider, &mock_token, &NoopCallbacks);
    world.flow_result = Some(match result {
        OAuthFlowResult::Success(_) => "success".to_string(),
        OAuthFlowResult::Cancelled => "cancelled".to_string(),
        OAuthFlowResult::Error(e) => format!("error:{}", e),
    });
}

#[when(expr = "I build the auth header for provider {string} with fallback key {string}")]
fn when_build_auth_header(
    world: &mut OAuthWorld,
    provider_str: String,
    fallback: String,
) {
    let provider = parse_provider(&provider_str);
    let mgr = world.manager.as_ref().expect("manager not initialised");
    world.auth_header = mgr.auth_header(&provider, Some(&fallback));
}

// ---------------------------------------------------------------------------
// Then
// ---------------------------------------------------------------------------

#[then("the token should not be expired")]
fn then_not_expired(world: &mut OAuthWorld) {
    assert!(
        !world.is_expired,
        "expected token to be fresh but it was expired"
    );
}

#[then("the token should be expired")]
fn then_expired(world: &mut OAuthWorld) {
    assert!(
        world.is_expired,
        "expected token to be expired but it was fresh"
    );
}

#[then(expr = "the valid token should equal {string}")]
fn then_valid_token_eq(world: &mut OAuthWorld, expected: String) {
    assert_eq!(
        world.valid_token.as_deref(),
        Some(expected.as_str()),
        "valid_token mismatch"
    );
}

#[then("no valid token should be returned")]
fn then_no_valid_token(world: &mut OAuthWorld) {
    assert!(
        world.valid_token.is_none(),
        "expected no valid token but got {:?}",
        world.valid_token
    );
}

#[then(expr = "the logged-in list should contain {string}")]
fn then_logged_in_contains(world: &mut OAuthWorld, provider_str: String) {
    assert!(
        world.logged_in.contains(&provider_str),
        "logged-in list {:?} does not contain {:?}",
        world.logged_in,
        provider_str
    );
}

#[then(expr = "the logged-in list should not contain {string}")]
fn then_logged_in_not_contains(world: &mut OAuthWorld, provider_str: String) {
    assert!(
        !world.logged_in.contains(&provider_str),
        "logged-in list {:?} should not contain {:?}",
        world.logged_in,
        provider_str
    );
}

#[then("the flow result should be success")]
fn then_flow_success(world: &mut OAuthWorld) {
    assert_eq!(
        world.flow_result.as_deref(),
        Some("success"),
        "expected flow result 'success' but got {:?}",
        world.flow_result
    );
}

#[then(expr = "provider {string} should be logged in")]
fn then_provider_logged_in(world: &mut OAuthWorld, provider_str: String) {
    let provider = parse_provider(&provider_str);
    let mgr = world.manager.as_ref().expect("manager not initialised");
    assert!(
        mgr.is_logged_in(&provider),
        "expected provider {:?} to be logged in",
        provider_str
    );
}

#[then(expr = "the valid token for provider {string} should equal {string}")]
fn then_provider_token_eq(
    world: &mut OAuthWorld,
    provider_str: String,
    expected: String,
) {
    let provider = parse_provider(&provider_str);
    let mgr = world.manager.as_ref().expect("manager not initialised");
    assert_eq!(
        mgr.valid_token(&provider),
        Some(expected.as_str()),
        "token mismatch for provider {:?}",
        provider_str
    );
}

#[then(expr = "the auth header should equal {string}")]
fn then_auth_header_eq(world: &mut OAuthWorld, expected: String) {
    assert_eq!(
        world.auth_header.as_deref(),
        Some(expected.as_str()),
        "auth header mismatch"
    );
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

fn main() {
    futures::executor::block_on(OAuthWorld::run(
        "tests/features/oauth_login.feature",
    ));
}
