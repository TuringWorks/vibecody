# VibeCody — Remaining Work Items

> Aggregated from all docs (FIT-GAP v1-v14, ROADMAP Appendices A-E, COMPETITIVE-ANALYSIS, AGENT-FRAMEWORK-BLUEPRINT, CHANGELOG).
> Items are ordered by priority (P0 first) then by effort.
> For the phase-by-phase ledger see [ROADMAP.md](./ROADMAP.md) (especially Appendices D + E). For competitive analysis see [FIT-GAP-ANALYSIS.md](./FIT-GAP-ANALYSIS.md).
>
> Last verified: 2026-05-19

---

## P0 — In-flight

No P0 items. **A1 MCP Apps React host closed 2026-05-19**; **B2 plugin bundle format closed 2026-05-18** — see "Recently Closed" below. The two patent-gated P1 items (A7, B3) remain queued for the next cycle pending design-distance proposals.

---

## P1 — High Priority (3 items)

### 1. Hosted Plugin / Model Hub

- **Source**: FIT-GAP v4, COMPETITIVE-ANALYSIS (Continue Hub)
- **Current State**: `marketplace.rs` provides local plugin management; **B2 signed MCPB bundle format shipped 2026-05-18** (`cea41606`..`32793d4d`). What's still missing is a hosted web hub for discovery/sharing (equivalent to `hub.continue.dev` or VS Code Marketplace) that serves signed bundles.
- **What's Needed**: Web-hosted registry with search, ratings, verified publishers, one-click install. With B2 bundle format and per-publisher P-256 signatures in place, the registry is now an infrastructure concern only.
- **Effort**: Medium (2-3 weeks infrastructure + hosting)
- **Blocking**: Distribution/adoption concern — local install works (`PluginGovernancePanel.tsx`).

### 2. SOC 2 Type II Certification

- **Source**: FIT-GAP v4, COMPETITIVE-ANALYSIS (Devin has SOC 2 Type II)
- **Current State**: `compliance_controls.rs` implements technical controls (audit trail, PII redaction, retention policies). Missing: formal certification.
- **What's Needed**: Organizational SOC 2 Type II audit engagement (not a code task).
- **Effort**: External process (3-6 months, auditor engagement)
- **Note**: Technical controls complete; this is a business/compliance process.

### 3. A7 — Browser-native UI-element annotation (Design Mode)

- **Source**: Phase 53 P1, FIT-GAP §16
- **Current State**: Patent-distance posture documented in fit-gap §18 (`403ea1c2`). No implementation yet — must remain distant from Cursor 3 annotation UX.
- **What's Needed**: Extend `desktop_agent.rs` browser-control track with DOM-element click-to-annotate, generating natural-language instructions tied to specific selectors. Per-feature note in `notes/`.
- **Effort**: Medium-high (2 wk design + 3 wk impl, patent-distance gated)

### 4. B3 — Always-on security-review agent class

- **Source**: Phase 54 P1 (v0.5.8)
- **Current State**: `/review` exists as on-demand command. Patent-distance posture documented (close to Cursor Security Review UX).
- **What's Needed**: Convert `/review` to a daemon-resident agent class; trigger on file-watcher / pre-commit / CI; route findings to the existing `Finding` schema; UI surface in `SecurityPanel.tsx`.
- **Effort**: Medium-high (3-4 weeks, patent-distance gated)


---

## P2 — Medium Priority (2 items)

### 6. Built-In Managed Hosting Domain

- **Source**: FIT-GAP v4 (Bolt → `.bolt.host`, Lovable → `.lovable.app`)
- **Current State**: `managed_deploy.rs` supports Docker / K8s / cloud providers; no VibeCody-hosted domain.
- **What's Needed**: `*.vibecody.app` domain with one-click deploy for generated apps.
- **Effort**: Medium (hosting infrastructure, DNS management, billing)

### 7. 100M+ Line Codebase Benchmarking

- **Source**: FIT-GAP v4 (Blitzy handles 100M+ lines)
- **Current State**: `infinite_context.rs` + `batch_builder.rs` handle large codebases but not benchmarked at 100M+ lines.
- **What's Needed**: Performance benchmarks, stress tests, optimization pass for truly massive monorepos.
- **Effort**: Low-medium (benchmarking + optimization pass)

---

## P3 — Low Priority / By Design (3 items)

### 8. Proprietary Frontier Coding Model

- **Source**: FIT-GAP v4 (Windsurf SWE-1.5, Cursor custom model)
- **Current State**: VibeCody is provider-agnostic (22+ providers).
- **Effort**: Very high (months of ML training)
- **Note**: Architectural choice — provider-agnostic approach is a strength, not a weakness. Not on the roadmap.

### 9. VS Code Extension Full Compatibility

- **Source**: ROADMAP v4, FIT-GAP v6
- **Current State**: `extension_compat.rs` covers high-value subset of VS Code extensions.
- **Effort**: Very high (ongoing, diminishing returns)
- **Decision**: Partial coverage is sufficient for most use cases; not pursuing full parity.

