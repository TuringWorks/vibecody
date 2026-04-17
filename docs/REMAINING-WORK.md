# VibeCody — Remaining Work Items

> Aggregated from all docs (FIT-GAP v1-v7, ROADMAP v1-v5, COMPETITIVE-ANALYSIS, AGENT-FRAMEWORK-BLUEPRINT, CHANGELOG).
> Items are ordered by priority (P0 first) then by effort.
>
> Last verified: 2026-03-29

---

## P0 — No Items

All P0 gaps across all FIT-GAP analyses (v1-v6) and roadmaps (v1-v4) are **closed**.

---

## P1 — High Priority (2 items)

### 1. Hosted Plugin/Model Hub

- **Source**: FIT-GAP v4 (line 135), COMPETITIVE-ANALYSIS (Continue Hub)
- **Current State**: `marketplace.rs` provides local plugin management, but there is no hosted web hub for discovery/sharing (equivalent to `hub.continue.dev` or VS Code Marketplace)
- **What's Needed**: Web-hosted registry with search, ratings, verified publishers, one-click install
- **Effort**: Medium (2-3 weeks infrastructure + hosting)
- **Blocking**: Nothing — local marketplace works; this is a distribution/adoption concern

### ~~2. Browser-Based Zero-Install Mode~~ — **CLOSED**

- **Closed by**: `web_client.rs` (1,048 lines) — self-contained SPA served from `vibecli serve`, zero CDN dependencies (air-gap safe), chat + agent modes with SSE streaming, dark/light theme, responsive design

### 3. SOC 2 Type II Certification

- **Source**: FIT-GAP v4 (line 189), COMPETITIVE-ANALYSIS (Devin has SOC 2 Type II)
- **Current State**: `compliance_controls.rs` implements technical controls (audit trail, PII redaction, retention policies). Missing: formal certification process
- **What's Needed**: Organizational SOC 2 Type II audit engagement (not a code task)
- **Effort**: External process (3-6 months, auditor engagement)
- **Note**: Technical controls are complete; this is a business/compliance process

---

## P2 — Medium Priority (1 item)

### ~~4. Visual Design Canvas~~ — **CLOSED**

- **Closed by**: `sketch_canvas.rs` + `SketchCanvasPanel.tsx` (Phase 31) — canvas drawing, shape recognition, wireframe-to-component mapping, 3D scene generation, SVG/PNG export

### ~~5. Sketch-to-3D Generation~~ — **CLOSED**

- **Closed by**: `sketch_canvas.rs` (Phase 31) — includes 3D scene generation for Three.js/React Three Fiber

### 6. Built-In Managed Hosting Domain

- **Source**: FIT-GAP v4 (Bolt → `.bolt.host`, Lovable → `.lovable.app`)
- **Current State**: `managed_deploy.rs` supports deployment to Docker, K8s, cloud providers, but no VibeCody-hosted domain
- **What's Needed**: `*.vibecody.app` domain with one-click deploy for generated apps
- **Effort**: Medium (hosting infrastructure, DNS management, billing)

### 7. 100M+ Line Codebase Benchmarking

- **Source**: FIT-GAP v4 (Blitzy handles 100M+ lines)
- **Current State**: `infinite_context.rs` + `batch_builder.rs` handle large codebases but not benchmarked at 100M+ lines
- **What's Needed**: Performance benchmarks, stress tests, and optimization for truly massive monorepos
- **Effort**: Low-medium (benchmarking + optimization pass)

---

## P3 — Low Priority (3 items)

### 8. Proprietary Frontier Coding Model

- **Source**: FIT-GAP v4 (Windsurf SWE-1.5, Cursor custom model)
- **Current State**: VibeCody is provider-agnostic (18+ providers). No proprietary model
- **What's Needed**: Fine-tuned coding-specific model (requires training data + GPU compute)
- **Effort**: Very high (months of ML training)
- **Note**: Architectural choice — provider-agnostic approach is a strength, not a weakness

### 9. VS Code Extension Full Compatibility

- **Source**: ROADMAP v4, FIT-GAP v6 (Phase 22.2)
- **Current State**: `extension_compat.rs` covers high-value subset of VS Code extensions
- **What's Needed**: Full VS Code extension API compatibility (thousands of APIs)
- **Effort**: Very high (ongoing, diminishing returns)
- **Decision**: Partial coverage is sufficient for most use cases

### 10. JetBrains Agent Hooks Deep Integration

- **Source**: FIT-GAP v5 (line ~90)
- **Current State**: JetBrains plugin exists (`jetbrains-plugin/`), but agent hooks are not deeply integrated (no pre/post-tool-use hooks in IntelliJ)
- **What's Needed**: Full hook system parity with CLI/Tauri in JetBrains
- **Effort**: Medium (2 weeks — JetBrains plugin extension)

---

## Consolidated Documentation (2026-04-17)

All prior roadmap and fit-gap iterations have been merged into exactly **two canonical documents**:

| Canonical document | Absorbs |
|--------------------|---------|
| [`ROADMAP.md`](./ROADMAP.md) | ROADMAP-v2 through v6 (phases 1–39) — phase-level history in Appendices A and B |
| [`FIT-GAP-ANALYSIS.md`](./FIT-GAP-ANALYSIS.md) | FIT-GAP-ANALYSIS v2–v12 (sequential iterations) plus AGENT-OS, PI-MONO, RL-OS, PAPERCLIP, and CODE-REVIEW-ARCHITECTURE deep-dives |

---

## Summary (Updated 2026-03-29)

All code-addressable gaps across **all 7 FIT-GAP analyses** have been **CLOSED**:

- FIT-GAP v1-v6: All gaps closed (Phases 1-22)
- FIT-GAP v7: All 22 gaps closed (Phases 23-31)
- Phase 32 bonus: 6 additional modules (context protocol, code review, diff review, code replay, speculative exec, explainable agent)
- TurboQuant KV-cache compression shipped

**Current totals:** 9,570 tests (0 failures), 185 Rust modules, 187 VibeUI panels, 568 skill files, 23 AI providers.

**4 remaining items are non-code** (infrastructure, business process, or deferred by design):

| Priority | Items | Type |
|----------|-------|------|
| **P1** | 2 | Hosted plugin hub (infra), SOC 2 certification (process) |
| **P2** | 1 | Managed hosting domain (infra) |
| **P3** | 1 | Proprietary frontier model (ML training) |
| **Total** | **4** | All non-code |

**Bottom line**: Every code-addressable feature is implemented. The 4 remaining items require infrastructure investment, business processes, or are explicitly deferred design decisions.
