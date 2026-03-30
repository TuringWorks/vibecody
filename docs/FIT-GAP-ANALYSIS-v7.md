---
layout: page
title: Fit-Gap Analysis v7 — Late March 2026 Agentic Systems Reset
permalink: /fit-gap-analysis-v7/
---


**Date:** 2026-03-26 | **Updated:** 2026-03-29
**Previous analysis:** FIT-GAP-ANALYSIS-v6.md (2026-03-20)
**Focus:** Agentic systems, open-source agents, new AI paradigms, protocol standards — 22 new gaps across 35+ competitors

## Executive Summary

Six days since v6, the agentic coding landscape has crystallized around five megatrends:

1. **A2A + MCP dual-protocol world** — Google's Agent-to-Agent (A2A) protocol hit Linux Foundation governance with 100+ corporate backers. MCP reached 97M monthly SDK downloads. Every serious agent platform must speak both protocols.
2. **Parallel worktree agents are the new normal** — Cursor runs 8 isolated agents in git worktrees, Windsurf Wave 13 does parallel Cascade sessions, Claude Code spawns sub-agents. Single-threaded agent execution is obsolete.
3. **Proactive agents** — Jules identifies optimizations unprompted, Codex Automations triage issues autonomously, Cursor Automations learn and improve across runs. The shift from reactive to proactive is defining the next generation.
4. **Open-source agent explosion** — OpenHands (188+ contributors), Open-SWE (7,700 stars in days), Goose (Block/Square), Junie CLI (now OSS). Building a coding agent from scratch is now commoditized.
5. **Cost-efficiency as differentiator** — Moatless solves issues at $0.01, Agentless at $0.34. Smart model routing and MCTS-based repair challenge the "throw frontier models at everything" approach.

VibeCody v6 gaps are **all closed** (19/19, Phases 15-22 complete). This v7 identified **22 new gaps** across 4 priority tiers based on 35+ tool analysis.

> **STATUS (2026-03-29): ALL 22 v7 GAPS CLOSED.** Phases 23-31 complete. Phase 32 added 6 additional modules (context protocol, code review agent, diff review, code replay, speculative execution, explainable agent). TurboQuant KV-cache compression also shipped. Current totals: **9,570 tests**, **185 Rust modules**, **187 VibeUI panels**, **568 skill files**, **23 AI providers**.

**New competitors/updates since v6:** Warp 2.0 ADE, JetBrains Junie CLI (OSS), Open-SWE (LangChain), Gemini CLI 1.0, Augment Intent GA, Moatless Tools, Agentless, Jules Tools CLI

---

## Part A — New Competitive Landscape Entries

### A.1 Warp 2.0 — Agentic Development Environment (ADE)

| Dimension | Detail |
|-----------|--------|
| **Concept** | Terminal that hosts Claude Code, Codex, Gemini CLI, and its own Oz agent simultaneously |
| **Oz Agent** | Full terminal use + computer use for visual verification; 71% SWE-bench Verified |
| **Multi-Agent** | Run multiple agents in one terminal, each with their own context |
| **Unique** | Agents can see and interact with the terminal visually (Computer Use) |
| **Impact on VibeCody** | VibeCody's REPL is single-agent; no multi-agent terminal hosting |

### A.2 JetBrains Junie CLI (Now Open Source)

| Dimension | Detail |
|-----------|--------|
| **Architecture** | LLM-agnostic CLI agent running in terminal, IDE, CI/CD, GitHub/GitLab |
| **Pricing** | BYOK with zero platform markup (first to do this at scale) |
| **MCP** | One-click server installation; auto-detects when MCP servers would help |
| **Next-Task** | Predicts developer's next action and pre-computes suggestions |
| **Impact on VibeCody** | VibeCody charges nothing (MIT/OSS) but lacks next-task prediction at workflow level |

### A.3 Open-SWE (LangChain)

| Dimension | Detail |
|-----------|--------|
| **Architecture** | Open-source async cloud-hosted coding agent on LangGraph Deep Agents |
| **Pipeline** | Research codebase → plan → code → test → self-review → open PR |
| **Scale** | 7,700+ GitHub stars in first week; trending #2 |
| **Impact on VibeCody** | Establishes a reusable "async agent pipeline" pattern VibeCody should adopt |

### A.4 Gemini CLI 1.0

