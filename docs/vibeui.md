---
layout: page
title: VibeUI Reference
permalink: /vibeui/
---

# VibeUI

**AI-powered desktop code editor built with Tauri 2 and Monaco.** VibeUI provides a VS Code-like editing experience with a native Rust backend, Monaco Editor frontend, integrated AI chat, autonomous agent mode, inline completions, terminal, Git panel, code review, and a WASM extension system.

### What's new in 0.5.5

- **Watch Devices panel** — `Governance → Watch Devices` lets you approve, rename, or revoke paired Apple Watches and Wear OS devices.
- **Handoff banner** — surfaces live sessions from paired phones and watches. Click to pull the stream onto the desktop.
- **Sandbox auto-focus** — when a paired watch opens a sandbox session, VibeUI auto-switches to the Sandbox tab so you can observe the container output.
- **Pairing panel refresh** — URL-only and URL + Bearer flows added alongside the existing QR code; emulator-friendly.
- **293 panels + 42 composites** registered in the panel host (up from 187 in 0.5.4).
- **Google-Docs-style sync** for chat and agent sessions — ID-based reconciliation with no 80/512-char truncation.

## Architecture Overview

```text
┌──────────────────────────────────────────────────────────────────┐
│                    Frontend (React + TypeScript)                 │
│  Monaco Editor │ AI Chat │ Agent Panel  │ Manager View           │
│  Git Panel │ Terminal │ Command Palette │ Review Panel           │
│  Checkpoints │ Artifacts │ Hooks Config │ History                │
│  Context Picker │ Memory/Rules │ Theme Toggle                    │
└──────────────────────────┬───────────────────────────────────────┘
                           │ Tauri IPC (invoke / events)
┌──────────────────────────▼───────────────────────────────────────┐
│                   Tauri Rust Backend                             │
│  commands.rs    — 1,045+ Tauri commands (files, git, AI, agent …)│
│  agent_executor — ToolExecutorTrait for agent tool calls         │
│  flow.rs        — Flow Awareness Engine (activity tracking)      │
│  memory.rs      — Workspace + global AI rules (.vibeui.md)       │
└──────────────────────────┬───────────────────────────────────────┘
                           │
         ┌─────────────────┴──────────────────────┐
         │          Rust Library Crates           │
         ├─────────────┬──────────────────────────┤
         │ vibe-core   │ vibe-ai                  │
         │ vibe-lsp    │ vibe-extensions          │
         │ vibe-collab │                          │
         └─────────────┴──────────────────────────┘
```

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

## Features

### Editor

- **Monaco Editor** — same engine as VS Code; full syntax highlighting, IntelliSense, multi-cursor
- **Rope-based buffer** — built on `ropey` for efficient O(log n) edits on large files
- **Batch edits** — `apply_batch_edits` for bulk insert/delete operations
- **Multi-cursor** — `update_cursors` for synchronised cursor state
- **Inline AI completions** — ghost-text suggestions via `request_inline_completion` (FIM for Ollama, chat prompt for cloud providers)
- **Next-Edit Prediction** — `predict_next_edit` analyses recent edits to suggest the next likely change; wired as inline completions provider with 500ms debounce + Tab acceptance
- **Inline Chat (Cmd+K)** — floating `InlineChat` overlay; select code, describe change, view streamed result, Accept to apply edit in-place
- **File watching** — auto-detects external changes using `notify`
- **Multi-workspace** — open multiple folders simultaneously
- **Language detection** — automatic language mode from file extension

### AI Integration

The AI chat panel supports all 22 providers via the shared `vibe-ai` crate:

- **Local**: Ollama (default, no API key), LocalEdit
- **Cloud**: Claude, OpenAI, Gemini, Grok, Groq, Mistral, Cerebras, DeepSeek, Zhipu, MiniMax
- **Platform**: OpenRouter, Azure OpenAI, Bedrock, Copilot, Vercel AI
- **Inference**: Perplexity, Together AI, Fireworks AI, SambaNova
- **Meta**: Failover (automatic provider fallback chain)

Select the provider from the dropdown in the header, or switch per chat tab in `ChatTabManager`. Provider configuration is handled through the **Keys**settings tab (BYOK), environment variables, or `~/.vibecli/config.toml`.

#### Provider Advanced Options

| Option | Config key | Description |
|--------|-----------|-------------|
| `api_key_helper` | `[claude] api_key_helper = "~/.vibecli/get-key.sh"` | Shell script that emits a fresh API key on stdout; falls back to static `api_key` |
| `thinking_budget_tokens` | `[claude] thinking_budget_tokens = 10000` | Enable Claude extended thinking mode with N token budget |

#### @ Context System

Type `@` in the chat input to open the **Context Picker** — a dropdown that lets you inject additional context:

| Reference | Description |
|-----------|-------------|
| `@git` | Current branch, changed files, and diff excerpt |
| `@file:<path>` | Contents of a specific file |
| `@file:<path>:N-M` | Specific line range from a file |
| `@folder:<path>` | Recursive directory tree listing |
| `@web:<url>` | Fetched & stripped plain text from a URL (6 000 char limit) |
| `@terminal` | Last 200 lines of terminal output (ANSI-stripped) |
| `@symbol:<name>` | Symbol search via `CodebaseIndex`, returns source snippet |
| `@codebase:<query>` | Semantic codebase search via `CodebaseIndex` |
| `@github:owner/repo#N` | Fetch GitHub issue/PR title, state, author, labels, body |
| `@jira:PROJECT-123` | Fetch Jira issue summary, status, assignee, description |
| `@html-selected` | Inject the last HTML element selected via Browser panel inspect mode |

The backend resolves references via `resolve_at_references()` in `commands.rs` and injects them into the system prompt.

**Jira**requires `JIRA_BASE_URL`, `JIRA_EMAIL`, and `JIRA_API_TOKEN` environment variables. **GitHub**uses optional `GITHUB_TOKEN` for higher rate limits.

#### Smart Context Builder

The `ContextBuilder` in `vibe-core` builds a ranked, token-budget-aware context by combining:

1. **Git branch + changed file list** — always included
2. **Git diff** — up to 25% of budget
3. **Top-ranked symbols** (via codebase index) — up to 30% of budget
4. **Open file contents** — remaining budget

#### Flow Awareness

The **Flow Awareness Engine** (`flow.rs`) tracks developer activity (file opens, edits, saves, terminal commands) in a 100-event ring buffer. Recent activity is injected into AI context via `get_flow_context()`.

### Agent Mode

The **Agent Panel**provides autonomous multi-step task execution:

- **Plan mode** — agent generates a plan before executing
- **Approval tiers** — `auto`, `suggest`, `always-ask` policies
- **Tool execution** — `TauriToolExecutor` supports: `read_file`, `write_file`, `bash`, `search_files`, `list_directory`, `web_search`, `fetch_url`, `task_complete`
- **Step timeline** — each tool call emits `agent:step` events rendered in the UI
- **Pending approval** — destructive operations emit `agent:pending` for user review
- **Streaming** — LLM output streamed via `agent:chunk` Tauri events
- **Diff review** — pending writes shown in a Monaco DiffEditor with Accept/Reject

### Multi-Agent Orchestration (Manager View)

The **Manager View**enables running multiple agents in parallel:

- **Task board** — submit multiple tasks simultaneously
- **Git worktrees** — each agent operates on an isolated worktree branch
- **Parallel execution** — `start_parallel_agents` spawns concurrent agent loops
- **Status tracking** — `manager:agent_update` and `manager:agent_step` events
- **Branch merging** — `merge_agent_branch` with merge/squash/rebase strategies
- **Status polling** — `get_orchestrator_status` for initial render

