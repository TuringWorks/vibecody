---
layout: page
title: "Demo 20: Agent Teams"
permalink: /demos/agent-teams/
nav_order: 20
parent: Demos
---

# Demo 20: Agent Teams

## Overview

Agent Teams enable you to spin up multiple AI agents with specialized roles -- Architect, Coder, Reviewer, Tester, and SecurityAuditor -- that collaborate on a task through an inter-agent messaging bus. Each agent has its own system prompt, tool access, and domain expertise. A team coordinator delegates subtasks, collects results, and synthesizes a final output. Teams can run locally or in Docker-based cloud agents for isolated, reproducible execution.

**Time to complete:** ~15 minutes

## Prerequisites

- VibeCody installed and configured with at least one AI provider
- Docker installed (required for cloud agent execution)
- (Optional) VibeUI installed for the graphical Agent Teams panel

## Step-by-Step Walkthrough

### Step 1: Create a team

Open the VibeCLI REPL and create a new agent team.

```bash
vibecli repl
```

```
/team create "feature-build" --roles architect,coder,reviewer,tester
```

Expected output:

```
Created team 'feature-build' with 4 agents:
  - Architect   (plans structure, defines interfaces)
  - Coder       (implements code changes)
  - Reviewer    (reviews code for quality and correctness)
  - Tester      (writes and runs tests)
```

### Step 2: Start a team task

Assign a high-level task to the team. The coordinator breaks it into subtasks and delegates to the appropriate agents.

```
/team start "feature-build" --task "Add a user preferences API with CRUD endpoints, validation, and tests"
```

The team coordinator produces a plan and begins execution:

```
[Architect] Planning API structure...
  -> Defined endpoints: GET/POST/PUT/DELETE /api/v1/preferences/:user_id
  -> Schema: UserPreferences { theme, language, notifications, timezone }
  -> Delegating implementation to Coder

[Coder] Implementing endpoints...
  -> Created src/api/preferences.rs (4 handlers)
  -> Created src/models/preferences.rs (struct + validation)
  -> Delegating review to Reviewer

[Reviewer] Reviewing code changes...
  -> 2 suggestions: add input length validation, use transactions for PUT
  -> Sending feedback to Coder

[Coder] Applying review feedback...
  -> Updated validation logic
  -> Added database transaction wrapper
  -> Delegating to Tester

[Tester] Generating and running tests...
  -> Created tests/api/preferences_test.rs (12 tests)
  -> All 12 tests passing
```

### Step 3: Monitor team status

```
/team status "feature-build"
```

```
Team: feature-build
Status: Running (3 of 4 phases complete)
Duration: 2m 14s

Agents:
  Architect       DONE     Plan delivered (4 endpoints, 1 model)
  Coder           DONE     8 files modified, 2 review rounds
  Reviewer        DONE     2 suggestions (both applied)
  Tester          RUNNING  12/12 tests written, executing...

Messages exchanged: 14
```

### Step 4: Send a message to a specific agent

You can inject instructions into any agent mid-run.

```
/team message "feature-build" --agent tester "Also add a test for concurrent preference updates"
```

```
[Tester] Acknowledged. Adding concurrency test...
  -> Added test_concurrent_preference_update (passed)
  -> Total: 13 tests passing
```

### Step 5: Add the SecurityAuditor role

```
/team add-role "feature-build" security-auditor
```

```
[SecurityAuditor] Scanning preferences API...
  -> Checking for injection vulnerabilities: PASS
  -> Checking for authorization bypass: PASS
  -> Checking for mass assignment: WARNING - all fields updatable
  -> Recommendation: Add field-level write permissions
```

### Step 6: Configure team governance policies

Governance policies control how agents interact, what approvals are required, and budget limits.

```
/team governance "feature-build" --require-review --max-tokens 50000 --timeout 10m
```

```
Governance updated for 'feature-build':
  Review required: yes (Reviewer must approve before merge)
  Token budget: 50,000 (currently used: 12,340)
  Timeout: 10 minutes
```

### Step 7: Run agents in Docker (cloud execution)

For isolated execution, run agents inside Docker containers.

```
/team start "feature-build" --cloud --task "Refactor the auth module"
```

```
[Cloud] Pulling agent image vibecody/agent:latest...
[Cloud] Starting 4 containers (1 per agent)...
[Cloud] Architect container: running (ID: a3b2c1d)
[Cloud] Coder container: running (ID: e4f5g6h)
[Cloud] Reviewer container: running (ID: i7j8k9l)
[Cloud] Tester container: running (ID: m0n1o2p)
```

Cloud agents have full filesystem isolation. Changes are collected and presented as a unified diff when the team completes.

### Step 8: Using Agent Teams in VibeUI

Open the **Agent Teams** panel from the sidebar.

1. **Create Tab** -- Select roles from checkboxes, name your team, set governance.
2. **Monitor Tab** -- Live view of agent activity with message log and progress bars.
3. **Messages Tab** -- Send messages to individual agents or broadcast to the team.
4. **History Tab** -- Browse previous team runs, review diffs, and replay sessions.

The monitor view shows a Mermaid-style diagram of agent interactions updated in real time.

