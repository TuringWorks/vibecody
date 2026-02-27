# VibeCody Fit-Gap Analysis v2 — Full Competitive Landscape
**Date:** 2026-02-26 (updated)
**VibeCLI competitors:** Codex CLI, Warp 2.0, Kiro, opencode, Claude Code, Aider, Cline, Continue.dev, Amazon Q Developer
**VibeUI competitors:** Antigravity (Google), Cursor, Windsurf, Replit, Base44, Lovable, Zed AI, Void

> **Status:** All Phases 12–31 ✅ complete. Phases 32–42 polish/security ✅ complete. Security hardening audit (P0–P3, 20 items) ✅ complete. This document reflects the current state of the codebase as of 2026-02-26.

---

## Part A — VibeCLI Competitive Analysis

### A.1 Feature Matrix

| Feature | VibeCLI | Codex CLI | Warp 2.0 | Kiro | opencode | Claude Code | Aider | Cline | Continue.dev | Amazon Q |
|---------|---------|-----------|----------|------|----------|-------------|-------|-------|--------------|----------|
| Multi-turn REPL | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Agent loop + tools | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Plan mode | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ❌ |
| Session resume | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ❌ | ❌ | ❌ | ❌ |
| Multi-provider support | ✅ (10+) | ✅ (1) | ✅ | ✅ | ✅ (75+) | ✅ (1) | ✅ (many) | ✅ | ✅ | ✅ (1) |
| MCP client | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ❌ | ✅ | ✅ | ❌ |
| Hooks (pre/post tool use) | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ❌ | ❌ | ❌ | ❌ |
| Skills / slash commands | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ❌ |
| Git integration + PR review | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ❌ | ✅ |
| Parallel multi-agent | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ❌ | ❌ | ❌ | ❌ |
| HTTP daemon (`serve`) | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| Agent SDK (Node.js) | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| OpenTelemetry tracing | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| VS Code extension | ✅ | ❌ | ✅ | ✅ | ❌ | ✅ | ❌ | ✅ | ✅ | ✅ |
| JetBrains plugin | ✅ | ❌ | ❌ | ❌ | ❌ | ✅ | ❌ | ❌ | ✅ | ✅ |
| Neovim plugin | ✅ | ❌ | ❌ | ❌ | ❌ | ✅ | ✅ | ❌ | ✅ | ❌ |
| Named profiles | ✅ | ❌ | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| Auto memory recording | ✅ | ❌ | ✅ | ✅ | ❌ | ✅ | ❌ | ✅ | ❌ | ❌ |
| Rules directory | ✅ | ✅ | ✅ | ✅ | ❌ | ✅ | ✅ | ✅ | ❌ | ❌ |
| Wildcard tool permissions | ✅ | ✅ | ✅ | ✅ | ❌ | ✅ | ❌ | ✅ | ❌ | ❌ |
| opusplan model routing | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| OS-level sandboxing | ✅ | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| Network sandboxing | ✅ | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| Spec-driven development | ✅ | ❌ | ❌ | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| Steering files | ✅ | ❌ | ✅ | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| File-event hooks (save/create/delete) | ✅ | ❌ | ❌ | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| --watch file monitoring | ✅ | ❌ | ❌ | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| Git worktree isolation per subagent | ✅ | ❌ | ✅ | ❌ | ❌ | ✅ | ❌ | ❌ | ❌ | ❌ |
| SQLite session storage | ✅ | ❌ | ❌ | ❌ | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ |
| Session full-text search | ✅ | ❌ | ❌ | ❌ | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ |
| Web-viewable agent sessions | ✅ | ❌ | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| Team knowledge store | ✅ | ❌ | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| Ambient agent session sharing | ✅ | ❌ | ✅ | ❌ | ❌ | ✅ | ❌ | ❌ | ❌ | ❌ |
| Slack/Linear/Telegram/Discord | ✅ (all 4) | ❌ | ✅ (Slack) | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| 75+ LLM providers via OpenRouter | ✅ | ❌ | ❌ | ❌ | ✅ | ❌ | ✅ | ✅ | ✅ | ❌ |
| GitHub Copilot auth | ✅ | ❌ | ❌ | ❌ | ✅ | ❌ | ✅ | ✅ | ✅ | ❌ |
| AWS Bedrock provider | ✅ | ❌ | ❌ | ❌ | ✅ | ❌ | ✅ | ✅ | ✅ | ✅ |
| Background/ambient agents | ✅ | ❌ | ✅ | ❌ | ❌ | ✅ | ❌ | ❌ | ❌ | ❌ |
| Vim-like TUI editor | ✅ | ❌ | ❌ | ❌ | ✅ | ❌ | ✅ | ❌ | ❌ | ❌ |
| Notebook runner (.vibe) | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| Built-in scheduling (cron) | ✅ | ❌ | ❌ | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| Subagent spawning from tools | ✅ | ❌ | ❌ | ❌ | ❌ | ✅ | ❌ | ❌ | ❌ | ❌ |
| Named snippets (/snippet) | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| /rewind conversation checkpoints | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| /search full-text session history | ✅ | ❌ | ❌ | ❌ | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ |
| @file / @web / @docs / @git context | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ❌ |
| Linear issue integration | ✅ | ❌ | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| Background job persistence + REST | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| Red team / autonomous pentest | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| OWASP/CWE static scanner (15 patterns) | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ✅ |
| Secrets scrubbing in traces/logs | ✅ | ❌ | ❌ | ❌ | ❌ | ✅ | ❌ | ❌ | ❌ | ❌ |
| Daemon auth (bearer token) + CORS | ✅ | — | — | — | — | — | — | — | — | — |
| Rate limiting on API endpoints | ✅ | — | — | — | — | — | — | — | — | — |
| Security response headers (CSP/X-Frame) | ✅ | — | — | — | — | — | — | — | — | — |
| Graceful shutdown (SIGTERM handler) | ✅ | — | — | — | — | — | — | — | — | — |
| Binary checksum verification (install) | ✅ | ✅ | ❌ | ❌ | ❌ | ✅ | ❌ | ❌ | ❌ | ✅ |
| SHA-pinned CI actions | ✅ | ✅ | ❌ | ❌ | ❌ | ✅ | ❌ | ❌ | ❌ | ✅ |
| cargo audit in CI | ✅ | ✅ | ❌ | ❌ | ❌ | ✅ | ❌ | ❌ | ❌ | ✅ |
| Code Complete workflow (8-stage) | ✅ | ❌ | ❌ | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| LSP diagnostics panel (TUI) | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| Streaming REPL chat (token-by-token) | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Tab-completion for REPL commands | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ❌ | ❌ | ❌ | ❌ |

