---
layout: page
title: Development Guide
permalink: /development/
---


Internal guide for engineers contributing to VibeCody. Covers build procedures, testing, debugging, code organization, and common development workflows.

## Prerequisites

| Tool | Version | Purpose |
|------|---------|---------|
| Rust | stable (1.77+) | Backend, shared crates, CLI |
| Node.js | LTS 18+ | VibeUI frontend |
| pnpm/npm | latest | Frontend package management |
| Tauri CLI | 2.x | Desktop app builds |
| Docker | 20+ | Container sandbox, on-prem deployment |
| sqlite3 | 3.x | Database panel (optional) |

```bash
# Verify toolchain
rustup show && node --version && cargo tauri --version
```

## Repository Structure

```text
vibecody/
├── Cargo.toml                    # Workspace root (9 members)
├── vibecli/vibecli-cli/          # VibeCLI binary crate
│   ├── src/main.rs               # CLI entry, REPL loop, 126 slash commands
│   ├── src/tool_executor.rs      # Agent tool execution (file I/O, bash, search)
│   ├── src/project_init.rs       # Smart project auto-detection
│   ├── src/gateway.rs            # 18-platform messaging gateway + channel daemon
│   ├── src/channel_daemon.rs     # Always-on daemon with automation routing
│   ├── src/branch_agent.rs       # Agent-per-branch workflow
│   ├── src/spec_pipeline.rs      # EARS spec-driven development
│   ├── src/vm_orchestrator.rs    # Parallel VM agent orchestration
│   └── skills/                   # 599 skill files
├── vibeui/
│   ├── src/                      # React + TypeScript frontend
│   │   ├── App.tsx               # Root component, keyboard shortcuts
│   │   └── components/           # 235+ panel components (plus 39 composites)
│   ├── src-tauri/src/
│   │   ├── lib.rs                # Tauri command registration (1,045+ commands)
│   │   ├── commands.rs           # All Tauri command implementations
│   │   └── agent_executor.rs     # Agent tool execution for VibeUI
│   └── crates/
│       ├── vibe-ai/              # AIProvider trait, agent loop, 22 providers
│       ├── vibe-core/            # Text buffer, filesystem, git, search
│       ├── vibe-lsp/             # LSP client
│       ├── vibe-extensions/      # WASM extension system
│       └── vibe-collab/          # CRDT collaboration
├── docs/                         # Jekyll documentation site
├── jetbrains-plugin/             # IntelliJ/WebStorm plugin
└── neovim-plugin/                # Neovim integration
```

## Build Commands

### Quick Reference

```bash
# Check everything compiles (fastest feedback loop)
cargo check --workspace --exclude vibe-collab

# Build CLI binary (release)
cargo build --release -p vibecli

# Run all tests
cargo test --workspace --exclude vibe-collab

# Run tests for a specific crate
cargo test -p vibecli
cargo test -p vibe-ai
cargo test -p vibe-core

# Run tests matching a pattern
cargo test -p vibecli -- project_init
cargo test -p vibecli -- channel_daemon::tests

# VibeUI development mode
cd vibeui && npm install && npm run tauri:dev

# Clippy (linting)
cargo clippy --workspace --exclude vibe-collab -- -W clippy::all

# Format check
cargo fmt --all -- --check
```

### Linux Notes

On Linux, the npm `rustup` package can shadow the system cargo binary. If you see unexpected build failures:

```bash
# Check which cargo is being used
which cargo
# Should be ~/.cargo/bin/cargo, NOT node_modules/.bin/cargo
```

See `linux-dev-setup.md` for full Linux environment setup.

### VibeUI Development

```bash
cd vibeui
npm install          # Install frontend dependencies
npm run tauri:dev    # Start Tauri dev server (use tauri:dev not tauri dev on Linux)
npm run lint         # ESLint
npm run typecheck    # TypeScript type checking
```

## Testing

### Test Organization

| Crate | Tests | Focus |
|-------|-------|-------|
| vibecli | ~5,600+ | CLI commands, tool executor, providers, security, all feature modules |
| vibe-ai | ~1,020+ | Provider implementations, agent loop, circuit breaker, tracing |
| vibe-core | ~290+ | Text buffer, filesystem, git, search, embeddings |
| vibe-ui | ~230+ | Tauri commands, agent executor, panel components |
| vibe-extensions | ~46 | WASM extension loading, manifest parsing |
| vibe-lsp | ~34 | LSP client protocol |

### Running Tests

