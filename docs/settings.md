---
layout: page
title: Settings
permalink: /settings/
---

# Settings

> Everything you can configure in VibeCody — and *where* your configuration is actually stored. Per AGENTS.md → Zero-Config First, every value lives in the encrypted ProfileStore (or your local `localStorage` for cosmetic preferences). No `.env` files. No plaintext credentials. Ever.

The Settings panel in VibeUI groups configuration into seven sections. Most users never need to touch any of them — VibeCody self-configures with safe defaults — but here's what each section does, where the values live, and how to change them from the terminal if you prefer.

---

## Where settings live

| Layer | Storage | Examples |
|---|---|---|
| **Provider keys (sensitive)** | `~/.vibecli/profile_settings.db` (encrypted, machine-bound) | API keys, OAuth tokens, HF_TOKEN, OpenMemory passphrase |
| **User preferences (cosmetic)** | `localStorage` (per-window) | Theme, density, sidebar state, recap toggles |
| **Daemon-level config** | `~/.vibecli/config.toml` | Network ports, telemetry, default model overrides |
| **Workspace secrets** | `<workspace>/.vibecli/workspace.db` (encrypted) | Per-project tokens, integration credentials |

> **No plaintext API keys, ever.** `~/.vibecli/api_keys.json` was deleted in the migration to ProfileStore. If you find one, it's a leftover — delete it.

---

## The seven sections

### 1. Profile
- **Display name** + **avatar style** (initials, color, icon).
- Saved to `localStorage` (cosmetic only — never sent to a server).

### 2. Appearance
- **Theme** (8+ themes via `themes.ts`) and **density** (compact / cozy / comfortable).
- Live-applies via the `applyThemeById` helper — no restart needed.
- Saved to `localStorage`.

### 3. OAuth Login
- Connect provider OAuth flows (Google, Microsoft, GitHub) for *user-account* sign-in, distinct from API keys.
- Tokens land in the encrypted ProfileStore via `cloud_oauth_save_client_config`.
- Use this when a provider gives you a personal token instead of an org-issued API key.

### 4. Customizations
- Custom system prompts, response formatting preferences, default UI layouts.
- Saved per-workspace in `<workspace>/.vibecli/workspace.db`.

### 5. API Keys
- The **security baseline** — every cloud provider needs a key here.
- Auto-saves to ProfileStore on each keystroke (1-second debounce).
- Each key has a **Test** button that probes the provider's auth endpoint and reports `OK (latency_ms) | Invalid API key | Network error`.
- Status badges per provider: ✓ OK · ⊝ Not set · ⚠ Error.
- Re-registers all cloud providers in the chat engine on save (no restart needed).

### 6. Integrations
- Email, calendar, Slack, Linear, Drive integrations.
- OAuth token storage same as section 3 (ProfileStore).

### 7. Sessions
- Recap toggles (on tab close, on idle, generator selection).
- Auto-resume last session.
- Saved as a single `vibeui-sessions` JSON blob in `localStorage`.

---

## Provider keys — managed from the terminal

The `vibecli` CLI exposes the same ProfileStore that the Settings panel writes to. Useful for headless deployments, CI, or when you're not running VibeUI:

```bash
# List configured providers (no values shown — names only)
vibecli list-keys

# Set or update a key
vibecli set-key anthropic        sk-ant-...
vibecli set-key openai           sk-...
vibecli set-key huggingface      hf_...               # for gated mistralrs models
vibecli set-key openmemory_passphrase 'my-pass'       # encrypts memory at rest

# Remove a key
vibecli unset-key openai

# Stdin form (avoids shell history)
printf '%s' "$ANTHROPIC_KEY" | vibecli set-key anthropic
```

Supported providers (the canonical list in `KEY_PROVIDERS`):
`anthropic` · `openai` · `gemini` · `grok` · `groq` · `openrouter` · `azure_openai` · `mistral` · `cerebras` · `deepseek` · `zhipu` · `vercel_ai` · `minimax` · `perplexity` · `together` · `fireworks` · `sambanova` · `ollama` · `huggingface` · `openmemory_passphrase`

