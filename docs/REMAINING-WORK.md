# VibeCody — Remaining Work Items

> Aggregated from all docs (FIT-GAP v1-v14, ROADMAP Appendices A-E, COMPETITIVE-ANALYSIS, AGENT-FRAMEWORK-BLUEPRINT, CHANGELOG).
> Items are ordered by priority (P0 first) then by effort.
> For the phase-by-phase ledger see [ROADMAP.md](./ROADMAP.md) (especially Appendices D + E). For competitive analysis see [FIT-GAP-ANALYSIS.md](./FIT-GAP-ANALYSIS.md).
>
> Last verified: 2026-05-18

---

## P0 — In-flight (1 code item)

### 1. B2 — Plugin bundle format with admin install policies

- **Source**: Phase 54 P0 (Appendix E of ROADMAP)
- **Current State**: Patent-distance posture documented in fit-gap §18 (`403ea1c2`). No implementation yet. The `B2.x` broker commits in git log are unrelated (sandbox egress broker).
- **What's Needed**: `vibecli-plugin.toml` manifest (MCP servers + skills + subagents + rules + hooks); `vibecli plugin install <path-or-url>`; `WorkspaceStore` per-plugin policy (`Off` / `On` / `Required`); governance-panel surface; 6 BDD scenarios.
- **Effort**: Medium (1-2 weeks)
- **Blocking**: Patent-distance check passed; cleared to start. Must avoid Cursor marketplace layout/UX terminology.

---

## P1 — High Priority (5 items)

### 2. Hosted Plugin / Model Hub

- **Source**: FIT-GAP v4, COMPETITIVE-ANALYSIS (Continue Hub)
- **Current State**: `marketplace.rs` provides local plugin management; no hosted web hub for discovery/sharing (equivalent to `hub.continue.dev` or VS Code Marketplace).
- **What's Needed**: Web-hosted registry with search, ratings, verified publishers, one-click install.
- **Effort**: Medium (2-3 weeks infrastructure + hosting)
- **Blocking**: Distribution/adoption concern — local marketplace works. Pairs naturally with item #1 (B2 bundle format) once that lands.

### 3. SOC 2 Type II Certification

- **Source**: FIT-GAP v4, COMPETITIVE-ANALYSIS (Devin has SOC 2 Type II)
- **Current State**: `compliance_controls.rs` implements technical controls (audit trail, PII redaction, retention policies). Missing: formal certification.
- **What's Needed**: Organizational SOC 2 Type II audit engagement (not a code task).
- **Effort**: External process (3-6 months, auditor engagement)
- **Note**: Technical controls complete; this is a business/compliance process.

### 4. A7 — Browser-native UI-element annotation (Design Mode)

- **Source**: Phase 53 P1, FIT-GAP §16
- **Current State**: Patent-distance posture documented in fit-gap §18 (`403ea1c2`). No implementation yet — must remain distant from Cursor 3 annotation UX.
- **What's Needed**: Extend `desktop_agent.rs` browser-control track with DOM-element click-to-annotate, generating natural-language instructions tied to specific selectors. Per-feature note in `notes/`.
- **Effort**: Medium-high (2 wk design + 3 wk impl, patent-distance gated)

### 5. B3 — Always-on security-review agent class

- **Source**: Phase 54 P1 (v0.5.8)
- **Current State**: `/review` exists as on-demand command. Patent-distance posture documented (close to Cursor Security Review UX).
- **What's Needed**: Convert `/review` to a daemon-resident agent class; trigger on file-watcher / pre-commit / CI; route findings to the existing `Finding` schema; UI surface in `SecurityPanel.tsx`.
- **Effort**: Medium-high (3-4 weeks, patent-distance gated)

### 6. A1 — MCP Apps generic React embedding host

- **Source**: Phase 53 P0 (carry-over)
- **Current State**: Payload parser shipped (`647b58de`). Generic React UI host for rendering `application/vnd.mcp.app+json` payloads in `AIChat.tsx` not yet wired.
- **What's Needed**: React embed component sharing the WASM extension host's CSP; 4 BDD scenarios.
- **Effort**: Low-medium (3-5 days)

