//! Catalog of embedding models we ship metadata for.
//!
//! **The catalog is a hint list, not an allow-list.** Nothing in this crate
//! refuses a model because it is absent here — Ollama users pull their own
//! models constantly, and a gate would break them. What the catalog buys is
//! metadata we cannot discover from an API: the prefix a model expects, the
//! dimensions it can be truncated to, and how much text fits in one call.
//!
//! Where a field is not documented for a model, it is `None` or empty rather
//! than a plausible guess. A guessed dimension that reaches an index header
//! is worse than an absent one, because absent triggers a probe and wrong
//! triggers silence.

use crate::{EmbedKind, ProviderKind};
use serde::Serialize;

/// Static metadata for one embedding model.
///
/// Serialize-only: the catalog is compiled in, so it is published to clients
/// (daemon route, Tauri command) but never read back from JSON.
#[derive(Debug, Clone, Serialize)]
pub struct EmbeddingModel {
    pub provider: ProviderKind,
    /// Wire identifier sent to the provider.
    pub id: &'static str,
    pub display_name: &'static str,
    /// Native output dimension, when documented. `None` means "probe it".
    pub dimension: Option<usize>,
    /// Dimensions the model can be truncated to (Matryoshka). Empty when the
    /// model has a single fixed size.
    pub supported_dimensions: &'static [usize],
    /// Documented input limit in tokens, when published.
    pub max_input_tokens: Option<usize>,
    /// Prefix prepended to stored passages. Empty for models that take the
    /// distinction natively (Voyage, Cohere, Gemini) or not at all.
    pub document_prefix: &'static str,
    /// Prefix prepended to search queries.
    pub query_prefix: &'static str,
    /// True for models trained or benchmarked specifically on code retrieval.
    pub recommended_for_code: bool,
    pub notes: &'static str,
}

impl EmbeddingModel {
    /// Apply this model's asymmetric prefix, if it has one.
    ///
    /// Only used by providers that have no native input-type field. Returns
    /// the input untouched when no prefix applies, so the common case does
    /// not allocate a second copy of every chunk.
    pub fn apply_prefix<'a>(&self, text: &'a str, kind: EmbedKind) -> std::borrow::Cow<'a, str> {
        let prefix = match kind {
            EmbedKind::Document => self.document_prefix,
            EmbedKind::Query => self.query_prefix,
        };
        if prefix.is_empty() {
            std::borrow::Cow::Borrowed(text)
        } else {
            std::borrow::Cow::Owned(format!("{prefix}{text}"))
        }
    }

    /// Whether `dim` is a valid explicit output dimension for this model.
    pub fn supports_dimension(&self, dim: usize) -> bool {
        self.supported_dimensions.contains(&dim) || self.dimension == Some(dim)
    }
}

