# VibeCody — Agent Guidelines

This file instructs AI coding agents (Claude Code, Cursor, Windsurf, etc.) on conventions, storage patterns, and rules for working in this repository.

---

## Product Matrix — know every surface before you change code

VibeCody is **not a single app**. It's a toolchain of ~13 clients that share one Rust daemon. Before editing anything that crosses a boundary (RPC, auth, pairing, settings, provider list, artifact name, OS floor), consult this table so you don't leave half the matrix broken.

| # | Product | Path | Stack | Purpose | Talks to |
|---|---------|------|-------|---------|----------|
| 1 | **VibeCLI** (daemon + TUI + REPL) | `vibecli/vibecli-cli/` | Rust, Axum, Ratatui | Terminal AI assistant; `--serve` daemon is the **source of truth** for every other client. ~354 modules. | Providers direct · serves `/mobile/*` · `/watch/*` · `/api/*` |
| 2 | **VibeUI** (desktop editor) | `vibeui/` | Tauri 2 + React/TS, Monaco | Full desktop code editor. **1,045+ Tauri commands**, ~293 panels + 42 composites. | Embeds VibeCLI crates · Tauri IPC to frontend |
| 3 | **VibeCLI App** (secondary Tauri shell) | `vibeapp/` | Tauri 2 + React/TS | Lightweight desktop chat shell. | Same Tauri commands as VibeUI (subset) |
| 4 | **VibeMobile** | `vibemobile/` | Flutter (Dart) | Phone / tablet / web companion. 11 screens, 6 services. | HTTPS/SSE to VibeCLI daemon `/mobile/*` + `/watch/*` relay |
| 5 | **VibeCodyWatch** (Apple Watch) | `vibewatch/VibeCodyWatch Watch App/` | SwiftUI, watchOS 10+ | Wrist client. Secure Enclave P-256 keys. | HTTPS/SSE `/watch/*` or WatchConnectivity relay |
| 6 | **VibeCodyWatchCompanion** (iOS) | `vibewatch/VibeCodyWatchCompanion/` | Swift, WatchConnectivity | Phone-side relay when watch is off-LAN. | Bridges watch ↔ VibeMobile ↔ daemon |
| 7 | **VibeCodyWear** (Wear OS) | `vibewatch/VibeCodyWear/` | Kotlin / Compose, Wear OS 3+ | Wrist client. Android Keystore / StrongBox P-256. | HTTPS/SSE `/watch/*` or Wearable Data Layer |
| 8 | **VibeCodyWearCompanion** (Android) | `vibewatch/VibeCodyWearCompanion/` | Kotlin, Wearable Data Layer | Phone-side relay when watch is off-LAN. | Bridges watch ↔ VibeMobile ↔ daemon |
| 9 | **VS Code extension** | `vscode-extension/` | TypeScript | Inline chat, code actions, sidebar. | HTTP to VibeCLI daemon |
| 10 | **JetBrains plugin** | `jetbrains-plugin/` | Kotlin, Gradle | IntelliJ / WebStorm / PyCharm integration. | HTTP to VibeCLI daemon |
| 11 | **Neovim plugin** | `neovim-plugin/` | Lua | Neovim + Telescope integration. | HTTP to VibeCLI daemon |
| 12 | **Agent SDK** | `packages/agent-sdk/` | TypeScript | Programmatic SDK for third-party integrations. | HTTP to VibeCLI daemon |
| 13 | **vibe-indexer** | `vibe-indexer/` | Rust | Standalone code-indexing service (semantic search, embeddings). | Standalone HTTP service |

**Shared crates** (`vibeui/crates/`): `vibe-core` (buffers/FS/Git), `vibe-ai` (22 providers), `vibe-lsp`, `vibe-extensions` (Wasmtime), `vibe-collab` (CRDT).

**Single source of truth**: the VibeCLI Rust daemon. If a client has drifted from the daemon's API, the client is wrong. Never fork protocol semantics into a client.

**Per-feature coverage** across VibeCLI / VibeUI / Mobile / Watch / plugins lives in:

- [`docs/FEATURE-MATRIX.md`](./docs/FEATURE-MATRIX.md) — at-a-glance ✅/⚙️/🔬/❌ per capability
- [`docs/FEATURE-REFERENCE.md`](./docs/FEATURE-REFERENCE.md) — detailed reference per feature
- [`docs/FIT-GAP-ANALYSIS.md`](./docs/FIT-GAP-ANALYSIS.md) — competitive catalogue (142 gaps tracked across iterations)

