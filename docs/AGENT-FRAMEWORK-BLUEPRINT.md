# VibeCody as an Autonomous Agent Framework

## Blueprint: Enabling Perplexity Computer / OpenClaw / Operator-class Capabilities

> **STATUS: ALL 4 GAPS CLOSED (2026-03-24)**
>
> - `browser_agent.rs` (2,159 lines, 73 tests) — CDP browser automation
> - `observe_act.rs` (1,661 lines, 59 tests) — Continuous visual grounding loop
> - `desktop_agent.rs` (2,191 lines, 92 tests) — Cross-platform GUI automation
> - `serve.rs` v1 API (+474 lines) — Agent-as-a-Service endpoints
>
> VibeCody now exceeds all 4 competitor frameworks across every capability dimension.

---

## Architecture Diagram

![Agent Framework Architecture](/agent-framework.svg)

## 1. Executive Summary

VibeCody already possesses ~85% of the infrastructure needed to function as a full autonomous agent framework. Its agent loop, tool system, multi-provider support, container sandboxing, MCP integration, and 18-platform gateway form a foundation that exceeds many dedicated agent frameworks. The primary gaps are **native browser automation** (DOM interaction via CDP/Playwright), **real-time visual grounding** (continuous screen observation), and a **unified agent-as-a-service API** that exposes these capabilities to external consumers.

---

## 2. Competitor Landscape

| Framework | Core Capability | Execution Model |
|-----------|----------------|-----------------|
| **Perplexity Computer** | Web browsing + search + code execution in cloud VM | Cloud-hosted, browser-native, search-augmented |
| **OpenClaw** | Open-source computer-use agent with tool plugins | Local/cloud, screenshot+click loop, extensible tools |
| **OpenAI Operator** | Browser automation via GPT-4V + action primitives | Cloud VM, visual grounding, click/type/scroll actions |
| **Anthropic Computer Use** | Screenshot → action loop with Claude vision | API-based, screenshot capture, coordinate-based actions |
| **Devin (Cognition)** | Full IDE + browser + terminal in cloud sandbox | Cloud VM, persistent workspace, multi-tool |

---

## 3. VibeCody Current Capabilities (What Already Exists)

### 3.1 Core Agent Loop (`agent.rs` — 1,744 lines)
- **Streaming LLM interaction** with retry + exponential backoff + jitter
- **11 built-in tools**: ReadFile, WriteFile, ApplyPatch, Bash, SearchFiles, ListDirectory, WebSearch, FetchUrl, TaskComplete, SpawnAgent, Think
- **Circuit breaker** with 5 health states (Progress, Stalled, Spinning, Degraded, Blocked) + half-open recovery
- **Prompt injection defense** on tool outputs
- **Context pruning** to maintain 80K token budget
- **Hook system** for pre/post validation (SessionStart, UserPromptSubmit, PreToolUse, PostToolUse, FileSaved)

### 3.2 Multi-Agent Orchestration (`multi_agent.rs`, `agent_teams_v2.rs`)
- Parallel agent execution in isolated git worktrees
- Agent roles: Lead, Teammate, Reviewer, Specialist
- Peer messaging bus for inter-agent communication
- Recursive sub-agent spawning (depth 5)
- Conflict detection (multiple agents editing same file)

### 3.3 Computer Use (`computer_use.rs`)
- Cross-platform screenshot capture (macOS `screencapture`, Linux `scrot`, Windows PowerShell)
- LLM-based visual assertions with confidence scoring
- Visual test sessions (action → screenshot → assertion triples)
- Screen recording with frame-by-frame captions (`screen_recorder.rs`)

### 3.4 Container Sandboxing (`container_runtime.rs`)
- Unified `ContainerRuntime` trait: Docker, Podman, OpenSandbox
- Resource limits (CPU, memory, PIDs)
- Network policies: None (isolated), Restricted (domain whitelist), Full
- Volume mounts with read-only flag
- Cloud agent execution in Docker containers (`cloud_agent.rs`)

### 3.5 Web Interaction
- **WebSearch tool**: DuckDuckGo search (no API key required)
- **FetchUrl tool**: Extract text from web pages
- **Web crawler** (`web_crawler.rs`): robots.txt, sitemaps, rate limiting, link extraction
- **Perplexity provider**: Search-augmented LLM (sonar models)
- **HTTP Playground panel**: Manual API testing