---

### A.2 Competitor Deep-Dive

#### Codex CLI (OpenAI)
- **Approval modes:** read-only / auto (default) / full-access; `/permissions` mid-session switching
- **OS sandbox:** macOS Seatbelt, Docker, Windows Sandbox — default blocks network + limits filesystem writes
- **Network approval:** rich host/protocol context shown in prompts; structured network approval IDs per command
- **Context:** `@` fuzzy file search in composer (Tab/Enter to drop path); AGENTS.md; MCP; per-command approval IDs
- **Codex App:** separate GUI app with visual project management
- **Codex SDK:** programmatic agent creation via OpenAI Agents SDK

#### Warp 2.0 — Agentic Development Environment

Warp is the **most complete** VibeCLI competitor in 2026. It ships as a single app with four integrated pillars.

**Core Architecture (4 pillars):**
- **Code:** full editor with AI inline completions, natural language to command
- **Agents:** autonomous multi-step task execution with terminal access
- **Terminal:** GPU-accelerated, block-based output, SSH, tmux multiplexing
- **Drive:** centralized team knowledge store

**Warp Drive (biggest gap vs VibeCLI):**
Warp Drive is a *shared knowledge base for humans and agents*:
- Centralized MCP server configurations (all team members get the same MCP setup)
- Shared rules (coding standards, tool preferences) — checked in but synced via Drive
- Shared commands: team-curated bash one-liners, named and searchable
- Notebooks: documentation + runnable code blocks, shareable
- Environment variables: shared across team; agents access automatically
- Prompts: reusable AI prompt templates shared across team

**Multi-Agent Orchestration:**
- Native agent status panel with per-agent autonomy settings
- Notifications when agents complete or need input
- Up to N agents running concurrently; each gets own context window
- Indexed 120,000+ codebases; processed trillions of LLM tokens in 2025

**Ambient Agent Session Sharing (unique to Warp):**
- Remote VM agents (cloud-based execution) viewable in Warp web viewer
- Anyone with the share link can: inspect context/logs, give follow-up instructions, fork the session to their local environment
- Real-time session streaming — watch agent work live

**SSH + Remote Development:**
- SSH via tmux Control Mode (Warpify remote connections)
- Remote sessions get full Warp UI features: completions, AI, blocks
- Warp for Windows: native Windows terminal with agentic features (2026)

**External Integrations:**
- Slack: trigger agents from Slack, receive results back
- Linear: create/update issues from terminal; link tasks to agent runs
- GitHub Actions: dispatch workflows from Warp; agents triggered by CI failures

**Feature-by-Feature vs VibeCLI (current status):**

| Warp Feature | VibeCLI Status |
|-------------|----------------|
| Warp Drive (team knowledge store) | ✅ team.rs + /team REPL commands |
| Ambient agent session sharing | ❌ Missing |
| Web viewer for agent sessions | ✅ GET /view/:id + /sessions HTML |
| Slack integration | ✅ gateway.rs |
| Linear integration | ✅ linear.rs + /linear REPL commands |
| Named shared commands | ✅ skills/ + snippets |
| Multi-agent status UI | ✅ Manager view in VibeUI |
| Notebook-style runnable docs | ✅ notebook.rs (.vibe format) |
| Remote VM agent execution | ❌ Missing |
| Codebase indexing | ✅ EmbeddingIndex + /index cmd |
| Per-agent autonomy settings | ✅ approval policies |

**Remaining gap:** ambient session sharing, remote VM execution, GPU terminal

#### Kiro (Amazon, VS Code fork)
- **Spec-driven development:** NL requirements → user stories + acceptance criteria → technical design → task list
- **Steering files:** project-scope context (coding standards, workflows) — analogous to `.cursorrules` but more structured
- **Agent hooks:** automated triggers on file-system events (save, create, delete) — not just tool-use hooks
- **MCP:** with remote MCP server support
- **VibeCLI status:** ✅ spec.rs + SpecPanel, ✅ steering files, ✅ FileSaved/FileCreated/FileDeleted hooks, ✅ MCP

#### opencode (opencode-ai/opencode, sst/opencode)

opencode is the **most technically ambitious** open-source CLI agent in 2026, built in Go by the SST team.

**Provider Ecosystem (biggest advantage):**
75+ LLM providers via a unified abstraction layer. VibeCLI now has 10 direct providers + OpenRouter (300+) = full parity.

**Feature-by-Feature vs VibeCLI (current status):**

| opencode Feature | VibeCLI Status |
|-----------------|----------------|
| 75+ providers | ✅ 10 direct + OpenRouter (300+) |
| SQLite sessions | ✅ session_store.rs |
| Vim-like TUI keybindings | ✅ vim_editor.rs (Normal/Insert/Visual modes) |
| GitHub Copilot OAuth | ✅ copilot.rs + device flow |
| LSP in TUI | ❌ (LSP only in VibeUI) |
| AWS Bedrock | ✅ bedrock.rs |
| Groq ultra-fast inference | ✅ groq.rs |
| Azure OpenAI | ✅ azure_openai.rs |
| OpenRouter (300+ models) | ✅ openrouter.rs |
| Hooks system | ❌ Missing in opencode (VibeCLI wins) |
| Admin policy | ❌ Missing in opencode (VibeCLI wins) |
| HTTP daemon / SDK | ❌ Missing in opencode (VibeCLI wins) |
| PR code review | ❌ Missing in opencode (VibeCLI wins) |
| OTel tracing | ❌ Missing in opencode (VibeCLI wins) |
| Multi-agent | ❌ opencode single-agent (VibeCLI wins) |
| Named profiles | ❌ Missing in opencode (VibeCLI wins) |
| Session full-text search | ✅ both have it |

