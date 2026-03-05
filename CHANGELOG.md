# Changelog

All notable changes to this project are documented here.
Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

---

## [Unreleased]

### Features (Phases 7.26–7.34)
- **Phase 7.26**: API Documentation Viewer (`ApiDocsPanel.tsx`) — load OpenAPI 3.x/Swagger 2.x, endpoint list grouped by tag, "Try It" sub-panel.
- **Phase 7.27**: Log Viewer (`LogPanel.tsx`) — tail logs, search, AI-powered analysis. `ErrorBoundary` component wrapping all AI panel tabs.
- **Phase 7.28**: Script Runner (`ScriptPanel.tsx`) — auto-detects npm/cargo/make/python/go/just scripts, live output. Notebook Panel (`NotebookPanel.tsx`) — executable code cells with inline output.
- **Phase 7.29**: SSH Remote Manager (`SshPanel.tsx`) — profile CRUD, quick-command chips, remote command execution.
- **Phase 7.30**: Bookmark Manager (`BookmarkPanel.tsx`), Git Bisect Assistant (`BisectPanel.tsx`), Snippet Library (`SnippetPanel.tsx`), API Mock Server (`MockServerPanel.tsx`).
- **Phase 7.31**: GraphQL Playground (`GraphQLPanel.tsx`) — schema introspection, variable editor, query execution.
- **Phase 7.32**: Code Metrics (`CodeMetricsPanel.tsx`) — LOC/file/function counts, language breakdown. Load Test (`LoadTestPanel.tsx`) — HTTP load testing with latency percentiles.
- **Phase 7.33**: Network Tools — port scanner, DNS lookup, TLS inspector.
- **Phase 7.34**: Agent Teams (`AgentTeamPanel.tsx`), Cloud Agents (`CloudAgentPanel.tsx`), CI Review Bot, Marketplace (`MarketplacePanel.tsx`), Visual Testing, Code Transforms (`TransformPanel.tsx`), ACP protocol, Screen Recording.
- **64 AI panel tabs** total across all features.

### Testing
- **735 new unit tests** across 50+ files, bringing the workspace total from 344 → **1074 tests** (1046 main + 28 vibe-collab).
- **Round 4 (155 tests)** across 12 files:
  - `serve.rs` (25): now_ms, persist/load jobs, JobRecord serde, AgentEventPayload constructors,
    ChatMessage/ChatRequest/AgentRequest serde, RateLimiter sliding window.
  - `config.rs` (26): Config default, approval_policy_from_flags, RoutingConfig resolve/is_configured,
    GatewayConfig resolve helpers, CopilotConfig/BedrockConfig, IndexConfig, OtelConfig, WebSearchConfig,
    TOML roundtrip, ProviderConfig, RedTeamCfg.
  - `review.rs` (19): split_diff_by_file edge cases, compute_score ranges, exit_code mapping,
    to_markdown output, extract_files_from_diff, ReviewConfig default, serde roundtrips.
  - `session_store.rs` (38): escape_html (all 5 entities), format_ts/format_age/chrono_simple,
    render_sessions_index_html/render_session_html, count/search multi-keyword, full lifecycle,
    list ordering, tree hierarchy with parent/depth, serde roundtrips.
  - Provider tests (48 across 8 files): claude (8), openai (8), gemini (5), grok (4), groq (5),
    openrouter (5), azure_openai (7), copilot (6) — name, is_available, config serde, response deser.
- **Round 1 (164 tests)** across 8 files:
  - `provider.rs` (22): TokenUsage pricing for all 6 tiers, ProviderConfig builder/serialization,
    base64 padding, Message/CompletionResponse serde roundtrips.
  - `tools.rs` (30): ToolCall::name/is_destructive/is_terminal/summary for all 10 tools,
    ToolResult::ok/err/truncation, format_tool_result, parse edge cases (list_directory default,
    web_search defaults, spawn_agent, unknown tool, multiple calls).
  - `diff.rs` (12): generate_diff identical/changed/added/removed/empty, format_unified_diff
    headers, apply_diff roundtrip for all change types, hunk line counts.
  - `search.rs` (8): search_files matching/multi-file/case-sensitivity/hidden-files/invalid-regex.
  - `executor.rs` (18): is_safe_command blocklist (10 dangerous patterns), execute/execute_in,
    execute_with_approval, output_to_string stdout/stderr/both/empty.
  - `symbol.rs` (16): Language/SymbolKind enums, extract_symbols for Rust/Python/Go/TypeScript.
  - `bedrock.rs` (13): SHA-256 known vectors, HMAC-SHA256, SigV4 signing key derivation,
    epoch_days_to_ymd calendar math (epoch/2000/leap-day/year-end), sigv4_auth_header.
  - `error.rs` (13): CollabError Display for all 8 variants, StatusCode conversion.
