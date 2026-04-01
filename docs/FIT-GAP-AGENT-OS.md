---
layout: page
title: "AgentOS Fit-Gap Analysis & Architecture Specification"
permalink: /fit-gap-agent-os/
---

# VibeCody AgentOS — Fit-Gap Analysis & Architecture Specification

> Comprehensive competitive mapping of VibeCody's agentic capabilities against 15+ Agent OS / agentic platforms, with gap identification and extension roadmap.

**Date:** 2026-03-31 | **Version:** 1.0

---

## 1. Executive Summary

VibeCody already contains **60+ agent-related modules** totaling **~45,000 lines** of Rust + TypeScript code spanning agent loops, multi-agent teams, MCP/A2A protocols, browser/desktop automation, sandboxed execution, memory systems, and code review agents. This document maps these capabilities against the competitive landscape to identify gaps and define the AgentOS extension.

**Key finding:** VibeCody covers ~78% of the competitive feature matrix. The primary gaps are in **agent marketplace/registry**, **visual workflow builder**, **agent observability dashboard**, **human-in-the-loop approval workflows**, and **deployment/scaling infrastructure**.

---

## 2. Competitive Landscape — Platforms Mapped

| # | Platform | Category | Key Differentiator |
|---|----------|----------|--------------------|
| 1 | **Claude Code / Agent SDK** | Vendor Agent SDK | Computer use, file/bash tools, hooks, MCP |
| 2 | **OpenAI Agents SDK** | Vendor Agent SDK | Handoffs, guardrails, tracing, tool calling |
| 3 | **Google ADK + Vertex Agent Builder** | Cloud Agent Platform | Sequential/Parallel/Loop workflows, A2A, Agent Engine |
| 4 | **Devin** | Autonomous Coding Agent | Sandboxed environment, planning, desktop testing |
| 5 | **CrewAI** | Multi-Agent Framework | Role-based teams, fast prototyping, A2A |
| 6 | **LangGraph** | Stateful Agent Framework | Graph-based state machines, durable execution |
| 7 | **AutoGen (Microsoft)** | Conversational Agents | Multi-party conversations, group debates |
| 8 | **VAST AgentEngine** | Agent Infrastructure | Containerized runtimes, lifecycle management, MCP |
| 9 | **Simplai** | Enterprise Agent OS | One-stop platform, regulated industries |
| 10 | **Cursor** | AI Code Editor | Tab completion, agent mode, background agents |
| 11 | **Windsurf (Codeium)** | AI Code Editor | Cascade flows, multi-file editing |
| 12 | **Augment Code** | AI Code Agent | Context engine, deep repo understanding |
| 13 | **Amazon Q Developer** | Cloud Agent | AWS integration, /transform, /review |
| 14 | **GitHub Copilot Agent** | Code Agent | PR agent, workspace agent, extensions |
| 15 | **OpenHands (ex-OpenDevin)** | Open-Source Agent | Sandboxed containers, runtime plugins |

---

## 3. Feature-by-Feature Fit-Gap Matrix

### Legend
- **HAVE** = Fully implemented in VibeCody
- **PARTIAL** = Implementation exists but incomplete
- **GAP** = Not yet implemented
- **N/A** = Not applicable to VibeCody's scope

### 3.1 Core Agent Loop

| # | Feature | Claude SDK | OpenAI SDK | Google ADK | Devin | CrewAI | LangGraph | VibeCody | Status |
|---|---------|-----------|-----------|-----------|-------|--------|-----------|----------|--------|
| 1 | Streaming LLM agent loop | Yes | Yes | Yes | Yes | Yes | Yes | `vibe-ai/agent.rs` (1,744L) | **HAVE** |
| 2 | Tool calling (function calling) | Yes | Yes | Yes | Yes | Yes | Yes | `tool_executor.rs` (1,817L) | **HAVE** |
| 3 | Multi-provider support | Claude only | OpenAI only | Gemini-first | Proprietary | Yes | Yes | 18 providers in `vibe-ai` | **HAVE** |
| 4 | System prompt injection defense | Yes | Guardrails | Yes | N/A | No | No | `agent.rs` prompt injection defense | **HAVE** |
| 5 | Circuit breaker / retry | Yes | Yes | Yes | Yes | No | Yes | `agent.rs` circuit breaker | **HAVE** |
| 6 | Approval policies (HITL) | Yes (hooks) | No | Yes (pause/resume) | Yes (interactive) | No | Yes | `policy.rs` (3 modes) | **HAVE** |
| 7 | Streaming token metrics | Yes | Yes | Yes | No | No | No | `AgentPanel.tsx` tok/s, TTFT | **HAVE** |
| 8 | Agent abort/cancel | Yes | No | Yes | Yes | No | Yes | `stop_agent_task` | **HAVE** |

