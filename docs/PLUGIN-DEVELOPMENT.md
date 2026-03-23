---
layout: page
title: Plugin Development Guide
permalink: /plugin-development/
nav_order: 10
---


Complete reference for building VibeCody plugins, WASM extensions, MCP integrations, skills, and hooks. This document is designed for both human developers and AI coding assistants.

## Table of Contents

1. [Architecture Overview](#architecture-overview)
2. [Plugin System](#plugin-system)
3. [Skills](#skills)
4. [Hooks](#hooks)
5. [WASM Extensions](#wasm-extensions)
6. [MCP Server Integration](#mcp-server-integration)
7. [ACP Protocol](#acp-protocol)
8. [HTTP Daemon API](#http-daemon-api)
9. [AI Provider Trait](#ai-provider-trait)
10. [Tool System](#tool-system)
11. [Agent Loop](#agent-loop)
12. [Configuration Reference](#configuration-reference)
13. [Examples](#examples)

## Architecture Overview

VibeCody has a **five-layer extensibility architecture**:

| Layer | Type | Runtime | Location |
|-------|------|---------|----------|
| **Skills** | Markdown with YAML front-matter | Loaded at startup, matched by trigger keywords | `~/.vibecli/skills/` or `<workspace>/.vibecli/skills/` |
| **Hooks** | Shell scripts, LLM prompts, or HTTP webhooks | Fired on agent events (pre/post tool use, file save, etc.) | Configured in `~/.vibecli/config.toml` |
| **Plugins** | Bundles of skills + hooks + commands | Installed to `~/.vibecli/plugins/<name>/` | Registry at `https://registry.vibecody.dev/api/v1` |
| **WASM Extensions** | Sandboxed WebAssembly modules | Loaded by VibeUI (Tauri desktop app) | `~/.vibeui/extensions/` |
| **MCP/ACP** | JSON-RPC over stdio or HTTP | Bidirectional tool integration | Configured in `config.toml` or via `--mcp-server` flag |

### Monorepo Structure

```sh
vibecody/
  vibecli/vibecli-cli/src/     # CLI binary (Rust)
    main.rs                     # Entry point, CLI args, command dispatch
    plugin.rs                   # Plugin loader & manifest parsing
    plugin_sdk.rs               # Plugin SDK types, scaffolding
    plugin_registry.rs          # Registry client, search, publish
    plugin_lifecycle.rs         # Install, enable, disable, update
    tool_executor.rs            # Tool call execution
    config.rs                   # Configuration at ~/.vibecli/config.toml
    serve.rs                    # HTTP daemon (REST + SSE)
    mcp_server.rs               # MCP server (JSON-RPC over stdio)
    acp.rs                      # Agent Communication Protocol
  vibecli/vibecli-cli/skills/   # 526 built-in skill files
  vibeui/crates/
    vibe-ai/src/
      provider.rs               # AIProvider trait
      agent.rs                  # AgentLoop, AgentContext, AgentEvent
      tools.rs                  # ToolCall, ToolResult, parsing
      hooks.rs                  # HookRunner, HookEvent, HookDecision
      skills.rs                 # SkillLoader, Skill, SkillWatcher
      mcp.rs                    # MCP client
    vibe-core/src/              # Text buffer, filesystem, git, search, index
    vibe-extensions/src/
      lib.rs                    # Extension loader (wasmtime)
      api.rs                    # Host function constants
      manifest.rs               # ExtensionManifest, Permission, Registry
  vibeui/src/                   # React/TypeScript frontend (Tauri 2)
```

## Plugin System

### Plugin Directory Layout

```sh
~/.vibecli/plugins/my-plugin/
  plugin.toml          # Required manifest
  skills/              # Auto-activated skill files (.md)
    my-skill.md
  hooks/               # Event-triggered scripts
    pre-tool.sh
    post-write.sh
  commands/            # REPL commands (/my-plugin-cmd)
    lint.sh
  README.md            # Optional documentation
```

### Plugin Manifest (`plugin.toml`)

#### Basic Manifest

```toml
name = "my-plugin"
version = "1.0.0"
description = "What this plugin does"
author = "Your Name"

[[hooks]]
event = "PostToolUse"
tools = ["write_file", "apply_patch"]
command = "hooks/post-write.sh"
```

#### Full Manifest (V2)

```toml
name = "my-plugin"
version = "1.0.0"
display_name = "My Plugin"
description = "A detailed description of the plugin"
author = "Your Name"
license = "MIT"
repository = "https://github.com/user/vibecody-my-plugin"
homepage = "https://example.com"
kind = "connector"       # connector | adapter | optimizer | theme | skillpack | workflow | extension
min_vibecli_version = "0.1.0"
max_vibecli_version = "2.0.0"
keywords = ["jira", "project-management", "issues"]
icon = "icon.png"
platforms = ["macos", "linux", "windows"]

# Capabilities the plugin requires
capabilities = ["FileRead", "FileWrite", "NetworkAccess"]

# Dependencies on other plugins
[[dependencies]]
name = "vibecody-core-utils"
version = ">=1.0.0"

# Hook definitions
[[hooks]]
event = "PostToolUse"
tools = ["write_file"]
handler = "hooks/post-write.sh"
priority = 100           # Lower runs first
async_exec = false       # true = non-blocking

# Command definitions
[[commands]]
name = "lint"
description = "Run linter on changed files"
handler = "commands/lint.sh"

[[commands.args]]
name = "fix"
description = "Auto-fix issues"
required = false
default = "false"

# Plugin-specific settings
[[settings]]
key = "api_url"
description = "API endpoint URL"
type = "string"
required = true

[[settings]]
key = "api_token"
description = "API authentication token"
type = "secret"          # string | number | boolean | secret | filepath | enum
required = true

[[settings]]
key = "severity"
description = "Minimum severity to report"
type = "enum"
values = ["low", "medium", "high", "critical"]
default = "medium"
```

### Plugin Kind

| Kind | Purpose | Example |
|------|---------|---------|
| `Connector` | Integrate external services (Jira, Linear, Notion) | vibecody-jira |
| `Adapter` | Add AI providers, gateways, container runtimes | vibecody-terraform |
| `Optimizer` | Linters, formatters, refactoring tools | vibecody-prettier |
| `Theme` | UI themes and color schemes | vibecody-dracula-theme |
| `SkillPack` | Bundles of skill markdown files | vibecody-devops-pack |
| `Workflow` | Pre-built agent workflow templates | vibecody-code-review |
| `Extension` | WASM extensions for VibeUI | vibecody-minimap |

### Plugin Capabilities

| Capability | Description | Dangerous? |
|-----------|-------------|------------|
| `FileRead` | Read files in workspace | No |
| `FileWrite` | Write/modify files | Yes |
| `NetworkAccess` | Make HTTP requests | Yes |
| `ProcessExec` | Execute shell commands | Yes |
| `EnvRead` | Read environment variables | No |
| `Notification` | Show notifications | No |
| `Clipboard` | Access clipboard | No |
| `GitAccess` | Run git commands | No |
| `DatabaseAccess` | Access databases | Yes |
| `HttpServer` | Start HTTP servers | No |
| `WebSocket` | Open WebSocket connections | No |

Plugins requesting dangerous capabilities must provide descriptions of at least 20 characters.

### Plugin CLI Commands

```bash
# Create a new plugin from template
vibecli --plugin create my-plugin --kind connector

# Install from registry or Git repo
vibecli --plugin install vibecody-jira
vibecli --plugin install https://github.com/user/vibecody-my-plugin

# Lifecycle management
vibecli --plugin enable my-plugin
vibecli --plugin disable my-plugin
vibecli --plugin uninstall my-plugin
vibecli --plugin update my-plugin
vibecli --plugin update              # Update all

# Discovery
vibecli --plugin list
vibecli --plugin info my-plugin
vibecli --plugin search "jira"

# Development
vibecli --plugin dev --watch         # Hot-reload during development

# Publishing
vibecli --plugin publish ./my-plugin
```

### Plugin Lifecycle States

```text
Installed → Enabled ↔ Disabled
              ↓
           Outdated (newer version available)
              ↓
           DevMode (locally linked)
              ↓
           Errored (load failure)
```

### Plugin State Storage

Stored in `~/.vibecli/plugin-state.json`:

```json
{
  "version": "1",
  "plugins": [
    {
      "name": "vibecody-jira",
      "version": "1.2.0",
      "state": "Enabled",
      "install_dir": "/Users/you/.vibecli/plugins/vibecody-jira",
      "installed_at": "2026-03-01T10:00:00Z",
      "config": {
        "api_url": "https://company.atlassian.net",
        "api_token": "***"
      },
      "checksum": "sha256:abc123..."
    }
  ]
}
```

## Skills

Skills are context-aware capability bundles injected into the agent's system prompt when trigger keywords match the user's input.

### Skill File Format

```markdown
triggers: ["docker", "container", "dockerfile", "docker-compose"]
tools_allowed: ["read_file", "write_file", "bash"]
category: devops

# Docker Best Practices

When working with Docker:

1. **Use multi-stage builds** to reduce image size...
2. **Never run as root** — add a non-root user...
3. **Pin base image versions** — use `node:20-alpine` not `node:latest`...
```

### Skill Front-Matter Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `triggers` | `string[]` | Yes | Keywords that activate this skill (case-insensitive substring match) |
| `tools_allowed` | `string[]` | No | Restrict which tools the agent can use (empty = no restriction) |
| `category` | `string` | No | Grouping category for organization |
| `name` | `string` | No | Override the display name (defaults to filename) |
| `description` | `string` | No | Short description |
| `version` | `string` | No | Semantic version |
| `requires.bins` | `string[]` | No | Required CLI binaries (e.g., `cargo`, `docker`) |
| `requires.env` | `string[]` | No | Required environment variables |
| `requires.os` | `string[]` | No | OS filter: `macos`, `linux`, `windows` |
| `install.brew` | `string` | No | Homebrew package to auto-install |
| `install.npm` | `string` | No | npm package to auto-install |
| `install.cargo` | `string` | No | Cargo package to auto-install |
| `install.pip` | `string` | No | pip package to auto-install |
| `config.<KEY>` | `string` | No | Per-skill configuration key-value |
| `webhook_trigger` | `string` | No | URL path that triggers this skill (e.g., `/webhook/deploy`) |

### Skill Loading Priority

1. `<workspace>/.vibecli/skills/` (project-specific)
2. `~/.vibecli/skills/` (user-global)
3. `~/.vibecli/plugins/*/skills/` (from installed plugins)
4. `vibecli/vibecli-cli/skills/` (built-in, 543 skills)

Later sources do NOT override earlier ones. All matching skills are injected.

### Skill Matching

Skills match when **any** trigger keyword appears as a substring in the user's message (case-insensitive). Multiple skills can match simultaneously.

```rust
// Pseudo-code for matching
fn matches(skill: &Skill, user_message: &str) -> bool {
    let lower = user_message.to_lowercase();
    skill.triggers.iter().any(|t| lower.contains(&t.to_lowercase()))
}
```

## Hooks

Hooks intercept agent events and can allow, block, or inject context.

### Hook Configuration (`~/.vibecli/config.toml`)

```toml
# Shell command hook — runs on every file write
[[hooks]]
event = "PostToolUse"
tools = ["write_file"]
handler = { command = "sh .vibecli/hooks/format.sh" }
async = false

# LLM hook — AI-powered safety check
[[hooks]]
event = "PreToolUse"
tools = ["bash"]
handler = { llm = "Check if this bash command is safe. Block if it could delete important data." }

# HTTP webhook — external service integration
[[hooks]]
event = "FileSaved"
paths = ["**/*.rs"]
handler = { url = "https://linter.example.com/check", method = "POST", timeout_ms = 5000 }
async = true
```

### Hook Events

| Event | Description | Blockable? |
|-------|-------------|------------|
| `SessionStart` | Agent session begins | No |
| `UserPromptSubmit` | User sends a message | Yes |
| `PreToolUse` | Before a tool call executes | Yes |
| `PostToolUse` | After a tool call completes | No (can inject context) |
| `Stop` | Agent stops | No |
| `TaskCompleted` | Agent completes task | No |
| `SubagentStart` | Sub-agent spawned | No |
| `FileSaved` | File written to disk | No (can inject context) |
| `FileCreated` | New file created | No |
| `FileDeleted` | File deleted | No |

### Hook Handlers

#### Shell Command Handler

Receives JSON event on **stdin**. Protocol:

| Exit Code | Result |
|-----------|--------|
| `0` | Allow |
| `2` | Block (stderr = reason) |
| Other | Allow (treated as error) |

Or write JSON to stdout:

```json
{"allow": false, "reason": "Blocked: command deletes production data"}
```

```json
{"context": "Note: the previous write triggered a lint warning on line 42"}
```

**Example hook script:**

```bash
#!/bin/bash
# .vibecli/hooks/format.sh
# Auto-format Rust files after write

EVENT=$(cat)  # Read JSON from stdin
FILE=$(echo "$EVENT" | jq -r '.call.path // empty')

if [[ "$FILE" == *.rs ]]; then
    rustfmt "$FILE" 2>/dev/null
fi

exit 0  # Always allow
```

#### LLM Handler

The prompt is sent to the configured LLM provider with the event context. Expected response:

```json
{"ok": true}
```

or

```json
{"ok": false, "reason": "This command would delete the database"}
```

#### HTTP Handler

POST the event JSON to the URL. Expected response:

```json
{
  "decision": "allow",
  "reason": "",
  "context": ""
}
```

`decision` can be `"allow"`, `"block"`, or `"inject"`.

### Hook Filtering

- `tools`: Array of tool name substrings. Only fires for matching tool calls.
- `paths`: Array of glob patterns. Only fires for matching file paths.

```toml
[[hooks]]
event = "PostToolUse"
tools = ["write_file", "apply_patch"]   # Only file-write tools
paths = ["src/**/*.rs", "tests/**"]     # Only Rust source files
handler = { command = "cargo clippy" }
```

## WASM Extensions

Sandboxed WebAssembly modules that extend VibeUI (the desktop app).

### Extension Structure

```sh
~/.vibeui/extensions/my-extension/
  extension.json         # Manifest
  extension.wasm         # Compiled WASM module
```

### Extension Manifest (`extension.json`)

```json
{
  "name": "my-extension",
  "version": "1.0.0",
  "display_name": "My Extension",
  "author": "Your Name",
  "description": "What this extension does",
  "permissions": ["FileRead", "Notify"],
  "min_host_version": "0.1.0",
  "wasm_path": "extension.wasm"
}
```

### Permissions

| Permission | Auto-granted? | Description |
|-----------|---------------|-------------|
| `FileRead` | Yes | Read files in workspace |
| `FileWrite` | No (dangerous) | Write/modify files |
| `Network` | No (dangerous) | Make network requests |
| `ProcessExec` | No (dangerous) | Execute processes |
| `EnvRead` | No | Read environment variables |
| `Notify` | Yes | Show notifications |
| `Clipboard` | No | Access clipboard |

### WASM Host API

Extensions are loaded by wasmtime. The host module is `"vibeui_host"`.

#### Host Functions (imported by WASM)

```rust
// Log a message
fn log(ptr: i32, len: i32)

// Read a file; returns bytes written to out buffer, or -1 on error
fn read_file(path_ptr: i32, path_len: i32, out_ptr: i32, out_cap: i32) -> i32

// Write a file; returns 0 on success, -1 on error
fn write_file(path_ptr: i32, path_len: i32, data_ptr: i32, data_len: i32) -> i32

// Show a notification
fn notify(ptr: i32, len: i32)
```

#### Extension Exports (implemented by WASM)

```rust
// Required: memory allocator for string passing
fn alloc(len: i32) -> i32

// Required: linear memory
memory: WebAssembly.Memory

// Optional: called once on load
fn init() -> i32

// Optional: called when a file is saved
fn on_file_save(ptr: i32, len: i32)

// Optional: called when text changes in editor
fn on_text_change(ptr: i32, len: i32)
```

#### Memory Limits

- Max WASM memory: **64 MiB** (`MAX_MEMORY_BYTES = 64 * 1024 * 1024`)

### Writing an Extension in Rust

```rust
// lib.rs — compiled to wasm32-wasi target

extern "C" {
    fn log(ptr: *const u8, len: usize);
    fn notify(ptr: *const u8, len: usize);
    fn read_file(path_ptr: *const u8, path_len: usize, out_ptr: *mut u8, out_cap: usize) -> i32;
}

#[no_mangle]
pub extern "C" fn alloc(len: usize) -> *mut u8 {
    let mut buf = Vec::with_capacity(len);
    let ptr = buf.as_mut_ptr();
    std::mem::forget(buf);
    ptr
}

#[no_mangle]
pub extern "C" fn init() -> i32 {
    let msg = "Extension loaded!";
    unsafe { log(msg.as_ptr(), msg.len()); }
    0
}

#[no_mangle]
pub extern "C" fn on_file_save(ptr: *const u8, len: usize) {
    let path = unsafe { std::str::from_utf8_unchecked(std::slice::from_raw_parts(ptr, len)) };
    if path.ends_with(".rs") {
        let msg = format!("Rust file saved: {}", path);
        unsafe { notify(msg.as_ptr(), msg.len()); }
    }
}
```

Compile: `cargo build --target wasm32-wasi --release`

## MCP Server Integration

VibeCLI can run as an MCP (Model Context Protocol) server, exposing its tools to Claude Desktop and other MCP clients.

### Starting the MCP Server

```bash
vibecli --mcp-server
```

### Claude Desktop Configuration

Add to `~/Library/Application Support/Claude/claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "vibecli": {
      "command": "vibecli",
      "args": ["--mcp-server"],
      "cwd": "/path/to/your/project"
    }
  }
}
```

### Exposed MCP Tools

| Tool | Description | Parameters |
|------|-------------|------------|
| `read_file` | Read a file from the workspace | `path: string` |
| `write_file` | Write content to a file | `path: string, content: string` |
| `list_directory` | List directory contents | `path: string` |
| `bash` | Execute a shell command | `command: string` |
| `search_files` | Full-text search across files | `query: string, glob?: string` |
| `agent_run` | Spawn an agent for a task | `task: string` |

### Protocol

- Transport: **stdio** (newline-delimited JSON-RPC 2.0)
- Spec: [https://spec.modelcontextprotocol.io/](https://spec.modelcontextprotocol.io/)

### Connecting to External MCP Servers

VibeCLI can also act as an MCP **client**, connecting to external MCP servers:

```toml
# ~/.vibecli/config.toml

[[mcp_servers]]
name = "github"
command = "npx"
args = ["@modelcontextprotocol/server-github", "--token", "$GITHUB_TOKEN"]

[[mcp_servers]]
name = "filesystem"
command = "npx"
args = ["@modelcontextprotocol/server-filesystem", "/path/to/allowed"]
```

External MCP tools become available to the agent as additional tool calls.

## ACP Protocol

The Agent Communication Protocol (ACP) enables external IDEs and tools to submit tasks to VibeCLI's HTTP daemon.

### Endpoints

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/acp/v1/capabilities` | List agent capabilities |
| `POST` | `/acp/v1/tasks` | Submit a task |
| `GET` | `/acp/v1/tasks/:id` | Get task status |
| `GET` | `/acp/v1/tasks/:id/events` | SSE stream of task events |

### Capabilities Response

```json
{
  "protocol_version": "1.0",
  "agent_name": "vibecli",
  "agent_version": "0.1.0",
  "supported_tools": ["read_file", "write_file", "bash", "search_files", "list_directory", "web_search", "fetch_url", "spawn_agent"],
  "supported_models": ["claude-sonnet-4-20250514"],
  "features": ["streaming", "vision", "hooks", "mcp"]
}
```

### Submit Task

```bash
curl -X POST http://localhost:7878/acp/v1/tasks \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "task": "Add error handling to the login function",
    "context": {
      "workspace_root": "/path/to/project",
      "files": [
        {"path": "src/auth.rs", "content": "...", "selection": {"start_line": 10, "start_col": 0, "end_line": 25, "end_col": 0}}
      ]
    },
    "model": "claude-sonnet-4-20250514",
    "approval_policy": "auto_edit"
  }'
```

### Task Status Response

```json
{
  "id": "task-abc123",
  "status": "complete",
  "summary": "Added Result return type and ? propagation to login()",
  "files_modified": ["src/auth.rs"],
  "steps_completed": 3
}
```

## HTTP Daemon API

Start the daemon:

```bash
vibecli serve --port 7878 --provider claude
```

All endpoints require `Authorization: Bearer <token>` (token from config or `VIBECLI_API_TOKEN`).

### Endpoints

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/health` | Health check (returns version, provider, status) |
| `POST` | `/chat` | Single-turn chat completion |
| `POST` | `/chat/stream` | Streaming chat (SSE) |
| `POST` | `/agent` | Start background agent task |
| `GET` | `/stream/:session_id` | SSE stream of agent events |
| `GET` | `/jobs` | List all job records |
| `GET` | `/jobs/:id` | Get single job record |
| `POST` | `/jobs/:id/cancel` | Cancel a running job |
| `GET` | `/sessions` | HTML index of sessions |
| `GET` | `/sessions.json` | JSON list of sessions |
| `GET` | `/view/:id` | HTML view of session transcript |
| `GET` | `/share/:id` | Shareable readonly view |

### Chat Request

```bash
curl -X POST http://localhost:7878/chat \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"messages": [{"role": "user", "content": "Explain async in Rust"}]}'
```

Response:

```json
{"content": "In Rust, async/await works through..."}
```

### Agent Request

```bash
curl -X POST http://localhost:7878/agent \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"task": "Fix the failing test in tests/auth.rs", "approval": "full_auto"}'
```

Response:

```json
{"session_id": "abc123"}
```

### SSE Event Stream

```bash
curl -N http://localhost:7878/stream/abc123 \
  -H "Authorization: Bearer $TOKEN"
```

Event types:

```
data: {"kind": "chunk", "content": "Let me read the test file..."}
data: {"kind": "step", "step_num": 1, "tool_name": "read_file", "success": true}
data: {"kind": "step", "step_num": 2, "tool_name": "write_file", "success": true}
data: {"kind": "complete", "content": "Fixed the assertion on line 42"}
```

## AI Provider Trait

All 17 AI providers implement the `AIProvider` trait:

```rust
#[async_trait]
pub trait AIProvider: Send + Sync {
    fn name(&self) -> &str;
    async fn is_available(&self) -> bool;
    async fn complete(&self, context: &CodeContext) -> Result<CompletionResponse>;
    async fn stream_complete(&self, context: &CodeContext) -> Result<CompletionStream>;
    async fn chat(&self, messages: &[Message], context: Option<String>) -> Result<String>;
    async fn stream_chat(&self, messages: &[Message]) -> Result<CompletionStream>;
    async fn chat_response(&self, messages: &[Message], context: Option<String>) -> Result<CompletionResponse>;
    async fn chat_with_images(
        &self, messages: &[Message], images: &[ImageAttachment], context: Option<String>
    ) -> Result<String>;
    fn supports_vision(&self) -> bool;
}
```

### Key Types

```rust
pub struct Message {
    pub role: MessageRole,     // System, User, Assistant
    pub content: String,
}

pub struct CodeContext {
    pub language: String,
    pub file_path: Option<String>,
    pub prefix: String,        // Code before cursor
    pub suffix: String,        // Code after cursor
    pub additional_context: Vec<String>,
}

pub struct ProviderConfig {
    pub provider_type: String, // "claude", "openai", "ollama", etc.
    pub api_key: Option<String>,
    pub api_url: Option<String>,
    pub model: String,
    pub max_tokens: Option<usize>,
    pub temperature: Option<f32>,
    pub api_key_helper: Option<String>,
    pub thinking_budget_tokens: Option<u32>,
}

pub struct CompletionResponse {
    pub text: String,
    pub model: String,
    pub usage: Option<TokenUsage>,
}

pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
}

pub struct ImageAttachment {
    pub base64: String,
    pub media_type: String,    // "image/png", "image/jpeg", etc.
}
```

### Supported Providers (17)

| # | Provider | Config Key | Models |
|---|----------|-----------|--------|
| 1 | Ollama | `ollama` | Any local model |
| 2 | Anthropic Claude | `claude` | claude-sonnet-4, claude-opus-4 |
| 3 | OpenAI | `openai` | gpt-4o, gpt-4o-mini, o1 |
| 4 | Google Gemini | `gemini` | gemini-2.0-flash, gemini-pro |
| 5 | xAI Grok | `grok` | grok-2, grok-3 |
| 6 | Groq | `groq` | llama-3.3-70b, mixtral |
| 7 | OpenRouter | `openrouter` | 300+ models |
| 8 | Azure OpenAI | `azure_openai` | Deployed models |
| 9 | AWS Bedrock | `bedrock` | Claude, Titan, Llama |
| 10 | GitHub Copilot | `copilot` | Copilot models |
| 11 | Mistral | `mistral` | mistral-large, codestral |
| 12 | Cerebras | `cerebras` | Fast inference |
| 13 | DeepSeek | `deepseek` | deepseek-coder |
| 14 | Zhipu | `zhipu` | GLM-4 |
| 15 | Vercel AI | `vercel_ai` | Unified proxy |
| 16 | LocalEdit | `local_edit` | Inline edits |
| 17 | Failover | `failover` | Auto-failover chain |

## Tool System

The agent uses XML-based tool calling that works with all providers.

### Available Tools

| Tool | XML Tag | Description |
|------|---------|-------------|
| `read_file` | `<tool_call><name>read_file</name><path>...</path></tool_call>` | Read file contents |
| `write_file` | `<tool_call><name>write_file</name><path>...</path><content>...</content></tool_call>` | Write/create file |
| `apply_patch` | `<tool_call><name>apply_patch</name><path>...</path><patch>...</patch></tool_call>` | Apply unified diff |
| `bash` | `<tool_call><name>bash</name><command>...</command></tool_call>` | Execute shell command |
| `search_files` | `<tool_call><name>search_files</name><query>...</query></tool_call>` | Full-text search |
| `list_directory` | `<tool_call><name>list_directory</name><path>...</path></tool_call>` | List directory |
| `web_search` | `<tool_call><name>web_search</name><query>...</query></tool_call>` | Web search |
| `fetch_url` | `<tool_call><name>fetch_url</name><url>...</url></tool_call>` | Fetch URL content |
| `task_complete` | `<tool_call><name>task_complete</name><summary>...</summary></tool_call>` | Signal completion |
| `spawn_agent` | `<tool_call><name>spawn_agent</name><task>...</task></tool_call>` | Spawn sub-agent |

### Tool Result Format

```rust
pub struct ToolResult {
    pub tool_name: String,
    pub output: String,      // Truncated at 8000 chars
    pub success: bool,
    pub truncated: bool,
}
```

### Destructive Tools

These tools require approval under `Suggest` and `AutoEdit` policies:

- `bash`
- `write_file`
- `apply_patch`
- `spawn_agent`

## Agent Loop

### Approval Policies

| Policy | Behavior |
|--------|----------|
| `Suggest` | Show each tool call, wait for user approval (y/n/a) |
| `AutoEdit` | Auto-approve file operations, prompt for bash |
| `FullAuto` | Execute all tool calls without prompting |

### Agent Events

```rust
pub enum AgentEvent {
    StreamChunk(String),           // LLM output token
    ToolCallPending { call, tx },  // Awaiting approval
    ToolCallExecuted(AgentStep),   // Tool completed
    Complete(String),              // Task finished
    Error(String),                 // Fatal error
    CircuitBreak { state, reason },// Health check triggered
}
```

### Circuit Breaker

The agent monitors its own health:

| State | Trigger | Action |
|-------|---------|--------|
| `Progress` | Normal operation | Continue |
| `Stalled` | No file changes for 5 steps | Suggest new approach |
| `Spinning` | Same error 3 times | Suggest alternative |
| `Degraded` | Output volume dropped 50%+ | Warn user |
| `Blocked` | External blocker | Report and stop |

### Agent Context

```rust
pub struct AgentContext {
    pub workspace_root: PathBuf,
    pub open_files: Vec<String>,
    pub git_branch: Option<String>,
    pub git_diff_summary: Option<String>,
    pub flow_context: Option<String>,
    pub approved_plan: Option<String>,
    pub extra_skill_dirs: Vec<PathBuf>,
    pub parent_session_id: Option<String>,
    pub depth: u32,                       // Nesting depth (0 = root)
    pub team_bus: Option<TeamMessageBus>, // Inter-agent messaging
}
```

## Configuration Reference

### Main Config (`~/.vibecli/config.toml`)

```toml
# Provider selection
[ollama]
model = "llama3.2"
api_url = "http://localhost:11434"

[claude]
api_key = "sk-..."
model = "claude-sonnet-4-20250514"
api_url = "https://api.anthropic.com"       # Configurable
thinking_budget_tokens = 10000               # Extended thinking

[openai]
api_key = "sk-..."
model = "gpt-4o"
api_url = "https://api.openai.com/v1"       # Configurable

# Failover chain — auto-switch on provider failure
[failover]
providers = ["claude", "openai", "ollama"]

# Index configuration
[index]
embedding_provider = "ollama"
embedding_model = "nomic-embed-text"

# Hook definitions
[[hooks]]
event = "PostToolUse"
tools = ["write_file"]
handler = { command = "sh .vibecli/hooks/format.sh" }

# MCP server connections
[[mcp_servers]]
name = "github"
command = "npx"
args = ["@modelcontextprotocol/server-github"]

# UI preferences
[ui]
theme = "dark"

# Safety settings
[safety]
max_file_size = 1048576
blocked_commands = ["rm -rf /", "mkfs", "dd if="]

# Tool configuration
[tools]
search_engine = "duckduckgo"    # duckduckgo | tavily | brave
tavily_api_key = ""
brave_api_key = ""

# Memory (auto-facts)
[memory]
enabled = true
max_facts = 100

# Shell environment policy
[tools.env]
inherit = "core"                 # all | core | none
include = ["PATH", "HOME", "LANG", "NODE_*"]
exclude = ["AWS_SECRET*", "*_TOKEN"]

[tools.env.set]
EDITOR = "vim"

# Routing (use different providers for planning vs execution)
[routing]
planner_provider = "claude"
executor_provider = "ollama"

# Sandbox settings
[sandbox]
enabled = false
runtime = "docker"               # docker | podman | opensandbox
image = "ubuntu:22.04"
```

### Environment Variables

| Variable | Description |
|----------|-------------|
| `ANTHROPIC_API_KEY` | Anthropic Claude API key |
| `OPENAI_API_KEY` | OpenAI API key |
| `GEMINI_API_KEY` | Google Gemini API key |
| `GROK_API_KEY` | xAI Grok API key |
| `GROQ_API_KEY` | Groq API key |
| `OPENROUTER_API_KEY` | OpenRouter API key |
| `MISTRAL_API_KEY` | Mistral API key |
| `DEEPSEEK_API_KEY` | DeepSeek API key |
| `VIBECLI_API_TOKEN` | HTTP daemon auth token |
| `GITHUB_TOKEN` | GitHub/Copilot token |

## Examples

### Example 1: Jira Connector Plugin

```
vibecody-jira/
  plugin.toml
  skills/
    jira-workflow.md
  hooks/
    sync-on-complete.sh
  commands/
    create-ticket.sh
    list-tickets.sh
```

**plugin.toml:**

```toml
name = "vibecody-jira"
version = "1.2.0"
display_name = "Jira Integration"
description = "Sync VibeCLI tasks with Jira issues"
author = "VibeCody Team"
license = "MIT"
kind = "connector"
capabilities = ["NetworkAccess", "FileRead"]
keywords = ["jira", "project-management", "issues", "agile"]

[[settings]]
key = "jira_url"
description = "Jira instance URL"
type = "string"
required = true

[[settings]]
key = "jira_token"
description = "Jira API token"
type = "secret"
required = true

[[hooks]]
event = "TaskCompleted"
handler = "hooks/sync-on-complete.sh"

[[commands]]
name = "create-ticket"
description = "Create a Jira ticket from the current task"
handler = "commands/create-ticket.sh"

[[commands.args]]
name = "project"
description = "Jira project key"
required = true

[[commands.args]]
name = "type"
description = "Issue type"
required = false
default = "Task"
```

**skills/jira-workflow.md:**

```markdown
triggers: ["jira", "ticket", "sprint", "backlog", "story", "epic"]
tools_allowed: ["bash", "read_file"]

# Jira Workflow

When the user mentions Jira tasks:
1. Use the /create-ticket command to create issues
2. Reference ticket IDs in commit messages (e.g., "PROJ-123: fix login")
3. Update ticket status when tasks complete
```

**hooks/sync-on-complete.sh:**

```bash
#!/bin/bash
EVENT=$(cat)
SUMMARY=$(echo "$EVENT" | jq -r '.summary')
JIRA_URL=$(vibecli --plugin info vibecody-jira | jq -r '.config.jira_url')
JIRA_TOKEN=$(vibecli --plugin info vibecody-jira | jq -r '.config.jira_token')

curl -s -X POST "$JIRA_URL/rest/api/3/issue" \
  -H "Authorization: Basic $JIRA_TOKEN" \
  -H "Content-Type: application/json" \
  -d "{\"fields\":{\"summary\":\"$SUMMARY\",\"project\":{\"key\":\"PROJ\"}}}"

exit 0
```

### Example 2: Auto-Formatter Hook

```toml
# In ~/.vibecli/config.toml
[[hooks]]
event = "PostToolUse"
tools = ["write_file"]
paths = ["**/*.ts", "**/*.tsx", "**/*.js"]
handler = { command = "npx prettier --write $FILE" }
async = true
```

### Example 3: Custom Skill

```markdown
triggers: ["graphql", "query", "mutation", "schema", "resolver"]
tools_allowed: ["read_file", "write_file", "bash"]
category: api
requires.bins: ["node"]
install.npm: "graphql"

# GraphQL Development

When working with GraphQL APIs:

1. **Schema-first design** — Define the schema before resolvers
2. **Use DataLoader** for N+1 prevention
3. **Input validation** — Use custom scalars for emails, dates, URLs
4. **Error handling** — Return typed errors in the `errors` array
5. **Testing** — Use `graphql-tag` for query parsing in tests
```

### Example 4: WASM Extension (Rust)

**extension.json:**

```json
{
  "name": "word-counter",
  "version": "1.0.0",
  "display_name": "Word Counter",
  "author": "Dev",
  "description": "Shows word count notification on file save",
  "permissions": ["FileRead", "Notify"],
  "wasm_path": "extension.wasm"
}
```

**src/lib.rs:**

```rust
extern "C" {
    fn log(ptr: *const u8, len: usize);
    fn read_file(path_ptr: *const u8, path_len: usize, out_ptr: *mut u8, out_cap: usize) -> i32;
    fn notify(ptr: *const u8, len: usize);
}

#[no_mangle]
pub extern "C" fn alloc(len: usize) -> *mut u8 {
    let mut buf = Vec::with_capacity(len);
    let ptr = buf.as_mut_ptr();
    std::mem::forget(buf);
    ptr
}

#[no_mangle]
pub extern "C" fn on_file_save(ptr: *const u8, len: usize) {
    let path = unsafe { std::str::from_utf8_unchecked(std::slice::from_raw_parts(ptr, len)) };

    let mut buf = vec![0u8; 1_000_000];
    let bytes_read = unsafe { read_file(path.as_ptr(), path.len(), buf.as_mut_ptr(), buf.len()) };

    if bytes_read > 0 {
        let content = unsafe { std::str::from_utf8_unchecked(&buf[..bytes_read as usize]) };
        let words = content.split_whitespace().count();
        let msg = format!("{}: {} words", path, words);
        unsafe { notify(msg.as_ptr(), msg.len()); }
    }
}
```

**Build:**

```bash
cargo build --target wasm32-wasi --release
cp target/wasm32-wasi/release/word_counter.wasm ~/.vibeui/extensions/word-counter/extension.wasm
```

## Quick Reference

### Creating a Plugin

```bash
# 1. Scaffold
vibecli --plugin create my-plugin --kind connector

# 2. Edit plugin.toml, add skills/hooks/commands

# 3. Test locally
vibecli --plugin dev --watch

# 4. Publish
vibecli --plugin publish ./my-plugin
```

### Adding a Skill

```bash
# Create a .md file in your project
mkdir -p .vibecli/skills
cat > .vibecli/skills/my-rules.md << 'EOF'
triggers: ["react", "component", "hook", "useState"]
tools_allowed: ["read_file", "write_file"]

# React Rules

1. Use functional components with hooks
2. Keep components under 200 lines
3. Extract custom hooks for reusable logic
EOF
```

### Adding a Hook

```toml
# Add to ~/.vibecli/config.toml
[[hooks]]
event = "PreToolUse"
tools = ["bash"]
handler = { command = "python3 .vibecli/hooks/safety-check.py" }
```

### Connecting an MCP Server

```toml
# Add to ~/.vibecli/config.toml
[[mcp_servers]]
name = "my-service"
command = "node"
args = ["my-mcp-server.js"]
```