| Dimension | Detail |
|-----------|--------|
| **Context** | 1M token context window with Gemini 3 models |
| **Grounding** | Built-in Google Search grounding (no separate MCP server needed) |
| **Free tier** | 60 req/min, 1,000 req/day with Gemini 2.5 Pro |
| **Impact on VibeCody** | VibeCody lacks built-in web search grounding in the agent loop |

### A.5 Augment Intent (GA)

| Dimension | Detail |
|-----------|--------|
| **Context Engine** | Indexes 100K+ files with semantic understanding: function signatures, class hierarchies, import chains, API contracts |
| **Intent** | Desktop app for spec-driven multi-agent orchestration with living documents |
| **Certification** | First AI coding assistant with ISO/IEC 42001 certification |
| **Impact on VibeCody** | VibeCody's embeddings index is simpler; lacks deep semantic graph (call hierarchies, import chains) |

### A.6 Moatless Tools / Agentless

| Dimension | Detail |
|-----------|--------|
| **Moatless** | MCTS (Monte Carlo Tree Search) with custom reward function; 39% SWE-bench at $0.14/issue, 30.7% with DeepSeek at **$0.01/issue** |
| **Agentless** | Three-phase (localize → repair → validate) in 700 lines; $0.34/issue |
| **Impact on VibeCody** | VibeCody's agent loop is standard ReAct; lacks cost-optimized search strategies |

---

## Part B — Gap Analysis Matrix

### Priority Tier Definitions

| Priority | Meaning | Timeframe |
|----------|---------|-----------|
| **P0** | Competitors already shipping; market expectation in 30 days | Immediate |
| **P1** | Strong competitive signal; expected within 60 days | Near-term |
| **P2** | Differentiation opportunity; 90-day horizon | Medium-term |
| **P3** | Forward-looking; positions for H2 2026 | Strategic |

---

### P0 — Critical (5 gaps)

#### Gap 1: A2A Protocol Support
| Dimension | Detail |
|-----------|--------|
| **What** | Google's Agent-to-Agent protocol enables agents from different vendors to discover, negotiate capabilities, and collaborate as peers (not just tools) |
| **Who ships it** | Google A2A SDK, 100+ corporate backers, Linux Foundation governance |
| **VibeCody status** | **CLOSED** — `a2a_protocol.rs` implements full A2A: agent card generation, server/client modes, task lifecycle, SSE streaming, capability negotiation. Integrated with `acp_protocol.rs`. Shipped in Phase 23 (ce6ac1e). |
| **Gap** | ~~No A2A agent card, no A2A server/client, no capability negotiation with external agents~~ |
| **Deliverable** | `a2a_protocol.rs` + `A2aPanel.tsx` + `/a2a` REPL command — **SHIPPED** |

#### Gap 2: Parallel Worktree Agent Execution
| Dimension | Detail |
|-----------|--------|
| **What** | Run N agents in parallel, each in an isolated git worktree with its own branch, full file system isolation |
| **Who ships it** | Cursor (8 parallel agents in worktrees), Windsurf Wave 13 (parallel Cascade sessions), Claude Code (sub-agent spawning) |
| **VibeCody status** | **CLOSED** — `worktree_pool.rs` implements full worktree pool with parallel agent dispatch, merge orchestration, and auto-PR. Shipped in Phase 24 (3c7de4a). |
| **Gap** | ~~Cannot spin up 4-8 lightweight agent workers using `git worktree` without container overhead~~ |
| **Deliverable** | `worktree_pool.rs` + `WorktreePoolPanel.tsx` + `/worktree` REPL command — **SHIPPED** |

#### Gap 3: Proactive Agent Intelligence
| Dimension | Detail |
|-----------|--------|
| **What** | Agent scans codebase and proactively suggests improvements, identifies bugs, tech debt, performance issues — without being asked |
| **Who ships it** | Jules (proactive optimization), Codex Automations (autonomous triage), Cursor Automations (learn across runs) |
| **VibeCody status** | **CLOSED** — `proactive_agent.rs` implements background scanner with configurable cadence, issue categorization, confidence scoring, and learning store. Shipped in Phase 25 (3c7de4a). |
| **Gap** | ~~No proactive scanning mode; no learning from previous automation runs~~ |
| **Deliverable** | `proactive_agent.rs` + `ProactivePanel.tsx` + `/proactive` REPL command — **SHIPPED** |