```bash
# Full workspace (~10,535 tests)
cargo test --workspace --exclude vibe-collab

# Single crate with output
cargo test -p vibecli -- --nocapture

# Single test function
cargo test -p vibecli -- project_init::tests::scan_rust_project --nocapture

# Tests matching a keyword
cargo test -p vibecli -- security
cargo test -p vibe-core -- safe_command
```

### Writing Tests

Tests live in `#[cfg(test)] mod tests { }` blocks at the bottom of each module. Follow these conventions:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    // Helper functions at the top
    fn test_fixture() -> MyStruct {
        MyStruct::default()
    }

    // Test names: {function_under_test}_{scenario}
    #[test]
    fn parse_query_empty_string_returns_none() {
        assert!(parse_query("").is_none());
    }

    // Async tests use #[tokio::test]
    #[tokio::test]
    async fn fetch_url_blocks_internal_ips() {
        let result = fetch("http://169.254.169.254").await;
        assert!(result.is_err());
    }
}
```

## Architecture Patterns

### AI Provider Trait

All 22 AI providers implement `AIProvider` (in `vibe-ai/src/provider.rs`):

```rust
#[async_trait]
pub trait AIProvider: Send + Sync {
    fn name(&self) -> &str;
    fn is_available(&self) -> bool;
    async fn chat(&self, messages: &[Message], context: Option<String>) -> Result<String>;
    async fn stream_chat(&self, messages: &[Message]) -> Result<CompletionStream>;
    async fn complete(&self, context: &CodeContext) -> Result<CompletionResponse>;
    async fn stream_complete(&self, context: &CodeContext) -> Result<CompletionStream>;
}
```

To add a new provider:

1. Create `vibeui/crates/vibe-ai/src/providers/my_provider.rs`
2. Register in `providers.rs`: `pub mod my_provider;` + `pub use my_provider::MyProvider;`
3. Add config handling in `vibecli/vibecli-cli/src/main.rs` `create_provider()` function
4. Add to `Config` struct in `vibecli/vibecli-cli/src/config.rs`

### Agent Loop

The agent loop (`vibe-ai/src/agent.rs`) follows a plan-act-observe cycle:

```text
User task → System prompt (tools + context) → LLM stream
  → Parse tool calls → Check hooks/approval → Execute tool
  → Feed result back → Repeat until task_complete
```

Key components:

- **CircuitBreaker** — Detects stalls, spins, degradation (4 health states)
- **Hooks** — Pre/post tool execution interception (JSON stdin/stdout)
- **Context pruning** — Keeps within 80K token budget
- **Think tool** — Free reasoning step (no side effects, no step count)

### Tool Execution

Two executor implementations:

- **`vibecli/src/tool_executor.rs`** — CLI executor with sandbox support, SSRF validation, command blocklist
- **`vibeui/src-tauri/src/agent_executor.rs`** — Tauri executor with workspace-boundary path validation, command blocklist, 120s timeout

### Tauri Commands

VibeUI exposes 1,045+ Tauri commands. Each is a `#[tauri::command]` function in `commands.rs`:

```rust
#[tauri::command]
pub async fn my_command(param: String) -> Result<ResponseType, String> {
    // Implementation
}
```

Register in `lib.rs`:

```rust
tauri::generate_handler![
    commands::my_command,
    // ...
]
```

## Security Checklist

Before submitting code that touches these areas, verify:

### File I/O

- [ ] All user-supplied paths go through `safe_resolve_path()` or `TauriToolExecutor::resolve()`
- [ ] No absolute paths outside workspace are allowed
- [ ] Symlink resolution via `canonicalize()` prevents traversal

### Shell Execution

- [ ] Commands pass through the blocklist (`is_blocked_command` / `is_safe_command`)
- [ ] Timeout applied (120s default for agent bash, 300s for scripts)
- [ ] User-visible commands from AI responses are filtered before execution

### Network Requests

- [ ] URLs validated with `validate_url_for_ssrf()` — blocks loopback, RFC 1918, link-local, metadata
- [ ] Only `http://` and `https://` schemes allowed
- [ ] No `file:///` access

### Secrets

- [ ] API keys stored with `chmod 0o600` file permissions
- [ ] Trace files run through `redact_secrets()` before writing
- [ ] Error messages don't expose API keys or internal paths
- [ ] Test fixtures use clearly fake keys (e.g., `sk-abcdefghij1234567890`)

### SQL

- [ ] SQLite dot-commands blocked (`.shell`, `.system`, `.import`, `.load`)
- [ ] `ATTACH DATABASE` blocked
- [ ] Path validation before opening database files

### Dependencies

- [ ] No `unsafe` blocks (none in the codebase currently)
- [ ] No `danger_accept_invalid_certs(true)`
- [ ] Cryptographic operations use `hmac`/`sha2` crates, not hand-rolled implementations

