//! Static model catalog — the single source of truth for the daemon's
//! `/models` endpoint.
//!
//! Thin daemon clients (VibeDesk, VibeApp, VibeMobile, the watch apps, the editor
//! plugins) render whatever `/models` returns instead of carrying their own
//! hardcoded lists. This module is that list. It mirrors the desktop registry
//! (`vibecoder/src/hooks/useModelRegistry.ts` + `constants/ollamaModels.ts`) — keep
//! them in sync when models change.
//!
//! Provider ids match the `create_provider` match arms in the daemon
//! (`vibecli-cli/src/main.rs`), so a model a client selects here round-trips
//! back to a provider the daemon can actually build. Ollama is served
//! separately by the endpoint (live `/api/tags` locals + [`OLLAMA_CHAT_MODELS`]
//! + `providers::ollama::OLLAMA_CLOUD_MODELS`), so it is intentionally absent
//! from [`PROVIDER_MODELS`].

/// Ollama chat models addressable via a local pull or ollama.com. Excludes the
/// `*-cloud` datacenter models, which live in
/// [`crate::providers::ollama::OLLAMA_CLOUD_MODELS`] and are unioned in by the
/// endpoint. Source: <https://ollama.com/library?sort=newest>.
pub const OLLAMA_CHAT_MODELS: &[&str] = &[
    // Cloud-hosted flagship coding / agentic (run on Ollama Cloud with a token).
    "devstral-2",
    "devstral-small-2",
    "nemotron-3-super",
    "nemotron-3-nano",
    "cogito-2.1",
    "gemma4",
    "ministral-3",
    "rnj-1",
    "gemini-3-flash-preview",
    // Latest / flagship (mixed origin).
    "qwen3-coder",
    "qwen3.6",
    "qwen3.5",
    "qwen3",
    "qwen3-next",
    "qwen3-coder-next",
    "deepseek-v4-pro",
    "deepseek-v4-flash",
    "deepseek-v3.2",
    "deepseek-v3",
    "deepseek-r1",
    "llama4",
    "llama3.3",
    "llama3.2",
    "gemma3",
    "gemma3n",
    "phi4",
    "phi4-reasoning",
    "phi4-mini-reasoning",
    "phi4-mini",
    "mistral-large-3",
    "mistral-small3.2",
    "mistral-small3.1",
    // Strong reasoning / agentic.
    "glm-5.1",
    "glm-5",
    "glm-4.7",
    "glm-4.7-flash",
    // Code-specialised smaller models.
    "codellama",
    "codegemma",
    "starcoder2",
    "qwen2.5-coder",
];

const CLAUDE: &[&str] = &[
    "claude-opus-4-8",
    "claude-opus-4-7",
    "claude-opus-4-6",
    "claude-sonnet-4-6",
    "claude-haiku-4-5",
    "claude-sonnet-4-5",
    "claude-3-5-sonnet-20241022",
];

const OPENAI: &[&str] = &[
    "gpt-5.5",
    "gpt-5.4",
    "gpt-5.3-codex",
    "gpt-5.3-codex-spark",
    "gpt-5",
    "gpt-4o",
    "gpt-4o-mini",
    "gpt-4-turbo",
    "o4-mini",
    "o3",
    "o3-mini",
    "gpt-4.1",
    "gpt-4.1-mini",
    "gpt-4.1-nano",
];

const GEMINI: &[&str] = &[
    "gemini-3.5-pro",
    "gemini-3.5-flash",
    "gemini-3.1-pro",
    "gemini-3-pro",
    "gemini-2.5-pro",
    "gemini-2.5-flash",
    "gemini-2.0-flash",
    "gemini-2.0-flash-lite",
];

const GROK: &[&str] = &["grok-3", "grok-3-mini", "grok-2"];

const GROQ: &[&str] = &[
    "llama-3.3-70b-versatile",
    "llama-3.1-8b-instant",
    "mixtral-8x7b-32768",
    "gemma2-9b-it",
];

const MISTRAL: &[&str] = &[
    "mistral-large-latest",
    "mistral-medium-latest",
    "mistral-small-latest",
    "codestral-latest",
];

const DEEPSEEK: &[&str] = &[
    "deepseek-v4",
    "deepseek-v4-flash",
    "deepseek-chat",
    "deepseek-reasoner",
    "deepseek-coder",
];

const CEREBRAS: &[&str] = &["llama-3.3-70b", "llama-3.1-8b"];

const PERPLEXITY: &[&str] = &["sonar-pro", "sonar", "sonar-reasoning"];

const TOGETHER: &[&str] = &[
    "meta-llama/Llama-3.3-70B-Instruct",
    "mistralai/Mixtral-8x7B-Instruct-v0.1",
];

const FIREWORKS: &[&str] = &[
    "accounts/fireworks/models/llama-v3p3-70b-instruct",
    "accounts/fireworks/models/mixtral-8x7b-instruct",
];

