---
layout: page
title: Architecture
permalink: /architecture/
---


VibeCody is a Rust workspace (monorepo) with 9 crate members: three binary applications, five shared library crates, and a standalone indexing service, plus editor plugins, an agent SDK, and a skills library.


## Workspace Layout

```text
vibecody/                          ← Cargo workspace root
├── vibecli/
│   └── vibecli-cli/               ← Binary: terminal assistant
│       └── skills/                ← 543 skill files (25+ categories)
├── vibeui/
│   ├── src/                       ← React + TypeScript frontend (155+ panel components)
│   ├── src-tauri/                 ← Binary: Tauri desktop app
│   └── crates/
│       ├── vibe-core/             ← Library: editor primitives
│       ├── vibe-ai/               ← Library: AI provider + agent
│       ├── vibe-lsp/              ← Library: LSP client
│       ├── vibe-extensions/       ← Library: WASM extensions
│       └── vibe-collab/           ← Library: CRDT collaboration
├── vibeapp/                       ← Alternate Tauri shell
├── vibe-indexer/                  ← Standalone indexing service
├── vscode-extension/              ← VS Code extension
├── jetbrains-plugin/              ← JetBrains IDE plugin (Gradle)
├── neovim-plugin/                 ← Neovim plugin
└── packages/
    └── agent-sdk/                 ← TypeScript Agent SDK
```

### Dependency Graph

```text
vibecli-cli ──────────────-────┐
                               ▼
                         vibe-ai  ──── reqwest, async-trait, futures
                               │
vibe-ui (Tauri) ───────-───────┤
                               ▼
                         vibe-core ── ropey, git2, notify, walkdir,
                               │      portable-pty, similar, regex
                               │
                         vibe-lsp ── lsp-types, jsonrpc-core, tokio-util
                               │
                         vibe-extensions ── wasmtime, wasmtime-wasi
```


## `vibe-core` — Editor Primitives

The foundation crate — no AI dependencies, no UI dependencies.

### Text Buffer (`buffer.rs`)

Built on `ropey`, a rope data structure optimized for text editing:

- **O(log n)** insert, delete, and slice operations
- Undo/redo stack
- Multi-cursor support
- Line/column indexing
- Unicode-correct

```rust
pub struct TextBuffer {
    rope: Rope,
    undo_stack: Vec<EditOperation>,
    redo_stack: Vec<EditOperation>,
    cursors: Vec<Cursor>,
}
```

### File System (`file_system.rs`)

Async file operations built on `tokio`:

- `open(path)` → reads file into `TextBuffer`
- `save(path, content)` → async write
- File watcher via `notify` — emits events on external changes
- Directory listing for sidebar

### Workspace (`workspace.rs`)

Multi-folder project management:

- Maintains a list of open root paths
- Aggregates file trees across folders
- Coordinates Git status across workspace roots

### Git Integration (`git.rs`)

Uses `git2` (libgit2 bindings) for zero-overhead Git operations:

| Function | Description |
|----------|-------------|
| `get_status(path)` | Branch name + file status map |
| `get_diff(path, file)` | Unified diff for a single file |
| `get_repo_diff(path)` | Full workspace diff |
| `commit(path, message, files)` | Stage and commit |
| `push(path, remote, branch)` | Push to remote |
| `pull(path, remote, branch)` | Fast-forward pull |
| `list_branches(path)` | All local branches |
| `switch_branch(path, branch)` | Checkout |
| `get_history(path, limit)` | Recent commits |
| `discard_changes(path, file)` | Restore from HEAD |
| `is_git_repo(path)` | Check if path is in a repo |

### Diff Engine (`diff.rs`)

Text-level diffing using the `similar` crate:

- `DiffEngine::diff(old, new)` → `Vec<DiffHunk>`
- Hunk types: Equal, Insert, Delete
- Inline word-level diff within hunks
- Used by both VibeCLI diff viewer and VibeUI Git panel

### Terminal (`terminal.rs`)

PTY-based terminal using `portable-pty`:

- Spawns a shell process in a PTY
- Reads/writes raw bytes
- Used by VibeUI's terminal panel via Tauri IPC

### Search (`search.rs`)

File and content search:

- `walkdir` for recursive directory traversal
- `regex` for content pattern matching
- Respects `.gitignore` (via filtering)

### Command Executor (`executor.rs`)

Sandboxed shell command execution:

- Wraps `std::process::Command`
- Captures stdout/stderr
- Used by VibeCLI's `!` prefix and `/exec` command

### Codebase Index (`index/`)

Two-layer indexing for codebase intelligence:

