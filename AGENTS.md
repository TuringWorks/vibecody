# VibeCody — Agent Guidelines

This file instructs AI coding agents (Claude Code, Cursor, Windsurf, etc.) on conventions, storage patterns, and rules for working in this repository.

---

## Product Matrix — know every surface before you change code

VibeCody is **not a single app**. It's a toolchain of ~13 clients that share one Rust daemon. Before editing anything that crosses a boundary (RPC, auth, pairing, settings, provider list, artifact name, OS floor), consult this table so you don't leave half the matrix broken.

| # | Product | Path | Stack | Purpose | Talks to |
|---|---------|------|-------|---------|----------|
| 1 | **VibeCLI** (daemon + TUI + REPL) | `vibecli/vibecli-cli/` | Rust, Axum, Ratatui | Terminal AI assistant; `--serve` daemon is the **source of truth** for every other client. ~354 modules. | Providers direct · serves `/mobile/*` · `/watch/*` · `/api/*` |
| 2 | **VibeCoder** (desktop editor) | `vibecoder/` | Tauri 2 + React/TS, Monaco | Full desktop code editor. **1,045+ Tauri commands**, ~293 panels + 42 composites. | Embeds VibeCLI crates · Tauri IPC to frontend |
| 3 | **VibeCLI App** (secondary Tauri shell) | `vibeapp/` | Tauri 2 + React/TS | Lightweight desktop chat shell. | Same Tauri commands as VibeCoder (subset) |
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

**Shared crates** (`vibecoder/crates/`): `vibe-core` (buffers/FS/Git), `vibe-ai` (22 providers), `vibe-lsp`, `vibe-extensions` (Wasmtime), `vibe-collab` (CRDT).

**Single source of truth**: the VibeCLI Rust daemon. If a client has drifted from the daemon's API, the client is wrong. Never fork protocol semantics into a client.

**Per-feature coverage** across VibeCLI / VibeCoder / Mobile / Watch / plugins lives in:

- [`docs/FEATURE-MATRIX.md`](./docs/FEATURE-MATRIX.md) — at-a-glance ✅/⚙️/🔬/❌ per capability
- [`docs/FEATURE-REFERENCE.md`](./docs/FEATURE-REFERENCE.md) — detailed reference per feature

When you add a feature or close a gap, update whichever of those tables names the feature — otherwise the matrix drifts from reality.

---

## Change-Surface Cookbook — "when I change X, I also need to touch …"

Use this table as a pre-flight checklist. Cross-cutting changes that miss a surface create silent drift that only surfaces weeks later.

### Adding a new HTTP / RPC endpoint to the daemon

| Also touch | Why |
|------------|-----|
| `vibecli/vibecli-cli/src/serve.rs` (or `watch_bridge.rs` for `/watch/*`) | Route registration |
| `vibecli/vibecli-cli/tests/` | BDD harness for the endpoint |
| `vibecoder/src-tauri/src/commands.rs` | Tauri wrapper if VibeCoder/VibeApp need it |
| `vibecoder/src-tauri/src/lib.rs` | Register the new command via `generate_handler!` |
| `vibemobile/lib/services/api_client.dart` | Flutter client method |
| `vibewatch/VibeCodyWatch Watch App/WatchNetworkManager.swift` | Swift client if wrist-relevant |
| `vibewatch/VibeCodyWear/app/src/main/kotlin/com/vibecody/wear/` | Kotlin client if wrist-relevant |
| `vscode-extension/src/api-client.ts` | VS Code if editor-relevant |
| `packages/agent-sdk/src/index.ts` | SDK method if public-facing |
| `docs/WATCH-INTEGRATION.md` / `docs/connectivity.md` / `docs/vibecli.md` | Docs for the new route |

### Touching daemon startup, health, or discovery

**There is exactly one implementation of "reach the daemon":
`vibecli/vibecli-cli/src/daemon_bootstrap.rs`.** Every desktop client autostarts
the daemon on launch; before this module each had its own copy, and each was
broken differently. Do not add a fourth.

| Rule | Why |
|---|---|
| **Identity, not liveness.** Require `service == "vibecli"` from `GET /health`. | A bare TCP connect (or "any HTTP response counts") treats *whatever* holds the port as the daemon. Every panel then fails against a stranger, with an error blaming the daemon. |
| **Poll to a deadline; never sleep a guess.** | A measured cold start on a dev Mac took **~16 s** (memory-health warm-up + mDNS announce). The old path slept 2 s, checked once, and reported failure — on a daemon that was fine. |
| **Distinguish every failure.** `PortTakenByOther` / `BinaryNotFound` / `SpawnFailed` / `TimedOut` are separate states with separate messages. | "Is `vibecli` on your PATH?" is wrong advice for a port conflict, and sends the user down a dead end. |
| **Resolve the binary beyond `PATH`.** | A Finder-launched `.app` gets a minimal `PATH`, so `~/.cargo/bin` and Homebrew prefixes must be probed directly or autostart fails for every GUI launch. |
| **Reuse, never duplicate.** | The daemon is shared infra for the mobile / watch / IDE clients. |
| **Check "is this our child?" before "is the port taken?"** | The launch hook spawns the daemon, and the frontend's first poll lands while it is still binding. The wrong order reports our own starting daemon as a foreign process. |

| Also touch | Why |
|------------|-----|
| `vibecli/vibecli-cli/src/serve.rs` (`/health`) | The `service` field is the contract clients match on |
| `vibecli/vibecli-cli/tests/daemon_bootstrap_integration.rs` | Runs the real binary; fails if a short fixed wait comes back |
| `vibecoder/src-tauri/src/{lib.rs,commands.rs}` · `vibedesk/src-tauri/src/{lib.rs,commands.rs}` | Launch hook + `start_daemon` / `get_daemon_status` |
| `vibecoder/src/hooks/useDaemonMonitor.ts` | Frontend health poll — must verify `service` too |
| `vibemobile/lib/services/api_client.dart` (`isVibeCliHealthBody`) + `handoff_service.dart` | Mobile races transports and adopts the first URL that answers — a captive portal must not win |
| `vscode-extension/src/api-client.ts` · `packages/agent-sdk/src/index.ts` | Same identity check |
| `docs/api-reference.md` (`GET /health`) | The documented contract |

