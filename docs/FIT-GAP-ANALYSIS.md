# VibeCody Fit-Gap Analysis & Implementation Roadmap

**Date:** 2026-02-25
**Scope:** VibeCLI (vs Claude Code CLI, Codex CLI) · VibeUI (vs Cursor, Windsurf/Antigravity)

---

## 1. VibeCLI vs Claude Code CLI

### Feature Comparison

| Feature | VibeCLI | Claude Code |
|---------|---------|-------------|
| Multi-turn REPL | ✅ | ✅ |
| Agent loop (tools) | ✅ | ✅ |
| Plan mode | ✅ | ✅ |
| Session resume | ✅ | ✅ |
| Hooks system (Pre/PostToolUse) | ✅ | ✅ |
| Skills / slash commands | ✅ | ✅ |
| MCP client | ✅ | ✅ |
| Git integration | ✅ | ✅ |
| Web search tool | ✅ | ✅ |
| Multi-agent / parallel execution | ✅ | ✅ |
| PR code review | ✅ | ✅ |
| OpenTelemetry tracing | ✅ | ✅ |
| Admin policy | ✅ | ✅ |
| HTTP daemon (`serve`) | ✅ | ✅ |
| VS Code extension | ✅ | ✅ |
| Agent SDK (Node.js) | ✅ | ✅ |
| Named profiles | ✅ | ✅ |
| Doctor command | ✅ | ✅ |
| REPL tab-completion | ✅ | ✅ |
| **Image / screenshot attachment (`-i`)** | ✅ | ✅ |
| **`/model` mid-session switching** | ✅ | ✅ |
| **`/cost` token + USD tracking** | ✅ | ✅ |
| **`/context` window usage** | ✅ | ✅ |
| **`/status` provider status** | ✅ | ✅ |
| **Named sessions (`--session-name`)** | ✅ | ✅ |
| **Session forking (`/fork`)** | ✅ | ✅ |
| **UserPromptSubmit hook event** | ✅ | ✅ |
| **LLM-based hook execution** | ✅ | ✅ |
| **Wildcard tool permission patterns** | ✅ | ✅ |
| **`apiKeyHelper` rotating credentials** | ✅ | ✅ |
| **Extended thinking mode** | ✅ | ✅ |
| **`--add-dir` additional workspace dirs** | ✅ | ✅ |
| **JSON streaming output (`--json`)** | ✅ | ✅ |
| **Typed parallel agent roles** | ✅ | ✅ |
| **Auto memory recording** | ✅ | ✅ |
| **Rules directory (`.vibecli/rules/`)** | ✅ | ✅ |
| **CLI `/rewind` session checkpoint** | ✅ | ✅ |
| **PTY-backed bash tool** | ✅ | ✅ |

---

## 2. VibeCLI vs Codex CLI

### Feature Comparison

| Feature | VibeCLI | Codex CLI |
|---------|---------|-----------|
| Agent loop | ✅ | ✅ |
| Plan mode | ✅ | ✅ |
| Multi-provider support | ✅ (5 providers) | ✅ (OpenAI only) |
| Web search | ✅ | ✅ |
| HTTP daemon | ✅ | ❌ |
| VS Code extension | ✅ | ❌ |
| Agent SDK | ✅ | ❌ |
| Multi-agent orchestration | ✅ | ❌ |
| PR code review | ✅ | ❌ |
| Voice input (VibeUI) | ✅ | ❌ |
| **Sandboxed execution (Docker/macOS)** | ❌ | ✅ |
| **`--approval=auto-edit`** granular modes | ✅ | ✅ |
| **Desktop app (floating window)** | ❌ | ✅ |

---

## 3. VibeUI vs Cursor

### Feature Comparison

| Feature | VibeUI | Cursor |
|---------|--------|--------|
| Monaco code editor | ✅ | ✅ |
| AI chat panel | ✅ | ✅ |
| Agent panel | ✅ | ✅ |
| File tree | ✅ | ✅ |
| Terminal panel | ✅ | ✅ |
| Git integration | ✅ | ✅ |
| LSP support | ✅ | ✅ |
| MCP client | ✅ | ✅ |
| Hooks UI | ✅ | ✅ |
| Multiple AI providers | ✅ | ✅ |
| Voice input | ✅ | ❌ |
| Background job persistence | ✅ | ❌ |
| @web context | ✅ | ❌ |
| Browser panel | ✅ | ❌ |
| Artifact system | ✅ | ❌ |
| Parallel agents (Manager view) | ✅ | ❌ |
| **Inline chat / Cmd+K edit overlay** | ✅ | ✅ |
| **Next-edit prediction (Tab ghost text)** | ✅ | ✅ |
| **@symbol context (LSP-resolved)** | ✅ | ✅ |
| **@codebase semantic search context** | ✅ | ✅ |
| **@folder context injection** | ✅ | ✅ |
| **@terminal context injection** | ✅ | ✅ |
| **@docs context (library docs)** | ❌ | ✅ |
| **@file:path:N-M line-range context** | ✅ | ✅ |
| **Chunk-level diff accept/reject** | ❌ | ✅ |
| **Multiple chat tabs** | ✅ | ✅ |
| **Per-chat model switching** | ✅ | ✅ |
| **BYOK settings UI** | ✅ | ✅ |
| **Linter integration (auto-fix after write)** | ❌ | ✅ |
| **MCP server manager UI** | ✅ | ✅ |
| **Passive context tracking (Flows)** | ✅ | ✅ |
| **Named checkpoint descriptions** | ✅ | ✅ |
| **Rules directory UI** | ✅ | ✅ |

