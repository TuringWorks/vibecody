# VibeCody Feature Matrix

> **At-a-glance reference** for every capability across VibeCLI (terminal) and VibeCoder (desktop editor).
> ✅ = available · ⚙️ = configurable/optional · 🔬 = experimental · ❌ = not available

---

## AI Providers

**25 providers.** Model lists and per-provider defaults live in one file — `vibecoder/src/hooks/useModelRegistry.ts` (see [CLAUDE.md → Adding / updating providers and models](../CLAUDE.md)). Every panel that calls an LLM must honour the toolbar's provider/model selection; **no panel may hard-code a vendor** ([AGENTS.md → Provider-Agnostic Panels — STRICT](../AGENTS.md)).

| Provider | VibeCLI | VibeCoder | Notes (models current as of 2026-07-30) |
|---|:---:|:---:|---|
| Anthropic Claude | ✅ | ✅ | Opus 5, Sonnet 5 (1M ctx), Opus 4.8, Fable 5 |
| Claude Code (subscription auth) | ✅ | ✅ | Uses an existing Claude Code seat instead of an API key |
| OpenAI | ✅ | ✅ | GPT-5.6 Sol / Terra / Luna, GPT-5.5, GPT-5.3-Codex |
| Google Gemini | ✅ | ✅ | Gemini 3.6 Flash, 3.5 Flash / Flash-Lite, 3.1 Pro |
| Ollama (local + Cloud/Turbo) | ✅ | ✅ | Any Ollama-served model, auto-detect; Cloud models (`*-cloud`) via bearer token |
| mistral.rs (in-process local) | ✅ | ✅ | GGUF / quantised local inference — no server required |
| AWS Bedrock | ✅ | ✅ | Claude, Titan, Llama via Bedrock API + SigV4 |
| Azure OpenAI | ✅ | ✅ | Custom deployment endpoint |
| Groq | ✅ | ✅ | Ultra-fast inference |
| Grok (xAI) | ✅ | ✅ | Grok 4.5 — 500K ctx, $2/$6 |
| Mistral AI | ✅ | ✅ | Codestral for code |
| DeepSeek | ✅ | ✅ | V4 / V4-Flash (MIT open weights) |
| Moonshot (Kimi) | ⚙️ | ⚙️ | K3 (2.8T MoE, 1M ctx) / K2.7-Code — via OpenRouter today; native provider pending ᴬ |
| Zhipu GLM | ✅ | ✅ | GLM-5.2 (744B) |
| MiniMax | ✅ | ✅ | MiniMax-M3 |
| Qwen (via OpenRouter / Ollama) | ✅ | ✅ | Qwen 3.6-Coder (Apache 2.0) |
| Cerebras | ✅ | ✅ | Fast inference |
| Perplexity | ✅ | ✅ | Search-augmented |
| Together AI | ✅ | ✅ | Open model hosting |
| Fireworks AI | ✅ | ✅ | |
| SambaNova | ✅ | ✅ | |
| Poolside AI | ✅ | ✅ | Laguna S 2.1 / XS 2.1 / M.1 coding models |
| OpenRouter | ✅ | ✅ | 300+ models via a single key |
| Vercel AI Gateway | ✅ | ✅ | Unified proxy |
| GitHub Copilot | ✅ | ✅ | Device-flow auth |
| Provider failover chain | ✅ | ✅ | Auto-retry on the next provider |
| Provider health tracking | ✅ | ✅ | `ResilientProvider` wrapper |
| Per-tab provider override | ❌ | ✅ | VibeCoder chat-tab selector |
| Per-request effort knob | ✅ | ✅ | `low\|medium\|high\|xhigh` → Claude/Gemini thinking budget, OpenAI `reasoning_effort` |
| Cost-optimized routing (heuristic) | ✅ | ⚙️ | `/route` + `cost_router.rs` |

ᴬ **Registry append pending.** The provider integration works; these specific new model IDs are not yet listed in `useModelRegistry.ts`.

> ⚠ **Known issue:** the `gemini` provider default currently points at an unreleased model ID and will be corrected to `gemini-3.6-flash`.

---

