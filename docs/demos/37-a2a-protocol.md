---
layout: page
title: "Demo 37: A2A (Agent-to-Agent) Protocol"
permalink: /demos/37-a2a-protocol/
---


## Overview

VibeCody implements the Agent-to-Agent (A2A) protocol, enabling VibeCLI instances and external AI agents to discover each other, exchange capabilities, and delegate tasks over a standardized JSON-RPC interface. You can expose your local agent as an A2A service, discover agents on your network or the internet, and call remote agents to handle specialized tasks such as code review, security scanning, or documentation generation.

**Time to complete:** ~10 minutes

## Prerequisites

- VibeCLI v0.5.1 installed and on your PATH
- At least one AI provider configured
- Network access between agents (localhost for local demos, or open ports for remote)
- For VibeUI: the desktop app running with the **A2A** panel visible

## A2A Concepts

| Concept       | Description                                                       |
|---------------|-------------------------------------------------------------------|
| **Agent Card** | JSON manifest describing an agent's name, skills, endpoint, auth |
| **A2A Server** | An agent exposing its capabilities over HTTP/JSON-RPC            |
| **Discovery**  | Finding agents via mDNS, a registry URL, or direct endpoint      |
| **Task Call**  | Sending a task to a remote agent and receiving a streamed result  |

## Step-by-Step Walkthrough

### 1. Generate Your Agent Card

An agent card describes what your VibeCLI instance can do. Generate one from your current configuration.

**REPL:**

```bash
vibecli
> /a2a card
```

Example output:

```
Agent Card Generated:
{
  "name": "vibecody-alice",
  "description": "VibeCody AI coding assistant",
  "version": "0.5.1",
  "url": "http://localhost:7860",
  "capabilities": {
    "streaming": true,
    "push_notifications": false,
    "state_transition_history": true
  },
  "skills": [
    {
      "id": "code-review",
      "name": "Code Review",
      "description": "Review code for bugs, style, and security issues",
      "tags": ["review", "security", "quality"]
    },
    {
      "id": "code-generation",
      "name": "Code Generation",
      "description": "Generate code from natural language descriptions",
      "tags": ["generation", "scaffold"]
    },
    {
      "id": "test-generation",
      "name": "Test Generation",
      "description": "Generate unit and integration tests",
      "tags": ["testing", "quality"]
    },
    {
      "id": "documentation",
      "name": "Documentation",
      "description": "Generate and update documentation",
      "tags": ["docs", "markdown"]
    }
  ],
  "authentication": {
    "schemes": ["bearer"]
  }
}

Saved to: ~/.vibecli/a2a-card.json
```

You can edit `~/.vibecli/a2a-card.json` to customize the agent name, description, and skill list.

### 2. Start an A2A Server

Expose your VibeCLI instance as an A2A-compatible agent server.

**REPL:**

```bash
vibecli
> /a2a serve
```

Example output:

```
A2A Server starting...
  Endpoint:    http://0.0.0.0:7860/.well-known/agent.json
  Agent name:  vibecody-alice
  Skills:      4 registered
  Auth:        bearer token (set A2A_TOKEN env var or use --a2a-token)
  mDNS:        broadcasting on local network

A2A server is running. Press Ctrl+C to stop.
Waiting for incoming task requests...
```

The server exposes:
- `GET /.well-known/agent.json` -- returns the agent card
- `POST /a2a/tasks/send` -- submit a task (JSON-RPC)
- `POST /a2a/tasks/sendSubscribe` -- submit a task with streaming response

**CLI (non-interactive):**

```bash
vibecli --serve --a2a --port 7860
```

### 3. Discover External Agents

Find other A2A agents on your local network or via a registry URL.

**REPL:**

```bash
vibecli
> /a2a discover
```

Example output:

```
Discovering A2A agents...

Local Network (mDNS):
  1. vibecody-bob       http://192.168.1.42:7860   Skills: code-review, security-scan
  2. vibecody-carol     http://192.168.1.55:7860   Skills: documentation, translation
  3. qa-agent           http://192.168.1.70:8080   Skills: test-generation, coverage

Registry (https://a2a.vibecody.dev):
  4. docgen-service     https://docgen.example.com Skills: documentation, api-docs
  5. security-scanner   https://sec.example.com    Skills: sast, dast, dependency-audit

Found 5 agents. Use /a2a call <name> "task" to delegate.
```

You can also discover a specific agent by URL:

```bash
vibecli
> /a2a discover http://192.168.1.42:7860
```

Example output:

```
Agent: vibecody-bob
  Version:  0.5.1
  Skills:   code-review, security-scan
  Streaming: yes
  Auth:     bearer token required
```

### 4. Call a Remote Agent

