---
layout: page
title: Competitive Roadmap v4 — March 2026 Reset
permalink: /roadmap-v4/
---


**Date:** 2026-03-20
**Previous:** ROADMAP-v3.md (March 2026) — Phases 10-14 complete
**Scope:** 19 new gaps from FIT-GAP-ANALYSIS-v6.md; 8 implementation phases across 4 priority tiers

## Current State

All phases from Roadmap v1 (1-5), v2 (6-9), and v3 (10-14) are **complete**. FIT-GAP v4 (23 gaps) and v5 (12 gaps) are **all closed**. FIT-GAP v6 identifies **19 new gaps** based on analysis of **30 competitors**.

| Metric | Count |
|--------|-------|
| Unit tests | ~6,628+ |
| Skill files | 543+ |
| AI providers | 23 direct + OpenRouter (300+) |
| VibeUI panels | 162+ |
| REPL commands | 93+ |
| Gateway platforms | 18 |
| Rust modules | 165+ |
| Competitors analyzed | 30 |

## Phase 15: Always-On Agent Infrastructure (P0)

**Why:** Claude Code Channels, Cursor Automations, and Codex Cloud Automations have established "always-on AI teammate" as a category. Developers now expect agents that work 24/7 on triggers — not just on-demand REPL interactions. This is the single highest-impact gap.

### 15.1 Channel Daemon

**Deliverables:**

- [ ] `channel_daemon.rs` — Persistent daemon process that:
  - Listens on configured gateway channels (Slack, Discord, GitHub webhooks, Linear, PagerDuty, custom HTTP, MCP channels)
  - Routes incoming events to automations.rs trigger evaluator
  - Manages long-running agent sessions per channel (session affinity)
  - Health monitoring, graceful shutdown, auto-restart on crash
  - Rate limiting per channel source
- [ ] Daemon CLI mode: `vibecli daemon --channels slack,github --port 7879`
- [ ] Daemon config in `config.toml`: `[daemon]` section with channel configs
- [ ] REPL: `/daemon start|stop|status|channels|logs`
- [ ] Tests: 30+ unit tests

### 15.2 Cloud VM Agent Orchestration

**Deliverables:**

- [ ] `vm_orchestrator.rs` — Multi-environment agent manager:
  - Launch isolated Docker containers or cloud VMs per agent task
  - Each environment gets: fresh git clone, dedicated branch, resource limits (CPU/RAM/time)
  - Support for 1-N parallel environments (configurable max)
  - Environment lifecycle: provision → clone → work → commit → PR → cleanup
  - Integration with container_runtime.rs (Docker/Podman)
  - Cloud VM support via SSH + cloud provider APIs (deferred to 15.2b)
- [ ] Auto-PR generation on task completion (push branch, create PR with AI title/description)
- [ ] Conflict detection between parallel agent branches
- [ ] `VmOrchestratorPanel.tsx` — view active environments, branches, status, PRs
- [ ] Tests: 35+ unit tests

**Effort:** High (5-7 days total for 15.1 + 15.2)

## Phase 16: Spec-Driven Development (P0)

**Why:** Kiro (AWS) has validated spec-driven development as a methodology. Augment is shipping "Intent" — a living-spec workspace. Structured requirements pipelines differentiate professional workflows from ad-hoc "vibe coding."

### 16.1 EARS Pipeline

**Deliverables:**

- [ ] `spec_pipeline.rs` — Structured development pipeline:
  - **Requirements phase:** EARS (Easy Approach to Requirements Syntax) parser with 5 pattern types:
    - Ubiquitous: "The [system] shall [action]"
    - Event-driven: "When [trigger], the [system] shall [action]"
    - Unwanted behavior: "If [condition], then the [system] shall [action]"
    - State-driven: "While [state], the [system] shall [action]"
    - Optional: "Where [feature], the [system] shall [action]"
  - **Design phase:** Architecture decisions document auto-generated from requirements (components, interfaces, data flow)
  - **Tasks phase:** Implementation plan with ordered tasks, estimated effort, dependencies
  - **Validation:** Cross-reference consistency between requirements↔design↔tasks
