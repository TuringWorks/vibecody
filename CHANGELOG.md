# Changelog

All notable changes to this project are documented here.
Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

---

## [Unreleased]

### Added
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
