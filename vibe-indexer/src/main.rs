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
    /// Completed indexes, keyed by job id.
    indexes: RwLock<HashMap<String, EmbeddingIndex>>,
    /// All jobs (including running/failed).
    jobs: RwLock<HashMap<String, IndexJob>>,
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
        Some(job) => (StatusCode::OK, Json(serde_json::to_value(job).unwrap())),
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
            (StatusCode::OK, Json(serde_json::to_value(SearchResponse { hits, total }).unwrap()))
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
    Json(serde_json::to_value(&list).unwrap())
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

    let state = Arc::new(AppState {
        provider,
        indexes: RwLock::new(HashMap::new()),
        jobs: RwLock::new(HashMap::new()),
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

use std::cmp::Reverse;
