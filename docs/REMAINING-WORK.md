# VibeCody — Remaining Work Items

> Aggregated from all docs (FIT-GAP v1-v15, ROADMAP Appendices A-F, COMPETITIVE-ANALYSIS, AGENT-FRAMEWORK-BLUEPRINT, CHANGELOG).
> Items are ordered by priority (P0 first) then by effort.
> For the phase-by-phase ledger see [ROADMAP.md](./ROADMAP.md) (especially Appendices D + E + F). For competitive analysis see [FIT-GAP-ANALYSIS.md](./FIT-GAP-ANALYSIS.md).
>
> Last verified: 2026-06-13 (v15 competitor delta folded in — C1–C6 queued for Phase 55; A7/B3 now competitor-shipped, see escalation notes)

---

## P0 — Phase 55 newly queued (v0.5.8 cycle)

The v15 competitor delta ([FIT-GAP §16.6](./FIT-GAP-ANALYSIS.md) / [ROADMAP Appendix F](./ROADMAP.md)) opened six new gaps (C1–C6). Prior P0 items are clear — **A1 MCP Apps React host closed 2026-05-19**, **B2 plugin bundle format closed 2026-05-18** (see "Recently Closed"). Three C-gaps are P0 this cycle:

### C1 — Recurring / scheduled / self-paced agent ergonomics ("Routines" + `/loop`)

- **Source**: Phase 55 P0, FIT-GAP §16.6 (Claude Code Routines + Managed Agents + **`/loop`**; Codex `/goal` CLI 0.128.0 + Automations; Cursor Automations; Antigravity 2.0 scheduled tasks)
- **Current State**: Trigger *engine* exists — `automations.rs` ships **Cron / FileWatch / Webhook** triggers → sandboxed agent task; `/goal` durable-intent ships (`exec_goal.rs` — parity with Claude Code + Codex `/goal`, **shipped first**). **No `/loop` command** (grep-confirmed absent), no machine-off hosted execution, no `WorkspaceStore` secret injection.
- **What's Needed**: (1) a **`/loop <interval|self-paced> <prompt>`** REPL command — cron-cadence re-run **or self-paced loop-until-provably-done**, with auto-expiry + job ID + Esc-to-stop (Claude Code `/loop`; `MAX_ITER≈20` guard); (2) machine-off hosted execution + `WorkspaceStore` secret injection (never env-plaintext) à la Managed Agents; (3) `AutomationsPanel.tsx` + `/routine` + `/loop` surface; 6 BDD scenarios.
- **Effort**: Medium (2-3 weeks — trigger plumbing exists in `automations.rs`; the `/loop` ergonomic + self-pacing loop controller is the new code).

### C3 — MCP Tasks extension + stateless transport (2026-07-28 RC)

- **Source**: Phase 55 P0, FIT-GAP §16.6 (MCP 2026-07-28 spec release candidate)
- **Current State**: `mcp_streamable.rs` ships streamable HTTP + OAuth 2.1; A3 `/.well-known/mcp.json` descriptor shipped (`b13f9106`). Missing the RC's Tasks extension + stateless session model.
- **What's Needed**: Implement the **Tasks extension** (async task IDs + poll/cancel) and the **stateless core** (no server-held state, horizontal-scale-safe); 5 BDD scenarios against RC conformance vectors.
- **Effort**: Medium (2 weeks).

### C6 — ACP + MCP Registry self-listing

- **Source**: Phase 55 P0, FIT-GAP §16.6 (ACP Registry 28+ agents; MCP Registry v0.1 freeze)
- **Current State**: A4 ACP server mode (`acp_stdio.rs`) shipped (`e9dc09af`); VibeCLI is not yet listed in the ACP Registry (Zed + JetBrains) or the MCP Registry.
- **What's Needed**: Register VibeCLI as an ACP agent + publish the daemon / `vibecli-skills-mcp` server in the MCP Registry. Packaging + a registry PR, minimal new runtime code.
- **Effort**: Low (registry submission + manifest; ~2-3 days).

---

