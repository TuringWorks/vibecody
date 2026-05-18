# VibeCody Feature Matrix

> **At-a-glance reference** for every capability across VibeCLI (terminal) and VibeUI (desktop editor).
> ✅ = available · ⚙️ = configurable/optional · 🔬 = experimental · ❌ = not available

---

## AI Providers

| Provider | VibeCLI | VibeUI | Notes |
|---|:---:|:---:|---|
| Anthropic Claude | ✅ | ✅ | claude-3-5-sonnet, claude-opus-4, etc. |
| OpenAI GPT-4 / GPT-4o | ✅ | ✅ | All model tiers |
| Google Gemini | ✅ | ✅ | Gemini 1.5 Pro/Flash, 2.0 |
| Ollama (local) | ✅ | ✅ | Any Ollama-served model, auto-detect |
| AWS Bedrock | ✅ | ✅ | Claude, Titan, Llama via Bedrock API |
| Azure OpenAI | ✅ | ✅ | Custom deployment endpoint |
| Groq | ✅ | ✅ | Ultra-fast inference |
| Grok (X.ai) | ✅ | ✅ | |
| Mistral AI | ✅ | ✅ | |
| DeepSeek | ✅ | ✅ | Code-focused |
| Cerebras | ✅ | ✅ | Fast inference |
| Perplexity | ✅ | ✅ | Search-augmented |
| Together AI | ✅ | ✅ | Open model hosting |
| Fireworks AI | ✅ | ✅ | |
| SambaNova | ✅ | ✅ | |
| OpenRouter | ✅ | ✅ | 300+ models via single key |
| Zhipu GLM | ✅ | ✅ | Chinese market |
| MiniMax | ✅ | ✅ | Chinese market |
| Vercel AI Gateway | ✅ | ✅ | Unified proxy |
| GitHub Copilot | ✅ | ✅ | Device-flow auth |
| Provider failover chain | ✅ | ✅ | Auto-retry on next provider |
| Provider health tracking | ✅ | ✅ | ResilientProvider wrapper |
| Per-tab provider override | ❌ | ✅ | VibeUI chat tab selector |
| Cost-optimized routing | ✅ | ⚙️ | `/route` command |

---

## Chat & Conversation

| Feature | VibeCLI | VibeUI | Notes |
|---|:---:|:---:|---|
| Streaming responses | ✅ | ✅ | Token-by-token streaming |
| Multi-turn conversation | ✅ | ✅ | Full history context |
| Chat tabs (parallel sessions) | ❌ | ✅ | Multiple tabs, each with own state |
| Inline tab rename | ❌ | ✅ | Double-click to rename |
| Session history browser | ✅ | ✅ | Browse + restore past sessions |
| Auto-save on tab close | ❌ | ✅ | Persisted to localStorage |
| Conversation auto-compaction | ✅ | ✅ | Triggers at ~80k chars |
| Manual compaction (`/compact`) | ✅ | ✅ | Summarize + truncate old messages |
| Chat memory panel | ❌ | ✅ | Extracted facts + pin to prompt |
| Pinned facts injected into prompt | ❌ | ✅ | Persist across sessions |
| Voice input | ✅ | ✅ | Web Speech API + Groq Whisper |
| Image/file attachments | ✅ | ✅ | Up to 10 files, 20 MB each |
| Slash commands | ✅ | ✅ | `/fix`, `/explain`, `/test`, etc. |
| @ file mentions | ✅ | ✅ | Add file content to context |
| Syntax-highlighted code blocks | ✅ | ✅ | |
| Message retry | ✅ | ✅ | Resend last user message |
| Stop streaming | ✅ | ✅ | Cancel in-flight response |
| Token / speed metrics | ✅ | ✅ | Tokens/sec display |
| Thinking blocks (extended thinking) | ✅ | ✅ | Collapsible `<thinking>` UI |
| Context from open file | ✅ | ✅ | Current file auto-injected |
| Context from workspace rules | ✅ | ✅ | `.vibeui.md` / `.vibecli/rules/` |

---

## Agent Capabilities

