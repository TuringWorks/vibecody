---
layout: page
title: Fit-Gap Analysis v7 — Late March 2026 Agentic Systems Reset
permalink: /fit-gap-analysis-v7/
---


**Date:** 2026-03-26
**Previous analysis:** FIT-GAP-ANALYSIS-v6.md (2026-03-20)
**Focus:** Agentic systems, open-source agents, new AI paradigms, protocol standards — 22 new gaps across 35+ competitors

## Executive Summary

Six days since v6, the agentic coding landscape has crystallized around five megatrends:

1. **A2A + MCP dual-protocol world** — Google's Agent-to-Agent (A2A) protocol hit Linux Foundation governance with 100+ corporate backers. MCP reached 97M monthly SDK downloads. Every serious agent platform must speak both protocols.
2. **Parallel worktree agents are the new normal** — Cursor runs 8 isolated agents in git worktrees, Windsurf Wave 13 does parallel Cascade sessions, Claude Code spawns sub-agents. Single-threaded agent execution is obsolete.
3. **Proactive agents** — Jules identifies optimizations unprompted, Codex Automations triage issues autonomously, Cursor Automations learn and improve across runs. The shift from reactive to proactive is defining the next generation.
4. **Open-source agent explosion** — OpenHands (188+ contributors), Open-SWE (7,700 stars in days), Goose (Block/Square), Junie CLI (now OSS). Building a coding agent from scratch is now commoditized.
5. **Cost-efficiency as differentiator** — Moatless solves issues at $0.01, Agentless at $0.34. Smart model routing and MCTS-based repair challenge the "throw frontier models at everything" approach.

VibeCody v6 gaps are **all closed** (19/19, Phases 15-22 complete). This v7 identifies **22 new gaps** across 4 priority tiers based on 35+ tool analysis.

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
| **VibeCody status** | `acp_protocol.rs` implements ACP (Agent Client Protocol) but NOT A2A. A2A is complementary: MCP = tool access, ACP = agent hosting, A2A = agent-to-agent communication |
| **Gap** | No A2A agent card, no A2A server/client, no capability negotiation with external agents |
| **Deliverable** | `a2a_protocol.rs` — A2A agent card generation, server mode (expose VibeCody as an A2A agent), client mode (discover and collaborate with external A2A agents), task lifecycle management |

#### Gap 2: Parallel Worktree Agent Execution
| Dimension | Detail |
|-----------|--------|
| **What** | Run N agents in parallel, each in an isolated git worktree with its own branch, full file system isolation |
| **Who ships it** | Cursor (8 parallel agents in worktrees), Windsurf Wave 13 (parallel Cascade sessions), Claude Code (sub-agent spawning) |
| **VibeCody status** | `branch_agent.rs` does single-branch agents; `vm_orchestrator.rs` does Docker containers. No native git worktree parallelism without Docker overhead |
| **Gap** | Cannot spin up 4-8 lightweight agent workers using `git worktree` without container overhead |
| **Deliverable** | `worktree_pool.rs` — Worktree lifecycle management, parallel agent dispatcher, merge conflict detection across worktrees, progress aggregation, auto-PR per worktree |

#### Gap 3: Proactive Agent Intelligence
| Dimension | Detail |
|-----------|--------|
| **What** | Agent scans codebase and proactively suggests improvements, identifies bugs, tech debt, performance issues — without being asked |
| **Who ships it** | Jules (proactive optimization), Codex Automations (autonomous triage), Cursor Automations (learn across runs) |
| **VibeCody status** | All current agent modes are reactive (user initiates). `automations.rs` triggers on events but doesn't learn or proactively scan |
| **Gap** | No proactive scanning mode; no learning from previous automation runs |
| **Deliverable** | `proactive_agent.rs` — Background scanner with configurable cadence, issue categorization (perf/security/debt/correctness), confidence scoring, learning store (what was accepted/rejected), priority ranking |

#### Gap 4: Web Search Grounding in Agent Loop
| Dimension | Detail |
|-----------|--------|
| **What** | Agent can natively search the web during code generation to find API docs, library usage, error solutions |
| **Who ships it** | Gemini CLI (Google Search grounding built-in), Devin (full internet access), Cursor Background Agents (internet access in VMs) |
| **VibeCody status** | `web_crawler.rs` fetches known URLs. No integration of web search results into the agent reasoning loop |
| **Gap** | Agent cannot search the web mid-task to resolve unknowns; must rely on training data or user-provided URLs |
| **Deliverable** | `web_grounding.rs` — Search provider abstraction (Google, Bing, Brave, SearXNG), result ranking/filtering, citation tracking, agent tool integration (`search_web` tool in system prompt), rate limiting, cache |