/// Every model we ship metadata for.
pub const CATALOG: &[EmbeddingModel] = &[
    // ── Ollama (local) ───────────────────────────────────────────────────────
    EmbeddingModel {
        provider: ProviderKind::Ollama,
        id: "nomic-embed-text",
        display_name: "Nomic Embed Text",
        dimension: Some(768),
        supported_dimensions: &[],
        max_input_tokens: Some(8192),
        document_prefix: "search_document: ",
        query_prefix: "search_query: ",
        recommended_for_code: false,
        notes: "Long context, strong general retrieval. Requires task prefixes — \
                without them recall drops noticeably.",
    },
    EmbeddingModel {
        provider: ProviderKind::Ollama,
        id: "mxbai-embed-large",
        display_name: "mxbai Embed Large",
        dimension: Some(1024),
        supported_dimensions: &[],
        max_input_tokens: Some(512),
        document_prefix: "",
        query_prefix: "Represent this sentence for searching relevant passages: ",
        recommended_for_code: false,
        notes: "Asymmetric: queries take an instruction prefix, passages do not.",
    },
    EmbeddingModel {
        provider: ProviderKind::Ollama,
        id: "bge-m3",
        display_name: "BGE-M3",
        dimension: Some(1024),
        supported_dimensions: &[],
        max_input_tokens: Some(8192),
        document_prefix: "",
        query_prefix: "",
        recommended_for_code: false,
        notes: "Multilingual, long context, symmetric — no prefixes needed.",
    },
    EmbeddingModel {
        provider: ProviderKind::Ollama,
        id: "all-minilm",
        display_name: "all-MiniLM-L6-v2",
        dimension: Some(384),
        supported_dimensions: &[],
        max_input_tokens: Some(256),
        document_prefix: "",
        query_prefix: "",
        recommended_for_code: false,
        notes: "Smallest and fastest. Short input limit — chunk aggressively.",
    },
    EmbeddingModel {
        provider: ProviderKind::Ollama,
        id: "snowflake-arctic-embed2",
        display_name: "Snowflake Arctic Embed 2",
        dimension: Some(1024),
        supported_dimensions: &[],
        max_input_tokens: None,
        document_prefix: "",
        query_prefix: "query: ",
        recommended_for_code: false,
        notes: "Multilingual. Queries take a `query: ` prefix.",
    },
    EmbeddingModel {
        provider: ProviderKind::Ollama,
        id: "embeddinggemma",
        display_name: "EmbeddingGemma",
        dimension: Some(768),
        supported_dimensions: &[],
        max_input_tokens: None,
        document_prefix: "title: none | text: ",
        query_prefix: "task: search result | query: ",
        recommended_for_code: false,
        notes: "Google's small on-device embedder. Uses structured task prefixes.",
    },
    EmbeddingModel {
        provider: ProviderKind::Ollama,
        id: "granite-embedding",
        display_name: "IBM Granite Embedding",
        dimension: Some(384),
        supported_dimensions: &[],
        max_input_tokens: None,
        document_prefix: "",
        query_prefix: "",
        recommended_for_code: false,
        notes: "Symmetric, permissively licensed.",
    },
    // ── OpenAI ───────────────────────────────────────────────────────────────
    EmbeddingModel {
        provider: ProviderKind::OpenAI,
        id: "text-embedding-3-small",
        display_name: "text-embedding-3-small",
        dimension: Some(1536),
        supported_dimensions: &[512, 1536],
        max_input_tokens: Some(8191),
        document_prefix: "",
        query_prefix: "",
        recommended_for_code: false,
        notes: "Cheap general-purpose default. Truncatable to 512 dimensions.",
    },
    EmbeddingModel {
        provider: ProviderKind::OpenAI,
        id: "text-embedding-3-large",
        display_name: "text-embedding-3-large",
        dimension: Some(3072),
        supported_dimensions: &[256, 1024, 3072],
        max_input_tokens: Some(8191),
        document_prefix: "",
        query_prefix: "",
        recommended_for_code: false,
        notes: "Highest-quality OpenAI embedder; truncatable via `dimensions`.",
    },
    EmbeddingModel {
        provider: ProviderKind::OpenAI,
        id: "text-embedding-ada-002",
        display_name: "text-embedding-ada-002 (legacy)",
        dimension: Some(1536),
        supported_dimensions: &[],
        max_input_tokens: Some(8191),
        document_prefix: "",
        query_prefix: "",
        recommended_for_code: false,
        notes: "Superseded by the v3 models. No dimension truncation.",
    },
    // ── Voyage ───────────────────────────────────────────────────────────────
    EmbeddingModel {
        provider: ProviderKind::Voyage,
        id: "voyage-code-3",
        display_name: "voyage-code-3",
        dimension: Some(1024),
        supported_dimensions: &[256, 512, 1024, 2048],
        max_input_tokens: Some(32000),
        document_prefix: "",
        query_prefix: "",
        recommended_for_code: true,
        notes: "Trained for code retrieval — the strongest option for indexing \
                a source tree. Native query/document input types.",
    },
    EmbeddingModel {
        provider: ProviderKind::Voyage,
        id: "voyage-3-large",
        display_name: "voyage-3-large",
        dimension: Some(1024),
        supported_dimensions: &[256, 512, 1024, 2048],
        max_input_tokens: Some(32000),
        document_prefix: "",
        query_prefix: "",
        recommended_for_code: false,
        notes: "General-purpose flagship.",
    },
    EmbeddingModel {
        provider: ProviderKind::Voyage,
        id: "voyage-3.5",
        display_name: "voyage-3.5",
        dimension: Some(1024),
        supported_dimensions: &[256, 512, 1024, 2048],
        max_input_tokens: Some(32000),
        document_prefix: "",
        query_prefix: "",
        recommended_for_code: false,
        notes: "Balanced quality/cost.",
    },
    EmbeddingModel {
        provider: ProviderKind::Voyage,
        id: "voyage-3.5-lite",
        display_name: "voyage-3.5-lite",
        dimension: Some(1024),
        supported_dimensions: &[256, 512, 1024, 2048],
        max_input_tokens: Some(32000),
        document_prefix: "",
        query_prefix: "",
        recommended_for_code: false,
        notes: "Cheapest Voyage tier.",
    },
    // ── Cohere ───────────────────────────────────────────────────────────────
    EmbeddingModel {
        provider: ProviderKind::Cohere,
        id: "embed-v4.0",
        display_name: "Cohere Embed v4",
        dimension: Some(1536),
        supported_dimensions: &[256, 512, 1024, 1536],
        max_input_tokens: None,
        document_prefix: "",
        query_prefix: "",
        recommended_for_code: false,
        notes: "Long context and Matryoshka truncation. Native input types.",
    },
    EmbeddingModel {
        provider: ProviderKind::Cohere,
        id: "embed-english-v3.0",
        display_name: "Cohere Embed English v3",
        dimension: Some(1024),
        supported_dimensions: &[],
        max_input_tokens: Some(512),
        document_prefix: "",
        query_prefix: "",
        recommended_for_code: false,
        notes: "English-only v3 generation.",
    },
    EmbeddingModel {
        provider: ProviderKind::Cohere,
        id: "embed-multilingual-v3.0",
        display_name: "Cohere Embed Multilingual v3",
        dimension: Some(1024),
        supported_dimensions: &[],
        max_input_tokens: Some(512),
        document_prefix: "",
        query_prefix: "",
        recommended_for_code: false,
        notes: "100+ languages.",
    },
    EmbeddingModel {
        provider: ProviderKind::Cohere,
        id: "embed-english-light-v3.0",
        display_name: "Cohere Embed English Light v3",
        dimension: Some(384),
        supported_dimensions: &[],
        max_input_tokens: Some(512),
        document_prefix: "",
        query_prefix: "",
        recommended_for_code: false,
        notes: "Smallest Cohere embedder.",
    },
    // ── Gemini ───────────────────────────────────────────────────────────────
    EmbeddingModel {
        provider: ProviderKind::Gemini,
        id: "gemini-embedding-001",
        display_name: "Gemini Embedding 001",
        dimension: Some(3072),
        supported_dimensions: &[768, 1536, 3072],
        max_input_tokens: Some(2048),
        document_prefix: "",
        query_prefix: "",
        recommended_for_code: false,
        notes: "Matryoshka via `outputDimensionality`. Native task types.",
    },
    EmbeddingModel {
        provider: ProviderKind::Gemini,
        id: "text-embedding-004",
        display_name: "text-embedding-004",
        dimension: Some(768),
        supported_dimensions: &[],
        max_input_tokens: Some(2048),
        document_prefix: "",
        query_prefix: "",
        recommended_for_code: false,
        notes: "Previous-generation Gemini embedder.",
    },
    // ── Local (in-process candle) ────────────────────────────────────────────
    EmbeddingModel {
        provider: ProviderKind::Local,
        id: "all-MiniLM-L6-v2",
        display_name: "all-MiniLM-L6-v2 (in-process)",
        dimension: Some(384),
        supported_dimensions: &[],
        max_input_tokens: Some(256),
        document_prefix: "",
        query_prefix: "",
        recommended_for_code: false,
        notes: "Runs inside the process via candle — no daemon, no network, no \
                API key. Requires a build with --features candle.",
    },
];