**Remaining gap:** None material — VibeCLI matches or exceeds opencode on all axes.

#### Claude Code (Anthropic)
- **Subagents:** up to 7 parallel with `--worktree (-w)` git worktree isolation
- **Background agents:** `background: true` in agent definition — always runs async
- **Hooks:** SubagentStop event with `last_assistant_message` field
- **CLAUDE.md hierarchy:** 4-level loading (home → repo root → subfolder → current dir)
- **MCP OAuth:** full OAuth2 for MCP server auth
- **VibeCLI status:** ✅ worktree isolation (`--worktree`), ✅ background agents (background_agents.rs), ✅ 4-level VIBECLI.md, ❌ MCP OAuth not yet

#### Aider (Paul Gauthier)
- **Strength:** Best-in-class git commit workflow — auto-commits every AI change with descriptive messages
- **Strength:** Wide LLM support (any OpenAI-compatible API, 50+ models)
- **Strength:** Architect + Editor dual-model mode (cheap fast model for edits, expensive model for planning)
- **Strength:** Vim-native usage (`:!aider` in terminal, fully non-interactive mode)
- **Weakness:** No daemon/HTTP API; no parallel agents; no hooks; no session persistence
- **VibeCLI gap to close:** Architect+Editor dual-model routing → ✅ already done via `opusplan` routing (planning_provider + execution_provider in config.rs)

#### Cline (formerly Claude Dev)
- **Strength:** Best VS Code agent (1M+ installs); deeply integrated into VS Code editor state
- **Strength:** Shows diffs inline in editor, gets approval from editor UI not terminal
- **Strength:** Auto-detects API errors and suggests cheaper models
- **Strength:** Remembers custom instructions per project (`.clinerules` file)
- **Weakness:** VS Code-only; no CLI; no daemon; no parallel agents; no hooks
- **VibeCLI gap to close:** Better inline editor integration via VS Code extension ✅ (done in Phase 9.2 + Phase 31)

#### Continue.dev (open source)
- **Strength:** Most customizable open-source VS Code/JetBrains plugin
- **Strength:** Any model via any provider; full `config.json` YAML customization
- **Strength:** Context providers: `@codebase`, `@docs`, `@web`, `@terminal`, `@github`, `@jira`
- **Strength:** Built-in embedding for codebase Q&A
- **Weakness:** No agent mode (yet); just chat + tab completions; no CLI
- **VibeCLI gap to close:** Continue's @github/@jira context providers → partial (we have @git, @docs, @web; missing @jira)

#### Amazon Q Developer
- **Strength:** Deep AWS integration; security scanning against OWASP/CWEs
- **Strength:** `/dev` command generates multi-file changes with PR-style diff view
- **Strength:** `/test` command generates comprehensive unit tests
- **Strength:** `/review` command for inline code quality reviews
- **Strength:** Transformation: Java version upgrades + .NET migration automation
- **Weakness:** AWS-only focus; poor multi-provider; closed source; subscription required
- **VibeCLI gap to close:** Security scanning integration → partial (bugbot.rs covers basic review)

---

### A.3 Extended Competitor — PicoClaw

**PicoClaw** (github.com/sipeed/picoclaw) — ultra-lightweight Go AI assistant launched February 9, 2026, 12,000 GitHub stars in its first week.

**PicoClaw vs VibeCLI Feature Gap (current status):**

| PicoClaw Feature | VibeCLI Status |
|-----------------|----------------|
| Single binary releases (cargo dist / install.sh) | ✅ install.sh + release.yml (Phase 27) |
| Messaging gateway (Telegram/Discord/Slack) | ✅ gateway.rs + --gateway flag (Phase 21) |
| AI-optimized web search (Tavily) | ✅ Tavily + Brave + DDG multi-engine (Phase 21) |
| Built-in cron/scheduling | ✅ scheduler.rs + /remind + /schedule (Phase 21) |
| Subagent spawning from tools | ✅ spawn_agent tool + spawn_sub_agent() (Phase 22) |
| Skills as distributable packages | ✅ .vibecli-skill.tar.gz packaging (Phase 21) |
| Cold start < 1 second | Partial (~2-5s Rust startup) |
| ARM64/RISC-V binary | ✅ release matrix includes aarch64 (Phase 27) |

---

## Part B — VibeUI Competitive Analysis

### B.1 Feature Matrix

