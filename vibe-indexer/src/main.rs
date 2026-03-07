//! vibe-indexer — Remote codebase indexing HTTP service.
//!
//! Exposes an Axum HTTP server that wraps `EmbeddingIndex` from `vibe-core`.
//! Designed to run as a sidecar for large monorepos where the VibeUI/VibeCLI
//! clients need a shared, persistently-warm semantic search index.
//!
//! # Endpoints
//!
//! | Method | Path                  | Description                          |
//! |--------|-----------------------|--------------------------------------|
//! | POST   | `/index`              | Start a new indexing job             |
//! | GET    | `/index/status/:id`   | Poll job progress                    |
//! | POST   | `/search`             | Semantic search over indexed content |
//! | GET    | `/health`             | Liveness probe                       |
//!
//! # Quick start
//! ```bash
//! vibe-indexer --port 9999 --provider ollama --model nomic-embed-text
//! ```

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tracing::{info, warn};
use vibe_core::index::embeddings::{EmbeddingIndex, EmbeddingProvider, SearchHit};

// ── State ─────────────────────────────────────────────────────────────────────

/// A single indexing job.
#[derive(Debug, Clone, Serialize)]
pub struct IndexJob {
    pub id: String,
    pub workspace: String,
    pub status: JobStatus,
    pub started_at: u64,
    pub finished_at: Option<u64>,
    pub files_indexed: usize,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum JobStatus {
    Running,
    Complete,
    Failed,
}

/// Shared server state.
pub struct AppState {
    /// Active embedding provider config.
    provider: EmbeddingProvider,
    /// Completed indexes, keyed by workspace path string.
    indexes: RwLock<HashMap<String, EmbeddingIndex>>,
    /// All jobs (including running/failed).
    jobs: RwLock<HashMap<String, IndexJob>>,
    /// Directory where completed indexes are persisted (`~/.vibe-indexer/indexes/`).
    persist_dir: PathBuf,
}

// ── Request / response types ──────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct IndexRequest {
    /// Absolute or relative path to the workspace root to index.
    pub workspace: String,
}

#[derive(Serialize)]
pub struct IndexResponse {
    pub job_id: String,
    pub message: String,
}

#[derive(Deserialize)]
pub struct SearchRequest {
    /// The natural-language query.
    pub query: String,
    /// Which indexed workspace to search (same path given to POST /index).
    pub workspace: String,
    /// Maximum number of results to return (default 10).
    #[serde(default = "default_limit")]
    pub limit: usize,
}

fn default_limit() -> usize { 10 }

#[derive(Serialize)]
pub struct SearchResponse {
    pub hits: Vec<SearchHit>,
    pub total: usize,
}

// ── Handlers ──────────────────────────────────────────────────────────────────

/// GET /health — liveness probe.
async fn health() -> impl IntoResponse {
    Json(serde_json::json!({ "status": "ok", "service": "vibe-indexer" }))
}

/// POST /index — kick off an async indexing job.
async fn start_index(
    State(state): State<Arc<AppState>>,
    Json(req): Json<IndexRequest>,
) -> impl IntoResponse {
    let job_id = uuid::Uuid::new_v4().to_string();
    let workspace = req.workspace.clone();
    let now = unix_ms();

    let job = IndexJob {
        id: job_id.clone(),
        workspace: workspace.clone(),
        status: JobStatus::Running,
        started_at: now,
        finished_at: None,
        files_indexed: 0,
        error: None,
    };

    {
        let mut jobs = state.jobs.write().await;
        jobs.insert(job_id.clone(), job);
    }

    info!("Starting index job {} for workspace: {}", job_id, workspace);

    // Spawn background task
    let state_clone = state.clone();
    let job_id_clone = job_id.clone();
    tokio::spawn(async move {
        let workspace_path = PathBuf::from(&workspace);
        let result = EmbeddingIndex::build(&workspace_path, &state_clone.provider).await;

        let mut jobs = state_clone.jobs.write().await;
        if let Some(job) = jobs.get_mut(&job_id_clone) {
            match result {
                Ok(index) => {
                    let count = index.chunk_count();
                    job.status = JobStatus::Complete;
                    job.finished_at = Some(unix_ms());
                    job.files_indexed = count;
                    info!("Job {} complete: {} documents indexed", job_id_clone, count);
                    drop(jobs);

                    // Persist to disk so the index survives restarts
                    let encoded = urlencoding_encode(&workspace);
                    let save_path = state_clone.persist_dir.join(format!("{}.json", encoded));
                    if let Err(e) = index.save(&save_path) {
                        warn!("Could not persist index for {}: {}", workspace, e);
                    } else {
                        info!("Persisted index to {}", save_path.display());
                    }

                    state_clone
                        .indexes
                        .write()
                        .await
                        .insert(workspace.clone(), index);
                }
                Err(e) => {
                    warn!("Job {} failed: {}", job_id_clone, e);
                    job.status = JobStatus::Failed;
                    job.finished_at = Some(unix_ms());
                    job.error = Some(e.to_string());
                }
            }
        }
    });

    Json(IndexResponse {
        job_id,
        message: format!("Indexing started for workspace: {}", req.workspace),
    })
}