| Module | Description |
|--------|-------------|
| `mod.rs` + `symbol.rs` | Tree-sitter based symbol extraction |
| `embeddings.rs` | HNSW vector index with local (Ollama) or cloud (OpenAI) embedding models |

`EmbeddingIndex` supports incremental updates, semantic search, and persistence:

```rust
impl EmbeddingIndex {
    pub async fn build(workspace: &Path, provider: &EmbeddingProvider) -> Result<Self>;
    pub async fn update(&mut self, changed_files: &[PathBuf]) -> Result<()>;
    pub async fn search(&self, query: &str, k: usize) -> Result<Vec<SearchHit>>;
}
```


## `vibe-ai` — AI Provider Abstraction

A unified interface over multiple LLM providers.

### The `AIProvider` Trait

```rust
#[async_trait]
pub trait AIProvider: Send + Sync {
    fn name(&self) -> &str;
    async fn is_available(&self) -> bool;
    async fn complete(&self, context: &CodeContext) -> Result<CompletionResponse>;
    async fn stream_complete(&self, context: &CodeContext) -> Result<CompletionStream>;
    async fn chat(&self, messages: &[Message], context: Option<String>) -> Result<String>;
    async fn stream_chat(&self, messages: &[Message]) -> Result<CompletionStream>;
}
```

`CompletionStream` is a `Pin<Box<dyn Stream<Item = Result<String>> + Send>>` — a type-erased async stream of text chunks for real-time streaming.

### `CodeContext`

Carries the context for inline code completions:

```rust
pub struct CodeContext {
    pub language: String,
    pub file_path: Option<String>,
    pub prefix: String,   // Code before cursor
    pub suffix: String,   // Code after cursor
    pub additional_context: Vec<String>,
}
```

### `ProviderConfig`

Builder pattern for provider configuration:

```rust
let config = ProviderConfig::new("claude".into(), "claude-3-5-sonnet-20241022".into())
    .with_api_key(std::env::var("ANTHROPIC_API_KEY")?)
    .with_max_tokens(4096)
    .with_temperature(0.2);
```

### Provider Implementations

All 22 providers follow the same pattern:

1. Send HTTP request to provider API using `reqwest`
2. For `chat()`: wait for full response
3. For `stream_chat()`: return a `CompletionStream` backed by SSE or chunked HTTP
4. Parse provider-specific JSON envelope to extract text

| Provider | API Type | Base URL |
|----------|----------|----------|
| Ollama | REST + NDJSON streaming | `http://localhost:11434` |
| Claude | REST + SSE streaming | `https://api.anthropic.com` |
| OpenAI | REST + SSE streaming | `https://api.openai.com` |
| Gemini | REST + SSE streaming | `https://generativelanguage.googleapis.com` |
| Grok | OpenAI-compatible | `https://api.x.ai` |
| Groq | OpenAI-compatible | `https://api.groq.com` |
| OpenRouter | OpenAI-compatible | `https://openrouter.ai/api` |
| Azure OpenAI | REST + SSE streaming | User-configured endpoint |
| Bedrock | AWS SigV4 signed | AWS regional endpoints |
| Copilot | OpenAI-compatible | `https://api.githubcopilot.com` |
| LocalEdit | Ollama FIM | `http://localhost:11434` |
| Mistral | REST + SSE streaming | `https://api.mistral.ai` |
| Cerebras | OpenAI-compatible | `https://api.cerebras.ai` |
| DeepSeek | OpenAI-compatible | `https://api.deepseek.com` |
| Zhipu | REST + SSE streaming | `https://open.bigmodel.cn` |
| Vercel AI | REST + SSE streaming | User-configured endpoint |
| Failover | Meta-provider | Wraps multiple providers with auto-fallback |

### `ChatEngine`

Session manager on top of `AIProvider`:

- Maintains `Vec<Message>` conversation history
- Injects system prompts
- Handles context truncation via message history management

### `CompletionEngine`

Inline code completion coordinator:

- Calls `stream_complete()` with current `CodeContext`
- Debounces requests
- Used by VibeUI for as-you-type completions

### Agent Loop (`agent.rs`)

The autonomous plan→act→observe loop:

- Multi-step execution with configurable approval tiers
- Tool use framework (read, write, patch, bash, search, list, complete)
- Streaming events via `AgentEvent` enum
- 30-step maximum with configurable limit

### Planner Agent (`planner.rs`)

Plan Mode — generates structured execution plans without executing:

- `PlannerAgent::plan()` produces `ExecutionPlan` with typed `PlanStep` entries
- Used by `/plan` in VibeCLI and "Plan first" toggle in VibeUI
- Steps can be approved individually before execution