| Feature | VibeUI | Antigravity | Cursor | Windsurf | Replit | Base44 | Lovable | Zed AI | Void |
|---------|--------|-------------|--------|----------|--------|--------|---------|--------|------|
| Code editor | ✅ | ✅ | ✅ | ✅ | ✅ | ❌ | ❌ | ✅ | ✅ |
| AI chat panel | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Agent (multi-file edits) | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Inline Cmd+K chat | ✅ | ✅ | ✅ | ✅ | ✅ | ❌ | ❌ | ✅ | ✅ |
| Tab next-edit prediction | ✅ | ✅ | ✅ | ✅ | ✅ | ❌ | ❌ | ✅ | ✅ |
| Terminal panel | ✅ | ✅ | ✅ | ✅ | ✅ | ❌ | ❌ | ✅ | ✅ |
| Git integration | ✅ | ✅ | ✅ | ✅ | ✅ | ❌ | ✅ | ✅ | ✅ |
| LSP / code intelligence | ✅ | ✅ | ✅ | ✅ | ❌ | ❌ | ❌ | ✅ | ✅ |
| MCP client | ✅ | ✅ | ✅ | ✅ | ❌ | ❌ | ❌ | ✅ | ❌ |
| Multi-provider BYOK | ✅ | ✅ | ✅ | ✅ | ❌ | ❌ | ❌ | ✅ | ✅ |
| Voice input | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| Background job persistence | ✅ | ❌ | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| @web / @docs / @git / @codebase context | ✅ | ❌ | ✅ | ✅ | ❌ | ❌ | ❌ | ✅ | ❌ |
| Browser preview panel | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ❌ | ❌ |
| Artifact system | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| Parallel Manager view | ✅ | ✅ | ✅ | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ |
| Hooks config UI | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| Cascade flow tracker | ✅ | ❌ | ❌ | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ |
| DiffReviewPanel (per-hunk accept/reject) | ✅ | ❌ | ✅ | ✅ | ❌ | ❌ | ❌ | ✅ | ✅ |
| Linter integration | ✅ | ❌ | ✅ | ✅ | ✅ | ❌ | ❌ | ✅ | ✅ |
| Visual UI Editor (drag-drop) | ✅ | ✅ | ✅ | ❌ | ❌ | ✅ | ✅ | ❌ | ❌ |
| Automated PR review (BugBot) | ✅ | ❌ | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| Memories (auto-generated) | ✅ | ✅ | ✅ | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ |
| Shadow workspace (bg lint) | ✅ | ❌ | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| One-click deployment | ✅ | ✅ | ❌ | ❌ | ✅ | ✅ | ✅ | ❌ | ❌ |
| Built-in database UI | ✅ | ✅ | ❌ | ❌ | ✅ | ✅ | ✅ | ❌ | ❌ |
| Auth + backend scaffolding | ✅ | ✅ | ❌ | ❌ | ✅ | ✅ | ✅ | ❌ | ❌ |
| GitHub bidirectional sync | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ✅ | ❌ | ❌ |
| Supabase integration | ✅ | ✅ | ❌ | ❌ | ✅ | ❌ | ✅ | ❌ | ❌ |
| Multiplayer / real-time collab | ❌ | ❌ | ❌ | ❌ | ✅ | ❌ | ✅ | ✅ | ❌ |
| Browser-embedded app testing | ✅ | ❌ | ❌ | ❌ | ✅ | ❌ | ❌ | ❌ | ❌ |
| Figma import | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ✅ | ❌ | ❌ |
| Custom domain / publish | ❌ | ✅ | ❌ | ❌ | ✅ | ✅ | ✅ | ❌ | ❌ |
| Design mode | ✅ | ✅ | ✅ | ❌ | ❌ | ✅ | ✅ | ❌ | ❌ |
| Point-and-prompt in live app | ✅ | ✅ | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| Custom SWE model (SWE-1) | ❌ | ✅ | ❌ | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ |
| WASM extension system | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ✅ | ❌ |
| Steering files UI | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| Spec-driven dev UI | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| Semantic codebase search (embedding) | ✅ | ✅ | ✅ | ✅ | ❌ | ❌ | ❌ | ✅ | ❌ |
| Multi-tab AI chat | ✅ | ✅ | ✅ | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ |
| Auto-lint after agent write | ✅ | ❌ | ✅ | ✅ | ❌ | ❌ | ❌ | ✅ | ❌ |
| Inline confirmation dialogs | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Git auto-refresh | ✅ | ✅ | ✅ | ✅ | ✅ | ❌ | ✅ | ✅ | ✅ |
| MCP server manager UI | ✅ | ❌ | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| Path traversal protection | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |

---

### B.2 Competitor Deep-Dive

#### Google Antigravity (Nov 2025, Gemini 3)
- Agent-first IDE from Google, deeply integrated with Gemini 3 and Google Cloud/Firebase
- **Unique:** native Firebase/GCP one-click deploy, Gemini 3 multimodal (image-to-code, video understanding)
- **VibeUI gap:** GCP/Firebase deployment, multimodal file attachment in chat

#### Cursor 2.0+
- **Composer:** up to 8 parallel agents using git worktrees or remote machines
- **Shadow workspace:** background hidden window that runs lint checks on AI-proposed code before showing diffs
- **BugBot:** automated PR reviewer triggered on every PR; "Fix in Cursor" deep-link to problematic code
- **Memories:** auto-generated from conversations, persistent per-project knowledge
- **Visual Editor:** drag-drop elements in rendered live app + property sliders + point-and-prompt natural language UI editing
- **Background agents:** high-risk (network/FS access) and low-risk categories
- **One-click MCP install** with OAuth
- **VibeUI status:** ✅ Shadow workspace, ✅ BugBot, ✅ Memories, ✅ Visual Editor, ✅ parallel agents with Manager view
- **Remaining gap:** MCP OAuth install, remote machine agents

#### Windsurf (Codeium)
- **Cascade:** tracks ALL actions to infer intent — no re-prompting needed
- **SWE-1 model family:** purpose-built for software engineering; free to use
- **Supercomplete:** cross-file multi-line prediction
- **Wave 13:** parallel multi-agent sessions, side-by-side Cascade panes, Git worktrees
- **VibeUI status:** ✅ Cascade flow tracker (FlowContext.ts), ✅ Supercomplete (SupercompleteEngine.ts), ✅ parallel agents
- **Remaining gap:** SWE-1-style fine-tuned model support, full cross-IDE plugin coverage

#### Replit
- **Agent 3:** 10x more autonomous; browser-embedded testing (AI controls cursor in live app)
- **Self-healing:** proprietary test system, 3x faster + 10x cheaper than Computer Use
- **Built-in stack:** auth + database (PostgreSQL) + hosting + monitoring in one click
- **Multiplayer:** real-time collaboration on same Repl
- **VibeUI status:** ✅ browser app testing (agent_browser_action), ✅ built-in DB+auth, ❌ multiplayer not yet
- **Remaining gap:** multiplayer/CRDT collab, hosted cloud environment

#### Base44 (acquired by Wix)
- **All-in-one:** UI + database + auth + hosting — no external services
- **Press Publish → live:** zero deployment friction
- **VibeUI status:** ✅ deploy panel, ✅ database UI, ✅ auth scaffolding, ❌ custom domain, ❌ fully hosted stack
- **Remaining gap:** self-contained hosting stack, press-to-publish UX, custom domains