#### Gap 4: Web Search Grounding in Agent Loop
| Dimension | Detail |
|-----------|--------|
| **What** | Agent can natively search the web during code generation to find API docs, library usage, error solutions |
| **Who ships it** | Gemini CLI (Google Search grounding built-in), Devin (full internet access), Cursor Background Agents (internet access in VMs) |
| **VibeCody status** | **CLOSED** — `web_grounding.rs` implements search provider abstraction (Google, Bing, Brave, SearXNG, Tavily), result ranking, citation tracking, agent tool integration, rate limiting, and caching. Shipped in Phase 26 (574bf0a). |
| **Gap** | ~~Agent cannot search the web mid-task to resolve unknowns~~ |
| **Deliverable** | `web_grounding.rs` + `WebGroundingPanel.tsx` + `/search` REPL command — **SHIPPED** |

#### Gap 5: Deep Semantic Codebase Index
| Dimension | Detail |
|-----------|--------|
| **What** | Full semantic understanding of codebase: function signatures, class hierarchies, import chains, API contracts, call graphs — not just embeddings |
| **Who ships it** | Augment Context Engine (100K+ files), Sourcegraph Cody (cross-repo graph), Supermaven Long Context (1M tokens) |
| **VibeCody status** | **CLOSED** — `semantic_index.rs` implements AST-level codebase understanding with call graph extraction, type hierarchy mapping, import chain resolution, incremental updates, and query API. Shipped in Phase 26 (574bf0a). |
| **Gap** | ~~Cannot answer "what calls this function across the codebase?" without LSP~~ |
| **Deliverable** | `semantic_index.rs` + `SemanticIndexPanel.tsx` + `/index` REPL command — **SHIPPED** |

---

### P1 — High Priority (6 gaps)

#### Gap 6: Cross-Tool Agent Skills Standard
| Dimension | Detail |
|-----------|--------|
| **What** | Open standard for agent skills interoperable across Claude Code, Cursor, Gemini CLI, Junie — 1,234+ community skills |
| **Who ships it** | Anthropic (introduced), adopted by Cursor, Gemini CLI, JetBrains Junie |
| **VibeCody status** | **CLOSED** — `agent_skills_compat.rs` implements import/export of standard Agent Skills format, community registry discovery, bidirectional conversion, and skill validation. 568 skill files. Shipped in Phase 23 (ce6ac1e). |
| **Gap** | ~~VibeCody skills cannot be shared to/from the community standard~~ |
| **Deliverable** | `agent_skills_compat.rs` + `/skills import|export|search|validate` — **SHIPPED** |

#### Gap 7: MCP Streamable HTTP + OAuth 2.1
| Dimension | Detail |
|-----------|--------|
| **What** | Next-gen MCP transport: Streamable HTTP for remote servers (replacing SSE), OAuth 2.1 for enterprise authentication |
| **Who ships it** | MCP 2026 roadmap (official), GitHub Copilot (auto-approve MCP), Cursor (MCP Apps) |
| **VibeCody status** | **CLOSED** — `mcp_streamable.rs` implements Streamable HTTP transport, OAuth 2.1 client/server, PKCE flow, token refresh, enterprise SSO integration, connection pooling, and backward compatibility. Shipped in Phase 27. |
| **Gap** | ~~Cannot serve as a remote MCP server with enterprise auth~~ |
| **Deliverable** | `mcp_streamable.rs` + extended McpPanel — **SHIPPED** |

#### Gap 8: MCTS Code Repair Strategy
| Dimension | Detail |
|-----------|--------|
| **What** | Monte Carlo Tree Search applied to code repair: explore multiple fix paths, evaluate with test execution, backtrack on failures |
| **Who ships it** | Moatless Tools (39% SWE-bench, $0.01-$0.14/issue), SWE-Agent (65% with mini-SWE-agent) |
| **VibeCody status** | **CLOSED** — `mcts_repair.rs` implements MCTS with UCB1 selection, rollout via test execution, configurable reward function, depth/breadth control, agentless mode, and cost tracking. Shipped in Phase 28. |
| **Gap** | ~~No tree-search code repair; agent commits to first fix attempt~~ |
| **Deliverable** | `mcts_repair.rs` + `MctsRepairPanel.tsx` + `/repair` REPL command — **SHIPPED** |