---

## P2 — Medium Priority (2 items)

### 7. Built-In Managed Hosting Domain

- **Source**: FIT-GAP v4 (Bolt → `.bolt.host`, Lovable → `.lovable.app`)
- **Current State**: `managed_deploy.rs` supports Docker / K8s / cloud providers; no VibeCody-hosted domain.
- **What's Needed**: `*.vibecody.app` domain with one-click deploy for generated apps.
- **Effort**: Medium (hosting infrastructure, DNS management, billing)

### 8. 100M+ Line Codebase Benchmarking

- **Source**: FIT-GAP v4 (Blitzy handles 100M+ lines)
- **Current State**: `infinite_context.rs` + `batch_builder.rs` handle large codebases but not benchmarked at 100M+ lines.
- **What's Needed**: Performance benchmarks, stress tests, optimization pass for truly massive monorepos.
- **Effort**: Low-medium (benchmarking + optimization pass)

---

## P3 — Low Priority / By Design (3 items)

### 9. Proprietary Frontier Coding Model

- **Source**: FIT-GAP v4 (Windsurf SWE-1.5, Cursor custom model)
- **Current State**: VibeCody is provider-agnostic (22+ providers).
- **Effort**: Very high (months of ML training)
- **Note**: Architectural choice — provider-agnostic approach is a strength, not a weakness. Not on the roadmap.

### 10. VS Code Extension Full Compatibility

- **Source**: ROADMAP v4, FIT-GAP v6
- **Current State**: `extension_compat.rs` covers high-value subset of VS Code extensions.
- **Effort**: Very high (ongoing, diminishing returns)
- **Decision**: Partial coverage is sufficient for most use cases; not pursuing full parity.

### 11. JetBrains Agent Hooks Deep Integration

- **Source**: FIT-GAP v5
- **Current State**: JetBrains plugin exists; agent hooks (pre/post-tool-use) are not deeply integrated.
- **What's Needed**: Hook system parity with CLI/Tauri in JetBrains.
- **Effort**: Medium (~2 weeks plugin extension)

---

## Parked (1 item)

### B5 — NVFP4 (Blackwell native) TurboQuant target

- **Source**: Phase 54 P0
- **Current State**: PARKED 2026-05-10 (no Blackwell hardware available for testing — RTX 5090 / B200 / GB200). Existing MXFP4 + AWQ-Marlin paths continue to ship.
- **Resume Trigger**: Hardware procurement → ~3-5 days code work (CubeCL/Burn ban unchanged; hand-written CUDA + Metal kernels via Candle/mistralrs-quant).

---

## Recently Closed (since 2026-03-29)

**Phase 53 — Trend delta + audit reconciliation (10 of 11):**

| Item | Module | Commit |
|---|---|---|
| A1 — MCP Apps payload parser | `mcp_app.rs` | `647b58de` |
| A2 — MCPB bundle format (pack / extract / digest) | `mcpb_bundle.rs` | `84a7e636` |
| A3 — `/.well-known/mcp.json` descriptor builder | `serve.rs` | `b13f9106` |
| A4 — ACP stdio JSON-RPC dispatcher | `acp_stdio.rs` | `e9dc09af` |
| A5 — Async subagent state machine | `nested_agents.rs` | `8560eaf0` |
| A6 — Multi-root workspace permission resolver | `workspace_roots.rs` | `32c6c710` |
| A8 — Bounded verify-repair loop | `desktop_agent.rs` | `4e199396` |
| A9 — Cloud-agent session resume handoff | `serve.rs` + watch/mobile | `c984f980` |
| A10 — Skills hot-reload watcher | `skill_watcher.rs` | `182a3f28` |
| A11 — `vibecli --migrate from-{claude-code,codex}` | `migrate.rs` | `d2f9209e` |

**Phase 54 — May 2026 weekly delta:**

