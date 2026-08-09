//! Static model catalog — the single source of truth for the daemon's
//! `/models` endpoint.
//!
//! Thin daemon clients (VibeDesk, VibeAIChat, VibeMobile, the watch apps, the editor
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
    "claude-opus-5",
    "claude-sonnet-5",
    "claude-opus-4-8",
    "claude-opus-4-7",
    "claude-opus-4-6",
    "claude-sonnet-4-6",
    "claude-haiku-4-5",
    "claude-sonnet-4-5",
];

const OPENAI: &[&str] = &[
    "gpt-5.6-sol-pro",
    "gpt-5.6-sol",
    "gpt-5.6-terra-pro",
    "gpt-5.6-terra",
    "gpt-5.6-luna-pro",
    "gpt-5.6-luna",
    "gpt-5.5-pro",
    "gpt-5.5",
    "gpt-5.4",
    "gpt-5.4-mini",
    "gpt-5.3-codex",
    "gpt-5.3-chat",
    "gpt-5",
    "gpt-4.1",
    "gpt-4.1-mini",
    "gpt-4o",
    "gpt-4o-mini",
];

const GEMINI: &[&str] = &[
    "gemini-3.5-pro",
    "gemini-3.6-flash",
    "gemini-3.5-flash",
    "gemini-3.5-flash-lite",
    "gemini-3.1-pro",
    "gemini-3-pro",
    "gemini-2.5-pro",
    "gemini-2.5-flash",
];

const GROK: &[&str] = &["grok-4.5", "grok-4.3", "grok-4.20"];

// llama-3.1-8b-instant / llama-3.3-70b-versatile deprecated 2026-06-17;
// mixtral-8x7b-32768 and gemma2-9b-it are gone.
const GROQ: &[&str] = &[
    "openai/gpt-oss-120b",
    "openai/gpt-oss-20b",
    "qwen/qwen3.6-27b",
    "minimaxai/minimax-m2.7",
    "groq/compound",
    "groq/compound-mini",
];

const MISTRAL: &[&str] = &[
    "mistral-large-latest",
    "mistral-medium-latest",
    "mistral-small-latest",
    "codestral-latest",
];

const DEEPSEEK: &[&str] = &[
    "deepseek-v4-pro",
    "deepseek-v4-flash",
    "deepseek-chat",
    "deepseek-reasoner",
];

const CEREBRAS: &[&str] = &["gpt-oss-120b", "gemma-4-31b", "zai-glm-4.7"];

const PERPLEXITY: &[&str] = &[
    "sonar-pro",
    "sonar",
    "sonar-reasoning-pro",
    "sonar-deep-research",
];

const TOGETHER: &[&str] = &[
    "moonshotai/Kimi-K2.7-Code",
    "Qwen/Qwen3.8-Max",
    "Qwen/Qwen3.5-397B-A17B",
    "deepseek-ai/DeepSeek-V4-Pro",
];

const FIREWORKS: &[&str] = &[
    "accounts/fireworks/models/llama-v3p3-70b-instruct",
    "accounts/fireworks/models/mixtral-8x7b-instruct",
];

// Verified against the live openrouter.ai/api/v1/models catalog on 2026-08-05.
const OPENROUTER: &[&str] = &[
    "moonshotai/kimi-k3",
    "moonshotai/kimi-k2.7-code",
    "moonshotai/kimi-k2.6",
    "z-ai/glm-5.2",
    "qwen/qwen3.8-max",
    "deepseek/deepseek-v4-pro",
    "minimax/minimax-m3",
    "x-ai/grok-4.5",
    "anthropic/claude-opus-5",
    "anthropic/claude-sonnet-5",
    "openai/gpt-5.6-sol",
    "google/gemini-3.6-flash",
];

const AZURE_OPENAI: &[&str] = &["gpt-4o", "gpt-4-turbo"];

const BEDROCK: &[&str] = &[
    "anthropic.claude-opus-5",
    "anthropic.claude-sonnet-5",
    "anthropic.claude-opus-4-8",
    "anthropic.claude-haiku-4-5",
];

const COPILOT: &[&str] = &["gpt-4o"];

const ZHIPU: &[&str] = &["glm-5.2", "glm-5.1", "glm-5", "glm-4.7", "glm-4.7-flash"];

const MINIMAX: &[&str] = &["MiniMax-M3", "MiniMax-M2.7"];

const SAMBANOVA: &[&str] = &["Meta-Llama-3.3-70B-Instruct"];

const POOLSIDE: &[&str] = &["laguna-s-2.1", "laguna-xs-2.1", "laguna-m-1"];

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
    ("poolside", POOLSIDE),
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
        "poolside",
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
        assert_eq!(
            set.len(),
            OLLAMA_CHAT_MODELS.len(),
            "duplicate ollama chat model"
        );
    }
}