### 10. JetBrains Agent Hooks Deep Integration

- **Source**: FIT-GAP v5
- **Current State**: **Foundation shipped 2026-05-19** (`7709bc0b` + `1ebe79c4`). `HookExecutor` service mirrors the CLI hook protocol (`hook_abort.rs`): subprocess invocation, exit-code semantics (0 allow / 2 block), structured JSON-decision stdout override, 30 s timeout, multi-hook chains. Settings table under IDE Settings → Tools → VibeCLI with Name / Event / Command / Enabled columns. `UserPromptSubmit` wired into `AgentPanel.startAgent` as the meaningful gate event — BLOCK prevents the agent from starting and surfaces the hook reason. The seven event kinds match `plugin_manifest::ALLOWED_HOOK_EVENTS`.
- **What's Needed**: Advisory firings for SSE-arriving events (`PreToolUse` / `PostToolUse` / `Stop` / `Notification`), `InlineEditAction` integration, optional per-project scoping (currently APP-level), and JUnit test harness for the executor. None of these are blocking; the foundation is usable today for users who want to gate task submissions.
- **Effort**: Remaining work is ~1 wk of plugin polish + targeted edits per call site.

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

**B2 — Plugin bundle format with admin install policies (shipped 2026-05-18 → 2026-05-19):**

| Slice | Surface | Commit |
|---|---|---|
| B2.1 — `vibecli-plugin.toml` manifest format + validator | `plugin_manifest.rs` | `cea41606` |
| B2.2 — Detached P-256 ECDSA signing (`vibecli-plugin.sig`) | `plugin_signing.rs` | `6275cf06` |
| B2.3 — `WorkspaceStore` per-plugin policy (Off / On / Required) | `workspace_store.rs` | `eb7dcbfe` |
| B2.4 — Core install function (atomic stage→swap) | `plugin_install.rs` | `2d52bb4e` |
| B2.5 — Runtime view filtered by policy | `plugin_runtime.rs` | `82d4a00b` |
| B2.6 — Governance panel + 5 Tauri commands | `PluginGovernancePanel.tsx`, `commands.rs` | `fb9b80b6` |
| B2.7 — Skill-loader activation (MCP list_skills) | `skill_catalog.rs`, `mcp_server.rs` | `9c0ac982` + `32793d4d` |
| B2.8 — MCP-server registration helper (`register_plugin_servers`) | `mcp_governance.rs` | `16da6354` |
| B2.12 — URL install (`plugin_install_from_url`, `vibecli plugin install <https://…>`) | `plugin_install.rs`, panel | `b7e7f988` |

All four patent-distance §18 principles anchored: no telemetry (#1), client-side admin-authored policy (#2), open MCPB lineage (#3), per-publisher P-256 trust roots (#4). Remaining loader activations (hook_abort, rules, subagent) are not strictly required for a working install path; the `plugin_runtime::enabled_*` API is the contract their consumer code can adopt when those init paths are next being refactored.

**A1 — MCP Apps generic React embedding host (shipped 2026-05-19):**

| Slice | Surface | Commit |
|---|---|---|
| A1 — `McpAppEmbed.tsx` + `mcp_apps_parse` Tauri command + AIChat fence detection | `McpAppEmbed.tsx`, `commands.rs`, `AIChat.tsx` | `39e95b17` |

Backend parser (`647b58de`) is the authoritative validator. Frontend host renders fenced ```mcp.app blocks via a typed React card with action buttons that dispatch `vibeui:mcp-app-action` window events. Component allow-list (`react@18`, `react@19`, `json-view`, `list`, `card`) — unknown components render a warning, never arbitrary JSX. Future iframe-sandboxed renderer honoring `payload.csp` is the next iteration; current host surfaces CSP declarations informationally.

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

**Open code items**: 4 (0 P0 + 2 P1 + 2 P2 — both P1 are patent-gated A7/B3). With A1 shipped, the next-cycle code work depends on the patent-distance audits clearing for A7 / B3, plus the discretionary P2 items (managed hosting, 100M+ benchmarking) and the non-code roadmap items.

**Open non-code items**: 4 (SOC 2 cert, managed hosting domain, frontier model, VS Code full compat) — all infrastructure / business-process / explicit-design-choice items.

**Parked**: 1 (B5 NVFP4 — hardware-blocked).

**Bottom line**: Phase 53 (A1–A11) is now **fully closed** (A1 React host shipped 2026-05-19); Phase 54 P0 (B1, B2, B6 + trivial closes c1, c2) is closed; only **A7** (P1, patent-gated) and **B3** (P1 v0.5.8, patent-gated) remain queued for the next cycle. Three large design tracks (RL-OS productionization, Recap & Resume, Sandbox tiers) are landing slice-by-slice and tracked in their own design docs rather than here.
