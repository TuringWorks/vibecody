//! BDD coverage for the real HTTP web-grounding backends (US-001).
//!
//! Exercises `WebGroundingEngine::search_async` against a mock axum server that
//! mimics SearXNG's JSON schema, plus error paths for transport failures and
//! missing API keys.

use axum::{Router, extract::State, http::StatusCode, response::IntoResponse, routing::get};
use cucumber::{World, given, then, when};
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::net::TcpListener;
use vibecli_cli::web_grounding::{SearchConfig, SearchProvider, SearchResult, WebGroundingEngine};
use vibecli_cli::web_grounding_backend::{
    BraveBackend, SearchBackend, SearxngBackend,
};

#[derive(Clone, Default)]
struct MockState {
    canned_title: Arc<std::sync::Mutex<String>>,
    status: Arc<std::sync::Mutex<u16>>,
    hits: Arc<AtomicUsize>,
}

async fn searxng_handler(State(s): State<MockState>) -> impl IntoResponse {
    s.hits.fetch_add(1, Ordering::SeqCst);
    let status = *s.status.lock().unwrap();
    if status != 200 {
        return (
            StatusCode::from_u16(status).unwrap(),
            "service unavailable".to_string(),
        );
    }
    let title = s.canned_title.lock().unwrap().clone();
    let body = format!(
        r#"{{"results":[{{"title":"{title}","url":"https://mock.example/1","content":"mock snippet","score":0.9}}]}}"#
    );
    (StatusCode::OK, body)
}

#[derive(Default, World)]
pub struct GroundingWorld {
    base_url: Option<String>,
    mock_state: Option<MockState>,
    engine: Option<WebGroundingEngine>,
    backend: Option<Box<dyn SearchBackend>>,
    last_results: Option<Result<Vec<SearchResult>, String>>,
}

impl std::fmt::Debug for GroundingWorld {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GroundingWorld")
            .field("base_url", &self.base_url)
            .field("last_results_is_ok", &self.last_results.as_ref().map(|r| r.is_ok()))
            .finish()
    }
}

async fn start_mock_server(state: MockState) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr: SocketAddr = listener.local_addr().expect("local_addr");
    let app = Router::new()
        .route("/search", get(searxng_handler))
        .with_state(state);
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    format!("http://{}/search", addr)
}

// ── Given ───────────────────────────────────────────────────────────────────

#[given(regex = r#"^a mock SearXNG server that returns one result titled "([^"]+)"$"#)]
async fn given_searxng_returning(w: &mut GroundingWorld, title: String) {
    let state = MockState {
        canned_title: Arc::new(std::sync::Mutex::new(title)),
        status: Arc::new(std::sync::Mutex::new(200)),
        hits: Arc::new(AtomicUsize::new(0)),
    };
    let url = start_mock_server(state.clone()).await;
    w.base_url = Some(url);
    w.mock_state = Some(state);
}

#[given(regex = r#"^a mock SearXNG server that always returns HTTP 503$"#)]
async fn given_searxng_503(w: &mut GroundingWorld) {
    let state = MockState {
        canned_title: Arc::new(std::sync::Mutex::new(String::from("unused"))),
        status: Arc::new(std::sync::Mutex::new(503)),
        hits: Arc::new(AtomicUsize::new(0)),
    };
    let url = start_mock_server(state.clone()).await;
    w.base_url = Some(url);
    w.mock_state = Some(state);
}

#[given(regex = r#"^a web grounding engine configured to target that mock server$"#)]
fn given_engine_targets_mock(w: &mut GroundingWorld) {
    let base_url = w.base_url.clone().expect("mock server must be set first");
    let config = SearchConfig {
        provider: SearchProvider::SearXNG,
        api_key: None,
        base_url: Some(base_url),
        max_results: 5,
        cache_ttl_secs: 60,
        rate_limit_per_min: 30,
        privacy_mode: false,
    };
    w.engine = Some(WebGroundingEngine::new(config));
    w.backend = Some(Box::new(SearxngBackend::new(reqwest::Client::new())));
}

