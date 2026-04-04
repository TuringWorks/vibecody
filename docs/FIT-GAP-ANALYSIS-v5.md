---
layout: page
title: Fit-Gap Analysis v5 — Futureproofing 2026-2027
permalink: /fit-gap-analysis-v5/
---


**Date:** 2026-03-12
**Previous analysis:** FIT-GAP-ANALYSIS-v4 (removed, 2026-03-08)
**Focus:** Q1 2026 emerging trends, new entrants, and futureproofing gaps

## Executive Summary

The AI coding assistant market is entering a new phase in March 2026. Key shifts since v4:

1. **MCP ecosystem explosion** — $1.8B ecosystem; Claude Code v2.1.74 introduced lazy loading (95% context reduction); 30+ MCP partner plugins in Cursor
2. **Agent Client Protocol (ACP)** — Zed and others pushing standardized inter-agent communication
3. **Enterprise governance hardening** — Shadow AI controls, SOC 2 Type II, ISO 42001 becoming table-stakes for enterprise sales
4. **New entrants gaining traction** — Replit Agent 3, Amazon Q Developer, Qodo, Roo Code, Cline, Zed agentic editing, Visual Studio 2026 AI-native
5. **Market scale** — 57% of companies running AI agents in production; $8.5B autonomous agent market; SWE-bench top scores at 80.9% (Claude Opus 4.5)
6. **Copilot expanding** — Spaces (curated context bundles), JetBrains GA with agent hooks, agentic code review GA (March 5, 2026)

VibeCody's v4 gaps are **all closed** (23/23). This v5 identifies **12 new futureproofing gaps** to maintain competitive leadership through 2026-2027.

**New competitors added:** Replit Agent 3, Amazon Q Developer, Qodo, Roo Code, Cline, Zed

## Part A — New Competitor Developments (Since v4)

### A.1 Claude Code v2.1.74+ (Anthropic)

| New Feature | Description | VibeCody Status |
|-------------|-------------|-----------------|
| **MCP Tool Search lazy loading** | Tools loaded on-demand via ToolSearch; 95% context reduction; deferred tool discovery | GAP — MCP tools loaded eagerly at startup; no lazy/deferred loading |
| **ExitWorktree tool** | Explicit tool for leaving git worktree isolation | FIT — worktree management implemented |
| **Memory leak fixes** | Systematic memory optimization in long sessions | Partial — no explicit memory profiling/leak detection for long sessions |
| **Compact mode improvements** | Better context compression when approaching limits | FIT — context pruning exists |

### A.2 Cursor (March 2026 Updates)

| New Feature | Description | VibeCody Status |
|-------------|-------------|-----------------|
| **30+ MCP partner plugins** | Curated ecosystem of verified MCP integrations (Sentry, Datadog, LaunchDarkly, etc.) | GAP — MCP client exists but no curated/verified plugin directory |
| **Credit-based billing model** | Shifting from flat subscription to usage-based credits; granular cost tracking | Partial — cost observatory tracks spend but no credit/metering system |
| **Background agent improvements** | Agents run in isolated cloud sandboxes with event triggers refined | FIT — automations.rs + cloud_sandbox.rs |

### A.3 GitHub Copilot (March 2026)

| New Feature | Description | VibeCody Status |
|-------------|-------------|-----------------|
| **Copilot Spaces** | Curated context bundles: pin files, add instructions, share with team for consistent AI behavior | GAP — no equivalent "context bundle" sharing mechanism |
| **Agentic code review GA** | AI reviews PRs automatically with structural understanding (March 5, 2026) | FIT — bugbot.rs + self_review.rs |
| **JetBrains GA with agent hooks** | Full agent mode in JetBrains with hook system for custom workflows | Partial — JetBrains plugin exists but no agent hooks integration |
| **Multi-model picker per task** | GPT-5.4, Claude Opus 4.6, Gemini 2.5 Pro selectable per task | FIT — 17 providers + per-task model selection |

### A.4 New Entrants

#### Replit Agent 3

| Feature | Description | VibeCody Status |
|---------|-------------|-----------------|
| **Browser-based agent IDE** | Full development environment in browser with AI agent | Partial — no browser-based mode |
| **Deployments included** | Every project auto-deploys to replit.dev | Partial — deploy panel generates configs but no built-in hosting |
| **Mobile development** | Build and test from mobile devices | Partial — remote_control.rs enables mobile interaction but not full mobile dev |