When you add a feature or close a gap, update whichever of those tables names the feature — otherwise the matrix drifts from reality.

---

## Change-Surface Cookbook — "when I change X, I also need to touch …"

Use this table as a pre-flight checklist. Cross-cutting changes that miss a surface create silent drift that only surfaces weeks later.

### Adding a new HTTP / RPC endpoint to the daemon

| Also touch | Why |
|------------|-----|
| `vibecli/vibecli-cli/src/serve.rs` (or `watch_bridge.rs` for `/watch/*`) | Route registration |
| `vibecli/vibecli-cli/tests/` | BDD harness for the endpoint |
| `vibeui/src-tauri/src/commands.rs` | Tauri wrapper if VibeUI/VibeApp need it |
| `vibeui/src-tauri/src/lib.rs` | Register the new command via `generate_handler!` |
| `vibemobile/lib/services/api_client.dart` | Flutter client method |
| `vibewatch/VibeCodyWatch Watch App/WatchNetworkManager.swift` | Swift client if wrist-relevant |
| `vibewatch/VibeCodyWear/app/src/main/kotlin/com/vibecody/wear/` | Kotlin client if wrist-relevant |
| `vscode-extension/src/api-client.ts` | VS Code if editor-relevant |
| `packages/agent-sdk/src/index.ts` | SDK method if public-facing |
| `docs/WATCH-INTEGRATION.md` / `docs/connectivity.md` / `docs/vibecli.md` | Docs for the new route |

### Adding a new Tauri command

