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
| `--exec <task>` | — | Run an agent task non-interactively (CI mode) |
| `--auto-edit` | false | Auto-apply file edits; prompt for bash commands |
| `--full-auto` | false | Auto-execute everything (use with `--sandbox` in CI) |
| `--output-format <fmt>` | `json` | Report format for `--exec`: `json`, `markdown`, `verbose` |
| `--output <file>` | stdout | Write `--exec` report to a file instead of stdout |
| `--resume <id>` | — | Resume a previous agent session by trace ID |
| `--parallel <n>` | — | Run task across N parallel agents on git worktrees |
| `--sandbox` | false | Enable OS-level sandbox (sandbox-exec/bwrap) |
| `--review` | false | Run code review agent on current diff |

---

## TUI Commands

Once inside the TUI, type messages naturally or use slash commands:

| Command | Description |
|---------|-------------|
| `/chat <message>` | Start or continue a conversation with AI |
| `/diff` | Show full multi-file git diff of current workspace |
| `/diff <file>` | Show git diff for a specific file |
| `/files` | Browse workspace file tree |
| `/quit` or `/exit` | Exit VibeCLI |
| `Tab` | Toggle between Chat and Diff views |

---

## REPL Commands

In REPL mode, the following slash commands are available:

| Command | Description |
|---------|-------------|
| `/chat <message>` | Chat with the AI (maintains conversation history) |
| `/chat [image.png] <message>` | Chat with a vision model and attach an image |
| `/generate <prompt>` | Generate code from a natural language description |
| `/agent <task>` | Run the autonomous agent loop for a multi-step task |
| `/diff <file>` | Show the git diff for a file |
| `/apply <file> <changes>` | Apply AI-generated changes to a file (with diff preview) |
| `/exec <task>` | Generate a shell command from a description and optionally run it |
| `/trace` | List recent agent session traces |
| `/trace view <id>` | Show detailed timeline for a trace session |
| `/plan <task>` | Generate an execution plan without acting |
| `/review` | Run code review agent on current diff |
| `/review --pr <n>` | Review a GitHub PR and post comments |
| `/resume <id>` | Resume a previous agent session |
| `/skills` | List available skills and their triggers |
| `/mcp list` | List configured MCP servers |
| `/mcp tools <server>` | List tools provided by an MCP server |
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

## CI / Non-Interactive Mode

VibeCLI can run agent tasks headlessly — no prompts, no TUI:

```bash
# Run an agent task and get a JSON report on stdout
vibecli --exec "add error handling to src/lib.rs" --full-auto

# Write a Markdown report to a file
vibecli --exec "fix all clippy warnings" --full-auto \
        --output-format markdown --output report.md

# Stream progress to stderr while writing JSON to stdout
vibecli --exec "add docstrings to all public functions" \
        --auto-edit --output-format verbose
```

**Exit codes:** `0` = success, `1` = partial, `2` = failed, `3` = approval required.

---

## Multimodal Input (Vision)

Claude and GPT-4o providers support image attachments. Use `[path/to/image.png]` syntax in `/chat`:

```
> /chat [screenshot.png] what error is shown in this screenshot?
> /chat [diagram.jpg] [schema.png] explain this database design
```

Images are base64-encoded and sent with the message. Non-vision providers fall back to text-only.

---

## Trace / Audit Log

Every agent session is recorded to `~/.vibecli/traces/<timestamp>.jsonl`. Browse with:

```
> /trace                   # list recent sessions
> /trace view 1740000000   # show detailed timeline
```

Each entry records: step, tool, input summary, output, duration, and approval source (`user` / `auto` / `ci-auto` / `rejected`).

---

## MCP Integration

