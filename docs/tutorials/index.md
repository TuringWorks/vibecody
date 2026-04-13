---
layout: page
title: Tutorials
permalink: /tutorials/
---


Hands-on guides to get the most out of VibeCody. Each tutorial is self-contained -- pick the one that matches what you want to do.


## Beginner

| Tutorial | Time | Description |
|----------|------|-------------|
| [Setting Up Your First AI Provider](./first-provider/) | 10 min | Connect Ollama, Claude, or OpenAI and verify it works |
| [Using the Agent to Fix Bugs](./agent-workflow/) | 15 min | Run the agent loop to fix bugs, add features, and refactor code |
| [AI-Powered Code Review](./code-review/) | 10 min | Review uncommitted changes, branches, and GitHub PRs |
| Using @ Context References | 10 min | Attach files, URLs, and symbols to your prompts with `@` |

| [Project Init & Auto-Context](./project-init/) | 10 min | `/init` project scanning, auto-context in every agent conversation |


## Intermediate

| Tutorial | Time | Description |
|----------|------|-------------|
| Multi-Provider Arena Comparisons | 15 min | Blind A/B test two models and build a leaderboard |
| Creating Custom Skills | 20 min | Write skill files that trigger on keywords and inject context |
| Setting Up MCP Servers | 15 min | Connect external tools via Model Context Protocol |
| Configuring Hooks for CI/CD | 15 min | Run pre/post hooks on agent actions for validation and logging |
| [Building a Long-Term Memory Store](./memory/) | 20 min | Memories, facts, verbatim drawers, recall benchmarking, cross-project tunnels |

| [Always-On Channel Daemon](./channel-daemon/) | 20 min | Run a persistent bot on Slack/Discord/GitHub with automation rules |


## Advanced

| Tutorial | Time | Description |
|----------|------|-------------|
| Building a Custom AI Provider | 30 min | Implement the `AIProvider` trait for a new backend |
| WASM Extension Development | 30 min | Build and load WASM plugins with `vibe-extensions` |
| Multi-Agent Orchestration | 25 min | Coordinate agent teams with the inter-agent messaging bus |
| Deploying VibeCody for Teams | 20 min | HTTP daemon mode, RBAC, usage metering, and team governance |
| Air-Gapped Deployment with Ollama | 15 min | Docker Compose setup with no internet access |
| Agent-Per-Branch Workflow | 15 min | Isolated git branches per task, auto-commit, auto-PR |
| Spec-Driven Development (EARS) | 20 min | Structured requirements → design → tasks pipeline |
| VM Agent Orchestration | 20 min | Run 4+ parallel agents in isolated Docker containers |
| Vulnerability Scanner | 15 min | CVE scanning, SAST rules, lockfile parsing |
| [Model Wizard](/vibecody/tutorials/model-wizard/) | 30 min | Fine-tune, quantize, and deploy custom models in 7 steps |


## How to Use These Tutorials

1. **Start with the [Quickstart](../../quickstart/)** if you have not installed VibeCody yet.
2. **Pick a tutorial** from the table above based on your goal.
3. **Follow it step by step** -- each tutorial lists prerequisites at the top.
4. **Refer to the reference docs** ([VibeCLI](../../vibecli/), [Configuration](../../configuration/)) for full details on any command or option.


## Suggest a Tutorial

Something missing? Open an issue on [GitHub](https://github.com/TuringWorks/vibecody/issues) with the label `docs`.