#### Lovable 2.0
- **Full-stack generation:** React + Supabase from NL description
- **GitHub bidirectional sync:** real-time sync to/from GitHub repo
- **Supabase integration:** auth, database, storage out-of-the-box
- **Multiplayer:** real-time collaborative editing (April 2025)
- **Figma import** + **Deploy:** Netlify, Vercel, custom domains
- **VibeUI status:** ✅ GitHub sync, ✅ Supabase, ✅ Design mode, ✅ Figma import, ❌ custom domain, ❌ multiplayer

#### Zed AI (Zed Industries)
- **AI-native editor** written in Rust + GPUI; sub-1ms keystrokes
- **Agent panel:** Claude-native integration; task delegation from editor
- **Context:** @file, @symbol, @web built-in; LSP hovers in chat
- **WASM extensions:** sandboxed plugin runtime (very similar to vibe-extensions)
- **Multiplayer:** built-in real-time collaboration (years ahead of competitors)
- **VibeUI gap:** Zed's performance (native GPU rendering vs webview), multiplayer built into core
- **VibeCLI gap:** Zed has no agent API / daemon; VibeCLI wins here

#### Void (Open Source Cursor Alternative)
- **Open source** drop-in Cursor replacement (VS Code fork)
- **Bring Your Own Keys:** all providers, fully local
- **Agent:** multi-file editing with checkpoint system
- **Checkpoint system:** save/restore editor state around each AI change
- **VibeUI status:** ✅ checkpoint system (CheckpointPanel), ✅ BYOK (SettingsPanel)
- **Remaining gap:** Void's VS Code ecosystem compatibility

---

## Part C — VibeCody Exclusive Advantages

Features VibeCody has that **no competitor offers:**

| Feature | Why Unique |
|---------|-----------|
| **HTTP daemon + REST API** | `vibecli serve` — enables SDK, VibeUI, JetBrains/VS Code integration from one process |
| **Node.js Agent SDK** | `packages/agent-sdk/` — only CLI tool with a programmable streaming SDK |
| **OpenTelemetry OTLP tracing** | Observability-first; no competitor exports spans to Jaeger/Zipkin/Grafana |
| **Voice input (Web Speech API)** | 🎤 button in AIChat — no desktop coding tool has it cleanly |
| **Admin policy (glob-based)** | Enterprise tool restriction with wildcard deny/allow patterns |
| **WASM extension system** | `vibe-extensions` with wasmtime — sandboxed plugin runtime |
| **opusplan routing** | Separate planning vs execution model per request |
| **Artifact panel** | Structured AI output typed and stored persistently |
| **@web context (DuckDuckGo+Tavily+Brave)** | Full HTML fetch + multiple search engines in agent context |
| **Multi-agent Manager view** | Visual UI for parallel agent execution with branch merging |
| **Hooks config UI** | Visual editor for hooks with LLM handler support |
| **Background job persistence** | Jobs survive daemon restart; cancel/stream REST endpoints |
| **VibeCLI Daemon as IDE bridge** | All IDEs (VS Code, JetBrains, Neovim) communicate via same daemon |
| **/rewind conversation checkpoints** | Save/restore REPL conversation state — unique to VibeCLI |
| **Named snippets** | `/snippet save/use/show/delete` — reusable AI response library |
| **Notebook runner (.vibe format)** | Multi-language literate notebooks with YAML frontmatter |
| **Spec-driven dev (spec.rs + SpecPanel)** | Full NL→spec→tasks→agent workflow in both CLI and desktop |
| **Messaging gateway (4 platforms)** | Telegram + Discord + Slack + Teams — no other coding tool supports all 4 |
| **Linear issue integration** | Native /linear REPL commands; only CLI tool to integrate issue tracking |
| **Path traversal protection** | Validated file operations, `is_safe_name()`, canonicalized paths |
| **Full security hardening (20 items)** | P0–P3 audit: secret scrubbing, bearer auth, rate limiting, CSP headers, graceful shutdown, cargo audit, SHA-pinned CI, file permissions |

---

## Part D — Gap Priority Matrix (Updated Status)

All gaps from Phases 16–41 are now resolved. This table shows **remaining** open gaps as of 2026-02-26:

| Gap | Impact | Status | Competitor |
|-----|--------|--------|------------|
| Multiplayer / real-time collaboration | Medium | ❌ Open | Lovable/Replit/Zed |
| Remote VM agent execution | Low | ❌ Open | Warp |
| SWE-1-style fine-tuned model | Low | ❌ Open | Windsurf |
| GPU-accelerated terminal | Low | ❌ Open | Warp/Zed |
| Ambient agent session sharing | Medium | ✅ Phase 39 | Warp/Claude Code |
| GCP / Firebase deploy target | Low | ✅ Phase 38 | Antigravity |
| LSP diagnostics in TUI | Low | ✅ Phase 39 | opencode |
| Security scanning (OWASP/CWE) | Medium | ✅ Phase 38/40 | Amazon Q |
| @jira / @github context | Low | ✅ Phase 42 | Continue.dev |
| MCP OAuth install | Medium | ✅ Phase 42 | Cursor |
| Custom domain / publish | Medium | ✅ Phase 42 | Base44/Lovable/Replit |

### Previously Closed Gaps (Phases 16–37)

