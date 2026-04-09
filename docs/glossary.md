---
layout: page
title: Glossary
permalink: /glossary/
---


An alphabetical reference of terms used throughout VibeCody's documentation and codebase.


**ACP (Agent Client Protocol)** — An open protocol for communication between AI agents and client applications. VibeCody supports ACP in both server and client modes, enabling interoperability with other agent frameworks.

**Agent** — The core AI loop in VibeCody that receives a task, plans steps, invokes tools, and iterates until the task is complete. The agent orchestrates all AI-powered actions such as code generation, editing, and debugging.

**Agent Teams** — A multi-agent collaboration system where multiple agents with different roles (architect, coder, reviewer, tester) work together on a task, communicating through an inter-agent messaging bus.

**Air-Gapped Mode** — A deployment configuration where VibeCody operates with zero external network access. Uses Ollama for local AI inference and Docker for sandboxing, with no data leaving the host machine.

**Approval Policy** — A configuration that controls how much autonomy the agent has. Three levels: `suggest` (manual approval for each action), `auto-edit` (automatic file edits, manual command approval), and `full-auto` (no approval required).

**Arena Mode** — A blind A/B comparison feature that sends the same prompt to two different providers simultaneously and lets the user evaluate which response is better, without knowing which provider produced which result.

**Artifact** — A generated output from an agent session, such as a code file, configuration, diagram, or document. Artifacts are tracked in the session history and can be exported.

**Batch Builder** — A system for running large-scale autonomous code generation tasks with multiple specialized agent roles, checkpoint intervals, and pause/resume/cancel support. Targets 3M+ lines of code in 8-12 hour autonomous runs.

**Blue Team** — The defensive security module focused on incident response, IOC (Indicator of Compromise) tracking, SIEM integration, forensic case management, and threat hunting. Complements Red Team and Purple Team modules.

**BugBot** — An automated bug detection agent that scans code for potential issues, generates fix suggestions, and can create pull requests with autofix patches.

**BYOK (Bring Your Own Key)** — The model where users supply their own API keys for cloud AI providers rather than routing through a centralized service. VibeCody never stores or proxies API keys on its own servers.

**Channel Daemon** — An always-on background listener (`vibecli serve`) that exposes VibeCody as an HTTP API, enabling integration with chat platforms, CI systems, and other tools.

**Checkpoint** — A snapshot of agent state saved during a session. Checkpoints allow you to revert to a previous point if the agent goes off track.

**Context Bundle** — A shareable, named collection of files, instructions, and configuration that defines a reusable context for the agent. Bundles can be imported, exported, and applied to different projects.

**Context Window** — The maximum number of tokens an AI model can process in a single request. VibeCody's InfiniteContextManager handles context that exceeds this limit through hierarchical summarization, scoring, and eviction.

**ContainerRuntime** — A Rust trait that provides a unified interface for running agent commands in Docker, Podman, or OpenSandbox containers. Defines 16 async methods for container lifecycle management.

**CRDT (Conflict-free Replicated Data Type)** — A data structure used in VibeCody's collaboration system (`vibe-collab`) that allows multiple users to edit the same document simultaneously without conflicts.

**Debug Mode** — An agent mode that automatically sets breakpoints, inspects variables, and traces execution flow to diagnose bugs. Integrates with the agent loop to iteratively narrow down root causes.

**DiffEngine** — A component in `vibe-core` that computes and displays differences between file versions. Supports unified and side-by-side views with syntax-aware line rendering.

**Doctor** — A planned diagnostic command for checking system prerequisites, API key validity, provider connectivity, and configuration integrity. Not yet implemented.

**DORA Metrics** — DevOps Research and Assessment metrics (deployment frequency, lead time, change failure rate, mean time to recovery) tracked by the IDP module's scorecard system.

**EARS (Easy Approach to Requirements Syntax)** — A structured syntax for writing requirements specifications used in VibeCody's spec-driven development pipeline.

**Embeddings** — Dense vector representations of text used for semantic search and retrieval-augmented generation (RAG). VibeCody generates embeddings for code indexing and context retrieval with optimized cosine similarity.