/// All catalog entries.
pub fn all_models() -> &'static [EmbeddingModel] {
    CATALOG
}

/// Catalog entries for one provider.
pub fn models_for(provider: ProviderKind) -> impl Iterator<Item = &'static EmbeddingModel> {
    CATALOG.iter().filter(move |m| m.provider == provider)
}

/// Look up a model, tolerating an Ollama `:tag` suffix (`nomic-embed-text:v1.5`
/// resolves to the `nomic-embed-text` entry) and an `owner/` prefix.
pub fn lookup(provider: ProviderKind, model: &str) -> Option<&'static EmbeddingModel> {
    let bare = model.rsplit('/').next().unwrap_or(model);
    let untagged = bare.split(':').next().unwrap_or(bare);
    CATALOG.iter().find(|m| {
        m.provider == provider
            && (m.id.eq_ignore_ascii_case(model)
                || m.id.eq_ignore_ascii_case(bare)
                || m.id.eq_ignore_ascii_case(untagged))
    })
}

/// The model to offer first for a provider — the code-specialised one where
/// we have it, otherwise the first catalog entry.
pub fn default_model_for(provider: ProviderKind) -> Option<&'static EmbeddingModel> {
    models_for(provider)
        .find(|m| m.recommended_for_code)
        .or_else(|| models_for(provider).next())
}

