# Changelog

All notable changes to this project are documented here.
Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

---

## [Unreleased]

### Added
- **Phase 43**: CRDT multiplayer collaboration — new `vibe-collab` crate powered by `yrs` (Yjs
  Rust port) + `dashmap` concurrent room registry; `CollabServer` manages rooms, `CollabRoom`
  holds a `Y.Doc` per room with per-file `Y.Text` and broadcast fan-out; Yjs binary sync protocol
  (SyncStep1/SyncStep2/Update) over Axum 0.7 WebSocket; `AwarenessState` for cursor tracking with
  8-color peer palette; `serve.rs` gains `/ws/collab/:room_id` WebSocket handler (token auth via
  query param) + REST endpoints (`POST /collab/rooms`, `GET /collab/rooms`,
  `GET /collab/rooms/:room_id/peers`); 5 Tauri commands (`create_collab_session`,
  `join_collab_session`, `leave_collab_session`, `list_collab_peers`, `get_collab_status`);
  `CollabPanel.tsx` (create/join room, peer list with color indicators, copy invite link, leave
  session); `useCollab.ts` React hook for WebSocket lifecycle and awareness state; "👥 Collab"
  25th AI panel tab; `yjs`, `y-monaco`, `y-websocket` npm dependencies added; 15 unit tests
  (room lifecycle, peer management, Y.Doc sync convergence, incremental updates).
- **Phase 43**: Test runner system — `detect_test_framework` auto-detects Cargo/npm/pytest/Go
  from project files; `run_tests` Tauri command spawns test subprocess and streams `test:log`
  events to the frontend; parses `cargo test --message-format=json`, pytest `-v`, and go test
  `-v` output into structured `TestRunResult` with per-test details; `TestPanel.tsx` (🧪 Tests
  AI panel tab) shows framework badge, ▶ Run Tests button, custom command input, pass-rate
  progress bar, pass/fail/ignored counts, per-test rows with colored status icons and expandable
  output; `/test [command]` REPL command in VibeCLI auto-detects and runs tests.
- **Phase 43**: AI commit message generation — `generate_commit_message` Tauri command runs
  `git diff --staged`, feeds diff to the active AI provider, returns an imperative one-liner
  commit message; "✨ AI" button in `GitPanel.tsx` fills the commit textarea on click.
- **Phase 43**: `TestPanel.tsx` — full test runner UI in a new "🧪 Tests" AI panel tab; shows
  framework badge (Cargo/npm/pytest/Go), ▶ Run Tests button, custom command input, pass-rate
  progress bar, pass/fail/ignored counts, per-test rows with colored status icons and expandable
  output, and live log stream during execution.
- **Phase 43**: `detect_test_framework` Tauri command — auto-detects Cargo.toml → cargo,
  package.json (with `test` script) → npm/yarn/bun, pytest.ini/pyproject.toml → pytest,
  go.mod → go test.
- **Phase 43**: `run_tests` Tauri command — spawns test subprocess, streams `test:log` events
  to the frontend, parses `cargo test --message-format=json` (name/event/exec_time/stdout),
  pytest `-v` (PASSED/FAILED line patterns), and go test `-v` (--- PASS/FAIL: lines), returns
  `TestRunResult` with summary counts and per-test details.
- **Phase 43**: `generate_commit_message` Tauri command — runs `git diff --staged --stat` +
  `git diff --staged --unified=3`, feeds diff to the active AI provider with a concise prompt,
  returns the AI-generated one-liner message.
- **Phase 43**: "✨ AI" button overlaid on the commit message textarea in `GitPanel.tsx` —
  calls `generate_commit_message` and fills the textarea on success.
- **Phase 43**: `/test` REPL command in VibeCLI — auto-detects test framework from CWD and
  runs tests; accepts an optional custom command override; added to COMMANDS array with hint
  "[command]  — run project tests (auto-detects cargo/npm/pytest/go)".
