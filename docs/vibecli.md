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
| `--provider <name>` | `ollama` | AI provider: `ollama`, `claude`, `openai`, `gemini`, `grok`, `groq`, `openrouter`, `azure`, `bedrock`, `copilot`, `mistral`, `cerebras`, `deepseek`, `zhipu`, `vercel`, `local_edit` |
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
| `--redteam <url>` | — | Run autonomous red team scan against target URL |
| `--redteam-config <file>` | — | YAML config file for auth flows, scope, depth |
| `--redteam-report <id>` | — | Generate pentest report from a previous session |
| `--voice` | false | Enable voice input via Groq Whisper |
| `--tailscale` | false | Enable Tailscale funnel for remote access |
| `--serve` | false | Run as HTTP daemon (REST + SSE API) |

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
| `/workflow new <name> <desc>` | Create a Code Complete workflow (8-stage development pipeline) |
| `/workflow list` | List all workflows with current stage and progress |
| `/workflow show <name>` | Display workflow stages, checklist progress, and current stage items |
| `/workflow advance <name>` | Mark current stage complete and advance to the next |
| `/workflow check <name> <id>` | Toggle a checklist item in the current stage |
| `/workflow generate <name>` | AI-generate a checklist for the current stage |
| `/redteam scan <url>` | Start an autonomous red team scan against a target URL |
| `/redteam list` | List all red team sessions |
| `/redteam show <id>` | Display findings for a session |
| `/redteam report <id>` | Generate a full pentest report |
| `/redteam config` | Show current red team configuration |
| `/test [command]` | Run project tests (auto-detects cargo/npm/pytest/go); optional custom command override |
| `/arena compare <p1> <p2> [prompt]` | Blind A/B model comparison — hidden identities, vote, reveal |
| `/arena stats` | Show arena leaderboard (wins/losses/ties per provider) |
| `/arena history` | Show arena vote history |
| `/sessions` | List all stored agent sessions |
| `/sessions show <id>` | View a session's messages and steps |
| `/sessions search <query>` | Search across all sessions |
| `/voice` | Toggle voice input (Groq Whisper) |
| `/discover` | Discover nearby VibeCLI instances via mDNS |
| `/pair` | Generate QR code for device pairing |
| `/orchestrate status` | Show orchestration state (lessons, tasks, progress) |
| `/orchestrate lesson <text>` | Record a lesson learned |
| `/orchestrate todo <text>` | Add a todo item |
| `/collab create [room]` | Create a CRDT collaboration room |
| `/collab join <room>` | Join an existing collaboration room |
| `/coverage` | Run code coverage with auto-detected tool |
| `/transform <type>` | Run code transforms (Python 2→3, Vue 2→3, etc.) |
| `/deps` | Show dependency tree and outdated packages |
| `/docker` | Docker container management |
| `/sandbox` | Container sandbox management |
| `/gateway` | Gateway messaging (Telegram, Discord, Slack, etc.) |
| `/config` | Display current configuration |
| `/help` | Show command reference |
| `/exit` or `/quit` | Exit VibeCLI |
| `! <command>` | Execute a shell command directly (e.g. `!ls -la`) |

> **Safety**: By default, all shell command execution requires user confirmation (`y/N`). Disable this with `require_approval_for_commands = false` in config.

---

## Workflow Examples

### Chat with AI

```text
> /chat explain how the ropey crate works
> What are the time complexities for common rope operations?
```

Once a conversation is started, you can type freely without `/chat`.

### Generate Code

```text
> /generate a Rust function that parses TOML from a string and returns a HashMap
💾 Save to file? (y/N or filename): parser.rs
✅ Saved to: parser.rs
```

### Apply AI Changes to a File

```text
> /apply src/main.rs add proper error handling using anyhow

📊 Proposed changes:
--- a/src/main.rs
+++ b/src/main.rs
...

✅ Apply these changes? (y/N): y
✅ Changes applied to: src/main.rs
```

### AI-Suggested Command

```text
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

## @ Context Types

VibeCLI supports inline context injection using `@` references in chat messages:

| Reference | Description |
|-----------|-------------|
| `@file:<path>` | Contents of a specific file |
| `@file:<path>:N-M` | Specific line range from a file |
| `@folder:<path>` | Recursive directory tree listing |
| `@web:<url>` | Fetched & stripped plain text from a URL |
| `@docs:<lib>` | Library documentation (e.g. `@docs:tokio`, `@docs:npm:express`, `@docs:py:requests`) |
| `@git` | Current branch, changed files, and diff excerpt |
| `@terminal` | Last 200 lines of terminal output |
| `@symbol:<name>` | Source code for a named symbol (function, struct, etc.) |
| `@codebase:<query>` | Semantic search over the codebase |
| `@github:owner/repo#N` | Fetch a GitHub issue or PR (uses `GITHUB_TOKEN` env var) |
| `@jira:PROJECT-123` | Fetch a Jira issue (uses `JIRA_BASE_URL`, `JIRA_EMAIL`, `JIRA_API_TOKEN` env vars) |

