//! AI provider implementations

pub mod azure_openai;
pub mod bedrock;
pub mod cerebras;
pub mod claude;
pub mod copilot;
pub mod deepseek;
pub mod failover;
pub mod fireworks;
pub mod gemini;
pub mod grok;
pub mod groq;
pub mod local_edit;
pub mod minimax;
pub mod mistral;
pub mod ollama;
pub mod openai;
pub mod openai_compat;
pub mod openrouter;
pub mod perplexity;
pub mod poolside;
pub mod sambanova;
pub mod together;
pub mod vercel_ai;
pub mod vibecli_mistralrs;
pub mod zhipu;

/// Every provider's `name()` label, paired with the `provider_type` that built
/// it — the reverse of the `format!("<Label> ({})", config.model)` each
/// constructor writes.
///
/// `ChatEngine::get_provider_names()` returns those display names, and that is
/// what fills the desktop toolbar's provider dropdown, so a client's `provider`
/// field routinely arrives as `"Ollama (gpt-oss:120b-cloud)"` rather than
/// `"ollama"`. Anything dispatching on `provider_type` has to come back through
/// here first or it falls off the end of its match with "not configured".
///
/// Keep in sync with `DISPLAY_LABEL_TO_PROVIDER` in
/// `vibecoder/src/hooks/useModelRegistry.ts`.
pub const DISPLAY_LABELS: &[(&str, &str)] = &[
    ("AzureOpenAI", "azure_openai"),
    ("Bedrock", "bedrock"),
    ("Cerebras", "cerebras"),
    ("Claude", "claude"),
    ("Copilot", "copilot"),
    ("DeepSeek", "deepseek"),
    ("Fireworks AI", "fireworks"),
    ("Gemini", "gemini"),
    ("Grok", "grok"),
    ("Groq", "groq"),
    ("MiniMax", "minimax"),
    ("Mistral", "mistral"),
    ("Ollama", "ollama"),
    ("OpenAI", "openai"),
    ("OpenRouter", "openrouter"),
    ("Perplexity", "perplexity"),
    ("Poolside", "poolside"),
    ("SambaNova", "sambanova"),
    ("Together AI", "together"),
    ("VercelAI", "vercel_ai"),
    ("VibeCLI mistralrs", "vibecli-mistralrs"),
    ("Zhipu", "zhipu"),
];

/// Split a provider display name into its `provider_type` and model.
///
/// Returns `None` for anything that is not a display name — a bare
/// `provider_type` included — so callers can pass their input through
/// unchanged rather than have a guess substituted for it.
pub fn parse_display_name(display: &str) -> Option<(&'static str, &str)> {
    let trimmed = display.trim();
    let open = trimmed.rfind(" (")?;
    let model = trimmed.strip_suffix(')')?.get(open + 2..)?.trim();
    let label = &trimmed[..open];
    DISPLAY_LABELS
        .iter()
        .find(|(l, _)| l.eq_ignore_ascii_case(label))
        .map(|(_, provider_type)| (*provider_type, model))
}

pub use azure_openai::AzureOpenAIProvider;
pub use bedrock::BedrockProvider;
pub use cerebras::CerebrasProvider;
pub use claude::ClaudeProvider;
pub use copilot::CopilotProvider;
pub use deepseek::DeepSeekProvider;
pub use failover::FailoverProvider;
pub use fireworks::FireworksProvider;
pub use gemini::GeminiProvider;
pub use grok::GrokProvider;
pub use groq::GroqProvider;
pub use local_edit::LocalEditProvider;
pub use minimax::MiniMaxProvider;
pub use mistral::MistralProvider;
pub use ollama::OllamaProvider;
pub use openai::OpenAIProvider;
pub use openrouter::OpenRouterProvider;
pub use perplexity::PerplexityProvider;
pub use poolside::PoolsideProvider;
pub use sambanova::SambaNovaProvider;
pub use together::TogetherProvider;
pub use vercel_ai::VercelAIProvider;
pub use zhipu::ZhipuProvider;