## Chat & Conversation

| Feature | VibeCLI | VibeCoder | Notes |
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
| Voice input | ✅ | ✅ | Shared hook → daemon `/voice/transcribe`; see [Voice Input](#voice-input) for every client |
| Image/file attachments | ✅ | ✅ | Up to 10 files, 20 MB each |
| Slash commands | ✅ | ✅ | `/fix`, `/explain`, `/test`, etc. |
| @ file mentions | ✅ | ✅ | Add file content to context |
| Syntax-highlighted code blocks | ✅ | ✅ | |
| Message retry | ✅ | ✅ | Resend last user message |
| Stop streaming | ✅ | ✅ | Cancel in-flight response |
| Token / speed metrics | ✅ | ✅ | Tokens/sec display |
| Thinking blocks (extended thinking) | ✅ | ✅ | Collapsible `<thinking>` UI |
| Context from open file | ✅ | ✅ | Current file auto-injected |
| Context from workspace rules | ✅ | ✅ | `.vibecoder.md` / `.vibecli/rules/` |

---

## Agent Capabilities

| Feature | VibeCLI | VibeCoder | Notes |
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

## Code Editing (VibeCoder)

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

| Feature | VibeCLI | VibeCoder | Notes |
|---|:---:|:---:|---|
| File @ mention | ✅ | ✅ | |
| Context picker (visual) | ❌ | ✅ | |
| Context bundles (named sets) | ✅ | ✅ | Save/share context configs |
| Infinite context mode | ✅ | ⚙️ | 5-level hierarchical compression |
| Sliding window eviction | ✅ | ✅ | LRU / hybrid strategy |
| Auto-summarise old context | ✅ | ✅ | After compaction threshold |
| Workspace rules injection | ✅ | ✅ | `.vibehints`, `rules/*.md` |
| `.vibecoder.md` workspace rules | ❌ | ✅ | Injected into every AI prompt |
| Semantic index (fast search) | ✅ | ✅ | Trigram + LRU cache |
| Code Graph (kodegraph) | ✅ | ✅ | tree-sitter → SQLite graph at `.vibecli/codegraph.db`; god-node/community summary replaces the dir-tree repo map in the agent system prompt; TUI seeds `## Relevant Symbols` via blast-radius. Background build on daemon startup; `/graph/*` + `/watch/graph/*` routes; `/semindex` CLI (`build/query/node/callers/callees/hierarchy/stats`) |
| SkillForge (skill optimisation) | ✅ | ✅ | `skilllensai-rs` (analyse: trajectory → extract → score) + `skilloptai-rs` (train: rollout → bounded edit → strict held-out gate → epoch) wired through one daemon bridge `skillforge_index.rs`; `/v1/skilllens/*` + `/v1/skillopt/*` + `/watch/skilllens/*` routes; VibeCoder `SkillForgePanel` (Catalog / Lens / Optimize) in `AiMlComposite`; full surface in VS Code + Agent SDK, read-only catalog/status on Flutter + Watch + Wear. Provider-agnostic (toolbar `selectedProvider`/`selectedModel`); promote writes `*.opt.md` (shipped 1,144 skills untouched) |
| Hierarchical project memory | ✅ | ✅ | system → user → project → dir |
| Session memory (auto-extracted) | ✅ | ✅ | Facts from assistant messages |
| Pinned memory in system prompt | ❌ | ✅ | ChatMemoryPanel |

---

## Code Review & Analysis

| Feature | VibeCLI | VibeCoder | Notes |
|---|:---:|:---:|---|
| AI code review | ✅ | ✅ | 7 detectors (security, complexity, style, docs, tests, duplication, architecture) |
| Security / OWASP scan | ✅ | ✅ | |
| Complexity analysis | ✅ | ✅ | |
| Duplication detection | ✅ | ✅ | |
| Quality gates (pass/fail threshold) | ✅ | ✅ | |
| Mermaid diagram generation | ✅ | ✅ | |
| PR summary generation | ✅ | ⚙️ | |
| Post review to GitHub PR | ✅ | ❌ | `--post-github` |
| **BugBot diff review** | ✅ | ❌ | `--bugbot` · static OWASP/CWE scan + LLM pass · exits 1 on error-severity |
| BugBot on staged index | ✅ | ❌ | `--bugbot --staged` |
| Full-diff coverage + reported caveats | ✅ | n/a | Per-file batching; skipped/truncated files are named, never silently dropped |
| Multi-pass review (rotated file order) | ✅ | n/a | `--passes N` · deterministic rotation · findings deduped across passes |
| **Committable fix suggestions** | ✅ | ❌ | `--propose-fixes` → GitHub ```` ```suggestion ```` blocks; anchors verified against the diff post-image, fixes **not** compiled or tested |
| Apply proposed fixes locally | ✅ | ❌ | `--apply-fixes`; skips any file that moved since the diff |
| GitHub App PR review (webhook) | ✅ | n/a | `POST /webhook/github` · inline comments + `vibecody/review` status · **webhook secret required** — unsigned requests are rejected |
| GitHub App auto-fix suggestions | ✅ | n/a | `[github_app] auto_fix = true` · attaches suggestions, never pushes a commit |
| Architecture spec (TOGAF, C4, ADR) | ✅ | ✅ | |
| Dependency analysis | ✅ | ✅ | |
| Self-review mode | ✅ | ✅ | |
| Review protocol enforcement | ✅ | ✅ | |

---

## Testing

| Feature | VibeCLI | VibeCoder | Notes |
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

| Feature | VibeCLI | VibeCoder | Notes |
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

| Feature | VibeCLI | VibeCoder | Notes |
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

## Plugin Governance (signed MCPB bundles)

| Feature | VibeCLI | VibeCoder | Notes |
|---|:---:|:---:|---|
| `vibecli-plugin.toml` inner manifest schema | ✅ | ✅ | name + version + publisher (P-256 JWK) + components |
| Per-publisher P-256 ECDSA signing | ✅ | ✅ | Detached `vibecli-plugin.sig` (B6 key infra) |
| MCPB outer container (A2) | ✅ | ✅ | Open lineage to `.vsix` / MetaPK |
| Tampered-bundle rejection at install | ✅ | ✅ | Digest mismatch / signature mismatch / wrong key |
| Per-workspace install policy (Off / On / Required) | ✅ | ✅ | `plugin_policies` table in `workspace.db` |
| Required-pin admin guard | ✅ | ✅ | Only `PolicySetter::Admin` can raise to / lower from Required |
| Atomic install (stage → swap) | ✅ | ✅ | RAII cleanup on failure |
| Required-pin survives force re-install | ✅ | ✅ | Admin pin not lowered by re-install |
| Runtime view filtered by policy | ✅ | n/a | `plugin_runtime::enabled_*` |
| Component-source provenance | ✅ | ✅ | Skill / hook / rule tagged `Builtin` or `Plugin(name)` |
| MCP `list_skills` / `get_skill` honors policy | ✅ | n/a | Built-in + enabled-plugin skills only |
| MCP server components register under policy | ✅ | n/a | `mcp_governance::register_plugin_servers` namespaces `plugin:<plugin>:<component>` |
| Plugin hooks fire on CLI agent path | ✅ | n/a | `merge_with_plugin_hooks` (B2.9) — orchestrator + REPL |
| Plugin hooks fire on daemon agent path | ✅ | n/a | `serve.rs` `/v1/agent` + ACP + timed-task (B2.9.daemon) |
| Plugin rules land in system context | ✅ | n/a | `collect_plugin_rules` → `plugin_rules` ContextSection at priority 1 (B2.10) |
| HTTPS install (`vibecli plugin install <url>`) | ✅ | ✅ | 60 s timeout, 50 MB cap, scheme guard (B2.12) |
| Governance panel (Plugin Governance) | n/a | ✅ | Install + per-row policy buttons + publisher fingerprint |
| Tauri commands | n/a | ✅ | `plugin_install_from_file`, `plugin_install_from_url`, `plugin_list_installed`, `plugin_uninstall`, `plugin_get_policy`, `plugin_set_policy` |
| Sensitive-path gating on install paths | n/a | ✅ | `reject_sensitive_path` on workspace + bundle |
| Plugin subagents (B2.11) | ⏳ | ⏳ | Deferred — no existing file-based subagent loader to plug into |

---

## Goals — Durable Execution Intent

| Feature | VibeCLI | VibeCoder | Notes |
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

| Feature | VibeCLI | VibeCoder | Notes |
|---|:---:|:---:|---|
| Full terminal emulator | ✅ | ✅ | xterm.js in VibeCoder |
| Multiple terminal tabs | ❌ | ✅ | VibeCoder only |
| Shell completions | ✅ | ❌ | bash/zsh/fish/powershell/elvish |
| Command history | ✅ | ✅ | |
| Shell aliases | ✅ | ❌ | |
| TUI mode (Ratatui) | ✅ | ❌ | VibeCLI only |

---

## Security & Sandbox

| Feature | VibeCLI | VibeCoder | Notes |
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

| Feature | VibeCLI | VibeCoder | Notes |
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

| Feature | VibeCLI | VibeCoder | Notes |
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

| Feature | VibeCLI | VibeCoder | Notes |
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

## Voice Input

Every client can dictate. All of them transcribe through the daemon's
`POST /voice/transcribe`, which runs a local whisper model when one is
downloaded and falls back to Groq Whisper — so no client re-implements a
speech provider and offline transcription works everywhere at once.

| Client | Capture | Notes |
|---|---|---|
| VibeCLI (REPL) | SoX `rec` | `/voice` slash command; `--voice` flag |
| VibeCoder | Web Speech → `MediaRecorder` | Shared hook (`@vibe/shared/voice`) |
| VibeDesk | Web Speech → `MediaRecorder` | Mic in the composer and the side chat |
| VibeAIChat | Web Speech → `MediaRecorder` | Mic in the chat box |
| VibeMobile | `speech_to_text` → `record` | On-device recogniser first, upload as fallback |
| VibeCodyWatch | `SFSpeechRecognizer` | Native watchOS dictation |
| VibeCodyWear | Android `SpeechRecognizer` | Native Wear OS dictation |
| VS Code | SoX `rec` | Mic in the chat view + `VibeCLI: Dictate` command |
| JetBrains | SoX `rec` | Mic button in the Chat tool window |
| Neovim | SoX `rec` | `:VibeCLIVoice` (`!` submits as a task) |
| Agent SDK | caller-supplied bytes | `agent.transcribe(audio)` · `agent.voiceStatus()` |

**Engines.** Browser and mobile clients prefer the platform recogniser (free,
streams partial text, stays on-device). Everything else records a clip and
uploads it. The daemon reports which engine produced each transcript
(`local_whisper` or `cloud_whisper`).

**Dependencies.** SoX is required for the terminal and IDE clients
(`brew install sox` · `apt install sox` · `choco install sox`); its absence is
reported with that hint, never as a generic failure. Local transcription needs a
Whisper runtime — `brew install whisper-cpp` (which provides `whisper-cli`), a
source build of whisper.cpp, or `pip install openai-whisper` — plus a downloaded
model (`/voice download base`). ffmpeg is additionally required to transcribe
non-WAV audio locally, which includes everything the browser clients record;
without it those recordings fall back to the cloud. `GET /voice/status` reports
exactly which of these is present.

---

## Platform Support

| Platform | VibeCLI | VibeCoder |
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
| **Review** | `--bugbot` (diff review + committable fixes — see [BugBot](bugbot.md)), `--review`, `/review` |
| **Infra** | `/sandbox`, `/docker`, `/container`, `/cloud`, `/vm` |
| **Integrations** | `/linear`, `/mcp`, `/skills`, `/connect` |
| **Advanced** | `/arena`, `/profiler`, `/bisect`, `/repair`, `/loop`, `/goal`, `/voice` |
| **System** | `/cost`, `/config`, `/status`, `/theme`, `/wizard` — plus `vibecli --doctor` (CLI flag) |

*Last updated: 2026-07-30 · See [FEATURE-REFERENCE.md](FEATURE-REFERENCE.md) for deep-dive per-feature documentation.*