**Extension** — A plugin compiled to WebAssembly (WASM) that extends VibeCody's functionality. Extensions run in a sandboxed WASM runtime via the `vibe-extensions` crate.

**FailoverProvider** — A meta-provider that wraps a chain of AI providers and automatically retries with the next provider in the chain if the current one fails due to timeouts, rate limits, or errors.

**Fast Context** — A high-speed code search system (inspired by SWE-grep) that uses structural pattern matching to find relevant code across large repositories without full indexing.

**Gateway** — The system that connects VibeCody to external messaging platforms. Supports 18 platforms: Telegram, Discord, Slack, Signal, Matrix, Twilio SMS, iMessage, WhatsApp, Teams, IRC, Twitch, WebChat, Nostr, QQ, Tlon, and three additional adapters.

**Golden Path** — In the Internal Developer Platform (IDP) module, a recommended, pre-configured workflow or template for common development tasks. Golden paths encode organizational best practices.

**Hook** — An event-triggered automation script that runs before or after agent actions. Hooks receive JSON on stdin and return JSON on stdout. Exit code 0 allows the action; exit code 2 blocks it.

**IDP (Internal Developer Platform)** — A module supporting 12 platform providers (Backstage, Humanitec, Port, etc.) for service catalogs, golden paths, scorecards, and self-service infrastructure provisioning.

**Infinite Context** — A context management system with a 5-level hierarchy that handles conversations exceeding the model's context window. Uses token budgeting, eviction policies, compression, multi-signal scoring, and LRU caching to maintain relevant context.

**LLM (Large Language Model)** — The AI model that powers VibeCody's code understanding and generation. VibeCody supports LLMs from 23 different providers, both cloud-hosted and local.

**LSP (Language Server Protocol)** — A protocol for providing language intelligence features (completion, diagnostics, go-to-definition). VibeCody includes an LSP client in the `vibe-lsp` crate.

**MCP (Model Context Protocol)** — An open standard for connecting AI models to external data sources and tools. VibeCody supports MCP as both a client and a server, and includes a verified plugin directory.

**Monaco Editor** — The code editor component used in VibeUI, the same editor that powers VS Code. Provides syntax highlighting, IntelliSense, and multi-cursor editing.

**Legacy Migration** — A module for converting codebases from older languages (COBOL, Fortran, VB6, and 15 others) to modern target languages. Supports strategies including Strangler Fig pattern and service boundary detection.

**Ollama** — An open-source tool for running LLMs locally. VibeCody's default local provider, supporting models like Llama 3, Mistral, and DeepSeek without cloud dependencies.

**OpenRouter** — A unified API gateway that provides access to hundreds of AI models from different providers through a single API key. Supported as a first-class provider in VibeCody.

**Orchestration** — The workflow orchestration system that manages task tracking (`tasks/todo.md`), lessons learned (`tasks/lessons.md`), and complexity estimation. Context from orchestration is automatically injected into the agent loop.

**Panel** — A UI component in VibeUI that provides a focused interface for a specific feature. VibeUI includes 187 panels covering AI, security, DevOps, development tools, and more.

**Policy File** — A TOML configuration file (`.vibecli/policy.toml`) that enforces organizational restrictions such as allowed providers, blocked commands, and mandatory sandbox usage. Cannot be overridden by user configuration.

**Provider** — An AI model backend that VibeCody connects to for inference. Providers implement the `AIProvider` trait and handle prompt formatting, streaming, and error handling specific to their API.

**PTY (Pseudo-Terminal)** — A virtual terminal device used by VibeCody's integrated terminal to run shell commands with full terminal emulation, including colors and interactive programs.

**Purple Team** — A security exercise module that combines offensive (red team) and defensive (blue team) techniques using the MITRE ATT&CK framework. Tracks attack simulations, detection validation, and coverage scoring.

**RAG (Retrieval-Augmented Generation)** — A technique that enhances AI responses by retrieving relevant documents from a vector database before generating. VibeCody's RAG pipeline supports multi-format ingestion (Markdown, HTML, PDF, JSON, CSV, XML, code) and multiple vector backends.