| Item | Module | Commit |
|---|---|---|
| B1 — Skills as MCP primitives | `skill_catalog.rs` | `4a9f4275` |
| B4 — Cursor SDK parity audit | `docs/audit/08-cursor-sdk-parity.md` | `9b5b3709` |
| B6 — A2A signed agent-card (P-256 ECDSA) | `signed_agent_card.rs` | `decff6e1` |
| c1 — Ollama `/v1/messages` route | `serve.rs` | PR #8 |
| c2 — `GEMINI.md` fallback in `REPO_CANDIDATES` | `memory.rs` | PR #9 |

**`/goal` durable execution intent (entire feature shipped end-to-end):**

| Slice | Surface | Commit |
|---|---|---|
| G1.1 – G1.7 | Daemon CRUD + plan/link/start + REPL + VibeUI + curated /watch + VS Code + SDK + design docs | `55cf91ea` |
| G3.1 – G3.6 | TUI Goals screen, tree query, slash hybrid, planner override, Wear OS detail | `0ef69c24` |
| G4 + G5 | Recursive tree, current-pin, LLM recap, /agent auto-link, VS Code tree-view, SDK parity | `4c0294fc` |
| G6 | VibeUI + mobile pin chips, agent-stream attribution | `e76f17b4` |

**DREAD threat-model closures (selected):**

DREAD #1 (Tainted data-flow containment — slices A through G, including CLI / HTTP / RAG / MCP / log redaction / mobile / watch / WebView) · #2 (path-guard promotion + Tauri delegation) · #3 (cross-platform credential-dir parity + Windows pen-test harness) · #10 (WebView DOMPurify sanitizer + eslint + semgrep gates) · #11 + #12 (pairing-URL bearer drop) · #16 (`Redact<T>` newtype + credential-logging semgrep gate) · #18 (bind-address guidance) · #20 (bearer rotation + `/health` signal) · #21 (HTTP error-body redaction).

**In-flight design tracks (not yet roadmap-counted):**

| Track | Status | Design |
|---|---|---|
| RL-OS productionization | 7-slice plan; Path C hybrid backend; ~31k LOC of `rl_*_os.rs` orphaned, slices wire them | `docs/design/rl-os/` |
| Recap & Resume | Phase D/F slices landing; cross-cutting `Recap` shape; `/v1/recap` + `/v1/resume` | `docs/design/recap-resume/` |
| Sandbox tiers (egress broker) | B1.x – B6.x broker work landing (UDS listener, TLS interception, cloud-creds injection, audit sink, daemon entry point) | `docs/design/sandbox-tiers/` |

---

## Consolidated Documentation

All prior roadmap and fit-gap iterations have been merged into exactly **two canonical documents**, with this file as the lean "what's left" index:

| Canonical document | Absorbs |
|--------------------|---------|
| [`ROADMAP.md`](./ROADMAP.md) | ROADMAP-v2 through v6 (phases 1–39); Phase 53 (Appendix D); Phase 54 (Appendix E) |
| [`FIT-GAP-ANALYSIS.md`](./FIT-GAP-ANALYSIS.md) | FIT-GAP-ANALYSIS v2–v14; AGENT-OS, PI-MONO, RL-OS, PAPERCLIP, CODE-REVIEW-ARCHITECTURE deep-dives |

---

## Summary

**Open code items**: 6 (1 P0 + 4 P1 + 2 P2 — minus #4/#5/#6 patent-gated). The single highest-leverage next item is **B2 (plugin bundle format)** — patent-distance posture cleared, ~1-2 weeks of code work, unblocks #2 (hosted plugin hub) distribution.

**Open non-code items**: 4 (SOC 2 cert, managed hosting domain, frontier model, VS Code full compat) — all infrastructure / business-process / explicit-design-choice items.

**Parked**: 1 (B5 NVFP4 — hardware-blocked).

**Bottom line**: Phase 53 (A1–A11) is 10-of-11 closed; Phase 54 (B1, B4, B6 + c1, c2) is closed; only **B2** remains as the cycle's in-flight code work. Three large design tracks (RL-OS productionization, Recap & Resume, Sandbox tiers) are landing slice-by-slice and tracked in their own design docs rather than here.
