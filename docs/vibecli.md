---
layout: page
title: VibeCLI Reference
permalink: /vibecli/
---

# VibeCLI

**AI-powered coding assistant for the terminal.**

VibeCLI provides two interaction modes: a rich **Terminal UI (TUI)** powered by Ratatui, and a **REPL** mode for quick, scriptable use.

## Installation

### Prerequisites

- Rust stable (≥ 1.75) — install via [rustup](https://rustup.rs/)
- `git` (for Git integration features)
- Ollama (optional, for local AI — [ollama.ai](https://ollama.ai))

### Build from Source

```bash
git clone https://github.com/TuringWorks/vibecody.git
cd vibecody

cargo build --release -p vibecli

# Optional: install to PATH
cp target/release/vibecli /usr/local/bin/
```

## Usage

### TUI Mode (Recommended)

```bash
vibecli --tui
vibecli --tui --provider claude
vibecli --tui --provider openai --model gpt-4o
```

### REPL Mode

```bash
vibecli
vibecli --provider gemini
```

### Command-Line Arguments

| Flag | Default | Description |
|------|---------|-------------|
| `--provider <name>` | `ollama` | AI provider: `ollama`, `claude`, `openai`, `gemini`, `grok`, `groq`, `openrouter`, `azure`, `bedrock`, `copilot`, `mistral`, `cerebras`, `deepseek`, `zhipu`, `vercel`, `minimax`, `perplexity`, `together`, `fireworks`, `sambanova` |
| `--channel-daemon <platform>` | — | Start always-on channel daemon: `telegram`, `discord`, `slack` |
| `--model <name>` | provider default | Override the model for the selected provider |
| `--tui` | false | Launch the Terminal UI instead of REPL |
| `--exec <task>` | — | Run an agent task non-interactively (CI mode) |
| `--auto-edit` | false | Auto-apply file edits; prompt for bash commands |
| `--full-auto` | false | Auto-execute everything (use with `--sandbox` in CI) |
| `--output-format <fmt>` | `json` | Report format for `--exec`: `json`, `markdown`, `verbose` |
| `--output <file>` | stdout | Write `--exec` report to a file instead of stdout |
| `--resume <id>` | — | Resume a previous agent session by trace ID |
| `--parallel <n>` | — | Run task across N parallel agents on git worktrees |
| `--sandbox` | false | Enable OS-level sandbox (sandbox-exec/bwrap) |
| `--review` | false | Run code review agent on current diff |
| `--redteam <url>` | — | Run autonomous red team scan against target URL |
| `--redteam-config <file>` | — | YAML config file for auth flows, scope, depth |
| `--redteam-report <id>` | — | Generate pentest report from a previous session |
| `--voice` | false | Enable voice input via Groq Whisper |
| `--tailscale` | false | Enable Tailscale funnel for remote access |
| `--serve` | false | Run as HTTP daemon (REST + SSE API) |

## TUI Commands

Once inside the TUI, type messages naturally or use slash commands:

| Command | Description |
|---------|-------------|
| `/chat <message>` | Start or continue a conversation with AI |
| `/diff` | Show full multi-file git diff of current workspace |
| `/diff <file>` | Show git diff for a specific file |
| `/files` | Browse workspace file tree |
| `/quit` or `/exit` | Exit VibeCLI |
| `Tab` | Toggle between Chat and Diff views |

## REPL Commands

In REPL mode, the following slash commands are available:

| Command | Description |
|---------|-------------|
| `/chat <message>` | Chat with the AI (maintains conversation history) |
| `/chat [image.png] <message>` | Chat with a vision model and attach an image |
| `/generate <prompt>` | Generate code from a natural language description |
| `/agent <task>` | Run the autonomous agent loop for a multi-step task |
| `/diff <file>` | Show the git diff for a file |
| `/apply <file> <changes>` | Apply AI-generated changes to a file (with diff preview) |
| `/exec <task>` | Generate a shell command from a description and optionally run it |
| `/trace` | List recent agent session traces |
| `/trace view <id>` | Show detailed timeline for a trace session |
| `/plan <task>` | Generate an execution plan without acting |
| `/review` | Run code review agent on current diff |
| `/review --pr <n>` | Review a GitHub PR and post comments |
| `/resume <id>` | Resume a previous agent session |
| `/skills` | List available skills and their triggers |
| `/mcp list` | List configured MCP servers |
| `/mcp tools <server>` | List tools provided by an MCP server |
| `/workflow new <name> <desc>` | Create a Code Complete workflow (8-stage development pipeline) |
| `/workflow list` | List all workflows with current stage and progress |
| `/workflow show <name>` | Display workflow stages, checklist progress, and current stage items |
| `/workflow advance <name>` | Mark current stage complete and advance to the next |
| `/workflow check <name> <id>` | Toggle a checklist item in the current stage |
| `/workflow generate <name>` | AI-generate a checklist for the current stage |
| `/redteam scan <url>` | Start an autonomous red team scan against a target URL |
| `/redteam list` | List all red team sessions |
| `/redteam show <id>` | Display findings for a session |
| `/redteam report <id>` | Generate a full pentest report |
| `/redteam config` | Show current red team configuration |
| `/test [command]` | Run project tests (auto-detects cargo/npm/pytest/go); optional custom command override |
| `/arena compare <p1> <p2> [prompt]` | Blind A/B model comparison — hidden identities, vote, reveal |
| `/arena stats` | Show arena leaderboard (wins/losses/ties per provider) |
| `/arena history` | Show arena vote history |
| `/sessions` | List all stored agent sessions |
| `/sessions show <id>` | View a session's messages and steps |
| `/sessions search <query>` | Search across all sessions |
| `/voice` | Toggle voice input (Groq Whisper) |
| `/discover` | Discover nearby VibeCLI instances via mDNS |
| `/pair` | Generate QR code for device pairing |
| `/orchestrate status` | Show orchestration state (lessons, tasks, progress) |
| `/orchestrate lesson <text>` | Record a lesson learned |
| `/orchestrate todo <text>` | Add a todo item |
| `/collab create [room]` | Create a CRDT collaboration room |
| `/collab join <room>` | Join an existing collaboration room |
| `/coverage` | Run code coverage with auto-detected tool |
| `/transform <type>` | Run code transforms (Python 2→3, Vue 2→3, etc.) |
| `/deps` | Show dependency tree and outdated packages |
| `/docker` | Docker container management |
| `/sandbox` | Container sandbox management |
| `/gateway` | Gateway messaging (Telegram, Discord, Slack, etc.) |
| `/demo list` | List all recorded feature demos |
| `/demo run <name> <json>` | Record a demo by executing JSON steps (CDP browser automation) |
| `/demo generate <desc>` | AI-generate demo steps from a feature description |
| `/demo export <id> [html\|md]` | Export a demo as HTML slideshow or markdown |
| `/soul` | Generate SOUL.md for the current project (scans structure, license, languages) |
| `/soul show` | View existing SOUL.md |
| `/soul scan` | Show detected project signals (languages, frameworks, license, etc.) |
| `/soul regenerate` | Overwrite existing SOUL.md |
| `/soul prompt` | Get LLM prompt for richer AI-assisted generation |
| `/bundle create <name>` | Create a new context bundle (like Copilot Spaces) |
| `/bundle activate <id>` | Activate a context bundle |
| `/bundle deactivate <id>` | Deactivate a context bundle |
| `/bundle list` | List all context bundles with active/inactive status |
| `/bundle share <id>` | Export a bundle to .vibebundle.toml |
| `/bundle import <file>` | Import a bundle from file |
| `/cloud scan` | Scan project for AWS/GCP/Azure service usage |
| `/cloud iam` | Generate least-privilege IAM policy from detected services |
| `/cloud terraform` | Generate Terraform template from detected services |
| `/cloud cost` | Estimate monthly cloud costs |
| `/cloud providers` | List supported cloud providers and services |
| `/benchmark run` | Start a SWE-bench benchmark run |
| `/benchmark compare` | Compare multiple benchmark runs |
| `/benchmark list` | List all benchmark runs |
| `/metering status` | Show usage metering status (tokens, cost, budgets) |
| `/metering budget` | Manage credit budgets |
| `/metering report` | Generate usage report by provider/model/task |
| `/metering alerts` | Show budget alerts |
| `/blueteam status` | Blue team defensive security overview |
| `/blueteam scan` | Run threat scan across IOC feeds and log sources |
| `/blueteam incidents` | List and manage security incidents (P1-P4) |
| `/blueteam iocs` | IOC database — search and manage indicators of compromise |
| `/blueteam rules` | Detection rule management (Splunk, Sentinel, Elastic, etc.) |
| `/blueteam forensics` | Forensic case management with chain of custody |
| `/blueteam playbooks` | Incident response playbook management |
| `/blueteam siem` | SIEM connection management (8 platforms) |
| `/blueteam hunt` | Threat hunting — hypothesis-driven queries |
| `/blueteam report` | Generate blue team security report |
| `/purpleteam status` | Purple team ATT&CK exercise overview |
| `/purpleteam exercise` | Create and manage purple team exercises |
| `/purpleteam simulate` | Run attack simulations against MITRE techniques |
| `/purpleteam validate` | Validate detection rules against simulated attacks |
| `/purpleteam matrix` | Display MITRE ATT&CK coverage matrix |
| `/purpleteam gaps` | Identify detection coverage gaps |
| `/purpleteam heatmap` | Generate ATT&CK tactic heatmap |
| `/purpleteam report` | Generate purple team exercise report |
| `/idp status` | Internal developer platform overview |
| `/idp catalog` | Browse and search the service catalog |
| `/idp register` | Register a new service in the catalog |
| `/idp golden` | List golden paths (paved roads) for new services |
| `/idp scorecard` | Evaluate service scorecard (DORA + quality metrics) |
| `/idp infra` | Self-service infrastructure provisioning |
| `/idp team` | Team management and member directory |
| `/idp onboard` | Team onboarding checklist and progress |
| `/idp backstage` | Backstage integration (catalog-info.yaml, templates) |
| `/idp platforms` | List all 12 supported IDP platforms and their status |
| `/idp report` | Generate IDP report |
| `/init` | Scan project and cache profile (languages, frameworks, build/test/lint commands) |
| `/orient` | AI-powered project analysis (architecture, entry points, recommendations) |
| `/daemon start\|stop\|status\|channels` | Always-on channel daemon management (Slack, Discord, GitHub webhooks) |
| `/vm launch\|list\|status\|stop` | Cloud VM agent orchestration (parallel agents in isolated containers) |
| `/branch-agent create\|list\|status` | Agent-per-branch workflow (auto-creates feature branches, auto-PR on completion) |
| `/design import <url\|path>` | Design-to-code: import Figma, SVG, or screenshots → React/Vue/Svelte components |
| `/audio speak\|changelog\|summary` | Text-to-speech output for changelogs, summaries, and agent results |
| `/org index\|search\|patterns` | Cross-repo org-wide context engine (shared embeddings, conventions) |
| `/share-session export\|import\|spectate` | Agent session sharing — export for review, live spectating |
| `/data load\|query\|viz\|summary` | Data analysis mode — load CSV/JSON, run SQL, generate visualizations |
| `/ci-gates list\|validate\|add` | Source-controlled CI quality gates |
| `/extension install\|list\|remove` | VS Code extension compatibility (TextMate grammars, snippets, themes) |
| `/agentic fix-build\|gen-tests\|resolve-merge` | Agentic CI/CD — auto-fix builds, generate tests, resolve conflicts |
| `/counsel new <topic>` | Create a multi-LLM deliberation session on a topic |
| `/counsel run <id>` | Run a deliberation round (all participants respond) |
| `/counsel inject <id> <msg>` | Inject a user interjection into a counsel session |
| `/counsel synthesize <id>` | Generate moderator synthesis of all deliberation rounds |
| `/counsel vote <id> <round> <participant> <+1\|-1>` | Vote on a participant's response |
| `/counsel list` | List all counsel sessions |
| `/counsel show <id>` | Show a counsel session's rounds and synthesis |
| `/superbrain <query>` | Route a query to the best provider via keyword analysis (SmartRouter mode) |
| `/superbrain consensus <query>` | All providers respond; synthesize majority view |
| `/superbrain chain <query>` | Sequential refinement — each model builds on previous |
| `/superbrain best <query>` | All providers respond; judge picks the best |
| `/superbrain specialist <query>` | Decompose query into subtasks assigned to different models |
| `/superbrain modes` | List available SuperBrain routing modes |
| `/config` | Display current configuration |
| `/help` | Show command reference |
| `/exit` or `/quit` | Exit VibeCLI |
| `! <command>` | Execute a shell command directly (e.g. `!ls -la`) |

### Session & Context

| Command | Description |
|---------|-------------|
| `/status` | Show provider, model, and session info |
| `/cost` | Show token usage and estimated cost for the current session |
| `/context` | Show active context window size |
| `/fork [session-name]` | Fork current session into a named branch |
| `/rewind [list \| <timestamp>]` | Save or restore a conversation checkpoint |
| `/share <session_id>` | Print shareable URL for a session (requires `vibecli serve`) |
| `/jobs` | List background agent jobs |
| `/model <provider> [model]` | Switch active AI provider and model |

### Profiles & Memory

| Command | Description |
|---------|-------------|
| `/profile list\|show\|create\|delete` | Manage user profiles |
| `/memory show\|edit` | View or edit the auto-recording `memory.md` file |
| `/openmemory` | Cognitive memory engine (shows help + live stats) |
| `/openmemory add <text>` | Store a memory (sector auto-classified) |
| `/openmemory query <text>` | Semantic search with 5-signal composite scoring |
| `/openmemory list` | List all memories with sector, salience, tags |
| `/openmemory stats` | Counts by sector, storage size, encryption status |
| `/openmemory health` | Full health dashboard — diversity, at-risk, staleness |
| `/openmemory at-risk` | Memories near the purge threshold (salience ≤ 0.15) |
| `/openmemory dedup [thresh]` | Remove near-duplicate memories (default cosine: 0.92) |
| `/openmemory fact <s> <p> <o>` | Add a temporal fact; auto-closes previous same-key fact |
| `/openmemory facts` | Browse active and closed bi-temporal facts |
| `/openmemory decay` | Run exponential salience decay manually |
| `/openmemory consolidate` | Sleep-cycle: merge weak memories + generate reflections |
| `/openmemory reflect` | One-off reflective memory generation |
| `/openmemory summary` | User memory profile — top sectors, dominant tags |
| `/openmemory export` | Dump all memories as markdown |
| `/openmemory import [fmt] <file>` | Import from mem0 / Zep / native JSON (`auto` detects format) |
| `/openmemory ingest <file>` | Chunk & store document as cognitive memories (400-char chunks) |
| `/openmemory encrypt` | Enable AES-256-GCM encryption at rest |
| `/openmemory context [query]` | Preview the 4-layer context block the agent receives |
| `/openmemory layered [query]` | Alias for `context` with explicit layer labels |
| `/openmemory chunk <text\|file:path>` | Ingest text as verbatim 800-char chunks (MemPalace — no LLM summarisation) |
| `/openmemory drawers` | Verbatim drawer stats: total, Wing/Room distribution, dedup rate |
| `/openmemory tunnel <id1> <id2> [w]` | Create a bidirectional cross-project waypoint (weight 0–1) |
| `/openmemory auto-tunnel [thresh]` | Auto-detect similar memories across stores and create Tunnels |
| `/openmemory benchmark [k]` | LongMemEval recall@K benchmark (default k=5) |

### Code Intelligence

| Command | Description |
|---------|-------------|
| `/qa <question>` | Ask a question about the codebase (uses embeddings) |
| `/index [embedding-model]` | Build or rebuild codebase embeddings index |
| `/semindex <sub>` | Deep semantic codebase index: `build`, `query`, `callers`, `callees`, `hierarchy`, `deps`, `stats` |
| `/autofix [clippy\|eslint\|ruff\|gofmt\|prettier]` | Run linter auto-fix and show diff |
| `/notebook <file.vibe>` | Run interactive notebook cells |
| `/snippet list\|save\|use\|show\|delete` | Manage reusable code snippets |
| `/markers scan\|list\|bookmarks` | Scan TODO/FIXME/HACK markers and manage bookmarks |
| `/replay` | Code replay — step through agent changes |
| `/speculate` | Speculative execution — preview agent actions without applying |
| `/explain` | Explainable agent — show reasoning behind agent decisions |

### Project & Workspace

| Command | Description |
|---------|-------------|
| `/workspace-detect` | Detect workspace languages/frameworks and suggest skills |
| `/spec list\|show\|new\|run\|done` | Spec-driven development — manage feature specifications |
| `/recipe [list \| show <file>]` | List or inspect bundled/local recipes |
| `/theme [dark\|light\|monokai\|solarized\|nord]` | Switch TUI color theme |
| `/research <topic>` | Research a topic in the context of the codebase |
| `/search web\|cache\|providers\|config` | Web search grounding |
| `/websearch web\|search\|citations\|cache` | Integrated web search grounding with citations |

### Agents & Teams

| Command | Description |
|---------|-------------|
| `/agents list\|status\|new` | Manage background agent definitions |
| `/team create\|status\|messages\|show\|knowledge\|sync` | Agent teams and peer communication |
| `/worktree spawn\|list\|merge\|cleanup\|config` | Parallel worktree agent execution |
| `/host add\|list\|route\|remove\|ask` | Multi-agent terminal hosting |
| `/proactive scan\|config\|accept\|reject\|history\|digest` | Proactive agent intelligence |
| `/triage run\|rules\|labels\|history\|batch` | Autonomous issue triage |
| `/nexttask suggest\|accept\|reject\|learn\|stats` | Next-task prediction |
| `/a2a card\|serve\|discover\|call\|tasks\|status` | A2A agent-to-agent protocol |
| `/dispatch register\|unregister\|machines\|pair\|unpair\|devices\|send\|cancel\|status\|stats\|heartbeat` | Mobile gateway and dispatch |

### DevOps & Deployment

| Command | Description |
|---------|-------------|
| `/deploy <target>\|list` | Deploy to cloud platforms: `vercel`, `netlify`, `railway`, `github-pages`, `gcp`, `firebase`, `aws`, `aws-apprunner`, `aws-s3`, `aws-lambda`, `aws-ecs`, `azure`, `azure-appservice`, `azure-container`, `azure-static`, `digitalocean`, `kubernetes`, `helm`, `oci`, `ibm` |
| `/env list\|get\|set\|delete\|switch\|files\|create` | Environment variable management |
| `/profiler run\|top\|list-tools` | CPU/memory profiling |
| `/logs tail\|sources\|errors\|analyze` | Log viewer and analyzer |
| `/migration status\|migrate\|rollback\|generate` | Database migration management |
| `/mock start\|stop\|add\|remove\|list\|log\|import` | API mock server management |

### Security & Compliance

| Command | Description |
|---------|-------------|
| `/compliance soc2\|fedramp` | Generate compliance reports (SOC2/FedRAMP) |
| `/vulnscan <sub>` | Vulnerability scanner: `scan`, `deps`, `file`, `lockfile`, `sarif`, `report`, `summary`, `db-update`, `db-status`, `cache-clear` |
| `/verify full\|quick\|security\|performance\|testing` | Structured verification checklist |
| `/trust scores\|history\|config\|explain` | Agent trust scoring |
| `/vverify screenshot\|diff\|baseline\|ci` | Visual verification via screenshots |

### Git & Version Control

| Command | Description |
|---------|-------------|
| `/bisect start\|good\|bad\|skip\|reset\|log\|analyze` | Git bisect workflow with AI analysis |
| `/gitplatform add\|list\|remove\|default\|pr\|issue\|pipeline\|webhook` | Multi-platform Git management (GitHub, GitLab, Bitbucket, etc.) |

### Plugins & Extensions

| Command | Description |
|---------|-------------|
| `/plugin list\|install\|remove\|info` | Plugin management |
| `/marketplace` | Browse the plugin marketplace |
| `/skills import\|export\|search\|validate\|publish` | Cross-tool agent skills standard |
| `/resources status\|export\|verify\|path` | Manage externalized resource files |

### Scheduling & Reminders

| Command | Description |
|---------|-------------|
| `/remind in <dur> "task" \| list \| cancel <id>` | Set timed reminders |
| `/schedule every <dur> "task" \| list \| cancel <id>` | Schedule recurring tasks |

### Issue Tracking

| Command | Description |
|---------|-------------|
| `/linear list\|new\|open\|attach` | Linear issue tracker integration |

### Handoffs & Collaboration

| Command | Description |
|---------|-------------|
| `/handoff list\|show\|create` | Session handoff documents |

### Batch & QA

| Command | Description |
|---------|-------------|
| `/batch new\|start\|pause\|resume\|cancel\|status\|list\|estimate\|history` | Batch task processing |
| `/qavalidate run\|status\|report\|findings\|resolve\|config\|history` | QA validation engine |

### Legacy Migration

| Command | Description |
|---------|-------------|
| `/legacymigrate analyze\|plan\|translate\|validate\|report\|rules\|pairs` | Legacy code migration (analyze, translate, validate) |

### App Builder & Context

| Command | Description |
|---------|-------------|
| `/appbuilder enhance\|template\|provision\|scaffold\|templates` | App scaffolding and template management |
| `/icontext status\|expand\|compress\|refresh\|summary\|clear` | Intelligent context window management |

### Quantum Computing

| Command | Description |
|---------|-------------|
| `/quantum languages\|os\|hardware\|algorithms\|circuits\|projects\|create\|export\|compat\|status` | Quantum computing tools (circuits, algorithms, hardware info) |

### Autonomous Research

| Command | Description |
|---------|-------------|
| `/autoresearch new\|start\|stop\|pause\|status\|list\|analyze\|export\|suggest\|lessons\|config` | Autonomous iterative research agent |

### AI/ML & Routing

| Command | Description |
|---------|-------------|
| `/aiml` | AI/ML model training and evaluation tools |
| `/repair mcts\|agentless\|compare\|config` | MCTS-based code repair |
| `/route cost\|budget\|model\|stats\|compare` | Cost-optimized agent routing |
| `/rlcef train\|eval\|mistakes\|patterns\|reset\|export` | RLCEF (reinforcement learning from code execution feedback) training loop |
| `/langgraph serve\|connect\|status\|checkpoint` | LangGraph pipeline bridge |
| `/smartdeps resolve\|compare\|patch\|audit\|graph` | Agentic package manager with dependency resolution |

### Documentation & Integration

| Command | Description |
|---------|-------------|
| `/docsync status\|reconcile\|watch\|freshness` | Living documentation sync (detect stale docs) |
| `/connect list\|add\|test\|remove\|webhook` | Native integration connectors |
| `/analytics dashboard\|export\|roi\|compare` | Enterprise agent analytics |
| `/mcp-http serve\|oauth\|tokens\|remote` | MCP streamable HTTP transport with OAuth 2.1 |

### Sketch & Design

| Command | Description |
|---------|-------------|
| `/sketch new\|recognize\|generate\|export` | Sketch canvas to code (draw UI, generate components) |

### Company Management

| Command | Description |
|---------|-------------|
| `/company <sub>` | Company/org management: `create`, `list`, `switch`, `delete`, `status`, `agent`, `goal`, `task`, `approval`, `budget`, `secret`, `routine`, `doc`, `heartbeat`, `adapter`, `export`, `import` |

### Wizards

| Command | Description |
|---------|-------------|
| `/wizard` | Interactive guided setup wizards |

> **Safety**: By default, all shell command execution requires user confirmation (`y/N`). Disable this with `require_approval_for_commands = false` in config.

## Workflow Examples

### Chat with AI

```text
> /chat explain how the ropey crate works
> What are the time complexities for common rope operations?
```

Once a conversation is started, you can type freely without `/chat`.

### Generate Code

```text
> /generate a Rust function that parses TOML from a string and returns a HashMap
Save to file? (y/N or filename): parser.rs
Yes Saved to: parser.rs
```

### Apply AI Changes to a File

```text
> /apply src/main.rs add proper error handling using anyhow

Proposed changes:
--- a/src/main.rs
+++ b/src/main.rs
...

Yes Apply these changes? (y/N): y
Yes Changes applied to: src/main.rs
```

### AI-Suggested Command

```text
> /exec list all Rust files modified in the last 7 days
Suggested command: find . -name "*.rs" -mtime -7
Warning:  Execute this command? (y/N): y
```

### Project Initialization

Scan a project to auto-detect its type, build system, and test framework:

```text
> /init
Scanning project...

Project: my-web-app
Architecture: full-stack application
Languages: TypeScript, Rust
Frameworks: React, Vite, Axum, Tokio
Package managers: pnpm, cargo

Build commands:
  pnpm run build
  cargo build --workspace

Test commands:
  pnpm run test (Vitest)
  cargo test --workspace (cargo test)

Entry points: src/App.tsx, src/main.rs
Expected env vars: DATABASE_URL, API_KEY, JWT_SECRET

Yes Project profile cached to .vibecli/project-profile.json
   This context will be auto-injected into every agent session.
```

After running `/init`, the agent automatically understands your project structure in every conversation.

### Agent-Per-Branch Workflow

Run agent tasks in isolated git branches with automatic PR creation:

```text
> /branch-agent create add JWT auth middleware
Creating branch agent for: add JWT auth middleware
  1. Create a feature branch from current HEAD
  2. Run the agent autonomously on the branch
  3. Auto-commit changes with descriptive messages
  4. Push and create a PR on completion
```

### Always-On Channel Daemon

Start a persistent daemon that listens on Slack/Discord/GitHub and auto-responds:

```bash
# Start daemon (listens for messages, routes through automation rules)
vibecli --channel-daemon slack

# Or manage from the REPL
> /daemon start
> /daemon status
> /daemon channels
```

### Data Analysis

Load and query data files directly:

```text
> /data load sales.csv
Loading data from: sales.csv

> /data query "SELECT region, SUM(amount) FROM sales GROUP BY region"
> /data viz bar    # Generate bar chart
> /data summary    # Statistical summary
```

### Agentic CI/CD

Auto-fix failed builds and generate tests:

```text
> /agentic fix-build
Auto-fixing build failures...
  Reads CI logs, identifies errors, generates patches.

> /agentic gen-tests src/auth.rs
Generating tests for: src/auth.rs
```

## Git Context Awareness

VibeCLI automatically injects Git context into AI conversations:

- **Current branch** — detected from the working directory
- **Modified/staged files** — summarized from `git status`
- **Current diff** — full patch injected as system context

This gives the AI complete awareness of what you are working on without any extra prompting.

## Syntax Highlighting

Code blocks in AI responses are highlighted in the terminal using `syntect`. Supported languages include Rust, Python, TypeScript, JavaScript, Go, TOML, YAML, JSON, Markdown, and more.

## Configuration

VibeCLI reads from `~/.vibecli/config.toml`. See the [Configuration Guide](../configuration/) for full details.

**Minimal working config (Ollama):**

```toml
[ollama]
enabled = true
api_url = "http://localhost:11434"
model = "qwen2.5-coder:7b"
```

## CI / Non-Interactive Mode

VibeCLI can run agent tasks headlessly — no prompts, no TUI:

```bash
# Run an agent task and get a JSON report on stdout
vibecli --exec "add error handling to src/lib.rs" --full-auto

# Write a Markdown report to a file
vibecli --exec "fix all clippy warnings" --full-auto \
        --output-format markdown --output report.md

# Stream progress to stderr while writing JSON to stdout
vibecli --exec "add docstrings to all public functions" \
        --auto-edit --output-format verbose
```

**Exit codes:** `0` = success, `1` = partial, `2` = failed, `3` = approval required.

## @ Context Types

VibeCLI supports inline context injection using `@` references in chat messages:

| Reference | Description |
|-----------|-------------|
| `@file:<path>` | Contents of a specific file |
| `@file:<path>:N-M` | Specific line range from a file |
| `@folder:<path>` | Recursive directory tree listing |
| `@web:<url>` | Fetched & stripped plain text from a URL |
| `@docs:<lib>` | Library documentation (e.g. `@docs:tokio`, `@docs:npm:express`, `@docs:py:requests`) |
| `@git` | Current branch, changed files, and diff excerpt |
| `@terminal` | Last 200 lines of terminal output |
| `@symbol:<name>` | Source code for a named symbol (function, struct, etc.) |
| `@codebase:<query>` | Semantic search over the codebase |
| `@github:owner/repo#N` | Fetch a GitHub issue or PR (uses `GITHUB_TOKEN` env var) |
| `@jira:PROJECT-123` | Fetch a Jira issue (uses `JIRA_BASE_URL`, `JIRA_EMAIL`, `JIRA_API_TOKEN` env vars) |

Example:

```text
> /chat @jira:AUTH-456 explain this ticket and suggest an implementation plan
> /chat @github:torvalds/linux#1234 summarize this issue
```

## Multimodal Input (Vision)

Claude and GPT-4o providers support image attachments. Use `[path/to/image.png]` syntax in `/chat`:

```text
> /chat [screenshot.png] what error is shown in this screenshot?
> /chat [diagram.jpg] [schema.png] explain this database design
```

Images are base64-encoded and sent with the message. Non-vision providers fall back to text-only.

## Trace / Audit Log

Every agent session is recorded to `~/.vibecli/traces/<timestamp>.jsonl`. Browse with:

```
> /trace                   # list recent sessions
> /trace view 1740000000   # show detailed timeline
```

Each entry records: step, tool, input summary, output, duration, and approval source (`user` / `auto` / `ci-auto` / `rejected`).

## MCP Integration

[Model Context Protocol](https://modelcontextprotocol.io/) servers expose additional tools to the agent. Configure in `~/.vibecli/config.toml`:

```toml
[[mcp_servers]]
name = "github"
command = "npx"
args = ["@modelcontextprotocol/server-github"]

[[mcp_servers]]
name = "postgres"
command = "npx"
args = ["@modelcontextprotocol/server-postgres", "postgresql://localhost/mydb"]
```

Then in the REPL:

```
> /mcp list                # show configured servers
> /mcp tools github        # list tools from the github server
```

## Code Review Agent

Run structured code reviews from the CLI:

```bash
vibecli review                       # review uncommitted changes
vibecli --review --staged               # review staged changes only
vibecli --review --branch main..HEAD    # review branch diff
vibecli --review --pr 42                # review a GitHub PR
vibecli --review --focus security,perf  # limit review focus
```

Output is a structured `ReviewReport` with issues (severity: info/warning/critical), suggestions, and a numeric score.

Use `--pr` to post the review directly to a GitHub PR as a comment (via `gh` CLI).

## Server Mode (`vibecli serve`)

Run VibeCLI as a long-lived HTTP daemon for the VS Code extension and Agent SDK:

```bash
vibecli --serve --port 7878
```

**Security**: All authenticated routes require a `Bearer <token>` header and are rate-limited to 60 requests per 60 seconds. Request bodies are limited to 1 MB. Responses include CSP, X-Frame-Options, and other security headers.

Endpoints:

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/health` | No | Liveness check — returns `{"status":"ok"}` |
| POST | `/chat` | Yes | Single-turn chat, returns accumulated response |
| POST | `/chat/stream` | Yes | Streaming chat via Server-Sent Events |
| POST | `/agent` | Yes | Start an agent task, returns `{session_id}` |
| GET | `/stream/:session_id` | Yes | SSE stream of agent events in real-time |
| GET | `/jobs` | Yes | List all persisted background job records |
| GET | `/jobs/:id` | Yes | Get a single job record by ID |
| POST | `/jobs/:id/cancel` | Yes | Cancel a running background job |
| GET | `/sessions` | Yes | HTML index page of all agent sessions |
| GET | `/sessions.json` | Yes | JSON array of all session metadata |
| GET | `/view/:id` | Yes | Dark-mode HTML viewer for a session trace |
| GET | `/share/:id` | Yes | Shareable readonly session view (noindex) |
| GET | `/ws/collab/:room_id` | Token | WebSocket for CRDT collab (`?token=` query param) |
| POST | `/collab/rooms` | Yes | Create or get a collaboration room |
| GET | `/collab/rooms` | Yes | List all active collaboration rooms |
| GET | `/collab/rooms/:room_id/peers` | Yes | List peers in a room (names, cursor colors) |
| POST | `/acp/v1/tasks` | Yes | Create an ACP (Agent Communication Protocol) task |
| GET | `/acp/v1/tasks/:id` | Yes | Get an ACP task by ID |
| GET | `/acp/v1/capabilities` | No | List ACP server capabilities |
| POST | `/webhook/github` | No | GitHub App webhook receiver for CI review bot |
| POST | `/webhook/skill/:skill_name` | No | Trigger a skill by webhook name |
| GET | `/pair` | No | Device pairing endpoint — generates a one-time pairing URL |

## Counsel (Multi-LLM Deliberation)

Counsel enables structured multi-round debates between multiple AI providers, each assigned a distinct role. A moderator synthesizes the final consensus.

### Participant Roles

| Role | System Prompt Focus |
|------|-------------------|
| **Expert** | Deep domain expertise, comprehensive analysis |
| **Devil's Advocate** | Challenge assumptions, find weaknesses |
| **Skeptic** | Demand evidence, question feasibility |
| **Creative** | Novel approaches, outside-the-box thinking |
| **Pragmatist** | Practical implementation, real-world constraints |
| **Researcher** | Data-driven analysis, cite prior art |

### Usage

```text
> /counsel new "Should we migrate from REST to GraphQL?"
Session created: abc123

> /counsel run abc123
Round 1: Expert (Claude), Devil's Advocate (GPT-4o), Pragmatist (Gemini)
... responses from each participant ...

> /counsel inject abc123 "What about the migration cost for existing clients?"
> /counsel run abc123
Round 2: ... responses incorporating user feedback ...

> /counsel synthesize abc123
=== Moderator Synthesis ===
Consensus: ...
Disagreements: ...
Recommendations: ...
```

Sessions are persisted at `~/.vibecli/counsel/sessions.json`.

## SuperBrain (Multi-Provider Routing)

SuperBrain intelligently routes queries to the best provider or combines responses from multiple providers.

### Modes

| Mode | Description |
|------|-------------|
| **SmartRouter** | Keyword-based routing to the best model for the task type (code, math, creative, analysis, factual) |
| **Consensus** | All models respond; synthesizes the majority view |
| **ChainRelay** | Sequential refinement — each model builds on the previous response |
| **BestOfN** | All models respond; a judge picks the best |
| **Specialist** | Decomposes complex queries into subtasks assigned to different models |

### Usage

```text
> /superbrain "Write a binary search in Rust"
[SmartRouter → code category → Claude]
... response ...

> /superbrain consensus "What's the best database for time-series data?"
[Consensus → all providers → synthesis]
... combined response ...

> /superbrain chain "Design a microservices architecture for an e-commerce platform"
[ChainRelay → Model 1 → Model 2 → Model 3 (refined)]
... progressively refined response ...
```

## Web Client

VibeCLI can serve a self-contained browser-based UI when running in server mode. No CDN dependencies — fully air-gap safe.

```bash
vibecli --serve --port 7878
# Open http://localhost:7878 in your browser
```

Features:

- Chat and Agent modes with SSE streaming
- Markdown rendering with syntax highlighting
- Dark/light theme toggle
- File upload support (configurable)
- Responsive mobile-friendly design
- Keyboard shortcuts (Enter to send, Shift+Enter for newline)

## Hooks System

Hooks execute on agent events. Configure in `~/.vibecli/hooks.toml` or `.vibecli/hooks.toml`:

```toml
[[hooks]]
event = "PreToolUse"
pattern = "bash"
handler = { type = "command", command = "echo 'Running shell command' >> /tmp/hooks.log" }
```

Events: `SessionStart`, `PreToolUse`, `PostToolUse`, `Stop`, `TaskCompleted`, `SubagentStart`.

## Plan Mode

Generate execution plans without running tools:

```text
> /plan refactor the auth module to use JWT tokens

Execution Plan:
  1. Read current auth module (auth.rs)
  2. Add JWT dependency to Cargo.toml
  3. Implement JWT token generation
  4. Update auth middleware
  5. Write tests

Yes Execute this plan? (y/N):
```

## Code Complete Workflow

Guide application development through 8 structured stages inspired by Steve McConnell's *Code Complete*. Each stage has an AI-generated checklist that serves as a quality gate before advancing.

### The 8 Stages

| # | Stage | Focus |
|---|-------|-------|
| 1 | **Requirements** | User stories, functional/non-functional reqs, scope boundaries |
| 2 | **Architecture** | Subsystem decomposition, data design, security, build-vs-buy |
| 3 | **Design** | Classes, interfaces, patterns, algorithms, state management |
| 4 | **Construction Planning** | Tooling, coding standards, CI/CD, integration order, estimates |
| 5 | **Coding** | Implementation, defensive programming, naming, DRY, control flow |
| 6 | **Quality Assurance** | Code review, unit tests, static analysis, security scan |
| 7 | **Integration & Testing** | E2E tests, regression, performance, cross-platform |
| 8 | **Code Complete** | Docs, changelog, no TODOs, version tag, deploy runbook |

### Usage

```text
> /workflow new my_app Build a REST API for user management

Yes Workflow 'my_app' created with 8 stages (Code Complete methodology)
   Current stage: Requirements
   Use /workflow generate my_app to AI-generate a checklist for the current stage.

> /workflow generate my_app

Generating Requirements checklist for 'my_app'...
Yes Generated 10 checklist items:
   [ ] 1: Define core user CRUD endpoints (GET, POST, PUT, DELETE)
   [ ] 2: Specify authentication method (JWT vs session)
   ...

> /workflow check my_app 1
Yes Toggled item 1 in 'my_app'

> /workflow show my_app

Workflow: my_app  [12% complete]
   Build a REST API for user management

   ▶ 1. Requirements (1/10)
   ○ 2. Architecture
   ○ 3. Design
   ...

> /workflow advance my_app
Yes Advanced to stage: Architecture
```

Workflows are stored as markdown files in `.vibecli/workflows/` with YAML front-matter. The stage advancement gate requires ≥80% checklist completion in VibeUI.

## Red Team Security Testing

Run autonomous penetration tests against applications you build with VibeCody. The red team module executes a 5-stage pipeline: **Recon → Analysis → Exploitation → Validation → Report**.

### 5-Stage Pipeline

| # | Stage | Focus |
|---|-------|-------|
| 1 | **Recon** | Target discovery, tech fingerprinting, endpoint enumeration |
| 2 | **Analysis** | Source-code-aware vulnerability identification (white-box) |
| 3 | **Exploitation** | Active validation via HTTP requests + browser actions |
| 4 | **Validation** | Confirm exploitability, generate PoC payloads |
| 5 | **Report** | Structured findings with CVSS scores + remediation |

### Attack Vectors

15 built-in attack vectors covering OWASP Top 10: SQL injection, XSS, SSRF, IDOR, path traversal, auth bypass, command injection, mass assignment, open redirect, XXE, insecure deserialization, NoSQL injection, template injection, CSRF, and cleartext transmission.

### Usage

```bash
# Non-interactive scan (CI mode)
vibecli --redteam http://localhost:3000

# With auth config and scope restrictions
vibecli --redteam http://localhost:3000 --redteam-config auth.yaml

# Generate report from a previous session
vibecli --redteam-report <session-id>
```

```bash
# Interactive REPL
> /redteam scan http://localhost:3000
> /redteam list
> /redteam show <session-id>
> /redteam report <session-id>
> /redteam config
```

Sessions are persisted as JSON at `~/.vibecli/redteam/`. Findings include CVSS severity scores, PoC payloads, and remediation guidance.

> **Authorization**: Red team features require explicit user consent and target only user-controlled applications (localhost / staging).

## Test Runner

Run project tests directly from the REPL with auto-detection of the test framework:

```text
> /test

Running: cargo test
...
Yes Tests passed

> /test npm test -- --coverage

Running: npm test -- --coverage
...
Yes Tests passed
```

### Framework Auto-Detection

| File Detected | Command Used |
|---------------|-------------|
| `Cargo.toml` | `cargo test` |
| `package.json` | `npm test` |
| `pytest.ini` / `pyproject.toml` / `setup.py` | `python -m pytest -v` |
| `go.mod` | `go test ./...` |

If no framework is detected, provide a custom command: `/test <command>`.

In VibeUI, the **Tests** panel provides a richer experience with live streaming log output, per-test pass/fail results, expandable failure details, filter tabs (All / Failed / Passed), and a custom command input field.

## Skills System

VibeCody ships with **568 skill files** across 25+ categories covering finance, healthcare, security, cloud (AWS/Azure/GCP), data engineering, robotics, compliance, SRE, and more. Skills activate based on trigger keywords. Place custom `.md` files in `.vibecli/skills/` or `~/.vibecli/skills/`:

```markdown
name: rust-testing
triggers: [test, testing, cargo test]
category: rust
tools_allowed: [read_file, write_file, bash]
Use `#[tokio::test]` for async tests...
```

## Gateway Messaging

VibeCLI can act as a bot on 18 messaging platforms via the gateway system:

Telegram, Discord, Slack, Signal, Matrix, Twilio SMS, iMessage, WhatsApp, Teams, IRC, Twitch, WebChat, Nostr, QQ, Tlon (+ 3 original).

Configure gateways in `~/.vibecli/config.toml`:

```toml
[[gateway]]
platform = "telegram"
bot_token = "..."
whitelist = ["@username"]
```

## Session Resume

Resume a previous agent session:

```bash
vibecli --resume 1740000000
```

Restores the full message history, context, and trace from the JSONL log.

## Admin Policy

Workspace administrators can restrict agent behavior via `.vibecli/policy.toml`:

```toml
[tools]
deny = ["bash"]
require_approval = ["write_file", "patch_file"]

[paths]
deny = ["*.env", "secrets/**"]

[limits]
max_steps = 10
```

## OpenTelemetry

Export agent tracing spans to any OTLP collector:

```toml
[otel]
enabled = true
endpoint = "http://localhost:4318"
service_name = "vibecli"
```

Spans include session ID, task, tool name, and step metadata.

## Project Structure

```
vibecli/
└── vibecli-cli/
    ├── skills/             # 568 skill files (25+ categories)
    └── src/
        ├── main.rs         # CLI argument parsing, command dispatch
        ├── config.rs       # Config loading/saving (TOML)
        ├── ci.rs           # Non-interactive CI mode (--exec)
        ├── review.rs       # Code review agent (vibecli review)
        ├── serve.rs        # HTTP daemon (vibecli serve)
        ├── otel_init.rs    # OpenTelemetry pipeline setup
        ├── diff_viewer.rs  # Renders unified diffs in terminal
        ├── syntax.rs       # Syntax highlighting for code blocks
        ├── repl.rs         # Rustyline helper (tab completion, hints)
        ├── redteam.rs      # Red team 5-stage pentest pipeline
        ├── bugbot.rs       # OWASP/CWE static scanner (15 patterns)
        ├── workflow.rs     # Code Complete 8-stage workflow
        ├── workflow_orchestration.rs  # Lessons + todo orchestration
        ├── gateway.rs      # 18-platform messaging gateway
        ├── transform.rs    # Code transforms (Python 2→3, Vue 2→3, etc.)
        ├── marketplace.rs  # Plugin marketplace
        ├── background_agents.rs  # Background agent definitions
        ├── session_store.rs     # SQLite session persistence
        ├── sandbox.rs      # Container sandbox (Docker/Podman)
        ├── voice.rs        # Voice input (Groq Whisper)
        ├── pairing.rs      # QR code device pairing
        ├── tailscale.rs    # Tailscale funnel
        ├── discovery.rs    # mDNS service discovery
        ├── scheduler.rs    # /remind and /schedule commands
        ├── counsel.rs      # Multi-LLM deliberation engine
        ├── superbrain.rs   # Multi-provider query routing and synthesis
        ├── web_client.rs   # Browser-based zero-install web client
        └── tui/
            ├── mod.rs      # TUI run loop and event handling
            ├── app.rs      # TUI application state machine
            ├── ui.rs       # Ratatui layout and widget rendering
            └── components/
                ├── diff_view.rs  # Multi-file diff viewer widget
                ├── vim_editor.rs # Vim-style modal editor (Normal/Insert/Visual/Command)
                └── diagnostics.rs # Cargo/eslint diagnostics panel
```

## Dependencies

| Crate | Purpose |
|-------|---------|
| `clap` | CLI argument parsing |
| `ratatui` + `crossterm` | Terminal UI framework |
| `rustyline` | REPL readline with history |
| `syntect` | Syntax highlighting |
| `tokio` | Async runtime |
| `axum` + `tower-http` | HTTP server (serve mode) |
| `opentelemetry` + `tracing-opentelemetry` | OTLP tracing |
| `vibe-ai` | AI provider, agent, hooks, skills, artifacts |
| `vibe-core` | Git, diff, file utilities, codebase indexing |
| `vibe-collab` | CRDT multiplayer collaboration (yrs + DashMap + Axum WS) |
