# VibeCody — Claude Code Guidelines

See **[AGENTS.md](./AGENTS.md)** for the full storage architecture, security rules, and Rust/Tauri API references that apply to all AI coding agents.

See **[vibeui/design-system/README.md](./vibeui/design-system/README.md)** for the complete UI/UX design system — tokens, components, and patterns that all panels must follow.

---

## Quick Reference

### Build

```bash
cargo build --release -p vibecli          # CLI binary
cargo test --workspace                    # all workspace tests
cargo check --workspace --exclude vibe-collab
cd vibeui && npm install && npm run tauri:dev   # VibeUI dev

# Mobile + watch (platform-gated — iOS/watchOS targets require macOS + Xcode)
make mobile-ios                # Flutter iOS build (unsigned)
make mobile-android            # Flutter Android APK + AAB
make watch-ios                 # watchOS Simulator build (Xcode)
make watch-wear                # Wear OS APK (gradlew)
make build-all                 # what CI builds — Rust + Tauri + Mobile + Watch
```

### Module declaration pattern

`vibecli/vibecli-cli/src/` currently holds ~354 `.rs` files. Both `lib.rs` (`pub mod foo;`) and `main.rs` (`mod foo;`) must declare a module before it can be used in its respective crate artifact. When adding a new `.rs` file, register it in the crate(s) that consume it.

### Key storage rules (summary — see AGENTS.md for full details)

- API keys → `ProfileStore` (`~/.vibecli/profile_settings.db`)
- Project secrets → `WorkspaceStore` (`<workspace>/.vibecli/workspace.db`)
- Never write keys to `*.toml`, `*.json`, or any plaintext file
- Never read from `~/.vibeui/api_keys.json` — deleted and migrated

### Tauri commands

1,045+ commands registered via `tauri::generate_handler!` in `vibeui/src-tauri/src/lib.rs`. When adding a new Tauri command: implement in `commands.rs`, register in `tauri::generate_handler!` in `lib.rs`.

### Testing encrypted stores

Use `open_with(path, key)` variants to avoid touching production DBs:

```rust
let store = ProfileStore::open_with(&tmp_dir.join("test.db"), [42u8; 32]).unwrap();
let store = WorkspaceStore::open_with(&tmp_dir.join("ws.db"), [42u8; 32]).unwrap();
```

### Adding / updating providers and models

**Frontend only (model list / default)** — edit one file:
> `vibeui/src/hooks/useModelRegistry.ts`

| Goal | What to edit |
|---|---|
| Add a new provider | Add model array to `STATIC_MODELS` + default to `PROVIDER_DEFAULT_MODEL` |
| Add a model to existing provider | Append to the array in `STATIC_MODELS` |
| Change a provider's default model | Update `PROVIDER_DEFAULT_MODEL[provider]` |

All panels (Arena, MultiModel, BackgroundJobs, SuperBrain, Counsel, …) consume `useModelRegistry()` — no other frontend file needs changing.

**Full backend provider** (new Rust implementation) — touch 6 files in order:

1. `vibeui/crates/vibe-ai/src/providers/{name}.rs` — implement `AIProvider` trait (copy `groq.rs` for OpenAI-compat APIs)
2. `vibeui/crates/vibe-ai/src/providers.rs` — `pub mod {name}; pub use {name}::MyProvider`
3. `vibecli/vibecli-cli/src/config.rs` — add `pub {name}: Option<ProviderConfig>` to `Config`
4. `vibecli/vibecli-cli/src/main.rs` — match arm in `create_raw_provider()`
5. `vibecli/vibecli-cli/src/api_key_monitor.rs` — match arm + env var in `resolve_env_key()` + name in `configured_providers()`
6. `vibeui/src-tauri/src/commands.rs` — `build_temp_provider()` match arm + key field mapping

Then add the frontend entry in `useModelRegistry.ts` as above.

---

## Claude Code Setup

- **Plan model**: `claude-opus-4-6` — plan mode uses Opus (set in `~/.claude/settings.json` globally; `.claude/settings.json` reinforces `RUST_BACKTRACE=1` and hooks)
- **LSP**: `rust-analyzer` + `typescript-lsp` active globally
- **PostToolUse hooks** (`.claude/settings.json`): after any `Edit`/`Write`, automatically runs:
  - `.rs` files → `cargo check --workspace --exclude vibe-collab` (tail 8 lines)
  - `.ts`/`.tsx` files → `npx --prefix vibeui tsc --noEmit` (tail 5 lines)
- **Env**: `RUST_BACKTRACE=1` set in all sessions

---

## Repo Layout

```
vibecli/vibecli-cli/src/   ← Rust CLI (~354 modules, ~16 kloc in main.rs alone)
vibecli/vibecli-cli/tests/ ← 62+ BDD/integration harnesses
vibecli/vibecli-cli/skills/← 711 skill files (25+ categories)
vibeui/src/                ← React/TypeScript frontend (~293 panels + 42 composites)
vibeui/src-tauri/src/      ← Tauri backend + commands (1,045+ via generate_handler!)
vibeui/crates/             ← vibe-core, vibe-ai (22 providers), vibe-lsp, vibe-extensions, vibe-collab
vibeapp/                   ← Secondary Tauri shell
vibemobile/                ← Flutter mobile companion (11 screens, 6 services)
vibewatch/                 ← Apple Watch (SwiftUI) + Wear OS (Kotlin Compose) + companions
vibe-indexer/              ← Standalone indexing service
vscode-extension/          ← VS Code extension
jetbrains-plugin/          ← JetBrains plugin
neovim-plugin/             ← Neovim plugin
packages/agent-sdk/        ← TypeScript Agent SDK
docs/                      ← Jekyll GitHub Pages
```

### Watch, mobile, and connectivity (recent additions)

- **Watch integration** — `watch_auth.rs` (P256 ECDSA / Secure Enclave), `watch_bridge.rs` (Axum `/watch/*` routes), `watch_session_relay.rs` (OLED-optimised payloads). Full spec: [docs/WATCH-INTEGRATION.md](./docs/WATCH-INTEGRATION.md).
- **Mobile pairing** — `pairing.rs` generates one-time pairing URLs; all platforms (Watch, iOS, Android, Wear OS) support URL/JSON manual pairing plus QR.
- **Zero-config connectivity** — `mdns_announce.rs` (LAN), `tailscale.rs` (mesh + Funnel public HTTPS), `ngrok.rs` (auto-detect + auto-start). App races all reachable paths on `/mobile/beacon`. Full spec: [docs/connectivity.md](./docs/connectivity.md).
- **Cryptography note**: watch registration uses **P256 ECDSA (secp256r1)** — the only algorithm Apple Secure Enclave supports. Do not reintroduce Ed25519 for device keys.