Example:

```
> /chat @jira:AUTH-456 explain this ticket and suggest an implementation plan
> /chat @github:torvalds/linux#1234 summarize this issue
```

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

**Security**: All authenticated routes require a `Bearer <token>` header and are rate-limited to 60 requests per 60 seconds. Request bodies are limited to 1 MB. Responses include CSP, X-Frame-Options, and other security headers.

Endpoints:

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/health` | No | Liveness check — returns `{"status":"ok"}` |
| POST | `/chat` | Yes | Single-turn chat, returns accumulated response |
| POST | `/chat/stream` | Yes | Streaming chat via Server-Sent Events |
| POST | `/agent` | Yes | Start an agent task, returns `{session_id}` |
| GET | `/stream/:session_id` | Yes | SSE stream of agent events in real-time |
| GET | `/jobs` | Yes | List all persisted background job records |
| GET | `/jobs/:id` | Yes | Get a single job record by ID |
| POST | `/jobs/:id/cancel` | Yes | Cancel a running background job |
| GET | `/sessions` | Yes | HTML index page of all agent sessions |
| GET | `/sessions.json` | Yes | JSON array of all session metadata |
| GET | `/view/:id` | Yes | Dark-mode HTML viewer for a session trace |
| GET | `/share/:id` | Yes | Shareable readonly session view (noindex) |
| GET | `/ws/collab/:room_id` | Token | WebSocket for CRDT collab (`?token=` query param) |
| POST | `/collab/rooms` | Yes | Create or get a collaboration room |
| GET | `/collab/rooms` | Yes | List all active collaboration rooms |
| GET | `/collab/rooms/:room_id/peers` | Yes | List peers in a room (names, cursor colors) |
| POST | `/acp/v1/tasks` | Yes | Create an ACP (Agent Communication Protocol) task |
| GET | `/acp/v1/tasks/:id` | Yes | Get an ACP task by ID |
| GET | `/acp/v1/capabilities` | No | List ACP server capabilities |
| POST | `/webhook/github` | No | GitHub App webhook receiver for CI review bot |
| POST | `/webhook/skill/:skill_name` | No | Trigger a skill by webhook name |
| GET | `/pair` | No | Device pairing endpoint — generates a one-time pairing URL |

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

## Code Complete Workflow

Guide application development through 8 structured stages inspired by Steve McConnell's *Code Complete*. Each stage has an AI-generated checklist that serves as a quality gate before advancing.

### The 8 Stages

| # | Stage | Focus |
|---|-------|-------|
| 1 | **Requirements** | User stories, functional/non-functional reqs, scope boundaries |
| 2 | **Architecture** | Subsystem decomposition, data design, security, build-vs-buy |
| 3 | **Design** | Classes, interfaces, patterns, algorithms, state management |
| 4 | **Construction Planning** | Tooling, coding standards, CI/CD, integration order, estimates |
| 5 | **Coding** | Implementation, defensive programming, naming, DRY, control flow |
| 6 | **Quality Assurance** | Code review, unit tests, static analysis, security scan |
| 7 | **Integration & Testing** | E2E tests, regression, performance, cross-platform |
| 8 | **Code Complete** | Docs, changelog, no TODOs, version tag, deploy runbook |

### Usage

```
> /workflow new my_app Build a REST API for user management

✅ Workflow 'my_app' created with 8 stages (Code Complete methodology)
   Current stage: Requirements
   Use /workflow generate my_app to AI-generate a checklist for the current stage.

> /workflow generate my_app

🤖 Generating Requirements checklist for 'my_app'...
✅ Generated 10 checklist items:
   [ ] 1: Define core user CRUD endpoints (GET, POST, PUT, DELETE)
   [ ] 2: Specify authentication method (JWT vs session)
   ...

> /workflow check my_app 1
✅ Toggled item 1 in 'my_app'

> /workflow show my_app

🏗️  Workflow: my_app  [12% complete]
   Build a REST API for user management

   ▶ 1. Requirements (1/10)
   ○ 2. Architecture
   ○ 3. Design
   ...

