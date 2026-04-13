# VibeCody — Agent Guidelines

This file instructs AI coding agents (Claude Code, Cursor, Windsurf, etc.) on conventions, storage patterns, and rules for working in this repository.

---

## Secure Settings Storage

VibeCody uses **two encrypted SQLite databases** for all sensitive settings. Never write API keys, tokens, or secrets to plaintext files.

### System Store — `~/.vibecli/profile_settings.db`

Encrypted with ChaCha20-Poly1305 (per-value random nonces). Key derived from machine identity (SHA-256 of HOME + USER). Accessible to both VibeCLI and VibeUI.

| Table | Contents |
|---|---|
| `profiles` | Named profiles (default: `"default"`) |
| `panel_settings` | UI panel settings per profile |
| `api_keys` | Provider API keys (anthropic, openai, gemini, grok, groq, openrouter, cerebras, ollama, etc.) |
| `provider_configs` | Provider settings — model, endpoint URL, etc. |
| `global_settings` | App-wide settings (theme, safety flags, etc.) |
| `master_keys` | Company encryption master keys |

**Rust API (vibecli_cli::profile_store::ProfileStore):**
```rust
let store = ProfileStore::new()?;
store.set_api_key("default", "anthropic", "sk-ant-...")?;
store.get_api_key("default", "anthropic")?;          // → Option<String>
store.set_provider_config("default", "openai", "model", "gpt-4o")?;
store.set_global("default", "ui.theme", "dark")?;
store.set_master_key(company_id, &key_bytes)?;
```

**Tauri commands (invoke from frontend):**
```ts
invoke("profile_api_key_set",         { profileId, provider, apiKey })
invoke("profile_api_key_get",         { profileId, provider })       // → string | null
invoke("profile_api_key_list",        { profileId })                 // → string[]
invoke("profile_api_key_delete",      { profileId, provider })
invoke("profile_provider_config_set", { profileId, provider, key, value })
invoke("profile_provider_config_get", { profileId, provider, key })
invoke("profile_provider_config_get_all", { profileId, provider })
invoke("profile_global_set",          { profileId, key, value })
invoke("profile_global_get",          { profileId, key })
invoke("profile_global_get_all",      { profileId })
invoke("profile_global_delete",       { profileId, key })
// Panel settings (unchanged API):
invoke("panel_settings_set",          { profileId, panel, key, value })
invoke("panel_settings_get",          { profileId, panel, key })
invoke("panel_settings_get_all",      { profileId, panel })
```

### Project Store — `<workspace>/.vibecli/workspace.db`

Encrypted with ChaCha20-Poly1305. Key derived from machine identity + workspace path, so secrets from project-A cannot be decrypted in project-B.

| Table | Contents |
|---|---|
| `workspace_settings` | Project-level settings (default provider, model, etc.) |
| `workspace_secrets` | Versioned project secrets (DB URLs, project API keys, `.env` values) |

**Rust API (vibecli_cli::workspace_store::WorkspaceStore):**
```rust
let store = WorkspaceStore::open(Path::new("/path/to/project"))?;
store.setting_set("provider", "claude")?;
store.setting_get("provider")?;                        // → Option<String>
store.secret_set("DATABASE_URL", "postgres://...", Some("agent-id"))?;
store.secret_get("DATABASE_URL")?;                     // → Option<String>
store.secret_list()?;                                  // → Vec<WorkspaceSecretMeta> (no values)
```

**Tauri commands:**
```ts
invoke("workspace_setting_get",    { workspacePath, key })
invoke("workspace_setting_set",    { workspacePath, key, value })
invoke("workspace_setting_delete", { workspacePath, key })
invoke("workspace_setting_list",   { workspacePath })
invoke("workspace_secret_get",     { workspacePath, keyName })
invoke("workspace_secret_set",     { workspacePath, keyName, value, createdBy? })
invoke("workspace_secret_delete",  { workspacePath, keyName })
invoke("workspace_secret_list",    { workspacePath })        // metadata only
```

---

## Rules for Agents

### DO

- Read and write API keys via `ProfileStore` or the `profile_api_key_*` Tauri commands.
- Read project secrets via `WorkspaceStore` or the `workspace_secret_*` Tauri commands.
- Store any new sensitive value (token, credential, secret) in the appropriate encrypted store.
- Check `workspace_settings` before falling back to global `profile_settings` for provider/model preferences.

### DO NOT

- Write API keys, tokens, or credentials to any plaintext file (`*.json`, `*.toml`, `*.env`).
- Read from or write to `~/.vibeui/api_keys.json` — this file has been deleted and migrated.
- Read from or write to `~/.vibeui/panel_settings.db` — this has been replaced by `profile_settings.db`.
- Store company master keys in `~/.vibecli/keys/*.key` files — use `ProfileStore.set_master_key()`.
- Hard-code API keys in source code, config files, or test fixtures.
- Commit any file containing real credentials.

---

## Storage Hierarchy

```
~/.vibecli/
├── profile_settings.db   ← encrypted: API keys, panel settings, global config, master keys
├── company.db            ← company orchestration data (unencrypted)
├── sessions.db           ← agent session history (unencrypted)
└── config.toml           ← CLI feature flags, provider enable/disable (no keys here)

<workspace>/
└── .vibecli/
    └── workspace.db      ← encrypted: project settings + project secrets
```

`config.toml` is for non-sensitive configuration only (enabling providers, setting model names, feature flags). API keys belong in `profile_settings.db`.

---

## Key Derivation & Security Model

