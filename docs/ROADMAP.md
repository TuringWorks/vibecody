---
layout: page
title: Roadmap & Implementation Plan
permalink: /roadmap/
---

# Fit-Gap Analysis & Feature Implementation Roadmap

**Date:** February 2026
**Scope:** VibeCLI vs. OpenAI Codex CLI, Anthropic Claude Code — VibeUI vs. Google Antigravity, Cursor, Windsurf

---

## 1. Competitive Landscape Summary

### VibeCLI Competitors

| Tool | Owner | Stack | Standout Capability |
|------|-------|-------|-------------------|
| **OpenAI Codex CLI** | OpenAI / TuringWorks fork | Rust + TypeScript | Full agent loop, OS sandbox, approval tiers, MCP |
| **Claude Code** | Anthropic | Node.js + CLI | Agentic multi-file edits, subagents, 300+ MCP integrations, hooks |

### VibeUI Competitors

| Tool | Owner | Stack | Standout Capability |
|------|-------|-------|-------------------|
| **Google Antigravity** | Google | Electron + Gemini | Agent-first IDE, Manager View (5 parallel agents), Artifacts, knowledge base |
| **Cursor** | Anysphere | Electron + VS Code fork | Tab model (next-action prediction), 8 parallel agents in git worktrees, 200k-token codebase indexing |
| **Windsurf** | Codeium | Electron + VS Code fork | Cascade agent with flow-awareness (tracks every edit/command), planning agent, memory system, checkpoints |

---

## 2. Current VibeCLI — Feature Inventory

| Feature | Status | Notes |
|---------|--------|-------|
| Multi-provider (17 providers) | ✅ Done | All 17 providers implemented |
| TUI (Ratatui) | ✅ Done | Chat, FileTree, DiffView, Agent screens |
| REPL mode (rustyline) | ✅ Done | History, tab completion, 14 slash commands |
| Git context injection | ✅ Done | Branch, status, diff in system prompt |
| `/apply` — single-file AI edits | ✅ Done | Shows diff, requires confirmation |
| `/exec` — AI-generated shell commands | ✅ Done | Confirmation gate |
| `!cmd` — direct shell execution | ✅ Done | Config-gated approval |
| TOML config (`~/.vibecli/config.toml`) | ✅ Done | Per-provider + safety settings |
| Syntax highlighting in REPL | ✅ Done | syntect |
| Streaming responses | ✅ Done | Token-by-token via CompletionStream; TUI + REPL |
| Agent loop (autonomous multi-step) | ✅ Done | plan→act→observe, 30-step max, `AgentLoop` |
| Structured tool use framework | ✅ Done | 7 tools: read/write/patch/bash/search/list/complete |
| Approval tiers (Suggest/AutoEdit/FullAuto) | ✅ Done | 3-tier; `--suggest`/`--auto-edit`/`--full-auto` flags |
| OS sandbox for command execution | ✅ Done | macOS `sandbox-exec`, Linux `bwrap` |
| Codebase indexing / semantic search | 🔧 Partial | Regex + heuristic symbol index; embeddings pending |
| Multi-file editing (batch apply) | ✅ Done | Agent WriteFile tool handles any number of files |
| AGENTS.md / project memory | ✅ Done | Loads VIBECLI.md / AGENTS.md / CLAUDE.md + global |
| MCP server integration | ✅ Done | JSON-RPC 2.0 stdio; `/mcp list`, `/mcp tools` |
| Non-interactive / CI mode | ✅ Done | `--exec` flag; JSON/Markdown report; exit codes 0-3 |
| Multimodal input (images/screenshots) | ✅ Done | `[image.png]` syntax; Claude + OpenAI vision |
| Trace / audit log | ✅ Done | JSONL per session; `/trace` + `/trace view <id>` |
| GitHub Actions integration | ✅ Done | `.github/actions/vibecli/action.yml` |

---

## 3. Current VibeUI — Feature Inventory

| Feature | Status | Notes |
|---------|--------|-------|
| Monaco Editor integration | ✅ Done | Full VS Code engine |
| Rope-based text buffer | ✅ Done | ropey |
| Async file I/O + file watching | ✅ Done | notify |
| Multi-workspace | ✅ Done | Multiple root folders |
| Git panel (status, diff, commit, push, pull) | ✅ Done | git2; stash, branch list/switch, history |
| Terminal panel (PTY) | ✅ Done | portable-pty + xterm.js |
| AI chat panel | ✅ Done | All 17 providers; streaming |
| Command palette | ✅ Done | fuse.js fuzzy search |
| Dark/light theme | ✅ Done | localStorage persistence |
| LSP client (completions, hover, go-to-def) | ✅ Done | Wired to Monaco; lazy-start per language |
| Extension system (WASM) | ✅ Done | Full wasmtime host; loads `~/.vibeui/extensions/*.wasm` |
| Inline AI completions (FIM) | ✅ Done | Monaco `registerInlineCompletionsProvider`; Ollama FIM format |
| Agent mode (autonomous multi-file edits) | ✅ Done | AgentPanel: steps, approval, streaming, events |
| @ context (reference files/symbols in chat) | ✅ Done | `@query` popup; file search + `@git` context |
| Flow-awareness (edit/command tracking) | ✅ Done | FlowTracker ring buffer; injected into AI context |
| Memory / rules system | ✅ Done | MemoryPanel; `.vibeui.md` + `~/.vibeui/rules.md` |
| Diff preview before AI apply | ✅ Done | Monaco DiffEditor; accept/reject; auto git stash |
| Checkpoint / undo AI session | ✅ Done | Backend (git stash) + CheckpointPanel UI |
| Trace / audit log (History panel) | ✅ Done | HistoryPanel; list + detail view; JSONL traces |
| Multimodal (screenshot in chat) | ✅ Done | Backend (Claude + OpenAI) + AIChat UI |
| Codebase indexing (semantic) | ✅ Done | Regex/heuristic + embedding-based vector search (Ollama/OpenAI) |
| Planning agent | ✅ Done | PlannerAgent; plan generation, approval, guided execution |
| Multi-agent parallel execution | ✅ Done | MultiAgentOrchestrator; git worktrees; ManagerView UI |
| Web context (@web) | ✅ Done | `@web:<url>` in chat/agent; fetch + HTML-strip; ContextPicker autocomplete |
| Artifacts (task lists, plans, recordings) | ✅ Done | ArtifactStore + ArtifactsPanel; annotations, async feedback |
| Voice input | ✅ Done | Web Speech API hook + 🎤 mic button in AIChat; pulse animation |
| Knowledge base (persistent snippets) | ✅ Done | MemoryPanel + SkillLoader; auto-activating skills |