### Multi-Agent Orchestrator (`multi_agent.rs`)

Parallel agent execution across isolated git worktrees:

- `MultiAgentOrchestrator::run_parallel()` splits tasks across N agents
- Each agent operates on its own worktree
- Results merged back to main branch
- Events streamed via `OrchestratorEvent`

### Hooks System (`hooks.rs`)

Event-driven hooks matching Claude Code's model:

- Events: `SessionStart`, `PreToolUse`, `PostToolUse`, `Stop`, `TaskCompleted`, `SubagentStart`
- Handler types: shell commands or LLM evaluations
- `HookRunner` matches events against configured patterns and executes handlers
- Supports block/modify/react decisions

### Skills System (`skills.rs`)

Context-aware capability snippets:

- `SkillLoader` discovers skills from workspace and global directories
- Skills activate based on trigger keyword matching
- YAML frontmatter + Markdown body format

### Artifacts (`artifacts.rs`)

Structured output from agent operations:

- Types: `TaskList`, `ImplementationPlan`, `FileChange`, `CommandOutput`, `TestResults`, `ReviewReport`, `Text`
- `ArtifactStore` with annotations and timestamps
- Async feedback queue for user input

### Admin Policy (`policy.rs`)

Workspace-level security restrictions:

- Loaded from `.vibecli/policy.toml` (workspace) or `~/.vibecli/policy.toml` (global)
- Tool blocking/approval, path allow/deny lists, step limits
- Minimal glob matcher (no external crate dependency)

### Trace / Session Resume (`trace.rs`)

JSONL audit logging and session resume:

- `TraceWriter` records every tool call with input, output, timing, and approval source
- `load_session()` restores full `SessionSnapshot` for `--resume`
- `list_traces()` for browsing past sessions

### OpenTelemetry (`otel.rs`)

Span attribute constants for the OTLP pipeline:

- Defines `ATTR_SESSION_ID`, `ATTR_TASK`, `SPAN_SESSION`, `SPAN_STEP`, etc.
- Used by `otel_init.rs` in VibeCLI for tracing spans

### Red Team Pipeline (`redteam.rs`)

Autonomous 5-stage penetration testing module:

- **Recon** → endpoint enumeration, tech fingerprinting
- **Analysis** → white-box LLM-assisted vulnerability identification using codebase index
- **Exploitation** → active HTTP-based validation with browser actions
- **Validation** → confirm exploitability, generate PoC payloads
- **Report** → markdown report with CVSS scores + remediation

Key types: `AttackVector` (15 variants), `CvssSeverity`, `RedTeamSession`, `VulnFinding`, `RedTeamManager`

Sessions persisted as JSON at `~/.vibecli/redteam/`. Integrates with `bugbot.rs` CWE patterns and existing agent browser actions.

### BugBot Static Scanner (`bugbot.rs`)

OWASP/CWE static pattern scanner:

- 15 regex-based vulnerability patterns (CWE-89, CWE-79, CWE-22, CWE-798, CWE-338, CWE-78, CWE-601, CWE-918, CWE-611, CWE-502, CWE-943, CWE-1336, CWE-639, CWE-352, CWE-319)
- Runs before LLM analysis on every diff
- Results merged with LLM findings (static first)

### MCP Client (`mcp.rs`)

Model Context Protocol integration:

- JSON-RPC 2.0 over stdio
- Discovers and invokes tools from external MCP servers
- `/mcp list` and `/mcp tools <server>` in REPL


## `vibe-lsp` — Language Server Protocol

LSP client infrastructure for editor intelligence features.

### Architecture

```text
Editor ──→ LspClient ──→ ChildProcess (language server)
               │               │
          JSON-RPC request  stdin/stdout
               │
         Response dispatcher
```

- Uses `jsonrpc-core` for message framing
- `tokio-util` codec for framed I/O over process stdio
- Implements the LSP spec types from `lsp-types`

### Features

- Go-to-definition
- Hover documentation
- Diagnostics (errors, warnings)
- Code completion
- Code actions (quick fixes)
- Rename symbol


## `vibe-extensions` — WASM Extension System

Provides a sandbox for third-party extensions via WebAssembly.

### Design

- Extensions are compiled to WASM (targeting `wasm32-wasi`)
- `wasmtime` hosts and executes extension modules
- `wasmtime-wasi` provides WASI system call stubs
- Extensions communicate via a VS Code–compatible host API

### Extension API