## Agent Roles Reference

| Role | Responsibility | Tools Available |
|------|---------------|-----------------|
| **Architect** | Plans structure, defines interfaces, creates specs | File read, search, diagram generation |
| **Coder** | Implements code changes based on plans | File read/write, shell commands, LSP |
| **Reviewer** | Reviews code quality, correctness, style | File read, diff view, lint |
| **Tester** | Writes tests, runs test suites, reports coverage | File read/write, test runner, coverage |
| **SecurityAuditor** | Scans for vulnerabilities, checks OWASP compliance | File read, security scanner, dependency audit |

## CLI Command Reference

| Command | Description |
|---------|-------------|
| `/team create <name> --roles <roles>` | Create a new agent team |
| `/team start <name> --task <description>` | Start a team on a task |
| `/team start <name> --cloud --task <desc>` | Start in Docker containers |
| `/team status <name>` | View team progress and agent states |
| `/team message <name> --agent <role> <msg>` | Send a message to a specific agent |
| `/team add-role <name> <role>` | Add an agent role to a running team |
| `/team governance <name> [flags]` | Set governance policies |
| `/team stop <name>` | Stop a running team |
| `/team list` | List all teams and their statuses |

## Demo Recording

```json
{
  "demoRecording": {
    "version": "1.0",
    "title": "Agent Teams Demo",
    "description": "Create a multi-agent team, delegate a feature task, and observe coordinated execution",
    "duration_seconds": 240,
    "steps": [
      {
        "timestamp": 0,
        "action": "repl_command",
        "command": "/team create \"feature-build\" --roles architect,coder,reviewer,tester",
        "output": "Created team 'feature-build' with 4 agents:\n  - Architect\n  - Coder\n  - Reviewer\n  - Tester",
        "narration": "Create a team with four specialized agent roles"
      },
      {
        "timestamp": 20,
        "action": "repl_command",
        "command": "/team start \"feature-build\" --task \"Add a user preferences API with CRUD endpoints, validation, and tests\"",
        "output": "[Architect] Planning API structure...\n  -> Defined 4 endpoints\n  -> Delegating to Coder",
        "narration": "Assign a feature task to the team coordinator"
      },
      {
        "timestamp": 45,
        "action": "agent_event",
        "agent": "Coder",
        "event": "implementation_started",
        "details": "Creating src/api/preferences.rs and src/models/preferences.rs",
        "narration": "The Coder agent begins implementing the Architect's plan"
      },
      {
        "timestamp": 75,
        "action": "agent_event",
        "agent": "Reviewer",
        "event": "review_started",
        "details": "Reviewing 8 changed files, found 2 suggestions",
        "narration": "The Reviewer agent identifies improvements"
      },
      {
        "timestamp": 100,
        "action": "agent_event",
        "agent": "Coder",
        "event": "feedback_applied",
        "details": "Applied 2 review suggestions: input validation, transaction wrapper",
        "narration": "The Coder applies review feedback automatically"
      },
      {
        "timestamp": 120,
        "action": "repl_command",
        "command": "/team status \"feature-build\"",
        "output": "Team: feature-build\nStatus: Running (3 of 4 phases complete)\nAgents:\n  Architect  DONE\n  Coder      DONE\n  Reviewer   DONE\n  Tester     RUNNING",
        "narration": "Check team progress -- three agents done, Tester still running"
      },
      {
        "timestamp": 140,
        "action": "repl_command",
        "command": "/team message \"feature-build\" --agent tester \"Also add a test for concurrent preference updates\"",
        "output": "[Tester] Acknowledged. Adding concurrency test...\n  -> Total: 13 tests passing",
        "narration": "Send a direct message to the Tester agent"
      },
      {
        "timestamp": 165,
        "action": "repl_command",
        "command": "/team add-role \"feature-build\" security-auditor",
        "output": "[SecurityAuditor] Scanning preferences API...\n  -> 3 checks passed, 1 warning",
        "narration": "Add a SecurityAuditor to scan the new code"
      },
      {
        "timestamp": 190,
        "action": "repl_command",
        "command": "/team governance \"feature-build\" --require-review --max-tokens 50000",
        "output": "Governance updated: review required, token budget 50,000",
        "narration": "Configure governance policies for the team"
      },
      {
        "timestamp": 210,
        "action": "ui_interaction",
        "panel": "AgentTeams",
        "tab": "Monitor",
        "action_detail": "view_agent_graph",
        "narration": "View the live agent interaction diagram in VibeUI"
      },
      {
        "timestamp": 230,
        "action": "agent_event",
        "agent": "Coordinator",
        "event": "team_complete",
        "details": "All agents finished. 8 files modified, 13 tests passing, 0 vulnerabilities.",
        "narration": "The team completes with a unified summary"
      }
    ]
  }
}
```

## What's Next

- [Demo 21: CRDT Collaboration](../crdt-collab/) -- Pair with teammates on the same file in real time
- [Demo 22: Gateway Messaging](../gateway/) -- Run your AI agents across 18 messaging platforms
- Combine agent teams with context bundles so each role gets domain-specific context
