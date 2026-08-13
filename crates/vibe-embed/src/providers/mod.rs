//! HTTP (and in-process) embedding backends.
//!
//! Each backend implements [`Embedder`](crate::Embedder) over one provider's
//! wire format. They share a single [`reqwest::Client`] per timeout setting —
//! building a client per request throws away the connection pool, which for a
//! full workspace index means thousands of fresh TLS handshakes.

use crate::{EmbeddingConfig, EmbeddingError, ProviderKind, Result, SharedEmbedder};
use std::sync::{Arc, OnceLock};

mod cohere;
mod gemini;
mod ollama;
mod openai;
mod voyage;

pub use cohere::CohereEmbedder;
pub use gemini::GeminiEmbedder;
pub use ollama::OllamaEmbedder;
pub use openai::OpenAIEmbedder;
pub use voyage::VoyageEmbedder;

/// Default endpoint per provider.
pub const fn default_base_url(provider: ProviderKind) -> &'static str {
    match provider {
        ProviderKind::Ollama => "http://127.0.0.1:11434",
        ProviderKind::OpenAI => "https://api.openai.com/v1",
        ProviderKind::Voyage => "https://api.voyageai.com/v1",
        ProviderKind::Cohere => "https://api.cohere.com/v2",
        ProviderKind::Gemini => "https://generativelanguage.googleapis.com/v1beta",
        ProviderKind::Local => "",
    }
}

// ---------------------------------------------------------------------------
// Shared HTTP client
// ---------------------------------------------------------------------------

/// The process-wide client for `timeout_secs`.
///
/// This crate used to own the pool; it now lives in `vibe-http-pool` so the
/// Tauri commands and the daemon share one set of connections instead of each
/// building their own. Kept as a thin alias so provider call sites are unchanged.
pub(crate) fn http_client(timeout_secs: u64) -> reqwest::Client {
    vibe_http_pool::client(timeout_secs)
}

// ---------------------------------------------------------------------------
// Response helpers
// ---------------------------------------------------------------------------

/// Read a JSON body, turning a non-2xx status into a typed error that carries
/// the provider's own message.
///
/// Without this, an expired API key surfaces as a serde "missing field
/// `data`" error, and the user goes looking in the wrong place.
pub(crate) async fn json_or_http_error<T: serde::de::DeserializeOwned>(
    provider: &'static str,
    response: reqwest::Response,
) -> Result<T> {
    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|source| EmbeddingError::Transport { provider, source })?;

    if !status.is_success() {
        return Err(EmbeddingError::Http {
            provider,
            status: status.as_u16(),
            body: truncate_for_error(&body),
        });
    }

    serde_json::from_str(&body).map_err(|e| EmbeddingError::MalformedResponse {
        provider,
        detail: format!("{e} (body: {})", truncate_for_error(&body)),
    })
}

/// Keep provider error bodies readable in a terminal — an HTML error page from
/// a misconfigured proxy is otherwise several kilobytes of noise.
fn truncate_for_error(body: &str) -> String {
    const MAX: usize = 400;
    let trimmed = body.trim();
    match trimmed.char_indices().nth(MAX) {
        Some((idx, _)) => format!("{}…", &trimmed[..idx]),
        None => trimmed.to_string(),
    }
}

/// Verify a provider returned one vector per input, in order.
pub(crate) fn check_batch_len(
    provider: &'static str,
    want: usize,
    vectors: Vec<Vec<f32>>,
) -> Result<Vec<Vec<f32>>> {
    if vectors.len() == want {
        Ok(vectors)
    } else {
        Err(EmbeddingError::BatchSizeMismatch {
            provider,
            want,
            got: vectors.len(),
        })
    }
}

pub(crate) fn require_api_key(config: &EmbeddingConfig) -> Result<String> {
    config
        .api_key
        .clone()
        .filter(|k| !k.trim().is_empty())
        .ok_or(EmbeddingError::MissingApiKey(
            config.model.provider.as_str(),
        ))
}

// ---------------------------------------------------------------------------
// Local backend registration
// ---------------------------------------------------------------------------

type LocalFactory = Box<dyn Fn(&EmbeddingConfig) -> Result<SharedEmbedder> + Send + Sync>;

fn local_factory() -> &'static OnceLock<LocalFactory> {
    static FACTORY: OnceLock<LocalFactory> = OnceLock::new();
    &FACTORY
}

