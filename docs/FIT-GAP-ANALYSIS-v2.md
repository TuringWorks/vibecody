# VibeCody Fit-Gap Analysis v2 — Full Competitive Landscape
**Date:** 2026-02-25
**VibeCLI competitors:** Codex CLI, Warp 2.0, Kiro, opencode, Claude Code
**VibeUI competitors:** Antigravity (Google), Cursor, Windsurf, Replit, Base44, Lovable

> **Status:** All Phases 12–15 items are ✅ complete. This document covers the next horizon.

---

## Part A — VibeCLI Competitive Analysis

### A.1 Feature Matrix

| Feature | VibeCLI | Codex CLI | Warp 2.0 | Kiro | opencode | Claude Code |
|---------|---------|-----------|----------|------|----------|-------------|
| Multi-turn REPL | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Agent loop + tools | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Plan mode | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Session resume | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Multi-provider support | ✅ (5) | ✅ (1) | ✅ | ✅ | ✅ (75+) | ✅ (1) |
| MCP client | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Hooks (pre/post tool use) | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Skills / slash commands | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Git integration + PR review | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Parallel multi-agent | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| HTTP daemon (`serve`) | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ |
| Agent SDK (Node.js) | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ |
| OpenTelemetry tracing | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ |
| VS Code extension | ✅ | ❌ | ✅ | ✅ | ❌ | ✅ |
| JetBrains plugin | ✅ | ❌ | ❌ | ❌ | ❌ | ✅ |
| Named profiles | ✅ | ❌ | ✅ | ❌ | ❌ | ❌ |
| Auto memory recording | ✅ | ❌ | ✅ | ✅ | ❌ | ✅ |
| Rules directory | ✅ | ✅ | ✅ | ✅ | ❌ | ✅ |
| Wildcard tool permissions | ✅ | ✅ | ✅ | ✅ | ❌ | ✅ |
| opusplan model routing | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ |
| **OS-level sandboxing** | ✅ | ✅ | ❌ | ❌ | ❌ | ❌ |
| **Network sandboxing** | ✅ | ✅ | ❌ | ❌ | ❌ | ❌ |
| **Spec-driven development** | ✅ | ❌ | ❌ | ✅ | ❌ | ❌ |
| **Steering files** | ✅ | ❌ | ✅ | ✅ | ❌ | ❌ |
| **File-event hooks (save/create/delete)** | ✅ | ❌ | ❌ | ✅ | ❌ | ❌ |
| **Git worktree isolation per subagent** | ✅ | ❌ | ✅ | ❌ | ❌ | ✅ |
| **SQLite session storage** | ❌ | ❌ | ❌ | ❌ | ✅ | ❌ |
| **Team knowledge store (Drive)** | ❌ | ❌ | ✅ | ❌ | ❌ | ❌ |
| **Ambient agent session sharing** | ❌ | ❌ | ✅ | ❌ | ❌ | ✅ |
| **Slack/Linear/GitHub integrations** | ❌ | ❌ | ✅ | ❌ | ❌ | ❌ |
| **75+ LLM provider support** | ❌ | ❌ | ❌ | ❌ | ✅ | ❌ |
| **GitHub Copilot auth** | ❌ | ❌ | ❌ | ❌ | ✅ | ❌ |
| **`background: true` ambient agents** | ❌ | ❌ | ✅ | ❌ | ❌ | ✅ |
| **Vim-like TUI editor** | ❌ | ❌ | ❌ | ❌ | ✅ | ❌ |
| **Web-viewable agent sessions** | ❌ | ❌ | ✅ | ❌ | ❌ | ❌ |

---

### A.2 Competitor Deep-Dive

#### Codex CLI (OpenAI)
- **Approval modes:** read-only / auto (default) / full-access; `/permissions` mid-session switching
- **OS sandbox:** macOS Seatbelt, Docker, Windows Sandbox — default blocks network + limits filesystem writes
- **Network approval:** rich host/protocol context shown in prompts; structured network approval IDs per command
- **Context:** `@` fuzzy file search in composer (Tab/Enter to drop path); AGENTS.md; MCP; per-command approval IDs
- **Codex App:** separate GUI app with visual project management
- **Codex SDK:** programmatic agent creation via OpenAI Agents SDK
- **Gap VibeCLI must close:** OS-level sandboxing, network sandboxing, richer per-command approval IDs

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

**Feature-by-Feature vs VibeCLI:**

| Warp Feature | VibeCLI Status | Phase |
|-------------|----------------|-------|
| Warp Drive (team knowledge store) | ❌ Missing | 17 |
| Ambient agent session sharing | ❌ Missing | 17 |
| Web viewer for agent sessions | ❌ Missing | 17 |
| Slack/Linear integration | ❌ Missing | 17 |
| Named shared commands | Partial (skills/) | 17 |
| Multi-agent status UI | ✅ Manager view | — |
| Notebook-style runnable docs | ❌ Missing | 17 |
| Remote VM agent execution | ❌ Missing | 17 |
| SSH + Warpify | ❌ (no terminal) | N/A |
| GPU-accelerated TUI | ❌ Missing | — |
| Codebase indexing (120K repos) | ✅ EmbeddingIndex | — |
| Natural language → command | ✅ Agent loop | — |
| Per-agent autonomy settings | Partial | 17 |

**Gap VibeCLI must close:** Warp Drive equivalent, ambient session sharing, Slack/Linear integration

#### Kiro (Amazon, VS Code fork)
- **Spec-driven development:** NL requirements → user stories + acceptance criteria → technical design → task list (tick-able)
- **Steering files:** project-scope context (coding standards, workflows, tool preferences) — analogous to `.cursorrules` but more structured
- **Agent hooks:** automated triggers on file-system events (save, create, delete) — not just tool-use hooks
- **MCP:** with remote MCP server support
- **Gap VibeCLI must close:** spec-to-code workflow, file-event hooks (save/create/delete triggers)

#### opencode (opencode-ai/opencode, sst/opencode)

opencode is the **most technically ambitious** open-source CLI agent in 2026, built in Go by the SST team.

**Provider Ecosystem (biggest advantage):**
75+ LLM providers via a unified abstraction layer:
- **Cloud:** Claude (all versions), GPT-4o/o3, Gemini 1.5/2.0, Grok
- **Infrastructure:** AWS Bedrock (Llama, Titan, Anthropic on AWS), Azure OpenAI, Groq
- **Aggregators:** OpenRouter (300+ models including open-weights), Together AI
- **Local:** Ollama, LM Studio, vLLM
- **Specialized:** Kimi (Moonshot), DeepSeek, Qwen3 Coder, Mistral, Cohere
- **Jan 2026:** Official GitHub Copilot OAuth integration (enterprise auth path for companies with GitHub Business/Enterprise)

**TUI Architecture:**
- Built with **Bubble Tea** (Go TUI framework — equivalent to Ratatui)
- **Vim-like keybindings** throughout: `j/k` navigation, `:` command mode
- Split-pane layout: file tree + conversation + output
- **Rich syntax highlighting** in all code blocks (chroma library)
- Smooth animations, real-time streaming, no flicker

**Session Management (SQLite):**
- All sessions in `~/.config/opencode/sessions.db` (SQLite)
- Queryable: `opencode sessions --search "react"` full-text search
- Sessions stored as: metadata + messages + tool calls + artifacts
- `VACUUM` on exit keeps DB compact
- Atomic writes (no partial session corruption)

**LSP Integration:**
- Real-time LSP diagnostics shown inline in TUI
- When agent writes code, LSP errors appear in conversation
- Goto-definition, hover docs available in TUI file browser
- Language server configured automatically from workspace

**Tool Execution:**
- Same tool set (bash, read_file, write_file, search) but **permission model** is simpler (yes/no per tool type)
- No hooks system
- No admin policy
- No opusplan routing

**MCP:**
- Full MCP client with stdio + SSE transport
- No GUI for MCP management (CLI only: `opencode mcp add <name> <cmd>`)

**Feature-by-Feature vs VibeCLI:**