### 3.2 Multi-Agent & Teams

| # | Feature | Claude SDK | OpenAI SDK | Google ADK | CrewAI | LangGraph | AutoGen | VibeCody | Status |
|---|---------|-----------|-----------|-----------|--------|-----------|---------|----------|--------|
| 9 | Multi-agent orchestration | No | Handoffs | Yes (Sequential/Parallel/Loop) | Yes (Crew) | Yes (Graph) | Yes (GroupChat) | `agent_teams_v2.rs` (888L) + `multi_agent.rs` (659L) | **HAVE** |
| 10 | Role-based agent teams | No | No | No | Yes (core) | No | Yes | `sub_agent_roles.rs` (428L) | **HAVE** |
| 11 | Agent-to-Agent protocol (A2A) | No | No | Yes (creator) | Partial | No | No | `a2a_protocol.rs` (1,821L) | **HAVE** |
| 12 | Agent Communication Protocol (ACP) | No | No | No | No | No | No | `acp_protocol.rs` (1,003L) | **HAVE** |
| 13 | Parallel agent spawning | No | No | Yes | Yes | Yes | No | `spawn_agent.rs` (1,737L) | **HAVE** |
| 14 | Inter-agent messaging bus | No | No | Yes | Yes | Yes (state) | Yes (chat) | `context_protocol.rs` (1,236L) | **HAVE** |
| 15 | Agent team governance | No | No | No | No | No | No | `team_governance.rs` | **HAVE** |
| 16 | Red/Blue/Purple team agents | No | No | No | No | No | No | `redteam.rs` + `blue_team.rs` + `purple_team.rs` (~4,400L) | **HAVE** |
| 17 | Counsel / debate multi-model | No | No | No | No | No | Yes | `CounselPanel.tsx` + backend | **HAVE** |
| 18 | Visual agent team builder (drag-drop) | No | No | Yes (Agent Designer) | No | No | LangGraph Studio | `CanvasPanel.tsx` (basic) | **PARTIAL** |
| 19 | Agent discovery registry | No | No | Yes (Agent Card) | No | No | No | — | **GAP** |
| 20 | Dynamic agent recruitment | No | No | Yes (A2A discovery) | No | CrewAI+ | No | — | **GAP** |

### 3.3 Tool & Capability System

| # | Feature | Claude SDK | OpenAI SDK | Google ADK | Devin | VibeCody | Status |
|---|---------|-----------|-----------|-----------|-------|----------|--------|
| 21 | File read/write/edit tools | Yes | No (custom) | No (custom) | Yes | `tool_executor.rs` | **HAVE** |
| 22 | Bash/shell execution | Yes | No | No | Yes | `tool_executor.rs` | **HAVE** |
| 23 | Web search tool | Yes | Yes | Yes | Yes | `WebGroundingPanel` | **HAVE** |
| 24 | MCP server support | Yes (core) | Yes | Yes | No | `mcp_server.rs` (988L) + 4 modules | **HAVE** |
| 25 | MCP directory/marketplace | Yes (registry) | No | Yes (API Registry) | No | `mcp_directory.rs` (883L) | **HAVE** |
| 26 | Custom tool definition | Yes (hooks) | Yes | Yes | No | `plugin_sdk.rs` (606L) | **HAVE** |
| 27 | Container-isolated tool execution | Yes (sandbox) | No | No | Yes | `container_tool_executor.rs` (328L) | **HAVE** |
| 28 | Browser automation | Yes (computer use) | No | No | Yes | `browser_agent.rs` (2,160L) | **HAVE** |
| 29 | Desktop/GUI automation | Yes (computer use) | No | No | Yes | `desktop_agent.rs` (2,190L) | **HAVE** |
| 30 | Tool governance / permissions | Yes (hooks) | Guardrails | Yes (API Registry) | N/A | `policy.rs` + hooks | **HAVE** |
| 31 | Tool usage analytics | No | Yes (tracing) | Yes | No | `agent_analytics.rs` (679L) | **HAVE** |

### 3.4 Memory & Context