/// Install the in-process embedding backend.
///
/// `vibe-embed` deliberately does not depend on candle — pulling an ML
/// toolchain into the crate that the daemon, the indexer and the memory
/// stores all link would slow every build in the workspace. Instead a build
/// that *does* enable candle calls this once at startup to make
/// [`ProviderKind::Local`] resolvable. Until it does, `Local` fails with
/// [`EmbeddingError::BackendNotEnabled`] rather than silently falling back to
/// a cloud provider.
///
/// Returns `false` if a factory was already registered.
pub fn register_local_backend<F>(factory: F) -> bool
where
    F: Fn(&EmbeddingConfig) -> Result<SharedEmbedder> + Send + Sync + 'static,
{
    local_factory().set(Box::new(factory)).is_ok()
}

/// Whether an in-process backend is available in this build.
pub fn local_backend_available() -> bool {
    local_factory().get().is_some()
}

// ---------------------------------------------------------------------------
// Construction
// ---------------------------------------------------------------------------

pub(crate) fn build(config: EmbeddingConfig) -> Result<SharedEmbedder> {
    match config.model.provider {
        ProviderKind::Ollama => Ok(Arc::new(OllamaEmbedder::new(config))),
        ProviderKind::OpenAI => Ok(Arc::new(OpenAIEmbedder::new(config)?)),
        ProviderKind::Voyage => Ok(Arc::new(VoyageEmbedder::new(config)?)),
        ProviderKind::Cohere => Ok(Arc::new(CohereEmbedder::new(config)?)),
        ProviderKind::Gemini => Ok(Arc::new(GeminiEmbedder::new(config)?)),
        ProviderKind::Local => match local_factory().get() {
            Some(f) => f(&config),
            None => Err(EmbeddingError::BackendNotEnabled("candle")),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ModelRef, ProviderKind};

    /// Indexing a workspace is thousands of requests; a fresh client per call
    /// would discard the connection pool every time.
    /// The pooling property itself is now tested in `vibe-http-pool`, which
    /// owns the pool; this only pins that providers still go through it.
    #[test]
    fn http_client_delegates_to_the_shared_pool() {
        let a = http_client(4242);
        let b = vibe_http_pool::client(4242);
        assert_eq!(format!("{a:?}"), format!("{b:?}"));
    }

    #[test]
    fn cloud_providers_refuse_to_build_without_a_key() {
        for p in [
            ProviderKind::OpenAI,
            ProviderKind::Voyage,
            ProviderKind::Cohere,
            ProviderKind::Gemini,
        ] {
            let cfg = EmbeddingConfig::new(ModelRef::new(p, "any"));
            assert!(
                matches!(build(cfg), Err(EmbeddingError::MissingApiKey(_))),
                "{p} built without an API key"
            );
        }
    }

    #[test]
    fn ollama_builds_without_a_key() {
        let cfg = EmbeddingConfig::new(ModelRef::new(ProviderKind::Ollama, "nomic-embed-text"));
        assert!(build(cfg).is_ok());
    }

    /// `Local` must fail loudly rather than quietly routing to a cloud model.
    #[test]
    fn local_without_a_registered_backend_is_a_typed_error() {
        if local_backend_available() {
            return; // another test in this binary registered one
        }
        let cfg = EmbeddingConfig::new(ModelRef::new(ProviderKind::Local, "all-MiniLM-L6-v2"));
        assert!(matches!(
            build(cfg),
            Err(EmbeddingError::BackendNotEnabled("candle"))
        ));
    }

    #[test]
    fn batch_length_mismatch_is_caught() {
        let r = check_batch_len("openai", 3, vec![vec![1.0], vec![2.0]]);
        assert!(matches!(
            r,
            Err(EmbeddingError::BatchSizeMismatch {
                want: 3,
                got: 2,
                ..
            })
        ));
    }

    #[test]
    fn error_bodies_are_truncated() {
        let long = "x".repeat(10_000);
        assert!(truncate_for_error(&long).chars().count() <= 401);
    }

    #[test]
    fn truncate_does_not_split_a_multibyte_char() {
        let s = "é".repeat(10_000);
        let out = truncate_for_error(&s);
        assert!(out.ends_with('…'));
    }
}
