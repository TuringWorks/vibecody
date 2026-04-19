# 05 — FIT-GAP Overstatements

> Modules claimed as "implemented" in FIT-GAP docs that are actually simulation-only stubs (data structures + in-memory tests, no real I/O).

## Pattern

Each gap from the FIT-GAP analyses was "closed" by creating:
1. A Rust file with correct data structures, enums, and method signatures
2. Unit tests exercising in-memory state manipulation
3. A VibeUI panel
4. A REPL command

But the modules **lack the actual I/O layer** (HTTP clients, process spawning, FFI bindings, external API calls) that would make them functional. The FIT-GAP documents conflate "designed and typed" with "implemented and working."

## Simulation-Only Modules (P1)

| Module | Lines | FIT-GAP Claim | Reality |
|--------|-------|---------------|---------|
| ~~`web_grounding.rs`~~ | ~~1,277~~ | ~~v7 Gap 4~~ | **RESOLVED (US-001):** `web_grounding_backend.rs` now ships `SearxngBackend`, `BraveBackend`, `TavilyBackend` using real `reqwest` I/O. `WebGroundingEngine::search_async` routes through the cache + rate limiter + classifier. Tauri `web_search` calls the real engine with a provider chosen from env (`VIBECLI_SEARCH_PROVIDER`, `TAVILY_API_KEY`, `BRAVE_API_KEY`, `VIBECLI_SEARXNG_URL`). Covered by 8 unit tests + 4 BDD scenarios against an in-process axum mock server. |
| ~~`mcp_streamable.rs`~~ | ~~1,511~~ | ~~v7 Gap 7~~ | **RESOLVED (US-004):** `mcp_http.rs` now ships real PKCE S256 challenges backed by `sha2::Sha256` + `rand::rngs::OsRng` (replacing the hand-rolled SHA-256 and xorshift), `build_auth_url` with `urlencoding`-safe values, `McpOAuthClient::exchange_code` and `refresh_token` posting `application/x-www-form-urlencoded` grants to real token endpoints via `reqwest`, and `McpStreamClient::open_stream` opening a Bearer-authenticated SSE stream. `McpError::Unauthorized` is separated from `Server` so callers can re-trigger the auth flow. Covered by 3 unit tests + 6 BDD scenarios against an axum mock OAuth + MCP server (PKCE shape, auth URL shape, code exchange, refresh, Bearer-gated SSE success, Bearer-gated SSE 401). |
| ~~`a2a_protocol.rs`~~ | ~~1,821~~ | ~~v7 Gap 1~~ | **RESOLVED (US-002):** `a2a_http.rs` now ships an axum-based HTTP server (`GET /a2a/card`, `POST /a2a/tasks`, `GET /a2a/tasks/:id`, `GET /a2a/events` SSE) wired to the shared `A2aServer` state, plus a reqwest-based `A2aHttpClient` with matching `fetch_card`, `submit_task`, `get_task`, `read_events` methods. SSE handler replays buffered events and fans out new events via a `tokio::sync::broadcast` channel. Covered by 5 unit tests + 3 BDD scenarios (card discovery, task submit/poll, SSE stream). |
| ~~`worktree_pool.rs`~~ | ~~1,255~~ | ~~v7 Gap 2~~ | **RESOLVED (US-003):** `worktree_git.rs` now ships `GitWorktreePool` with `spawn` / `remove` / `merge_into` wrappers around the real `git` CLI (`git worktree add -b`, `git worktree remove --force`, `git merge --no-ff`, `git merge --abort`). Conflict detection parses `git status --porcelain` for `UU`/`AA`/`DU`/`UD`/`AU`/`UA` codes. Covered by 5 unit tests + 5 BDD scenarios against real temp-dir git repos (spawn, remove, clean merge, conflicting merge with abort, capacity enforcement). The existing in-memory `worktree_pool.rs` keeps its pure business logic (task splitting, parallelism estimation, branch naming). |
| ~~`proactive_agent.rs`~~ | ~~1,157~~ | ~~v7 Gap 3~~ | **RESOLVED (US-006):** `proactive_scanner.rs` now ships real filesystem I/O — `discover_files` walks a project tree with `walkdir` and a default ignore list (`.git`, `target`, `node_modules`, `dist`, `build`, `.next`, `.venv`, `__pycache__`), `categorize_by_ext` maps filenames to scan categories, `scan_project` wires discovery + categorization into the existing `SuggestionGenerator`, and `start_watcher` opens a `notify::RecommendedWatcher` that forwards create/modify/remove events onto a `tokio::sync::mpsc::Receiver`. Covered by 3 unit tests + 4 BDD scenarios (tree walk, ignore-dir skipping, suggestions-from-real-files, real watcher fires on file creation within 5s). The stub keeps its pure business logic (suggestion state machine, digests, learning store). |
| `issue_triage.rs` | 1,214 | v7 Gap 10: Autonomous issue classification, GitHub/Linear integration | No HTTP calls to GitHub/Linear APIs |
| `native_connectors.rs` | — | v7 Gap 14: Connector trait + 20 service implementations, OAuth flow management | No reqwest, no async, no OAuth. Just endpoint URL strings |
| `langgraph_bridge.rs` | — | v7 Gap 19: LangGraph-compatible REST API, agent state serialization | No HTTP/REST implementation whatsoever |
| ~~`voice_local.rs`~~ | ~~—~~ | ~~v7 Gap 15~~ | **RESOLVED (US-005):** `voice_whisper.rs` now ships real I/O — `download_model` streams whisper GGML files over HTTP via chunked `bytes_stream`, `parse_wav` / `load_wav_pcm` read RIFF/WAVE PCM16 mono/multi-channel files into the `Vec<f32>` shape whisper.cpp expects, `encode_wav_mono_pcm16` round-trips for tests, and a `Transcriber` trait with `NullTranscriber` (always-available) + `WhisperTranscriber` (gated behind the `voice-whisper` cargo feature + `whisper-rs` FFI) separates the audio pipeline from the backend. Covered by 4 unit tests + 5 BDD scenarios (successful download, 404 error, WAV round-trip, empty-input rejection, transcript shape report). |
| `mcts_repair.rs` | 1,785 | v7 Gap 8: MCTS with UCB1 selection, rollout via test execution | Has select/expand/backpropagate but "rollout" never runs actual tests |
| `sketch_canvas.rs` | — | v7 Gap 20: Canvas drawing, shape recognition, wireframe-to-component, 3D scene | Basic shape data. No WebGL, no three.js, no 3D |
| `cost_router.rs` | — | v7 Gap: Intelligent cost-aware request routing | Data structures only |

