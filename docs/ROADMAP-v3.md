---
layout: page
title: Competitive Roadmap v3 — Futureproofing 2026-2027
permalink: /roadmap-v3/
---


**Date:** March 2026
**Previous:** ROADMAP-v2.md (February 2026) — all phases complete
**Scope:** Futureproofing gaps identified in FIT-GAP-ANALYSIS-v5.md; 12 new phases across 4 priority tiers

---

## Current State

All phases from Roadmap v1 (1-5) and v2 (6-9) are **complete**. FIT-GAP v4's 23 gaps are **all closed** (Phases 53-67). VibeCody has:

| Metric | Count |
|--------|-------|
| Unit tests | ~5,745 |
| Skill files | 539+ |
| AI providers | 17 direct + OpenRouter (300+) |
| VibeUI panels | 128+ |
| REPL commands | 65+ |
| Gateway platforms | 18 |
| Rust modules | 100+ |

---

## Phase 10: MCP Ecosystem & Scalability

### 10.1 MCP Lazy Loading / Tool Search (P0)

**Why:** Claude Code v2.1.74 showed 95% context reduction through deferred tool loading. As MCP ecosystems grow (30+ plugins in Cursor alone), eager loading of all tool schemas at startup becomes unsustainable.

**Deliverables:**
- [ ] `mcp_lazy.rs` — LazyToolRegistry with manifest-only startup loading
- [ ] ToolSearch: keyword search across all MCP servers, loads matching tools on demand
- [ ] LRU eviction: auto-unload unused tool schemas after configurable idle timeout
- [ ] Config flag: `mcp.lazy_loading = true|false` (default: true)
- [ ] Metrics: track context savings, cache hit rates, load times
- [ ] Tests: 30+ unit tests

### 10.2 MCP Verified Plugin Directory (P1)

**Why:** Cursor has 30+ verified MCP partner plugins. VibeCody needs a curated directory to compete.

**Deliverables:**
- [ ] `mcp_directory.rs` — PluginDirectory with categories, ratings, verification status
- [ ] Plugin manifest format: `mcp-plugin.toml`
- [ ] Verification pipeline: checksum, permission audit, sandboxed test
- [ ] `McpDirectoryPanel.tsx` — browse, search, install, rate
- [ ] REPL: `/mcp install|search|update|uninstall|verify`
- [ ] Tests: 25+ unit tests

---

## Phase 11: Context & Collaboration

### 11.1 Context Bundles / Spaces (P0)

**Why:** Copilot Spaces allow teams to share curated context sets. No open-source equivalent exists. This is a differentiation opportunity.

**Deliverables:**
- [ ] `context_bundles.rs` — ContextBundle struct (pinned files, instructions, excludes, model prefs)
- [ ] Bundle file format: `.vibebundle.toml` (portable, versionable)
- [ ] Operations: create, activate, deactivate, share, import, export
- [ ] Multiple active bundles with priority ordering
- [ ] Auto-injection into agent system prompt
- [ ] `ContextBundlePanel.tsx` — create/edit/share/browse team bundles
- [ ] REPL: `/bundle create|activate|list|share|import`
- [ ] Tests: 35+ unit tests

### 11.2 Usage Metering / Credit System (P1)

**Why:** Credit-based billing is replacing flat subscriptions (Cursor, Devin ACU). Even for BYOK users, teams need per-user/per-project cost allocation.

**Deliverables:**
- [ ] `usage_metering.rs` — UsageMeter with per-agent/task/user token tracking
- [ ] Credit budgets: configurable limits per team/user/project with alerts
- [ ] Usage reports: daily/weekly/monthly by provider, model, task type
- [ ] Chargeback: allocate AI costs to projects/departments
- [ ] Integration with cost observatory
- [ ] `UsageMeteringPanel.tsx` — dashboards, budgets, alerts, reports
- [ ] Tests: 30+ unit tests

---

## Phase 12: Cloud & Infrastructure

### 12.1 Cloud Provider Deep Integration (P0)

**Why:** Amazon Q Developer's deep AWS integration is a moat. VibeCody needs equivalent coverage for AWS/GCP/Azure to serve enterprise teams.

**Deliverables:**
- [ ] `cloud_providers.rs` — CloudProviderManager with AWS/GCP/Azure adapters
- [ ] IAM policy generation: analyze code → detect service usage → generate least-privilege policies
- [ ] IaC template generation: CloudFormation, Terraform, Pulumi from project structure
- [ ] Service cost estimation from usage patterns
- [ ] Cloud-specific skill files (20+): aws-lambda, gcp-cloud-run, azure-functions, etc.
- [ ] REPL: `/cloud aws|gcp|azure` with subcommands
- [ ] Tests: 40+ unit tests

### 12.2 Browser-Based Mode (P2)

**Why:** Bolt.new, Replit, and Devin all offer zero-install web experiences. VibeCody should offer a lightweight web mode for onboarding and demos.

**Deliverables:**
- [ ] Compile vibe-core to WASM (text buffer, search, file system abstraction)
- [ ] Static web app with Monaco editor, terminal emulator, file tree
- [ ] WebContainer or server-side sandbox for command execution
- [ ] Feature subset: chat, basic agent, file editing, preview
- [ ] Progressive enhancement: web → desktop migration path
- [ ] Hosted at vibecody.dev (or similar)

---

## Phase 13: Protocol & Interoperability

### 13.1 Agent Client Protocol (ACP) Support (P1)

**Why:** Zed is pushing ACP as a standard for agent-editor communication. Early adoption ensures VibeCody can participate in multi-agent ecosystems.