- **Phase 42**: `@jira:PROJECT-123` context in both VibeCLI (`expand_at_refs`) and VibeUI
  (`resolve_at_references`): fetches Jira issue summary, status, assignee, and description
  via REST API v2; uses `JIRA_BASE_URL` + `JIRA_EMAIL` + `JIRA_API_TOKEN` env vars;
  `re_at_jira()` OnceLock regex + `JiraIssue`/`JiraFields` Deserialize types;
  `ContextPicker.tsx` autocompletes `@jira:` with a dynamic hint `PROJ-123`; file-search
  skipped for `jira:` prefix.
- **Phase 42**: MCP OAuth install flow in `McpPanel.tsx` — each server gains an "OAuth" button
  that opens a two-step modal: enter Client ID / Auth URL / Token URL / Scopes → "Open Browser"
  launches the OAuth authorization URL; paste the authorization code back to complete the token
  exchange; token stored at `~/.vibeui/mcp-tokens.json`; green `🔑 OAuth` badge on connected
  servers; three new Tauri commands: `initiate_mcp_oauth` (URL builder + system browser),
  `complete_mcp_oauth` (code exchange + persist), `get_mcp_token_status` (expiry check);
  `url.workspace = true` added to `vibeui/src-tauri/Cargo.toml`.
- **Phase 42**: Custom domain / publish in `DeployPanel.tsx` — "🌐 Custom Domain" input below
  the deploy button; `set_custom_domain` Tauri command returns per-provider DNS instructions:
  Vercel calls the REST API (requires `VERCEL_TOKEN`); Netlify/Railway/GitHub Pages/GCP Cloud
  Run/Firebase Hosting return CNAME record instructions; result rendered in a pre block.
- **Phase 40**: Code Complete workflow system (`workflow.rs`) — 8-stage development pipeline
  inspired by Steve McConnell's *Code Complete*: Requirements → Architecture → Design →
  Construction Planning → Coding → Quality Assurance → Integration & Testing → Code Complete;
  workflows stored as YAML front-matter markdown files in `.vibecli/workflows/`; `/workflow`
  REPL command with `new|list|show|advance|check|generate` sub-commands; `/workflow generate`
  uses LLM to populate the checklist for the current stage; `progress_pct()` shown in
  `/workflow show` stage summary; TUI tab-completion for all sub-commands; 11 unit tests;
  127 tests passing total.
- **Phase 41**: Red Team security testing module (`redteam.rs`) — autonomous 5-stage pentest
  pipeline (Recon → Analysis → Exploitation → Validation → Report); 15 attack vectors including
  SQL injection, XSS, SSRF, IDOR, path traversal, auth bypass; `run_recon()`, `analyze_recon()`,
  `exploit_candidate()` async stages; `RedTeamManager` with JSON-persisted sessions at
  `~/.vibecli/redteam/`; `/redteam scan|list|show|report|config` REPL commands; `--redteam`
  CLI flag; `start_redteam_scan` VibeUI Tauri command; `RedTeamCfg` in `config.rs`
  (`max_depth`, `timeout_secs`, `parallel_agents`, `auto_report`).
- **Phase 41**: Extended `detect_security_patterns()` in `bugbot.rs` with 8 additional CWE patterns:
  CWE-918 (SSRF), CWE-611 (XXE), CWE-502 (insecure deserialization), CWE-943 (NoSQL injection),
  CWE-1336 (template injection), CWE-639 (IDOR), CWE-352 (missing CSRF), CWE-319 (cleartext
  transmission); total 15 vulnerability patterns; `RedTeamPanel.tsx` added as 🛡️ RedTeam tab in
  VibeUI AI panel; `docs/SHANNON-COMPARISON.md` feature comparison document.
