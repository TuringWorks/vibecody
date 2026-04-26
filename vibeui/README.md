# VibeUI — AI-Powered Code Editor

A modern, high-performance desktop code editor built with Rust + Tauri 2, featuring a full AI agent loop, multi-provider LLM support, LSP integration, Monaco Editor, and a rich extension system.

## Features

### Core Editor
- ✅ **Efficient Text Buffer**: Built on the `ropey` rope data structure for fast editing of large files
- ✅ **File System Operations**: Async file I/O with file watching capabilities
- ✅ **Multi-Workspace Support**: Work with multiple project folders simultaneously
- ✅ **Syntax Highlighting**: Monaco Editor with full language grammar support
- ✅ **LSP Support**: Language Server Protocol client for completions, hover, go-to-definition
- ✅ **Diff View**: Side-by-side diff viewer with Accept/Reject for agent edits
- ✅ **DiffComplete (⌘.)**: Explicit-chord AI edit — select code, describe change, model returns a unified diff for modal review (per-hunk accept/reject, optional Monaco edit-before-apply, regenerate-with-refinement). Patent-distant alternative to keystroke-driven ghost text.
- ✅ **Git Integration**: Branch display, file status indicators, diff view, commit UI
- ✅ **Terminal**: Integrated PTY-backed terminal panel
- ✅ **Browser Panel**: Embedded iframe browser for localhost previews (quick-launch: 3000/5173/8080)
- ✅ **Command Palette**: Fuzzy-search over all editor commands
- ✅ **Theme System**: Dark/Light toggle with CSS custom properties
- ✅ **Resizable Panes**: Drag-to-resize sidebar, terminal panel, AI panel

### AI Integration (17 Providers)
- ✅ **Local**: Ollama (auto-detected), LocalEdit
- ✅ **Cloud**: Claude, OpenAI, Gemini, Grok, Groq, Mistral, DeepSeek, Cerebras, Zhipu
- ✅ **Platform**: OpenRouter, Azure OpenAI, AWS Bedrock, GitHub Copilot, Vercel AI
- ✅ **Meta**: Failover (auto-failover wrapper, chains multiple providers)
- ✅ **BYOK Settings**: In-editor API key management (⚙️ Keys tab), keys stored encrypted in `~/.vibecli/profile_settings.db`
- ✅ **`apiKeyHelper`**: Run a custom script to supply rotating credentials per provider
- ✅ **Multiple Chat Tabs**: Independent chat sessions with per-tab provider selection
- ✅ **Voice Input**: Web Speech API mic button (🎤) with interim transcript and pulse animation
- ✅ **@-context system**: `@file:`, `@folder:`, `@git`, `@web:<url>`, `@terminal`, `@symbol:`, `@codebase:`

### AI Agent (Agentic Mode)
- ✅ **Full Agent Loop**: Multi-step tool-use with streaming events
- ✅ **Approval Policies**: Suggest / Auto-edit / Full-auto per session
- ✅ **Turbo Mode**: One-click ⚡ switch to full-auto (amber highlight)
- ✅ **Plan Mode**: Generate + confirm execution plan before running
- ✅ **Session Trace**: Full JSONL trace saved per session for audit/replay
- ✅ **Checkpoints**: Snapshot + restore workspace state before/after agent runs
- ✅ **MCP Client**: Connect to external Model Context Protocol servers
- ✅ **Hooks System**: Pre/PostToolUse shell hooks with JSON stdin/stdout protocol
- ✅ **Hooks Config UI**: Visual hook editor (🪝 Hooks tab)
- ✅ **Skills**: Auto-loaded `/skills/*.md` slash commands injected into system prompt
- ✅ **Rules Directory**: `.vibecli/rules/*.md` with YAML front-matter path-pattern injection
- ✅ **Admin Policy**: Glob-based tool allow/deny lists with `denied_tool_patterns = ["bash(rm*)"]`
- ✅ **Web Search**: DuckDuckGo Lite integration as agent tool
- ✅ **Background Jobs**: Submit agent tasks to VibeCLI daemon, persist across restarts (📋 Jobs tab)
- ✅ **Manager View**: Launch parallel sub-agents per branch, merge results (🧑‍💼 tab)

### Context Picker (`@` references)
| Syntax | Resolves to |
|--------|-------------|
| `@file:path` | File contents |
| `@file:path:N-M` | Specific line range |
| `@folder:path` | Recursive directory tree |
| `@git` | Current git diff |
| `@web:<url>` | Fetched + stripped web page (6000 char limit) |
| `@terminal` | Last 200 lines of terminal output |
| `@symbol:name` | Symbol search with source snippet |
| `@codebase:query` | Semantic codebase search |

### Observability & Artifacts
- ✅ **Session History**: Browse past agent traces with expandable steps (📜 tab)
- ✅ **Artifacts Panel**: View files, diffs, annotations, task lists created by agent (📦 tab)
- ✅ **Memory Panel**: View/edit `~/.vibecli/memory.md` directly in editor (🧠 tab)
- ✅ **OpenTelemetry**: Optional OTLP/HTTP span export for tracing

### Extension System
- ✅ **WASM Extensions**: Load WebAssembly plugins via `wasmtime`
- ✅ **VS Code API Compatibility**: `vscode.window`, `vscode.workspace`, `vscode.commands` shims
- ✅ **Extension Host Worker**: Sandboxed execution in a Web Worker

---

## Architecture

