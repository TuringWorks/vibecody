//! Provider-agnostic text embedding for VibeCody.
//!
//! One [`Embedder`] trait, many models. This crate is the single abstraction
//! every RAG surface in the workspace embeds through — the code index
//! (`vibe-core::index`), the memory stores (`vibe-memory`, OpenMemory), the
//! remote indexer, and the daemon's `/embeddings/*` routes.
//!
//! # Why a separate crate
//!
//! Before this crate there were two disjoint abstractions: a two-variant
//! `EmbeddingProvider` enum inside the code index, and an `Embedder` trait in
//! `vibe-infer` that only the candle backend implemented. Nothing bridged
//! them, so "switch embedding model" meant two unrelated edits in two
//! unrelated subsystems — and the memory stores used hash buckets, not a
//! model at all. `vibe-embed` sits below all of them (no candle, no sqlite,
//! no tauri) so every consumer can share one catalog and one trait.
//!
//! # Three invariants worth knowing
//!
//! 1. **Dimension is observed, never assumed.** [`EmbeddingModel::dimension`]
//!    is a documented hint; the number that gets persisted is the length of a
//!    vector the model actually returned. Users pull arbitrary Ollama models,
//!    and a catalog that guesses is a catalog that silently corrupts an index.
//! 2. **Documents and queries embed differently.** Retrieval quality on
//!    `nomic-embed-text`, `bge-*`, `e5-*`, Voyage, Cohere and Gemini depends
//!    on telling the model which side of the search it is embedding. That is
//!    [`EmbedKind`], and it is a required argument — not an option someone
//!    forgets to pass.
//! 3. **A model's identity travels with its vectors.** [`ModelRef::slug`]
//!    yields a filesystem-safe key so indexes built with different models sit
//!    side by side instead of overwriting each other.
//!
//! # Quick start
//! ```no_run
//! use vibe_embed::{EmbedKind, Embedder, EmbeddingConfig, ModelRef, ProviderKind};
//! # async fn example() -> Result<(), vibe_embed::EmbeddingError> {
//! let model = ModelRef::new(ProviderKind::Ollama, "nomic-embed-text");
//! let embedder = EmbeddingConfig::new(model).build()?;
//! let doc = embedder.embed("fn authenticate(user: &User)", EmbedKind::Document).await?;
//! let qry = embedder.embed("how do we authenticate?", EmbedKind::Query).await?;
//! # let _ = (doc, qry);
//! # Ok(()) }
//! ```

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub mod catalog;
pub mod providers;
pub mod settings;

pub use catalog::{all_models, models_for, EmbeddingModel};
pub use settings::{provider_catalog, Availability, EmbeddingSettings, ProviderCatalog};

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug, thiserror::Error)]
pub enum EmbeddingError {
    #[error("embedding provider `{0}` needs an API key — add one in Settings → Providers")]
    MissingApiKey(&'static str),

    #[error("{provider} embedding request failed: {source}")]
    Transport {
        provider: &'static str,
        #[source]
        source: reqwest::Error,
    },

    #[error("{provider} returned HTTP {status}: {body}")]
    Http {
        provider: &'static str,
        status: u16,
        body: String,
    },

    #[error("{provider} returned a response this client cannot read: {detail}")]
    MalformedResponse {
        provider: &'static str,
        detail: String,
    },

    #[error("{provider} returned {got} vectors for {want} inputs")]
    BatchSizeMismatch {
        provider: &'static str,
        want: usize,
        got: usize,
    },

    #[error(
        "model `{model}` produced a {got}-dimension vector but this index stores {expected}; \
         rebuild the index for this model"
    )]
    DimensionMismatch {
        model: String,
        expected: usize,
        got: usize,
    },

    #[error("embedding backend `{0}` is not compiled in — rebuild with --features {0}")]
    BackendNotEnabled(&'static str),

    #[error("{0}")]
    Backend(String),
}

pub type Result<T> = std::result::Result<T, EmbeddingError>;

// ---------------------------------------------------------------------------
// EmbedKind
// ---------------------------------------------------------------------------

/// Which side of a retrieval pair a text is being embedded for.
///
/// Asymmetric embedding models place stored passages and search queries in
/// deliberately different regions of the space. Getting this wrong does not
/// error — it quietly costs recall, which is why the argument is required
/// rather than defaulted.
///
/// Each provider expresses it natively where it can (Voyage `input_type`,
/// Cohere `input_type`, Gemini `task_type`) and via a text prefix where it
/// cannot (Ollama-hosted `nomic-embed-text`, `bge-*`, `e5-*`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EmbedKind {
    /// Text being stored in the index.
    Document,
    /// Text being searched with.
    Query,
}