- **Phase 39**: LSP / linter diagnostics panel in VibeCLI TUI — `DiagnosticsComponent`
  (`tui/components/diagnostics.rs`); `/check` TUI command runs `cargo check --message-format=json`
  (or `npx eslint --format json` for npm projects), parses output via `parse_cargo_check()`,
  and populates a 4-line panel between the main area and input bar; panel is hidden when empty;
  `DiagSeverity` E/W/I icons with color coding; `App` gains `diagnostics_panel` field; `/check`
  also added to the TUI command dispatcher; 116 tests passing.
- **Phase 39**: Ambient agent session sharing — `GET /share/:id` Axum route in `serve.rs`
  renders the session HTML with a green "Shared" banner and `<meta name="robots" content="noindex">`
  so search engines don't index it; `/share <session_id>` added to REPL `COMMANDS` array, hint
  text, and command handler (prints `http://localhost:7878/share/<id>`); `/share` added as TUI
  command in `tui/mod.rs`; module docs updated.
- **Phase 38**: `@github:owner/repo#N` context in both VibeCLI (`expand_at_refs`) and VibeUI
  (`resolve_at_references`): fetches GitHub issue/PR title, state, author, labels, and body
  (first 20–30 lines) from the GitHub REST API; uses `GITHUB_TOKEN` env var if present.
  `ContextPicker.tsx` autocompletes `@github:` with a dynamic hint `owner/repo#N`.
- **Phase 38**: GCP Cloud Run and Firebase Hosting deploy targets added to `DeployPanel.tsx`
  (6 targets total) and `run_deploy()` Tauri command; `detect_deploy_target()` detects
  `firebase.json` → Firebase and `app.yaml` / `Dockerfile` → GCP Cloud Run; URL extraction
  extended to match Firebase `Hosting URL:` and Cloud Run `Service URL:` output lines.
- **Phase 38**: `detect_security_patterns()` static OWASP/CWE scanner in `bugbot.rs` — runs
  before LLM analysis on every diff; detects 7 vulnerability classes:
  SQL injection (CWE-89), XSS (CWE-79), path traversal (CWE-22), hardcoded credentials
  (CWE-798), insecure RNG (CWE-338), command injection (CWE-78), open redirect (CWE-601);
  results merged with LLM findings (static first); 3 new unit tests (secret, XSS, clean diff).