### 3.6 MCP Integration (`mcp.rs`, `mcp_lazy.rs`, `mcp_directory.rs`)
- JSON-RPC 2.0 over stdio
- Lazy schema loading with LRU cache (64 slots, 5 min idle timeout)
- Verified plugin directory with categories, reviews, permissions
- Semantic tool search via embeddings
- ACP (Agent Client Protocol) support for agent-to-agent capability negotiation

### 3.7 Gateway & API (`serve.rs`, `gateway.rs`)
- REST/SSE daemon: `/chat`, `/chat/stream`, `/agent`, `/stream/:id`
- Job persistence and session sharing
- 18-platform messaging gateway (Telegram, Discord, Slack, Signal, Matrix, Teams, IRC, Twitch, WhatsApp, iMessage, etc.)

### 3.8 Memory & Context
- **OpenMemory**: 5 cognitive sectors, HNSW index, AES-256-GCM encryption, bi-temporal knowledge graph
- **Infinite Context**: 5-level hierarchy with token budget eviction
- **Context Bundles**: Named, shareable context sets with priority ordering
- **Session persistence**: SQLite-backed with WAL mode

### 3.9 Provider Ecosystem (18 providers)
- Claude, OpenAI, Gemini, Groq, Grok, Ollama, OpenRouter, Azure OpenAI, Bedrock, Copilot, Mistral, Cerebras, DeepSeek, Zhipu, Vercel AI, Perplexity, MiniMax, SambaNova
- Failover provider with health-aware dynamic ordering
- Universal XML tool calling (works with any provider)

---

## 4. Gap Analysis: What's Missing for Full Agent Framework Parity

### 4.1 GAP: Native Browser Automation (P0 — Critical)

| Aspect | Current State | Target State |
|--------|--------------|--------------|
| DOM interaction | None — screenshot-only | CDP/Playwright: click, type, scroll, navigate |
| Element targeting | LLM vision → coordinates | DOM selectors + visual grounding hybrid |
| Form filling | Not supported | Automated form detection and filling |
| JavaScript execution | Not supported in browser | `page.evaluate()` for dynamic content |
| Multi-tab management | Not supported | Tab creation, switching, closing |
| Cookie/session management | Not supported | Persistent browser profiles |
| File download/upload | Not supported in browser | Automated file transfer via browser |

**What exists**: `computer_use.rs` has screenshot capture + LLM visual assertions. The `BrowserPanel.tsx` references CDP (Chrome DevTools Protocol) in the UI.

**What's needed**: A `browser_agent.rs` module that wraps CDP or drives a headless Chromium instance, providing action primitives (navigate, click, type, scroll, wait, screenshot, extract) that integrate as agent tools.

### 4.2 GAP: Real-time Visual Grounding Loop (P0 — Critical)

| Aspect | Current State | Target State |
|--------|--------------|--------------|
| Screen observation | One-shot screenshot | Continuous observation loop (1-5 fps) |
| Action feedback | No visual verification | Screenshot-after-action verification |
| Coordinate mapping | Raw pixel coordinates | Element-aware coordinate mapping |
| OCR / text extraction | LLM vision only | Hybrid OCR + LLM for speed |
| Error recovery | None (single attempt) | Visual diff → retry on unexpected state |

**What exists**: Screenshot capture, LLM vision via multimodal providers, visual assertions.

**What's needed**: An `observe_act_loop` that continuously captures screen state, sends to vision LLM, receives action instructions, executes them, and verifies the result — the core Anthropic Computer Use / OpenClaw pattern.

### 4.3 GAP: Desktop GUI Interaction (P1 — High)

| Aspect | Current State | Target State |
|--------|--------------|--------------|
| Mouse control | Not supported | Programmatic mouse move/click/drag |
| Keyboard input | Not supported (except terminal) | Synthetic keystrokes to any application |
| Window management | Not supported | Focus, resize, minimize, enumerate windows |
| Accessibility tree | Not supported | AT-SPI (Linux) / AX (macOS) for element discovery |

**What exists**: Terminal-based interaction via Bash tool, screenshot capture.

**What's needed**: Platform-specific desktop automation bindings (macOS: CoreGraphics + AX API, Linux: X11/Wayland + AT-SPI, Windows: Win32 + UI Automation).

### 4.4 GAP: Agent-as-a-Service API (P1 — High)

| Aspect | Current State | Target State |
|--------|--------------|--------------|
| Task submission | `/agent` POST endpoint | Full task lifecycle API with webhooks |
| Authentication | Basic (rate limiting only) | API key + OAuth2 + team scoping |
| Streaming results | SSE per session | WebSocket + SSE + webhook callbacks |
| Task queuing | Single-threaded per request | Priority queue with concurrency limits |
| Billing/metering | `usage_metering.rs` exists | Per-task cost tracking + chargeback API |
| SDK | None for external consumers | Python/JS/Go client SDKs |