impl EmbedKind {
    /// The wire value used by providers that spell this `search_document` /
    /// `search_query` (Cohere).
    pub const fn cohere_input_type(self) -> &'static str {
        match self {
            Self::Document => "search_document",
            Self::Query => "search_query",
        }
    }

    /// Voyage spells it `document` / `query`.
    pub const fn voyage_input_type(self) -> &'static str {
        match self {
            Self::Document => "document",
            Self::Query => "query",
        }
    }

    /// Gemini spells it as a task type.
    pub const fn gemini_task_type(self) -> &'static str {
        match self {
            Self::Document => "RETRIEVAL_DOCUMENT",
            Self::Query => "RETRIEVAL_QUERY",
        }
    }
}

// ---------------------------------------------------------------------------
// ProviderKind
// ---------------------------------------------------------------------------

/// Backend that turns text into vectors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderKind {
    /// Local Ollama daemon (`/api/embed`).
    Ollama,
    /// OpenAI `/v1/embeddings`, or anything speaking that shape (Azure,
    /// LiteLLM, vLLM, text-embeddings-inference) via a base-URL override.
    OpenAI,
    /// Voyage AI — `voyage-code-3` is the strongest code-retrieval model here.
    Voyage,
    /// Cohere `/v2/embed`.
    Cohere,
    /// Google Gemini embeddings.
    Gemini,
    /// In-process candle model — no network, no API key.
    Local,
}

impl ProviderKind {
    pub const ALL: &'static [ProviderKind] = &[
        ProviderKind::Ollama,
        ProviderKind::OpenAI,
        ProviderKind::Voyage,
        ProviderKind::Cohere,
        ProviderKind::Gemini,
        ProviderKind::Local,
    ];

    /// Stable wire/config identifier. Matches the `provider` string used by
    /// the ProfileStore key entries and the daemon routes.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ollama => "ollama",
            Self::OpenAI => "openai",
            Self::Voyage => "voyage",
            Self::Cohere => "cohere",
            Self::Gemini => "gemini",
            Self::Local => "local",
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Self::Ollama => "Ollama",
            Self::OpenAI => "OpenAI",
            Self::Voyage => "Voyage AI",
            Self::Cohere => "Cohere",
            Self::Gemini => "Google Gemini",
            Self::Local => "Local (built-in)",
        }
    }

    /// Whether an API key must be present before this provider can be used.
    pub const fn requires_api_key(self) -> bool {
        match self {
            Self::Ollama | Self::Local => false,
            Self::OpenAI | Self::Voyage | Self::Cohere | Self::Gemini => true,
        }
    }

    /// Whether the provider runs without leaving the machine. Surfaced in the
    /// UI so a user indexing a private repo can see, before they pick, that a
    /// cloud model means shipping source to a third party.
    pub const fn is_local(self) -> bool {
        matches!(self, Self::Ollama | Self::Local)
    }

    pub fn parse(s: &str) -> Option<Self> {
        Self::ALL
            .iter()
            .copied()
            .find(|p| p.as_str().eq_ignore_ascii_case(s))
    }
}

impl std::fmt::Display for ProviderKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

// ---------------------------------------------------------------------------
// ModelRef
// ---------------------------------------------------------------------------

/// Identity of the model that produced a set of vectors.
///
/// This is what gets persisted alongside an index. Two indexes whose
/// `ModelRef`s differ are not comparable, and [`slug`](Self::slug) keeps them
/// in separate files rather than letting one silently overwrite the other.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelRef {
    pub provider: ProviderKind,
    pub model: String,
    /// Requested output dimension for models that support truncation
    /// (Matryoshka). `None` means the model's native size.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dimensions: Option<usize>,
}

impl ModelRef {
    pub fn new(provider: ProviderKind, model: impl Into<String>) -> Self {
        Self {
            provider,
            model: model.into(),
            dimensions: None,
        }
    }

    pub fn with_dimensions(mut self, dimensions: Option<usize>) -> Self {
        self.dimensions = dimensions;
        self
    }

