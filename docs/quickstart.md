---
layout: page
title: Quickstart
permalink: /quickstart/
---


**Zero to productive in 5 minutes.**


## What is VibeCody?

VibeCody is an AI-powered developer toolchain built in Rust. It gives you two ways to work: **VibeCLI**, a terminal-first AI coding assistant with a rich TUI and REPL, and **VibeUI**, a full desktop code editor with Monaco and 187 AI panels. Both share the same backend crates, supporting 23 AI providers (local and cloud), an autonomous agent loop, code review, multi-agent orchestration, MCP integration, and 568 built-in skills. You can start with a local model and zero API keys.


## Choose Your Surface

| | **VibeCLI** | **VibeUI** |
|---|---|---|
| **Best for** | Terminal users, CI/CD, scripting | Visual editing, panel-rich workflows |
| **Interface** | TUI (Ratatui) or REPL | Desktop app (Tauri + Monaco) |
| **Setup time** | 2 minutes | 5 minutes (needs Node.js) |
| **Works headless** | Yes | No |
| **AI features** | All 23 providers, agent, review, skills | All CLI features + visual panels |

**Recommendation:** Start with VibeCLI. You can add VibeUI later -- they share the same config and crates.


## Install in 60 Seconds

Pick one method:

### Option A: Build from Source

Requires Rust stable (1.75+) and Git.

```bash
git clone https://github.com/TuringWorks/vibecody.git
cd vibecody
cargo build --release -p vibecli
```

The binary lands at `./target/release/vibecli`. Optionally copy it to your PATH:

```bash
cp target/release/vibecli /usr/local/bin/
```

### Option B: One-Liner Installer

Downloads the latest release binary for your platform (macOS and Linux, x86_64 and ARM):

```bash
curl -fsSL https://raw.githubusercontent.com/TuringWorks/vibecody/main/install.sh | sh
```

The binary is installed to `~/.local/bin/vibecli` by default. Override with:

```bash
INSTALL_DIR=/usr/local/bin curl -fsSL https://raw.githubusercontent.com/TuringWorks/vibecody/main/install.sh | sh
```

### Option C: Docker

Run VibeCLI in a container with Ollama as a sidecar (no host dependencies):

```bash
git clone https://github.com/TuringWorks/vibecody.git
cd vibecody
docker-compose up
```

This starts VibeCLI with a local Ollama instance. No API keys required.


## Your First Chat

Launch VibeCLI with no arguments to enter REPL mode:

```bash
vibecli
```

You will see the prompt:

```
VibeCLI v0.3.3 — AI coding assistant
Provider: ollama (qwen3-coder:480b-cloud)
Type a message or /help for commands.

vibecli>
```

Type a question:

```sh
vibecli> What does the #[derive(Debug)] macro do in Rust?
```

Expected output (streamed):

```sh
The #[derive(Debug)] attribute macro automatically implements the
`Debug` trait for a struct or enum, allowing you to print it with
`{:?}` formatting in println!, dbg!, or format!.

Example:
  #[derive(Debug)]
  struct Point { x: f64, y: f64 }

  let p = Point { x: 1.0, y: 2.0 };
  println!("{:?}", p);  // Point { x: 1.0, y: 2.0 }
```

That is it -- you are chatting with an AI. Press `Ctrl+C` or type `/quit` to exit.


## Your First Agent Task

The agent loop lets VibeCody autonomously read files, write code, and run commands. Use `--agent` for interactive mode or `--exec` for non-interactive (CI) mode:

```bash
# Interactive mode (asks for approval on each step)
vibecli --agent "add error handling to main.rs"

# Non-interactive mode (full-auto, JSON output)
vibecli --exec "add error handling to main.rs"
```

Example output (interactive mode with default `suggest` policy):

```sh
 Agent   add error handling to main.rs
  Policy: suggest (ask before every action)  |  Press Ctrl+C to stop

 ✓ Reading src/main.rs
 ✓ Searching: "error handling"

  bash  Running: cargo check
   Approve? (y/n/a=approve-all): y

 ✓ Running: cargo check
 ✓ Patching src/main.rs (3 hunks)

Agent complete: Added Result<()> return type, wrapped I/O in match blocks.
   Files modified: src/main.rs
   Commands run: 1
   Steps: 4/4 succeeded
   Trace saved: ~/.vibecli/traces/1711234567.jsonl
   Resume with: vibecli --resume 1711234567
```

In `suggest` mode (default), the agent asks before shell commands and file writes. Type `y` to approve, `n` to reject, or `a` to auto-approve all remaining steps.

### Approval Policies

