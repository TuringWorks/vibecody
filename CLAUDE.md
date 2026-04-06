# VibeCody — Claude Code Guidelines

See **[AGENTS.md](./AGENTS.md)** for the full storage architecture, security rules, and Rust/Tauri API references that apply to all AI coding agents.

---

## Quick Reference

### Build
```bash
cargo build --release -p vibecli          # CLI binary
cargo test --workspace                    # all tests (~10,535)
cargo check --workspace --exclude vibe-collab
cd vibeui && npm install && npm run tauri:dev   # VibeUI dev
```

### Module declaration pattern
Both `lib.rs` and `main.rs` in `vibecli/vibecli-cli/src/` must declare new modules. When adding a new `.rs` file, add `pub mod foo;` to **both** files.

### Key storage rules (summary — see AGENTS.md for full details)
- API keys → `ProfileStore` (`~/.vibecli/profile_settings.db`)
- Project secrets → `WorkspaceStore` (`<workspace>/.vibecli/workspace.db`)
- Never write keys to `*.toml`, `*.json`, or any plaintext file
- Never read from `~/.vibeui/api_keys.json` — deleted and migrated

### Tauri commands
360+ commands registered in `vibeui/src-tauri/src/lib.rs`. When adding a new Tauri command: implement in `commands.rs`, register in `invoke_handler!` in `lib.rs`.

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
vibecli/vibecli-cli/src/   ← Rust CLI (~196 modules)
vibeui/src/                ← React/TypeScript frontend
vibeui/src-tauri/src/      ← Tauri backend + commands
vibeui/crates/             ← vibe-core, vibe-ai, vibe-lsp, vibe-extensions
docs/                      ← Jekyll GitHub Pages
```