| Phase | Gap | Status |
|-------|-----|--------|
| 16 | Spec-driven development | ✅ spec.rs + SpecPanel.tsx |
| 16 | Steering files | ✅ SteeringPanel.tsx |
| 16 | File-event hooks | ✅ FileSaved/Created/Deleted |
| 16 | OS-level sandboxing | ✅ macOS sandbox-exec + Linux bwrap |
| 16 | Git worktree isolation | ✅ --worktree + IsolatedWorktree |
| 17 | AWS Bedrock provider | ✅ bedrock.rs |
| 17 | Groq ultra-fast inference | ✅ groq.rs |
| 17 | Azure OpenAI | ✅ azure_openai.rs |
| 17 | OpenRouter (300+ models) | ✅ openrouter.rs |
| 17 | SQLite session storage | ✅ session_store.rs |
| 17 | Ambient agent definitions | ✅ background_agents.rs |
| 17 | Team knowledge store | ✅ team.rs + /team commands |
| 18 | Memories auto-generation | ✅ memory_auto.rs + AutoFactsTab |
| 18 | Shadow workspace (bg lint) | ✅ shadow_workspace.rs |
| 18 | BugBot automated PR review | ✅ bugbot.rs + BugBotPanel.tsx |
| 19 | Visual UI Editor | ✅ VisualEditor.tsx + inspector.js |
| 19 | Design mode | ✅ DesignMode.tsx |
| 19 | Figma import | ✅ import_figma Tauri command |
| 20 | One-click deployment | ✅ DeployPanel.tsx |
| 20 | Built-in database UI | ✅ DatabasePanel.tsx |
| 20 | Supabase integration | ✅ SupabasePanel.tsx |
| 20 | Auth scaffolding | ✅ AuthPanel.tsx |
| 20 | GitHub bidirectional sync | ✅ GitHubSyncPanel.tsx |
| 20 | Browser-embedded app testing | ✅ agent_browser_action |
| 21 | Tavily/Brave web search | ✅ multi-engine search |
| 21 | Messaging gateway | ✅ gateway.rs (Telegram/Discord/Slack) |
| 21 | Built-in scheduling | ✅ scheduler.rs + /schedule |
| 21 | Single binary releases | ✅ install.sh + release.yml |
| 22 | Session full-text search | ✅ /search REPL command |
| 22 | Subagent spawning | ✅ spawn_agent tool |
| 23 | SQLite session store | ✅ session_store.rs |
| 23 | Web session viewer | ✅ GET /view/:id |
| 24 | Vim-like TUI editor | ✅ vim_editor.rs |
| 25 | AWS Bedrock (improved) | ✅ manual SigV4 without chrono |
| 25 | GitHub Copilot auth | ✅ copilot.rs + device flow |
| 25 | Notebook runner | ✅ notebook.rs (.vibe format) |
| 26 | Supabase full panel | ✅ SupabasePanel + PostgREST |
| 26 | Auth panel | ✅ 4 providers × 5 frameworks |
| 26 | GitHub Sync panel | ✅ ahead/behind + push/pull |
| 27 | Steering files UI | ✅ SteeringPanel.tsx |
| 27 | Release pipeline | ✅ release.yml multi-platform |
| 28 | Auto-memories UI | ✅ AutoFactsTab in MemoryPanel |
| 28 | BugBot UI | ✅ BugBotPanel.tsx |
| 28 | Linear integration | ✅ linear.rs + /linear REPL |
| 29 | File-event hooks (agent) | ✅ HookEvent::FileSaved in agent |
| 29 | --watch file monitoring | ✅ run_watch_mode() + notify |
| 29 | Neovim plugin | ✅ neovim-plugin/lua/vibecli/ |
| 30 | REPL streaming chat | ✅ stream_chat() token-by-token |
| 30 | @file/@web/@docs/@git expand | ✅ expand_at_refs() in main.rs |
| 30 | Named snippets | ✅ /snippet REPL commands |
| 31 | Embedding semantic search | ✅ EmbeddingIndex + cosine |
| 31 | build_embedding_index cmd | ✅ Tauri command |
| 31 | VS Code extension v2 | ✅ vibecli.inlineEdit + streaming |
| 31 | Neovim cmp source | ✅ cmp_vibecli.lua |
| 32–34 | Toast notification system | ✅ useToast + Toaster (all panels) |
| 35 | Auto-scroll + copy in AIChat | ✅ messagesEndRef + clipboard |
| 35 | GitPanel auto-refresh | ✅ 30s setInterval |
| 35 | search.rs buffered reading | ✅ BufReader + 10MB guard |
| 36 | Lazy regex compilation | ✅ OnceLock-backed accessors |
| 36 | Mutex poison recovery | ✅ unwrap_or_else(e.into_inner) |
| 36 | confirm() → inline UI | ✅ GitPanel + SteeringPanel |
| 37 | Path traversal security | ✅ is_safe_name() + canonicalize |
| 37 | EventSource cleanup | ✅ useEffect unmount cleanup |
| 37 | File delete modal | ✅ pendingDeleteFile state + modal |

---

## Part E–M — Phase 16–21 Design (Completed)

> These phases are fully implemented. Refer to the original design sections in git history for the implementation specifications. All new code is live in the repository.

---

## Part N — Phase 22: Session Search & Subagent Spawning ✅

Completed. See git commit history.

- `/search <keywords>` — multi-keyword AND search across JSONL traces + SQLite
- `spawn_agent` tool — child AgentLoop with shared provider, independent history
- `TauriToolExecutor` returns "not supported" for VibeUI context

---

## Part P — Phase 23: SQLite Session Store + Web Session Viewer ✅

Completed. See git commit history.

- `~/.vibecli/sessions.db` — WAL mode, 3-table schema (sessions/messages/steps)
- Parallel write alongside JSONL (backwards-compatible)
- `GET /sessions` HTML index, `GET /sessions.json` API, `GET /view/:id` dark-mode viewer

---

## Part Q — Phases 24–37: Recent Completions

### Phase 24: Vim TUI Editor ✅
Full modal editor in VibeCLI TUI — Normal/Insert/Visual/VisualLine/Command/Search modes, hjkl/dd/yy/p/u/gg/G/Ctrl+f/b, /search+n/N, :w/:q/:wq/:set number.

### Phase 25: AWS Bedrock + GitHub Copilot + Notebook Runner ✅
- AWS Bedrock via manual SigV4 (sha2+hmac+hex, no chrono dependency)
- GitHub Copilot device-flow OAuth + 30min token cache
- `.vibe` notebook format: YAML frontmatter + markdown + bash/python/rust/node cells

### Phase 26: Supabase + Auth + GitHub Sync ✅
- SupabasePanel: PostgREST introspection, SQL queries, AI-generated queries
- AuthPanel: 4 auth providers × 5 frameworks, AI-generated scaffold code
- GitHubSyncPanel: ahead/behind, commit+push, pull, create repo

