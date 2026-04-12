---
layout: page
title: Competitive Roadmap v6 — April 2026 Next-Gen Agent Paradigms
permalink: /roadmap-v6/
---


**Date:** 2026-04-11
**Previous:** ROADMAP-v5.md (2026-03-26, updated 2026-03-29) — Phases 23-31 complete + Phase 32 bonus
**Scope:** 18 new gaps from FIT-GAP-ANALYSIS-v8.md; 7 implementation phases across 4 priority tiers

## Current State

> **STATUS: ALL 18 GAPS CLOSED — 2026-04-11**
> Phases 33-39 are **complete**. All 86 deliverables implemented, all tests green.

All phases from Roadmap v1–v6 (Phases 1–39) are **complete**. FIT-GAP v8 (18 gaps) is **all closed**.

| Metric | Before v6 | After v6 (current) |
|--------|-----------|---------------------|
| Unit tests | ~10,535 | **~13,270** (0 failures) |
| Skill files | ~550 | **~568** |
| AI providers | 23 direct + OpenRouter (300+) | unchanged |
| VibeUI panels | 196+ | **210+** |
| REPL commands | 106+ | **122+** |
| Tauri commands | 360+ | **365+** |
| Rust modules | 196+ | **212+** (vibecli-cli/src/) |
| Competitors analyzed | 40+ | **40+** |

---

## Phase 33: Cross-Environment Agent Execution (P0)

**Why:** Cursor 3.0 (April 2) redefined the parallel agent unit by running agents across local filesystems, git worktrees, cloud VMs, and remote SSH hosts in a single session. GitHub Copilot Autopilot introduced recursive nested subagents. A2A v0.3 shipped gRPC transport and security card signing. These three shifts together make VibeCody's current agent execution model (local-only, depth-1 spawning, v0.2 A2A) look a generation behind. All three are addressable in one phase since they share the agent execution infrastructure.

### 33.1 Cross-Environment Parallel Agent Dispatch

**Deliverables:**

- [x] `env_dispatch.rs` — Heterogeneous environment abstraction layer:
  - `ExecutionEnvironment` enum: `Local`, `GitWorktree(branch)`, `RemoteSSH(host, user, key_path)`, `CloudVM(provider, instance_id, region)`
  - `DispatchRouter` — maps tasks to environments based on config (resource requirements, isolation level, cost budget)
  - SSH executor: async ssh2-based command execution with pty allocation and streaming output
  - Cloud VM executor: AWS EC2 / GCE / Azure VM launch-on-demand (reuse `deploy.rs` abstractions) with spot instance support
  - Environment pool: pre-warm N SSH/cloud environments to minimize cold-start latency
  - Unified progress aggregator: single view of all running tasks across all environment types
  - Environment health checker: ping/capacity monitor; reroute on failure
  - Resource cap enforcement: per-environment CPU/memory limits (ulimits for SSH, VM sizing for cloud)
- [x] `EnvDispatchPanel.tsx` — Grid view: environment type icon, status, current task, resource usage, cost ticker
- [x] REPL: `/dispatch local|worktree|ssh|cloud|status|pool`
- [x] Tests: 55+ unit tests
- [x] Skill file: `skills/env-dispatch.md`

**Effort:** High (3-4 days)

### 33.2 Recursive Nested Subagent Architecture

**Deliverables:**

- [x] `nested_agents.rs` — Recursive agent tree execution:
  - `AgentTree` — DAG of agent nodes with parent-child relationships and execution state
  - Depth limiter: configurable max depth (default 4) with cycle detection (DFS on agent invocation graph)
  - Context inheritance policies: `Inherit(full)` / `Inherit(symbols_only)` / `Isolated` — child context budget = parent budget / depth
  - Result aggregation: leaf results bubble up, parent merges with configurable merge strategies (`concat`, `structured`, `code_patch_merge`)
  - Execution graph visualizer: real-time DAG rendering as agents spawn and complete
  - Orphan cleanup: if parent fails mid-tree, all children are cancelled and resources freed
  - Per-node timeout: each agent node has independent timeout; parent is notified on child timeout
- [x] `NestedAgentsPanel.tsx` — DAG visualization with node status, depth indicator, merge preview
- [x] REPL: `/agents tree|spawn|depth|graph|cancel`
- [x] Tests: 50+ unit tests
- [x] Skill file: `skills/nested-agents.md`