#### Amazon Q Developer

| Feature | Description | VibeCody Status |
|---------|-------------|-----------------|
| **AWS service integration** | Deep integration with 200+ AWS services; auto-generates IAM policies, CloudFormation | GAP — no deep cloud provider integration (AWS/GCP/Azure) |
| **Code transformation** | Automated Java 8→17, .NET Framework→.NET 6+ upgrades | FIT — legacy_migration.rs covers language migrations |
| **Security scanning** | Built-in vulnerability scanning with auto-remediation | FIT — security_scanning.rs |

#### Qodo (formerly CodiumAI)

| Feature | Description | VibeCody Status |
|---------|-------------|-----------------|
| **Test generation focus** | AI-powered test generation with edge case discovery | FIT — test runner + coverage panel |
| **PR review agent** | Automated PR quality analysis | FIT — bugbot.rs |

#### Roo Code / Cline

| Feature | Description | VibeCody Status |
|---------|-------------|-----------------|
| **Open-source VS Code agent** | Community-driven, fully open-source agent in VS Code | FIT — VibeCody is open-source |
| **MCP-first architecture** | Built around MCP from ground up; any tool via MCP | FIT — MCP client implemented |
| **Custom agent modes** | User-defined agent personalities and tool configs | FIT — agent_modes.rs |

#### Zed (Agentic Editing)

| Feature | Description | VibeCody Status |
|---------|-------------|-----------------|
| **Agent Client Protocol (ACP)** | Open protocol for standardized agent-editor communication | GAP — uses custom protocol; no ACP compliance |
| **Native GPU rendering** | Metal/Vulkan-based editor with sub-millisecond rendering | Partial — gpu_terminal.rs exists but editor is Electron-based (Tauri) |
| **Built-in agent with tool use** | Agent runs in editor process, no external dependencies | FIT — VibeUI has integrated agent |

#### Visual Studio 2026

| Feature | Description | VibeCody Status |
|---------|-------------|-----------------|
| **AI-native IDE** | AI woven into every IDE feature (not bolt-on) | N/A — different architecture approach |
| **Copilot deep integration** | Tightest Copilot integration of any IDE | N/A — VibeCody is its own platform |

## Part B — New Gap Priority Matrix

### P0 — Critical (Competitors Shipping, High Impact)

| # | Gap | Competitors | Description | Effort |
|---|-----|-------------|-------------|--------|
| 1 | **MCP lazy loading / tool search** | Claude Code v2.1.74 | Deferred tool discovery: tools loaded on-demand via search, not all at startup. 95% context reduction in conversations with many MCP servers. Critical for scaling MCP ecosystem. | Medium |
| 2 | **Copilot Spaces (context bundles)** | GitHub Copilot | Curated, shareable context sets: pinned files, custom instructions, team-wide consistency. Different from rules/memory — these are task-specific, composable, and sharable. | Medium |
| 3 | **Cloud provider deep integration** | Amazon Q, Copilot | Native integration with AWS/GCP/Azure services: IAM policy generation, CloudFormation/Terraform scaffolding, service-specific code generation, cost estimation. Goes beyond generic deploy panel. | High |

### P1 — Important (Emerging Standards, Medium-High Impact)

| # | Gap | Competitors | Description | Effort |
|---|-----|-------------|-------------|--------|
| 4 | **Agent Client Protocol (ACP)** | Zed, emerging standard | Open protocol for agent-editor communication. As ACP gains adoption, VibeCody should support it alongside its custom protocol for interoperability with external agents. | Medium |
| 5 | **MCP verified plugin directory** | Cursor (30+ partners) | Curated, security-verified MCP plugin directory with ratings, reviews, and one-click install. marketplace.rs exists but needs MCP-specific curation and verification pipeline. | Medium |
| 6 | **Usage metering / credit system** | Cursor, Devin ACU | Granular usage tracking with credit-based budgets: per-agent, per-task, per-team cost allocation. Goes beyond cost observatory to enable team billing, usage quotas, and chargeback. | Medium |

### P2 — Nice-to-Have (Competitive Differentiation)