- [ ] `.vibespec/` directory convention: `requirements.md`, `design.md`, `tasks.md`
- [ ] Spec watcher: file watcher auto-validates on change
- [ ] AI-assisted refinement at each stage (suggest missing requirements, flag design gaps)
- [ ] `SpecPipelinePanel.tsx` — 3-tab panel with linked navigation
- [ ] REPL: `/spec init|requirements|design|tasks|validate|status`
- [ ] Tests: 40+ unit tests

**Effort:** Medium (3-4 days)

## Phase 17: Agent-Per-Branch Workflow (P0)

**Why:** Cursor and v0 both create branches per agent task. This enables true parallel development — multiple agents working simultaneously without stepping on each other. It's also the natural pairing with Phase 15.2 (VM orchestration).

**Deliverables:**

- [ ] `branch_agent.rs` — Per-task branching:
  - On task start: `git checkout -b agent/<task-slug>-<short-id>`
  - Work in isolation (all edits on branch)
  - On completion: commit, push, create PR with AI-generated description
  - On failure: push partial work with WIP PR for human review
  - On conflict: detect, alert, suggest resolution or auto-rebase
- [ ] Parallel agent awareness: track all active agent branches, prevent duplicate work
- [ ] PR template: title, description, test plan, files changed summary, agent trace link
- [ ] Integration with vm_orchestrator.rs for isolated environments
- [ ] `BranchAgentPanel.tsx` — active agents, branches, PRs, conflict alerts
- [ ] REPL: `/branch agents|status|pr|rebase|cleanup`
- [ ] Tests: 35+ unit tests

**Effort:** Medium (2-3 days)

## Phase 18: Design & Voice Pipeline (P1)

**Why:** Bolt.new's Figma import, Replit's sketch-to-code, and Lovable's visual edits show demand for design-first workflows. Jules's audio summaries are a novel UX that no other tool offers. Both fill clear gaps in VibeCody's input/output modalities.

### 18.1 Design Import

**Deliverables:**

- [ ] `design_import.rs` — Design-to-code pipeline:
  - Figma API client (read-only): extract frames, components, styles, auto-layout
  - Image/sketch recognition: send wireframe image to vision provider, extract UI structure
  - SVG parsing: extract shapes, text, layout for component generation
  - Code generation: React/TypeScript components from extracted designs
  - Style extraction: colors, typography, spacing → CSS variables or Tailwind classes
- [ ] Supported inputs: Figma URL, PNG/JPG wireframe, SVG mockup, PDF design doc
- [ ] `DesignImportPanel.tsx` — drag-and-drop upload, preview generated components
- [ ] REPL: `/design figma <url>|sketch <path>|import <path>`
- [ ] Tests: 25+ unit tests

### 18.2 Audio Output / TTS

**Deliverables:**

- [ ] `audio_output.rs` — Text-to-speech synthesis:
  - Cloud TTS: Google Cloud TTS, AWS Polly, Azure Speech (configurable)
  - Local TTS: Piper TTS (open-source, runs offline for air-gapped)
  - Audio changelog: summarize recent git commits as spoken narration
  - PR narration: speak PR description, key changes, test results
  - Project status: spoken summary of open PRs, failing tests, active agents
  - Output formats: MP3, WAV, OGG
- [ ] Integration with voice.rs (input) for bidirectional voice interaction
- [ ] REPL: `/audio summary|changelog|narrate|status`
- [ ] Tests: 20+ unit tests

**Effort:** High (4-5 days total for 18.1 + 18.2)

## Phase 19: Enterprise Context & Collaboration (P1)

**Why:** Tabnine's Enterprise Context Engine and Augment's organizational understanding demonstrate enterprise demand for cross-repo intelligence. Amp's thread sharing and Lovable's real-time collab show agents need to be social, not solitary.

