---
layout: page
title: Competitive Roadmap v2 — Surpassing the Competition
permalink: /roadmap-v2/
---

# VibeCody Competitive Roadmap v2

**Date:** February 2026
**Scope:** Detailed fit-gap analysis and feature-by-feature implementation plan to surpass Codex CLI, Claude Code (VibeCLI), and Cursor, Windsurf, Google Antigravity (VibeUI).

---

## 1. Current State (All Phases Complete)

All nine roadmap phases (1–5 original, 6–9 in this document) are complete. VibeCody has:

| Feature | VibeCLI | VibeUI |
|---------|---------|--------|
| Agent loop (plan→act→observe) | ✅ 30-step max, streaming | ✅ Full panel UI |
| 7 tools (read/write/patch/bash/search/list/complete) | ✅ | ✅ (via Tauri commands) |
| 3 approval tiers (Suggest / AutoEdit / FullAuto) | ✅ | ✅ dropdown |
| 5 AI providers (Ollama/Claude/OpenAI/Gemini/Grok) | ✅ | ✅ |
| Streaming responses | ✅ | ✅ |
| Codebase indexing (regex/heuristic + embeddings) | ✅ | ✅ |
| Memory/rules system (VIBECLI.md, AGENTS.md) | ✅ | ✅ MemoryPanel |
| MCP client (STDIO, JSON-RPC 2.0) | ✅ | — |
| CI/non-interactive mode (--exec) | ✅ | — |
| Multimodal input (Claude + OpenAI vision) | ✅ | ✅ AIChat UI |
| OS sandbox (sandbox-exec / bwrap) | ✅ | ✅ |
| Trace/audit log (JSONL per session) | ✅ | ✅ HistoryPanel |
| Diff review before apply | — | ✅ Monaco DiffEditor |
| Inline AI completions (FIM) | — | ✅ |
| @ context system | — | ✅ |
| Flow tracker (ring buffer + auto-injection) | ✅ | ✅ |
| WASM extension system (wasmtime) | — | ✅ |
| Checkpoint system | — | ✅ backend + CheckpointPanel UI |
| LSP integration | — | ✅ |
| Hooks system (events + shell + LLM handlers) | ✅ | ✅ (via config) |
| Plan Mode (PlannerAgent) | ✅ /plan command | ✅ Agent panel toggle |
| Session resume | ✅ --resume flag | — |
| Web search tool | ✅ | ✅ |
| Shell environment policy / Admin policy | ✅ | — |
| Parallel multi-agent (git worktrees) | ✅ --parallel flag | ✅ ManagerView |
| Embedding-based semantic indexing | ✅ | ✅ |
| Code review agent | ✅ vibecli review | ✅ GitPanel review |
| Skills system | ✅ | ✅ |
| Artifacts | ✅ | ✅ ArtifactsPanel |
| OpenTelemetry | ✅ | — |
| GitHub Actions | ✅ | — |
| Red team pentest pipeline (5-stage) | ✅ --redteam + /redteam | ✅ RedTeamPanel |
| OWASP/CWE static scanner (15 patterns) | ✅ bugbot.rs | ✅ BugBotPanel |
| Code Complete workflow (8-stage) | ✅ /workflow | ✅ WorkflowPanel |
| LSP diagnostics panel | ✅ /check TUI command | — |
| Session sharing | ✅ /share | — |
| @jira context | ✅ @jira:PROJECT-123 | ✅ ContextPicker |
| MCP OAuth install flow | — | ✅ McpPanel OAuth modal |
| Custom domain / publish | — | ✅ DeployPanel domain config |
| VibeCLI daemon (serve) | ✅ | — |
| VS Code extension | ✅ | — |
| Agent SDK (TypeScript) | ✅ | — |

---

## 2. Competitive Analysis

### 2.1 VibeCLI vs. Codex CLI (TuringWorks) and Claude Code

#### Codex CLI Key Capabilities

- **OS-level sandbox** with a two-axis security model (sandbox mode × approval policy configured independently)
- **Shell environment policy** — `shell_environment_policy` controls exactly which env vars subprocesses inherit (all / core / none / include/exclude patterns)
- **Web search tool** — cached, live, or disabled; first-class tool alongside file tools
- **Session resume** — `codex resume` restores full session transcript, files, draft, and approvals
- **Code review agent** — dedicated mode that diffs against branches or commits and produces a structured review
- **OpenTelemetry** — native span export for enterprise CI observability
- **Admin policy enforcement** — `requirements.toml` org-wide enforcement; `approval_policy.reject.mcp_elicitations = true` per category
- **Cloud tasks** — `codex cloud` launches and manages remote agent tasks
- **PTY-backed exec** — more robust unified exec tool (beta)
- **Per-server MCP controls** — tool allowlists/denylists, startup timeouts, bearer auth on HTTP servers
- **Multiple profiles** — named config sets with different providers / sandbox modes

#### Claude Code Key Capabilities (February 2026 state)

- **Hooks system** — 17 event types (`PreToolUse`, `PostToolUse`, `Stop`, `TaskCompleted`, `SubagentStart`, `WorktreeCreate`, ...); 3 handler types: shell command, single-turn LLM eval, full subagent (up to 50 turns); `updatedInput` allows hooks to mutate tool parameters before execution; `async: true` for non-blocking hooks
- **Subagents / Parallel agents** — up to 7 concurrent; built-in: Explore (Haiku, read-only, thoroughness levels), Plan (read-only), General-purpose (all tools), Bash; custom subagents defined as Markdown files with YAML frontmatter; `isolation: worktree` runs agent in auto-created git worktree
- **Agent Teams** (Opus 4.6) — multiple Claude Code instances with shared task list + dependency tracking, inter-agent messaging, per-agent dedicated context windows
- **Persistent subagent memory** — `memory: user|project|local` in frontmatter gives agent private files that survive context resets
- **Skills** — auto-activating context-loaded capabilities in `.claude/skills/`, activate without explicit invocation based on task context
- **Plugins** — distributable packages bundling commands + hooks + skills + agents + MCP servers; 12 official Anthropic plugins
- **Session portability** — `/teleport`, `/desktop` to move sessions between terminal, Desktop app, browser
- **IDE integrations** — VS Code, JetBrains, Desktop app, Web, iOS, Slack, GitHub Actions, GitLab CI/CD, Chrome extension
- **Agent SDK** — TypeScript (v0.2.34) + Python SDK for building custom agents
- **1M-token context** via Opus 4.6
- **CLAUDE.md hierarchical merging** — enterprise policy → user → project → directory-specific