- **Profile key**: `SHA-256("vibecli-profile-store-v1:" + $HOME + ":" + $USER)` — machine-bound
- **Workspace key**: `SHA-256("vibecli-workspace-store-v1:" + $HOME + ":" + $USER + ":" + workspace_path)` — machine + project bound
- **Company master keys**: encrypted inside `profile_settings.db` using the profile key. Secrets in `company.db` are then encrypted with those master keys (two-layer encryption).
- **Nonces**: 12-byte random nonce prepended to every ciphertext blob; each write generates a fresh nonce.

---

## Adding / Updating Providers and Models

### Frontend only (update model list or default)

Edit **one file**: `vibeui/src/hooks/useModelRegistry.ts`

| Goal | What to change |
|---|---|
| Add a new provider | Add model array to `STATIC_MODELS`; add default to `PROVIDER_DEFAULT_MODEL` |
| Add a model to an existing provider | Append to its array in `STATIC_MODELS` |
| Change a provider's default model | Update `PROVIDER_DEFAULT_MODEL[provider]` |

All UI panels consume `useModelRegistry()` — no other frontend file needs updating.

### Full backend provider (new Rust implementation)

Touch these files in order:

1. **`vibeui/crates/vibe-ai/src/providers/{name}.rs`** — implement the `AIProvider` trait.  
   For OpenAI-compatible APIs, copy `groq.rs` — it's the thinnest implementation.

2. **`vibeui/crates/vibe-ai/src/providers.rs`** — export the new module:
   ```rust
   pub mod {name};
   pub use {name}::MyProvider;
   ```

3. **`vibecli/vibecli-cli/src/config.rs`** — add a field to `Config`:
   ```rust
   pub {name}: Option<ProviderConfig>,
   ```

4. **`vibecli/vibecli-cli/src/main.rs`** — add a match arm in `create_raw_provider()`:
   ```rust
   "{name}" => Ok(Arc::new(MyProvider::new(config))),
   ```

5. **`vibecli/vibecli-cli/src/api_key_monitor.rs`** — three edits:
   - `build_provider()` — add match arm
   - `resolve_env_key()` — add `"{name}" => "PROVIDER_NAME_API_KEY"`
   - `configured_providers()` — add `"{name}"` to the names array

6. **`vibeui/src-tauri/src/commands.rs`** — add match arm in `build_temp_provider()` and map the API key field in `load_api_key_settings()` / `save_api_key_settings_to_store()`.

Then update `useModelRegistry.ts` as described above.

---

## Codebase Layout

```
vibecli/vibecli-cli/src/
├── profile_store.rs     ← system-level encrypted store
├── workspace_store.rs   ← project-level encrypted store
├── company_secrets.rs   ← company secret vault (uses profile_store for master keys)
└── config.rs            ← VibeCLI TOML config (non-sensitive)

vibeui/src-tauri/src/
├── panel_store.rs       ← thin re-export of ProfileStore
└── commands.rs          ← Tauri commands (profile_*, workspace_*, panel_settings_*)

vibeui/src/hooks/
└── useModelRegistry.ts  ← single source of truth for provider list + model lists

vibeui/src/constants/
└── ollamaModels.ts      ← Ollama static fallback model list
```

---

## Icons

All icons across VibeUI **must** be thin, themable SVGs. No emoji, Unicode symbols (▶ ▼ ◀ ×), or raster images as icons.

### Use the `<Icon>` component

```tsx
import { Icon } from "./Icon";

<Icon name="chevron-right" size={14} />
<Icon name="maximize" size={16} style={{ color: "var(--accent-color)" }} />
```

All available names are declared in the `IconName` union type in `vibeui/src/components/Icon.tsx`. TypeScript will error on unknown names — check that file before using a name.

### Rules

| Rule | Detail |
|---|---|
| **Thin strokes only** | Use `strokeWidth={1.5}` (Lucide default). Never fill-only icons. |
| **Themable via `currentColor`** | All icon paths must use `stroke="currentColor"` or `fill="currentColor"` — never hard-coded hex colors. Set color on the parent element or via the `style` prop. |
| **Size from prop** | Pass explicit `size` (px). Do not hard-code `width`/`height` attributes inside SVG definitions. |
| **No text symbols** | Never use `▶`, `▼`, `▲`, `◀`, `▸`, `▾`, `×`, `⊘`, `📌` or other Unicode/emoji as icons. Replace with the equivalent Lucide icon name. |
| **Add missing icons to Icon.tsx** | If the Lucide set lacks a needed icon, add a custom SVG path entry to the `ICONS` map in `Icon.tsx` following the existing pattern (24×24 viewBox, `strokeWidth` 1.5, `currentColor`). |

### Common replacements

| Symbol | Icon name |
|---|---|
| `▶` / `▸` | `chevron-right` or `play` |
| `▼` / `▾` | `chevron-down` |
| `▲` | `chevron-up` |
| `◀` | `chevron-left` |
| `×` (close) | `x` |
| `📌` | `pin` |
| `⊘` | `pin-off` |
| `+` (add) | `plus` |
| `≡` (menu) | `menu` |

---

## Testing

Use `ProfileStore::open_with(path, key)` and `WorkspaceStore::open_with(db_path, key)` for unit tests — these accept an explicit path and key, avoiding production DB and machine-specific key derivation.

```rust
let dir = std::env::temp_dir().join(format!("test_{}", rand::random::<u32>()));
std::fs::create_dir_all(&dir).unwrap();
let store = ProfileStore::open_with(&dir.join("test.db"), [42u8; 32]).unwrap();
```
