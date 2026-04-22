//! Reverse-proxy backend that forwards to a local `ollama serve`.
//!
//! Why a proxy and not `reqwest::Client::execute()` of the user's raw bytes?
//! Two reasons:
//!   1. The router needs typed frames (`ChatChunk`, …) to decide e.g. whether
//!      to inject metadata or rate-limit per-token. Forwarding raw bytes
//!      would punch a hole through that abstraction.
//!   2. Downstream clients (VibeUI, mobile, watch) talk to *us* over HTTPS
//!      with our auth scheme. Surfacing ollama's plain-HTTP socket via a
//!      transparent passthrough would expose model traffic outside the
//!      daemon's auth boundary.
//!
//! So: deserialize on the way in, re-serialize on the way out. The cost is
//! one extra parse per chunk; the benefit is one HTTP contract for clients.

use async_trait::async_trait;
use futures::stream::{BoxStream, StreamExt, TryStreamExt};
use reqwest::Client;
use serde::de::DeserializeOwned;

use super::backend::{
    Backend, BackendError, BackendKind, BackendResult, ChatChunk, ChatRequest, GenerateChunk,
    GenerateRequest, ModelInfo, PullProgress, PullRequest,
};

pub struct OllamaProxyBackend {
    base_url: String,
    client: Client,
}

impl OllamaProxyBackend {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            client: Client::new(),
        }
    }

    /// Default — `http://localhost:11434`.
    pub fn local() -> Self {
        Self::new("http://localhost:11434")
    }

    /// Build the upstream URL for an Ollama API path.
    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url.trim_end_matches('/'), path)
    }

    /// POST `body` and forward the response NDJSON as a typed stream.
    /// `T` must match the schema of one frame (e.g. `ChatChunk`).
    async fn ndjson_post<B, T>(
        &self,
        path: &str,
        body: &B,
    ) -> BackendResult<BoxStream<'static, BackendResult<T>>>
    where
        B: serde::Serialize + ?Sized,
        T: DeserializeOwned + Send + 'static,
    {
        let resp = self
            .client
            .post(self.url(path))
            .json(body)
            .send()
            .await
            .map_err(unavailable)?;
        if !resp.status().is_success() {
            return Err(map_status(resp.status(), resp.text().await.unwrap_or_default()));
        }

        // bytes_stream → split on '\n' → parse each as T.
        // Box-pin the byte stream so we don't have to thread `Unpin` bounds
        // through `MapErr` and `NdjsonLines`.
        let pinned: Pin<Box<dyn Stream<Item = reqwest::Result<Bytes>> + Send>> =
            Box::pin(resp.bytes_stream());
        let mapped = pinned.map_err(upstream::<reqwest::Error>);
        let lines = NdjsonLines::new(Box::pin(mapped));
        let typed = lines.map(|line_result| {
            let line = line_result?;
            let trimmed = line.trim();
            if trimmed.is_empty() {
                return Err(BackendError::Upstream("empty NDJSON frame".into()));
            }
            serde_json::from_str::<T>(trimmed)
                .map_err(|e| BackendError::Upstream(format!("ollama frame parse: {e}")))
        });
        Ok(Box::pin(typed))
    }
}

#[async_trait]
impl Backend for OllamaProxyBackend {
    fn kind(&self) -> BackendKind {
        BackendKind::Ollama
    }