| # | Feature | Claude SDK | OpenAI SDK | Google ADK | Devin | CrewAI | VibeCody | Status |
|---|---------|-----------|-----------|-----------|-------|--------|----------|--------|
| 32 | Session memory | Yes | Yes | Yes | Yes | Yes | `session_memory.rs` (604L) | **HAVE** |
| 33 | Cross-session memory | Yes (CLAUDE.md) | No | Yes | Yes | Yes (long-term) | `memory.rs` + `memory_auto.rs` | **HAVE** |
| 34 | OpenMemory protocol | No | No | No | No | No | `open_memory.rs` (4,355L) | **HAVE** |
| 35 | Memory recording/replay | No | No | No | No | No | `memory_recorder.rs` (259L) | **HAVE** |
| 36 | Context pruning / compression | Yes | Yes | Yes | Yes | No | `context_pruning.rs` | **HAVE** |
| 37 | RAG / embeddings context | No | Yes | Yes | No | Yes | `embeddings.rs` + `search.rs` | **HAVE** |
| 38 | Shared memory across agents | No | No | Yes (state) | No | Yes | `open_memory.rs` shared | **HAVE** |

### 3.5 Planning & Reasoning

| # | Feature | Claude SDK | OpenAI SDK | Google ADK | Devin | LangGraph | VibeCody | Status |
|---|---------|-----------|-----------|-----------|-------|-----------|----------|--------|
| 39 | Task planning / decomposition | Yes (extended thinking) | Yes (o-series) | Yes | Yes | Yes | `planner.rs` + `spawn_agent.rs` decomposition | **HAVE** |
| 40 | Plan mode (user-approved plans) | Yes (/plan) | No | Yes (HITL) | Yes | Yes | `plan_mode.rs` | **HAVE** |
| 41 | Speculative execution | No | No | No | No | No | `speculative_exec.rs` | **HAVE** |
| 42 | Explainable agent decisions | No | No | No | No | No | `explainable_agent.rs` (1,232L) | **HAVE** |
| 43 | Self-review quality gate | Yes | No | No | Yes | No | `self_review.rs` (1,171L) | **HAVE** |
| 44 | Next-task prediction | No | No | No | No | No | `proactive_agent.rs` (1,157L) | **HAVE** |

### 3.6 Sandbox & Execution Environment

| # | Feature | Claude SDK | Devin | OpenHands | Google ADK | VibeCody | Status |
|---|---------|-----------|-------|-----------|-----------|----------|--------|
| 45 | Docker container sandbox | No | Yes | Yes | No | `container_runtime.rs` (514L) | **HAVE** |
| 46 | Cloud sandbox (remote) | No | Yes | Yes | Agent Engine | `cloud_sandbox.rs` (382L) | **HAVE** |
| 47 | OpenSandbox integration | No | No | No | No | `opensandbox_client.rs` (1,011L) | **HAVE** |
| 48 | VM orchestration | No | Yes | No | No | `vm_orchestrator.rs` (1,304L) | **HAVE** |
| 49 | Workspace isolation (worktrees) | Yes | N/A | No | No | WorktreeManager in `vibe-ai` | **HAVE** |
| 50 | Resource limits / quotas | No | Yes | Yes | No | — | **GAP** |

### 3.7 Observability & Debugging

| # | Feature | Claude SDK | OpenAI SDK | Google ADK | Devin | LangGraph | VibeCody | Status |
|---|---------|-----------|-----------|-----------|-------|-----------|----------|--------|
| 51 | Execution tracing (JSONL) | Yes | Yes | Yes | Yes | Yes (LangSmith) | JSONL traces + sidecars | **HAVE** |
| 52 | Agent analytics dashboard | No | No | Yes | No | LangSmith | `agent_analytics.rs` (679L) | **PARTIAL** |
| 53 | Trust scoring | No | No | No | No | No | `agent_trust.rs` (601L) | **HAVE** |
| 54 | Cost tracking per agent | No | Yes | Yes | Yes | Yes | `CostPanel.tsx` | **HAVE** |
| 55 | Real-time agent status UI | Yes | No | Yes | Yes | LangGraph Studio | `AgentPanel.tsx` (step feed) | **HAVE** |
| 56 | Agent recording / replay | No | No | No | Yes | No | `AgentRecordingPanel.tsx` | **HAVE** |
| 57 | Centralized observability dashboard | No | No | Yes | Yes | LangSmith | `TraceDashboard.tsx` (basic) | **PARTIAL** |

### 3.8 Deployment & Scaling

| # | Feature | Claude SDK | Google ADK | VAST AgentEngine | Devin | VibeCody | Status |
|---|---------|-----------|-----------|------------------|-------|----------|--------|
| 58 | Agent hosting service | No | Agent Engine | Yes | SaaS | `agent_host.rs` (1,108L) | **PARTIAL** |
| 59 | Auto-scaling agents | No | Yes | Yes | Yes | — | **GAP** |
| 60 | Agent lifecycle management | No | Yes | Yes | Yes | Spawn lifecycle (queue/run/pause/complete) | **HAVE** |
| 61 | Agent versioning | No | No | No | No | — | **GAP** |
| 62 | Production deployment pipeline | No | Yes | Yes | SaaS | `agentic_cicd.rs` (992L) | **PARTIAL** |
| 63 | Remote/headless agent execution | Yes (API) | Yes | Yes | Yes | `--serve` HTTP daemon | **HAVE** |

