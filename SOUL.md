
# The Soul of Vibecody

## Our Core Philosophy

Vibecody is built on the belief that code understanding should be intuitive, contextual, and deeply integrated into the developer's workflow. We don't just index code—we understand its *vibe*.

## What Drives Us

### **Context Over Content**

We prioritize understanding the *why* behind code over just the *what*. Every function, class, and module has a story to tell about the intentions of its creators.

### **Developer Empathy**

Every feature we build starts with a simple question: "How does this make the developer's life easier?" We're not just building tools; we're crafting experiences.

### **Universal Accessibility**

Code understanding shouldn't be locked behind specific IDEs or platforms. Whether you're in VS Code, JetBrains, Neovim, the terminal, a desktop editor, a phone, a tablet, a web browser, an Apple Watch, or a Wear OS device — Vibecody is there, speaking the same protocol against the same Rust daemon.

## Our North Star

> "To create a world where every line of code speaks clearly, every repository tells its story, and every developer feels understood."

## How We Live This Daily

### In Our Codebase

- **One daemon, many faces.** The VibeCLI Rust daemon (`vibecli/`) is the single source of truth for protocol, auth, pairing, and AI orchestration. Every other surface — `vibeui/` desktop editor, `vibeapp/` secondary shell, `vibemobile/` Flutter app, `vibewatch/` Apple Watch + Wear OS clients (with paired iOS / Android companions), `vscode-extension/`, `jetbrains-plugin/`, `neovim-plugin/`, `packages/agent-sdk/`, and the standalone `vibe-indexer/` — is a thin client over that one API. If a client disagrees with the daemon, the client is wrong.
- **Modular Rust workspace.** `vibe-core` (buffers, FS, Git), `vibe-ai` (22 AI providers + failover), `vibe-lsp`, `vibe-extensions` (Wasmtime), and `vibe-collab` (CRDT) are shared crates reused across every Rust artifact.
- **Cross-device continuity.** Apple-Handoff-style handoff between desktop, phone, and watch; Google-Docs-style full-content sync (no truncation); zero-config mDNS / Tailscale / ngrok connectivity so the experience follows you regardless of network.
- **Real-time understanding.** We process and index code as you write it, and the understanding is accessible from every surface above.

### In Our Community

- **Open Source First**: Transparency in development and clear contribution paths
- **Developer-Centric Documentation**: Not just API references, but guides that teach concepts
- **Feedback Loops**: We listen, we learn, we iterate

### In Our Vision

- **Beyond Search**: Moving from finding code to understanding systems
- **Collaborative Intelligence**: Making team knowledge as accessible as individual knowledge
- **Learning Systems**: Codebases that get smarter over time

## Our Promise

When you use Vibecody, you're not just getting another development tool—you're gaining a teammate that understands your codebase as deeply as you do, and sometimes even better.

---

*This document is alive, just like our code. Feel free to suggest changes.*