#### VibeCLI Gaps — All Closed ✅

All previously-identified gaps have been closed:

| Gap | Status | Implementation |
|-----|--------|----------------|
| Hooks system | ✅ Closed | `vibe-ai/src/hooks.rs` — HookRunner with shell + LLM handlers |
| Parallel multi-agent | ✅ Closed | `vibe-ai/src/multi_agent.rs` — git worktrees |
| Plan Mode | ✅ Closed | `vibe-ai/src/planner.rs` — PlannerAgent |
| Session resume | ✅ Closed | `vibe-ai/src/trace.rs` — SessionSnapshot + load_session |
| Web search tool | ✅ Closed | WebSearch + FetchUrl in ToolCall enum |
| Shell environment policy | ✅ Closed | `vibe-ai/src/policy.rs` — AdminPolicy |
| Code review agent | ✅ Closed | `vibecli/src/review.rs` — GitHub PR posting |
| OpenTelemetry | ✅ Closed | `vibe-ai/src/otel.rs` + `vibecli/src/otel_init.rs` |
| Admin policy enforcement | ✅ Closed | `vibe-ai/src/policy.rs` |
| Skills system | ✅ Closed | `vibe-ai/src/skills.rs` — SkillLoader |
| Cloud/remote tasks | ✅ Closed | `serve.rs` job persistence (`~/.vibecli/jobs/`), GET /jobs, cancel; BackgroundJobsPanel |
| Agent SDK | ✅ Closed | `packages/agent-sdk/` — TypeScript SDK |

---

### 2.2 VibeUI vs. Cursor, Windsurf, Google Antigravity

#### Cursor (v2.0, October 2025) Key Capabilities

- **Tab model** — proprietary always-on low-latency model; predicts multi-line edits AND next cursor position AND required imports; never stops running
- **Composer model** — mixture-of-experts, RL-trained in real codebases, 4x faster than comparable models; can launch integrated Chromium browser to test/debug web apps
- **8-way parallel agents** — each in its own git worktree or remote machine; ensemble approach for competing solutions
- **Background agents** (beta) — remote, sandboxed; clone + branch + push without local IDE
- **BugBot** — integrates with GitHub PRs; automatic diff analysis, inline bug comments with fixes
- **Embedding-based codebase index** — encrypted paths, plaintext discarded after embedding; background indexing; `@folders` context injection
- **`.cursorrules`** — project-level persistent AI context file

#### Windsurf (Wave 13, December 2025) Key Capabilities

- **Supercomplete** — next-edit prediction: rename variable → AI suggests all subsequent renames; predicts intent not just token
- **Real-time flow awareness** — Cascade continuously observes file edits, cursor movements, terminal output without prompting; developer never has to re-contextualize the AI
- **Persistent cross-session memory** — auto-learned coding style + manual rules; survives context window resets; builds per-developer personality model
- **SWE-1.5** — proprietary model: Claude 4.5-quality at 13x speed; purpose-trained for edit-run-test agentic loops; supports images
- **Plan Mode** — distinct planning phase before code execution; plan presented for review before execution
- **Named checkpoints** per conversation — full project state snapshots, revertible at any time
- **Agent Skills** — standardized execution templates, auto-invoked by matching prompts
- **Parallel agents** (Wave 13) — git worktrees, side-by-side panes, dedicated zsh terminal
- **Turbo Mode** — fully autonomous terminal command execution without per-command confirmation
- **MCP integrations** — GitHub, Slack, Stripe, Figma, databases

#### Google Antigravity (Public Preview, November 2025) Key Capabilities

- **Manager View** — dedicated high-level orchestration layer; spawn/monitor/inspect multiple agents at task level, not file level; designed for teams running many parallel workstreams
- **Artifacts** — structured, inspectable deliverables: task lists, implementation plans, screenshots, browser recordings, diagrams; each artifact is commentable while agent continues running
- **Async feedback** — comment on artifact without interrupting agent execution (most unique capability in the field)
- **Multi-model** — Gemini 3 Pro/Flash natively; Claude Sonnet 4.5 + Opus 4.5; GPT-OSS 120B
- **Free during preview** — no cost barrier for adoption

#### VibeUI Gaps — All Critical/High Items Closed ✅

| Gap | Status | Implementation |
|-----|--------|----------------|
| Parallel multi-agent with UI | ✅ Closed | `ManagerView.tsx` — multi-agent task board |
| Plan Mode in VibeUI | ✅ Closed | AgentPanel "Plan first" toggle |
| Checkpoint UI | ✅ Closed | `CheckpointPanel.tsx` — timeline + restore |
| Next-edit prediction | ✅ Closed | Inline completion with edit tracking |
| Real-time flow injection | ✅ Closed | FlowTracker auto-injection into prompts |
| GitHub PR integration | ✅ Closed | `review.rs` + GitPanel review button |
| Artifacts system | ✅ Closed | `ArtifactsPanel.tsx` — rich cards + annotations |
| Manager View | ✅ Closed | `ManagerView.tsx` — 8 parallel agents |
| Embedding-based codebase index | ✅ Closed | `vibe-core/src/index/embeddings.rs` |
| Background agents (remote) | ✅ Closed | `serve.rs` job persistence + BackgroundJobsPanel; Jobs tab in AI panel |
| Agent Skills | ✅ Closed | `vibe-ai/src/skills.rs` |
| Async artifact feedback | ✅ Closed | ArtifactsPanel annotation queue |
| Browser integration for web apps | ✅ Closed | BrowserPanel.tsx (iframe + quick-launch chips); Browser tab in bottom panel |
| VS Code extension | ✅ Closed | `vscode-extension/src/extension.ts` |

---

## 3. VibeCody Differentiators to Exploit

These are our *current* advantages that we must protect and amplify:

| Differentiator | Why it matters |
|---------------|---------------|
| **Full Rust backend** | 10x lower memory than Electron; sub-100ms startup; no V8 heap issues at scale |
| **Ollama first-class** | Cursor/Windsurf treat local models as afterthoughts; we should be the *best* local-AI dev tool |
| **Privacy by design** | No telemetry, no cloud indexing, fully local; growing market demand |
| **Open source** | Inspect everything, self-host, community extensions |
| **CLI + GUI unified** | VibeCLI and VibeUI share crates; agent work done once applies both |
| **5 providers** | More than Cursor (3) or Windsurf (own + limited); unique for non-OpenAI shops |
| **Hooks system** (planned) | With ours, we can match Claude Code's most differentiated feature |