| opencode Feature | VibeCLI Status | Phase |
|-----------------|----------------|-------|
| 75+ providers | ✅ 5 providers (needs expansion) | 17 |
| SQLite sessions | ❌ JSONL (needs migration) | 17 |
| Vim-like TUI keybindings | Partial (basic keybindings) | 17 |
| GitHub Copilot OAuth | ❌ Missing | 17 |
| LSP in TUI | ❌ (LSP only in VibeUI) | — |
| AWS Bedrock | ❌ Missing | 17 |
| Groq ultra-fast inference | ✅ groq.rs (OpenAI-compatible) | 17 ✅ |
| Azure OpenAI | ✅ azure_openai.rs (deployment URL) | 17 ✅ |
| OpenRouter (300+ models) | ✅ openrouter.rs (300+ via unified API) | 17 ✅ |
| Hooks system | ❌ Missing in opencode | VibeCLI wins |
| Admin policy | ❌ Missing in opencode | VibeCLI wins |
| HTTP daemon / SDK | ❌ Missing in opencode | VibeCLI wins |
| MCP GUI | ❌ Missing in opencode | VibeCLI wins |
| PR code review | ❌ Missing in opencode | VibeCLI wins |
| OTel tracing | ❌ Missing in opencode | VibeCLI wins |
| Multi-agent | ❌ opencode single-agent | VibeCLI wins |
| Named profiles | ❌ Missing in opencode | VibeCLI wins |
| Session full-text search | ❌ Missing in VibeCLI | 17 |
| Rich diff syntax highlighting | Partial | 17 |

**Gap VibeCLI must close:** Provider breadth (Bedrock/Groq/Azure/OpenRouter), SQLite sessions, Copilot OAuth, Vim keybindings improvement, LSP in TUI

#### Claude Code (Anthropic)
- **Subagents:** up to 7 parallel with `--worktree (-w)` git worktree isolation
- **Background agents:** `background: true` in agent definition — always runs async
- **Hooks:** SubagentStop event with `last_assistant_message` field
- **CLAUDE.md hierarchy:** 4-level loading (home → repo root → subfolder → current dir)
- **MCP OAuth:** full OAuth2 for MCP server auth
- **Gap VibeCLI must close:** per-subagent worktree isolation, background/ambient agent definitions

---

### A.3 Extended Competitor — PicoClaw

**PicoClaw** (github.com/sipeed/picoclaw) is a breakout competitor launched February 9, 2026 — 12,000 GitHub stars in its first week. It is NOT a CLI coding agent like VibeCLI; it is an **ultra-lightweight personal AI assistant** targeting edge hardware and chat gateway use cases. However, several of its architectural choices are directly applicable to VibeCLI's roadmap.

#### What PicoClaw Is
- **Language:** Go (rewrite of OpenClaw/Moltbot/Clawdbot which was Python ~430k lines)
- **Size:** <10MB RAM, <1 second cold start (vs. OpenClaw 500s+ startup)
- **Hardware:** Runs on $10 RISC-V boards (Sipeed LicheeRV Nano, SG2002 SoC), ARM64, x86
- **95% AI-generated code:** self-bootstrapping — the AI agent wrote its own Go implementation
- **Two modes:** `picoclaw agent` (interactive CLI), `picoclaw gateway` (24/7 daemon connecting to messaging platforms)

#### PicoClaw Feature Set

| Feature | Description |
|---------|-------------|
| **Skills system** | `SKILL.md` with YAML frontmatter (name, tools, description) + bundled scripts/assets; distributable as `.tar.gz` packages |
| **Tool calling** | File ops, web search (Tavily/Brave), bash execution, scheduling, subagent spawning — all JSON Schema-defined |
| **Web search** | Tavily (AI-optimized, 1000 req/mo free) + Brave Search (2000 req/mo free) — better than DuckDuckGo Lite |
| **Messaging gateway** | Telegram, Discord, QQ, DingTalk — runs 24/7 as background service |
| **Scheduling/cron** | Built-in cron: one-time reminders, recurring tasks, cron expressions |
| **Subagent spawning** | Tools can spawn child agents; results piped back to parent |
| **LLM support** | OpenRouter (all models), Claude, OpenAI, Gemini, DeepSeek, Groq, Zhipu |
| **Single binary** | `curl | sh` install; no runtime dependencies |

#### PicoClaw vs VibeCLI Feature Gap

| PicoClaw Feature | VibeCLI Status | Roadmap Phase |
|-----------------|----------------|---------------|
| Single binary releases (cargo dist / goreleaser) | Partial (cargo build only) | 21 |
| Messaging gateway (Telegram/Discord/Slack) | ❌ Missing | 21 |
| AI-optimized web search (Tavily) | Partial (DuckDuckGo Lite) | 21 |
| Built-in cron/scheduling | ❌ Missing | 21 |
| Subagent spawning from tools | ❌ (multi-agent via CLI only) | 21 |
| Skills as distributable packages | Partial (simple markdown files) | 21 |
| Cold start <1 second | Partial (~2-5s Rust startup) | — |
| ARM64/RISC-V binary | ❌ (x86/ARM64 needed) | 21 |

#### PicoClaw Skills That VibeCLI Should Adopt
The `SKILL.md` format is richer than VibeCLI's skills:
```yaml
# SKILL.md frontmatter
name: code-reviewer
description: Review code for issues and improvements
tools:
  - read_file
  - web_search
  - bash
triggers:
  - "review|check|audit"
resources:
  - checklists/security.md
  - checklists/performance.md
```

VibeCLI's skills are simpler markdown prompts — they should gain:
1. Tool declarations (which tools the skill requires)
2. Pattern triggers (regex/keywords that auto-activate)
3. Bundled resources (reference docs, checklists)
4. Distributable packaging (`.vibecli-skill.tar.gz`)

#### Unique PicoClaw Insights for VibeCLI Roadmap
1. **Tavily web search** outperforms DuckDuckGo Lite for agent use cases (structured JSON results, relevance scoring)
2. **Gateway mode** enables VibeCLI to run as a Slack/Teams/Discord bot — huge enterprise use case
3. **Scheduling** fills a major gap: users want to say "run this agent every night at 2am"
4. **Distributable skill packages** create an ecosystem (skill marketplace potential)
5. **Single binary with `curl | sh`** is a UX differentiator for adoption

---

## Part B — VibeUI Competitive Analysis

### B.1 Feature Matrix

| Feature | VibeUI | Antigravity | Cursor | Windsurf | Replit | Base44 | Lovable |
|---------|--------|-------------|--------|----------|--------|--------|---------|
| Code editor | ✅ | ✅ | ✅ | ✅ | ✅ | ❌ | ❌ |
| AI chat panel | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Agent (multi-file edits) | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Inline Cmd+K chat | ✅ | ✅ | ✅ | ✅ | ✅ | ❌ | ❌ |
| Tab next-edit prediction | ✅ | ✅ | ✅ | ✅ | ✅ | ❌ | ❌ |
| Terminal panel | ✅ | ✅ | ✅ | ✅ | ✅ | ❌ | ❌ |
| Git integration | ✅ | ✅ | ✅ | ✅ | ✅ | ❌ | ✅ |
| LSP / code intelligence | ✅ | ✅ | ✅ | ✅ | ❌ | ❌ | ❌ |
| MCP client | ✅ | ✅ | ✅ | ✅ | ❌ | ❌ | ❌ |
| Multi-provider | ✅ | ✅ | ✅ | ✅ | ❌ | ❌ | ❌ |
| Voice input | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| Background job persistence | ✅ | ❌ | ✅ | ❌ | ❌ | ❌ | ❌ |
| @web context | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| Browser preview panel | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Artifact system | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| Parallel Manager view | ✅ | ✅ | ✅ | ✅ | ❌ | ❌ | ❌ |
| Hooks config UI | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| Cascade flow tracker | ✅ | ❌ | ❌ | ✅ | ❌ | ❌ | ❌ |
| DiffReviewPanel (per-hunk) | ✅ | ❌ | ✅ | ✅ | ❌ | ❌ | ❌ |
| Linter integration | ✅ | ❌ | ✅ | ✅ | ✅ | ❌ | ❌ |
| **Visual UI Editor (drag-drop)** | ✅ | ✅ | ✅ | ❌ | ❌ | ✅ | ✅ |
| **Automated PR review (BugBot)** | ✅ | ❌ | ✅ | ❌ | ❌ | ❌ | ❌ |
| **Memories (auto-generated)** | ✅ | ✅ | ✅ | ✅ | ❌ | ❌ | ❌ |
| **Shadow workspace (bg lint)** | ✅ | ❌ | ✅ | ❌ | ❌ | ❌ | ❌ |
| **One-click deployment** | ✅ | ✅ | ❌ | ❌ | ✅ | ✅ | ✅ |
| **Built-in database UI** | ✅ | ✅ | ❌ | ❌ | ✅ | ✅ | ✅ |
| **Auth + backend scaffolding** | ❌ | ✅ | ❌ | ❌ | ✅ | ✅ | ✅ |
| **GitHub bidirectional sync** | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ✅ |
| **Supabase integration** | ❌ | ✅ | ❌ | ❌ | ✅ | ❌ | ✅ |
| **Multiplayer / real-time collab** | ❌ | ❌ | ❌ | ❌ | ✅ | ❌ | ✅ |
| **Browser-embedded app testing** | ❌ | ❌ | ❌ | ❌ | ✅ | ❌ | ❌ |
| **Figma import** | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ✅ |
| **Custom domain / publish** | ❌ | ✅ | ❌ | ❌ | ✅ | ✅ | ✅ |
| **Design mode** | ✅ | ✅ | ✅ | ❌ | ❌ | ✅ | ✅ |
| **Multi-IDE plugin** | ❌ | ✅ | ❌ | ✅ | ❌ | ❌ | ❌ |
| **Point-and-prompt in live app** | ✅ | ✅ | ✅ | ❌ | ❌ | ❌ | ❌ |
| **Custom SWE model (SWE-1)** | ❌ | ✅ | ❌ | ✅ | ❌ | ❌ | ❌ |
| **WASM extension system** | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |

---

### B.2 Competitor Deep-Dive

#### Google Antigravity (Nov 2025, Gemini 3)
- Agent-first IDE from Google, deeply integrated with Gemini 3 and Google Cloud/Firebase
- **Unique:** native Firebase/GCP one-click deploy, Gemini 3 multimodal (image-to-code, video understanding)
- **Gap VibeUI must close:** GCP/Firebase deployment, multimodal file attachment in chat

#### Cursor 2.0+
- **Composer:** up to 8 parallel agents using git worktrees or remote machines
- **Shadow workspace:** background hidden window that runs lint checks on AI-proposed code before showing diffs
- **BugBot:** automated PR reviewer triggered on every PR; "Fix in Cursor" deep-link to problematic code
- **Memories:** auto-generated from conversations, persistent per-project knowledge
- **Visual Editor:** drag-drop elements in rendered live app + property sliders + point-and-prompt natural language UI editing
- **Background agents:** high-risk (network/FS access) and low-risk categories
- **One-click MCP install** with OAuth
- **Gap VibeUI must close:** Visual Editor, BugBot, Memories auto-generation, shadow workspace

#### Windsurf (Codeium)
- **Cascade:** tracks ALL actions (edits + terminal commands + clipboard + conversation) to infer intent — no re-prompting needed
- **SWE-1 model family:** purpose-built for software engineering (not general-purpose); free to use
- **Supercomplete:** cross-file multi-line prediction
- **Memories:** automatically generated by Cascade, persist across sessions
- **Wave 13:** parallel multi-agent sessions, side-by-side Cascade panes, Git worktrees
- **Forge:** Chrome extension for code review
- **Multi-IDE:** VS Code, JetBrains, Neovim, Sublime Text, Jupyter, Colab
- **Gap VibeUI must close:** SWE-1-style fine-tuned model support, full cross-IDE plugin support

#### Replit
- **Agent 3:** 10x more autonomous; browser-embedded testing (AI controls cursor in live app)
- **Self-healing:** proprietary test system, 3x faster + 10x cheaper than Computer Use
- **Built-in stack:** auth + database (PostgreSQL) + hosting + monitoring in one click
- **Ghostwriter:** completions, explain, fix, test generation, refactor
- **Multiplayer:** real-time collaboration on same Repl
- **Zero setup:** browser-based, instant environments, 22.5M users
- **Gap VibeUI must close:** browser-automated app testing, built-in DB+auth+hosting, multiplayer

#### Base44 (acquired by Wix)
- **All-in-one:** UI + database + auth + hosting — no external services
- **Press Publish → live:** zero deployment friction
- **Built-in analytics + custom domains**
- **Natural language → full app** in minutes, no code required
- **Gap VibeUI must close:** self-contained deploy stack, press-to-publish UX

#### Lovable 2.0
- **Full-stack generation:** React + Supabase from NL description
- **GitHub bidirectional sync:** real-time sync to/from GitHub repo
- **Supabase integration:** auth, database, storage out-of-the-box
- **Design mode:** dedicated visual editing mode separate from chat
- **Multiplayer:** real-time collaborative editing (April 2025)
- **Figma import:** convert Figma designs to code
- **Deploy:** Netlify, Vercel, custom domains
- **Error debugging:** auto-detect and fix runtime errors
- **Gap VibeUI must close:** GitHub sync, Supabase integration, design mode, Figma import

---

## Part C — VibeCody Exclusive Advantages

Features VibeCody has that **no competitor offers:**

| Feature | Why Unique |
|---------|-----------|
| **HTTP daemon + REST API** | `vibecli serve` — enables SDK, VibeUI, JetBrains/VS Code integration from one process |
| **Node.js Agent SDK** | `packages/agent-sdk/` — only CLI tool with a programmable streaming SDK |
| **OpenTelemetry OTLP tracing** | Observability-first; no competitor exports spans to Jaeger/Zipkin/Grafana |
| **Voice input (Web Speech API)** | 🎤 button in AIChat — no competitor has it cleanly |
| **Admin policy (glob-based)** | Enterprise tool restriction with wildcard deny/allow patterns |
| **WASM extension system** | `vibe-extensions` with wasmtime — sandboxed plugin runtime |
| **opusplan routing** | Separate planning vs execution model per request |
| **Artifact panel** | Structured AI output typed and stored persistently |
| **@web context (DuckDuckGo)** | Full HTML fetch + search engine results in agent context |
| **Multi-agent Manager view** | Visual UI for parallel agent execution with branch merging |
| **Hooks config UI** | Visual editor for hooks with LLM handler support |
| **Background job persistence** | Jobs survive daemon restart; cancel/stream endpoints |

---

## Part D — Gap Priority Matrix (New Phases 16–20)

| Gap | Impact | Effort | Phase | Competitor |
|-----|--------|--------|-------|------------|
| Spec-driven dev (specs + tasks) | Critical | L | 16 | Kiro |
| Steering files | High | S | 16 | Kiro/Warp |
| File-event hooks (save/create/delete) | High | M | 16 | Kiro |
| Spec UI panel in VibeUI | High | M | 16 | Kiro |
| OS-level sandboxing | High | XL | 16 | Codex |
| Network sandboxing per command | Medium | L | 16 | Codex |
| Git worktree isolation per subagent | High | L | 16 | Claude Code |
| SQLite session storage | Medium | L | 17 | opencode |
| 75+ provider support (Bedrock/Groq/Azure) | High | XL | 17 ✅ | opencode |
| GitHub Copilot auth | Medium | M | 17 | opencode |
| Ambient agent definitions (`background: true`) | High | M | 17 ✅ | Claude Code |
| Team knowledge store (Warp Drive equivalent) | High | XL | 17 ✅ | Warp |
| Agent session sharing / web viewer | Medium | XL | 17 | Warp |
| Slack/Linear/GitHub Actions integration | Medium | L | 17 | Warp |
| Memories auto-generation | Critical | L | 18 ✅ | Cursor/Windsurf |
| Shadow workspace (bg lint worker) | High | M | 18 ✅ | Cursor |
| BugBot automated PR review | High | M | 18 ✅ | Cursor |
| Visual UI Editor (drag-drop in live app) | Critical | XL | 19 ✅ | Cursor/Antigravity |
| Point-and-prompt in browser panel | High | L | 19 ✅ | Cursor |
| Design mode (visual editing tab) | High | L | 19 ✅ | Lovable/Antigravity |
| Figma import | Medium | L | 19 ✅ | Lovable |
| One-click deployment (Vercel/Netlify/Railway) | Critical | L | 20 ✅ | All |
| Built-in database UI (SQLite/Postgres) | High | L | 20 ✅ | Replit/Base44 |
| Supabase integration | High | M | 20 | Lovable/Replit |
| Auth scaffolding (OAuth, JWT) | High | M | 20 | Replit/Base44 |
| GitHub bidirectional sync | High | L | 20 | Lovable |
| Browser-embedded app testing | High | XL | 20 | Replit |
| Multiplayer / real-time collaboration | Medium | XL | 20 | Lovable/Replit |
| Custom domain deployment | Medium | M | 20 | Base44/Lovable |

