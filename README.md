# VibeCody

**VibeCody** is an AI-powered developer toolchain built entirely in Rust. It combines a terminal-first CLI coding assistant (**VibeCLI**) with a full-featured desktop code editor (**VibeCoder**), both powered by a shared library of AI and editor primitives.

> **Want to use VibeCody?** [Download a release.](#install) You do not need
> this repository, Rust, or Node.js.
>
> **Want to change VibeCody?** Then clone it —
> [building from source](#for-developers-building-from-source) is for
> development, and the rest of this README is written for that.

---

## Install

Every release is built by CI and published with SHA-256 checksums. Nothing here
needs a toolchain.

### Desktop — VibeCoder, VibeDesk, VibeAIChat

Take the file for your platform from the
[**latest release**](https://github.com/TuringWorks/vibecody/releases/latest):

| Platform | File |
|---|---|
| macOS (Apple Silicon) | `<App>_<version>_aarch64.dmg` |
| macOS (Intel) | `<App>_<version>_x64.dmg` |
| Windows | `<App>_<version>_x64-setup.exe`, or the `_x64_en-US.msi` |
| Linux (x86_64) | `<App>_<version>_amd64.AppImage` (portable) or `_amd64.deb` |
| Linux (arm64) | `<App>_<version>_arm64.deb` |

Each app starts the VibeCLI daemon itself and reuses one that is already
running, so installing more than one shell is fine and there is nothing else to
set up.

### Terminal — VibeCLI

```bash
curl -fsSL https://raw.githubusercontent.com/TuringWorks/vibecody/main/install.sh | sh
```

macOS and Linux, x86_64 and ARM. It resolves the latest release, checks the
download against that release's `SHA256SUMS.txt`, and installs to
`~/.local/bin/vibecli` — set `INSTALL_DIR` to put it somewhere else. A mismatch
aborts; a sums file it cannot fetch is a warning and the install continues, so
verify by hand if that matters to you. On Windows, or to read the script before
running it, take the `vibecli-*` archive from the release page directly.

```bash
vibecli --version
```

### Phone and watch

`.apk` / `.aab`, `.ipa`, watchOS `.app.zip` and Wear OS builds ship on the same
release. **iOS and watchOS builds are unsigned** — sideload via AltStore,
Sideloadly or Xcode. macOS builds are Developer ID signed and notarized when the
release carries the signing credentials; the release notes say when they do not.

Full walkthrough, including what each surface is for:
[Quickstart](https://turingworks.github.io/vibecody/quickstart/) ·
[every published artifact](https://turingworks.github.io/vibecody/release/) ·
[hardware requirements](https://turingworks.github.io/vibecody/sizing/).

---

## Projects

| Project | Description | Status |
|---------|-------------|--------|
| [VibeCLI](./vibecli/) | AI coding assistant for the terminal (TUI + REPL + `--serve` daemon) | Active |
| [VibeCoder](./vibecoder/) | AI-powered desktop code editor (Tauri + Monaco) | Active |
| [VibeDesk](./vibedesk/) | Task-first, conversation-driven desktop companion (Tauri + React) — type a task, watch it happen | Active |
| [VibeAIChat](./vibeaichat/) | Secondary Tauri shell | Active |
| [VibeMobile](./vibemobile/) | Mobile companion app (Flutter — iOS, Android, macOS, Linux, Windows, Web) | Active |
| [VibeWatch](./vibewatch/) | Apple Watch (SwiftUI, watchOS 10+) + Wear OS (Kotlin/Compose) clients with companion relays | Active |

---

## For developers: building from source

**This is not how to install VibeCody** — for that, see [Install](#install)
above. Build from source when you intend to change the code: a source build is
unsigned, uses your local toolchain, and is not the artifact the release
publishes. Everything from here on assumes that is what you are doing.

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

### Run VibeCoder (Desktop Editor)

```bash
make ui
```

### Run VibeDesk (Task-First Companion)

```bash
make vibedesk               # needs the VibeCLI daemon running (`vibecli --serve`)
```

### Build & Run VibeCLI (Terminal AI Assistant)

```bash
make cli                    # Build release binary
./target/release/vibecli --tui   # Run with TUI

# Or with a specific provider
./target/release/vibecli --tui --provider claude --model claude-opus-5
```

### Verify Your Environment

```bash
make doctor    # Checks all required + optional tools
```

### All Make Targets

Run `make help` for the full list, or `make help-surfaces` for the matrix below.
Every surface has a consistent `build-<surface>` / `test-<surface>` pair:

```bash
# Surface            Dev            Build                Test
# ─────────────────  ─────────────  ───────────────────  ──────────────────
# VibeCLI (Rust)     make cli-run   make build-cli       make test-cli
# VibeCoder  (Tauri) make ui        make build-ui        make test-ui
# VibeAIChat (Tauri) make aichat    make build-aichat    make test-aichat
# VibeDesk   (Tauri) make vibedesk  make build-vibedesk  make test-vibedesk
# Agent SDK (TS)     —              make build-sdk       make test-sdk
# vibe-indexer       —              make build-indexer   make test-indexer
# vibe-memory        —              make build-memory    make test-memory
# vibe-rl-py (uv)    —              make build-rl        make test-rl
# VS Code ext        —              make build-vscode    make lint-vscode
# JetBrains plugin   —              make build-jetbrains make test-jetbrains
# Mobile (Flutter)   —              make build-mobile    make test-mobile
# Watch (iOS/Wear)   —              make build-watch     make test-watch
```

```bash
# Setup & environment
make setup              Install all prerequisites
make doctor             Verify dev environment is ready

# Aggregates
make build              Build all desktop shells (CLI + UI + App + VibeDesk)
make build-apps         Build the three Tauri shells (UI + App + VibeDesk)
make build-all          Rust + Tauri + Mobile + Watch (what CI builds)
make test               Run all Rust workspace tests (fast path)
make test-fast          Run Rust tests (excluding collab crate)
make test-all           Test every Rust + Node surface
make ci                 Mirror the GitHub CI gate locally
make check              Fast type-check (Rust + UI/App/VibeDesk TypeScript)
make lint               Run clippy + UI TypeScript check
make eval-check         Validate the eval suites (no agent, no provider)
make eval-offline       Run the capability evals (needs a provider)
make clean              Remove build artifacts
make docker             Build Docker image
make icons              Regenerate every app icon from the shared brand mark
make icons-check        Fail if a committed icon is stale (see assets/brand/)

# Voice (speech synthesis — macOS)
make voice-sidecar      Build + install the streaming speech sidecar
make voice-kokoro       Install the neural (Kokoro) engine — Apple Silicon
make voice-status       Report which engine the daemon will actually use

# Mobile (Flutter — iOS / Android)
make mobile-setup       flutter pub get
make mobile-ios         Build iOS .app (unsigned, simulator-friendly)
make mobile-ios-ipa     Archive + export signed iOS .ipa (needs APPLE_TEAM_ID)
make mobile-android     Release APK
make mobile-android-bundle   Release AAB for Play Store
make build-mobile       Everything above (platform-gated)
make test-mobile        flutter test    ·    make analyze-mobile   dart analyze

# Watch (Xcode + Gradle)
make watch-ios          Build watchOS app for Simulator (Xcode)
make watch-ios-archive  Archive watchOS app for a real device
make watch-wear         Wear OS release APK
make watch-wear-bundle  Wear OS release AAB
make build-watch        Everything above (platform-gated)
make test-watch         Wear OS unit tests (gradle test)
```

---

## Workspace Structure

```txt
vibecody/
├── Cargo.toml                  # Workspace root (36 members)
├── Dockerfile                  # Multi-stage musl build (Alpine runtime)
├── docker-compose.yml          # VibeCLI + Ollama sidecar (air-gapped)
├── install.sh                  # One-liner installer (SHA-256 verified)
├── vibecli/
│   └── vibecli-cli/            # CLI binary (TUI + REPL + HTTP daemon)
│       ├── src/                # 436 modules
│       │   ├── main.rs         # Entry point, command routing
│       │   ├── config.rs       # TOML config (~/.vibecli/config.toml)
│       │   ├── serve.rs        # HTTP daemon for VS Code ext / SDK / mobile / watch
│       │   ├── repl.rs         # Rustyline REPL helper
│       │   ├── watch_auth.rs   # P256 ECDSA device registration + JWT lifecycle
│       │   ├── watch_bridge.rs # Axum /watch/* routes (SSE streaming)
│       │   ├── watch_session_relay.rs  # OLED-optimised payloads
│       │   ├── mdns_announce.rs / tailscale.rs / ngrok.rs  # zero-config connectivity
│       │   ├── pairing.rs      # one-time pairing URL + QR rendering
│       │   └── tui/            # Ratatui TUI (app, ui, components)
│       ├── tests/              # 89 BDD / integration harnesses
│       └── skills/             # 1,144 skill files (155 categories)
├── vibecoder/
│   ├── src/                    # React + TypeScript frontend
│   │   ├── App.tsx             # Root component
│   │   └── components/         # 246 *Panel.tsx + 41 composite dashboards (303 top-level components)
│   ├── src-tauri/              # Tauri Rust backend (1,367 commands)
│   └── crates/                 # Shared Rust library crates
│       ├── vibe-core/          # Text buffer, FS, workspace, Git, index
│       ├── vibe-ai/            # 23 provider backends + failover + openai_compat; agents, hooks, planner
│       ├── vibe-lsp/           # Language Server Protocol client
│       ├── vibe-extensions/    # WASM-based extension system
│       └── vibe-collab/        # CRDT multiplayer collaboration
├── vibedesk/                   # Task-first desktop companion (Tauri + React, dev :1422)
│   ├── src/                    # Three-column shell: project nav · conversation · Environment
│   └── src-tauri/              # Thin daemon bridge — no agent logic of its own
├── vibeaichat/                 # Secondary Tauri shell
├── vibemobile/                 # Flutter mobile companion app
│   ├── lib/screens/            # 11 screens (home, chat, pair, machines, sessions, sandbox, watch…)
│   ├── lib/services/           # api_client, auth, discovery, handoff, notifications, watch_sync
│   └── lib/models/             # Machine / device models
├── vibewatch/                  # Watch clients
│   ├── VibeCodyWatch Watch App/        # SwiftUI (watchOS 10+, Secure Enclave P-256)
│   ├── VibeCodyWatchCompanion/         # iOS WatchConnectivity bridge
│   ├── VibeCodyWear/                   # Kotlin/Compose (Wear OS 3+, Android Keystore)
│   └── VibeCodyWearCompanion/          # Android Wearable Data Layer bridge
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

The `vibecoder/crates/` libraries are designed to be reused across both VibeCLI and VibeCoder:

### `vibe-core`

Core editor primitives — text buffer (rope-based), file system operations, workspace management, Git integration, terminal PTY, diff engine, code search, and embedding-based codebase indexing.

### `vibe-ai`

Unified AI provider abstraction with agent loop, hooks, planner, multi-agent orchestration, skills, artifacts, admin policy, trace/session resume, and OpenTelemetry.

**25 providers are selectable in the UI.** The canonical list is `vibecoder/src/hooks/useModelRegistry.ts` — `STATIC_MODELS` and `PROVIDER_DEFAULT_MODEL` must stay in sync, and nothing else needs to change to add one. The crate underneath (`vibe-ai/src/providers/`) holds 23 concrete backends plus a failover meta-provider and two shared `*compat` helper modules.

- **Ollama** — local and Ollama Cloud models (no API key for local pulls)
- **vibecli-mistralrs** — in-process local inference, no server; weights cached under `~/.cache/huggingface/hub`
- **vLLM** — self-hosted OpenAI-compatible endpoint
- **LM Studio** — local desktop model server
- **Anthropic Claude** — Claude Opus 5, Fable 5, Sonnet 5, Opus 4.x, Haiku 4.5
- **Claude Code** — routes through the local Claude Code CLI (Free/Pro/Max/Team/Enterprise plans, no API credits)
- **OpenAI** — GPT-5.6 Sol/Terra/Luna, GPT-5.5, GPT-5.3-Codex, GPT-4.1, GPT-4o
- **Google Gemini** — Gemini 3.6 Flash, 3.5 Flash / Flash-Lite
- **xAI Grok** — Grok 4.5, 4.3, 4.20
- **Groq** — fast inference (gpt-oss, Qwen)
- **OpenRouter** — multi-provider gateway (Kimi K3, and 300+ models)
- **Azure OpenAI** — enterprise Azure-hosted models
- **AWS Bedrock** — AWS-hosted Claude, Llama, Titan
- **GitHub Copilot** — Copilot integration
- **Mistral** — Mistral Large / Medium / Small
- **Cerebras** — wafer-scale inference (gpt-oss-120b, Gemma 4, GLM 4.7)
- **DeepSeek** — DeepSeek V4 Pro / Flash
- **Zhipu** — GLM-5.2 / 5.1 / 5
- **Vercel AI** — Vercel AI SDK gateway
- **MiniMax** — MiniMax-M3 / M2.7
- **Perplexity** — search-augmented Sonar models
- **Together AI** — open model hosting (Kimi, Qwen)
- **Fireworks AI** — fast open model inference
- **SambaNova** — hardware-accelerated inference
- **Poolside** — Laguna models

Two more live in the crate but not in the model picker:

- **LocalEdit** — local code-editing model backend
- **Failover** — meta-provider that chains backends and retries the next on timeout, rate-limit, or error

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

> **Security note**: Do not put API keys in `config.toml`. Keys are stored encrypted in
> `~/.vibecli/profile_settings.db`. Use `vibecli secret set` or the VibeCoder Settings panel (⚙️ Keys tab) to manage them.

```toml
[ollama]
enabled = true
api_url = "http://localhost:11434"
model = "qwen2.5-coder:7b"

[claude]
enabled = false
model = "claude-opus-5"

[openai]
enabled = false
model = "gpt-5.6-sol"

[gemini]
enabled = false
model = "gemini-2.0-flash"

[grok]
enabled = false
model = "grok-4.6"

[groq]
enabled = false
model = "llama-3.3-70b-versatile"

[mistral]
enabled = false
model = "mistral-large-latest"

# See docs/configuration.md for all 25 providers

[safety]
require_approval_for_commands = true
require_approval_for_file_changes = true
```

---

## Mobile Companion App

VibeMobile (`vibemobile/`) is a Flutter app for remote management of VibeCody sessions from any device.

**Features:**

- QR code **and** manual URL/JSON pairing (works without a camera)
- Zero-config LAN discovery via mDNS — plus Tailscale and ngrok auto-detection for off-LAN access
- Apple-Handoff-style session continuity between desktop and phone
- Google-Docs-style bidirectional message sync (no truncation)
- Remote chat with any configured AI provider
- Machine management (register, monitor, heartbeat)
- Session browser and management
- Watch device browser + sandbox chat panel
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

See [docs/connectivity.md](./docs/connectivity.md) for the full mDNS / Tailscale / ngrok URL race design.

---

## Watch Companions

VibeCody extends to wrist-worn devices via two parallel native clients that share the same backend (`watch_auth.rs`, `watch_bridge.rs`, `watch_session_relay.rs`) exposed under `/watch/*`.

| Platform | Path | Stack | Key storage |
|----------|------|-------|-------------|
| **Apple Watch** | `vibewatch/VibeCodyWatch Watch App/` | Swift / SwiftUI, watchOS 10+ | Secure Enclave **P-256 ECDSA** via CryptoKit; Keychain for tokens |
| **Apple Watch companion** | `vibewatch/VibeCodyWatchCompanion/` | Swift, WatchConnectivity | Phone-side relay when watch is off-LAN |
| **Wear OS** | `vibewatch/VibeCodyWear/` | Kotlin / Jetpack Compose for Wear, Wear OS 3+ | Android Keystore (StrongBox) P-256; EncryptedSharedPreferences |
| **Wear OS companion** | `vibewatch/VibeCodyWearCompanion/` | Kotlin, Wearable Data Layer | Android phone relay service |

**Pairing** happens with a single URL (or Bearer token for emulators) — no JSON copy required. The Watch Devices panel in VibeCoder (`Governance → Watch Devices`) surfaces live device status, transport, and Secure Enclave / StrongBox attestation.

**Transports (priority order)**: Direct LAN → Tailscale mesh → phone-relay (WatchConnectivity / Wearable Data Layer).

See [docs/WATCH-INTEGRATION.md](./docs/WATCH-INTEGRATION.md) for the full architecture, auth flow, and TDD/BDD coverage.

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

**17,170 test functions + 89 BDD/integration harnesses** across the workspace.

> Counted at v0.5.11 by `#[test]` / `#[tokio::test]` attributes across `crates/`,
> `vibecli/`, `vibecoder/crates/` and `vibecoder/src-tauri/`, plus harness files in
> `vibecli/vibecli-cli/tests/`. A count is not a pass rate — run `make test` for that.

```bash
make test          # All Rust workspace tests (fast path)
make test-fast     # Skip collab crate (faster)
make test-all      # Every Rust + Node surface
make check         # Type-check only (Rust + UI/App/VibeDesk TypeScript)
make ci            # Mirror the GitHub CI gate locally

# Evaluations — how good VibeCody actually is (see evals/README.md)
make eval-check    # Validate the suites: no agent, no provider, CI-safe
make eval-surfaces # Transport conformance across all 14 clients (costs nothing)
make eval-offline  # Coding, agentic, work-task and safety suites

# Per-surface tests
make test-cli      # VibeCLI (Rust)      make test-ui     # VibeCoder  (vitest)
make test-aichat      # VibeAIChat (typecheck) make test-vibedesk  # VibeDesk   (typecheck + guard)
make test-sdk      # Agent SDK (vitest)  make test-mobile # Flutter
make test-indexer  # vibe-indexer        make test-memory # vibe-memory
make test-rl       # vibe-rl-py (pytest) make test-jetbrains  # JetBrains plugin
```

---

## Troubleshooting

| Problem | Fix |
|---------|-----|
| `rustup could not choose a version of cargo` | Run `rustup default stable` |
| `npm run tauri dev` can't find cargo (Linux) | Use `make ui` or `npm run tauri:dev` — these prepend `~/.cargo/bin` to PATH |
| Port 1420 already in use | Kill stale Vite: `lsof -i :1420` then `kill <pid>` |
| `"VibeCoder" is damaged` (macOS) | Run `xattr -cr /Applications/VibeCoder.app` (unsigned app — Gatekeeper quarantine) |
| Missing `libwebkit2gtk-4.1-dev` (Linux) | Run `make setup` or install manually (see Prerequisites) |
| `Failed to run cargo: No such file` (macOS .app) | Fixed in v0.3.0 — app now inherits shell PATH at startup |

---

## Documentation

Full documentation is available at the [GitHub Pages site](https://vibecody.github.io/vibecody/) *(replace with actual URL)*.

- [Architecture Overview](./docs/architecture.md)
- [VibeCLI Reference](./docs/vibecli.md)
- [VibeCoder Reference](./docs/vibecoder.md)
- [VibeDesk README](./vibedesk/README.md)
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