/// Heuristic for "this Ollama model is an embedder, not a chat model".
///
/// Used to *include* embedding models in the embedding picker, and to keep
/// excluding them from the chat picker. Name-based because Ollama's
/// `/api/tags` does not report an embedding capability flag.
pub fn looks_like_embedding_model(name: &str, family: &str) -> bool {
    const FAMILIES: &[&str] = &["nomic-bert", "bert", "all-minilm", "gemma3"];
    const NAME_MARKERS: &[&str] = &[
        "embed",
        "all-minilm",
        "bge-",
        "bge_",
        "gte-",
        "e5-",
        "paraphrase-multilingual",
    ];
    let lower = name.to_ascii_lowercase();
    // `gemma3` alone is a chat family; only embeddinggemma counts.
    let family_hit = FAMILIES
        .iter()
        .any(|f| family.eq_ignore_ascii_case(f) && *f != "gemma3");
    family_hit || NAME_MARKERS.iter().any(|m| lower.contains(m))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lookup_ignores_ollama_tag() {
        let m = lookup(ProviderKind::Ollama, "nomic-embed-text:v1.5");
        assert_eq!(m.map(|m| m.id), Some("nomic-embed-text"));
    }

    #[test]
    fn lookup_ignores_registry_owner_prefix() {
        let m = lookup(ProviderKind::Ollama, "library/bge-m3:latest");
        assert_eq!(m.map(|m| m.id), Some("bge-m3"));
    }

    #[test]
    fn lookup_is_provider_scoped() {
        assert!(lookup(ProviderKind::OpenAI, "nomic-embed-text").is_none());
    }

    #[test]
    fn nomic_prefixes_differ_by_kind() {
        let m = lookup(ProviderKind::Ollama, "nomic-embed-text").expect("catalog entry");
        let d = m.apply_prefix("fn main() {}", EmbedKind::Document);
        let q = m.apply_prefix("fn main() {}", EmbedKind::Query);
        assert_ne!(d, q);
        assert!(d.starts_with("search_document: "));
        assert!(q.starts_with("search_query: "));
    }

    /// A symmetric model must not allocate — the borrowed branch is what keeps
    /// prefixing off the hot path for the majority of models.
    #[test]
    fn symmetric_model_borrows_input() {
        let m = lookup(ProviderKind::Ollama, "bge-m3").expect("catalog entry");
        let out = m.apply_prefix("hello", EmbedKind::Document);
        assert!(matches!(out, std::borrow::Cow::Borrowed("hello")));
    }

    /// mxbai prefixes queries only — a symmetric application would be wrong.
    #[test]
    fn mxbai_prefixes_query_only() {
        let m = lookup(ProviderKind::Ollama, "mxbai-embed-large").expect("catalog entry");
        assert_eq!(m.apply_prefix("x", EmbedKind::Document), "x");
        assert!(m.apply_prefix("x", EmbedKind::Query).starts_with("Represent this sentence"));
    }

    #[test]
    fn every_catalog_entry_has_a_dimension_or_says_so() {
        // Every shipped entry is documented, so all of them declare one. This
        // guards against someone adding an entry with a guessed `None` that
        // then reads as "probe me" for a model we do know.
        assert!(CATALOG.iter().all(|m| m.dimension.is_some()));
    }

    #[test]
    fn catalog_ids_are_unique_per_provider() {
        let dupes = CATALOG.iter().filter(|a| {
            CATALOG
                .iter()
                .filter(|b| b.provider == a.provider && b.id == a.id)
                .count()
                > 1
        });
        assert_eq!(dupes.count(), 0);
    }

    #[test]
    fn supported_dimensions_include_native() {
        for m in CATALOG.iter().filter(|m| !m.supported_dimensions.is_empty()) {
            let native = m.dimension.expect("catalog entries declare a dimension");
            assert!(
                m.supports_dimension(native),
                "{} rejects its own native dimension",
                m.id
            );
        }
    }

    #[test]
    fn voyage_code_is_the_code_default() {
        let d = default_model_for(ProviderKind::Voyage).expect("voyage has models");
        assert_eq!(d.id, "voyage-code-3");
    }

    #[test]
    fn every_provider_has_a_default() {
        for p in ProviderKind::ALL {
            assert!(default_model_for(*p).is_some(), "{p} has no catalog entry");
        }
    }

    #[test]
    fn embedding_model_detection() {
        assert!(looks_like_embedding_model("nomic-embed-text:latest", "nomic-bert"));
        assert!(looks_like_embedding_model("bge-m3", ""));
        assert!(looks_like_embedding_model("embeddinggemma", "gemma3"));
        assert!(!looks_like_embedding_model("llama3.2:3b", "llama"));
        // A chat Gemma must not be mistaken for an embedder by family alone.
        assert!(!looks_like_embedding_model("gemma3:4b", "gemma3"));
    }
}
