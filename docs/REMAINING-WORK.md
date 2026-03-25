# VibeCody — Remaining Work Items

> Aggregated from all docs (FIT-GAP v1-v6, ROADMAP v1-v4, COMPETITIVE-ANALYSIS, AGENT-FRAMEWORK-BLUEPRINT, CHANGELOG).
> Items are ordered by priority (P0 first) then by effort.
>
> Last verified: 2026-03-24

---

## P0 — No Items

All P0 gaps across all FIT-GAP analyses (v1-v6) and roadmaps (v1-v4) are **closed**.

---

## P1 — High Priority (3 items)

### 1. Hosted Plugin/Model Hub

- **Source**: FIT-GAP v4 (line 135), COMPETITIVE-ANALYSIS (Continue Hub)
- **Current State**: `marketplace.rs` provides local plugin management, but there is no hosted web hub for discovery/sharing (equivalent to `hub.continue.dev` or VS Code Marketplace)
- **What's Needed**: Web-hosted registry with search, ratings, verified publishers, one-click install
- **Effort**: Medium (2-3 weeks infrastructure + hosting)
- **Blocking**: Nothing — local marketplace works; this is a distribution/adoption concern

### 2. Browser-Based Zero-Install Mode

- **Source**: FIT-GAP v5 (line 61), ROADMAP v3 Phase 12.2 (P2), ROADMAP v4
- **Current State**: VibeCody runs as desktop app (Tauri) or CLI. No browser-only mode like Bolt.new, v0, or Replit
- **What's Needed**: WebAssembly build or thin web client that connects to `vibecli serve` backend
- **Effort**: High (3-4 weeks — requires WASM porting or full web frontend)
- **Note**: `browser_agent.rs` enables browser automation, but this gap is about VibeCody itself running _in_ a browser

### 3. SOC 2 Type II Certification

- **Source**: FIT-GAP v4 (line 189), COMPETITIVE-ANALYSIS (Devin has SOC 2 Type II)
- **Current State**: `compliance_controls.rs` implements technical controls (audit trail, PII redaction, retention policies). Missing: formal certification process
- **What's Needed**: Organizational SOC 2 Type II audit engagement (not a code task)
- **Effort**: External process (3-6 months, auditor engagement)
- **Note**: Technical controls are complete; this is a business/compliance process

---

## P2 — Medium Priority (4 items)

### 4. Visual Design Canvas

- **Source**: ROADMAP v4 "Gaps Not Addressed" (line 350)
- **Current State**: `CanvasPanel.tsx` exists for whiteboarding but is not a full design tool (no vector editing, component library, constraints)
- **What's Needed**: Figma-lite in-editor design tool with component drag-drop, constraints, export to code
- **Effort**: Very high (4-6 weeks)
- **Decision**: Explicitly deferred — "low differentiation vs Figma/native design tools"

### 5. Sketch-to-3D Generation

- **Source**: ROADMAP v4 "Gaps Not Addressed" (line 352), Replit Agent 4 feature
- **Current State**: Not implemented
- **What's Needed**: Convert 2D sketches/wireframes to 3D models using AI
- **Effort**: Very high (novel capability, depends on emerging models)
- **Decision**: Explicitly deferred — "wait for ecosystem maturity"

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

## Superseded Documentation (Archived)

These docs are complete and superseded by newer versions. They can be archived but contain useful historical context:

| Document | Superseded By | Action |
|----------|--------------|--------|
| `ROADMAP.md` (v1, Feb 2026) | `ROADMAP-v4.md` | Archive — Phases 1-5 complete |
| `ROADMAP-v2.md` (Feb 2026) | `ROADMAP-v4.md` | Archive — Phases 6-9 complete |
| `ROADMAP-v3.md` (Mar 2026) | `ROADMAP-v4.md` | Archive — Phases 10-14 complete |
| `FIT-GAP-ANALYSIS.md` (v1) | `FIT-GAP-ANALYSIS-v6.md` | Archive — all gaps closed |
| `FIT-GAP-ANALYSIS-v2.md` | `FIT-GAP-ANALYSIS-v6.md` | Archive — all gaps closed |
| `FIT-GAP-ANALYSIS-v3.md` | `FIT-GAP-ANALYSIS-v6.md` | Archive — 18/18 gaps closed |
| `FIT-GAP-ANALYSIS-v4.md` | `FIT-GAP-ANALYSIS-v6.md` | Archive — 23/23 gaps closed |
| `FIT-GAP-ANALYSIS-v5.md` | `FIT-GAP-ANALYSIS-v6.md` | Archive — 12/12 gaps closed |

---

## Summary

| Priority | Items | Code Addressable | Process/Infra Only |
|----------|-------|------------------|--------------------|
| **P0** | 0 | — | — |
| **P1** | 3 | 1 (browser mode) | 2 (hub hosting, SOC 2) |
| **P2** | 4 | 1 (benchmarking) | 3 (design canvas, 3D, hosting) |
| **P3** | 3 | 2 (extensions, JetBrains) | 1 (proprietary model) |
| **Total** | **10** | **4** | **6** |

**Bottom line**: All critical code features are implemented. The 10 remaining items are either deferred by design (canvas, 3D), infrastructure/business concerns (hosting, certification, hub), or diminishing-returns completeness work (VS Code compat, benchmarking).
