//! Real HTTP search backends for `web_grounding::WebGroundingEngine`.
//!
//! The engine itself stays provider-agnostic. Each [`SearchBackend`] implementation
//! owns the wire format for one search provider and translates it into the engine's
//! [`SearchResult`] type. Tests can inject a [`StubBackend`] so the engine can be
//! exercised without network access.

use crate::web_grounding::{ResultContentType, SearchConfig, SearchProvider, SearchResult};
use async_trait::async_trait;
use serde::Deserialize;

/// A pluggable search backend. One impl per supported provider, plus test stubs.
#[async_trait]
pub trait SearchBackend: Send + Sync + std::fmt::Debug {
    /// Execute a search against the backing provider and return normalized results.
    ///
    /// Implementations must not implement caching, rate limiting, or classification —
    /// those are the engine's job.
    async fn search(
        &self,
        query: &str,
        config: &SearchConfig,
    ) -> Result<Vec<SearchResult>, String>;

    /// Which provider this backend serves. Used for telemetry + `SearchResult.source`.
    fn provider(&self) -> SearchProvider;
}

/// Test backend that returns a canned set of results.
#[derive(Debug, Clone)]
pub struct StubBackend {
    pub provider: SearchProvider,
    pub results: Vec<SearchResult>,
}

impl StubBackend {
    pub fn new(provider: SearchProvider, results: Vec<SearchResult>) -> Self {
        Self { provider, results }
    }
}

#[async_trait]
impl SearchBackend for StubBackend {
    async fn search(
        &self,
        _query: &str,
        _config: &SearchConfig,
    ) -> Result<Vec<SearchResult>, String> {
        Ok(self.results.clone())
    }

    fn provider(&self) -> SearchProvider {
        self.provider.clone()
    }
}

// ── SearXNG ─────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct SearxngResponse {
    #[serde(default)]
    results: Vec<SearxngResult>,
}

#[derive(Debug, Deserialize)]
struct SearxngResult {
    title: String,
    url: String,
    #[serde(default)]
    content: String,
    #[serde(default)]
    score: Option<f64>,
}

/// Backend for a self-hosted SearXNG instance. No API key required; caller
/// configures `base_url` in [`SearchConfig`].
#[derive(Debug)]
pub struct SearxngBackend {
    client: reqwest::Client,
}

impl SearxngBackend {
    pub fn new(client: reqwest::Client) -> Self {
        Self { client }
    }
}

pub(crate) fn parse_searxng_response(
    body: &str,
    now: u64,
) -> Result<Vec<SearchResult>, String> {
    let parsed: SearxngResponse = serde_json::from_str(body)
        .map_err(|e| format!("SearXNG JSON parse error: {e}"))?;
    Ok(parsed
        .results
        .into_iter()
        .map(|r| SearchResult {
            title: r.title,
            url: r.url,
            snippet: r.content,
            source: SearchProvider::SearXNG,
            relevance_score: r.score.unwrap_or(0.5).clamp(0.0, 1.0),
            fetched_at: now,
            content_type: ResultContentType::Unknown,
        })
        .collect())
}

#[async_trait]
impl SearchBackend for SearxngBackend {
    async fn search(
        &self,
        query: &str,
        config: &SearchConfig,
    ) -> Result<Vec<SearchResult>, String> {
        let base = config
            .base_url
            .as_deref()
            .unwrap_or_else(|| SearchProvider::SearXNG.default_endpoint());
        let resp = self
            .client
            .get(base)
            .query(&[("q", query), ("format", "json")])
            .send()
            .await
            .map_err(|e| format!("SearXNG request failed: {e}"))?;
        let status = resp.status();
        let body = resp
            .text()
            .await
            .map_err(|e| format!("SearXNG response read failed: {e}"))?;
        if !status.is_success() {
            return Err(format!("SearXNG HTTP {status}: {body}"));
        }
        let now = chrono::Utc::now().timestamp() as u64;
        parse_searxng_response(&body, now)
    }

    fn provider(&self) -> SearchProvider {
        SearchProvider::SearXNG
    }
}

// ── Brave ───────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct BraveResponse {
    #[serde(default)]
    web: Option<BraveWeb>,
}

#[derive(Debug, Deserialize)]
struct BraveWeb {
    #[serde(default)]
    results: Vec<BraveResult>,
}

#[derive(Debug, Deserialize)]
struct BraveResult {
    title: String,
    url: String,
    #[serde(default)]
    description: String,
}