```
vibeui/
├── src/                        # React + TypeScript frontend
│   ├── App.tsx                 # Root component, Monaco wiring, agent dispatch
│   └── components/
│       ├── AIChat.tsx          # Chat panel + voice input
│       ├── ChatTabManager.tsx  # Multi-tab chat with per-tab provider
│       ├── AgentPanel.tsx      # Agent runner (Turbo, approval, live events)
│       ├── DiffCompleteModal.tsx # ⌘. AI edit modal (diff-mode)
│       ├── DiffReviewPanel.tsx # Per-hunk diff review with Monaco edit-before-apply
│       ├── ContextPicker.tsx   # @ autocomplete menu
│       ├── MemoryPanel.tsx     # ~/.vibecli/memory.md viewer
│       ├── HistoryPanel.tsx    # Trace session browser
│       ├── CheckpointPanel.tsx # Workspace snapshot/restore
│       ├── ArtifactsPanel.tsx  # Agent artifact browser
│       ├── ManagerView.tsx     # Parallel sub-agent orchestration
│       ├── HooksPanel.tsx      # Visual hooks editor
│       ├── BackgroundJobsPanel.tsx # VibeCLI daemon job queue
│       ├── BrowserPanel.tsx    # Embedded iframe browser
│       ├── SettingsPanel.tsx   # BYOK API key management
│       ├── Terminal.tsx        # PTY terminal wrapper
│       ├── GitPanel.tsx        # Git status + diff
│       └── ...
├── src-tauri/                  # Tauri 2 Rust backend
│   └── src/
│       ├── commands.rs         # 60+ Tauri commands
│       ├── agent_executor.rs   # Tool executor (read/write/bash/search/web/MCP)
│       └── lib.rs              # Plugin registration
└── crates/
    ├── vibe-core/              # Text buffer (ropey), FS, Workspace, Git, Terminal, Search
    ├── vibe-ai/                # AIProvider trait + 17 providers + AgentLoop + MCP + Hooks
    ├── vibe-lsp/               # LSP client (jsonrpc + tokio-util)
    └── vibe-extensions/        # WASM extension system (wasmtime)
```

---

## Getting Started

### Prerequisites
- Rust (latest stable)
- Node.js 18+
- npm

### Development

```bash
# Install frontend dependencies
cd vibeui && npm install

# Run in development mode (hot-reload frontend + Rust backend)
npm run tauri dev

# Build for production
npm run tauri build
```

### Testing

```bash
# Run all tests (2,810+ unit tests across workspace)
cargo test --workspace

# Type-check the frontend
cd vibeui && npx tsc --noEmit

# Build Tauri backend only
cargo build --manifest-path vibeui/src-tauri/Cargo.toml
```

---

## AI Provider Configuration

### Ollama (Local — No API Key Needed)
1. Install [Ollama](https://ollama.ai) and start it
2. Pull a model: `ollama pull codellama` or `ollama pull qwen3-coder`
3. VibeUI auto-detects all running Ollama models on startup

### Cloud Providers (via ⚙️ Keys tab in-editor)
Open the **⚙️ Keys** tab in the AI panel and enter your keys:

| Provider | Key variable | Default model |
|----------|-------------|---------------|
| Anthropic Claude | `ANTHROPIC_API_KEY` | `claude-sonnet-4-6` |
| OpenAI | `OPENAI_API_KEY` | `gpt-4o` |
| Google Gemini | `GEMINI_API_KEY` | `gemini-2.0-flash` |
| xAI Grok | `GROK_API_KEY` | `grok-2-latest` |
| Groq | `GROQ_API_KEY` | `llama-3.3-70b-versatile` |
| Mistral | `MISTRAL_API_KEY` | `mistral-large-latest` |
| DeepSeek | `DEEPSEEK_API_KEY` | `deepseek-chat` |
| Cerebras | `CEREBRAS_API_KEY` | `llama3.1-70b` |
| Zhipu | `ZHIPU_API_KEY` | `glm-4` |
| OpenRouter | `OPENROUTER_API_KEY` | `anthropic/claude-3.5-sonnet` |
| Azure OpenAI | `AZURE_OPENAI_API_KEY` | `gpt-4o` |
| AWS Bedrock | `AWS_ACCESS_KEY_ID` | `anthropic.claude-3-5-sonnet` |

Keys can also be set via environment variables or `~/.vibecli/config.toml`.

### `apiKeyHelper` (Rotating Credentials)
For secrets management systems, configure a helper script in `~/.vibecli/config.toml`:
```toml
[claude]
api_key_helper = "~/.vibecli/get-key.sh claude"
```
The script's stdout is used as the Bearer token; static `api_key` is the fallback.

### Extended Thinking (Claude)
```toml
[claude]
thinking_budget_tokens = 10000
```

---

## Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| `Cmd/Ctrl+.` | Open DiffComplete on selected code (or whole file) |
| `Cmd/Ctrl+B` | Toggle sidebar |
| `Cmd/Ctrl+P` / `Cmd/Ctrl+Shift+P` | Command Palette |
| `Esc` | Close any modal |

---

## Contributing

Contributions and suggestions are welcome! See [CONTRIBUTING.md](../docs/contributing.md).

## License

MIT

## Acknowledgments

- Editor powered by [Monaco Editor](https://microsoft.github.io/monaco-editor/)
- Desktop runtime: [Tauri 2](https://tauri.app/)
- Text buffer: [Ropey](https://github.com/cessen/ropey)
- Local AI: [Ollama](https://ollama.ai)