- **Round 2 (153 tests)** across 12 files:
  - `flow.rs` (17): FlowTracker ring buffer eviction at 100 events, dedup of opens/edits,
    context_string category filtering (opens/edits/cmds), limit parameter, unknown kind ignored.
  - `syntax.rs` (22): detect_language heuristics (Rust/Python/JS/Go/prose/empty), highlight
    with language/without/unknown, highlight_code_blocks (fenced/no-lang/unclosed/empty/multiple).
  - `diff_viewer.rs` (9): colorize_diff ANSI coloring (+green/-red/@@cyan), header lines not
    colored, context lines uncolored, mixed diff, empty input.
  - `memory.rs` (6): combined_rules section headers, workspace-only rules, save/load roundtrip,
    missing file returns empty, global_rules_path structure.
  - `chat.rs` (14): Conversation role accessors, ChatEngine default/providers/conversations,
    set_active_provider/conversation out-of-bounds errors, active_conversation_mut, serde.
  - `completion.rs` (16): estimate_confidence empty/short/medium/long, syntactic endings
    (;/}/)/,), uncertainty markers (case-insensitive), cap at 1.0, Completion struct.
  - `agent_executor.rs` (10): truncate at/over MAX_OUTPUT limit, resolve absolute/relative/dot/empty
    paths, execute_call routing (apply_patch/spawn_agent errors, task_complete, missing file).
  - `mcp_server.rs` (12): resolve paths, tool_defs (6 tools, name/description/inputSchema fields),
    expected tool names, required params, RpcOk/RpcErr serialization.
  - `manager.rs` (9): LspManager 4 default configs (rust-analyzer, typescript-language-server,
    pylsp), client lookup returns None for unknown, default() equivalence.
  - `workspace.rs` (12): from_config, default name, add_folder dedup, setting types
    (string/number/bool/array), setting overwrite, close_file not open, WorkspaceConfig serde.
  - `multi_agent.rs` (10): AgentTask new/serialization, AgentStatus serde (4 variants)/equality,
    AgentResult serialization, AgentInstance clone, branch_name with large IDs.
  - `scheduler.rs` (16): format_interval (s/m/h/d/zero/boundaries), parse_duration whitespace/zero/large,
    ScheduleExpr Once/Recurring serde roundtrip, ScheduledJob deserialization.
- **Round 3 (173 tests)** across 10 files:
  - `index/mod.rs` (30): score_symbol exact/prefix/contains/no-match, tokenize split/filter/lowercase,
    should_skip expanded (git/.hidden/min.js/lockfiles/pycache), build/search/refresh with tempfiles,
    relevant_symbols ranking, IndexSearchResult serde, relevance_score name/sig/file/no-match.
  - `hooks.rs` (37): type_name all 10 variants, tool_name pre/post/none, file_path saved/created/deleted,
    is_empty, matches with path filters, glob_match_path double-star/single-star/exact, segment_match
    star/prefix/suffix, HookHandler/HookConfig serde, build_payload pre_tool/file_saved/subagent.
  - `buffer.rs` (25): from_file/save/save_as with tempfiles, line_len/line out-of-bounds, apply_edits
    batch insert/delete, cursors default/set_cursors, slice single-line/cross-line, Position/Range/Edit
    serde, undo/redo empty stack no-op, delete empty range no-op.
  - `git.rs` (19): list_branches, get_history with limit, get_commit_files, get_diff changed/unchanged,
    get_repo_diff clean/dirty, discard_changes, commit new file, switch_branch, pop_stash,
    FileStatus/CommitInfo/CheckpointInfo/WorktreeInfo/MergeResult serde.
  - `rules.rs` (14): empty/no-pattern match, load from tempdir with/without frontmatter, skip non-md,
    glob_match double-star/exact/question-mark, load_for_workspace dedup, load_steering clears
    path_pattern, load_all combines, Rule serde.
  - `background_agents.rs` (14): cancel_run, AgentRunStatus Display/serde, AgentDef serde, AgentRun
    new/finish, init creates dir, list_runs sorted, get_run nonexistent, load_def error, for_workspace.
  - `team.rs` (10): context_string empty/shared_commands/no-name/tags/no-tags, TeamConfig serde,
    save/load roundtrip, add_knowledge dedup, remove_knowledge nonexistent.
  - `linear.rs` (9): priority_label all values including edge cases, LinearIssue serde with/without
    assignee, handle_linear_command unknown/attach-empty/new-empty/open-empty/no-key.
  - `context.rs` (8): with_token_budget, with_git_changed_files, with_open_files real/nonexistent,
    with_index + relevant symbols, empty diff not shown, no changed files omits section, Default.
  - `config.rs` (7): default all-none, load_from_file success/nonexistent/invalid TOML,
    ProviderConfigFile serde, AIConfig serde, empty TOML loads as default.