#### Gap 5: Deep Semantic Codebase Index
| Dimension | Detail |
|-----------|--------|
| **What** | Full semantic understanding of codebase: function signatures, class hierarchies, import chains, API contracts, call graphs — not just embeddings |
| **Who ships it** | Augment Context Engine (100K+ files), Sourcegraph Cody (cross-repo graph), Supermaven Long Context (1M tokens) |
| **VibeCody status** | `embeddings.rs` in vibe-core does vector search; `fast_context.rs` does grep-based context. No AST-level semantic graph |
| **Gap** | Cannot answer "what calls this function across the codebase?" or "what's the full type hierarchy for this interface?" without LSP |
| **Deliverable** | `semantic_index.rs` — Tree-sitter AST parsing for 20+ languages, call graph extraction, type hierarchy mapping, import chain resolution, incremental updates on file change, query API (callers, callees, implementations, dependents) |

---

### P1 — High Priority (6 gaps)

#### Gap 6: Cross-Tool Agent Skills Standard
| Dimension | Detail |
|-----------|--------|
| **What** | Open standard for agent skills interoperable across Claude Code, Cursor, Gemini CLI, Junie — 1,234+ community skills |
| **Who ships it** | Anthropic (introduced), adopted by Cursor, Gemini CLI, JetBrains Junie |
| **VibeCody status** | 511+ skill files in proprietary markdown format. Not compatible with the cross-tool standard |
| **Gap** | VibeCody skills cannot be shared to/from the community standard; users must maintain separate skill sets |
| **Deliverable** | `agent_skills_compat.rs` — Import/export standard Agent Skills format, skill discovery from community registries, bidirectional conversion (VibeCody ↔ standard), skill validation |

#### Gap 7: MCP Streamable HTTP + OAuth 2.1
| Dimension | Detail |
|-----------|--------|
| **What** | Next-gen MCP transport: Streamable HTTP for remote servers (replacing SSE), OAuth 2.1 for enterprise authentication |
| **Who ships it** | MCP 2026 roadmap (official), GitHub Copilot (auto-approve MCP), Cursor (MCP Apps) |
| **VibeCody status** | `mcp_server.rs` supports stdio and basic HTTP. No Streamable HTTP transport, no OAuth 2.1 |
| **Gap** | Cannot serve as a remote MCP server with enterprise auth; cannot connect to OAuth-protected remote MCP servers |
| **Deliverable** | `mcp_streamable.rs` — Streamable HTTP transport implementation, OAuth 2.1 client/server, token refresh, PKCE flow, enterprise SSO integration |

#### Gap 8: MCTS Code Repair Strategy
| Dimension | Detail |
|-----------|--------|
| **What** | Monte Carlo Tree Search applied to code repair: explore multiple fix paths, evaluate with test execution, backtrack on failures |
| **Who ships it** | Moatless Tools (39% SWE-bench, $0.01-$0.14/issue), SWE-Agent (65% with mini-SWE-agent) |
| **VibeCody status** | Agent uses linear ReAct loop. `auto_research.rs` has Bayesian/Genetic search but not applied to code repair |
| **Gap** | No tree-search code repair; agent commits to first fix attempt without exploring alternatives |
| **Deliverable** | `mcts_repair.rs` — MCTS with UCB1 selection, rollout via test execution, reward function (tests pass + minimal diff + no regressions), configurable depth/breadth, cost-tracking per exploration |

#### Gap 9: Multi-Agent Terminal Hosting
| Dimension | Detail |
|-----------|--------|
| **What** | Single terminal/REPL hosting multiple AI agents simultaneously, each with independent context |
| **Who ships it** | Warp 2.0 (hosts Claude Code + Codex + Gemini CLI + Oz), JetBrains Junie (works alongside other agents) |
| **VibeCody status** | REPL runs one agent at a time. `multi_agent.rs` coordinates but in same process |
| **Gap** | Cannot host external agents (Claude Code, Gemini CLI) alongside VibeCody agent in same session |
| **Deliverable** | `agent_host.rs` — External agent process manager, shared terminal multiplexing, context isolation, output interleaving, agent selection/routing |