#### Gap 9: Multi-Agent Terminal Hosting
| Dimension | Detail |
|-----------|--------|
| **What** | Single terminal/REPL hosting multiple AI agents simultaneously, each with independent context |
| **Who ships it** | Warp 2.0 (hosts Claude Code + Codex + Gemini CLI + Oz), JetBrains Junie (works alongside other agents) |
| **VibeCody status** | **CLOSED** — `agent_host.rs` implements external agent process manager, terminal multiplexing, context isolation, output interleaving, and agent selection/routing. Shipped in Phase 24 (3c7de4a). |
| **Gap** | ~~Cannot host external agents alongside VibeCody agent in same session~~ |
| **Deliverable** | `agent_host.rs` + `AgentHostPanel.tsx` + `/host` REPL command — **SHIPPED** |

#### Gap 10: Autonomous Issue Triage
| Dimension | Detail |
|-----------|--------|
| **What** | Agent autonomously triages incoming GitHub/Linear issues: classifies, labels, assigns, drafts initial response, links to related code |
| **Who ships it** | OpenAI Codex Automations, Cursor Automations (with memory across runs), GitHub Copilot Coding Agent |
| **VibeCody status** | **CLOSED** — `issue_triage.rs` implements autonomous issue classification, severity estimation, auto-labeling, code-linking, draft response generation, and triage memory. Shipped in Phase 25 (3c7de4a). |
| **Gap** | ~~No autonomous issue classification, labeling, or initial response generation~~ |
| **Deliverable** | `issue_triage.rs` + `TriagePanel.tsx` + `/triage` REPL command — **SHIPPED** |

#### Gap 11: Visual Verification via Computer Use
| Dimension | Detail |
|-----------|--------|
| **What** | Agent takes screenshots of running application and compares against expected design to verify UI correctness |
| **Who ships it** | Warp Oz (Computer Use for visual verification), Claude Computer Use, Playwright MCP |
| **VibeCody status** | **CLOSED** — `visual_verify.rs` implements screenshot capture via CDP/Playwright, visual diff with perceptual hashing, reference baselines, design compliance scoring, responsive verification, and CI integration. Shipped in Phase 29. |
| **Gap** | ~~Cannot automatically verify that a UI change matches the intended design~~ |
| **Deliverable** | `visual_verify.rs` + `VisualVerifyPanel.tsx` + `/verify` REPL command — **SHIPPED** |

---

### P2 — Medium Priority (6 gaps)

#### Gap 12: Cost-Optimized Agent Routing
| Dimension | Detail |
|-----------|--------|
| **What** | Smart routing of tasks to the cheapest model/strategy that meets quality threshold |
| **Who ships it** | Moatless ($0.01/issue with DeepSeek), Agentless ($0.34/issue), Cursor credits system |
| **VibeCody status** | **CLOSED** — `cost_router.rs` implements task complexity estimation, model cost database, quality-vs-cost optimizer, A/B routing with feedback loop, per-task cost tracking, and budget enforcement. Shipped in Phase 28. |
| **Gap** | ~~All tasks use the same model regardless of difficulty; no cost optimization~~ |
| **Deliverable** | `cost_router.rs` + `CostRouterPanel.tsx` + `/route` REPL command — **SHIPPED** |

#### Gap 13: Next-Task Prediction
| Dimension | Detail |
|-----------|--------|
| **What** | Predict developer's next action at workflow level (not just next edit) and pre-compute suggestions |
| **Who ships it** | JetBrains Junie (next-task prediction), GitHub Copilot (Next Edit Suggestions) |
| **VibeCody status** | **CLOSED** — `next_task.rs` implements workflow state machine, developer intent inference, task suggestion engine, confidence scoring, and accept/reject feedback loop. Shipped in Phase 29. |
| **Gap** | ~~Cannot predict and suggest workflow-level next steps~~ |
| **Deliverable** | `next_task.rs` + `NextTaskPanel.tsx` + `/nexttask` REPL command — **SHIPPED** |

#### Gap 14: Native Integration Connectors
| Dimension | Detail |
|-----------|--------|
| **What** | Pre-built connectors for 30+ services (Stripe, Figma, Notion, Salesforce, Jira, etc.) that go beyond MCP |
| **Who ships it** | Replit Agent (30+ integrations), Bolt.new (Bolt Cloud), Lovable (expanding integrations) |
| **VibeCody status** | **CLOSED** — `native_connectors.rs` implements connector trait + 20 service implementations, auto-discovery, OAuth flow management, webhook receiver, and agent tool generation. Shipped in Phase 30. |
| **Gap** | ~~Users must configure MCP servers manually for each service~~ |
| **Deliverable** | `native_connectors.rs` + `ConnectorsPanel.tsx` + `/connect` REPL command — **SHIPPED** |

