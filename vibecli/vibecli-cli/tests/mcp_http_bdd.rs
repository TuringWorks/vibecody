//! BDD coverage for real MCP Streamable HTTP + OAuth 2.1 PKCE (US-004).

use axum::{
    Router,
    extract::{Form, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Sse, sse::Event as SseEvent},
    routing::{get, post},
};
use cucumber::{World, given, then, when};
use futures::stream::Stream;
use serde::Deserialize;
use std::convert::Infallible;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use vibecli_cli::mcp_http::{
    AuthUrlParams, McpError, McpOAuthClient, McpStreamClient, PkceChallengeV2,
    build_auth_url,
};

#[derive(Clone, Default)]
struct OAuthMock {
    expected_verifier: Option<String>,
    expected_refresh: Option<String>,
    issue_access: String,
    issue_refresh: Option<String>,
}

#[derive(Clone, Default)]
struct StreamMock {
    required_bearer: Option<String>,
    emit_count: usize,
}

#[derive(Default)]
struct MockServers {
    oauth: OAuthMock,
    stream: StreamMock,
}

type SharedMocks = Arc<Mutex<MockServers>>;

#[derive(Default, World)]
pub struct McpWorld {
    pkce: Option<PkceChallengeV2>,
    auth_url: Option<String>,
    oauth_cfg_client_id: Option<String>,
    oauth_cfg_redirect: Option<String>,
    oauth_cfg_scopes: Vec<String>,
    mocks: Option<SharedMocks>,
    server_addr: Option<std::net::SocketAddr>,
    last_token: Option<vibecli_cli::mcp_http::OAuthTokenResponse>,
    last_messages: Vec<vibecli_cli::mcp_http::SseMessage>,
    last_stream_error: Option<String>,
}

impl std::fmt::Debug for McpWorld {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("McpWorld")
            .field("auth_url", &self.auth_url)
            .field("last_token", &self.last_token.as_ref().map(|t| t.access_token.clone()))
            .field("msgs", &self.last_messages.len())
            .field("server_addr", &self.server_addr)
            .finish()
    }
}

// ── Mock HTTP handlers ──────────────────────────────────────────────────────

#[derive(Deserialize)]
struct TokenForm {
    grant_type: String,
    code: Option<String>,
    code_verifier: Option<String>,
    refresh_token: Option<String>,
    #[allow(dead_code)]
    client_id: Option<String>,
}

async fn token_handler(
    State(mocks): State<SharedMocks>,
    Form(f): Form<TokenForm>,
) -> impl IntoResponse {
    let m = mocks.lock().await.oauth.clone();
    match f.grant_type.as_str() {
        "authorization_code" => {
            if let Some(exp) = &m.expected_verifier {
                if f.code_verifier.as_deref() != Some(exp.as_str()) {
                    return (
                        StatusCode::UNAUTHORIZED,
                        axum::Json(serde_json::json!({
                            "error": "invalid_grant",
                            "detail": "code_verifier mismatch"
                        })),
                    )
                        .into_response();
                }
            }
            if f.code.is_none() {
                return (
                    StatusCode::BAD_REQUEST,
                    axum::Json(serde_json::json!({"error":"invalid_request"})),
                )
                    .into_response();
            }
            let mut body = serde_json::json!({
                "access_token": m.issue_access,
                "token_type": "Bearer",
                "expires_in": 3600,
            });
            if let Some(rt) = m.issue_refresh {
                body["refresh_token"] = serde_json::Value::String(rt);
            }
            (StatusCode::OK, axum::Json(body)).into_response()
        }
        "refresh_token" => {
            if let Some(exp) = &m.expected_refresh {
                if f.refresh_token.as_deref() != Some(exp.as_str()) {
                    return (
                        StatusCode::UNAUTHORIZED,
                        axum::Json(serde_json::json!({"error":"invalid_grant"})),
                    )
                        .into_response();
                }
            }
            let body = serde_json::json!({
                "access_token": m.issue_access,
                "token_type": "Bearer",
                "expires_in": 3600,
            });
            (StatusCode::OK, axum::Json(body)).into_response()
        }
        _ => (
            StatusCode::BAD_REQUEST,
            axum::Json(serde_json::json!({"error":"unsupported_grant_type"})),
        )
            .into_response(),
    }
}

