# VibeCody Feature Reference

> Deep-dive documentation for every major feature area. For the quick matrix view see [FEATURE-MATRIX.md](FEATURE-MATRIX.md).

---

## Table of Contents

1. [AI Providers](#1-ai-providers)
2. [Chat & Conversation](#2-chat--conversation)
3. [Agent System](#3-agent-system)
4. [Code Editing (VibeUI)](#4-code-editing-vibeui)
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
20. [UI Panels (VibeUI)](#20-ui-panels-vibeui)

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

### Per-Tab Override (VibeUI)
Each chat tab in VibeUI has its own provider selector. Changing it marks the tab as "manually overridden" (shown in gold). Resetting follows the top-bar selection.

### Resilient Provider
All providers are wrapped with `ResilientProvider` which adds:
- Exponential backoff on transient errors
- Rate-limit detection and waiting
- Health score tracking (exposed in VibeUI status bar)

---

## 2. Chat & Conversation

### Streaming
All chat uses token-by-token Server-Sent Events from the Tauri backend. Tokens/sec is displayed live. Stop button cancels in-flight requests via `stop_chat_stream`.

### Auto-Compaction
When conversation history exceeds **80,000 characters**, VibeCLI/VibeUI:
1. Waits for the current response to complete
2. Calls `summarise_messages` Tauri command on messages before the last 20
3. Replaces them with a single summary message (marked with a "Conversation compacted" divider)
4. Requires 10,000+ new characters before triggering again

Threshold is defined as `COMPACTION_THRESHOLD = 80_000` in `AIChat.tsx`.

### Chat Memory Panel (VibeUI)
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

### Slash Commands (VibeUI)
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

## 4. Code Editing (VibeUI)

### Monaco Integration
VibeUI uses Monaco Editor with `automaticLayout: true`. File writes from AI are split across two `requestAnimationFrame` callbacks to avoid ResizeObserver / React state race conditions:
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

**LongMemEval benchmark** — built-in recall@K evaluation across 20 probe cases spanning all 5 sectors. Available as `/openmemory benchmark [k]` (REPL), `GET /memory/benchmark?k=5` (HTTP daemon), and in the VibeUI **Drawers** tab.

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

### `.vibeui.md` (VibeUI-specific)
Place a `.vibeui.md` file in the workspace root. It is injected as `## Project AI Rules` into every AI system prompt, guiding AI-generated code to follow project conventions (e.g., "use the custom Icon system, not lucide-react").

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
Coverage data is collected per-run and displayed in the VibeUI CoveragePanel as:
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
Provider-specific pricing tables map (input_tokens, output_tokens) → USD cost. Displayed in the `/cost` command and CostPanel in VibeUI.

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
All LSP features are surfaced in Monaco (VibeUI) and as structured output in VibeCLI review mode: go-to-definition, find-references, hover, rename, code actions, diagnostics, call hierarchy, workspace symbols.

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
VibeUI uses CRDTs (Conflict-free Replicated Data Types) for real-time collaborative editing. Multiple users can edit the same file simultaneously with automatic conflict resolution. Presence indicators show remote cursors and selections.

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

## 20. UI Panels (VibeUI)

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

*For the quick matrix view see [FEATURE-MATRIX.md](FEATURE-MATRIX.md). For competitive analysis see [FIT-GAP-ANALYSIS-v7.md](FIT-GAP-ANALYSIS-v7.md).*