---

## 4. Fit-Gap Matrix

### VibeCLI vs. Codex CLI / Claude Code

| Capability | VibeCLI | Codex CLI | Claude Code | Gap Priority |
|-----------|---------|-----------|-------------|-------------|
| Agent loop (plan→act→observe) | ✅ | ✅ | ✅ | Closed |
| Structured tool use | ✅ | ✅ | ✅ | Closed |
| Streaming TUI responses | ✅ | ✅ | ✅ | Closed |
| Multi-file batch edits | ✅ | ✅ | ✅ | Closed |
| Approval tiers (3 levels) | ✅ | ✅ | ✅ | Closed |
| Codebase indexing | ✅ | ✅ | ✅ | Closed |
| OS sandbox | ✅ | ✅ | partial | Closed |
| Project memory (AGENTS.md) | ✅ | ✅ | ✅ | Closed |
| MCP integration | ✅ | ✅ | ✅ | Closed |
| Multi-provider | ✅ | ✅ | partial | Fit (advantage) |
| Git context | ✅ | ✅ | ✅ | Fit |
| Config file | ✅ | ✅ | ✅ | Fit |
| Non-interactive/CI mode | ✅ | ✅ | ✅ | Closed |
| Multimodal input | ✅ | ✅ | ✅ | Closed |
| Trace/audit log | ✅ | ✅ | partial | Closed (advantage) |
| Hooks system | ✅ | ❌ | ✅ | Closed |
| Parallel multi-agent | ✅ | experimental | ✅ | Closed |
| Plan Mode | ✅ | ❌ | ✅ | Closed |
| Session resume | ✅ | ✅ | ✅ | Closed |
| Web search tool | ✅ | ✅ | ✅ | Closed |
| Shell environment policy | ✅ | ✅ | ❌ | Closed |
| Code review agent | ✅ | ✅ | ✅ | Closed |
| GitHub Actions | ✅ | ✅ | ✅ | Closed |
| OpenTelemetry | ✅ | ✅ | ❌ | Closed |

### VibeUI vs. Antigravity / Cursor / Windsurf

| Capability | VibeUI | Antigravity | Cursor | Windsurf | Gap Priority |
|-----------|--------|-------------|--------|----------|-------------|
| Inline AI completions | ✅ | ✅ | ✅ | ✅ | Closed |
| Agent mode (autonomous edits) | ✅ | ✅ | ✅ | ✅ | Closed |
| Diff review before apply | ✅ | ✅ | ✅ | ✅ | Closed |
| @ context system | ✅ | partial | ✅ | ✅ | Closed |
| Flow-awareness (tracking + injection) | ✅ | partial | partial | ✅ | Closed |
| Memory / rules | ✅ | ✅ | ✅ | ✅ | Closed |
| LSP integration | ✅ | ✅ | ✅ | ✅ | Closed |
| Multi-file AI editing | ✅ | ✅ | ✅ | ✅ | Closed |
| Extension system (WASM) | ✅ | ✅ | ✅ | ✅ | Closed |
| Trace / audit log | ✅ | ❌ | ❌ | ❌ | Closed (advantage) |
| Checkpoint / undo AI | ✅ | ✅ | partial | ✅ | Closed |
| Multimodal chat | ✅ | partial | ✅ | partial | Closed |
| Codebase indexing (semantic) | ✅ | ✅ | ✅ | ✅ | Closed |
| Planning agent | ✅ | ✅ | ✅ | ✅ | Closed |
| Parallel agents | ✅ | ✅ (async) | ✅ (8) | ✅ | Closed |
| Next-edit prediction (Tab/Supercomplete) | ✅ | partial | ✅ | ✅ | Closed |
| Manager View (orchestration UI) | ✅ | ✅ | ❌ | ❌ | Closed |
| GitHub PR review (BugBot) | ✅ | ❌ | ✅ | ❌ | Closed |
| Artifacts | ✅ | ✅ | ❌ | ❌ | Closed |
| Monaco Editor | ✅ | ✅ | ✅ | ✅ | Fit |
| Git panel | ✅ | ✅ | ✅ | ✅ | Fit |
| Terminal | ✅ | ✅ | ✅ | ✅ | Fit |
| Multi-provider | ✅ | ✅ | partial | partial | Fit (advantage) |
| Rust native backend | ✅ | partial | ❌ | ❌ | **Differentiator** |
| Local/private AI (Ollama) | ✅ | ❌ | partial | partial | **Differentiator** |
| Open source | ✅ | ❌ | ❌ | ❌ | **Differentiator** |