### Tech Debt & Code Quality
- **vibe-indexer HTTP handlers**: Replaced 3 `.unwrap()` panics on `serde_json::to_value()` with match blocks returning HTTP 500.
- **OnceLock regex**: Converted 10 `Regex::new().unwrap()` calls in `expand_at_refs()` to `static OnceLock<Regex>` pattern (compiled once, reused).
- **tracing over eprintln**: Replaced 7 `eprintln!` in `policy.rs` with `tracing::warn!/debug!` and 3 in `client.rs` with `tracing::error!`.
- **Dead code removal**: Deleted empty `tui/components/chat.rs` placeholder, removed unused `_util` module from `background_agents.rs`, removed empty `on_tick()` method.
- **Debug log cleanup**: Removed ~20 `console.log`/`console.error` calls from `App.tsx` (all DEBUG-prefixed + informational logs; kept legitimate error handlers).

### Performance Benchmarks
- **Criterion benchmark suite** for vibe-core hot paths (`vibeui/crates/vibe-core/benches/index_bench.rs`):
  - `cosine_similarity` (384d ~363ns, 1536d ~1.2µs, 1000-vector batch ~311µs)
  - `extract_symbols` (50 fns ~1.7ms, 500 fns ~2.4ms)
  - `index_build` (100 files ~1.1ms)
  - `search_symbols` and `relevant_symbols` (~25ns and ~105ns for 1000-symbol index)
- Made `cosine_similarity` public API for benchmarking and external use.

### Documentation
- **Serve endpoint table**: Expanded from 9 → 16 routes with auth requirements.
- **Jekyll navigation**: Added ROADMAP-v2, FIT-GAP-ANALYSIS-v2, SHANNON-COMPARISON, CHANGELOG to header pages.
- **Test counts**: Updated across all docs to reflect 1074 total tests.