- **Phase 37**: Security — `/snippet save|use|show|delete` now validate names with `is_safe_name()`
  (alphanumeric, `-`, `_`, `.` only; rejects `/`, `..`, `\`) to prevent path traversal attacks.
- **Phase 37**: Security — `/rewind <ts>` now validates the timestamp is digits-only before
  constructing the path, preventing `../../etc/passwd` style traversal.
- **Phase 37**: Security — `write_auth_scaffold` Tauri command now canonicalizes the workspace
  path and resolves `..` components in `target_path`, rejecting any path that escapes the
  workspace root directory.
- **Phase 37**: Resource leak — `BackgroundJobsPanel` now closes all active `EventSource`
  connections in a `useEffect` cleanup on unmount, preventing connections from running in the
  background after the component is destroyed.
- **Phase 37**: App.tsx — `confirm()` dialog for file deletion replaced with a proper
  confirmation modal (`pendingDeleteFile` state, backdrop + card with Cancel / Delete buttons).
- **Phase 37**: App.tsx — Extension worker init failure now shown via `toast.error()` instead
  of silent `console.error()`; high-frequency cursor update failures are truly silenced
  (removed noisy `console.error` from the 100ms-debounced handler).
- **Phase 36**: GitPanel — all `console.error()` calls replaced with `toast.error()` for
  visible user feedback; redundant duplicate error logs removed; `confirm()` dialog for
  "Discard changes" replaced with inline "Discard? Yes / No" confirmation row that clears
  automatically on cancel, consistent with other panels.
- **Phase 36**: SteeringPanel — `confirm()` dialog for file deletion replaced with inline
  `pendingDelete` state; first click shows "Del / ✕" confirmation buttons, second confirms.
- **Phase 36**: `commands.rs` — `rename_item` path parent now uses `.ok_or_else(...)` instead
  of `.unwrap()` to avoid panic on root-level paths; all 8 regex patterns moved from inline
  `Regex::new(...).unwrap()` to `OnceLock`-backed lazy accessors (compiled once, reused on
  every call) using `std::sync::OnceLock` (no new dependencies).
- **Phase 36**: `shadow_workspace.rs` — `.lock().unwrap()` replaced with
  `.lock().unwrap_or_else(|e| e.into_inner())` on all 3 `lint_results` mutex accesses to
  recover gracefully from poisoned locks instead of panicking.
- **Phase 36**: `/jobs <session_id>` — show full detail for a single background job (status,
  provider, task, started-N-seconds-ago, duration, summary) directly in the VibeCLI REPL.
  Help text updated to document the `<session_id>` sub-command.
- **Phase 36**: `/rewind list` — corrupt or unreadable checkpoint files now display a descriptive
  error (`(corrupt: ...)` / `(unreadable: ...)`) instead of silently showing "0 messages".
- **Phase 35**: `inline_edit` and `predict_next_edit` Tauri commands now respect the `provider`
  parameter — calls `ChatEngine::set_provider_by_name()` before inference so the correct model
  is used per-request rather than always using the engine's default.
- **Phase 35**: AIChat — auto-scroll to latest message (`messagesEndRef` + `scrollIntoView`);
  "Copy" button on assistant messages with 1.5s "✓ Copied" feedback; voice-input unavailability
  now shown as a toast warning instead of a blocking `alert()`.
- **Phase 35**: GitPanel — 30-second auto-refresh of git status via `setInterval`; `BrowserPanel`
  "open external URL" failure now shown as a toast error.
- **Phase 35**: `AgentPanel` — approval/rejection failures shown as toast errors with `<Toaster>`
  rendered inside the panel; eliminated last two `console.error`-only failure paths.
- **Phase 35**: `search.rs` — buffered line-by-line reading with `BufReader`; 10 MB file-size
  guard skips large binaries; 500-result total cap with labeled outer loop `break`.
- **Phase 34**: Toast notification system (`useToast` hook, `Toaster` component, `Toaster.css`).
  Replaced all `alert()` / `confirm()` blocking dialogs in `App.tsx`, `GitPanel.tsx`,
  `BackgroundJobsPanel.tsx`, `DatabasePanel.tsx`, `AIChat.tsx`, and `MemoryPanel.tsx`.
- **Phase 33**: REPL tab-completion extended — `/jobs`, `/linear`, `/remind`, `/schedule`,
  `/snippet` added to `COMMANDS` array with sub-command tables and inline hints.
- **Phase 33**: `/jobs` REPL command — lists persisted background jobs from `~/.vibecli/jobs/`
  with status icons and timestamps.
- **Phase 33**: `get_commit_files` in `vibe-core/git.rs` using `diff_tree_to_tree`;
  `git_get_commit_files` Tauri command; `GitPanel` now shows real per-commit file list.
- **Phase 33**: Fixed `@docs:npm:` package registry detection and `@symbol:` / `@codebase:`
  context handlers in `expand_at_refs()`.
- **Phase 33**: `gateway.rs` panic-free HTTP client construction (`.unwrap_or_else`).
- **Phase 33**: `vibe-ai` re-exports `BedrockProvider`, `CopilotProvider`, `AzureOpenAIProvider`,
  `OpenRouterProvider`, `GroqProvider`.
- **Phase 32**: Agent SDK `JobRecord` interface and `listJobs()`, `getJob()`, `cancelJob()`
  methods on `VibeCLIAgent`.
- **Phase 32**: `@codebase:` context handler upgraded to use `EmbeddingIndex` (cosine) with
  keyword-search fallback.
- **Phase 32**: `vibe-indexer` persistence — warm-loads from `~/.vibe-indexer/indexes/` on
  startup; saves completed indexes to disk.
- **Phase 32**: `BedrockConfig` and `CopilotConfig` structs in VibeCLI config.
- **Phase 32**: Monaco `revealLineInCenter` + `setPosition` scroll-to-line in `App.tsx`.

### Security
- **P0**: SHA-256 checksum verification in `install.sh` — downloaded binaries are verified
  against `SHA256SUMS.txt` before installation; hard-fails on mismatch.
- **P0**: Path traversal prevention — `resolve_safe()` in `tool_executor.rs`, `safe_join()`
  in `shadow_workspace.rs`, and `safe_resolve_path()` in `commands.rs` canonicalize and
  jail-check all file paths against workspace/shadow boundaries; blocks `../` escapes.
- **P0**: Cryptographic session IDs — `serve.rs` daemon sessions now use 128-bit random hex
  IDs (`rand::thread_rng().gen::<u128>()`) instead of predictable millisecond timestamps.
- **P1**: CORS restriction + bearer-token auth on daemon — `serve.rs` CORS limited to
  localhost origins only; API endpoints require `Authorization: Bearer <token>` (random
  token generated on startup, printed to stderr); health check and session viewer remain
  public.
- **P1**: HTTP client timeouts — `reqwest::Client::builder()` with 90s request / 10s connect
  timeout on `bedrock.rs` and `copilot.rs`; 30s / 10s on `bugbot.rs` (PR diff fetch and
  review posting). Prevents resource exhaustion from hung connections.
- **P1**: GitHub Actions SHA pinning — all 6 actions in `release.yml` pinned to full commit
  SHAs (`actions/checkout@11bd719...`, `dtolnay/rust-toolchain@631a55b...`, etc.) to prevent
  tag mutation supply-chain attacks.
- **P2**: Secrets scrubbing in traces — `redact_secrets()` in `trace.rs` applies 9 regex
  patterns (OpenAI `sk-*`, GitHub `ghp_*`, Bearer tokens, AWS `AKIA*`, URL `?key=` params,
  PEM private keys, generic `password=`/`secret=`/`api_key=`) before writing to JSONL traces
  and message sidecars; 7 unit tests.
- **P2**: Request body size limits — `DefaultBodyLimit::max(1 MB)` layer on all daemon
  endpoints prevents memory exhaustion from oversized requests.
- **P2**: Error response sanitization — all 6 error handlers in `serve.rs` replaced with
  generic `"Internal server error"` messages; real errors logged server-side via
  `tracing::error!()`. Session-not-found responses no longer echo the requested ID.
- **P2**: Temp file TOCTOU fixes — screenshot path changed from millisecond timestamp to
  128-bit random hex; sandbox profile path changed from fixed `/tmp/vibecli_sandbox.sb` to
  PID + 64-bit random suffix.
- **P2**: `cargo audit` in CI — new `audit` job in `release.yml` runs before the build
  matrix; blocks release if known vulnerabilities exist.
- **P2**: Rate limiting — sliding-window rate limiter (60 req/60s) on all authenticated API
  endpoints; returns `429 Too Many Requests` with `retry-after` header.
- **P2**: Gemini API key moved from URL query parameter (`?key=`) to `x-goog-api-key` header
  to prevent key leakage in error messages and logs.
- **P3**: Security response headers — `X-Content-Type-Options: nosniff`, `X-Frame-Options:
  DENY`, `Content-Security-Policy: default-src 'self'; script-src 'none'`, and
  `Referrer-Policy: no-referrer` added to all daemon HTTP responses.
- **P3**: Graceful shutdown — `shutdown_signal()` handles SIGINT/SIGTERM; wired into
  `axum::serve().with_graceful_shutdown()` for clean drain of SSE streams and in-flight
  requests.
- **P3**: Restrictive file permissions — `~/.vibecli/` directory set to `0o700`, config file
  and job files set to `0o600` (owner-only) on Unix to protect API keys.
- **P3**: Hardened command blocklist — `is_safe_command()` upgraded from substring matching to
  regex-based detection; normalizes whitespace; resists flag-reorder, quoting, and spacing
  bypasses; 8 patterns covering `rm -rf`, `dd`, fork bombs, `mkfs`, `chmod 777 /`, `shred`.
- **P3**: Log injection prevention — `tracing::warn!` calls in `review.rs` and `executor.rs`
  switched from format-string interpolation to structured field syntax (`file = %file`) to
  prevent field injection in JSON-format log sinks.
- **P3**: Shadow workspace temp path randomized — PID + 64-bit random hex suffix prevents
  TOCTOU pre-creation race by local attackers.

### Changed
- **Phase 33**: Removed 4 `println!("DEBUG: ...")` calls from `commands.rs`.
- **Phase 33**: `estimate_confidence()` heuristic replaces hardcoded `0.8` in `completion.rs`.
- **Phase 33**: LSP `features.rs` fully implemented (completions, hover, goto-def, symbols,
  format, references, rename); added corresponding methods to `LspClient`.

---

- **Phase 31**: Semantic search upgrade — `semantic_search_codebase` now uses
  `EmbeddingIndex` (cosine similarity) when `.vibeui/embeddings/index.json` exists,
  with keyword-search fallback. New `build_embedding_index` Tauri command.
- **Phase 31**: VS Code extension — `vibecli.inlineEdit` (Cmd/Ctrl+Shift+K),
  `vibecli.viewJobs` (background job QuickPick), `vibecli.sendSelection`
  (Cmd/Ctrl+Shift+Enter), streaming chat in webview (SSE), auto current-file context.
- **Phase 31**: Neovim plugin — `cmp_vibecli.lua` nvim-cmp completion source with
  slash-command and `@context` completions; auto-registered by `vibecli.setup()`.
- **Phase 30**: VibeCLI REPL streaming chat — `llm.stream_chat()` for token-by-token
  output; `@file:`, `@web:`, `@docs:`, `@git` context expansion before messages.
- **Phase 30**: `/snippet` REPL command — `save`, `list`, `use`, `show`, `delete`
  (stored at `~/.vibecli/snippets/`).
- **Phase 30**: `request_ai_completion` Tauri command — implemented with FIM for Ollama,
  chat-based prompt for cloud providers.
- **Phase 29**: `agent_browser_action` Tauri command (Navigate/GetText, Screenshot, WaitFor).
- **Phase 29**: Neovim plugin (`neovim-plugin/`) with `:VibeCLI`, `:VibeCLIAsk`,
  `:VibeCLIInline`, `:VibeCLIJob`; SSE streaming into `*VibeCLI*` split buffer.
- **Phase 29**: `--watch` / `--watch-glob` / `--sandbox` CLI flags; `run_watch_mode()`.
- **Phase 28**: `/linear` REPL command with GraphQL client (`list`, `new`, `open`, `attach`).
- **Phase 28**: `--worktree` CLI flag; `BugBotPanel.tsx`; auto-memory extraction.
- **Phase 27**: Steering files (`SteeringPanel.tsx`, `get/save/delete_steering_file`).
- **Phase 27**: GitHub Actions release workflow (multi-platform binaries, SHA256SUMS).
- **Phase 27**: `install.sh` one-liner installer.

---

## [0.9.0] — Phases 24–26

### Added
- Vim TUI mode (`--vim` flag), AWS Bedrock provider, GitHub Copilot provider.
- Notebook runner (`--notebook` flag), Supabase integration, OAuth PKCE auth.
- GitHub sync (`git_sync` Tauri command with fork/clone/PR).

---

## [0.8.0] — Phases 20–23

### Added
- Admin Policy (`policy.rs`, glob-based tool allow/deny).
- Hooks Config UI (`HooksPanel.tsx`, `get/save_hooks_config` Tauri commands).
- Turbo Mode toggle (`⚡ Turbo` button in `AgentPanel.tsx`).
- BYOK Settings UI (`SettingsPanel.tsx`, `get/save_provider_api_keys`).
- Multi-tab chat (`ChatTabManager.tsx`, per-tab provider selection).
- Phase 12: `/model`, `/cost`, `/context`, `/status`, `/fork` REPL commands.
- Phase 12: `@folder:`, `@terminal`, `@symbol:`, `@codebase:` context types.

---

## [0.7.0] — Phases 14–19

### Added
- Inline Chat Cmd+K (`InlineChat.tsx`, `inline_edit` Tauri command).
- Next-edit prediction with Tab key (`predict_next_edit`, Monaco completion provider).
- SupercompleteEngine (cross-file semantic completion via embedding index).
- Rules system (`rules.rs`, path-pattern front-matter, agent injection).
- Auto memory recording (`memory_recorder.rs`).
- Extended thinking mode for Claude (`thinking_budget_tokens`).
- API key helper rotating credentials (`apiKeyHelper`, `resolve_api_key()`).
- Phase 13: wildcard tool permissions (`denied_tool_patterns`, `check_tool_with_args()`).

---

## [0.6.0] — Phases 12–13

### Added
- GitHub PR Code Review (`review.rs`, `--review`/`--base`/`--branch`/`--pr` flags).
- Phase 11: Named profiles (`~/.vibecli/profiles/`), `--doctor` health check.
- Phase 11: REPL tab-completion (19 commands, sub-commands, cyan highlighting).
- Phase 10: Hierarchical `VIBECLI.md` merging (4 levels), plugin system.
- Phase 10: `ReviewPanel.tsx` (AI code review in GitPanel).

---

## [0.5.0] — Phases 9–10

### Added
- Manager View (`ManagerView.tsx`, parallel agent branches).
- VibeCLI Daemon (`serve.rs`, `--serve`/`--port` flags) + VS Code extension.
- Agent SDK (`packages/agent-sdk/`, streaming async generator API).
- JetBrains plugin (`jetbrains-plugin/`).
- Phase 9.3: OpenAPI-based Daemon HTTP API.

---

## [0.4.0] — Phases 7–8

### Added
- Multi-agent orchestration (`MultiAgentOrchestrator`, `--parallel N`).
- Embedding index (`EmbeddingIndex`, Ollama + OpenAI providers, cosine search).
- Skills system (`skills.rs`, auto-activation in system prompt).
- OpenTelemetry integration (`otel.rs`, OTLP/HTTP export).
- Artifacts system (`artifacts.rs`, `ArtifactsPanel.tsx`).
- GitHub Actions (`action.yml`).
- Phase 7.3: `predict_next_edit` Tauri command.
- Phase 7.4: `CheckpointPanel.tsx`.

---

## [0.3.0] — Phases 5–6

### Added
- MCP server mode (`--mcp-server`).
- `/index` + `/qa` codebase Q&A commands.
- Auto-commit after agent.
- Session resume (`--resume`), plan mode (`--plan`).
- Web search (DuckDuckGo Lite + Tavily + Brave).
- Shell environment policy (`allow`/`deny`/`restrict`).
- Flow injection (activity → AI context).

---

## [0.2.0] — Phases 3–4

### Added
- Streaming responses for all providers.
- 5 AI providers: Ollama, Claude, OpenAI, Gemini, Grok.
- Hooks system (JSON stdin/stdout, `exit 0` allow / `exit 2` block).
- Multi-agent parallel branches with git worktrees.

---

## [0.1.0] — Phases 1–2

### Added
- Initial VibeCLI terminal agent (Ratatui TUI + Rustyline REPL).
- Initial VibeUI desktop editor (Tauri 2 + Monaco Editor + React).
- Agent loop with XML tool calling (ReadFile, WriteFile, RunBash, WebSearch, etc.).
- Shared crates: `vibe-core`, `vibe-ai`, `vibe-lsp`, `vibe-extensions`.