### Code Review (Review Panel)

AI-powered code review via the **Review Panel**:

- Compares `base_ref` vs `target_ref` (defaults to HEAD vs working tree)
- Returns structured `ReviewReport` with:
  - **Issues** — file, line, severity (hint/warning/error/critical), category (security/performance/correctness/style/testing), description, suggested fix
  - **Suggestions** — general improvement recommendations
  - **Scores** — overall, correctness, security, performance, style (0–100)
- Color-coded severity badges and score bars in the UI

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
| Git stash (create / pop) | Done |
| Commit history | Done |

Modified files appear **yellow** (M), new files **green** (N), deleted files **red** (D).

### Checkpoints

The **Checkpoint Panel**provides a timeline of AI checkpoints backed by Git stashes:

- **Create checkpoint** — `create_checkpoint(label)` saves current workspace state
- **List checkpoints** — browse all stash entries with labels and timestamps
- **Restore checkpoint** — apply a stash without dropping it
- **Auto-checkpoint** — agent mode can auto-create checkpoints before destructive operations

### Artifacts

The **Artifacts Panel**displays structured AI output:

- Rich artifact cards with metadata
- Annotations and inline comments
- Async feedback mechanism

### Hooks Configuration

The **Hooks Panel**provides a UI for configuring event-driven hooks:

| Hook Event | Description |
|------------|-------------|
| `PreToolUse` | Fires before an agent tool call |
| `PostToolUse` | Fires after an agent tool call |
| `SessionStart` | Fires when an agent session begins |
| `TaskCompleted` | Fires when an agent task finishes |
| `Stop` | Fires when an agent stops |

Each hook can be configured with:

- **Handler type** — `command` (runs a shell command) or `llm` (sends a prompt)
- **Tool filters** — restrict to specific tools
- **Async exec** — run hook in background

Configuration is saved to `.vibeui/hooks.json` (workspace or global).

### Memory & Rules

The **Memory Panel**provides editors for AI rules:

- **Project rules** — `.vibeui.md` in workspace root (committed alongside code)
- **Global rules** — `~/.vibeui/rules.md` (personal defaults)
- Both are injected into every AI system prompt via `combined_rules()`

### Agent History

The **History Panel**displays an audit log of past agent sessions:

- **Session list** — `list_trace_sessions()` enumerates JSONL trace files
- **Session detail** — `load_trace_session()` returns all entries (tool calls, LLM turns, approvals)
- Expand and browse individual trace entries

### Terminal

An integrated terminal panel using `portable-pty`:

- Full PTY — supports interactive programs (vim, htop, etc.)
- Powered by xterm.js on the frontend
- Multiple terminal instances (`spawn_terminal` returns terminal IDs)
- Terminal resize support (`resize_terminal`)
- Accessible via the status bar "Show Terminal" button

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

## AI Panel Tabs

The AI panel (toggle with **AI Chat** in the header) has **293 panel components + 42 composites** across categories:

### Core AI & Chat

| Tab | Component | Description |
|-----|-----------|-------------|
| **Chat** | `ChatTabManager` | Multiple independent chat tabs, each with per-tab provider selection; voice input |
| **Agent** | `AgentPanel` | Autonomous multi-step agent with step timeline, approval UI, Turbo mode, and plan mode |
| **Agent Modes** | `AgentModesPanel` | Switch between Smart, Rush, and Deep agent modes |
| **Memory** | `MemoryPanel` | Edit per-workspace `.vibeui.md` and global `~/.vibeui/rules.md` |
| **History** | `HistoryPanel` | Audit log of past agent sessions; browse and expand trace entries |
| **Checkpoints** | `CheckpointPanel` | Timeline of AI checkpoints with restore, auto-checkpoint |
| **Artifacts** | `ArtifactsPanel` | Structured output cards with annotations and async feedback |
| **Manager** | `ManagerView` | Multi-agent orchestration: task board, worktrees, parallel execution |
| **Hooks** | `HooksPanel` | Configure event-driven hooks (PreToolUse, PostToolUse, etc.) |
| **Jobs** | `BackgroundJobsPanel` | Submit tasks to VibeCLI daemon; live SSE stream; job persistence across restarts |
| **Keys** | `SettingsPanel` | BYOK API key management for all cloud providers |
| **Counsel** | `CounselPanel` | Multi-LLM deliberation: structured debates between multiple AI providers with role-based personas, voting, and synthesis |
| **Compare** | `MultiModelPanel` | Side-by-side dual-provider comparison: provider/model selectors, timing/tokens, Ctrl+Enter |
| **Model Wizard** | `ModelWizardPanel` | Guided model selection wizard for optimal provider/model choice |
| **Discussion** | `DiscussionModePanel` | Discussion and planning mode for exploring ideas before coding |
| **Clarifying** | `ClarifyingQuestionsPanel` | AI-driven clarifying questions for ambiguous requests |

### Development & Code

| Tab | Component | Description |
|-----|-----------|-------------|
| **Specs** | `SpecPanel` | Spec-driven development: AI-generated user stories, tasks, and acceptance criteria |
| **Spec Pipeline** | `SpecPipelinePanel` | End-to-end spec-to-code pipeline with validation |
| **Workflow** | `WorkflowPanel` | Code Complete 8-stage development pipeline with AI-generated checklists per stage |
| **Orchestrate** | `OrchestrationPanel` | Workflow orchestration with lessons and task tracking |
| **Autofix** | `AutofixPanel` | Codemod auto-fix: detect linter (clippy/eslint/ruff/gofmt/prettier), run fix, diff preview, apply/revert |
| **AST Edit** | `AstEditPanel` | AST-based code editing and structural transformations |
| **Transform** | `TransformPanel` | Code transformation and refactoring tools |
| **Edit Predict** | `EditPredictionPanel` | RL Q-learning next-edit prediction with confidence scoring |
| **Full-Stack Gen** | `FullStackGenPanel` | Full-stack application generation from prompts |
| **App Builder** | `AppBuilderPanel` | App scaffolding: quick start, templates, provisioning, managed backend |
| **Scaffold** | `ScaffoldPanel` | Project scaffolding with framework templates |
| **Build** | `BuildPanel` | Build system management and monitoring |
| **Cascade** | `CascadePanel` | Cascade AI pipeline with multi-step reasoning |
| **Self Review** | `SelfReviewPanel` | AI self-review of generated code |
| **Automations** | `AutomationsPanel` | Automation rules and triggers |

### Testing & Quality

| Tab | Component | Description |
|-----|-----------|-------------|
| **Tests** | `TestPanel` | Test runner with framework detection, live log stream, filter tabs, pass/fail badges |
| **Coverage** | `CoveragePanel` | Code coverage: auto-detect tool, run coverage, per-file % bars, uncovered lines, filter tabs, raw output |
| **QA Validation** | `QaValidationPanel` | Multi-round QA validation pipeline with severity-weighted scoring |
| **Batch Builder** | `BatchBuilderPanel` | Batch code generation: 10 agent roles, 3M+ line target, pause/resume |
| **SWE-bench** | `SweBenchPanel` | SWE-bench benchmarking harness: run, compare, export |
| **Arena** | `ArenaPanel` | Blind A/B model comparison: hidden identities, vote (A/B/Tie/Both bad), reveal, persistent leaderboard |
| **BugBot** | `BugBotPanel` | AI code scanner with severity/category filter and fix snippets |
| **Metrics** | `CodeMetricsPanel` | Code metrics: complexity, LOC, duplication analysis |
| **Visual** | `VisualTestPanel` | Visual regression testing with screenshot comparison |
| **Render Opt** | `RenderOptimizePanel` | React re-render optimization analysis (~74% reduction) |

