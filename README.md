# VibeCody

**VibeCody** is an AI-powered developer toolchain built entirely in Rust. It combines a terminal-first CLI coding assistant (**VibeCLI**) with a full-featured desktop code editor (**VibeUI**), both powered by a shared library of AI and editor primitives.

---

## Projects

| Project | Description | Status |
|---------|-------------|--------|
| [VibeCLI](./vibecli/) | AI coding assistant for the terminal (TUI + REPL) | Active |
| [VibeUI](./vibeui/) | AI-powered desktop code editor (Tauri + Monaco) | Active |
| [VibeApp](./vibeapp/) | Secondary Tauri app | Active |

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
├── Cargo.toml                  # Workspace root (6 members)
├── Dockerfile                  # Multi-stage musl build (Alpine runtime)
├── docker-compose.yml          # VibeCLI + Ollama sidecar (air-gapped)
├── install.sh                  # One-liner installer (SHA-256 verified)
├── vibecli/
│   └── vibecli-cli/            # CLI binary (TUI + REPL)
│       ├── src/
│       │   ├── main.rs         # Entry point, command routing
│       │   ├── config.rs       # TOML config (~/.vibecli/config.toml)
│       │   ├── serve.rs        # HTTP daemon for VS Code ext/SDK
│       │   ├── repl.rs         # Rustyline REPL helper
│       │   └── tui/            # Ratatui TUI (app, ui, components)
│       └── skills/             # 479 skill files (25 categories)
├── vibeui/
│   ├── src/                    # React + TypeScript frontend
│   │   ├── App.tsx             # Root component
│   │   └── components/         # 60+ panel components
│   ├── src-tauri/              # Tauri Rust backend
│   └── crates/                 # Shared Rust library crates
│       ├── vibe-core/          # Text buffer, FS, workspace, Git, index
│       ├── vibe-ai/            # 17 AI providers, agents, hooks, planner
│       ├── vibe-lsp/           # Language Server Protocol client
│       ├── vibe-extensions/    # WASM-based extension system
│       └── vibe-collab/        # CRDT multiplayer collaboration
├── vibeapp/                    # Secondary Tauri app
├── vibe-indexer/               # Remote indexing service
├── vscode-extension/           # VS Code extension (chat + completions)
├── jetbrains-plugin/           # JetBrains IDE plugin
├── neovim-plugin/              # Neovim plugin
├── packages/
│   └── agent-sdk/              # TypeScript Agent SDK
├── docs/                       # Jekyll GitHub Pages site
└── .github/workflows/          # CI/CD (pages, release)
```

---

## Shared Crates

The `vibeui/crates/` libraries are designed to be reused across both VibeCLI and VibeUI:

### `vibe-core`

Core editor primitives — text buffer (rope-based), file system operations, workspace management, Git integration, terminal PTY, diff engine, code search, and embedding-based codebase indexing.

### `vibe-ai`

Unified AI provider abstraction with agent loop, hooks, planner, multi-agent orchestration, skills, artifacts, admin policy, trace/session resume, and OpenTelemetry. Supports 17 providers:

- **Ollama** — Local/private models (default)
- **Anthropic Claude** — Claude 3.5 Sonnet/Opus
- **OpenAI** — GPT-4 and variants
- **Google Gemini** — Gemini Pro 1.5
- **xAI Grok** — Grok Beta
- **Groq** — Fast inference
- **OpenRouter** — Multi-provider gateway
- **Azure OpenAI** — Enterprise Azure-hosted models
- **AWS Bedrock** — AWS-hosted models
- **GitHub Copilot** — Copilot integration
- **LocalEdit** — Local code editing model
- **Mistral** — Mistral AI models
- **Cerebras** — Wafer-scale inference
- **DeepSeek** — DeepSeek models
- **Zhipu** — GLM models
- **Vercel AI** — Vercel AI SDK
- **Failover** — Auto-failover wrapper (chains multiple providers)

### `vibe-lsp`

Language Server Protocol client for intelligent code features (go-to-definition, diagnostics, completions).

### `vibe-extensions`

WASM-based extension runtime (Wasmtime), enabling a plugin API.

### `vibe-collab`

CRDT-based multiplayer collaboration for real-time shared editing sessions.

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
    // + chat_response, chat_with_images, and more
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
model = "gemini-2.0-flash"

[grok]
enabled = false
api_key = "..."
model = "grok-3-mini"

[groq]
enabled = false
api_key = "gsk_..."
model = "llama-3.3-70b-versatile"

[mistral]
enabled = false
api_key = "..."
model = "mistral-large-latest"

# See docs/configuration.md for all 17 providers

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
| Docker | any | Optional — for container sandbox |

---

## Running Tests

**2,810 unit tests** across the workspace.

```bash
# All workspace tests (excluding collab for faster iteration)
cargo test --workspace --exclude vibe-collab

# All tests including collab
cargo test --workspace

# Specific crates
cargo test -p vibe-core    # 293 tests
cargo test -p vibe-ai      # 843 tests
cargo test -p vibecli      # 1,264 tests

# Type-check frontend
cd vibeui && npx tsc --noEmit
```

---

## Documentation

Full documentation is available at the [GitHub Pages site](https://vibecody.github.io/vibecody/) *(replace with actual URL)*.

- [Architecture Overview](./docs/architecture.md)
- [VibeCLI Reference](./docs/vibecli.md)
- [VibeUI Reference](./docs/vibeui.md)
- [Roadmap](./docs/ROADMAP.md)
- [Roadmap v2 (Phases 6–9)](./docs/ROADMAP-v2.md)
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