Delegate a task to a discovered agent. The remote agent processes the task and streams results back.

**REPL:**

```bash
vibecli
> /a2a call vibecody-bob "Review src/main.rs for security issues"
```

Example output:

```
Calling agent: vibecody-bob
  Endpoint: http://192.168.1.42:7860
  Task:     Review src/main.rs for security issues
  Skill:    code-review

Streaming response from vibecody-bob:

## Security Review: src/main.rs

### Findings

1. **[HIGH] Unsanitized user input on line 45**
   The `user_query` variable is passed directly to `format!()` without
   escaping. This could allow prompt injection if the input comes from
   an external source.

   Recommendation: Sanitize input before interpolation.

2. **[MEDIUM] Hardcoded timeout on line 78**
   The HTTP client uses a 30-second timeout. Consider making this
   configurable to prevent hanging in CI environments.

3. **[LOW] Unused import on line 3**
   `std::fs::File` is imported but never used.

Summary: 1 high, 1 medium, 1 low severity finding.

Task completed. Duration: 4.2s
```

### 5. Call with Authentication

If the remote agent requires a bearer token, pass it inline or set it in your config.

**REPL:**

```bash
vibecli
> /a2a call security-scanner "Scan this project for vulnerabilities" --token sk-a2a-abc123
```

Or configure tokens persistently in `~/.vibecli/config.toml`:

```toml
[a2a.agents.security-scanner]
url = "https://sec.example.com"
token = "sk-a2a-abc123"
```

### 6. Chain Multiple Agents

Delegate a multi-step workflow across agents.

**REPL:**

```bash
vibecli
> /a2a call vibecody-bob "Review src/auth.rs" | /a2a call qa-agent "Generate tests for these findings"
```

Example output:

```
Step 1: vibecody-bob reviewing src/auth.rs...
  Found 2 issues (1 high, 1 medium)

Step 2: qa-agent generating tests...
  Generated 3 test cases covering the reported issues:
    - test_auth_rejects_empty_token
    - test_auth_handles_expired_jwt
    - test_auth_rate_limits_failed_attempts

Tests written to: tests/auth_security_tests.rs
```

### 7. Monitor A2A Activity in VibeUI

Open the **A2A** panel in VibeUI to see:

- **Agent Card** tab: edit your agent card, toggle skills on/off
- **Server** tab: view incoming requests, active tasks, connection log
- **Discovery** tab: browse discovered agents, test connectivity
- **Tasks** tab: history of sent and received tasks with status and duration

## Configuration Reference

Add A2A settings to `~/.vibecli/config.toml`:

```toml
[a2a]
enabled = true
port = 7860
agent_name = "vibecody-alice"
mdns = true
registry_url = "https://a2a.vibecody.dev"

[a2a.auth]
scheme = "bearer"
token = "your-server-token"
```

## Demo Recording JSON

```json
{
  "meta": {
    "title": "A2A (Agent-to-Agent) Protocol",
    "description": "Expose VibeCLI as an A2A agent, discover peers, and delegate tasks to remote agents.",
    "duration_seconds": 180,
    "version": "0.5.1"
  },
  "steps": [
    {
      "id": 1,
      "action": "repl",
      "commands": [
        { "input": "/a2a card", "delay_ms": 3000 }
      ],
      "description": "Generate an agent card from current configuration"
    },
    {
      "id": 2,
      "action": "repl",
      "commands": [
        { "input": "/a2a serve", "delay_ms": 5000 }
      ],
      "description": "Start the A2A server"
    },
    {
      "id": 3,
      "action": "repl",
      "commands": [
        { "input": "/a2a discover", "delay_ms": 4000 }
      ],
      "description": "Discover agents on the local network and registry"
    },
    {
      "id": 4,
      "action": "repl",
      "commands": [
        { "input": "/a2a call vibecody-bob \"Review src/main.rs for security issues\"", "delay_ms": 8000 }
      ],
      "description": "Delegate a code review task to a remote agent"
    },
    {
      "id": 5,
      "action": "vibeui_interaction",
      "panel": "A2A",
      "tab": "Discovery",
      "description": "Browse discovered agents and test connectivity in VibeUI"
    },
    {
      "id": 6,
      "action": "vibeui_interaction",
      "panel": "A2A",
      "tab": "Tasks",
      "description": "View task history with status and duration"
    }
  ]
}
```

## What's Next

- [Demo 20: Agent Teams](../20-agent-teams/) -- Coordinate multiple local agents with roles
- [Demo 38: Parallel Worktrees](../38-parallel-worktrees/) -- Run agents in isolated git worktrees
- [Demo 16: MCP Server Integration](../16-mcp-servers/) -- Connect external tool servers via MCP
