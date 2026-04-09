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
| `web_grounding.rs` | 1,277 | v7 Gap 4: Search provider abstraction (Google, Bing, Brave, SearXNG, Tavily), result ranking, citation tracking | No HTTP client. Tauri `web_search` command returns **hardcoded** "Result 1 for: {query}" |
| `mcp_streamable.rs` | 1,511 | v7 Gap 7: Streamable HTTP transport, OAuth 2.1, PKCE flow, token refresh, connection pooling | No HTTP server/client, no OAuth flow, no PKCE. Pure data structures |
| `a2a_protocol.rs` | 1,821 | v7 Gap 1: A2A server/client modes, SSE streaming, capability negotiation | No async, no HTTP server, no SSE. Data structures and in-memory state only |
| `worktree_pool.rs` | 1,255 | v7 Gap 2: Full worktree pool with parallel agent dispatch, merge orchestration | No `git worktree` commands, no git2 usage, no process spawning |
| `proactive_agent.rs` | 1,157 | v7 Gap 3: Background scanner with configurable cadence | No async, no file system scanning, no background tasks. Data structures only |
| `issue_triage.rs` | 1,214 | v7 Gap 10: Autonomous issue classification, GitHub/Linear integration | No HTTP calls to GitHub/Linear APIs |
| `native_connectors.rs` | — | v7 Gap 14: Connector trait + 20 service implementations, OAuth flow management | No reqwest, no async, no OAuth. Just endpoint URL strings |
| `langgraph_bridge.rs` | — | v7 Gap 19: LangGraph-compatible REST API, agent state serialization | No HTTP/REST implementation whatsoever |
| `voice_local.rs` | — | v7 Gap 15: whisper.cpp integration for local speech recognition | References whisper model URLs but no FFI bindings, no audio capture (cpal/rodio) |
| `mcts_repair.rs` | 1,785 | v7 Gap 8: MCTS with UCB1 selection, rollout via test execution | Has select/expand/backpropagate but "rollout" never runs actual tests |
| `sketch_canvas.rs` | — | v7 Gap 20: Canvas drawing, shape recognition, wireframe-to-component, 3D scene | Basic shape data. No WebGL, no three.js, no 3D |
| `cost_router.rs` | — | v7 Gap: Intelligent cost-aware request routing | Data structures only |

## Partially Implemented / Overstated (P2)

| Module | Claim | Reality |
|--------|-------|---------|
| `semantic_index.rs` (1,554 lines) | v7 Gap 5: AST-level codebase understanding, call graph extraction, type hierarchy | Uses **line-by-line regex** (`trimmed.starts_with("pub fn")`), not tree-sitter or any AST parser. No call graph or import chain resolution |
| `ai_code_review.rs` LinterAggregator | 8 linters: clippy, eslint, pylint... | `simulate_linter()` returns canned "Linter check passed" for every file. Doc acknowledges this as P3/simulated |

## RL-OS Subsystem — Entire Subsystem Overstated (P1)

**FIT-GAP-RL-OS.md** claims:
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

`ROADMAP-v5.md` marks all Phase 23-31 items as `[x]` complete. The majority of "shipped" modules (a2a_protocol, worktree_pool, proactive_agent, web_grounding, mcp_streamable, cost_router, next_task, native_connectors, voice_local, doc_sync, langgraph_bridge, sketch_canvas, visual_verify, rlcef_loop) are data-structure-only implementations.