Remote clients (mobile, VS Code, SDK) accept a pre-`service` daemon via its exact
legacy body shape (`status: "ok"` **and** a `version`), because they talk to a
desktop that may not have been upgraded yet. A body naming a *different* service
is never accepted. The local autostart path requires the exact match — it
controls the daemon it spawns.

### Adding a new Tauri command

`vibecoder/src-tauri/src/commands.rs` (implementation) → `vibecoder/src-tauri/src/lib.rs` (register in `tauri::generate_handler!`). VibeApp (`vibeapp/src-tauri/`) has its own `lib.rs` — register there too if the command is needed there. **Frontend consumers**: `vibecoder/src/` panels call `invoke("your_command", …)` from TypeScript. No mobile/watch impact (mobile/watch don't speak Tauri IPC, only HTTP).

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
| `vibecoder/src/panels/Governance/WatchDevices/` | Approval/revoke panel |
| `docs/WATCH-INTEGRATION.md` + `docs/vibemobile.md` + `docs/watchos.md` + `docs/wearos.md` | Doc sync |
| **Cryptography**: device keys MUST be **P-256 ECDSA (secp256r1)** — the only algorithm Apple Secure Enclave supports. Do not reintroduce Ed25519. |

### Adding a new setting / config key

1. Sensitive → `ProfileStore` (global) or `WorkspaceStore` (per-project). Never `config.toml`.
2. Non-sensitive → `vibecli/vibecli-cli/src/config.rs` (`Config` struct).
3. Surface it:
   - CLI: `vibecli config` subcommands.
   - VibeCoder / VibeApp: `invoke("profile_global_set", …)` from a Settings panel.
   - Mobile: add a field to `vibemobile/lib/services/` settings; expose in `settings_screen.dart`.
   - Watch: most settings are *inherited* from the desktop; only add on-watch toggles when the watch needs to override (battery mode, relay prefer, …).
4. Document it in `docs/configuration.md`.

### Changing an OS / SDK floor

| Target | File(s) |
|--------|---------|
| iOS deployment target | `vibemobile/ios/Runner.xcodeproj/project.pbxproj` (3× `IPHONEOS_DEPLOYMENT_TARGET`), `vibemobile/ios/Flutter/AppFrameworkInfo.plist` (`MinimumOSVersion`), `vibemobile/ios/Podfile` (commented `platform :ios, 'X.Y'`), `docs/vibemobile.md` Platform-requirements table |
| watchOS deployment target | `vibewatch/project.yml` (`deploymentTarget.watchOS`), regenerate with `xcodegen`, `docs/watchos.md` |
| Wear OS / Android `compileSdk` / `targetSdk` / `minSdk` | `vibewatch/VibeCodyWear/app/build.gradle.kts`, `vibewatch/VibeCodyWear/gradle/libs.versions.toml` (`compileSdk` / `minSdk`), `docs/wearos.md` |
| macOS `minimumSystemVersion` | `vibecoder/src-tauri/tauri.conf.json` and `vibeapp/src-tauri/tauri.conf.json` (`bundle.macOS.minimumSystemVersion`) |
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

`Cargo.toml` (`[workspace.package].version`) → `vibecoder/package.json` → `vibeapp/package.json` → `vibecoder/src-tauri/tauri.conf.json` → `vibeapp/src-tauri/tauri.conf.json` → `vibemobile/pubspec.yaml` (`version:`) → `docs/release.md` + `docs/CHANGELOG.md` + `RELEASE.md`. Watch apps inherit version from their project files (`vibewatch/project.yml`, `vibewatch/VibeCodyWear/app/build.gradle.kts` `versionName`). Keep them in lockstep.

---

## Secure Settings Storage

VibeCody uses **two encrypted SQLite databases** for all sensitive settings. Never write API keys, tokens, or secrets to plaintext files.

### System Store — `~/.vibecli/profile_settings.db`

Encrypted with ChaCha20-Poly1305 (per-value random nonces). Key derived from machine identity (SHA-256 of HOME + USER). Accessible to both VibeCLI and VibeCoder.

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

**Existing code that violates this.** When you find one, fix it on the way past. Recent examples already corrected: plaintext `api_key = "..."` lines stripped from `~/.vibecli/config.toml`; `~/.vibecoder/api_keys.json` deleted and migrated. Recent example pending: HF_TOKEN currently has no ProfileStore path — it should be settable as `vibecli set-key huggingface hf_...` and read back at daemon startup.

---

## Rules for Agents

### DO

- Read and write API keys via `ProfileStore` or the `profile_api_key_*` Tauri commands.
- Read project secrets via `WorkspaceStore` or the `workspace_secret_*` Tauri commands.
- Store any new sensitive value (token, credential, secret) in the appropriate encrypted store.
- Check `workspace_settings` before falling back to global `profile_settings` for provider/model preferences.
- **Make every new feature work zero-config** — pick a sane default, log the trade-off, document the override.
- **Surface every required knob** in the daemon startup banner, `/health`, and `docs/`.
- **Honour the toolbar model dropdown in every panel that calls an LLM** — see [Provider-Agnostic Panels](#provider-agnostic-panels--strict) below.
- **Explain non-trivial changes with an ASCII architecture diagram before writing code** (see [Explaining Changes](#explaining-changes--diagrams-before-prose) below).
- **Write in a functional style and refactor toward it** — pure functions, immutable bindings, iterator/combinator chains, and total error handling. See [Functional Style & Safe Refactoring](#functional-style--safe-refactoring--rust--typescript) below.
- **Use the target language's own idiom** rather than importing a pattern's name — `enum`+`match`, `sealed class`+`when`, discriminated union+`switch`. See [the idiom table](#reach-for-the-languages-idiom-not-the-patterns-name).
- **Measure before optimising, and attribute from the call tree** — then re-measure like-for-like and confirm the feature still works on screen. See [Performance](#performance--measure-attribute-verify).
- **Run it and look at it** before calling a change done — open the panel, hit the route, read stderr. See [Verification](#verification--a-green-build-proves-nothing).
- **Say what you verified and what you didn't**, including which client surfaces you exercised.

### DO NOT

- Write API keys, tokens, or credentials to any plaintext file (`*.json`, `*.toml`, `*.env`).
- Read from or write to `~/.vibecoder/api_keys.json` — this file has been deleted and migrated.
- Read from or write to `~/.vibecoder/panel_settings.db` — this has been replaced by `profile_settings.db`.
- Store company master keys in `~/.vibecli/keys/*.key` files — use `ProfileStore.set_master_key()`.
- Hard-code API keys in source code, config files, or test fixtures.
- Commit any file containing real credentials.
- **Ship a feature that requires the user to `export FOO=...` before it works** — that value belongs in `ProfileStore` and must be settable via `vibecli set-key`. Env-var fallback is fine for compatibility; env-var-only is not.
- **Fail silently when a configured value is missing** — log it at startup, surface it in `/health`, document the fix.
- **Hard-code Anthropic (or any single provider) as the LLM backend in a panel.** Every panel that talks to an LLM must route through the toolbar's selected provider/model — see [Provider-Agnostic Panels](#provider-agnostic-panels--strict) below.
- **`.unwrap()` / `.expect()` / `panic!` on a value that can legitimately be absent or fail** in daemon, library, or command code — return `Result`/`Option` and propagate with `?`. Panics are for tests and provably-infallible invariants (with a comment saying why). See [Functional Style & Safe Refactoring](#functional-style--safe-refactoring--rust--typescript).
- **Reach for `let mut` + an index loop when an iterator chain says it more clearly**, or `.clone()` to dodge the borrow checker in a hot path when a borrow, `Arc`, or `Cow` would do. See [Functional Style & Safe Refactoring](#functional-style--safe-refactoring--rust--typescript).
- **Substitute a plausible default for missing data** — `unwrap_or_else(Utc::now)` on a timestamp, a clamp bound rendered as if it were a value. Absent stays absent; a bound that binds says so. See [Modelling Honesty](#modelling-honesty--a-model-that-cannot-be-wrong-is-not-a-model).
- **Claim a change works because `cargo check` and `tsc` are clean.** They are the floor. See [Verification](#verification--a-green-build-proves-nothing).
- **Optimise something you have not measured as a problem**, or report a number without saying whether the feature still works.
- **Ship a UI affordance with no code behind it**, or add a route to `serve.rs` without walking the [Change-Surface Cookbook](#change-surface-cookbook--when-i-change-x-i-also-need-to-touch). See [Traps a Build Cannot Catch](#traps-a-build-cannot-catch).
- **Use `INSERT OR REPLACE` for a partial-column refresh** — it NULLs every column you didn't name. Use `ON CONFLICT … DO UPDATE SET`.

---

## Functional Style & Safe Refactoring — Rust & TypeScript

VibeCody is a large, long-lived daemon with 13 clients. Code that is **pure, immutable, and total** is easier to test, parallelize, and reason about across that surface. Write new code this way, and when you touch existing code, leave it a little more functional than you found it — as long as the refactor is behaviour-preserving and covered by tests.

**Guiding principle:** separate *computation* (pure, deterministic, easy to test) from *effects* (IO, DB, network, mutation). Push effects to the edges; keep the core a set of pure functions over immutable data. A function that both computes a result and writes to the DB is two functions wearing a trenchcoat.

### The four rules, in priority order

1. **Pure by default.** Output depends only on input. Push IO, clocks, RNG, and mutation to the edges; keep the core a tree of pure transforms.
2. **Total functions.** Handle every case. Model absence and failure in the type — `Result`/`Option` in Rust, discriminated unions in TS, `sealed class` in Kotlin, `enum` with associated values in Swift.
3. **Illegal states unrepresentable.** Encode invariants in types so the compiler rejects bad states before runtime.
4. **Immutability by default.** Reserve mutation for a locally-owned accumulator inside an otherwise-pure function.

The payoff is testability: a pure core can be tested without a process, a socket, or a clock — which is why the hardest logic belongs there. Put the whole grammar / rule / decision in a pure function returning a typed result; keep transport, retries, and persistence outside it. In this repo that means: **protocol semantics belong in a pure function; `serve.rs` / `commands.rs` / `api_client.dart` are transport**.

**Where to stop being pure:** measured hot paths. A render loop, a tokenizer, or an indexing kernel may use indexed loops and reused buffers — with a comment saying a profiler said so, and a link to the measurement. *Functional by default; imperative where measured.*

### Reach for the language's idiom, not the pattern's name

Most GoF patterns already exist as a language feature in every stack this repo ships. Using the feature is shorter, faster, and reads as native to whoever maintains that client.

| Intent | Rust | TypeScript / React | Swift (watchOS) | Kotlin (Wear / JetBrains) | Dart (Flutter) |
|---|---|---|---|---|---|
| Sum type / visitor | `enum` + exhaustive `match` | discriminated union + `switch` with `never` default | `enum` w/ associated values + `switch` | `sealed class` + `when` (exhaustive) | `sealed class` + `switch` pattern-match |
| Strategy | trait object, or `fn` value | function as a prop/param | protocol, or closure property | function type, or `fun interface` | function type / callback |
| Scope guard / RAII | `Drop` | `try…finally`, cleanup fn from `useEffect` | `defer` | `use { }` | `try…finally` |
| Builder | typestate builder | options object | result builder / `init` w/ defaults | `apply { }` / named args w/ defaults | named + default constructor args |
| Iterator pipeline | `Iterator` adapters | `map`/`filter`/`reduce`/`flatMap` | `map`/`filter`/`reduce`, `lazy` | sequences, `map`/`filter`/`fold` | `Iterable` methods, `expand` |
| Newtype | `struct DeviceId(String)` | branded type | `struct Id: RawRepresentable` | `@JvmInline value class` | `extension type` |
| Absence / failure | `Option` / `Result` + `?` | `T \| undefined`, `Result`-shaped union | `Optional` + `guard let`, `throws` | nullable `T?` + `?.`, `Result<T>` | `T?` + `??`, `Future`/`Either` |
| Observation | channel / `watch` | `useSyncExternalStore`, hooks | `@Observable` / Combine | `StateFlow` | `Stream` / `ValueNotifier` |

**Refactor toward a pattern when the smell is there** — a growing `switch` on a type tag, a boolean parameter selecting behaviour, construction logic sprawling across call sites. **Never because the pattern is admired.** A `Factory` that produces one type, or a `Strategy` with a single implementation, is indirection with no payer.

### Rust

- **Prefer iterator combinators over manual loops.** `map` / `filter` / `filter_map` / `fold` / `try_fold` / `collect` / `partition` express intent and eliminate off-by-one and index-out-of-bounds classes entirely. Reach for a `for` loop only when the body has early-exit side effects that read worse as a chain.
- **Total error handling — no panics in library/daemon/command paths.** Return `Result<_, _>` / `Option<_>` and propagate with `?`. Replace `match`-pyramids with `?`, `map_err`, `and_then`, `ok_or_else`, `unwrap_or_else`, `unwrap_or_default`. `.unwrap()`/`.expect()` are allowed in tests and for invariants that cannot fail (leave a one-line comment proving it).
- **Immutable by default.** Start every binding as `let`; add `mut` only when the compiler forces you to. Prefer building a new value (`Vec::from_iter`, struct update syntax `..old`) over mutating in place.
- **Borrow, don't clone.** Take `&str` / `&[T]` / `&T` (or `impl AsRef<str>`, `Cow<'_, str>`) in function signatures instead of owned `String` / `Vec<T>`. Share with `Arc<T>` rather than deep-cloning in hot or fan-out paths. Every `.clone()` in a loop is a refactor candidate — audit whether a borrow, `Arc::clone` (cheap), or `Cow` removes it.
- **Return `impl Iterator` instead of allocating a `Vec`** when the caller just iterates. Avoid `.collect::<Vec<_>>()` followed immediately by another iteration.
- **Make illegal states unrepresentable.** Prefer enums over `bool` flags and stringly-typed status; newtype wrappers (`struct DeviceId(String)`) over bare `String`; `NonZeroU32` / `NonEmpty` where the domain forbids the empty/zero case. Exhaustive `match` (no catch-all `_` on domain enums) so new variants become compile errors.
- **Use the map APIs that avoid double lookups:** `entry(k).or_insert_with(..)`, `HashMap::get_or_insert`, `.retain(..)`. Replace an O(n²) nested-loop membership test with a `HashSet`/`HashMap` built once.
- **Parallelize independent CPU-bound work with `rayon` `par_iter`** where the crate already depends on it and the work is side-effect-free. Never block the async runtime — wrap blocking IO/CPU in `tokio::task::spawn_blocking`.
- **Use `itertools`** (already a workspace dep) for `chunks`, `group_by`, `unique`, `partition_map`, `try_collect` rather than hand-rolling.

### TypeScript / React

- **`const` over `let`; never mutate props or state.** Build new values with spread / `map` / `filter` / `structuredClone`. Treat arrays and objects flowing through the UI as `readonly`.
- **Combinators over loops.** `map` / `filter` / `reduce` / `flatMap` / `Object.entries().map()` instead of `for`/`push`. Early returns over deeply nested `if`.
- **Pure render, derived state.** Derive values during render (memoized with `useMemo`/`useCallback` when the computation is expensive) rather than storing a duplicate in `useState` and syncing it in an effect. Effects are for *effects* (subscriptions, IO), not for deriving state.
- **Total types.** No `any` — use `unknown` + narrowing. Model variants as discriminated unions and switch exhaustively with a `never` default so a new variant is a compile error. Use optional chaining `?.` and nullish coalescing `??` over manual `&&` guards.
- **Small composable functions** with explicit return types on exported functions. Keep Tauri `invoke` calls (the effects) at the edge; keep transforms pure and unit-testable.

### Swift / Kotlin / Dart clients

The watch and mobile clients are thin, which makes it cheap to keep them pure — and expensive not to, because a bug there is invisible from the daemon's tests.

- **Swift (`vibewatch/VibeCodyWatch*`).** `let` over `var`; `struct` over `class` unless identity or reference semantics are required. Model responses as `Codable` value types and decode once at the boundary — never pass `[String: Any]` inward. `guard let … else { return }` over pyramids of `if let`. `enum` with associated values for state (`case idle / loading / loaded(Session) / failed(Error)`) instead of parallel `isLoading` + `error` + `data` properties. Keep `WatchNetworkManager` as the only place that knows about URLs.
- **Kotlin (`vibewatch/VibeCodyWear*`, `jetbrains-plugin/`).** `val` over `var`; `data class` for payloads; `sealed interface` + exhaustive `when` for UI state. Prefer `map`/`filter`/`fold` on sequences over index loops. Use `Result`/`runCatching` at the transport edge, never a bare `try { } catch { }` that swallows. Suspend functions for IO; no blocking calls on the main dispatcher.
- **Dart (`vibemobile/`).** `final` by default; `const` constructors for widgets so Flutter can skip rebuilds. Immutable model classes with `copyWith`. `sealed class` + exhaustive `switch` for screen state. Keep `api_client.dart` the only file that constructs URLs or parses JSON; screens receive typed models.

Whichever client you touch, the decoding boundary is the place to be total: **parse into a typed value once, at the edge, and let everything inward be non-optional.**

### Refactor triggers (safe, high-value — do these when you see them)

| Smell | Refactor | Wins |
|---|---|---|
| `.unwrap()`/`.expect()` off a fallible value in non-test code | `?` + `Result`, or `unwrap_or_else`/`ok_or_else` | safety (no panic) |
| `.clone()` inside a loop / per-request | borrow, `Arc::clone`, or `Cow` | speed (fewer allocs) |
| Nested loop doing membership/lookup | build a `HashSet`/`HashMap` once | speed (O(n²)→O(n)) |
| `let mut v = Vec::new(); for … { v.push(…) }` | `iter().map(…).collect()` / `filter_map` | clarity + safety |
| `match` pyramid on `Result`/`Option` | `?`, `map_err`, `and_then` | clarity |
| `bool` flag pair encoding a state | enum with exhaustive `match` | safety |
| Blocking IO/CPU on the async runtime | `spawn_blocking` / `rayon` | responsiveness |
| String built by `+=` in a loop | `push_str` into one buffer, or `join` / `format!` | speed |
| `useState` mirror kept in sync via `useEffect` | derive with `useMemo` | fewer renders, no drift |
| `any` on a boundary type | `unknown` + a narrowing type guard | safety |
| `Vec::new()` then `push` a known number of times | `Vec::with_capacity(n)` / `collect` | one alloc instead of log₂n |
| Container constructed inside a loop | hoist it, or keep a scratch buffer on the struct | fewer allocs |
| Owned `String` / `Vec<T>` parameter that's only read | `&str` / `&[T]` / `impl AsRef<str>` | no copy at every call site |
| `HashMap<A, HashMap<B, V>>` | one map keyed by `(A, B)` | one lookup, one alloc |
| Regex / JSON parse before a cheap reject | first-byte or length test first | skips the expensive path |
| Cache that only *ignores* stale entries | evict on insert, or rebuild every N ops | bounded memory |
| Poll interval faster than the data changes | match cadence to the source | the biggest win on this list |

### Allocation discipline

Apply this **after** cadence and structure — see [Performance](#performance--measure-attribute-verify).

**Do not allocate.** Reuse instead of recreating; hoist containers out of loops; `with_capacity`/`reserve` when the size is known; `static`/`const` tables over lazily-built ones; give every cache an eviction path.

**Do not copy.** View types at boundaries (`&str`, `&[T]`, `Cow<'_, str>`). Borrow, don't clone — clone when you need *ownership*, not to quiet the borrow checker. Move for transfers; store indices instead of duplicating elements.

**Lay it out for the cache.** Flat over pointer-chasing (`Vec`/`HashMap` beats a node-per-entry tree). Order struct fields to cut padding and group hot fields. `u32` indices instead of 64-bit pointers where the domain fits. Flatten nested maps into one compound key. Arrays or bit vectors when the key domain is small and enumerable.

**Do less work.** Fast path first, rare handling out-of-line. Precompute at construction what would be recomputed per call. Hoist loop invariants. Cheap reject before an expensive check. Batch — bulk APIs amortise per-call overhead and often unlock a better algorithm. Sample statistics (one event in 32, power-of-two mask) so instrumentation doesn't dominate what it measures.

> Every item in this subsection is worth less than one correctly-sized poll interval. Reach for it **after** the performance loop below, not instead of it.

### Discipline for refactors

- **Behaviour-preserving only.** A speed/safety refactor must not change observable output. If a test doesn't already pin the behaviour, add one *first* (red/green — see [Test discipline](#test-discipline-redgreen-tdd--bdd)), then refactor under it.
- **Let the hooks gate you.** Every `.rs` edit runs `cargo check --workspace --exclude vibe-collab`; every `.ts`/`.tsx` edit runs `tsc --noEmit` (see CLAUDE.md → Claude Code Setup). A refactor isn't done until both are clean.
- **One concern per commit.** Don't fold a broad style sweep into a feature change — it makes review and `git bisect` painful. Mechanical FP refactors go in their own commit.
- **Don't refactor what you can't test or measure.** For a "this is faster" claim on a hot path, prefer a `criterion` bench or a before/after measurement over intuition. Micro-optimizing cold code adds risk for no user-visible win.

---

## Performance — measure, attribute, verify

**A green build proves almost nothing, and a good number proves less.** Every performance change in this repo follows the same five steps, in order:

```
1. MEASURE      a number, not a hunch. Is this CPU, memory, or latency?
2. ATTRIBUTE    profile. Read the CALL TREE, not the leaf histogram.
3. FIX          the cause you attributed, not the symptom you noticed.
4. RE-MEASURE   same protocol, like-for-like.
5. VERIFY       the feature still works — on screen, not in a metric.
```

### Step 1 — measure before you form a theory

The first number should reframe the problem, not confirm your plan.

```bash
ps -o pid=,rss=,%cpu= -p $(pgrep -x vibecli)      # daemon
ps -o pid=,rss=,%cpu= -p $(pgrep -x VibeCoder)    # desktop app
```

**Idle CPU is the cheapest health check we have, and nothing in CI watches it.** A daemon with no active session and an editor with no open job should both cost ~0 %. Check by hand after touching anything on a per-tick path — timers, `tokio::interval`, mDNS announce, the API-key monitor, watch-bridge polling, hosted-loop scheduling, React subscriptions. A regression there is invisible to every test we have.

### Step 2 — attribute, and read the call tree

| Surface | Symptom | Attribution |
|---|---|---|
| Rust daemon | `ps`, `tokio-console` | `cargo flamegraph`, `dhat` |
| macOS (daemon or Tauri shell) | `ps` | `sample <pid> 10`, `vmmap --summary`, `heap <pid>` |
| VibeCoder frontend | browser Task Manager | DevTools Performance → call tree; Memory → heap snapshot |
| Flutter | `flutter run --profile` | DevTools timeline |

A flat "hottest functions" list is a list of **symptoms** — it tells you what is expensive, never *who asked for it*. Always walk down from the entry point. For memory, get the **shape** first (`vmmap --summary`: is this our objects, the GPU/scene graph, or the JS VM?) before you start naming types.

### Step 3 — where the wins are, in yield order

Work down this list. The top items routinely return 10–100×; the bottom is where most people start.

1. **Cadence — is it running at all, and how often?** The cheapest work is work you don't do. Match every poll to how fast the data actually changes, and check whether something else already refreshes it. In a system with 13 clients polling one daemon, duplicate cadence is the default failure, not the exception.
2. **Eager instantiation — is it built before it's needed?** 314 components live under `vibecoder/src/components/`. A panel that mounts on app start because it's a direct child of a tab container costs its whole subtree at launch. Use `React.lazy()` + `Suspense` for panel routes; `OnceCell`/`LazyLock` for expensive Rust singletons. **Defer, don't unload** — latch activation false-until-first-shown, true forever after; tearing down on exit trades startup cost for navigation jank and loses panel state.
3. **Recycling — is it rebuilt on every scroll or frame?** Virtualise long lists (file trees, log views, chat transcripts). Recycling reduces *churn*, not the resident set — claim the right win. Check the precondition: recycled rows skip one-time init, so per-index work in a mount effect goes silently stale.
4. **Dirty-checking — does it notify when nothing changed?** A change signal that drives a re-render or an SSE broadcast costs the same whether or not anything moved. **Make a dirty check conservative:** compare *more strictly* than the thing you're guarding, so you may over-notify but never under-notify. A dirty check that can suppress a real update produces a silently stale UI — far worse than the cost it saves. Exclude fields that change every tick by definition (timestamps, sequence numbers); including one makes the check always true.
5. **Only now:** algorithms, containers, allocation → [Allocation discipline](#allocation-discipline).

### Step 4 — re-measure like-for-like

Same protocol, same warm-up, same state, or you are comparing runs rather than changes. Write the protocol down ("fresh daemon start + 35 s idle"). If a counter reads 397 / 487 / 362 / 415 MB across runs, quote the **peak** and say the metric is noisy.

### Step 5 — verify the feature

**A performance number that improves because a feature stopped working is the easiest way to ship a regression while celebrating it.** Idle CPU of 0 % can mean the app stopped fetching. Memory down 39 % can mean a panel renders empty. Before/after numbers are not evidence on their own — open the panel and look at it.

### Knowing when to stop

Optimising something you haven't measured as a problem is churn that reads as progress. If you fix a latent issue because the fix is genuinely small, **say plainly that it is not a measured win**. "Grows forever" is worth removing from a daemon that runs for weeks even when it's currently trivial — but don't dress it up as a performance result. Record what you deliberately left alone, and why.

---

## Modelling Honesty — a model that cannot be wrong is not a model

These are correctness bugs that no compiler, linter, or type check will ever catch, because the code is valid and the output is plausible.

### A fallback default can be a data-integrity bug

`unwrap_or(now())` on a missing timestamp reads like ordinary defensive coding. It stamps every record with the import time — and anything downstream that classifies by date is then silently wrong, with nothing on screen looking odd. **A blank invites a question; a plausible value does not.**

**Ask what the default asserts.** `unwrap_or(0)` on a count asserts "none" — usually true. `unwrap_or_else(Utc::now)` on a date asserts "this happened now" — a claim about the world nobody checked. `unwrap_or_default()` on a config struct asserts "the user configured nothing", which is a different statement from "we failed to read the config".

Absent data should stay absent: model it (`Option`, or `0` = unknown *contractually*), render it blank, **refuse to derive from it**, and say so at the boundary. This applies directly to provider metadata, token counts, session timestamps, and anything the watch/mobile clients render as a fact.

### A clamp is not a value

A guard rail that silently substitutes its own bound converts "this is broken" into "this has an opinion". If a clamp binds — context-window truncation, a token budget, a rate-limit backoff ceiling, a retry cap — **say so in the type and mark it in the UI**. Return `Clamped { value, bound_hit: true }`, not a bare number.

**Detection habit:** when a displayed number looks wrong, check whether it is *exactly* a bound, a default, or a round multiple of another field on screen.

### Decorative parameters invite false confidence

**If a parameter can be removed without changing any output, it is not modelling anything.** A scoring function whose weights never move the ranking, a config knob every code path ignores, a `temperature` forwarded into a provider that drops it — ship the simple form and name it accurately. When a constant was tuned on a handful of examples, record that it is provisional rather than derived.

### Units and bases don't survive multiplication

Wherever two numbers from different sources meet in one expression, check they share a basis — tokens vs. characters, ms vs. s, per-request vs. per-session, one provider's token accounting vs. another's. Prefer deriving through a **unit-free ratio** the source also supplies, which fixes every affected case rather than the one that was noticed. Two of three sources agreeing is not validation; it's how the bug hid.

---

## Verification — a green build proves nothing

The PostToolUse hooks run `cargo check` and `tsc --noEmit` after every edit. **Treat a clean hook as the floor, not as evidence.** Ranked by what this class of check actually catches:

| Check | What it catches |
|---|---|
| typecheck / `cargo check` / build | signature mismatches — rarely the bug you shipped |
| linter | little that's new when there's a standing warning backlog |
| **run it and read stderr** | wiring errors, missing config, panics on startup |
| **open the panel and look at it** | wrong data, empty state, wrong provider |
| **reconcile one number against an external reference** | unit, clamp, and default bugs |
| **hit the route / load the page** | runtime crashes a typecheck passes |

A typecheck-clean build can still crash at runtime — e.g. a constant removed while a concurrently-edited file still references it. Type checking does not protect you from concurrent edits to the same file.

### Lazily-validated code is unverified code

A React panel is only exercised when something **renders** it; a Tauri command is only exercised when something **invokes** it; a daemon route is only exercised when a client **calls** it. Anything behind a tab nobody opened during review is unverified no matter how green the build is.

**Corollary for reviews:** "it builds and lints" is not evidence a change works. **If the change is behind a tab, the tab is part of the change.** Say explicitly which surfaces you exercised and which you did not.

### Prove the artifact carries the change

Building a binary is not the same as shipping it. Bundlers and installers can embed a *staged copy* refreshed by a separate step, and a supervisor that reuses an already-running instance will show you correct behaviour from a package that would misbehave on a clean machine.

```bash
strings /Applications/VibeCoder.app/Contents/MacOS/vibecli | grep -c 'a_string_only_new_code_has'   # 0 means stale
```

When the artifact is compiled or compressed, `strings` finds nothing — compare the content hash, or `cmp` against the exact build you tested. This matters most for the 13-client matrix, where a stale sidecar looks exactly like a protocol bug.

### Reporting

State plainly what you ran, what you saw, and what you did not check. "Tests pass" without naming the command is not a report. If a step was skipped, say so.

---

## Traps a Build Cannot Catch

### An affordance is a claim

A UI that offers a capability is asserting the system has it. The gap between "the screen offers it" and "the system does it" is where the worst surprises live — a settings toggle nothing reads, a panel writing to a table nobody queries, an enum-driven registry where one variant never appears at a call site.

**Detection habits:**
- For registry/enum dispatch (providers, transports, tool kinds), grep the dispatch function and confirm **every** variant appears at a real call site. Absence of a compile error proves nothing when the enum is data, not control flow.
- When auditing capability, trace each UI affordance to the code that fulfils it — panel → Tauri command → daemon route → the thing that actually does the work.
- **A schema is not a feature.** A table plus its writer can sit behind `#[allow(dead_code)] // wire or remove` for months with the table empty and the writer never called. Either wire it this week or delete it.

### Duplicated contracts drift silently

This repo defines the same contract in up to eight places — `serve.rs`, two Tauri wrappers, `api_client.dart`, `WatchNetworkManager.swift`, Wear Kotlin, `api-client.ts`, and the SDK. **Nothing fails when one copy gains a route and another doesn't** — that client simply can never call it. This is exactly why the [Change-Surface Cookbook](#change-surface-cookbook--when-i-change-x-i-also-need-to-touch) exists; skipping a row there produces a silent capability gap, not a build error. If you add a route and deliberately skip a client, write that down in the PR.

### Porting copies conventions, not just code

When a feature moves between surfaces (desktop → watch, CLI → mobile), the defaults travel differently from the logic. A ported view can look right and be driven by a different window size, page limit, or timeout — neither codebase wrong in isolation. **Diff the inputs, not just the render:** run both surfaces against the same daemon and the same session, and compare.

### Upsert semantics

`INSERT OR REPLACE` deletes the row and reinserts it, so refreshing one column silently NULLs every column the statement didn't name. Use `INSERT … ON CONFLICT(pk) DO UPDATE SET`, touching only the columns you own. **Corollary:** when one row has two independently-refreshed halves, give each its own timestamp — sharing one lets whichever writer ran last decide the other's apparent freshness.

### "An error appeared after my change" ≠ "my change caused it"

Check provenance before you assume causation, and say which it was. A pre-existing bug surfaced by your change is worth fixing and worth *labelling* as pre-existing — otherwise the next reader learns the wrong lesson about the change.

---

## Provider-Agnostic Panels — STRICT

**Every panel that calls an LLM MUST use the provider and model selected in the toolbar dropdown.** No panel may hard-code Anthropic, OpenAI, or any other provider as the only path to an LLM. This rule is non-negotiable: a user who has switched the toolbar to OpenAI / Gemini / Groq / Cerebras / Ollama / OpenRouter / a local mistralrs model expects every panel to obey, and a panel that ignores the selection is a bug.

**What this means in practice:**

1. **Source of truth.** The toolbar's `selectedProvider` and `selectedModel` state live in `vibecoder/src/App.tsx` and are forwarded as props (or read from a shared hook). New panels must accept them as props or call `useModelRegistry()` + the toolbar selectors — never instantiate a provider client directly.

2. **Forward into Tauri commands.** Every Tauri command that calls an LLM must take `provider: String` and `model: String` parameters and route through `build_temp_provider()` (or equivalent dispatch) in `vibecoder/src-tauri/src/commands.rs`. No `commands.rs` handler may default to `"anthropic"` when the caller didn't specify — refuse the call instead.

3. **Forward into the daemon.** HTTP routes that proxy to a provider must read the provider/model from the request body, NOT from `config.toml` defaults. The daemon's `/api/chat`-class routes already do this; preserve the contract.

4. **No silent fallback to a hard-coded default.** If the toolbar provider/model is empty (e.g. very first launch), the panel must surface a "select a model" empty-state — it must not silently invoke Anthropic.

5. **Reference implementation.** `vibecoder/src/components/GitPanel.tsx` is the canonical example: it accepts `selectedProvider?: string` from `App.tsx`, forwards it to AI git commands, and degrades gracefully when unset. New panels with LLM calls must follow this pattern.

**When auditing a panel for compliance:**

- `grep -n "anthropic\|claude-" <panel>.tsx <related>.rs` — any literal that pins a provider/model is a bug.
- Confirm the panel accepts `selectedProvider` (and `selectedModel` where relevant) as a prop, or reads it from a shared toolbar hook.
- Confirm every Tauri command it calls forwards those values into the Rust provider dispatch.
- Confirm the daemon route it ultimately hits reads provider/model from the request, not from a static config.

**Exceptions** (narrow, must be documented in the panel header comment):

- Panels whose entire purpose is one specific provider (e.g. a hypothetical "Anthropic Console" debug panel) — and even then, prefer a generic implementation.
- Local-only inference paths that explicitly bypass cloud providers (e.g. mistralrs / Ollama-only panels) — must still respect the toolbar when the user picks one of those.

If a feature genuinely cannot work without a specific provider's capability (e.g. computer-use), the panel must (a) surface that requirement in its empty-state, (b) gate the call so users on other providers see a clear "this feature requires provider X" message, not an opaque API error.

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
├── jobs.db               ← encrypted: async job records + scratchpad
├── openmemory/           ← cognitive memory store (encrypted at rest option)
└── config.toml           ← CLI feature flags, provider enable/disable (no keys here)

<workspace>/
└── .vibecli/
    ├── workspace.db      ← encrypted: project settings + project secrets
    ├── MEMORY.md         ← auto-generated from OpenMemory (project tier)
    └── openmemory/       ← project-scoped memory (optional)
```

`config.toml` is for non-sensitive configuration only (enabling providers, setting model names, feature flags). API keys belong in `profile_settings.db`.

For detailed architecture including the five memory stores, Context Assembler, and storage security model, see [`docs/memory-architecture.md`](./docs/memory-architecture.md).

---

## Key Derivation & Security Model

- **Profile key**: `SHA-256("vibecli-profile-store-v1:" + $HOME + ":" + $USER)` — machine-bound
- **Workspace key**: `SHA-256("vibecli-workspace-store-v1:" + $HOME + ":" + $USER + ":" + workspace_path)` — machine + project bound
- **Company master keys**: encrypted inside `profile_settings.db` using the profile key. Secrets in `company.db` are then encrypted with those master keys (two-layer encryption).
- **Nonces**: 12-byte random nonce prepended to every ciphertext blob; each write generates a fresh nonce.

---

## Adding / Updating Providers and Models

### Frontend only (update model list or default)

Edit **one file**: `vibecoder/src/hooks/useModelRegistry.ts`

| Goal | What to change |
|---|---|
| Add a new provider | Add model array to `STATIC_MODELS`; add default to `PROVIDER_DEFAULT_MODEL` |
| Add a model to an existing provider | Append to its array in `STATIC_MODELS` |
| Change a provider's default model | Update `PROVIDER_DEFAULT_MODEL[provider]` |

All UI panels consume `useModelRegistry()` — no other frontend file needs updating.

### Full backend provider (new Rust implementation)

Touch these files in order:

1. **`vibecoder/crates/vibe-ai/src/providers/{name}.rs`** — implement the `AIProvider` trait.  
   For OpenAI-compatible APIs, copy `groq.rs` — it's the thinnest implementation.

2. **`vibecoder/crates/vibe-ai/src/providers.rs`** — export the new module:

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

6. **`vibecoder/src-tauri/src/commands.rs`** — add match arm in `build_temp_provider()` and map the API key field in `load_api_key_settings()` / `save_api_key_settings_to_store()`.

Then update `useModelRegistry.ts` as described above.

---

## Design System — mandatory for every panel, tab, and UI feature

VibeCoder ships its own token-based design system at **`vibecoder/design-system/`**. It is **not optional**. Every new panel, tab, dialog, modal, popover, or in-line widget that you add to `vibecoder/src/components/` must consume tokens and CSS classes from there. Reviewers will reject a PR that introduces hard-coded colors, ad-hoc spacing, reinvented toast/empty/loading states, or `<div onClick>` where a `<button>` belongs.

### Read first

| File | Read when |
|---|---|
| [`vibecoder/design-system/README.md`](./vibecoder/design-system/README.md) | Always. Has the 10 rules every panel must follow + minimal panel template + color/spacing/font quick-pick. |
| `vibecoder/design-system/tokens.css` | Looking up a CSS variable. |
| `vibecoder/design-system/foundations/{color,typography,spacing,elevation,motion}.md` | Picking semantic colors / sizing / shadows. |
| `vibecoder/design-system/components/{panel,button,input,card,badge-tag,progress,table,tabs}.md` | Implementing a primitive. Use the documented `panel-*` class — do not roll your own. |
| `vibecoder/design-system/patterns/{data-states,forms}.md` | Loading/empty/error states or any form. |

### Hard rules

1. **Never write hex colors** (`#fff`, `#4caf50`, etc.) — always `var(--text-primary)`, `var(--success-color)`, etc. The only legal exception is icon assets that already carry `currentColor`.
2. **Never write a panel without the `panel-container` → `panel-header` → `panel-body` (→ `panel-footer`) skeleton.** The minimal template at the bottom of `vibecoder/design-system/README.md` is the starting point.
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

1. **Red.** Add the failing scenario to `*.bdd.test.tsx` first (or create one). Run `npm test --prefix vibecoder -- --run <PanelName>` and confirm it fails for the *expected* reason.
2. **Green.** Implement the smallest change that passes. Use the design-system classes — do not stub with inline styles "for now" intending to refactor later.
3. **Refactor.** Extract repeated markup into a shared component in `src/components/composite/` or a sub-component file.

Backend changes that span the daemon use the cucumber-style feature files in `vibecli/vibecli-cli/tests/features/*.feature` paired with a `*_bdd.rs` step file. Frontend-only changes stay in `*.bdd.test.tsx`.

### When you are unsure

Open an existing well-formed panel as your reference: `SettingsPanel.tsx`, `CostPanel.tsx`, `BackgroundJobsPanel.tsx`, `DiffReviewPanel.tsx`, `DesignHubPanel.tsx`, `DesignAnnotationsPanel.tsx`, or `DesignImportPanel.tsx` (the design-panel cluster was migrated to the design system in April 2026 — see `vibecoder/src/components/__tests__/DesignHubPanel.bdd.test.tsx` for the BDD scenarios that lock in the contract). **Do not** copy from `DesignMode.tsx` — it still predates the design system and is being migrated.

---

## Codebase Layout

```
vibecli/vibecli-cli/src/
├── profile_store.rs     ← system-level encrypted store
├── workspace_store.rs   ← project-level encrypted store
├── company_secrets.rs   ← company secret vault (uses profile_store for master keys)
└── config.rs            ← VibeCLI TOML config (non-sensitive)

vibecoder/src-tauri/src/
├── panel_store.rs       ← thin re-export of ProfileStore
└── commands.rs          ← Tauri commands (profile_*, workspace_*, panel_settings_*)

vibecoder/src/hooks/
└── useModelRegistry.ts  ← single source of truth for provider list + model lists

vibecoder/src/constants/
└── ollamaModels.ts      ← Ollama static fallback model list

vibecoder/crates/vibe-infer/
├── src/lib.rs           ← Embedder + TextGenerator traits, StubBackend (default)
├── src/minilm.rs        ← candle BERT MiniLM-L6-v2 backend (feature: candle)
└── examples/embed.rs    ← end-to-end candle smoke-test
```

`vibe-infer` is the in-process inference layer (see also "Adding / Updating Providers and Models" — it complements the sidecar-based providers in `vibecoder/crates/vibe-ai/`). Default builds pull no ML deps; opt in with `--features candle` (or `candle-metal` on macOS for GPU). `LocalEmbeddingEngine` in `vibecli/vibecli-cli/src/open_memory.rs` implements `vibe_infer::Embedder`, so async sites can take `&dyn vibe_infer::Embedder` and swap backends without touching OpenMemory.

---

## Icons

All icons across VibeCoder **must** be thin, themable SVGs. No emoji, Unicode symbols (▶ ▼ ◀ ×), or raster images as icons.

### Use the `<Icon>` component

```tsx
import { Icon } from "./Icon";

<Icon name="chevron-right" size={14} />
<Icon name="maximize" size={16} style={{ color: "var(--accent-color)" }} />
```

All available names are declared in the `IconName` union type in `vibecoder/src/components/Icon.tsx`. TypeScript will error on unknown names — check that file before using a name.

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

---

## The Craft Checklist

Short enough to actually run. Copy the relevant block into the PR description.

**Before optimising**
- [ ] A measurement, not a hunch — CPU, memory, or latency?
- [ ] Attributed from the **call tree**, not a leaf list
- [ ] Baseline recorded, with the exact protocol written down

**While optimising**
- [ ] Cadence → eager instantiation → recycling → dirty-checks, *before* micro-work
- [ ] Dirty checks conservative: may over-notify, never under-notify
- [ ] Nothing "fixed" that was not measured as a problem

**After optimising**
- [ ] Re-measured like-for-like, same protocol
- [ ] **The feature still works — checked on screen, not in a metric**
- [ ] Noisy counters reported at their stable extreme, with the noise stated

**Before calling it shipped**
- [ ] Ran it and read stderr (`vibecli --serve --port N`, `npm run tauri:dev`)
- [ ] Opened the panel — including the one behind a tab
- [ ] Every affordance traces to code that fulfils it
- [ ] Walked the [Change-Surface Cookbook](#change-surface-cookbook--when-i-change-x-i-also-need-to-touch) row for this change type
- [ ] Reconciled at least one number against an external reference
- [ ] Proved the **deployed artifact** contains the change (string, hash, or `cmp`)

**Recording it**
- [ ] The number before and after
- [ ] The cause, not just the change
- [ ] What you deliberately left alone, and why
- [ ] What is still **untested**, marked as such