---

## Part E — Phase 16: Spec-Driven Development

**Goal:** Implement Kiro-style spec-driven workflow for both VibeCLI and VibeUI.
**Win condition:** Developer writes requirements in NL → VibeCody generates spec → agent executes from spec.

### E.1 Spec System for VibeCLI

**New file: `vibecli/vibecli-cli/src/spec.rs`**
```rust
/// A spec file lives at .vibecli/specs/<name>.md
/// Front-matter (TOML) + markdown body
pub struct Spec {
    pub name: String,
    pub status: SpecStatus,    // "draft" | "approved" | "in-progress" | "done"
    pub requirements: String,  // original NL requirements
    pub tasks: Vec<SpecTask>,  // generated task list
    pub design: String,        // technical design section
}

pub struct SpecTask {
    pub id: u32,
    pub description: String,
    pub done: bool,
}
```

**New REPL commands:**
- `/spec new <name>` → LLM generates spec from current task context
- `/spec show <name>` → display spec with task checklist
- `/spec run <name>` → run agent against each pending task in order
- `/spec list` → list all specs in `.vibecli/specs/`
- `/spec done <name> <task-id>` → mark task complete

**New CLI flags:**
- `--spec <name>` → run agent against approved spec (CI mode)
- `--spec-new <requirements>` → create and immediately execute a spec

**Spec generation prompt:**
```
Given these requirements: {requirements}

Generate a spec with:
1. User stories (Given/When/Then format)
2. Acceptance criteria (bullet list)
3. Technical design (architecture decisions)
4. Task list (numbered, atomic, implementable)

Output as markdown with TOML front-matter.
```

**Files to modify:**
- `vibecli/vibecli-cli/src/main.rs` — add `--spec`, `--spec-new` flags
- `vibecli/vibecli-cli/src/repl.rs` — add `/spec` commands to COMMANDS array
- `vibecli/vibecli-cli/src/agent.rs` — wire spec tasks as context in system prompt

---

### E.2 Steering Files

Steering files provide project-scope context injected at the top of every agent system prompt — more powerful than per-session rules.

**Location:** `.vibecli/steering/` (project-level) + `~/.vibecli/steering/` (global)

**File format:**
```toml
# .vibecli/steering/architecture.md
---
scope = "project"      # "project" | "global" | "path:<glob>"
always_include = true
---
# Architecture Decisions

This is a Rust monorepo. All shared logic lives in vibeui/crates/.
...
```

**Files to modify:**
- `vibeui/crates/vibe-ai/src/rules.rs` — extend `RulesLoader` to also load `steering/` directory; steering files always inject (no path gating)
- `vibecli/vibecli-cli/src/main.rs` — load steering files into `AgentContext.system_extra`
- New Tauri command `get_steering_files()` / `save_steering_file()` for VibeUI
- New `SteeringPanel.tsx` — CRUD UI for steering files (alongside MemoryPanel)

---

### E.3 File-Event Agent Hooks (Kiro-Style)

Extend the hooks system to fire on filesystem events, not just tool-use events.

**New `HookEvent` variants in `vibe-ai/src/hooks.rs`:**
```rust
pub enum HookEvent {
    // existing variants...
    FileSaved { path: String, content: String, language: String },
    FileCreated { path: String },
    FileDeleted { path: String },
}
```

**Config example:**
```toml
[[hooks]]
event = "FileSaved"
paths = ["**/*.rs"]
handler = { command = "cargo clippy --message-format json 2>&1" }

[[hooks]]
event = "FileCreated"
paths = ["src/components/*.tsx"]
handler = { command = "npx tsc --noEmit" }
```

**Files to modify:**
- `vibeui/crates/vibe-ai/src/hooks.rs` — add `FileSaved`, `FileCreated`, `FileDeleted` variants; add `paths: Vec<String>` field to `HookConfig`
- `vibeui/src-tauri/src/commands.rs` — fire `FileSaved` hook in `write_file` handler
- `vibeui/src/components/HooksPanel.tsx` — add "File Event Hooks" section with event type + path glob

---

### E.4 Spec UI Panel in VibeUI

**New file: `vibeui/src/components/SpecPanel.tsx`**
- Two tabs: "Specs" (list) + "Editor" (markdown spec view with task checklist)
- "New Spec" form: name + requirements textarea → calls `generate_spec` Tauri command
- Task list: checkbox per task; completion syncs back to spec file
- "Run Agent on Spec" button → calls `start_agent_task` with spec context injected
- Status badges: Draft / Approved / In Progress / Done

**New Tauri commands in `commands.rs`:**
- `list_specs()` → reads `.vibecli/specs/*.md`
- `get_spec(name)` → returns parsed spec
- `generate_spec(name, requirements)` → LLM generates spec, writes to disk
- `update_spec_task(name, task_id, done)` → toggles task completion
- `run_spec(name)` → fires agent with spec as context

**`App.tsx`:** Add "📋 Specs" as 13th AI panel tab.

---

### E.5 OS-Level Sandboxing

**Target:** wrap agent bash tool execution in OS sandbox when `config.safety.sandbox = true`.

**macOS (Seatbelt / sandbox-exec):**
```rust
// tool_executor.rs
fn build_sandbox_profile(workspace: &Path, allow_network: bool) -> String {
    format!(r#"
(version 1)
(deny default)
(allow process-exec (literal "/bin/sh") (literal "/usr/bin/env"))
(allow file-read* (subpath "{workspace}"))
(allow file-write* (subpath "{workspace}"))
{network_rule}
"#,
        workspace = workspace.display(),
        network_rule = if allow_network { "(allow network*)" } else { "(deny network*)" }
    )
}

fn exec_sandboxed(cmd: &str, workspace: &Path) -> Result<String> {
    let profile = build_sandbox_profile(workspace, false);
    let profile_file = tempfile_with_content(&profile)?;
    std::process::Command::new("sandbox-exec")
        .args(["-f", profile_file.path().to_str().unwrap(), "sh", "-c", cmd])
        .output()
}
```

**Linux (bubblewrap):**
```rust
fn exec_sandboxed_linux(cmd: &str, workspace: &Path) -> Result<String> {
    std::process::Command::new("bwrap")
        .args(["--bind", workspace.to_str().unwrap(), workspace.to_str().unwrap()])
        .args(["--ro-bind", "/usr", "/usr"])
        .args(["--ro-bind", "/lib", "/lib"])
        .args(["--unshare-net"])   // network isolation
        .args(["--", "sh", "-c", cmd])
        .output()
}
```

**Files to modify:**
- `vibecli/vibecli-cli/src/tool_executor.rs` — `exec_bash` and `exec_bash_pty` dispatch to sandboxed variant when `config.safety.sandbox`
- `vibecli/vibecli-cli/src/config.rs` — add `sandbox_profile: Option<String>` to `SafetyConfig`
- `--doctor` — check `sandbox-exec` (macOS) or `bwrap` (Linux) availability

---

### E.6 Git Worktree Isolation per Subagent

Give each subagent its own git worktree so parallel agents don't conflict on the same files.

**New trait method in `WorktreeManager`:**
```rust
pub trait WorktreeManager: Send + Sync {
    fn create_worktree(&self, branch: &str) -> Result<PathBuf>;
    fn delete_worktree(&self, path: &Path) -> Result<()>;
    fn merge_worktree(&self, path: &Path, base: &str) -> Result<MergeResult>;
    // NEW:
    fn create_isolated_worktree(&self, agent_id: &str) -> Result<IsolatedWorktree>;
}

pub struct IsolatedWorktree {
    pub path: PathBuf,
    pub branch: String,
    pub agent_id: String,
}

impl Drop for IsolatedWorktree {
    fn drop(&mut self) { /* auto-cleanup on scope exit */ }
}
```

**`multi_agent.rs` changes:**
- Each `spawn_agent()` call gets its own `IsolatedWorktree`
- Worktree path passed as `workspace_root` in the agent's `AgentContext`
- `--worktree` CLI flag mirrors Claude Code's `-w` flag

---

## Part F — Phase 17: Provider Expansion & Team Features

**Goal:** Match opencode's 75+ providers, add ambient agents, team collaboration primitives.

### F.1 Provider Expansion (AWS Bedrock, Groq, Azure, OpenRouter)

**New provider files in `vibeui/crates/vibe-ai/src/providers/`:**