---

## 5. Differentiators to Exploit

VibeCody has unique advantages to lean into:

1. **Full Rust backend** — lower memory, faster startup, native performance vs. Electron apps
2. **Ollama first-class** — Cursor/Windsurf treat local models as afterthoughts; VibeCody should be the best local-AI dev tool
3. **Monorepo synergy** — VibeCLI and VibeUI share the same `vibe-ai` and `vibe-core`; agent work done once applies both
4. **Privacy by design** — no telemetry, no cloud indexing, fully local option
5. **Open source** — full transparency, extensibility, self-hostable

---

## 6. Implementation Plan

Organized into 5 phases. Each phase builds on the previous and targets specific gap areas.

> **Status:** All 9 phases (1–5 in this document, 6–9 in [ROADMAP-v2](../roadmap-v2/)) are **complete** as of February 2026. VibeCody now has feature parity with Codex CLI, Claude Code, Cursor, Windsurf, and Antigravity across all critical capabilities.

---

### Phase 1 — Agent Foundation ✅ Complete

**Goal:** Give VibeCLI a real agent loop with streaming, tool use, and approval tiers. This is the most critical gap — without it, VibeCLI is just a chat wrapper.

#### 1.1 Streaming TUI Responses

**Crate:** `vibecli-cli/src/tui/`
**Why:** Currently AI responses appear all at once; competitors stream token-by-token.

- In `mod.rs`: replace `llm.chat()` calls with `llm.stream_chat()` in the TUI event loop
- Add `TuiMessage::AssistantChunk(String)` variant to accumulate streaming tokens
- Render partial message with a blinking cursor indicator in `ui.rs`
- Wire up `CompletionStream` → `tokio::spawn` → `mpsc::Sender<AppEvent::Chunk(String)>`

**Files:** `tui/mod.rs`, `tui/app.rs`, `tui/ui.rs`
**Estimate:** 3 days

---

#### 1.2 Tool Use Framework (`vibe-ai`)

**Crate:** `vibeui/crates/vibe-ai/`
**Why:** All competitors give the LLM structured tools. Without this, no agent loop is possible.

Add to `vibe-ai`:

```rust
// src/tools.rs
pub enum ToolCall {
    ReadFile { path: String },
    WriteFile { path: String, content: String },
    ApplyPatch { path: String, patch: String },
    BashCommand { command: String },
    SearchFiles { query: String, glob: Option<String> },
    ListDirectory { path: String },
    GetGitStatus,
    GetGitDiff { file: Option<String> },
}

pub struct ToolResult {
    pub tool: String,
    pub output: String,
    pub success: bool,
}

pub trait ToolExecutor: Send + Sync {
    async fn execute(&self, call: &ToolCall) -> Result<ToolResult>;
}
```

- Implement `VibeTool` executor in `vibe-core` that dispatches each variant
- Extend `AIProvider` trait with `chat_with_tools()` that sends tools in the provider's native format (OpenAI function calling, Claude tool use, Ollama tool use)
- Parse tool call responses from each provider

**Files:** `vibe-ai/src/tools.rs` (new), `vibe-ai/src/provider.rs`, each `providers/*.rs`
**Estimate:** 1 week

---

#### 1.3 Agent Loop (`vibe-ai`)

**Why:** The core of Codex CLI and Claude Code — an autonomous plan-act-observe cycle.

```rust
// src/agent.rs
pub struct AgentLoop {
    provider: Arc<dyn AIProvider>,
    tools: Arc<dyn ToolExecutor>,
    approval: ApprovalPolicy,
    max_steps: usize,
}

pub enum ApprovalPolicy {
    Suggest,     // Show every action, require y/N
    AutoEdit,    // Auto-apply file patches, prompt for commands
    FullAuto,    // Execute everything autonomously
}

impl AgentLoop {
    pub async fn run(&self, task: &str, context: &AgentContext) -> Result<AgentResult> {
        // 1. Build system prompt with tools + context
        // 2. Loop: LLM → parse tool calls → approve → execute → feed result back
        // 3. Stop on: task_complete tool, max_steps, error
    }
}
```

- VibeCLI TUI: add `/agent <task>` command that invokes `AgentLoop`
- Show a live "action feed" panel listing each step as it executes
- Wire `ApprovalPolicy` to existing safety config + add `--auto` / `--suggest` / `--full-auto` CLI flags

**Files:** `vibe-ai/src/agent.rs` (new), `vibecli-cli/src/main.rs`, `vibecli-cli/src/tui/`
**Estimate:** 1 week

---

#### 1.4 Approval Tiers (3-level)

**Why:** Codex CLI's most visible safety feature; binary approve/deny is not enough.

