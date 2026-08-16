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

// claude-fable-5 restored 2026-08-10: US export controls were lifted on
// 2026-06-30 and Fable 5 returned globally on 07-01 after a 19-day suspension.
// claude-mythos-5 is deliberately still absent — it came back only for approved
// US organisations, and a flat list cannot express "available to some callers",
// so offering it would 403 for most users. It waits on per-model availability
// metadata rather than being listed optimistically.
const CLAUDE: &[&str] = &[
    "claude-opus-5",
    "claude-fable-5",
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

// gemini-3.5-pro removed 2026-08-10 (R1): it has never GA'd. Announced at I/O
// on 2026-05-19, delayed three times, and as of August 2026 it is still a
// limited Vertex AI preview for selected enterprise customers — not in the
// consumer app, not in AI Studio. Offering it here made `/models` advertise an
// id the API rejects. Do not re-add until it ships; see the registry rule in
// useModelRegistry.ts.
const GEMINI: &[&str] = &[
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

/// Models commonly served by a local **vLLM** instance. vLLM serves whatever
/// the operator passed to `vllm serve`, so this is a starting list rather than
/// a claim about any particular machine — the pickers all accept a typed-in id,
/// and `/v1/models` on the running server is the authority.
const VLLM: &[&str] = &[
    "meta-llama/Llama-3.1-8B-Instruct",
    "meta-llama/Llama-3.3-70B-Instruct",
    "Qwen/Qwen2.5-Coder-32B-Instruct",
    "Qwen/Qwen2.5-Coder-7B-Instruct",
    "mistralai/Mistral-7B-Instruct-v0.3",
    "microsoft/Phi-3.5-mini-instruct",
];

/// Models commonly loaded in **LM Studio**. Same caveat as vLLM: the server
/// serves whatever the user loaded in the UI, and its `/v1/models` is the
/// authority.
const LM_STUDIO: &[&str] = &[
    "qwen2.5-coder-7b-instruct",
    "qwen2.5-coder-14b-instruct",
    "meta-llama-3.1-8b-instruct",
    "mistral-7b-instruct-v0.3",
    "phi-3.5-mini-instruct",
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
    ("vllm", VLLM),
    ("lmstudio", LM_STUDIO),
];

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    /// Provider ids must be aliases the daemon's `create_provider` can build,
    /// so a selected model round-trips back to a real provider.
    const KNOWN_PROVIDER_IDS: &[&str] = &[
        "vllm",
        "lmstudio",
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

    /// Walk up from this crate to the repository root, or `None` when the crate
    /// is vendored outside the monorepo.
    fn repo_root() -> Option<std::path::PathBuf> {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .find(|dir| dir.join("vscode-extension").is_dir() && dir.join("vibecli").is_dir())
            .map(std::path::Path::to_path_buf)
    }

    /// Every catalog provider must be selectable from the VS Code settings UI.
    ///
    /// `vibecli.provider` is a closed `enum` in the extension manifest, so a
    /// provider missing from it cannot be chosen at all — the daemon supports it
    /// and the user simply has no way to ask for it. `poolside` sat in exactly
    /// that state: shipped, keyed, documented, unselectable.
    ///
    /// Cross-language lists cannot share a constant, so this reads the manifest
    /// and fails when the two drift.
    #[test]
    fn vscode_settings_offer_every_catalog_provider() {
        let Some(root) = repo_root() else {
            return; // vendored outside the monorepo — nothing to check against
        };
        let manifest = root.join("vscode-extension/package.json");
        let Ok(text) = std::fs::read_to_string(&manifest) else {
            return;
        };
        let json: serde_json::Value =
            serde_json::from_str(&text).expect("vscode-extension/package.json is valid JSON");

        let offered: HashSet<&str> = json["contributes"]["configuration"]["properties"]
            ["vibecli.provider"]["enum"]
            .as_array()
            .expect("vibecli.provider declares an enum")
            .iter()
            .filter_map(serde_json::Value::as_str)
            .collect();

        let missing: Vec<&str> = PROVIDER_MODELS
            .iter()
            .map(|(id, _)| *id)
            .filter(|id| !offered.contains(id))
            .collect();

        assert!(
            missing.is_empty(),
            "these providers are in the catalog but absent from the \
             `vibecli.provider` enum in vscode-extension/package.json, so VS Code \
             users cannot select them: {missing:?}"
        );
    }

    /// Every catalog provider must be offered by the JetBrains settings combo.
    ///
    /// That box listed five providers while the daemon supported twenty, so
    /// most keys a user had configured could not be selected in the IDE. The
    /// Kotlin can't be compiled on every machine (it needs a JDK 17 toolchain),
    /// so this checks the source text for each id — enough to catch the
    /// omission that actually happens.
    #[test]
    fn jetbrains_settings_offer_every_catalog_provider() {
        let Some(root) = repo_root() else {
            return;
        };
        let settings = root.join(
            "jetbrains-plugin/src/main/kotlin/com/vibecody/vibecli/VibeCLISettingsConfigurable.kt",
        );
        let Ok(text) = std::fs::read_to_string(&settings) else {
            return;
        };

        let missing: Vec<&str> = PROVIDER_MODELS
            .iter()
            .map(|(id, _)| *id)
            .filter(|id| !text.contains(&format!("\"{id}\"")))
            .collect();

        assert!(
            missing.is_empty(),
            "these providers are in the catalog but absent from `PROVIDERS` in \
             VibeCLISettingsConfigurable.kt, so JetBrains users cannot select \
             them: {missing:?}"
        );
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
