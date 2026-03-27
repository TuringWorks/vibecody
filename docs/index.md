---
layout: home
title: VibeCody Documentation
---

# VibeCody

**AI-powered developer toolchain built in Rust.**

VibeCody is a monorepo that provides two complementary tools for AI-assisted development:

- **[VibeCLI](./vibecli/)** — A terminal-first AI coding assistant with a rich TUI and REPL
- **[VibeUI](./vibeui/)** — A full-featured AI-powered desktop code editor (Tauri + Monaco)

Both tools are backed by a shared set of Rust library crates for AI provider integration, text editing, Git operations, LSP, and extensions.


## Navigation

| Page | Description |
|------|-------------|
| [Quickstart](./quickstart/) | Zero to productive in 5 minutes |
| [Tutorials](./tutorials/) | Step-by-step guides for common workflows |
| [VibeCLI Reference](./vibecli/) | CLI commands, TUI usage, and configuration |
| [VibeUI Reference](./vibeui/) | Desktop editor features and setup |
| [Configuration Guide](./configuration/) | All configuration options for providers and UI |
| [Architecture](./architecture/) | Crate structure, data flow, and design decisions |
| [Roadmap v2](./roadmap-v2/) | Current roadmap and planned features |
| [Plugin Development](./plugin-development/) | Build plugins, skills, hooks, WASM extensions, and MCP integrations |
| [VibeMobile](vibemobile/) | Mobile companion app — Flutter, pairing, remote chat |
| [Competitive Analysis](./shannon-comparison/) | Feature comparison with other AI coding tools |
| [FIT-GAP v7](fit-gap-analysis-v7/) | Competitive analysis — 35+ agentic tools |
| [Roadmap v5](roadmap-v5/) | Implementation phases 23-31 |
| [Development Guide](./development/) | Build, test, debug, and code organization for contributors |
| [Security](./security/) | Security model, SSRF/path-traversal prevention, command blocklists |
| [Release Notes](./release/) | What's new in v0.4.0 — downloads, upgrade guide |
| [Contributing](./contributing/) | How to build, test, and contribute |


## Key Features

### Multi-Provider AI (23 Providers)

Both VibeCLI and VibeUI share the `vibe-ai` provider abstraction:

| Category | Providers |
|----------|-----------|
| **Local** | Ollama, LocalEdit |
| **Cloud** | Claude, OpenAI, Gemini, Grok, Groq, Mistral, Cerebras, DeepSeek, Zhipu, MiniMax |
| **Platform** | OpenRouter, Azure OpenAI, Bedrock, Copilot, Vercel AI |
| **Inference** | Perplexity, Together AI, Fireworks AI, SambaNova |
| **Meta** | Failover (automatic provider fallback) |

All providers support streaming. Local providers require no API key.

### VibeCLI Highlights

- Rich TUI powered by [Ratatui](https://ratatui.rs/) with REPL mode (readline history, tab completion)
- 543 skill files across 25+ categories (93 REPL commands)
- Voice input via Groq Whisper (`--voice` flag)
- Tailscale pairing with QR code sharing and mDNS discovery
- 18 gateway platforms (Telegram, Discord, Slack, Signal, Matrix, Teams, IRC, Twitch, and more)
- Red team security pipeline and compliance reporting
- Workflow orchestration with 8-stage Code Complete pipeline
- MCP (Model Context Protocol) integration
- Session persistence with `/sessions` and `/resume` commands
- Container sandbox (Docker, Podman, OpenSandbox) with unified runtime trait
- Git-aware context injection, multi-file diff view with syntax highlighting
- AI-assisted code apply with interactive confirmation and approval gate
- HTTP daemon mode (`vibecli serve --port 7878`)

### VibeUI Highlights

- Monaco Editor integration (same engine as VS Code)
- 184+ AI panel tabs (Chat, Agent, Counsel, Tests, Docker, K8s, Profiler, Design Canvas, and many more)
- CSS variable theming across all panels with dark/light toggle
- Multiplayer CRDT collaboration (real-time co-editing)
- Agent teams with inter-agent messaging bus
- CI review bot (GitHub App integration)
- Marketplace for sharing extensions and skills
- Visual testing via Chrome DevTools Protocol
- Container sandbox management (Docker, Podman, OpenSandbox)
- Deploy to 6 targets from the editor
- Rope-based text buffer, async file I/O with file-watching
- Full Git panel, integrated terminal (PTY), LSP client, WASM extension system


### VibeMobile (Flutter Companion)
- QR code pairing with VibeCLI/VibeUI instances
- Remote AI chat from iOS, Android, macOS, Linux, Windows, Web
- Machine management with health monitoring
- Session browser and push notifications


## Getting Started in 60 Seconds

```bash
# Clone
git clone https://github.com/TuringWorks/vibecody.git
cd vibecody

# Build VibeCLI
cargo build --release -p vibecli

# Launch the TUI (requires Ollama running locally)
./target/release/vibecli --tui

# Or launch VibeUI
cd vibeui && npm install && npm run tauri dev
```

See the [Configuration Guide](./configuration/) to set up cloud providers.