### Security

| Tab | Component | Description |
|-----|-----------|-------------|
| **RedTeam** | `RedTeamPanel` | Autonomous 5-stage pentest pipeline with findings feed, CVSS scores, and report export |
| **BlueTeam** | `BlueTeamPanel` | Defensive security: incident management, IOC tracking, SIEM integration, forensics, playbooks |
| **PurpleTeam** | `PurpleTeamPanel` | ATT&CK exercises: attack simulation, detection validation, coverage gap analysis |
| **Security Scan** | `SecurityScanPanel` | Security scanning with vulnerability detection |
| **Compliance** | `CompliancePanel` | Compliance reporting and policy enforcement |

### Agent Teams & Orchestration

| Tab | Component | Description |
|-----|-----------|-------------|
| **Teams** | `AgentTeamPanel` | Multi-agent team orchestration with inter-agent messaging |
| **Agent Teams** | `AgentTeamsPanel` | Agent team hierarchy with role-based composition |
| **Sub-Agent** | `SubAgentPanel` | Sub-agent role management and delegation |
| **Team Govern** | `TeamGovernancePanel` | Team plugin marketplace governance and policies |
| **Watch Devices** | `WatchDevicesPanel` | Approve / rename / revoke paired Apple Watch and Wear OS clients (new in 0.5.5) |
| **Paired Mobiles** | `PairedMobilesPanel` | Manage paired VibeMobile devices; view JWT expiry, revoke (new in 0.5.5) |
| **Branch Agent** | `BranchAgentPanel` | Branch-per-task agent execution with auto-merge |
| **Cloud Autofix** | `CloudAutofixPanel` | Cloud-hosted BugBot autofix agents |
| **GH Actions** | `GhActionsPanel` | GitHub Actions agent for CI/CD automation |
| **CI Bot** | `CIReviewPanel` | CI review bot for automated PR analysis |
| **Soul** | `SoulPanel` | AI personality and behavior configuration |

### Context & Memory

| Tab | Component | Description |
|-----|-----------|-------------|
| **Steering** | `SteeringPanel` | Workspace/global steering files with templates |
| **Open Memory** | `OpenMemoryPanel` | Cognitive memory engine: 5 sectors, associative graph, HNSW index, AES-256-GCM encryption |
| **Context Bundle** | `ContextBundlePanel` | Context bundles/spaces with priority ordering and TOML serialization |
| **Fast Context** | `FastContextPanel` | Fast context retrieval and SWE-grep |
| **Infinite Context** | `InfiniteContextPanel` | 5-level context hierarchy with token budget, eviction, and compression |
| **Session Memory** | `SessionMemoryPanel` | Long-session memory profiling with leak detection |
| **Session Browser** | `SessionBrowserPanel` | Browse and resume past sessions |
| **Session Sharing** | `SessionSharingPanel` | Share sessions across users and devices |
| **Project Context** | `ProjectContextPanel` | Project-level context and configuration |
| **Org Context** | `OrgContextPanel` | Organization-wide context and policies |
| **Plan Document** | `PlanDocumentPanel` | Plan documentation and tracking |

### Infrastructure & DevOps

| Tab | Component | Description |
|-----|-----------|-------------|
| **Docker** | `DockerPanel` | Docker container/image management with build, run, logs |
| **K8s** | `K8sPanel` | Kubernetes cluster management: pods, services, deployments, scale, logs |
| **CI/CD** | `CicdPanel` | CI/CD pipeline configuration and status monitoring |
| **CI Gates** | `CiGatesPanel` | CI quality gates with automated pass/fail criteria |
| **CI Status** | `CiStatusPanel` | CI pipeline status monitoring |
| **Sandbox** | `SandboxPanel` | Container sandbox for safe code execution (Docker/Podman) |
| **Cloud Sandbox** | `CloudSandboxPanel` | Cloud-hosted sandbox execution environments |
| **Cloud** | `CloudAgentPanel` | Cloud-hosted agent execution with Docker backends |
| **Cloud Provider** | `CloudProviderPanel` | AWS/GCP/Azure integration: service detection, IAM, Terraform/CloudFormation templates |
| **VM Orchestrator** | `VmOrchestratorPanel` | Virtual machine orchestration and management |
| **Channel Daemon** | `ChannelDaemonPanel` | Background channel daemon service management |
| **Deploy** | `DeployPanel` | Deploy to 6 targets (Vercel/Netlify/Railway/GH Pages/GCP/Firebase) with custom domain support |
| **Env** | `EnvPanel` | Environment variable manager with .env file support |
| **SSH** | `SshPanel` | SSH connection manager with terminal sessions |
| **Profiler** | `ProfilerPanel` | Performance profiling with flame graph visualization |
| **Process** | `ProcessPanel` | System process monitor with kill capability |
| **Health** | `HealthMonitorPanel` | Service health monitor with uptime tracking |
| **Resilience** | `ResiliencePanel` | Resilience testing and chaos engineering |
| **IDP** | `IdpPanel` | Internal Developer Platform: Backstage, Cycloid, Humanitec, Port, scorecards |

### Data, AI & ML

| Tab | Component | Description |
|-----|-----------|-------------|
| **Auto Research** | `AutoResearchPanel` | Autonomous iterative research: 5 strategies, 7 domains, cross-run learning |
| **Training** | `TrainingPanel` | Distributed ML training: DeepSpeed, FSDP, LoRA configuration |
| **Inference** | `InferencePanel` | ML inference server management: vLLM, TGI, Triton, llama.cpp |
| **FineTune** | `FineTuningPanel` | Model fine-tuning configuration and monitoring |
| **Vector DB** | `VectorDbPanel` | Vector database management: Qdrant, Pinecone, pgvector |
| **Doc Ingest** | `DocumentIngestPanel` | Multi-format document ingestion for RAG pipelines |
| **Web Crawler** | `WebCrawlerPanel` | Web crawler with robots.txt, sitemaps, rate limiting |
| **Data Analysis** | `DataAnalysisPanel` | Data analysis and visualization |
| **Image Gen** | `ImageGenPanel` | AI image generation |
| **AI/ML Workflow** | `AiMlWorkflowPanel` | AI/ML workflow pipeline management |
| **Quantum** | `QuantumComputingPanel` | Quantum computing simulation and algorithms |
| **Knowledge** | `KnowledgeGraphPanel` | Knowledge graph visualization |
| **Conversational Search** | `ConversationalSearchPanel` | Natural language code search with conversation context |

### Git & Collaboration

| Tab | Component | Description |
|-----|-----------|-------------|
| **Git** | `GitPanel` | Full Git workflow panel; PR review; AI commit message button |
| **GH Sync** | `GitHubSyncPanel` | GitHub sync with ahead/behind status, push/pull, repo management |
| **Collab** | `CollabPanel` | CRDT multiplayer collaboration: create/join rooms, peer list with color indicators, copy invite link |
| **Review** | `ReviewPanel` | AI code review with issues, scores, and suggestions |
| **Diff Review** | `DiffReviewPanel` | Diff review and commentary |
| **Migrations** | `MigrationsPanel` | Database migration manager with up/down tracking |
| **Agile** | `AgilePanel` | Agile project management with sprints and stories |
| **Work Mgmt** | `WorkManagementPanel` | Work item tracking and management |

### MCP & Extensions