### Accessibility (WCAG 2.1 AA)
- **Keyboard shortcuts**: 8 new shortcuts — `Cmd+J` toggle AI panel, `Cmd+`` toggle terminal,
  `Cmd+Shift+P` command palette (VS Code alias), `Cmd+1`–`Cmd+9` switch AI tab, `Cmd+Shift+E`
  focus explorer, `Cmd+Shift+G` focus git panel (`App.tsx`).
- **Focus-visible outlines**: All interactive elements show 2px blue outline on keyboard focus
  (`:focus-visible`), suppressed on mouse (`:focus`) — meets WCAG 2.4.7 (`App.css`).
- **Command palette ARIA**: `role="dialog"`, `role="combobox"` on input, `role="listbox"` on list,
  `role="option"` on items, `aria-activedescendant` for screen reader tracking (`CommandPalette.tsx`).
- **Modal focus trap**: Tab cycles within modal; Escape closes; previous focus restored on close;
  `aria-modal="true"`, `aria-labelledby` (`Modal.tsx`).
- **Agent status announcements**: `aria-live="polite"` region announces agent status changes
  (running / complete / error / idle) to screen readers (`AgentPanel.tsx`).
- **Skip-to-content link**: Hidden link appears on Tab focus, jumps past sidebar to main editor
  region (`App.css` + `App.tsx`).
- **Screen-reader utility**: `.sr-only` CSS class for visually-hidden accessible text (`App.css`).
- **OnboardingTour component**: First-run guided tour (localStorage gate), dismissible, introduces
  key features to new users (`OnboardingTour.tsx`).
- **EmptyState / LoadingSpinner components**: Reusable UI primitives for consistent empty and
  loading states across panels (`EmptyState.tsx`, `LoadingSpinner.tsx`).

### Provider Hardening
- **HTTP client timeouts (all providers)**: Every AI provider now uses `reqwest::Client::builder()`
  with 90s request + 10s connect timeouts — Ollama, OpenAI, Claude, Gemini, Groq, OpenRouter,
  Azure OpenAI, Bedrock, Copilot (`*.rs` in `providers/`).
- **Copilot device flow hardening**: Token exchange and device flow use timeout-configured client;
  error handling improved (`copilot.rs`).
- **Gemini streaming**: Improved SSE chunk parsing and error resilience (`gemini.rs`).
- **Agent stream buffer**: Pre-allocated `String::with_capacity(8192)` + move instead of clone
  per token, eliminating one heap allocation per LLM token streamed (`agent.rs`).

### Performance
- **Agent loop**: Pre-allocate `accumulated` response buffer (`String::with_capacity(8192)`)
  and move stream chunks into the event channel instead of cloning — eliminates one heap
  allocation per LLM token streamed (`agent.rs`).
- **Embedding index `update()`**: Replaced O(n²) sequential `Vec::remove()` loop with a
  single O(n) drain-zip-filter-unzip pass; removal of k chunks from an index of n is now
  O(n) regardless of how many files change (`embeddings.rs`).
- **Cosine similarity**: Fused 3-pass (dot + norm_a + norm_b) into a single `fold` pass,
  reducing memory-bandwidth usage by ~3× for high-dimensional vectors (`embeddings.rs`).
- **Shared HTTP client**: `reqwest::Client` for Ollama/OpenAI embedding calls is now a
  `OnceLock<Client>` shared across all requests, enabling connection keep-alive and avoiding
  a new connection-pool allocation per embedded chunk (`embeddings.rs`).
- **File search**: `search_files()` now uses `entry.metadata()` (from WalkDir's cached
  directory entry) instead of `fs::metadata(path)`, eliminating one extra `stat(2)` syscall
  per file visited (`search.rs`).
- **Async file I/O**: `read_file`, `write_file`, `apply_patch` in `ToolExecutor` now use
  `tokio::fs` instead of blocking `std::fs`, preventing runtime-thread stalls on slow/cold
  filesystems (`tool_executor.rs`).
- **HTML entity decoding**: Replaced 6 chained `.replace()` calls (6 full string copies) with
  a single left-to-right `decode_html_entities()` pass; also replaced
  `chars.clone().take(12).collect::<String>()` per `<` with a cheap byte-slice peek
  (`tool_executor.rs`).
- 3 new entity-decoder unit tests; 1 new cosine-clamp test; 1 new embedding-update correctness
  test. Total: **829 tests** passing across the workspace (490 new in this release).

### Phase 7.21: Real-time Chat Streaming in AIChat
- **`stream_chat_message` Tauri command** (`commands.rs`): Immediately returns `Ok(())` and
  spawns a background tokio task that calls `provider.stream_chat()` and emits three event types:
  `chat:chunk` (each token as it arrives), `chat:complete` (full `ChatResponse` with tool-call
  processing), and `chat:error`. Any previously running chat stream is automatically cancelled
  before starting a new one. `ChatResponse` gained `#[derive(Clone)]` to satisfy the Tauri emitter
  bound. Added `futures = "0.3"` to `vibeui/src-tauri/Cargo.toml`.
- **`stop_chat_stream` Tauri command** (`commands.rs`): Aborts the background task via the stored
  `AbortHandle`. The frontend commits any partial text as the final assistant message.
- **`AppState.chat_abort_handle`** (`commands.rs` + `lib.rs`): New `Arc<Mutex<Option<AbortHandle>>>`
  field — mirrors `agent_abort_handle` already used by the agent pipeline.
- **`AIChat.tsx` streaming rewrite**:
  - `sendMessage` now calls `invoke("stream_chat_message")` (non-blocking kick-start) instead of
    the blocking `invoke("send_chat_message")`.
  - A one-time `useEffect` registers `chat:chunk`, `chat:complete`, `chat:error` Tauri event
    listeners that build up `streamingText` state token-by-token.
  - While streaming: the typing-indicator is replaced with live text and a blinking cursor.
  - After first chunk: `streamStartMsRef` / `streamCharsRef` compute `tokensPerSec` on every
    chunk; an ⚡ badge shows `N tok/s · ~M tokens` below the text (hides on completion).
  - `stopMessage` callback: invokes `stop_chat_stream` + commits the partial streaming text as the
    final assistant message. The ⏹ Stop button now calls this instead of just flipping `isLoading`.