| # | Gap | Competitors | Description | Effort |
|---|-----|-------------|-------------|--------|
| 7 | **Browser-based mode (WebAssembly)** | Bolt.new, Replit, Devin | Zero-install browser experience for quick prototyping. Compile VibeCLI core to WASM, serve via static site. Not replacing desktop — complementary for onboarding and demos. | High |
| 8 | **Long-session memory profiling** | Claude Code | Memory leak detection and automatic cleanup for 8+ hour agent sessions. Monitor heap growth, detect leaked contexts, auto-compact. | Low |
| 9 | **SWE-bench benchmarking harness** | Augment, Blitzy, Devin | Built-in SWE-bench evaluation runner: download benchmark, run agent on tasks, measure pass@1 rate. Enables users to benchmark their provider+config combination. | Medium |
| 10 | **JetBrains agent hooks** | GitHub Copilot | Extend JetBrains plugin with hook system for agent lifecycle events (pre-edit, post-commit, review triggers). Matches Copilot's JetBrains GA agent hooks. | Low |

### P3 — Forward-Looking (2027 Preparation)

| # | Gap | Competitors | Description | Effort |
|---|-----|-------------|-------------|--------|
| 11 | **SOC 2 Type II / ISO 27001 path** | Augment, Blitzy, Copilot | Certification path documentation, compliance controls inventory, audit trail enhancements. Not the certification itself (requires organizational process) but the technical controls. | Medium |
| 12 | **Multi-modal agent (voice + vision + code)** | Emerging | Unified agent that processes voice commands (voice.rs), screenshots (vision), and code edits in a single conversation turn. Current implementation has these as separate features. | High |

## Part C — Competitive Strengths Matrix (Updated)

### Features Where VibeCody Leads or Is Unique

| Feature | VibeCody | Claude Code | Cursor | Copilot | Devin | Augment | Amp | Replit | Amazon Q | Zed |
|---------|----------|-------------|--------|---------|-------|---------|-----|--------|----------|-----|
| Open-source + self-hostable | Yes | No | No | No | No | No | No | No | No | Yes |
| 17 direct AI providers + BYOK | Yes | 1 | ~5 | ~4 | 1 | ~3 | ~3 | 1 | 1 | ~3 |
| 18-platform messaging gateway | Yes | No | Slack | No | Slack | No | No | No | No | No |
| 539+ domain skills | Yes | ~20 | No | Community | No | No | No | No | AWS-specific | No |
| Dual-surface (CLI + Desktop IDE) | Yes | CLI only | IDE only | IDE+CLI | Web only | IDE only | Multi | Web only | IDE only | IDE only |
| Soul.md generator | Yes | No | No | No | No | No | No | No | No | No |
| Batch generation (3M+ lines) | Yes | No | No | No | No | No | No | No | No | No |
| Legacy migration (18 languages) | Yes | No | No | No | Partial | No | No | No | Java/.NET | No |
| OpenTelemetry tracing | Yes | No | No | No | No | No | No | No | No | No |
| Arena mode (blind A/B) | Yes | No | No | No | No | No | No | No | No | No |
| Red team / pentest pipeline | Yes | No | No | No | No | No | No | No | No | No |
| WASM extension system | Yes | No | No | No | No | No | No | No | No | Yes |
| Air-gapped mode (Ollama) | Yes | No | No | No | No | No | No | No | No | No |
| 128+ tool panels | Yes | N/A | ~10 | ~5 | ~3 | ~3 | ~3 | ~5 | ~5 | ~3 |
| MCP lazy loading | No | Yes | Partial | No | No | No | No | No | No | No |
| Context bundles (Spaces) | No | No | No | Yes | No | No | No | No | No | No |
| Browser-based zero-setup | No | No | No | No | Yes | No | No | Yes | No | No |
| Deep cloud provider integration | No | No | No | Partial | No | No | No | No | Yes | No |
| ACP compliance | No | No | No | No | No | No | No | No | No | Yes |
| SOC 2 Type II certified | No | No | No | Yes | No | Yes | No | No | Yes | No |

### VibeCody's Structural Advantages (Unchanged)

1. **Open-source + BYOK** — No vendor lock-in; 17+ providers or OpenRouter's 300+ models; free forever
2. **Dual-surface** — CLI (VibeCLI) + Desktop IDE (VibeUI) from one codebase; competitors pick one
3. **Extensibility** — WASM plugins, 539+ skills, hooks, MCP, Agent SDK — deepest customization stack
4. **Domain coverage** — Only tool with skills for aerospace (DO-178C), medical (HIPAA), finance (SOX), safety-critical (MISRA/SPARK), and 25+ industry verticals
5. **Self-hosting** — Docker + Ollama air-gapped mode; critical for defense, healthcare, regulated industries
6. **Observability** — OpenTelemetry OTLP tracing to Jaeger/Zipkin/Grafana
7. **Cost control** — Budget limits, cost observatory, arena mode for model evaluation