const OPENROUTER: &[&str] = &[
    "moonshotai/kimi-k2.7-code",
    "moonshotai/kimi-k2.6",
    "z-ai/glm-5.2",
    "qwen/qwen3.6-coder",
    "deepseek/deepseek-v4",
    "anthropic/claude-3.5-sonnet",
    "openai/gpt-4o",
    "google/gemini-2.0-flash-001",
];

const AZURE_OPENAI: &[&str] = &["gpt-4o", "gpt-4-turbo"];

const BEDROCK: &[&str] = &[
    "anthropic.claude-3-5-sonnet-20241022-v2:0",
    "anthropic.claude-3-haiku-20240307-v1:0",
];

const COPILOT: &[&str] = &["gpt-4o"];

const ZHIPU: &[&str] = &["glm-5.2", "glm-5.1", "glm-4-plus", "glm-4-flash"];

const MINIMAX: &[&str] = &["MiniMax-M3", "abab6.5s-chat"];

const SAMBANOVA: &[&str] = &["Meta-Llama-3.3-70B-Instruct"];

const VIBECLI_MISTRALRS: &[&str] = &[
    "meta-llama/Llama-3.1-8B-Instruct",
    "meta-llama/Llama-3.2-3B-Instruct",
    "Qwen/Qwen3.6-Coder-7B-Instruct",
    "Qwen/Qwen3.6-7B-Instruct",
    "Qwen/Qwen2.5-Coder-7B-Instruct",
    "Qwen/Qwen2.5-7B-Instruct",
    "microsoft/Phi-3.5-mini-instruct",
];

/// `(provider_id, models)` for every non-ollama provider the daemon supports.
/// Provider ids are the canonical (first) alias of each `create_provider` arm.
pub const PROVIDER_MODELS: &[(&str, &[&str])] = &[
    ("claude", CLAUDE),
    ("openai", OPENAI),
    ("gemini", GEMINI),
    ("grok", GROK),
    ("groq", GROQ),
    ("mistral", MISTRAL),
    ("deepseek", DEEPSEEK),
    ("cerebras", CEREBRAS),
    ("perplexity", PERPLEXITY),
    ("together", TOGETHER),
    ("fireworks", FIREWORKS),
    ("openrouter", OPENROUTER),
    ("azure_openai", AZURE_OPENAI),
    ("bedrock", BEDROCK),
    ("copilot", COPILOT),
    ("zhipu", ZHIPU),
    ("minimax", MINIMAX),
    ("sambanova", SAMBANOVA),
    ("vibecli-mistralrs", VIBECLI_MISTRALRS),
];

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    /// Provider ids must be aliases the daemon's `create_provider` can build,
    /// so a selected model round-trips back to a real provider.
    const KNOWN_PROVIDER_IDS: &[&str] = &[
        "claude",
        "openai",
        "gemini",
        "grok",
        "groq",
        "openrouter",
        "azure_openai",
        "bedrock",
        "copilot",
        "mistral",
        "cerebras",
        "deepseek",
        "zhipu",
        "minimax",
        "perplexity",
        "together",
        "fireworks",
        "sambanova",
        "vibecli-mistralrs",
    ];

    #[test]
    fn every_provider_id_is_buildable_by_the_daemon() {
        for (provider, _) in PROVIDER_MODELS {
            assert!(
                KNOWN_PROVIDER_IDS.contains(provider),
                "catalog provider `{provider}` has no create_provider arm"
            );
        }
    }

    #[test]
    fn every_provider_lists_at_least_one_model() {
        for (provider, models) in PROVIDER_MODELS {
            assert!(!models.is_empty(), "provider `{provider}` has no models");
        }
    }

    #[test]
    fn provider_ids_are_unique() {
        let ids: HashSet<_> = PROVIDER_MODELS.iter().map(|(p, _)| *p).collect();
        assert_eq!(ids.len(), PROVIDER_MODELS.len(), "duplicate provider id");
    }

    #[test]
    fn model_ids_do_not_collide_within_a_provider() {
        for (provider, models) in PROVIDER_MODELS {
            let set: HashSet<_> = models.iter().collect();
            assert_eq!(set.len(), models.len(), "duplicate model in `{provider}`");
        }
    }

    /// `*-cloud` models are datacenter-hosted and live in
    /// `providers::ollama::OLLAMA_CLOUD_MODELS`; the chat catalog is pull-able.
    #[test]
    fn ollama_chat_catalog_excludes_cloud_models() {
        for m in OLLAMA_CHAT_MODELS {
            assert!(
                !m.contains("cloud"),
                "`{m}` is a cloud model — belongs in OLLAMA_CLOUD_MODELS"
            );
        }
        let set: HashSet<_> = OLLAMA_CHAT_MODELS.iter().collect();
        assert_eq!(set.len(), OLLAMA_CHAT_MODELS.len(), "duplicate ollama chat model");
    }
}
