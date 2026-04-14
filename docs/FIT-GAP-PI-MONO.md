---
layout: page
title: "FIT-GAP: VibeCody vs Pi (pi-mono)"
permalink: /fit-gap-pi-mono/
---

# FIT-GAP Analysis — VibeCody vs Pi (pi-mono)

**pi-mono** is an open-source TypeScript AI agent harness by Mario Zechner (`badlogic`).  
Repo: `TuringWorks/pi-mono` | Site: `shittycodingagent.ai` | Stack: TypeScript, 7 npm packages, MIT

> This document evaluates pi feature-by-feature, scores VibeCody's current coverage, and identifies the highest-value capabilities to adopt.

---

## Pi Package Overview

| Package | Purpose |
|---------|---------|
| `pi-coding-agent` | Interactive coding agent CLI — the `pi` command |
| `pi-agent-core` | Agent runtime: tool calling, streaming, event bus, message queues |
| `pi-ai` | Unified LLM API: 20 providers, streaming, tool calling, OAuth |
| `pi-tui` | Terminal UI library: differential rendering, overlays, IME support |
| `pi-mom` | Slack bot: per-channel isolation, dual-log, events system |
| `pi-pods` | GPU pod management: vLLM deployment for self-hosted models |
| `pi-web-ui` | Web UI: IndexedDB, artifacts, attachments, CORS proxy |

---

## Feature-by-Feature Comparison

### 1. Session System

| Feature | Pi | VibeCody | Gap |
|---------|-----|----------|-----|
| Session persistence | JSONL per-session | SQLite via `ProfileStore` / `WorkspaceStore` | Even |
| **In-file tree branching** | Every entry has `id` + `parentId`; `/tree` command to navigate, fold, continue from any node — **no new files created** | Sessions are linear; branching creates new sessions | **GAP** |
| Compaction | Summarises older messages; accumulates file-operation history across prior compactions; extensions can intercept and replace | `context_pruning.rs` — sliding window + summary | Partial gap — file-operation-awareness missing |
| Custom JSONL entries | Extension-defined record types stored in session without polluting LLM context | No equivalent | **GAP** |
| Session export | `/export` → standalone HTML | Session export to JSONL/JSON | Partial gap — no standalone HTML export |
| Session sharing | `/share` → private GitHub Gist with HTML viewer link | No gist sharing | **GAP** |
| OSS session publishing | `pi-share-hf` pushes sessions to Hugging Face datasets | No equivalent | Gap (low priority) |
| Session filters | default / no-tools / user-only / labeled-only / all | No filter modes | Minor gap |
| Labels & bookmarks | `/label`, `/bookmark` within sessions | Bookmark in `bookmarks.rs` | Even |

---

### 2. Agent Execution Modes

| Feature | Pi | VibeCody | Gap |
|---------|-----|----------|-----|
| Interactive TUI | Full Ratatui-style TUI with streaming, inline images, thinking | Ratatui TUI with streaming | Even |
| Print mode (`-p`) | One-shot non-interactive, accepts piped stdin | `--prompt` flag / REPL one-shot | Even |
| **JSON/events mode** | Strict JSONL event stream (`assistant_start`, `tool_call`, `token_usage`, …) — designed for programmatic consumers | HTTP daemon emits events; no strict JSONL event mode on stdout | Gap |
| **RPC mode** | Bidirectional stdin/stdout JSONL protocol for embedding pi in non-Node.js processes; strict LF-only framing (avoids readline splitting on Unicode separators) | HTTP daemon (`--serve --port`) — TCP, not stdio | Gap — stdio RPC not available |
| SDK | `createAgentSession()` / `AgentSessionRuntime` — embeddable in any TypeScript app | `vibe-ai` crate is the Rust SDK; Tauri commands as IPC | Different approach (Rust vs TS), no TS SDK |

---

### 3. Tool System

| Feature | Pi | VibeCody | Gap |
|---------|-----|----------|-----|
| Built-in tools | `read`, `write`, `edit` (multi-edit + diff), `bash`, `grep`, `find`, `ls` | Same set plus docker, k8s, http, db tools | VibeCody ahead |
| **Parallel tool execution** | Default — tools within one assistant message execute concurrently after sequential `beforeToolCall` preflight | Sequential tool execution | **GAP** |
| `beforeToolCall` / `afterToolCall` hooks | Agent-level hooks; `beforeToolCall` can block; `afterToolCall` can mutate result | Hooks via `hook_abort.rs`; post-tool hooks run shell commands | Partial gap — Rust hooks don't mutate results in-process |
| **Pluggable tool operations** | `BashOperations` / `EditOperations` interfaces — extensions redirect built-in tool I/O to SSH, Docker, or any remote backend transparently | Tools run locally; SSH panel is separate | **GAP** — no unified pluggable I/O interface |
| `!!cmd` bash prefix | Runs bash; output excluded from LLM context | No equivalent | Gap |
| `!cmd` bash prefix | Runs bash; output included in LLM context | `!` prefix in REPL | Even |