- `Suggest` (default): every file write and command shows diff/preview, requires `y`
- `AutoEdit`: file patches auto-applied; bash commands require approval
- `FullAuto`: all actions execute (only in sandbox or explicit opt-in)

Extend config:

```toml
[safety]
approval_policy = "suggest"   # suggest | auto-edit | full-auto
sandbox = true                # enable OS-level sandbox when full-auto
```

CLI flags: `--suggest`, `--auto-edit`, `--full-auto`

**Files:** `vibecli-cli/src/config.rs`, `vibe-ai/src/agent.rs`
**Estimate:** 2 days

---

#### 1.5 Multi-File Batch Edits

**Why:** `/apply` only handles one file. Real agent work touches many files.

- Agent tool `WriteFile` + `ApplyPatch` already handles this at the tool level
- Add `BatchApply` confirmation UI in TUI: show all proposed changes as a unified diff across files, single y/N to accept all or file-by-file review
- Preserve undo: create a git stash before applying batch changes

**Files:** `vibecli-cli/src/tui/components/` (new `batch_diff.rs`), `vibe-core/src/git.rs`
**Estimate:** 3 days

---

### Phase 2 — Context Intelligence ✅ Complete

**Goal:** Make VibeCLI and VibeUI context-aware at the codebase level — the core of Cursor's competitive moat.

#### 2.1 Codebase Indexing Engine (`vibe-core`)

**Why:** Cursor indexes 200k tokens of codebase. Currently VibeCLI only injects a truncated git diff.

New module: `vibe-core/src/index/`

```rust
pub struct CodebaseIndex {
    // tree-sitter parsed symbol table
    symbols: HashMap<String, Vec<SymbolInfo>>,
    // file content cache with modification times
    file_cache: HashMap<PathBuf, (SystemTime, String)>,
    // optional: vector embeddings for semantic search
    embeddings: Option<EmbeddingStore>,
}

pub struct SymbolInfo {
    pub name: String,
    pub kind: SymbolKind,  // Function, Struct, Trait, Class, etc.
    pub file: PathBuf,
    pub line: usize,
    pub signature: String,
}
```

Implementation:

- Use `tree-sitter` + language grammars (Rust, TypeScript, Python, Go) to parse symbols
- Walk workspace with `walkdir`, skip `.gitignore` entries
- Incremental re-index on file-change events from `notify`
- Expose `search_symbols(query)` → ranked `Vec<SymbolInfo>`
- Expose `search_content(regex)` → `Vec<(PathBuf, line, snippet)>`
- For semantic search: embed code chunks using a local embedding model via Ollama (`/api/embeddings`) and store in an in-memory HNSW index (`instant-distance` crate)

**Files:** `vibe-core/src/index/` (new directory: `mod.rs`, `symbol.rs`, `content.rs`, `embeddings.rs`)
**Estimate:** 2 weeks

---

#### 2.2 Context Injection Upgrade

**Why:** Current context injection is naive — 2000 chars of git diff. Competitors provide full codebase understanding.

Replace the static diff injection in `tui/mod.rs` with a smart context builder:

```rust
pub struct ContextBuilder<'a> {
    index: &'a CodebaseIndex,
    git: &'a GitStatus,
    open_files: &'a [PathBuf],
    budget: usize,  // token budget
}

impl ContextBuilder<'_> {
    pub fn build_for_task(&self, task: &str) -> String {
        // 1. Always include: branch, changed files, full diff of changed files
        // 2. Include: symbols most relevant to the task (BM25 + semantic)
        // 3. Fill budget with: content of open files
        // 4. Truncate intelligently at symbol/function boundaries
    }
}
```

**Files:** `vibe-core/src/context.rs` (new), `vibecli-cli/src/tui/mod.rs`
**Estimate:** 3 days

---

#### 2.3 AGENTS.md / Project Memory

**Why:** Claude Code uses CLAUDE.md; Codex uses AGENTS.md for persistent project-specific instructions.

- On startup, look for `AGENTS.md` or `VIBECLI.md` in CWD and parent directories
- Inject contents as the first system message (before git context)
- VibeCLI command: `/memory edit` — opens `VIBECLI.md` in `$EDITOR`
- VibeCLI command: `/memory show` — prints current memory
- Support tiered memory: global (`~/.vibecli/memory.md`) → repo → directory

**Files:** `vibecli-cli/src/memory.rs` (new), `vibecli-cli/src/main.rs`, `vibecli-cli/src/tui/mod.rs`
**Estimate:** 2 days

---

#### 2.4 @ Context System (VibeUI)

**Why:** Cursor's most-loved UX feature — `@file`, `@symbol`, `@web`, `@docs` in the chat box.

In `vibeui/src/components/AIChat.tsx`:

- Detect `@` in the input box and open a fuzzy-search popup
- Options: `@file:<path>`, `@symbol:<name>`, `@web:<url>`, `@git:diff`, `@git:history`
- Inject the referenced content into the message before sending
- Backend: Tauri commands `search_files_for_context`, `get_symbol_context`, `fetch_url_content`

**Files:** `vibeui/src/components/AIChat.tsx`, `vibeui/src/components/ContextPicker.tsx` (new), `vibeui/src-tauri/src/commands/context.rs` (new)
**Estimate:** 1 week

---

### Phase 3 — Inline Intelligence ✅ Complete

**Goal:** Wire up LSP and inline AI completions in VibeUI to match Cursor/Windsurf's core editor experience.