    async fn chat(
        &self,
        mut req: ChatRequest,
    ) -> BackendResult<BoxStream<'static, BackendResult<ChatChunk>>> {
        // Force stream=true on the wire so we can yield per-frame to the
        // caller. The HTTP layer collapses for unary callers.
        req.stream = Some(true);
        // Strip our extension field so upstream doesn't see an unknown key.
        // Ollama tolerates unknown fields today but defensive cleanup is
        // cheap and decouples us from that tolerance.
        req.backend = None;
        self.ndjson_post("/api/chat", &req).await
    }

    async fn generate(
        &self,
        mut req: GenerateRequest,
    ) -> BackendResult<BoxStream<'static, BackendResult<GenerateChunk>>> {
        req.stream = Some(true);
        req.backend = None;
        self.ndjson_post("/api/generate", &req).await
    }

    async fn list_models(&self) -> BackendResult<Vec<ModelInfo>> {
        #[derive(serde::Deserialize)]
        struct Tags {
            models: Vec<TagEntry>,
        }
        #[derive(serde::Deserialize)]
        struct TagEntry {
            name: String,
            modified_at: String,
            size: u64,
            #[serde(default)]
            digest: Option<String>,
        }

        let resp = self
            .client
            .get(self.url("/api/tags"))
            .send()
            .await
            .map_err(unavailable)?;
        if !resp.status().is_success() {
            return Err(map_status(resp.status(), resp.text().await.unwrap_or_default()));
        }
        let tags: Tags = resp.json().await.map_err(upstream)?;
        Ok(tags
            .models
            .into_iter()
            .map(|m| ModelInfo {
                name: m.name,
                modified_at: m.modified_at,
                size: m.size,
                backend: BackendKind::Ollama,
                digest: m.digest,
            })
            .collect())
    }

    async fn pull(
        &self,
        mut req: PullRequest,
    ) -> BackendResult<BoxStream<'static, BackendResult<PullProgress>>> {
        req.stream = Some(true);
        req.backend = None;
        self.ndjson_post("/api/pull", &req).await
    }

    async fn show(&self, name: &str) -> BackendResult<ModelInfo> {
        #[derive(serde::Serialize)]
        struct ShowReq<'a> {
            name: &'a str,
        }
        // Ollama's `/api/show` returns a richer shape than `/api/tags`
        // (modelfile, parameters, template, …). We only project the few
        // fields ModelInfo carries — clients that need the rest can hit
        // ollama directly with their own credentials in dev setups.
        #[derive(serde::Deserialize)]
        struct ShowResp {
            #[serde(default)]
            digest: Option<String>,
            #[serde(default)]
            size: Option<u64>,
            #[serde(default)]
            modified_at: Option<String>,
        }
        let resp = self
            .client
            .post(self.url("/api/show"))
            .json(&ShowReq { name })
            .send()
            .await
            .map_err(unavailable)?;
        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(BackendError::ModelNotFound(name.to_string()));
        }
        if !resp.status().is_success() {
            return Err(map_status(resp.status(), resp.text().await.unwrap_or_default()));
        }
        let show: ShowResp = resp.json().await.map_err(upstream)?;
        Ok(ModelInfo {
            name: name.to_string(),
            modified_at: show.modified_at.unwrap_or_default(),
            size: show.size.unwrap_or(0),
            backend: BackendKind::Ollama,
            digest: show.digest,
        })
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn unavailable(e: reqwest::Error) -> BackendError {
    BackendError::Unavailable(format!("ollama proxy: {e}"))
}

fn upstream<E: std::fmt::Display>(e: E) -> BackendError {
    BackendError::Upstream(format!("ollama proxy: {e}"))
}

fn map_status(status: reqwest::StatusCode, body: String) -> BackendError {
    if status == reqwest::StatusCode::NOT_FOUND {
        BackendError::ModelNotFound(body)
    } else if status.is_client_error() {
        BackendError::InvalidRequest(format!("ollama returned {status}: {body}"))
    } else {
        BackendError::Upstream(format!("ollama returned {status}: {body}"))
    }
}

// ---------------------------------------------------------------------------
// NDJSON line splitter — turns a byte-chunk stream into a line stream
// without buffering the whole response in memory. ollama frames are short
// (<2 KiB), but a long pull stream can run for minutes; we have to stay
// streaming-correct.
// ---------------------------------------------------------------------------

use bytes::Bytes;
use futures::Stream;
use std::pin::Pin;
use std::task::{Context, Poll};

struct NdjsonLines<S> {
    inner: S,
    buf: Vec<u8>,
    done: bool,
}

impl<S> NdjsonLines<S> {
    fn new(inner: S) -> Self {
        Self {
            inner,
            buf: Vec::new(),
            done: false,
        }
    }
}

impl<S> Stream for NdjsonLines<S>
where
    S: Stream<Item = BackendResult<Bytes>> + Unpin,
{
    type Item = BackendResult<String>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        loop {
            // Drain a complete line out of the buffer if one's there.
            if let Some(pos) = self.buf.iter().position(|&b| b == b'\n') {
                let line: Vec<u8> = self.buf.drain(..=pos).collect();
                // Strip trailing \n (and \r if present).
                let mut end = line.len() - 1;
                if end > 0 && line[end - 1] == b'\r' {
                    end -= 1;
                }
                let s = String::from_utf8_lossy(&line[..end]).to_string();
                return Poll::Ready(Some(Ok(s)));
            }
            // No newline yet — pull more bytes (or finish if upstream is done).
            if self.done {
                if self.buf.is_empty() {
                    return Poll::Ready(None);
                }
                let s = String::from_utf8_lossy(&self.buf).to_string();
                self.buf.clear();
                return Poll::Ready(Some(Ok(s)));
            }
            match Pin::new(&mut self.inner).poll_next(cx) {
                Poll::Pending => return Poll::Pending,
                Poll::Ready(None) => {
                    self.done = true;
                    // Loop to flush any trailing partial line.
                }
                Poll::Ready(Some(Err(e))) => return Poll::Ready(Some(Err(e))),
                Poll::Ready(Some(Ok(chunk))) => {
                    self.buf.extend_from_slice(&chunk);
                }
            }
        }
    }
}