#### Gap 15: Offline Voice Coding
| Dimension | Detail |
|-----------|--------|
| **What** | Voice-to-code without internet dependency; runs speech recognition locally on device |
| **Who ships it** | Aider (offline Mac voice, 3.75x faster than typing), Whisper.cpp local |
| **VibeCody status** | **CLOSED** — `voice_local.rs` implements whisper.cpp integration for local speech recognition, voice activity detection, streaming transcription, Groq fallback, and configurable model sizes. Shipped in Phase 29. |
| **Gap** | ~~Voice coding unusable offline or in air-gapped environments~~ |
| **Deliverable** | `voice_local.rs` + `/voice local` REPL command — **SHIPPED** |

#### Gap 16: Living Documentation Sync
| Dimension | Detail |
|-----------|--------|
| **What** | Bidirectional sync between specs/docs and code — code changes update docs, doc changes generate tasks |
| **Who ships it** | Kiro (bidirectional spec-code sync), Augment Intent (living documents) |
| **VibeCody status** | **CLOSED** — `doc_sync.rs` implements bidirectional spec-code sync with code change detection, spec impact analysis, auto-update proposals, doc freshness scoring, drift alerts, and reconciliation engine. Shipped in Phase 29. |
| **Gap** | ~~Specs drift from implementation over time; no automatic reconciliation~~ |
| **Deliverable** | `doc_sync.rs` + `DocSyncPanel.tsx` + `/docsync` REPL command — **SHIPPED** |

#### Gap 17: Enterprise Agent Analytics
| Dimension | Detail |
|-----------|--------|
| **What** | Admin dashboards showing per-user/team agent usage, acceptance rates, cost, productivity impact |
| **Who ships it** | GitHub Copilot (per-user CCA metrics), Cursor Enterprise, Tabnine governance |
| **VibeCody status** | **CLOSED** — `agent_analytics.rs` implements per-user/team metrics, acceptance rates, cost analytics, ROI calculator, exportable reports, and trend analysis. Shipped in Phase 30. |
| **Gap** | ~~No enterprise visibility into agent effectiveness or ROI~~ |
| **Deliverable** | `agent_analytics.rs` + `AnalyticsPanel.tsx` + `/analytics` REPL command — **SHIPPED** |

---

### P3 — Strategic (5 gaps)

#### Gap 18: RLCEF Training Loop
| Dimension | Detail |
|-----------|--------|
| **What** | Reinforcement Learning from Code Execution Feedback — use test results and runtime signals to improve agent behavior |
| **Who ships it** | Poolside ($12B valuation, RLCEF-trained models), Magic.dev (100M token context) |
| **VibeCody status** | **CLOSED** — `rlcef_loop.rs` implements execution outcome tracking, reward signal computation, strategy adjustment, mistake pattern database, per-language error clustering, and training data export. Shipped in Phase 31. |
| **Gap** | ~~Agent makes the same class of mistakes repeatedly; no runtime learning~~ |
| **Deliverable** | `rlcef_loop.rs` + `RlcefPanel.tsx` + `/rlcef` REPL command — **SHIPPED** |

#### Gap 19: LangGraph/Deep Agent Pipeline Compatibility
| Dimension | Detail |
|-----------|--------|
| **What** | Compatibility with LangChain/LangGraph agent pipelines for building custom async coding agents |
| **Who ships it** | Open-SWE (LangGraph Deep Agents), LangChain ecosystem |
| **VibeCody status** | **CLOSED** — `langgraph_bridge.rs` implements LangGraph-compatible REST API, agent state serialization, checkpoint format compatibility, event stream adapter, and composable pipeline nodes. Shipped in Phase 31. |
| **Gap** | ~~Cannot serve as a node in a LangGraph pipeline~~ |
| **Deliverable** | `langgraph_bridge.rs` + `LangGraphPanel.tsx` + `/langgraph` REPL command — **SHIPPED** |