## Part D — Market & Competitive Positioning Shifts (Since v4)

### New Market Dynamics

| Trend | Impact on VibeCody |
|-------|--------------------|
| **57% companies running AI agents in production** | Market is mainstream; enterprise features (governance, audit, compliance) are now required, not nice-to-have |
| **$8.5B autonomous agent market** | Validates VibeCody's agent-first architecture |
| **MCP ecosystem at $1.8B** | MCP is the winning protocol; VibeCody's early MCP adoption is vindicated; need to scale MCP ecosystem support |
| **SWE-bench top scores at 80.9%** | Benchmark scores increasingly marketing-driven; VibeCody should offer benchmarking-as-a-feature |
| **Credit-based billing emerging** | Flat subscriptions giving way to usage-based; VibeCody's BYOK model remains strongest counter but needs metering for teams |
| **Copilot CLI 1.0 GA** | GitHub now competes directly in terminal agent space; validates VibeCLI's architecture but increases competitive pressure |
| **Visual Studio 2026 AI-native** | Microsoft embedding AI deeply into VS; enterprises on VS may not look beyond Copilot |

### Emerging Threats (Updated)

1. **Copilot Spaces** — Sharable context bundles could become a developer workflow standard; no open-source equivalent exists
2. **MCP lazy loading** — Claude Code's 95% context reduction sets new expectation for MCP scalability
3. **Amazon Q's AWS depth** — Deep cloud integration is a moat VibeCody can't match without significant investment; consider plugin approach
4. **Zed ACP standardization** — If ACP becomes the standard agent protocol, non-compliant tools may be excluded from multi-agent workflows
5. **Browser-based competitors** — Bolt.new, Replit, Devin all have zero-install web experiences; barrier to VibeCody adoption for quick prototyping
6. **Enterprise certification gap** — SOC 2 / ISO 27001 increasingly required in procurement; VibeCody lacks certification path

### Opportunities

1. **MCP ecosystem leadership** — Build the largest open-source MCP plugin directory with verification
2. **Regulated industry focus** — No competitor addresses aerospace/defense/medical/finance with domain skills at VibeCody's depth
3. **On-premises AI coding** — Growing demand in defense, government, healthcare for air-gapped AI coding
4. **Context bundle standardization** — Define an open standard for sharable context bundles (like Copilot Spaces but open)
5. **Multi-agent orchestration** — VibeCody's agent teams + batch builder is the most complete open-source multi-agent coding system
6. **Soul.md movement** — VibeCody is the only tool that generates project philosophy documents; could become a community standard

## Part E — Recommended Roadmap for New Gaps

### Phase 68: MCP Lazy Loading / Tool Search (P0)

**Goal:** Reduce MCP context overhead by 90%+ through deferred tool loading.

**Implementation:**

- `mcp_lazy.rs`: LazyToolRegistry that discovers available tools without loading full schemas
- Tool manifests (name + description only) loaded at startup; full schema loaded on first use
- ToolSearch command: keyword search across all MCP servers, loads matching tools on demand
- LRU eviction: unload unused tool schemas after configurable idle timeout
- Backward-compatible: eager loading still available via config flag

**Effort:** Medium (2-3 days)

### Phase 69: Context Bundles / Spaces (P0)

**Goal:** Sharable, composable context sets for consistent AI behavior across team.

**Implementation:**

- `context_bundles.rs`: ContextBundle with pinned files, custom instructions, excluded paths, model preferences
- Bundle file format: `.vibebundle.toml` (portable, version-controlled)
- Bundle operations: create, activate, deactivate, share, import, export
- Multiple active bundles with priority ordering
- Auto-inject bundle context into agent system prompt
- `ContextBundlePanel.tsx`: create/edit/share bundles, browse team bundles
- REPL: `/bundle create|activate|deactivate|list|share|import`

**Effort:** Medium (2-3 days)

### Phase 70: Cloud Provider Integration (P0)

**Goal:** Deep integration with AWS/GCP/Azure for infrastructure-aware AI coding.

**Implementation:**