#### Gap 10: Autonomous Issue Triage
| Dimension | Detail |
|-----------|--------|
| **What** | Agent autonomously triages incoming GitHub/Linear issues: classifies, labels, assigns, drafts initial response, links to related code |
| **Who ships it** | OpenAI Codex Automations, Cursor Automations (with memory across runs), GitHub Copilot Coding Agent |
| **VibeCody status** | `linear.rs` and `github_app.rs` read issues but don't auto-triage. `automations.rs` can trigger but lacks triage logic |
| **Gap** | No autonomous issue classification, labeling, or initial response generation |
| **Deliverable** | `issue_triage.rs` — Issue classifier (bug/feature/question/docs), severity estimator, auto-labeler, code-linking (which files relate), draft response generator, triage memory (learn from user corrections) |

#### Gap 11: Visual Verification via Computer Use
| Dimension | Detail |
|-----------|--------|
| **What** | Agent takes screenshots of running application and compares against expected design to verify UI correctness |
| **Who ships it** | Warp Oz (Computer Use for visual verification), Claude Computer Use, Playwright MCP |
| **VibeCody status** | `computer_use.rs` exists but focused on desktop interaction, not visual verification/comparison |
| **Gap** | Cannot automatically verify that a UI change matches the intended design by visual comparison |
| **Deliverable** | `visual_verify.rs` — Screenshot capture of running app (headless Chrome/Playwright), visual diff against reference images, perceptual hashing, design compliance scoring, integration with CI pipeline |

---

### P2 — Medium Priority (6 gaps)

#### Gap 12: Cost-Optimized Agent Routing
| Dimension | Detail |
|-----------|--------|
| **What** | Smart routing of tasks to the cheapest model/strategy that meets quality threshold |
| **Who ships it** | Moatless ($0.01/issue with DeepSeek), Agentless ($0.34/issue), Cursor credits system |
| **VibeCody status** | `failover.rs` routes on failure. No cost-aware routing that considers task complexity |
| **Gap** | All tasks use the same model regardless of difficulty; no cost optimization |
| **Deliverable** | `cost_router.rs` — Task complexity estimator, model cost database, quality-vs-cost optimizer, A/B routing with feedback loop, per-task cost tracking, budget enforcement |

#### Gap 13: Next-Task Prediction
| Dimension | Detail |
|-----------|--------|
| **What** | Predict developer's next action at workflow level (not just next edit) and pre-compute suggestions |
| **Who ships it** | JetBrains Junie (next-task prediction), GitHub Copilot (Next Edit Suggestions) |
| **VibeCody status** | `edit_prediction.rs` predicts next edit location. No workflow-level prediction (e.g., "you'll probably want to write tests for this next") |
| **Gap** | Cannot predict and suggest workflow-level next steps |
| **Deliverable** | `next_task.rs` — Workflow state machine, developer intent inference from recent actions, task suggestion engine (write tests, update docs, run lint, commit, deploy), confidence scoring, accept/reject learning |

#### Gap 14: Native Integration Connectors
| Dimension | Detail |
|-----------|--------|
| **What** | Pre-built connectors for 30+ services (Stripe, Figma, Notion, Salesforce, Jira, etc.) that go beyond MCP |
| **Who ships it** | Replit Agent (30+ integrations), Bolt.new (Bolt Cloud), Lovable (expanding integrations) |
| **VibeCody status** | MCP provides generic tool access. No native, zero-config connectors for popular services |
| **Gap** | Users must configure MCP servers manually for each service |
| **Deliverable** | `native_connectors.rs` — Pre-built connector trait + implementations for top 20 services (Stripe, Figma, Notion, Jira, Slack, PagerDuty, Datadog, Sentry, LaunchDarkly, Vercel, Netlify, Supabase, Firebase, AWS, GCP, Azure, GitHub, GitLab, Linear, Confluence), auto-discovery, OAuth flow management |

#### Gap 15: Offline Voice Coding
| Dimension | Detail |
|-----------|--------|
| **What** | Voice-to-code without internet dependency; runs speech recognition locally on device |
| **Who ships it** | Aider (offline Mac voice, 3.75x faster than typing), Whisper.cpp local |
| **VibeCody status** | `voice.rs` requires Groq Whisper API (cloud-dependent) |
| **Gap** | Voice coding unusable offline or in air-gapped environments |
| **Deliverable** | `voice_local.rs` — whisper.cpp integration for local speech recognition, voice activity detection, streaming transcription, fallback to Groq when online, configurable model size (tiny/base/small/medium) |

