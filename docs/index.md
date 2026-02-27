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

---

## Navigation

| Page | Description |
|------|-------------|
| [VibeCLI Reference](./vibecli/) | CLI commands, TUI usage, and configuration |
| [VibeUI Reference](./vibeui/) | Desktop editor features and setup |
| [Configuration Guide](./configuration/) | All configuration options for providers and UI |
| [Architecture](./architecture/) | Crate structure, data flow, and design decisions |
| [Contributing](./contributing/) | How to build, test, and contribute |

---

## Key Features

### Multi-Provider AI

Both VibeCLI and VibeUI share the `vibe-ai` provider abstraction, supporting:

| Provider | Type | Streaming | Notes |
|----------|------|-----------|-------|
| Ollama | Local | Yes | Default, no API key needed |
| Anthropic Claude | Cloud | Yes | Claude 3.5 Sonnet/Opus |
| OpenAI | Cloud | Yes | GPT-4o, GPT-4-turbo |
| Google Gemini | Cloud | Yes | Gemini 1.5 Pro |
| xAI Grok | Cloud | Yes | Grok Beta |

### VibeCLI Highlights

- Rich TUI powered by [Ratatui](https://ratatui.rs/)
- REPL mode with readline history and tab completion
- Git-aware context injection (branch, status, diff)
- Multi-file diff view with syntax highlighting
- AI-assisted code apply with interactive confirmation
- Direct shell command execution with approval gate
- Code Complete workflow: 8-stage development pipeline with AI-generated checklists
- Built-in test runner with auto-detection of Cargo, npm, pytest, and Go

### VibeUI Highlights

- Monaco Editor integration (same engine as VS Code)
- Rope-based text buffer for large-file performance
- Async file I/O with file-watching
- Full Git panel (status, diff, commit, branch)
- Integrated terminal (PTY)
- LSP client foundation
- WASM extension system (Wasmtime)
- Dark / light theme toggle
- Test runner panel with live log streaming and pass/fail visualization

---

## Getting Started in 60 Seconds

```bash
# Clone
git clone https://github.com/vibecody/vibecody.git
cd vibecody

# Build VibeCLI
cargo build --release -p vibecli

# Launch the TUI (requires Ollama running locally)
./target/release/vibecli --tui

# Or launch VibeUI
cd vibeui && npm install && npm run tauri dev
```

See the [Configuration Guide](./configuration/) to set up cloud providers.