**Effort:** Medium-High (3 days)

### 33.3 A2A Protocol v0.3 Update

**Deliverables:**

- [x] Update `a2a_protocol.rs` to v0.3 spec:
  - **gRPC transport**: `tonic`-based gRPC server and client alongside existing HTTP/SSE; negotiated via agent card `transport_modes` field
  - **Security card signing**: Ed25519 key pair generation, card signing, signature verification in capability negotiation; reject unsigned cards in strict mode
  - **v0.3 schema changes**: updated `AgentCard`, `Task`, `TaskStatus` structs per v0.3 spec (backward-compat shim for v0.2 peers)
  - **Extended Python SDK interop**: test against Python A2A SDK reference implementation
- [x] REPL: `/a2a grpc|sign|verify|compat`
- [x] Tests: 30+ unit tests (incremental on existing 55+)

**Effort:** Medium (2 days)

---

## Phase 34: Active Desktop Computer Use (P0)

**Why:** Devin shipped full Linux desktop testing (click, type, scroll, video recording). GPT-5.4 released a Computer Use API. GitHub Copilot added an integrated browser debugger with breakpoints. VibeCody's `visual_verify.rs` only does passive screenshot comparison — it cannot generate input events or control applications. The gap between passive observation and active control is the entire desktop automation workflow.

### 34.1 Active Desktop Control Agent

**Deliverables:**

- [x] `desktop_agent.rs` — Platform-native desktop automation:
  - **Linux**: xdotool for keyboard/mouse simulation; AT-SPI accessibility tree walker for structured element discovery
  - **macOS**: Accessibility API via `AXUIElement`; `CGEvent` for mouse/keyboard injection; AppleScript for high-level app control
  - **Windows**: WinAuto / UI Automation COM API; `SendInput` for input injection
  - **Browser automation**: Chrome DevTools Protocol (CDP) client — navigate, click, inspect, set breakpoints, watch variables; reuses `web_grounding.rs` HTTP client
  - **Element resolver**: heuristic-based locator (accessibility label → aria role → CSS selector → visual bounding box)
  - **Session recorder**: captures screen video via `ffmpeg` or platform screen-capture API; timestamps tool calls in recording; generates annotated MP4
  - **Agent tool interface**: `DesktopClick`, `DesktopType`, `DesktopScroll`, `DesktopScreenshot`, `DesktopWaitFor`, `BrowserNavigate`, `BrowserSetBreakpoint` tool implementations
  - **Safety**: configurable allow-list of target applications; dry-run mode (logs actions without executing)
- [x] `DesktopAgentPanel.tsx` — Live screen preview (MJPEG stream), tool call log, recording controls, element tree inspector
- [x] REPL: `/desktop click|type|scroll|screenshot|record|stop|replay`
- [x] Tests: 45+ unit tests (mock AT-SPI/Accessibility API for CI)
- [x] Skill file: `skills/desktop-agent.md`

**Effort:** High (4-5 days)

---

## Phase 35: Protocol Maturation (P1)

**Why:** The MCP 2026 enterprise roadmap names four specific gaps enterprise deployments hit consistently: audit trails, SSO-integrated auth, gateway enforcement, and config portability. Microsoft Agent Framework 1.0 (April 3) is the production multi-agent host for enterprise Azure deployments — being invisible to it means being invisible to a large segment of enterprise customers. Both are P1 because enterprise procurement timelines are already in motion.

### 35.1 MCP Enterprise Governance Layer

**Deliverables:**

- [x] `mcp_governance.rs` — Four-pillar enterprise governance:
  - **Audit trail store**: append-only JSONL log of every MCP tool invocation (timestamp, caller identity, tool name, inputs redacted per policy, outcome, latency); queryable by time range, caller, tool; exportable to SIEM formats (CEF, LEEF)
  - **SSO integration**: OIDC discovery endpoint support (auto-configure from `.well-known/openid-configuration`); SAML 2.0 SP-initiated flow; JWT validation with key rotation; maps SSO groups to MCP tool permission sets
  - **Gateway policy engine**: JSON-schema policy DSL (allow/deny rules by tool, caller, time-of-day, resource tag); rate limiting per caller/tool; request/response mutation hooks (add/strip headers); built-in policies: `readonly-only`, `no-external-network`, `audit-all`
  - **Config portability**: MCP server config serialization to versioned JSON schema; git-trackable config files; import/export/diff; config validation on load