/// GET /index/status/:id — poll a job.
async fn index_status(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let jobs = state.jobs.read().await;
    match jobs.get(&id) {
        Some(job) => match serde_json::to_value(job) {
            Ok(v) => (StatusCode::OK, Json(v)),
            Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": e.to_string() }))),
        },
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": format!("Job '{}' not found", id) })),
        ),
    }
}

/// POST /search — semantic search over an indexed workspace.
async fn search(
    State(state): State<Arc<AppState>>,
    Json(req): Json<SearchRequest>,
) -> impl IntoResponse {
    let indexes = state.indexes.read().await;
    let index = match indexes.get(&req.workspace) {
        Some(idx) => idx,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({
                    "error": format!(
                        "No index found for workspace '{}'. POST /index first.",
                        req.workspace
                    )
                })),
            );
        }
    };

    match index.search(&req.query, req.limit).await {
        Ok(hits) => {
            let total = hits.len();
            match serde_json::to_value(SearchResponse { hits, total }) {
                Ok(v) => (StatusCode::OK, Json(v)),
                Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": e.to_string() }))),
            }
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        ),
    }
}

/// GET /index/jobs — list all jobs.
async fn list_jobs(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let jobs = state.jobs.read().await;
    let mut list: Vec<&IndexJob> = jobs.values().collect();
    list.sort_by_key(|j| std::cmp::Reverse(j.started_at));
    match serde_json::to_value(&list) {
        Ok(v) => (StatusCode::OK, Json(v)),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        ),
    }
}

// ── CLI + main ────────────────────────────────────────────────────────────────

fn unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_millis() as u64
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    // Simple arg parsing (no clap dep to keep the binary light)
    let args: Vec<String> = std::env::args().collect();
    let port: u16 = arg_value(&args, "--port").unwrap_or("9999".into()).parse().unwrap_or(9999);
    let provider_name = arg_value(&args, "--provider").unwrap_or("ollama".into());
    let model = arg_value(&args, "--model").unwrap_or("nomic-embed-text".into());
    let api_key = arg_value(&args, "--api-key").unwrap_or_default();

    let provider = match provider_name.as_str() {
        "openai" => EmbeddingProvider::OpenAI { api_key, model },
        _ => EmbeddingProvider::Ollama {
            model,
            api_url: arg_value(&args, "--ollama-url")
                .unwrap_or("http://localhost:11434".into()),
        },
    };

    // Persistence directory: ~/.vibe-indexer/indexes/
    let persist_dir = std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(".vibe-indexer")
        .join("indexes");
    std::fs::create_dir_all(&persist_dir).ok();

    // Warm up: load any previously-saved indexes from disk
    let mut warmed: HashMap<String, EmbeddingIndex> = HashMap::new();
    if let Ok(rd) = std::fs::read_dir(&persist_dir) {
        for entry in rd.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                // The file stem is a percent-encoded workspace path
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    let workspace = urlencoding_decode(stem);
                    match EmbeddingIndex::load(&path) {
                        Ok(idx) => {
                            info!("Loaded persisted index for workspace: {}", workspace);
                            warmed.insert(workspace, idx);
                        }
                        Err(e) => {
                            warn!("Could not load persisted index {}: {}", path.display(), e);
                        }
                    }
                }
            }
        }
    }

    let state = Arc::new(AppState {
        provider,
        indexes: RwLock::new(warmed),
        jobs: RwLock::new(HashMap::new()),
        persist_dir,
    });

    let app = Router::new()
        .route("/health", get(health))
        .route("/index", post(start_index))
        .route("/index/jobs", get(list_jobs))
        .route("/index/status/:id", get(index_status))
        .route("/search", post(search))
        .with_state(state)
        .layer(
            tower_http::cors::CorsLayer::new()
                .allow_methods([axum::http::Method::GET, axum::http::Method::POST])
                .allow_headers(tower_http::cors::Any)
                .allow_origin(tower_http::cors::Any),
        );

    let addr = format!("0.0.0.0:{}", port);
    info!("vibe-indexer listening on http://{}", addr);
    info!("  POST /index           — start indexing job");
    info!("  GET  /index/status/:id — poll job");
    info!("  GET  /index/jobs       — list all jobs");
    info!("  POST /search          — semantic search");
    info!("  GET  /health          — liveness probe");

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