### 19.1 Org-Wide Context Engine

**Deliverables:**

- [ ] `org_context.rs` — Cross-repository organizational intelligence:
  - Multi-repo indexing: aggregate embeddings from configured repository list
  - Pattern detection: identify shared patterns, idioms, architecture decisions across org
  - Convention tracking: coding standards, naming conventions, framework preferences
  - Dependency awareness: cross-repo dependency graph, shared library versions
  - SQLite-backed with incremental indexing (only re-index changed files)
  - Configurable scope: org-wide, team-level, project-group
- [ ] `OrgContextPanel.tsx` — browse patterns, search across repos, view conventions
- [ ] REPL: `/org index|search|patterns|conventions|deps`
- [ ] Tests: 30+ unit tests

### 19.2 Agent Session Sharing

**Deliverables:**

- [ ] `session_sharing.rs` — Shareable agent sessions:
  - Export session as shareable JSON (tool calls, reasoning, file changes, timestamps)
  - Import shared sessions for review or continuation
  - Live session spectator mode (read-only WebSocket stream)
  - Session annotations: team members can comment on specific agent decisions
  - Privacy controls: redact sensitive data before sharing
- [ ] `SessionSharingPanel.tsx` — share/view/annotate/export sessions
- [ ] REPL: `/share session|export|view|annotate`
- [ ] Tests: 25+ unit tests

**Effort:** High (5-6 days total for 19.1 + 19.2)

## Phase 20: Extended Platform Features (P2)