async fn stream_handler(
    State(mocks): State<SharedMocks>,
    headers: HeaderMap,
) -> Result<Sse<impl Stream<Item = Result<SseEvent, Infallible>>>, (StatusCode, String)> {
    let s = mocks.lock().await.stream.clone();
    if let Some(req_tok) = &s.required_bearer {
        let got = headers
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .unwrap_or("");
        if got != req_tok {
            return Err((
                StatusCode::UNAUTHORIZED,
                "bearer mismatch".to_string(),
            ));
        }
    }
    let count = s.emit_count;
    let events: Vec<SseEvent> = (0..count)
        .map(|i| {
            let data = if i == 0 {
                "hello"
            } else if i == 1 {
                "world"
            } else {
                "more"
            };
            SseEvent::default().event("message").data(data)
        })
        .collect();
    let stream = futures::stream::iter(events.into_iter().map(Ok::<_, Infallible>));
    Ok(Sse::new(stream))
}

async fn spawn_server(mocks: SharedMocks) -> std::net::SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let app = Router::new()
        .route("/token", post(token_handler))
        .route("/mcp/stream", get(stream_handler))
        .with_state(mocks);
    tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });
    addr
}

// ── Given ───────────────────────────────────────────────────────────────────

#[given(regex = r#"^an OAuth config with client "([^"]+)", redirect "([^"]+)", scopes "([^"]+)"$"#)]
fn given_oauth_config(w: &mut McpWorld, client: String, redirect: String, scopes: String) {
    w.oauth_cfg_client_id = Some(client);
    w.oauth_cfg_redirect = Some(redirect);
    w.oauth_cfg_scopes = scopes.split_whitespace().map(|s| s.to_string()).collect();
}

#[given(regex = r#"^a mock OAuth token server that requires code_verifier "([^"]+)" and issues access "([^"]+)" refresh "([^"]+)"$"#)]
async fn given_oauth_mock(w: &mut McpWorld, verifier: String, access: String, refresh: String) {
    let mocks: SharedMocks = Arc::new(Mutex::new(MockServers::default()));
    {
        let mut m = mocks.lock().await;
        m.oauth.expected_verifier = Some(verifier);
        m.oauth.issue_access = access;
        m.oauth.issue_refresh = Some(refresh);
    }
    let addr = spawn_server(mocks.clone()).await;
    w.mocks = Some(mocks);
    w.server_addr = Some(addr);
}

#[given(regex = r#"^a mock OAuth token server that accepts refresh "([^"]+)" and issues access "([^"]+)"$"#)]
async fn given_oauth_refresh_mock(w: &mut McpWorld, refresh: String, access: String) {
    let mocks: SharedMocks = Arc::new(Mutex::new(MockServers::default()));
    {
        let mut m = mocks.lock().await;
        m.oauth.expected_refresh = Some(refresh);
        m.oauth.issue_access = access;
    }
    let addr = spawn_server(mocks.clone()).await;
    w.mocks = Some(mocks);
    w.server_addr = Some(addr);
}

#[given(regex = r#"^a mock MCP server that requires bearer "([^"]+)" and emits (\d+) SSE messages$"#)]
async fn given_stream_mock(w: &mut McpWorld, bearer: String, count: usize) {
    let mocks: SharedMocks = Arc::new(Mutex::new(MockServers::default()));
    {
        let mut m = mocks.lock().await;
        m.stream.required_bearer = Some(bearer);
        m.stream.emit_count = count;
    }
    let addr = spawn_server(mocks.clone()).await;
    w.mocks = Some(mocks);
    w.server_addr = Some(addr);
}

// ── When ────────────────────────────────────────────────────────────────────

#[when(regex = r#"^a PKCE S256 challenge is generated$"#)]
fn when_pkce(w: &mut McpWorld) {
    w.pkce = Some(PkceChallengeV2::generate());
}

#[when(regex = r#"^the client builds an authorization URL with state "([^"]+)" and a fresh PKCE challenge$"#)]
fn when_build_auth(w: &mut McpWorld, state: String) {
    let pkce = PkceChallengeV2::generate();
    let client = w.oauth_cfg_client_id.clone().expect("client");
    let redirect = w.oauth_cfg_redirect.clone().expect("redirect");
    let scope_refs: Vec<&str> = w.oauth_cfg_scopes.iter().map(|s| s.as_str()).collect();
    let url = build_auth_url(&AuthUrlParams {
        auth_url: "https://idp.example/auth",
        client_id: &client,
        redirect_uri: &redirect,
        scopes: &scope_refs,
        state: &state,
        pkce: &pkce,
    });
    w.auth_url = Some(url);
    w.pkce = Some(pkce);
}