fn arg_value(args: &[String], flag: &str) -> Option<String> {
    args.windows(2)
        .find(|w| w[0] == flag)
        .map(|w| w[1].clone())
}

/// Percent-encode a workspace path so it can be used as a filename.
/// Encodes any character that is not alphanumeric, `-`, `_`, or `.`.
fn urlencoding_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' => {
                out.push(b as char);
            }
            _ => {
                out.push('%');
                out.push(char::from_digit((b >> 4) as u32, 16).unwrap_or('0').to_ascii_uppercase());
                out.push(char::from_digit((b & 0xf) as u32, 16).unwrap_or('0').to_ascii_uppercase());
            }
        }
    }
    out
}

/// Decode a percent-encoded string back to a workspace path.
fn urlencoding_decode(s: &str) -> String {
    let mut out = Vec::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let (Some(hi), Some(lo)) = (
                char::from(bytes[i + 1]).to_digit(16),
                char::from(bytes[i + 2]).to_digit(16),
            ) {
                out.push(((hi << 4) | lo) as u8);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── urlencoding_encode ───────────────────────────────────────────────────

    #[test]
    fn encode_plain_alphanumeric() {
        assert_eq!(urlencoding_encode("hello123"), "hello123");
    }

    #[test]
    fn encode_preserves_dash_underscore_dot() {
        assert_eq!(urlencoding_encode("my-project_v2.0"), "my-project_v2.0");
    }

    #[test]
    fn encode_encodes_slashes() {
        let encoded = urlencoding_encode("/home/user/project");
        assert_eq!(encoded, "%2Fhome%2Fuser%2Fproject");
    }

    #[test]
    fn encode_encodes_spaces() {
        let encoded = urlencoding_encode("my project");
        assert_eq!(encoded, "my%20project");
    }

    #[test]
    fn encode_empty_string() {
        assert_eq!(urlencoding_encode(""), "");
    }

    // ── urlencoding_decode ───────────────────────────────────────────────────

    #[test]
    fn decode_plain_string() {
        assert_eq!(urlencoding_decode("hello123"), "hello123");
    }

    #[test]
    fn decode_encoded_slashes() {
        assert_eq!(urlencoding_decode("%2Fhome%2Fuser"), "/home/user");
    }

    #[test]
    fn decode_empty_string() {
        assert_eq!(urlencoding_decode(""), "");
    }

    #[test]
    fn decode_trailing_percent_without_hex_digits() {
        // Incomplete percent sequence should be passed through literally
        assert_eq!(urlencoding_decode("abc%"), "abc%");
    }

    #[test]
    fn decode_single_percent_digit() {
        // %2 without a second hex digit should be passed through
        assert_eq!(urlencoding_decode("a%2"), "a%2");
    }

    // ── encode/decode roundtrip ──────────────────────────────────────────────

    #[test]
    fn encode_decode_roundtrip_simple_path() {
        let original = "/home/user/my project/src";
        let encoded = urlencoding_encode(original);
        let decoded = urlencoding_decode(&encoded);
        assert_eq!(decoded, original);
    }

    #[test]
    fn encode_decode_roundtrip_special_chars() {
        let original = "/tmp/a b&c=d";
        let encoded = urlencoding_encode(original);
        let decoded = urlencoding_decode(&encoded);
        assert_eq!(decoded, original);
    }

    // ── arg_value ────────────────────────────────────────────────────────────

    #[test]
    fn arg_value_finds_flag() {
        let args: Vec<String> = vec!["bin", "--port", "8080", "--model", "nomic"]
            .into_iter().map(String::from).collect();
        assert_eq!(arg_value(&args, "--port"), Some("8080".to_string()));
        assert_eq!(arg_value(&args, "--model"), Some("nomic".to_string()));
    }

    #[test]
    fn arg_value_returns_none_for_missing_flag() {
        let args: Vec<String> = vec!["bin", "--port", "8080"]
            .into_iter().map(String::from).collect();
        assert_eq!(arg_value(&args, "--model"), None);
    }

    #[test]
    fn arg_value_returns_none_for_empty_args() {
        let args: Vec<String> = vec![];
        assert_eq!(arg_value(&args, "--port"), None);
    }

    #[test]
    fn arg_value_returns_none_for_flag_at_end() {
        // Flag is the last element so there's no value after it
        let args: Vec<String> = vec!["bin", "--port"]
            .into_iter().map(String::from).collect();
        assert_eq!(arg_value(&args, "--port"), None);
    }

    // ── unix_ms ──────────────────────────────────────────────────────────────

    #[test]
    fn unix_ms_returns_nonzero() {
        let ms = unix_ms();
        // Should be well past epoch — at least year 2020 in milliseconds
        assert!(ms > 1_577_836_800_000);
    }

    // ── default_limit ────────────────────────────────────────────────────────

    #[test]
    fn default_limit_is_10() {
        assert_eq!(default_limit(), 10);
    }

    // ── JobStatus serialization ──────────────────────────────────────────────

    #[test]
    fn job_status_serializes_lowercase() {
        assert_eq!(serde_json::to_string(&JobStatus::Running).unwrap(), "\"running\"");
        assert_eq!(serde_json::to_string(&JobStatus::Complete).unwrap(), "\"complete\"");
        assert_eq!(serde_json::to_string(&JobStatus::Failed).unwrap(), "\"failed\"");
    }

    #[test]
    fn job_status_equality() {
        assert_eq!(JobStatus::Running, JobStatus::Running);
        assert_ne!(JobStatus::Running, JobStatus::Failed);
    }

    // ── IndexJob serialization ───────────────────────────────────────────────

    #[test]
    fn index_job_serializes_to_json() {
        let job = IndexJob {
            id: "abc-123".to_string(),
            workspace: "/tmp/ws".to_string(),
            status: JobStatus::Complete,
            started_at: 1000,
            finished_at: Some(2000),
            files_indexed: 42,
            error: None,
        };
        let json = serde_json::to_string(&job).unwrap();
        assert!(json.contains("\"id\":\"abc-123\""));
        assert!(json.contains("\"status\":\"complete\""));
        assert!(json.contains("\"files_indexed\":42"));
        assert!(json.contains("\"error\":null"));
    }

    #[test]
    fn index_job_with_error_serializes() {
        let job = IndexJob {
            id: "err-1".to_string(),
            workspace: "/ws".to_string(),
            status: JobStatus::Failed,
            started_at: 100,
            finished_at: Some(200),
            files_indexed: 0,
            error: Some("disk full".to_string()),
        };
        let json = serde_json::to_string(&job).unwrap();
        assert!(json.contains("\"error\":\"disk full\""));
    }

    // ── IndexResponse serialization ──────────────────────────────────────────

    #[test]
    fn index_response_serializes() {
        let resp = IndexResponse {
            job_id: "j1".to_string(),
            message: "started".to_string(),
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"job_id\":\"j1\""));
        assert!(json.contains("\"message\":\"started\""));
    }

    // ── SearchRequest deserialization ─────────────────────────────────────────

    #[test]
    fn search_request_deserializes_with_defaults() {
        let json = r#"{"query":"find bugs","workspace":"/tmp/ws"}"#;
        let req: SearchRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.query, "find bugs");
        assert_eq!(req.workspace, "/tmp/ws");
        assert_eq!(req.limit, 10); // default_limit
    }

    #[test]
    fn search_request_deserializes_with_custom_limit() {
        let json = r#"{"query":"q","workspace":"w","limit":25}"#;
        let req: SearchRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.limit, 25);
    }

    // ── IndexRequest deserialization ──────────────────────────────────────────

    #[test]
    fn index_request_deserializes() {
        let json = r#"{"workspace":"/home/user/project"}"#;
        let req: IndexRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.workspace, "/home/user/project");
    }
}

