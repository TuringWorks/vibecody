# VibeCody

**VibeCody** is an AI-powered developer toolchain built entirely in Rust. It combines a terminal-first CLI coding assistant (**VibeCLI**) with a full-featured desktop code editor (**VibeUI**), both powered by a shared library of AI and editor primitives.

---

## Projects

| Project | Description | Status |
|---------|-------------|--------|
| [VibeCLI](./vibecli/) | AI coding assistant for the terminal (TUI + REPL) | Active |
| [VibeUI](./vibeui/) | AI-powered desktop code editor (Tauri + Monaco) | Active |

---

## Quick Start

### VibeCLI — Terminal AI Assistant

```bash
# Build
cargo build --release -p vibecli

# Run with TUI
./target/release/vibecli --tui

# Run with a specific provider
./target/release/vibecli --tui --provider claude --model claude-3-5-sonnet-20241022
```

### VibeUI — Desktop Editor

```bash
cd vibeui

# Install frontend dependencies
npm install

# Run in development mode
npm run tauri dev

# Build for production
npm run tauri build
```

---

## Workspace Structure

```
vibecody/
├── Cargo.toml                  # Workspace root
├── vibecli/
│   ├── README.md
│   └── vibecli-cli/            # CLI binary (TUI + REPL)
│       └── src/
│           ├── main.rs         # Entry point, command routing
│           ├── config.rs       # TOML config (~/.vibecli/config.toml)
│           ├── diff_viewer.rs  # Diff rendering
│           ├── syntax.rs       # Syntax highlighting
│           ├── repl.rs         # Rustyline REPL helper
│           └── tui/            # Ratatui TUI implementation
│               ├── mod.rs      # TUI run loop
│               ├── app.rs      # Application state
│               ├── ui.rs       # Layout & rendering
│               └── components/ # Chat, DiffView, etc.
└── vibeui/
    ├── README.md
    ├── src/                    # React + TypeScript frontend
    │   ├── App.tsx             # Root component
    │   └── components/         # AIChat, GitPanel, Terminal, etc.
    ├── src-tauri/              # Tauri Rust backend
    └── crates/                 # Shared Rust library crates
        ├── vibe-core/          # Text buffer, FS, workspace, Git
        ├── vibe-ai/            # AI provider abstraction + implementations
        ├── vibe-lsp/           # Language Server Protocol client
        └── vibe-extensions/    # WASM-based extension system
```

---

## Shared Crates

The `vibeui/crates/` libraries are designed to be reused across both VibeCLI and VibeUI:

### `vibe-core`
Core editor primitives — text buffer (rope-based), file system operations, workspace management, Git integration, terminal PTY, diff engine, and code search.

### `vibe-ai`
Unified AI provider abstraction. Supports:
- **Ollama** — Local/private models (default)
- **Anthropic Claude** — Claude 3.5 Sonnet/Opus
- **OpenAI** — GPT-4 and variants
- **Google Gemini** — Gemini Pro 1.5
- **xAI Grok** — Grok Beta

### `vibe-lsp`
Language Server Protocol client for intelligent code features (go-to-definition, diagnostics, completions).

### `vibe-extensions`
WASM-based extension runtime (Wasmtime), enabling a VSCode-compatible plugin API.

---

## AI Providers

All providers implement the `AIProvider` trait from `vibe-ai`:

```rust
#[async_trait]
pub trait AIProvider: Send + Sync {
    fn name(&self) -> &str;
    async fn is_available(&self) -> bool;
    async fn complete(&self, context: &CodeContext) -> Result<CompletionResponse>;
    async fn stream_complete(&self, context: &CodeContext) -> Result<CompletionStream>;
    async fn chat(&self, messages: &[Message], context: Option<String>) -> Result<String>;
    async fn stream_chat(&self, messages: &[Message]) -> Result<CompletionStream>;
}
```

Configure providers in `~/.vibecli/config.toml`:

```toml
[ollama]
enabled = true
api_url = "http://localhost:11434"
model = "qwen2.5-coder:7b"

[claude]
enabled = false
api_key = "sk-ant-..."
model = "claude-3-5-sonnet-20241022"

[openai]
enabled = false
api_key = "sk-..."
model = "gpt-4o"

[gemini]
enabled = false
api_key = "AIza..."
model = "gemini-1.5-pro"

[grok]
enabled = false
api_key = "..."
model = "grok-beta"

[safety]
require_approval_for_commands = true
require_approval_for_file_changes = true
```

---

## Prerequisites

| Requirement | Version | Notes |
|-------------|---------|-------|
| Rust | stable (≥ 1.75) | `rustup update stable` |
| Node.js | ≥ 18 | For VibeUI frontend |
| npm / pnpm | any | For VibeUI frontend |
| Ollama | any | Optional — for local AI |

---

## Running Tests

```bash
# All workspace tests
cargo test --workspace

# Specific crates
cargo test -p vibe-core
cargo test -p vibe-ai
cargo test -p vibecli

# Type-check frontend
cd vibeui && npx tsc --noEmit
```

---

## Documentation

Full documentation is available at the [GitHub Pages site](https://vibecody.github.io/vibecody/) *(replace with actual URL)*.

- [Architecture Overview](./docs/architecture.md)
- [VibeCLI Reference](./docs/vibecli.md)
- [VibeUI Reference](./docs/vibeui.md)
- [Configuration Guide](./docs/configuration.md)
- [Contributing](./docs/contributing.md)

---

## License

MIT — see individual crate `Cargo.toml` files.

---

## Acknowledgments

- [Tauri](https://tauri.app/) — Desktop application framework
- [Monaco Editor](https://microsoft.github.io/monaco-editor/) — Code editor component
- [Ratatui](https://ratatui.rs/) — Terminal UI framework
- [Ropey](https://github.com/cessen/ropey) — Rope data structure for text buffers
- [Ollama](https://ollama.ai) — Local LLM runtime