**`bedrock.rs`** — AWS Bedrock:
```rust
pub struct BedrockProvider {
    client: BedrockRuntimeClient,  // aws-sdk-bedrockruntime
    model_id: String,              // "anthropic.claude-3-5-sonnet-20241022-v2:0"
    region: String,
}
// Auth: AWS_ACCESS_KEY_ID + AWS_SECRET_ACCESS_KEY + AWS_REGION env vars
// OR ~/.aws/credentials profile
```

**`groq.rs`** — Groq (OpenAI-compatible, ultra-fast inference):
```rust
pub struct GroqProvider {
    api_key: String,
    model: String,  // "llama-3.3-70b-versatile", "mixtral-8x7b-32768"
    base_url: String,  // "https://api.groq.com/openai/v1"
}
// Reuses OpenAI-compatible chat completions API
```

**`azure_openai.rs`** — Azure OpenAI:
```rust
pub struct AzureOpenAIProvider {
    endpoint: String,        // "https://{resource}.openai.azure.com"
    deployment_id: String,   // Azure deployment name
    api_version: String,     // "2024-12-01-preview"
    api_key: String,
}
```

**`openrouter.rs`** — OpenRouter (300+ models via unified API):
```rust
pub struct OpenRouterProvider {
    api_key: String,
    model: String,   // "anthropic/claude-3.5-sonnet", "google/gemini-pro-1.5"
    // Fully OpenAI-compatible
}
```

**`config.rs` additions:**
```toml
[bedrock]
enabled = true
region = "us-east-1"
model = "anthropic.claude-3-5-sonnet-20241022-v2:0"

[groq]
enabled = true
api_key = "gsk_..."
model = "llama-3.3-70b-versatile"

[azure_openai]
enabled = true
endpoint = "https://myresource.openai.azure.com"
deployment_id = "gpt-4o"
api_key = "..."

[openrouter]
enabled = true
api_key = "sk-or-..."
model = "anthropic/claude-3.5-sonnet"
```

**Files to modify:**
- `vibeui/crates/vibe-ai/src/lib.rs` — export new providers
- `vibecli/vibecli-cli/src/main.rs` — `create_provider()` matches new provider names
- `vibecli/vibecli-cli/src/config.rs` — add `bedrock`, `groq`, `azure_openai`, `openrouter` fields
- **`Cargo.toml`:** add `aws-sdk-bedrockruntime = "1"` to workspace

---

### F.2 SQLite Session Storage

Replace JSONL trace files with SQLite for reliability, queryability, and performance.

**New file: `vibeui/crates/vibe-ai/src/session_store.rs`**
```rust
pub struct SessionStore {
    db: rusqlite::Connection,
}

impl SessionStore {
    pub fn open(path: &Path) -> Result<Self>;
    pub fn create_session(&self, id: &str, task: &str) -> Result<()>;
    pub fn append_message(&self, session_id: &str, msg: &Message) -> Result<()>;
    pub fn append_trace(&self, session_id: &str, event: &TraceEvent) -> Result<()>;
    pub fn load_session(&self, id_prefix: &str) -> Result<Option<SessionSnapshot>>;
    pub fn list_sessions(&self, limit: usize) -> Result<Vec<SessionMeta>>;
    pub fn search_sessions(&self, query: &str) -> Result<Vec<SessionMeta>>;
    pub fn delete_session(&self, id: &str) -> Result<()>;
}
```

**Schema:**
```sql
CREATE TABLE sessions (
    id TEXT PRIMARY KEY, task TEXT, status TEXT,
    provider TEXT, model TEXT, started_at INTEGER, finished_at INTEGER,
    total_tokens INTEGER, cost_usd REAL
);
CREATE TABLE messages (
    id INTEGER PRIMARY KEY, session_id TEXT, seq INTEGER,
    role TEXT, content TEXT, timestamp INTEGER
);
CREATE TABLE trace_events (
    id INTEGER PRIMARY KEY, session_id TEXT, seq INTEGER,
    tool TEXT, summary TEXT, output TEXT, success INTEGER, timestamp INTEGER
);
CREATE INDEX idx_session ON messages(session_id);
CREATE INDEX idx_trace_session ON trace_events(session_id);
```

**Migration:** On first run, import existing JSONL traces into SQLite.
**New REPL commands:** `/sessions search <query>` — full-text search across all sessions.

---

### F.3 Ambient Agent Definitions

Background agents that run without blocking the REPL.

**New config format in `.vibecli/agents/`:**
```toml
# .vibecli/agents/test-runner.toml
name = "test-runner"
background = true           # always runs async
trigger = "file_saved"      # "file_saved" | "on_demand" | "scheduled"
trigger_paths = ["**/*.rs"]
task = "Run cargo test and report failures"
approval_policy = "full-auto"
max_steps = 10
```

**New REPL commands:**
- `/agents list` — list defined background agents
- `/agents start <name>` — start a background agent
- `/agents status` — show running/completed background agent sessions
- `/agents stop <name>` — cancel a running background agent

**Files to modify:**
- New `vibecli/vibecli-cli/src/background_agents.rs` — `BackgroundAgentManager`, polling loop
- `vibecli/vibecli-cli/src/main.rs` — start background agent manager on REPL init
- `vibecli/vibecli-cli/src/serve.rs` — `GET /agents` → list background agent statuses

---

### F.4 Team Knowledge Store (Warp Drive Equivalent)

Centralized team configuration shared across team members.

**New file: `.vibecli/team.toml`** (checked into git):
```toml
[team]
name = "VibeCody Dev Team"
knowledge_base_url = "https://github.com/org/repo/blob/main/.vibecli/knowledge/"

[[shared_commands]]
name = "deploy-staging"
command = "npm run deploy:staging"
description = "Deploy to staging environment"

[[shared_rules]]
name = "api-standards"
path_pattern = "src/api/**"
source = ".vibecli/rules/api-standards.md"

[[shared_mcp]]
name = "github"
command = "npx @modelcontextprotocol/server-github"
```

**REPL commands:**
- `/team sync` — pull latest team.toml from git
- `/team knowledge add <name> <text>` — add entry to shared knowledge
- `/team knowledge list` — show shared knowledge entries

---

## Part G — Phase 18: Memories + Shadow Workspace + BugBot

### G.1 Memories Auto-Generation

**Goal:** After each session, auto-extract reusable facts and append to `~/.vibecli/memory.md` (already exists via `auto_record`). Extend to also generate per-session "learnings" stored per project.

**New: `vibecli/vibecli-cli/src/memory_auto.rs`**
```rust
pub struct MemoryAutoExtractor {
    pub llm: Arc<dyn LLMProvider>,
}

impl MemoryAutoExtractor {
    pub async fn extract(&self, messages: &[Message]) -> Vec<MemoryFact> {
        // Prompt: "Extract 3-5 reusable facts from this session.
        //          Format: { fact: str, confidence: f32, tags: [str] }
        //          Only include stable facts, not task-specific details."
    }

    pub async fn deduplicate(&self, existing: &[MemoryFact], new: &[MemoryFact]) -> Vec<MemoryFact> {
        // LLM-based deduplication: don't add facts already captured
    }
}
```

**VibeUI `MemoryPanel.tsx`:** Add "Auto-Facts" tab showing auto-extracted memories with confidence scores and ability to pin/discard.

**New Tauri command:** `get_auto_memories()` / `delete_auto_memory(id)` / `pin_auto_memory(id)`

---

### G.2 Shadow Workspace (Background Lint Worker)

A background process that continuously runs linters/type-checkers on AI-proposed changes before showing the diff — so the user sees pre-validated diffs.

**New: `vibeui/src-tauri/src/shadow_workspace.rs`**
```rust
pub struct ShadowWorkspace {
    pub path: PathBuf,           // temp dir copy of workspace
    pub linter_results: Arc<Mutex<HashMap<String, LintResult>>>,
}

impl ShadowWorkspace {
    pub fn sync_file(&self, path: &str, content: &str) -> Result<()>;
    pub fn run_lint(&self, path: &str) -> Result<LintResult>;
    pub fn cleanup(&self);
}
```

**Flow:**
1. Agent proposes `write_file` → write to shadow workspace instead of real workspace
2. Run linter on shadow file
3. Show `DiffReviewPanel` annotated with lint errors inline
4. On Accept → write to real workspace

**New Tauri events:** `shadow:lint_result { path, errors, warnings }` — emitted after background lint completes.

---

### G.3 BugBot Automated PR Review