| Tab | Component | Description |
|-----|-----------|-------------|
| **MCP** | `McpPanel` | MCP server management with OAuth 2.0 install flow and tool testing |
| **MCP Lazy** | `McpLazyPanel` | MCP lazy loading with tool search and LRU eviction |
| **MCP Directory** | `McpDirectoryPanel` | MCP verified plugin directory: search, install, review |
| **ACP** | `AcpPanel` | Agent Client Protocol: server/client modes, capability negotiation |
| **Marketplace** | `MarketplacePanel` | Extension marketplace with install/update/ratings |
| **Remote Control** | `RemoteControlPanel` | Remote control and session management |
| **Usage Metering** | `UsageMeteringPanel` | Credit system: per-user/project/team budgets, alerts, chargeback |

### Developer Toolbox

| Tab | Component | Description |
|-----|-----------|-------------|
| **HTTP** | `HttpPlayground` | HTTP request builder: method/URL/headers/body, quick-launch localhost, route discovery, response viewer with JSON pretty-print |
| **Cost** | `CostPanel` | AI cost observatory: per-provider breakdown, total spend, budget limit, cost history table, clear history |
| **Deps** | `DepsPanel` | Dependency viewer with outdated detection, update, add/remove |
| **Scripts** | `ScriptPanel` | Script runner with task detection (npm, make, cargo) |
| **Notebook** | `NotebookPanel` | Jupyter-style notebook with code cells and outputs |
| **Bookmarks** | `BookmarkPanel` | Code bookmark manager with annotations |
| **Bisect** | `BisectPanel` | Git bisect automation to find regression commits |
| **Snippets** | `SnippetPanel` | Code snippet library with search and insertion |
| **Mock** | `MockServerPanel` | Mock HTTP server for API development and testing |
| **GraphQL** | `GraphQLPanel` | GraphQL playground with schema introspection |
| **Load Test** | `LoadTestPanel` | Load testing with configurable concurrency and latency charts |
| **API Docs** | `ApiDocsPanel` | OpenAPI/Swagger documentation viewer |
| **Utilities** | `UtilitiesPanel` | Developer utilities: hash, UUID, lorem ipsum, timestamp |
| **Database** | `DatabasePanel` | SQLite/PostgreSQL browser with AI query generation |
| **VibeSql** | `VibeSqlPanel` | SQL editor with syntax highlighting and results |
| **Streaming** | `StreamingPanel` | Streaming data viewer (Kafka, NATS, Pulsar) |
| **Webhooks** | `WebhookPanel` | Webhook endpoint manager with request logging |
| **WebSocket** | `WebSocketPanel` | WebSocket client for testing real-time connections |
| **Network** | `NetworkPanel` | Network tools: port scan, DNS lookup, TLS inspection |

### UI & Design

| Tab | Component | Description |
|-----|-----------|-------------|
| **Canvas** | `CanvasPanel` | Freeform drawing canvas for diagrams and wireframes |
| **Design Canvas** | `DesignCanvasPanel` | Design canvas for UI prototyping |
| **Design Import** | `DesignImportPanel` | Import from Figma and other design tools |
| **Img2App** | `ScreenshotToApp` | Screenshot-to-code: upload image, generate UI code |
| **Colors** | `ColorPalettePanel` | Color palette generator with design token export |
| **ColorConv** | `ColorConverterPanel` | Color format converter (HEX/RGB/HSL/CMYK) |
| **Diff** | `DiffToolPanel` | Visual diff comparison tool |
| **Markdown** | `MarkdownPanel` | Markdown editor with live preview |

### Converters & Utilities

| Tab | Component | Description |
|-----|-----------|-------------|
| **Cron** | `CronPanel` | Cron expression builder with human-readable descriptions |
| **Regex** | `RegexPanel` | Regex tester with match highlighting and group extraction |
| **JWT** | `JwtPanel` | JWT decoder/encoder with claims inspection |
| **Encoding** | `EncodingPanel` | Base64/URL/HTML encoding and decoding |
| **NumBase** | `NumberBasePanel` | Number base converter (binary, octal, decimal, hex) |
| **DataGen** | `DataGenPanel` | Test data generator with customizable schemas |
| **Timestamp** | `TimestampPanel` | Unix timestamp converter with timezone support |
| **JSON** | `JsonToolsPanel` | JSON formatter, validator, and path query tool |
| **CIDR** | `CidrPanel` | CIDR/subnet calculator for network planning |
| **Unicode** | `UnicodePanel` | Unicode character lookup and conversion |
| **Units** | `UnitConverterPanel` | Unit converter (length, weight, temperature, etc.) |
| **CSV** | `CsvPanel` | CSV viewer and editor with filtering |

### Monitoring & Admin

| Tab | Component | Description |
|-----|-----------|-------------|
| **Dashboard** | `DashboardPanel` | Project overview dashboard with stats and activity |
| **Admin** | `AdminPanel` | Admin panel with RBAC, audit log, user management |
| **Log** | `LogPanel` | Structured log viewer with filtering and search |
| **Traces** | `TraceDashboard` | OpenTelemetry trace viewer with span timeline |
| **Recording** | `AgentRecordingPanel` | Record and replay agent sessions |
| **GPU** | `GpuTerminalPanel` | GPU-accelerated terminal with performance monitoring |
| **Audio Output** | `AudioOutputPanel` | Text-to-speech audio output |
| **Debug Mode** | `DebugModePanel` | AI debug mode with step-through analysis |
| **Demo** | `DemoPanel` | Demo and showcase mode |

### Agents

| Tab | Component | Description |
|-----|-----------|-------------|
| **Browser** | `BrowserPanel` | Embedded iframe browser with inspect mode, element selection |
| **Browser Agent** | `BrowserAgentPanel` | Browser automation agent with CDP |
| **Desktop Agent** | `DesktopAgentPanel` | Desktop automation agent |
| **Observe-Act** | `ObserveActPanel` | Observe-act agent loop visualization |

### Integration & Migration

| Tab | Component | Description |
|-----|-----------|-------------|
| **Supabase** | `SupabasePanel` | Supabase connection, table browser, SQL editor, AI query |
| **Auth** | `AuthPanel` | Auth scaffold generator (4 providers × 5 frameworks) |
| **Legacy Migrate** | `MigrationsPanel` | Legacy migration engine: 18 source languages, 10 targets, 6 strategies |

## UI Components