---

## 4. Implementation Plan — Phases 6–9

---

### Phase 6 — Hooks, Planning & Intelligence ✅ Complete

**Goal:** The two most powerful missing capabilities: a hooks system matching Claude Code's + planning mode matching Windsurf. Also: session resume, web search, flow injection.

---

#### 6.1 Hooks System

**Priority: Critical — Claude Code's most differentiated feature**

The hooks system intercepts every agent event and allows shell scripts or LLM evaluations to block, modify, or react to tool calls. This enables: guaranteed lint-on-edit, format-on-save, security enforcement, test gates, and custom CI policies — all independent of model behavior.

**New file:** `vibeui/crates/vibe-ai/src/hooks.rs`

```rust
// Core types
pub enum HookEvent {
    SessionStart,
    PreToolUse { call: ToolCall },
    PostToolUse { call: ToolCall, result: ToolResult },
    Stop { reason: StopReason },
    TaskCompleted { summary: String },
    SubagentStart { name: String },
    StreamChunk { text: String },
}

pub enum HookDecision {
    Allow,
    Block { reason: String },
    ModifyInput { updated: ToolCall }, // mutate tool params
    InjectContext { text: String },    // feed text back to model
}

pub enum HookHandler {
    Command { shell: String },         // exit 0=allow, exit 2=block
    Llm { prompt: String },            // single-turn eval returning {ok, reason}
}

pub struct HookConfig {
    pub event: String,            // "PreToolUse", "PostToolUse", etc.
    pub tools: Option<Vec<String>>, // tool name filter (regex)
    pub handler: HookHandler,
    pub async_exec: bool,         // non-blocking if true
}

pub struct HookRunner {
    configs: Vec<HookConfig>,
    provider: Arc<dyn AIProvider>,
}

impl HookRunner {
    pub async fn run(&self, event: HookEvent) -> HookDecision;
}
```

**Configuration in `~/.vibecli/config.toml`:**

```toml
[[hooks]]
event = "PostToolUse"
tools = ["write_file", "apply_patch"]
handler = { command = "sh .vibecli/hooks/format.sh" }

[[hooks]]
event = "PreToolUse"
tools = ["bash"]
handler = { command = "sh .vibecli/hooks/security-check.sh" }

[[hooks]]
event = "Stop"
handler = { command = "sh .vibecli/hooks/test-gate.sh" }
async = false  # must pass before session ends
```

**Hook payload via stdin (JSON):**

```json
{
  "event": "PreToolUse",
  "tool": "bash",
  "input": { "command": "rm -rf dist/" },
  "session_id": "1740000000"
}
```

**Hook response via stdout:**

```json
{ "allow": false, "reason": "Deletion blocked by security hook" }
// or:
{ "allow": true, "updatedInput": { "command": "rm -rf dist/ --dry-run" } }
// or (PostToolUse inject):
{ "context": "Format check failed: 3 warnings. Claude should fix before completing." }
```

**VibeUI:** Add hooks configuration panel in Settings. Show hook execution timeline in HistoryPanel alongside trace entries.

**Files:**

- `vibe-ai/src/hooks.rs` (new)
- `vibe-ai/src/agent.rs` (integrate HookRunner into agent loop)
- `vibecli-cli/src/config.rs` (add `[[hooks]]` array)
- `vibeui/src-tauri/src/commands.rs` (add hook management commands)
- `vibeui/src/components/HooksPanel.tsx` (new — config UI)

---

#### 6.2 Plan Mode (Planning Before Execution)

**Priority: Critical — Windsurf Wave 13 + Claude Code differentiator**

A dedicated planning phase separates reasoning from action. The model generates a structured plan; the user reviews and optionally edits it; then execution proceeds step by step against the approved plan.