[Model Context Protocol](https://modelcontextprotocol.io/) servers expose additional tools to the agent. Configure in `~/.vibecli/config.toml`:

```toml
[[mcp_servers]]
name = "github"
command = "npx"
args = ["@modelcontextprotocol/server-github"]

[[mcp_servers]]
name = "postgres"
command = "npx"
args = ["@modelcontextprotocol/server-postgres", "postgresql://localhost/mydb"]
```

Then in the REPL:

```
> /mcp list                # show configured servers
> /mcp tools github        # list tools from the github server
```

---

## Code Review Agent

Run structured code reviews from the CLI:

```bash
vibecli review                       # review uncommitted changes
vibecli review --staged               # review staged changes only
vibecli review --branch main..HEAD    # review branch diff
vibecli review --pr 42                # review a GitHub PR
vibecli review --focus security,perf  # limit review focus
```

Output is a structured `ReviewReport` with issues (severity: info/warning/critical), suggestions, and a numeric score.

Use `--pr` to post the review directly to a GitHub PR as a comment (via `gh` CLI).

---

## Server Mode (`vibecli serve`)

Run VibeCLI as a long-lived HTTP daemon for the VS Code extension and Agent SDK:

```bash
vibecli serve --port 7878
```

Endpoints:

| Method | Path | Description |
|--------|------|-------------|
| GET | `/health` | Health check |
| POST | `/chat` | Send chat messages, get response |
| POST | `/chat/stream` | Streaming SSE chat |
| POST | `/agent/start` | Start an agent task |
| GET | `/agent/:id/stream` | Stream agent events via SSE |

---

## Hooks System

Hooks execute on agent events. Configure in `~/.vibecli/hooks.toml` or `.vibecli/hooks.toml`:

```toml
[[hooks]]
event = "PreToolUse"
pattern = "bash"
handler = { type = "command", command = "echo 'Running shell command' >> /tmp/hooks.log" }
```

Events: `SessionStart`, `PreToolUse`, `PostToolUse`, `Stop`, `TaskCompleted`, `SubagentStart`.

---

## Plan Mode

Generate execution plans without running tools:

```
> /plan refactor the auth module to use JWT tokens

📝 Execution Plan:
  1. Read current auth module (auth.rs)
  2. Add JWT dependency to Cargo.toml
  3. Implement JWT token generation
  4. Update auth middleware
  5. Write tests

✅ Execute this plan? (y/N):
```

---

## Skills System

Skills are context snippets that activate based on trigger keywords. Place `.md` files in `.vibecli/skills/` or `~/.vibecli/skills/`:

```markdown
---
name: rust-testing
triggers: [test, testing, cargo test]
---
Use `#[tokio::test]` for async tests...
```

---

## Session Resume

Resume a previous agent session:

```bash
vibecli --resume 1740000000
```

Restores the full message history, context, and trace from the JSONL log.

---

## Admin Policy

Workspace administrators can restrict agent behavior via `.vibecli/policy.toml`:

```toml
[tools]
deny = ["bash"]
require_approval = ["write_file", "patch_file"]

[paths]
deny = ["*.env", "secrets/**"]

[limits]
max_steps = 10
```

---

## OpenTelemetry

Export agent tracing spans to any OTLP collector:

```toml
[otel]
enabled = true
endpoint = "http://localhost:4318"
service_name = "vibecli"
```

Spans include session ID, task, tool name, and step metadata.

## Project Structure

```
vibecli/
└── vibecli-cli/
    └── src/
        ├── main.rs         # CLI argument parsing, command dispatch
        ├── config.rs       # Config loading/saving (TOML)
        ├── ci.rs           # Non-interactive CI mode (--exec)
        ├── review.rs       # Code review agent (vibecli review)
        ├── serve.rs        # HTTP daemon (vibecli serve)
        ├── otel_init.rs    # OpenTelemetry pipeline setup
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
| `axum` + `tower-http` | HTTP server (serve mode) |
| `opentelemetry` + `tracing-opentelemetry` | OTLP tracing |
| `vibe-ai` | AI provider, agent, hooks, skills, artifacts |
| `vibe-core` | Git, diff, file utilities, codebase indexing |