#### 3.1 LSP Client — Wire to Monaco

**Why:** The LSP stub exists; it needs to connect to Monaco for real editor intelligence.

Complete `vibe-lsp`:

- Spawn language server process (e.g., `rust-analyzer`, `typescript-language-server`, `pyright`)
- Bridge LSP `textDocument/completion`, `textDocument/hover`, `textDocument/definition`, `textDocument/publishDiagnostics` → Tauri events → Monaco `registerCompletionItemProvider`, `registerHoverProvider`, `setModelMarkers`
- Language server discovery: look for executables in PATH; show install prompt if missing
- Auto-start LSP on file open based on language detection

**Files:** `vibe-lsp/src/client.rs` (complete), `vibe-lsp/src/bridge.rs` (new), `vibeui/src-tauri/src/commands/lsp.rs` (new), `vibeui/src/App.tsx`
**Estimate:** 2 weeks

---

#### 3.2 Inline AI Completions

**Why:** Cursor's Tab model is the #1 reason developers pay for it.

Implementation strategy:

- Wire `CompletionEngine` from `vibe-ai` to Monaco's `registerInlineCompletionsProvider`
- Debounce: trigger completion 300ms after the user stops typing
- `CodeContext` is built from: Monaco cursor position, surrounding 1000 chars prefix/suffix, active file language
- Render ghost text (grayed-out suggestion) in Monaco
- Accept with `Tab`, dismiss with `Escape`
- For local mode (Ollama): use FIM (fill-in-the-middle) format with `<|fim_prefix|>`, `<|fim_suffix|>`, `<|fim_middle|>` tokens
- For cloud models: use standard prefix+suffix prompt

**Files:** `vibeui/src-tauri/src/commands/completion.rs` (new), `vibeui/src/App.tsx`, `vibeui/crates/vibe-ai/src/completion.rs`
**Estimate:** 1 week

---

#### 3.3 Flow Awareness Engine (VibeUI)

**Why:** Windsurf's key differentiator — Cascade knows everything you've done. Replicate this.

New Tauri event bus: `FlowTracker`

Track and persist:

- Files opened/closed (with timestamps)
- Files edited (which lines)
- Terminal commands run (command + exit code)
- Clipboard content (on focus events, opt-in)
- Recent AI chat exchanges

Expose as context to AI agent:

```rust
pub struct FlowContext {
    pub recently_viewed: Vec<(PathBuf, Instant)>,
    pub recently_edited: Vec<(PathBuf, Vec<Range>)>,
    pub recent_commands: Vec<(String, i32)>,   // (command, exit_code)
    pub current_file: Option<PathBuf>,
    pub cursor_position: Option<Position>,
}
```

This gets injected into every AI request to give the model full awareness of what the developer is doing.

**Files:** `vibeui/src-tauri/src/flow.rs` (new), `vibeui/src/App.tsx`
**Estimate:** 1 week

---

#### 3.4 Diff Review Before AI Apply (VibeUI)

**Why:** AI edits currently applied without review — a critical trust gap.

- Any AI-proposed file change goes through a `DiffReview` modal: unified diff with syntax highlighting, accept/reject per hunk
- Before applying: create git stash automatically (silent, named `vibeui-pre-ai-TIMESTAMP`)
- After applying: show "Changes applied — Undo all" button that pops the stash

**Files:** `vibeui/src/components/DiffReview.tsx` (new), `vibeui/src-tauri/src/commands/git.rs`
**Estimate:** 3 days

---

### Phase 4 — Agentic Editor ✅ Complete

**Goal:** Make VibeUI a full agentic IDE — matching Antigravity's Manager View and Cursor's Composer.

#### 4.1 Agent Mode in VibeUI

**Why:** The gap between VibeUI (chat panel) and Cursor/Windsurf (full agent) is the biggest competitive delta.

- New "Agent" tab in the AI panel (alongside "Chat")
- User describes a high-level task: "Add OAuth2 login to the Express app"
- Agent uses the tool framework from Phase 1 to:
  1. Read relevant files via `ReadFile` tool
  2. Search for symbols via `SearchFiles`
  3. Plan a list of changes (shown as a todo list, à la Windsurf)
  4. Execute changes file by file with diff preview
  5. Run tests via `BashCommand` to verify
  6. Report result

- Show a live "Steps" panel listing each action with status (pending/in-progress/done/error)
- Each step is expandable to show tool input/output

**Files:** `vibeui/src/components/AgentPanel.tsx` (new), `vibeui/src-tauri/src/agent.rs` (new), `vibeui/crates/vibe-ai/src/agent.rs`
**Estimate:** 2 weeks

---

#### 4.2 Memory / Rules System (VibeUI)

**Why:** Both Cursor (`.cursorrules`) and Windsurf (Cascade Memories) have persistent AI instructions.

- Support `.vibeui.md` in workspace root as project-level AI instructions
- Global rules in `~/.vibeui/rules.md`
- Settings panel: "AI Rules" tab for editing rules inline
- Cascade-style auto-memory: after each AI session, offer to save key decisions as a memory snippet
- Knowledge base: searchable store of code snippets and past solutions, surfaced automatically in context

**Files:** `vibeui/src/components/MemoryPanel.tsx` (new), `vibeui/src-tauri/src/memory.rs` (new)
**Estimate:** 1 week