**Deliverables:**
- [ ] `acp_protocol.rs` — ACP message types, capability negotiation, tool registration
- [ ] ACP server mode: expose VibeCody tools to ACP-compatible editors
- [ ] ACP client mode: connect to external ACP agents as tool providers
- [ ] Dual-protocol support: VibeCody native + ACP
- [ ] Protocol version negotiation and graceful fallback
- [ ] Tests: 25+ unit tests

### 13.2 JetBrains Agent Hooks (P2)

**Why:** Copilot's JetBrains GA includes agent hooks. VibeCody's JetBrains plugin should match.

**Deliverables:**
- [ ] Extend JetBrains plugin with hook registration API
- [ ] Events: pre-edit, post-edit, pre-commit, post-commit, agent-start, agent-complete
- [ ] Hook types: shell command, HTTP webhook, LLM-based
- [ ] Settings UI in JetBrains for hook configuration
- [ ] Tests: 15+ unit tests

---

## Phase 14: Quality & Enterprise Readiness

### 14.1 SWE-bench Benchmarking Harness (P2)

**Why:** SWE-bench scores are the primary marketing metric in AI coding. VibeCody should let users benchmark their own provider+config.

**Deliverables:**
- [ ] `swe_bench.rs` — BenchmarkRunner with task download, agent execution, scoring
- [ ] Support SWE-bench Verified, SWE-bench Pro, custom suites
- [ ] Provider/model comparison across runs
- [ ] `BenchmarkPanel.tsx` — run, compare, export reports
- [ ] REPL: `/benchmark run|compare|export`
- [ ] Tests: 20+ unit tests

### 14.2 Long-Session Memory Profiling (P2)

**Why:** Claude Code has fixed memory leaks in long sessions. VibeCody's 8+ hour batch runs need similar safeguards.

**Deliverables:**
- [ ] `session_memory.rs` — MemoryProfiler with periodic heap sampling
- [ ] Leak detection: track allocation growth rate, flag anomalies
- [ ] Auto-compact: evict stale context, compress history
- [ ] Session health dashboard with memory usage graphs
- [ ] Tests: 15+ unit tests

### 14.3 SOC 2 Technical Controls (P3)

**Why:** Enterprise procurement increasingly requires SOC 2 / ISO 27001. Technical controls can be built now even if certification is later.

**Deliverables:**
- [ ] `compliance_controls.rs` — ControlInventory mapped to SOC 2 Trust Service Criteria
- [ ] Immutable audit trail for all AI-generated code changes
- [ ] Access control documentation: RBAC policies, key rotation, session management
- [ ] Data retention policies: configurable log retention, PII redaction
- [ ] Compliance report generation for auditors
- [ ] Tests: 20+ unit tests

### 14.4 Multi-Modal Unified Agent (P3)

**Why:** Voice (voice.rs), vision (providers), and code (agent.rs) currently operate as separate features. Unifying them enables more natural developer interaction.

**Deliverables:**
- [ ] `multimodal_agent.rs` — UnifiedAgent processing mixed inputs per turn
- [ ] Voice → text → agent action pipeline
- [ ] Screenshot → vision → code generation pipeline
- [ ] Interleaved conversation context (voice + images + code)
- [ ] Auto-detection of input mode
- [ ] Tests: 25+ unit tests

---

## Priority & Timeline Summary

| Phase | Feature | Priority | Estimated Effort |
|-------|---------|----------|-----------------|
| 10.1 | MCP Lazy Loading | P0 | 2-3 days |
| 10.2 | MCP Plugin Directory | P1 | 3-4 days |
| 11.1 | Context Bundles | P0 | 2-3 days |
| 11.2 | Usage Metering | P1 | 2-3 days |
| 12.1 | Cloud Provider Integration | P0 | 4-5 days |
| 12.2 | Browser-Based Mode | P2 | 2-3 weeks |
| 13.1 | ACP Protocol | P1 | 2-3 days |
| 13.2 | JetBrains Agent Hooks | P2 | 1-2 days |
| 14.1 | SWE-bench Harness | P2 | 3-4 days |
| 14.2 | Memory Profiling | P2 | 1-2 days |
| 14.3 | SOC 2 Controls | P3 | 3-4 days |
| 14.4 | Multi-Modal Agent | P3 | 4-5 days |

**P0 total:** ~8-11 days (3 features)
**P1 total:** ~7-10 days (3 features)
**P2 total:** ~3-5 weeks (4 features)
**P3 total:** ~7-9 days (2 features)

---

## Success Criteria

After completing Phases 10-14, VibeCody will:

1. **Scale MCP** — Handle 100+ MCP servers without context bloat (lazy loading)
2. **Match Copilot Spaces** — Open-source context bundle standard (`.vibebundle.toml`)
3. **Compete with Amazon Q** — Cloud provider integration for AWS/GCP/Azure
4. **Interoperate** — ACP compliance for multi-agent ecosystem participation
5. **Enterprise-ready** — Usage metering, SOC 2 controls, compliance reports
6. **Benchmarkable** — Built-in SWE-bench runner for transparent evaluation
7. **Accessible** — Browser-based mode for zero-install onboarding

---

## Competitive Position After v3

With all v3 phases complete, VibeCody would be the **only tool** that combines:

- Open-source + self-hostable + 17 AI providers
- CLI + Desktop IDE + Browser (3 surfaces)
- 539+ domain skills across 25+ industry verticals
- MCP lazy loading + verified plugin directory
- Context bundles (open standard)
- Cloud provider deep integration (AWS/GCP/Azure)
- ACP protocol compliance
- SWE-bench benchmarking
- SOC 2 technical controls
- Multi-modal unified agent (voice + vision + code)
- 18-platform messaging gateway
- Batch generation (3M+ lines) + multi-QA validation
- Air-gapped deployment with Ollama

No competitor currently offers more than 4-5 of these in combination.