#### Gap 16: Living Documentation Sync
| Dimension | Detail |
|-----------|--------|
| **What** | Bidirectional sync between specs/docs and code — code changes update docs, doc changes generate tasks |
| **Who ships it** | Kiro (bidirectional spec-code sync), Augment Intent (living documents) |
| **VibeCody status** | `spec_pipeline.rs` generates specs → code. No reverse sync (code → spec update) |
| **Gap** | Specs drift from implementation over time; no automatic reconciliation |
| **Deliverable** | `doc_sync.rs` — Code change detection, spec impact analysis, auto-update proposals, doc freshness scoring, drift alerts, bidirectional reconciliation engine |

#### Gap 17: Enterprise Agent Analytics
| Dimension | Detail |
|-----------|--------|
| **What** | Admin dashboards showing per-user/team agent usage, acceptance rates, cost, productivity impact |
| **Who ships it** | GitHub Copilot (per-user CCA metrics), Cursor Enterprise, Tabnine governance |
| **VibeCody status** | `usage_metering.rs` tracks credits. No admin-facing analytics dashboard with acceptance rates |
| **Gap** | No enterprise visibility into agent effectiveness or ROI |
| **Deliverable** | `agent_analytics.rs` — Per-user/team metrics (suggestions accepted/rejected, time saved, cost, lines generated), exportable reports, trend analysis, ROI calculator, integration with existing admin panel |

---

### P3 — Strategic (5 gaps)

#### Gap 18: RLCEF Training Loop
| Dimension | Detail |
|-----------|--------|
| **What** | Reinforcement Learning from Code Execution Feedback — use test results and runtime signals to improve agent behavior |
| **Who ships it** | Poolside ($12B valuation, RLCEF-trained models), Magic.dev (100M token context) |
| **VibeCody status** | No execution-based feedback loop. Agent doesn't learn from whether its code ran correctly |
| **Gap** | Agent makes the same class of mistakes repeatedly; no runtime learning |
| **Deliverable** | `rlcef_loop.rs` — Execution outcome tracking (pass/fail/error), reward signal computation, strategy adjustment based on accumulated outcomes, mistake pattern database, per-language/framework error clustering |

#### Gap 19: LangGraph/Deep Agent Pipeline Compatibility
| Dimension | Detail |
|-----------|--------|
| **What** | Compatibility with LangChain/LangGraph agent pipelines for building custom async coding agents |
| **Who ships it** | Open-SWE (LangGraph Deep Agents), LangChain ecosystem |
| **VibeCody status** | Proprietary agent loop in Rust. Not composable with Python/LangGraph agent ecosystem |
| **Gap** | Cannot serve as a node in a LangGraph pipeline or consume LangGraph agent outputs |
| **Deliverable** | `langgraph_bridge.rs` — LangGraph-compatible REST API, agent state serialization, checkpoint format compatibility, event stream adapter, Python SDK wrapper |

#### Gap 20: Sketch-to-3D / Design Canvas
| Dimension | Detail |
|-----------|--------|
| **What** | Sketch or wireframe on a canvas and generate 3D/interactive UI from the sketch |
| **Who ships it** | Replit Agent 4 (Design Canvas, sketch-to-3D), Trae (multimodal design-to-code) |
| **VibeCody status** | `design_import.rs` imports Figma/images to code. No freeform sketch canvas |
| **Gap** | No in-app sketch/drawing surface for rapid design iteration |
| **Deliverable** | `sketch_canvas.rs` — Canvas drawing primitives, shape recognition (button, input, card, list, nav), wireframe-to-component mapping, 3D scene generation for Three.js/React Three Fiber, export to SVG/PNG |

#### Gap 21: Agent Reputation & Trust Scoring
| Dimension | Detail |
|-----------|--------|
| **What** | Trust scoring for agent outputs based on historical accuracy, test pass rates, review acceptance |
| **Who ships it** | Emerging in Devin (67% merge rate as trust signal), GitHub Copilot (self-review confidence) |
| **VibeCody status** | No tracking of agent output quality over time |
| **Gap** | All agent outputs treated with equal confidence; no historical quality signal |
| **Deliverable** | `agent_trust.rs` — Per-agent/model trust score, rolling accuracy window, confidence calibration, auto-review threshold (high-trust = auto-merge, low-trust = manual review), trust decay on failures |