| Component | File | Description |
|-----------|------|-------------|
| `App` | `src/App.tsx` | Root component, global state, layout |
| `AIChat` | `src/components/AIChat.tsx` | Streaming AI chat panel; multimodal input |
| `AgentPanel` | `src/components/AgentPanel.tsx` | Autonomous agent UI; plan mode toggle |
| `ManagerView` | `src/components/ManagerView.tsx` | Multi-agent parallel orchestration UI |
| `CheckpointPanel` | `src/components/CheckpointPanel.tsx` | Checkpoint timeline; restore, auto-checkpoint |
| `ArtifactsPanel` | `src/components/ArtifactsPanel.tsx` | Rich artifact cards; annotations, feedback |
| `MemoryPanel` | `src/components/MemoryPanel.tsx` | Rules / memory editor |
| `HistoryPanel` | `src/components/HistoryPanel.tsx` | Agent session trace viewer |
| `HooksPanel` | `src/components/HooksPanel.tsx` | Hooks configuration UI; event/handler/filter editor |
| `ReviewPanel` | `src/components/ReviewPanel.tsx` | AI code review; issues, scores, suggestions |
| `ChatTabManager` | `src/components/ChatTabManager.tsx` | Multi-tab chat manager with per-tab provider selection |
| `InlineChat` | `src/components/InlineChat.tsx` | Cmd+K floating edit overlay with Accept/Cancel |
| `BackgroundJobsPanel` | `src/components/BackgroundJobsPanel.tsx` | VibeCLI daemon job queue with live SSE stream |
| `BrowserPanel` | `src/components/BrowserPanel.tsx` | Embedded iframe browser with inspect mode (), element selection, Send to Chat |
| `ArenaPanel` | `src/components/ArenaPanel.tsx` | Blind A/B model comparison with voting, leaderboard, and Send winner to Chat |
| `CostPanel` | `src/components/CostPanel.tsx` | AI cost observatory with per-provider breakdown, budget limit, and history |
| `AutofixPanel` | `src/components/AutofixPanel.tsx` | Codemod auto-fix with linter detection, diff preview, and apply/revert |
| `SettingsPanel` | `src/components/SettingsPanel.tsx` | BYOK API key management for all cloud providers |
| `SpecPanel` | `src/components/SpecPanel.tsx` | Spec-driven development with AI task generation |
| `WorkflowPanel` | `src/components/WorkflowPanel.tsx` | Code Complete 8-stage workflow with pipeline visualization and checklists |
| `RedTeamPanel` | `src/components/RedTeamPanel.tsx` | Autonomous pentest pipeline with 5-stage visualization and CVSS findings |
| `McpPanel` | `src/components/McpPanel.tsx` | MCP server management with OAuth 2.0, tool testing, add/edit/delete |
| `DeployPanel` | `src/components/DeployPanel.tsx` | Deploy to 6 targets with custom domain support and history |
| `BugBotPanel` | `src/components/BugBotPanel.tsx` | AI code scanner with severity badges and fix snippets |
| `SupabasePanel` | `src/components/SupabasePanel.tsx` | Supabase connection, table browser, SQL, AI queries |
| `AuthPanel` | `src/components/AuthPanel.tsx` | Auth scaffold generator (4 providers × 5 frameworks) |
| `GitHubSyncPanel` | `src/components/GitHubSyncPanel.tsx` | GitHub sync with commit/push/pull and repo management |
| `SteeringPanel` | `src/components/SteeringPanel.tsx` | Workspace/global steering files with templates |
| `TestPanel` | `src/components/TestPanel.tsx` | Test runner with framework detection, ▶ Run Tests, live log stream, filter tabs, pass/fail badges |
| `CollabPanel` | `src/components/CollabPanel.tsx` | CRDT multiplayer session management: create/join rooms, peer list with color indicators, invite link |
| `DatabasePanel` | `src/components/DatabasePanel.tsx` | SQLite/PostgreSQL browser with AI query generation |
| `ContextPicker` | `src/components/ContextPicker.tsx` | @ context dropdown; file, folder, git, web, terminal, symbol, github, jira, html-selected picker |
| `GitPanel` | `src/components/GitPanel.tsx` | Full Git workflow panel; PR review; AI commit message button (AI) |
| `Terminal` | `src/components/Terminal.tsx` | xterm.js terminal integration |
| `CommandPalette` | `src/components/CommandPalette.tsx` | Fuzzy search command palette |
| `ThemeToggle` | `src/components/ThemeToggle.tsx` | Dark/light theme switcher |
| `Modal` | `src/components/Modal.tsx` | Reusable modal dialog |
| `MarkdownPreview` | `src/components/MarkdownPreview.tsx` | Rendered markdown preview |
| `OnboardingTour` | `src/components/OnboardingTour.tsx` | First-run guided tour with feature highlights; localStorage-gated, dismissible |
| `EmptyState` | `src/components/EmptyState.tsx` | Reusable empty state placeholder with icon and message |
| `LoadingSpinner` | `src/components/LoadingSpinner.tsx` | Reusable loading spinner with optional label |

## Tauri Commands (Backend API)

The React frontend communicates with the Rust backend using Tauri's `invoke()` IPC and Tauri event emitters.

### File Operations

| Command | Description |
|---------|-------------|
| `read_file(path)` | Read file contents |
| `write_file(path, content)` | Write file to disk |
| `list_directory(path)` | List files/folders for sidebar |
| `create_directory(path)` | Create a new directory |
| `delete_item(path)` | Delete file or directory |
| `rename_item(path, new_name)` | Rename file or directory |
| `save_file(path)` | Save buffer contents to disk |

### Workspace Operations

| Command | Description |
|---------|-------------|
| `add_workspace_folder(path)` | Add folder to workspace |
| `get_workspace_folders()` | List workspace folders |
| `open_file_in_workspace(path)` | Open file within workspace |

### Text Buffer Operations

| Command | Description |
|---------|-------------|
| `insert_text(params)` | Insert text at buffer position |
| `delete_text(params)` | Delete text range from buffer |
| `apply_batch_edits(params)` | Batch insert/delete operations |
| `update_cursors(params)` | Synchronise cursor positions |

### Search

| Command | Description |
|---------|-------------|
| `search_files(query, case_sensitive)` | Search file contents with `walkdir` + `regex` |
| `search_files_for_context(query)` | File path search for @ picker (max 20 results) |

### Git Operations

| Command | Description |
|---------|-------------|
| `get_git_status()` | Get branch + file status map |
| `git_commit(path, message, files)` | Stage and commit selected files |
| `git_push(path, remote, branch)` | Push to remote |
| `git_pull(path, remote, branch)` | Pull from remote |
| `git_diff(path, file_path)` | Get diff for a file |
| `git_list_branches(path)` | List all branches |
| `git_switch_branch(path, branch)` | Switch to a branch |
| `git_get_history(path, limit)` | Get commit history |
| `git_discard_changes(path, file_path)` | Discard changes for a file |
| `git_stash_create(path, name)` | Create a named stash |
| `git_stash_pop(path)` | Pop the most recent stash |
| `get_git_context()` | Formatted git context for AI injection |

### LSP Operations

| Command | Description |
|---------|-------------|
| `lsp_completion(language, root_path, params)` | Request completions from language server |
| `lsp_hover(language, root_path, params)` | Request hover info |
| `lsp_goto_definition(language, root_path, params)` | Go to definition |
| `lsp_did_open(language, root_path, uri, text)` | Notify LSP of document open |
| `lsp_did_change(language, root_path, uri, text, version)` | Notify LSP of document change |
| `lsp_did_save(language, root_path, uri)` | Notify LSP of document save |

### AI Operations

| Command | Description |
|---------|-------------|
| `send_chat_message(request)` | Send messages to AI provider; returns response + pending writes |
| `request_ai_completion(request)` | Request AI code completion |
| `request_inline_completion(prefix, suffix, language, provider)` | Inline ghost-text completion (FIM/chat) |
| `get_available_ai_providers()` | List configured AI providers |
| `predict_next_edit(current_file, content, cursor_line, recent_edits, provider)` | AI-predicted next edit location + text |
| `fetch_url_for_context(url)` | Fetch & strip a URL for AI context |
| `inline_edit(file_path, language, selected_text, start_line, end_line, instruction, provider)` | AI-powered inline edit for Cmd+K overlay |
| `get_provider_api_keys()` | Load BYOK API key settings |
| `save_provider_api_keys(settings)` | Persist BYOK API keys to `~/.vibecli/profile_settings.db` (encrypted) |
| `search_workspace_symbols(query, workspace_path)` | Regex-based symbol search across workspace |
| `semantic_search_codebase(query, workspace_path)` | Semantic codebase search via `CodebaseIndex` |

