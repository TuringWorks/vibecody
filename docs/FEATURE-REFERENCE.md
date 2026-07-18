# VibeCody Feature Reference

> Deep-dive documentation for every major feature area. For the quick matrix view see [FEATURE-MATRIX.md](FEATURE-MATRIX.md).

---

## Table of Contents

1. [AI Providers](#1-ai-providers)
2. [Chat & Conversation](#2-chat--conversation)
3. [Agent System](#3-agent-system)
4. [Code Editing (VibeCoder)](#4-code-editing-vibecoder)
5. [Context & Memory Management](#5-context--memory-management)
6. [Code Review & Analysis](#6-code-review--analysis)
7. [Testing & Quality](#7-testing--quality)
8. [Git Integration](#8-git-integration)
9. [Session Management](#9-session-management)
10. [Security & Sandbox](#10-security--sandbox)
11. [MCP Support](#11-mcp-model-context-protocol)
12. [Recipes & Automation](#12-recipes--automation)
13. [Observability & Cost](#13-observability--cost)
14. [LSP & Language Support](#14-lsp--language-support)
15. [Plugins & Extensions](#15-plugins--extensions)
16. [Collaboration](#16-collaboration)
17. [Deployment & Infrastructure](#17-deployment--infrastructure)
18. [Daemon & API Mode](#18-daemon--api-mode)
19. [RL-OS (Advanced Training)](#19-rl-os)
20. [UI Panels (VibeCoder)](#20-ui-panels-vibecoder)
21. [Advanced Runtime Capabilities (FIT-GAP v12)](#21-advanced-runtime-capabilities-fit-gap-v12)
22. [Goals — Durable Execution Intent](#22-goals--durable-execution-intent)
23. [Plugin Governance (signed MCPB bundles)](#23-plugin-governance-signed-mcpb-bundles)

---

## 1. AI Providers

### Configuration

Providers are configured in `~/.vibecli/config.toml`. Each provider section accepts `api_key`, `model`, and `api_url` (for local/custom endpoints).

```toml
[claude]
api_key = "sk-ant-..."
model = "claude-sonnet-4-6"

[ollama]
api_url = "http://localhost:11434"
model = "llama3"
```

### Provider Failover

Chain multiple providers so VibeCLI automatically retries on the next if one fails:

```toml
failover_chain = ["claude", "openai", "ollama"]
```

### Per-Tab Override (VibeCoder)

Each chat tab in VibeCoder has its own provider selector. Changing it marks the tab as "manually overridden" (shown in gold). Resetting follows the top-bar selection.

### Resilient Provider

All providers are wrapped with `ResilientProvider` which adds:

- Exponential backoff on transient errors
- Rate-limit detection and waiting
- Health score tracking (exposed in VibeCoder status bar)

---

## 2. Chat & Conversation

### Streaming

All chat uses token-by-token Server-Sent Events from the Tauri backend. Tokens/sec is displayed live. Stop button cancels in-flight requests via `stop_chat_stream`.

### Auto-Compaction

When conversation history exceeds **80,000 characters**, VibeCLI/VibeCoder:

1. Waits for the current response to complete
2. Calls `summarise_messages` Tauri command on messages before the last 20
3. Replaces them with a single summary message (marked with a "Conversation compacted" divider)
4. Requires 10,000+ new characters before triggering again

Threshold is defined as `COMPACTION_THRESHOLD = 80_000` in `AIChat.tsx`.

### Chat Memory Panel (VibeCoder)

The collapsible **Memory** strip below each chat tab:

- Extracts facts automatically from assistant messages matching bullet-point or "Note:"/"Remember:" patterns
- **Pin** a fact → injected into every subsequent AI message as a system-prompt prefix
- **Edit** (click text inline), **delete** (×), **add manual note**
- Pinned facts persist to `localStorage` across sessions under key `vibecody:pinned-memory-facts`
- Max 50 pinned facts, max 100 session facts per tab

### Session History

- Auto-saved to `localStorage` when a tab is closed (if it has messages)
- Browse via **History** tab in the tab strip
- Restore into a new tab via **Restore** button
- Manually save at any time with the **Save** button

### Slash Commands (VibeCoder)

| Command | Behaviour |
|---|---|
| `/fix` | Prepends "Fix the following errors:" |
| `/explain` | Prepends "Explain the following code in detail:" |
| `/test` | Prepends "Generate comprehensive tests for:" |
| `/doc` | Prepends "Generate documentation for:" |
| `/refactor` | Prepends "Refactor the following code..." |
| `/review` | Prepends "Perform a thorough code review of:" |
| `/compact` | Summarise conversation into key points |

---

## 3. Agent System

### Approval Policies

| Policy | Behaviour | CLI Flag |
|---|---|---|
| `chat-only` | No tool calls; conversational only | `--approval chat-only` |
| `suggest` (default) | Prompt before every action | `--approval suggest` |
| `auto-edit` | Auto-apply file edits, prompt for shell | `--approval auto-edit` |
| `full-auto` | Execute all tools without prompting | `--approval full-auto` |

### Agent Modes

- **`/plan`** — generates a numbered execution plan, then waits for approval before acting
- **`/agent`** — full autonomous loop: plan → act → observe → repeat until done
- **`--exec`** — non-interactive CI mode; exits with code 1 on error

### Tool Execution

Tools are declared as XML in the system prompt and parsed from assistant responses. Supported tools: `read_file`, `write_file`, `apply_patch`, `list_directory`, `bash`, `search_files`, `web_search`, `fetch_url`, `spawn_agent`, `think`, `task_complete`.

### Multi-Agent Teams

```
/team create --role planner,executor,reviewer
/team run "Implement OAuth login with tests"
```

Teams share a knowledge graph and communicate via the A2A protocol. Roles are configurable; each sub-agent runs its own agent loop.

### Sub-Agent Spawning

The `spawn_agent` tool creates child agents. Results are returned to the parent. Children inherit the parent's provider config but can override it. Session depth is tracked (`parent_session_id`, `depth` in SQLite).

---

## 4. Code Editing (VibeCoder)

### Monaco Integration

VibeCoder uses Monaco Editor with `automaticLayout: true`. File writes from AI are split across two `requestAnimationFrame` callbacks to avoid ResizeObserver / React state race conditions:

- **Frame 0**: close diff overlay, invoke `write_file`
- **Frame 1**: show 30-second undo strip (`setLastApply`)
- **Frame 2**: sync Monaco model content (`setOpenFiles`, `setActiveFilePath`)

### Diff Review Panel

When the AI proposes a file write:

1. The diff is shown hunk-by-hunk with Myers LCS diff
2. Each hunk has **✓ Accept** / **✗ Reject** toggle buttons
3. **Accept All** / **Reject All** header buttons apply to all hunks
4. **Apply (N)** assembles the final file: accepted hunks use modified lines, rejected hunks revert to original
5. **Cancel** discards the proposal

LCS guard: files > 800,000 character-product fall back to a whole-file replace/delete diff.

### File Operations

- Create, rename, delete files and directories from the file tree
- Right-click context menu
- Drag-and-drop reordering
- Keyboard shortcuts (VS Code compatible)

---

## 5. Context & Memory Management

### Three Memory Layers

VibeCody provides three complementary memory systems, all injected into agent context automatically:

| Layer | Storage | Best for |
|-------|---------|----------|
| **Auto-recording** | `~/.vibecli/memory.md` | Short-term session learnings |
| **OpenMemory cognitive store** | `~/.local/share/vibecli/openmemory/` | Structured long-term memory, semantic search |
| **Verbatim drawers** (MemPalace) | `drawers.json` inside the same store | Lossless recall of runbooks, specs, logs |

### Auto-Recording (`memory.md`)

When `[memory] auto_record = true`, session learnings are appended automatically after sessions with ≥ `min_session_steps` tool calls. The file is injected verbatim into every future system prompt.

```toml
[memory]
auto_record = true
min_session_steps = 3
```

### OpenMemory Cognitive Engine (`open_memory.rs`)

**5 cognitive sectors** (each with independent decay rate):

| Sector | Decay/day | Purpose |
|--------|-----------|---------|
| Episodic | 0.015 | Events and experiences |
| Semantic | 0.005 | Facts and knowledge |
| Procedural | 0.008 | How-to and workflows |
| Emotional | 0.020 | Sentiment and reactions |
| Reflective | 0.001 | Auto-generated meta-patterns |

**Retrieval scoring** (composite of 5 signals):

```
score = 0.45 × semantic_similarity (HNSW + TF-IDF)
      + 0.20 × salience             (post-decay)
      + 0.15 × recency
      + 0.10 × waypoint_graph_score
      + 0.10 × sector_match_bonus
```

**4-layer context injection** — assembled before each agent turn:

| Layer | Always? | Content |
|-------|---------|---------|
| L0 | Yes | User identity header (~100 tokens) |
| L1 | Yes | Essential story — highest-salience memories (~700 tokens) |
| L2 | When relevant | Wing/Room-scoped semantic search, top-8 results |
| L3 | L2 fallback | Full semantic search + verbatim drawer chunks |

**Temporal knowledge graph** — subject/predicate/object facts with `valid_from`/`valid_to` windows. Adding a new fact for the same key auto-closes the previous one. Bi-temporal design supports point-in-time queries.

### Verbatim Drawers — MemPalace Technique (`open_memory.rs` DrawerStore)

Lossless 800-char chunk storage. No LLM summarisation. Achieves 96.6% Recall@5 on LongMemEval vs ~74% for cognitive-only stores.

Key parameters:

- **Chunk size**: 800 chars
- **Overlap**: 100 chars
- **Exact dedup**: FNV-1a hash (O(1))
- **Near-dedup**: cosine ≥ 0.85 within trailing 20-chunk window
- **Wing** (project namespace) + **Room** (sector) — spatial pre-filter before vector search

**Cross-project Tunnels** — bidirectional weighted waypoints between memories in different project stores. Created manually (`/openmemory tunnel`) or automatically (`/openmemory auto-tunnel [threshold]`).

**LongMemEval benchmark** — built-in recall@K evaluation across 20 probe cases spanning all 5 sectors. Available as `/openmemory benchmark [k]` (REPL), `GET /memory/benchmark?k=5` (HTTP daemon), and in the VibeCoder **Drawers** tab.

### VibeMemory — SQLite Vector Store (`vibe-memory/` crate)

**Local SQLite vector memory** for per-project and per-machine context, using rusqlite with ChaCha20-Poly1305 encryption. Zero external dependencies, no API keys, no network.

**Storage layout:**
```
~/.vibecli/
├── memory/
│   ├── global.db          ← computer-scoped store (all projects share)
│   └── workspaces/
│       └── {workspace-hash}/
│           └── memory.db  ← project-scoped store
```

**Key derivation:**
- Global key: `SHA-256("vibememory-global-v1:" + $HOME + ":" + $USER)`
- Project key: `SHA-256("vibememory-project-v1:" + $HOME + ":" + $USER + ":" + workspace_path)`

**API endpoints** (VibeCLI daemon):
| Route | Method | Purpose |
|-------|--------|---------|
| `/vibememory/store` | POST | Store memory (project or global) |
| `/vibememory/search` | POST | Semantic search with cosine similarity |
| `/vibememory/context` | POST | Assemble layered context for LLM injection |
| `/vibememory/list` | GET | List memories with optional sector filter |
| `/vibememory/stats` | GET | Store statistics (counts, sizes) |
| `/vibememory/consolidate` | POST | Apply decay + purge low-salience entries |

**Tauri commands** (VibeCoder):
| Command | Purpose |
|---------|---------|
| `vibememory_store` | Store memory entry |
| `vibememory_search` | Search by semantic similarity |
| `vibememory_context` | Get context string for LLM |
| `vibememory_list` | List all memories |
| `vibememory_stats` | Get store statistics |
| `vibememory_consolidate` | Run decay + purge |
| `vibememory_delete` | Delete entry by ID |

**Sector classification** — ML-lite keyword matching:
| Sector | Signals |
|--------|---------|
| Episodic | yesterday, today, session, happened, meeting |
| Semantic | means, defined, fact, concept, api, protocol |
| Procedural | step, how to, command, process, workflow, run |
| Emotional | frustrated, happy, love, hate, great, terrible |
| Reflective | realize, insight, pattern, lesson, principle |

**Consolidation** — periodic decay + purge:
- Decay applies per-sector lambda (episodic: 0.015/day, emotional: 0.020/day)
- Pinned memories immune to decay and purge
- Default purge threshold: salience < 0.1

### Workspace Rules

Rules are loaded in priority order:

1. **System rules** (built-in)
2. **User rules** (`~/.vibecli/rules/`)
3. **Project rules** (`.vibecli/rules/*.md`)
4. **Directory rules** (subdirectory `.vibehints` files)

Each `.md` file can include YAML front-matter:

```yaml
---
name: rust-safety
path_pattern: "**/*.rs"
---
When editing Rust files, always check for unwrap() calls...
```

Rules without `path_pattern` always inject. Rules with a pattern only inject when the open file matches.

### `.vibecoder.md` (VibeCoder-specific)

Place a `.vibecoder.md` file in the workspace root. It is injected as `## Project AI Rules` into every AI system prompt, guiding AI-generated code to follow project conventions (e.g., "use the custom Icon system, not lucide-react").

### Context Bundles

Named, shareable context packages stored as `.vibebundle.toml`:

- Pinned files, custom instructions, excluded paths, model preferences
- Up to 10 active bundles per session
- Priority ordering (lower = higher priority)

### Infinite Context Mode

For very large codebases, `infinite_context.rs` provides 5-level hierarchical compression:

| Level | Content |
|---|---|
| 0 | Full file content |
| 1 | Function-level summaries |
| 2 | File skeleton (signatures only) |
| 3 | Module-level summary (one sentence) |
| 4 | Project-level summary (architecture overview) |

Eviction uses a hybrid strategy combining recency decay, keyword match, file proximity, and access frequency.

---

## 6. Code Review & Analysis

### AI Code Review (`ai_code_review.rs`)

7 built-in detectors:

1. **Security** — OWASP Top 10 patterns
2. **Complexity** — cyclomatic complexity thresholds
3. **Style** — language-specific conventions
4. **Documentation** — missing/outdated docs
5. **Tests** — coverage gaps
6. **Duplication** — copy-paste detection
7. **Architecture** — coupling, cohesion, layer violations

Output: quality gate score (0–100), per-issue severity, Mermaid diagrams for PR summaries.

### Architecture Spec (`architecture_spec.rs`)

- **TOGAF ADM** — 9-phase architecture development
- **Zachman Framework** — 6×6 matrix
- **C4 Model** — Context → Container → Component → Code with Mermaid export
- **ADRs** — Architecture Decision Records with lifecycle management

### `/review` CLI Command

```bash
vibecli --review --base main --branch feature/auth
vibecli --review --pr 42 --post-github
```

---

## 7. Testing & Quality

### Auto-Detection

VibeCLI detects the test framework from the project structure:

- `Cargo.toml` → `cargo test`
- `package.json` (jest/vitest) → `npm test`
- `pytest.ini` / `pyproject.toml` → `pytest`
- `go.mod` → `go test ./...`

### Coverage

Coverage data is collected per-run and displayed in the VibeCoder CoveragePanel as:

- File-level line coverage percentages
- Branch coverage
- Trend over time (session-scoped)
- Diff coverage (lines changed in this session)

---

## 8. Git Integration

### Branch Agent

`/branch-agent` creates a branch, implements a feature, runs tests, and opens a PR — all autonomously. It uses `--worktree` isolation by default.

### Git Bisect (`/bisect`)

Interactive bisect workflow:

```
/bisect start --good v1.2.0 --bad HEAD
# VibeCLI runs tests at each midpoint, marks good/bad automatically
/bisect result
```

### Commit Generation

AI generates commit messages from the staged diff. Templates in `.vibecli/commit-template.txt` customize the format.

---

## 9. Session Management

### Storage

Sessions are stored in `~/.vibecli/sessions.db` (SQLite). Schema:

- `sessions` — id, task, provider, model, status, summary, step_count, parent_session_id, depth
- `messages` — role, content, created_at
- `steps` — step_num, tool_name, input_summary, output, success

### Forking

```bash
vibecli --fork <session-id>
```

Creates a child session copying all messages and steps. The child's task is prefixed with `[fork of <id>]`.

### Export

```bash
vibecli --export-session <id> --output session.md    # Markdown
vibecli --export-session <id> --output session.json  # JSON
vibecli --export-session <id> --output session.html  # HTML
```

### Trace Files

Every session writes:

- `~/.vibecli/traces/<id>.jsonl` — streaming event log
- `~/.vibecli/traces/<id>-messages.json` — full message history
- `~/.vibecli/traces/<id>-context.json` — context snapshot

---

## 10. Security & Sandbox

### OS-Level Sandbox

On macOS, the `sandbox-exec` profile restricts:

- File system access (read-only except workspace)
- Network access (configurable)
- Process spawning

On Linux, `bwrap` (Bubblewrap) provides similar isolation. Both are activated via `[sandbox] enabled = true` in config.

### Container Runtime (`ContainerRuntime` trait)

Unified interface over Docker, Podman, and OpenSandbox with 16 async methods: `create_container`, `start`, `exec`, `copy_in/out`, `logs`, `stats`, `kill`, `remove`, etc.

### Policy Engine (`policy_engine.rs`)

Cerbos-style RBAC/ABAC:

- 14 condition operators (eq, neq, in, startsWith, regex, etc.)
- Derived role computation from principal attributes
- Policy testing framework
- YAML serialization/import
- Conflict detection and coverage analysis

---

## 11. MCP (Model Context Protocol)

### Server Mode

```bash
vibecli --mcp-server
```

Starts a stdio-based MCP server. Compatible with any MCP client (Claude Desktop, Cursor, etc.). Implements JSON-RPC 2.0.

### Available Tools

| Tool | Description |
|---|---|
| `read_file` | Read file contents |
| `write_file` | Create or overwrite a file |
| `list_directory` | List directory contents |
| `bash` | Execute shell command |
| `search_files` | Regex search across workspace |
| `agent_run` | Spawn autonomous agent task |

### Multi-Server Support

Configure multiple MCP servers in `~/.vibecli/config.toml`:

```toml
[[mcp_servers]]
name = "github"
command = "npx @modelcontextprotocol/server-github"

[[mcp_servers]]
name = "linear"
command = "npx @linear/mcp-server"
```

---

## 12. Recipes & Automation

### Recipe Format

```yaml
name: add-feature
description: "Scaffold a new feature with tests"
provider: claude
parameters:
  feature_name:
    description: "Name of the feature"
    required: true
  module:
    description: "Module to add it to"
    default: "src"

steps:
  - prompt: "Create {{ feature_name }} in {{ module }} with full tests"
  - prompt: "Write documentation for {{ feature_name }}"
    provider: ollama  # per-step override
```

### Running Recipes

```bash
vibecli --recipe features/add-auth.yaml --param feature_name=OAuth
vibecli --recipe add-auth.yaml --dry-run          # preview without executing
vibecli --recipe add-auth.yaml --param key=value --param key2=value2
```

### Scheduling

```
/schedule --cron "0 9 * * 1" --recipe weekly-review.yaml
/remind "run tests" --in 30m
```

---

## 13. Observability & Cost

### Token Usage

Token counts are estimated at ~4 chars/token and tracked per:

- Individual messages
- Session totals
- Daily/weekly aggregates

### Cost Estimation

Provider-specific pricing tables map (input_tokens, output_tokens) → USD cost. Displayed in the `/cost` command and CostPanel in VibeCoder.

### Budget Alerts

```toml
[cost]
monthly_budget_usd = 50.0
alert_threshold = 0.8   # alert at 80% of budget
```

### OpenTelemetry

```bash
vibecli --otel-endpoint http://localhost:4318 agent "task"
```

Exports spans for: tool calls, API requests, agent steps, session lifecycle.

### Session Memory Profiling

`session_memory.rs` samples every 60 seconds and:

- Detects sustained growth (linear regression)
- Alerts at > 50% growth
- Triggers auto-compaction at 512 MB

---

## 14. LSP & Language Support

### Configuration

LSP servers are auto-detected or configured in `.vibecli/lsp.toml`:

```toml
[[servers]]
language = "rust"
command = "rust-analyzer"

[[servers]]
language = "python"
command = "pyright-langserver --stdio"
```

### Features

All LSP features are surfaced in Monaco (VibeCoder) and as structured output in VibeCLI review mode: go-to-definition, find-references, hover, rename, code actions, diagnostics, call hierarchy, workspace symbols.

---

## 15. Plugins & Extensions

### WASM Plugin System

Plugins are compiled to WASM and loaded at startup. Plugin API surface:

- Register custom REPL commands
- Hook into tool execution (pre/post)
- Add custom LSP capabilities
- Integrate external services

### Development

```bash
vibecli /plugin dev ./my-plugin     # hot-reload mode
vibecli /plugin install my-plugin   # from registry
vibecli /plugin list                # show loaded plugins
```

### Built-in Integrations

- **GitHub** — PR review bot, issue triage
- **Linear** — issue creation, status updates
- **Jira** — ticket management
- **Docker / Kubernetes** — container orchestration
- **Terraform / Pulumi** — infrastructure as code

---

## 16. Collaboration

### CRDT Multiplayer

VibeCoder uses CRDTs (Conflict-free Replicated Data Types) for real-time collaborative editing. Multiple users can edit the same file simultaneously with automatic conflict resolution. Presence indicators show remote cursors and selections.

### Session Sharing

```
/share                           # share current session
/share --format json             # export as JSON
/share --visibility team         # team-visible URL
```

Sessions can be shared as Markdown, JSON, or HTML exports. The `SessionSharingManager` supports annotations, secret redaction, visibility controls, and import/export.

### Agent Teams

```
/team create researcher,coder,reviewer
/team task "Implement rate limiting middleware with tests"
```

Team agents communicate via the A2A protocol. The knowledge graph is shared across agents. Team governance policies control which agents can take which actions.

---

## 17. Deployment & Infrastructure

### Cloud Providers

VibeCLI's `/deploy` command supports:

- **AWS**: EC2, Lambda, ECS, Fargate
- **Azure**: VM, ACI, Container Apps, Functions
- **GCP**: Compute Engine, Cloud Run, Cloud Functions
- **DigitalOcean**: Droplets, App Platform
- **Vercel**: Serverless functions, static sites

### Container Workflow

```bash
vibecli /docker build --tag myapp:latest
vibecli /container run --image myapp:latest --sandbox
vibecli /cloud deploy --provider aws --service ecs
```

### Kubernetes

```bash
vibecli /k8s apply --file deployment.yaml
vibecli /k8s logs --deployment myapp
vibecli /k8s scale --deployment myapp --replicas 3
```

---

## 18. Daemon & API Mode

### Starting the Daemon

```bash
vibecli --serve --port 7878 --provider claude
vibecli --serve --tailscale          # public HTTPS via Tailscale Funnel
```

### REST Endpoints

| Method | Endpoint | Description |
|---|---|---|
| `POST` | `/api/chat` | Send message, stream response |
| `POST` | `/api/agent` | Spawn agent task |
| `GET` | `/api/providers` | List configured providers |
| `GET` | `/api/sessions` | List recent sessions |
| `GET` | `/api/traces` | View execution traces |
| `GET` | `/sse` | Server-Sent Events stream |

### Diagnostics

```bash
vibecli --diagnostics              # print env, config, DB status
vibecli --diagnostics --resume <id>  # include session context
```

---

## 19. RL-OS

The Reinforcement Learning Operating System provides end-to-end ML training infrastructure:

| Component | Description |
|---|---|
| **TrainOS** | 30+ RL algorithms (PPO, SAC, DQN, A3C, MCTS, etc.) |
| **EvalOS** | SWE-bench, custom benchmarks |
| **OptOS** | Bayesian hyperparameter optimization, NAS |
| **ServeOS** | Production model serving with auto-scaling |
| **ObserveOS** | Trajectory collection, reward shaping |
| **RLHF** | Reward model training from human feedback |

### Usage

```
/aiml train --algorithm ppo --env my-env
/aiml eval --benchmark swe-bench
/aiml serve --model ./checkpoints/best
```

---

## 20. UI Panels (VibeCoder)

### Core Panels

| Panel | Description |
|---|---|
| `AIChat` | Inline AI assistant with tabs, memory, history |
| `EditorPanel` | Monaco code editor |
| `TerminalPanel` | xterm.js terminal emulator |
| `GitPanel` | Diff, commit, branch, history |
| `FileExplorer` | Directory tree with context menu |

### Analysis & Review

| Panel | Description |
|---|---|
| `DiffReviewPanel` | Per-hunk AI diff accept/reject |
| `AiCodeReviewPanel` | 7-detector code analysis |
| `ArchitectureSpecPanel` | TOGAF/C4/ADR management |
| `ReviewProtocolPanel` | Policy-driven review enforcement |

### Development Tools

| Panel | Description |
|---|---|
| `AppBuilderPanel` | Full-stack app generation |
| `DesignCanvasPanel` | Sketch-to-code UI |
| `SemanticIndexPanel` | Codebase semantic search |
| `DatabasePanel` | Database management UI |
| `APIDocsPanel` | Auto-generated API docs |

### Security

| Panel | Description |
|---|---|
| `SecurityScanPanel` | Vulnerability scanning |
| `BlueteamPanel` | Defensive security controls |
| `RedteamPanel` | Offensive security tests |
| `PurpleTeamPanel` | MITRE ATT&CK exercises |
| `PolicyEnginePanel` | RBAC/ABAC policy management |
| `CompliancePanel` | SOC2/FedRAMP reports |

### Agent & Automation

| Panel | Description |
|---|---|
| `AgentPanel` | Agent task runner |
| `AgentTeamPanel` | Multi-agent team management |
| `WorkflowPanel` | Workflow state machine |
| `SchedulerPanel` | Cron/reminder management |

### Observability

| Panel | Description |
|---|---|
| `CostPanel` | Token usage + cost budget |
| `CoveragePanel` | Test coverage visualization |
| `HealthScorePanel` | System health metrics |
| `AnalyticsPanel` | Enterprise usage analytics |
| `SessionMemoryPanel` | Memory profiling (backend) |
| `ChatMemoryPanel` | Chat fact memory (frontend) |

---

## 21. Advanced Runtime Capabilities (FIT-GAP v12)

Modules added in the v12 gap-closure wave, targeting Devin 2.0, Claude Code 1.x, and Cursor 4.x parity.

### Reasoning & Intelligence

| Module | REPL | Description |
|--------|------|-------------|
| `reasoning_provider` | `/reasoning` | Extended chain-of-thought with thinking-token budgets and scratchpad visibility |
| `autodream` | `/autodream` | Autonomous goal decomposition — breaks high-level intents into sub-tasks, consolidates memory |
| `prompt_cache` | `/cache` | FNV-1a prefix cache with TTL management; cache-hit rate reported in `/cache stats` |
| `alt_explore` | `/explore` | Tournament-style agent candidate scoring: pass_rate × diff_penalty × compile_success → winner |

### Desktop Automation

| Module | REPL | Description |
|--------|------|-------------|
| `computer_use` | `/computer-use` | GUI visual self-testing — launch app, screenshot, LLM visual assertions, step recording |

### Session & Lifecycle Management

| Module | REPL | Description |
|--------|------|-------------|
| `long_session` | `/session budget` | 7+ hour autonomous sessions: 2M-token hard limit, compact at 75%, halt at 100% |
| `focus_view` | `/focus` | Deep-focus gating — enter/exit sessions, distraction counting, auto-exit after configurable duration |
| `task_scheduler` | `/tasks` | BinaryHeap priority queue (Low/Normal/High/Critical) with per-task `run_after` timestamps |

### Plugin & Deployment Infrastructure

| Module | REPL | Description |
|--------|------|-------------|
| `plugin_bundle` | `/plugins` | Manifest-driven plugin packaging: dependency resolution, duplicate-ID detection, bundle validation |
| `app_server` | `/appserver` | Embedded HTTP application server for serving generated web apps on a local port |
| `dispatch_remote` | `/dispatch` | Priority job queue with remote worker dispatch, status polling, and retry back-off |
| `sandbox_windows` | `/sandbox` | Windows-style ACL sandbox: deny-over-allow path rules + per-host network policy |

### Usage Examples

```
# Score two agent patches and pick the winner
> /explore rank --pass-rate 0.95 --diff-lines 42 --compiles true

# Start a deep-focus session (auto-exit after 90 min)
> /focus enter --auto-exit 5400

# Check autonomous session budget
> /session budget
  Tokens used: 1,487,233 / 2,000,000  (74%)  → Continue

# Schedule a high-priority task for 09:00
> /tasks add --id deploy-prod --priority high --run-after 1744970400

# Validate a plugin bundle
> /plugins validate ./my-plugin-bundle.json
  ✓ Bundle valid: 3 plugins, 0 missing deps, 0 duplicate IDs
```

---

## 22. Goals — Durable Execution Intent

A persistent, cross-session record of **what the user is working toward**. Forward-looking sibling of Recap (backward-looking) and Resume (cursor + seed). See [design/goal/README.md](./design/goal/README.md) for the full design.

### Data shape

Goals + their link graph live in `~/.vibecli/sessions.db` (same store as sessions and recaps, so JOINs are cheap). Workspace is nullable — `None` = global goal visible from anywhere; `Some(path)` = workspace-bound. A plan-invalidation rule clears `current_plan` when `statement` or `success_criteria` changes; a stale plan against a re-stated goal is worse than no plan.

```rust
pub struct Goal {
    pub id: String,
    pub workspace: Option<PathBuf>,
    pub title: String,
    pub statement: String,
    pub status: GoalStatus,           // Active | Paused | Done | Abandoned
    pub success_criteria: Vec<String>,
    pub tags: Vec<String>,
    pub parent_goal_id: Option<String>,
    pub current_plan: Option<ExecutionPlan>,
    // + timestamps + schema_version
}
```

### HTTP surface (under `authed_routes`)

| Method | Path | Purpose |
|---|---|---|
| `POST`/`GET`/`PATCH`/`DELETE` | `/v1/goals[/:id]` | CRUD |
| `POST` | `/v1/goals/:id/plan` | Generate `ExecutionPlan`; honors per-request `{provider, model}` when key resolves |
| `POST` | `/v1/goals/:id/start` | Spawn session linked to this goal |
| `POST` | `/v1/goals/:id/link` | Attach session / job / recap / note |
| `POST` | `/v1/goals/:id/recap` | Aggregate recap; LLM synthesis when `{provider, model}` supplied, heuristic fallback; `recap_synthesizer` returned |
| `GET` | `/v1/goals/:id/children` | One-level tree query |
| `GET` | `/v1/goals/:id/tree?depth=N` | Recursive subtree walk (clamped 1..10, default 3, cycle-safe) |
| `GET`/`PUT`/`DELETE` | `/v1/goals/current` | Per-workspace "current pin" (empty workspace = global slot) |
| `POST` | `/v1/graph/build` | Kick off a background kodegraph build; returns `{status:"indexing"}` (no LLM call) |
| `GET` | `/v1/graph/status` | Probe `{status,node_count,edge_count,last_built_at?}` |
| `POST` | `/v1/graph/query` | `{query,budget?}` → token-budgeted subgraph `{seeds,nodes,edges,est_tokens}` |
| `GET` | `/v1/graph/node/:name` | One node's payload (404 if not found) |
| `GET` | `/v1/graph/neighbors/:name` | Adjacent nodes |
| `GET` | `/v1/graph/path/:from/:to` | `{path:[labels],hops}` (404 if no path) |
| `POST` | `/v1/graph/blast` | `{name,max_hops?}` → blast radius `{seed,affected,by_hop}` |
| `GET` | `/v1/graph/report` | Full `GRAPH_REPORT.md` text `{report}` |
| `GET` | `/v1/skilllens/skills` | SkillForge catalogue `{skills:[{name,category,summary,source,…}]}` (no LLM) |
| `GET` | `/v1/skilllens/skills/:name` | One skill detail + cached `SkillReport` (no LLM) |
| `POST` | `/v1/skilllens/refresh` | Reload the catalogue from disk (no LLM) |
| `POST` | `/v1/skilllens/convert` | `{runs}` → normalised `Trajectory[]` (no LLM) |
| `POST` | `/v1/skilllens/extract` | `{pool,method,provider,model}` → candidate skills (LLM) |
| `POST` | `/v1/skilllens/score` | `{skill,tasks?,provider,model}` → `{report:{trigger_coverage,extraction_efficacy?,target_evolvability?}}` (LLM) |
| `POST` | `/v1/skillopt/train` | `{skill,env:{kind:"repo"\|"static",tasks?},config,provider,model}` → `{job_id}` (LLM, async) |
| `GET` | `/v1/skillopt/status/:job` | `{id,skill,llm,state:"running"\|"done"\|"failed"\|"cancelled",report?,error?}` |
| `POST` | `/v1/skillopt/cancel/:job` | Best-effort cancel (no LLM) |
| `POST` | `/v1/skillopt/promote` | `{skill,content}` → writes `*.opt.md` (shipped skill untouched; no LLM) |

### Watch surface (curated)

| Method | Path | Notes |
|---|---|---|
| `GET` | `/watch/goals` | Active only, ≤25, slim payload |
| `GET` | `/watch/goals/:id` | Full goal + links |
| `POST` | `/watch/goals/:id/start` | Wrapper for `do_v1_exec_goal_start` |
| `GET` | `/watch/graph/status` | Compact `{status,n,m}` for the wrist form factor |
| `POST` | `/watch/graph/query` | `{query,budget?}` → subgraph capped to ≤5 nodes/edges |
| `GET` | `/watch/skilllens/skills` | SkillForge catalogue compact `{count,top5:[{name,category,summary}]}` |
| `GET` | `/watch/skilllens/skills/:name` | One-line `{name,category,summary}` |

### REPL

```bash
/goal new <title>             # create
/goal list [status]           # filter: active|paused|done|abandoned
/goal show <id>               # full detail
/goal status <id> <status>    # transition lifecycle
/goal link <id> <kind> <tgt>  # attach session/job/recap/note
/goal start <id> [task]       # spawn linked session
/goal children <id>           # direct children only
/goal reparent <id> <parent|none>
/goal pin <id> [--global]     # set as current for workspace
/goal unpin [--global]
/goal current [--global]
/goal delete <id>             # cascade-deletes links
/goal plan <id>               # via `vibecli serve`
```

### Per-client surface

| Client | What it shipped |
|---|---|
| **VibeCLI TUI** | Read-only `Goals` screen — `/goal` from chat opens it; `f` cycles status filter, `j/k` scroll, `r` refresh |
| **VibeCoder** | `GoalPanel` (tab `goals`) — list + detail + status switcher + Generate Plan + Start session + Linked sessions; tree-view toggle; Aggregate recap routed through toolbar `selectedProvider` + `selectedModel` |
| **VibeCoder slash palette + AIChat** | `/goal` opens the panel; `/goal <text>` seeds the New Goal modal |
| **VibeMobile** | `listGoals`, `getGoal`, `startGoal`, `getGoalTree`, `getCurrentGoal`, `pinGoal`, `unpinGoal` |
| **Apple Watch** | `loadGoals`, `fetchGoal`, `startGoal` |
| **Wear OS** | `listGoals`, `getGoal`, `startGoal` + `GoalDetailScreen` + `GoalsTileService` Tile (freshest active goal) |
| **VS Code** | `vibecli.goalsView` sidebar tree-view (`goals-tree.ts`) with refresh + per-row context-menu actions; `listGoals`, `createGoal`, `startGoal` |
| **Agent SDK (TypeScript)** | `agent.goals.{list,get,create,update,delete,plan,start,link,tree,pin,unpin,current,recap}` |
| **`/agent`** | New sessions auto-link to the pinned goal for the daemon's workspace (or the global slot) — silent best-effort, never blocks session creation |

### Why `exec_goal_*`?

VibeCoder already has `CompanyGoalsPanel` (company strategy goals via `company_cmd "goal …"`) and `AgilePanel` (sprint goals). The HTTP path stays friendly (`/v1/goals`) but the Rust module is `exec_goal.rs` and the Tauri commands are `exec_goal_*` so future maintainers reading `commands.rs` see no ambiguity.

---

## 23. Plugin Governance (signed MCPB bundles)

Phase 54 P0 (B2). Installs and runs third-party plugins as **signed MCPB bundles** carrying an inner `vibecli-plugin.toml` manifest. Components — MCP servers, skills, subagents, rules, hooks — register into VibeCody's existing surfaces, gated by a per-workspace policy.

### Bundle layout

An MCPB bundle (`.mcpb`, the A2 open format) carrying VibeCody plugins includes two extra files at its root:

```
my-plugin.mcpb (ZIP)
├── manifest.json            ← MCPB outer (A2)
├── vibecli-plugin.toml      ← B2.1 inner manifest
├── vibecli-plugin.sig       ← B2.2 detached P-256 ECDSA signature
└── …skills/, hooks/, rules/, agents/, server bins…
```

### Inner manifest (`vibecli-plugin.toml`)

```toml
name = "my-plugin"               # kebab-case, lowercase
version = "1.0.0"                # semver-shaped
description = "What it does"     # ≤ 500 B

[publisher]
name = "Publisher Inc"
url  = "https://publisher.example"

[publisher.key]                  # P-256 ECDSA, JWK (RFC 7517/7518)
kty = "EC"
crv = "P-256"
x   = "<base64url>"
y   = "<base64url>"

default_policy = "off"           # off | on | required

[[components.mcp_servers]]
name = "my-srv"
path = "bin/srv"

[[components.skills]]
name = "my-skill"
path = "skills/my.md"
category = "tools"

# subagents, rules, hooks follow the same shape
```

### Detached signature (`vibecli-plugin.sig`)

```json
{
  "kid":             "publisher-default",
  "algorithm":       "ES256",
  "value":           "<base64url(ECDSA(SHA-256(canonical_json(manifest))))>",
  "manifest_digest": "<sha256-hex>"
}
```

Verification anchors trust to the `publisher.key` embedded in the manifest (TOFU — the user explicitly trusts a key the first time they install). No opaque trust chain.

### Per-workspace install policy

| Policy | Runtime behavior | Who can set |
|---|---|---|
| `off`  | Components not enumerated, never run | Anyone |
| `on`   | Components active in this workspace | Anyone |
| `required` | Components active; cannot be lowered to `off` except by admin | Admin only |

Stored unencrypted in `<workspace>/.vibecli/workspace.db` `plugin_policies` table. Unknown plugin (no row) resolves to `off` — the safe default.

### Install layout

```
<workspace>/.vibecli/plugins/<plugin-name>/
  ├── vibecli-plugin.toml
  ├── vibecli-plugin.sig
  └── …component payload (skills, hooks, MCP server bins, …)
```

Atomic: bundle extracts to `.staging.<pid>.<uuid>/`; only renamed into the final `<name>/` slot once signature verification AND policy write both succeed. RAII guard auto-cleans staging on any failure.

### REPL / CLI / panel surface

| Surface | Entry point |
|---|---|
| **REPL** (today) | Existing `/plugin install <url-or-git>` registry path unchanged. Signed-MCPB install lands via VibeCoder panel; REPL parity is a follow-up. |
| **Tauri commands** | `plugin_install_from_file(workspace_path, bundle_path, force)`, `plugin_list_installed(workspace_path)`, `plugin_uninstall(workspace_path, name, is_admin)`, `plugin_get_policy(workspace_path, name)`, `plugin_set_policy(workspace_path, name, policy, is_admin)` — all sensitive-path-gated. |
| **VibeCoder** | `PluginGovernancePanel.tsx` under **Enterprise Governance** → **Plugin Governance**. Install form + per-plugin row with publisher fingerprint + policy buttons + Uninstall. |
| **MCP** | `list_skills` / `get_skill` already return enabled-plugin skills alongside built-ins, tagged with provenance: `{"kind": "builtin"}` or `{"kind": "plugin", "plugin": "<name>"}`. |

### Patent-distance anchors (fit-gap §18)

1. **No telemetry-driven personalization.** No "for-you" surface, no usage analytics. Panel shows installed plugins for THIS workspace and nothing else.
2. **Policy enforcement is client-side and admin-authored.** No remote endpoint can flip a workspace plugin from Off to Required.
3. **Bundle format is open MCPB.** Lineage to `.vsix` + MetaPK keeps prior art clear; no proprietary wrapping.
4. **Trust roots are per-publisher P-256 ECDSA keys.** Same key infra as A2A signed agent cards (B6) and watch pairing.

### Module map

| Module | Role |
|---|---|
| `plugin_manifest.rs` (B2.1) | `vibecli-plugin.toml` schema + validator |
| `plugin_signing.rs` (B2.2) | Detached P-256 ECDSA sign / verify |
| `workspace_store.rs::plugin_*` (B2.3) | `plugin_policies` table + Required-pin guard |
| `plugin_install.rs` (B2.4) | Atomic install / list / uninstall |
| `plugin_runtime.rs` (B2.5) | Policy-filtered component enumeration |
| `PluginGovernancePanel.tsx` + 5 Tauri commands (B2.6) | VibeCoder surface |
| `skill_catalog.rs::load_from_with_plugins` (B2.7) | First per-loader activation; `mcp_server.rs` consumes it |
| `mcp_governance::register_plugin_servers` (B2.8) | MCP-server components registered as `plugin:<plugin>:<component>` |
| `plugin_install::install_from_url` (B2.12) | HTTPS install (`vibecli plugin install <https://…>`), 60 s timeout, 50 MB cap |
| `plugin_runtime::merge_with_plugin_hooks` (B2.9) | Plugin hooks fire on CLI agent path (orchestrator + REPL) |
| `serve.rs::start_agent` etc. (B2.9.daemon) | Plugin hooks fire on `/v1/agent` + ACP + timed-task paths |
| `context_assembler::collect_plugin_rules` (B2.10) | Plugin rules land in chat + agent system context as `plugin_rules` section |

### Remaining wiring (not blocking ship)

Plugin **subagents** (B2.11) — no built-in file-based subagent loader to plug into. `sub_agents.rs` uses a hardcoded `SubAgentRole` enum; wiring plugin-defined subagents requires designing a registry / dispatch layer first.

---

*For the quick matrix view see [FEATURE-MATRIX.md](FEATURE-MATRIX.md). For competitive analysis see [FIT-GAP-ANALYSIS.md](FIT-GAP-ANALYSIS.md).*