#### Gap 20: Sketch-to-3D / Design Canvas
| Dimension | Detail |
|-----------|--------|
| **What** | Sketch or wireframe on a canvas and generate 3D/interactive UI from the sketch |
| **Who ships it** | Replit Agent 4 (Design Canvas, sketch-to-3D), Trae (multimodal design-to-code) |
| **VibeCody status** | **CLOSED** — `sketch_canvas.rs` implements canvas drawing primitives, shape recognition, wireframe-to-component mapping, 3D scene generation, and SVG/PNG export. Shipped in Phase 31. |
| **Gap** | ~~No in-app sketch/drawing surface for rapid design iteration~~ |
| **Deliverable** | `sketch_canvas.rs` + `SketchCanvasPanel.tsx` + `/sketch` REPL command — **SHIPPED** |

#### Gap 21: Agent Reputation & Trust Scoring
| Dimension | Detail |
|-----------|--------|
| **What** | Trust scoring for agent outputs based on historical accuracy, test pass rates, review acceptance |
| **Who ships it** | Emerging in Devin (67% merge rate as trust signal), GitHub Copilot (self-review confidence) |
| **VibeCody status** | **CLOSED** — `agent_trust.rs` implements per-agent/model trust scoring, rolling accuracy windows, confidence calibration, auto-review thresholds, trust decay, and per-domain trust. Shipped in Phase 30. |
| **Gap** | ~~All agent outputs treated with equal confidence; no historical quality signal~~ |
| **Deliverable** | `agent_trust.rs` + `TrustPanel.tsx` + `/trust` REPL command — **SHIPPED** |

#### Gap 22: Agentic Package Manager
| Dimension | Detail |
|-----------|--------|
| **What** | Agent-native package/dependency management: auto-detect needed packages, resolve conflicts, generate lockfiles, handle security advisories |
| **Who ships it** | Amazon Q Developer (code transformation), Tabnine (Jira-to-code with auto-deps), Replit (auto-installs) |
| **VibeCody status** | **CLOSED** — `smart_deps.rs` implements dependency graph analysis, conflict resolution, package comparison, CVE auto-patching, lockfile management, monorepo support, and license compliance. Shipped in Phase 30. |
| **Gap** | ~~Agent cannot autonomously resolve dependency conflicts~~ |
| **Deliverable** | `smart_deps.rs` + `SmartDepsPanel.tsx` + `/deps` REPL command — **SHIPPED** |

---

## Part C — Competitive Heatmap

| Capability | VibeCody | Cursor | Claude Code | Copilot | Kiro | Warp 2.0 | Gemini CLI | Junie | Augment | Devin |
|-----------|----------|--------|-------------|---------|------|----------|------------|-------|---------|-------|
| A2A Protocol | **Full** | Partial | Partial | No | No | No | Partial | No | No | No |
| Parallel Worktrees | **Pool+Host** | **8 agents** | Sub-agents | CCA | No | Multi-agent | No | No | Multi-agent | Full VM |
| Proactive Agent | **Full** | Automations | Channels | Partial | No | No | No | Next-task | Intent | Partial |
| Web Search Grounding | **5 providers** | VM internet | No | No | No | No | **Built-in** | No | No | Full internet |
| Deep Semantic Index | **AST+Graph** | Partial | No | No | No | No | No | No | **100K files** | No |
| MCP OAuth 2.1 | **Full** | Partial | Partial | Auto-approve | No | No | Partial | One-click | No | No |
| MCTS Repair | **Full** | No | No | No | No | No | No | No | No | No |
| Agent Skills Std | **568+compat** | **1,234+** | **1,234+** | No | No | No | **1,234+** | **1,234+** | No | No |
| Cost Routing | **Full** | Credits | No | Premium req | No | No | Free tier | BYOK | Enterprise | ACU |
| Visual Verify | **Full** | No | Comp Use | No | No | **Oz CU** | No | No | No | Comp Use |
| Issue Triage | **Full** | Automations | Channels | CCA | No | No | No | No | No | Auto |
| Living Doc Sync | **Bidir** | No | No | No | **Bidir** | No | No | No | **Intent** | No |
| Offline Voice | **whisper.cpp** | No | No | No | No | No | No | No | No | No |
| Next-Task | **Full** | No | No | NES | No | No | No | **Predicts** | No | No |
| Agent Analytics | **Full** | Enterprise | Teams | **Per-user** | No | No | No | No | Enterprise | Dashboard |

Legend: **Bold** = industry-leading, Partial = some implementation, No = competitor lacks it too

---

## Part D — Implementation Phases