---

## 4. VibeUI vs Windsurf (Antigravity)

### Feature Comparison

| Feature | VibeUI | Windsurf |
|---------|--------|----------|
| AI agent with file tools | ✅ | ✅ |
| Streaming responses | ✅ | ✅ |
| File tree + editor | ✅ | ✅ |
| Terminal | ✅ | ✅ |
| Voice input | ✅ | ❌ |
| Multi-agent orchestration | ✅ | ❌ |
| MCP client | ✅ | ✅ |
| Background jobs | ✅ | ❌ |
| **Cascade (unified flow: chat + inline + terminal)** | ❌ | ✅ |
| **Per-file AI awareness (Flows)** | ❌ | ✅ |
| **Cloud-based agent execution** | ❌ | ✅ |
| **Cross-file next-edit prediction** | ❌ | ✅ |
| **Dual-scale codebase indexing** | ❌ | ✅ |
| **Remote collaboration** | ❌ | ✅ |

---

## 5. VibeCody Competitive Advantages

These features VibeCody has that **none** of the competitors offer cleanly:

| Feature | Notes |
|---------|-------|
| **Voice input in AI chat** | Web Speech API integration, 🎤 button, interim transcripts |
| **Multi-provider (5 providers)** | Ollama, Claude, OpenAI, Gemini, Grok — Codex is OpenAI-only |
| **HTTP daemon + Agent SDK** | `vibecli serve` + Node.js SDK — unique among CLI tools |
| **OpenTelemetry tracing** | OTLP/HTTP export for observability-first teams |
| **Admin policy with glob rules** | Enterprise tool restriction |
| **@web context (fetch+strip)** | Full HTML fetch + DuckDuckGo search in agent context |
| **Browser panel (localhost preview)** | Built-in iframe panel for web app development |
| **Background job persistence** | Jobs survive daemon restart; cancel/stream endpoints |
| **Multi-agent Manager view** | Visual parallel agent execution with branch merging |
| **Artifacts panel** | Structured AI output as typed artifacts |
| **Hooks config UI** | Visual hook editor with LLM handler support |
| **PR code review** | `--review`, `--base`, `--branch`, `--pr`, `--post-github` flags |
| **WASM extension system** | `vibe-extensions` with wasmtime runtime |

---

## 6. Gap Priority Matrix

| Gap | Impact | Effort | Phase |
|-----|--------|--------|-------|
| Inline chat (Cmd+K) | Critical | XL | 14 |
| Next-edit prediction (Tab) | Critical | L | 14 |
| `/model` mid-session switch | High | S | 12 |
| `/cost` token tracking | High | M | 12 |
| Multiple chat tabs | High | M | 12 |
| Chunk-level diff accept/reject | High | L | 12 |
| UserPromptSubmit hook | High | M | 12 |
| @symbol context | High | M | 12 |
| @codebase context | High | M | 12 |
| BYOK settings UI | High | M | 12 |
| Image attachment (`-i`) | High | S | 12 |
| LLM-based hooks | Medium | M | 12 |
| Rules directory | Medium | M | 13 |
| Auto memory recording | Medium | M | 13 |
| Wildcard permissions | Medium | S | 13 |
| `apiKeyHelper` | Medium | S | 13 |
| Named sessions | Low | S | 13 |
| MCP manager UI | Medium | M | 13 |
| Linter integration | High | L | 14 |
| PTY-backed bash | Medium | L | 14 |
| @docs context | High | M | 14 |
| opusplan model routing | Medium | M | 14 |
| Remote indexing service | High | XL | 15 |
| JetBrains plugin | High | XL | 15 |
| @claude GitHub PR bot | Medium | L | 15 |
| Supercomplete | High | XL | 15 |

---

## 7. Implementation Roadmap

### Phase 12 — Quick Wins (1 week)

**Goal:** Close the most visible gaps with small-to-medium effort. These are standalone features that don't require architectural changes.