| Feature | VibeCLI | VibeUI | Notes |
|---|:---:|:---:|---|
| Autonomous agent loop | ✅ | ✅ | Plan → Act → Observe |
| Planning mode | ✅ | ✅ | Generates plan before execution |
| Chat-only mode | ✅ | ✅ | No tool calls |
| Suggest mode (approve each tool) | ✅ | ✅ | Manual approval per action |
| Auto-edit mode | ✅ | ✅ | Auto-apply files, ask for shell |
| Full-auto mode | ✅ | ✅ | Execute all without prompting |
| Sub-agent spawning | ✅ | ✅ | `spawn_agent` tool |
| Multi-agent teams | ✅ | ⚙️ | `/team` command |
| Agent-to-agent (A2A) protocol | ✅ | 🔬 | |
| Parallel agent execution | ✅ | ❌ | `--parallel N` |
| Background agents | ✅ | ❌ | `/agents` to manage |
| Agent trust scoring | ✅ | ⚙️ | |
| Worktree isolation | ✅ | ❌ | `--worktree` |
| CI/exec mode (non-interactive) | ✅ | ❌ | `--exec` |

**Agent Tools:**

| Tool | Available |
|---|:---:|
| `read_file` | ✅ |
| `write_file` | ✅ |
| `apply_patch` | ✅ |
| `list_directory` | ✅ |
| `bash` (shell execution) | ✅ |
| `search_files` (regex) | ✅ |
| `web_search` | ✅ |
| `fetch_url` | ✅ |
| `spawn_agent` | ✅ |
| `think` (internal reasoning) | ✅ |

---

## Code Editing (VibeUI)

| Feature | Available | Notes |
|---|:---:|---|
| Monaco editor | ✅ | VS Code engine |
| 100+ language syntax highlighting | ✅ | |
| LSP-driven code completion | ✅ | |
| Multi-file tabs | ✅ | Unsaved indicators |
| Minimap navigation | ✅ | |
| Code folding | ✅ | |
| Find & replace (regex) | ✅ | |
| Go to definition | ✅ | Via LSP |
| Inline diagnostics | ✅ | Real-time errors |
| Diff review panel | ✅ | Per-hunk accept/reject |
| Undo strip (30-second post-apply) | ✅ | Revert last AI apply |
| Auto-format on save | ✅ | |
| File type detection | ✅ | |
| Image preview | ✅ | |

---

## Context Management

| Feature | VibeCLI | VibeUI | Notes |
|---|:---:|:---:|---|
| File @ mention | ✅ | ✅ | |
| Context picker (visual) | ❌ | ✅ | |
| Context bundles (named sets) | ✅ | ✅ | Save/share context configs |
| Infinite context mode | ✅ | ⚙️ | 5-level hierarchical compression |
| Sliding window eviction | ✅ | ✅ | LRU / hybrid strategy |
| Auto-summarise old context | ✅ | ✅ | After compaction threshold |
| Workspace rules injection | ✅ | ✅ | `.vibehints`, `rules/*.md` |
| `.vibeui.md` workspace rules | ❌ | ✅ | Injected into every AI prompt |
| Semantic index (fast search) | ✅ | ✅ | Trigram + LRU cache |
| Hierarchical project memory | ✅ | ✅ | system → user → project → dir |
| Session memory (auto-extracted) | ✅ | ✅ | Facts from assistant messages |
| Pinned memory in system prompt | ❌ | ✅ | ChatMemoryPanel |

---

## Code Review & Analysis

| Feature | VibeCLI | VibeUI | Notes |
|---|:---:|:---:|---|
| AI code review | ✅ | ✅ | 7 detectors (security, complexity, style, docs, tests, duplication, architecture) |
| Security / OWASP scan | ✅ | ✅ | |
| Complexity analysis | ✅ | ✅ | |
| Duplication detection | ✅ | ✅ | |
| Quality gates (pass/fail threshold) | ✅ | ✅ | |
| Mermaid diagram generation | ✅ | ✅ | |
| PR summary generation | ✅ | ⚙️ | |
| Post review to GitHub PR | ✅ | ❌ | `--post-github` |
| Architecture spec (TOGAF, C4, ADR) | ✅ | ✅ | |
| Dependency analysis | ✅ | ✅ | |
| Self-review mode | ✅ | ✅ | |
| Review protocol enforcement | ✅ | ✅ | |

---

## Testing

| Feature | VibeCLI | VibeUI | Notes |
|---|:---:|:---:|---|
| Auto-detect test framework | ✅ | ✅ | Cargo, Jest, pytest, Go test |
| Test runner execution | ✅ | ✅ | |
| Coverage collection | ✅ | ✅ | Line + branch |
| Coverage visualization | ❌ | ✅ | Panel with trend charts |
| Load testing | ❌ | ✅ | LoadTestPanel |
| Visual regression testing | ❌ | ✅ | VisualTestPanel |
| QA validation workflow | ❌ | ✅ | QaValidationPanel |

---

## Git Integration