**Why:** These features close gaps against specific competitors (Continue's CI gates, Lovable's data analysis, Google's Gemini) without being existential threats. Implementing them strengthens VibeCody's "most complete" positioning.

### 20.1 CI-Enforceable AI Checks

**Deliverables:**

- [ ] `ci_gates.rs` — AI review rules as CI gates:
  - Source-controlled rules in `.viberules/ci/` directory
  - Rule types: code quality, security, performance, style, test coverage thresholds
  - Exit codes: 0 (pass), 1 (fail/block merge), 2 (warn/allow merge)
  - Compatible with: GitHub Actions, GitLab CI, Jenkins, Azure Pipelines
  - CLI: `vibecli ci-check --rules .viberules/ci/ --diff HEAD~1`
  - Pre-built rule templates: OWASP top 10, performance regression, API breaking changes
- [ ] GitHub Action: `vibecody/ci-check@v1`
- [ ] Tests: 25+ unit tests

### 20.2 Data Analysis Mode

**Deliverables:**

- [ ] `data_analysis.rs` — AI-assisted data exploration:
  - Load datasets: CSV, Parquet, JSON, SQLite databases
  - Statistical summaries: distributions, correlations, outliers
  - Visualization specs: generate Vega-Lite/ECharts chart definitions
  - Dashboard definitions: composable chart layouts
  - Natural language queries: "Show me revenue by month" → chart spec
  - Export: PNG charts, HTML dashboards, Jupyter notebooks
- [ ] `DataAnalysisPanel.tsx` — upload data, preview charts, build dashboards
- [ ] REPL: `/data load|query|chart|dashboard|export`
- [ ] Tests: 25+ unit tests

### 20.3 Gemini Native Provider

**Deliverables:**

- [ ] `gemini.rs` in vibe-ai — Google Gemini provider:
  - Gemini 2.5 Pro, Gemini 2.5 Flash, Gemini 2.0 Flash
  - Streaming support with SSE parsing
  - Tool use / function calling support
  - Multimodal: image + text input
  - System instruction support
  - API key authentication (GEMINI_API_KEY)
  - Safety settings configuration
- [ ] Register in provider factory; add to VibeUI Keys panel
- [ ] Tests: 20+ unit tests

**Effort:** Medium-High (5-7 days total for 20.1-20.3)

## Phase 21: Managed Deploy (P2)

**Why:** Lovable, Bolt Cloud, and v0/Vercel all offer one-click deployment. VibeCody generates deploy configs but doesn't manage infrastructure. Closing this gap eliminates a key reason developers choose app builders over VibeCody.

**Deliverables:**

- [ ] `managed_deploy.rs` — One-click deployment manager:
  - Platform adapters: Vercel, Netlify, Fly.io, Railway, Render, self-hosted Docker
  - Build detection: auto-detect framework (Next.js, Vite, Remix, etc.) and configure build
  - Environment variables: sync from `.env` or config to deploy platform
  - Domain management: custom domain configuration, SSL provisioning
  - Deploy preview: per-PR preview deployments
  - Rollback: one-click rollback to previous deploy
- [ ] `deploy_monitor.rs` — Post-deploy monitoring:
  - Health check polling
  - Error rate tracking (if platform provides)
  - Deploy history with timestamps and commit refs
  - Auto-rollback on error threshold
- [ ] `DeployPanel.tsx` (enhanced) — deploy target, domains, monitoring, rollback
- [ ] REPL: `/deploy vercel|netlify|fly|railway|docker|status|rollback|preview`
- [ ] Tests: 30+ unit tests

**Effort:** High (4-5 days)

## Phase 22: Future Architecture (P3)

**Why:** These are architectural investments that won't pay off immediately but position VibeCody for the 2027 landscape where 100M-token context, cross-surface agents, and model marketplaces become standard.

### 22.1 Massive Context Architecture

**Deliverables:**

- [ ] `context_streaming.rs` — Foundation for 10M-100M token windows:
  - Hierarchical summarization: multi-level abstractions (file→module→package→project)
  - Sliding window with retrieval: keep recent context live, retrieve older via embeddings
  - Priority-based eviction: score context relevance, evict lowest-value segments
  - On-demand expansion: zoom into detail for specific code regions
  - Extends infinite_context.rs with streaming architecture
- [ ] Tests: 20+ unit tests

### 22.2 VS Code Extension Compatibility

**Deliverables:**

- [ ] `extension_compat.rs` — VS Code extension compatibility layer:
  - Load TextMate grammar bundles (.tmLanguage, .tmTheme) for syntax highlighting
  - Load snippet definitions (.code-snippets)
  - Load language configuration (brackets, comments, auto-closing pairs)
  - NOT full VS Code API compat — targeted at high-value, low-complexity categories
  - Extension registry: browse VS Code marketplace, identify compatible extensions
- [ ] Tests: 20+ unit tests

### 22.3 Model Provider Marketplace

**Deliverables:**

- [ ] `model_marketplace.rs` — Model comparison and discovery:
  - Model registry: name, provider, capabilities, pricing, benchmarks, context window
  - Pre-loaded with 50+ models across all 17 providers + OpenRouter
  - Search/filter: by capability (code gen, chat, vision), price, speed, quality score
  - One-click configure: select model → auto-update config.toml
  - Community ratings from arena mode results
  - Price calculator: estimate monthly cost based on usage pattern
- [ ] `ModelMarketplacePanel.tsx` — browse, compare, configure
- [ ] REPL: `/models search|compare|configure|pricing`
- [ ] Tests: 25+ unit tests

### 22.4 Cross-Surface Agent Routing

**Deliverables:**

- [ ] `cross_surface_routing.rs` — Multi-surface task routing:
  - Task handoff protocol: CLI ↔ IDE ↔ Cloud ↔ Mobile
  - Session sync: start on mobile (via channel daemon), work executes in cloud VM, results appear in IDE
  - Surface capabilities: each surface advertises what it can do (edit, terminal, browser, etc.)
  - Routing logic: auto-select best surface based on task requirements
  - Requires: channel_daemon.rs (Phase 15) + vm_orchestrator.rs (Phase 15) + remote_control.rs
- [ ] Tests: 20+ unit tests

### 22.5 Agentic CI/CD Pipeline

**Deliverables:**

- [ ] `agentic_cicd.rs` — AI agents as CI/CD participants:
  - Auto-fix failing builds: agent analyzes CI failure, creates fix PR
  - Missing test generation: detect uncovered code in PR, generate tests
  - Dependency updates: auto-create PRs for outdated dependencies with compatibility check
  - Merge conflict resolution: AI-assisted conflict resolution in parallel branches
  - Pipeline optimization: suggest CI/CD speed improvements
  - Triggered by ci_gates.rs (Phase 20) failures or GitHub Actions webhooks
- [ ] Tests: 25+ unit tests

**Effort:** Very High (8-12 days total for 22.1-22.5)

## Priority & Timeline Summary

| Phase | Feature | Priority | Gaps Covered | Estimated Effort |
|-------|---------|----------|-------------|-----------------|
| 15 | Always-On Agent Infrastructure | P0 | #1, #2 | 5-7 days |
| 16 | Spec-Driven Development | P0 | #3 | 3-4 days |
| 17 | Agent-Per-Branch Workflow | P0 | #4 | 2-3 days |
| 18 | Design & Voice Pipeline | P1 | #5, #6 | 4-5 days |
| 19 | Enterprise Context & Collab | P1 | #7, #8 | 5-6 days |
| 20 | Extended Platform Features | P2 | #12, #11, #14 | 5-7 days |
| 21 | Managed Deploy | P2 | #10 | 4-5 days |
| 22 | Future Architecture | P3 | #15-#19 | 8-12 days |

**P0 total:** ~10-14 days (4 gaps, 3 phases)
**P1 total:** ~9-11 days (4 gaps, 2 phases)
**P2 total:** ~9-12 days (4 gaps + Gemini, 2 phases)
**P3 total:** ~8-12 days (5 gaps, 1 phase)
**Grand total:** ~36-49 days

### Gaps Not Addressed in This Roadmap

| # | Gap | Reason |
|---|-----|--------|
| 9 | Visual design canvas | Very high effort, low differentiation vs Figma/native design tools |
| 13 | Sketch-to-3D generation | Novel but niche; wait for ecosystem maturity |
| 16 | VS Code extension compat (partial) | Phase 22.2 covers high-value subset |

## Success Criteria

After completing Phases 15-22, VibeCody will:

1. **Always-on** — Daemon mode with channel listeners, matching Claude Code Channels and Cursor Automations
2. **Spec-driven** — Structured EARS pipeline, matching Kiro and Augment Intent
3. **Parallel agents** — Agent-per-branch with VM isolation, matching Cursor Background Agents
4. **Design-aware** — Figma/sketch import, matching Bolt.new and Replit
5. **Voice I/O** — Bidirectional voice (STT + TTS), matching and exceeding Jules
6. **Org-intelligent** — Cross-repo context engine, matching Tabnine Enterprise
7. **Social** — Session sharing for team collaboration
8. **Deployable** — One-click deploy to 6+ platforms, matching Lovable/Bolt/v0
9. **CI-native** — AI checks as CI gates, matching Continue
10. **18-provider** — Gemini added as 18th native provider

## Competitive Position After v4

With all v4 roadmap phases complete, VibeCody would be the **only tool** combining:

- Open-source + self-hostable + 18 AI providers
- CLI + Desktop IDE (3 surfaces with browser mode)
- Always-on channel daemon (18 platform gateways → automation triggers)
- Spec-driven development (EARS pipeline)
- Agent-per-branch with VM isolation
- Design-to-code (Figma + sketch + SVG)
- Bidirectional voice (STT + TTS audio summaries)
- Org-wide cross-repo context engine
- Agent session sharing
- One-click managed deployment (6+ platforms)
- CI-enforceable AI gates
- 539+ domain skills across 25+ industries
- Batch generation (3M+ lines) + multi-QA validation
- Red/Blue/Purple team security pipeline
- 162+ tool panels + SWE-bench benchmarking
- Air-gapped deployment with Ollama

No competitor currently offers more than 6-7 of these in combination.