---

#### 4.3 Checkpoint System (VibeUI)

**Why:** Windsurf's checkpoints let you rewind the entire AI session.

- Before any agent action: create a named git snapshot (`git stash push -m "vibe-checkpoint-N"`)
- Show checkpoint history in a timeline panel
- "Restore to checkpoint N" — pops stash, restores file state
- Checkpoints are auto-created at: session start, before each agent step

**Files:** `vibeui/src/components/CheckpointPanel.tsx` (new), `vibeui/src-tauri/src/checkpoint.rs` (new), `vibe-core/src/git.rs`
**Estimate:** 4 days

---

#### 4.4 Planning Agent (two-level)

**Why:** Windsurf separates a "planner" (long-horizon) from an "executor" (single-step). This dramatically improves complex task performance.

Implement in `vibe-ai/src/planner.rs`:

```rust
pub struct PlannerAgent {
    planner_model: Arc<dyn AIProvider>,  // frontier model for planning
    executor_model: Arc<dyn AIProvider>, // fast model for execution steps
}

pub struct Plan {
    pub goal: String,
    pub steps: Vec<PlanStep>,
}

pub struct PlanStep {
    pub description: String,
    pub estimated_files: Vec<PathBuf>,
    pub status: StepStatus,
}
```

- Planner LLM: generates the full plan as structured JSON
- For each step: executor LLM performs the actual tool calls
- Planner re-evaluates after each step completes (adaptive planning)
- UI: plan shown as a todo list at the top of the Agent panel; steps update in real time

**Files:** `vibe-ai/src/planner.rs` (new), `vibeui/src/components/AgentPanel.tsx`
**Estimate:** 1 week

---

#### 4.5 Multi-Agent Parallel Execution

**Why:** Cursor runs 8 parallel agents; Antigravity runs 5. This is the throughput multiplier.

- VibeCLI: `vibecli --agent <task> --parallel N` spawns N sub-processes, each a `AgentLoop` on a git worktree
- VibeUI: "Parallel Agents" view in the Manager tab — spawn up to 5 agents on different tasks simultaneously
- Each agent operates on an isolated git worktree (no conflicts)
- Results merged: show diff comparison of each agent's output, user picks winner or merges

**Files:** `vibe-ai/src/multi_agent.rs` (new), `vibeui/src/components/ManagerView.tsx` (new), `vibe-core/src/git.rs` (add worktree support)
**Estimate:** 2 weeks

---

### Phase 5 — Ecosystem & Polish ✅ Complete

**Goal:** Close the remaining gaps, ship differentiating features, and establish the open ecosystem.

#### 5.1 MCP (Model Context Protocol) Integration

**Why:** Claude Code has 300+ MCP integrations; VibeCLI/VibeUI have zero.

- Implement MCP client in `vibe-ai/src/mcp.rs` — JSON-RPC 2.0 over stdio or SSE
- MCP servers auto-discovered from config:

  ```toml
  [[mcp_servers]]
  name = "github"
  command = "npx @modelcontextprotocol/server-github"

  [[mcp_servers]]
  name = "postgres"
  command = "npx @modelcontextprotocol/server-postgres"
  args = ["postgresql://localhost/mydb"]
  ```

- MCP tools exposed to the agent alongside built-in tools
- MCP resources (e.g., database schema, API docs) injected into context

**Files:** `vibe-ai/src/mcp.rs` (new), `vibecli-cli/src/config.rs`, `vibeui/src-tauri/src/mcp.rs` (new)
**Estimate:** 1.5 weeks

---

#### 5.2 OS Sandbox for Command Execution

**Why:** Codex CLI uses Apple Seatbelt + Linux seccomp. VibeCLI runs commands unrestricted.

- macOS: wrap command execution in `sandbox-exec` with a restricted profile (no network, write only to CWD)
- Linux: use `bwrap` (bubblewrap) for namespace isolation
- Windows: use `Job Objects` for process isolation
- `FullAuto` mode requires sandbox OR explicit `--no-sandbox` flag

**Files:** `vibe-core/src/executor.rs`, `vibecli-cli/src/config.rs`
**Estimate:** 1 week

---

#### 5.3 Non-Interactive / CI Mode (VibeCLI)

**Why:** Codex CLI supports `codex exec` for automation pipelines.

```bash
vibecli exec "Add docstrings to all public functions in src/" --auto-edit --output report.md
vibecli exec "Fix all clippy warnings" --full-auto --sandbox
```

- No TUI, no user prompts (except in `suggest` mode which fails with error)
- Writes a structured JSON/markdown report of all actions taken
- Exit codes: 0 (success), 1 (partial), 2 (failed), 3 (approval required)
- GitHub Actions marketplace action: `vibecody/vibecli-action@v1`

**Files:** `vibecli-cli/src/main.rs`, `vibecli-cli/src/ci.rs` (new), `.github/actions/vibecli/` (new)
**Estimate:** 1 week

---

#### 5.4 Multimodal Input (VibeCLI + VibeUI)

**Why:** Cursor and Codex CLI support pasting screenshots for visual debugging.

- VibeCLI: detect image paths in input (`/chat [image.png] explain this error`)
- VibeUI: drag-and-drop or paste image into chat; encode as base64 and send with the message
- Providers: Claude and OpenAI support vision natively; add image encoding to those providers

