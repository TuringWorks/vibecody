---
layout: page
title: Competitive Landscape & Roadmap
permalink: /roadmap/
---

# Fit-Gap Analysis & Feature Implementation Roadmap

**Originally published:** February 2026 &middot; **Last refreshed:** 2026-05-03 (v0.5.7 cycle, v14 industry delta on top of v13 fitgap)
**Scope:** VibeCody compared against **30+** competing AI coding products across terminal, editor, cloud-agent, code-review, completions-only, and mobile/watch categories.

> This is now the **single canonical roadmap** — earlier iterations (v5, v6) and the 13 deep-dive fit-gap files are absorbed here and in the companion **[Fit-Gap Analysis](./fit-gap-analysis/)**. The original five-competitor delta (Codex CLI, Claude Code, Antigravity, Cursor, Windsurf) is preserved as a historical record below; phases 23–39 are summarised in the **History appendices** at the end of this document. The 2026-04-26 trend snapshot (§1bis) and the v13 audit-reconciliation work (Phase 53, [Appendix D](#appendix-d--phase-53-april-2026-trend-delta--audit-reconciliation)) are the freshest layer.

## 1bis. Q2 2026 industry trend snapshot (added 2026-04-26)

The eight days between the v0.5.5 fitgap refresh (2026-04-17) and today produced a denser-than-usual stream of AI-coding-tool releases. Headline shifts that the rest of this document does NOT yet absorb:

| Date | Release | What changed | Impact on VibeCody positioning |
|------|---------|--------------|--------------------------------|
| 2025-07 | **Cognition acquires Windsurf** | Devin + Windsurf + Cascade are now one company | §1.2 + §1.3 should treat them as the **Cognition family** rather than three competitors. |
| 2026-04-01 | **GitHub Copilot Cloud Agent expanded** | No longer PR-only; can branch-only work; CLI sessions controllable from web/mobile | New gap A9 — VibeMobile pairs with hosts but doesn't *resume* CLI sessions. |
| 2026-04-02 | **Cursor 3.0** | Agents Window, Design Mode (live DOM annotation), Agent Tabs (grid view), multi-repo | New gaps A5–A7. The `await_tool` parity claim from v8 is unchanged but the surface around it widened. |
| 2026-04-16 | **Claude Opus 4.7 GA** | 87.6% SWE-bench Verified (up from 80.8%); 64.3% SWE-bench Pro; 70% CursorBench | Refresh `useModelRegistry.ts` defaults; add to Counsel default panels. |
| 2026-04-22 | **Devin 2.2** | Full Linux desktop access; agent self-verifies via computer use, auto-fixes | New gap A8 — `visual_verify.rs` doesn't yet feed failures back into the agent loop. |
| 2026-04-24 | **Cursor 3.2** | Async subagents, multi-root workspaces | New gap A5 (async) + A6 (multi-root). |
| 2026-04-24 | **Copilot Inline Agent in JetBrains** | Agent-mode inline chat in JetBrains IDEs | Our JetBrains plugin should match within one minor cycle. |
| 2026-Q1–Q2 | **MCP 2026 roadmap published** | MCP Apps (interactive UI), MCPB bundle format, `.well-known` capability discovery, stateless transport, enterprise extensions | New gaps A1–A4. Largest single ecosystem shift this quarter. |
| 2026-Mar | **JetBrains Junie CLI Beta** | LLM-agnostic; runs in IDE/CLI/CI/CD/GitHub/GitLab; **1-click migration from Claude Code + Codex configs** | New gap A11 — migration tooling. Junie sets the bar for switching cost. |
| 2026-Q1 | **OpenAI Codex CLI v0.116+** | GPT-5.3-Codex-Spark (1000+ TPS), hooks GA, plugin marketplace, **AWS Bedrock auth + SigV4** | Bedrock partial; Spark drops in via existing routing layer. |
| 2026-Mar | **ACP v0.11.0** | Zed + JetBrains official partnership Oct 2025; Anthropic, OpenAI, GitHub, Google all ship implementations | New gap A4 — VibeCLI/VibeUI as ACP **server**, not just client. |
| 2026-Apr | **Antigravity 1.20.3 → 1.22.2** | AGENTS.md fallback, Linux sandboxing, MCP auth | One-line `memory.rs` addition for `GEMINI.md` fallback. |
| 2026-Apr | **Augment Code 72.0% SWE-bench Verified pass@1** | Highest open-system score, no best-of-N tricks | Updates §1.4 / §9.3 SWE-bench callout. |
| 2026-Q1 | **Sandbox infrastructure GA wave** | Cloudflare Sandboxes GA; E2B/Northflank/Modal/Vercel/Docker microVM platforms shipped | Optional roadmap track: VibeCLI `--cloud` provider for users without local sandboxing. |

The 17 gaps surfaced by these releases (11 new + 6 covered by existing infra) are catalogued in §16.1 of the [Fit-Gap Analysis](./fit-gap-analysis/) and queued in [Phase 53](#appendix-d--phase-53-april-2026-trend-delta--audit-reconciliation) below.

## 1ter. May 2026 weekly delta + missed-quarter items (added 2026-05-03)

The seven days between the v13 snapshot (2026-04-26) and today shipped another dense wave; a handful of Q1-Q2 items v13 missed are folded in here as well. Sources surveyed: `cursor.com/changelog`, GitHub Copilot blog, Anthropic Claude Code releases, OpenAI Codex/ChatGPT release notes, Cognition Devin docs, `blog.modelcontextprotocol.io`, `a2a-protocol.org`, Linux Foundation press, JetBrains Junie + Air blogs, Ollama releases, `ggml-org/llama.cpp`, vLLM releases, SWE-bench leaderboards, sandbox provider coverage (E2B / Daytona / Modal / Blaxel / SmolVM), and OSS coding-agent repos (Cline / OpenHands / Aider / Continue).

### 1ter.1 Headline shifts not yet absorbed elsewhere

| Date | Release | What changed | Impact on VibeCody positioning |
|------|---------|--------------|--------------------------------|
| 2026-05-04 | **MCP `experimental-ext-skills`** | Skills discovery & distribution as MCP primitives (`modelcontextprotocol/experimental-ext-skills`) | New gap **B1** — VibeCody's 711 skill files could become MCP-discoverable across every host that speaks MCP. |
| 2026-05-01 | **Cursor Plugin Marketplace v2** | Plugins bundle MCP servers + skills + subagents + rules + hooks; admin install policy (Default Off / Default On / Required); Team Marketplace decoupled from any specific repo | New gap **B2** — VibeCody plugins are still single-purpose; no bundle format with admin-policy tiers. |
| 2026-04-30 | **Cursor Security Review (beta)** | Always-on Security Reviewer + Vulnerability Scanner agents on Teams + Enterprise plans | New gap **B3** — `/review` runs on demand; no always-on security-agent pattern. |
| 2026-04-29 | **VS 2026 + VS Code Integrated Cloud Agent** | "Assign a task, close the IDE, get a PR" — Copilot Cloud Agent now controllable from inside the editor | Extends **A9** — cloud-agent remote-control is now IDE-native, not just web/mobile. |
| 2026-04-23 | **OpenAI GPT-5.5 GA** | Recommended default Codex model (replaces 5.4); GPT-5 latency at higher intelligence; fewer tokens per Codex task; computer-use focus | Add to `useModelRegistry.ts`; route default Codex calls to 5.5. GPT-5.3-Codex-Spark from §1bis remains the latency tier. |
| 2026-Apr | **Cursor SDK (TypeScript)** | Same agent runtime / harness / models as desktop, CLI, and web — exposed as a `@cursor/sdk` TS library | Direct competitor to `packages/agent-sdk/`; parity audit (subagents, hooks, plugins, skills, sandbox tiers). |
| 2026-04-21 | **llama.cpp NVFP4 (PR #22196 reposted)** | Blackwell-native FP4 path merged; MXFP4 progressing in `ik_llama.cpp`; b8196+ runs MXFP4 MoE on Blackwell | TurboQuant target list: add **NVFP4** alongside MXFP4 + AWQ-Marlin. CubeCL/Burn ban scope unchanged. |
| 2026-Apr | **Chinese frontier wave** | DeepSeek V4-Flash $0.14/$0.28 per 1M (~7.7× cheaper than Qwen 3.6-Plus on chatbot loads); Qwen 3.6-Plus + Qwen 3.6-35B-A3B (Apache 2.0); Kimi K2.6 long-horizon agentic; MiniMax M2.7; GLM-5.1 | Counsel default panels need an open-weight slot; `cost_router.rs` learns the new floor; `useModelRegistry.ts` Ollama defaults shift to Qwen 3.6-Coder family when released. |
| 2026-Apr–May | **Ollama 0.22.x** | `/v1/messages` (Anthropic Messages API compat — Claude Code can drive Ollama-hosted open models); `ollama launch` registers Claude Desktop / Cowork / Code; Gemma 4 thinking + tool calls; MLX runner gains logprobs + fused top-P/K | Ollama-compat `/api/*` surface should mirror `/v1/messages`; existing routing layer absorbs this with one new route. |
| 2026-Q1 | **A2A v1.2 (Linux Foundation Agentic AI Foundation)** | 150+ orgs in production; signed agent cards (cryptographic signatures for domain verification); GA across Google / Microsoft / AWS | Extends **A4** — VibeCody's A2A server façade needs P-256-signed agent cards at `/.well-known/agent.json`; aligns with watch-pairing's existing P-256 ECDSA constraint. |
| 2026-Q1 | **ACP Registry live** | Built into Zed + JetBrains; lists Claude Code, Codex CLI, GitHub Copilot CLI, OpenCode, Gemini CLI | Concrete follow-through on **A4** — register VibeCLI as an ACP agent so Zed / JetBrains users discover it natively. |
| 2026-Q1 | **SWE-bench Verified contamination disclosed** | OpenAI stopped reporting Verified scores; audit found **59.4% of hard tasks have flawed tests**; all frontier models contaminated | §9.3 leaderboard callout needs a contamination caveat; **SWE-bench Pro / SWE-rebench / SWE-bench-Live** become primary references. |
| 2026-Q1 | **DAPO mainstreamed** | OpenRLHF, verl, NeMo-RL all ship DAPO as default reasoning-RL alongside PPO/GRPO; ByteDance paper open-sourced (50% fewer training steps for AIME-class tasks) | RL-OS slice 2 should make GRPO + DAPO peers, not roll-forward; current `vibe_rl/algos/ppo.py` is the only algo file with recent edits. |
| 2026-Q1 | **Sandbox cold-start floor** | Blaxel 25 ms; Daytona 27–90 ms (Docker); E2B Firecracker microVMs ~150 ms; Modal gVisor; SmolVM debuted 2026-04-17 | Sandbox-tiers Firecracker tier should target ≤100 ms cold start; document the latency floor per backend in `docs/design/sandbox-tiers/`. |
| 2026-05-19 (planned) | **Google I/O 2026 — Gemini 4 + Android 17 + Agentic Coding Dev Preview** | I/O keynote with agentic coding as a headline track; Gemini 3.1 Pro Preview already shipping ahead of it | Calendar item: prep `useModelRegistry.ts` Gemini 4 entry; Android 17 may move VibeMobile API floor — re-check `pubspec.yaml` `minSdkVersion` after I/O. |

### 1ter.2 Context-only signals (positioning, not new gaps)

- **Claude Sonnet 4.8 leaked** (Mar 2026) — Anthropic skips a 4.7 designation for the Sonnet tier; pricing held at $3 / $15 per 1M ([Anthropic source-code leak coverage](https://www.ainexusdaily.com/article/anthropic-source-leak-claude-sonnet-4-8-undercover-mode), unverified for exact GA week). Add to `useModelRegistry.ts` once the API exposes it.
- **Cursor Interactive Canvases** (Apr 15, missed in v13) — agent responses render as dashboards / forms / charts inside the chat panel. Overlaps MCP Apps (A1 / SEP-1865); a single workstream now covers both surfaces.
- **Copilot training-default opt-in** (Apr 2026) — GitHub flipped users' AI-training preference to opt-in by default; community backlash drove visible migration to Cline (58k stars), OpenHands (72k), Aider (27k). VibeCody's "no training on user code" stance becomes a measurable sales axis.
- **JetBrains Air** (Mar 2026) — agentic IDE rebuilt on Fleet remnants; supports OpenAI Codex, Anthropic Claude Agent, Gemini CLI, and Junie as native agents. Adds to §1.2 IDE list as a watch item, not a direct VibeUI competitor today.
- **Doe v. GitHub Copilot** (ongoing, 2026 status) — DMCA claims dismissed; license / contract claims still proceeding; reshaping AI-gen-code compliance standards. Reinforces (3) above; informs `/review`'s open-source-license-scan UX.
- **Devin v3 API GA** (Q1 2026) — Cognition's v3 API is now the primary surface (out of beta); HTML/PDF/SVG inline rendering in session sidebar; focus mode. Devin-family parity continues to track A8 (self-verifying loop) more than UI ergonomics.

The 6 new external gaps surfaced (B1–B6) plus the 9 informational rows are catalogued in §16.4 of the [Fit-Gap Analysis](./fit-gap-analysis/) and queued for **Phase 54** in [Appendix D](#appendix-d--phase-53-april-2026-trend-delta--audit-reconciliation). The §16.4 text also revises §15.4 (SWE-bench callout) to flag the contamination finding.

## 1. Competitive Landscape Summary

AI-assisted development splits into six tool categories. VibeCody is the only project that ships a competitive entry in **every single one** of them from a shared Rust + TypeScript monorepo — plus two surfaces (Watch, Flutter Mobile) that have **no serious competitor** as of 0.5.5.

### 1.1 Terminal / CLI agents

| Tool | Owner | Stack | Standout capability |
|------|-------|-------|---------------------|
| **OpenAI Codex CLI** | OpenAI | Rust + TS | Reference agent loop, OS sandbox, approval tiers, MCP |
| **Claude Code** | Anthropic | Node.js | Agentic multi-file edits, 300+ MCP integrations, subagents, hooks, skills |
| **Gemini CLI** | Google | Node.js | Gemini-centric, built-in Google Search grounding, long-context |
| **Aider** | Paul Gauthier (OSS) | Python | Git-aware pair programming, repo-map, very low-cost fastest iteration |
| **Goose** | Block / Square (OSS) | Rust | MCP-native extensible agent, session replay, any-provider |
| **OpenHands** (ex-OpenDevin) | All Hands AI (OSS) | Python + sandbox | Full browser + shell sandbox agent, SWE-bench benchmark leader |
| **Cline** (ex-Claude Dev) | OSS | VS Code ext. | In-editor autonomous agent, plan/act toggle, tight terminal loop |
| **Amp** | Sourcegraph | TS | Terminal companion to Cody, agent over an indexed monorepo |
| **Plandex** | Plandex AI (OSS) | Go | Long-running planning + diff review, self-hosted |
| **`llm`** | Simon Willison (OSS) | Python | Minimal provider-agnostic CLI, plugin ecosystem |
| **Warp AI** | Warp | Rust terminal | AI-native terminal, command-suggest and block AI |
| **Mentat** | AbanteAI (OSS) | Python | Interactive code-edit REPL with repo-map |
| **VibeCLI** | TuringWorks (this project) | Rust | All of the above + 22 providers, TUI + REPL + `--serve` daemon, 711 skills, OpenTelemetry |

### 1.2 AI-native IDE / editor

| Tool | Owner | Stack | Standout capability |
|------|-------|-------|---------------------|
| **Cursor** | Anysphere | Electron + VS Code fork | Tab model (next-action prediction), 8 parallel agents in git worktrees, 200k-token indexing, BugBot |
| **Windsurf** | Codeium | Electron + VS Code fork | Cascade agent with flow-awareness, planning agent, memory, checkpoints |
| **Google Antigravity** | Google | Electron + Gemini | Agent-first IDE, Manager View (5 parallel agents), Artifacts, knowledge base |
| **GitHub Copilot Workspace** | GitHub / Microsoft | Web + VS Code | Spec → plan → implementation workflow, deep GitHub integration |
| **JetBrains AI Assistant** | JetBrains | All JetBrains IDEs | Deep language tooling, on-prem option, Junie agent |
| **Amazon Q Developer** | AWS | VS Code / JetBrains | AWS-aware completions, IAM/infra transformations |
| **Zed** | Zed Industries | Rust + GPUI | Collaborative editor with multi-model AI panel, low latency |
| **Cody** | Sourcegraph | VS Code / JetBrains / web | Repo-wide embeddings, enterprise graph, bring-your-own-LLM |
| **Continue.dev** | OSS | VS Code / JetBrains | Open-source Copilot-style autocompletion + chat |
| **Aide** | CodeStory (OSS) | VS Code fork | Open-source Cursor alternative |
| **Void** | OSS | VS Code fork | Open-source agent editor |
| **PearAI** | OSS | VS Code fork | Cursor-alternative with marketplace |
| **Melty** | OSS | Electron | Structured agent + changelog first editor |
| **Tabnine** | Tabnine | VS Code / JetBrains | Privacy-first completions, on-prem |
| **VibeUI** (this project) | TuringWorks | Tauri + React + Rust | 293 panels + 42 composites, all 22 providers, CRDT multiplayer, hooks, skills, WASM extensions |

### 1.3 Cloud / remote-agent products

| Tool | Owner | Stack | Standout capability |
|------|-------|-------|---------------------|
| **Devin** | Cognition | Cloud + web | Fully hosted autonomous engineer, browser + shell in VM |
| **Replit Agent** | Replit | Replit cloud | App-generation agent + hosted runtime, mobile companion |
| **Bolt.new** | StackBlitz | Web (WebContainers) | "Prompt to full-stack app" running in the browser |
| **v0** | Vercel | Web | Shadcn/Next.js UI generator; deep Vercel integration |
| **Lovable** | Lovable | Web | Prompt-to-app with Supabase + Stripe scaffolds |
| **Builder.io Visual Copilot** | Builder.io | Web | Figma-to-code + visual editor |
| **Sweep AI** | Sweep | GitHub App | Issue → PR automation |
| **VibeCLI `--serve` + VibeUI + agent-sdk** | TuringWorks | Self-hosted | Same capability, **self-hostable and open-source**; pair from any device over mDNS/Tailscale/ngrok |

### 1.4 AI code review / CI bots

| Tool | Owner | Integration | Standout capability |
|------|-------|-------------|---------------------|
| **CodeRabbit** | CodeRabbit | GitHub / GitLab | Line-by-line PR review, chat back to author |
| **Qodo** (ex-Codium AI) | Qodo | GitHub / IDE | PR-Agent, tests generation, coverage |
| **Greptile** | Greptile | GitHub App | Repo-aware review using a graph index |
| **Cursor BugBot** | Anysphere | GitHub | Cursor-branded review bot |
| **Ellipsis.dev** | Ellipsis | GitHub | Q&A + review across the repo |
| **Graphite AI** | Graphite | GitHub stacked-PRs | AI review tuned for stacked-diff workflow |
| **VibeCLI `/review` + VibeUI CIReviewPanel** | this project | GitHub App + CLI | Same capability, runs locally or in CI; red-team mode + Counsel multi-LLM deliberation |

### 1.5 Completion-only / IDE helper

| Tool | Owner | Integration | Standout capability |
|------|-------|-------------|---------------------|
| **GitHub Copilot** | GitHub / MS | All IDEs | Industry-default inline completions |
| **Tabnine** | Tabnine | All IDEs | On-prem / privacy-first |
| **Codeium** (free tier) | Codeium | All IDEs | Free Copilot-tier completions |
| **Continue.dev** | OSS | VS Code / JetBrains | Self-hosted model completions |
| **Supermaven** | Supermaven | VS Code / JetBrains | 1M-token context window completions |
| **VibeUI inline completions** | this project | VS Code + Monaco | FIM-enabled local Ollama + any cloud provider |

### 1.6 Mobile / Watch — no serious competitor

| Tool | Owner | Platforms | Status |
|------|-------|-----------|--------|
| **Replit mobile** | Replit | iOS, Android | Replit-only; runs Replit cloud |
| **Cursor mobile (preview)** | Anysphere | iOS | Early preview; read-only chat |
| **Windsurf mobile** | Codeium | — | Announced, not shipped |
| **Devin mobile web** | Cognition | Web PWA | Read-only session viewer |
| **VibeMobile** | this project | iOS, Android, macOS, Linux, Windows, Web | Full-duplex pairing + chat + session control against any host |
| **VibeWatch — Apple Watch** | this project | watchOS 10+ | **No peer** — native SwiftUI client with dictated reply + approval flow |
| **VibeWatch — Wear OS** | this project | Wear OS 3+ | **No peer** — native Kotlin/Compose client with the same capability |

### Coverage summary

| Category | Total tools surveyed | VibeCody ships a competitive entry |
|----------|----------------------|-------------------------------------|
| Terminal / CLI agents | 13 | ✅ VibeCLI |
| AI-native IDE / editor | 15 | ✅ VibeUI |
| Cloud / remote-agent | 7 | ✅ `--serve` + agent-sdk (self-hosted) |
| AI code review bots | 7 | ✅ `/review` + CIReviewPanel |
| Completion-only | 6 | ✅ VibeUI inline completions (FIM) |
| Mobile / watch | 4 mobile, 0 watch | ✅ VibeMobile + **first-class VibeWatch** |
| **Total** | **52 tools across 6 categories** | **VibeCody is the only project that ships in all six — and the only one with a native watch client.** |


## 2. Current VibeCLI — Feature Inventory

| Feature | Status | Notes |
|---------|--------|-------|
| Multi-provider (22 providers) | Yes Done | All 22 providers implemented with failover |
| TUI (Ratatui) | Yes Done | Chat, FileTree, DiffView, Agent screens |
| REPL mode (rustyline) | Yes Done | History, tab completion, 14 slash commands |
| Git context injection | Yes Done | Branch, status, diff in system prompt |
| `/apply` — single-file AI edits | Yes Done | Shows diff, requires confirmation |
| `/exec` — AI-generated shell commands | Yes Done | Confirmation gate |
| `!cmd` — direct shell execution | Yes Done | Config-gated approval |
| TOML config (`~/.vibecli/config.toml`) | Yes Done | Per-provider + safety settings |
| Syntax highlighting in REPL | Yes Done | syntect |
| Streaming responses | Yes Done | Token-by-token via CompletionStream; TUI + REPL |
| Agent loop (autonomous multi-step) | Yes Done | plan→act→observe, 30-step max, `AgentLoop` |
| Structured tool use framework | Yes Done | 7 tools: read/write/patch/bash/search/list/complete |
| Approval tiers (Suggest/AutoEdit/FullAuto) | Yes Done | 3-tier; `--suggest`/`--auto-edit`/`--full-auto` flags |
| OS sandbox for command execution | Yes Done | macOS `sandbox-exec`, Linux `bwrap` |
| Codebase indexing / semantic search | Partial | Regex + heuristic symbol index; embeddings pending |
| Multi-file editing (batch apply) | Yes Done | Agent WriteFile tool handles any number of files |
| AGENTS.md / project memory | Yes Done | Loads VIBECLI.md / AGENTS.md / CLAUDE.md + global |
| MCP server integration | Yes Done | JSON-RPC 2.0 stdio; `/mcp list`, `/mcp tools` |
| Non-interactive / CI mode | Yes Done | `--exec` flag; JSON/Markdown report; exit codes 0-3 |
| Multimodal input (images/screenshots) | Yes Done | `[image.png]` syntax; Claude + OpenAI vision |
| Trace / audit log | Yes Done | JSONL per session; `/trace` + `/trace view <id>` |
| GitHub Actions integration | Yes Done | `.github/actions/vibecli/action.yml` |

## 3. Current VibeUI — Feature Inventory

| Feature | Status | Notes |
|---------|--------|-------|
| Monaco Editor integration | Yes Done | Full VS Code engine |
| Rope-based text buffer | Yes Done | ropey |
| Async file I/O + file watching | Yes Done | notify |
| Multi-workspace | Yes Done | Multiple root folders |
| Git panel (status, diff, commit, push, pull) | Yes Done | git2; stash, branch list/switch, history |
| Terminal panel (PTY) | Yes Done | portable-pty + xterm.js |
| AI chat panel | Yes Done | All 22 providers; streaming |
| Command palette | Yes Done | fuse.js fuzzy search |
| Dark/light theme | Yes Done | localStorage persistence |
| LSP client (completions, hover, go-to-def) | Yes Done | Wired to Monaco; lazy-start per language |
| Extension system (WASM) | Yes Done | Full wasmtime host; loads `~/.vibeui/extensions/*.wasm` |
| Inline AI completions (FIM) | Yes Done | Monaco `registerInlineCompletionsProvider`; Ollama FIM format |
| Agent mode (autonomous multi-file edits) | Yes Done | AgentPanel: steps, approval, streaming, events |
| @ context (reference files/symbols in chat) | Yes Done | `@query` popup; file search + `@git` context |
| Flow-awareness (edit/command tracking) | Yes Done | FlowTracker ring buffer; injected into AI context |
| Memory / rules system | Yes Done | MemoryPanel; `.vibeui.md` + `~/.vibeui/rules.md` |
| Diff preview before AI apply | Yes Done | Monaco DiffEditor; accept/reject; auto git stash |
| Checkpoint / undo AI session | Yes Done | Backend (git stash) + CheckpointPanel UI |
| Trace / audit log (History panel) | Yes Done | HistoryPanel; list + detail view; JSONL traces |
| Multimodal (screenshot in chat) | Yes Done | Backend (Claude + OpenAI) + AIChat UI |
| Codebase indexing (semantic) | Yes Done | Regex/heuristic + embedding-based vector search (Ollama/OpenAI) |
| Planning agent | Yes Done | PlannerAgent; plan generation, approval, guided execution |
| Multi-agent parallel execution | Yes Done | MultiAgentOrchestrator; git worktrees; ManagerView UI |
| Web context (@web) | Yes Done | `@web:<url>` in chat/agent; fetch + HTML-strip; ContextPicker autocomplete |
| Artifacts (task lists, plans, recordings) | Yes Done | ArtifactStore + ArtifactsPanel; annotations, async feedback |
| Voice input | Yes Done | Web Speech API hook + mic button in AIChat; pulse animation |
| Knowledge base (persistent snippets) | Yes Done | MemoryPanel + SkillLoader; auto-activating skills |

## 4. Fit-Gap Matrix

### 4.1 VibeCLI vs. CLI agents (broad)

`Y` = native; `P` = partial; `—` = not supported.

| Capability | VibeCLI | Codex CLI | Claude Code | Gemini CLI | Aider | Goose | OpenHands | Cline | Amp | Plandex |
|-----------|:---:|:---:|:---:|:---:|:---:|:---:|:---:|:---:|:---:|:---:|
| Full agent loop | **Y** | Y | Y | P | P | Y | Y | Y | Y | Y |
| Streaming TUI | **Y** | Y | P | Y | Y | Y | P | — | — | Y |
| Multi-file batch edits | **Y** | Y | Y | P | Y | Y | Y | Y | Y | Y |
| Approval tiers (3-level) | **Y** | Y | Y | — | P | Y | Y | Y | P | Y |
| Codebase indexing | **Y** | Y | P | P | P (repo-map) | P | P | P | Y | P |
| OS sandbox | **Y** | Y | P | — | — | — | Y | — | — | — |
| Project memory (`*.md`) | **Y** | Y | Y (CLAUDE.md) | — | P | P | — | P | Y | Y |
| MCP integration | **Y** | Y | Y (300+) | — | — | Y | P | — | P | — |
| Multi-provider (≥10) | **Y** (22) | P | — | — | Y | Y | Y | Y | Y | Y |
| Hooks system | **Y** | — | Y | — | — | — | — | — | — | — |
| Skills system | **Y** (711) | — | Y | — | — | — | — | — | — | — |
| Parallel multi-agent | **Y** | P | Y | — | — | — | — | — | — | — |
| Plan Mode | **Y** | — | Y | — | — | — | Y | Y | — | Y |
| Session resume | **Y** | Y | Y | P | Y | Y | Y | Y | Y | Y |
| Web search tool | **Y** | Y | Y | Y | — | Y | Y | — | Y | — |
| Code review agent | **Y** | Y | Y | — | — | — | — | — | Y | — |
| Red-team / pentest pipeline | **Y** | — | — | — | — | — | — | — | — | — |
| Counsel (multi-LLM debate) | **Y** | — | — | — | — | — | — | — | — | — |
| GitHub Actions integration | **Y** | Y | Y | — | — | — | — | — | — | — |
| OpenTelemetry | **Y** | Y | — | — | — | — | — | — | — | — |
| Daemon / REST + SSE server | **Y** | P | P | — | — | Y | — | — | Y | — |
| Mobile companion | **Y** | — | — | — | — | — | — | — | — | — |
| Watch companion | **Y** | — | — | — | — | — | — | — | — | — |

### 4.2 VibeUI vs. IDE / editor competitors

| Capability | VibeUI | Cursor | Windsurf | Antigravity | Copilot WS | JetBrains AI | Amazon Q | Zed | Cody | Continue | Aide | Void |
|-----------|:---:|:---:|:---:|:---:|:---:|:---:|:---:|:---:|:---:|:---:|:---:|:---:|
| Inline AI completions | **Y** | Y | Y | Y | Y | Y | Y | Y | P | Y | Y | Y |
| Agent mode (multi-file) | **Y** | Y | Y | Y | Y | Y | P | P | P | P | Y | Y |
| Diff review before apply | **Y** | Y | Y | Y | Y | Y | P | P | Y | Y | Y | Y |
| @ context system | **Y** | Y | Y | P | P | Y | P | Y | Y | Y | Y | Y |
| Flow-awareness | **Y** | P | Y | P | — | P | — | — | P | — | P | P |
| Memory / rules | **Y** | Y | Y | Y | P | Y | — | — | Y | Y | Y | Y |
| LSP | **Y** | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y |
| WASM extension host | **Y** | — | — | — | — | — | — | — | — | — | — | — |
| Trace / audit log | **Y** | — | — | — | — | — | P | — | — | — | — | — |
| Checkpoint / undo | **Y** | P | Y | Y | P | P | — | — | — | P | Y | Y |
| Multimodal chat | **Y** | Y | P | P | P | P | — | P | P | Y | Y | Y |
| Semantic codebase index | **Y** | Y | Y | Y | Y | Y | P | — | Y | P | Y | Y |
| Planning agent | **Y** | P | Y | Y | Y | Y | — | — | — | — | Y | Y |
| Parallel agents | **Y** | Y (8) | Y | Y (5) | — | — | — | — | — | — | P | — |
| Next-edit prediction (Tab) | **Y** | **Y** | Y | P | — | P | — | — | — | — | P | P |
| Manager View (orchestration) | **Y** | — | — | Y | — | — | — | — | — | — | — | — |
| CI review bot | **Y** | Y (BugBot) | — | — | — | — | — | — | — | — | — | — |
| Artifacts | **Y** | — | — | Y | Y | — | — | — | — | — | — | — |
| Multiplayer CRDT | **Y** | — | — | — | — | — | — | Y | — | — | — | — |
| Rust native backend | **Y** | — | — | P | — | — | — | Y | — | — | — | — |
| Local / private AI (Ollama) | **Y** | P | P | — | — | — | — | P | P | Y | Y | Y |
| Open source | **Y** | — | — | — | — | — | — | **Y** | P | **Y** | **Y** | **Y** |
| Mobile + watch companions | **Y** | P | — | — | — | — | — | — | — | — | — | — |

### 4.3 VibeCLI `--serve` vs. cloud-agent products

| Capability | VibeCLI + agent-sdk | Devin | Replit Agent | Bolt.new | v0 | Sweep AI |
|-----------|:---:|:---:|:---:|:---:|:---:|:---:|
| Self-hostable | **Y** | — | — | — | — | — |
| Works offline (local model) | **Y** | — | — | — | — | — |
| Bring-your-own-LLM | **Y** | — | — | — | — | — |
| Full-stack code generation | P | Y | Y | Y | Y (UI only) | P |
| Long-horizon autonomy (hrs) | P | **Y** | Y | P | P | P |
| Browser / shell sandbox | Y | Y | Y | Y (WC) | — | — |
| GitHub issue → PR automation | Y | Y | P | — | — | Y |
| Mobile companion | **Y** | P | Y | — | — | — |
| Open source | **Y** | — | — | — | — | P |

### 4.4 VibeCLI `/review` vs. AI review bots

| Capability | VibeCLI `/review` + CIReviewPanel | CodeRabbit | Qodo | Greptile | Cursor BugBot | Ellipsis |
|-----------|:---:|:---:|:---:|:---:|:---:|:---:|
| Inline PR comments | Y | **Y** | Y | Y | Y | Y |
| Security-focused review | **Y** (red-team) | Y | P | P | P | P |
| Self-hosted option | **Y** | — | P | — | — | — |
| Bring-your-own-LLM | **Y** | — | — | — | — | — |
| Runs locally from CLI | **Y** | — | — | — | — | — |
| Multi-LLM deliberation (Counsel) | **Y** | — | — | — | — | — |
| Cost metering / budgets | **Y** | — | — | — | — | — |
| Compliance reporting | **Y** | — | Y | — | — | — |

### 4.5 VibeMobile / VibeWatch vs. mobile + watch surfaces

| Capability | VibeMobile + VibeWatch | Replit mobile | Cursor mobile (preview) | Devin web | Others |
|-----------|:---:|:---:|:---:|:---:|:---:|
| Native iOS | **Y** | Y | Y | PWA | — |
| Native Android | **Y** | Y | — | PWA | — |
| macOS / Linux / Windows / Web | **Y** | — | — | Web | — |
| Apple Watch native | **Y** | — | — | — | — |
| Wear OS native | **Y** | — | — | — | — |
| Pairs with self-hosted host | **Y** | — | — | — | — |
| Full-duplex session (not read-only) | **Y** | Y | P | P | — |
| Zero-config LAN / Tailscale / ngrok | **Y** | — | — | — | — |
| Handoff-style continuity | **Y** | — | — | P | — |
| Dictated reply on watch | **Y** | — | — | — | — |
| Open source | **Y** | — | — | — | — |

## 5. Differentiators to Exploit

VibeCody has unique advantages to lean into — refreshed for v0.5.5:

1. **Full Rust backend** — lower memory, faster startup, native performance vs. Electron apps (Cursor, Windsurf, Antigravity, Continue, Aide, Void, Cline, PearAI).
2. **Ollama first-class + 22 providers** — the widest provider catalog of any AI coding tool. Cursor/Windsurf treat local models as afterthoughts; Cody requires explicit configuration.
3. **Monorepo synergy** — VibeCLI, VibeUI, VibeCLI App, VibeMobile, and VibeWatch share `vibe-ai` and `vibe-core`; one piece of agent work applies everywhere.
4. **Privacy by design** — no telemetry, no cloud indexing, fully local option. Only OpenHands, Aider, Goose, and Cody offer a comparable story, and none of those ship a polished desktop IDE.
5. **Open source** — full transparency, extensibility, self-hostable. Cursor, Windsurf, Antigravity, Copilot, JetBrains AI, Amazon Q, Devin, Replit, Bolt.new, v0 are all closed.
6. **Wrist-to-terminal coverage** — the *only* product that lets a developer move from desktop → phone → watch within the same session. This is a category VibeCody effectively owns as of 0.5.5.
7. **Zero-config networking** — mDNS, Tailscale Funnel, and ngrok auto-detection give developers a Dropbox-simple setup story no competitor matches.
8. **Counsel + red-team pipelines** — native multi-LLM deliberation and security-focused review bake capabilities into the CLI that competitors position as separate paid products (CodeRabbit, Qodo).

## 6. Implementation Plan

Organized into 5 phases. Each phase builds on the previous and targets specific gap areas.

> **Status:** Phases 1–5 in this document are **complete** as of February 2026. Subsequent phases 6–39 (spanning the v5 and v6 cycles, March–April 2026) are summarised in the **History appendices** at the end of this document. VibeCody has feature parity with Codex CLI, Claude Code, Cursor, Windsurf, and Antigravity across all critical capabilities.

### Phase 1 — Agent Foundation Yes Complete

**Goal:** Give VibeCLI a real agent loop with streaming, tool use, and approval tiers. This is the most critical gap — without it, VibeCLI is just a chat wrapper.

#### 1.1 Streaming TUI Responses

**Crate:** `vibecli-cli/src/tui/`
**Why:** Currently AI responses appear all at once; competitors stream token-by-token.

- In `mod.rs`: replace `llm.chat()` calls with `llm.stream_chat()` in the TUI event loop
- Add `TuiMessage::AssistantChunk(String)` variant to accumulate streaming tokens
- Render partial message with a blinking cursor indicator in `ui.rs`
- Wire up `CompletionStream` → `tokio::spawn` → `mpsc::Sender<AppEvent::Chunk(String)>`

**Files:** `tui/mod.rs`, `tui/app.rs`, `tui/ui.rs`
**Estimate:** 3 days

#### 1.2 Tool Use Framework (`vibe-ai`)

**Crate:** `vibeui/crates/vibe-ai/`
**Why:** All competitors give the LLM structured tools. Without this, no agent loop is possible.

Add to `vibe-ai`:

```rust
// src/tools.rs
pub enum ToolCall {
    ReadFile { path: String },
    WriteFile { path: String, content: String },
    ApplyPatch { path: String, patch: String },
    BashCommand { command: String },
    SearchFiles { query: String, glob: Option<String> },
    ListDirectory { path: String },
    GetGitStatus,
    GetGitDiff { file: Option<String> },
}

pub struct ToolResult {
    pub tool: String,
    pub output: String,
    pub success: bool,
}

pub trait ToolExecutor: Send + Sync {
    async fn execute(&self, call: &ToolCall) -> Result<ToolResult>;
}
```

- Implement `VibeTool` executor in `vibe-core` that dispatches each variant
- Extend `AIProvider` trait with `chat_with_tools()` that sends tools in the provider's native format (OpenAI function calling, Claude tool use, Ollama tool use)
- Parse tool call responses from each provider

**Files:** `vibe-ai/src/tools.rs` (new), `vibe-ai/src/provider.rs`, each `providers/*.rs`
**Estimate:** 1 week

#### 1.3 Agent Loop (`vibe-ai`)

**Why:** The core of Codex CLI and Claude Code — an autonomous plan-act-observe cycle.

```rust
// src/agent.rs
pub struct AgentLoop {
    provider: Arc<dyn AIProvider>,
    tools: Arc<dyn ToolExecutor>,
    approval: ApprovalPolicy,
    max_steps: usize,
}

pub enum ApprovalPolicy {
    Suggest,     // Show every action, require y/N
    AutoEdit,    // Auto-apply file patches, prompt for commands
    FullAuto,    // Execute everything autonomously
}

impl AgentLoop {
    pub async fn run(&self, task: &str, context: &AgentContext) -> Result<AgentResult> {
        // 1. Build system prompt with tools + context
        // 2. Loop: LLM → parse tool calls → approve → execute → feed result back
        // 3. Stop on: task_complete tool, max_steps, error
    }
}
```

- VibeCLI TUI: add `/agent <task>` command that invokes `AgentLoop`
- Show a live "action feed" panel listing each step as it executes
- Wire `ApprovalPolicy` to existing safety config + add `--auto` / `--suggest` / `--full-auto` CLI flags

**Files:** `vibe-ai/src/agent.rs` (new), `vibecli-cli/src/main.rs`, `vibecli-cli/src/tui/`
**Estimate:** 1 week

#### 1.4 Approval Tiers (3-level)

**Why:** Codex CLI's most visible safety feature; binary approve/deny is not enough.

- `Suggest` (default): every file write and command shows diff/preview, requires `y`
- `AutoEdit`: file patches auto-applied; bash commands require approval
- `FullAuto`: all actions execute (only in sandbox or explicit opt-in)

Extend config:

```toml
[safety]
approval_policy = "suggest"   # suggest | auto-edit | full-auto
sandbox = true                # enable OS-level sandbox when full-auto
```

CLI flags: `--suggest`, `--auto-edit`, `--full-auto`

**Files:** `vibecli-cli/src/config.rs`, `vibe-ai/src/agent.rs`
**Estimate:** 2 days

#### 1.5 Multi-File Batch Edits

**Why:** `/apply` only handles one file. Real agent work touches many files.

- Agent tool `WriteFile` + `ApplyPatch` already handles this at the tool level
- Add `BatchApply` confirmation UI in TUI: show all proposed changes as a unified diff across files, single y/N to accept all or file-by-file review
- Preserve undo: create a git stash before applying batch changes

**Files:** `vibecli-cli/src/tui/components/` (new `batch_diff.rs`), `vibe-core/src/git.rs`
**Estimate:** 3 days

### Phase 2 — Context Intelligence Yes Complete

**Goal:** Make VibeCLI and VibeUI context-aware at the codebase level — the core of Cursor's competitive moat.

#### 2.1 Codebase Indexing Engine (`vibe-core`)

**Why:** Cursor indexes 200k tokens of codebase. Currently VibeCLI only injects a truncated git diff.

New module: `vibe-core/src/index/`

```rust
pub struct CodebaseIndex {
    // tree-sitter parsed symbol table
    symbols: HashMap<String, Vec<SymbolInfo>>,
    // file content cache with modification times
    file_cache: HashMap<PathBuf, (SystemTime, String)>,
    // optional: vector embeddings for semantic search
    embeddings: Option<EmbeddingStore>,
}

pub struct SymbolInfo {
    pub name: String,
    pub kind: SymbolKind,  // Function, Struct, Trait, Class, etc.
    pub file: PathBuf,
    pub line: usize,
    pub signature: String,
}
```

Implementation:

- Use `tree-sitter` + language grammars (Rust, TypeScript, Python, Go) to parse symbols
- Walk workspace with `walkdir`, skip `.gitignore` entries
- Incremental re-index on file-change events from `notify`
- Expose `search_symbols(query)` → ranked `Vec<SymbolInfo>`
- Expose `search_content(regex)` → `Vec<(PathBuf, line, snippet)>`
- For semantic search: embed code chunks using a local embedding model via Ollama (`/api/embeddings`) and store in an in-memory HNSW index (`instant-distance` crate)

**Files:** `vibe-core/src/index/` (new directory: `mod.rs`, `symbol.rs`, `content.rs`, `embeddings.rs`)
**Estimate:** 2 weeks

#### 2.2 Context Injection Upgrade

**Why:** Current context injection is naive — 2000 chars of git diff. Competitors provide full codebase understanding.

Replace the static diff injection in `tui/mod.rs` with a smart context builder:

```rust
pub struct ContextBuilder<'a> {
    index: &'a CodebaseIndex,
    git: &'a GitStatus,
    open_files: &'a [PathBuf],
    budget: usize,  // token budget
}

impl ContextBuilder<'_> {
    pub fn build_for_task(&self, task: &str) -> String {
        // 1. Always include: branch, changed files, full diff of changed files
        // 2. Include: symbols most relevant to the task (BM25 + semantic)
        // 3. Fill budget with: content of open files
        // 4. Truncate intelligently at symbol/function boundaries
    }
}
```

**Files:** `vibe-core/src/context.rs` (new), `vibecli-cli/src/tui/mod.rs`
**Estimate:** 3 days

#### 2.3 AGENTS.md / Project Memory

**Why:** Claude Code uses CLAUDE.md; Codex uses AGENTS.md for persistent project-specific instructions.

- On startup, look for `AGENTS.md` or `VIBECLI.md` in CWD and parent directories
- Inject contents as the first system message (before git context)
- VibeCLI command: `/memory edit` — opens `VIBECLI.md` in `$EDITOR`
- VibeCLI command: `/memory show` — prints current memory
- Support tiered memory: global (`~/.vibecli/memory.md`) → repo → directory

**Files:** `vibecli-cli/src/memory.rs` (new), `vibecli-cli/src/main.rs`, `vibecli-cli/src/tui/mod.rs`
**Estimate:** 2 days

#### 2.4 @ Context System (VibeUI)

**Why:** Cursor's most-loved UX feature — `@file`, `@symbol`, `@web`, `@docs` in the chat box.

In `vibeui/src/components/AIChat.tsx`:

- Detect `@` in the input box and open a fuzzy-search popup
- Options: `@file:<path>`, `@symbol:<name>`, `@web:<url>`, `@git:diff`, `@git:history`
- Inject the referenced content into the message before sending
- Backend: Tauri commands `search_files_for_context`, `get_symbol_context`, `fetch_url_content`

**Files:** `vibeui/src/components/AIChat.tsx`, `vibeui/src/components/ContextPicker.tsx` (new), `vibeui/src-tauri/src/commands/context.rs` (new)
**Estimate:** 1 week

### Phase 3 — Inline Intelligence Yes Complete

**Goal:** Wire up LSP and inline AI completions in VibeUI to match Cursor/Windsurf's core editor experience.

#### 3.1 LSP Client — Wire to Monaco

**Why:** The LSP stub exists; it needs to connect to Monaco for real editor intelligence.

Complete `vibe-lsp`:

- Spawn language server process (e.g., `rust-analyzer`, `typescript-language-server`, `pyright`)
- Bridge LSP `textDocument/completion`, `textDocument/hover`, `textDocument/definition`, `textDocument/publishDiagnostics` → Tauri events → Monaco `registerCompletionItemProvider`, `registerHoverProvider`, `setModelMarkers`
- Language server discovery: look for executables in PATH; show install prompt if missing
- Auto-start LSP on file open based on language detection

**Files:** `vibe-lsp/src/client.rs` (complete), `vibe-lsp/src/bridge.rs` (new), `vibeui/src-tauri/src/commands/lsp.rs` (new), `vibeui/src/App.tsx`
**Estimate:** 2 weeks

#### 3.2 Inline AI Completions

**Why:** Cursor's Tab model is the #1 reason developers pay for it.

Implementation strategy:

- Wire `CompletionEngine` from `vibe-ai` to Monaco's `registerInlineCompletionsProvider`
- Debounce: trigger completion 300ms after the user stops typing
- `CodeContext` is built from: Monaco cursor position, surrounding 1000 chars prefix/suffix, active file language
- Render ghost text (grayed-out suggestion) in Monaco
- Accept with `Tab`, dismiss with `Escape`
- For local mode (Ollama): use FIM (fill-in-the-middle) format with `<|fim_prefix|>`, `<|fim_suffix|>`, `<|fim_middle|>` tokens
- For cloud models: use standard prefix+suffix prompt

**Files:** `vibeui/src-tauri/src/commands/completion.rs` (new), `vibeui/src/App.tsx`, `vibeui/crates/vibe-ai/src/completion.rs`
**Estimate:** 1 week

#### 3.3 Flow Awareness Engine (VibeUI)

**Why:** Windsurf's key differentiator — Cascade knows everything you've done. Replicate this.

New Tauri event bus: `FlowTracker`

Track and persist:

- Files opened/closed (with timestamps)
- Files edited (which lines)
- Terminal commands run (command + exit code)
- Clipboard content (on focus events, opt-in)
- Recent AI chat exchanges

Expose as context to AI agent:

```rust
pub struct FlowContext {
    pub recently_viewed: Vec<(PathBuf, Instant)>,
    pub recently_edited: Vec<(PathBuf, Vec<Range>)>,
    pub recent_commands: Vec<(String, i32)>,   // (command, exit_code)
    pub current_file: Option<PathBuf>,
    pub cursor_position: Option<Position>,
}
```

This gets injected into every AI request to give the model full awareness of what the developer is doing.

**Files:** `vibeui/src-tauri/src/flow.rs` (new), `vibeui/src/App.tsx`
**Estimate:** 1 week

#### 3.4 Diff Review Before AI Apply (VibeUI)

**Why:** AI edits currently applied without review — a critical trust gap.

- Any AI-proposed file change goes through a `DiffReview` modal: unified diff with syntax highlighting, accept/reject per hunk
- Before applying: create git stash automatically (silent, named `vibeui-pre-ai-TIMESTAMP`)
- After applying: show "Changes applied — Undo all" button that pops the stash

**Files:** `vibeui/src/components/DiffReview.tsx` (new), `vibeui/src-tauri/src/commands/git.rs`
**Estimate:** 3 days

### Phase 4 — Agentic Editor Yes Complete

**Goal:** Make VibeUI a full agentic IDE — matching Antigravity's Manager View and Cursor's Composer.

#### 4.1 Agent Mode in VibeUI

**Why:** The gap between VibeUI (chat panel) and Cursor/Windsurf (full agent) is the biggest competitive delta.

- New "Agent" tab in the AI panel (alongside "Chat")
- User describes a high-level task: "Add OAuth2 login to the Express app"
- Agent uses the tool framework from Phase 1 to:
  1. Read relevant files via `ReadFile` tool
  2. Search for symbols via `SearchFiles`
  3. Plan a list of changes (shown as a todo list, à la Windsurf)
  4. Execute changes file by file with diff preview
  5. Run tests via `BashCommand` to verify
  6. Report result

- Show a live "Steps" panel listing each action with status (pending/in-progress/done/error)
- Each step is expandable to show tool input/output

**Files:** `vibeui/src/components/AgentPanel.tsx` (new), `vibeui/src-tauri/src/agent.rs` (new), `vibeui/crates/vibe-ai/src/agent.rs`
**Estimate:** 2 weeks

#### 4.2 Memory / Rules System (VibeUI)

**Why:** Both Cursor (`.cursorrules`) and Windsurf (Cascade Memories) have persistent AI instructions.

- Support `.vibeui.md` in workspace root as project-level AI instructions
- Global rules in `~/.vibeui/rules.md`
- Settings panel: "AI Rules" tab for editing rules inline
- Cascade-style auto-memory: after each AI session, offer to save key decisions as a memory snippet
- Knowledge base: searchable store of code snippets and past solutions, surfaced automatically in context

**Files:** `vibeui/src/components/MemoryPanel.tsx` (new), `vibeui/src-tauri/src/memory.rs` (new)
**Estimate:** 1 week

#### 4.3 Checkpoint System (VibeUI)

**Why:** Windsurf's checkpoints let you rewind the entire AI session.

- Before any agent action: create a named git snapshot (`git stash push -m "vibe-checkpoint-N"`)
- Show checkpoint history in a timeline panel
- "Restore to checkpoint N" — pops stash, restores file state
- Checkpoints are auto-created at: session start, before each agent step

**Files:** `vibeui/src/components/CheckpointPanel.tsx` (new), `vibeui/src-tauri/src/checkpoint.rs` (new), `vibe-core/src/git.rs`
**Estimate:** 4 days

#### 4.4 Planning Agent (two-level)

**Why:** Windsurf separates a "planner" (long-horizon) from an "executor" (single-step). This dramatically improves complex task performance.

Implement in `vibe-ai/src/planner.rs`:

```rust
pub struct PlannerAgent {
    planner_model: Arc<dyn AIProvider>,  // frontier model for planning
    executor_model: Arc<dyn AIProvider>, // fast model for execution steps
}

pub struct Plan {
    pub goal: String,
    pub steps: Vec<PlanStep>,
}

pub struct PlanStep {
    pub description: String,
    pub estimated_files: Vec<PathBuf>,
    pub status: StepStatus,
}
```

- Planner LLM: generates the full plan as structured JSON
- For each step: executor LLM performs the actual tool calls
- Planner re-evaluates after each step completes (adaptive planning)
- UI: plan shown as a todo list at the top of the Agent panel; steps update in real time

**Files:** `vibe-ai/src/planner.rs` (new), `vibeui/src/components/AgentPanel.tsx`
**Estimate:** 1 week

#### 4.5 Multi-Agent Parallel Execution

**Why:** Cursor runs 8 parallel agents; Antigravity runs 5. This is the throughput multiplier.

- VibeCLI: `vibecli --agent <task> --parallel N` spawns N sub-processes, each a `AgentLoop` on a git worktree
- VibeUI: "Parallel Agents" view in the Manager tab — spawn up to 5 agents on different tasks simultaneously
- Each agent operates on an isolated git worktree (no conflicts)
- Results merged: show diff comparison of each agent's output, user picks winner or merges

**Files:** `vibe-ai/src/multi_agent.rs` (new), `vibeui/src/components/ManagerView.tsx` (new), `vibe-core/src/git.rs` (add worktree support)
**Estimate:** 2 weeks

### Phase 5 — Ecosystem & Polish Yes Complete

**Goal:** Close the remaining gaps, ship differentiating features, and establish the open ecosystem.

#### 5.1 MCP (Model Context Protocol) Integration

**Why:** Claude Code has 300+ MCP integrations; VibeCLI/VibeUI have zero.

- Implement MCP client in `vibe-ai/src/mcp.rs` — JSON-RPC 2.0 over stdio or SSE
- MCP servers auto-discovered from config:

  ```toml
  [[mcp_servers]]
  name = "github"
  command = "npx @modelcontextprotocol/server-github"

  [[mcp_servers]]
  name = "postgres"
  command = "npx @modelcontextprotocol/server-postgres"
  args = ["postgresql://localhost/mydb"]
  ```

- MCP tools exposed to the agent alongside built-in tools
- MCP resources (e.g., database schema, API docs) injected into context

**Files:** `vibe-ai/src/mcp.rs` (new), `vibecli-cli/src/config.rs`, `vibeui/src-tauri/src/mcp.rs` (new)
**Estimate:** 1.5 weeks

#### 5.2 OS Sandbox for Command Execution

**Why:** Codex CLI uses Apple Seatbelt + Linux seccomp. VibeCLI runs commands unrestricted.

- macOS: wrap command execution in `sandbox-exec` with a restricted profile (no network, write only to CWD)
- Linux: use `bwrap` (bubblewrap) for namespace isolation
- Windows: use `Job Objects` for process isolation
- `FullAuto` mode requires sandbox OR explicit `--no-sandbox` flag

**Files:** `vibe-core/src/executor.rs`, `vibecli-cli/src/config.rs`
**Estimate:** 1 week

#### 5.3 Non-Interactive / CI Mode (VibeCLI)

**Why:** Codex CLI supports `codex exec` for automation pipelines.

```bash
vibecli exec "Add docstrings to all public functions in src/" --auto-edit --output report.md
vibecli exec "Fix all clippy warnings" --full-auto --sandbox
```

- No TUI, no user prompts (except in `suggest` mode which fails with error)
- Writes a structured JSON/markdown report of all actions taken
- Exit codes: 0 (success), 1 (partial), 2 (failed), 3 (approval required)
- GitHub Actions marketplace action: `vibecody/vibecli-action@v1`

**Files:** `vibecli-cli/src/main.rs`, `vibecli-cli/src/ci.rs` (new), `.github/actions/vibecli/` (new)
**Estimate:** 1 week

#### 5.4 Multimodal Input (VibeCLI + VibeUI)

**Why:** Cursor and Codex CLI support pasting screenshots for visual debugging.

- VibeCLI: detect image paths in input (`/chat [image.png] explain this error`)
- VibeUI: drag-and-drop or paste image into chat; encode as base64 and send with the message
- Providers: Claude and OpenAI support vision natively; add image encoding to those providers

**Files:** `vibe-ai/src/provider.rs` (add `ImageContent` to `Message`), `vibe-ai/src/providers/claude.rs`, `vibe-ai/src/providers/openai.rs`, `vibeui/src/components/AIChat.tsx`
**Estimate:** 4 days

#### 5.5 Extension System (VibeUI) — Complete

**Why:** The wasmtime stub exists. Complete it so third parties can extend VibeUI.

Define the extension host API:

```rust
// Host functions exposed to WASM extensions
pub trait ExtensionHost {
    fn register_command(&self, name: &str, handler: Box<dyn Fn(&[&str]) -> Result<String>>);
    fn on_file_save(&self, handler: Box<dyn Fn(&Path)>);
    fn on_text_change(&self, handler: Box<dyn Fn(&Path, &str)>);
    fn read_file(&self, path: &Path) -> Result<String>;
    fn write_file(&self, path: &Path, content: &str) -> Result<()>;
    fn show_notification(&self, message: &str);
    fn get_ai_completion(&self, prompt: &str) -> Result<String>;
}
```

- Extensions loaded from `~/.vibeui/extensions/*.wasm`
- Extension marketplace page on the docs site
- Example extensions: `prettier-format.wasm`, `rustfmt-on-save.wasm`

**Files:** `vibe-extensions/src/host.rs` (complete), `vibe-extensions/src/api.rs` (new), `vibeui/src-tauri/src/extensions.rs`
**Estimate:** 2 weeks

#### 5.6 Trace / Audit Log

**Why:** Codex CLI records every action for inspection and debugging.

- Agent loop writes a structured JSONL trace: `~/.vibecli/traces/<timestamp>.jsonl`
- Each entry: `{ timestamp, step, tool, input, output, duration_ms, approved_by }`
- VibeCLI command: `/trace` — lists recent traces
- VibeCLI command: `/trace view <id>` — renders trace as a human-readable timeline in TUI
- VibeUI: "History" panel showing recent agent sessions with expandable trace

**Files:** `vibe-ai/src/trace.rs` (new), `vibecli-cli/src/tui/components/trace_view.rs` (new), `vibeui/src/components/HistoryPanel.tsx` (new)
**Estimate:** 3 days

## 7. Prioritized Feature Backlog

### Yes Completed — Phases 1–2 (Agent Foundation + Context Intelligence)

| # | Feature | Addresses | Status |
|---|---------|-----------|--------|
| 1 | Streaming TUI responses | Codex, Claude Code | Yes Done |
| 2 | Tool use framework (7 tools) | All | Yes Done |
| 3 | Agent loop (plan→act→observe) | Codex, Claude Code | Yes Done |
| 4 | Approval tiers (Suggest/AutoEdit/FullAuto) | Codex, Claude Code | Yes Done |
| 5 | Multi-file batch edits | All | Yes Done |
| 6 | Codebase indexing (regex/heuristic + embeddings) | Cursor, Windsurf | Yes Done |
| 7 | Project memory (AGENTS.md / VIBECLI.md) | Codex, Claude Code | Yes Done |
| 8 | Diff review before apply | All | Yes Done |

### Yes Completed — Phase 3 (Inline Intelligence)

| # | Feature | Addresses | Status |
|---|---------|-----------|--------|
| 9 | LSP in Monaco (completions, hover, go-to-def) | Cursor, Windsurf | Yes Done |
| 10 | Inline AI completions (FIM) | Cursor, Windsurf | Yes Done |
| 11 | @ context system | Cursor, Windsurf | Yes Done |
| 12 | Flow-awareness engine (FlowTracker) | Windsurf | Yes Done |

### Yes Completed — Phases 4–5 (Agentic Editor + Ecosystem)

| # | Feature | Addresses | Status |
|---|---------|-----------|--------|
| 13 | Agent mode in VibeUI (AgentPanel) | Antigravity, Cursor | Yes Done |
| 14 | Memory / rules (MemoryPanel) | Cursor, Windsurf | Yes Done |
| 15 | Checkpoint system (backend + UI) | Windsurf | Yes Done |
| 16 | MCP integration (JSON-RPC 2.0 stdio) | Claude Code, Codex | Yes Done |
| 17 | OS sandbox (sandbox-exec / bwrap) | Codex | Yes Done |
| 18 | CI mode (--exec, JSON/Markdown reports) | Codex, Claude Code | Yes Done |
| 19 | Multimodal input (Claude + OpenAI vision) | Cursor, Claude Code | Yes Done |
| 20 | Extension system (WASM wasmtime) | Cursor, Windsurf | Yes Done |
| 21 | Trace / audit log (JSONL + HistoryPanel) | Codex | Yes Done |
| 22 | Multi-agent parallel (git worktrees + ManagerView) | Cursor, Antigravity | Yes Done |
| 23 | Planning agent (PlannerAgent) | Windsurf, Antigravity | Yes Done |

### Yes Completed — Phases 6–9 (see ROADMAP-v2)

| # | Feature | Addresses | Status |
|---|---------|-----------|--------|
| 24 | Hooks system (events + shell + LLM handlers) | Claude Code | Yes Done |
| 25 | Plan Mode (PlannerAgent) | Windsurf, Claude Code | Yes Done |
| 26 | Parallel multi-agent + git worktrees | Cursor (8), Windsurf | Yes Done |
| 27 | Embedding-based semantic indexing | Cursor, Windsurf | Yes Done |
| 28 | Next-edit prediction (Tab/Supercomplete) | Cursor, Windsurf | Yes Done |
| 29 | Checkpoint UI panel | Windsurf | Yes Done |
| 30 | Session resume | Codex, Claude Code | Yes Done |
| 31 | Web search tool | Codex | Yes Done |
| 32 | GitHub PR review agent (BugBot equiv.) | Cursor BugBot | Yes Done |
| 33 | Shell environment policy / Admin policy | Codex | Yes Done |
| 34 | Skills system | Claude Code, Windsurf | Yes Done |
| 35 | Artifacts panel | Antigravity | Yes Done |
| 36 | OpenTelemetry | Codex | Yes Done |
| 37 | GitHub Actions | Codex, Claude Code | Yes Done |
| 38 | Manager View (parallel UI) | Antigravity | Yes Done |
| 39 | VS Code extension | All | Yes Done |
| 40 | Agent SDK (TypeScript) | Claude Code | Yes Done |

## 8. Architecture Summary (All Phases Complete)

```text
vibecli-cli
├── REPL / TUI (streaming, hooks, /agent, /plan, /multi-agent, /review)
├── CI mode (--exec, --parallel, --review)
├── Server mode (vibecli serve — API for VS Code extension + SDK)
└── src/
    ├── ci.rs, review.rs, serve.rs, otel_init.rs
    └── hooks (config loading)

vibe-ai
├── provider.rs         (AIProvider trait + vision + tool use)
├── agent.rs            (plan→act→observe loop, approval tiers)
├── planner.rs          (PlannerAgent: plan generation + guided execution)
├── multi_agent.rs      (parallel agents on git worktrees)
├── hooks.rs            (HookRunner: command + LLM handlers, event bus)
├── skills.rs           (SkillLoader: auto-activating context snippets)
├── artifacts.rs        (Artifact types, annotation queue)
├── mcp.rs              (McpClient JSON-RPC 2.0)
├── tools.rs            (ToolCall enum + WebSearch + FetchUrl)
├── trace.rs            (JSONL audit + session resume)
├── policy.rs           (AdminPolicy: tool/path restrictions)
└── otel.rs             (OpenTelemetry span attributes)

vibe-core
├── index/
│   ├── mod.rs, symbol.rs  (tree-sitter symbol index)
│   └── embeddings.rs      (HNSW vector index, Ollama/OpenAI embeddings)
├── context.rs          (smart context builder: flow + semantic + git)
├── executor.rs         (sandboxed execution + shell env policy)
└── git.rs              (worktree: create, remove, merge)

vibe-extensions
└── loader.rs           (wasmtime WASM host)

vibeui (React + Tauri)
├── AgentPanel          (single-agent: steps, approval, artifacts)
├── ManagerView         (multi-agent: task board, worktrees, merge)
├── CheckpointPanel     (timeline, restore, auto-checkpoint)
├── ArtifactsPanel      (rich cards, annotations, async feedback)
├── MemoryPanel         (rules editor)
├── HistoryPanel        (trace viewer)
├── ContextPicker       (@ context popup)
├── GitPanel            (git + PR review)
└── Terminal, AIChat, CommandPalette, ThemeToggle

vscode-extension        (chat, inline completions, agent mode)
packages/agent-sdk      (TypeScript SDK: @vibecody/agent-sdk)
.github/actions/vibecli (GitHub Actions marketplace action)
```

## 9. Key Differentiators (Current — v0.5.5, April 2026)

### 9.1 Broad feature matrix — 14 competitors

`Y` = native support; `P` = partial / limited; `—` = not supported. Ordered by the features we care about most.

| Dimension | VibeCody 0.5.5 | Cursor | Windsurf | Antigravity | Claude Code | Codex CLI | Aider | Goose | OpenHands | Cline | Cody | Copilot | JetBrains AI | Devin |
|-----------|:---:|:---:|:---:|:---:|:---:|:---:|:---:|:---:|:---:|:---:|:---:|:---:|:---:|:---:|
| Open source | **Y** | — | — | — | — | P | Y | Y | Y | Y | P | — | — | — |
| Rust native backend | **Y** | — | — | P | — | Y | — | Y | — | — | — | — | — | — |
| Local AI first (Ollama) | **Y** | P | P | — | — | P | Y | Y | P | Y | P | — | — | — |
| Self-hostable daemon | **Y** | — | — | — | — | P | Y | Y | Y | Y | P | — | — | — |
| CLI + GUI unified | **Y** | P | — | P | P | — | — | — | — | — | Y | — | P | — |
| Terminal TUI | **Y** | — | — | — | P | Y | Y | Y | P | — | — | — | — | — |
| Desktop IDE (Monaco) | **Y** | Y | Y | Y | — | — | — | — | — | — | Y | Y | Y | — |
| Flutter mobile (6 platforms) | **Y** | P | — | — | — | — | — | — | — | — | — | — | — | P |
| **Apple Watch native** | **Y** | — | — | — | — | — | — | — | — | — | — | — | — | — |
| **Wear OS native** | **Y** | — | — | — | — | — | — | — | — | — | — | — | — | — |
| Handoff-style continuity | **Y** | — | — | — | — | — | — | — | — | — | — | — | — | P |
| mDNS / Tailscale / ngrok zero-config | **Y** | — | — | — | — | — | — | — | — | — | — | — | — | — |
| Multi-provider (≥10) | **Y** (22) | P | P | P | — | P | Y | Y | Y | Y | Y | — | P | — |
| Full agent loop | **Y** | Y | Y | Y | Y | Y | P | Y | Y | Y | P | P | Y | Y |
| Parallel agents | **Y** | Y (8) | Y | Y (5) | Y | P | — | — | — | — | — | — | — | — |
| Plan Mode | **Y** | — | Y | Y | Y | — | — | — | Y | Y | — | Y | Y | Y |
| MCP integration | **Y** | Y | P | Y | Y (300+) | Y | — | Y | P | — | P | — | P | — |
| Hooks system | **Y** | — | — | — | Y | — | — | — | — | — | — | — | — | — |
| Skills system | **Y** (711) | — | Y | — | Y | — | — | — | — | — | — | — | — | — |
| OS sandbox | **Y** | — | — | — | P | Y | — | — | Y | — | — | — | — | Y |
| Inline completions (FIM) | **Y** | Y | Y | Y | — | — | — | — | — | — | P | Y | Y | — |
| Semantic codebase index | **Y** | Y | Y | Y | P | Y | P | P | P | P | Y | P | P | Y |
| Multi-file batch edits | **Y** | Y | Y | Y | Y | Y | Y | Y | Y | Y | P | P | Y | Y |
| Diff review before apply | **Y** | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | P | Y | Y |
| Checkpoint / rewind | **Y** | P | Y | Y | — | P | — | Y | P | — | — | — | — | Y |
| Trace / audit log (JSONL) | **Y** | — | — | — | P | Y | — | Y | Y | — | — | — | — | P |
| OpenTelemetry | **Y** | — | — | — | — | Y | — | — | — | — | — | — | — | — |
| VS Code extension | **Y** | Y | Y | Y | Y | Y | — | — | — | Y | Y | Y | — | — |
| JetBrains plugin | **Y** | — | Y | — | — | — | — | — | — | — | Y | Y | Y | — |
| Neovim plugin | **Y** | — | — | — | — | — | — | — | — | — | P | Y | — | — |
| Agent SDK (TypeScript) | **Y** | — | — | — | Y | P | — | Y | — | — | — | — | — | P |
| Red-team security pipeline | **Y** | — | — | — | — | — | — | — | — | — | — | — | — | — |
| Counsel (multi-LLM deliberation) | **Y** | — | — | — | — | — | — | — | — | — | — | — | — | — |
| Artifacts + Manager View | **Y** | — | — | Y | — | — | — | — | — | — | — | — | — | Y |
| Multiplayer CRDT collab | **Y** | — | — | — | — | — | — | — | — | — | — | — | — | — |
| CRDT / real-time sync on mobile+watch | **Y** | — | — | — | — | — | — | — | — | — | — | — | — | — |

### 9.2 Where VibeCody is **unique**

As of **v0.5.5 (April 2026)**, VibeCody is the only product in our 52-tool survey that ships all of these simultaneously:

1. **A native Apple Watch client** — SwiftUI, P-256 ECDSA via Secure Enclave, dictated reply.
2. **A native Wear OS client** — Kotlin/Compose, Android Keystore / StrongBox attestation.
3. **A 6-platform Flutter mobile companion** (iOS, Android, macOS, Linux, Windows, Web) with Handoff-style continuity.
4. **Zero-config device discovery** — mDNS `_vibecli._tcp.local.` + Tailscale Funnel + ngrok, auto-raced.
5. **A Rust-native, fully open-source, self-hostable daemon** that drives all of the above.
6. **22 AI providers** behind a single abstraction with failover — the widest catalog of any AI coding tool.
7. **A terminal (VibeCLI) + full desktop IDE (VibeUI) + chat desktop app (VibeCLI App) + mobile + watch** — every surface built on the same crates.
8. **Counsel** — structured multi-LLM deliberation (expert / devil's advocate / skeptic / pragmatist) with a moderator synthesis; no other tool ships this.
9. **Red-team security pipeline + compliance reporting** built into the CLI.

### 9.3 Where we still have parity gaps to close

The honest list (tracked in the consolidated [Fit-Gap Analysis](./fit-gap-analysis/)):

- **Cursor's Tab model** — next-edit prediction quality is still best-in-class; we ship FIM completions but haven't matched their specialized model.
- **Devin-level long-horizon autonomy** — Devin chains hours of work in a cloud VM; our agent loop tops out at ~50 steps.
- **Claude Code's 300+ MCP servers** — we ship a compliant MCP client, but the community server catalog is still dominated by Anthropic.
- **SWE-bench leaderboard** — Claude Mythos Preview now leads SWE-bench Verified at 93.9% (Opus 4.7 87.6%, GPT-5.3-Codex 85%, Augment 72.0% open-system pass@1). **Caveat (2026-05-03):** OpenAI stopped reporting Verified scores after a contamination audit found 59.4% of hard tasks have flawed tests — SWE-bench Pro, SWE-rebench, and SWE-bench-Live are now the primary references. Tracked in the benchmark panel.
- **Enterprise SSO / audit packaging** — Cody and Copilot for Business are further along on SOC 2, SSO, central policy.

### 9.4 Headline positioning

> **VibeCody is the only open-source AI developer toolchain that lets you work on the same coding session from a terminal, a desktop IDE, a phone, and a watch — all running against a self-hosted Rust daemon with 22 providers and zero-config networking.**

Every competitor has a better story in *one* dimension; none of them ship a coherent answer across **all six** categories (terminal, editor, cloud, review, completions, mobile/watch) the way VibeCody does as of v0.5.5.

---

## Appendix A — History: Phases 23–32 (v5 cycle, March 2026)

**Input:** 22 competitive gaps catalogued in the v7 fit-gap iteration (see [Fit-Gap Analysis](./fit-gap-analysis/)) plus six bonus modules in Phase 32.
**Outcome:** All 22 gaps closed; 28 new Rust modules, 9,570 unit tests (0 failures), 187 panels, 568 skills, 100+ REPL commands. Completed 2026-03-29.

### Phase 23 — Dual-Protocol Agent Communication (P0)

- **23.1 A2A Protocol Support** — `a2a_protocol.rs` (agent card, server mode, client discovery, task lifecycle, SSE streaming, capability negotiation), `A2aPanel.tsx`, `/a2a card|serve|discover|call|tasks|status`, 55+ tests.
- **23.2 Agent Skills Standard Compatibility** — `agent_skills_compat.rs` (cross-tool skills interop, import/export, registry client, dependency resolution), `/skills import|export|search|validate|publish`, 35+ tests.

### Phase 24 — Parallel Agent Workers (P0)

- **24.1 Worktree Pool** — `worktree_pool.rs` with N lightweight git-worktree agents, auto-merge orchestration, per-worktree cgroup/ulimit caps; `WorktreePoolPanel.tsx`; `/worktree spawn|list|merge|cleanup|config`; 50+ tests.
- **24.2 Multi-Agent Terminal Host** — `agent_host.rs` hosts external CLI agents (Claude Code, Gemini CLI, Aider, …) with interleaved output and shared clipboard; `AgentHostPanel.tsx`; `/host add|list|route|remove|ask`; 40+ tests.

### Phase 25 — Proactive Intelligence (P0)

- **25.1 Proactive Agent** — `proactive_agent.rs` background scanner across performance/security/tech-debt/correctness/a11y/testing categories with learning store and digest mode; `ProactivePanel.tsx`; `/proactive scan|config|accept|reject|history|digest`; 45+ tests.
- **25.2 Autonomous Issue Triage** — `issue_triage.rs` classifier + severity estimator + auto-labeler + draft-response generator with GitHub/Linear integration; `TriagePanel.tsx`; `/triage run|rules|labels|history|batch`; 40+ tests.

### Phase 26 — Agent Grounding & Context (P0/P1)

- **26.1 Web Search Grounding** — `web_grounding.rs` with Google/Bing/Brave/SearXNG/Tavily providers, citation tracking, privacy mode; `WebGroundingPanel.tsx`; `/search web|cache|providers|config`; 40+ tests.
- **26.2 Deep Semantic Codebase Index** — `semantic_index.rs` with call-graph, type-hierarchy, import-chain, incremental updates; `SemanticIndexPanel.tsx`; `/index build|query|callers|callees|hierarchy|deps|stats`; 55+ tests.

### Phase 27 — MCP Protocol Evolution (P1)

- **27.1 Streamable HTTP + OAuth 2.1** — `mcp_streamable.rs` bidirectional streamable HTTP, PKCE OAuth client + server, SAML→OAuth bridge, connection pooling; `/mcp serve-http|oauth|tokens|remote`; 45+ tests.

### Phase 28 — Smart Repair & Routing (P1/P2)

- **28.1 MCTS Code Repair** — `mcts_repair.rs` Monte-Carlo Tree Search for code repair (UCB1, rollout tests, agentless mode, cost tracking, SWE-bench integration); `MctsRepairPanel.tsx`; `/repair mcts|agentless|compare|config`; 50+ tests.
- **28.2 Cost-Optimized Agent Routing** — `cost_router.rs` task-complexity-aware model routing with budget enforcement and A/B tracking; `CostRouterPanel.tsx`; `/route cost|budget|model|stats|compare`; 40+ tests.

### Phase 29 — Developer Experience (P1/P2)

- **29.1 Visual Verification** — `visual_verify.rs` headless-Chrome screenshot + perceptual-diff + baseline mgmt + CI integration; `VisualVerifyPanel.tsx`; `/verify screenshot|diff|baseline|ci`; 35+ tests.
- **29.2 Next-Task Prediction** — `next_task.rs` workflow-state-machine-driven suggestion engine; `NextTaskPanel.tsx`; `/nexttask suggest|accept|reject|learn|stats`; 40+ tests.
- **29.3 Offline Voice Coding** — `voice_local.rs` whisper.cpp integration with model mgmt, VAD, streaming, fallback to Groq; `/voice local|model|download|config`; 30+ tests.
- **29.4 Living Documentation Sync** — `doc_sync.rs` bidirectional spec↔code reconciliation with drift alerts; `DocSyncPanel.tsx`; `/docsync status|reconcile|watch|freshness`; 35+ tests.

### Phase 30 — Enterprise & Ecosystem (P2/P3)

- **30.1 Native Integration Connectors** — `native_connectors.rs` with 20 pre-built connectors (Stripe, Figma, Notion, Jira, Slack, PagerDuty, Datadog, Sentry, LaunchDarkly, Vercel, Netlify, Supabase, Firebase, AWS, GCP, Azure, GitHub, GitLab, Linear, Confluence); `ConnectorsPanel.tsx`; `/connect list|add|test|remove|webhook`; 50+ tests.
- **30.2 Enterprise Agent Analytics** — `agent_analytics.rs` per-user/team/project metrics, ROI calculator, CSV/JSON/PDF export; `AnalyticsPanel.tsx`; `/analytics dashboard|export|roi|compare`; 40+ tests.
- **30.3 Agent Trust Scoring** — `agent_trust.rs` 0-100 per-agent/per-domain trust with decay, auto-review thresholds, transparent explanations; `TrustPanel.tsx`; `/trust scores|history|config|explain`; 35+ tests.
- **30.4 Agentic Package Manager** — `smart_deps.rs` dependency-graph analysis, CVE auto-patch, license compliance, monorepo-aware lockfile mgmt; `SmartDepsPanel.tsx`; `/deps resolve|compare|patch|audit|graph`; 40+ tests.

### Phase 31 — Strategic Frontiers (P3)

- **31.1 RLCEF Training Loop** — `rlcef_loop.rs` execution-based learning with outcome tracker, reward signals, mistake clustering, fine-tuning export (opt-in, local-only); `RlcefPanel.tsx`; `/rlcef train|eval|mistakes|patterns|reset|export`; 45+ tests.
- **31.2 LangGraph Bridge** — `langgraph_bridge.rs` LangGraph-compatible REST API, checkpoint format interop, Python SDK wrapper; `LangGraphPanel.tsx`; `/langgraph serve|connect|status|checkpoint`; 35+ tests.
- **31.3 Sketch Canvas** — `sketch_canvas.rs` freeform drawing → React/HTML/SwiftUI component generation, 3D scene export; `SketchCanvasPanel.tsx`; `/sketch new|recognize|generate|export`; 30+ tests.

### Phase 32 — Advanced Agent Intelligence (Bonus)

- **32.1** `context_protocol.rs` (streaming long-running context), `code_review_agent.rs` (rule-driven review), `diff_review.rs` (diff-aware review).
- **32.2** `code_replay.rs` (reproducible past sessions), `speculative_exec.rs` (predictive path execution), `explainable_agent.rs` (interpretable reasoning).
- **32.3** TurboQuant KV-cache compression (PolarQuant + QJL, ~3 bits/dim) with benchmark panel and REPL command.

---

## Appendix B — History: Phases 33–39 (v6 cycle, April 2026)

**Input:** 18 competitive gaps catalogued in the v8 fit-gap iteration (see [Fit-Gap Analysis](./fit-gap-analysis/)).
**Outcome:** All 18 gaps closed; 18 new Rust modules, ~13,270 unit tests (0 failures), 210+ panels, 122+ REPL commands, 212+ Rust modules under `vibecli-cli/src/`. Completed 2026-04-11.

### Phase 33 — Cross-Environment Agent Execution (P0)

- **33.1 Cross-Environment Parallel Dispatch** — `env_dispatch.rs` with `Local | GitWorktree | RemoteSSH | CloudVM` executors, pool pre-warming, unified progress aggregator, cost ticker; `EnvDispatchPanel.tsx`; `/dispatch local|worktree|ssh|cloud|status|pool`; 55+ tests.
- **33.2 Recursive Nested Subagents** — `nested_agents.rs` DAG of parent/child agents with depth limiter, context-inheritance policies, merge strategies, real-time graph visualiser; `NestedAgentsPanel.tsx`; `/agents tree|spawn|depth|graph|cancel`; 50+ tests.
- **33.3 A2A v0.3 Update** — `a2a_protocol.rs` extended with gRPC transport (tonic), Ed25519 security-card signing, v0.3 schema + v0.2 shim, Python-SDK interop tests; `/a2a grpc|sign|verify|compat`; 30+ tests.

### Phase 34 — Active Desktop Computer Use (P0)

- **34.1 Active Desktop Control Agent** — `desktop_agent.rs` with xdotool/AT-SPI (Linux), AXUIElement/CGEvent (macOS), UI Automation (Windows), CDP browser debugger, MJPEG live preview, video recording, allow-list safety; `DesktopAgentPanel.tsx`; `/desktop click|type|scroll|screenshot|record|stop|replay`; 45+ tests.

### Phase 35 — Protocol Maturation (P1)

- **35.1 MCP Enterprise Governance** — `mcp_governance.rs` with append-only audit log (SIEM-exportable), OIDC/SAML SSO, JSON-schema policy DSL for allow/deny + rate limits, versioned config portability; `McpGovernancePanel.tsx`; `/mcp audit|sso|gateway|config`; 50+ tests.
- **35.2 Microsoft Agent Framework 1.0** — `msaf_compat.rs` MSAF manifest generation, Azure-AD token validation, MCP↔MSAF envelope shim, Azure Agent Catalog registration + heartbeat; `MsafPanel.tsx`; `/msaf register|manifest|catalog|health|token`; 35+ tests.

### Phase 36 — Agent Intelligence Primitives (P1)

- **36.1 Agent Await** — `agent_await.rs` first-class conditional-pause tool (`ProcessExit | LogPattern | FileChange | PortOpen | HttpReady | TimerElapsed | ManualResume`); tokio-select poller; `/await list|cancel|status`; 40+ tests.
- **36.2 Streaming Thoughts** — `thought_stream.rs` parses `<thinking>` blocks from Claude/Gemini/GPT streams, categorises Planning/Reasoning/Uncertainty/Decision/Observation, tags confidence, exports annotated Markdown; `ThoughtStreamPanel.tsx`; `/thoughts live|history|export|filter`; 35+ tests.
- **36.3 Codebase-Vocabulary Voice** — `voice_vocab.rs` mines identifiers from `semantic_index.rs`, injects them into Whisper as `initial_prompt` + hotwords, tracks WER improvement; `/voice vocab build|inject|stats|test`; 30+ tests.

### Phase 37 — Context & Collaboration (P2)

- **37.1 Ultra-Long Context Adapter (2M–10M tokens)** — `long_context.rs` with Gemini 3.1 Pro (2M), Llama 4 Scout (10M), Claude Opus 4.6 (1M) routing, semantic-boundary chunking, sliding-window pagination, cost estimator, monorepo ingestion; `LongContextPanel.tsx`; `/ctx route|estimate|ingest|window`; 45+ tests.
- **37.2 Interactive Design Mode** — `design_mode.rs` SVG annotation canvas (Arrow/Region/TextLabel/BeforeAfter/ColorSwatch/Measurement) → structured natural-language instruction generator with design-token extraction; `DesignModePanel.tsx`; `/design screenshot|annotate|generate|history`; 40+ tests.
- **37.3 VibeCLI ↔ VibeUI Context Bridge** — `ide_bridge.rs` (UDS on macOS/Linux, named pipe on Windows) publishing open files, cursor, test/build output, terminal tail; VibeCLI client auto-discovers and injects `<ide_context>` into the agent window; `IdeBridgePanel.tsx`; `/ide connect|status|sync|disconnect`; 35+ tests.

### Phase 38 — Private & Robust Intelligence (P2)

- **38.1 On-Device Private Inference** — `on_device.rs` with GGUF model registry (SHA-256 verified HF download), `llama-cpp-rs` FFI + `candle` fallback, Metal/CUDA/ROCm/AVX2 backends, hardware capability probe, `--local-only` network-isolation enforcement, benchmark runner; `OnDevicePanel.tsx`; `/ondevice download|list|run|bench|enforce|hardware`; 45+ tests.
- **38.2 Hard Problem-Solving Strategy Engine** — `hard_problem.rs` with task decomposition + assumption surfacer + incremental hypothesis tester + ambiguity resolver + multi-file change planner + complexity estimator; `HardProblemPanel.tsx`; `/plan decompose|assume|hypothesize|clarify|estimate`; 40+ tests.

### Phase 39 — Strategic Ecosystem (P3)

- **39.1 Autonomous Deploy Pipeline Agent** — `auto_deploy.rs` closed-loop plan→build→test→stage→health-check→promote pipeline over `DockerCompose | Kubernetes | Serverless | StaticHosting`, health gates, auto-rollback, dry-run; `AutoDeployPanel.tsx`; `/deploy plan|dry-run|stage|promote|rollback|status`; 50+ tests.
- **39.2 Claw Code Framework Compatibility** — `clawcode_compat.rs` JSON-RPC worker protocol, registry file interop, task-type routing, capability advertisement, bidirectional client/server; `/clawcode register|serve|workers|status|call`; 35+ tests.
- **39.3 Team Onboarding Intelligence** — `team_onboarding.rs` new-member detector + usage-pattern analyzer + knowledge-gap report + auto-generated ramp-up guide + hotspot map + team admin view; `TeamOnboardingPanel.tsx`; `/onboard generate|track|guide|hotspots|team`; 35+ tests.
- **39.4 Reproducibility-First Agent Architecture** — `repro_agent.rs` hermetic session snapshot (lockfiles + env hash + seed), deterministic replayer, session differ, CI reproducibility gate, non-determinism tagger, portable `repro-bundle.tar.gz`; `ReproAgentPanel.tsx`; `/repro snapshot|replay|diff|verify|export|import`; 40+ tests.

---

## Appendix C — Phases 40–52 and topic-specific deep-dives

Iterations v10 (phases 40–43), v11 (phases 45–48), v12 (phases 49–52) and the five topic-specific fit-gaps (AgentOS, Pi-mono, RL-OS, Paperclip, Code-Review+Architecture) are fully absorbed into the [Fit-Gap Analysis](./fit-gap-analysis/). That document is the canonical source for the module-by-module ledger; this roadmap tracks competitive positioning and phase-level history only.

---

## Appendix D — Phase 53: April 2026 trend delta + audit reconciliation

**Input:** §16 of the [Fit-Gap Analysis](./fit-gap-analysis/) — 11 newly-identified open gaps (A1–A11) from the 2026-04-26 industry trend survey, plus 8 audit-flagged modules + the RL-OS subsystem queued for real-I/O conversion (US-007…US-015) following the same playbook as the already-shipped US-001…US-006.

**Outcome target:** v0.5.6 ships with all eight audit-flagged modules converted to real I/O (US-007…US-014), one RL-OS algorithm shipped end-to-end via candle (US-015), and at least 6 of the 11 v13 trend-delta gaps shipped (A1–A4 MCP work + A11 Junie-style migration tool + A6 multi-root agent). The remaining 5 v13 items (A5/A7/A8/A9/A10) are scheduled into v0.5.7 since they require larger UX work or new dependencies.

**Why these are grouped into one phase rather than split:** the audit-reconciliation work and the new MCP/ACP work both need the same `axum` + `reqwest` + mock-server-BDD-harness scaffolding. Sequencing them lets each US-### conversion reuse the harness from the previous one — that's the productivity pattern that produced the US-001…US-006 cadence in 6 weeks.

### 53.1 Audit reconciliation — real-I/O conversions (P0)

Same playbook as US-001 (web grounding) → US-006 (proactive scanner). Each conversion adds an axum mock-server BDD harness identical in shape to the existing ones in `vibecli-cli/tests/`.

- **US-007 — `issue_triage.rs` GitHub/Linear HTTP** — `octocrab` + Linear SDK; `VIBECLI_GITHUB_TOKEN` / `VIBECLI_LINEAR_TOKEN` env gating; mock server for issue listing + label updates + comment posting; 4 BDD scenarios.
- **US-008 — `native_connectors.rs` first 5 connectors** — Stripe + Slack + Linear + Notion + GitHub with real `oauth2`-crate flow; the remaining 15 connectors stay endpoint-string-only and are explicitly marked **deferred**; 8 BDD scenarios across the 5 connectors.
- **US-009 — `langgraph_bridge.rs` REST API** — `axum` server exposing LangGraph's documented routes (`/threads/{id}/runs`, `/threads/{id}/state`); checkpoint JSON schema validation; 5 BDD scenarios using the LangGraph Python SDK as the conformance test.
- **US-010 — `mcts_repair.rs` real rollouts** — wire `rollout` to spawn `cargo test` / `pytest` / `npm test` per detected language; per-rollout time budget; reward = test exit code; 4 BDD scenarios on synthetic broken-test repos under `tests/fixtures/mcts/`.
- **US-011 — `sketch_canvas.rs` 2D wireframe → React JSX** — defer 3D entirely (mark out-of-scope in the doc + scoreboard); ship the 2D path against tldraw's JSON schema or an OSS recognizer like `react-sketch-canvas`; 3 BDD scenarios.
- **US-012 — `cost_router.rs` real routing** — wire to `provider.rs` retry + circuit breaker; track per-(provider, model) latency/cost in `agent_analytics.rs`; routing decisions become a real function of observed data; 4 BDD scenarios using a mock provider with controllable latency.
- **US-013 — `semantic_index.rs` tree-sitter rewrite** — replace the `trimmed.starts_with("pub fn")` regex with `tree-sitter` + per-language grammars (Rust, TS, Python, Go); reuse the existing tree-sitter setup from `vibe-core/src/index/symbol.rs`; add call-graph extraction; 6 BDD scenarios.
- **US-014 — `linter_aggregator` real linters** — replace `simulate_linter()` with subprocess spawn for clippy / eslint / pylint / golangci-lint / shellcheck / hadolint / yamllint / mypy; parse stdout into the existing `Finding` schema; the FP-filter LLM pass already exists; 8 BDD scenarios (one per linter).
- **US-015 — RL-OS one real algorithm end-to-end** — ship PPO with `candle` on CPU as the proof-of-shape; expose via PyO3 bindings; the 52 type-system entries become "real" once *one* training loop is real and the others can follow incrementally. Test: `tests/rl_ppo_cartpole.rs` runs the loop on a small env and asserts reward improves over 100 episodes.

### 53.2 v13 trend-delta items — new builds (P0/P1)

Numbered to match §16.1 of the [Fit-Gap Analysis](./fit-gap-analysis/).

**P0 (this cycle):**

- **A1 — MCP Apps support** — render `application/vnd.mcp.app+json` payloads as embedded React in `AIChat.tsx`; security: same CSP as the WASM extension host; 4 BDD scenarios.
- **A2 — MCPB bundle format** — `vibecli mcp install <bundle.mcpb>` extracts + verifies signature + registers the local server; round-trip with `vibecli mcp pack` for our own example servers; 3 BDD scenarios.
- **A3 — MCP `.well-known` capability discovery** — `vibecli serve` exposes `GET /.well-known/mcp.json` listing tool/prompt/resource catalogs; capability advertisement without a live SSE connection; 3 BDD scenarios.
- **A4 — ACP server mode** — VibeCLI as an ACP server callable from Zed/JetBrains/Neovim; reuse the JSON-RPC scaffolding from our existing MCP client; ship the matching JetBrains plugin update (the Neovim plugin already speaks ACP-shaped JSON-RPC); 5 BDD scenarios using the official ACP test harness.
- **A6 — Multi-root workspace agent** — extend `--add-dir` from read-only to read+write; agent tool calls accept a `workspace_root: <path>` field; sandbox enforces per-root permissions; 4 BDD scenarios across 2-root and 3-root configurations.
- **A11 — Migration tool from Claude Code / Codex configs** — `vibecli migrate from-claude-code` / `vibecli migrate from-codex`: read existing `CLAUDE.md`, `~/.claude.json`, `codex.toml`, MCP server lists; emit `VIBECLI.md` + `~/.vibecli/config.toml` + `~/.vibecli/mcp_servers.toml`; 6 BDD scenarios covering common config shapes.

**P1 (next cycle, v0.5.7):**

- **A5 — Async subagents** — extend `nested_agents.rs` with `await_later: bool`; long-running subagents persist state in SQLite, reachable via `/agents resume <id>`; UI surface in `NestedAgentsPanel.tsx` showing "running in background" + notification on completion.
- **A7 — Browser-native UI-element annotation Design Mode** — extend `desktop_agent.rs` browser-control track with DOM-element click-to-annotate, generating natural-language instructions tied to specific selectors. **Patent-distance check required** — design must remain distant from Cursor 3's annotation UX; consult [notes/PATENT_AUDIT_INLINE.md](../notes/PATENT_AUDIT_INLINE.md) before building.
- **A8 — Self-verifying agent loop** — feed `visual_verify.rs` failures back into the agent loop as `ToolResult { success: false, output: "<diff details>" }`; bound the auto-fix loop at 3 iterations; `desktop_agent.rs` tests the running app via real clicks/typing.
- **A9 — Cloud-agent remote-control protocol** — extend the VibeMobile/VibeWatch pairing flow with a `/sessions/<id>/resume` endpoint; the server-side session state is already persisted in `~/.vibecli/sessions/`; needs a session-handoff UX in the watch + mobile clients.
- **A10 — Skills hot-reload + real-time progress** — `notify`-based watcher on the skills directory; emit `SkillEvent::Reloaded` to all attached agents; UI streams skill execution events via the existing event bus.

### 53.3 Patent-distance posture

Three of the v13 items (A5 async subagents, A7 Design Mode, A10 manager-style UI consolidation hinted in §1bis) sit close to product surfaces that competitors have invested heavily in. Per the existing patent-distance posture for `diffcomplete` ([notes/PATENT_AUDIT_INLINE.md](../notes/PATENT_AUDIT_INLINE.md), gitignored working doc), each of these designs must:

1. Pass a patent-distance check before implementation begins (consult Phase 1 + 2 diffcomplete protocol).
2. Document the chosen design's distance from the nearest prior-art claim in a per-feature note inside `notes/`.
3. Avoid copying the layout/interaction language used in the competitor's documentation or marketing.

This is a deliberate choice to keep design freedom on AI-editing surfaces, not just a CYA posture — it's already shaped Phase 1 + Phase 2 of `diffcomplete` and was validated by the user as the right trade-off (memory: feedback_patent_distance_priority).

### 53.4 Out-of-scope for Phase 53

To keep the phase tractable, the following are **explicitly deferred**:

- **Sketch Canvas 3D / WebGL / three.js** (US-011 ships only the 2D wireframe → JSX path). 3D scene export is removed from the gap list as a non-goal.
- **The other 15 native connectors** in US-008 (only 5 ship with real OAuth this cycle).
- **The remaining 30+ RL algorithms** in US-015 (only PPO ships end-to-end this cycle; the rest follow once the candle/PyO3 scaffolding is proven).
- **Devin-level hours-long autonomy** (long-horizon item from §15 of the fitgap; not addressed here).
- **Cursor's proprietary Tab model** (long-horizon; we continue to use Ollama/cloud FIM).
- **Enterprise SSO / audit packaging** (long-horizon; tracked separately under MCP enterprise extensions in Phase 54).

---

## Appendix E — Phase 54: May 2026 weekly delta (B1–B6 + trivial closes)

**Input:** [§1ter](#1ter-may-2026-weekly-delta--missed-quarter-items-added-2026-05-03) of this document + [Fit-Gap §16.4](./fit-gap-analysis/#164-v14--may-2026-weekly-delta--missed-quarter-items-added-2026-05-03).
**Window:** v0.5.7 cycle (target completion: 2026-05-17).
**Output:** Six new gap closures (B1–B6) + five trivial closes already enumerated in §16.4.

### 54.1 New gap closures

**P0 (this cycle):**

- **B1 — Skills as MCP primitives** — expose each entry under `vibecli/vibecli-cli/skills/` via an MCP server's `list_skills` / `get_skill` resources; reuse `skill_loader.rs`; ship a single `vibecli-skills-mcp` binary registered with the daemon; 5 BDD scenarios. Highest-leverage item this cycle — every MCP host (Claude Code, Cursor, Cline, Zed, JetBrains, …) inherits all 711 skills with no per-host work.
- **B2 — Plugin bundle format with admin install policies** — define `vibecli-plugin.toml` manifest bundling MCP servers + skills + subagents + rules + hooks; `vibecli plugin install <path-or-url>`; `WorkspaceStore` persists per-plugin policy (`Off` / `On` / `Required`); governance panel surfaces this; 6 BDD scenarios. Patent-distance check: avoid copying Cursor's marketplace layout/UX terminology.
- **B5 — NVFP4 (Blackwell native) TurboQuant target** — add NVFP4 Metal+CUDA kernels to TurboQuant alongside existing MXFP4 + AWQ-Marlin paths; benchmark on RTX 5090 / B200 / GB200; CubeCL/Burn ban scope unchanged.
- **B6 — A2A signed agent-card façade** — serve `/.well-known/agent.json` with a P-256 ECDSA signature reused from the watch-pairing key infrastructure; register VibeCLI as an A2A server in the LF Agentic AI Foundation registry; 4 BDD scenarios covering signature verification + task lifecycle.

**P1 (next cycle, v0.5.8):**

- **B3 — Always-on agent classes (security review, vuln scan)** — convert `/review` from on-demand to a daemon-resident agent class; trigger on file-watcher / pre-commit / CI; route findings to the existing `Finding` schema; UI surface in `SecurityPanel.tsx`. **Patent-distance check required** — design must remain distant from Cursor's Security Review UX.
- **B4 — Cursor SDK parity audit** — compare `packages/agent-sdk/` to `@cursor/sdk` along: subagents, hooks, plugins, skills, sandbox tiers, recap/resume, multi-client (mobile/watch). Items where Cursor's surface is wider become roadmap entries (deferred until parity assessment is complete; this is research, not implementation).

### 54.2 Trivial closes (one-line / one-route)

- **Ollama `/v1/messages` route** — one route handler in `vibecli/vibecli-cli/src/serve.rs`; the existing Anthropic provider format already matches.
- **GPT-5.5 / GPT-5.4 model entries** — append to `useModelRegistry.ts` `STATIC_MODELS.openai`.
- **Sonnet 4.8 entry** (when Anthropic exposes it) — same one-file change.
- **Qwen 3.6 / DeepSeek V4 / Kimi K2.6 entries** — append to the Ollama section of `useModelRegistry.ts` once GGUF / vLLM weights land.
- **`GEMINI.md` fallback in `memory.rs`** — already noted in v13 as one-line; remains pending.

### 54.3 ACP Registry follow-through

A4 from Phase 53 (ACP server mode) has a concrete deliverable now that the ACP Registry is live: register VibeCLI in the registry once A4 ships so Zed + JetBrains users discover it natively. Coordinate with the JetBrains plugin update referenced in 53.2 P0.

### 54.4 Patent-distance posture (carries forward)

B2 (Plugin Marketplace) and B3 (Always-on Security Review) sit close to recently-shipped Cursor surfaces. Same protocol as 53.3 — patent-distance check before implementation begins, per-feature note in `notes/`, and avoid copying competitor layout/interaction language. The user-validated stance from the diffcomplete cycles applies (memory: `feedback_patent_distance_priority`).

### 54.5 Out-of-scope for Phase 54

- **MCP Apps generic UI host** (A1 from Phase 53 — already P0 there; B2 reuses the same React embedding work, no separate item).
- **Long-horizon items from §15** of the fitgap (Tab model, Devin autonomy, MCP catalog breadth, SWE-bench leaderboard entry, enterprise SSO/audit, BYOA adapters) — none addressed here.
- **Computer-use feedback into the agent loop beyond `desktop_agent.rs`** — A8 in Phase 53 covers it.
- **Patent-audit working doc updates** — handled per-feature in `notes/PATENT_AUDIT_INLINE.md`, not here.
