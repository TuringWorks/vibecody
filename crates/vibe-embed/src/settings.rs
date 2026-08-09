//! The user's embedding-model choice, and how it becomes an [`Embedder`].
//!
//! Every client — the CLI, the daemon, VibeCoder, the remote indexer — must
//! resolve a model the same way, for the same reason `daemon_bootstrap.rs`
//! exists: divergent copies of "which model are we using?" produce panels
//! that disagree about what is indexed and why search returns nothing.
//!
//! This module deliberately does **not** depend on the ProfileStore. It takes
//! a lookup closure instead, so the credential path stays in the crates that
//! already own encryption, and `vibe-embed` stays linkable from the indexer
//! and the mobile bridge without pulling sqlite in.
//!
//! [`Embedder`]: crate::Embedder

use crate::{catalog, EmbeddingConfig, ModelRef, ProviderKind, Result, SharedEmbedder};
use serde::{Deserialize, Serialize};

/// The persisted embedding-model choice.
///
/// Stored in the ProfileStore (global default) and the WorkspaceStore (per
/// project override) by the caller. Never in a plaintext TOML — the resolved
/// [`EmbeddingConfig`] carries an API key, this struct does not.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmbeddingSettings {
    pub provider: ProviderKind,
    pub model: String,
    /// Explicit Matryoshka output dimension. `None` = the model's native size.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dimensions: Option<usize>,
    /// Endpoint override — remote Ollama, Azure, a proxy, a TEI server.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
}

impl Default for EmbeddingSettings {
    /// Zero-config first: a local Ollama model that needs no API key, no
    /// account, and no configuration file. If Ollama is not installed the
    /// error names it — which is a better first run than a provider picker
    /// with nothing selected.
    fn default() -> Self {
        Self {
            provider: ProviderKind::Ollama,
            model: "nomic-embed-text".to_string(),
            dimensions: None,
            base_url: None,
        }
    }
}

impl EmbeddingSettings {
    pub fn new(provider: ProviderKind, model: impl Into<String>) -> Self {
        Self {
            provider,
            model: model.into(),
            dimensions: None,
            base_url: None,
        }
    }

    /// Parse the string form used by config files and CLI flags
    /// (`ollama/nomic-embed-text`, or a bare model name with a provider).
    ///
    /// Returns `None` for an unrecognised provider rather than silently
    /// falling back to a default — a typo'd provider name should be visible,
    /// not quietly rerouted to Ollama.
    pub fn parse(provider: &str, model: &str) -> Option<Self> {
        // Tolerate the combined `provider/model` form in either argument.
        let (provider, model) = match model.split_once('/') {
            Some((p, m)) if ProviderKind::parse(p).is_some() && provider.is_empty() => (p, m),
            _ => (provider, model),
        };
        let kind = ProviderKind::parse(provider)?;
        (!model.trim().is_empty()).then(|| Self::new(kind, model.trim()))
    }

    pub fn with_dimensions(mut self, dimensions: Option<usize>) -> Self {
        self.dimensions = dimensions;
        self
    }

    pub fn with_base_url(mut self, base_url: Option<String>) -> Self {
        self.base_url = base_url.filter(|s| !s.trim().is_empty());
        self
    }

    pub fn model_ref(&self) -> ModelRef {
        ModelRef::new(self.provider, self.model.clone()).with_dimensions(self.dimensions)
    }

    /// Catalog metadata, if we ship any for this model.
    pub fn catalog_entry(&self) -> Option<&'static catalog::EmbeddingModel> {
        catalog::lookup(self.provider, &self.model)
    }

    /// Turn the choice into a config, pulling the API key from `api_key` — a
    /// closure over the encrypted store.
    ///
    /// The closure is called only for providers that need a key, so selecting
    /// a local model never touches the credential store.
    pub fn to_config<F>(&self, api_key: F) -> EmbeddingConfig
    where
        F: FnOnce(ProviderKind) -> Option<String>,
    {
        let key = self.provider.requires_api_key().then(|| api_key(self.provider)).flatten();
        EmbeddingConfig::new(self.model_ref())
            .with_base_url(self.base_url.clone())
            .with_api_key(key)
    }

    /// Resolve all the way to a live embedder.
    pub fn build<F>(&self, api_key: F) -> Result<SharedEmbedder>
    where
        F: FnOnce(ProviderKind) -> Option<String>,
    {
        self.to_config(api_key).build()
    }

    /// One-line description for the startup banner and `/health`.
    pub fn describe(&self) -> String {
        let dim = self
            .model_ref()
            .known_dimension()
            .map_or_else(|| "dimension probed on first use".to_string(), |d| format!("{d}d"));
        let locality = if self.provider.is_local() {
            "local"
        } else {
            "cloud"
        };
        format!(
            "{}/{} ({dim}, {locality})",
            self.provider.as_str(),
            self.model
        )
    }
}

// ---------------------------------------------------------------------------
// Availability
// ---------------------------------------------------------------------------

/// Why a provider can or cannot be used right now. Surfaced in the model
/// picker so an unavailable provider explains itself instead of failing at
/// index time.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum Availability {
    /// Usable now.
    Ready,
    /// Needs a key the user has not supplied.
    NeedsApiKey,
    /// Compiled out (the in-process backend without `--features candle`).
    NotCompiledIn,
}

/// A provider plus its models and current availability.
#[derive(Debug, Clone, Serialize)]
pub struct ProviderCatalog {
    pub provider: ProviderKind,
    pub id: &'static str,
    pub display_name: &'static str,
    pub requires_api_key: bool,
    /// True when embedding stays on this machine. Shown in the picker so a
    /// user indexing a private repo knows before they choose that a cloud
    /// model ships their source to a third party.
    pub is_local: bool,
    pub availability: Availability,
    pub models: Vec<&'static catalog::EmbeddingModel>,
    /// Model offered first for this provider.
    pub default_model: Option<&'static str>,
}

