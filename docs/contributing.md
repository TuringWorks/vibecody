---
layout: page
title: Contributing
permalink: /contributing/
---

# Contributing to VibeCody

Thank you for your interest in contributing! This guide covers how to get your development environment set up, the project conventions, and the contribution workflow.

---

## Development Setup

### 1. Clone the Repository

```bash
git clone https://github.com/vibecody/vibecody.git
cd vibecody
```

### 2. Install Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup update stable
```

### 3. Install Node.js (for VibeUI)

Download from [nodejs.org](https://nodejs.org/) (LTS ≥ 18), or via a version manager:

```bash
# Using nvm
nvm install --lts
nvm use --lts
```

### 4. Install Tauri Prerequisites (for VibeUI)

Follow the Tauri v2 system prerequisites guide for your OS:
[tauri.app/start/prerequisites](https://tauri.app/start/prerequisites/)

**macOS quick setup:**

```bash
xcode-select --install
```

**Linux quick setup:**

```bash
sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev
```

### 5. Install Frontend Dependencies (for VibeUI)

```bash
cd vibeui && npm install && cd ..
```

---

## Building

### Build Everything

```bash
cargo build --workspace
```

### Build Specific Targets

```bash
# VibeCLI binary
cargo build -p vibecli

# VibeUI (Tauri + React)
cd vibeui && npm run tauri build
```

### Development Mode

```bash
# VibeCLI with auto-reload (cargo-watch required)
cargo watch -x 'run -p vibecli -- --tui'

# VibeUI with hot reload
cd vibeui && npm run tauri dev
```

---

## Running Tests

```bash
# All workspace tests
cargo test --workspace

# Specific crate
cargo test -p vibe-core
cargo test -p vibe-ai
cargo test -p vibecli

# With output (useful for debugging)
cargo test -p vibe-core -- --nocapture

# TypeScript type check
cd vibeui && npx tsc --noEmit
```

---

## Code Style

### Rust

This project follows standard Rust conventions enforced by `rustfmt` and `clippy`:

```bash
# Format code
cargo fmt --all

# Lint
cargo clippy --workspace -- -D warnings
```

Key conventions:

- Use `anyhow::Result` for fallible public APIs
- Use `thiserror` for library-level error types
- Prefer `async_trait` for async trait methods
- Document public APIs with `///` doc comments
- Add integration tests in `tests/` subdirectories for complex features

### TypeScript / React

```bash
cd vibeui

# Type check
npx tsc --noEmit
```

Key conventions:

- Functional components with hooks
- Explicit TypeScript types (avoid `any`)
- CSS modules or co-located `.css` files for component styles
- Tauri IPC calls wrapped in typed helper functions

---

## Project Conventions

### Commit Messages

Use conventional commits format:

```text
feat: add streaming support to Gemini provider
fix: handle empty git repo in get_repo_diff
docs: update VibeCLI configuration reference
refactor: extract ChatEngine from main.rs
test: add unit tests for DiffEngine
chore: update tokio to 1.43
```

Types: `feat`, `fix`, `docs`, `refactor`, `test`, `chore`, `perf`, `ci`

### Branch Names

```text
feat/gemini-streaming
fix/ollama-parse-error
docs/architecture-diagram
```

### Pull Requests

- Target the `main` branch
- Include a description of what changed and why
- Reference any related issues
- Ensure all tests pass before requesting review

---

## Areas to Contribute

### Good First Issues

- Add more unit tests for `vibe-core` modules
- Improve error messages in the TUI
- Add syntax highlighting for additional languages in VibeCLI
- Document Tauri commands in `src-tauri/src/`

### Larger Features

- **Additional AI providers** — implement new `AIProvider` backends (17 providers exist today)
- **WASM extensions** — develop plugins using the `vibe-extensions` WASM runtime
- **CRDT collaboration** — enhance the `vibe-collab` real-time multiplayer editing
- **Agent skills** — write new skill files in `vibecli/vibecli-cli/skills/` (507 skills exist today)
- **Gateway adapters** — add new messaging platform adapters (18 platforms supported)
- **TUI enhancements** — improve the Ratatui-based terminal interface

---

## Adding a New AI Provider

1. Create `vibeui/crates/vibe-ai/src/providers/<name>.rs`
2. Implement `AIProvider` for your new struct
3. Export from `vibeui/crates/vibe-ai/src/providers.rs`
4. Wire up in VibeCLI's `create_provider()` in `vibecli-cli/src/main.rs`
5. Add to VibeUI's provider dropdown in `src/App.tsx`
6. Document in the [Configuration Guide](../configuration/)

```rust
// providers/myprovider.rs
use crate::provider::{AIProvider, CodeContext, CompletionResponse, CompletionStream, Message, ProviderConfig};
use anyhow::Result;
use async_trait::async_trait;

pub struct MyProvider {
    config: ProviderConfig,
    client: reqwest::Client,
}

impl MyProvider {
    pub fn new(config: ProviderConfig) -> Self {
        Self { config, client: reqwest::Client::new() }
    }
}

#[async_trait]
impl AIProvider for MyProvider {
    fn name(&self) -> &str { "myprovider" }
    // ... implement remaining methods
}
```

---

## Documentation

The `docs/` folder is a Jekyll site deployed to GitHub Pages.

To preview locally:

```bash
cd docs
gem install bundler jekyll
bundle install
bundle exec jekyll serve
# Open http://localhost:4000
```

Pages use GitHub-flavored Markdown with Jekyll front matter. Add a new page by creating `docs/mypage.md` with:

```markdown
---
layout: page
title: My Page
permalink: /mypage/
---
```

Then add it to `docs/_config.yml` under `header_pages`.

---

## License

By contributing to VibeCody, you agree that your contributions will be licensed under the MIT License.