---

## Readiness — `/health.providers`

The daemon's `/health` endpoint reports which providers are configured:

```bash
curl http://127.0.0.1:7878/health | jq '.providers'
```

```json
{
  "configured_count": 3,
  "names": ["anthropic", "huggingface", "openai"]
}
```

Any feature that depends on "an AI provider exists" inherits its readiness signal from this block — `features.diffcomplete.available`, `features.memory`, etc., all read `providers.configured_count > 0`. Names only, never values.

---

## Troubleshooting

### "Save failed: Permission denied"

The daemon couldn't write to `~/.vibecli/profile_settings.db`. Common causes:

- Running under a different UID than the one that owns the file.
- `~/.vibecli/` is on a read-only filesystem (Docker volume, network mount).

Fix: `chmod u+rw ~/.vibecli/profile_settings.db` and re-run.

### Test button shows "401 Unauthorized" but the key looks right

- Anthropic / OpenAI rotate trial keys aggressively — a key from last week may already be expired.
- Some providers (Anthropic, OpenAI) require **organization-level** activation before keys work — check the provider's dashboard.
- For Azure OpenAI, the `api_url` field must point to your specific deployment endpoint, not the generic `openai.azure.com`.

### Test button shows "Network error"

- Check your firewall — corporate networks often block `api.openai.com`, `api.anthropic.com`, etc.
- If you're behind a proxy, set `HTTPS_PROXY` and restart the daemon.

### Keys disappeared after upgrade

The pre-2026-04 `~/.vibeui/api_keys.json` was migrated to ProfileStore on first launch. If migration failed, the keys are still in that file — check, then re-enter.

### "Invalid model" after switching providers

The Settings panel re-registers cloud providers on save. If the chat engine doesn't pick up the new model, restart VibeUI — there's a one-time provider-cache miss after a fresh provider key.

---

## Observability

Every Settings change emits a structured `tracing` event under the `vibecody::settings` target:

```bash
RUST_LOG=vibecody::settings=info vibecli serve
```

Examples:

```
INFO vibecody::settings: settings.api_keys: persisted and re-registered cloud providers
  configured_count=3 active_providers=18

WARN vibecody::settings: settings.api_keys: persist failed (changes not saved)
  configured_count=2 error="Permission denied (os error 13)"
```

User content is never logged — only counts and stable enums (provider names). No telemetry leaves your machine without explicit opt-in.

---

## Cross-client scope

| Client | Settings access |
|---|---|
| **VibeUI (desktop)** | Full Settings panel: 7 sections |
| **VibeCLI** | `vibecli set-key` / `unset-key` / `list-keys` for the API Keys section; `~/.vibecli/config.toml` for daemon config |
| **VibeMobile** | Read-only — picks up settings from the daemon over HTTPS; no in-app editor by design |
| **VibeWatch** | None — too small for credential entry |
| **IDE plugins (VS Code / JetBrains / Neovim)** | Per-IDE: each surfaces its own minimal settings (active provider, daemon URL); credentials still live in the central ProfileStore |
| **Agent SDK** | Reads `vibecli set-key` values via the daemon's `/health.providers` block |

The daemon is the single source of truth — every client either reads from it (mobile, watch, plugins) or writes to it via the same encrypted ProfileStore (desktop, CLI). If two clients disagree, the one talking to the freshest daemon wins.

---

## Related

- **[Configuration Reference](../configuration/)** — `[memory]`, `[openmemory]`, and other `~/.vibecli/config.toml` blocks.
- **[Memory Guide](../memory-guide/)** — for `openmemory_passphrase` setup and encrypted memory.
- **[Diffcomplete (⌘.)](../diffcomplete/)** — what gets unlocked when you set your first provider key.
