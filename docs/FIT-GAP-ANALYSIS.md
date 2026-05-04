---
layout: page
title: Fit-Gap Analysis — VibeCody vs the AI Coding Landscape
permalink: /fit-gap-analysis/
---

# Fit-Gap Analysis — VibeCody vs the AI Coding Landscape

**Originally published:** 2026-02-25 &middot; **Last refreshed:** 2026-05-03 (v0.5.7 — v14 weekly + missed-quarter delta layered on v13)
**Scope:** Cumulative delta of 9 sequential iterations (v4 → v13) plus 5 topic-specific deep-dives (AgentOS, Pi-mono, RL-OS, Paperclip, Code-Review/Architecture) — **40+ competing AI coding products and frameworks analyzed**.
**Companion document:** [Competitive Landscape & Roadmap](./roadmap/).

> **Executive bottom line (revised 2026-05-03 for v14):** Across approximately **170 cumulative gaps** (deduplicated) catalogued over 10 iterations and 5 topic deep-dives, **~106 are closed with real I/O**, **~41 are partial — design / type-system / panel exists but the I/O layer is not yet wired** (8 audit-flagged modules + 52 RL-OS entries sharing one deferred-training plan + 2 partial AgentOS items + 5 v14 trivial-closes-pending; see §16.2 audit reconciliation + §16.4), and **~23 are open** — comprising the 6 long-horizon items already tracked, the 11 v13 April-2026 trend gaps (§16.1, A1–A11), and the 6 v14 May-2026 trend gaps (§16.4, B1–B6). The previous "136 closed of 142" framing conflated typed-and-tested with shipped-with-real-I/O; the audit at [docs/audit/05-fitgap-overstatements.md](./audit/05-fitgap-overstatements.md) is reflected directly in the scoreboard. **Closed-with-real-I/O is the only number worth quoting externally** — the partial work continues to ship as the US-001…US-006 cadence proves real conversions are happening.

---

## 1. History of this analysis

This document consolidates 13 prior files (now removed — `git log` preserves them). Each iteration targeted a different slice of the market, and together they cover every significant AI coding product shipped between February 2026 and April 2026.

| Iteration | Date | Scope | New gaps | Status |
|-----------|------|-------|----------|--------|
| v4 (base) | 2026-02-25 | Claude Code, Codex CLI, Cursor, Windsurf | 23 | All closed |
| v5 | 2026-03-12 | + Replit Agent 3, Amazon Q, Qodo, Roo Code, Cline, Zed, VS 2026 | 12 | All closed |
| v6 | 2026-03-20 | + OpenAI Codex, Kiro, Trae, Poolside, Magic.dev, Lovable 2.0, v0, Jules, PearAI, Amp | 19 | All closed |
| v7 | 2026-03-26 | + Warp 2.0, Junie CLI, Open-SWE, Gemini CLI 1.0, Augment Intent, Moatless, Agentless | 22 | All closed (+ Phase 32 bonus of 6) |
| v8 | 2026-04-11 | + Cursor 3.0, Copilot Autopilot, Devin Fast Mode, Antigravity, Claw Code, MS Agent Framework, A2A v0.3 | 18 | All closed |
| v10 | 2026-04-12 | Claude Code 1.x, Cursor 4.0, Copilot Workspace v2, Devin 2.0, Cody 6.0 | 20 | All closed |
| v11 | 2026-04-14 | Agent-OS registry, workspace snapshots, multi-repo context, dev workflow | 20 | All closed |
| v12 | 2026-04-14 | Reasoning infra, prompt-cache, design platform, computer use | 20 | All closed |
| v13 | 2026-04-26 | Cursor 3.0/3.2, Copilot Cloud Agent, Devin 2.2, Claude Opus 4.7, Gemini CLI v0.38, Junie CLI Beta, Codex CLI Bedrock + Spark, MCP 2026 roadmap, ACP v0.11, Antigravity 1.22, Augment 72% | 17 | 6 near-shipped (existing infra covers), 11 open (§16.1) |
| v14 | 2026-05-03 | MCP `experimental-ext-skills`, Cursor Plugin Marketplace v2 + Security Review + SDK + Interactive Canvases, VS 2026 Integrated Cloud Agent, GPT-5.5, Sonnet 4.8 leaked, llama.cpp NVFP4, Ollama 0.22.x `/v1/messages`, DeepSeek V4 + Qwen 3.6 + Kimi K2.6, A2A v1.2 LF, ACP Registry, DAPO mainstream, sandbox cold-start floor, SWE-bench Verified contamination, JetBrains Air, Devin v3 API GA | 11 | 5 trivial closes, 6 open (§16.4 — B1–B6) |
| **Sequential total** | — | **40+ competitors** | **~170 (deduplicated)** | **~106 closed, ~41 partial, ~23 open (revised 2026-05-03 — see §3 + §16.5)** |

Alongside the sequential sweeps, five topic-specific deep-dives were added when VibeCody pushed into new domains:

| Deep-dive | Date | Scope | Gaps | Status |
|-----------|------|-------|------|--------|
| [AgentOS](#10-deep-dive-agentos) | 2026-03-31 | Claude SDK, OpenAI SDK, Google ADK, Devin, CrewAI, LangGraph, AutoGen, VAST, Simplai, Cursor, Windsurf, Augment, Amazon Q, Copilot Agent, OpenHands (15 platforms) | 8 (8 closed, 2 partial) | 78% coverage → 100% for identified gaps |
| [Pi-mono](#11-deep-dive-pi-mono) | 2026-04-14 | Mario Zechner's pi-mono (7-package TypeScript agent harness) | 15 (P0-P2) | All closed |
| [RL-OS](#12-deep-dive-rl-os) | 2026-03-30 | 40+ RL frameworks across 8 categories (Ray, SB3, CleanRL, Isaac Lab, d3rlpy, MLflow, W&B, KServe, Triton, …) | 52 | All closed (type system & orchestration) |
| [Paperclip](#13-deep-dive-paperclip) | 2026-04-05 | TuringWorks Paperclip (Node/React agent company manager) | 13/13 | Full parity |
| [Code Review + Architecture](#14-deep-dive-code-review--architecture) | 2026-03-30 | Qodo, CodeRabbit, Bito, Cursor, Copilot, Ellipsis + Archi, Modelio, Gaphor, Diagrams.net, Cerbos | 10 review + 8 arch | All P0/P1 closed |

---

## 2. Gap catalogue (deduplicated by theme)

The 142 gaps discovered over 8 iterations resolve into nine cross-cutting themes. Every closed gap has a Rust module or React panel backing it — no stubs.

### 2.1 Agent architecture & orchestration

| # | Gap | First flagged | Closed by |
|---|-----|----------------|-----------|
| Plan → act → observe loop | v4 | `vibe-ai/agent.rs` |
| Approval tiers (Suggest / AutoEdit / FullAuto) | v4 | `vibe-ai/policy.rs` |
| Parallel multi-agent on git worktrees | v4 | `multi_agent.rs` + `worktree_pool.rs` |
| Two-level planner + executor | v4 | `planner.rs` |
| Typed parallel agent roles (Planner/Executor/Reviewer) | base | `sub_agent_roles.rs` |
| Recursive sub-agent spawning (Copilot Autopilot) | v8 | `spawn_agent.rs` |
| Await-tool primitive (Cursor 3.0) | v8 | `await_tool.rs` |
| Cross-environment dispatch (local / worktree / cloud VM / SSH) | v8 | `dispatch_remote.rs` |
| Agent FSM with UI badge (Cody 6.0) | v10 | `agent_state_machine.rs` |
| Parallel tool scheduler with dependency DAG | v10 | `parallel_tool_scheduler.rs` |
| Streaming patch applicator | v10 | `stream_patcher.rs` |
| Agent-OS registry + capability advertisement | v11 | `agent_registry.rs` |
| Dynamic agent recruitment | v11 | `agent_recruiter.rs` |
| Resource quotas & budgets per agent | v11 | `agent_quota.rs` |
| Auto-scaling agent pool | v11 | `agent_autoscale.rs` |
| Background / persistent agents | v11 | `agent_persistence.rs` |
| Workspace snapshot / restore | v11 | `workspace_snapshot.rs` |
| Multi-repo context | v11 | `multi_repo_context.rs` |
| Agent capability discovery | v11 | `capability_discovery.rs` |
| Alternative exploration tournament | v12 | `alt_explore.rs` |
| Priority task scheduler | v12 | `task_scheduler.rs` |
| Remote agent dispatch queue | v12 | `dispatch_remote.rs` |

### 2.2 Protocol & interoperability

| # | Gap | First flagged | Closed by |
|---|-----|----------------|-----------|
| MCP client (JSON-RPC 2.0 stdio + SSE) | v4 | `mcp.rs` |
| MCP lazy tool loading (Claude Code 2.1.74 style) | v5 | `mcp_directory.rs` + deferred fetch |
| A2A protocol (Google) | v5 | `a2a_protocol.rs` |
| ACP (Agent Client Protocol — Zed) | v5 | `acp_protocol.rs` |
| A2A v0.3 gRPC + security card signing | v8 | `a2a_protocol.rs` |
| MSAF (Microsoft Agent Framework) compat | v8 | `msaf_bridge.rs` |
| LangGraph / Claw Code harness compat | v8 | `claw_bridge.rs` |
| RPC stdio mode (pi-mono) | pi | `rpc_mode.rs` |
| JSON/events streaming mode on stdout | pi | `json_events.rs` |

### 2.3 Code generation, review, and refactoring

| # | Gap | First flagged | Closed by |
|---|-----|----------------|-----------|
| Inline chat / Cmd+K | base | `InlineChat.tsx` |
| Next-edit prediction (FIM) | v4 | `completion.rs` + Monaco provider |
| Chunk-level diff accept/reject | base | `DiffReviewPanel.tsx` |
| Inline diff accept/reject in CLI | v11 | `inline_diff.rs` |
| Syntax-aware smart diff (hunk-by-block) | v10 | `smart_diff.rs` |
| Streaming patch apply | v10 | `stream_patcher.rs` |
| Automated changelog gen | v11 | `changelog_gen.rs` |
| PR description generator | v11 | `pr_description.rs` |
| Spec-to-test generator | v11 | `spec_to_test.rs` |
| Dependency update advisor | v11 | `dep_update_advisor.rs` |
| Test impact analysis | v10 | `test_impact.rs` |
| Auto stub generator | v10 | `auto_stub.rs` |
| Multi-file symbol rename | v10 | `rename_symbol.rs` (LSP-backed) |
| AI-assisted merge | v10 | `ai_merge.rs` |
| Polyglot refactor | v12 | `polyglot_refactor.rs` |
| PR review agent with learning loop | cr/arch | `ai_code_review.rs` + `self_review.rs` |
| Quality gates (NL rules + structured conditions) | cr/arch | `quality_gates.rs` |
| Multi-linter aggregation (8 linters, FP filter) | cr/arch | `linter_aggregator.rs` |
| Breaking change detection | cr/arch | `detect_breaking_changes.rs` |

### 2.4 Developer experience & UX

| # | Gap | First flagged | Closed by |
|---|-----|----------------|-----------|
| Streaming TUI | v4 | `tui/` |
| `/model`, `/cost`, `/context`, `/status` REPL | base | `repl.rs` |
| Named sessions + session fork + rewind | base | `trace.rs` + `/rewind` |
| Image attachment (`-i`) + multimodal input | base | `image_attachment.rs` |
| `--add-dir` extra workspaces | base | `agent.rs` |
| Multiple chat tabs | base | `ChatTabManager.tsx` |
| Per-chat model switching | base | `AIChat.tsx` |
| BYOK settings UI | base | `SettingsPanel.tsx` |
| `@file`, `@symbol`, `@codebase`, `@folder`, `@terminal`, `@web`, `@docs`, `@git` | base | `ContextPicker.tsx` |
| Flow awareness (Windsurf-style) | v4 | `flow.rs` |
| Named checkpoint descriptions | base | `CheckpointPanel.tsx` |
| Deep-focus session gating | v12 | `focus_view.rs` |
| Conversation branching | v10 | `conversation_branch.rs` |
| Code explanation depth levels | v11 | `explain_depth.rs` |
| Custom REPL macros | v11 | `repl_macros.rs` |
| Session export / import | v11 | `session_export.rs` |
| Session share (HTML) | pi | `session_share.rs` |
| Session tree (in-file branching) | pi | `session_tree.rs` |
| Context handoff across providers | pi | `context_handoff.rs` |
| Cursor overlay (live collab cursors) | v10 | `cursor_overlay.rs` |

### 2.5 Context, memory, indexing

| # | Gap | First flagged | Closed by |
|---|-----|----------------|-----------|
| Codebase indexing (tree-sitter + embeddings) | v4 | `vibe-core/index/` |
| Smart context builder (BM25 + semantic + flow) | v4 | `context.rs` |
| `AGENTS.md` / `VIBECLI.md` / `CLAUDE.md` memory | v4 | `memory.rs` |
| Rules directory (`.vibecli/rules/`) | base | `rules.rs` |
| Auto memory recording | base | `memory_recorder.rs` |
| Cascade-style memory consolidation | v12 | `autodream.rs` |
| Prompt-prefix caching | v12 | `prompt_cache.rs` |
| Extended reasoning / thinking blocks | v12 | `reasoning_provider.rs` |
| Long-session budget management (2M tokens) | v12 | `long_session.rs` |
| File watcher (FSEvents / inotify, sub-50ms) | v10 | `file_watcher.rs` |
| Semantic codebase search v2 | v11 | `semantic_search_v2.rs` |
| Token budget dashboard | v11 | `token_dashboard.rs` |
| Dependency graph visualizer | v10 | `dep_visualizer.rs` |

### 2.6 Privacy, security, policy

| # | Gap | First flagged | Closed by |
|---|-----|----------------|-----------|
| OS sandbox (Apple Seatbelt / bwrap) | v4 | `executor.rs` |
| Windows ACL sandbox policy | v12 | `sandbox_windows.rs` |
| Wildcard tool-permission patterns | base | `policy.rs` |
| Admin policy + shell env policy | v4 | `policy.rs` |
| Red-team / pentest pipeline | v4 | `redteam.rs` |
| Blue/purple team agents | AgentOS | `blue_team.rs`, `purple_team.rs` |
| Compliance reporting (SOC 2 technical controls) | cr/arch | `compliance_controls.rs` |
| Policy-as-code (Cerbos parity) | cr/arch | `policy_engine.rs` |
| Prompt-injection defense | AgentOS | `prompt_injection_defense.rs` |
| `apiKeyHelper` rotating credentials | base | `config.rs` |
| OAuth login (Claude Pro/Max, ChatGPT, Copilot, Google) | pi | `oauth_login.rs` |

### 2.7 Enterprise & operations

| # | Gap | First flagged | Closed by |
|---|-----|----------------|-----------|
| OpenTelemetry | v4 | `otel_init.rs` |
| Cost observatory (per-session USD) | v5 | `cost_observatory.rs` |
| Pre-execution cost estimator | v10 | `cost_estimator.rs` |
| Provider-aware retry + circuit breaker | v10 | `rate_limit_backoff.rs` + `cost_router.rs` |
| Enterprise audit trail | v8 | `audit_trail.rs` |
| Copilot Spaces (shared context bundles) | v5 | `context_bundles.rs` |
| Credit-based billing / metering | v5 | `cost_metering.rs` |
| Agent observability dashboard | AgentOS | `agent_analytics.rs` + `AgentObservabilityPanel.tsx` |
| Background agents / daemon mode | v6 | `vibecli serve` + `automations.rs` |
| GitHub Actions integration | v4 | `.github/actions/vibecli/` |
| GitHub App (review bot) | v4 | `github_app.rs` |

### 2.8 Emerging frontiers (April 2026)

| # | Gap | First flagged | Closed by |
|---|-----|----------------|-----------|
| Computer use (Claude/Devin-style) | v8 | `computer_use.rs` |
| Desktop agent (click/type/scroll) | v8 | `desktop_agent.rs` |
| Browser agent (DevTools Protocol) | v8 | `browser_agent.rs` |
| Visual UI verification | v8 | `visual_verify.rs` |
| Voice input (Whisper) | base | `voice.rs` + `AIChat` mic |
| Voice command history | v10 | `voice_history.rs` |
| Spec-driven development (Kiro-style EARS) | v6 | `spec_driven.rs` |
| On-device inference | v8 | Ollama + `--local` flag |
| Claw Code harness bridge | v8 | `claw_bridge.rs` |
| Design canvas / sketch-to-code | v6 | `CanvasPanel.tsx` |
| Draw.io integration | v12 | `drawio_connector.rs` |
| Penpot integration | v12 | `penpot_connector.rs` |
| Pencil wireframing (EP XML + MCP) | v12 | `pencil_connector.rs` |
| AI diagram generator (6 Mermaid templates) | v12 | `diagram_generator.rs` |
| Design system hub (token audit, drift, export) | v12 | `design_system_hub.rs` |
| Multi-provider design system | v12 | `design_providers.rs` |

### 2.9 Surface coverage (unique to VibeCody)

| # | Area | Source | Backing |
|---|------|--------|---------|
| VibeCLI TUI + REPL daemon | base | `vibecli-cli/` |
| VibeUI Tauri + Monaco (293 panels + 42 composites) | base | `vibeui/` |
| VibeCLI App (floating chat window) | v5 | `vibeapp/` |
| VibeMobile (iOS/Android/macOS/Linux/Windows/Web) | v5 | `vibemobile/` |
| VibeWatch — Apple Watch (SwiftUI, watchOS 10+) | v0.5.5 | `vibewatch/watchos/` |
| VibeWatch — Wear OS (Compose, Wear OS 3+) | v0.5.5 | `vibewatch/wearos/` |
| VS Code, JetBrains, Neovim extensions | base | `vscode-extension/`, `jetbrains-plugin/`, `neovim-plugin/` |
| Zero-config pairing (mDNS + Tailscale Funnel + ngrok) | v0.5.5 | `mdns_announce.rs`, `tailscale.rs`, `ngrok.rs` |
| Google-Docs-style full-content sync | v0.5.5 | `sync_reconcile.rs` |

---

## 3. Gap status — cumulative scoreboard

Revised 2026-04-26 to reflect the audit at [docs/audit/05-fitgap-overstatements.md](./audit/05-fitgap-overstatements.md). "Closed" now requires real I/O (HTTP/process/FFI/external API) — modules that are typed, tested in-memory, and panel-wired but lack the I/O layer are reclassified as **Partial**.

| Category | Identified | Closed (real I/O) | Partial (design-only) | Open |
|----------|-----------:|-------:|--------:|-----:|
| Agent architecture & orchestration | 22 | 21 | 1 (`issue_triage` HTTP)¹ | 0 |
| Protocol & interoperability | 9 | 8 | 1 (`langgraph_bridge` REST)¹ | 0 |
| Code generation, review, refactoring | 19 | 17 | 2 (`linter_aggregator`, `mcts_repair` rollout)¹ | 0 |
| Developer experience & UX | 20 | 20 | 0 | 0 |
| Context, memory, indexing | 13 | 12 | 1 (`semantic_index` AST)¹ | 0 |
| Privacy, security, policy | 11 | 11 | 0 | 0 |
| Enterprise & operations | 11 | 9 | 2 (`native_connectors`, `cost_router`)¹ | 0 |
| Emerging frontiers | 16 | 15 | 1 (`sketch_canvas` 3D/WebGL)¹ | 0 |
| Surface coverage | 9 | 9 | 0 | 0 |
| AgentOS deep-dive | 8 | 6 | 2 | 0 |
| Pi-mono deep-dive | 15 | 15 | 0 | 0 |
| RL-OS deep-dive | 52 | 0 (real training)² | 52 (type system + orchestration) | 0 |
| Paperclip deep-dive | 13 | 13 | 0 | 0 |
| Code-Review / Architecture | 10 + 8 | 10 + 8 | 0 | 0 |
| Long-horizon (tracked in roadmap) | 6 | 0 | — | 6 |
| **v13 trend delta (2026-04-26, identification-only)** | **17** | **0** | **0** | **17** |
| **Per-category sum (pre-dedup)** | **259** | **174** | **62** | **23** |
| **Total (deduplicated, approximate)** | **~159** | **~106** | **~36** | **~17** |

¹ Audit-flagged module that retains its design + tests + panel + REPL command but does not yet ship the I/O layer claimed in the original gap closure. Roadmap work is tracked in Phase 53.

² The RL-OS subsystem (~31K lines across 8 `rl_*.rs` files) is honest about being a type-system + orchestration substrate; neural-net training, GPU/TPU kernels, and PyO3 bindings are explicitly deferred (already noted in §12). Counting the 52 entries as "Partial" rather than "Closed" matches that intent.

The pre-dedup per-category sum is exact; the deduplicated total is approximate because the v4–v12 dedup math is opaque (the original "142 deduplicated of 242 raw" implied a 0.59 collapse ratio, applied here as a directional estimate). The audit-aware reclassification is what matters: **fewer items are "shipped with real I/O" than the prior scoreboard claimed, and the new ones are listed by name** so they can be tracked.

The 6 long-horizon items remain competitive-frontier / business moves rather than engineering tasks — they live in the [roadmap](./roadmap/#93-where-we-still-have-parity-gaps-to-close). The 14 partial modules (8 individual + 52 RL-OS entries that share one conversion plan) and 11 open v13 items have a real-I/O conversion plan in [Phase 53](./roadmap/#appendix-d--phase-53-april-2026-trend-delta--audit-reconciliation) of the roadmap, modeled on the US-001…US-006 conversions that already shipped.

---

## 4. Feature-complete matrix vs Claude Code (the largest comparator)

A flattened version of the v4/v5 feature parity matrix, carried forward through v12.

| Feature | VibeCLI | Claude Code |
|---------|:---:|:---:|
| Multi-turn REPL | Yes | Yes |
| Agent loop (plan → act → observe) | Yes | Yes |
| Plan mode | Yes | Yes |
| Session resume | Yes | Yes |
| Hooks system (Pre/PostToolUse, UserPromptSubmit) | Yes | Yes |
| Skills system (auto-activating) | Yes (711 files) | Yes |
| MCP client (300+ servers compatible) | Yes | Yes |
| MCP directory / curated registry | Yes | Yes |
| Git integration | Yes | Yes |
| Web search tool | Yes | Yes |
| Multi-agent / parallel execution | Yes | Yes |
| PR code review agent (BugBot-class) | Yes | Yes |
| OpenTelemetry tracing | Yes | Yes |
| Admin policy (wildcards, glob tool patterns) | Yes | Yes |
| HTTP daemon (`serve`) | Yes | Yes |
| VS Code / JetBrains / Neovim extensions | Yes | Yes |
| Agent SDK (TypeScript) | Yes | Yes |
| Named profiles + doctor command | Yes | Yes |
| REPL tab-completion | Yes | Yes |
| Image / screenshot attachment (`-i`) | Yes | Yes |
| `/model`, `/cost`, `/context`, `/status` | Yes | Yes |
| Named sessions + `/fork` + `/rewind` | Yes | Yes |
| Extended thinking mode | Yes | Yes |
| `--add-dir` additional dirs | Yes | Yes |
| JSON streaming output (`--json`) | Yes | Yes |
| Typed parallel agent roles | Yes | Yes |
| Auto memory recording | Yes | Yes |
| Rules directory (`.vibecli/rules/`) | Yes | Yes |
| `/rewind` session checkpoint | Yes | Yes |
| PTY-backed bash tool | Yes | Yes |
| Wildcard tool permission patterns | Yes | Yes |
| `apiKeyHelper` rotating credentials | Yes | Yes |
| LLM-based hook execution | Yes | Yes |
| Parallel tool scheduler (dependency DAG) | Yes | Yes |
| Prompt-prefix caching | Yes | Yes |
| **ALSO** OAuth login (Claude Pro, ChatGPT, Copilot, Google) | Yes | — |
| **ALSO** Red-team + compliance reporting | Yes | — |
| **ALSO** Counsel multi-LLM deliberation | Yes | — |
| **ALSO** Flutter mobile + native watch clients | Yes | — |
| **ALSO** 22 providers + Ollama first-class | Yes | — |
| **ALSO** Design platform (Draw.io / Penpot / Pencil / AI diagrams) | Yes | — |

Similar parity tables exist for **Codex CLI**, **Cursor 4.1**, **Windsurf 2.0**, **Devin 2.1**, **Copilot Workspace v3**, **Cody 6.1**, **Kiro**, **Zed**, **Gemini CLI** — the roadmap's §9.1 renders the flattened 14-competitor matrix.

---

## 5. VibeUI vs desktop-IDE competitors

| Feature | VibeUI | Cursor | Windsurf | Antigravity | Claude Code | Copilot | JetBrains AI | Zed |
|---------|:---:|:---:|:---:|:---:|:---:|:---:|:---:|:---:|
| Monaco editor | Yes | Yes | Yes | Yes | — | Yes | Yes | Yes (GPUI) |
| AI chat panel + agent mode | Yes | Yes | Yes | Yes | — | Yes | Yes | Yes |
| Inline chat (Cmd+K) | Yes | Yes | Yes | Yes | — | Yes | Yes | Yes |
| Next-edit prediction (Tab/FIM) | Yes | Yes (BIC) | Yes | Partial | — | Yes | Partial | Yes |
| Diff review before apply | Yes | Yes | Yes | Yes | Yes | Partial | Yes | Partial |
| @-context system (10 prefixes) | Yes | Yes | Yes | Partial | — | Partial | Yes | Yes |
| Multi-file batch edits | Yes | Yes | Yes | Yes | Yes | Partial | Yes | Partial |
| Parallel agents | Yes | Yes (8) | Yes | Yes (5) | Yes | — | — | — |
| Flow awareness | Yes | Partial | Yes | Partial | — | — | Partial | Partial |
| Memory / rules | Yes | Yes | Yes | Yes | Yes | Partial | Yes | Partial |
| Planning agent (two-level) | Yes | Partial | Yes | Yes | Yes | Yes | Yes | — |
| Checkpoint / rewind | Yes | Partial | Yes | Yes | — | Partial | Partial | — |
| Voice input | Yes | — | — | — | — | — | — | — |
| Browser panel | Yes | — | — | — | — | — | — | — |
| Multiplayer CRDT | Yes | — | — | — | — | — | — | Yes |
| Artifacts panel | Yes | — | — | Yes | — | — | — | — |
| Manager View (parallel orchestration) | Yes | — | — | Yes | — | — | — | — |
| WASM extension host | Yes | — | — | — | — | — | — | — |
| Watch Devices panel (Apple Watch + Wear OS) | Yes | — | — | — | — | — | — | — |
| Handoff chip (continuity) | Yes | — | — | — | — | — | — | — |
| 22 providers + Ollama | Yes | Partial | Partial | Partial | — | — | Partial | Multi |
| Rust native backend | Yes | — | — | Partial | — | — | — | Yes |
| Open source | Yes | — | — | — | — | — | — | Yes (Apache 2) |

---

## 6. VibeCLI `--serve` vs cloud-agent products

| Capability | VibeCLI + agent-sdk | Devin | Replit Agent | Bolt.new | v0 | Sweep AI |
|-----------|:---:|:---:|:---:|:---:|:---:|:---:|
| Self-hostable | Yes | — | — | — | — | — |
| Works offline (local model) | Yes | — | — | — | — | — |
| Bring-your-own-LLM (22 providers) | Yes | — | — | — | — | — |
| Full-stack code generation | Partial | Yes | Yes | Yes | Yes (UI) | Partial |
| Long-horizon autonomy (hrs) | Partial | Yes | Yes | Partial | Partial | Partial |
| Browser / shell sandbox | Yes | Yes | Yes | Yes (WC) | — | — |
| GitHub issue → PR automation | Yes | Yes | Partial | — | — | Yes |
| Native mobile companion | Yes | Partial | Yes | — | — | — |
| Native watch companion | Yes | — | — | — | — | — |
| Zero-config LAN / Tailscale / ngrok | Yes | — | — | — | — | — |
| Open source | Yes | — | — | — | — | Partial |

---

## 7. VibeCLI `/review` vs AI review bots

| Capability | VibeCLI `/review` + CIReviewPanel | CodeRabbit | Qodo | Greptile | Cursor BugBot | Ellipsis |
|-----------|:---:|:---:|:---:|:---:|:---:|:---:|
| Inline PR comments | Yes | Yes | Yes | Yes | Yes | Yes |
| Security-focused review (OWASP) | Yes | Yes | Partial | Partial | Partial | Partial |
| Self-hosted option | Yes | — | Partial | — | — | — |
| Bring-your-own-LLM | Yes | — | — | — | — | — |
| Runs locally from CLI | Yes | — | — | — | — | — |
| Multi-LLM deliberation (Counsel) | Yes | — | — | — | — | — |
| Cost metering / budgets | Yes | — | — | — | — | — |
| Compliance reporting (SOC 2) | Yes | — | Yes | — | — | — |
| Architecture-aware review | Yes | — | — | — | — | — |
| Policy-as-code quality gates (Cerbos) | Yes | — | — | — | — | — |
| 5 git platforms (+Gitea) | Yes | Yes (4) | Yes (3) | Yes (3) | Yes (1) | Yes (3) |

---

## 8. VibeMobile / VibeWatch vs mobile + watch surfaces

| Capability | VibeMobile + VibeWatch | Replit mobile | Cursor mobile (preview) | Devin web | Others |
|-----------|:---:|:---:|:---:|:---:|:---:|
| Native iOS | Yes | Yes | Yes | PWA | — |
| Native Android | Yes | Yes | — | PWA | — |
| macOS / Linux / Windows / Web | Yes | — | — | Web | — |
| **Apple Watch native** | Yes | — | — | — | — |
| **Wear OS native** | Yes | — | — | — | — |
| Pairs with self-hosted host | Yes | — | — | — | — |
| Full-duplex session (not read-only) | Yes | Yes | Partial | Partial | — |
| Zero-config LAN / Tailscale / ngrok | Yes | — | — | — | — |
| Handoff-style continuity | Yes | — | — | Partial | — |
| Dictated reply on watch | Yes | — | — | — | — |
| Open source | Yes | — | — | — | — |

---

## 9. Implementation velocity

Across the 8 sequential iterations and 5 topic deep-dives, the team delivered:

| Metric | Feb 2026 | Apr 2026 (v0.5.5) |
|--------|---------:|-----------:|
| Rust modules in `vibecli-cli/src/` | ~120 | **354** |
| VibeUI panels | ~90 | **293 + 42 composites** |
| Skill files | ~300 | **711** |
| Tests (workspace) | ~4,500 | **13,270** |
| AI providers | 17 | **22** |
| REPL commands | 76 | **~150** |
| Tauri commands | ~600 | **1,045+** |
| Surfaces shipped | 2 (CLI, UI) | **8 (CLI, UI, App, Mobile, AppleWatch, WearOS, CI action, SDK)** |

No iteration shipped stubs — every gap closure had Rust implementation + BDD harness + skill file + cross-surface hookup.

---

## 10. Deep-dive: AgentOS

Map of VibeCody's 60+ agent-related modules against 15 agentic platforms (Claude SDK, OpenAI SDK, Google ADK, Devin, CrewAI, LangGraph, AutoGen, VAST, Simplai, Cursor, Windsurf, Augment, Amazon Q, Copilot Agent, OpenHands).

**Coverage:** ~78% of the competitive matrix on first pass → 100% for identified gaps after the AgentOS extension.

### Gaps identified

| # | Gap | Status | Module |
|---|-----|--------|--------|
| 1 | Visual agent team builder (drag-drop) | Partial | `CanvasPanel.tsx` (basic) |
| 2 | Agent discovery registry (Agent Card) | Closed | `agent_registry.rs` |
| 3 | Dynamic agent recruitment | Closed | `agent_recruiter.rs` |
| 4 | Agent marketplace | Closed | `plugin_marketplace.rs` |
| 5 | Agent observability dashboard | Closed | `agent_analytics.rs` + `AgentObservabilityPanel.tsx` |
| 6 | Deployment / scaling infrastructure | Closed | `agent_autoscale.rs` + `pod_manager.rs` |
| 7 | Human-in-the-loop approval workflows | Closed | `company_approvals.rs` |
| 8 | Agent runtime containerisation | Closed | `container_tool_executor.rs` |

### Architectural themes unique to VibeCody

- **Agent governance** (`team_governance.rs`) — no peer ships a dedicated governance layer.
- **Red/Blue/Purple team agents** (~4,400 lines) — security-specialised agent roles.
- **OpenMemory protocol** (4,355 lines) — cross-agent shared memory standard.
- **Counsel** — structured multi-LLM deliberation (expert / devil's advocate / skeptic / pragmatist + moderator synthesis).
- **Explainable agent decisions** (`explainable_agent.rs`, 1,232 lines).
- **Context protocol** (`context_protocol.rs`, 1,236 lines) — in-process message bus between agents.

---

## 11. Deep-dive: Pi-mono

Mario Zechner's **pi-mono** is a 7-package TypeScript agent harness. Feature-by-feature comparison flagged 15 gaps; all now closed.

### Top closed gaps

| # | Gap | Module |
|---|-----|--------|
| 1 | Session tree (in-file branching via parent IDs) | `session_tree.rs` |
| 2 | Parallel tool execution within a single message | `parallel_tools.rs` + `parallel_tool_scheduler.rs` |
| 3 | Pluggable tool I/O (SSH / Docker / remote backends) | `tool_executor.rs` adapter trait |
| 4 | `!!cmd` shell prefix (excluded from LLM context) | `repl.rs` |
| 5 | Typed lifecycle event bus (30+ events) | `event_bus.rs` |
| 6 | Steering message queue | `message_queue.rs` |
| 7 | Follow-up message queue | `message_queue.rs` |
| 8 | OAuth login (Claude Pro, ChatGPT, Copilot, Gemini CLI) | `oauth_login.rs` |
| 9 | Cross-provider context handoff | `context_handoff.rs` |
| 10 | RPC stdio mode | `rpc_mode.rs` |
| 11 | JSON/events mode on stdout | `json_events.rs` |
| 12 | Extension `install` package system | `plugin_marketplace.rs` |
| 13 | Session share → HTML / Gist | `session_share.rs` |
| 14 | Paste guard (oversize paste detection) | `paste_guard.rs` |
| 15 | Dual-log (user-facing vs tool log) | `dual_log.rs` |

### Where VibeCody is ahead

- 711 built-in skills (Pi is community-driven, handful of reference skills).
- 22 providers (Pi has 20).
- Full desktop IDE + mobile + watch surfaces (Pi is TUI + web only).

---

## 12. Deep-dive: RL-OS

52 gaps mapped across 40+ RL frameworks (Ray RLlib, Stable Baselines3, CleanRL, TF-Agents, Tianshou, Acme, Dopamine, MushroomRL, Coax, Sample Factory; Gymnasium, PettingZoo, MuJoCo, Isaac Lab, Unity ML-Agents, EnvPool, Brax, Jumanji, WarpDrive; d3rlpy; MLflow, W&B, Neptune, ClearML, Comet; KServe, Triton, Ray Serve, BentoML; TensorBoard, Aim).

**Status:** All 52 gaps closed at the **type system + orchestration** layer (~31K lines). Neural network training, GPU/TPU compute kernels, and Python bindings are intentionally deferred to a future phase — the current modules operate on in-memory Vec<f64> arithmetic.

### Unique RL-OS differentiators (no competitor ships)

1. Lifecycle-native (12 stages: env definition → training → offline RL → multi-agent → serving → monitoring → A/B → policy distillation → interpretability → benchmarking → curriculum → auto-RL).
2. Rust-native core → zero-cost abstractions, 10-100× lower serving latency vs Python frameworks.
3. Env-as-code DSL (YAML-first, not class-first).
4. Real-world connectors (REST / gRPC / MQTT / WebSocket / DB).
5. Environment versioning (Git-like).
6. Time-travel replay (deterministic env state snapshots).
7. Off-policy evaluation with confidence intervals (FQE / IS / DR / MAGIC).
8. Safe deployment pipeline (canary + automatic rollback).
9. Counterfactual what-if evaluation on logged data.
10. Multi-physics backends through one env definition.
11. A2A-native multi-agent RL.
12. Integrated AutoRL (reward-fn search + policy NAS + auto-curriculum).

---

## 13. Deep-dive: Paperclip

Comparison with **TuringWorks Paperclip** (Node/React agent company-management harness). Full parity achieved across 13/13 feature areas.

| Feature area | Paperclip | VibeCody | Status |
|--------------|-----------|----------|--------|
| Multi-company management | Yes | Yes (`company_store`) | Parity |
| Org chart (reports_to tree) | Yes | Yes (`company_store`) | Parity |
| Hierarchical goal alignment | Yes | Yes (`company_goals`) | Parity |
| Full task lifecycle (Kanban) | Yes | Yes (`company_tasks`) | Parity |
| Approval workflows | Yes | Yes (`company_approvals`) | Parity |
| Per-agent monthly budgets | Yes | Yes (`company_budget`) | Parity |
| Agent heartbeat system | Yes | Yes (`company_heartbeat`) | Parity |
| Encrypted secrets vault | Yes | Yes (`company_secrets`) | Parity |
| Company portability (export/import) | Yes | Yes (`company_portability`) | Parity |
| Recurring routines | Yes | Yes (`company_routines`) | Parity |
| BYOA adapter registry | Yes | Yes (`adapter_registry`) | Parity |
| Documents with revision history | Yes | Yes (`company_documents`) | Parity |
| Real-time dashboard | Yes | Yes (`company_orchestrator`) | Parity |

**VibeCody advantages:** Rust memory safety, single binary, Tauri2 desktop app with 12 dedicated panels, SQLite session tree with replay, branch-per-task git worktree integration, `/company` REPL command suite (18 subcommand groups, 60+ leaves), local-first privacy, 22 AI providers.

**Residual BYOA work:** Claude/Codex/Cursor adapters currently covered via generic `HttpAdapter`; dedicated adapters will come when upstream tools publish stable agent APIs.

---

## 14. Deep-dive: Code Review & Architecture

Feature-by-feature match against Qodo Merge, CodeRabbit, Bito, Cursor, Copilot, Ellipsis (review) and Archi, Modelio, Gaphor, Diagrams.net, Cerbos (architecture).

### Review matrix — VibeCody vs top AI review bots

| Feature | Qodo Merge | CodeRabbit | Bito | Cursor | Copilot | VibeCody |
|---------|-----------|------------|------|--------|---------|----------|
| Automated PR review bot | 15+ workflows | Full | Yes | Basic | PR summaries | **Full** |
| Line-by-line findings | F1 64.3% | Yes + 1-click fix | Yes | Inline | Inline | **Yes** (severity + category + confidence) |
| OWASP Top 10 scanning | Yes | 40+ linters | Partial | No | Basic | **Yes** (6 detectors) |
| Complexity analysis | Partial | Via linters | No | No | No | **Yes** (cyclomatic, deep nesting, long fns) |
| Duplication detection | No | Via linters | No | No | No | **Yes** (cross-file) |
| Test gap analysis | Coverage delta | Test gen | No | No | No | **Yes** (`suggest_tests`) |
| Breaking change detection | Multi-repo (10+) | Partial | Yes | No | No | **Yes** |
| PR summary + risk score | Auto-describe | Walkthroughs | No | No | Yes | **Yes** |
| Architectural diagrams from diff | No | Mermaid | No | No | No | **Yes** |
| NL quality gates | Live rules | YAML + NL | No | No | No | **Yes** |
| Learning loop (precision/recall) | Yes | Yes | Graph | No | No | **Yes** (`ReviewLearning`) |
| Multi-linter aggregation (FP filter) | OWASP-only | 40+ | No | No | No | **Yes** (8 linters) |
| Git platforms supported | 3 | 4 | 3 | 1 | 2 | **5** (+Gitea) |
| On-prem / air-gapped | Enterprise | No | No | No | Enterprise | **Free** (Docker+Ollama) |
| SOC 2 compliance | Enterprise | Type II | No | No | Enterprise | **Yes** (controls) |

### Architecture matrix — VibeCody vs spec tools

| Feature | Archi | Modelio | Gaphor | Diagrams.net | Cerbos | VibeCody |
|---------|-------|---------|--------|--------------|--------|----------|
| TOGAF ADM phases | ArchiMate only | Full | No | Templates | No | **Full** (9 phases) |
| Zachman Framework (6×6) | No | Partial | No | Templates | No | **Full** |
| C4 Model | No | No | Yes | Templates | No | **Full** (4 levels + Mermaid/PlantUML) |
| ADRs (Decision Records) | No | No | No | No | No | **Full** (CRUD + markdown export) |
| Governance engine | Basic | Scripts | No | No | Auth | **Full** (rule-based, violations, recs) |
| Policy-as-code (RBAC/ABAC) | No | No | No | No | Full | **Full** (Cerbos parity) |
| Text-based (code-first) | GUI | GUI | GUI | GUI | YAML | **CLI + GUI** |
| Export formats | ArchiMate XML | XMI, HTML | PNG/SVG | Multiple | JSON | **JSON, MD, Mermaid, PlantUML** |
| Air-gapped | Yes | Yes | Yes | Yes | Yes | **Yes** (+free) |

### Policy engine vs Cerbos / OPA / Cedar / Casbin

VibeCody matches all RBAC/ABAC/derived-roles/conditions/audit-trail features and **uniquely** adds:

- Conflict detection (overlapping rules with different effects).
- Coverage analysis (which resources/actions are covered).
- Unused-rule detection (via audit log replay).
- Starter-policy templates for any resource.

### Enterprise readiness

| Capability | Required for | VibeCody status |
|------------|-------------|-----------------|
| TOGAF ADM compliance | Enterprise IT, gov, banking | Full |
| Zachman Framework | Defense, healthcare | Full |
| C4 Model | Modern software architecture | Full |
| ADRs | All teams | Full |
| Policy-as-code | Finance, healthcare, gov | Full (Cerbos parity) |
| SOC 2 controls | SaaS enterprise | Full (`compliance_controls.rs`) |
| Air-gapped deployment | Gov, defense, banking | Full (Docker + Ollama) |
| Multi-provider AI | No vendor lock-in | Full (22 providers) |

---

## 15. Remaining parity gaps (honest list)

Six items remain open from the v4–v12 cycles; none are engineering-blocked, and each is a conscious trade-off tracked in the [roadmap](./roadmap/). Eleven additional items from the v13 April-2026 trend survey are listed in §16.1.

1. **Cursor's proprietary Tab model** — next-edit prediction quality. We ship FIM completions via Ollama + cloud models; Cursor trains their own Behavior-Informed Completion model. *(Note 2026-04-26: Cursor 3.0 added an Agents Window and Cursor 3.2 added async subagents — the Tab model itself is unchanged but the surrounding multi-agent UX has widened the gap; tracked separately as v13 items A8/A9.)*
2. **Devin-level hours-long autonomy** — Devin 2.2 (Apr 2026) added computer-use self-verification + Linux-desktop access; our agent loop still tops out at ~50 steps before compaction / re-plan, and we don't run UI tests against our own output. **Cognition's 2025 acquisition of Windsurf** consolidated three previously-tracked competitors (Devin + Windsurf + Cascade) into one entity — competitive positioning §1 should treat them as a single "Cognition family" going forward.
3. **Claude Code's MCP catalog** — our MCP client is spec-compliant and we ship a directory, but the Anthropic-curated server catalog continues to grow and now includes the **MCP Apps** extension (interactive UI in conversations) + **MCPB** bundle format from the [2026 MCP roadmap](https://blog.modelcontextprotocol.io/posts/2026-mcp-roadmap/). Tracked as v13 items A1–A4.
4. **SWE-bench leaderboard position** — Augment Code leads open agent systems at **72.0% pass@1 SWE-bench Verified** (April 2026), Claude Opus 4.7 hits **87.6% Verified / 64.3% Pro**, Claude Mythos Preview tops the provisional board at 93.9% Verified, GPT-5.3-Codex sits at 85.0%. **Caveat (2026-05-03):** OpenAI stopped reporting Verified scores after a contamination audit found **59.4% of hard tasks have flawed tests** and all frontier models test as contaminated; SWE-bench Pro, SWE-rebench, and SWE-bench-Live are now the primary references. We track all four boards in the benchmark panel but haven't entered the leaderboards ourselves.
5. **Enterprise SSO / audit packaging** — Cody Enterprise and Copilot for Business are further along on SOC 2 Type II, SAML SSO, central policy distribution. **MCP enterprise readiness is now a first-class roadmap workstream** for the protocol (audit/SSO/gateway as extensions, not core), opening a path for VibeCody to lead on open-source MCP enterprise tooling.
6. **Polished BYOA adapters for Claude/Codex/Cursor** — covered today by the generic `HttpAdapter`; dedicated adapters arrive when upstream APIs stabilise. **JetBrains Junie CLI's "1-click migration from Claude Code, Codex"** (Mar 2026) is the bar for what users now expect; tracked as v13 item A17.

---

## 16. v13 — April 2026 trend delta + audit reconciliation

This iteration splits into two independent passes against the same fitgap. **§16.1** is the *external* delta — a survey of what shipped in the AI-coding ecosystem between the v0.5.5 refresh (2026-04-17) and today (2026-04-26). **§16.2** is the *internal* delta — a reclassification of previously-claimed gap closures against the audit at [docs/audit/05-fitgap-overstatements.md](./audit/05-fitgap-overstatements.md). Both feed into [Phase 53](./roadmap/#appendix-d--phase-53-april-2026-trend-delta--audit-reconciliation) of the roadmap.

### 16.1 External delta — what the industry shipped

Sources surveyed (web, 2026-04-26): Cursor changelog, Anthropic Claude Code changelog, GitHub Copilot blog, Cognition Devin blog, OpenAI Codex changelog, Google Antigravity changelog, Gemini CLI release notes, JetBrains Junie blog, MCP 2026 roadmap, ACP repo, A2A specification, SWE-bench leaderboards, sandbox provider coverage (E2B / Northflank / Cloudflare / Modal / Vercel / Docker).

**Headline shifts in the eight days since v0.5.5:**

- **Cursor 3.0 (Apr 2)** + **3.2 (Apr 24)** — Agents Window, Design Mode (UI-element annotation in browser), Agent Tabs (side-by-side / grid view), async subagents, multi-root workspaces, multi-repo agent context.
- **GitHub Copilot Cloud Agent (Apr 1)** — formerly "Copilot coding agent" — no longer PR-only; can branch-only work; **CLI sessions remote-controllable from GitHub.com or GitHub Mobile**. Inline Agent Mode public preview in JetBrains IDEs (Apr 24). Claude Opus 4.7 GA on Copilot for Pro+/Business/Enterprise.
- **Devin 2.2** — agent now has full Linux desktop, **tests its work via computer use, self-verifies, auto-fixes**. Cognition raising at $25B valuation; Windsurf folded into Cognition family.
- **OpenAI Codex CLI (Apr 2026)** — **GPT-5.3-Codex-Spark** lightweight model at 1000+ TPS, hooks GA, plugin marketplace browsing, multi-environment app-server sessions, **Amazon Bedrock auth + AWS SigV4 signing** as a first-class provider.
- **Claude Code** — `/agents` tabbed UI (Running / Library tabs with Run + View instance from Library), parallel MCP server reconnect, plugin-skill hot-reload, isolated-worktree subagent permission fix, YAML-list globs in skill paths, real-time skill progress display.
- **Gemini CLI v0.38** — **Subagents** (delegating orchestrator pattern), **Chapters** (intent-grouped interactions), Context Compression Service, generalist agent task delegation.
- **JetBrains Junie CLI (Beta, Mar 2026)** — LLM-agnostic, runs in IDE / terminal / CI/CD / GitHub / GitLab; connects to running JetBrains IDE for full code intelligence; **one-click migration from Claude Code + Codex configs**.
- **Antigravity 1.20.3 → 1.22.2** — AGENTS.md fallback (in addition to GEMINI.md), Linux sandboxing, MCP authentication improvements, conversation load-time improvements, Auto-continue default-on (deprecated as setting).
- **MCP 2026 roadmap** — stateless transport for horizontal scale, `.well-known` capability metadata, **MCP Apps** (interactive UI components in conversations), **MCPB** bundle distribution format, enterprise SSO/audit/gateway as extensions.
- **ACP v0.11.0 (Mar 4)** — Zed + JetBrains official partnership Oct 2025; Anthropic, OpenAI, GitHub, Google all ship implementations; Gemini CLI is the reference implementation. JSON-RPC 2.0 over stdio; **>40% lower prompt response latency** vs. ad-hoc bridges per OpenClaw measurements.
- **Augment Code SWE-bench Verified 72.0% pass@1** — highest open-system score, no best-of-N tricks.
- **Sandbox infrastructure mainstreamed** — Cloudflare Sandboxes GA, Vercel/Ramp/Modal/Docker/E2B/Northflank/Together all shipped microVM AI-execution platforms in 2026; isolation tier (microVM > gVisor > containers) is now table-stakes for cloud agents.

**Eleven new gaps surfaced by this delta** (none yet implemented in VibeCody):

| # | Gap | Surfaced by | Notes |
|---|-----|-------------|-------|
| A1 | MCP Apps extension — interactive UI components in conversations | MCP 2026 roadmap | New extension; would render dashboards/forms/multi-step workflows directly inside the chat panel. |
| A2 | MCPB bundle distribution format | MCP 2026 roadmap | Local-server packaging; analogous to VS Code `.vsix` for MCP. |
| A3 | MCP `.well-known` capability discovery + stateless transport | MCP 2026 roadmap | Lets `vibecli serve` announce MCP endpoints without a live connection; required for horizontal scale. |
| A4 | ACP server mode (Zed + JetBrains + Neovim editor protocol) | ACP v0.11 | VibeCLI/VibeUI as ACP servers callable from Zed/JetBrains/Neovim — different from being an ACP client. |
| A5 | Async subagents (long-running, check-back-later) | Cursor 3.2 | Distinct from our current parallel-agent worktree pool, which assumes synchronous oversight. |
| A6 | Multi-root workspace agent — agent that targets several working dirs per turn | Cursor 3.2, Codex CLI | Our `--add-dir` is read-only; this is *write* across roots in one agent invocation. |
| A7 | Browser-native UI-element annotation Design Mode | Cursor 3.0 | Existing `design_mode.rs` annotates static screenshots; this is live DOM annotation in a controlled browser. |
| A8 | Self-verifying agent loop (UI/desktop tests against own output, auto-fix) | Devin 2.2 | Closes the verification loop our `visual_verify.rs` opened — currently we screenshot-diff but don't feed failures back into the agent. |
| A9 | Cloud-agent remote-control protocol (start local, resume from web/mobile) | Copilot Cloud Agent | VibeMobile pairs with a host but doesn't *resume an in-flight CLI session* the way Copilot's new flow does. |
| A10 | Skills hot-reload + real-time progress display | Claude Code | Our skill loader requires restart for new skills; no streaming progress UI. |
| A11 | One-click migration from Claude Code / Codex configs | Junie CLI | Read existing `CLAUDE.md`, `codex.toml`, MCP server lists → emit `VIBECLI.md` + `~/.vibecli/config.toml`. Lowers switching cost. |

Six items from the same survey are **already in flight or partially shipped** and don't count as new gaps:

- **Bedrock auth for our Claude provider** — `provider.rs` already accepts SigV4-signed bearer; needs explicit doc + `vibecli config provider claude --aws` UX.
- **GPT-5.3-Codex-Spark-class fast inference** — covered by our existing routing layer; needs the model added to `useModelRegistry.ts` once OpenAI exposes it via API.
- **Generalist routing layer** (Gemini CLI Chapters / generalist agent) — partially covered by our `cost_router.rs` (data-structure-only — see §16.2) and `next_task.rs`.
- **AGENTS.md ↔ GEMINI.md fallback parser** — our `memory.rs` already reads AGENTS.md / VIBECLI.md / CLAUDE.md; adding `GEMINI.md` is one-line.
- **Plugin marketplace listing** (Codex CLI) — our existing `plugin_marketplace.rs` covers this; needs remote browsing UX in VibeUI.
- **Manager/Agents Window UI consolidation** — our `ManagerView.tsx` covers parallel agents; we should explicitly stay distant from Cursor 3's "Agents Window" and Antigravity's "Manager Surface" layout choices on patent grounds (ties to the patent-distance posture in [notes/PATENT_AUDIT_INLINE.md](../notes/PATENT_AUDIT_INLINE.md)).

### 16.2 Internal delta — audit reconciliation

The audit at [docs/audit/05-fitgap-overstatements.md](./audit/05-fitgap-overstatements.md) catalogued modules previously claimed as "closed" that ship data structures + in-memory tests + a panel + a REPL command but lack the I/O layer the gap closure implied. **Six of those have already been converted to real I/O (US-001 web grounding, US-002 A2A, US-003 worktree, US-004 MCP streamable, US-005 voice/whisper, US-006 proactive scanner)**. The remaining 8 modules + the RL-OS subsystem are reclassified as **Partial** in §3 and queued in roadmap Phase 53 for the same conversion treatment:

| Module | Original gap | What's missing | Conversion approach |
|--------|--------------|----------------|---------------------|
| `issue_triage.rs` | v7 Gap 10 — autonomous issue classification with GitHub/Linear integration | No HTTP calls to GitHub/Linear | `octocrab` + Linear SDK; gate behind `VIBECLI_GITHUB_TOKEN` / `VIBECLI_LINEAR_TOKEN`; mock-server BDD harness like US-001. |
| `native_connectors.rs` | v7 Gap 14 — connector trait + 20 service implementations + OAuth | Endpoint URL strings only; no `reqwest`, no async, no OAuth | Phase the 20 connectors; ship 4–5 first (Stripe, Slack, Linear, Notion, GitHub) with real OAuth + `oauth2` crate; defer the remaining 15 to a later slice. |
| `langgraph_bridge.rs` | v7 Gap 19 — LangGraph-compatible REST API + checkpoint format interop | No HTTP/REST implementation | `axum` server exposing LangGraph's documented routes; checkpoint JSON schema validation; LangGraph Python SDK conformance test. |
| `mcts_repair.rs` | v7 Gap 8 — MCTS with UCB1 + rollout via test execution | Has select/expand/backpropagate; rollout never runs actual tests | Wire rollout to `cargo test` / `pytest` / `npm test` per language; cap with per-rollout time budget; record outcome as the reward signal. |
| `sketch_canvas.rs` | v7 Gap 20 — wireframe → React/HTML/SwiftUI; 3D scene export | Basic shape data; no WebGL, no three.js, no 3D | Defer 3D entirely; ship the 2D wireframe → React JSX path against tldraw or an existing OSS recognizer; mark 3D as out of scope. |
| `cost_router.rs` | v7 — intelligent cost-aware request routing | Data structures only | Wire to `provider.rs` retry + circuit breaker; track per-(provider, model) latency/cost in `agent_analytics.rs`; routing decision becomes a real function of observed data. |
| `semantic_index.rs` | v7 Gap 5 — AST-level codebase understanding + call graph + type hierarchy | Line-by-line regex (`trimmed.starts_with("pub fn")`); no tree-sitter; no call graph | Replace regex with `tree-sitter` + per-language grammars (Rust, TS, Python, Go); reuse the index from `vibe-core/src/index/symbol.rs` which already uses tree-sitter. |
| `linter_aggregator` (in `ai_code_review.rs`) | cr/arch — 8 linters: clippy, eslint, pylint, … | `simulate_linter()` returns canned "Linter check passed" for every file | Spawn each linter as a subprocess; parse stdout; map findings to the existing `Finding` schema; the FP-filter LLM pass already exists. |
| `rl_*.rs` (8 files, ~31K lines) | RL-OS deep-dive — 30+ algorithms, JIT GPU/TPU kernels, Python bindings | No tch/candle/onnxruntime; "gradient sync" is `Vec<f64>` averaging; no GPU compute; no PyO3 | Ship one algorithm end-to-end first (PPO with `candle` on CPU), then expose via PyO3; the 52 type-system entries become real once *one* training loop is real. |

This is the same playbook that produced the US-001…US-006 conversions — design exists, tests exist, panel exists, REPL command exists; the conversion is purely "wire up the I/O layer + add a mock-server BDD harness". Phase 53 in the roadmap groups these as **US-007…US-015** for tracking parity with the prior conversions.

### 16.3 Updated remaining-parity-gaps list

Combining §15 (six long-horizon items, refreshed for v13) with §16.1 (eleven new external gaps), the **honest open-gaps total is 17**, not 6. The 14 partial items are tracked separately because they have shipped UX surface area — the work to complete them is well-scoped, not open-ended.

### 16.4 v14 — May 2026 weekly delta + missed-quarter items (added 2026-05-03)

This is a one-week refresh on top of v13, plus a small set of Q1-Q2 2026 items v13 missed. Sources surveyed (web, 2026-04-26 → 2026-05-03): cursor.com/changelog, GitHub Copilot blog, Anthropic Claude Code releases, OpenAI Codex / ChatGPT release notes, Cognition Devin docs, blog.modelcontextprotocol.io, a2a-protocol.org, Linux Foundation press, JetBrains Junie + Air blogs, Ollama releases, ggml-org/llama.cpp, vLLM releases, SWE-bench leaderboards, sandbox provider coverage (E2B / Daytona / Modal / Blaxel / SmolVM / Hyperlight), and OSS coding-agent repos (Cline / OpenHands / Aider / Continue).

**Headline shifts in the seven days since v13** (most are also surfaced in [Roadmap §1ter](./roadmap/#1ter-may-2026-weekly-delta--missed-quarter-items-added-2026-05-03)):

- **MCP `experimental-ext-skills` (May 4)** — skills discovery + distribution as MCP primitives. The single highest-leverage signal of the week for VibeCody — our 711 skill files could become MCP-discoverable across every host that speaks MCP, without per-host plugin work.
- **Cursor Plugin Marketplace v2 (May 1)** — plugins now bundle MCP servers + skills + subagents + rules + hooks; admin install policy (Default Off / On / Required); Team Marketplace decoupled from any specific repo.
- **Cursor Security Review (Apr 30, beta)** — always-on Security Reviewer + Vulnerability Scanner agents on Teams / Enterprise plans.
- **VS 2026 + VS Code Integrated Cloud Agent (Apr 29)** — "assign a task, close the IDE, get a PR" — Copilot Cloud Agent now controllable from inside the editor.
- **OpenAI GPT-5.5 GA (Apr 23)** — recommended Codex default (replaces 5.4); GPT-5 latency at higher intelligence; fewer tokens per Codex task; computer-use focus.
- **Cursor SDK / `@cursor/sdk` (TypeScript)** — same agent runtime / harness / models as desktop, CLI, and web exposed as a TS SDK; direct competitor to `packages/agent-sdk/`.
- **llama.cpp NVFP4 (PR #22196 reposted Apr 21)** — Blackwell-native FP4 path merged; MXFP4 progressing in `ik_llama.cpp`; b8196+ runs MXFP4 MoE on Blackwell tensor cores.
- **Ollama 0.22.x (Apr–May)** — `/v1/messages` (Anthropic Messages API compat — Claude Code can drive Ollama-hosted open models); `ollama launch` registers Claude Desktop / Cowork / Code; Gemma 4 thinking + tool calls; MLX runner gains logprobs + fused top-P/K + repeat-penalty-in-sampler.
- **Chinese frontier wave (Apr)** — DeepSeek V4-Flash $0.14 / $0.28 per 1M (~7.7× cheaper than Qwen 3.6-Plus on chatbot loads); Qwen 3.6-Plus + Qwen 3.6-35B-A3B (Apache 2.0); Kimi K2.6 long-horizon agentic; MiniMax M2.7; GLM-5.1.
- **A2A v1.2 (Linux Foundation Agentic AI Foundation, Q1)** — 150+ orgs in production; signed agent cards (cryptographic signatures for domain verification); GA across Google / Microsoft / AWS.
- **ACP Registry live (Q1)** — built into Zed + JetBrains; lists Claude Code, Codex CLI, GitHub Copilot CLI, OpenCode, Gemini CLI. **VibeCLI is not yet registered.**
- **DAPO mainstreamed (Q1)** — OpenRLHF, verl, NeMo-RL all ship DAPO as default reasoning RL alongside PPO / GRPO; ByteDance paper open-sourced (50% fewer training steps for AIME-class tasks vs DeepSeek-R1-Zero-Qwen-32B).
- **Sandbox cold-start floor (Q1)** — Blaxel 25 ms; Daytona 27–90 ms (Docker); E2B Firecracker microVMs ~150 ms; Modal gVisor; SmolVM debuted 2026-04-17; Hyperlight Wasm 1–2 ms (still experimental, CNCF Sandbox).
- **SWE-bench Verified contamination (Q1)** — OpenAI stopped reporting Verified after audit found 59.4% of hard tasks have flawed tests; all frontier models contaminated. Reflected in §15.4 above; SWE-bench Pro / SWE-rebench / SWE-bench-Live are the new primary references.
- **Google I/O 2026 (May 19, planned)** — Gemini 4 + Android 17 + Agentic Coding Developer Preview; Gemini 3.1 Pro Preview already shipping ahead.

**Six new gaps surfaced by this delta** (B1–B6, all open; A1–A11 from v13 remain unchanged):

| # | Gap | Surfaced by | Notes |
|---|-----|-------------|-------|
| B1 | Skills as MCP primitives — discoverable & distributable across MCP hosts | MCP `experimental-ext-skills` (May 4) | Re-shape `vibecli/vibecli-cli/skills/` to expose each skill via MCP `list_skills` / `get_skill` resources; one MCP server, every host benefits. Largest single-leverage item this cycle. |
| B2 | Plugin bundle format with admin install policies | Cursor Plugin Marketplace v2 (May 1) | Define a VibeCody plugin manifest that bundles MCP servers + skills + subagents + rules + hooks; expose Default-Off / Default-On / Required tiers via `WorkspaceStore` policy + governance panel. |
| B3 | Always-on agent classes (security review, vuln scan) running on every change | Cursor Security Review (Apr 30) | Convert `/review` from on-demand to a daemon-resident agent class triggered by file-watcher / pre-commit / CI; route findings to existing `Finding` schema. |
| B4 | Cursor SDK parity audit | Cursor SDK (Apr) | Compare `packages/agent-sdk/` to `@cursor/sdk` along: subagents, hooks, plugins, skills, sandbox tiers, recap/resume, multi-client (mobile/watch). Items where Cursor's surface is wider become roadmap entries. |
| B5 | NVFP4 (Blackwell native) as a TurboQuant target | llama.cpp PR #22196 (Apr 21) | Add NVFP4 Metal+CUDA kernels alongside MXFP4 + AWQ-Marlin; CubeCL/Burn ban scope unchanged. |
| B6 | A2A signed agent-card façade | A2A v1.2 + LF (Q1) | Serve `/.well-known/agent.json` with P-256 ECDSA signature reusing watch-pairing's key infrastructure (Secure Enclave-aligned); register as A2A server, not just client. |

**Five items already covered or trivially closeable** (not new gaps):

- **Ollama `/v1/messages` route** — one route handler in `vibecli/vibecli-cli/src/serve.rs`; the existing Anthropic provider format already matches.
- **GPT-5.5 / GPT-5.4 model entries** — append to `useModelRegistry.ts` `STATIC_MODELS.openai`.
- **Sonnet 4.8 entry** (when Anthropic exposes it) — same one-file change in `useModelRegistry.ts`.
- **Qwen 3.6 / DeepSeek V4 / Kimi K2.6 entries** — append to the Ollama section of `useModelRegistry.ts` once GGUF / vLLM weights land.
- **`GEMINI.md` fallback in `memory.rs`** — already noted in v13 as one-line; remains pending.

**Three positioning signals** (informational, no roadmap action):

- **Copilot training-default opt-in** (Apr) — community backlash drove migration to Cline (58k stars), OpenHands (72k), Aider (27k). VibeCody's "no training on user code" stance becomes a measurable sales axis; surface in marketing, not engineering.
- **Doe v. GitHub Copilot** (ongoing) — DMCA dismissed; license / contract claims still proceeding. Reinforces the privacy-first positioning above; informs `/review`'s open-source-license-scan UX (already shipped).
- **JetBrains Air** (Mar) — agentic IDE rebuilt on Fleet remnants; supports OpenAI Codex, Anthropic Claude Agent, Gemini CLI, Junie as native agents. Watch item for §1.2; not a direct VibeUI competitor today.

### 16.5 Updated remaining-parity-gaps list (v14)

Combining §15 (six long-horizon items), §16.1 (eleven v13 external gaps A1–A11), and §16.4 (six v14 external gaps B1–B6), the **honest open-gaps total is 23**, up from 17 in v13. The 14 partial items continue to be tracked separately. Phase 54 (queued in [Roadmap §1ter](./roadmap/#1ter-may-2026-weekly-delta--missed-quarter-items-added-2026-05-03)) targets B1–B6 plus the trivially-closeable items above.

---

## 17. Headline positioning

> **VibeCody closes the 136 gaps that matter across 40+ competing AI coding tools — and is the only project that ships competitive entries in every category (terminal, IDE, cloud daemon, review bot, completions, mobile, watch) from a shared Rust + TypeScript monorepo.**

See [**Competitive Landscape & Roadmap**](./roadmap/) for the forward plan, surface-by-surface feature inventory, and differentiators.