#### Gap 22: Agentic Package Manager
| Dimension | Detail |
|-----------|--------|
| **What** | Agent-native package/dependency management: auto-detect needed packages, resolve conflicts, generate lockfiles, handle security advisories |
| **Who ships it** | Amazon Q Developer (code transformation), Tabnine (Jira-to-code with auto-deps), Replit (auto-installs) |
| **VibeCody status** | `deps.rs` (assumed from DepsPanel.tsx) manages dependencies. No agent-native smart resolution |
| **Gap** | Agent cannot autonomously resolve dependency conflicts, choose between alternative packages based on criteria, or auto-patch vulnerabilities |
| **Deliverable** | `smart_deps.rs` — Dependency graph analysis, conflict resolution strategies, alternative package comparison (downloads, maintenance, security), auto-patch for CVEs, lockfile management, monorepo-aware |

---

## Part C — Competitive Heatmap

| Capability | VibeCody | Cursor | Claude Code | Copilot | Kiro | Warp 2.0 | Gemini CLI | Junie | Augment | Devin |
|-----------|----------|--------|-------------|---------|------|----------|------------|-------|---------|-------|
| A2A Protocol | GAP | Partial | Partial | No | No | No | Partial | No | No | No |
| Parallel Worktrees | GAP | **8 agents** | Sub-agents | CCA | No | Multi-agent | No | No | Multi-agent | Full VM |
| Proactive Agent | GAP | Automations | Channels | Partial | No | No | No | Next-task | Intent | Partial |
| Web Search Grounding | GAP | VM internet | No | No | No | No | **Built-in** | No | No | Full internet |
| Deep Semantic Index | GAP | Partial | No | No | No | No | No | No | **100K files** | No |
| MCP OAuth 2.1 | GAP | Partial | Partial | Auto-approve | No | No | Partial | One-click | No | No |
| MCTS Repair | GAP | No | No | No | No | No | No | No | No | No |
| Agent Skills Std | GAP | **1,234+** | **1,234+** | No | No | No | **1,234+** | **1,234+** | No | No |
| Cost Routing | GAP | Credits | No | Premium req | No | No | Free tier | BYOK | Enterprise | ACU |
| Visual Verify | GAP | No | Comp Use | No | No | **Oz CU** | No | No | No | Comp Use |
| Issue Triage | GAP | Automations | Channels | CCA | No | No | No | No | No | Auto |
| Living Doc Sync | GAP | No | No | No | **Bidir** | No | No | No | **Intent** | No |
| Offline Voice | GAP | No | No | No | No | No | No | No | No | No |
| Next-Task | GAP | No | No | NES | No | No | No | **Predicts** | No | No |
| Agent Analytics | GAP | Enterprise | Teams | **Per-user** | No | No | No | No | Enterprise | Dashboard |

Legend: **Bold** = industry-leading, Partial = some implementation, GAP = not present, No = competitor lacks it too

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

| Phase | Gaps | New Modules | Est. Tests | New Panels | Priority |
|-------|------|-------------|------------|------------|----------|
| 23 | 1, 6 | 2 | 90+ | 1 | P0 |
| 24 | 2, 9 | 2 | 90+ | 2 | P0 |
| 25 | 3, 10 | 2 | 85+ | 2 | P0 |
| 26 | 4, 5 | 2 | 95+ | 2 | P0/P1 |
| 27 | 7 | 1 | 45+ | 0 (extends) | P1 |
| 28 | 8, 12 | 2 | 90+ | 2 | P1/P2 |
| 29 | 11, 13, 15, 16 | 4 | 140+ | 3 | P1/P2 |
| 30 | 14, 17, 21, 22 | 4 | 165+ | 4 | P2/P3 |
| 31 | 18, 19, 20 | 3 | 110+ | 3 | P3 |
| **Total** | **22** | **22** | **910+** | **19** | |

**Post-implementation totals (projected):**
- ~7,538+ unit tests
- 261+ Rust modules
- 183+ VibeUI panels
- 100+ REPL commands
- 35+ competitors analyzed

---

## Part F — Competitive Positioning After v7

After closing all 22 gaps, VibeCody would be the **only** tool that:

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