### Agent Operations

| Command | Description |
|---------|-------------|
| `start_agent_task(task, approval_policy, provider)` | Start autonomous agent loop |
| `respond_to_agent_approval(approved)` | Approve or reject a pending agent tool call |

**Agent events emitted:**

| Event | Payload | Description |
|-------|---------|-------------|
| `agent:chunk` | `String` | Streaming LLM text |
| `agent:pending` | `AgentPendingPayload` | Tool call needs approval |
| `agent:step` | `AgentStepPayload` | Step completed |
| `agent:done` | `String` | Agent finished |
| `agent:error` | `String` | Error occurred |

### Multi-Agent Orchestration

| Command | Description |
|---------|-------------|
| `start_parallel_agents(tasks, provider, policy)` | Spawn parallel agents on worktrees |
| `get_orchestrator_status()` | Snapshot of all agent statuses |
| `merge_agent_branch(agent_id, strategy)` | Merge agent branch (merge/squash/rebase) |

**Orchestrator events emitted:**

| Event | Payload | Description |
|-------|---------|-------------|
| `manager:agent_update` | `AgentInstanceInfo` | Agent status change |
| `manager:agent_step` | `{id, step_num, tool}` | Per-step progress |

### Memory & Rules

| Command | Description |
|---------|-------------|
| `get_vibeui_rules()` | Load project-level rules (`.vibeui.md`) |
| `save_vibeui_rules(content)` | Save project-level rules |
| `get_global_rules()` | Load global rules (`~/.vibeui/rules.md`) |
| `save_global_rules(content)` | Save global rules |

### Checkpoints

| Command | Description |
|---------|-------------|
| `create_checkpoint(label)` | Create a git stash checkpoint |
| `list_checkpoints()` | List all checkpoints |
| `restore_checkpoint(index)` | Restore checkpoint by index |

### Trace / History

| Command | Description |
|---------|-------------|
| `list_trace_sessions()` | List all agent trace sessions |
| `load_trace_session(session_id)` | Load entries from a trace session |

### Hooks Configuration

| Command | Description |
|---------|-------------|
| `get_hooks_config(workspace_path?)` | Load hooks configuration |
| `save_hooks_config(hooks, workspace_path?)` | Save hooks configuration |

### Code Review

| Command | Description |
|---------|-------------|
| `run_code_review(workspace_path, base_ref?, target_ref?)` | Run AI code review, return structured report |

### MCP Server Management

| Command | Description |
|---------|-------------|
| `get_mcp_servers()` | Load configured MCP servers from `~/.vibeui/mcp.json` |
| `save_mcp_servers(servers)` | Persist MCP server configurations |
| `test_mcp_server(server)` | Test a server and list its tools |
| `initiate_mcp_oauth(server_name, client_id, auth_url, token_url, redirect_uri, scopes)` | Build OAuth URL and open browser for authorization |
| `complete_mcp_oauth(server_name, auth_code, client_id, token_url, redirect_uri)` | Exchange auth code for token, persist to `~/.vibeui/mcp-tokens.json` |
| `get_mcp_token_status(server_name)` | Check OAuth token status (connected/expired) |

#### MCP OAuth Flow

The MCP panel supports OAuth 2.0 for authenticating with MCP servers:

1. Click **OAuth**on a server → enter Client ID, Auth URL, Token URL, and Scopes
2. Click **Open Browser** → authorize in your browser
3. Paste the authorization code back into the modal
4. Token is exchanged and stored at `~/.vibeui/mcp-tokens.json`

Connected servers show a green **OAuth**badge.

### Deploy & Custom Domains

| Command | Description |
|---------|-------------|
| `detect_deploy_target(workspace)` | Auto-detect deploy target from project files |
| `run_deploy(target, workspace)` | Deploy to selected target (Vercel/Netlify/Railway/GH Pages/GCP/Firebase) |
| `get_deploy_history()` | List previous deployments |
| `set_custom_domain(target, domain)` | Configure a custom domain for the selected target |

#### Custom Domain Support

After deploying, enter a custom domain in the **Custom Domain**field:

- **Vercel**: calls the Vercel REST API (requires `VERCEL_TOKEN` env var)
- **Other targets**: returns CNAME record instructions for manual DNS configuration

### Code Complete Workflow

| Command | Description |
|---------|-------------|
| `list_workflows(workspace_path)` | List all workflows with stage progress |
| `get_workflow(workspace_path, name)` | Get a workflow by name |
| `create_workflow(workspace_path, name, description)` | Create a new 8-stage workflow |
| `advance_workflow_stage(workspace_path, name)` | Mark current stage complete, advance to next |
| `update_workflow_checklist_item(workspace_path, name, stage_index, item_id, done)` | Toggle a checklist item |
| `generate_stage_checklist(workspace_path, name, stage_index, provider)` | AI-generate checklist for a stage |

### Test Runner

| Command | Description |
|---------|-------------|
| `detect_test_framework(workspace)` | Auto-detect test framework from project files (Cargo/npm/pytest/Go) |
| `run_tests(app, workspace, command?)` | Run tests, stream `test:log` events, return `TestRunResult` with summary and per-test details |
| `generate_commit_message(workspace)` | Run `git diff --staged` → AI prompt → imperative one-liner commit message |

### Code Coverage

| Command | Description |
|---------|-------------|
| `detect_coverage_tool(workspace)` | Auto-detect coverage tool (cargo-llvm-cov / nyc / coverage.py / go-cover) |
| `run_coverage(app, workspace, tool)` | Run coverage, parse LCOV/Go coverprofile, return `CoverageResult` with per-file percentages and uncovered lines |

### Multi-Model Comparison

| Command | Description |
|---------|-------------|
| `compare_models(prompt, provider_a, model_a, provider_b, model_b)` | Send same prompt to two providers in parallel, return `CompareResult` with timing, tokens, and errors |

### HTTP Playground

| Command | Description |
|---------|-------------|
| `send_http_request(method, url, headers, body?)` | Send HTTP request with 30s timeout, return `HttpResponseData` (status, headers, body, duration) |
| `discover_api_endpoints(workspace)` | Grep workspace for Express/Axum/FastAPI/Spring route patterns (max 60 results) |

### Cost & Performance Observatory

| Command | Description |
|---------|-------------|
| `record_cost_entry(session_id, provider, model, prompt_tokens, completion_tokens, task_hint?)` | Append cost record to `~/.vibeui/cost-log.jsonl` with `estimated_cost_usd` pricing |
| `get_cost_metrics()` | Load all entries, compute per-provider aggregates, total cost/tokens, budget remaining |
| `set_cost_limit(limit_usd?)` | Set or clear monthly budget cap (persisted at `~/.vibeui/cost-config.json`) |
| `clear_cost_history()` | Delete the cost log file |

### AI Git Workflow

| Command | Description |
|---------|-------------|
| `suggest_branch_name(task_description)` | AI-generated concise hyphenated branch name from task description |
| `resolve_merge_conflict(file_path, conflict_text)` | AI-resolved merge conflict preserving both sides |
| `generate_changelog(workspace, since_ref?)` | Convert `git log` into Keep-a-Changelog format via LLM |

### Codemod & Lint Auto-Fix

| Command | Description |
|---------|-------------|
| `run_autofix(workspace, framework?)` | Auto-detect and run linter fix mode (clippy/eslint/ruff/gofmt/prettier), return diff + file count |
| `apply_autofix(workspace, apply)` | Stage fixed changes (`apply=true`) or revert via `git restore` (`apply=false`) |

### Arena Mode (Blind A/B Comparison)