| Feature | Effort | Impact | Target | Files to Change |
|---------|--------|--------|--------|----------------|
| `/model` mid-session switching | S | High | VibeCLI | `repl.rs`, `main.rs` — swap `Arc<dyn AIProvider>` at runtime |
| `/cost` token tracking | M | High | VibeCLI | `provider.rs` (add `TokenUsage`), `claude.rs`/`openai.rs` (parse usage), `repl.rs`, `main.rs` |
| `/context` window display | S | Medium | VibeCLI | `repl.rs` — display current messages size vs model context limit |
| `/status` provider status | S | Medium | VibeCLI | `repl.rs` — ping provider + print model, API key mask, token counts |
| Named sessions (`--session-name`) | S | Medium | VibeCLI | `main.rs`, `trace.rs` — prefix trace files with name |
| `/fork` session branching | M | Medium | VibeCLI | `main.rs`, `trace.rs` — `clone()` messages vec into new session file |
| UserPromptSubmit hook event | M | High | VibeCLI | `hooks.rs` (new variant), `agent.rs` (fire before first message) |
| LLM hook execution | M | Medium | VibeCLI | `hooks.rs` (implement stubbed `HookHandler::Llm` arm) |
| Image attachment (`-i` flag) | S | High | VibeCLI | `main.rs` — `Vec<PathBuf>` arg; `ImageAttachment::from_path` already exists |
| `--add-dir` additional dirs | S | Medium | VibeCLI | `main.rs`, `agent.rs` — extend `AgentContext.additional_dirs` |
| `--json` streaming output | M | Medium | VibeCLI | `main.rs`, `serve.rs` — emit `AgentEvent` as JSON lines to stdout |
| Multiple chat tabs in VibeUI | M | High | VibeUI | new `ChatTabManager.tsx`, `App.tsx` — array of tab states |
| Per-chat model switching | M | High | VibeUI | `AIChat.tsx`, `App.tsx` — model dropdown per tab |
| BYOK settings UI | M | High | VibeUI | new `SettingsPanel.tsx`, `commands.rs` — read/write config API keys |
| `@symbol` context | M | High | VibeUI | `ContextPicker.tsx`, `commands.rs` — LSP workspace_symbol query |
| `@codebase` semantic search | M | High | VibeUI | `ContextPicker.tsx`, `commands.rs` — call `EmbeddingIndex::search` |
| `@folder` context | S | Medium | VibeUI | `ContextPicker.tsx` — list all files under a folder path |
| `@terminal` context | S | Medium | VibeUI | `ContextPicker.tsx`, `App.tsx` — inject last N lines of terminal buffer |
| `@file:path:N-M` line ranges | S | Medium | VibeUI | `ContextPicker.tsx`, `commands.rs` — slice file at resolve time |
| Chunk-level diff accept/reject | L | High | VibeUI | new `DiffReviewPanel.tsx`, `AgentPanel.tsx`, `agent_executor.rs` — buffer writes |

### Phase 13 — Medium Complexity (1-2 weeks)

**Goal:** Close remaining CLI gaps and add new differentiation features requiring new modules.

| Feature | Effort | Impact | Target | Files to Change | Description |
|---------|--------|--------|--------|----------------|-------------|
| Wildcard tool permission patterns | S | Medium | VibeCLI | `policy.rs` — call `glob_match()` in `check_tool`; add `tool_patterns` field |
| `apiKeyHelper` rotating credentials | S | Medium | VibeCLI | `config.rs`, each provider — run helper script before each API call |
| Extended thinking mode | M | Medium | VibeCLI | `config.rs`, `claude.rs` — add `thinking_budget_tokens` to request |
| Rules directory (`.vibecli/rules/`) | M | Medium | Both | new `rules.rs` (vibe-ai), `agent.rs` (inject matching rules into system prompt) |
| Auto memory recording | M | Medium | VibeCLI | new `memory_recorder.rs`, `main.rs` — LLM summarizes session → appends to memory.md |
| Typed parallel agent roles | M | Medium | VibeCLI | `multi_agent.rs` — `AgentRole` enum (Planner/Executor/Reviewer); role-specific system prompts |
| CLI checkpoint/rewind (`/rewind`) | M | Medium | VibeCLI | `main.rs`, `repl.rs`, `trace.rs` — snapshot files + messages at each major step |
| Named checkpoint descriptions | S | Medium | VibeUI | `CheckpointPanel.tsx` — `.vibecli/checkpoints.json` sidecar with descriptions |
| MCP server manager UI | M | High | VibeUI | new `McpManagerPanel.tsx`, `commands.rs` — list/add/remove/test MCP servers |
| Passive context tracking enhancement | L | High | VibeUI | `App.tsx`, new `FlowTracker.ts` — record viewed lines, clipboard, terminal bursts |
| Notebook (.ipynb) tool | L | Medium | VibeCLI | `tools.rs`, `tool_executor.rs` — `ReadNotebook`/`EditNotebook` tool calls |

### Phase 14 — Advanced (1-2 weeks)

**Goal:** Implement features requiring significant new modules or major UI changes. These close the largest gaps against Cursor's inline experience.