| Feature | VibeCLI | VibeUI | Notes |
|---|:---:|:---:|---|
| Diff viewing | ✅ | ✅ | |
| AI commit message generation | ✅ | ✅ | |
| Branch creation/switching | ✅ | ✅ | |
| PR creation (GitHub/GitLab/Azure) | ✅ | ⚙️ | |
| Git blame | ✅ | ✅ | |
| Git bisect workflow | ✅ | ❌ | `/bisect` |
| Merge conflict resolution | ✅ | ✅ | |
| Rebase assistance | ✅ | ✅ | |
| Stash management | ✅ | ✅ | |
| Git history viewer | ✅ | ✅ | |
| Tag management | ✅ | ✅ | |
| Worktree isolation | ✅ | ❌ | |

---

## Session Management

| Feature | VibeCLI | VibeUI | Notes |
|---|:---:|:---:|---|
| Session persistence (SQLite) | ✅ | ✅ | `~/.vibecli/sessions.db` |
| Resume session (`--resume`) | ✅ | ✅ | |
| Fork session (`--fork`) | ✅ | ❌ | Creates child session |
| Export session (`--export-session`) | ✅ | ⚙️ | Markdown / JSON / HTML |
| Session sharing (URL) | ✅ | ⚙️ | |
| Session search | ✅ | ✅ | |
| Checkpoint / rewind | ✅ | ❌ | `/rewind` |
| Trace inspection | ✅ | ⚙️ | JSONL + `-messages.json` sidecars |

---

## Goals — Durable Execution Intent

| Feature | VibeCLI | VibeUI | Notes |
|---|:---:|:---:|---|
| Persistent goal record (intent + statement + criteria) | ✅ | ✅ | `goals` + `goal_links` in `~/.vibecli/sessions.db` |
| Lifecycle: Active / Paused / Done / Abandoned | ✅ | ✅ | `/goal status <id> <s>` |
| `ExecutionPlan` decomposition (PlannerAgent) | ✅ | ✅ | `POST /v1/goals/:id/plan`; per-request `{provider, model}` override |
| Link graph (sessions / jobs / recaps / notes) | ✅ | ✅ | `/goal link` and panel "Linked sessions" |
| Aggregate recap (LLM synthesis + heuristic fallback) | ✅ | ✅ | `POST /v1/goals/:id/recap`; response carries `recap_synthesizer` |
| Hierarchy (parent / children / reparent) | ✅ | ✅ | `/goal children`, `/goal reparent`; tree-view toggle |
| Recursive subtree walk (cycle-safe, depth-clamped) | ✅ | ✅ | `GET /v1/goals/:id/tree?depth=N` (1..10) |
| Per-workspace "current pin" + global slot | ✅ | ✅ | `GET/PUT/DELETE /v1/goals/current` |
| `/agent` auto-link to pinned goal | ✅ | ✅ | Silent best-effort |
| Read-only TUI Goals screen | ✅ | n/a | `/goal` from chat opens; `f` cycles filter |
| Slash hybrid in chat input | n/a | ✅ | AIChat `/goal <text>` opens panel + seeds modal |
| REPL subcommands | ✅ | n/a | `new`, `list`, `show`, `status`, `link`, `start`, `children`, `reparent`, `pin`, `unpin`, `current`, `delete`, `plan` |
| Mobile remote control (Flutter) | ✅ | n/a | `listGoals`, `getGoal`, `startGoal`, `getGoalTree`, `getCurrentGoal`, `pinGoal`, `unpinGoal` |
| Apple Watch (curated `/watch/goals`) | ✅ | n/a | `loadGoals`, `fetchGoal`, `startGoal` |
| Wear OS (curated `/watch/goals`) | ✅ | n/a | `listGoals`, `getGoal`, `startGoal`; `GoalDetailScreen` + `GoalsTileService` Tile |
| VS Code sidebar tree-view | ✅ | n/a | `vibecli.goalsView` (`goals-tree.ts`) with refresh + context-menu actions |
| Agent SDK namespace | ✅ | n/a | `agent.goals.{list,get,create,update,delete,plan,start,link,tree,pin,unpin,current,recap}` |

---

## Terminal & Shell

| Feature | VibeCLI | VibeUI | Notes |
|---|:---:|:---:|---|
| Full terminal emulator | ✅ | ✅ | xterm.js in VibeUI |
| Multiple terminal tabs | ❌ | ✅ | VibeUI only |
| Shell completions | ✅ | ❌ | bash/zsh/fish/powershell/elvish |
| Command history | ✅ | ✅ | |
| Shell aliases | ✅ | ❌ | |
| TUI mode (Ratatui) | ✅ | ❌ | VibeCLI only |

---

## Security & Sandbox