**Ratatui** — The Rust TUI library used to build VibeCLI's terminal user interface. Provides widgets, layouts, and rendering for building rich terminal applications.

**Red Team** — The offensive security module for penetration testing simulation. Generates attack scenarios, tracks findings by severity, and produces remediation recommendations.

**REPL (Read-Eval-Print Loop)** — VibeCLI's interactive command-line interface where you type prompts and commands. Supports 93+ slash commands (e.g., `/agent`, `/config`, `/session`, `/counsel`, `/superbrain`) powered by Rustyline.

**Sandbox** — An isolated execution environment (Docker or Podman container) that restricts what the agent can access on the host system. Configured via the `[sandbox]` section in `config.toml`.

**Session** — A conversation between the user and the agent, including all messages, tool calls, and generated artifacts. Sessions are persisted in SQLite and can be resumed later.

**Skill** — A Markdown-based capability definition that gives the agent domain-specific knowledge and instructions. VibeCody ships with 500+ built-in skills and supports custom user-defined skills.

**Soul.md** — A project philosophy document that captures high-level design principles, values, and architectural decisions. Used as persistent context for the agent across sessions.

**SSRF (Server-Side Request Forgery)** — A class of vulnerability where an attacker tricks a server into making requests to unintended destinations. VibeCody prevents SSRF through URL scheme validation in the tool executor.

**Steering** — A system for adjusting agent behavior through natural-language directives without changing the underlying model. Steering rules are persisted and applied automatically to agent prompts.

**Streaming** — The real-time delivery of AI model responses token-by-token as they are generated, rather than waiting for the full response. VibeCody supports streaming for all cloud providers and most local models.

**SWE-bench** — A benchmark suite for evaluating AI coding assistants on real-world software engineering tasks. VibeCody includes a benchmarking harness for running, comparing, and exporting SWE-bench results.

**Tauri** — The framework used to build VibeUI. Tauri v2 combines a Rust backend with a web frontend (React/TypeScript), producing lightweight, secure desktop applications that use the system WebView.

**Token** — The basic unit of text processed by an AI model. A token is roughly 4 characters or 0.75 words in English. Token counts determine context window usage and API costs.

**Trace** — A JSONL audit log that records every action the agent takes during a session, including tool calls, model responses, and timing information. Stored with `-messages.json` and `-context.json` sidecars.

**TUI (Terminal User Interface)** — VibeCLI's graphical terminal interface built with Ratatui, providing visual panels, syntax highlighting, and keyboard navigation within the terminal.

**Usage Metering** — A credit-based system for tracking and limiting AI API consumption across users, projects, and teams. Supports budgets, threshold alerts, and chargeback reporting.

**Vector Database** — A storage system for embedding vectors used in retrieval-augmented generation (RAG). VibeCody supports in-memory vectors with cosine/euclidean/dot/manhattan similarity, plus schema generation for Qdrant, Pinecone, and pgvector.

**VibeCLI** — The command-line interface for VibeCody, located in `vibecli/vibecli-cli/`. Provides a TUI, REPL, and HTTP daemon mode.

**VibeUI** — The desktop application for VibeCody, located in `vibeui/`. Built with Tauri 2, React, and Monaco Editor. Provides a full graphical IDE experience with 187 integrated panels.

**WASM (WebAssembly)** — A binary instruction format used for VibeCody's extension system. WASM extensions run in a sandboxed runtime, providing safe third-party plugin execution.

**Workspace** — The root directory of a project that VibeCody operates on. The workspace defines the boundary for file operations, indexing, and search. Configured automatically from the current working directory or explicitly via command-line flags.

**WorktreeManager** — A Rust trait that decouples the AI crate (`vibe-ai`) from the core crate (`vibe-core`), allowing the agent to interact with the filesystem and workspace through an abstract interface.

**XML Tool Calling** — VibeCody's approach to tool invocation where the agent uses XML-formatted tool calls in the system prompt. This format works consistently across all providers, unlike provider-specific function calling APIs.
