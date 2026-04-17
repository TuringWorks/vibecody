---
layout: page
title: Changelog
permalink: /changelog/
---


All notable changes to VibeCody are documented here. This project follows [Semantic Versioning](https://semver.org/).


## [0.5.5] — 2026-04-17

### Added

- **Apple Watch client** (SwiftUI, watchOS 10+) and **Wear OS client** (Kotlin / Compose, Wear OS 3+) — native `VibeCodyWatch` / `VibeCodyWear` apps with pairing, session list, live transcript, and dictated reply; share a single `/watch/*` backend.
- **`/watch/*` Axum routes** — `/watch/pair/challenge`, `/watch/pair/confirm`, `/watch/sessions`, `/watch/sessions/{id}/stream`, `/watch/sessions/{id}/reply`. New modules: `watch_auth`, `watch_bridge`, `watch_session_relay`.
- **P-256 ECDSA (secp256r1) pairing** — replaces Ed25519 for Apple Secure Enclave compatibility; 64-byte raw public key, signature over `SHA-256(nonce ‖ device_id ‖ issued_at_be)`.
- **URL-only / Bearer pairing everywhere** — no QR code or JSON clipboard required; emulator-friendly.
- **Zero-config connectivity** — mDNS advertising (`_vibecli._tcp.local.`), Tailscale Funnel auto-detection, ngrok auto-detect + opt-in auto-start. Clients race all reachable paths.
- **Apple-Handoff-style session continuity** — paired devices see live sessions in real time; VibeUI auto-switches to the Sandbox tab when a watch joins.
- **Google-Docs-style real-time sync** — ID-based message reconciliation with content-window dedup; no more 80/512-char truncation.
- **Watch Devices panel** in VibeUI (`Governance → Watch Devices`) to approve / rename / revoke devices.
- **CI release artifacts** — `VibeCodyWatch-watchOS.app.zip` + `VibeCodyWear-wearos.apk` / `.aab` alongside existing binaries.
- **Makefile targets** — `build-watch`, `watch-ios`, `watch-wear`, `watch-wear-bundle`, `build-all`.

### Fixed

- **80 / 512-char message truncation** — the legacy ring-buffer fallback was replaced with full-content sync.
- **DMG bundling race on macOS 15** — hardened fallback against transient `hdiutil attach` failures under concurrent DiskImages2 load.
- **Emulator pairing** — pairing now works with a pasted URL + Bearer token on Android emulators and watchOS simulators.

### Changed

- **Pairing algorithm**: Ed25519 → P-256 ECDSA. Previously-paired devices must re-pair once on v0.5.5.
- **Watch / phone auth** — JWT (HS256), 32-byte secret in `ProfileStore`, 30-day default TTL.
- Version bumped to 0.5.5 across all manifests.

---

## [0.5.4] — 2026-04-03

### Added

- **Claude Code System Prompts** — integrated 254 prompts from TuringWorks/claude-code-system-prompts: core behavioral guidelines baked into TOOL_SYSTEM_PROMPT; all prompts stored as reference skills in `skills/claude-code-prompts/`.
- **Auto-mode guidance** — when FullAuto approval policy is active, agent receives autonomous execution rules.
- **Error Boundary** — React ErrorBoundary catches render crashes with error + stack trace display.
- **5 dynamic skill files** — git-commit, pr-creation, security-review, debugging, simplify.
- **WebView DevTools** — auto-open in debug builds for crash diagnosis.

### Fixed

- **GLM/Qwen tool call parsing** — normalize `<|tag|>` delimiters so XML tool calls are correctly executed.
- **Incremental file saves during streaming** — `<write_file>` blocks flushed to disk as closing tag streams in.
- **Leading newline in generated files** — strip `\n` after `<write_file path="...">`.
- **`<build>` and `<run>` tag variants** — recognize block form in addition to self-closing.
- **Apply crash** — DiffReviewPanel overlays editor with deferred unmount; removed React.StrictMode.
- **Terminal buffer cleared on tab switch** — Terminal stays mounted with display toggle.
- **Duplicate provider keys** — 14 providers now return unique `"Provider (model)"` names.
- **LSP invoke params** — fixed snake_case to camelCase field names for hover, completion, goto-definition.
- **Diff review toolbar** — thinner, outlined ghost buttons, visible text with ellipsis.
- **Tool call card icons** — replaced emoji with thin-line SVG icons using CSS variables.

### Changed

- Agent context window: 80K → 200K tokens; max_steps: 30 → 50.
- Claude max_tokens: 4,096 → 16,384; Ollama num_predict: 2,048 → 16,384.
- Retry attempts: 4 → 2 (500ms initial, 5s max backoff).
- Ollama HTTP timeout: 90s → 300s.

---

## [0.5.3] — 2026-04-02

### Added

- **Document & Media Viewers** — DocumentViewer, ImageViewer, HtmlPreview, DrawioPreview for VibeUI.
- **Per-Provider Model Lists** — provider-appropriate models with auto-selection; Ollama uses live-discovered models.
- **RL-OS Core Modules** — 8 modules (EnvOS, TrainOS, EvalOS, OptiOS, ModelHub, ServeOS, RLHF, MultiAgent) with 660 tests, 10 panels, 20 Tauri commands.
- **Sketch Canvas** — drawing with Move tool, inline text, SVG/PNG export, shape recognition, code generation.
- **Training Run Wizard** — step-by-step RL training setup wizard.

### Fixed

- **Vibe App: Empty AI Responses** — SSE parser read `ev["text"]` but daemon sends `ev["content"]`.
- **Vibe App: Duplicate Streaming Text** — guarded against React StrictMode double-mount race.
- **Vibe App: Response Never Completing** — fallback completion event on agent exit.
- **Vibe App: Stale Model/Token** — `useCallback` dependencies updated.
- **Vibe App: Window Icon** — replaced default Tauri icon with VibeUI icon.
- **Ollama Model List Slow/Missing** — removed per-model chat probe; instant name-based filter.
- **Monaco Crash on Apply All** — editor kept always mounted.
- **VibeUI Panel Bugs** — TLS Inspector, Design Mode, Screenshot to App, file explorer, Fast Context, SemanticIndexPanel.

### Changed

- **Agent Identity** — renamed from "VibeCLI" to "Vibe Agent" across all system prompts.
- RL-OS composite panels registered in panel host, tab groups, and search.

---

## [0.5.2] — 2026-03-30

### Added

- **RL-OS: Unified Reinforcement Learning Lifecycle Platform** — exhaustive fit-gap analysis against 40+ RL competitors (Ray RLlib, Stable Baselines3, Isaac Lab, TRL, d3rlpy, PettingZoo, SageMaker RL, etc.) identifying 52 gaps across 8 categories and 12 unique capabilities no existing tool provides.
- **RL-OS Architecture Specification** — production-grade architecture for 7 core modules (EnvOS, TrainOS, EvalOS, OptiOS, ModelHub, ServeOS, RLHF) with Rust crate structure (`vibe-rl/`), declarative YAML DSL, RL-aware quantization, and 8-phase roadmap.
- **12-Stage RL Lifecycle Scorecard** — comprehensive lifecycle coverage model; closest competitor scores 5/12 vs. RL-OS target of 12/12.

---

## [0.5.1] — 2026-03-29

### Added

- **AI Code Review** (`ai_code_review.rs`, 97 tests) — Qodo/CodeRabbit/Bito parity: 7 detectors, 8-linter aggregation, quality gates, learning loop, PR summary + Mermaid diagrams; `/aireview` REPL.
- **Architecture Spec Engine** (`architecture_spec.rs`, 108 tests) — TOGAF ADM, Zachman, C4 Model, ADRs, governance engine; `/archspec` REPL.
- **Policy Engine** (`policy_engine.rs`, 91 tests) — Cerbos-style RBAC/ABAC, 14 condition operators, derived roles, policy testing, YAML, audit trail; `/policy` REPL.
- **Health Score** (`health_score.rs`, 92 tests) — multi-dimensional codebase health scoring.
- **Intent Refactor** (`intent_refactor.rs`, 89 tests) — natural-language-driven refactoring.
- **Review Protocol** (`review_protocol.rs`, 50 tests) — structured code review workflow.
- **Skill Distillation** (`skill_distillation.rs`, 82 tests) — extract reusable skills from agent traces.
- **Phase 32 P0** — context_protocol, code_review_agent, diff_review, code_replay, speculative_exec, explainable_agent.
- **TurboQuant KV-Cache** — PolarQuant + QJL (~3 bits/dim) for vector DB integration.
- **Phase 32 — Advanced Agent Intelligence** (6 new modules):
  - `context_protocol.rs` — Streaming context protocol for long-running agent sessions.
  - `code_review_agent.rs` — Automated code review with configurable rulesets.
  - `diff_review.rs` — Change-aware review focused on diff hunks.
  - `code_replay.rs` — Reproduce past interactions for debugging and auditing.
  - `speculative_exec.rs` — Predictive code path execution.
  - `explainable_agent.rs` — Interpretable reasoning chain for agent decisions.

- **FIT-GAP v7 — All 22 Gaps Closed** (Phases 23-31):
  - Phase 23: `a2a_protocol.rs` (A2A protocol), `agent_skills_compat.rs` (cross-tool skills standard).
  - Phase 24: `worktree_pool.rs` (parallel worktree agents), `agent_host.rs` (multi-agent terminal host).
  - Phase 25: `proactive_agent.rs` (background intelligence scanner), `issue_triage.rs` (autonomous issue classification).
  - Phase 26: `web_grounding.rs` (5-provider web search grounding), `semantic_index.rs` (AST-level codebase understanding).
  - Phase 27: `mcp_streamable.rs` (Streamable HTTP + OAuth 2.1).
  - Phase 28: `mcts_repair.rs` (MCTS code repair), `cost_router.rs` (cost-optimized agent routing).
  - Phase 29: `visual_verify.rs` (UI screenshot verification), `next_task.rs` (workflow-level prediction), `voice_local.rs` (offline whisper.cpp voice), `doc_sync.rs` (bidirectional spec-code sync).
  - Phase 30: `native_connectors.rs` (20 service connectors), `agent_analytics.rs` (enterprise metrics), `agent_trust.rs` (trust scoring), `smart_deps.rs` (agentic package manager).
  - Phase 31: `rlcef_loop.rs` (execution-based learning), `langgraph_bridge.rs` (LangGraph compatibility), `sketch_canvas.rs` (sketch-to-code).

- **File Attachments** — `[file.ext]` bracket syntax in VibeCLI REPL and VibeUI chat for attaching documents, code, and images.
- **Image Lightbox** — Click image attachments in chat to view full size with download button.
- **System Theme Detection** — `ThemeToggle` now respects `prefers-color-scheme` on first visit.
- **Data Analysis Panel Backend** — 9 new `da_*` Tauri commands.
- **Counsel — Multi-LLM Deliberation** (`counsel.rs`, 534 lines, 20+ tests).
- **SuperBrain — Multi-Provider Query Routing** (`superbrain.rs`, 424 lines, 14+ tests).
- **Web Client** (`web_client.rs`, 1,048 lines) — zero CDN dependencies (air-gap safe).
- FIT-GAP Code Review Architecture comparison across 12+ competitors.
- VibeCody vs OpenClaw whitepaper.
- Demo guides 36-60.
- 3 VibeUI composite panels, 7 skill files, 10 new Tauri commands.

### Changed

- **Zero Demo Panels** — All 23 previously demo-only panels wired to real Tauri backends (34 new commands, 17 new AppState fields). Panel status: 196+ total.
- **Theme Variable Migration** — Converted 85+ hardcoded colors to CSS variables.
- Tests: ~10,535 (0 failures). REPL commands: 106+. Rust modules: 196+. Skill files: ~550. Tauri commands: 360+.
- Documentation: FIT-GAP through v7, ROADMAP through v5.
- Provider count: 23 direct + OpenRouter (300+).

### Fixed

- **Production Hardening** — Zero compiler warnings. Safe unwraps, flush-on-exit, configurable A2A server, poison recovery for Mutex locks.
- **Clippy Clean** — All lints resolved across workspace.
- **Tokio Mutex Fix** — 45 instances corrected.
- **Crate Metadata** — Added `description` field to 6 Cargo.toml files.
- **Ollama Streaming** — Status check fix + streaming hot path optimization.
- Suppressed warnings in ai_code_review, architecture_spec, diff_review modules.
- Duplicate REPL handlers removed; missing module stubs created.

---

## [0.5.0] - 2026-03-24

### Added

- **9 Quantum Computing Tools**:
  - **Statevector Simulator** — pure Rust simulator supporting up to 16 qubits with all 14 quantum gates (H, X, Y, Z, S, T, Rx, Ry, Rz, CNOT, CZ, SWAP, Toffoli, Measure). Complex number arithmetic, probability extraction, amplitude readout, and shot-based sampling.
  - **Visual Circuit Builder** — SVG-based editor with categorized gate palette, click-to-place on qubit wires, multi-qubit gate workflow (control then target), click-to-delete, and live metrics bar (gate count, depth, 2Q gates, circuit volume).
  - **Circuit Optimizer** — multi-pass optimization: identity cancellation (HH, XX, YY, ZZ, CNOT pairs), gate merging (SS to Z, TT to S), rotation merging (adjacent Rx/Ry/Rz on same qubit), with savings percentage reporting.
  - **Bloch Sphere Visualizer** — SVG rendering of single-qubit states with oblique projection, axis labels, state arrow, and theta/phi readout.
  - **Cost Estimator** — pricing comparison for IBM Quantum ($1.60/sec), Amazon Braket ($0.30/task + per-shot), and IonQ (per-gate) with itemized breakdowns.
  - **Project Scaffolding** — complete project generation for Qiskit, Cirq, PennyLane, and Q# with source, tests, requirements, CI config, and README.
  - **Algorithm Templates** — 8 pre-built circuits: Bell State, GHZ(n), QFT(n), Grover 2-qubit, Deutsch-Jozsa, Bernstein-Vazirani, VQE ansatz, QAOA.
  - **Hardware Topology Viewer** — SVG connectivity maps for IBM Eagle (127q), Google Sycamore (53q), IonQ Aria (25q), Rigetti Ankaa-2 (84q), Quantinuum H2 (32q).
  - **Multi-language Code Examples** — 11 algorithms with implementations in Qiskit, Cirq, and PennyLane (Grover, Shor, VQE, QAOA, QPE, Deutsch-Jozsa, BV, HHL, Quantum Walk, QSVM, QNN).
- **Panel Consolidation** (137 tabs to 36):
  - 33 composite panels replacing 137 individual tabs, organized into 9 renamed groups (AI, Project, Code Quality, Source Control, Infrastructure, Data & APIs, Developer Tools, Toolkit, Settings).
  - Reusable `TabbedPanel` component with keep-alive behavior for sub-tabs.
  - `createComposite()` factory for one-liner composite panel definitions.
  - Alias-based search — typing old panel names (e.g., "docker") still finds the consolidated tab ("Containers").
- **Full-stack Resilience**:
  - `ResilientProvider` wrapper — automatic retry with exponential backoff and jitter on all 21 AI providers.
  - `retry_async()` generic utility for any async operation with configurable max attempts, backoff, and error classification.
  - `is_retryable()` classifier covering 20+ transient error patterns (429, 503, timeouts, connection resets, decode errors).
  - Agent loop: stream-level retry (5 attempts, 1-60s backoff), `RetryableError` event, frontend Retry button preserving completed work.
  - Streaming chat: full retry loop with mid-stream error recovery.
  - 30+ HTTP API calls wrapped with retry: JIRA, GitHub, Linear, Groq Whisper, ElevenLabs, Telegram, Discord, Slack, Signal, Matrix, Twilio, WhatsApp, Teams, OpenSandbox (all operations), BugBot.
- 11 new Tauri commands for quantum operations (add/remove gate, simulate, optimize, cost estimate, templates, scaffold, circuit detail/delete/clear).
- 105 quantum computing tests (32 new for simulator, optimizer, templates, cost, scaffold).

### Fixed

- Quantum circuit lookup uses `index` field instead of array position (circuits remained accessible after deletions).
- Missing `gates` array in circuit detail for pre-existing circuits (defaults to empty).
- Quantum simulator returns tuple arrays matching frontend TypeScript types.
- `TabbedPanel` display:contents breaking child panel height.
- `LazyPanels` props aligned with refactored `createComposite` pattern.

### Changed

- Quantum panel expanded from 6 tabs to 11 (Circuit Builder, Simulator, Optimizer, Cost, Templates, Scaffold, Topology, Languages, Quantum OS, Projects, Algorithms).
- Version bumped to 0.5.0 across all manifests (Cargo.toml, package.json, tauri.conf.json).
- VibeCLI crate now also builds as a library (`vibecli_cli`) for Tauri backend integration.
- Canvas workflow panel properties sidebar added.
- Tab labels corrected for Red Team, Blue Team, Purple Team.
- AI/ML Workflow and Model Wizard added to tab groups.


## [0.4.0] - 2026-03-21

### Added

- **Warp terminal-style features** (`warp_features.rs`, 55 tests):
  - `# natural language` command — type `# find large files` to generate shell commands via AI with explanation and confirmation.
  - Command corrections (thefuck-style) — 13 built-in rules for typos (`gti` to `git`), missing sudo, git push upstream, permission denied, wrong Python version, etc.
  - Secret redaction — auto-detects and masks API keys (`sk-****`), AWS keys (`AKIA****`), GitHub tokens (`ghp_****`), Bearer tokens, passwords, and private keys in command output.
  - Next command suggestions — proactive hints after successful commands (git add to git commit, cargo build to cargo test, etc.).
  - Block-style output — shell command output formatted with colored left border (green=success, red=failure), command header, and duration display.
  - Desktop notifications — macOS/Linux notifications for commands taking longer than 30 seconds.
  - Output filtering and AI error explanation prompts.
- **Auth scaffolding expanded to 85+ frameworks across 17 languages** — Go (Gin, Fiber, Echo, Chi, Hertz), Java (Spring Boot, Quarkus, Micronaut, Vert.x, Helidon, Javalin), Kotlin (Ktor, http4k), C# (ASP.NET Core, FastEndpoints), TypeScript (Next.js, Fastify, NestJS, Hono, Elysia), Python (FastAPI, Django, Flask, Starlette, Litestar), Rust (Axum, Actix, Rocket), Ruby (Rails, Sinatra), PHP (Laravel, Symfony), Elixir (Phoenix), Scala, Swift, Dart, Clojure, Haskell, Crystal, Nim, Zig. Auth providers expanded to 40+ including SAML, LDAP, OIDC, Passkey, TOTP, and 10 BaaS platforms. UI: searchable grid with language filter.
- **Best-in-class documentation** (20 new files, ~5,500 lines):
  - `llms.txt` and `llms-full.txt` — AI-agent-optimized project docs following the llms.txt standard. First AI coding tool to support this.
  - `quickstart.md` — zero-to-productive in 5 minutes.
  - 3 tutorials: first-provider setup, agent workflow, AI code review.
  - `api-reference.md` — complete HTTP daemon API reference with curl examples for all endpoints.
  - Per-provider setup guides: Ollama, Claude, OpenAI, DeepSeek, Gemini.
  - `troubleshooting.md` (24 issues), `faq.md` (22 questions), `glossary.md` (50+ terms), `security.md` (13 sections), `CHANGELOG.md`.
  - Jekyll nav reorganized: quickstart-first user journey ordering.
- **Full ANSI markdown rendering in VibeCLI REPL**:
  - Headers (H1-H4) in bold green/cyan/magenta/blue.
  - Bold, italic, bold+italic text styling.
  - Inline code with gray background and cyan text.
  - Unordered and ordered lists with styled bullets/numbers.
  - Blockquotes with green pipe and italic text.
  - Task lists with checkbox symbols.
  - Horizontal rules, links with underlined text and dim URLs.
  - Code blocks with line numbers, language labels, dark background, and syntect syntax highlighting.
- **Claude Code-style tool call rendering** — dark background boxes with terminal-width padding, green checkmark or red cross, tool output displayed below (capped at 30 lines).
- **MCP panels consolidated** — merged MCP, MCP Lazy, and MCP Directory into a single unified panel with 4 tabs (Servers, Tools, Directory, Metrics).
- **Model name in REPL prompt** — `[vibecli ollama (deepseek-chat)] >` shows both provider and model.

### Fixed

- All GitHub URLs corrected from `vibecody/vibecody`, `AceCana662/vibecody`, `AiChefDev/vibecody` to `TuringWorks/vibecody` across 18 files (docs, GitHub Actions, package.json, config).
- DeepSeek default model: `deepseek-coder` to `deepseek-chat` (V3 current).
- Gemini default model: `gemini-2.0-flash` to `gemini-2.5-flash` (latest).
- Streaming chat response rendering — replaced flawed stream-then-clear-then-rerender with direct rendering, eliminating blank line artifacts.
- Rustyline prompt double-bracket and cursor offset — switched to plain text prompt for reliable cursor positioning.
- Tool output now displayed in agent REPL (was captured but not shown).
- REPL args trimming — extra spaces after commands removed.
- All decorative emojis removed from REPL output (70+) and documentation (1,138 replacements across 14 files).
- Build warnings resolved (zero warnings, zero errors).
- `.vibecli/` added to `.gitignore` (auto-generated local data).

### Changed

- **23 direct AI providers** (was 17): added MiniMax, Perplexity, Together AI, Fireworks AI, SambaNova, plus Gemini provider upgrade.
- Cost estimation expanded to cover all 23 providers with per-model pricing (was only Claude + OpenAI).
- Doctor command checks all 14 cloud provider API keys (was 4).
- Help text reorganized by popularity with all 23 providers listed.
- 55 new unit tests (warp_features), 130 new provider tests, 812 gap-closure tests.
- Documentation icons replaced with plain text (Yes/No/Warning instead of emoji checkmarks).


## [0.3.3] - 2026-03-20

### Added

- 5 new AI providers: MiniMax, Perplexity, Together AI, Fireworks AI, SambaNova — bringing the total to 23 supported providers.
- FIT-GAP v6: 19 new competitive gaps identified and closed across agent capabilities, context management, and cloud integrations.
- 17 new Rust modules:
  - `channel_daemon.rs` — Always-on background listener for multi-platform integration.
  - `vm_orchestrator.rs` — Virtual machine lifecycle management for cloud sandboxes.
  - `spec_pipeline.rs` — Spec-driven development pipeline with EARS syntax support.
  - `branch_agent.rs` — Autonomous branch management with PR creation workflows.
  - `design_import.rs` — Figma/Sketch design-to-code import pipeline.
  - `audio_output.rs` — Text-to-speech for agent responses and accessibility.
  - `org_context.rs` — Organization-wide context sharing across teams.
  - `session_sharing.rs` — Share agent sessions with teammates via link or export.
  - `ci_gates.rs` — Quality gates for CI pipelines with configurable thresholds.
  - `data_analysis.rs` — Tabular data analysis with chart generation.
  - `managed_deploy.rs` — One-click deploy to Vercel, Netlify, Railway, Fly.io.
  - `context_streaming.rs` — Streaming context injection for long-running sessions.
  - `extension_compat.rs` — Extension compatibility verification and migration.
  - `model_marketplace.rs` — Browse and install models from community marketplace.
  - `agentic_cicd.rs` — AI-driven CI/CD pipeline generation and optimization.
  - `cross_surface_routing.rs` — Route agent actions across CLI, UI, and API surfaces.
  - `soul.rs` — Project philosophy document management with agent integration.
- 10 new VibeUI panels: Soul, McpLazy, ContextBundle, CloudProvider, ACP, McpDirectory, UsageMetering, SweBench, SessionMemory, IDP.
- Gemini provider upgraded to native implementation (previously OpenRouter-only).
- Best-in-class support documentation: troubleshooting guide, FAQ, glossary, security practices, and this changelog.
- `llms.txt` for AI-friendly project context.
- Tutorial guides for getting started, provider configuration, and skill development.

### Fixed

- DeepSeek default model updated from deprecated `deepseek-coder` to `deepseek-chat` (V3).
- Gemini default model updated to `gemini-2.5-flash`.
- Cost estimation now covers all 23 providers (previously only Claude and OpenAI).
- Doctor command checks all 14 cloud provider API keys (previously only 4).
- Session resume stability improved for cross-version session files.
- Monaco editor performance with files over 1 MB (disabled minimap by default for large files).

### Changed

- Provider help text reorganized by popularity tier (Local, Major Cloud, Specialized, Meta).
- 812 new unit tests across 17 modules, bringing the workspace total to approximately 6,050.
- All production `unwrap()` calls replaced with `expect()` with descriptive messages.
- Release profile optimized: LTO enabled, symbols stripped, panic set to abort, opt-level=s for workspace with opt-level=2 for vibecli.


## [0.3.2] - 2026-03-14

### Added

- **Blue Team** module (`blue_team.rs`, 49 tests) — Defensive security operations: incident management with P1-P4 severity, IOC tracking across 9 indicator types, SIEM integration for 8 platforms (Splunk, Sentinel, Elastic, QRadar, CrowdStrike, Wazuh, Datadog, SumoLogic), forensic case management, detection rules with platform-specific query generation, playbooks with 8 action types, and threat hunting workflows.
- **Purple Team** module (`purple_team.rs`, 38 tests) — ATT&CK-aligned security exercises: 14 tactics, 20 pre-loaded techniques, attack simulation with outcome tracking, detection validation, coverage gap analysis, heatmap generation, and cross-exercise comparison.
- **IDP** module (`idp.rs`, 80 tests) — Internal Developer Platform support for 12 platforms: Backstage, Cycloid, Humanitec, Port, Qovery, Mia Platform, OpsLevel, Roadie, Cortex, Morpheus Data, CloudBolt, Harness. Includes service catalogs, golden paths, DORA-metric scorecards, self-service infrastructure provisioning, and team onboarding.
- 3 new VibeUI panels: BlueTeamPanel (7 tabs), PurpleTeamPanel (5 tabs), IdpPanel (7 tabs).
- 3 new REPL commands: `/blueteam`, `/purpleteam`, `/idp` with full subcommand sets.
- Workspace total: approximately 5,912 tests with 0 failures.


## [0.3.1] - 2026-03-13

### Added

- **Futureproofing Phases 10-14**: 10 new Rust modules implementing 12 FIT-GAP v5 gaps (419 tests total):
  - MCP lazy loading with tool search and LRU eviction.
  - Context bundles (Spaces) with priority ordering and TOML serialization.
  - AWS/GCP/Azure deep integration: service detection, IAM policy generation, Terraform/CloudFormation/Pulumi templates, cost estimation.
  - ACP (Agent Client Protocol) server/client modes with capability negotiation.
  - MCP verified plugin directory with search, install, and review pipeline.
  - Usage metering credit system with per-user/project/team budgets and alerts.
  - SWE-bench benchmarking harness for run/compare/export.
  - Session memory profiling with leak detection and auto-compact.
  - SOC 2 compliance controls with audit trail, PII redaction, and retention policies.
  - Unified voice+vision+code multimodal agent.
- 8 new VibeUI panels and 4 new REPL commands.
- 12 v5 fit-gap competitive gaps catalogued and Phases 10–14 planned (now consolidated into [Fit-Gap Analysis](./fit-gap-analysis/) and [Roadmap](./roadmap/)).
- Workspace total: approximately 5,745 tests with 0 failures and 136+ panels.


## [0.3.0] - 2026-03-09

### Added

- **FIT-GAP v4**: All 23 identified gaps closed, including automations, self-review, MCP apps, agent teams v2, semantic MCP, docgen, remote control, AST editing, CI status checks, VS Code sessions, cloud sandbox, plan documents, security scanning, sub-agent roles, and edit prediction (RL Q-learning).
- **Competitor Parity**: 13 new modules closing all code-addressable "Partial" entries from competitive analysis — debug mode, three agent modes (Smart/Rush/Deep), conversational search, clarifying questions, fast context (SWE-grep), image generation agent, discussion mode, full-stack generation, enhanced agent teams, team governance, cloud autofix, GitHub Actions agent, and render optimization.
- **App Builder** (`app_builder.rs`, 70 tests) — Template-based application scaffolding with AI enhancement.
- **Infinite Context** (`infinite_context.rs`, 79 tests) — 5-level context hierarchy with token budget, eviction, compression, and LRU caching.
- **Blitzy Parity**: Batch builder (109 tests), QA validation pipeline (99 tests), legacy migration engine supporting 18 source languages including COBOL and Fortran (101 tests), and unified git platform manager for 5 hosting services (111 tests).
- 13 new skill files and 4 new VibeUI panels.
- Workspace total: approximately 5,236 tests with 0 failures.

### Changed

- Security audit completed: 20 findings (P0-P3) all resolved, including path traversal prevention, cryptographic IDs, CORS hardening, and command blocklist.
- All production `unwrap()` calls replaced with `expect()` with descriptive messages.
- Release profile added to workspace: LTO, symbol stripping, panic=abort.


## [0.2.x] - 2026-02 through 2026-03

Earlier releases established the foundation:

- Core agent loop with tool calling, streaming, and multi-provider support.
- VibeCLI with TUI (Ratatui) and REPL (Rustyline).
- VibeUI with Tauri 2, React, Monaco Editor.
- 17 AI providers including Ollama, Claude, OpenAI, Gemini, and FailoverProvider.
- MCP client and server support.
- Container sandbox with Docker/Podman/OpenSandbox.
- 500+ built-in skills across 25+ categories.
- RAG pipeline with document ingestion, web crawling, and vector database support.
- Gateway system supporting 18 messaging platforms.
- Voice input (Groq Whisper), pairing (QR code), and Tailscale integration.

See the [Roadmap](/vibecody/roadmap/) for the complete feature history.
