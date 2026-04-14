---
layout: page
title: FIT-GAP Analysis v11
permalink: /fitgap-v11/
---

# FIT-GAP Analysis v11 — April 2026

Comparative analysis against the latest competitor releases as of April 2026.
**All v10 gaps (20) are closed.** This document identifies the next wave.

---

## Competitor Matrix (April 2026)

| Feature | VibeCody | Claude Code 1.x | Cursor 4.0 | Copilot Workspace v2 | Devin 2.0 | Cody 6.0 |
|---|---|---|---|---|---|---|
| Agent-OS registry | **gap** | — | — | — | ✓ | — |
| Dynamic agent recruitment | **gap** | — | — | — | ✓ | — |
| Resource quotas / budgets | **gap** | ✓ | — | — | ✓ | — |
| Auto-scaling agent pool | **gap** | — | — | — | ✓ | — |
| Background agent persistence | **gap** | ✓ | ✓ | — | ✓ | — |
| Workspace snapshot / restore | **gap** | — | ✓ | — | ✓ | — |
| Multi-repo context | **gap** | — | ✓ | ✓ | — | ✓ |
| Inline diff accept/reject | **gap** | ✓ | ✓ | ✓ | — | ✓ |
| Automated changelog gen | **gap** | — | — | ✓ | — | — |
| Semantic code search | **gap** | ✓ | ✓ | ✓ | ✓ | ✓ |
| PR description generator | **gap** | ✓ | ✓ | ✓ | ✓ | — |
| Spec-to-test generator | **gap** | — | — | ✓ | ✓ | — |
| Dependency update advisor | **gap** | — | — | — | — | ✓ |
| Session export / import | **gap** | ✓ | — | — | — | — |
| Performance regression detect | **gap** | — | — | — | ✓ | — |
| Token budget dashboard | **gap** | ✓ | — | — | — | — |
| Agent capability discovery | **gap** | — | ✓ | — | ✓ | — |
| Prompt version control | **gap** | — | — | — | — | ✓ |
| Code explanation depth levels | **gap** | ✓ | ✓ | — | — | ✓ |
| Custom REPL macros | **gap** | ✓ | — | — | — | — |

---

## Priority Tiers

### P0 — Critical (must close immediately)

| # | Gap | Module | Competitor has it |
|---|---|---|---|
| 1 | Agent-OS registry | `agent_registry.rs` | Devin 2.0 |
| 2 | Dynamic agent recruitment | `agent_recruiter.rs` | Devin 2.0 |
| 3 | Resource quotas & budgets | `agent_quota.rs` | Claude Code 1.x, Devin 2.0 |
| 4 | Auto-scaling agent pool | `agent_autoscale.rs` | Devin 2.0 |

### P1 — High Priority

| # | Gap | Module | Competitor has it |
|---|---|---|---|
| 5 | Background agent persistence | `agent_persistence.rs` | Claude Code 1.x, Cursor 4.0 |
| 6 | Workspace snapshot / restore | `workspace_snapshot.rs` | Cursor 4.0, Devin 2.0 |
| 7 | Multi-repo context | `multi_repo_context.rs` | Cursor 4.0, Copilot v2, Cody 6.0 |
| 8 | Inline diff accept/reject | `inline_diff.rs` | All major competitors |

### P2 — Medium Priority

| # | Gap | Module | Competitor has it |
|---|---|---|---|
| 9 | Automated changelog gen | `changelog_gen.rs` | Copilot Workspace v2 |
| 10 | PR description generator | `pr_description.rs` | Claude Code 1.x, Cursor 4.0 |
| 11 | Spec-to-test generator | `spec_to_test.rs` | Copilot v2, Devin 2.0 |
| 12 | Dependency update advisor | `dep_update_advisor.rs` | Cody 6.0 |

### P3 — Future

| # | Gap | Module | Competitor has it |
|---|---|---|---|
| 13 | Session export / import | `session_export.rs` | Claude Code 1.x |
| 14 | Performance regression detect | `perf_regression.rs` | Devin 2.0 |
| 15 | Token budget dashboard | `token_dashboard.rs` | Claude Code 1.x |
| 16 | Agent capability discovery | `capability_discovery.rs` | Cursor 4.0, Devin 2.0 |
| 17 | Prompt version control | `prompt_vcs.rs` | Cody 6.0 |
| 18 | Code explanation depth | `explain_depth.rs` | Claude Code 1.x, Cody 6.0 |
| 19 | Custom REPL macros | `repl_macros.rs` | Claude Code 1.x |
| 20 | Semantic code search | `semantic_search_v2.rs` | All major competitors |

---

## Implementation Plan

### Phase 45: Agent-OS (P0)

- `agent_registry.rs` — Agent discovery and capability advertisement
- `agent_recruiter.rs` — Dynamic recruitment with skill matching
- `agent_quota.rs` — Per-agent token/time/cost quotas with enforcement
- `agent_autoscale.rs` — Pool size management based on queue depth

### Phase 46: Context & Workspace (P1)

- `agent_persistence.rs` — Serialize/restore running agent state
- `workspace_snapshot.rs` — Point-in-time workspace snapshot/restore
- `multi_repo_context.rs` — Cross-repo import graph and context aggregation
- `inline_diff.rs` — Hunk-level accept/reject with partial application

### Phase 47: Developer Workflow (P2)

- `changelog_gen.rs` — Git history → conventional changelog
- `pr_description.rs` — Diff-aware PR title/body generation
- `spec_to_test.rs` — BDD spec → test stub generator
- `dep_update_advisor.rs` — SemVer constraint analysis + update safety

### Phase 48: P3 (deferred)

- session_export, perf_regression, token_dashboard, capability_discovery,
  prompt_vcs, explain_depth, repl_macros, semantic_search_v2

---

## Success Criteria

- [ ] All P0 gaps: Rust modules + tests (≥12 each) + skill files + Tauri commands
- [ ] All P1 gaps: same requirements
- [ ] All P2 gaps: same requirements
- [ ] `cargo test -p vibecli --lib` → 0 failures
- [ ] `cargo check --workspace --exclude vibe-collab` → 0 errors