#[given(regex = r#"^a web grounding engine configured for Brave without an API key$"#)]
fn given_engine_brave_no_key(w: &mut GroundingWorld) {
    let config = SearchConfig {
        provider: SearchProvider::Brave,
        api_key: None,
        base_url: None,
        max_results: 5,
        cache_ttl_secs: 60,
        rate_limit_per_min: 30,
        privacy_mode: false,
    };
    w.engine = Some(WebGroundingEngine::new(config));
    w.backend = Some(Box::new(BraveBackend::new(reqwest::Client::new())));
}

// ── When ────────────────────────────────────────────────────────────────────

#[when(regex = r#"^the engine searches for "([^"]+)"$"#)]
async fn when_search_once(w: &mut GroundingWorld, q: String) {
    let engine = w.engine.as_mut().expect("engine");
    let backend = w.backend.as_ref().expect("backend");
    let res = engine.search_async(&q, backend.as_ref()).await;
    w.last_results = Some(res);
}

#[when(regex = r#"^the engine searches for "([^"]+)" twice$"#)]
async fn when_search_twice(w: &mut GroundingWorld, q: String) {
    let engine = w.engine.as_mut().expect("engine");
    let backend = w.backend.as_ref().expect("backend");
    let _ = engine.search_async(&q, backend.as_ref()).await;
    let res = engine.search_async(&q, backend.as_ref()).await;
    w.last_results = Some(res);
}

// ── Then ────────────────────────────────────────────────────────────────────

#[then(regex = r#"^the engine returns (\d+) results?$"#)]
fn then_returns_n(w: &mut GroundingWorld, n: usize) {
    let results = w.last_results.as_ref().expect("ran").as_ref().expect("ok");
    assert_eq!(results.len(), n, "expected {n} results, got {results:?}");
}

#[then(regex = r#"^the first result title is "([^"]+)"$"#)]
fn then_first_title(w: &mut GroundingWorld, title: String) {
    let results = w.last_results.as_ref().expect("ran").as_ref().expect("ok");
    assert_eq!(results[0].title, title);
}

#[then(regex = r#"^the first result source is SearXNG$"#)]
fn then_first_source_searxng(w: &mut GroundingWorld) {
    let results = w.last_results.as_ref().expect("ran").as_ref().expect("ok");
    assert_eq!(results[0].source, SearchProvider::SearXNG);
}

#[then(regex = r#"^the engine cache has (\d+) entr(?:y|ies)$"#)]
fn then_cache_entries(w: &mut GroundingWorld, n: usize) {
    let engine = w.engine.as_ref().expect("engine");
    assert_eq!(engine.cache.entries.len(), n);
}

#[then(regex = r#"^the engine metrics report (\d+) total search(?:es)? and (\d+) cache hits?$"#)]
fn then_metrics(w: &mut GroundingWorld, total: u32, hits: u32) {
    let engine = w.engine.as_ref().expect("engine");
    assert_eq!(engine.metrics.total_searches, total);
    assert_eq!(engine.metrics.cache_hits, hits);
}

#[then(regex = r#"^the mock server received exactly (\d+) HTTP requests?$"#)]
fn then_mock_hits(w: &mut GroundingWorld, n: usize) {
    let state = w.mock_state.as_ref().expect("mock");
    assert_eq!(state.hits.load(Ordering::SeqCst), n);
}

#[then(regex = r#"^the engine returns an error containing "([^"]+)"$"#)]
fn then_error_contains(w: &mut GroundingWorld, needle: String) {
    let err = w
        .last_results
        .as_ref()
        .expect("ran")
        .as_ref()
        .err()
        .unwrap_or_else(|| panic!("expected error, got Ok"));
    assert!(err.contains(&needle), "error {err:?} did not contain {needle:?}");
}

#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() {
    GroundingWorld::run("tests/features/web_grounding_backend.feature").await;
}