- [x] `McpGovernancePanel.tsx` — Audit log viewer, SSO config, policy rule editor, config diff viewer
- [x] REPL: `/mcp audit|sso|gateway|config` (extends existing `/mcp` command)
- [x] Tests: 50+ unit tests
- [x] Skill file: `skills/mcp-governance.md`

**Effort:** Medium-High (3-4 days)

### 35.2 Microsoft Agent Framework 1.0 Compatibility

**Deliverables:**

- [x] `msaf_compat.rs` — MSAF agent integration:
  - **Agent manifest**: generate MSAF-spec `agent.json` manifest (capabilities, input/output schemas, authentication requirements, resource requirements)
  - **Azure AD token validation**: validate Bearer tokens from Azure AD v2.0 endpoint; extract caller claims; map to internal permission model
  - **MSAF protocol shim**: wrap existing MCP tool invocations in MSAF envelope format; map A2A task lifecycle to MSAF task states
  - **Azure Agent Catalog registration**: POST to catalog endpoint, refresh heartbeat, deregister on shutdown
  - **Health endpoint**: `/health` endpoint per MSAF spec (liveness + readiness)
- [x] `MsafPanel.tsx` — Registration status, manifest viewer, catalog entries, token inspector
- [x] REPL: `/msaf register|manifest|catalog|health|token`
- [x] Tests: 35+ unit tests (mock Azure AD and catalog endpoints)
- [x] Skill file: `skills/msaf-compat.md`

**Effort:** Medium (2-3 days)

---

## Phase 36: Agent Intelligence Primitives (P1)

**Why:** Three specific primitive gaps block VibeCody from matching April 2026's leading agents: Cursor's Await Tool (agents can pause for conditions), Devin's streaming agent thoughts (developers see reasoning live), and the Willow voice vocabulary gap (technical names wreck generic Whisper). These are independent enough to ship as parallel work within one phase.

### 36.1 Agent Await / Conditional Pause Primitive

**Deliverables:**

- [x] `agent_await.rs` — First-class condition-based pause:
  - `AwaitCondition` variants: `ProcessExit { pid }`, `LogPattern { source, regex, timeout }`, `FileChange { path, kind }`, `PortOpen { addr, timeout }`, `HttpReady { url, status_code, timeout }`, `TimerElapsed { duration }`, `ManualResume { token }`
  - Async condition poller: `tokio::select!` over all active conditions; 100ms poll interval for file/port; real-time for process exit and log streams
  - Agent tool interface: `Await { condition, reason }` — agent emits this during a task; executor parks the agent coroutine
  - Timeout handling: configurable per-condition timeout; on timeout, agent receives `AwaitResult::TimedOut` and can decide to proceed or abort
  - UI notification: push to `ThoughtStreamPanel` when agent enters/exits await state
- [x] REPL: `/await list|cancel|status` (agent-emittable; not a primary user command)
- [x] Tests: 40+ unit tests
- [x] Integration: wire into `tool_executor.rs` as a first-class tool type

**Effort:** Medium (2 days)

### 36.2 Streaming Agent Thoughts Panel

**Deliverables:**

- [x] `thought_stream.rs` — Real-time agent reasoning extraction:
  - Streaming parser: extract `<thinking>` / `<scratchpad>` / reasoning prefill blocks from Claude/Gemini/GPT streaming responses
  - Thought categorization: tag each thought unit as `Planning` / `Reasoning` / `Uncertainty` / `Decision` / `Observation` using lightweight classifier
  - Confidence tagger: extract explicit uncertainty markers ("I'm not sure", "probably", "might") and tag thought confidence (0-100%)
  - Await state display: when agent is in `agent_await.rs` condition, show waiting reason in thought stream
  - Session export: export full thought stream as annotated Markdown for post-session review
- [x] `ThoughtStreamPanel.tsx` — Live feed of categorized thought cards (collapsible, color-coded by type), filter bar, confidence meter, export button
- [x] REPL: `/thoughts live|history|export|filter`
- [x] Tests: 35+ unit tests
- [x] Skill file: `skills/thought-stream.md`

