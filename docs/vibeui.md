---
layout: page
title: VibeUI Reference
permalink: /vibeui/
---

# VibeUI

**AI-powered desktop code editor built with Tauri 2 and Monaco.**

VibeUI provides a VS Code-like editing experience with a native Rust backend, Monaco Editor frontend, integrated AI chat, autonomous agent mode, inline completions, terminal, Git panel, code review, and a WASM extension system.

---

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
│  commands.rs    — 60+ Tauri commands (files, git, AI, agent …)   │
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
- **Batch edits** — `apply_batch_edits` for bulk insert/delete operations
- **Multi-cursor** — `update_cursors` for synchronised cursor state
- **Inline AI completions** — ghost-text suggestions via `request_inline_completion` (FIM for Ollama, chat prompt for cloud providers)
- **Next-Edit Prediction** — `predict_next_edit` analyses recent edits to suggest the next likely change; wired as inline completions provider with 500ms debounce + Tab acceptance
- **Inline Chat (Cmd+K)** — floating `InlineChat` overlay; select code, describe change, view streamed result, Accept to apply edit in-place
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

Select the provider from the dropdown in the header, or switch per chat tab in `ChatTabManager`. Provider configuration is handled through the **⚙️ Keys** settings tab (BYOK), environment variables, or `~/.vibecli/config.toml`.

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

**Jira** requires `JIRA_BASE_URL`, `JIRA_EMAIL`, and `JIRA_API_TOKEN` environment variables. **GitHub** uses optional `GITHUB_TOKEN` for higher rate limits.

#### Smart Context Builder

The `ContextBuilder` in `vibe-core` builds a ranked, token-budget-aware context by combining:

1. **Git branch + changed file list** — always included
2. **Git diff** — up to 25% of budget
3. **Top-ranked symbols** (via codebase index) — up to 30% of budget
4. **Open file contents** — remaining budget

#### Flow Awareness

The **Flow Awareness Engine** (`flow.rs`) tracks developer activity (file opens, edits, saves, terminal commands) in a 100-event ring buffer. Recent activity is injected into AI context via `get_flow_context()`.

### Agent Mode

The **Agent Panel** provides autonomous multi-step task execution:

- **Plan mode** — agent generates a plan before executing
- **Approval tiers** — `auto`, `suggest`, `always-ask` policies
- **Tool execution** — `TauriToolExecutor` supports: `read_file`, `write_file`, `bash`, `search_files`, `list_directory`, `web_search`, `fetch_url`, `task_complete`
- **Step timeline** — each tool call emits `agent:step` events rendered in the UI
- **Pending approval** — destructive operations emit `agent:pending` for user review
- **Streaming** — LLM output streamed via `agent:chunk` Tauri events
- **Diff review** — pending writes shown in a Monaco DiffEditor with Accept/Reject

### Multi-Agent Orchestration (Manager View)

The **Manager View** enables running multiple agents in parallel:

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

The **Checkpoint Panel** provides a timeline of AI checkpoints backed by Git stashes:

- **Create checkpoint** — `create_checkpoint(label)` saves current workspace state
- **List checkpoints** — browse all stash entries with labels and timestamps
- **Restore checkpoint** — apply a stash without dropping it
- **Auto-checkpoint** — agent mode can auto-create checkpoints before destructive operations

### Artifacts

The **Artifacts Panel** displays structured AI output:

- Rich artifact cards with metadata
- Annotations and inline comments
- Async feedback mechanism

### Hooks Configuration

The **Hooks Panel** provides a UI for configuring event-driven hooks:

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

The **Memory Panel** provides editors for AI rules:

- **Project rules** — `.vibeui.md` in workspace root (committed alongside code)
- **Global rules** — `~/.vibeui/rules.md` (personal defaults)
- Both are injected into every AI system prompt via `combined_rules()`

### Agent History

The **History Panel** displays an audit log of past agent sessions:

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

---

## AI Panel Tabs

The AI panel (toggle with **💬 AI Chat** in the header) has the following tabs:

| Tab | Component | Description |
|-----|-----------|-------------|
| **💬 Chat** | `ChatTabManager` | Multiple independent chat tabs, each with per-tab provider selection; voice input (🎤) |
| **🤖 Agent** | `AgentPanel` | Autonomous multi-step agent with step timeline, approval UI, Turbo mode, and plan mode |
| **🧠 Memory** | `MemoryPanel` | Edit per-workspace `.vibeui.md` and global `~/.vibeui/rules.md` |
| **🕐 History** | `HistoryPanel` | Audit log of past agent sessions; browse and expand trace entries |
| **📝 Checkpoints** | `CheckpointPanel` | Timeline of AI checkpoints with restore, auto-checkpoint |
| **📦 Artifacts** | `ArtifactsPanel` | Structured output cards with annotations and async feedback |
| **👥 Manager** | `ManagerView` | Multi-agent orchestration: task board, worktrees, parallel execution |
| **🪝 Hooks** | `HooksPanel` | Configure event-driven hooks (PreToolUse, PostToolUse, etc.) |
| **📋 Jobs** | `BackgroundJobsPanel` | Submit tasks to VibeCLI daemon; live SSE stream; job persistence across restarts |
| **⚙️ Keys** | `SettingsPanel` | BYOK API key management for all cloud providers |
| **📐 Specs** | `SpecPanel` | Spec-driven development: AI-generated user stories, tasks, and acceptance criteria |
| **🏗️ Workflow** | `WorkflowPanel` | Code Complete 8-stage development pipeline with AI-generated checklists per stage |
| **🛡️ RedTeam** | `RedTeamPanel` | Autonomous 5-stage pentest pipeline with findings feed, CVSS scores, and report export |
| **🔌 MCP** | `McpPanel` | MCP server management with OAuth 2.0 install flow and tool testing |
| **🚀 Deploy** | `DeployPanel` | Deploy to 6 targets (Vercel/Netlify/Railway/GH Pages/GCP/Firebase) with custom domain support |
| **🐛 BugBot** | `BugBotPanel` | AI code scanner with severity/category filter and fix snippets |
| **🐘 Supabase** | `SupabasePanel` | Supabase connection, table browser, SQL editor, AI query |
| **🔐 Auth** | `AuthPanel` | Auth scaffold generator (4 providers × 5 frameworks) |
| **🐙 GH Sync** | `GitHubSyncPanel` | GitHub sync with ahead/behind status, push/pull, repo management |
| **🧭 Steering** | `SteeringPanel` | Workspace/global steering files with templates |
| **🧪 Tests** | `TestPanel` | Test runner with framework detection, live log stream, filter tabs, pass/fail badges |
| **👥 Collab** | `CollabPanel` | CRDT multiplayer collaboration: create/join rooms, peer list with color indicators, copy invite link |
| **🥊 Arena** | `ArenaPanel` | Blind A/B model comparison: hidden identities, vote (A/B/Tie/Both bad), reveal, persistent leaderboard |

---

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
| `BrowserPanel` | `src/components/BrowserPanel.tsx` | Embedded iframe browser with inspect mode (🔍), element selection, Send to Chat |
| `ArenaPanel` | `src/components/ArenaPanel.tsx` | Blind A/B model comparison with voting, leaderboard, and Send winner to Chat |
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
| `GitPanel` | `src/components/GitPanel.tsx` | Full Git workflow panel; PR review; AI commit message button (✨ AI) |
| `Terminal` | `src/components/Terminal.tsx` | xterm.js terminal integration |
| `CommandPalette` | `src/components/CommandPalette.tsx` | Fuzzy search command palette |
| `ThemeToggle` | `src/components/ThemeToggle.tsx` | Dark/light theme switcher |
| `Modal` | `src/components/Modal.tsx` | Reusable modal dialog |
| `MarkdownPreview` | `src/components/MarkdownPreview.tsx` | Rendered markdown preview |

---

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
| `save_provider_api_keys(settings)` | Persist BYOK API keys to `~/.vibeui/api_keys.json` |
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

1. Click **OAuth** on a server → enter Client ID, Auth URL, Token URL, and Scopes
2. Click **Open Browser** → authorize in your browser
3. Paste the authorization code back into the modal
4. Token is exchanged and stored at `~/.vibeui/mcp-tokens.json`

Connected servers show a green **🔑 OAuth** badge.

### Deploy & Custom Domains

| Command | Description |
|---------|-------------|
| `detect_deploy_target(workspace)` | Auto-detect deploy target from project files |
| `run_deploy(target, workspace)` | Deploy to selected target (Vercel/Netlify/Railway/GH Pages/GCP/Firebase) |
| `get_deploy_history()` | List previous deployments |
| `set_custom_domain(target, domain)` | Configure a custom domain for the selected target |

#### Custom Domain Support

After deploying, enter a custom domain in the **🌐 Custom Domain** field:

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

### Utility

| Command | Description |
|---------|-------------|
| `open_external_url(url)` | Open URL in system browser |

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

---

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