```rust
// Extension registers lifecycle hooks
extension.on_file_save(|path| { /* ... */ });
extension.on_key_press(|key| { /* ... */ });
extension.register_command("my-command", |args| { /* ... */ });
```


## VibeCLI TUI Architecture

The TUI is built with Ratatui and follows an Elm-like architecture:

```text
Input Event (crossterm)
        │
        ▼
   App::handle_event()     ← state mutation
        │
        ▼
   App state (AppState)
        │
        ▼
   ui::draw(frame, state)  ← pure rendering
```

**State (`app.rs`):**

```rust
pub struct AppState {
    pub mode: AppMode,           // Chat | DiffView | FileTree
    pub messages: Vec<ChatMessage>,
    pub input: String,
    pub provider: Box<dyn AIProvider>,
    pub scroll_offset: usize,
    pub diff_content: Option<String>,
}
```

**Rendering (`ui.rs`):**

Uses Ratatui's constraint-based layout system:

- Horizontal split: main content + side panels
- Vertical split: message history + input box
- Scrollable list for chat history
- Syntax-highlighted diff viewer

### VibeCLI Server (`serve.rs`)

HTTP daemon mode (`vibecli serve`) for VS Code extension and Agent SDK:

- Axum-based REST/SSE API
- Endpoints: `/health`, `/chat`, `/chat/stream`, `/agent/start`, `/agent/{id}/stream`
- Shared state with event broadcast channels

### Code Review Agent (`review.rs`)

Structured code review mode (`vibecli review`):

- Reviews git diffs (uncommitted, staged, branch, or PR)
- Produces `ReviewReport` with issues, suggestions, and scoring
- Posts directly to GitHub PRs via `gh` CLI
- Focus areas: Security, Performance, Correctness, Style, Testing

### OpenTelemetry Init (`otel_init.rs`)

OTLP pipeline initialization:

- Sets up `tracing-opentelemetry` bridge
- Exports spans via OTLP/HTTP to any collector (Jaeger, Grafana, etc.)
- `OtelGuard` ensures flush on shutdown


## VibeUI Tauri Architecture

Tauri bridges the React frontend and Rust backend via IPC.

### Data Flow (AI Chat Example)

```text
User types in AIChat.tsx
        │
        ▼
invoke("ai_chat", { provider, messages })
        │  Tauri IPC
        ▼
Rust command handler in src-tauri/src/
        │
        ▼
vibe_ai::ChatEngine::chat(messages)
        │
        ▼
HTTP request to AI provider
        │
        ▼
Stream response back via Tauri events
        │  emit("ai-chunk", text)
        ▼
AIChat.tsx renders chunks progressively
```

### Tauri Command Pattern

```rust
#[tauri::command]
async fn ai_chat(
    provider: String,
    messages: Vec<Message>,
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    let engine = state.chat_engine.lock().await;
    engine.chat(&messages, None).await.map_err(|e| e.to_string())
}
```


## Testing Strategy

**4,770 unit tests** across the workspace (0 failures).

| Crate | Tests | Key coverage areas |
|-------|-------|--------------------|
| `vibecli` | 5,600+ | session store, serve, config, review, workflow, REPL, redteam, gateway, channel daemon, branch agent, spec pipeline, VM orchestrator, transform, marketplace, background agents, TUI, security scan, automations, project init, all feature modules |
| `vibe-ai` | 1,020+ | 22 providers, tools, trace, hooks, policy, skills, agent, multi-agent, MCP, agent teams |
| `vibe-core` | 370+ | buffer, git, diff, context, file system, workspace, search, terminal, index/embeddings, executor |
| `vibe-ui` | 230+ | Tauri commands, coverage, cost, flow, agent executor, shadow workspace |
| `vibe-lsp` | 74 | LSP client, features, manager |
| `vibe-collab` | 53 | CRDT rooms, server registry, protocol, awareness |
| `vibe-extensions` | 46 | loader, manifest, permissions |
| TypeScript | — | `tsc --noEmit` type checking |


## VS Code Extension

The `vscode-extension/` directory contains a full VS Code extension:

- **Chat sidebar** — webview panel communicating with VibeCLI daemon
- **Inline completions** — `VibeCLIInlineCompletionProvider` with debouncing
- **Agent mode** — start agent tasks directly from VS Code
- **API client** — REST calls to `vibecli serve`


## Agent SDK

The `packages/agent-sdk/` TypeScript package (`@vibecody/agent-sdk`) provides a programmatic interface for building on VibeCody:

- Connect to VibeCLI daemon
- Start agent sessions, stream events
- Chat and completion APIs
- Typed event handling
