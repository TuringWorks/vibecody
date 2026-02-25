//! HTTP client for a remote `vibe-indexer` service.
//!
//! `RemoteEmbeddingIndex` provides the same `search()` interface as the
//! local `EmbeddingIndex` but delegates all computation to the vibe-indexer
//! daemon running at a configurable URL.
//!
//! # Example
//! ```no_run
//! use vibe_core::index::remote::RemoteEmbeddingIndex;
//! # async fn example() -> anyhow::Result<()> {
//! let client = RemoteEmbeddingIndex::new(
//!     "http://localhost:9999".to_string(),
//!     "/path/to/workspace".to_string(),
//!     None, // optional API key
//! );
//! // Trigger indexing (idempotent — server re-uses cached index if workspace unchanged)
//! let job_id = client.start_indexing().await?;
//! // Wait for completion
//! client.wait_for_index(&job_id, std::time::Duration::from_secs(300)).await?;
//! // Search
//! let hits = client.search("authenticate user", 5).await?;
//! # Ok(()) }
//! ```

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use crate::index::embeddings::SearchHit;

// ── Remote types (mirrors vibe-indexer JSON contracts) ────────────────────────

#[derive(Debug, Deserialize)]
struct IndexResponse {
    job_id: String,
    #[allow(dead_code)]
    message: String,
}

#[derive(Debug, Deserialize)]
struct JobStatus {
    status: String,
    files_indexed: Option<usize>,
    error: Option<String>,
}

#[derive(Debug, Serialize)]
struct SearchRequest<'a> {
    query: &'a str,
    workspace: &'a str,
    limit: usize,
}

#[derive(Debug, Deserialize)]
struct SearchResponse {
    hits: Vec<SearchHit>,
    #[allow(dead_code)]
    total: usize,
}

// ── RemoteEmbeddingIndex ──────────────────────────────────────────────────────

/// HTTP client wrapping a running `vibe-indexer` service.
#[derive(Debug, Clone)]
pub struct RemoteEmbeddingIndex {
    /// Base URL of the vibe-indexer service (e.g. `http://localhost:9999`).
    pub url: String,
    /// Workspace path to index/search (must match what was given to `start_indexing`).
    pub workspace: String,
    /// Optional API key for authenticated deployments.
    pub api_key: Option<String>,
}

impl RemoteEmbeddingIndex {
    /// Create a new client.
    pub fn new(url: String, workspace: String, api_key: Option<String>) -> Self {
        Self { url: url.trim_end_matches('/').to_string(), workspace, api_key }
    }

    /// POST /index — ask the service to index `self.workspace`.
    ///
    /// Returns the job ID that can be polled with [`wait_for_index`].
    pub async fn start_indexing(&self) -> Result<String> {
        let body = serde_json::json!({ "workspace": self.workspace });
        let resp = self
            .client()
            .post(format!("{}/index", self.url))
            .json(&body)
            .send()
            .await
            .context("POST /index failed")?
            .error_for_status()
            .context("indexer returned error")?
            .json::<IndexResponse>()
            .await
            .context("parsing /index response")?;

        Ok(resp.job_id)
    }

    /// Poll `GET /index/status/:id` until the job is complete or fails.
    ///
    /// `timeout` caps total wait time; returns an error if exceeded.
    pub async fn wait_for_index(&self, job_id: &str, timeout: Duration) -> Result<usize> {
        let start = Instant::now();
        let poll_interval = Duration::from_secs(2);

        loop {
            let status: JobStatus = self
                .client()
                .get(format!("{}/index/status/{}", self.url, job_id))
                .send()
                .await
                .context("GET /index/status failed")?
                .error_for_status()
                .context("indexer returned error")?
                .json()
                .await
                .context("parsing status response")?;

            match status.status.as_str() {
                "complete" => return Ok(status.files_indexed.unwrap_or(0)),
                "failed" => {
                    return Err(anyhow!(
                        "Indexing job {} failed: {}",
                        job_id,
                        status.error.unwrap_or_default()
                    ));
                }
                _ => {
                    if start.elapsed() >= timeout {
                        return Err(anyhow!(
                            "Indexing job {} timed out after {:?}",
                            job_id,
                            timeout
                        ));
                    }
                    tokio::time::sleep(poll_interval).await;
                }
            }
        }
    }

    /// POST /search — semantic search delegated to the remote service.
    ///
    /// This has the same signature as `EmbeddingIndex::search` so callers can
    /// swap local vs. remote transparently.
    pub async fn search(&self, query: &str, k: usize) -> Result<Vec<SearchHit>> {
        let req = SearchRequest { query, workspace: &self.workspace, limit: k };
        let resp: SearchResponse = self
            .client()
            .post(format!("{}/search", self.url))
            .json(&req)
            .send()
            .await
            .context("POST /search failed")?
            .error_for_status()
            .context("indexer returned error")?
            .json()
            .await
            .context("parsing search response")?;

        Ok(resp.hits)
    }

    /// Check if the remote service is reachable.
    pub async fn is_healthy(&self) -> bool {
        self.client()
            .get(format!("{}/health", self.url))
            .send()
            .await
            .is_ok()
    }

    fn client(&self) -> reqwest::Client {
        let mut builder = reqwest::Client::builder();
        if let Some(key) = &self.api_key {
            let mut headers = reqwest::header::HeaderMap::new();
            headers.insert(
                "X-Api-Key",
                reqwest::header::HeaderValue::from_str(key).expect("valid header value"),
            );
            builder = builder.default_headers(headers);
        }
        builder.build().expect("reqwest client")
    }
}

// ── IndexBackend ─────────────────────────────────────────────────────────────

/// Selects between a locally-computed embedding index and a remote service.
///
/// Add `[index] backend = "remote"` + `url = "http://…"` to `~/.vibecli/config.toml`
/// or `.vibecli/config.toml` to opt into the remote backend.
#[derive(Debug, Clone)]
pub enum IndexBackend {
    /// Use the local in-process `EmbeddingIndex`.
    Local,
    /// Delegate to a `vibe-indexer` HTTP service.
    Remote {
        /// Base URL of the service (no trailing slash).
        url: String,
        /// Optional API key for authenticated deployments.
        api_key: Option<String>,
    },
}

impl Default for IndexBackend {
    fn default() -> Self { Self::Local }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remote_client_defaults() {
        let client = RemoteEmbeddingIndex::new(
            "http://localhost:9999/".to_string(),
            "/workspace".to_string(),
            None,
        );
        // URL should have trailing slash stripped
        assert_eq!(client.url, "http://localhost:9999");
        assert!(client.api_key.is_none());
    }

    #[test]
    fn index_backend_default_is_local() {
        let backend = IndexBackend::default();
        assert!(matches!(backend, IndexBackend::Local));
    }

    #[test]
    fn remote_backend_stores_url_and_key() {
        let backend = IndexBackend::Remote {
            url: "http://indexer.example.com".to_string(),
            api_key: Some("secret".to_string()),
        };
        if let IndexBackend::Remote { url, api_key } = backend {
            assert_eq!(url, "http://indexer.example.com");
            assert_eq!(api_key.as_deref(), Some("secret"));
        }
    }
}