| Command | Description |
|---------|-------------|
| `save_arena_vote(vote)` | Persist a blind vote to `~/.vibeui/arena-votes.json` (provider A/B, winner, prompt, timestamp) |
| `get_arena_history()` | Load all arena votes and compute per-provider stats (wins/losses/ties/win-rate) |

### Multiplayer Collaboration

| Command | Description |
|---------|-------------|
| `create_collab_session(room_id?, user_name, daemon_port?)` | Create a new collab room; returns room ID, peer ID, and WebSocket URL |
| `join_collab_session(room_id, user_name, daemon_port?)` | Join an existing collab room by ID |
| `leave_collab_session()` | Leave the current collab session (WebSocket cleanup handled by frontend) |
| `list_collab_peers(room_id, daemon_port?, api_token?)` | List connected peers in a room via daemon REST API |
| `get_collab_status(room_id?)` | Get collab connection status (connected, room_id, peer_count) |

### Flow Tracking

| Command | Description |
|---------|-------------|
| `track_flow_event(kind, data)` | Record a developer activity event |
| `get_flow_context()` | Get recent activity as formatted context |

### Terminal Operations

| Command | Description |
|---------|-------------|
| `spawn_terminal()` | Spawn a new PTY terminal |
| `write_terminal(id, data)` | Send input to terminal |
| `resize_terminal(id, rows, cols)` | Resize terminal |

### Counsel (Multi-LLM Deliberation)

| Command | Description |
|---------|-------------|
| `counsel_create_session(topic, participants, moderator_idx)` | Create a multi-LLM deliberation session with role-based participants |
| `counsel_list_sessions()` | List all counsel session summaries |
| `counsel_get_session(session_id)` | Load a counsel session by ID |
| `counsel_run_round(session_id)` | Execute a deliberation round across all participants |
| `counsel_synthesize(session_id)` | Generate moderator synthesis of all rounds |
| `counsel_inject_message(session_id, message)` | Inject a user interjection between rounds |
| `counsel_vote(session_id, round_idx, participant_idx, delta)` | Vote on a participant's response |

**Counsel events emitted:**

| Event | Payload | Description |
|-------|---------|-------------|
| `counsel:chunk` | `String` | Streaming participant response text |

### SuperBrain (Multi-Provider Routing)

| Command | Description |
|---------|-------------|
| `superbrain_route(query)` | Route a query to the best provider using keyword-based analysis |
| `superbrain_query(query, mode)` | Execute query in a specific mode (SmartRouter/Consensus/ChainRelay/BestOfN/Specialist) |
| `superbrain_get_modes()` | List available SuperBrain modes with descriptions |

**SuperBrain events emitted:**

| Event | Payload | Description |
|-------|---------|-------------|
| `superbrain:chunk` | `String` | Streaming response from current model |

### Utility

| Command | Description |
|---------|-------------|
| `open_external_url(url)` | Open URL in system browser |

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
| `terminal` | `TerminalManager` | PTY terminal via `portable-pty` |
| `diff` | `DiffEngine`, `DiffHunk` | Text diff via `similar` |
| `executor` | `CommandExecutor` | Sandboxed command execution |
| `context` | `ContextBuilder` | Token-budget-aware smart context for AI requests |
| `index` | `CodebaseIndex` | Symbol indexing with TF-IDF embeddings for relevance ranking |
| `index/embeddings` | `EmbeddingIndex` | Vector similarity search over codebase symbols |
| `index/symbol` | `Symbol`, `SymbolKind` | Symbol extraction from source files |

### `vibe-ai`

AI abstraction layer:

| Module | Description |
|--------|-------------|
| `provider` | `AIProvider` trait, `Message`, `CodeContext`, `ImageAttachment` |
| `providers/ollama` | Ollama HTTP API (streaming) |
| `providers/claude` | Anthropic Claude API (streaming + vision) |
| `providers/openai` | OpenAI Chat Completions API (streaming + vision) |
| `providers/gemini` | Google Gemini API (streaming) |
| `providers/grok` | xAI Grok API (streaming) |
| `providers/groq` | Groq API (streaming) |
| `providers/mistral` | Mistral API (streaming) |
| `providers/cerebras` | Cerebras API (streaming) |
| `providers/deepseek` | DeepSeek API (streaming) |
| `providers/zhipu` | Zhipu GLM API (streaming) |
| `providers/openrouter` | OpenRouter multi-model API |
| `providers/azure_openai` | Azure OpenAI Service API |
| `providers/bedrock` | AWS Bedrock with SigV4 auth |
| `providers/copilot` | GitHub Copilot integration |
| `providers/vercel_ai` | Vercel AI SDK API |
| `providers/local_edit` | Local edit provider |
| `providers/failover` | Automatic provider fallback chain |
| `chat` | `ChatEngine` — session management |
| `completion` | `CompletionEngine` — inline code completion |
| `agent` | `AgentLoop` — plan→act→observe loop with approval tiers |
| `planner` | `PlannerAgent` — plan generation without execution |
| `multi_agent` | `MultiAgentOrchestrator` — parallel agents on git worktrees |
| `hooks` | `HookRunner` — event-driven hooks (shell + LLM handlers) |
| `skills` | `SkillLoader` — auto-activating context snippets |
| `artifacts` | `ArtifactStore` — structured output with annotations |
| `policy` | `AdminPolicy` — glob-based tool allow/deny with `denied_tool_patterns = ["bash(rm*)"]` |
| `rules` | `RulesLoader` — load `.vibecli/rules/*.md` with YAML front-matter path-pattern filtering |
| `tools` | `ToolCall`, `ToolResult`, prompt-based tool framework |
| `mcp` | `McpClient` — JSON-RPC 2.0 MCP server integration |
| `trace` | `TraceWriter` — JSONL audit log + session resume |
| `otel` | OpenTelemetry span attribute constants |

### `vibe-collab`

CRDT-based multiplayer collaboration:

| Module | Description |
|--------|-------------|
| `server` | `CollabServer` — DashMap concurrent room registry, get_or_create_room, cleanup empty rooms |
| `room` | `CollabRoom` — Y.Doc per room, Y.Text per file path, peer list, broadcast channel fan-out |
| `protocol` | Yjs binary sync protocol (SyncStep1/SyncStep2/Update), encode/decode/apply helpers |
| `awareness` | `PeerInfo`, `CursorState`, `AwarenessState`, 8-color peer palette |
| `error` | `CollabError` enum with HTTP status code conversion |

### `vibe-lsp`

Language Server Protocol client:

- JSON-RPC message framing via `jsonrpc-core` and `tokio-util`
- Async LSP server process management
- LSP types from `lsp-types`
- Full document lifecycle notifications (`didOpen`, `didChange`, `didSave`)
- Wired to Monaco for go-to-definition, hover, completions, diagnostics

### `vibe-extensions`

WASM extension system:

- Powered by `wasmtime` 27
- Auto-loads `*.wasm` files from `~/.vibeui/extensions/`
- Host functions: `host_log`, `host_notify`, `host_read_file`, `host_write_file`
- String ABI: extensions export `alloc(size) → ptr` and `memory`
- Extension lifecycle callbacks: `init()`, `on_file_save(path)`, `on_text_change(path, content)`
- **Extension Manager** — frontend `ExtensionManager.ts` + `ExtensionHost` Web Worker
- **VS Code API shim** — partial `vscode` namespace compatibility layer

#### Writing an Extension

Extensions are compiled to `wasm32-unknown-unknown`. A minimal Rust extension:

```rust
// In your extension's lib.rs (target: wasm32-unknown-unknown)
extern "C" {
    fn host_log(ptr: i32, len: i32);
}

#[no_mangle]
pub extern "C" fn init() -> i32 {
    let msg = "Hello from extension!";
    unsafe { host_log(msg.as_ptr() as i32, msg.len() as i32); }
    0
}

#[no_mangle]
pub extern "C" fn alloc(size: i32) -> i32 {
    let mut v: Vec<u8> = Vec::with_capacity(size as usize);
    let ptr = v.as_ptr() as i32;
    std::mem::forget(v);
    ptr
}
```

## Backend Modules

### `agent_executor.rs`

`TauriToolExecutor` implements `ToolExecutorTrait` for the desktop environment:

| Tool | Method | Description |
|------|--------|-------------|
| `read_file` | `read_file(path)` | Read file contents (truncated at 8 KB) |
| `write_file` | `write_file(path, content)` | Write file with auto-mkdir |
| `bash` | `run_bash(command)` | Shell command execution in workspace root |
| `search_files` | `search_files(query, glob?)` | Content search with optional glob filter |
| `list_directory` | `list_dir(path)` | Directory listing |
| `web_search` | `web_search(query)` | DuckDuckGo Lite search |
| `fetch_url` | `fetch_url(url)` | Fetch and strip HTML from URL |
| `task_complete` | — | Signal task completion |

> **Note:** `apply_patch` is not supported — the agent is instructed to use `write_file` with full contents instead.

### `flow.rs` — Flow Awareness Engine

Tracks developer activity in a 100-event ring buffer:

| Event Kind | Data | Trigger |
|------------|------|---------|
| `file_open` | File path | Opening a file |
| `file_edit` | File path | Editing a file (debounced) |
| `file_save` | File path | Saving a file |
| `terminal_cmd` | Command string | Running a terminal command |

`context_string(limit)` returns a formatted summary: recently opened files, recently edited files, and recent terminal commands.

### `memory.rs` — AI Rules

| Function | Description |
|----------|-------------|
| `load_workspace_rules(root)` | Load `<workspace>/.vibeui.md` |
| `load_global_rules()` | Load `~/.vibeui/rules.md` |
| `combined_rules(root)` | Merge both into a single system-prompt injection |
| `save_workspace_rules(root, content)` | Save project rules |
| `save_global_rules(content)` | Save global rules (creates `~/.vibeui/` if needed) |

### `counsel.rs` — Multi-LLM Deliberation

Structured multi-round debates between AI providers:

| Struct | Description |
|--------|-------------|
| `CounselSession` | Manages deliberation with topic, participants, rounds, and synthesis |
| `CounselParticipant` | AI provider assigned a role (Expert, Devil's Advocate, Skeptic, Creative, Pragmatist, Researcher) |
| `CounselRound` | Collection of responses with optional user feedback per round |
| `CounselResponse` | Individual model response with voting support |

### `superbrain.rs` — Multi-Provider Query Routing

Intelligent query routing and multi-model synthesis:

| Mode | Description |
|------|-------------|
| `SmartRouter` | Keyword-based routing to best model per task type |
| `Consensus` | All models respond; majority view synthesized |
| `ChainRelay` | Sequential refinement — each model builds on previous |
| `BestOfN` | All models respond; judge picks the best |
| `Specialist` | Decomposes query into subtasks assigned to different models |

### `web_client.rs` — Browser-Based Web Client

Self-contained single-page web client for zero-install access (similar to Bolt.new/v0):

- No external CDN dependencies (air-gap safe)
- Chat and Agent modes with SSE streaming
- Markdown rendering with syntax highlighting
- Dark/light theme support, responsive design
- Served from the VibeCLI `serve` command

## Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| `Cmd/Ctrl + S` | Save file |
| `Cmd/Ctrl + P` | Command palette |
| `Cmd/Ctrl + K` | Command palette |
| `Cmd/Ctrl + Shift + P` | Command palette (VS Code alias) |
| `Cmd/Ctrl + B` | Toggle sidebar |
| `Cmd/Ctrl + J` | Toggle AI panel |
| `Cmd/Ctrl + `` ` `` | Toggle terminal |
| `Cmd/Ctrl + 1`–`9` | Switch AI panel tab (Chat=1, Agent=2, Memory=3, …) |
| `Cmd/Ctrl + Shift + E` | Focus explorer sidebar |
| `Cmd/Ctrl + Shift + G` | Focus git sidebar |
| `Cmd/Ctrl + Option/Alt + I` | Open DevTools |
| `Tab` | Indent selection (in editor) |

## Accessibility (WCAG 2.1 AA)

VibeUI implements WCAG 2.1 Level AA accessibility:

| Feature | Description |
|---------|-------------|
| Focus-visible outlines | 2px blue outline on keyboard focus (`:focus-visible`); suppressed on mouse click |
| Skip-to-content link | Hidden link appears on first Tab press, jumps past sidebar to editor region |
| Command palette ARIA | `role="dialog"`, `combobox` input, `listbox` results, `option` items, `aria-activedescendant` |
| Modal focus trap | Tab cycles within modal; Escape closes; previous focus restored on close |
| Agent status announcements | `aria-live="polite"` region announces running/complete/error/idle to screen readers |
| Screen-reader utility | `.sr-only` CSS class for visually-hidden accessible text |
| Onboarding tour | First-run guided walkthrough (localStorage gate), dismissible |

## Testing

**9,570 tests** pass across the workspace (as of 2026-03-29, 0 failures).

| Crate | Tests | Key coverage areas |
|-------|-------|--------------------|
| vibecli | 5,262+ | session store, serve, config, review, workflow, REPL, redteam, gateway, transform, marketplace, background agents, TUI, tool executor, bugbot, vim editor, security scan, automations, legacy migration, batch builder, QA validation, RAG, GPU cluster, inference, training, counsel, superbrain, web client, open memory, auto research, blue/purple team, IDP, agent teams, cloud agents, workflow orchestration |
| vibe-ai | 1,020+ | 23 providers, tools, trace, hooks, policy, skills, artifacts, planner, MCP, agent teams, multi-agent, SigV4, gemini |
| vibe-core | 293 | buffer, git, diff, context, file system, workspace, search, terminal, index/embeddings |
| vibe-ui (Tauri) | 227 | Tauri commands, coverage, cost, flow, agent executor, shadow workspace |
| vibe-lsp | 74 | LSP client, features, manager, language configs |
| vibe-collab | 53 | CRDT rooms, server registry, protocol sync, awareness |
| vibe-extensions | 46 | loader, manifest, permissions, registry |

**Benchmarks:** 8 Criterion benchmarks covering cosine similarity (384d/1536d/batch), symbol extraction (50/500 fns), index build (100 files), symbol search, and relevance scoring.

```bash
# Rust unit tests
cargo test --workspace

# Specific crates
cargo test -p vibe-core
cargo test -p vibe-ai
cargo test -p vibe-collab

# TypeScript type check
cd vibeui && npx tsc --noEmit

# End-to-end tests
cd vibeui/e2e && npm test
```

See [TESTING.md](https://github.com/TuringWorks/vibecody/blob/main/vibeui/TESTING.md) for manual testing checklist.

## Debugging

Open DevTools in the running app:

- **macOS**: `Cmd + Option + I`
- **Windows/Linux**: `Ctrl + Shift + I`

Or right-click anywhere and select **Inspect**.

See [DEBUG.md](https://github.com/TuringWorks/vibecody/blob/main/vibeui/DEBUG.md) for common troubleshooting steps.