## Adding a REPL Command

1. Add the command string to `vibecli/vibecli-cli/src/repl.rs` `COMMANDS` array
2. Add the match arm in `main.rs` (search for the `_ =>` fallthrough near line 6100+)
3. Implement the handler (typically calling into a dedicated module)

Example:

```rust
// In repl.rs COMMANDS array:
"/mycommand",

// In main.rs match block:
"/mycommand" => {
    let sub = args.trim().split_whitespace().next().unwrap_or("help");
    match sub {
        "list" => { /* implementation */ }
        _ => println!("Usage: /mycommand [list|create|delete]\n"),
    }
}
```

## Adding a VibeUI Panel

1. Create `vibeui/src/components/MyPanel.tsx`
2. Follow the tab-bar pattern used by other panels:

```tsx
export function MyPanel({ workspacePath }: { workspacePath?: string | null }) {
  const [tab, setTab] = useState<"overview" | "detail">("overview");
  // ... panel implementation
}
```

1. Import and add to the AI panel tab list in `App.tsx`
2. If the panel needs Rust data, add a `#[tauri::command]` and register it

## Debugging

### VibeCLI

```bash
# Verbose logging
RUST_LOG=debug vibecli --provider ollama

# Trace agent steps
vibecli --agent "task" --json 2>&1 | jq .

# Inspect trace files
ls ~/.vibecli/traces/
cat ~/.vibecli/traces/<session-id>.jsonl | jq .
```

### VibeUI

```bash
# Open with DevTools
cd vibeui && npm run tauri:dev
# Press F12 or Cmd+Opt+I for WebView DevTools

# Rust backend logs
RUST_LOG=debug npm run tauri:dev
```

### Common Issues

| Issue | Fix |
|-------|-----|
| `cargo check` hangs | Check for build directory lock: `ls -la target/.cargo-lock` |
| Tauri build fails on Linux | Install system deps: `sudo apt install libwebkit2gtk-4.1-dev libappindicator3-dev` |
| `vibe-collab` won't compile | Exclude it: `--exclude vibe-collab` (requires specific CRDT dependencies) |
| Tests fail with "provider not available" | These are integration tests needing API keys — unit tests should all pass |
| npm `rustup` shadows cargo | Use `~/.cargo/bin/cargo` directly or uninstall the npm rustup package |

## Release Process

```bash
# 1. Update version in Cargo.toml
# 2. Update CHANGELOG.md
# 3. Build release binaries
cargo build --release -p vibecli

# 4. Cross-platform builds via CI
# See .github/workflows/release.yml

# 5. Docker image
docker build -t vibecody/vibecli:latest .

# 6. Install script verification
./install.sh  # SHA-256 verified download
```

## Key Module Reference

| Module | Lines | Tests | Purpose |
|--------|-------|-------|---------|
| `main.rs` | ~7,800 | — | CLI entry, REPL loop, 126 slash command handlers |
| `tool_executor.rs` | ~1,800 | 28+ | Agent tool execution with sandbox |
| `agent.rs` | ~1,500 | 40+ | Agent loop, circuit breaker, system prompt |
| `tools.rs` | ~500 | 20+ | Tool definitions, XML parsing, think tool |
| `project_init.rs` | ~1,300 | 17 | Smart project auto-detection and caching |
| `gateway.rs` | ~2,200 | 18+ | 18-platform gateway + channel daemon |
| `channel_daemon.rs` | ~1,300 | 47 | Always-on daemon, event routing, sessions |
| `branch_agent.rs` | ~1,500 | 56 | Agent-per-branch, auto-PR |
| `spec_pipeline.rs` | ~1,700 | 64 | EARS spec-driven development |
| `vm_orchestrator.rs` | ~1,300 | 59 | Parallel VM agent execution |
| `commands.rs` | ~30,000 | 227+ | All Tauri command implementations |
| `agent_executor.rs` | ~350 | 12 | VibeUI agent tool executor |

## Performance Notes

- `agent.rs`: `String::with_capacity(8192)` pre-allocates for LLM responses
- `embeddings.rs`: O(n) update, fused cosine similarity, shared HTTP client via `OnceLock`
- `search.rs`: Cached `WalkDir` metadata for fast file enumeration
- `tool_executor.rs`: Async I/O via `tokio::process::Command`, O(n) HTML entity decode
- Context pruning keeps agent within 80K tokens (configurable via `with_context_limit()`)
- Release profile: LTO enabled, symbols stripped, `panic=abort`, `opt-level=s`