| Feature | Effort | Impact | Target | Files to Change | Description |
|---------|--------|--------|--------|----------------|-------------|
| Inline chat overlay (Cmd+K) | XL | Critical | VibeUI | `App.tsx`, new `InlineChat.tsx`, Monaco overlay widget | Floating AI overlay at selection; stream diff; Accept/Reject |
| Next-edit prediction (Tab) | L | Critical | VibeUI | `App.tsx` — register `registerInlineCompletionItemProvider`; call existing `predict_next_edit` Tauri command |
| Linter integration (auto-fix) | L | High | VibeUI | new `LinterIntegration.ts`, `AgentPanel.tsx` — run linter after write; inject errors into agent |
| PTY-backed bash tool | L | High | VibeCLI | `tool_executor.rs` — use `portable-pty` (already in vibe-core) instead of `tokio::process::Command` |
| Inter-agent messaging | L | Medium | VibeCLI | `multi_agent.rs`, `agent.rs` — `mpsc` broadcast channel; `send_message_to_agent` tool |
| `@docs` context (docs.rs / npm) | M | High | VibeUI | `ContextPicker.tsx`, new `DocsResolver.ts`, Tauri fetch command |
| Path-targeted rules UI | M | Medium | VibeUI | new `RulesPanel.tsx`, `App.tsx` — CRUD for `.vibecli/rules/` files with `applies_to` glob |
| opusplan model routing | M | Medium | VibeCLI | `main.rs`, `config.rs` — separate `planning_provider` and `execution_provider` config |
| Cost tracking display in VibeUI | M | High | VibeUI | `AgentPanel.tsx`, `AIChat.tsx`, `provider.rs` — `TokenUsage` field + cumulative cost footer |

### Phase 15 — Differentiators (Ongoing)

**Goal:** Long-term investments in unique VibeCody capabilities that differentiate from all competitors.

| Feature | Effort | Impact | Target | Files to Change | Description |
|---------|--------|--------|--------|----------------|-------------|
| Remote codebase indexing service | XL | High | Both | new `vibe-indexer/` crate, `vibe-core/index/` — HTTP API over `EmbeddingIndex` |
| Cascade flows equivalent | XL | Critical | VibeUI | `App.tsx`, `AgentPanel.tsx`, `AIChat.tsx`, `Terminal.tsx`, `InlineChat.tsx` — unified context flow |
| JetBrains plugin | XL | High | VibeCLI | new `jetbrains-plugin/` (Kotlin/Gradle) — connects to `vibecli serve` daemon |
| Desktop app (floating window) | L | Medium | VibeCLI | new `vibeapp/` Tauri project — minimal macOS floating AI window |
| Supercomplete (cross-file multi-line) | XL | High | VibeUI | new `SupercompleteEngine.ts`, `App.tsx` — embedding-powered multi-file edit prediction |
| `@vibecli` GitHub PR bot | L | Medium | VibeCLI | `.github/workflows/pr-bot.yml`, `review.rs` — trigger on `@vibecli` PR comment mentions |
| Schema validation (`--output-schema`) | M | Medium | VibeCLI | `main.rs`, new `schema.rs` — validate `task_complete` artifact JSON against JSON Schema |
| TUI theme switching (`/theme`) | S | Low | VibeCLI | `tui/`, `config.rs` — switch `syntect` theme at runtime |

---

## 8. Detailed Implementation Specs

### Phase 12 Specs

#### 12.1 `/model` Mid-Session Switching

**Files to modify:**
- `vibecli/vibecli-cli/src/repl.rs` — add `"/model"` to `COMMANDS` array; dispatch to handler
- `vibecli/vibecli-cli/src/main.rs` — hold provider in `Arc<Mutex<Arc<dyn AIProvider>>>` for hot-swap

```rust
// In REPL dispatch:
"/model" => {
    let (provider_str, model_str) = parse_model_arg(args)?;
    let new_provider = build_provider(&provider_str, &model_str, &config)?;
    *current_provider.lock().await = Arc::new(new_provider);
    println!("Switched to {provider_str}/{model_str}");
}
```

**Acceptance criteria:**
- `/model ollama/codellama` switches immediately; subsequent turns use new provider
- `/model` with no args prints current provider and model
- Invalid provider name prints error without crashing

---

#### 12.2 `/cost` Token Tracking

**Files to modify:**
- `vibeui/crates/vibe-ai/src/provider.rs` — add `TokenUsage { prompt_tokens: u32, completion_tokens: u32 }` to `CompletionResponse`
- `vibeui/crates/vibe-ai/src/providers/claude.rs` — parse `usage` field from Claude API response
- `vibeui/crates/vibe-ai/src/providers/openai.rs` — parse `usage` field from OpenAI API response
- `vibecli/vibecli-cli/src/main.rs` — accumulate tokens; add `/cost` REPL command

```rust
pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
}

static PROVIDER_PRICING: &[(&str, f64, f64)] = &[
    ("claude-sonnet-4-6", 3.0 / 1_000_000.0, 15.0 / 1_000_000.0),
    ("gpt-4o",            2.5 / 1_000_000.0, 10.0 / 1_000_000.0),
    // ...
];
```

**Acceptance criteria:**
- `/cost` shows prompt tokens, completion tokens, total, and estimated USD cost
- Ollama shows token count but `$0.00`

---

#### 12.3 UserPromptSubmit Hook Event

**Files to modify:**
- `vibeui/crates/vibe-ai/src/hooks.rs` — add `HookEvent::UserPromptSubmit { prompt: String, session_id: String }`
- `vibeui/crates/vibe-ai/src/agent.rs` — fire hook in `run_inner` before building the messages vec; if `HookDecision::Block`, emit `AgentEvent::Error` and return

**Acceptance criteria:**
- Hook fires before every new agent task
- `exit 2` cancels the task with the hook's reason message
- `{"context": "..."}` JSON response prepends extra context to the user message

---

#### 12.4 LLM Hook Execution

