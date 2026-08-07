---
name: add-provider
description: How to add or update an AI provider or model in VibeCody — the one-file frontend edit for model lists and defaults, and the 6-file backend dance for a new Rust provider implementation. Use when adding a provider, adding a model to an existing provider, or changing a provider's default model.
---

# Adding / updating providers and models

## Frontend only (model list / default) — edit one file

> `vibecoder/src/hooks/useModelRegistry.ts`

| Goal | What to edit |
|---|---|
| Add a new provider | Add model array to `STATIC_MODELS` + default to `PROVIDER_DEFAULT_MODEL` |
| Add a model to existing provider | Append to the array in `STATIC_MODELS` |
| Change a provider's default model | Update `PROVIDER_DEFAULT_MODEL[provider]` |

All panels (Arena, MultiModel, BackgroundJobs, SuperBrain, Counsel, …) consume `useModelRegistry()` — no other frontend file needs changing.

## Full backend provider (new Rust implementation) — touch 6 files in order

1. `vibecoder/crates/vibe-ai/src/providers/{name}.rs` — implement `AIProvider` trait (copy `groq.rs` for OpenAI-compat APIs)
2. `vibecoder/crates/vibe-ai/src/providers.rs` — `pub mod {name}; pub use {name}::MyProvider`
3. `vibecli/vibecli-cli/src/config.rs` — add `pub {name}: Option<ProviderConfig>` to `Config`
4. `vibecli/vibecli-cli/src/main.rs` — match arm in `create_raw_provider()`
5. `vibecli/vibecli-cli/src/api_key_monitor.rs` — match arm + env var in `resolve_env_key()` + name in `configured_providers()`
6. `vibecoder/src-tauri/src/commands.rs` — `build_temp_provider()` match arm + key field mapping

Then add the frontend entry in `useModelRegistry.ts` as above.

## Constraints that still apply

- API keys go in the encrypted `ProfileStore` — never a `*.toml`/`*.json` plaintext file. See the key storage rules in [CLAUDE.md](../../../CLAUDE.md).
- No panel may hard-code the new provider (or Anthropic) as its LLM backend — see **Provider-Agnostic Panels — STRICT** in [CLAUDE.md](../../../CLAUDE.md) and [AGENTS.md](../../../AGENTS.md#provider-agnostic-panels--strict).
- A new provider has no mobile/watch/plugin impact.