#[cfg(test)]
mod display_name_tests {
    use super::*;
    use crate::provider::{AIProvider, ProviderConfig};
    use std::sync::Arc;

    fn config(model: &str) -> ProviderConfig {
        ProviderConfig {
            model: model.to_string(),
            ..Default::default()
        }
    }

    /// The map is only correct while it matches what the constructors write, so
    /// build every provider and parse its real `name()` back. A renamed label
    /// is otherwise invisible: the parse just starts returning `None` and each
    /// caller silently falls back to treating the label as a `provider_type`.
    #[test]
    fn every_provider_display_name_round_trips() {
        let m = "test-model-id";
        let providers: Vec<(&str, Arc<dyn AIProvider>)> = vec![
            ("azure_openai", Arc::new(AzureOpenAIProvider::new(config(m)))),
            ("bedrock", Arc::new(BedrockProvider::new(config(m)))),
            ("cerebras", Arc::new(CerebrasProvider::new(config(m)))),
            ("claude", Arc::new(ClaudeProvider::new(config(m)))),
            ("copilot", Arc::new(CopilotProvider::new(config(m)))),
            ("deepseek", Arc::new(DeepSeekProvider::new(config(m)))),
            ("fireworks", Arc::new(FireworksProvider::new(config(m)))),
            ("gemini", Arc::new(GeminiProvider::new(config(m)))),
            ("grok", Arc::new(GrokProvider::new(config(m)))),
            ("groq", Arc::new(GroqProvider::new(config(m)))),
            ("minimax", Arc::new(MiniMaxProvider::new(config(m)))),
            ("mistral", Arc::new(MistralProvider::new(config(m)))),
            ("ollama", Arc::new(OllamaProvider::new(config(m)))),
            ("openai", Arc::new(OpenAIProvider::new(config(m)))),
            ("openrouter", Arc::new(OpenRouterProvider::new(config(m)))),
            ("perplexity", Arc::new(PerplexityProvider::new(config(m)))),
            ("poolside", Arc::new(PoolsideProvider::new(config(m)))),
            ("sambanova", Arc::new(SambaNovaProvider::new(config(m)))),
            ("together", Arc::new(TogetherProvider::new(config(m)))),
            ("vercel_ai", Arc::new(VercelAIProvider::new(config(m)))),
            (
                "vibecli-mistralrs",
                Arc::new(vibecli_mistralrs::VibeCliMistralRsProvider::new(config(m))),
            ),
            ("zhipu", Arc::new(ZhipuProvider::new(config(m)))),
        ];

        for (expected_type, provider) in providers {
            let name = provider.name().to_string();
            assert_eq!(
                parse_display_name(&name),
                Some((expected_type, m)),
                "display name {name:?} no longer maps to {expected_type:?}"
            );
        }
    }

    #[test]
    fn model_ids_with_slashes_and_colons_survive() {
        assert_eq!(
            parse_display_name("Ollama (gpt-oss:120b-cloud)"),
            Some(("ollama", "gpt-oss:120b-cloud"))
        );
        assert_eq!(
            parse_display_name("Together AI (moonshotai/Kimi-K2.7-Code)"),
            Some(("together", "moonshotai/Kimi-K2.7-Code"))
        );
    }

    /// A bare `provider_type` is already what the dispatchers want; reporting
    /// `None` is what tells them to leave it alone.
    #[test]
    fn bare_provider_types_and_unknown_labels_are_not_display_names() {
        assert_eq!(parse_display_name("ollama"), None);
        assert_eq!(parse_display_name(""), None);
        assert_eq!(parse_display_name("Ollama"), None);
        assert_eq!(parse_display_name("Nonesuch (some-model)"), None);
        // No closing paren: not the shape a constructor writes.
        assert_eq!(parse_display_name("Ollama (devstral-2"), None);
    }
}