#[when(regex = r#"^the client exchanges code "([^"]+)" with verifier "([^"]+)"$"#)]
async fn when_exchange(w: &mut McpWorld, code: String, verifier: String) {
    let addr = w.server_addr.expect("addr");
    let token_url = format!("http://{}/token", addr);
    let c = McpOAuthClient::new(reqwest::Client::new());
    let tok = c
        .exchange_code(&token_url, &code, &verifier, "client-x", "https://x/cb")
        .await
        .expect("exchange");
    w.last_token = Some(tok);
}

#[when(regex = r#"^the client refreshes with refresh token "([^"]+)"$"#)]
async fn when_refresh(w: &mut McpWorld, refresh: String) {
    let addr = w.server_addr.expect("addr");
    let token_url = format!("http://{}/token", addr);
    let c = McpOAuthClient::new(reqwest::Client::new());
    let tok = c
        .refresh_token(&token_url, &refresh, "client-x")
        .await
        .expect("refresh");
    w.last_token = Some(tok);
}

#[when(regex = r#"^the client opens a stream with token "([^"]+)" and reads at most (\d+) messages$"#)]
async fn when_open_stream(w: &mut McpWorld, token: String, n: usize) {
    let addr = w.server_addr.expect("addr");
    let stream_url = format!("http://{}/mcp/stream", addr);
    let c = McpStreamClient::new(reqwest::Client::new());
    match c.open_stream(&stream_url, &token, n).await {
        Ok(msgs) => w.last_messages = msgs,
        Err(McpError::Unauthorized(e)) => w.last_stream_error = Some(format!("unauthorized: {e}")),
        Err(e) => w.last_stream_error = Some(format!("{e}")),
    }
}

// ── Then ────────────────────────────────────────────────────────────────────

#[then(regex = r#"^the verifier is at least (\d+) base64url characters$"#)]
fn then_verifier_len(w: &mut McpWorld, min: usize) {
    let p = w.pkce.as_ref().expect("pkce");
    assert!(p.code_verifier.len() >= min, "len={}", p.code_verifier.len());
    assert!(!p.code_verifier.contains('+'));
    assert!(!p.code_verifier.contains('/'));
    assert!(!p.code_verifier.contains('='));
}

#[then(regex = r#"^the challenge is the base64url of SHA-256\(verifier\)$"#)]
fn then_s256(w: &mut McpWorld) {
    let p = w.pkce.as_ref().expect("pkce");
    assert_eq!(p.code_challenge, PkceChallengeV2::s256(&p.code_verifier));
}

#[then(regex = r#"^the URL contains "([^"]+)"$"#)]
fn then_url_contains(w: &mut McpWorld, needle: String) {
    let url = w.auth_url.as_ref().expect("url");
    assert!(url.contains(&needle), "url {url} missing {needle}");
}

#[then(regex = r#"^the received access token is "([^"]+)"$"#)]
fn then_access_token(w: &mut McpWorld, expected: String) {
    let t = w.last_token.as_ref().expect("token");
    assert_eq!(t.access_token, expected);
}

#[then(regex = r#"^the received refresh token is "([^"]+)"$"#)]
fn then_refresh_token(w: &mut McpWorld, expected: String) {
    let t = w.last_token.as_ref().expect("token");
    assert_eq!(t.refresh_token.as_deref(), Some(expected.as_str()));
}

#[then(regex = r#"^the stream yields (\d+) messages$"#)]
fn then_msg_count(w: &mut McpWorld, n: usize) {
    assert_eq!(w.last_messages.len(), n, "got {:?}", w.last_messages);
}

#[then(regex = r#"^message (\d+) contains "([^"]+)"$"#)]
fn then_msg_contains(w: &mut McpWorld, idx: usize, needle: String) {
    let m = w.last_messages.get(idx - 1).expect("msg idx");
    assert!(m.data.contains(&needle), "data {:?} missing {needle}", m.data);
}

#[then(regex = r#"^opening the stream returns an authorization error$"#)]
fn then_stream_unauthorized(w: &mut McpWorld) {
    let err = w.last_stream_error.as_ref().expect("err");
    assert!(
        err.contains("unauthorized"),
        "expected unauthorized err, got {err}"
    );
}

#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() {
    McpWorld::run("tests/features/mcp_http.feature").await;
}