| Feature | VibeCLI | VibeUI | Notes |
|---|:---:|:---:|---|
| OS-level sandbox | ✅ | ❌ | sandbox-exec (macOS), bwrap (Linux) |
| Network isolation | ✅ | ❌ | `--no-network` |
| Container isolation (Docker/Podman) | ✅ | ⚙️ | |
| Worktree isolation | ✅ | ❌ | Separate git worktree |
| Policy engine (RBAC/ABAC/CEL) | ✅ | ✅ | 14 condition operators |
| Per-tool approval | ✅ | ✅ | |
| Secrets scanning | ✅ | ❌ | API key monitor |
| Red team scanning | ✅ | ✅ | `/redteam` |
| Blue team (defensive) | ✅ | ✅ | `/blueteam` |
| Purple team (ATT&CK) | ✅ | ✅ | `/purpleteam` |
| Vulnerability scanning | ✅ | ✅ | `/vulnscan` |
| SBOM generation | ✅ | ❌ | |
| SOC2 compliance report | ✅ | ✅ | |
| FedRAMP checklist | ✅ | ❌ | |
| Audit trail logging | ✅ | ✅ | |

---

## Memory System

| Feature | VibeCLI | VibeUI | Notes |
|---|:---:|:---:|---|
| Auto-memory recording | ✅ | ✅ | Facts extracted post-session |
| Project memory files | ✅ | ✅ | `.vibecli/memory.md` |
| Memory edit (`/memory`) | ✅ | ✅ | |
| Open Memory (cognitive engine) | ✅ | ✅ | Semantic search, decay, encryption |
| Chat memory panel | ❌ | ✅ | Per-tab extracted facts |
| Pin facts to system prompt | ❌ | ✅ | Persists to localStorage |
| **VibeMemory (SQLite vector store)** | ✅ | ✅ | Per-project + per-machine stores, sector classification |
| VibeMemory `/vibememory/*` API | ✅ | ✅ | Store, search, context, consolidate endpoints |
| VibeMemory Tauri commands | ❌ | ✅ | `vibememory_store`, `vibememory_search`, etc. |
| Workspace hints (`.vibehints`) | ✅ | ✅ | Always-active context |
| Rules directory (`.vibecli/rules/`) | ✅ | ✅ | Path-gated context injection |

---

## MCP (Model Context Protocol)

| Feature | Available | Notes |
|---|:---:|---|
| MCP server mode (`--mcp-server`) | ✅ | stdio JSON-RPC 2.0 |
| `read_file` tool | ✅ | |
| `write_file` tool | ✅ | |
| `bash` tool | ✅ | |
| `search_files` tool | ✅ | |
| `agent_run` tool | ✅ | |
| GitHub MCP server | ✅ | |
| Linear MCP server | ✅ | |
| Custom MCP server support | ✅ | |
| Streamable responses | ✅ | |
| Multi-server support | ✅ | |

---

## Recipes & Automation

| Feature | Available | Notes |
|---|:---:|---|
| YAML recipe format | ✅ | |
| Variable substitution (`{{ var }}`) | ✅ | |
| Dry-run mode | ✅ | `--dry-run` |
| Interactive param prompting | ✅ | |
| Multi-step recipes | ✅ | |
| Per-step provider override | ✅ | |
| Recipe library (bundled) | ✅ | |
| Scheduled recipes (`/schedule`) | ✅ | |

---

## Observability & Cost

| Feature | VibeCLI | VibeUI | Notes |
|---|:---:|:---:|---|
| Token counting per message | ✅ | ✅ | |
| Session cost estimation | ✅ | ✅ | `/cost` |
| Cost budget + alerts | ✅ | ✅ | |
| Cost by provider | ✅ | ✅ | |
| OpenTelemetry traces | ✅ | ❌ | OTLP/HTTP export |
| Execution traces (JSONL) | ✅ | ⚙️ | |
| Log aggregation | ✅ | ✅ | |
| Health check (`--doctor`) | ✅ | ✅ | |
| Session memory profiling | ✅ | ✅ | Leak detection, auto-compaction |
| Enterprise analytics dashboard | ✅ | ✅ | |

---

## LSP & Language Support

| Language | Status |
|---|:---:|
| Rust (rust-analyzer) | ✅ |
| TypeScript / JavaScript | ✅ |
| Python (pyright) | ✅ |
| Go (gopls) | ✅ |
| C / C++ (clangd) | ✅ |
| Java (jdtls) | ✅ |
| JSON / YAML / TOML | ✅ |
| HTML / CSS / SCSS | ✅ |
| Markdown | ✅ |
| SQL | ✅ |
| Vue / Svelte | ✅ |
| GraphQL | ✅ |
| Custom LSP (via config) | ✅ |