**What exists**: `serve.rs` with REST/SSE, `usage_metering.rs`, `gateway.rs`.

**What's needed**: A production-grade API layer with authentication, rate limiting per API key, task queuing, webhook notifications, and client SDKs.

### 4.5 GAP: Persistent Cloud VM Workspace (P2 — Medium)

| Aspect | Current State | Target State |
|--------|--------------|--------------|
| VM lifecycle | Docker container per task | Long-lived VM with state persistence |
| Pre-installed tools | Minimal container images | Dev environment with editors, browsers, tools |
| Snapshot/restore | Not supported | VM snapshots for rollback |
| Cost optimization | No idle detection | Auto-hibernate + resume on demand |

**What exists**: `container_runtime.rs` (Docker/Podman/OpenSandbox), `cloud_agent.rs`, `vm_orchestrator.rs`.

**What's needed**: VM orchestration with persistent state across sessions, pre-built dev environment images, and snapshot/restore.

---

## 5. Implementation Blueprint

### Phase 1: Browser Agent (2-3 weeks)

**New module**: `vibecli/vibecli-cli/src/browser_agent.rs`

```
BrowserAgent
├── ChromeDriver (CDP over WebSocket)
│   ├── connect(port) / launch_headless()
│   ├── navigate(url)
│   ├── click(selector | coordinates)
│   ├── type_text(selector, text)
│   ├── scroll(direction, amount)
│   ├── screenshot() -> PNG bytes
│   ├── extract_text(selector?) -> String
│   ├── evaluate_js(script) -> Value
│   ├── wait_for(selector, timeout)
│   └── get_page_info() -> PageInfo { url, title, dom_summary }
├── BrowserTool (new ToolCall variant)
│   ├── BrowserNavigate { url }
│   ├── BrowserClick { target: SelectorOrCoords }
│   ├── BrowserType { target, text }
│   ├── BrowserScroll { direction, amount }
│   ├── BrowserScreenshot {}
│   ├── BrowserExtract { selector? }
│   └── BrowserEvaluate { script }
└── BrowserSession
    ├── tabs: Vec<Tab>
    ├── cookies: CookieJar
    └── history: Vec<NavigationEntry>
```

**Integration points**:
- Add `BrowserNavigate`, `BrowserClick`, `BrowserType`, `BrowserScreenshot`, `BrowserExtract` to `ToolCall` enum in `tools.rs`
- Add tool descriptions to `TOOL_SYSTEM_PROMPT`
- Implement execution in `tool_executor.rs`
- Add `/browse` REPL command
- Add BrowserAgentPanel.tsx to VibeUI

**Dependencies**: `chromiumoxide` or `headless_chrome` Rust crate for CDP, or shell out to `chrome --remote-debugging-port`

### Phase 2: Observe-Act Loop (1-2 weeks)

**New module**: `vibecli/vibecli-cli/src/observe_act.rs`

```
ObserveActLoop
├── Config
│   ├── observation_interval_ms: u64 (default: 2000)
│   ├── max_actions_per_step: usize
│   ├── screenshot_resolution: (u32, u32)
│   └── vision_provider: String
├── Step cycle:
│   1. capture_screenshot()
│   2. send_to_vision_llm(screenshot, task, history)
│   3. parse_actions(llm_response) -> Vec<Action>
│   4. execute_actions(actions)
│   5. capture_verification_screenshot()
│   6. compare_expected_vs_actual()
│   7. if unexpected: retry or escalate
│   8. update_history(step_result)
│   9. check_completion_criteria()
├── Action types:
│   ├── Click(x, y)
│   ├── Type(text)
│   ├── KeyCombo(keys)
│   ├── Scroll(direction, amount)
│   ├── Wait(ms)
│   ├── Screenshot()
│   └── Done(summary)
└── Safety rails:
    ├── max_steps limit
    ├── action rate limiting
    ├── forbidden URL patterns
    └── human-in-the-loop breakpoints
```

**Reuses**: `computer_use.rs` (screenshot), multimodal providers (vision), `CircuitBreaker` (health monitoring)

### Phase 3: Desktop Automation Bindings (2-3 weeks)

**New module**: `vibecli/vibecli-cli/src/desktop_agent.rs`