| Flag | Behavior |
|------|----------|
| *(default)* | Ask before every edit and command |
| `--auto-edit` | Auto-apply file edits; ask before shell commands |
| `--full-auto` | Auto-execute everything (use with `--sandbox`) |

You can also use `/agent <task>` from the REPL to start agent tasks interactively, and `/plan <task>` to review a plan before executing.


## Connect a Cloud Provider

Local Ollama works out of the box, but cloud providers give you access to larger models. Here is how to connect Claude as an example.

**Step 1:** Get an API key from [console.anthropic.com](https://console.anthropic.com/).

**Step 2:** Set the environment variable:

```bash
export ANTHROPIC_API_KEY="sk-ant-your-key-here"
```

Add the line to your `~/.bashrc` or `~/.zshrc` to persist it.

**Step 3:** Launch with Claude:

```bash
vibecli --provider claude
```

Expected output:

```sh
VibeCLI v0.3.3 — AI coding assistant
Provider: claude (claude-sonnet-4-6)

vibecli>
```

**Step 4:** Verify it works:

```sh
vibecli> Hello, which model am I talking to?
```

You should see Claude identify itself. Done.

Other providers use the same pattern:

| Provider | Env Variable | Launch Flag |
|----------|-------------|-------------|
| OpenAI | `OPENAI_API_KEY` | `--provider openai` |
| Gemini | `GEMINI_API_KEY` | `--provider gemini` |
| Grok | `GROK_API_KEY` | `--provider grok` |
| Ollama | *(none)* | `--provider ollama` |

See the [Configuration Guide](/vibecody/configuration/) for all 23 providers.


## Your First Code Review

Navigate to any Git repository with uncommitted changes and run:

```bash
vibecli --review
```

Or from inside the REPL:

```sh
vibecli> /review
```

Expected output:

```sh
[review] Analyzing diff (3 files, +47 -12 lines)...

## Code Review Summary

### src/auth.rs (2 issues)
  [HIGH] Line 34: Unwrap on network call will panic in production.
         Suggestion: Use `?` operator or handle the error explicitly.
  [MED]  Line 51: Password comparison is not constant-time.
         Suggestion: Use `subtle::ConstantTimeEq` to prevent timing attacks.

### src/main.rs (1 issue)
  [LOW]  Line 12: Unused import `std::collections::HashMap`.
         Suggestion: Remove the import.

3 issues found (1 high, 1 medium, 1 low).
```

You can also review a GitHub PR directly:

```sh
vibecli> /review --pr 42
```

See the [Code Review Tutorial](/vibecody/tutorials/code-review/) for more options.


## Next Steps

You are up and running. Here is where to go next:

| Goal | Link |
|------|------|
| Set up more AI providers | [First Provider Tutorial](/vibecody/tutorials/first-provider/) |
| Learn the agent workflow | [Agent Workflow Tutorial](/vibecody/tutorials/agent-workflow/) |
| Deep-dive on code review | [Code Review Tutorial](/vibecody/tutorials/code-review/) |
| Browse all tutorials | [Tutorials Index](/vibecody/tutorials/) |
| Configure VibeCLI fully | [Configuration Guide](/vibecody/configuration/) |
| Set up the desktop editor | [VibeUI Reference](/vibecody/vibeui/) |
| Full CLI reference | [VibeCLI Reference](/vibecody/vibecli/) |


## Common Issues

### 1. "Connection refused" when using Ollama

Ollama must be running before you launch VibeCLI.

```bash
# Start the Ollama server
ollama serve

# In another terminal, pull a model if you have not already
ollama pull qwen3-coder:480b-cloud

# Now launch VibeCLI
vibecli
```

### 2. "API key not found" for cloud providers

Make sure the environment variable is exported in your current shell:

```bash
# Check if it is set
echo $ANTHROPIC_API_KEY

# If empty, export it
export ANTHROPIC_API_KEY="sk-ant-your-key-here"
```

To persist the key, add the `export` line to your `~/.bashrc` or `~/.zshrc`. In VibeUI, you can also store keys securely via the **Settings** panel, which saves them to the encrypted ProfileStore (`~/.vibecli/profile_settings.db`).

> **Security note:** Never store API keys in plaintext files such as `config.toml` or `.json` files. VibeCody's encrypted ProfileStore is the only safe persistent storage for secrets.

### 3. "cargo build" fails with missing dependencies

On Linux, you may need system libraries for TLS and terminal support:

```bash
# Ubuntu/Debian
sudo apt install pkg-config libssl-dev

# Fedora
sudo dnf install openssl-devel

# macOS (if using Homebrew OpenSSL)
brew install openssl
```

Then retry `cargo build --release -p vibecli`.