**Files:** `vibe-ai/src/provider.rs` (add `ImageContent` to `Message`), `vibe-ai/src/providers/claude.rs`, `vibe-ai/src/providers/openai.rs`, `vibeui/src/components/AIChat.tsx`
**Estimate:** 4 days

---

#### 5.5 Extension System (VibeUI) — Complete

**Why:** The wasmtime stub exists. Complete it so third parties can extend VibeUI.

Define the extension host API:

```rust
// Host functions exposed to WASM extensions
pub trait ExtensionHost {
    fn register_command(&self, name: &str, handler: Box<dyn Fn(&[&str]) -> Result<String>>);
    fn on_file_save(&self, handler: Box<dyn Fn(&Path)>);
    fn on_text_change(&self, handler: Box<dyn Fn(&Path, &str)>);
    fn read_file(&self, path: &Path) -> Result<String>;
    fn write_file(&self, path: &Path, content: &str) -> Result<()>;
    fn show_notification(&self, message: &str);
    fn get_ai_completion(&self, prompt: &str) -> Result<String>;
}
```

- Extensions loaded from `~/.vibeui/extensions/*.wasm`
- Extension marketplace page on the docs site
- Example extensions: `prettier-format.wasm`, `rustfmt-on-save.wasm`

**Files:** `vibe-extensions/src/host.rs` (complete), `vibe-extensions/src/api.rs` (new), `vibeui/src-tauri/src/extensions.rs`
**Estimate:** 2 weeks

---

#### 5.6 Trace / Audit Log

**Why:** Codex CLI records every action for inspection and debugging.

- Agent loop writes a structured JSONL trace: `~/.vibecli/traces/<timestamp>.jsonl`
- Each entry: `{ timestamp, step, tool, input, output, duration_ms, approved_by }`
- VibeCLI command: `/trace` — lists recent traces
- VibeCLI command: `/trace view <id>` — renders trace as a human-readable timeline in TUI
- VibeUI: "History" panel showing recent agent sessions with expandable trace

**Files:** `vibe-ai/src/trace.rs` (new), `vibecli-cli/src/tui/components/trace_view.rs` (new), `vibeui/src/components/HistoryPanel.tsx` (new)
**Estimate:** 3 days

---

## 7. Prioritized Feature Backlog

### ✅ Completed — Phases 1–2 (Agent Foundation + Context Intelligence)

| # | Feature | Addresses | Status |
|---|---------|-----------|--------|
| 1 | Streaming TUI responses | Codex, Claude Code | ✅ Done |
| 2 | Tool use framework (7 tools) | All | ✅ Done |
| 3 | Agent loop (plan→act→observe) | Codex, Claude Code | ✅ Done |
| 4 | Approval tiers (Suggest/AutoEdit/FullAuto) | Codex, Claude Code | ✅ Done |
| 5 | Multi-file batch edits | All | ✅ Done |
| 6 | Codebase indexing (regex/heuristic + embeddings) | Cursor, Windsurf | ✅ Done |
| 7 | Project memory (AGENTS.md / VIBECLI.md) | Codex, Claude Code | ✅ Done |
| 8 | Diff review before apply | All | ✅ Done |

### ✅ Completed — Phase 3 (Inline Intelligence)

| # | Feature | Addresses | Status |
|---|---------|-----------|--------|
| 9 | LSP in Monaco (completions, hover, go-to-def) | Cursor, Windsurf | ✅ Done |
| 10 | Inline AI completions (FIM) | Cursor, Windsurf | ✅ Done |
| 11 | @ context system | Cursor, Windsurf | ✅ Done |
| 12 | Flow-awareness engine (FlowTracker) | Windsurf | ✅ Done |

### ✅ Completed — Phases 4–5 (Agentic Editor + Ecosystem)

| # | Feature | Addresses | Status |
|---|---------|-----------|--------|
| 13 | Agent mode in VibeUI (AgentPanel) | Antigravity, Cursor | ✅ Done |
| 14 | Memory / rules (MemoryPanel) | Cursor, Windsurf | ✅ Done |
| 15 | Checkpoint system (backend + UI) | Windsurf | ✅ Done |
| 16 | MCP integration (JSON-RPC 2.0 stdio) | Claude Code, Codex | ✅ Done |
| 17 | OS sandbox (sandbox-exec / bwrap) | Codex | ✅ Done |
| 18 | CI mode (--exec, JSON/Markdown reports) | Codex, Claude Code | ✅ Done |
| 19 | Multimodal input (Claude + OpenAI vision) | Cursor, Claude Code | ✅ Done |
| 20 | Extension system (WASM wasmtime) | Cursor, Windsurf | ✅ Done |
| 21 | Trace / audit log (JSONL + HistoryPanel) | Codex | ✅ Done |
| 22 | Multi-agent parallel (git worktrees + ManagerView) | Cursor, Antigravity | ✅ Done |
| 23 | Planning agent (PlannerAgent) | Windsurf, Antigravity | ✅ Done |

### ✅ Completed — Phases 6–9 (see ROADMAP-v2)