`vibeui/src-tauri/src/commands.rs` (implementation) → `vibeui/src-tauri/src/lib.rs` (register in `tauri::generate_handler!`). VibeApp (`vibeapp/src-tauri/`) has its own `lib.rs` — register there too if the command is needed there. **Frontend consumers**: `vibeui/src/` panels call `invoke("your_command", …)` from TypeScript. No mobile/watch impact (mobile/watch don't speak Tauri IPC, only HTTP).

### Adding or updating an AI provider

Follow the 6-file dance in **"Adding / Updating Providers and Models"** below. **No changes needed** in VibeMobile, watch clients, plugins, or SDK — they use the provider through the CLI daemon's `/api/chat` route.

### Adding a new device-pairing / auth flow

| Also touch | Why |
|------------|-----|
| `vibecli/vibecli-cli/src/pairing.rs` | URL / bearer / QR generation |
| `vibecli/vibecli-cli/src/watch_auth.rs` | If wrist-specific (P-256 ECDSA flow) |
| `vibecli/vibecli-cli/src/serve.rs` + `watch_bridge.rs` | `/pair/*` routes |
| `vibemobile/lib/screens/pair_screen.dart` + `manual_connect_screen.dart` | Phone pairing UI |
| `vibewatch/VibeCodyWatch Watch App/` (PairingView.swift etc.) | Watch pairing UI |
| `vibewatch/VibeCodyWear/app/src/main/kotlin/…/pair/` | Wear pairing UI |
| `vibeui/src/panels/Governance/WatchDevices/` | Approval/revoke panel |
| `docs/WATCH-INTEGRATION.md` + `docs/vibemobile.md` + `docs/watchos.md` + `docs/wearos.md` | Doc sync |
| **Cryptography**: device keys MUST be **P-256 ECDSA (secp256r1)** — the only algorithm Apple Secure Enclave supports. Do not reintroduce Ed25519. |

### Adding a new setting / config key

1. Sensitive → `ProfileStore` (global) or `WorkspaceStore` (per-project). Never `config.toml`.
2. Non-sensitive → `vibecli/vibecli-cli/src/config.rs` (`Config` struct).
3. Surface it:
   - CLI: `vibecli config` subcommands.
   - VibeUI / VibeApp: `invoke("profile_global_set", …)` from a Settings panel.
   - Mobile: add a field to `vibemobile/lib/services/` settings; expose in `settings_screen.dart`.
   - Watch: most settings are *inherited* from the desktop; only add on-watch toggles when the watch needs to override (battery mode, relay prefer, …).
4. Document it in `docs/configuration.md`.

### Changing an OS / SDK floor

| Target | File(s) |
|--------|---------|
| iOS deployment target | `vibemobile/ios/Runner.xcodeproj/project.pbxproj` (3× `IPHONEOS_DEPLOYMENT_TARGET`), `vibemobile/ios/Flutter/AppFrameworkInfo.plist` (`MinimumOSVersion`), `vibemobile/ios/Podfile` (commented `platform :ios, 'X.Y'`), `docs/vibemobile.md` Platform-requirements table |
| watchOS deployment target | `vibewatch/project.yml` (`deploymentTarget.watchOS`), regenerate with `xcodegen`, `docs/watchos.md` |
| Wear OS / Android `compileSdk` / `targetSdk` / `minSdk` | `vibewatch/VibeCodyWear/app/build.gradle.kts`, `vibewatch/VibeCodyWear/gradle/libs.versions.toml` (`compileSdk` / `minSdk`), `docs/wearos.md` |
| macOS `minimumSystemVersion` | `vibeui/src-tauri/tauri.conf.json` and `vibeapp/src-tauri/tauri.conf.json` (`bundle.macOS.minimumSystemVersion`) |
| Linux runner pin | `.github/workflows/release.yml` (`ubuntu-22.04`, `ubuntu-22.04-arm`, `smoke-linux-next` uses `ubuntu-24.04`) |
| Xcode version | `.github/workflows/release.yml` — `maxim-lobanov/setup-xcode` `xcode-version` (currently `^26.0`, required for App Store submissions after **2026-04-28**) |

### Adding a new release artifact

| Also touch | Why |
|------------|-----|
| `.github/workflows/release.yml` | Add build job + include in `release.needs[]` |
| `Makefile` | Add `build-*` target so local reproduction works |
| `docs/release.md` | Download table entry |
| `docs/CHANGELOG.md` | Entry in `[Unreleased]` (or current version section) |
| Release-notes YAML body in `release.yml` | Platform matrix row |
| Root `README.md` "All Make Targets" section | Public-facing target list |

### Version bump

`Cargo.toml` (`[workspace.package].version`) → `vibeui/package.json` → `vibeapp/package.json` → `vibeui/src-tauri/tauri.conf.json` → `vibeapp/src-tauri/tauri.conf.json` → `vibemobile/pubspec.yaml` (`version:`) → `docs/release.md` + `docs/CHANGELOG.md` + `RELEASE.md`. Watch apps inherit version from their project files (`vibewatch/project.yml`, `vibewatch/VibeCodyWear/app/build.gradle.kts` `versionName`). Keep them in lockstep.

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

## Zero-Config First — the user-experience contract

VibeCody is shipped to users (developers, integrators, operators) who want to *use* it, not configure it. Every feature must work the moment the user reaches it. If a feature genuinely needs a value the daemon can't infer (an API key, a license token, a remote endpoint), that value belongs in the encrypted ProfileStore — never in env vars, never in plaintext files — and the requirement must be visible in the daemon's startup log, the relevant `/health` field, and `docs/`.

**The three rules.** Apply them when adding a feature, an integration, or a new third-party dependency:

1. **Default to working.** If a feature needs a setting, the daemon picks a safe default and logs why. Examples done right: `vibecli serve` self-generates a bearer token; mistralrs falls back to an Apache-2.0 ungated model when `HF_TOKEN` is missing; daemon binds `127.0.0.1:7878` unless `--host` overrides. Examples done wrong (and to be fixed when found): a feature that prints `set FOO_TOKEN to use this` and exits non-zero.

2. **Configuration goes through the encrypted ProfileStore.** Anything the user must supply — provider keys, OAuth tokens, integration secrets, license tokens, third-party `HF_TOKEN`-class values — is written via `ProfileStore::set_api_key()` (or the equivalent `set_provider_config`). The CLI surface for users is `vibecli set-key <provider> <value>` / `vibecli list-keys` / `vibecli unset-key <provider>`. Env vars are accepted only as a *fallback* read path for compatibility with existing toolchain expectations (`HF_TOKEN`, `OPENAI_API_KEY`); they are never the *only* way to set a value.

3. **Every config knob is documented and discoverable.** A user must be able to find out *what* they need to set without reading source. Three places matter: the daemon startup banner (warns when something is missing and tells the user how to set it), the `/health` JSON (machine-readable signal of which features are configured), and `docs/` (human-readable explanation of every key). If a knob exists but is documented in none of those, it doesn't really exist — fix that before the PR.

**When env-var-only is acceptable.** Build-time selection (`CARGO_FEATURES`, `RUSTFLAGS`) and developer-only knobs that change behavior during local debugging (`RUST_BACKTRACE`, `VIBE_INFER_KV_CACHE`, `VIBE_INFER_TURBOQUANT_BACKEND`) stay env-var-driven — they're not user-facing. The line is: **does a non-developer user ever need to set this?** If yes → ProfileStore. If no → env var is fine.

**Existing code that violates this.** When you find one, fix it on the way past. Recent examples already corrected: plaintext `api_key = "..."` lines stripped from `~/.vibecli/config.toml`; `~/.vibeui/api_keys.json` deleted and migrated. Recent example pending: HF_TOKEN currently has no ProfileStore path — it should be settable as `vibecli set-key huggingface hf_...` and read back at daemon startup.

---

## Rules for Agents

### DO

- Read and write API keys via `ProfileStore` or the `profile_api_key_*` Tauri commands.
- Read project secrets via `WorkspaceStore` or the `workspace_secret_*` Tauri commands.
- Store any new sensitive value (token, credential, secret) in the appropriate encrypted store.
- Check `workspace_settings` before falling back to global `profile_settings` for provider/model preferences.
- **Make every new feature work zero-config** — pick a sane default, log the trade-off, document the override.
- **Surface every required knob** in the daemon startup banner, `/health`, and `docs/`.
- **Explain non-trivial changes with an ASCII architecture diagram before writing code** (see [Explaining Changes](#explaining-changes--diagrams-before-prose) below).

### DO NOT

- Write API keys, tokens, or credentials to any plaintext file (`*.json`, `*.toml`, `*.env`).
- Read from or write to `~/.vibeui/api_keys.json` — this file has been deleted and migrated.
- Read from or write to `~/.vibeui/panel_settings.db` — this has been replaced by `profile_settings.db`.
- Store company master keys in `~/.vibecli/keys/*.key` files — use `ProfileStore.set_master_key()`.
- Hard-code API keys in source code, config files, or test fixtures.
- Commit any file containing real credentials.
- **Ship a feature that requires the user to `export FOO=...` before it works** — that value belongs in `ProfileStore` and must be settable via `vibecli set-key`. Env-var fallback is fine for compatibility; env-var-only is not.
- **Fail silently when a configured value is missing** — log it at startup, surface it in `/health`, document the fix.

---

## Explaining Changes — diagrams before prose

When a proposed change crosses file boundaries, introduces a new dispatch layer, or shifts how a request flows between modules, **lead the explanation with an ASCII architecture diagram, not a paragraph**. Diagrams make invariants visible at a glance that prose hides: who calls whom, where state lives, which boxes are new vs. existing, and what the happy-path trace looks like.

### When to draw one

- New or changed request flow (HTTP route, Tauri command, IPC message)
- New module that adds dispatch/routing (trait with multiple impls)
- Cross-process contracts (daemon ↔ client, sidecar integrations)
- Storage-layout changes (new DB, new cache dir, new file path)
- Any change to the Product Matrix or Change-Surface Cookbook above

### How to draw one

- ASCII box-drawing characters render cleanly in terminals and GitHub markdown
- Label boxes with **file path or module name** — readers should be able to grep the name
- Show flow direction explicitly (`→`, `▼`) at every hop
- Use a short legend when boxes differ in kind (in-process vs. external process, new vs. existing, sync vs. async)
- Follow the diagram with a concrete "request walk" — don't make reviewers simulate it mentally
- Two small, focused diagrams beat one that tries to show everything (e.g., "request flow" vs. "storage topology")

Prose alone is fine for single-file edits, bug fixes, or scoped refactors. The rule kicks in when a reviewer needs to understand **where something lives** or **how a request travels**, not just what line changed.

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

## Design System — mandatory for every panel, tab, and UI feature

VibeUI ships its own token-based design system at **`vibeui/design-system/`**. It is **not optional**. Every new panel, tab, dialog, modal, popover, or in-line widget that you add to `vibeui/src/components/` must consume tokens and CSS classes from there. Reviewers will reject a PR that introduces hard-coded colors, ad-hoc spacing, reinvented toast/empty/loading states, or `<div onClick>` where a `<button>` belongs.

### Read first

| File | Read when |
|---|---|
| [`vibeui/design-system/README.md`](./vibeui/design-system/README.md) | Always. Has the 10 rules every panel must follow + minimal panel template + color/spacing/font quick-pick. |
| `vibeui/design-system/tokens.css` | Looking up a CSS variable. |
| `vibeui/design-system/foundations/{color,typography,spacing,elevation,motion}.md` | Picking semantic colors / sizing / shadows. |
| `vibeui/design-system/components/{panel,button,input,card,badge-tag,progress,table,tabs}.md` | Implementing a primitive. Use the documented `panel-*` class — do not roll your own. |
| `vibeui/design-system/patterns/{data-states,forms}.md` | Loading/empty/error states or any form. |

### Hard rules

1. **Never write hex colors** (`#fff`, `#4caf50`, etc.) — always `var(--text-primary)`, `var(--success-color)`, etc. The only legal exception is icon assets that already carry `currentColor`.
2. **Never write a panel without the `panel-container` → `panel-header` → `panel-body` (→ `panel-footer`) skeleton.** The minimal template at the bottom of `vibeui/design-system/README.md` is the starting point.
3. **Use `panel-btn panel-btn-{primary|secondary|ghost|…}`** for buttons. Inline-style buttons get rejected.
4. **Loading/empty/error are `panel-loading` / `panel-empty` / `panel-error`** — do not invent your own status banner with `setStatusMsg + setTimeout`. Use `useToast()` from `src/hooks/useToast.ts` for transient feedback.
5. **Tabs use `panel-tab-bar` + `panel-tab`** with `role="tablist"` / `role="tab"` / `aria-selected`.
6. **Cards use `panel-card`.** Tags use `panel-tag panel-tag-{intent}`. Progress uses `progress-bar` + `progress-bar-fill` + `progress-bar-{color}`.
7. **Spacing is multiples of 4px** sourced from `--space-{1..8}`. Don't invent `padding: "13px 17px"`.
8. **Interactive elements are `<button>`/`<a>`** — never `<div onClick>`. Add `aria-label` when the button is icon-only.
9. **Persist UI state** (active tab, expanded panels, last-used inputs) via the `panel_settings_*` Tauri commands, not `localStorage`. Sensitive values (API keys, tokens) must use `profile_api_key_*` (see [Secure Settings Storage](#secure-settings-storage)).
10. **Run the existing visual smoke** before claiming a panel is done: open it in `npm run tauri:dev`, exercise the loading/empty/error paths, verify dark + light themes.

### Test discipline (red/green TDD + BDD)

Panels live or die on cross-component invariants — keyboard nav, error handling, focus management. Use the colocated `__tests__/` folder:

- **`*.test.tsx`** — focused unit tests against React Testing Library. Mock `@tauri-apps/api/core` `invoke`.
- **`*.bdd.test.tsx`** — scenario-style tests with a header comment listing the BDD scenarios (see `AgentPanel.bdd.test.tsx`, `BackgroundJobsPanel.bdd.test.tsx`).

The workflow when adding a panel feature:

1. **Red.** Add the failing scenario to `*.bdd.test.tsx` first (or create one). Run `npm test --prefix vibeui -- --run <PanelName>` and confirm it fails for the *expected* reason.
2. **Green.** Implement the smallest change that passes. Use the design-system classes — do not stub with inline styles "for now" intending to refactor later.
3. **Refactor.** Extract repeated markup into a shared component in `src/components/composite/` or a sub-component file.

Backend changes that span the daemon use the cucumber-style feature files in `vibecli/vibecli-cli/tests/features/*.feature` paired with a `*_bdd.rs` step file. Frontend-only changes stay in `*.bdd.test.tsx`.

### When you are unsure

Open an existing well-formed panel as your reference: `SettingsPanel.tsx`, `CostPanel.tsx`, `BackgroundJobsPanel.tsx`, `DiffReviewPanel.tsx`, `DesignHubPanel.tsx`, `DesignAnnotationsPanel.tsx`, or `DesignImportPanel.tsx` (the design-panel cluster was migrated to the design system in April 2026 — see `vibeui/src/components/__tests__/DesignHubPanel.bdd.test.tsx` for the BDD scenarios that lock in the contract). **Do not** copy from `DesignMode.tsx` — it still predates the design system and is being migrated.

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

vibeui/crates/vibe-infer/
├── src/lib.rs           ← Embedder + TextGenerator traits, StubBackend (default)
├── src/minilm.rs        ← candle BERT MiniLM-L6-v2 backend (feature: candle)
└── examples/embed.rs    ← end-to-end candle smoke-test
```

`vibe-infer` is the in-process inference layer (see also "Adding / Updating Providers and Models" — it complements the sidecar-based providers in `vibeui/crates/vibe-ai/`). Default builds pull no ML deps; opt in with `--features candle` (or `candle-metal` on macOS for GPU). `LocalEmbeddingEngine` in `vibecli/vibecli-cli/src/open_memory.rs` implements `vibe_infer::Embedder`, so async sites can take `&dyn vibe_infer::Embedder` and swap backends without touching OpenMemory.

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