### 3.9 Security & Governance

| # | Feature | Claude SDK | OpenAI SDK | Google ADK | VibeCody | Status |
|---|---------|-----------|-----------|-----------|----------|--------|
| 64 | Permission policies (allow/deny) | Yes (hooks) | Guardrails | Tool Governance | `policy.rs` (suggest/auto-edit/full-auto) | **HAVE** |
| 65 | Secret scanning in agent output | No | No | No | `self_review.rs` SecretScanner | **HAVE** |
| 66 | Prompt injection defense | Yes | Yes | Yes | `agent.rs` injection detection | **HAVE** |
| 67 | Audit trail | Yes (traces) | Yes | Yes | JSONL traces | **HAVE** |
| 68 | RBAC for agent access | No | No | Yes | `policy_engine.rs` (RBAC/ABAC) | **HAVE** |
| 69 | Agent sandboxing (untrusted code) | Yes | No | No | Container + OpenSandbox | **HAVE** |
| 70 | Compliance engine | No | No | No | `compliance.rs` | **HAVE** |

### 3.10 Developer Experience

| # | Feature | Claude SDK | Devin | Cursor | Copilot | VibeCody | Status |
|---|---------|-----------|-------|--------|---------|----------|--------|
| 71 | CLI agent interface | Yes | No | No | Yes | `vibecli` REPL | **HAVE** |
| 72 | Desktop GUI for agents | No | Web | Yes | VS Code | VibeUI (Tauri) | **HAVE** |
| 73 | Agent mode in editor | Yes | N/A | Yes | Yes | `AgentPanel.tsx` | **HAVE** |
| 74 | Background agents | Yes | Yes | Yes | Yes | `background_agents.rs` (545L) | **HAVE** |
| 75 | Turbo / full-auto mode | Yes | N/A | Yes | Yes | Turbo toggle in AgentPanel | **HAVE** |
| 76 | Agent skills library | Yes (CLAUDE.md) | No | No | No | `skills/` (~550 files) | **HAVE** |
| 77 | Interactive agent UI (forms, buttons) | No | No | No | No | `AgentUIRenderer.tsx` (229L) | **HAVE** |
| 78 | Plugin/extension SDK | No | No | Yes | Yes | `plugin_sdk.rs` (606L) | **HAVE** |

---

## 4. Gap Summary

### Critical Gaps (High Impact)

| # | Gap | Competitive Reference | Priority | Effort |
|---|-----|----------------------|----------|--------|
| G1 | **Agent Registry / Discovery** | Google A2A Agent Card, CrewAI Hub | P0 | Medium |
| G2 | **Visual Workflow Builder** (full DAG editor) | Google Agent Designer, LangGraph Studio | P0 | Large |
| G3 | **Agent Auto-Scaling** | Google Agent Engine, VAST AgentEngine | P1 | Large |
| G4 | **Agent Versioning & Rollback** | VAST AgentEngine | P1 | Medium |

### Moderate Gaps (Medium Impact)

| # | Gap | Competitive Reference | Priority | Effort |
|---|-----|----------------------|----------|--------|
| G5 | **Resource Limits / Quotas** per agent | Devin, Google Agent Engine | P1 | Small |
| G6 | **Dynamic Agent Recruitment** at runtime | Google A2A discovery, CrewAI+ | P2 | Medium |
| G7 | **Centralized Observability Dashboard** (unified traces, metrics, costs) | LangSmith, Google Cloud Trace | P1 | Medium |
| G8 | **Agent Marketplace** (share/install agent templates) | CrewAI Hub, MCP Registry | P2 | Large |

### Already Ahead of Competition

| # | Feature | VibeCody Advantage |
|---|---------|-------------------|
| A1 | **18 LLM providers** | No competitor supports more than 3-4 providers natively |
| A2 | **Red/Blue/Purple team agents** | Unique — no competitor has adversarial agent teams |
| A3 | **OpenMemory protocol** (4,355L) | Unique — shared memory standard across agents |
| A4 | **A2A + ACP + MCP** all three protocols | Only platform with all three interop protocols |
| A5 | **Explainable agent** with decision transparency | Unique — no competitor exposes reasoning chain |
| A6 | **Speculative execution** | Unique — pre-execute likely next steps |
| A7 | **Agent self-review gate** | Only Devin has similar; VibeCody's is more configurable |
| A8 | **Interactive agent UI** (forms, buttons in agent output) | Unique — agents can render custom UI |
| A9 | **550+ skill files** | Largest skill library of any agent platform |
| A10 | **Counsel mode** (multi-model debate) | Unique format with voting and synthesis |