---

### 4. Extension / Plugin System

| Feature | Pi | VibeCody | Gap |
|---------|-----|----------|-----|
| Extension loading | TypeScript modules from global `~/.pi/extensions/`, project `.pi/extensions/`, or npm/git packages | WASM extensions via `vibe-extensions` crate | Different approach — Pi TS vs VibeCody WASM |
| **30+ typed lifecycle events** | `session_init`, `agent_start`, `agent_end`, `tool_call`, `tool_result`, `model_change`, `before_provider_request`, `session_before_compact`, … | Post-tool hooks (shell); no typed in-process event bus | **GAP** |
| Custom tool registration | `registerTool()` with custom `renderCall`/`renderResult` TUI components | Tools added via WASM manifest | Partial gap — no custom TUI render |
| Slash commands | `registerCommand()` | `/skill` in REPL; custom REPL commands via `repl_macros.rs` | Partial even |
| CLI flags | `registerFlag()` | No equivalent | Gap |
| Keyboard shortcuts | `registerKeyboardShortcut()` | No equivalent | Gap |
| Block tool execution | `tool_call` handler can return `{ block: true }` | Hook exit code 2 blocks | Even (different mechanism) |
| Mutate tool arguments | In-place via `tool_call` event | No in-process mutation | Gap |
| Replace TUI components | `setEditorComponent()`, footer/header/status bar replacement, custom overlays | Fixed TUI layout; panels are VibeUI (desktop) | Gap |
| Package distribution | `pi install <npm/git/https>` | `vibe-extensions` WASM packages | Different approach |
| **`pi install` package system** | `pi` key in `package.json` declares extension/skill/prompt/theme paths; auto-discovers conventional directories | No install command for extensions | **GAP** |

---

### 5. Skills System

| Feature | Pi | VibeCody | Gap |
|---------|-----|----------|-----|
| Skill discovery | Walk parent dirs from cwd: `.pi/skills/`, `~/.pi/skills/`, npm package skills | Walk dirs from cwd: `.vibecli/skills/`, `~/.vibecli/skills/` | Even |
| Skills standard | Follows `agentskills.io` — `SKILL.md` frontmatter with `name`, `description`, `triggers` | Custom `SKILL.md` format with frontmatter | Even |
| Skill count | Community-driven; repo has reference skills | 702+ built-in skills | **VibeCody ahead** |
| On-demand loading | Agent reads skill list first, fetches full `SKILL.md` only when deciding to use | Same pattern | Even |
| **Self-writing skills** (via mom) | Mom writes her own skill scripts into workspace; discovers by scanning for `SKILL.md` | No agent-driven skill authoring | Gap |

---

### 6. Message Queue / Steering

| Feature | Pi | VibeCody | Gap |
|---------|-----|----------|-----|
| **Steering messages** | Independent queue; injects between tool calls during an active agent turn; `"one-at-a-time"` or `"all"` mode | No steering queue | **GAP** |
| **Follow-up messages** | Separate queue; injects after agent finishes all work | No follow-up queue | **GAP** |
| Message injection API | `agent.steer(msg)` / `agent.followUp(msg)` | No equivalent | **GAP** |

---

### 7. Provider & Auth

