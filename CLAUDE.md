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

---

## Repo Layout
```
vibecli/vibecli-cli/src/   ← Rust CLI (~196 modules)
vibeui/src/                ← React/TypeScript frontend
vibeui/src-tauri/src/      ← Tauri backend + commands
vibeui/crates/             ← vibe-core, vibe-ai, vibe-lsp, vibe-extensions
docs/                      ← Jekyll GitHub Pages
```