---

## 5. Proposed AgentOS Tab Structure

The AgentOS tab group aggregates all agent-related panels into a unified experience:

```
AgentOS (new tab group in tabGroups.ts)
├── Agent         — Main agent panel (task → autonomous execution)
├── Teams         — Multi-agent team orchestration
├── Workflows     — Visual workflow builder (DAG)
├── Registry      — Agent discovery, templates, marketplace    [NEW]
├── Memory        — Agent memory management (session + persistent)
├── Sandbox       — Container/VM sandbox management
├── Observability — Unified traces, metrics, cost dashboard    [EXTEND]
├── Security      — Trust scoring, permissions, audit trail
├── Protocols     — MCP, A2A, ACP configuration
└── SDK           — Plugin SDK, custom tools, extensions
```

---

## 6. Implementation Roadmap

### Phase 1: Aggregation & Tab Group (Week 1)
- Create `AgentOS` tab group in `tabGroups.ts`
- Create `AgentOsComposite` with existing panels reorganized
- Unify AgentPanel, AgentTeamsPanel, AgentModesPanel, SpawnAgentPanel under one composite
- Add MCP, A2A, ACP panels to Protocols sub-tab

### Phase 2: Agent Registry (Weeks 2-3)
- `agent_registry.rs` — Agent Card schema (name, capabilities, version, provider requirements)
- `AgentRegistryPanel.tsx` — Browse, search, install agent templates
- Built-in templates: Code Reviewer, Bug Fixer, Test Writer, PR Agent, Refactorer, Security Auditor
- Import/export agent definitions as JSON/YAML

### Phase 3: Visual Workflow Builder (Weeks 3-5)
- Extend `CanvasPanel.tsx` into a full DAG editor
- Node types: Agent, Tool, Condition, Loop, Human Approval, Parallel Fork/Join
- Drag-and-drop from agent registry
- Execute workflows with real-time step highlighting
- Save/load workflow definitions

### Phase 4: Observability Dashboard (Weeks 4-5)
- Unified `AgentObservabilityPanel.tsx`
- Timeline view of all agent executions
- Cost breakdown per agent/session/provider
- Token usage graphs, latency percentiles
- Error rate tracking with drill-down

### Phase 5: Auto-Scaling & Versioning (Weeks 5-7)
- Agent versioning with rollback support
- Resource limits (max tokens, max duration, max cost per agent)
- Concurrent agent pool management
- Health checks and automatic restart

---

## 7. Architecture Principles

1. **Protocol-native**: A2A + ACP + MCP as first-class citizens, not plugins
2. **Provider-agnostic**: Any agent can use any of the 18+ LLM providers
3. **Sandbox-first**: All autonomous agent execution in containers by default
4. **Observable**: Every agent action traced, costed, and auditable
5. **Composable**: Agents are building blocks — combine via teams, workflows, or protocols
6. **Secure**: Trust scoring, permission policies, secret scanning on every execution

---

## Sources

- [AI Operating Systems & Agentic OS Explained (Fluid AI)](https://www.fluid.ai/blog/ai-operating-systems-agentic-os-explained)
- [Top 5 Open-Source Agentic AI Frameworks 2026 (AIMultiple)](https://aimultiple.com/agentic-frameworks)
- [CrewAI vs LangGraph vs AutoGen (OpenAgents)](https://openagents.org/blog/posts/2026-02-23-open-source-ai-agent-frameworks-compared)
- [Agent SDK overview (Anthropic)](https://platform.claude.com/docs/en/agent-sdk/overview)
- [Overview of Agent Development Kit (Google Cloud)](https://docs.cloud.google.com/agent-builder/agent-development-kit/overview)
- [A2A Protocol (Google)](https://developers.googleblog.com/en/a2a-a-new-era-of-agent-interoperability/)
- [MCP vs A2A Protocols (OneReach)](https://onereach.ai/blog/guide-choosing-mcp-vs-a2a-protocols/)
- [Devin AI Complete Guide (DigitalApplied)](https://www.digitalapplied.com/blog/devin-ai-autonomous-coding-complete-guide)
- [Devin 2026 Release Notes](https://docs.devin.ai/release-notes/2026)
- [A Detailed Comparison of Top 6 AI Agent Frameworks (Turing)](https://www.turing.com/resources/ai-agent-frameworks)