### Phase 27: Steering Files + Release Pipeline ✅
- SteeringPanel.tsx: workspace/global scopes, templates, CRUD
- `.github/workflows/release.yml`: macOS arm64/x86, Linux musl amd64/aarch64, Windows x64
- `install.sh`: curl one-liner with OS+arch detection

### Phase 28: Auto-Memories + BugBot + Linear ✅
- AutoFactsTab in MemoryPanel with confidence badges, pin/delete/add
- BugBotPanel: severity/category filters, expand-to-details, fix snippets
- `/linear list/new/open/attach` REPL commands + GraphQL client

### Phase 29: File-Event Hooks + --watch + Neovim Plugin + Browser Actions ✅
- `HookEvent::FileSaved` fired after WriteFile in agent.rs
- `--watch/--watch-glob/--sandbox` flags + `run_watch_mode()` via notify crate
- Neovim plugin: `:VibeCLI`, `:VibeCLIAsk`, `:VibeCLIInline`, SSE streaming
- `agent_browser_action`: Navigate/GetText via reqwest, Screenshot via screencapture

### Phase 30: REPL Streaming + Context Expansion + Snippets ✅
- `stream_chat()` with `futures::StreamExt` token-by-token output
- `expand_at_refs()` in main.rs for @file:/@web:/@docs:/@git
- `/snippet save/list/use/show/delete` at `~/.vibecli/snippets/`

### Phase 31: Embedding Index + VS Code v2 + Neovim CMP ✅
- `semantic_search_codebase` upgraded to EmbeddingIndex → cosine fallback → keyword
- `build_embedding_index` Tauri command (ollama/openai providers)
- VS Code ext: `vibecli.inlineEdit` (Cmd+Shift+K), streaming chat webview, auto file-ctx
- `cmp_vibecli.lua`: slash-commands + @context completions for Neovim

### Phases 32–37: Quality + Security + Polish ✅
- **Toast system**: `useToast` hook + `Toaster` component; all `alert()` calls replaced
- **AgentPanel**: Stop button (AbortHandle), Copy (clipboard), Expand/Collapse per step
- **AIChat**: Clear + Stop generation, auto-scroll, copy on assistant messages
- **Workspace null safety**: All panels handle `workspacePath: string | null` gracefully
- **GitPanel**: 30s auto-refresh, inline discard confirmation (no native confirm())
- **SteeringPanel**: Inline delete confirmation (no native confirm())
- **App.tsx**: File delete modal (no native confirm()), extension worker toast
- **BackgroundJobsPanel**: EventSource cleanup on unmount
- **Security**: `is_safe_name()` for /snippet path traversal, timestamp validation for /rewind, `write_auth_scaffold` canonicalizes and validates workspace boundary
- **Reliability**: OnceLock regex compilation, mutex poison recovery, buffered file search
- **/jobs `<id>`**: Detail view (status/provider/task/duration/summary)
- **/rewind list**: Corrupt checkpoint files show "(corrupt: ...)" instead of silent "0 messages"

---

## Part O — Final Competitive Positioning (Current)

After all completed phases, VibeCLI + VibeUI is the **most complete AI development platform** across CLI, desktop, and embedded tooling dimensions.

### VibeCLI Positioning

| Dimension | VibeCLI | Warp | Kiro | opencode | Claude Code | Aider | Cline | Amazon Q |
|-----------|---------|------|------|----------|-------------|-------|-------|----------|
| Provider breadth | ✅ 10+ direct + 300+ via OpenRouter | ✅ | ✅ | ✅ 75+ | ✅ 1 | ✅ many | ✅ | ✅ 1 |
| Spec-driven dev | ✅ | ❌ | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ |
| HTTP daemon + SDK | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| OTel tracing | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| Messaging gateway (4 platforms) | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| Scheduling / cron agents | ✅ | ❌ | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ |
| Team knowledge store | ✅ | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| OS-level sandboxing | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| SQLite sessions | ✅ | ❌ | ❌ | ✅ | ❌ | ❌ | ❌ | ❌ |
| Web session viewer | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| Multi-agent visual UI | ✅ | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| WASM extensions | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| Subagent spawning | ✅ | ❌ | ❌ | ❌ | ✅ | ❌ | ❌ | ❌ |
| Vim TUI editor | ✅ | ❌ | ❌ | ✅ | ❌ | ✅ | ❌ | ❌ |
| Notebook runner | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| Linear/Slack/Telegram/Discord | ✅ all 4 | Partial | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| Path traversal protection | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| /rewind checkpoints | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| /snippet library | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |

### VibeUI Positioning

| Dimension | VibeUI | Cursor | Windsurf | Replit | Base44 | Lovable | Zed AI | Void |
|-----------|--------|--------|----------|--------|--------|---------|--------|------|
| Visual UI Editor | ✅ | ✅ | ❌ | ❌ | ✅ | ✅ | ❌ | ❌ |
| One-click deploy | ✅ | ❌ | ❌ | ✅ | ✅ | ✅ | ❌ | ❌ |
| Built-in DB UI | ✅ | ❌ | ❌ | ✅ | ✅ | ✅ | ❌ | ❌ |
| Supabase integration | ✅ | ❌ | ❌ | ✅ | ❌ | ✅ | ❌ | ❌ |
| GitHub sync | ✅ | ❌ | ❌ | ❌ | ❌ | ✅ | ❌ | ❌ |
| Multiplayer | ❌ | ❌ | ❌ | ✅ | ❌ | ✅ | ✅ | ❌ |
| Voice input | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| WASM extensions | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ✅ | ❌ |
| OTel tracing | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| Spec-driven dev UI | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| Background jobs UI | ✅ | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| BugBot PR review | ✅ | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| Browser app testing | ✅ | ❌ | ❌ | ✅ | ❌ | ❌ | ❌ | ❌ |
| Hooks config UI | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| Cascade flow tracker | ✅ | ❌ | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ |
| Steering files UI | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| MCP server manager UI | ✅ | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| Auto-memories UI | ✅ | ✅ | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ |
| Shadow workspace lint | ✅ | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| DiffReview per-hunk | ✅ | ✅ | ✅ | ❌ | ❌ | ❌ | ✅ | ✅ |
| Figma import | ✅ | ❌ | ❌ | ❌ | ❌ | ✅ | ❌ | ❌ |
| Neovim plugin | ✅ | ❌ | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ |
| JetBrains plugin | ✅ | ❌ | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ |
| VS Code extension | ✅ | ✅ | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ |

---

## Part R — Next Horizon Gaps (Post Phase 37)

These represent the remaining frontier for VibeCLI + VibeUI competitive parity:

| Gap | Impact | Effort | Competitor | Notes |
|-----|--------|--------|------------|-------|
| Multiplayer / real-time CRDT collab | High | XL | Replit/Lovable/Zed | `vibe-collab` crate (yrs) planned |
| Custom domain / publish UX | Medium | M | Base44/Lovable | Deploy panel missing final step |
| Ambient agent session sharing | Medium | L | Warp/Claude Code | Share link → browser session view |
| @jira / @github issue context | Medium | M | Continue.dev | Extend resolve_at_references |
| MCP OAuth 2.0 install flow | Medium | L | Cursor | tauri-plugin-oauth + PKCE |
| LSP diagnostics in TUI | Low | L | opencode | Wire vibe-lsp into TUI |
| GCP / Firebase deploy target | Low | M | Antigravity | Add to DeployPanel |
| Security scanning (OWASP/CWE patterns) | Medium | M | Amazon Q | ✅ Phase 38 + Phase 41 (15 CWE patterns) |
| Red team / autonomous pentest pipeline | High | L | Shannon (KeygraphHQ) | ✅ Phase 41: redteam.rs 5-stage pipeline |
| Zed-style GPU terminal | Low | XL | Warp/Zed | Out of scope (webview limitation) |
| Remote VM / cloud agent execution | Low | XL | Warp | Cloud infrastructure required |

---

## Part R — Code Complete Workflow (Phase 40)

Phase 40 introduces a structured application development workflow based on Steve McConnell's
*Code Complete* (2nd Edition). The system guides developers through 8 stages from requirements
gathering to release-ready code, with AI-generated checklists at each quality gate.

### Phase 40 Deliverables

| Component | File | Description |
|-----------|------|-------------|
| Workflow engine | `vibecli/vibecli-cli/src/workflow.rs` | WorkflowStage (8), ChecklistItem, StageData, Workflow, WorkflowManager; 11 unit tests |
| LLM prompts | `workflow.rs` | `stage_checklist_prompt()` — per-stage guidance (requirements, architecture, design, etc.) |
| Checklist parser | `workflow.rs` | `parse_checklist_response()` — numbered/bulleted list extraction from LLM output |
| REPL commands | `/workflow new\|list\|show\|advance\|check\|generate` | Full interactive workflow management |
| VibeUI panel | `WorkflowPanel.tsx` | Pipeline visualization (8 circles), checklist toggles, AI generate, 80% advance gate |
| Tauri commands | 6 commands | list_workflows, get_workflow, create_workflow, advance_workflow_stage, update_workflow_checklist_item, generate_stage_checklist |
| File format | `.vibecli/workflows/*.md` | YAML front-matter + `## Stage:` sections + `### Checklist` with `- [ ]` items |

### The 8 Stages

1. **Requirements** — Functional/non-functional reqs, user stories, scope, data constraints
2. **Architecture** — Subsystem decomposition, data storage, security, error strategy, build-vs-buy
3. **Design** — Classes, interfaces, patterns, algorithms, coupling/cohesion, edge cases
4. **Construction Planning** — Language/framework, coding standards, CI/CD, integration order, estimates
5. **Coding** — Naming, defensive programming, DRY, control flow, comments, input validation
6. **Quality Assurance** — Code review, unit tests, static analysis, security scan, performance
7. **Integration & Testing** — E2E, regression, load testing, cross-platform, API validation
8. **Code Complete** — All features done, docs updated, no TODOs, version tagged, deploy runbook

---

## Part S — Shannon Comparison (Phase 41)

VibeCody Phase 41 adds an autonomous red team security scanning module inspired by
[Shannon](https://github.com/KeygraphHQ/shannon). See full comparison at
[`docs/SHANNON-COMPARISON.md`](/shannon-comparison/).

Key differences:
- **Shannon**: Standalone pentesting tool, TypeScript + Temporal + Docker, Claude-primary, ~$50/scan, AGPL-3.0
- **VibeCody RedTeam**: Integrated into development workflow, Rust + Tokio, 10+ LLM providers, per-token cost, MIT

### Phase 41 Deliverables

| Component | File | Description |
|-----------|------|-------------|
| Red team pipeline | `vibecli/vibecli-cli/src/redteam.rs` | 5-stage pipeline: Recon → Analysis → Exploitation → Validation → Report |
| Expanded CWE scanner | `vibecli/vibecli-cli/src/bugbot.rs` | 15 CWE patterns (7 original + 8 new: SSRF, XXE, deserialization, NoSQL, template injection, IDOR, CSRF, cleartext) |
| CLI flags | `--redteam <url>`, `--redteam-config`, `--redteam-report` | Non-interactive scanning mode |
| REPL commands | `/redteam scan\|list\|show\|report\|config` | Interactive security scanning |
| VibeUI panel | `RedTeamPanel.tsx` | Pipeline visualization, findings feed, report export |
| Tauri commands | 5 commands | start_redteam_scan, get_redteam_sessions, get_redteam_findings, generate_redteam_report, cancel_redteam_scan |
| Config | `[redteam]` in config.toml | max_depth, timeout_secs, parallel_agents, scope_patterns, auto_report |
| Comparison doc | `docs/SHANNON-COMPARISON.md` | Full Shannon vs VibeCody feature matrix |

---

*Updated 2026-02-26 — reflects all phases 12–41 complete (including Phase 40 Code Complete Workflow). All file paths reference the VibeCody monorepo at github.com/TuringWorks/vibecody.*
