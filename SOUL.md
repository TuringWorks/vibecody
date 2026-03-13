# The Soul of VibeCody

## Why VibeCody Exists

The AI coding tool market is dominated by closed-source products that lock developers into specific providers, collect telemetry without consent, and treat the developer's own codebase as someone else's training data. VibeCody exists because developers deserve better.

VibeCody is an open-source, provider-agnostic developer toolchain where **you own everything** -- your tools, your data, your choice of AI, and your workflow. It runs on your machine, connects to any model you choose (or no model at all), and never phones home.

## Core Beliefs

### The developer is sovereign

Your code stays on your machine. Your conversations stay on your machine. Your configuration, your history, your memories -- all local files you can read, edit, and delete. No cloud accounts required. No telemetry. No lock-in. If VibeCody disappeared tomorrow, you'd lose nothing but the tool itself.

### Provider choice is a right, not a feature

AI is a commodity. Today's best model is tomorrow's second-best. VibeCody treats providers as interchangeable adapters behind a single trait. Seventeen providers ship today -- from local Ollama to cloud APIs -- and the FailoverProvider can chain them automatically. You should never be stuck because one company had a bad day.

### The terminal is not a lesser interface

VibeCLI is not a consolation prize for people who can't install an IDE. It's a first-class tool with its own TUI, REPL, agent loop, voice input, and 536 skill files. Many of our most powerful features -- red team scanning, workflow orchestration, batch generation, gateway bots -- were born in the terminal. The CLI and the desktop editor share the same Rust crates and the same capabilities.

### Simplicity over cleverness

A `config.toml` file you can read. JSONL trace logs you can `grep`. Skill files that are plain Markdown. Hooks that are shell scripts with exit codes. We choose boring, inspectable formats over clever abstractions. If you can't understand how something works by reading the file, we've failed.

### Ship the tool, not the promise

Every feature in VibeCody exists in code, has tests, and can be built from source today. We don't ship roadmap items as bullet points. If it's in the documentation, you can run it. If it doesn't work, that's a bug, not a "coming soon."

### Security is non-negotiable

Sandbox-by-default command execution. Approval gates before destructive actions. Path traversal prevention. SSRF validation. Crypto-random identifiers. Rate limiting. These aren't features -- they're the floor. An AI tool with access to your filesystem and shell has an enormous trust surface. We take that seriously.

## Design Principles

### Shared crates, separate surfaces

`vibe-core`, `vibe-ai`, `vibe-lsp`, and `vibe-extensions` are the foundation. VibeCLI and VibeUI are just different frontends to the same capabilities. A fix in `vibe-ai` improves both the terminal and the desktop. A new provider works everywhere instantly. This is not accidental -- it's the most important architectural decision in the project.

### Traits over implementations

The `AIProvider` trait. The `ContainerRuntime` trait. The `WorktreeManager` trait. VibeCody is built on abstract interfaces that decouple capabilities from specific implementations. This is what makes 17 providers possible without 17 codepaths, and what lets Docker, Podman, and OpenSandbox share a single integration surface.

### Tests are not optional

Over 5,300 tests run on every change. Zero failures is the baseline, not the goal. If you add a module, you add its tests. If you fix a bug, you add a regression test. The test suite is the project's immune system.

### Accessible by default

Modal focus traps. ARIA labels. Keyboard navigation. Screen reader support. Skip-to-content links. These aren't checkboxes on an accessibility audit -- they're part of how we think about UI from the start. A tool that only works for some developers isn't a good tool.

## What VibeCody Is Not

- **Not a cloud service.** There is no VibeCody account, no VibeCody server, no VibeCody subscription. You bring your own API keys or run local models.
- **Not a VS Code fork.** VibeUI is built from scratch with Tauri and Monaco. It shares VS Code's editor component, not its architecture, extension model, or telemetry.
- **Not a wrapper around one model.** VibeCody works with Claude, GPT, Gemini, Grok, Groq, Mistral, Ollama, and many others -- including offline. The agent loop uses XML tool calling that works with any instruction-following model.
- **Not a startup's growth hack.** VibeCody is MIT licensed. There are no paid tiers, no premium features, no usage limits. The full tool is the free tool.

## How to Know If a Change Belongs

Before adding a feature, ask:

1. **Does it respect developer sovereignty?** If it requires phoning home, creating an account, or sending data to a third party the developer didn't explicitly choose -- it doesn't belong.
2. **Does it work with any provider?** If it only works with one AI model or service, it needs a provider-agnostic abstraction first.
3. **Is it tested?** If you can't write tests for it, reconsider whether it's well-defined enough to ship.
4. **Is it inspectable?** Can a developer understand what it does by reading local files? Can they debug it with standard Unix tools?
5. **Does it earn its complexity?** A feature that helps one workflow but complicates ten others is a net negative. The bar for adding complexity is high. The bar for removing it is low.

## The Name

"Vibe" -- because the best development sessions have a flow state to them. The tool should amplify that feeling, not interrupt it with configuration dialogs, authentication flows, or loading spinners. Get in, do the work, ship it. That's the vibe.