| Feature | Pi | VibeCody | Gap |
|---------|-----|----------|-----|
| Provider count | 20 providers | 18 providers | Slight gap |
| **OAuth login** | Claude Pro/Max, ChatGPT Plus/Pro (Codex), GitHub Copilot, Google Gemini CLI, Google Antigravity — no API keys needed | API key-based only (`ProfileStore`) | **GAP** |
| Extended prompt cache | `PI_CACHE_RETENTION=long` — 1h Anthropic, 24h OpenAI | Standard cache TTL | Gap |
| **Cross-provider context handoff** | Serializable `Context` type (system prompt + messages + tools) can be handed verbatim to a different provider mid-session | No mid-session provider swap with context preservation | **GAP** |
| **Streaming partial tool args** | `toolcall_delta` events include progressively parsed partial JSON during streaming (show filename before content arrives) | Full tool call delivered at completion | **GAP** |
| **Thinking level abstraction** | `off`, `minimal`, `low`, `medium`, `high`, `xhigh` with per-level token budgets; shorthand `pi --model sonnet:high` | Extended thinking toggle; no granular levels | Gap |
| **TypeBox + AJV validation** | Tool parameters defined with TypeBox; auto-validates before execution | JSON Schema (manual) | Gap |
| OAuth credential type | `OAuthLoginCallbacks` + `OAuthCredentials` standardized type | No OAuth abstraction | Gap |
| `faux.ts` fake provider | Deterministic fake provider for tests without API calls | `mock_provider` in `vibe-ai` | Even |
| Dynamic API key callback | `getApiKey` callback per-call for expiring OAuth tokens | Static key at provider init | Gap |
| vLLM-specific build variants | `release`, `nightly`, `gpt-oss` (Responses API) with auto tool-call-parser selection per model | Generic OpenAI-compat endpoint | Gap |

---

### 8. Terminal UI

| Feature | Pi | VibeCody | Gap |
|---------|-----|----------|-----|
| **Differential rendering** | Three-strategy: first-render, width-changed, normal (cursor to first changed line only) wrapped in CSI 2026 synchronized output | Ratatui full-frame render with diff | Partial — CSI 2026 sync output not used |
| **IME / CJK input** | `CURSOR_MARKER` APC escape positions hardware cursor so IME candidate windows appear correctly | No IME support documented | **GAP** |
| **Inline images** | Kitty and iTerm2 protocols with dimension parsing and text fallback | No inline images in TUI | **GAP** |
| **Bracketed paste collapse** | Pastes >10 lines collapsed to `[paste #1 +N lines]` marker; full content accessible | No paste collapse | Gap |
| Overlays | Rich positioning (absolute, percentage, anchor), `minWidth`, `maxHeight`, `nonCapturing`, responsive `visible` callback | Panels in VibeUI (desktop); no TUI overlays | Context difference |
| `Ctrl+]` character jump | Jump cursor to next occurrence of typed character | No equivalent | Minor gap |
| **Virtual terminal for tests** | `VirtualTerminal` via `@xterm/headless` for TUI testing without a real terminal | No TUI test harness | **GAP** |
| **Debug stream log** | `PI_TUI_WRITE_LOG` captures raw ANSI stream to file | No equivalent | Minor gap |
| ANSI-aware text utils | `visibleWidth()`, `truncateToWidth()`, `wrapTextWithAnsi()` | Ratatui handles this | Even |

---

### 9. Slack / Messaging Bot (pi-mom)

| Feature | Pi | VibeCody | Gap |
|---------|-----|----------|-----|
| Slack integration | Full per-channel agent (`pi-mom`) | `channel_daemon.rs` — Slack/Discord/GitHub | Even |
| **Dual-log architecture** | `log.jsonl` (append-only, never compacted) + `context.jsonl` (compacted, LLM context) — agent can `grep log.jsonl` for history beyond context window | Single context file per channel | **GAP** |
| **Per-channel isolation** | Separate dir per Slack channel/DM: `log.jsonl`, `context.jsonl`, `MEMORY.md`, `attachments/`, `scratch/`, `skills/` | Per-workspace isolation | Partial gap — per-channel dirs not separate |
| **Events system** | Three event types: immediate (file-create), one-shot (ISO8601 timestamp + tz), periodic (cron + IANA tz); max 5 queued per channel | `task_scheduler.rs` + cron; not per-channel | Partial gap |
| `[SILENT]` response | Suppresses Slack output for empty periodic events | No equivalent | Gap |
| **Self-managing skills** | Bot writes its own skill scripts to `skills/`; discovers by scanning `SKILL.md` files | No agent-driven skill authoring | Gap |
| Docker sandbox mode | `--sandbox=docker:<name>` per bot instance | `sandbox_windows.rs` with ACL | Even |
| Artifacts server | Serves HTML/JS visualizations with live reload | No equivalent in bot context | Gap |
| Thread-based tool output | Clean main message; verbose tool results in thread | No thread separation | Gap |

---

### 10. GPU / vLLM Ops (pi-pods)