#[derive(Debug)]
pub struct BraveBackend {
    client: reqwest::Client,
}

impl BraveBackend {
    pub fn new(client: reqwest::Client) -> Self {
        Self { client }
    }
}

pub(crate) fn parse_brave_response(
    body: &str,
    now: u64,
) -> Result<Vec<SearchResult>, String> {
    let parsed: BraveResponse = serde_json::from_str(body)
        .map_err(|e| format!("Brave JSON parse error: {e}"))?;
    let results = parsed
        .web
        .map(|w| w.results)
        .unwrap_or_default()
        .into_iter()
        .map(|r| SearchResult {
            title: r.title,
            url: r.url,
            snippet: r.description,
            source: SearchProvider::Brave,
            relevance_score: 0.5,
            fetched_at: now,
            content_type: ResultContentType::Unknown,
        })
        .collect();
    Ok(results)
}

#[async_trait]
impl SearchBackend for BraveBackend {
    async fn search(
        &self,
        query: &str,
        config: &SearchConfig,
    ) -> Result<Vec<SearchResult>, String> {
        let api_key = config
            .api_key
            .as_deref()
            .ok_or_else(|| "Brave provider requires an API key".to_string())?;
        let base = config
            .base_url
            .as_deref()
            .unwrap_or_else(|| SearchProvider::Brave.default_endpoint());
        let resp = self
            .client
            .get(base)
            .header("X-Subscription-Token", api_key)
            .header("Accept", "application/json")
            .query(&[("q", query)])
            .send()
            .await
            .map_err(|e| format!("Brave request failed: {e}"))?;
        let status = resp.status();
        let body = resp
            .text()
            .await
            .map_err(|e| format!("Brave response read failed: {e}"))?;
        if !status.is_success() {
            return Err(format!("Brave HTTP {status}: {body}"));
        }
        let now = chrono::Utc::now().timestamp() as u64;
        parse_brave_response(&body, now)
    }

    fn provider(&self) -> SearchProvider {
        SearchProvider::Brave
    }
}

// ── Tavily ──────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct TavilyResponse {
    #[serde(default)]
    results: Vec<TavilyResult>,
}

#[derive(Debug, Deserialize)]
struct TavilyResult {
    title: String,
    url: String,
    #[serde(default)]
    content: String,
    #[serde(default)]
    score: Option<f64>,
}

#[derive(Debug)]
pub struct TavilyBackend {
    client: reqwest::Client,
}

impl TavilyBackend {
    pub fn new(client: reqwest::Client) -> Self {
        Self { client }
    }
}

pub(crate) fn parse_tavily_response(
    body: &str,
    now: u64,
) -> Result<Vec<SearchResult>, String> {
    let parsed: TavilyResponse = serde_json::from_str(body)
        .map_err(|e| format!("Tavily JSON parse error: {e}"))?;
    Ok(parsed
        .results
        .into_iter()
        .map(|r| SearchResult {
            title: r.title,
            url: r.url,
            snippet: r.content,
            source: SearchProvider::Tavily,
            relevance_score: r.score.unwrap_or(0.5).clamp(0.0, 1.0),
            fetched_at: now,
            content_type: ResultContentType::Unknown,
        })
        .collect())
}

#[async_trait]
impl SearchBackend for TavilyBackend {
    async fn search(
        &self,
        query: &str,
        config: &SearchConfig,
    ) -> Result<Vec<SearchResult>, String> {
        let api_key = config
            .api_key
            .as_deref()
            .ok_or_else(|| "Tavily provider requires an API key".to_string())?;
        let base = config
            .base_url
            .as_deref()
            .unwrap_or_else(|| SearchProvider::Tavily.default_endpoint());
        let body = serde_json::json!({
            "api_key": api_key,
            "query": query,
            "max_results": config.max_results,
        });
        let resp = self
            .client
            .post(base)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Tavily request failed: {e}"))?;
        let status = resp.status();
        let text = resp
            .text()
            .await
            .map_err(|e| format!("Tavily response read failed: {e}"))?;
        if !status.is_success() {
            return Err(format!("Tavily HTTP {status}: {text}"));
        }
        let now = chrono::Utc::now().timestamp() as u64;
        parse_tavily_response(&text, now)
    }

    fn provider(&self) -> SearchProvider {
        SearchProvider::Tavily
    }
}