**New file: `vibecli/vibecli-cli/src/bugbot.rs`**
```rust
pub struct BugBot {
    pub llm: Arc<dyn LLMProvider>,
    pub gh_token: Option<String>,
}

impl BugBot {
    /// Run review on all files changed in a PR or diff.
    pub async fn review_diff(&self, diff: &str) -> Vec<BugReport>;
    /// Post review as GitHub PR review with inline comments.
    pub async fn post_github_review(&self, pr: u64, reports: &[BugReport]) -> Result<()>;
}

pub struct BugReport {
    pub file: String,
    pub line: u32,
    pub severity: Severity,   // Error | Warning | Info
    pub message: String,
    pub suggestion: Option<String>,
    pub fix_command: Option<String>,  // "vibecli --agent 'fix line 42 in src/foo.rs'"
}
```

**CLI flags:**
- `vibecli --bugbot --pr 123` — review PR 123 and post inline comments
- `vibecli --bugbot --diff` — review staged changes, output to stdout
- `vibecli --bugbot --watch` — watch for new PRs and auto-review

**`VibeUI BugBot tab`** — shows current repo's open PRs with review status; click "Review" to run BugBot; inline annotation in Monaco editor.

---

## Part H — Phase 19: Visual Builder

**Goal:** Implement Cursor-style Visual Editor + Lovable-style Design mode for VibeUI.

### H.1 Visual UI Editor

Enable "point and prompt" editing of live web apps rendered in the browser panel.

**Architecture:**
1. **Inspector script** injected into iframe: `inspector.js`
   - On mouse hover → highlight element, show bounding box overlay
   - On click → capture element's DOM path, computed styles, React component name (if available)
   - Emit `postMessage` to parent with `{ selector, outerHTML, styles, reactComponent }`

2. **VibeUI receives element info** → shows floating "AI Edit" bar above element:
   - Text input: "Change this to..."
   - Property sliders: font-size, padding, color picker
   - "Replace component" button

3. **Edit request** → `invoke("visual_edit_element", { selector, instruction, currentHtml, language })` → inline edit → update in real workspace

**New file: `vibeui/src/components/VisualEditor.tsx`**
```tsx
interface SelectedElement {
    selector: string;
    outerHTML: string;
    boundingRect: DOMRect;
    reactComponent?: string;
    filePath?: string;
    lineNumber?: number;
}

export function VisualEditor({ iframeRef, onEdit }: VisualEditorProps) {
    const [selected, setSelected] = useState<SelectedElement | null>(null);
    const [instruction, setInstruction] = useState("");
    // Floating bar positioned near selected element
    // Sends AI edit request on submit
}
```

**New `vibeui/public/inspector.js`** — injected into browser panel iframe.

**`BrowserPanel.tsx`:** Add "Visual Edit" toggle button; when on, inject inspector.js + mount VisualEditor overlay.

---

### H.2 Design Mode