```
DesktopAgent (platform-specific)
├── macOS:
│   ├── CoreGraphics: mouse_move, mouse_click, mouse_drag
│   ├── CGEvent: key_press, key_release, key_combo
│   ├── AXUIElement: find_element, get_attribute, get_children
│   └── NSWorkspace: list_apps, activate_app, window_info
├── Linux:
│   ├── X11/XTest: mouse/keyboard simulation
│   ├── AT-SPI: accessibility tree traversal
│   └── xdotool fallback: window management
├── Windows:
│   ├── Win32 SendInput: mouse/keyboard
│   ├── UI Automation: element tree
│   └── PowerShell: window management
└── Cross-platform wrapper:
    ├── move_mouse(x, y)
    ├── click(button, x, y)
    ├── type_text(text)
    ├── press_key(key)
    ├── find_element(criteria) -> Element
    ├── list_windows() -> Vec<WindowInfo>
    └── focus_window(id)
```

### Phase 4: Agent-as-a-Service API (2 weeks)

**Enhance**: `vibecli/vibecli-cli/src/serve.rs`

```
API Enhancements
├── Authentication
│   ├── API key management (CRUD)
│   ├── Per-key rate limits and permissions
│   └── Team/org scoping
├── Task Lifecycle
│   ├── POST /v1/tasks — submit task
│   ├── GET /v1/tasks/:id — status + results
│   ├── POST /v1/tasks/:id/cancel — cancel
│   ├── GET /v1/tasks/:id/stream — SSE events
│   └── POST /v1/tasks/:id/feedback — human feedback
├── Webhook Callbacks
│   ├── task.started, task.progress, task.completed, task.failed
│   └── Configurable per-task callback URL
├── Browser Tasks
│   ├── POST /v1/browse — autonomous browsing task
│   ├── GET /v1/browse/:id/screenshots — screenshot history
│   └── POST /v1/browse/:id/intervene — human takeover
└── SDKs
    ├── vibecody-python (pip install vibecody)
    ├── vibecody-js (npm install @vibecody/sdk)
    └── vibecody-go (go get github.com/vibecody/sdk-go)
```

---

## 6. Fit-Gap Matrix

### Legend: Fully Implemented | Partially Implemented | Gap (Not Implemented)

| Capability | Perplexity Computer | OpenClaw | Operator | VibeCody Current | VibeCody Gap |
|-----------|-------------------|----------|----------|-----------------|-------------|
| **LLM Integration** | Single (internal) | Multi-provider | Single (GPT-4V) | **18 providers + failover** | None |
| **Tool Calling** | Internal tools | Plugin system | Built-in actions | **11 tools + MCP + plugins** | None |
| **Code Execution** | Cloud sandbox | Local/Docker | Cloud VM | **Docker/Podman/OpenSandbox** | None |
| **File Read/Write** | Yes | Yes | N/A | **Yes (ReadFile, WriteFile, ApplyPatch)** | None |
| **Web Search** | Native (Perplexity) | Via tools | Via browsing | **DuckDuckGo + Perplexity provider** | None |
| **Web Browsing (DOM)** | Full browser | CDP-based | Full browser | Screenshot + FetchUrl | **CDP/Playwright needed** |
| **Click/Type/Scroll** | Browser-native | Coordinate-based | Browser-native | Not in browser context | **Browser action primitives** |
| **Screenshot Capture** | Yes | Yes | Yes | **Yes (cross-platform)** | None |
| **Visual Grounding** | LLM vision | LLM vision | GPT-4V | **Multimodal providers** | **Continuous observe-act loop** |
| **Desktop GUI Control** | No | Optional | No | Screenshot only | **Mouse/keyboard/a11y bindings** |
| **Multi-Agent** | No | No | No | **Yes (teams, sub-agents, roles)** | None |
| **Memory/Context** | Session only | Session only | Session only | **OpenMemory + Infinite Context** | None |
| **Agent Health Monitoring** | Basic | None | None | **CircuitBreaker (5 states + recovery)** | None |
| **Sandboxed Execution** | Cloud VM | Docker | Cloud VM | **Docker/Podman + OS sandbox** | None |
| **API Access** | API product | No API | No API | **REST/SSE daemon** | **Auth + webhooks + SDKs** |
| **Platform Gateway** | Web only | CLI only | Web only | **18 platforms** | None |
| **Self-Improvement** | No | No | No | **RL edit prediction + auto-research** | None |
| **Compliance/Audit** | Enterprise | None | Enterprise | **SOC 2 controls + PII redaction** | None |
| **Voice Input** | No | No | No | **Groq Whisper + ElevenLabs TTS** | None |