### Phase 23: Dual-Protocol Agent Communication (P0) — Gaps 1, 6

**Scope:** A2A protocol support + Agent Skills standard compatibility

| Module | Tests | REPL | Panel |
|--------|-------|------|-------|
| `a2a_protocol.rs` | 55+ | `/a2a` (card, serve, discover, call, tasks) | A2aPanel.tsx |
| `agent_skills_compat.rs` | 35+ | `/skills import|export|search|validate` | (extends existing) |

### Phase 24: Parallel Agent Workers (P0) — Gaps 2, 9

**Scope:** Git worktree pool + multi-agent terminal hosting

| Module | Tests | REPL | Panel |
|--------|-------|------|-------|
| `worktree_pool.rs` | 50+ | `/worktree spawn|list|merge|cleanup` | WorktreePoolPanel.tsx |
| `agent_host.rs` | 40+ | `/host add|list|route|remove` | AgentHostPanel.tsx |

### Phase 25: Proactive Intelligence (P0) — Gaps 3, 10

**Scope:** Proactive scanning + autonomous issue triage

| Module | Tests | REPL | Panel |
|--------|-------|------|-------|
| `proactive_agent.rs` | 45+ | `/proactive scan|config|accept|reject|history` | ProactivePanel.tsx |
| `issue_triage.rs` | 40+ | `/triage run|rules|labels|history` | TriagePanel.tsx |

### Phase 26: Agent Grounding & Context (P0, P1) — Gaps 4, 5

**Scope:** Web search grounding + deep semantic codebase index

| Module | Tests | REPL | Panel |
|--------|-------|------|-------|
| `web_grounding.rs` | 40+ | `/search web|cache|providers` | WebGroundingPanel.tsx |
| `semantic_index.rs` | 55+ | `/index build|query|callers|callees|hierarchy` | SemanticIndexPanel.tsx |

### Phase 27: Protocol & Transport (P1) — Gap 7

**Scope:** MCP Streamable HTTP + OAuth 2.1

| Module | Tests | REPL | Panel |
|--------|-------|------|-------|
| `mcp_streamable.rs` | 45+ | `/mcp serve-http|oauth|tokens` | (extends McpPanel) |

### Phase 28: Smart Repair & Routing (P1, P2) — Gaps 8, 12

**Scope:** MCTS code repair + cost-optimized agent routing

| Module | Tests | REPL | Panel |
|--------|-------|------|-------|
| `mcts_repair.rs` | 50+ | `/repair mcts|agentless|compare` | MctsRepairPanel.tsx |
| `cost_router.rs` | 40+ | `/route cost|budget|model|stats` | CostRouterPanel.tsx |

### Phase 29: Developer Experience (P1, P2) — Gaps 11, 13, 15, 16

**Scope:** Visual verification + next-task prediction + offline voice + living docs

| Module | Tests | REPL | Panel |
|--------|-------|------|-------|
| `visual_verify.rs` | 35+ | `/verify screenshot|diff|baseline` | VisualVerifyPanel.tsx |
| `next_task.rs` | 40+ | `/nexttask suggest|accept|reject|learn` | NextTaskPanel.tsx |
| `voice_local.rs` | 30+ | `/voice local|model|download` | (extends existing) |
| `doc_sync.rs` | 35+ | `/docsync status|reconcile|watch` | DocSyncPanel.tsx |

### Phase 30: Enterprise & Ecosystem (P2, P3) — Gaps 14, 17, 21, 22

**Scope:** Native connectors + agent analytics + trust scoring + smart deps

| Module | Tests | REPL | Panel |
|--------|-------|------|-------|
| `native_connectors.rs` | 50+ | `/connect list|add|test|remove` | ConnectorsPanel.tsx |
| `agent_analytics.rs` | 40+ | `/analytics dashboard|export|roi` | AnalyticsPanel.tsx |
| `agent_trust.rs` | 35+ | `/trust scores|history|config` | TrustPanel.tsx |
| `smart_deps.rs` | 40+ | `/deps resolve|compare|patch|audit` | SmartDepsPanel.tsx |

### Phase 31: Strategic Frontiers (P3) — Gaps 18, 19, 20

**Scope:** RLCEF loop + LangGraph bridge + sketch canvas