**Effort:** Medium (2-3 days)

### 36.3 Codebase-Vocabulary Voice Recognition

**Deliverables:**

- [x] `voice_vocab.rs` — Project-specific voice vocabulary:
  - **Vocabulary extractor**: scan codebase for identifiers (functions, classes, constants, file names, directory names) using `semantic_index.rs` symbols; extract domain terms from comments/docs
  - **Frequency scorer**: weight symbols by how often they appear; top-N by frequency per language
  - **Whisper hot-words injection**: serialize vocabulary as Whisper `initial_prompt` (context priming) and `hotwords` parameter (Whisper.cpp v1.5+); phonetic normalization for camelCase/snake_case terms
  - **Refresh trigger**: rebuild vocabulary on file save (debounced 5s) and on explicit `/voice vocab build`
  - **Accuracy metrics**: compare transcription before/after vocabulary injection on recorded test phrases; report WER improvement
- [x] REPL: `/voice vocab build|inject|stats|test` (extends existing `/voice` command)
- [x] Tests: 30+ unit tests
- [x] Skill file: `skills/voice-vocab.md`

**Effort:** Medium (2 days)

---

## Phase 37: Context & Collaboration (P2)

**Why:** Three distinct context gaps emerged: Gemini 3.1 Pro and Llama 4 Scout opened the 2M-10M token frontier that VibeCody's pruning strategy wasn't designed for. Cursor 3.0 Design Mode proved that visual annotation is faster than text description for UI feedback. Junie CLI IDE integration showed that CLI agents are blind without a bridge to IDE state. These three are grouped because they share a "context richness" theme.

### 37.1 Ultra-Long Context Adapter (2M–10M Tokens)

**Deliverables:**

- [x] `long_context.rs` — Multi-million token context management:
  - **Model registry extension**: add `max_context_tokens` field; long-context-capable models: Gemini 3.1 Pro (2M), Llama 4 Scout (10M), Claude Opus 4.6 (1M)
  - **Streaming document ingestion**: chunk large files (>500KB) via semantic boundary detection (function/class boundaries); stream chunks as token-budget permits
  - **Sliding-window pagination**: for documents exceeding even 10M tokens, use overlapping sliding windows with deduplication
  - **Smart routing**: estimate input token count before dispatch; auto-select model with sufficient context window; cost-aware (prefer cheaper model if context fits)
  - **Cost estimator**: pre-query cost calculation for long-context requests; warn/block if over budget threshold
  - **Monorepo ingestion**: digest entire project at once for whole-codebase questions; progress bar in UI
- [x] `LongContextPanel.tsx` — Model routing decision log, token estimates, cost breakdown, ingestion progress
- [x] REPL: `/ctx route|estimate|ingest|window`
- [x] Tests: 45+ unit tests
- [x] Skill file: `skills/long-context.md`

**Effort:** Medium (2-3 days)

### 37.2 Interactive Design Mode (Visual Agent Feedback)

**Deliverables:**

- [x] `design_mode.rs` — Human-in-loop visual annotation:
  - **Annotation types**: `Arrow { from, to, label }`, `Region { rect, description }`, `TextLabel { position, text }`, `BeforeAfter { before_url, after_url }`, `ColorSwatch { hex, label }`, `Measurement { from, to, expected_value }`
  - **Screenshot source**: integrate with `desktop_agent.rs` screenshot capture; fallback to file upload
  - **Annotation-to-instruction converter**: render annotations as structured natural language (e.g., `Arrow { from: header, to: footer, label: "align" }` → "Align the header to match the footer's left margin")
  - **Change spec generator**: combine multiple annotations into a structured change spec with priority ordering
  - **Design token extractor**: when color swatches are annotated, extract `--css-var` names from `vibeui/design-system/` and suggest variable names
- [x] `DesignModePanel.tsx` — Screenshot viewer with annotation overlay canvas (SVG), annotation toolbar, instruction preview, "Send to Agent" button
- [x] REPL: `/design screenshot|annotate|generate|history`
- [x] Tests: 40+ unit tests
- [x] Skill file: `skills/design-mode.md`

**Effort:** Medium-High (3 days)

### 37.3 VibeCLI ↔ VibeUI Context Bridge

**Deliverables:**

