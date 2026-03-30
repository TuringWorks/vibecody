---
layout: page
title: FAQ
permalink: /faq/
---

# Frequently Asked Questions


## General

### What is VibeCody?

VibeCody is an open-source AI coding assistant that runs locally as a CLI tool (VibeCLI) and as a desktop editor (VibeUI). It connects to multiple AI providers and gives you agent-powered code generation, editing, debugging, and project management in your own environment.

### Is VibeCody free?

Yes. VibeCody is released under the MIT License. You bring your own API keys for cloud AI providers, or use local models (Ollama) at no cost. There are no usage fees from VibeCody itself.

### What is the difference between VibeCLI and VibeUI?

**VibeCLI** is a terminal-based interface with a TUI (built on Ratatui) and a REPL. It is ideal for SSH sessions, CI pipelines, and developers who prefer the command line.

**VibeUI** is a desktop application built with Tauri 2, React, and Monaco Editor. It provides a graphical code editor with 187 integrated panels for AI interaction, debugging, deployment, security, and more.

Both share the same Rust backend crates (vibe-core, vibe-ai, vibe-lsp).

### Which AI provider should I use?

It depends on your priorities:

- **Best quality:** Claude (Anthropic) or GPT-4o (OpenAI)
- **Best speed:** Gemini Flash (Google) or Groq
- **Best privacy / offline:** Ollama with a local model (Llama 3, Mistral, DeepSeek)
- **Best value:** DeepSeek or OpenRouter (access to many models, competitive pricing)
- **Enterprise:** Azure OpenAI or AWS Bedrock for managed deployments

### Can I use VibeCody offline / air-gapped?

Yes. Set up Ollama as your provider and use Docker for sandboxing. The `docker-compose.yml` in the repository includes an Ollama sidecar configuration for fully air-gapped operation with zero network egress.

```bash
docker-compose up  # Starts VibeCLI + Ollama, no external calls
```


## Providers

### How many AI providers does VibeCody support?

VibeCody supports 23 AI providers: Ollama, Claude, OpenAI, Gemini, Grok, Groq, OpenRouter, Azure OpenAI, Bedrock, Copilot, LocalEdit, Mistral, Cerebras, DeepSeek, Zhipu, Vercel AI, MiniMax, Perplexity, Together AI, Fireworks AI, SambaNova, plus the FailoverProvider meta-provider.

### Can I use multiple providers at once?

Yes, in several ways:

- **FailoverProvider** chains providers so requests automatically fall back on failure.
- **Arena Mode** runs two providers side-by-side for blind A/B comparison.
- **Agent Teams** can assign different providers to different agent roles.

### What is the FailoverProvider?

The FailoverProvider is a meta-provider that wraps a chain of other providers. If the first provider fails (timeout, rate limit, error), it automatically retries with the next provider in the chain.

```toml
[provider]
name = "failover"
chain = ["claude", "openai", "gemini"]
```

### How do I switch providers?

In the CLI:

```bash
vibecli --provider ollama
vibecli --provider claude
```

Or set a default in `~/.vibecli/config.toml`:

```toml
[provider]
name = "claude"
```

In VibeUI, use the Keys panel to configure and switch providers.

### What is OpenRouter?

[OpenRouter](https://openrouter.ai/) is a unified API gateway that provides access to hundreds of AI models from different providers through a single API key. VibeCody supports OpenRouter as a first-class provider, letting you access models from Anthropic, OpenAI, Meta, Mistral, and others without managing separate API keys for each.


## Privacy & Security

### Does VibeCody send my code to the cloud?

Only when you use a cloud AI provider. When you send a prompt, the relevant code context is sent to the selected provider's API. VibeCody does not send telemetry, analytics, or code to any VibeCody servers.

If you use Ollama or another local model, no code leaves your machine.

### How do I use VibeCody without sending data externally?

1. Use **Ollama** as your provider with a locally downloaded model.
2. Disable network access in the sandbox configuration.
3. Use the Docker Compose air-gapped setup.

See the [Security](/vibecody/security/) page for full details.

### What is the sandbox?

The sandbox runs agent commands inside an isolated container (Docker or Podman). This prevents the agent from accessing your host filesystem, network, or sensitive resources beyond what you explicitly allow. Configure it in `~/.vibecli/config.toml` under `[sandbox]`.

### How do approval policies work?

Approval policies control how much autonomy the agent has:

- **suggest** — The agent proposes changes; you approve or reject each one.
- **auto-edit** — The agent can edit files automatically but must ask before running commands.
- **full-auto** — The agent can edit files and run commands without asking. Use with sandbox enabled.


## Extensibility

### How do I create a custom skill?

Skills are Markdown files placed in `~/.vibecli/skills/` or the project-level `skills/` directory. Each file defines a capability with instructions the agent follows when the skill is invoked.

```markdown
name: my-custom-skill
description: Does something useful

# Instructions

When the user asks to [do something], follow these steps:
1. ...
2. ...
```

VibeCody ships with 500+ built-in skills covering cloud, security, data, DevOps, and more.

### How do I add a new AI provider?

Implement the `AIProvider` trait in `vibeui/crates/vibe-ai/src/provider.rs`. The trait requires methods for `chat`, `stream`, and `models`. See any existing provider (e.g., `ollama.rs`, `claude.rs`) as a reference. Register the new provider in the provider factory and add it to the CLI argument parser.

### What are WASM extensions?

VibeCody supports WebAssembly (WASM) extensions through the `vibe-extensions` crate. Extensions are compiled to WASM and run in a sandboxed runtime, allowing third-party plugins without native code execution risks.

### What is MCP?

MCP (Model Context Protocol) is an open standard for connecting AI models to external data sources and tools. VibeCody supports MCP both as a client (consuming MCP servers) and as a server (exposing VibeCody tools to other MCP clients). The MCP Directory panel in VibeUI lets you browse and install verified MCP plugins.


## Enterprise

### Can I self-host VibeCody?

Yes. VibeCody is fully self-hostable. The repository includes:

- A `Dockerfile` for building a static VibeCLI binary (Alpine-based, multi-stage musl build).
- A `docker-compose.yml` for running VibeCLI with an Ollama sidecar.
- An HTTP daemon mode (`vibecli serve`) for team deployments behind a reverse proxy.

### Is there team or enterprise support?

VibeCody is open source (MIT). Enterprise features available in the codebase include:

- **Usage Metering** — Per-user, per-project, and per-team credit budgets with alerts.
- **Admin Policies** — Centralized restriction policies via `.vibecli/policy.toml`.
- **Audit Trail** — Full JSONL traces of every agent action and session.
- **RBAC** — Role-based access control in the Admin panel.
- **SOC 2 Controls** — Compliance controls module with PII redaction and retention policies.

### SOC 2 / compliance support?

The `compliance_controls.rs` module provides technical controls aligned with SOC 2 requirements, including audit trail logging, PII detection and redaction, data retention policies, and exportable compliance reports. See the [Security](/vibecody/security/) page for details.

### How does usage metering work?

The usage metering system tracks token consumption and API costs across users, projects, and teams. You can set budgets, configure alerts at threshold percentages, and generate chargeback reports. Manage it via the `/metering` REPL command or the UsageMetering panel in VibeUI.


## More Questions?

If your question is not answered here, check the [Troubleshooting](/vibecody/troubleshooting/) guide or open an issue on [GitHub](https://github.com/TuringWorks/vibecody/issues).