> /workflow advance my_app
✅ Advanced to stage: Architecture
```

Workflows are stored as markdown files in `.vibecli/workflows/` with YAML front-matter. The stage advancement gate requires ≥80% checklist completion in VibeUI.

---

## Red Team Security Testing

Run autonomous penetration tests against applications you build with VibeCody. The red team module executes a 5-stage pipeline: **Recon → Analysis → Exploitation → Validation → Report**.

### 5-Stage Pipeline

| # | Stage | Focus |
|---|-------|-------|
| 1 | **Recon** | Target discovery, tech fingerprinting, endpoint enumeration |
| 2 | **Analysis** | Source-code-aware vulnerability identification (white-box) |
| 3 | **Exploitation** | Active validation via HTTP requests + browser actions |
| 4 | **Validation** | Confirm exploitability, generate PoC payloads |
| 5 | **Report** | Structured findings with CVSS scores + remediation |

### Attack Vectors

15 built-in attack vectors covering OWASP Top 10: SQL injection, XSS, SSRF, IDOR, path traversal, auth bypass, command injection, mass assignment, open redirect, XXE, insecure deserialization, NoSQL injection, template injection, CSRF, and cleartext transmission.

### Usage

```bash
# Non-interactive scan (CI mode)
vibecli --redteam http://localhost:3000

# With auth config and scope restrictions
vibecli --redteam http://localhost:3000 --redteam-config auth.yaml

# Generate report from a previous session
vibecli --redteam-report <session-id>
```

```
# Interactive REPL
> /redteam scan http://localhost:3000
> /redteam list
> /redteam show <session-id>
> /redteam report <session-id>
> /redteam config
```

Sessions are persisted as JSON at `~/.vibecli/redteam/`. Findings include CVSS severity scores, PoC payloads, and remediation guidance.

> **Authorization**: Red team features require explicit user consent and target only user-controlled applications (localhost / staging).

---

## Test Runner

Run project tests directly from the REPL with auto-detection of the test framework:

```
> /test

🧪 Running: cargo test
...
✅ Tests passed

> /test npm test -- --coverage

🧪 Running: npm test -- --coverage
...
✅ Tests passed
```

### Framework Auto-Detection

| File Detected | Command Used |
|---------------|-------------|
| `Cargo.toml` | `cargo test` |
| `package.json` | `npm test` |
| `pytest.ini` / `pyproject.toml` / `setup.py` | `python -m pytest -v` |
| `go.mod` | `go test ./...` |

If no framework is detected, provide a custom command: `/test <command>`.

In VibeUI, the **🧪 Tests** panel provides a richer experience with live streaming log output, per-test pass/fail results, expandable failure details, filter tabs (All / Failed / Passed), and a custom command input field.

---

## Skills System

VibeCody ships with **507 skill files** across 25+ categories covering finance, healthcare, security, cloud (AWS/Azure/GCP), data engineering, robotics, compliance, SRE, and more. Skills activate based on trigger keywords. Place custom `.md` files in `.vibecli/skills/` or `~/.vibecli/skills/`:

```markdown
---
name: rust-testing
triggers: [test, testing, cargo test]
category: rust
tools_allowed: [read_file, write_file, bash]
---
Use `#[tokio::test]` for async tests...
```

---

## Gateway Messaging

VibeCLI can act as a bot on 18 messaging platforms via the gateway system:

Telegram, Discord, Slack, Signal, Matrix, Twilio SMS, iMessage, WhatsApp, Teams, IRC, Twitch, WebChat, Nostr, QQ, Tlon (+ 3 original).

Configure gateways in `~/.vibecli/config.toml`:

```toml
[[gateway]]
platform = "telegram"
bot_token = "..."
whitelist = ["@username"]
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
    ├── skills/             # 507 skill files (25+ categories)
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
        ├── redteam.rs      # Red team 5-stage pentest pipeline
        ├── bugbot.rs       # OWASP/CWE static scanner (15 patterns)
        ├── workflow.rs     # Code Complete 8-stage workflow
        ├── workflow_orchestration.rs  # Lessons + todo orchestration
        ├── gateway.rs      # 18-platform messaging gateway
        ├── transform.rs    # Code transforms (Python 2→3, Vue 2→3, etc.)
        ├── marketplace.rs  # Plugin marketplace
        ├── background_agents.rs  # Background agent definitions
        ├── session_store.rs     # SQLite session persistence
        ├── sandbox.rs      # Container sandbox (Docker/Podman)
        ├── voice.rs        # Voice input (Groq Whisper)
        ├── pairing.rs      # QR code device pairing
        ├── tailscale.rs    # Tailscale funnel
        ├── discovery.rs    # mDNS service discovery
        ├── scheduler.rs    # /remind and /schedule commands
        └── tui/
            ├── mod.rs      # TUI run loop and event handling
            ├── app.rs      # TUI application state machine
            ├── ui.rs       # Ratatui layout and widget rendering
            └── components/
                ├── diff_view.rs  # Multi-file diff viewer widget
                ├── vim_editor.rs # Vim-style modal editor (Normal/Insert/Visual/Command)
                └── diagnostics.rs # Cargo/eslint diagnostics panel
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
| `vibe-collab` | CRDT multiplayer collaboration (yrs + DashMap + Axum WS) |