- `cloud_providers.rs`: CloudProviderManager with AWS/GCP/Azure adapters
- IAM policy generation from code analysis (detect S3, DynamoDB, Lambda usage → generate least-privilege IAM)
- CloudFormation/Terraform/Pulumi template generation from project structure
- Service cost estimation based on usage patterns
- Cloud-specific skill files (aws-lambda, gcp-cloud-run, azure-functions, etc.)
- Integrates with existing deploy panel
- REPL: `/cloud aws|gcp|azure` subcommands

**Effort:** High (4-5 days)

### Phase 71: Agent Client Protocol (ACP) (P1)

**Goal:** Support the emerging ACP standard for inter-agent communication.

**Implementation:**

- `acp_protocol.rs`: ACP message types, capability negotiation, tool registration
- Dual-protocol support: existing VibeCody protocol + ACP for external agents
- ACP server mode: expose VibeCody tools to external ACP-compatible editors (Zed, others)
- ACP client mode: connect to external ACP agents as tool providers
- Protocol version negotiation and graceful fallback

**Effort:** Medium (2-3 days)

### Phase 72: MCP Verified Plugin Directory (P1)

**Goal:** Curated, searchable directory of verified MCP plugins.

**Implementation:**

- `mcp_directory.rs`: PluginDirectory with categories, ratings, security verification status
- Plugin manifest format: `mcp-plugin.toml` with metadata, dependencies, permissions
- Verification pipeline: checksum validation, permission audit, sandboxed test execution
- One-click install/update/uninstall via CLI and VibeUI
- Community ratings and usage stats
- `McpDirectoryPanel.tsx`: browse, search, install, rate plugins
- REPL: `/mcp install|search|update|uninstall|verify`

**Effort:** Medium (3-4 days)

### Phase 73: Usage Metering / Credit System (P1)

**Goal:** Granular usage tracking for team billing and cost allocation.

**Implementation:**

- `usage_metering.rs`: UsageMeter with per-agent, per-task, per-user token tracking
- Credit budgets: configurable limits per team/user/project with alerts
- Usage reports: daily/weekly/monthly breakdown by provider, model, task type
- Chargeback support: allocate AI costs to projects/departments
- Integration with cost observatory for unified spend view
- `UsageMeteringPanel.tsx`: dashboards, budgets, alerts, reports

**Effort:** Medium (2-3 days)

### Phase 74: Browser-Based Mode (P2)

**Goal:** Zero-install web experience for onboarding and quick prototyping.

**Implementation:**

- Compile VibeCLI core to WASM (vibe-core + vibe-ai HTTP client)
- Static site hosting with Monaco editor, terminal emulator, file tree
- WebContainer or server-side sandbox for command execution
- Subset of features: chat, basic agent, file editing, preview
- Link to desktop install for full experience
- Progressive enhancement: web → desktop migration path

**Effort:** High (2-3 weeks)

### Phase 75: SWE-bench Benchmarking Harness (P2)

**Goal:** Built-in benchmark runner for evaluating agent performance.

**Implementation:**

- `swe_bench.rs`: BenchmarkRunner that downloads SWE-bench tasks, runs agent, measures pass@1
- Support SWE-bench Verified, SWE-bench Pro, and custom benchmark suites
- Automated scoring with detailed per-task reports
- Provider/model comparison across benchmark runs
- `BenchmarkPanel.tsx`: run benchmarks, compare results, export reports
- REPL: `/benchmark run|compare|export`

**Effort:** Medium (3-4 days)

### Phase 76: JetBrains Agent Hooks (P2)

**Goal:** Hook system for JetBrains plugin agent lifecycle events.

**Implementation:**

- Extend JetBrains plugin with hook registration API
- Events: pre-edit, post-edit, pre-commit, post-commit, agent-start, agent-complete
- Hook types: shell command, HTTP webhook, LLM-based (consistent with VibeCLI hooks)
- Settings UI in JetBrains for hook configuration

**Effort:** Low (1-2 days)

### Phase 77: Long-Session Memory Profiling (P2)

**Goal:** Detect and mitigate memory leaks in 8+ hour agent sessions.

**Implementation:**

- `session_memory.rs`: MemoryProfiler with periodic heap sampling
- Leak detection: track allocation growth rate, flag abnormal patterns
- Auto-compact: evict stale context, compress conversation history
- Session health dashboard with memory usage graphs
- Configurable thresholds and auto-cleanup policies

