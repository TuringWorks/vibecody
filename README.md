# VibeCody

**VibeCody** is an AI-powered developer toolchain built entirely in Rust. It combines a terminal-first CLI coding assistant (**VibeCLI**) with a full-featured desktop code editor (**VibeUI**), both powered by a shared library of AI and editor primitives.

---

## Projects

| Project | Description | Status |
|---------|-------------|--------|
| [VibeCLI](./vibecli/) | AI coding assistant for the terminal (TUI + REPL) | Active |
| [VibeUI](./vibeui/) | AI-powered desktop code editor (Tauri + Monaco) | Active |
| [VibeApp](./vibeapp/) | Secondary Tauri app | Active |
| **VibeMobile** | `vibemobile/` | Mobile companion app (Flutter — iOS, Android, macOS, Linux, Windows, Web) |

---

## Quick Start

### One-Command Setup (macOS / Linux / WSL)

```bash
git clone https://github.com/TuringWorks/vibecody.git
cd vibecody
make setup    # Installs Rust, Node.js, system libs, npm deps
```

Or run the setup script directly:

```bash
./scripts/setup.sh
```

### Run VibeUI (Desktop Editor)

```bash
make ui
```

### Build & Run VibeCLI (Terminal AI Assistant)

```bash
make cli                    # Build release binary
./target/release/vibecli --tui   # Run with TUI

# Or with a specific provider
./target/release/vibecli --tui --provider claude --model claude-3-5-sonnet-20241022
```

### Verify Your Environment

```bash
make doctor    # Checks all required + optional tools
```

### All Make Targets

```
make setup      Install all prerequisites
make doctor     Verify dev environment is ready
make ui         Run VibeUI in dev mode (Vite + Tauri)
make cli        Build VibeCLI release binary
make test       Run all workspace tests
make test-fast  Run tests (excluding collab crate)
make check      Fast type-check (Rust + TypeScript)
make lint       Run clippy + TypeScript check
make build      Build everything for production
make clean      Remove build artifacts
make docker     Build Docker image
```

---

## Workspace Structure

```
vibecody/
├── Cargo.toml                  # Workspace root (9 members)
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
│       └── skills/             # 568 skill files (25+ categories)
├── vibeui/
│   ├── src/                    # React + TypeScript frontend
│   │   ├── App.tsx             # Root component
│   │   └── components/         # 187 panel components (163 backend-wired + 24 utilities)
│   ├── src-tauri/              # Tauri Rust backend
│   └── crates/                 # Shared Rust library crates
│       ├── vibe-core/          # Text buffer, FS, workspace, Git, index
│       ├── vibe-ai/            # 23 AI providers, agents, hooks, planner
│       ├── vibe-lsp/           # Language Server Protocol client
│       ├── vibe-extensions/    # WASM-based extension system
│       └── vibe-collab/        # CRDT multiplayer collaboration
├── vibeapp/                    # Secondary Tauri app
├── vibemobile/                 # Flutter mobile companion app
│   ├── lib/screens/            # 9 screens (home, chat, pair, machines, sessions, settings...)
│   ├── lib/services/           # API client, auth, notifications
│   └── lib/models/             # Machine/device models
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

Unified AI provider abstraction with agent loop, hooks, planner, multi-agent orchestration, skills, artifacts, admin policy, trace/session resume, and OpenTelemetry. Supports 23 providers:

- **Ollama** — Local/private models (default)
- **Anthropic Claude** — Claude 4 Sonnet/Opus
- **OpenAI** — GPT-4o and variants
- **Google Gemini** — Gemini 2.5 Pro/Flash
- **xAI Grok** — Grok 2
- **Groq** — Fast inference (Llama, Mixtral)
- **OpenRouter** — Multi-provider gateway
- **Azure OpenAI** — Enterprise Azure-hosted models
- **AWS Bedrock** — AWS-hosted models (Claude, Llama, Titan)
- **GitHub Copilot** — Copilot integration
- **LocalEdit** — Local code editing model
- **Mistral** — Mistral AI models
- **Cerebras** — Wafer-scale inference
- **DeepSeek** — DeepSeek V3/R1
- **Zhipu** — GLM-4 models
- **Vercel AI** — Vercel AI SDK
- **MiniMax** — MiniMax-Text-01
- **Perplexity** — Search-augmented Sonar models
- **Together AI** — Open model hosting (Llama, Qwen)
- **Fireworks AI** — Fast open model inference
- **SambaNova** — Hardware-accelerated inference
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

# See docs/configuration.md for all 23 providers

[safety]
require_approval_for_commands = true
require_approval_for_file_changes = true
```