**New file:** `vibeui/crates/vibe-ai/src/planner.rs`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionPlan {
    pub goal: String,
    pub steps: Vec<PlanStep>,
    pub estimated_files: Vec<String>,
    pub risks: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanStep {
    pub id: usize,
    pub description: String,
    pub tool: String,              // which tool will be used
    pub estimated_path: Option<String>,
    pub status: PlanStepStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PlanStepStatus { Pending, InProgress, Done, Failed, Skipped }

pub struct PlannerAgent {
    provider: Arc<dyn AIProvider>,
}

impl PlannerAgent {
    /// Generate a structured execution plan without executing anything.
    pub async fn plan(&self, task: &str, context: &AgentContext) -> Result<ExecutionPlan>;

    /// Execute a previously approved plan step by step.
    pub async fn execute(
        &self,
        plan: &ExecutionPlan,
        executor: Arc<dyn ToolExecutorTrait>,
        approval: ApprovalPolicy,
        event_tx: mpsc::Sender<AgentEvent>,
    ) -> Result<String>;
}
```

**REPL:** `/plan <task>` — generates plan, shows it formatted, asks "Edit plan? (y/N) → Execute? (y/N)"

**VibeUI Agent Panel:** Add "Plan first" toggle. When enabled: run planner → display `ExecutionPlan` as editable todo list → "Execute Plan" button triggers executor.

**Plan prompt format (injected):**

```
You are a planning agent. Your task: <task>

Generate a detailed execution plan as JSON matching this schema:
{"goal": "...", "steps": [{"id": 1, "description": "...", "tool": "read_file", "estimated_path": "src/..."}], "estimated_files": [...], "risks": [...]}

DO NOT execute any actions. Generate ONLY the JSON plan.
```

---

#### 6.3 Session Resume

**Priority: High**

Extend trace storage to include full message history. `vibecli --resume <session-id>` restores complete conversation state.

**Extend `TraceWriter`:**

```rust
impl TraceWriter {
    /// Save full message history to <session_id>-messages.json alongside JSONL
    pub fn save_messages(&self, messages: &[Message]) -> Result<()>;

    /// Save agent context snapshot
    pub fn save_context(&self, ctx: &AgentContext) -> Result<()>;
}

/// Load a previous session's messages and context for resume
pub fn load_session(session_id: &str, dir: &Path) -> Result<SessionSnapshot>;

pub struct SessionSnapshot {
    pub messages: Vec<Message>,
    pub context: AgentContext,
    pub trace: Vec<TraceEntry>,
}
```

**CLI:** `vibecli --resume <session-id>` picks up where the session left off. `/resume` REPL command lists resumable sessions.

---

#### 6.4 Web Search Tool

**Priority: High — Codex has it, we don't**

Add `WebSearch` to `ToolCall` enum. Use DuckDuckGo's JSON API (no API key) with an optional Google CSE config.

```rust
// In tools.rs:
pub enum ToolCall {
    // ... existing variants ...
    WebSearch { query: String, num_results: usize },  // NEW
    FetchUrl { url: String },                          // NEW
}
```

**Tool system prompt addition:**

```text
web_search(query, num_results=5): Search the web for current information.
  Returns: list of {title, url, snippet}
fetch_url(url): Fetch and summarize a web page.
  Returns: page title + text content (truncated to 4000 chars)
```

**Implementation:** `DuckDuckGo Instant Answer API` for search, `reqwest` for URL fetching with `readability`-style content extraction.

**Config:**

```toml
[tools.web_search]
enabled = true
engine = "duckduckgo"   # "duckduckgo" | "google"
# google_cse_id = "..."  # optional
# google_api_key = "..."
max_results = 5
```

---

#### 6.5 Flow Context Auto-Injection

**Priority: High — Windsurf's core differentiator**

The flow tracker already records events. The missing piece: inject recent activity into every AI prompt automatically, giving the model full awareness without the user having to re-explain.

**In `AgentLoop::build_system_prompt()`:**

```rust
let flow_ctx = flow_tracker.context_string(10);
if !flow_ctx.is_empty() {
    system += &format!("\n\n## Recent Developer Activity\n{}", flow_ctx);
}
```

**Flow context format:**

```text
## Recent Developer Activity
[2m ago] Opened src/auth/login.rs (line 42)
[3m ago] Edited src/auth/login.rs — lines 38-55 changed
[5m ago] Ran: cargo test auth -- FAILED (2 tests)
[7m ago] Opened Cargo.toml
[9m ago] Edited src/auth/mod.rs — lines 1-10 changed
```

This appears in every agent request, giving the model full situational awareness.

**Also:** Inject flow context into VibeUI's AIChat `onSubmit` handler, not just the agent.

---

#### 6.6 Shell Environment Policy

**Priority: High — Codex differentiator for CI**

Fine-grained control over what environment variables subprocess tool calls inherit.

```toml
[safety.shell_environment]
inherit = "core"        # "all" | "core" | "none"
include = ["CARGO_HOME", "RUSTUP_HOME", "PATH"]
exclude = ["AWS_SECRET_*", "GITHUB_TOKEN", "*_API_KEY"]
set = { VIBECLI_AGENT = "1", CI = "true" }
```

**`ToolExecutor` change:**

```rust
fn build_env(policy: &ShellEnvPolicy) -> HashMap<String, String> {
    let base = match policy.inherit {
        "all" => std::env::vars().collect(),
        "core" => core_env_vars(), // PATH, HOME, USER, SHELL, TERM, LANG
        "none" => HashMap::new(),
    };
    // apply include, exclude, and set rules
}
```

---

### Phase 7 — Parallel Agents & Intelligence Upgrades ✅ Complete

**Goal:** Ship parallel multi-agent execution (closes the biggest throughput gap vs. Cursor/Windsurf), upgrade codebase indexing to embeddings, and ship next-edit prediction.

---

#### 7.1 Parallel Multi-Agent (Git Worktrees)

**Priority: Critical — both Cursor (8) and Windsurf (Wave 13) have this**

**VibeCLI:**

```bash
# Run 3 agents in parallel, each in its own worktree
vibecli --agent "refactor auth module" --parallel 3

# Or split a complex task across specialized agents
vibecli --multi-agent tasks.json  # JSON array of subtasks
```

**Architecture:** `MultiAgentOrchestrator` spawns N `AgentLoop` instances, each operating on a separate `git worktree`.

**New file:** `vibe-ai/src/multi_agent.rs`

```rust
pub struct MultiAgentOrchestrator {
    provider: Arc<dyn AIProvider>,
    approval: ApprovalPolicy,
    executor: Arc<dyn ToolExecutorTrait>,
    max_agents: usize,
}

pub struct AgentInstance {
    pub id: usize,
    pub task: String,
    pub worktree: PathBuf,
    pub branch: String,
    pub status: AgentStatus,
    pub steps: Vec<AgentStep>,
}

pub enum OrchestratorEvent {
    AgentStarted { id: usize, task: String },
    AgentStep { id: usize, step: AgentStep },
    AgentComplete { id: usize, summary: String, branch: String },
    AgentError { id: usize, error: String },
    AllComplete { results: Vec<AgentResult> },
}

impl MultiAgentOrchestrator {
    /// Split one task N ways and run in parallel.
    pub async fn run_parallel(
        &self,
        task: &str,
        n: usize,
        event_tx: mpsc::Sender<OrchestratorEvent>,
    ) -> Result<Vec<AgentResult>>;

    /// Run different tasks on different agents simultaneously.
    pub async fn run_tasks(
        &self,
        tasks: Vec<AgentTask>,
        event_tx: mpsc::Sender<OrchestratorEvent>,
    ) -> Result<Vec<AgentResult>>;
}
```

**Worktree management (add to `vibe-core/src/git.rs`):**

```rust
pub fn create_worktree(repo: &Path, branch: &str, worktree_path: &Path) -> Result<()>;
pub fn remove_worktree(repo: &Path, worktree_path: &Path) -> Result<()>;
pub fn list_worktrees(repo: &Path) -> Result<Vec<WorktreeInfo>>;
pub fn merge_worktree_branch(repo: &Path, branch: &str) -> Result<MergeResult>;
```

**TUI:** New `/multi-agent` command shows a split view with N panes, one per agent. Each pane streams its own steps.

**VibeUI:** New **Parallel** tab in AI panel. Shows N side-by-side agent cards. "Merge Best" button diffs all outputs and lets user pick.

---

#### 7.2 Embedding-Based Codebase Indexing

**Priority: High — Cursor's core competitive moat**

Upgrade from regex-based symbol search to semantic search using local embeddings.

**New file:** `vibe-core/src/index/embeddings.rs`

```rust
/// Store and query vector embeddings using an HNSW index.
pub struct EmbeddingIndex {
    index: HnswMap<Vec<f32>, EmbeddingDoc>,
    provider: EmbeddingProvider,
}

pub struct EmbeddingDoc {
    pub file: PathBuf,
    pub chunk_start: usize,
    pub chunk_end: usize,
    pub text: String,
}

pub enum EmbeddingProvider {
    Ollama { model: String, api_url: String },
    OpenAI { api_key: String, model: String },
}

impl EmbeddingIndex {
    /// Embed and index all files in workspace.
    pub async fn build(workspace: &Path, provider: &EmbeddingProvider) -> Result<Self>;

    /// Incrementally update changed files.
    pub async fn update(&mut self, changed_files: &[PathBuf]) -> Result<()>;

    /// Semantic search: return top-k most relevant chunks.
    pub async fn search(&self, query: &str, k: usize) -> Result<Vec<SearchHit>>;
}
```

**Chunking strategy:**

- Split files into 512-token chunks at function/class boundaries (using tree-sitter or heuristic line-counting)
- Overlap: 64 tokens between chunks
- Max file size: 500KB (skip larger files)
- Skip: `.git/`, `target/`, `node_modules/`, `dist/`, generated files

**Config:**

```toml
[index]
enabled = true
embedding_provider = "ollama"
embedding_model = "nomic-embed-text"  # or "text-embedding-3-small"
rebuild_on_startup = false
max_file_size_kb = 500
```

**Integration with agent:** When an agent task starts, semantic search finds the most relevant files to include in context automatically.

---

#### 7.3 Next-Edit Prediction in VibeUI

**Priority: Critical — Cursor Tab / Windsurf Supercomplete**

The current inline completion returns a single completion at cursor. True next-edit prediction watches what you've edited and predicts what you'll want to change next — in a different location.

**Architecture:**

1. After every keystroke (debounced 150ms), capture: current file state, cursor position, last 5 edits with positions and timestamps
2. Send to fast model (Ollama `qwen2.5-coder:7b` or similar) with a next-edit prediction prompt
3. If the model predicts an edit at a *different* location than the cursor: show a ghost annotation at that location
4. User presses `Tab` to jump to predicted location and accept the edit

**New Tauri command:** `predict_next_edit`

```rust
#[tauri::command]
async fn predict_next_edit(
    state: State<'_, AppState>,
    current_file: String,
    content: String,
    cursor_line: u32,
    cursor_col: u32,
    recent_edits: Vec<EditEvent>,  // last 5 edits: {line, col, old, new, elapsed_ms}
    provider: String,
) -> Result<Option<NextEditPrediction>, String>;

pub struct NextEditPrediction {
    pub target_line: u32,
    pub target_col: u32,
    pub suggested_text: String,
    pub confidence: f32,
}
```

**Prediction prompt:**

```text
Recent edits in {file}:
1. Line 42: renamed `user_name` → `username`
2. Line 67: renamed `user_name` → `username`
3. Line 83: still has `user_name` (unchanged)

Predict the next edit the developer will make. Respond ONLY with JSON:
{"line": 83, "col": 15, "replacement": "username", "confidence": 0.95}
```

**Monaco integration:** When prediction arrives, render a dimmed inline decoration at target location. `Tab` key handler: if prediction pending and Tab pressed, jump + accept; otherwise normal tab behavior.

---

#### 7.4 Checkpoint UI in VibeUI

**Priority: Critical — backend done, ship the UI**

The Tauri backend already has `create_checkpoint`, `list_checkpoints`, `restore_checkpoint`. Ship the React UI.

**New file:** `vibeui/src/components/CheckpointPanel.tsx`

```tsx
interface Checkpoint {
  index: number;
  label: string;
  timestamp: number;
  oid: string;
}

export function CheckpointPanel({ workspacePath }) {
  // Timeline view: vertical list of checkpoints with age
  // Each entry: index, label, timestamp, "Restore" button
  // "Create Checkpoint" button at top with label input
  // Before restore: confirm dialog showing which files will change
}
```

**Add to AI panel tabs:** alongside Chat / Agent / Rules / History.

**Auto-create checkpoint:** When agent starts a task (especially in FullAuto mode), automatically create a checkpoint named `before-agent-<task-summary>`.

---

#### 7.5 GitHub PR Integration (BugBot Equivalent)

**Priority: High — Cursor BugBot is a major differentiator**

A dedicated code review agent mode that analyzes diffs and produces structured reviews.

**VibeCLI:**

```bash
# Review uncommitted changes
vibecli review

# Review specific branch vs main
vibecli review --branch feature/auth --base main

# Post review as GitHub PR comment
vibecli review --pr 42 --post-github
```

**New file:** `vibecli-cli/src/review.rs`

```rust
pub struct ReviewConfig {
    pub base_ref: String,           // "main" | commit SHA | branch name
    pub target_ref: String,
    pub post_to_github: bool,
    pub github_pr: Option<u32>,
    pub focus: Vec<ReviewFocus>,    // Security, Performance, Correctness, Style
}

pub struct ReviewReport {
    pub summary: String,
    pub issues: Vec<ReviewIssue>,
    pub suggestions: Vec<ReviewSuggestion>,
    pub score: ReviewScore,
}

pub struct ReviewIssue {
    pub file: String,
    pub line: u32,
    pub severity: Severity,         // Critical, Warning, Info
    pub category: ReviewFocus,
    pub description: String,
    pub suggested_fix: Option<String>,
}
```

**Review prompt strategy:**

1. Get full diff: `git diff <base>..<target>`
2. For large diffs: chunk by file, review each file separately
3. Aggregate results, deduplicate, rank by severity
4. Output: structured JSON + human-readable Markdown

**GitHub integration:** Use `gh` CLI to post review comments. Requires `GITHUB_TOKEN` in environment or config.

**VibeUI:** Add "Review" button in GitPanel that opens a ReviewPanel showing issues with file/line links.

---

### Phase 8 — Ecosystem Features ✅ Complete

**Goal:** Skills system, OpenTelemetry, Artifacts, GitHub Actions, agent configurability.

---

#### 8.1 Skills System

**Priority: Medium — Claude Code's "Skills" are auto-activating capabilities**

Skills are context-aware capability definitions that activate automatically when a task matches their description — no explicit invocation needed.

**Directory:** `.vibecli/skills/` in repo root (or `~/.vibecli/skills/` for global).

**`rust-safety.md` example:**

```markdown
---
name: rust-safety
description: Activated when working on Rust code safety, memory, or correctness
triggers: ["unsafe", "memory", "panic", "lifetime", "borrow"]
tools_allowed: [read_file, write_file, bash]
---

When editing Rust code, always:
1. Check for `unwrap()` calls that should be `?` or `expect()`
2. Verify all `unsafe` blocks have a `// SAFETY:` comment
3. After writing, run `cargo clippy -- -D warnings` via bash tool
4. Prefer `Arc<Mutex<T>>` over raw shared state
```

**Skill activation:** Before each agent request, scan `.vibecli/skills/` directory. For each skill, check if any `triggers` keyword appears in the task description or recent tool outputs. Activated skills' content is appended to the system prompt.

**Implementation:** Add `SkillLoader` to `vibe-ai/src/skills.rs`. Call before building system prompt in `AgentLoop`.

---

#### 8.2 OpenTelemetry Integration

**Priority: Medium — Enterprise/CI observability**

Emit OpenTelemetry spans for agent steps, enabling Jaeger/Grafana/Datadog observability in CI pipelines.

```toml
[otel]
enabled = false
endpoint = "http://localhost:4317"  # OTLP gRPC
service_name = "vibecli"
```

**Spans emitted:**

- `agent.session` — root span for entire agent run
- `agent.step` — one span per tool call (tool name, input summary, success, duration)
- `agent.hook` — one span per hook execution
- `agent.llm_call` — LLM API call with model, token counts, latency

**Crate:** `opentelemetry`, `opentelemetry-otlp`, `opentelemetry-sdk`

---

#### 8.3 Artifacts System (Antigravity-Inspired)

**Priority: High — genuinely novel UX**

Agents produce structured, inspectable, annotatable deliverables alongside text responses.

**New type in `vibe-ai/src/artifacts.rs`:**

```rust
pub enum Artifact {
    TaskList { items: Vec<TaskItem> },
    ImplementationPlan { steps: Vec<PlanStep>, files: Vec<String> },
    FileChange { path: String, diff: String },
    CommandOutput { command: String, stdout: String, exit_code: i32 },
    TestResults { passed: usize, failed: usize, output: String },
    ReviewReport { issues: Vec<ReviewIssue> },
}

pub struct AgentArtifact {
    pub id: String,
    pub artifact: Artifact,
    pub timestamp: u64,
    pub annotations: Vec<Annotation>,  // user comments
}

pub struct Annotation {
    pub text: String,
    pub timestamp: u64,
    pub applied: bool,  // has the agent incorporated this feedback?
}
```

**VibeUI:** New `ArtifactsPanel` renders artifacts as rich cards. Users can expand, annotate, and mark artifacts as "feedback applied." Annotations are queued and injected into the agent's next context window as: `"User feedback on artifact: <annotation>"`.

This enables **async feedback** — the user annotates while the agent continues working on the next step.

---

#### 8.4 GitHub Actions Integration

**Priority: Medium**

Official GitHub Action for running VibeCLI in CI:

**`.github/actions/vibecli/action.yml`:**

```yaml
name: VibeCLI Agent
description: Run a VibeCLI agent task in CI
inputs:
  task:
    description: Task for the agent to perform
    required: true
  provider:
    description: AI provider (ollama/claude/openai)
    default: claude
  approval:
    description: Approval policy (auto-edit/full-auto)
    default: auto-edit
  output-format:
    description: Report format (json/markdown)
    default: markdown
runs:
  using: composite
  steps:
    - name: Run VibeCLI agent
      shell: bash
      env:
        ANTHROPIC_API_KEY: ${{ inputs.anthropic-api-key }}
      run: |
        vibecli exec "${{ inputs.task }}" \
          --provider ${{ inputs.provider }} \
          --${{ inputs.approval }} \
          --output-format ${{ inputs.output-format }} \
          --output vibecli-report.md
```

**Use cases:**

- Auto-fix failing test: `task: "Fix the failing test in CI"`
- Auto-refactor: `task: "Add error handling to all public API functions"`
- Auto-review: `vibecli review --pr $PR_NUMBER --post-github`

---

### Phase 9 — Manager View & Scale ✅ Complete

**Goal:** Ship the high-level orchestration UI (Manager View), VS Code extension, and Agent SDK.

---

#### 9.1 Manager View in VibeUI

**Priority: High — Antigravity's most unique feature**

A dedicated orchestration dashboard for managing multiple parallel agents at the **task level**, not the file level.

**New React component:** `vibeui/src/components/ManagerView.tsx`

**Layout:**

```text
┌─────────────────────────────────────────────────────-┐
│  Manager View                          + New Agent   │
├──────────┬──────────┬──────────┬────────────────────-┤
│ Agent 1  │ Agent 2  │ Agent 3  │ Task Board          │
│ ──────── │ ──────── │ ──────── │ ──────────────────  │
│ Status:  │ Status:  │ Status:  │ ☐ Task 1 → Agent 1  │
│ Running  │ Done x   │ Pending  │ x Task 2 → Agent 2  │
│          │          │          │ z Task 3 → Agent 3  │
│ Step 3/? │ 12 steps │ queued   │                     │
│ [expand] │ [review] │ [assign] │ [+ Add Task]        │
└──────────┴──────────┴──────────┴────────────────────-┘
```

**Features:**

- Spawn up to 8 agents (matching Cursor), each in a git worktree
- Task board with dependency tracking (Task 3 depends on Task 2)
- Each agent card expandable to show step-by-step trace
- "Review Changes" for done agents: opens Monaco diff viewer
- "Merge Best" for parallel runs: pick winner or cherry-pick across agents
- Real-time progress via Tauri events

**Tauri commands:**

- `start_parallel_agents(tasks: Vec<AgentTask>)` — spawns orchestrator
- `get_orchestrator_status()` → `Vec<AgentInstance>`
- `merge_agent_branch(agent_id, strategy)` — merge worktree into main

---

#### 9.2 VS Code Extension

**Priority: Medium — critical for distribution**

A VS Code extension that provides VibeCLI/VibeUI capabilities inside VS Code.

**Extension capabilities:**

- **Chat panel** — sidebar chat powered by VibeCLI's agent
- **Inline completions** — register `InlineCompletionItemProvider`; delegate to VibeCLI's FIM endpoint
- **Agent mode** — `/agent <task>` command runs VibeCLI agent, streams steps into output panel
- **Status bar** — shows current provider, branch, last agent status

**Implementation approach:**

- VS Code extension communicates with a local VibeCLI daemon (`vibecli serve --port 7878`)
- Daemon exposes REST/WebSocket API: `POST /chat`, `POST /agent`, `GET /stream/<session-id>`
- Extension is thin TypeScript client over this API

**New file:** `vibecli-cli/src/serve.rs` — Axum HTTP server exposing VibeCLI capabilities

---

#### 9.3 Agent SDK

**Priority: Low-Medium — community/enterprise adoption**

A library that lets developers build custom agents using VibeCLI's infrastructure.

**Rust crate:** Publish `vibe-ai` as a standalone crate on crates.io.

**TypeScript package:** `@vibecody/agent-sdk` wraps the VibeCLI daemon API:

```typescript
import { VibeCLIAgent } from '@vibecody/agent-sdk';

const agent = new VibeCLIAgent({
  provider: 'claude',
  approval: 'full-auto',
  tools: ['read_file', 'write_file', 'bash', 'web_search'],
  hooks: [
    { event: 'PostToolUse', tools: ['write_file'], command: 'npm run lint' }
  ]
});

for await (const event of agent.run('Add TypeScript strict mode to all files')) {
  if (event.type === 'step') console.log(`[${event.tool}] ${event.summary}`);
  if (event.type === 'complete') console.log('Done:', event.summary);
}
```

---

## 5. Current Feature Matrix (All Phases Complete)

| Capability | VibeCLI | Codex CLI | Claude Code | Cursor | Windsurf | Antigravity |
|-----------|---------|-----------|-------------|--------|----------|-------------|
| Agent loop | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Parallel agents | ✅ 8-way | experimental | ✅ 7-way | ✅ 8-way | ✅ | ✅ async |
| Hooks system | ✅ | ❌ | ✅ 17 events | ❌ | ❌ | ❌ |
| Plan Mode | ✅ | ❌ | ✅ | ❌ | ✅ | ✅ |
| Web search tool | ✅ | ✅ | ✅ | ❌ | ❌ | ❌ |
| Session resume | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| OS sandbox | ✅ | ✅ | ✅ | ❌ | ❌ | ❌ |
| Shell env policy | ✅ | ✅ | ❌ | ❌ | ❌ | ❌ |
| Code review agent | ✅ | ✅ | ✅ | BugBot | ❌ | ❌ |
| MCP support | ✅ | ✅ | ✅ 300+ | ❌ | ✅ | ❌ |
| Multimodal | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Semantic indexing | ✅ | ❌ | ❌ | ✅ | ✅ | partial |
| OTel | ✅ | ✅ | ❌ | ❌ | ❌ | ❌ |
| GitHub Actions | ✅ | ✅ | ✅ | ❌ | ❌ | ❌ |
| Skills | ✅ | ❌ | ✅ | ❌ | ✅ | ❌ |
| Ollama first-class | ✅ | ❌ | ❌ | partial | partial | ❌ |
| Open source | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ |
| Rust native | ✅ | ✅ | ❌ | ❌ | ❌ | partial |

| Capability | VibeUI | Cursor | Windsurf | Antigravity |
|-----------|--------|--------|----------|-------------|
| Next-edit prediction | ✅ | ✅ Tab | ✅ Supercomplete | partial |
| Parallel agents + UI | ✅ Manager View | ✅ | ✅ | ✅ |
| Plan Mode | ✅ | ❌ | ✅ | ✅ |
| Checkpoints UI | ✅ | ❌ | ✅ | Artifacts |
| Flow injection | ✅ | ❌ | ✅ | ❌ |
| Artifacts | ✅ | ❌ | ❌ | ✅ |
| GitHub PR review | ✅ | BugBot | ❌ | ❌ |
| Semantic indexing | ✅ | ✅ | ✅ | partial |
| WASM extensions | ✅ | ✅ | ✅ | ❌ |
| Agent skills | ✅ | ❌ | ✅ | ❌ |
| Multi-provider (5+) | ✅ | partial | partial | ✅ |
| Rust native backend | ✅ | ❌ | ❌ | partial |

---

## 6. Architecture (All Phases Complete)

```text
vibecli-cli
├── REPL / TUI (streaming, hooks, /agent, /plan, /multi-agent, /review)
├── CI mode (--exec, --parallel, --review)
├── Server mode (vibecli serve — API for VS Code extension + SDK)
└── src/
    ├── ci.rs, review.rs, serve.rs
    └── hooks.rs (config loading)

vibe-ai
├── provider.rs       (AIProvider trait + ImageAttachment + vision)
├── agent.rs          (plan→act→observe + hook integration)
├── planner.rs        (PlannerAgent: plan generation + guided execution)
├── multi_agent.rs    (parallel agents on git worktrees)
├── hooks.rs          (HookRunner: command + llm handlers, event bus)
├── skills.rs         (SkillLoader: auto-activating context snippets)
├── artifacts.rs      (Artifact types, annotation queue)
├── mcp.rs            (McpClient JSON-RPC 2.0)
├── tools.rs          (ToolCall enum + WebSearch + FetchUrl)
└── trace.rs          (JSONL audit + session resume)

vibe-core
├── index/
│   ├── mod.rs, symbol.rs, content.rs
│   └── embeddings.rs  (HNSW index + Ollama/OpenAI embeddings)
├── context.rs         (smart context builder: flow + semantic + git)
├── executor.rs        (sandboxed execution + shell env policy)
└── git.rs             (worktree: create, remove, merge)

vibe-extensions
└── loader.rs          (wasmtime WASM host)

vibeui (React + Tauri)
├── AgentPanel         (single-agent: steps, approval, artifacts)
├── ManagerView        (multi-agent: task board, worktrees, merge)
├── CheckpointPanel    (timeline, restore, auto-checkpoint)
├── ArtifactsPanel     (rich cards, annotations, async feedback)
├── HooksPanel         (hooks configuration UI)
├── MemoryPanel        (rules editor)
├── HistoryPanel       (trace viewer)
├── GitPanel           (git + PR review)
└── components/
    └── ReviewPanel    (code review issues with file/line links)
```

---

## 7. Completed Implementation Backlog

### Phase 6 ✅ Complete

| # | Feature | Gap Closed vs. | Status |
|---|---------|----------------|--------|
| 1 | Hooks system (events + shell + LLM handlers) | Claude Code | ✅ Done |
| 2 | Plan Mode (PlannerAgent + approval flow) | Windsurf, Claude Code | ✅ Done |
| 3 | Web search tool | Codex CLI | ✅ Done |
| 4 | Flow context auto-injection | Windsurf | ✅ Done |
| 5 | Shell environment policy | Codex CLI | ✅ Done |
| 6 | Session resume | Codex CLI | ✅ Done |

### Phase 7 ✅ Complete

| # | Feature | Gap Closed vs. | Status |
|---|---------|----------------|--------|
| 7 | Parallel multi-agent (git worktrees) | Cursor, Windsurf | ✅ Done |
| 8 | Embedding-based semantic indexing | Cursor, Windsurf | ✅ Done |
| 9 | Next-edit prediction in VibeUI | Cursor Tab, Windsurf Supercomplete | ✅ Done |
| 10 | Checkpoint UI in VibeUI | Windsurf | ✅ Done |
| 11 | GitHub PR review agent | Cursor BugBot | ✅ Done |

### Phase 8 ✅ Complete

| # | Feature | Gap Closed vs. | Status |
|---|---------|----------------|--------|
| 12 | Skills system | Claude Code, Windsurf | ✅ Done |
| 13 | Artifacts panel in VibeUI | Antigravity | ✅ Done |
| 14 | OpenTelemetry spans | Codex CLI | ✅ Done |
| 15 | GitHub Actions workflow | Codex CLI, Claude Code | ✅ Done |
| 16 | Hooks config UI in VibeUI | — | ✅ Done |
| 17 | Turbo Mode (VibeUI FullAuto toggle) | Windsurf | ✅ Done |

### Phase 9 ✅ Complete

| # | Feature | Gap Closed vs. | Status |
|---|---------|----------------|--------|
| 18 | Manager View (VibeUI parallel orchestration) | Antigravity | ✅ Done |
| 19 | VS Code extension | Cursor, Windsurf, all | ✅ Done |
| 20 | VibeCLI daemon (`vibecli serve`) | Enables SDK + extension | ✅ Done |
| 21 | Agent SDK (TypeScript) | Claude Code | ✅ Done |
| 22 | Admin policy enforcement | Codex CLI | ✅ Done |

---

## 7.10 Phase 41 — Red Team Security Testing ✅

**Status:** Complete
**Competitor reference:** Shannon (KeygraphHQ) — autonomous AI-powered pentesting framework
**Comparison:** [`docs/SHANNON-COMPARISON.md`](/shannon-comparison/)

| Item | Status | Details |
|------|--------|---------|
| `redteam.rs` — 5-stage autonomous pentest pipeline | ✅ | Recon → Analysis → Exploitation → Validation → Report; RedTeamConfig, RedTeamSession, VulnFinding, AttackVector (15 types), CvssSeverity with CVSS scoring, RedTeamManager at `~/.vibecli/redteam/` |
| Expanded CWE scanner (bugbot.rs) | ✅ | 8 new patterns: CWE-918 SSRF, CWE-611 XXE, CWE-502 deserialization, CWE-943 NoSQL injection, CWE-1336 template injection, CWE-639 IDOR, CWE-352 CSRF, CWE-319 cleartext; total: 15 CWE patterns |
| CLI flags | ✅ | `--redteam <url>`, `--redteam-config <file>`, `--redteam-report <session-id>` |
| REPL commands | ✅ | `/redteam` with sub-commands: scan, list, show, report, config; tab-completion + hints |
| Config section | ✅ | `[redteam]` in config.toml: max_depth, timeout_secs, parallel_agents, scope_patterns, exclude_patterns, auth_config, auto_report |
| RedTeamPanel.tsx | ✅ | Pipeline stage visualization, target URL input, findings feed with severity badges + CVSS scores, expand-to-details with PoC + remediation, report export button; 🛡️ RedTeam tab in AI panel |
| Tauri commands | ✅ | start_redteam_scan, get_redteam_sessions, get_redteam_findings, generate_redteam_report, cancel_redteam_scan |
| Shannon comparison doc | ✅ | `docs/SHANNON-COMPARISON.md` — full feature matrix, architectural comparison, integration opportunities |

---

## 7.11 Phase 42 — Jira Context, MCP OAuth, Custom Domains ✅

**Status:** Complete

| Item | Status | Details |
|------|--------|---------|
| `@jira:PROJECT-123` context | ✅ | VibeCLI `expand_at_refs()` + VibeUI `resolve_at_references()` + `ContextPicker.tsx` autocomplete; Jira REST API v2 with basic auth; env vars: `JIRA_BASE_URL`, `JIRA_EMAIL`, `JIRA_API_TOKEN` |
| MCP OAuth install flow | ✅ | `McpPanel.tsx` two-step modal (configure → paste auth code); 3 Tauri commands (`initiate_mcp_oauth`, `complete_mcp_oauth`, `get_mcp_token_status`); tokens at `~/.vibeui/mcp-tokens.json`; green 🔑 badge |
| Custom domain / publish | ✅ | `DeployPanel.tsx` domain input + `set_custom_domain` Tauri command; Vercel REST API with `VERCEL_TOKEN`; CNAME instructions for other targets |

---

## 8. VibeCody Wins — Competitive Position

With all phases complete, VibeCody is the **only** developer toolchain that combines:

| - | - | - |
|---|---------|----------------|
|1.| **Open source + fully local** | inspect every line, self-host, no telemetry |
|2.| **Rust native backend** | sub-100ms startup, <50MB memory vs. 300MB+ Electron |
|3.| **Hooks system depth** | matches Claude Code's 17-event architecture; no Electron IDE has this |
|4.| **Ollama first-class** | best local AI experience; Cursor/Windsurf treat it as an afterthought |
|5.| **CLI + GUI unified** | VibeCLI and VibeUI share the same agent, same tools, same memory |
|6.| **OS-level sandbox** | genuine security isolation, not just permission dialogs |
|7.| **5+ providers** | the only tool that's truly multi-cloud + local AI |
|8.| **Privacy by design** | embeddings computed locally via Ollama, code never leaves your machine |
|9.| **Shell environment policy** | production-grade CI env control matching Codex CLI |
|10.| **Artifacts + Manager View** | Antigravity-style orchestration in an open-source tool |