**Effort:** Low (1-2 days)

### Phase 78: SOC 2 Technical Controls (P3)

**Goal:** Implement technical controls required for SOC 2 Type II readiness.

**Implementation:**

- `compliance_controls.rs`: ControlInventory mapping to SOC 2 Trust Service Criteria
- Audit trail enhancements: immutable log of all AI-generated code changes
- Access control documentation: RBAC policies, API key rotation, session management
- Data retention policies: configurable log retention, PII redaction
- Compliance report generation for auditors
- Does NOT include the certification process itself (organizational requirement)

**Effort:** Medium (3-4 days)

### Phase 79: Multi-Modal Unified Agent (P3)

**Goal:** Single agent conversation handling voice + vision + code in unified turns.

**Implementation:**

- `multimodal_agent.rs`: UnifiedAgent that processes mixed input types per turn
- Voice → text → agent action pipeline (voice.rs integration)
- Screenshot → vision → code generation pipeline (vision provider integration)
- Unified conversation context: interleave voice commands, image references, and code edits
- Mode detection: auto-switch between voice, vision, and code input

**Effort:** High (4-5 days)

## Part F — Metrics Summary (Updated)

| Metric | v4 Count | v5 Count |
|--------|----------|----------|
| Total unit tests | ~5,335 | ~5,745 |
| Skill files | 536 | 539+ |
| AI providers | 17 + OpenRouter (300+) | 17 + OpenRouter (300+) |
| VibeUI panels | 119 | 136+ |
| REPL commands | 60+ | 65+ |
| Gateway platforms | 18 | 18 |
| Competitors analyzed | 11 | 17 (+ Replit, Amazon Q, Qodo, Roo Code, Cline, Zed) |
| v4 gaps (all closed) | 23 | 23 (all closed) |
| **v5 new gaps** | — | **12** |
| v5 P0 gaps | — | 3 |
| v5 P1 gaps | — | 3 |
| v5 P2 gaps | — | 4 |
| v5 P3 gaps | — | 2 |

## Part G — Remaining "Partial" Entries (Not Code-Addressable)

These items from v4 remain Partial and are **not closeable through code alone**:

| Item | Why Not Code-Addressable |
|------|--------------------------|
| No proprietary coding model (like SWE-1.5) | Requires training infrastructure, ML team, and significant compute investment |
| No browser-only mode (like Bolt.new) | Requires WASM compilation + hosting infrastructure (Phase 74 addresses partially) |
| SOC 2 Type II certification | Organizational process, not a feature (Phase 78 addresses technical controls) |
| Smaller plugin ecosystem vs Cursor (30+ partners) | Community/business development, not code (Phase 72 builds directory infrastructure) |
| No managed hosting domain | Business decision requiring infrastructure investment |

## Sources

*Carries forward all v4 sources, plus:*

- [Claude Code v2.1.74 Changelog — MCP Tool Search](https://github.com/anthropics/claude-code/blob/main/CHANGELOG.md)
- [GitHub Copilot Spaces](https://github.blog/ai-and-ml/github-copilot/introducing-copilot-spaces/)
- [Copilot JetBrains GA](https://github.blog/changelog/2026-03-05-copilot-in-jetbrains-is-generally-available/)
- [Copilot Agentic Code Review GA](https://github.blog/changelog/2026-03-05-copilot-code-review-is-generally-available/)
- [Zed Agent Client Protocol](https://zed.dev/blog/acp)
- [Amazon Q Developer Features](https://aws.amazon.com/q/developer/features/)
- [Replit Agent 3](https://replit.com/agent)
- [Qodo AI Testing](https://www.qodo.ai/)
- [Roo Code](https://github.com/RooVetGit/Roo-Code)
- [Cline VS Code Agent](https://github.com/cline/cline)
- [Visual Studio 2026 AI-native Preview](https://devblogs.microsoft.com/visualstudio/visual-studio-2026-preview/)
- [AI Agent Market Size $8.5B](https://www.marketsandmarkets.com/Market-Reports/ai-agent-market.html)
- [MCP Ecosystem Growth](https://modelcontextprotocol.io/ecosystem)
- [SWE-bench Leaderboard](https://www.swebench.com/)
- [57% Companies Running AI Agents](https://www.gartner.com/en/newsroom/press-releases/2026-03-ai-agents-production)