**Files to modify:**
- `vibeui/crates/vibe-ai/src/hooks.rs` — implement the stubbed `HookHandler::Llm` arm; `HookRunner` accepts optional `Arc<dyn AIProvider>`

```rust
pub struct HookRunner {
    configs: Vec<HookConfig>,
    llm_provider: Option<Arc<dyn AIProvider>>, // new
}

impl HookRunner {
    pub fn with_llm_provider(mut self, p: Arc<dyn AIProvider>) -> Self {
        self.llm_provider = Some(p); self
    }

    async fn exec_llm_hook(&self, config: &HookConfig, payload: &str) -> HookDecision {
        let provider = self.llm_provider.as_ref().unwrap();
        // Call provider with prompt + payload; parse {"ok": true/false, "reason": "..."}
    }
}
```

**Acceptance criteria:**
- `handler = { llm = "Is this command safe? Respond {\"ok\": true} or {\"ok\": false, \"reason\": \"...\"}" }` works
- LLM hook shares the same provider as the running agent

---

#### 12.5 Multiple Chat Tabs in VibeUI

**Files to modify:**
- `vibeui/src/App.tsx` — replace single `<AIChat />` with `<ChatTabManager />`
- New `vibeui/src/components/ChatTabManager.tsx` — manages array of `{ id, title, messages, provider, model }` states

```typescript
interface ChatTab {
    id: string;
    label: string;
    provider: string;
    model?: string;
    createdAt: number;
}

const [chatTabs, setChatTabs] = useState<ChatTab[]>([
    { id: "default", label: "Chat 1", provider: selectedProvider, createdAt: Date.now() }
]);
const [activeChatTabId, setActiveChatTabId] = useState("default");
```

**Acceptance criteria:**
- `+` button creates new tab with independent conversation
- Switching tabs preserves message history
- Each tab has its own provider/model dropdown
- At least one tab always remains (no close on last tab)

---

#### 12.6 Chunk-Level Diff Accept/Reject

**Files to modify:**
- New `vibeui/src/components/DiffReviewPanel.tsx` — parse unified diff into hunks; per-hunk Accept/Reject buttons
- `vibeui/src-tauri/src/agent_executor.rs` — buffer `FileChange` artifacts; emit `agent:files_pending_review` before writing
- `vibeui/src-tauri/src/commands.rs` — add `apply_file_changes(changes: Vec<FileChange>)` and `discard_file_changes()` Tauri commands
- `vibeui/src/App.tsx` — listen for `agent:files_pending_review`; show `DiffReviewPanel` modal

**Acceptance criteria:**
- Agent changes are held until user approves
- Per-hunk accept/reject works for new files and modifications
- "Accept All" applies all immediately; "Reject All" discards all
- Individual hunk rejection produces a valid partial patch

---

#### 12.7 BYOK Settings UI

**Files to modify:**
- New `vibeui/src/components/SettingsPanel.tsx` — inputs for API keys + Ollama URL; Save button
- `vibeui/src-tauri/src/commands.rs` — `get_provider_settings()` and `save_provider_settings()` Tauri commands reading `~/.vibecli/config.toml`
- `vibeui/src/App.tsx` — add `"settings"` to `aiPanelTab` union type

**Acceptance criteria:**
- Keys are displayed masked (last 4 chars visible)
- Saving writes to config without overwriting other fields
- New keys take effect immediately for subsequent requests

---

#### 12.8 `@symbol`, `@codebase`, `@folder`, `@terminal` Context

**Files to modify:**
- `vibeui/src/components/ContextPicker.tsx` — add new prefix handlers for each context type
- `vibeui/src-tauri/src/commands.rs` — `search_workspace_symbols(query)`, `semantic_search_codebase(query)` Tauri commands
- `vibeui/crates/vibe-lsp/src/features.rs` — `workspace_symbol(query) -> Vec<SymbolInfo>` LSP call

**Acceptance criteria:**
- `@SymbolName` resolves via LSP to function/struct source code
- `@codebase:query` runs embedding search; top-5 snippets injected
- `@folder:src/components` lists all files under that path
- `@terminal` injects last 200 lines of terminal buffer

---

### Phase 13 Specs

#### 13.1 Rules Directory (`.vibecli/rules/`)

**Files to create/modify:**
- New `vibeui/crates/vibe-ai/src/rules.rs` — `Rule { name, path_pattern, content }`, `RulesLoader::load(dir)`, `matching(open_files)`
- `vibeui/crates/vibe-ai/src/agent.rs` — inject matching rules into `build_system_prompt()` after skills section

```rust
// Rule file front-matter:
// ---
// name: rust-safety
// path_pattern: "**/*.rs"
// ---
// When editing Rust files, always check for unwrap() calls...

pub struct Rule {
    pub name: String,
    pub path_pattern: Option<String>,
    pub content: String,
    pub source: PathBuf,
}
```

**Acceptance criteria:**
- Rules with `path_pattern: "**/*.ts"` only inject when TypeScript files are open
- Rules with no pattern always inject
- `/memory show` displays active rules alongside memory content

---

#### 13.2 Auto Memory Recording