Dedicated full-screen design editing mode (like Lovable's Design mode).

**New `aiPanelTab` value:** `"design"`
**New file: `vibeui/src/components/DesignMode.tsx`**

- Side-by-side: file tree (left) + live preview (center) + AI chat (right)
- Component tree (left panel): parsed JSX component hierarchy
- Click component in tree → scroll to element in preview → select for AI edit
- Property inspector (right panel): AI-powered property editing with instant preview
- "Generate component" button → describe in NL → AI writes and hot-reloads

---

### H.3 Figma Import

**New Tauri command: `import_figma(url: String, token: String) -> ImportResult`**
```rust
// 1. Fetch Figma file via API: GET /v1/files/{key}
// 2. Parse frames/components into layout description
// 3. Use LLM to generate React components matching the layout
// 4. Return { files: Vec<GeneratedFile>, preview_html: String }
```

**`DesignMode.tsx`:** "Import from Figma" button → dialog → paste Figma URL + API token → preview generated components → Apply.

---

## Part I — Phase 20: Deployment & Hosting Stack

**Goal:** Make VibeUI a full "from idea to deployed app" platform, matching Replit/Lovable/Base44.

### I.1 One-Click Deployment

**New file: `vibeui/src/components/DeployPanel.tsx`**

Supported targets:
- **Vercel** — `vercel deploy` via CLI; show deployment URL + logs
- **Netlify** — `netlify deploy --prod`; show URL
- **Railway** — `railway up`; show service URL + metrics
- **GitHub Pages** — `npm run build && gh-pages -d dist`

**Deployment flow:**
1. Detect project type (Vite/Next.js/Remix/SvelteKit) from `package.json`
2. Show recommended target and build command
3. "Deploy" button → runs build + deploy CLI in VibeUI terminal
4. Stream output in real-time
5. Show success badge with live URL

**New Tauri commands:**
- `detect_deploy_target(workspace)` → `{ target, build_cmd, out_dir }`
- `run_deploy(target, workspace)` → streams `deploy:log` events → returns `{ url }`
- `get_deploy_history()` → list of past deployments with URLs + timestamps

**`App.tsx`:** Add "🚀 Deploy" as 14th AI panel tab.

---

### I.2 Built-In Database UI

**New file: `vibeui/src/components/DatabasePanel.tsx`**

Supports:
- **SQLite** (local) — auto-detected from `*.db` files in workspace
- **Supabase** — connect via URL + anon key
- **PostgreSQL** — connect via connection string

Features:
- Table browser: list tables, show rows (paginated)
- SQL query editor with AI assist: describe query in NL → SQL generated
- Schema viewer: column types, constraints, foreign keys
- "Generate migration" button → AI writes migration file

**New Tauri commands:**
- `list_db_tables(connection_string)` → `Vec<TableInfo>`
- `query_db(connection_string, sql)` → `QueryResult`
- `generate_migration(connection_string, description)` → `String` (SQL migration)

---

### I.3 Supabase Integration

**New file: `vibeui/src/components/SupabasePanel.tsx`**

- Connect: Supabase URL + anon key + service role key
- Auth management: list users, manage auth providers, test auth flows
- Database: browse tables (uses DatabasePanel internally)
- Storage: list buckets, upload/download files
- Edge functions: list + deploy

**New Tauri commands:**
- `supabase_connect(url, key)` → test connection + return project info
- `supabase_list_tables(url, key)` → list with row counts
- `supabase_list_users(url, service_key)` → paginated user list

**`vibeui/src-tauri/Cargo.toml`:** No new crate needed — uses `reqwest` to hit Supabase REST API.

---

### I.4 GitHub Bidirectional Sync

**New file: `vibeui/src/components/GitHubSyncPanel.tsx`**

- Connect to GitHub repo (OAuth via `tauri-plugin-opener` + GitHub device flow)
- Auto-commit + push on file save (configurable interval, or manual)
- Pull button: fetch latest + merge
- Branch switcher: switch branches from UI
- PR creation: "Create PR" from current branch

**New Tauri commands:**
- `github_auth_device_flow()` → starts OAuth device flow, returns token
- `github_push(repo, branch, message)` → commit + push staged changes
- `github_pull(repo, branch)` → fetch + merge latest
- `github_create_pr(repo, head, base, title, body)` → returns PR URL

---

### I.5 Browser-Embedded App Testing (Replit Agent 3 equivalent)

Give agents the ability to control the browser panel and test the app they just built.

**New Tauri command: `agent_browser_action(action: BrowserAction) -> BrowserResult`**
```rust
pub enum BrowserAction {
    Navigate { url: String },
    Click { selector: String },
    Fill { selector: String, value: String },
    Screenshot,
    GetText { selector: String },
    WaitFor { selector: String, timeout_ms: u64 },
}
```

**Agent tool: `browser_test`**
```xml
<browser_test>
  <action>click</action>
  <selector>#submit-button</selector>
</browser_test>
```

This uses the existing browser panel iframe; commands are sent via `postMessage`.

**`agent_executor.rs`:** Add `BrowserTest` tool that calls `agent_browser_action`.

---

### I.6 Multiplayer (Real-Time Collaboration)

Use WebRTC or WebSocket for real-time cursor sharing and edit sync.

**Architecture:**
- **Collaboration server:** new `vibe-collab` crate — Axum + tokio-tungstenite WebSocket server
- **CRDT sync:** use `yrs` (Yjs Rust port) for conflict-free document merging
- **Presence:** broadcast cursor positions, selections, user info

**New crate: `vibeui/crates/vibe-collab/`**
```rust
pub struct CollabServer {
    pub sessions: HashMap<String, CollabSession>,
    pub port: u16,
}

pub struct CollabSession {
    pub room_id: String,
    pub participants: Vec<Participant>,
    pub doc: yrs::Doc,          // CRDT document
    pub awareness: Awareness,   // cursor/selection presence
}
```

**`VibeUI`:** "Share" button → generates share URL → others open VibeUI with that URL → real-time sync.

---

## Part J — Implementation Sequence

### Phase 16 Execution Order (2 weeks)
1. Steering files (`rules.rs` extension, SteeringPanel.tsx) — 2 days
2. File-event hooks (`FileSaved`/`FileCreated`/`FileDeleted`) — 2 days
3. Spec system CLI (`spec.rs`, REPL commands, `--spec` flag) — 3 days
4. SpecPanel.tsx + Tauri commands — 2 days
5. Git worktree isolation per subagent — 2 days
6. OS-level sandboxing (macOS first, Linux second) — 3 days

### Phase 17 Execution Order (2 weeks)
1. Groq provider (OpenAI-compatible, 1 day) → Bedrock (2 days) → Azure (2 days) → OpenRouter (1 day)
2. SQLite session store (3 days, migrate JSONL traces)
3. Ambient agent definitions + background manager (3 days)
4. Team knowledge store (team.toml + REPL commands) (2 days)

### Phase 18 Execution Order (1 week)
1. Memories auto-generation (2 days)
2. Shadow workspace lint worker (2 days)
3. BugBot CLI + VibeUI tab (3 days)

### Phase 19 Execution Order (2 weeks)
1. Visual UI Editor (inspector.js + VisualEditor.tsx) (4 days)
2. Design mode (DesignMode.tsx, component tree) (3 days)
3. Figma import (fetch API + LLM codegen) (3 days)

### Phase 20 Execution Order (3 weeks)
1. Deploy panel (Vercel/Netlify/Railway/Pages) (3 days)
2. Supabase integration (2 days)
3. Database UI (3 days)
4. GitHub bidirectional sync (3 days)
5. Browser-embedded app testing (3 days)
6. Multiplayer/CRDT (5 days — highest complexity)

---

## Part K — New Files Summary

### Phase 16
| File | Purpose |
|------|---------|
| `vibecli/vibecli-cli/src/spec.rs` | Spec-driven development engine |
| `vibeui/src/components/SpecPanel.tsx` | Spec UI with task checklist |
| `vibeui/src/components/SteeringPanel.tsx` | Steering file CRUD UI |
| `.vibecli/specs/` (convention) | Spec file storage directory |
| `.vibecli/steering/` (convention) | Steering file directory |

### Phase 17
| File | Purpose |
|------|---------|
| `vibeui/crates/vibe-ai/src/providers/bedrock.rs` | AWS Bedrock provider |
| `vibeui/crates/vibe-ai/src/providers/groq.rs` | Groq ultra-fast inference |
| `vibeui/crates/vibe-ai/src/providers/azure_openai.rs` | Azure OpenAI provider |
| `vibeui/crates/vibe-ai/src/providers/openrouter.rs` | OpenRouter (300+ models) |
| `vibeui/crates/vibe-ai/src/session_store.rs` | SQLite session storage |
| `vibecli/vibecli-cli/src/background_agents.rs` | Ambient agent manager |
| `vibecli/vibecli-cli/src/team.rs` | Team knowledge store |

### Phase 18
| File | Purpose |
|------|---------|
| `vibecli/vibecli-cli/src/memory_auto.rs` | Memories auto-extraction |
| `vibeui/src-tauri/src/shadow_workspace.rs` | Background lint worker |
| `vibecli/vibecli-cli/src/bugbot.rs` | Automated PR reviewer |
| `vibeui/src/components/BugBotPanel.tsx` | BugBot VibeUI integration |

### Phase 19
| File | Purpose |
|------|---------|
| `vibeui/public/inspector.js` | Iframe DOM inspector script |
| `vibeui/src/components/VisualEditor.tsx` | Point-and-prompt overlay |
| `vibeui/src/components/DesignMode.tsx` | Full-screen design mode |

### Phase 20
| File | Purpose |
|------|---------|
| `vibeui/src/components/DeployPanel.tsx` | One-click deployment UI |
| `vibeui/src/components/DatabasePanel.tsx` | Database browser + SQL editor |
| `vibeui/src/components/SupabasePanel.tsx` | Supabase integration |
| `vibeui/src/components/GitHubSyncPanel.tsx` | GitHub bidirectional sync |
| `vibeui/crates/vibe-collab/` | WebRTC/WebSocket collab server |

---

## Part L — Differentiation After All Phases

After Phases 16–20, VibeCody will be the **only** tool with:

| Unique Capability | Why No Competitor Has It |
|------------------|------------------------|
| **Spec-driven dev + steering + file-event hooks** | Kiro is VS Code fork (no custom runtime); we embed it in both CLI and desktop app |
| **HTTP daemon + REST API + Node.js SDK** | Full programmatic access; Cursor/Windsurf are GUI-only |
| **OpenTelemetry OTLP tracing** | No competitor exports spans |
| **Voice input** | Web Speech API in desktop app |
| **WASM extension system** | Sandboxed plugins via wasmtime |
| **opusplan routing (planning vs execution model)** | No competitor separates planning/execution LLM |
| **75+ providers + BYOK + apiKeyHelper** | Most complete provider support |
| **Ambient agents + background:true definitions** | Matches Claude Code; exceeds Cursor/Windsurf |
| **Visual Editor + Spec-to-code + One-click Deploy** | The complete "vibe coding" platform end-to-end |
| **Multi-agent Manager view with branch merging** | Unique visual parallel agent UI |
| **Hooks UI + LLM hooks** | No competitor has a GUI for hook configuration |

---

---

## Part M — Phase 21: Edge, Messaging & PicoClaw-Inspired Features

**Goal:** Bring VibeCLI's deployment model and UX to match PicoClaw's distribution excellence while adding messaging platform integration and scheduling.

### M.1 Tavily / Brave Web Search

Replace DuckDuckGo Lite (HTML scraping) with AI-optimized search APIs.

**Files to modify:**
- `vibecli/vibecli-cli/src/tool_executor.rs` — `web_search()` dispatches to configured engine
- `vibecli/vibecli-cli/src/config.rs` — extend `WebSearchConfig`: add `engine = "tavily"` | `"brave"` | `"duckduckgo"`, `tavily_api_key`, `brave_api_key`

```rust
// Tavily response: JSON with ranked results + snippets
async fn tavily_search(query: &str, api_key: &str, max_results: usize) -> Result<Vec<SearchResult>> {
    let resp = reqwest::Client::new()
        .post("https://api.tavily.com/search")
        .json(&json!({
            "api_key": api_key,
            "query": query,
            "max_results": max_results,
            "search_depth": "basic",
            "include_answer": true,
        }))
        .send().await?;
    // Returns { answer: str, results: [{ title, url, content, score }] }
}

// Brave API: REST, 2000 req/mo free
async fn brave_search(query: &str, api_key: &str, max_results: usize) -> Result<Vec<SearchResult>> {
    let resp = reqwest::Client::new()
        .get("https://api.search.brave.com/res/v1/web/search")
        .header("X-Subscription-Token", api_key)
        .query(&[("q", query), ("count", &max_results.to_string())])
        .send().await?;
}
```

**`--doctor`:** Show configured search engine + key status.

---

### M.2 Messaging Gateway (Telegram / Discord / Slack)

Run VibeCLI as a 24/7 bot — users chat with their agent through familiar messaging apps.

**New file: `vibecli/vibecli-cli/src/gateway/mod.rs`**
```rust
pub trait GatewayAdapter: Send + Sync {
    async fn start(&self, handler: Arc<dyn MessageHandler>) -> Result<()>;
    async fn send_message(&self, chat_id: &str, text: &str) -> Result<()>;
    async fn send_file(&self, chat_id: &str, name: &str, data: &[u8]) -> Result<()>;
}

pub struct GatewayManager {
    adapters: Vec<Box<dyn GatewayAdapter>>,
    agent_factory: Arc<dyn Fn() -> AgentLoop>,
    session_store: Arc<SessionStore>,
}
```

**Per-platform adapters:**
- `gateway/telegram.rs` — teloxide (Rust Telegram bot framework); poll or webhook
- `gateway/discord.rs` — serenity (Rust Discord bot); slash commands
- `gateway/slack.rs` — Slack Events API (webhook-based)

**Config:**
```toml
[gateway]
enabled = true

[gateway.telegram]
token = "123:ABC..."
allowed_users = ["@alice", "@bob"]   # whitelist (empty = open)

[gateway.discord]
token = "MTI..."
guild_id = "12345"
channel_ids = ["678"]    # allowed channels

[gateway.slack]
app_token = "xapp-..."
bot_token = "xoxb-..."
```

**CLI flag:** `vibecli --gateway` starts all configured adapters.

**Message handling:**
- Each user message → new or resumed agent session (session per chat_id)
- Streaming: send partial chunks every ~3 seconds as "typing..." messages
- Tool outputs shown as code blocks
- File writes → agent sends file attachment to chat
- `/reset` → clears session
- `/cost` → shows session cost

**New dependency:** `teloxide = "0.13"` + `serenity = "0.12"` in `vibecli/Cargo.toml`

---

### M.3 Built-in Scheduling (Cron Agent Tasks)

Schedule recurring or one-time agent tasks — "run tests every night at 2am".

**New file: `vibecli/vibecli-cli/src/scheduler.rs`**
```rust
use tokio_cron_scheduler::{Job, JobScheduler};

pub struct ScheduledTask {
    pub id: String,
    pub cron: String,           // cron expression OR "in 1h" / "every day at 9am"
    pub task: String,           // agent task description
    pub provider: String,
    pub approval_policy: String,
    pub last_run: Option<u64>,
    pub next_run: u64,
    pub enabled: bool,
}

pub struct TaskScheduler {
    pub scheduler: JobScheduler,
    pub tasks: Vec<ScheduledTask>,
    pub store: Arc<SessionStore>,
}
```

**Persistence:** `~/.vibecli/scheduled-tasks.json`

**Natural language scheduling** (parse with LLM or regex):
- `"in 1 hour"` → one-time ISO 8601 datetime
- `"every day at 9am"` → `0 0 9 * * *`
- `"every Monday at 10am"` → `0 0 10 * * 1`

**REPL commands:**
- `/schedule add "run tests" --cron "0 2 * * *"` — add recurring task
- `/schedule add "deploy to staging" --in "30m"` — one-time task
- `/schedule list` — show all scheduled tasks with next run time
- `/schedule disable <id>` / `/schedule enable <id>`
- `/schedule delete <id>`

**CLI flag:** `vibecli --scheduler` — run in daemon mode, execute scheduled tasks

**Files to modify:**
- `vibecli/vibecli-cli/src/main.rs` — `--scheduler` flag
- `vibecli/vibecli-cli/src/repl.rs` — `/schedule` commands
- Add `tokio-cron-scheduler = "0.9"` to `vibecli/Cargo.toml`

---

### M.4 Enhanced Skills as Distributable Packages

Upgrade VibeCLI's simple markdown skills to PicoClaw-inspired structured packages.

**New skill format (`~/.vibecli/skills/<name>/`):**
```
~/.vibecli/skills/code-reviewer/
  SKILL.md              # required: YAML frontmatter + instructions
  checklists/
    security.md         # bundled reference docs
    performance.md
  hooks/
    post-review.sh      # optional post-execution hook
```

**SKILL.md format:**
```yaml
---
name: code-reviewer
version: "1.0.0"
description: Review code for security, performance, and style issues
author: vibecli-community
tools:               # declared tools the skill uses
  - read_file
  - web_search
  - bash
triggers:            # auto-activate on these patterns
  - "review|audit|check|analyze"
  - "/review"
requires:
  providers:
    - claude          # skill works best with this provider
resources:
  - checklists/security.md
  - checklists/performance.md
---
# Code Reviewer Skill

When asked to review code, follow these steps...
```

**Skill packaging:**
```bash
# Pack a skill for sharing
vibecli skills pack code-reviewer          # → code-reviewer-1.0.0.vibeskill

# Install a skill
vibecli skills install ./code-reviewer-1.0.0.vibeskill
vibecli skills install github:user/vibecli-skills/code-reviewer

# List installed skills
vibecli skills list

# Update all skills
vibecli skills update
```

**New file:** `vibecli/vibecli-cli/src/skill_manager.rs` — pack/install/update/list skills

---

### M.5 Single Binary Releases (cargo-dist)

Make VibeCLI as easy to install as PicoClaw — `curl | sh`.

**Files to create:**
- `.github/workflows/release.yml` — trigger on `v*` tags; build matrix (x86_64-linux, aarch64-linux, x86_64-macos, aarch64-macos); upload to GitHub Releases
- `dist-workspace.toml` — cargo-dist config

**`Cargo.toml` workspace additions:**
```toml
[workspace.metadata.dist]
cargo-dist-version = "0.22"
ci = ["github"]
installers = ["shell", "powershell", "homebrew"]
targets = ["x86_64-unknown-linux-gnu", "aarch64-unknown-linux-gnu",
           "x86_64-apple-darwin", "aarch64-apple-darwin",
           "x86_64-pc-windows-msvc"]
```

**Install script:** `curl -fsSL https://vibecody.dev/install.sh | sh`

**Homebrew tap:**
```ruby
# Formula/vibecli.rb
class Vibecli < Formula
  version "0.2.0"
  url "https://github.com/vibecody/vibecody/releases/download/v0.2.0/vibecli-x86_64-apple-darwin.tar.gz"
end
```

---

### M.6 Update Gap Priority Matrix (Phase 21)

| Gap | Impact | Effort | Phase |
|-----|--------|--------|-------|
| Tavily/Brave AI-optimized web search | High | S | 21 |
| Messaging gateway (Telegram/Discord/Slack) | High | L | 21 |
| Built-in scheduling (cron agent tasks) | High | M | 21 |
| Enhanced skill packages (SKILL.md format) | Medium | M | 21 |
| Single binary releases (cargo-dist) | High | S | 21 |
| Skill marketplace / registry | Medium | XL | 21 |
| Subagent spawning from tools | Medium | M | 21 |

---

## Part N — Final Competitive Positioning After All Phases

After completing Phases 16–21, VibeCLI + VibeUI becomes the **most complete AI development platform** across all dimensions:

| Dimension | VibeCLI | Warp | Kiro | opencode | Claude Code | PicoClaw |
|-----------|---------|------|------|----------|-------------|----------|
| Provider breadth | ✅ 75+ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Spec-driven dev | ✅ | ❌ | ✅ | ❌ | ❌ | ❌ |
| HTTP daemon + SDK | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ |
| OTel tracing | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ |
| Messaging gateway | ✅ | ❌ | ❌ | ❌ | ❌ | ✅ |
| Scheduling | ✅ | ❌ | ✅ | ❌ | ❌ | ✅ |
| Team knowledge store | ✅ | ✅ | ❌ | ❌ | ❌ | ❌ |
| OS-level sandboxing | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ |
| SQLite sessions | ✅ | ❌ | ❌ | ✅ | ❌ | ❌ |
| Multi-agent visual UI | ✅ | ✅ | ❌ | ❌ | ❌ | ❌ |
| WASM extensions | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ |

| Dimension | VibeUI | Cursor | Windsurf | Replit | Base44 | Lovable |
|-----------|--------|--------|----------|--------|--------|---------|
| Visual UI Editor | ✅ | ✅ | ❌ | ❌ | ✅ | ✅ |
| One-click deploy | ✅ | ❌ | ❌ | ✅ | ✅ | ✅ |
| Built-in DB UI | ✅ | ❌ | ❌ | ✅ | ✅ | ✅ |
| Supabase integration | ✅ | ❌ | ❌ | ✅ | ❌ | ✅ |
| GitHub sync | ✅ | ❌ | ❌ | ❌ | ❌ | ✅ |
| Multiplayer | ✅ | ❌ | ❌ | ✅ | ❌ | ✅ |
| Voice input | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ |
| WASM extensions | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ |
| OTel tracing | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ |
| Spec-driven dev UI | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ |
| Background jobs UI | ✅ | ✅ | ❌ | ❌ | ❌ | ❌ |
| BugBot PR review | ✅ | ✅ | ❌ | ❌ | ❌ | ❌ |
| Browser app testing | ✅ | ❌ | ❌ | ✅ | ❌ | ❌ |

---

*Generated from live competitor research (Feb 2026) + codebase audit. All file paths reference the VibeCody monorepo.*