## P1 — High Priority (7 items)

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
- **⚠ Escalated 2026-06-13**: Cursor shipped **Design Mode GA on 2026-06-05** — the exact surface (point / draw / narrate UI changes in the browser; agent edits code underneath). The *design proposal* is now overdue; the **build stays gated** on the §18.A7 slice audit.
- **Current State**: Patent-distance posture documented in fit-gap §18 (`403ea1c2`). No implementation yet — must remain distant from the Cursor Design Mode UX.
- **What's Needed**: The §18.A7 cleared shape only — diffcomplete-into-DOM: user clicks an element in their own (CDP-attached) browser, types an instruction, presses ⌘.; agent emits a CSS/HTML unified diff into `DiffReviewPanel`. No agent-controlled browser, no live DOM mutation. Per-feature note in `notes/`. (Also covers **C4 WebMCP** producer/consumer, which reuses this shape.)
- **Effort**: Medium-high (2 wk design + 3 wk impl, patent-distance gated)

### 4. B3 — Always-on security-review agent class

- **Source**: Phase 54 P1 (v0.5.8)
- **⚠ Escalated 2026-06-13**: GitHub Copilot moved code review to an **agentic always-on architecture on 2026-06-01** (runs on GitHub Actions); Cursor Security Review already shipped. The build stays gated on the §18.B3 slice audit.
- **Current State**: `/review` exists as on-demand command. Patent-distance posture documented (close to Cursor Security Review / Copilot agentic-review UX).
- **What's Needed**: §18.B3 cleared shape only — opt-in workspace flag → file-watcher rule → LLM call → `Finding` records (alongside clippy/eslint/semgrep) → existing `ReviewPanel` → user invokes diffcomplete (⌘.) to act. No system-imposed always-on default; no privileged "security agent" canvas. UI surface in `SecurityPanel.tsx`.
- **Effort**: Medium-high (3-4 weeks, patent-distance gated)

### 5. C2 — Dynamic large-scale workflow primitive

- **Source**: Phase 55 P1 (v0.5.9), FIT-GAP §16.6 (Claude Code Dynamic Workflows; Devin Spaces)
- **Current State**: `multi_agent.rs` + `planner.rs` + `nested_agents.rs` + the A8 verify-repair loop (`desktop_agent.rs`) exist but aren't fused into a single self-scaling primitive.
- **What's Needed**: `dynamic_workflow.rs` — auto-decompose a large task, fan out parallel sub-agents over `worktree_pool.rs`, verify each output (test runners / `visual_verify.rs`), report back; tuned for 100k-line migrations. Engineering complement to the P2 100M-line benchmark (which stays the stress-test).
- **Effort**: High (3-4 weeks).

### 6. C4 — WebMCP browser-tool exposure

- **Source**: Phase 55 P1 (v0.5.9), FIT-GAP §16.6 (Google I/O WebMCP, W3C; Chrome 149 origin trial)
- **Current State**: `browser_agent.rs` drives a CDP-attached browser; no WebMCP consume/produce.
- **What's Needed**: (a) **consumer** — discover + call WebMCP-annotated JS/HTML-form tools on authorized sites; (b) **producer** — expose selected VibeUI panels as WebMCP tools. Behind a feature flag while the spec is in origin trial. **Patent-distance gated** — reuses the §18.A7 cleared shape (no live DOM mutation by the agent).
- **Effort**: Medium (2-3 weeks; folds into A7 design work).

### 7. C5 — Per-request effort / compute control knob

- **Source**: Phase 55 P1 (v0.5.9), FIT-GAP §16.6 (Claude Opus 4.8 Effort Control; GPT-5.5 token efficiency)
- **Current State**: No per-request effort tier; `cost_router.rs` routes by task complexity but exposes no user-facing effort knob.
- **What's Needed**: Provider-agnostic `effort: low | medium | high | xhigh` mapped per provider (Opus 4.8 Effort Control, GPT-5.5 reasoning budget, open-model token/step cap), wired through `cost_router.rs` + the toolbar selector; default `high`. Touches every LLM call path. 4 BDD scenarios.
- **Effort**: Low-medium (1-2 weeks, but broad surface).


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
- **Current State**: `extension_compat.rs` covers high-value subset of VS Code extensions. **VS Code hook protocol parity shipped 2026-05-19** (`aeae6c83`) — `vscode-extension/src/hook-executor.ts` mirrors `vibecli-cli/src/hook_abort.rs` and JetBrains `HookExecutor` exactly, `UserPromptSubmit` gated at `startAgent` / `chat` / `inlineEdit` / `sendSelection` / chat-webview entry points, `vibecli.hooks` in the configuration schema.
- **Effort**: Very high (ongoing, diminishing returns)
- **Decision**: Partial coverage is sufficient for most use cases; not pursuing full parity.