| Feature | Pi | VibeCody | Gap |
|---------|-----|----------|-----|
| **GPU pod deployment** | `pi-pods` CLI: deploy vLLM on RunPod/Lambda/Vast.ai with VRAM pre-flight check | No GPU pod management | **GAP** |
| **Tool-call-parser auto-config** | Selects correct vLLM `--tool-call-parser` per model family (hermes, qwen3_coder, glm4_moe) | No equivalent | **GAP** |
| Multi-GPU assignment | Auto-assigns models to distinct GPUs without manual CUDA device config | No equivalent | **GAP** |
| Pre-baked model configs | Known models have validated vLLM flag sets | No equivalent | Gap |

---

### 11. Web UI Concepts (pi-web-ui)

| Feature | Pi | VibeCody | Gap |
|---------|-----|----------|-----|
| **Artifacts system** | `ArtifactsPanel`: HTML, SVG, MD, JSON, image, PDF, DOCX, XLSX rendering; persisted as `ArtifactMessage` in session | `DocumentViewer`, `MarkdownPreview`, separate viewers | Partial gap — not unified per-session artifacts |
| **Attachment processing** | `loadAttachment()`: File/URL/ArrayBuffer; PDF/DOCX/XLSX/PPTX text extraction; preview images; base64 for LLM | `attachment_support` in CLAUDE.md | Even |
| **JavaScript REPL tool** | `createJavaScriptReplTool()` with sandboxed browser execution; pluggable runtimes for files/artifacts | `ScriptPanel` with multiple languages | Partial gap — no pluggable sandbox runtime |
| CORS proxy | `createStreamFn()` reads proxy from storage; auto-applied for OAuth tokens | No built-in CORS proxy | Gap |
| Custom message renderers | `registerMessageRenderer(role, renderer)` | No equivalent | Gap |
| i18n | `i18n()` / `setLanguage()` / `translations` map | No i18n | Gap |
| Storage | IndexedDB (`AppStorage`) — sessions, keys, settings, custom providers | SQLite via `ProfileStore`/`WorkspaceStore` (superior — encrypted) | **VibeCody ahead** |

---

## Priority Gap Summary

### P0 — High-value, implement now

| # | Feature | Why VibeCody should adopt |
|---|---------|--------------------------|
| 1 | **In-file session tree branching** (`id`/`parentId` JSONL) | Non-destructive, no file proliferation; `/tree` navigation enables powerful "continue from any point" workflows — replaces linear sessions |
| 2 | **Parallel tool execution** | Most agent turns fire multiple independent tools (read file A + read file B); parallelism reduces latency significantly |
| 3 | **OAuth login for Claude Pro/Max, Copilot, Gemini CLI** | Huge DX improvement — users with subscriptions don't need API keys; expands accessible user base |
| 4 | **Steering message queue** | Inject guidance between tool calls in real time — critical for interactive agent steering without interrupting the turn |
| 5 | **Cross-provider context handoff** | Serializable context that can move mid-session to a different provider (cost routing, fallback, capability gap) |

### P1 — High-value, implement next sprint

| # | Feature | Why |
|---|---------|-----|
| 6 | **Pluggable tool I/O (`BashOperations`/`EditOperations`)** | Enables SSH remoting, container redirects, cloud VM agents — all transparently using existing built-in tools |
| 7 | **Streaming partial tool call args (`toolcall_delta`)** | Better perceived performance; show file being written as it streams rather than after completion |
| 8 | **Dual-log architecture** for channel daemon | Infinite searchable history (`log.jsonl`) + bounded LLM context (`context.jsonl`); grep-for-history pattern |
| 9 | **Thinking level abstraction** (`off`/`minimal`/`low`/`medium`/`high`/`xhigh` with token budgets) | Exposes granular reasoning control; `--model sonnet:high` shorthand is excellent UX |
| 10 | **GPU pod management (pi-pods concepts)** | As self-hosted LLM usage grows, vLLM VRAM pre-flight, model config validation, and multi-GPU assignment are enterprise must-haves |

### P2 — Medium-value

| # | Feature | Why |
|---|---------|-----|
| 11 | **IME / CJK input (`CURSOR_MARKER`)** | Required for East Asian users; missing = accessibility failure |
| 12 | **Inline images in TUI** (Kitty/iTerm2) | Increasingly expected in modern terminals; important for image-gen and visual output |
| 13 | **Session export to standalone HTML + GitHub Gist share** | Session sharing is a core growth/feedback loop |
| 14 | **RPC mode (stdin/stdout JSONL)** | Enables embedding VibeCLI in non-Rust processes (Python scripts, editors, CI) without HTTP overhead |
| 15 | **Bracketed paste collapse** | Safety UX; prevents 200-line accidental paste destroying context |
| 16 | **Typed lifecycle event bus** (30+ event types) | Current hook system is shell-subprocess-only; in-process event bus enables WASM extension reactions |
| 17 | **`!!cmd` bash prefix** (output excluded from LLM context) | Common workflow: run a command to check something without polluting context |
| 18 | **Compaction with file-operation history** | Ensures compacted summaries mention which files were touched — prevents agent amnesia after compaction |