| Module | Tests | REPL | Panel |
|--------|-------|------|-------|
| `rlcef_loop.rs` | 45+ | `/rlcef train|eval|mistakes|reset` | RlcefPanel.tsx |
| `langgraph_bridge.rs` | 35+ | `/langgraph serve|connect|status` | LangGraphPanel.tsx |
| `sketch_canvas.rs` | 30+ | `/sketch new|recognize|generate|export` | SketchCanvasPanel.tsx |

---

## Part E — Estimated Impact

| Phase | Gaps | New Modules | Est. Tests | New Panels | Priority | Status |
|-------|------|-------------|------------|------------|----------|--------|
| 23 | 1, 6 | 2 | 90+ | 1 | P0 | **COMPLETE** |
| 24 | 2, 9 | 2 | 90+ | 2 | P0 | **COMPLETE** |
| 25 | 3, 10 | 2 | 85+ | 2 | P0 | **COMPLETE** |
| 26 | 4, 5 | 2 | 95+ | 2 | P0/P1 | **COMPLETE** |
| 27 | 7 | 1 | 45+ | 0 (extends) | P1 | **COMPLETE** |
| 28 | 8, 12 | 2 | 90+ | 2 | P1/P2 | **COMPLETE** |
| 29 | 11, 13, 15, 16 | 4 | 140+ | 3 | P1/P2 | **COMPLETE** |
| 30 | 14, 17, 21, 22 | 4 | 165+ | 4 | P2/P3 | **COMPLETE** |
| 31 | 18, 19, 20 | 3 | 110+ | 3 | P3 | **COMPLETE** |
| 32 | (bonus) | 6 | 200+ | 6 | — | **COMPLETE** |
| **Total** | **22 + 6 bonus** | **28** | **1,110+** | **22** | | **ALL DONE** |

**Actual totals (2026-03-29):**
- **9,570 unit tests** (0 failures)
- **185 Rust modules** (vibecli-cli/src/)
- **187 VibeUI panels**
- **100+ REPL commands**
- **568 skill files**
- **23 AI providers** + OpenRouter (300+)
- **35+ competitors analyzed**

---

## Part F — Competitive Positioning After v7

With all 22 gaps closed plus Phase 32 bonus modules, VibeCody is now the **only** tool that:

1. **Speaks all three protocols** — MCP (tools) + ACP (agent hosting) + A2A (agent-to-agent). No competitor implements all three.
2. **Has MCTS code repair** — Tree-search bug fixing is only in research tools (Moatless); bringing it to a production IDE is a first.
3. **Offers offline voice coding** — Only Aider has offline voice; VibeCody would be the only IDE/GUI tool with it.
4. **Combines proactive + reactive agents** — Most tools are one or the other; VibeCody would seamlessly blend both.
5. **Runs parallel worktree agents without Docker** — Lighter weight than Cursor's VM approach for local development.
6. **Has RLCEF-style learning** — Execution feedback improving agent behavior is Poolside's moat; integrating it into an open-source tool democratizes it.
7. **Bridges Rust and Python agent ecosystems** — LangGraph compatibility means VibeCody can participate in the largest agent framework ecosystem while maintaining Rust performance.

---

## Appendix: Competitor URLs & Sources

| Competitor | Source |
|-----------|--------|
| Warp 2.0 | warp.dev/blog/reimagining-coding-agentic-development-environment |
| Junie CLI | blog.jetbrains.com/junie/2026/03/junie-cli-the-llm-agnostic-coding-agent-is-now-in-beta |
| Open-SWE | blog.langchain.com/introducing-open-swe-an-open-source-asynchronous-coding-agent |
| Gemini CLI | github.com/google-gemini/gemini-cli |
| A2A Protocol | developers.googleblog.com/en/a2a-a-new-era-of-agent-interoperability |
| A2A Linux Foundation | linuxfoundation.org/press/linux-foundation-launches-the-agent2agent-protocol-project |
| MCP 2026 Roadmap | blog.modelcontextprotocol.io/posts/2026-mcp-roadmap |
| MCP 97M Downloads | digitalapplied.com/blog/mcp-97-million-downloads-model-context-protocol-mainstream |
| Augment Intent | augmentcode.com/product |
| Moatless Tools | github.com/aorwall/moatless-tools |
| Jules Tools CLI | blog.google/technology/google-labs/jules-tools-jules-api |
| Cursor Background Agents | cursor.com/changelog/2-0 |
| Agent Skills Standard | Anthropic-introduced, adopted by Cursor/Gemini CLI/Junie |