    /// Catalog entry for this model, when we ship one. `None` for models the
    /// user pulled themselves — a perfectly normal case with Ollama, and the
    /// reason nothing in this crate gates on catalog membership.
    pub fn catalog_entry(&self) -> Option<&'static EmbeddingModel> {
        catalog::lookup(self.provider, &self.model)
    }

    /// Documented dimension, when known ahead of the first call. Callers must
    /// treat `None` as "ask the model", never as "assume a default".
    pub fn known_dimension(&self) -> Option<usize> {
        self.dimensions
            .or_else(|| self.catalog_entry().and_then(|m| m.dimension))
    }

    /// Filesystem- and URL-safe key identifying this model.
    ///
    /// Used to keep per-model indexes side by side:
    /// `.vibecli/index/ollama__nomic-embed-text.json`.
    pub fn slug(&self) -> String {
        let sanitized: String = self
            .model
            .chars()
            .map(|c| match c {
                'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '.' => c,
                _ => '_',
            })
            .collect();
        match self.dimensions {
            Some(d) => format!("{}__{}__{}d", self.provider.as_str(), sanitized, d),
            None => format!("{}__{}", self.provider.as_str(), sanitized),
        }
    }
}

impl std::fmt::Display for ModelRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.dimensions {
            Some(d) => write!(f, "{}/{} ({d}d)", self.provider.as_str(), self.model),
            None => write!(f, "{}/{}", self.provider.as_str(), self.model),
        }
    }
}

// ---------------------------------------------------------------------------
// Embedder
// ---------------------------------------------------------------------------

/// Text → vector.
///
/// Implementations return L2-comparable vectors of a single, stable length for
/// the lifetime of the instance. That length is *not* declared up front —
/// see [`dim`](Self::dim).
#[async_trait]
pub trait Embedder: Send + Sync {
    /// Identity of the underlying model. Persist this with the vectors.
    fn model(&self) -> &ModelRef;

    /// Output dimension **if it is known without calling the model**.
    ///
    /// Returns `None` for any model we have not documented — which includes
    /// every model a user pulls into Ollama themselves. Callers that need the
    /// real number must embed something and measure, then record what they
    /// measured. Returning a plausible default here would let a wrong number
    /// reach an index header, and a wrong header is worse than no header.
    fn dim(&self) -> Option<usize> {
        self.model().known_dimension()
    }

    /// Largest number of inputs to send in one request.
    fn max_batch(&self) -> usize {
        64
    }

    /// Embed a batch. Returns exactly `texts.len()` vectors, in order.
    async fn embed_batch(&self, texts: &[String], kind: EmbedKind) -> Result<Vec<Vec<f32>>>;

    /// Embed one text.
    async fn embed(&self, text: &str, kind: EmbedKind) -> Result<Vec<f32>> {
        let batch = self
            .embed_batch(std::slice::from_ref(&text.to_string()), kind)
            .await?;
        batch
            .into_iter()
            .next()
            .ok_or_else(|| EmbeddingError::MalformedResponse {
                provider: self.model().provider.as_str(),
                detail: "empty batch response for a single input".into(),
            })
    }

    /// Embed an arbitrarily large set, chunked to [`max_batch`](Self::max_batch).
    async fn embed_all(&self, texts: &[String], kind: EmbedKind) -> Result<Vec<Vec<f32>>> {
        let chunk = self.max_batch().max(1);
        let mut out = Vec::with_capacity(texts.len());
        for window in texts.chunks(chunk) {
            out.extend(self.embed_batch(window, kind).await?);
        }
        Ok(out)
    }
}

/// Shared handle to an embedder. Every consumer stores one of these rather
/// than a concrete provider type.
pub type SharedEmbedder = Arc<dyn Embedder>;

// ---------------------------------------------------------------------------
// EmbeddingConfig
// ---------------------------------------------------------------------------

/// Everything needed to construct an [`Embedder`].
///
/// `api_key` is passed in by the caller, which reads it from the encrypted
/// ProfileStore. This crate never touches the filesystem or environment
/// variables for credentials.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingConfig {
    pub model: ModelRef,
    /// Overrides the provider's default endpoint. Lets a user point at Azure
    /// OpenAI, a LiteLLM proxy, a text-embeddings-inference server, or an
    /// Ollama daemon on another host.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    /// Request timeout in seconds. Cold Ollama models can take a while on the
    /// first call because the model has to load.
    #[serde(default = "default_timeout_secs")]
    pub timeout_secs: u64,
}

fn default_timeout_secs() -> u64 {
    120
}