---

## Mobile Companion App

VibeMobile (`vibemobile/`) is a Flutter app for remote management of VibeCody sessions from any device.

**Features:**
- QR code pairing with VibeCLI/VibeUI instances
- Remote chat with AI providers
- Machine management (register, monitor, heartbeat)
- Session browser and management
- Push notifications for agent task completion
- Dark/light theme with Material Design 3

**Platforms:** iOS, Android, macOS, Linux, Windows, Web

```bash
cd vibemobile
flutter pub get
flutter run            # Run on connected device
flutter run -d chrome  # Run in browser
```

**Requirements:** Flutter SDK >=3.2.0, Dart >=3.2.0

---

## IDE Plugins

| Plugin | Path | Status |
|--------|------|--------|
| **VS Code** | `vscode-extension/` | Extension with inline chat, code actions, sidebar panel |
| **JetBrains** | `jetbrains-plugin/` | IntelliJ/WebStorm/PyCharm plugin with agent integration |
| **Neovim** | `neovim-plugin/` | Lua plugin with Telescope integration |

---

## Prerequisites

`make setup` installs everything automatically. If you prefer manual setup:

| Requirement | Version | Install |
|-------------|---------|---------|
| Rust | stable | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` |
| Node.js | >= 20 | [nodejs.org](https://nodejs.org/) or `nvm install 20` |
| Git | any | Usually pre-installed |
| Ollama | any | Optional — [ollama.ai](https://ollama.ai/) for local AI |
| Docker | any | Optional — for container sandbox |

**Linux only** (Tauri system dependencies):
```bash
# Debian/Ubuntu
sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev patchelf build-essential libssl-dev pkg-config

# Fedora
sudo dnf install webkit2gtk4.1-devel gtk3-devel libappindicator-gtk3-devel librsvg2-devel patchelf openssl-devel

# Arch
sudo pacman -S webkit2gtk-4.1 gtk3 libappindicator-gtk3 librsvg patchelf openssl base-devel
```

**macOS only**: Xcode command line tools (`xcode-select --install`)

---

## Running Tests

**7,300+ unit tests** across the workspace.

```bash
make test          # All workspace tests
make test-fast     # Skip collab crate (faster)
make check         # Type-check only (Rust + TypeScript)

# Specific crates
cargo test -p vibe-core
cargo test -p vibe-ai
cargo test -p vibecli
```

---

## Troubleshooting

| Problem | Fix |
|---------|-----|
| `rustup could not choose a version of cargo` | Run `rustup default stable` |
| `npm run tauri dev` can't find cargo (Linux) | Use `make ui` or `npm run tauri:dev` — these prepend `~/.cargo/bin` to PATH |
| Port 1420 already in use | Kill stale Vite: `lsof -i :1420` then `kill <pid>` |
| `"VibeUI" is damaged` (macOS) | Run `xattr -cr /Applications/VibeUI.app` (unsigned app — Gatekeeper quarantine) |
| Missing `libwebkit2gtk-4.1-dev` (Linux) | Run `make setup` or install manually (see Prerequisites) |
| `Failed to run cargo: No such file` (macOS .app) | Fixed in v0.3.0 — app now inherits shell PATH at startup |

---

## Documentation

Full documentation is available at the [GitHub Pages site](https://vibecody.github.io/vibecody/) *(replace with actual URL)*.

- [Architecture Overview](./docs/architecture.md)
- [VibeCLI Reference](./docs/vibecli.md)
- [VibeUI Reference](./docs/vibeui.md)
- [Roadmap](./docs/ROADMAP.md)
- [Roadmap v2 (Phases 6–9)](./docs/ROADMAP-v2.md)
- [FIT-GAP Analysis v7](docs/FIT-GAP-ANALYSIS-v7.md) — Competitive analysis (35+ tools)
- [Roadmap v5](docs/ROADMAP-v5.md) — Implementation phases 23-31
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