**Files to modify:**
- New `vibecli/vibecli-cli/src/memory_recorder.rs` — `MemoryRecorder::record_session(trace, messages, provider)` — LLM summarizes → appends to memory.md
- `vibecli/vibecli-cli/src/main.rs` — call after `AgentLoop::run()` completes if `config.memory.auto_record = true`
- `vibecli/vibecli-cli/src/config.rs` — add `MemoryConfig { auto_record: bool, min_session_steps: usize }`

**Acceptance criteria:**
- After a 5+ step session, 1-3 learning bullet points appended to `~/.vibecli/memory.md`
- Feature is opt-in via `[memory] auto_record = true`
- Recording is async and doesn't delay session completion

---

#### 13.3 Wildcard Tool Permission Patterns

**Files to modify:**
- `vibeui/crates/vibe-ai/src/policy.rs` — add `tool_patterns: Vec<String>` to `AdminPolicy`; parse `tool(arg_pattern)` syntax; use existing `glob_match()` in `check_tool_with_args()`

```rust
// Config:
// denied_tool_patterns = ["bash(rm*)"]  → blocks bash with rm args
// allowed_tool_patterns = ["bash(git*)"] → allows only git subcommands

fn check_tool_with_args(&self, tool_name: &str, primary_arg: &str) -> PolicyDecision {
    for pattern in &self.denied_tool_patterns {
        if let Some((tool_pat, arg_pat)) = parse_tool_pattern(pattern) {
            if glob_match(&tool_pat, tool_name) && glob_match(&arg_pat, primary_arg) {
                return PolicyDecision::Deny;
            }
        }
    }
    PolicyDecision::Allow
}
```

**Acceptance criteria:**
- `denied_tool_patterns = ["bash(rm*)"]` blocks `bash(rm -rf .)` but allows `bash(cargo build)`
- Existing `denied_tools` exact-match continues to work

---

#### 13.4 `apiKeyHelper` Rotating Credentials

**Files to modify:**
- `vibecli/vibecli-cli/src/config.rs` — add `api_key_helper: Option<String>` to `ProviderConfig`
- Each provider (`claude.rs`, `openai.rs`, etc.) — add `resolve_api_key()` async method; run helper script; use stdout as Bearer token

**Acceptance criteria:**
- `api_key_helper = "~/.vibecli/get-key.sh claude"` executed before each API call
- If script exits non-zero, falls back to static `api_key`

---

#### 13.5 MCP Server Manager UI

**Files to modify:**
- New `vibeui/src/components/McpManagerPanel.tsx` — list configured servers with status indicator; Add/Remove/Test buttons; tool browser per server
- `vibeui/src-tauri/src/commands.rs` — `list_mcp_servers()`, `add_mcp_server()`, `remove_mcp_server()`, `test_mcp_server()`, `list_mcp_server_tools()` Tauri commands
- `vibeui/src/App.tsx` — add `"mcp"` to `aiPanelTab`

**Acceptance criteria:**
- Configured MCP servers from config appear in the list
- "Test" button spawns server, runs initialize, reports tool count
- Tool browser shows names and descriptions

---

### Phase 14 Specs

#### 14.1 Inline Chat / Cmd+K Edit Overlay

**Files to modify:**
- `vibeui/src/App.tsx` — register Monaco `addCommand(KeyMod.CtrlCmd | KeyCode.KeyK, ...)` after editor mount; capture selection + range; show `<InlineChat>`
- New `vibeui/src/components/InlineChat.tsx` — Monaco overlay widget; text input; streaming response; Accept/Reject buttons
- `vibeui/src-tauri/src/commands.rs` — `inline_edit(request: InlineEditRequest) -> String` Tauri command

```typescript
interface InlineChatProps {
    selection: {
        text: string;
        startLine: number;
        endLine: number;
        filePath: string;
        language: string;
    };
    position: { top: number; left: number };
    onAccept: (newText: string) => void;
    onReject: () => void;
}
```

**Acceptance criteria:**
- Cmd+K opens floating input within 100ms
- Response streams in real-time in overlay
- Accept replaces selection; Reject/Escape closes with no changes
- Overlay positioned near selection, not offscreen

---

#### 14.2 Next-Edit Prediction (Tab Acceptance)

**Files to modify:**
- `vibeui/src/App.tsx` — in Monaco `onMount`, register `monaco.languages.registerInlineCompletionItemProvider('*', { provideInlineCompletions: ... })`; call `invoke("predict_next_edit", {...})` with 500ms debounce

The `predict_next_edit` Tauri command (Phase 7.3) is already implemented — only the frontend wiring is needed.

**Acceptance criteria:**
- Ghost text appears after 500ms of cursor inactivity
- Tab accepts suggestion; Escape dismisses
- Debounced to avoid excessive API calls

---

#### 14.3 Linter Integration (Auto-Fix After Write)

**Files to modify:**
- New `vibeui/src/utils/LinterIntegration.ts` — `runLinter(filePath, fileType): Promise<LintResult>`; reads from `.vibecli/linters.toml` or uses defaults (eslint, cargo clippy, etc.)
- `vibeui/src/components/AgentPanel.tsx` — after `write_file` step, call `runLinter`; if errors, inject as next agent context
- New Tauri command `inject_agent_context(text: String)` — appends user message to running agent's queue

