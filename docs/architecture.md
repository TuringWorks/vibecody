---
layout: page
title: Architecture
permalink: /architecture/
---

# Architecture

VibeCody is a Rust workspace (monorepo) containing two end-user applications and four shared library crates.

---

## Workspace Layout

```
vibecody/                          ← Cargo workspace root
├── vibecli/
│   └── vibecli-cli/               ← Binary: terminal assistant
└── vibeui/
    ├── src/                       ← React + TypeScript frontend
    ├── src-tauri/                 ← Binary: Tauri desktop app
    └── crates/
        ├── vibe-core/             ← Library: editor primitives
        ├── vibe-ai/               ← Library: AI provider abstraction
        ├── vibe-lsp/              ← Library: LSP client
        └── vibe-extensions/       ← Library: WASM extensions
```

### Dependency Graph

```
vibecli-cli ──────────────────┐
                               ▼
                         vibe-ai  ──── reqwest, async-trait, futures
                               │
vibe-ui (Tauri) ──────────────┤
                               ▼
                         vibe-core ── ropey, git2, notify, walkdir,
                               │      portable-pty, similar, regex
                               │
                         vibe-lsp ── lsp-types, jsonrpc-core, tokio-util
                               │
                         vibe-extensions ── wasmtime, wasmtime-wasi
```

---

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

---

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

All five providers follow the same pattern:

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

### `ChatEngine`

Session manager on top of `AIProvider`:

- Maintains `Vec<Message>` conversation history
- Injects system prompts
- Handles context truncation (planned)

### `CompletionEngine`

Inline code completion coordinator:

- Calls `stream_complete()` with current `CodeContext`
- Debounces requests
- Used by VibeUI for as-you-type completions

---

## `vibe-lsp` — Language Server Protocol

LSP client infrastructure for editor intelligence features.

### Architecture

```
Editor ──→ LspClient ──→ ChildProcess (language server)
               │               │
          JSON-RPC request  stdin/stdout
               │
         Response dispatcher
```

- Uses `jsonrpc-core` for message framing
- `tokio-util` codec for framed I/O over process stdio
- Implements the LSP spec types from `lsp-types`

### Planned Features

- Go-to-definition
- Hover documentation
- Diagnostics (errors, warnings)
- Code completion
- Code actions (quick fixes)
- Rename symbol

---

## `vibe-extensions` — WASM Extension System

Provides a sandbox for third-party extensions via WebAssembly.

### Design

- Extensions are compiled to WASM (targeting `wasm32-wasi`)
- `wasmtime` hosts and executes extension modules
- `wasmtime-wasi` provides WASI system call stubs
- Extensions communicate via a host-defined API (planned)

### Extension API (Planned)

```rust
// Extension registers lifecycle hooks
extension.on_file_save(|path| { /* ... */ });
extension.on_key_press(|key| { /* ... */ });
extension.register_command("my-command", |args| { /* ... */ });
```

---

## VibeCLI TUI Architecture

The TUI is built with Ratatui and follows an Elm-like architecture:

```
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

---

## VibeUI Tauri Architecture

Tauri bridges the React frontend and Rust backend via IPC.

### Data Flow (AI Chat Example)

```
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

---

## Testing Strategy

| Layer | Approach |
|-------|----------|
| `vibe-core` | Unit tests with `tempfile` for FS tests |
| `vibe-ai` | Mock HTTP server for provider tests |
| VibeCLI TUI | `tests.rs` in the `tui` module |
| VibeUI Tauri | E2E tests in `vibeui/e2e/` |
| TypeScript | `tsc --noEmit` type checking |