| # | Feature | Addresses | Status |
|---|---------|-----------|--------|
| 24 | Hooks system (events + shell + LLM handlers) | Claude Code | ✅ Done |
| 25 | Plan Mode (PlannerAgent) | Windsurf, Claude Code | ✅ Done |
| 26 | Parallel multi-agent + git worktrees | Cursor (8), Windsurf | ✅ Done |
| 27 | Embedding-based semantic indexing | Cursor, Windsurf | ✅ Done |
| 28 | Next-edit prediction (Tab/Supercomplete) | Cursor, Windsurf | ✅ Done |
| 29 | Checkpoint UI panel | Windsurf | ✅ Done |
| 30 | Session resume | Codex, Claude Code | ✅ Done |
| 31 | Web search tool | Codex | ✅ Done |
| 32 | GitHub PR review agent (BugBot equiv.) | Cursor BugBot | ✅ Done |
| 33 | Shell environment policy / Admin policy | Codex | ✅ Done |
| 34 | Skills system | Claude Code, Windsurf | ✅ Done |
| 35 | Artifacts panel | Antigravity | ✅ Done |
| 36 | OpenTelemetry | Codex | ✅ Done |
| 37 | GitHub Actions | Codex, Claude Code | ✅ Done |
| 38 | Manager View (parallel UI) | Antigravity | ✅ Done |
| 39 | VS Code extension | All | ✅ Done |
| 40 | Agent SDK (TypeScript) | Claude Code | ✅ Done |

---

## 8. Architecture Summary (All Phases Complete)

```text
vibecli-cli
├── REPL / TUI (streaming, hooks, /agent, /plan, /multi-agent, /review)
├── CI mode (--exec, --parallel, --review)
├── Server mode (vibecli serve — API for VS Code extension + SDK)
└── src/
    ├── ci.rs, review.rs, serve.rs, otel_init.rs
    └── hooks (config loading)

vibe-ai
├── provider.rs         (AIProvider trait + vision + tool use)
├── agent.rs            (plan→act→observe loop, approval tiers)
├── planner.rs          (PlannerAgent: plan generation + guided execution)
├── multi_agent.rs      (parallel agents on git worktrees)
├── hooks.rs            (HookRunner: command + LLM handlers, event bus)
├── skills.rs           (SkillLoader: auto-activating context snippets)
├── artifacts.rs        (Artifact types, annotation queue)
├── mcp.rs              (McpClient JSON-RPC 2.0)
├── tools.rs            (ToolCall enum + WebSearch + FetchUrl)
├── trace.rs            (JSONL audit + session resume)
├── policy.rs           (AdminPolicy: tool/path restrictions)
└── otel.rs             (OpenTelemetry span attributes)

vibe-core
├── index/
│   ├── mod.rs, symbol.rs  (tree-sitter symbol index)
│   └── embeddings.rs      (HNSW vector index, Ollama/OpenAI embeddings)
├── context.rs          (smart context builder: flow + semantic + git)
├── executor.rs         (sandboxed execution + shell env policy)
└── git.rs              (worktree: create, remove, merge)

vibe-extensions
└── loader.rs           (wasmtime WASM host)

vibeui (React + Tauri)
├── AgentPanel          (single-agent: steps, approval, artifacts)
├── ManagerView         (multi-agent: task board, worktrees, merge)
├── CheckpointPanel     (timeline, restore, auto-checkpoint)
├── ArtifactsPanel      (rich cards, annotations, async feedback)
├── MemoryPanel         (rules editor)
├── HistoryPanel        (trace viewer)
├── ContextPicker       (@ context popup)
├── GitPanel            (git + PR review)
└── Terminal, AIChat, CommandPalette, ThemeToggle

vscode-extension        (chat, inline completions, agent mode)
packages/agent-sdk      (TypeScript SDK: @vibecody/agent-sdk)
.github/actions/vibecli (GitHub Actions marketplace action)
```

---

## 9. Key Differentiators (Current)

With all phases complete, VibeCody's competitive positioning:

| Dimension | VibeCody | Cursor | Windsurf | Antigravity |
|-----------|---------|--------|----------|-------------|
| Rust native backend | ✅ | ❌ | ❌ | partial |
| Local AI first (Ollama) | ✅ | partial | partial | ❌ |
| Full agent loop | ✅ | ✅ | ✅ | ✅ |
| Codebase indexing (semantic) | ✅ | ✅ | ✅ | ✅ |
| Privacy (no telemetry) | ✅ | ❌ | ❌ | ❌ |
| Open source | ✅ | ❌ | ❌ | ❌ |
| CLI + GUI unified | ✅ | partial | ❌ | partial |
| MCP | ✅ | ✅ | partial | ✅ |
| Multi-provider (5+) | ✅ | partial | partial | partial |
| OS sandbox | ✅ | ❌ | ❌ | ❌ |
| Hooks system | ✅ | ❌ | ❌ | ❌ |
| Parallel agents | ✅ | ✅ (8) | ✅ | ✅ |
| Plan Mode | ✅ | ❌ | ✅ | ✅ |
| Skills system | ✅ | ❌ | ✅ | ❌ |
| Artifacts + Manager View | ✅ | ❌ | ❌ | ✅ |
| VS Code extension | ✅ | ✅ | ✅ | ✅ |
| Agent SDK | ✅ | ❌ | ❌ | ❌ |

The clearest win: **VibeCody is the only fully open-source, privacy-first, local-AI-capable development toolchain that works both in the terminal and as a desktop IDE, with hooks, parallel agents, artifacts, and a VS Code extension.**