**Acceptance criteria:**
- After writing a `.ts` file, eslint runs automatically (if configured)
- Lint errors injected as "[Linter] eslint found 2 errors: ..."
- Agent gets one auto-fix turn before returning to user

---

#### 14.4 PTY-Backed Bash Tool

**Files to modify:**
- `vibecli/vibecli-cli/src/tool_executor.rs` — implement `exec_bash_pty()` using `portable_pty` (already in Cargo.toml via vibe-core); strip ANSI codes before returning; enforce 120s timeout

**Acceptance criteria:**
- Interactive programs (npm install, cargo build) work correctly
- Output still capped at `MAX_TOOL_OUTPUT` chars
- Backward-compatible: existing tests pass unchanged

---

#### 14.5 `@docs` Context (Library Documentation)

**Files to modify:**
- `vibeui/src/components/ContextPicker.tsx` — add `@docs:` special prefix
- New `vibeui/src/utils/DocsResolver.ts` — `resolveDoc(name): Promise<string>`; detects language from open files; fetches from docs.rs/npmjs.com/pypi
- New Tauri command `fetch_doc_content(name, registry)` — HTTP fetch from Rust side (avoids CORS)

**Acceptance criteria:**
- `@docs:tokio` fetches and injects tokio crate API summary
- Results cached for 24 hours
- Fetch errors show inline warning

---

#### 14.6 opusplan Model Routing

**Files to modify:**
- `vibecli/vibecli-cli/src/config.rs` — add `[routing]` section with `planning_provider`, `planning_model`, `execution_provider`, `execution_model`
- `vibecli/vibecli-cli/src/main.rs` — build two providers; pass `planning_provider` to `PlannerAgent::new()` and `execution_provider` to `AgentLoop::new()`

```toml
# ~/.vibecli/config.toml
[routing]
planning_provider = "claude"
planning_model = "claude-opus-4-6"
execution_provider = "claude"
execution_model = "claude-sonnet-4-6"
```

**Acceptance criteria:**
- `/plan` uses `planning_model`; `--agent` uses `execution_model`
- Falls back to `--provider`/`--model` flags if routing config absent
- `vibecli --doctor` shows active planning and execution models

---

### Phase 15 Specs

#### 15.1 Remote Codebase Indexing Service

**Files to create:**
- New `vibe-indexer/` Cargo workspace member — Axum HTTP server with `POST /index`, `GET /index/status/:id`, `POST /search` endpoints; uses same `EmbeddingIndex` from `vibe-core`
- `vibeui/crates/vibe-core/src/index/remote.rs` — `RemoteEmbeddingIndex` implementing same `search()` interface over HTTP
- `vibeui/crates/vibe-core/src/index/mod.rs` — add `IndexBackend::Remote { url, api_key }` variant

**Acceptance criteria:**
- Indexes 100K-file monorepo in under 5 minutes
- Search latency under 200ms
- Configured via `[index] backend = "remote"` + `url`

---

#### 15.2 JetBrains Plugin

**Files to create:**
- New `jetbrains-plugin/` directory with Kotlin/Gradle plugin structure
- `VibeCLIService.kt` — HTTP client connecting to `vibecli serve` daemon
- `AgentToolWindow.kt` — IntelliJ tool window with Chat and Agent panels
- `InlineEditAction.kt` — Cmd+K equivalent using daemon API

**Acceptance criteria:**
- Installs from JetBrains Marketplace ZIP
- Connects to local `vibecli serve` daemon
- Chat and agent task submission works
- Works in IntelliJ IDEA 2024.1+

---

#### 15.3 `@vibecli` GitHub PR Bot

**Files to create:**
- `.github/workflows/pr-bot.yml` — triggered by `issue_comment` events containing `@vibecli`
- `.github/actions/vibecli-pr-bot/action.yml` — composite action: install VibeCLI, run agent on PR context, post comment

```yaml
on:
  issue_comment:
    types: [created]
jobs:
  vibecli-bot:
    if: contains(github.event.comment.body, '@vibecli')
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Run VibeCLI agent
        run: vibecli --agent "$TASK" --provider claude --json > result.json
        env: { ANTHROPIC_API_KEY: ${{ secrets.ANTHROPIC_API_KEY }} }
      - name: Post result
        run: gh pr comment ${{ github.event.issue.number }} --body "$(cat result.json | jq -r '.summary')"
```

**Acceptance criteria:**
- `@vibecli fix TypeScript errors` triggers agent on PR diff
- Result posted as reply to triggering comment
- Works with all supported providers via GitHub Secrets

---

## 9. File Modification Quick Reference

### VibeCLI Files