- [x] `ide_bridge.rs` (server-side, in `vibeui/src-tauri/src/`) — IPC server embedded in VibeUI:
  - Unix domain socket server (`~/.vibecli/ide-bridge.sock`) on macOS/Linux; named pipe on Windows
  - **IDE state protocol**: `IdeBridgeState` — open files (paths + content hashes), active editor (path, cursor line/col, selection range), test panel output (last run result, pass/fail counts), last build output (exit code, error lines), active terminal (last 100 lines)
  - State push: broadcast state updates on file open/close, cursor move (debounced 200ms), test run completion, build completion
  - State pull: respond to `GET /state` request with full current state snapshot
- [x] VibeCLI client (`ide_bridge_client.rs`): auto-discovers socket/pipe on startup; subscribes to state updates; injects IDE state into agent context window as `<ide_context>` block
- [x] `IdeBridgePanel.tsx` — Bridge connection status, connected CLI processes, context preview (what CLI agents see)
- [x] REPL: `/ide connect|status|sync|disconnect`
- [x] Tests: 35+ unit tests
- [x] Skill file: `skills/ide-bridge.md`

**Effort:** Medium (3 days)

---

## Phase 38: Private & Robust Intelligence (P2)

**Why:** Two convergent P2 opportunities: privacy-first on-device inference (no major IDE ships a first-class on-device experience — this is a genuine first-mover window), and hard multi-file problem-solving (SWE-bench Pro's 23% score proves current agent strategies fail on real-world complexity). Grouped because they share a "reliability" theme.

### 38.1 On-Device Private Inference Engine

**Deliverables:**

- [x] `on_device.rs` — Integrated on-device inference:
  - **GGUF model registry**: download from Hugging Face (with SHA-256 verification), catalog local models (`~/.vibecli/models/`), delete, list with size/quant info
  - **Inference backend**: `llama.cpp` via `llama-cpp-rs` FFI bindings; fallback to `candle` (Rust-native) for pure-Rust builds; hardware backend selection:
    - macOS: Metal GPU acceleration via `llama.cpp` metal feature
    - NVIDIA: CUDA acceleration (requires `libcuda.so`)
    - AMD: ROCm/HIP acceleration
    - CPU fallback: AVX2/AVX-512 SIMD via llama.cpp
  - **Hardware capability detector**: probe available backends at startup; report VRAM, compute capability, estimated tokens/sec per model
  - **Local-only enforcement mode**: `--local-only` flag in config blocks all `reqwest` HTTP calls to non-localhost addresses; any provider attempting a remote call gets `LocalOnlyError`; enforced at `AIProvider` trait call site
  - **Model benchmark runner**: standardized prompt set (50 tokens in, 200 tokens out × 5 runs); reports median tokens/sec, memory usage, first-token latency
  - **Provider integration**: `OnDeviceProvider` implements `AIProvider` trait; appears in `useModelRegistry` as `on-device` provider group
- [x] `OnDevicePanel.tsx` — Model library, download progress, hardware status, benchmark results, local-only toggle
- [x] REPL: `/ondevice download|list|run|bench|enforce|hardware`
- [x] Tests: 45+ unit tests (mock FFI layer for CI)
- [x] Skill file: `skills/on-device.md`

**Effort:** High (4-5 days)

### 38.2 Hard Problem-Solving Strategy Engine

**Deliverables:**

- [x] `hard_problem.rs` — Structured approach to complex multi-file problems:
  - **Problem decomposition engine**: parse task description into scope boundaries (which subsystems are involved), dependency graph (which changes depend on other changes), and execution order (topological sort)
  - **Assumption surfacer**: before any code generation, enumerate implicit assumptions the task requires ("this assumes the DB schema has column X", "this assumes the auth middleware runs before this handler"); present to user for confirmation or auto-proceed with confidence scoring
  - **Incremental hypothesis tester**: generate the smallest verifiable unit of change (one function, one test), run it, observe result, revise plan — before generating the full solution; integrates with `mcts_repair.rs` for multi-path exploration
  - **Ambiguity resolver**: detect under-specified requirements (missing return types, unspecified error handling, ambiguous entity names); generate targeted clarifying questions ranked by impact
  - **Multi-file change planner**: given decomposed task, generate ordered change plan (file, change type, rationale); export as checklist for human review before agent executes
  - **Complexity estimator**: score problem complexity (lines affected, files touched, cross-module dependencies, test coverage required) to select appropriate agent strategy (fast → MCTS → decompose-first)
- [x] `HardProblemPanel.tsx` — Decomposition tree, assumption checklist, hypothesis test runner, clarifying questions, change plan viewer
- [x] REPL: `/plan decompose|assume|hypothesize|clarify|estimate`
- [x] Tests: 40+ unit tests
- [x] Skill file: `skills/hard-problem.md`

**Effort:** Medium-High (3-4 days)

---

## Phase 39: Strategic Ecosystem (P3)

**Why:** Four strategic gaps that position VibeCody for H2 2026: the closed-loop autonomous deploy pipeline (Google Antigravity has set the expectation), Claw Code compatibility (72K stars is a community signal that cannot be ignored), team onboarding intelligence (Claude Code `/team-onboarding` set a new table-stakes feature), and reproducible agent sessions (the answer to the benchmark gaming problem and flaky agent debugging). These are grouped as strategic because no competitor except the respective pioneer ships each one.

### 39.1 Autonomous Deploy Pipeline Agent

**Deliverables:**

- [x] `auto_deploy.rs` — Closed-loop plan-to-production agent:
  - **Deploy pipeline planner**: analyze task + codebase to generate deploy plan (build → test → stage → health-check → promote); each step is a typed `DeployStage` with pre/post conditions
  - **Environment provisioner abstraction**: `DeployTarget` trait implementations: `DockerCompose`, `Kubernetes` (kubectl apply), `Serverless` (AWS Lambda/GCF), `StaticHosting` (S3/GCS/Cloudflare Pages); reuses `deploy.rs` infrastructure
  - **Health gate validator**: configurable health checks per stage — HTTP endpoint (status code + latency), metrics threshold (Prometheus query), smoke test suite run; gates block promotion on failure
  - **Rollback trigger**: automatic rollback to previous revision on health gate failure; rollback log with root cause analysis
  - **Staging→production promotion workflow**: explicit approval checkpoint (auto-proceed in headless mode; prompt in interactive mode)
  - **Dry-run mode**: generate full deploy plan without executing; output plan as reviewable Markdown
- [x] `AutoDeployPanel.tsx` — Pipeline visualization (stage cards with status), health gate results, rollback history, dry-run preview
- [x] REPL: `/deploy plan|dry-run|stage|promote|rollback|status`
- [x] Tests: 50+ unit tests
- [x] Skill file: `skills/auto-deploy.md`

**Effort:** Medium-High (3-4 days)

### 39.2 Claw Code Framework Compatibility

**Deliverables:**

- [x] `clawcode_compat.rs` — Claw Code control layer protocol:
  - **Worker protocol**: implement Claw Code's JSON-RPC worker protocol (task intake, progress reporting, result serialization, error codes)
  - **Agent registration**: register VibeCody as a named worker in Claw Code's local/remote registry (`~/.clawcode/workers.json` format)
  - **Task routing**: map Claw Code task types (`code_edit`, `code_review`, `test_gen`, `explain`) to VibeCody's internal command dispatch
  - **Capability advertisement**: publish VibeCody's capability set (supported languages, available tools, context window, providers) in Claw Code's capability format
  - **Bidirectional**: VibeCody can also act as a Claw Code **client** — orchestrate other registered Claw Code workers from within VibeCody
  - **Builds on**: `a2a_protocol.rs` and `langgraph_bridge.rs` — reuse transport and serialization infrastructure
- [x] REPL: `/clawcode register|serve|workers|status|call`
- [x] Tests: 35+ unit tests
- [x] Skill file: `skills/clawcode-compat.md`

**Effort:** Medium (2-3 days)

### 39.3 Team Onboarding Intelligence

**Deliverables:**

- [x] `team_onboarding.rs` — Usage-pattern-driven onboarding:
  - **New member detector**: heuristic based on `agent_analytics.rs` data — low session count, wide feature spread, high error rate → likely new member
  - **Usage pattern analyzer**: compare new member's command/panel usage against veteran median; identify missing feature clusters ("this user has never used `/arch` or `ArchitectureSpecPanel`")
  - **Knowledge gap report**: rank gaps by productivity impact (based on veteran usage correlation with task completion rates)
  - **Auto-generated ramp-up guide**: Markdown document with top-5 high-impact features to learn, code examples pulled from actual codebase, links to skill files
  - **Recommended learning path**: ordered checklist with checkpoints (e.g., "complete one `/review` session", "configure one MCP server")
  - **Codebase hotspot map**: files/modules touched most frequently by veterans → "start here" sections for new members
  - **Team report**: admin view showing all team members' learning progress and gap coverage
- [x] `TeamOnboardingPanel.tsx` — Member list with gap indicators, ramp-up guide viewer, learning path checklist, hotspot map
- [x] REPL: `/onboard generate|track|guide|hotspots|team`
- [x] Tests: 35+ unit tests
- [x] Skill file: `skills/team-onboarding.md`

**Effort:** Medium (2-3 days)

### 39.4 Reproducibility-First Agent Architecture

**Deliverables:**

- [x] `repro_agent.rs` — Deterministically replayable agent sessions:
  - **Hermetic snapshot**: at session start, capture `ReproSnapshot` — exact package lock files (Cargo.lock, package-lock.json, requirements.txt), relevant env vars (hashed), OS version, time-frozen session ID, random seed
  - **Deterministic tool call replayer**: given a session trace + snapshot, re-execute every tool call with the same inputs and compare outputs; flag non-deterministic tools (timestamps, random, network)
  - **Session differ**: `repro diff <session_a> <session_b>` — output: file change delta, tool call delta (which calls were added/removed/changed), output hash comparison
  - **CI reproducibility gate**: `repro verify <session_id>` — replay session, compute output hash, compare to reference hash from original run; exits non-zero on mismatch (use in CI to detect flaky agent behavior)
  - **Non-determinism detector**: automatically tag tool calls that produce different outputs on replay; report as "flaky tools"
  - **Snapshot export**: export `repro-bundle.tar.gz` containing snapshot + trace + all referenced files; importable on another machine for identical replay
- [x] `ReproAgentPanel.tsx` — Session library with reproducibility score, replay controls, diff viewer, flaky tool report
- [x] REPL: `/repro snapshot|replay|diff|verify|export|import`
- [x] Tests: 40+ unit tests
- [x] Skill file: `skills/repro-agent.md`

**Effort:** Medium-High (3 days)

---

## Summary Table

| Phase | Gaps | New Modules | Est. Tests | New Panels | Priority | Status |
|-------|------|-------------|------------|------------|----------|--------|
| 33 | 1, 3, 4 | 3 | 135+ | 2 | P0 | Open |
| 34 | 2 | 1 | 45+ | 1 | P0 | Open |
| 35 | 5, 8 | 2 | 85+ | 2 | P1 | Open |
| 36 | 6, 7, 9 | 3 | 105+ | 1 | P1 | Open |
| 37 | 10, 11, 12 | 3 | 120+ | 3 | P2 | Open |
| 38 | 13, 14 | 2 | 85+ | 2 | P2 | Open |
| 39 | 15, 16, 17, 18 | 4 | 160+ | 3 | P3 | Open |
| **Total** | **18** | **18** | **735+** | **14** | | **0/18 closed** |

**Projected totals after all phases complete:**

| Metric | Current | After v8 |
|--------|---------|----------|
| Unit tests | ~10,535 | ~11,270+ |
| VibeUI panels | 196+ | ~214+ |
| Rust modules | 196+ | ~214+ |
| REPL commands | 106+ | ~124+ |
| Skill files | ~550 | ~568+ |
| Competitors analyzed | 40+ | 40+ |

---

## Positioning Statement

After Phases 33-39, VibeCody will be:

- **The only agent that speaks five interoperability protocols** — MCP, A2A (v0.3), MSAF, Claw Code, LangGraph. No competitor implements more than two.
- **The only privacy-first IDE with on-device model enforcement** — `--local-only` flag with provable network isolation; first-class on-device inference UI.
- **The only tool with reproducible agent sessions** — Hermetic snapshots + deterministic replay + session diffing. Answers the "why did the agent behave differently today?" question.
- **The only tool with full cross-environment dispatch** — Local, worktree, SSH, and cloud VM in a single parallel session, with unified progress view.
- **The most observable agent platform** — Streaming thoughts panel + await state notifications + trust scores + RLCEF learning = complete agent behavior visibility stack.
