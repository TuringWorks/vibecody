---
name: add-provider
description: How to add or update an AI provider or model in VibeCody — the one-file frontend edit for model lists and defaults, the 8-file backend dance for a new Rust provider implementation, and the client lists (VS Code, JetBrains, VibeAIChat) that make it selectable. Use when adding a provider, adding a model to an existing provider, or changing a provider's default model.
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

## Full backend provider (new Rust implementation) — touch 8 files in order

1. `vibecoder/crates/vibe-ai/src/providers/{name}.rs` — implement `AIProvider` trait (copy `groq.rs` for OpenAI-compat APIs)
2. `vibecoder/crates/vibe-ai/src/providers.rs` — `pub mod {name}; pub use {name}::MyProvider`
3. `vibecoder/crates/vibe-ai/src/catalog.rs` — add to `PROVIDER_MODELS` **and** to `KNOWN_PROVIDER_IDS` in the test module. This is what `/models` serves, so it is what every daemon-driven picker sees.
4. `vibecli/vibecli-cli/src/config.rs` — add `pub {name}: Option<ProviderConfig>` to `Config`
5. `vibecli/vibecli-cli/src/main.rs` — match arm in `create_raw_provider()`
6. `vibecli/vibecli-cli/src/main.rs` — add the id to `KEY_PROVIDERS`, or `vibecli set-key {name}` answers *unknown provider* and the encrypted key path is unreachable
7. `vibecli/vibecli-cli/src/api_key_monitor.rs` — match arm + env var in `resolve_env_key()` + name in `configured_providers()`
8. `vibecoder/src-tauri/src/commands.rs` — `build_temp_provider()` match arm + key field mapping

## Then every surface that enumerates providers

A provider absent from a closed list is not "unstyled" — it is **unselectable**, no matter that the daemon supports it. Three clients keep their own list:

| Surface | What to edit |
|---|---|
| VibeCoder Settings | `vibecoder/src/components/SettingsPanel.tsx` — `{name}_api_key` field + default + `renderSecretField(...)` row |
| VibeCoder model picker | `vibecoder/src/hooks/useModelRegistry.ts` — `STATIC_MODELS` + `PROVIDER_DEFAULT_MODEL` (see above) |
| VS Code | `vscode-extension/package.json` — the `vibecli.provider` `enum`. Pinned by `catalog::tests::vscode_settings_offer_every_catalog_provider`, which fails the build if you forget |
| JetBrains | `jetbrains-plugin/.../VibeCLISettingsConfigurable.kt` — the `PROVIDERS` array |
| VibeAIChat | `vibeaichat/src/App.tsx` — `PROVIDER_LABELS` (cosmetic; an unknown id falls back to the raw string) |

**No edit needed:** VibeDesk (reads `/models` from the daemon), Neovim (`provider` is a free-form string), VibeMobile and the watch clients (no provider selection — the daemon chooses).

## Then the docs — a provider nobody can find is a provider nobody uses

Nothing in the build fails when these are skipped, which is exactly why `poolside` and `vibecli-mistralrs` both shipped fully working and completely undocumented.

| Surface | What to edit |
|---|---|
| Provider page | `docs/providers/{name}.md` — front matter (`layout: page`, `title: "Provider: X"`, `permalink: /providers/{name}/`), how to get a key, all three config routes, model table with the default marked, verify command, troubleshooting. Copy `docs/providers/sambanova.md` for a cloud API or `docs/providers/vibecli-mistralrs.md` for a local one |
| Comparison table | `docs/providers/index.md` — a row (provider, type, env var, default model, free tier, streaming), **the provider count in the opening line**, a "Choosing a provider" line if it is the best pick for something, and a `Quick Examples` snippet |

Sidebar nav (`docs/_config.yml`) lists only a curated handful of providers, so a new page does not need an entry there.

**Write the docs from the implementation, not from the vendor's marketing page.** Four surfaces name the model ids and base URL — the provider module, `catalog.rs`, `useModelRegistry.ts`, and the `commands.rs` engine wiring — and they can disagree. `poolside` shipped `laguna-s-2.1` in the catalog and `poolside/laguna-s-2.1` in the other three; `config.rs` documented `malibu` at `api.poolside.ai`, which was never any of them. Both are fixed. **A catalog `name` is the string a client sends as the model, so it must be what the vendor's API accepts** — clients send it verbatim — while the published `id` only has to be unique. Reconcile before writing, and say so in the page when you cannot.

## Constraints that still apply

- API keys go in the encrypted `ProfileStore` — never a `*.toml`/`*.json` plaintext file. See the key storage rules in [CLAUDE.md](../../../CLAUDE.md).
- No panel may hard-code the new provider (or Anthropic) as its LLM backend — see **Provider-Agnostic Panels — STRICT** in [CLAUDE.md](../../../CLAUDE.md) and [AGENTS.md](../../../AGENTS.md#provider-agnostic-panels--strict).
- A new provider **does** have plugin impact: VS Code and JetBrains each hardcode a provider list, and a provider missing from either is unselectable there. Mobile and watch are genuinely unaffected — they have no provider setting.