**LSP Features:** go-to-definition · find-references · hover · rename · code actions · diagnostics · call hierarchy · workspace symbols

---

## Plugins & Extensions

| Feature | Available | Notes |
|---|:---:|---|
| WASM plugin system | ✅ | |
| Custom REPL commands | ✅ | |
| Custom LSP plugins | ✅ | |
| Tool integrations (Jira, Linear, GitHub) | ✅ | |
| Hot reload (dev mode) | ✅ | |
| Plugin versioning | ✅ | |
| Python SDK bindings | ✅ | |

---

## Collaboration

| Feature | VibeCLI | VibeUI | Notes |
|---|:---:|:---:|---|
| CRDT multiplayer editing | 🔬 | ✅ | Conflict-free real-time |
| Presence awareness | ❌ | ✅ | Cursors, selections |
| Session sharing | ✅ | ✅ | Export or URL |
| Handoff documents | ✅ | ❌ | `/handoff` |
| Code snippets | ✅ | ✅ | `/snippet` |
| Agent team collaboration | ✅ | ⚙️ | Multi-agent with shared knowledge |

---

## Deployment & Infrastructure

| Feature | Available | Notes |
|---|:---:|---|
| Docker execution | ✅ | |
| Podman support | ✅ | |
| Kubernetes (K8s) | ✅ | |
| Terraform | ✅ | |
| AWS (EC2, Lambda, ECS, Bedrock) | ✅ | |
| Azure (VM, ACI, Functions, OpenAI) | ✅ | |
| GCP (Compute, Cloud Run) | ✅ | |
| Vercel | ✅ | |
| DigitalOcean | ✅ | |
| Blue/green deployment | ✅ | |
| Canary deployment | ✅ | |
| Auto-rollback | ✅ | |

---

## Daemon & API Mode

| Feature | Available | Notes |
|---|:---:|---|
| HTTP daemon (`--serve`) | ✅ | Port 7878 |
| Server-Sent Events (SSE) | ✅ | Streaming |
| `POST /api/chat` | ✅ | |
| `POST /api/agent` | ✅ | |
| `GET /api/sessions` | ✅ | |
| Tailscale Funnel (public HTTPS) | ✅ | `--tailscale` |
| Diagnostics bundle (`--diagnostics`) | ✅ | |

---

## Platform Support

| Platform | VibeCLI | VibeUI |
|---|:---:|:---:|
| macOS (Intel + Apple Silicon) | ✅ | ✅ |
| Linux (Ubuntu, Fedora, Arch, etc.) | ✅ | ✅ |
| Windows 10/11 | ✅ | ✅ |
| Docker / OCI container | ✅ | ❌ |
| ARM / Raspberry Pi | ✅ | ❌ |

**Installation:** binary · `cargo install` · Homebrew · DEB/RPM/APK · Docker · setup wizard

---

## REPL Command Categories (100+ total)

| Category | Commands |
|---|---|
| **Core** | `/chat`, `/agent`, `/plan`, `/exec`, `/help`, `/exit` |
| **Code** | `/fix`, `/explain`, `/test`, `/doc`, `/refactor`, `/review`, `/compact` |
| **Project** | `/deploy`, `/deps`, `/env`, `/spec`, `/autofix`, `/appbuilder` |
| **Analysis** | `/qa`, `/semindex`, `/search`, `/websearch`, `/research`, `/autoresearch` |
| **Sessions** | `/sessions`, `/share`, `/fork`, `/rewind`, `/snapshot`, `/trace` |
| **Memory** | `/memory`, `/openmemory`, `/bundle` |
| **Automation** | `/recipe`, `/workflow`, `/schedule`, `/remind`, `/notebook` |
| **Teams** | `/team`, `/agents`, `/a2a`, `/host`, `/dispatch` |
| **Security** | `/redteam`, `/blueteam`, `/purpleteam`, `/vulnscan`, `/compliance` |
| **Infra** | `/sandbox`, `/docker`, `/container`, `/cloud`, `/vm` |
| **Integrations** | `/linear`, `/mcp`, `/skills`, `/connect` |
| **Advanced** | `/arena`, `/profiler`, `/bisect`, `/repair`, `/arena`, `/voice` |
| **System** | `/cost`, `/config`, `/doctor`, `/status`, `/theme`, `/wizard` |

---

*Last updated: 2026-04-05 · See [FEATURE-REFERENCE.md](FEATURE-REFERENCE.md) for deep-dive per-feature documentation.*