### P3 — Lower priority / already covered

| # | Feature | Note |
|---|---------|------|
| 19 | Custom JSONL entries in sessions | VibeCody WASM can work around this |
| 20 | `pi install` package manager for extensions | Currently handled via `cargo install` / npm; deduplicate effort |
| 21 | Virtual terminal for TUI tests | `VirtualTerminal` via xterm headless — good for CI |
| 22 | Mom's per-channel dir isolation | Already have workspace isolation; per-channel dirs are a refinement |
| 23 | Artifacts unified panel (web) | Partially covered by existing VibeUI panels |
| 24 | CORS proxy for web UI | Tauri bypasses CORS; only relevant for pure web build |
| 25 | i18n | Not a current priority |

---

## What VibeCody is Already Ahead On

| Area | VibeCody advantage |
|------|--------------------|
| Skills library | 702+ built-in skills vs Pi's community-driven (small base) |
| Providers | 18 providers including Bedrock, Azure OpenAI, DeepSeek, Zhipu, Cerebras |
| Storage security | Encrypted SQLite (`ProfileStore`/`WorkspaceStore`) vs Pi's plaintext IndexedDB |
| Desktop editor | VibeUI (Tauri + Monaco + 215+ panels) — Pi has no IDE |
| Code analysis | SonarQube-style line-level findings (`sonar_rules.rs`), 126+ rules |
| LSP integration | `vibe-lsp` with rust-analyzer + TypeScript LSP |
| Agent orchestration | Multi-agent teams, CRDT collab, A2A protocol |
| Test runner | BDD harnesses, coverage, SWE-bench support |
| TIOBE top-50 coverage | 43 Language enum variants, 155+ extension mappings |
| Hooks | Pre/post hooks with JSON stdin/stdout, exit-code-based block |

---

## Recommended Implementation Order

```
Phase A (P0) — Session tree, parallel tools, OAuth, steering queue
  A1. session_tree.rs  — JSONL id/parentId branching + /tree REPL command
  A2. parallel_tools.rs — concurrent tool dispatch with sequential preflight
  A3. oauth_login.rs   — OAuth flows for Anthropic, Copilot, Gemini CLI
  A4. message_queue.rs — steering + follow-up queues with one-at-a-time/all modes

Phase B (P1) — Cross-provider handoff, pluggable tools, streaming args, dual-log
  B1. context_handoff.rs — serializable Context type, mid-session provider swap
  B2. tool_operations.rs — BashOperations/EditOperations trait, SSH redirect impl
  B3. stream_tool_args.rs — toolcall_delta events for partial arg streaming
  B4. dual_log.rs         — append-only log.jsonl + compacted context.jsonl
  B5. thinking_levels.rs  — 6-level thinking abstraction with token budgets

Phase C (P2) — TUI polish, sharing, RPC
  C1. tui_images.rs    — Kitty/iTerm2 inline image protocol
  C2. tui_ime.rs       — CURSOR_MARKER IME positioning
  C3. session_share.rs — HTML export + GitHub Gist upload
  C4. rpc_mode.rs      — stdin/stdout JSONL bidirectional protocol
  C5. paste_guard.rs   — bracketed paste collapse for large pastes
  C6. event_bus.rs     — typed in-process event bus (30+ events for WASM extensions)

Phase D (P1 ops) — GPU / vLLM
  D1. pod_manager.rs   — vLLM pod deploy, VRAM preflight, tool-call-parser config
```

---

## References

- **Pi mono repo**: [github.com/TuringWorks/pi-mono](https://github.com/TuringWorks/pi-mono)
- **Pi coding agent docs**: `packages/coding-agent/docs/`
- **agentskills.io standard**: Referenced in pi-mom and pi-coding-agent
- **Pi blog — skills vs MCP**: `packages/coding-agent/docs/2025-11-02-what-if-you-dont-need-mcp`
- **Prior FIT-GAP docs**: [FIT-GAP v12](./FIT-GAP-ANALYSIS-v12.md) | [FIT-GAP v11](./FIT-GAP-ANALYSIS-v11.md) | [Code Review Architecture](./FIT-GAP-CODE-REVIEW-ARCHITECTURE.md)