/// Describe every provider and its models, given which providers hold a key.
///
/// `has_key` is a closure over the credential store; it is not called for
/// providers that need no key.
pub fn provider_catalog<F>(mut has_key: F) -> Vec<ProviderCatalog>
where
    F: FnMut(ProviderKind) -> bool,
{
    ProviderKind::ALL
        .iter()
        .copied()
        .map(|provider| {
            let availability = match provider {
                ProviderKind::Local if !crate::providers::local_backend_available() => {
                    Availability::NotCompiledIn
                }
                p if p.requires_api_key() && !has_key(p) => Availability::NeedsApiKey,
                _ => Availability::Ready,
            };
            ProviderCatalog {
                provider,
                id: provider.as_str(),
                display_name: provider.display_name(),
                requires_api_key: provider.requires_api_key(),
                is_local: provider.is_local(),
                availability,
                models: catalog::models_for(provider).collect(),
                default_model: catalog::default_model_for(provider).map(|m| m.id),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_a_local_model_needing_no_key() {
        let d = EmbeddingSettings::default();
        assert_eq!(d.provider, ProviderKind::Ollama);
        assert!(!d.provider.requires_api_key());
        assert!(d.catalog_entry().is_some());
    }

    #[test]
    fn parse_accepts_provider_and_model() {
        let s = EmbeddingSettings::parse("voyage", "voyage-code-3").expect("parses");
        assert_eq!(s.provider, ProviderKind::Voyage);
        assert_eq!(s.model, "voyage-code-3");
    }

    #[test]
    fn parse_accepts_the_combined_form() {
        let s = EmbeddingSettings::parse("", "openai/text-embedding-3-large").expect("parses");
        assert_eq!(s.provider, ProviderKind::OpenAI);
        assert_eq!(s.model, "text-embedding-3-large");
    }

    /// An Ollama model name can contain a slash (`library/bge-m3`). The
    /// combined-form shortcut must not eat it.
    #[test]
    fn parse_keeps_slashes_inside_a_model_name() {
        let s = EmbeddingSettings::parse("ollama", "library/bge-m3").expect("parses");
        assert_eq!(s.model, "library/bge-m3");
    }

    /// A typo must be visible, not silently rerouted to the default provider.
    #[test]
    fn parse_rejects_an_unknown_provider() {
        assert!(EmbeddingSettings::parse("opnai", "text-embedding-3-small").is_none());
        assert!(EmbeddingSettings::parse("ollama", "  ").is_none());
    }

    #[test]
    fn local_providers_never_consult_the_credential_store() {
        let settings = EmbeddingSettings::default();
        let cfg = settings.to_config(|_| panic!("must not read a key for a local provider"));
        assert!(cfg.api_key.is_none());
    }

    #[test]
    fn cloud_providers_receive_their_key() {
        let settings = EmbeddingSettings::new(ProviderKind::Voyage, "voyage-code-3");
        let cfg = settings.to_config(|p| {
            assert_eq!(p, ProviderKind::Voyage);
            Some("secret".into())
        });
        assert_eq!(cfg.api_key.as_deref(), Some("secret"));
    }

    #[test]
    fn build_fails_cleanly_when_the_key_is_absent() {
        let settings = EmbeddingSettings::new(ProviderKind::Cohere, "embed-v4.0");
        assert!(matches!(
            settings.build(|_| None),
            Err(crate::EmbeddingError::MissingApiKey("cohere"))
        ));
    }

    #[test]
    fn describe_names_dimension_and_locality() {
        let d = EmbeddingSettings::default().describe();
        assert!(d.contains("768d"), "{d}");
        assert!(d.contains("local"), "{d}");

        let unknown = EmbeddingSettings::new(ProviderKind::Ollama, "mystery").describe();
        assert!(unknown.contains("probed"), "{unknown}");
    }

    #[test]
    fn catalog_marks_keyless_providers_ready() {
        let cats = provider_catalog(|_| false);
        let ollama = cats
            .iter()
            .find(|c| c.provider == ProviderKind::Ollama)
            .expect("ollama listed");
        assert_eq!(ollama.availability, Availability::Ready);
        assert!(!ollama.models.is_empty());
    }

    #[test]
    fn catalog_flags_cloud_providers_without_keys() {
        let cats = provider_catalog(|_| false);
        for c in cats.iter().filter(|c| c.requires_api_key) {
            assert_eq!(c.availability, Availability::NeedsApiKey, "{}", c.id);
        }
    }

    #[test]
    fn catalog_marks_cloud_providers_ready_once_keyed() {
        let cats = provider_catalog(|_| true);
        let voyage = cats
            .iter()
            .find(|c| c.provider == ProviderKind::Voyage)
            .expect("voyage listed");
        assert_eq!(voyage.availability, Availability::Ready);
        assert_eq!(voyage.default_model, Some("voyage-code-3"));
    }

    #[test]
    fn catalog_reports_locality_per_provider() {
        let cats = provider_catalog(|_| true);
        assert!(cats.iter().find(|c| c.provider == ProviderKind::Ollama).expect("ollama").is_local);
        assert!(!cats.iter().find(|c| c.provider == ProviderKind::OpenAI).expect("openai").is_local);
    }

    #[test]
    fn settings_roundtrip_through_json() {
        let s = EmbeddingSettings::new(ProviderKind::Gemini, "gemini-embedding-001")
            .with_dimensions(Some(768))
            .with_base_url(Some("https://proxy.example".into()));
        let json = serde_json::to_string(&s).expect("serialises");
        assert_eq!(
            serde_json::from_str::<EmbeddingSettings>(&json).expect("parses"),
            s
        );
    }
}
