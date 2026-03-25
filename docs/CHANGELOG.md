---
layout: page
title: Changelog
permalink: /changelog/
---


All notable changes to VibeCody are documented here. This project follows [Semantic Versioning](https://semver.org/).


## [Unreleased]

### Added

- **Counsel — Multi-LLM Deliberation** (`counsel.rs`, 534 lines, 20+ tests):
  - Structured multi-round debates between AI providers with 6 role-based personas (Expert, Devil's Advocate, Skeptic, Creative, Pragmatist, Researcher).
  - User interjection between rounds, voting system, moderator-driven synthesis.
  - Session persistence at `~/.vibecli/counsel/sessions.json`.
  - 7 Tauri commands, CounselPanel.tsx with setup/rounds/synthesis tabs.
  - REPL commands: `/counsel new|run|inject|synthesize|vote|list|show`.

- **SuperBrain — Multi-Provider Query Routing** (`superbrain.rs`, 424 lines, 14+ tests):
  - 5 routing modes: SmartRouter (keyword-based), Consensus, ChainRelay (sequential refinement), BestOfN (judge picks winner), Specialist (subtask decomposition).
  - Keyword-based confidence scoring, configurable routing rules.
  - 3 Tauri commands, REPL commands: `/superbrain`, `/superbrain consensus|chain|best|specialist|modes`.

- **Web Client** (`web_client.rs`, 1,048 lines):
  - Self-contained browser-based SPA served from `vibecli serve` — zero external CDN dependencies (air-gap safe).
  - Chat and Agent modes with SSE streaming, markdown rendering, syntax highlighting.
  - Dark/light theme, responsive design, file upload, keyboard shortcuts.

### Changed

- Documentation updated: panel listing expanded from 90 to 162 entries across 16 categories.
- Provider count corrected to 23 across all documentation (added MiniMax, Perplexity, Together AI, Fireworks AI, SambaNova).
- Test count updated to ~6,628+ across all documentation references.

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

- **23 direct AI providers** (was 17): added MiniMax, Perplexity, Together AI, Fireworks AI, SambaNova, plus Gemini native upgrade.
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
- FIT-GAP-ANALYSIS-v5.md and ROADMAP-v3.md published.
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