### Phase 7.20: Streaming Metrics + REPL Session Commands
- **`/sessions` REPL command** (`repl.rs` + `main.rs`): Lists last 15 root sessions from SQLite
  (`SessionStore::list_root_sessions(15)`) with ID, status icon (✅/🟡/❌), step count, task
  preview (45 chars), model name, elapsed age ("Xm ago"), and an inline `/resume` hint for each
  row. Optional `prefix` argument filters by session ID. No-DB and empty gracefully handled.
- **Enhanced `/resume` with SQLite fallback** (`main.rs`): When a JSONL trace exists but has no
  `-messages.json` sidecar, the agent now falls back to `store.get_messages(id)` from SQLite and
  converts `MessageRow` → `Vec<Message>` with full role mapping
  (user/assistant/system). When no JSONL trace exists at all, performs a pure SQLite prefix
  lookup across all root sessions. Prints clear feedback for each path taken.
- **Streaming tok/s metrics in `AgentPanel.tsx`**: Added `streamStartMsRef`,
  `streamCharsRef`, and `streamMetrics` state tracking to the `agent:chunk` listener.
  Computes `tokensPerSec = chars/4/elapsedSec` and `totalTokens = chars/4` live.
  A compact ⚡ badge (`{N} tok/s · ~{M} tokens`) appears below the streaming text while the
  agent is running; disappears on completion or error. Metrics reset on each new agent start.

### Phase 7.19: Context Window Safety + Process Manager
- **Context window safety (`agent.rs`)**: Added `estimate_tokens()` (1 token ≈ 4 chars) and
  `prune_messages()` to the agent loop. Before each LLM call the conversation history is checked
  against a configurable token budget (default 80 000 tokens). If over budget, middle messages
  (indices 2..tail−6) are drained and replaced with a single placeholder, preserving the system
  prompt, initial task, and the last 6 messages (recent tool results + LLM responses). 5 new unit
  tests cover estimate/prune semantics. `AgentLoop` gains `max_context_tokens: Option<usize>` field
  and a `with_context_limit(n)` builder method.
- **Process Manager panel (`ProcessPanel.tsx` + Tauri commands)**:
  - `list_processes` Tauri command: runs `ps aux` (macOS/Linux) or `tasklist /FO CSV` (Windows),
    returns up to 60 `ProcessInfo` records (pid, name, cpu_pct, mem_kb, status), sorted by memory.
  - `kill_process(pid)` Tauri command: sends SIGTERM via `kill -TERM <pid>` (POSIX) or
    `taskkill /PID <pid> /F` (Windows). Returns error if kill fails.
  - `ProcessPanel.tsx`: searchable/filterable live process table (auto-refresh every 5 s), memory
    formatted as KB/MB/GB, status emoji badges (🟢/😴/💀/⏸️), per-row Kill button with confirm
    dialog, optimistic row removal, aria-live feedback banner, footer with count display.
  - `⚙️ Procs` tab added as 32nd AI panel tab in `App.tsx`.
- **Total tests: 513** (508 + 5 new context pruning tests), all passing.

### Added
- **Phase 45**: Frontend panels — `CostPanel.tsx` (💰 Cost tab): per-provider cost breakdown
  chart, total spend summary, budget limit input, cost history table with provider/model/tokens/
  cost columns, clear history button. `AutofixPanel.tsx` (🔧 Autofix tab): auto-detect linter
  framework, run fix mode, diff preview with file count, apply/revert buttons. `GitPanel.tsx`
  gains 3 AI Git tools: 🌿 AI Branch Name (task input + suggest + copy), 📄 Generate Changelog
  (since-ref input + editable markdown result + copy), ⚡ Resolve Merge Conflict (file path +
  conflict textarea + AI resolve + copy resolution). `/autofix` REPL command added.
- **Phase 45**: Cost & Performance Observatory — `record_cost_entry` appends AI call cost
  records to `~/.vibeui/cost-log.jsonl` (JSONL); `get_cost_metrics` loads entries, computes
  per-provider aggregates (`ProviderCostSummary`), and returns total cost/tokens plus
  budget remaining; `set_cost_limit` sets monthly budget cap; `clear_cost_history` wipes log.
- **Phase 45**: AI Git Workflow — `suggest_branch_name` generates concise hyphenated branch
  names from task descriptions via LLM; `resolve_merge_conflict` AI-resolves merge conflicts
  preserving both sides; `generate_changelog` converts `git log --oneline` into Keep-a-Changelog
  format with Added/Fixed/Changed sections via LLM.