impl EmbeddingConfig {
    pub fn new(model: ModelRef) -> Self {
        Self {
            model,
            base_url: None,
            api_key: None,
            timeout_secs: default_timeout_secs(),
        }
    }

    pub fn with_base_url(mut self, base_url: Option<String>) -> Self {
        self.base_url = base_url.filter(|s| !s.trim().is_empty());
        self
    }

    pub fn with_api_key(mut self, api_key: Option<String>) -> Self {
        self.api_key = api_key.filter(|s| !s.trim().is_empty());
        self
    }

    pub fn with_timeout_secs(mut self, timeout_secs: u64) -> Self {
        self.timeout_secs = timeout_secs;
        self
    }

    /// Endpoint this config will actually call, after defaults.
    pub fn resolved_base_url(&self) -> String {
        let raw = self
            .base_url
            .clone()
            .unwrap_or_else(|| providers::default_base_url(self.model.provider).to_string());
        raw.trim_end_matches('/').to_string()
    }

    /// Construct the embedder.
    pub fn build(self) -> Result<SharedEmbedder> {
        providers::build(self)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slug_is_filesystem_safe() {
        let m = ModelRef::new(ProviderKind::Ollama, "nomic-embed-text:v1.5");
        assert_eq!(m.slug(), "ollama__nomic-embed-text_v1.5");
        assert!(!m.slug().contains('/'));
        assert!(!m.slug().contains(':'));
    }

    #[test]
    fn slug_separates_matryoshka_variants() {
        let base = ModelRef::new(ProviderKind::Voyage, "voyage-code-3");
        let small = base.clone().with_dimensions(Some(256));
        assert_ne!(base.slug(), small.slug());
        assert_eq!(small.slug(), "voyage__voyage-code-3__256d");
    }

    #[test]
    fn slug_distinguishes_same_model_on_two_providers() {
        let a = ModelRef::new(ProviderKind::Ollama, "bge-m3");
        let b = ModelRef::new(ProviderKind::OpenAI, "bge-m3");
        assert_ne!(a.slug(), b.slug());
    }

    /// An unknown model must be usable, and must NOT claim a dimension. This
    /// is the invariant that stops a guessed 768 from reaching an index header.
    #[test]
    fn unknown_model_reports_no_dimension() {
        let m = ModelRef::new(ProviderKind::Ollama, "someones-custom-embed");
        assert!(m.catalog_entry().is_none());
        assert_eq!(m.known_dimension(), None);
    }

    #[test]
    fn explicit_dimensions_override_catalog() {
        let m = ModelRef::new(ProviderKind::OpenAI, "text-embedding-3-large")
            .with_dimensions(Some(512));
        assert_eq!(m.known_dimension(), Some(512));
    }

    #[test]
    fn provider_roundtrips_through_str() {
        for p in ProviderKind::ALL {
            assert_eq!(ProviderKind::parse(p.as_str()), Some(*p));
        }
    }

    #[test]
    fn provider_parse_is_case_insensitive() {
        assert_eq!(ProviderKind::parse("OpenAI"), Some(ProviderKind::OpenAI));
        assert_eq!(ProviderKind::parse("nope"), None);
    }

    #[test]
    fn local_providers_need_no_key() {
        assert!(!ProviderKind::Ollama.requires_api_key());
        assert!(!ProviderKind::Local.requires_api_key());
        assert!(ProviderKind::Voyage.requires_api_key());
    }

    #[test]
    fn base_url_override_wins_and_is_normalised() {
        let cfg = EmbeddingConfig::new(ModelRef::new(ProviderKind::Ollama, "x"))
            .with_base_url(Some("http://gpu-box:11434/".into()));
        assert_eq!(cfg.resolved_base_url(), "http://gpu-box:11434");
    }

    #[test]
    fn blank_base_url_falls_back_to_default() {
        let cfg = EmbeddingConfig::new(ModelRef::new(ProviderKind::Ollama, "x"))
            .with_base_url(Some("   ".into()));
        assert_eq!(cfg.resolved_base_url(), "http://127.0.0.1:11434");
    }

    #[test]
    fn embed_kind_wire_values() {
        assert_eq!(EmbedKind::Query.cohere_input_type(), "search_query");
        assert_eq!(EmbedKind::Document.voyage_input_type(), "document");
        assert_eq!(EmbedKind::Query.gemini_task_type(), "RETRIEVAL_QUERY");
    }
}
