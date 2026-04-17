---
layout: home
title: VibeCody Documentation
---

# VibeCody

**AI-powered developer toolchain built in Rust.**

VibeCody is a monorepo that provides a full family of AI-assisted development surfaces:

- **[VibeCLI](./vibecli/)** — terminal-first AI coding assistant with a rich TUI + REPL
- **[VibeUI](./vibeui/)** — AI-powered desktop code editor (Tauri + Monaco, 293+ panels)
- **[VibeCLI App](./release/)** — dedicated desktop chat companion (Tauri)
- **[VibeMobile](./vibemobile/)** — Flutter companion for iOS, Android, macOS, Linux, Windows, Web
- **[VibeWatch](./watch-integration/)** — native Apple Watch (SwiftUI, watchOS 10+) and Wear OS (Compose, Wear OS 3+) clients
- **[Zero-config connectivity](./connectivity/)** — mDNS LAN, Tailscale Funnel, ngrok auto-detect; devices race all reachable paths

All surfaces are backed by a shared set of Rust crates for AI providers, text editing, Git, LSP, and extensions.


## Navigation

| Page | Description |
|------|-------------|
| [Easy Setup](./setup/) | Deploy VibeCody anywhere — cloud, desktop, or Raspberry Pi |
| [Use Cases](./use-cases/) | 80+ things you can do with VibeCody |
| [Deployment Guides](./guides/) | Step-by-step guides for 12 platforms (AWS, GCP, Azure, Pi, etc.) |
| [Quickstart](./quickstart/) | Zero to productive in 5 minutes |
| [Tutorials](./tutorials/) | Step-by-step guides for common workflows |
| [VibeCLI Reference](./vibecli/) | CLI commands, TUI usage, and configuration |
| [VibeUI Reference](./vibeui/) | Desktop editor features and setup |
| [Design System](./design-system/) | Token-based UI system — colors, spacing, typography, components |
| [Configuration Guide](./configuration/) | All configuration options for providers and UI |
| [Memory Guide](./memory-guide/) | All memory layers — auto-recording, cognitive store, verbatim drawers, benchmarking |
| [Architecture](./architecture/) | Crate structure, data flow, and design decisions |
| [Roadmap](./roadmap/) | Competitive landscape, phase-level history (1–39), and positioning |
| [Plugin Development](./plugin-development/) | Build plugins, skills, hooks, WASM extensions, and MCP integrations |
| [VibeMobile](vibemobile/) | Mobile companion (Flutter) — pairing, Handoff, remote chat, sync |
| [VibeWatch — watchOS](watchos/) | Apple Watch native client — pair, view transcripts, dictate reply |
| [VibeWatch — Wear OS](wearos/) | Wear OS native client — same feature set, Keystore/StrongBox attestation |
| [Connectivity](connectivity/) | mDNS, Tailscale Funnel, ngrok — zero-config device discovery |
| [Watch Integration](watch-integration/) | Full architecture: P-256 pairing, `/watch/*` routes, sync model |
| [Competitive Analysis](./shannon-comparison/) | Feature comparison with other AI coding tools |
| [Fit-Gap Analysis](fit-gap-analysis/) | Consolidated gap catalogue — 142 gaps tracked across 8 iterations and 5 deep-dives |
| [Whitepapers](./whitepapers/) | In-depth comparisons: VibeCody vs OpenClaw, PicoClaw, NemoClaw, and 12+ alternatives |
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
- 556 skill files across 25+ categories (106+ REPL commands)
- Voice input via Groq Whisper (`--voice` flag)
- Tailscale pairing with QR code sharing and mDNS discovery
- 18 gateway platforms (Telegram, Discord, Slack, Signal, Matrix, Teams, IRC, Twitch, and more)
- Red team security pipeline and compliance reporting
- Workflow orchestration with 8-stage Code Complete pipeline
- MCP (Model Context Protocol) server — 51 tools including email, calendar, tasks, Notion, Jira, Home Assistant
- Productivity integrations: Gmail/Outlook, Google/Outlook Calendar, Todoist, Notion, Jira, Home Assistant
- 12-platform deployment: AWS, GCP, Azure, Oracle Cloud, DigitalOcean, Linode, macOS, Linux, Windows, Raspberry Pi 3/4/5
- Session persistence with `/sessions` and `/resume` commands
- Container sandbox (Docker, Podman, OpenSandbox) with unified runtime trait
- Git-aware context injection, multi-file diff view with syntax highlighting
- AI-assisted code apply with interactive confirmation and approval gate
- HTTP daemon mode (`vibecli --serve --port 7878`)

### VibeUI Highlights

- Monaco Editor integration (same engine as VS Code)
- 196+ AI panel tabs (Chat, Agent, Counsel, Tests, Docker, K8s, Profiler, Design Canvas, and many more)
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