| File | Phases | Role |
|------|--------|------|
| `vibecli/vibecli-cli/src/main.rs` | 12, 13, 14 | CLI flags (`-i`, `--add-dir`, `--json`, `--session-name`), provider hot-swap, REPL loop |
| `vibecli/vibecli-cli/src/repl.rs` | 12, 13 | `/model`, `/cost`, `/context`, `/status`, `/fork`, `/rewind` REPL commands |
| `vibecli/vibecli-cli/src/config.rs` | 12, 13, 14 | `TokenUsage`, `MemoryConfig`, `RoutingConfig`, `apiKeyHelper` |
| `vibecli/vibecli-cli/src/tool_executor.rs` | 14 | PTY bash execution |
| `vibecli/vibecli-cli/src/memory.rs` | 13 | `auto_record()` method |
| `vibecli/vibecli-cli/src/serve.rs` | 14 | JSON streaming output |

### vibe-ai Crate Files

| File | Phases | Role |
|------|--------|------|
| `vibeui/crates/vibe-ai/src/agent.rs` | 12, 13 | UserPromptSubmit hook, `--add-dir`, rules injection |
| `vibeui/crates/vibe-ai/src/hooks.rs` | 12 | `UserPromptSubmit` event variant, LLM hook implementation |
| `vibeui/crates/vibe-ai/src/policy.rs` | 13 | Wildcard glob tool patterns |
| `vibeui/crates/vibe-ai/src/provider.rs` | 12 | `TokenUsage` in `CompletionResponse` |
| `vibeui/crates/vibe-ai/src/multi_agent.rs` | 13, 14 | Typed agent roles, inter-agent messaging |
| `vibeui/crates/vibe-ai/src/providers/claude.rs` | 12, 13 | Token usage parsing, extended thinking, apiKeyHelper |
| `vibeui/crates/vibe-ai/src/providers/openai.rs` | 12, 13 | Token usage parsing, apiKeyHelper |
| `vibeui/crates/vibe-ai/src/trace.rs` | 12, 13 | Named sessions, rewind checkpoint messages |
| `vibeui/crates/vibe-ai/src/planner.rs` | 14 | opusplan provider injection |

### VibeUI React/TypeScript Files

| File | Phases | Role |
|------|--------|------|
| `vibeui/src/App.tsx` | 12, 13, 14 | Chat tabs, new panel tabs, Cmd+K, Tab prediction registration |
| `vibeui/src/components/AIChat.tsx` | 12 | Per-tab state, provider/model selector props |
| `vibeui/src/components/ContextPicker.tsx` | 12, 14 | `@symbol`, `@codebase`, `@folder`, `@terminal`, `@docs`, line ranges |
| `vibeui/src/components/AgentPanel.tsx` | 12, 13, 14 | Chunk-level diff trigger, cost display, linter auto-fix |
| `vibeui/src/components/CheckpointPanel.tsx` | 13 | Named checkpoint descriptions sidecar |

### New Files to Create

| File | Phase | Purpose |
|------|-------|---------|
| `vibeui/src/components/ChatTabManager.tsx` | 12 | Multiple chat tabs manager |
| `vibeui/src/components/SettingsPanel.tsx` | 12 | BYOK API key management |
| `vibeui/src/components/DiffReviewPanel.tsx` | 12 | Chunk-level diff accept/reject |
| `vibeui/src/components/InlineChat.tsx` | 14 | Inline Cmd+K edit overlay |
| `vibeui/src/components/McpManagerPanel.tsx` | 13 | MCP server management UI |
| `vibeui/src/components/RulesPanel.tsx` | 14 | Rules directory UI |
| `vibeui/src/utils/LinterIntegration.ts` | 14 | Post-write linter runner |
| `vibeui/src/utils/DocsResolver.ts` | 14 | Library doc fetcher |
| `vibeui/src/utils/SupercompleteEngine.ts` | 15 | Cross-file edit prediction |
| `vibeui/crates/vibe-ai/src/rules.rs` | 13 | Rules directory loader |
| `vibecli/vibecli-cli/src/memory_recorder.rs` | 13 | Auto memory recording |
| `vibe-indexer/` (new crate) | 15 | Remote codebase indexing service |
| `jetbrains-plugin/` | 15 | JetBrains IDE plugin |
| `.github/workflows/pr-bot.yml` | 15 | GitHub PR bot workflow |

---

## 10. Implementation Sequence Recommendations

**Start Phase 12 with** the highest-leverage items first:
1. `/model` switching (small, unblocks per-chat model in VibeUI)
2. `/cost` display (small, high visibility)
3. Multiple chat tabs (medium, high user value)
4. `UserPromptSubmit` hook (medium, closes visible gap)
5. Image attachment `-i` flag (tiny, `ImageAttachment::from_path` already exists)

**Start Phase 13 with:**
1. Rules directory (enables path-targeted context injection in Phase 14)
2. Auto memory recording (differentiates from all competitors)
3. Wildcard tool patterns (closes admin policy gap cleanly)

**Start Phase 14 with:**
1. Inline chat Cmd+K (largest competitive gap vs Cursor, highest effort — start early)
2. Linter integration (leverages existing LSP plumbing in vibe-lsp)
3. opusplan routing (small change, big marketing value)

**Phase 15 items** should be evaluated quarterly based on user demand:
- JetBrains plugin → enterprise adoption
- Remote indexing → large monorepos
- GitHub PR bot → organic discovery

---

*Generated from codebase audit and competitor analysis. All file paths are absolute references to the VibeCody monorepo.*