// ── Factory ─────────────────────────────────────────────────────────────────

/// Build a backend matching the provider in `config`. Returns an error for providers
/// that do not yet have a live implementation (Google, Bing, DuckDuckGo).
pub fn backend_for(
    config: &SearchConfig,
    client: reqwest::Client,
) -> Result<Box<dyn SearchBackend>, String> {
    match config.provider {
        SearchProvider::SearXNG => Ok(Box::new(SearxngBackend::new(client))),
        SearchProvider::Brave => Ok(Box::new(BraveBackend::new(client))),
        SearchProvider::Tavily => Ok(Box::new(TavilyBackend::new(client))),
        ref p => Err(format!(
            "{:?} provider is not yet supported by a live backend",
            p
        )),
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_config(provider: SearchProvider) -> SearchConfig {
        SearchConfig {
            provider,
            api_key: Some("test-key".to_string()),
            base_url: None,
            max_results: 5,
            cache_ttl_secs: 60,
            rate_limit_per_min: 30,
            privacy_mode: false,
        }
    }

    #[test]
    fn searxng_response_parses_multiple_results() {
        let body = r#"{
            "results": [
                {"title": "Rust async guide", "url": "https://rust-lang.org/async", "content": "An intro", "score": 0.92},
                {"title": "Tokio tutorial", "url": "https://tokio.rs/tutorial", "content": "Async runtime"}
            ]
        }"#;
        let parsed = parse_searxng_response(body, 100).expect("parse ok");
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].title, "Rust async guide");
        assert_eq!(parsed[0].url, "https://rust-lang.org/async");
        assert_eq!(parsed[0].source, SearchProvider::SearXNG);
        assert!((parsed[0].relevance_score - 0.92).abs() < 1e-6);
        assert_eq!(parsed[0].fetched_at, 100);
        // Missing score falls back to 0.5
        assert!((parsed[1].relevance_score - 0.5).abs() < 1e-6);
    }

    #[test]
    fn brave_response_parses_web_results() {
        let body = r#"{
            "web": {
                "results": [
                    {"title": "Brave hit", "url": "https://example.com/1", "description": "desc"}
                ]
            }
        }"#;
        let parsed = parse_brave_response(body, 200).expect("parse ok");
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].title, "Brave hit");
        assert_eq!(parsed[0].source, SearchProvider::Brave);
        assert_eq!(parsed[0].snippet, "desc");
    }

    #[test]
    fn brave_response_missing_web_is_empty() {
        let body = r#"{}"#;
        let parsed = parse_brave_response(body, 200).expect("parse ok");
        assert!(parsed.is_empty());
    }

    #[test]
    fn tavily_response_parses_results() {
        let body = r#"{
            "results": [
                {"title": "T1", "url": "https://t.example/1", "content": "body", "score": 0.88}
            ]
        }"#;
        let parsed = parse_tavily_response(body, 300).expect("parse ok");
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].source, SearchProvider::Tavily);
        assert!((parsed[0].relevance_score - 0.88).abs() < 1e-6);
    }

    #[test]
    fn searxng_malformed_json_returns_error() {
        let err = parse_searxng_response("not-json", 0).unwrap_err();
        assert!(err.contains("SearXNG JSON parse error"));
    }

    #[tokio::test]
    async fn stub_backend_returns_configured_results() {
        let stub = StubBackend::new(
            SearchProvider::SearXNG,
            vec![SearchResult {
                title: "t".into(),
                url: "https://e.com".into(),
                snippet: "s".into(),
                source: SearchProvider::SearXNG,
                relevance_score: 0.7,
                fetched_at: 0,
                content_type: ResultContentType::Unknown,
            }],
        );
        let out = stub
            .search("ignored", &sample_config(SearchProvider::SearXNG))
            .await
            .unwrap();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].title, "t");
    }

    #[test]
    fn backend_for_rejects_unsupported_providers() {
        let cfg = sample_config(SearchProvider::Google);
        let client = reqwest::Client::new();
        let err = backend_for(&cfg, client).unwrap_err();
        assert!(err.contains("not yet supported"));
    }

    #[test]
    fn backend_for_returns_tavily_for_tavily_config() {
        let cfg = sample_config(SearchProvider::Tavily);
        let client = reqwest::Client::new();
        let backend = backend_for(&cfg, client).expect("build backend");
        assert_eq!(backend.provider(), SearchProvider::Tavily);
    }
}
