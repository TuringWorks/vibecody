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
//!   from [`PROVIDER_MODELS`].

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

// Every entry below is `Active` on Anthropic's own model-status table
// (platform.claude.com/docs/en/about-claude/model-deprecations, read
// 2026-09-02). Ordered newest-capability first.
//
// claude-fable-5-1 added 2026-09-02 — Active, retirement not sooner than
// 2027-09-01, the longest-lived entry on the table.
//
// claude-sonnet-4-5 dropped 2026-09-02: it is still Active, but its tentative
// retirement is "not sooner than September 29, 2026" — inside a release cycle
// from today. The rest of this list has a runway measured in months.
//
// claude-mythos-5 is deliberately still absent — it came back only for approved
// US organisations, and a flat list cannot express "available to some callers",
// so offering it would 403 for most users. It waits on per-model availability
// metadata rather than being listed optimistically.
//
// claude-haiku-4-5 is the shortest runway that remains (not sooner than
// 2026-10-15) and is kept only because it is the sole cheap tier Anthropic
// ships; re-check it before that date.
const CLAUDE: &[&str] = &[
    "claude-fable-5-1",
    "claude-opus-5",
    "claude-fable-5",
    "claude-sonnet-5",
    "claude-opus-4-8",
    "claude-opus-4-7",
    "claude-opus-4-6",
    "claude-sonnet-4-6",
    "claude-haiku-4-5",
];

// Swept 2026-09-02 against developers.openai.com/api/docs/models and
// .../deprecations.
//
// The three `*-pro` ids removed here were never model ids at all. "Pro" on the
// 5.6 family is a *request parameter* — `reasoning.mode: "pro"` on the
// Responses API — and OpenAI's own deprecation table spells the replacement for
// `gpt-5-pro-2025-10-06` as "gpt-5.6-sol (reasoning.mode: pro)". So
// `gpt-5.6-sol-pro` and its siblings were three picker entries that could only
// ever 404. (`gpt-5.5-pro` is real: the separate -pro id was retired *with* the
// 5.6 generation, not before it.)
//
// `gpt-5` removed: its only snapshot, gpt-5-2025-08-07, was deprecated
// 2026-06-11 with an API shutdown on 2026-12-11.
//
// gpt-4o / gpt-4o-mini / gpt-4.1 / gpt-4.1-mini are kept, at the end of the
// list. They are two generations behind everything above and OpenAI's model
// guidance routes all four to the 5.6 family, but the deprecation table still
// lists them **Active on the API** — leaving ChatGPT on 2026-02-13 retired them
// from the consumer product, not from here. This file's job is to drop ids that
// fail, not ids that are merely old, so "superseded" is not grounds for removal
// while the API still answers. They were briefly cut in the 2026-09-02 sweep and
// restored the same day.
//
// Their neighbours in that generation are a different matter and stay out:
// gpt-4.1-nano, gpt-4-turbo and gpt-3.5-turbo are deprecated with an API
// shutdown on 2026-10-23, and gpt-5's only snapshot (gpt-5-2025-08-07) shuts
// down 2026-12-11. Re-check the four below when OpenAI dates them.
const OPENAI: &[&str] = &[
    "gpt-5.6-sol",
    "gpt-5.6-terra",
    "gpt-5.6-luna",
    "gpt-5.5-pro",
    "gpt-5.5",
    "gpt-5.4",
    "gpt-5.4-mini",
    "gpt-5.3-codex",
    "gpt-5.3-chat",
    "gpt-4.1",
    "gpt-4.1-mini",
    "gpt-4o",
    "gpt-4o-mini",
];

// Swept 2026-09-02 against ai.google.dev/gemini-api/docs/models and
// .../deprecations.
//
// The Pro line has no GA id. `gemini-3.1-pro` and `gemini-3-pro` were both
// listed here and neither is a callable model code: Google ships the current
// Pro as `gemini-3.1-pro-preview`, and `gemini-3-pro-preview` is already in the
// deprecated/shut-down table. Same failure the gemini-3.5-pro entry caused on
// 2026-08-10 — a Pro id written the way the marketing name reads rather than
// the way the API spells it. The `-preview` suffix is load-bearing; keep it
// until Google GAs the model.
//
// gemini-2.5-pro removed: a newly created GCP project gets 404 "no longer
// available to new users", so for most callers it is retired in practice ahead
// of its published date. gemini-2.5-flash removed with it — the whole 2.5 line
// is scheduled to go no earlier than 2026-10-16.
//
// gemini-3.8-flash (2026-09-02) and gemini-3.7-flash (2026-08-13) added; the
// default stays on 3.6-flash until the newer two have a track record.
const GEMINI: &[&str] = &[
    "gemini-3.8-flash",
    "gemini-3.7-flash",
    "gemini-3.6-flash",
    "gemini-3.5-flash",
    "gemini-3.5-flash-lite",
    "gemini-3.1-flash-lite",
    "gemini-3.1-pro-preview",
];