- **Phase 45**: Codemod & Lint Auto-Fix — `run_autofix` auto-detects linter
  (clippy/eslint/ruff/gofmt/prettier), runs fix mode, returns `AutofixResult` with diff and
  file count; `apply_autofix` stages changes or reverts via `git restore`.
- **Phase 45**: UTF-8 safety — replaced byte-index string slicing with `char_indices()` across
  tool_executor.rs (web search truncation), tools.rs (ToolCall display), trace.rs (output
  truncation), commands.rs (git diff + @file + @web truncation), tui/mod.rs (agent summary),
  vim_editor.rs (cursor movement, backspace, delete, insert, tab — all char-boundary-safe).
- **Phase 44**: Arena Mode — blind A/B model comparison with hidden identities; `ArenaPanel.tsx`
  (🥊 Arena tab) with randomized provider assignment, vote buttons (A better / B better / Tie /
  Both bad), post-vote identity reveal with timing/token stats, persistent leaderboard with
  win/loss/tie/win-rate per provider, "Send winner to Chat" via `vibeui:inject-context`; Tauri
  commands `save_arena_vote` (persists to `~/.vibeui/arena-votes.json`) and `get_arena_history`
  (loads votes + computes per-provider stats); VibeCLI `/arena` REPL command with `compare`,
  `stats`, `history` sub-commands.
- **Phase 44**: Live Preview with Element Selection — BrowserPanel gains inspect mode toggle
  (🔍 button, localhost-only); injects `inspector.js` into iframe on activate; postMessage
  listener captures `vibe:element-selected` events; element info overlay panel shows tag name,
  CSS selector, React component (if detected), parent chain (3 ancestors), and truncated
  outerHTML; "Send to Chat" button dispatches `vibeui:inject-context` with formatted `@html-selected`
  context; `inspector.js` upgraded with `parentChain` in `buildInfo()`; `@html-selected` added
  to ContextPicker SPECIAL_ITEMS and `resolve_at_references()`.
- **Phase 44**: Recursive Subagent Trees — agents can spawn child agents that spawn grandchildren
  up to 5 levels deep; `AgentContext` gains `parent_session_id`, `depth`, and shared
  `active_agent_counter` (Arc<AtomicU32>); `ToolCall::SpawnAgent` gains `max_depth` parameter;
  `ToolExecutor::spawn_sub_agent()` enforces depth limit (max 5), per-parent child cap (10 via
  session store), and global agent cap (20 via atomic counter); `session_store.rs` gains
  `parent_session_id`/`depth` columns with idempotent `maybe_add_column()` migration,
  `get_children()`/`get_tree()`/`list_root_sessions()` queries, and `AgentTreeNode` struct;
  5 new unit tests for tree operations.
- **Phase 44**: Code coverage panel — `detect_coverage_tool` auto-detects cargo-llvm-cov / nyc /
  coverage.py / go-cover; `run_coverage` Tauri command runs coverage, parses LCOV and Go
  coverprofile formats into `FileCoverage` entries with per-file percentages and uncovered line
  numbers; `CoverageResult` struct with total percentage and raw output fallback extraction.
- **Phase 44**: Multi-model comparison — `compare_models` Tauri command sends the same prompt to
  two providers in parallel via `tokio::join!`; returns `CompareResult` with per-model response
  content, duration, token count, and errors; `build_temp_provider` factory supports 6 provider
  types (Claude/OpenAI/Gemini/Grok/Groq/Ollama) with env-var API key resolution.
- **Phase 44**: HTTP Playground — `send_http_request` Tauri command (method, URL, headers, body)
  with 30s timeout and URL validation; returns `HttpResponseData` (status, headers, body,
  duration); `discover_api_endpoints` greps workspace for Express/Axum/FastAPI/Spring route
  patterns across 8 file types (max 60 results, depth 6).
- **Phase 44**: Safety hardening — replaced `unwrap()` with proper error handling in 9 files:
  bugbot.rs (JSON slice bounds), gateway.rs (port bind panics → graceful return), redteam.rs
  (JSON slice + recon unwrap), agent.rs (empty tool_calls → Complete event), chat.rs
  (active_conversation_mut), buffer.rs (char count vs byte len), git.rs (branch ref name),
  index/mod.rs (NaN-safe sort), remote.rs (client builder + header value).
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
