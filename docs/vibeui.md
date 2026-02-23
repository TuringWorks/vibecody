---
layout: page
title: VibeUI Reference
permalink: /vibeui/
---

# VibeUI

**AI-powered desktop code editor built with Tauri 2 and Monaco.**

VibeUI provides a VS Code-like editing experience with a native Rust backend, Monaco Editor frontend, integrated AI chat, terminal, and Git panel.

---

## Architecture Overview

```
┌─────────────────────────────────────────┐
│           Frontend (React + TS)         │
│  Monaco Editor │ AI Chat │ Git Panel    │
│  Command Palette │ Terminal │ Sidebar   │
└──────────────────┬──────────────────────┘
                   │ Tauri IPC (invoke)
┌──────────────────▼──────────────────────┐
│          Tauri Rust Backend             │
│  Commands: open_file, save_file,        │
│            get_git_status, ai_chat, ... │
└──────────────────┬──────────────────────┘
                   │
    ┌──────────────┴───────────────────┐
    │         Rust Library Crates      │
    ├──────────────┬───────────────────┤
    │  vibe-core   │  vibe-ai          │
    │  vibe-lsp    │  vibe-extensions  │
    └──────────────┴───────────────────┘
```

---

## Getting Started

### Prerequisites

| Requirement | Version | Install |
|-------------|---------|---------|
| Rust | ≥ 1.75 stable | [rustup.rs](https://rustup.rs/) |
| Node.js | ≥ 18 | [nodejs.org](https://nodejs.org/) |
| npm | any | bundled with Node.js |
| Tauri prerequisites | v2 | [tauri.app/start/prerequisites](https://tauri.app/start/prerequisites/) |

On macOS, also ensure Xcode Command Line Tools are installed:
```bash
xcode-select --install
```

### Development Setup

```bash
cd vibeui

# Install JavaScript dependencies
npm install

# Start development server (hot reload for frontend + Rust backend)
npm run tauri dev
```

### Production Build

```bash
npm run tauri build
```

The installer is placed in `src-tauri/target/release/bundle/`.

---

## Features

### Editor

- **Monaco Editor** — same engine as VS Code; full syntax highlighting, IntelliSense, multi-cursor
- **Rope-based buffer** — built on `ropey` for efficient O(log n) edits on large files
- **File watching** — auto-detects external changes using `notify`
- **Multi-workspace** — open multiple folders simultaneously
- **Language detection** — automatic language mode from file extension

### AI Integration

The AI chat panel supports all five providers via the shared `vibe-ai` crate:

- **Ollama** (default, local) — no API key required
- **Anthropic Claude**
- **OpenAI**
- **Google Gemini**
- **xAI Grok**

Select the provider from the dropdown in the header. Provider configuration is handled through the settings UI (or environment variables — see [Configuration](../configuration/)).

### Git Panel

The Git panel provides a full Git workflow UI:

| Feature | Status |
|---------|--------|
| File status (M/N/D/R) with color coding | Done |
| Branch display in status bar | Done |
| Diff viewer | Done |
| Stage/unstage files | Done |
| Commit | Done |
| Push / Pull | Done |
| Branch list and switching | Done |
| Discard changes | Done |

Modified files appear **yellow** (M), new files **green** (N), deleted files **red** (D).

### Terminal

An integrated terminal panel using `portable-pty`:

- Full PTY — supports interactive programs (vim, htop, etc.)
- Powered by xterm.js on the frontend
- Accessible via the status bar "Show Terminal" button
- Keyboard shortcuts work correctly inside the terminal

### Command Palette

Press `Cmd+P` (macOS) / `Ctrl+P` (Windows/Linux) to open the Command Palette:

- Fuzzy file search (powered by `fuse.js`)
- Editor command list
- Quick navigation

### Theme System

- Dark and light themes
- Toggle via moon/sun icon in the status bar
- Theme persists across sessions (stored in localStorage)
- All UI elements (editor, panels, status bar) respect the active theme

---

## UI Components

| Component | File | Description |
|-----------|------|-------------|
| `App` | `src/App.tsx` | Root component, global state, layout |
| `AIChat` | `src/components/AIChat.tsx` | Streaming AI chat panel |
| `GitPanel` | `src/components/GitPanel.tsx` | Full Git workflow panel |
| `Terminal` | `src/components/Terminal.tsx` | xterm.js terminal integration |
| `CommandPalette` | `src/components/CommandPalette.tsx` | Fuzzy search command palette |
| `ThemeToggle` | `src/components/ThemeToggle.tsx` | Dark/light theme switcher |
| `Modal` | `src/components/Modal.tsx` | Reusable modal dialog |
| `MarkdownPreview` | `src/components/MarkdownPreview.tsx` | Rendered markdown preview |

---

## Tauri Commands (Backend API)

The React frontend communicates with the Rust backend using Tauri's `invoke()` IPC:

| Command | Description |
|---------|-------------|
| `open_file(path)` | Read file contents into editor |
| `save_file(path, content)` | Write editor contents to disk |
| `list_directory(path)` | List files/folders for sidebar |
| `get_git_status(path)` | Get branch + file status map |
| `git_commit(path, message, files)` | Stage and commit selected files |
| `git_push(path, remote, branch)` | Push to remote |
| `git_pull(path, remote, branch)` | Pull from remote |
| `get_git_diff(path, file)` | Get diff for a file |
| `get_git_history(path, limit)` | Get commit history |
| `ai_chat(provider, messages)` | Send messages to AI provider |
| `write_to_terminal(data)` | Send input to PTY |

---

## Rust Crates

### `vibe-core`

Foundational editor primitives:

| Module | Struct/Fn | Description |
|--------|-----------|-------------|
| `buffer` | `TextBuffer` | Rope-based text buffer with undo/redo, multi-cursor |
| `file_system` | `FileSystem` | Async open/save, file watching |
| `workspace` | `Workspace` | Multi-folder workspace management |
| `git` | `get_status`, `commit`, `push`, etc. | Git operations via `git2` |
| `search` | — | File and content search with `walkdir` + `regex` |
| `terminal` | — | PTY terminal via `portable-pty` |
| `diff` | `DiffEngine`, `DiffHunk` | Text diff via `similar` |
| `executor` | `CommandExecutor` | Sandboxed command execution |

### `vibe-ai`

AI abstraction layer:

| Module | Description |
|--------|-------------|
| `provider` | `AIProvider` trait, `Message`, `CodeContext`, `ProviderConfig` |
| `providers/ollama` | Ollama HTTP API (streaming) |
| `providers/claude` | Anthropic Claude API (streaming) |
| `providers/openai` | OpenAI Chat Completions API (streaming) |
| `providers/gemini` | Google Gemini API (streaming) |
| `providers/grok` | xAI Grok API (streaming) |
| `chat` | `ChatEngine` — session management |
| `completion` | `CompletionEngine` — inline code completion |

### `vibe-lsp`

Language Server Protocol client:

- JSON-RPC message framing via `jsonrpc-core` and `tokio-util`
- Async LSP server process management
- LSP types from `lsp-types`
- Foundation for go-to-definition, hover, diagnostics, completions

### `vibe-extensions`

WASM extension system:

- Powered by `wasmtime` and `wasmtime-wasi`
- VSCode-compatible extension API (planned)
- Extensions run in isolated WASM sandboxes

---

## Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| `Cmd/Ctrl + S` | Save file |
| `Cmd/Ctrl + P` | Command palette |
| `Cmd/Ctrl + Option/Alt + I` | Open DevTools |
| `Tab` | Indent selection (in editor) |

---

## Testing

```bash
# Rust unit tests
cargo test --workspace

# Specific crates
cargo test -p vibe-core
cargo test -p vibe-ai

# TypeScript type check
cd vibeui && npx tsc --noEmit

# End-to-end tests
cd vibeui/e2e && npm test
```

See [TESTING.md](https://github.com/vibecody/vibecody/blob/main/vibeui/TESTING.md) for manual testing checklist.

---

## Debugging

Open DevTools in the running app:
- **macOS**: `Cmd + Option + I`
- **Windows/Linux**: `Ctrl + Shift + I`

Or right-click anywhere and select **Inspect**.

See [DEBUG.md](https://github.com/vibecody/vibecody/blob/main/vibeui/DEBUG.md) for common troubleshooting steps.
