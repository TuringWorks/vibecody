---
layout: page
title: VibeCLI Reference
permalink: /vibecli/
---

# VibeCLI

**AI-powered coding assistant for the terminal.**

VibeCLI provides two interaction modes: a rich **Terminal UI (TUI)** powered by Ratatui, and a **REPL** mode for quick, scriptable use.

---

## Installation

### Prerequisites

- Rust stable (≥ 1.75) — install via [rustup](https://rustup.rs/)
- `git` (for Git integration features)
- Ollama (optional, for local AI — [ollama.ai](https://ollama.ai))

### Build from Source

```bash
git clone https://github.com/vibecody/vibecody.git
cd vibecody

cargo build --release -p vibecli

# Optional: install to PATH
cp target/release/vibecli /usr/local/bin/
```

---

## Usage

### TUI Mode (Recommended)

```bash
vibecli --tui
vibecli --tui --provider claude
vibecli --tui --provider openai --model gpt-4o
```

### REPL Mode

```bash
vibecli
vibecli --provider gemini
```

### Command-Line Arguments

| Flag | Default | Description |
|------|---------|-------------|
| `--provider <name>` | `ollama` | AI provider: `ollama`, `openai`, `claude`, `gemini`, `grok` |
| `--model <name>` | provider default | Override the model for the selected provider |
| `--tui` | false | Launch the Terminal UI instead of REPL |

---

## TUI Commands

Once inside the TUI, type messages naturally or use slash commands:

| Command | Description |
|---------|-------------|
| `/chat <message>` | Start or continue a conversation with AI |
| `/diff` | Show full multi-file git diff of current workspace |
| `/diff <file>` | Show git diff for a specific file |
| `/files` | Browse file tree (coming soon) |
| `/quit` or `/exit` | Exit VibeCLI |
| `Tab` | Toggle between Chat and Diff views |

---

## REPL Commands

In REPL mode, the following slash commands are available:

| Command | Description |
|---------|-------------|
| `/chat <message>` | Chat with the AI (maintains conversation history) |
| `/generate <prompt>` | Generate code from a natural language description |
| `/diff <file>` | Show the git diff for a file |
| `/apply <file> <changes>` | Apply AI-generated changes to a file (with diff preview) |
| `/exec <task>` | Generate a shell command from a description and optionally run it |
| `/config` | Display current configuration |
| `/help` | Show command reference |
| `/exit` or `/quit` | Exit VibeCLI |
| `! <command>` | Execute a shell command directly (e.g. `!ls -la`) |

> **Safety**: By default, all shell command execution requires user confirmation (`y/N`). Disable this with `require_approval_for_commands = false` in config.

---

## Workflow Examples

### Chat with AI

```
> /chat explain how the ropey crate works
> What are the time complexities for common rope operations?
```

Once a conversation is started, you can type freely without `/chat`.

### Generate Code

```
> /generate a Rust function that parses TOML from a string and returns a HashMap
💾 Save to file? (y/N or filename): parser.rs
✅ Saved to: parser.rs
```

### Apply AI Changes to a File

```
> /apply src/main.rs add proper error handling using anyhow

📊 Proposed changes:
--- a/src/main.rs
+++ b/src/main.rs
...

✅ Apply these changes? (y/N): y
✅ Changes applied to: src/main.rs
```

### AI-Suggested Command

```
> /exec list all Rust files modified in the last 7 days
📝 Suggested command: find . -name "*.rs" -mtime -7
⚠️  Execute this command? (y/N): y
```

---

## Git Context Awareness

VibeCLI automatically injects Git context into AI conversations:

- **Current branch** — detected from the working directory
- **Modified/staged files** — summarized from `git status`
- **Current diff** — full patch injected as system context

This gives the AI complete awareness of what you are working on without any extra prompting.

---

## Syntax Highlighting

Code blocks in AI responses are highlighted in the terminal using `syntect`. Supported languages include Rust, Python, TypeScript, JavaScript, Go, TOML, YAML, JSON, Markdown, and more.

---

## Configuration

VibeCLI reads from `~/.vibecli/config.toml`. See the [Configuration Guide](../configuration/) for full details.

**Minimal working config (Ollama):**

```toml
[ollama]
enabled = true
api_url = "http://localhost:11434"
model = "qwen2.5-coder:7b"
```

---

## Project Structure

```
vibecli/
└── vibecli-cli/
    └── src/
        ├── main.rs         # CLI argument parsing, command dispatch
        ├── config.rs       # Config loading/saving (TOML)
        ├── diff_viewer.rs  # Renders unified diffs in terminal
        ├── syntax.rs       # Syntax highlighting for code blocks
        ├── repl.rs         # Rustyline helper (tab completion, hints)
        └── tui/
            ├── mod.rs      # TUI run loop and event handling
            ├── app.rs      # TUI application state machine
            ├── ui.rs       # Ratatui layout and widget rendering
            └── components/
                ├── chat.rs       # Chat message list widget
                └── diff_view.rs  # Multi-file diff viewer widget
```

---

## Dependencies

| Crate | Purpose |
|-------|---------|
| `clap` | CLI argument parsing |
| `ratatui` + `crossterm` | Terminal UI framework |
| `rustyline` | REPL readline with history |
| `syntect` | Syntax highlighting |
| `tokio` | Async runtime |
| `vibe-ai` | AI provider abstraction |
| `vibe-core` | Git, diff, and file utilities |