### 10. JetBrains Agent Hooks Deep Integration

- **Source**: FIT-GAP v5
- **Current State**: **Meaningful gate coverage shipped 2026-05-19** across four commits — `HookExecutor` service with CLI-parity protocol (`7709bc0b`), `UserPromptSubmit` wired into `AgentPanel.startAgent` (`1ebe79c4`), 14 JUnit tests covering decision semantics / structured-JSON override / chain short-circuit / payload-on-stdin / event allow-list (`a9170448`), and `UserPromptSubmit` wired into `InlineEditAction` (`080bf920`). Both user-driven prompt-submission paths now gate through the configured hook chain; settings table under IDE Settings → Tools → VibeCLI authors hooks per event.
- **What's Needed (nice-to-have polish, not blocking)**: Advisory firings on SSE-arriving `PreToolUse` / `PostToolUse` / `Stop` / `Notification` (the daemon has already run its own pre/post chain by the time these arrive — would double-fire; deferred until a concrete audit-trail use case emerges). Per-project hook scoping on top of the current APP-level scope. Windows PowerShell variant for the `sh -c` invocation. None blocking.
- **Effort**: Closed for the meaningful-parity scope; further work is targeted ~1-day edits when a concrete demand arrives.

---

## Parked (1 item)

### Security CI advisory backlog — triage queue

- **Source**: Surfaced 2026-05-20 by `6ab99683` (cargo-deny-action bump that re-enabled the `cargo metadata` parser). Until that commit, the Security workflow was failing at the install step on every push, so cargo-deny / cargo-audit silently never reached the actual checks. Other security jobs (gitleaks, pip-audit) were also red for separate pre-existing reasons (gitleaks org-license requirement; torch 2.11.0 has 11 PYSEC advisories with no upstream fix).
- **Current State**: Run [26178130526](https://github.com/TuringWorks/vibecody/actions/runs/26178130526) shows 27 distinct RUSTSEC IDs across `cargo deny` + `cargo audit` (mostly unmaintained transitives — `derivative`, `instant`, `paste`, `proc-macro-error`, `rustls-pemfile`, gtk-rs GTK3, `unic-*`, `yaml-rust` — plus one unsound `Buf` and one Marvin-Attack vulnerability). All pre-existing; none introduced by recent commits.
- **What's Needed**: Per-advisory triage decisions in `deny.toml [advisories.ignore]` per the policy already documented there (`reason` + 90-day `expiration`). Quarterly review per the threat-model. Separately: decide whether to license gitleaks ($ org-tier) or migrate to GitHub's native secret scanning.
- **Effort**: 1-2 days of focused triage + dep upgrades where viable.

---

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

**Open code items**: 9 (3 P0 — C1/C3/C6, the Phase 55 trio; 5 P1 — A7/B3 patent-gated + C2/C4/C5; 1 P2 — 100M-line benchmark). A7 and B3 are now **competitor-shipped** (Cursor Design Mode GA 2026-06-05; Copilot agentic review 2026-06-01) — design proposals are overdue, builds still gated on the §18 patent-distance audits.

**Open non-code items**: 4 (SOC 2 cert, managed hosting domain, frontier model, VS Code full compat) — unchanged; all infrastructure / business-process / explicit-design-choice items. Plus the c-series trivial closes (Opus 4.8 / Gemini 3.5 Flash / GPT-5.5 / open-weight model-registry entries; MCP Registry self-listing) — append-only.

**Parked**: 1 (B5 NVFP4 — hardware-blocked).

**Bottom line**: Phase 53 (A1–A11) and Phase 54 P0 (B1, B2, B4, B6 + trivial closes c1, c2) are fully closed. The **v15 competitor delta (2026-06-13)** opened six new gaps (**C1–C6**, Phase 55, v0.5.8/0.5.9) and escalated **A7** + **B3** — both now shipped at competitors (Cursor Design Mode; Copilot agentic review) but held behind their §18 patent-distance audits. Three large design tracks (RL-OS productionization, Recap & Resume, Sandbox tiers) continue landing slice-by-slice in their own design docs rather than here.