// grok-4.6 (2026-08-12) is xAI's current flagship. Bare "grok-4.20" dropped
// 2026-09-02: docs.x.ai lists no such id — the 4.20 generation is addressed as
// grok-4.20-0309-reasoning / -non-reasoning, and 4.6/4.5/4.3 supersede it.
const GROK: &[&str] = &["grok-4.6", "grok-4.5", "grok-4.3"];

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

// deepseek-chat / deepseek-reasoner removed 2026-09-02: DeepSeek retired both
// legacy names on 2026-07-24, and its model-list endpoint now returns only the
// v4 pair. deepseek-chat was also this provider's default model, so every
// unconfigured DeepSeek call was aimed at a retired id.
const DEEPSEEK: &[&str] = &["deepseek-v4-pro", "deepseek-v4-flash"];

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

// Both previous entries were off serverless: Fireworks pulled its Llama models
// from serverless after 2026-05-14 (llama-v3p3-70b-instruct migrated to
// gpt-oss-120b) and mixtral-8x7b-instruct is two years stale. Fireworks serves
// a large rotating catalogue and `GET /v1/models` on the account is the
// authority — these two are a starting point, and the picker accepts a typed id.
const FIREWORKS: &[&str] = &[
    "accounts/fireworks/models/gpt-oss-120b",
    "accounts/fireworks/models/minimax-m3",
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

// Azure deployment names are chosen by the operator, so this is a hint list of
// what Foundry currently offers rather than a claim about any resource.
// gpt-4-turbo and gpt-4o both dropped 2026-09-02: gpt-4-turbo retired long ago,
// gpt-4o retires on Foundry 2026-10-01 (gpt-4o-mini went 2026-03-31).
const AZURE_OPENAI: &[&str] = &[
    "gpt-5.6-sol",
    "gpt-5.6-terra",
    "gpt-5.6-luna",
    "gpt-5.5",
    "gpt-5.4",
];

const BEDROCK: &[&str] = &[
    "anthropic.claude-opus-5",
    "anthropic.claude-sonnet-5",
    "anthropic.claude-opus-4-8",
    "anthropic.claude-haiku-4-5",
];

// GitHub Copilot brokers models from several vendors, but only the OpenAI ids
// are listed here: Copilot's own slugs for the Anthropic and Google models are
// not the vendors' ids (it spells them `claude-opus-4.1`-style), and this file
// ships no id it has not verified. `GET /models` on api.githubcopilot.com is
// the authority for a given account, and the picker accepts a typed id.
// gpt-4o removed 2026-09-02 — superseded, and it was the provider default.
const COPILOT: &[&str] = &[
    "gpt-5.6-sol",
    "gpt-5.6-terra",
    "gpt-5.6-luna",
    "gpt-5.5",
    "gpt-5.4",
    "gpt-5.4-mini",
    "gpt-5.3-codex",
];

const ZHIPU: &[&str] = &["glm-5.2", "glm-5.1", "glm-5", "glm-4.7", "glm-4.7-flash"];

const MINIMAX: &[&str] = &["MiniMax-M3", "MiniMax-M2.7"];

// Meta-Llama-3.3-70B-Instruct is not deprecated — SambaNova still lists it as
// its most battle-tested model — but it was the only entry, which made the
// picker look like a one-model provider. The rest are the other models
// SambaNova Cloud currently serves.
const SAMBANOVA: &[&str] = &[
    "DeepSeek-V3.2",
    "MiniMax-M3",
    "MiniMax-M2.7",
    "gpt-oss-120b",
    "gemma-4-31B-it",
    "Meta-Llama-3.3-70B-Instruct",
];

// Fully qualified, because `name` is the string a client sends as the model
// and Poolside's API rejects the bare form: `laguna-s-2.1` comes back as
// `{"error":"please check the model you provided"}` while
// `poolside/laguna-s-2.1` succeeds (measured against inference.poolside.ai).
//
// Clients do send `name` verbatim — `ProviderPill.tsx` calls
// `onSelect(pick.provider, pick.name)` — so a bare entry here is an id the
// picker hands over and the API refuses. `catalog_rows` does not double the
// namespace when a name already carries it; see the test below.
const POOLSIDE: &[&str] = &[
    "poolside/laguna-s-2.1",
    "poolside/laguna-xs-2.1",
    "poolside/laguna-m-1",
];

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

    /// `name` is the string a client sends as the model, so it has to be what
    /// the provider's API accepts.
    ///
    /// Clients do send it verbatim — `ProviderPill.tsx` calls
    /// `onSelect(pick.provider, pick.name)`. Poolside is the one provider whose
    /// API model parameter is itself namespaced, so a bare `laguna-s-2.1` here
    /// is an id the picker hands over and the API refuses with
    /// `{"error":"please check the model you provided"}`.
    #[test]
    fn poolside_names_are_the_ids_its_api_accepts() {
        assert!(
            POOLSIDE.contains(&"poolside/laguna-s-2.1"),
            "clients send `name` verbatim, so it must be the API's own id"
        );
        for name in POOLSIDE {
            assert!(
                name.starts_with("poolside/"),
                "`{name}` would be rejected bare"
            );
        }
    }

    /// Mirrors `serve.rs::catalog_rows`, which is the only thing that turns a
    /// catalog entry into a published id.
    fn composed_id(provider: &str, name: &str) -> String {
        match name.starts_with(&format!("{provider}/")) {
            true => name.to_string(),
            false => format!("{provider}/{name}"),
        }
    }

    /// The published id must not repeat the namespace.
    ///
    /// This is the trap: prefixing a catalog name without teaching the composer
    /// about it publishes `poolside/poolside/laguna-s-2.1`, which the API also
    /// rejects. Verified on the wire against a running daemon, after making
    /// exactly that mistake.
    #[test]
    fn a_published_id_never_doubles_the_provider_namespace() {
        assert_eq!(
            composed_id("poolside", "poolside/laguna-s-2.1"),
            "poolside/laguna-s-2.1"
        );
        // A name namespaced by someone *else* still gets the provider prefix —
        // `groq`'s `openai/gpt-oss-120b` is a Groq-hosted OpenAI model, and its
        // id is `groq/openai/gpt-oss-120b`.
        assert_eq!(
            composed_id("groq", "openai/gpt-oss-120b"),
            "groq/openai/gpt-oss-120b"
        );
        assert_eq!(
            composed_id("claude", "claude-opus-5"),
            "claude/claude-opus-5"
        );

        for (provider, names) in PROVIDER_MODELS {
            for name in *names {
                let id = composed_id(provider, name);
                assert!(
                    !id.starts_with(&format!("{provider}/{provider}/")),
                    "`{id}` repeats the `{provider}` namespace"
                );
            }
        }
    }

    /// The Rust and TypeScript cloud-model lists must hold the same tags.
    ///
    /// Both files carry a "keep in sync" comment and nothing enforced it. A tag
    /// added to one and not the other is a model the desktop offers and the
    /// daemon does not — or the reverse — and the only symptom is a picker
    /// entry that fails at request time.
    ///
    /// These are cloud tags, never reported by a local `/api/tags`, so there is
    /// no live source to reconcile against at runtime: the two static lists are
    /// the whole truth and have to agree.
    #[test]
    fn the_cloud_model_lists_agree_across_languages() {
        let Some(root) = repo_root() else {
            return; // vendored outside the monorepo — nothing to check against
        };
        let Ok(text) =
            std::fs::read_to_string(root.join("vibecoder/src/constants/ollamaModels.ts"))
        else {
            return;
        };
        let Some(start) = text.find("export const OLLAMA_CLOUD_MODELS") else {
            return;
        };
        let Some(len) = text[start..].find("\n];") else {
            return;
        };

        let ts: Vec<&str> = text[start..start + len]
            .lines()
            .filter_map(|line| line.trim().strip_prefix('"'))
            .filter_map(|rest| rest.split('"').next())
            .collect();

        assert!(
            !ts.is_empty(),
            "parsed no tags out of ollamaModels.ts — the shape changed and this \
             test silently stopped checking anything"
        );
        assert_eq!(
            ts,
            crate::providers::ollama::OLLAMA_CLOUD_MODELS.to_vec(),
            "ollamaModels.ts and providers/ollama.rs disagree about the cloud \
             models (order included, so the two files stay readable side by side)"
        );
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