## Partially Implemented / Overstated (P2)

| Module | Claim | Reality |
|--------|-------|---------|
| `semantic_index.rs` (1,554 lines) | v7 Gap 5: AST-level codebase understanding, call graph extraction, type hierarchy | Uses **line-by-line regex** (`trimmed.starts_with("pub fn")`), not tree-sitter or any AST parser. No call graph or import chain resolution |
| `ai_code_review.rs` LinterAggregator | 8 linters: clippy, eslint, pylint... | `simulate_linter()` returns canned "Linter check passed" for every file. Doc acknowledges this as P3/simulated |

## RL-OS Subsystem — Entire Subsystem Overstated (P1)

**FIT-GAP-ANALYSIS.md** (RL-OS deep-dive section, originally FIT-GAP-RL-OS.md) claims:
- "30+ RL algorithms (PPO/SAC/DQN/TD3/CQL/IQL/MAPPO/QMIX/DreamerV3)"
- "JIT-compiled env kernels on GPU/TPU"
- "Rust-native core engine, Python bindings for ecosystem compat"
- "First vertically-integrated RL operating system"

**Reality**: ~31K lines across 8 `rl_*.rs` files of **pure data structures**:
- No neural network training (no tch/candle/onnxruntime crate)
- "Gradient sync" is averaging `Vec<f64>`
- No GPU compute whatsoever
- No Python bindings (no PyO3)
- Tests only exercise in-memory state manipulation

## Genuinely Functional Modules (for reference)

These modules have **real I/O** and are accurately represented:

| Module | Evidence of Real Implementation |
|--------|-------------------------------|
| `browser_agent.rs` (2,160 lines) | Real CDP integration via reqwest, launches Chrome, navigates pages |
| `desktop_agent.rs` (2,190 lines) | Real shell-out to cliclick/xdotool for mouse/keyboard |
| `company_store.rs` | Real SQLite (rusqlite) CRUD operations |
| `ai_code_review.rs` | Real pattern-matching detectors for security/complexity |
| `architecture_spec.rs` | Real TOGAF/Zachman/C4 data modeling |
| `policy_engine.rs` | Real RBAC/ABAC evaluation logic |

## Roadmap Items

`ROADMAP.md` (Appendix A, phases 23–31) marks all items as `[x]` complete. The majority of "shipped" modules (a2a_protocol, worktree_pool, proactive_agent, ~~web_grounding~~, mcp_streamable, cost_router, next_task, native_connectors, voice_local, doc_sync, langgraph_bridge, sketch_canvas, visual_verify, rlcef_loop) are data-structure-only implementations. (web_grounding was converted to real I/O in US-001.)