### Scoring Summary

| Framework | Capabilities (of 19) | VibeCody Parity |
|-----------|---------------------|-----------------|
| Perplexity Computer | 12 | 10 match, 2 gaps (browser DOM, observe-act) |
| OpenClaw | 10 | 8 match, 2 gaps (browser DOM, observe-act) |
| OpenAI Operator | 11 | 9 match, 2 gaps (browser DOM, observe-act) |
| Anthropic Computer Use | 9 | 7 match, 2 gaps (observe-act, desktop GUI) |

**VibeCody exceeds all competitors in**: Multi-agent orchestration, provider ecosystem, memory/context, platform gateway, self-improvement, compliance, and voice input.

**VibeCody's primary gaps**: Browser DOM interaction and continuous visual grounding loop — both addressable in Phases 1-2 (~4 weeks).

---

## 7. Recommended Prioritization

| Priority | Phase | Effort | Impact | Description |
|----------|-------|--------|--------|-------------|
| **P0** | 1 | 2-3 weeks | Critical | Browser Agent (CDP) — unlocks web automation use cases |
| **P0** | 2 | 1-2 weeks | Critical | Observe-Act Loop — enables autonomous visual agents |
| **P1** | 4 | 2 weeks | High | Agent-as-a-Service API — enables external consumption |
| **P2** | 3 | 2-3 weeks | Medium | Desktop GUI Automation — enables non-browser desktop tasks |

**Total estimated effort**: 7-10 weeks for full parity.

**Quick wins** (buildable today with existing infrastructure):
- Combine `computer_use.rs` + `agent_modes.rs (Deep)` + `container_runtime.rs` for screenshot-based autonomous agents in Docker
- Use `web_crawler.rs` + `FetchUrl` + Perplexity provider for search-augmented agent tasks
- Expose `serve.rs` + `gateway.rs` as agent API endpoint for Slack/Discord/Telegram bot agents

---

## 8. Architecture: How It All Fits Together

```
                    ┌──────────────────────────────────────────┐
                    │            VibeCody Agent Framework       │
                    ├──────────────────────────────────────────┤
                    │                                          │
  External API ────>│  serve.rs (REST/SSE)                     │
  Gateway ─────────>│  gateway.rs (18 platforms)               │
  REPL ────────────>│  repl.rs (interactive)                   │
  VibeUI ──────────>│  Tauri commands                          │
                    │         │                                │
                    │         ▼                                │
                    │  ┌─────────────────┐                     │
                    │  │   Agent Loop    │ agent.rs            │
                    │  │  (LLM + Tools)  │                     │
                    │  └────┬───┬───┬────┘                     │
                    │       │   │   │                          │
                    │  ┌────┘   │   └────┐                     │
                    │  ▼        ▼        ▼                     │
                    │ Tools   Browser   Desktop                │
                    │ (11)    Agent*    Agent*                  │
                    │  │      (CDP)    (GUI)                    │
                    │  │        │        │                      │
                    │  ▼        ▼        ▼                      │
                    │ ┌──────────────────────┐                 │
                    │ │  Observe-Act Loop*    │                 │
                    │ │  (screenshot→LLM→act) │                 │
                    │ └──────────────────────┘                 │
                    │         │                                │
                    │  ┌──────┴──────────┐                     │
                    │  ▼                 ▼                      │
                    │ Sandbox          Multi-Agent              │
                    │ (Docker/         (teams, roles,           │
                    │  Podman/          sub-agents)             │
                    │  OS-level)                                │
                    │         │                                │
                    │  ┌──────┴──────────┐                     │
                    │  ▼                 ▼                      │
                    │ Memory            Providers               │
                    │ (OpenMemory,      (18 LLMs +              │
                    │  Infinite Ctx,     failover +              │
                    │  Session Store)    health tracker)         │
                    │                                          │
                    └──────────────────────────────────────────┘

                    * = Gap — to be implemented in Phases 1-3
```

---

## 9. Conclusion

VibeCody is not merely a code editor — it is an **agent orchestration platform** with production-grade infrastructure that already surpasses dedicated agent frameworks in multi-agent coordination, provider diversity, memory, and platform reach. The two critical gaps (browser DOM automation and continuous observe-act loop) are well-scoped, buildable on existing foundations, and would position VibeCody as a **superset of Perplexity Computer, OpenClaw, and Operator combined**.
