---
layout: page
title: "Demo 51: Profiles & Session Management"
permalink: /demos/51-profiles-sessions/
---


## Overview

VibeCody supports named profiles and persistent sessions. Profiles let you maintain separate configurations for work, personal projects, or specific clients -- each with its own provider, model, approval policy, and rules. Sessions automatically save your conversation history and can be resumed later, even across machine restarts. Together, these features let you context-switch between projects without losing state.

**Time to complete:** ~10 minutes

## Prerequisites

- VibeCLI 0.5.1 or later installed and on your PATH
- At least one AI provider configured in `~/.vibecli/config.toml`
- For session resumption: an existing session (created during normal REPL use)

## Step-by-Step Walkthrough

### Step 1: Create a work profile

Profiles are TOML files stored in `~/.vibecli/profiles/`. Create one for your work environment:

```bash
mkdir -p ~/.vibecli/profiles
cat > ~/.vibecli/profiles/work.toml << 'EOF'
[provider]
name = "claude"
model = "claude-sonnet-4-6"
api_key_env = "ANTHROPIC_API_KEY"

[approval]
auto_approve = ["read_file", "list_files", "search"]
require_approval = ["write_file", "execute_command", "delete_file"]

[rules]
project_rules = [
  "Follow the company coding standards in CONTRIBUTING.md",
  "Always write tests for new functions",
  "Use structured logging, never println!",
]

[context]
default_files = ["README.md", "CONTRIBUTING.md"]
max_tokens = 100000
EOF
```

### Step 2: Launch with a profile

```bash
vibecli --profile work
```

```
vibecli 0.5.1 | Profile: work | Provider: claude | Model: claude-sonnet-4-6
Approval: auto-approve reads, require approval for writes
Rules: 3 project rules loaded
Type /help for commands, /quit to exit

>
```

The profile overrides the default configuration. The provider, model, approval policy, and rules all come from `work.toml`.

### Step 3: Create a personal profile

```bash
cat > ~/.vibecli/profiles/personal.toml << 'EOF'
[provider]
name = "ollama"
model = "llama3"

[approval]
auto_approve = ["all"]

[rules]
project_rules = []

[context]
max_tokens = 8000
EOF
```

Now switch between them:

```bash
vibecli --profile personal
```

```
vibecli 0.5.1 | Profile: personal | Provider: ollama | Model: llama3
Approval: auto-approve all tools
Type /help for commands, /quit to exit

>
```

### Step 4: List available sessions

Sessions are saved automatically during REPL use. View them with:

```bash
vibecli
```

```
> /sessions list
```

```
Saved Sessions:

  ID          Created              Duration   Messages  Profile   Summary
  ─────────────────────────────────────────────────────────────────────────
  sess_a1b2   2026-03-29 10:15     42m        28        work      Refactored auth module
  sess_c3d4   2026-03-28 14:30     1h 15m     64        work      Built inventory API
  sess_e5f6   2026-03-27 09:00     25m        12        personal  Explored Axum routing
  sess_g7h8   2026-03-25 16:45     18m        8         default   Fixed CI pipeline

4 sessions found. Use /resume <id> to continue a session.
```

### Step 5: Resume a previous session

```
> /resume sess_c3d4
```

```
Resuming session sess_c3d4 (2026-03-28, "Built inventory API")...

Session restored:
  Messages:  64 (conversation history loaded)
  Profile:   work (provider: claude, model: claude-sonnet-4-6)
  Files:     src/inventory.rs, src/models.rs, tests/inventory_test.rs
  Last msg:  "The inventory API endpoints are complete. Want me to add
             pagination to the list endpoint?"

Context restored. You can continue where you left off.

> Yes, add cursor-based pagination to the list endpoint.
```

```
I'll add cursor-based pagination to the inventory list endpoint. Let me
update the handler and add the pagination types...

[Tool: write_file] src/inventory.rs — adding CursorPage struct and
paginated list handler
[Tool: write_file] src/models.rs — adding PaginationParams
[Tool: write_file] tests/inventory_test.rs — adding pagination tests

Done. The list endpoint now accepts `?cursor=<id>&limit=20` parameters
and returns a `CursorPage<InventoryItem>` with `next_cursor` for the
client to fetch the next page.
```

### Step 6: View session details

```
> /sessions list --verbose sess_c3d4
```

```
Session: sess_c3d4
  Created:    2026-03-28 14:30:22 UTC
  Updated:    2026-03-29 10:42:18 UTC
  Duration:   1h 27m (across 2 resumptions)
  Messages:   67
  Profile:    work
  Provider:   claude (claude-sonnet-4-6)
  Tokens:     input: 142,800 | output: 38,400
  Est. cost:  $0.54

  Files touched:
    src/inventory.rs         (4 writes)
    src/models.rs            (2 writes)
    tests/inventory_test.rs  (3 writes)
    src/routes.rs            (1 write)

  Tool usage:
    read_file:        18
    write_file:       10
    execute_command:   6
    search:            4
```

### Step 7: Profile-specific environment

Profiles can also set environment-specific behavior:

```bash
cat > ~/.vibecli/profiles/staging.toml << 'EOF'
[provider]
name = "claude"
model = "claude-sonnet-4-6"

[approval]
auto_approve = ["read_file", "search"]
require_approval = ["write_file", "execute_command"]
blocked = ["delete_file"]

[rules]
project_rules = [
  "This is a staging environment -- never run destructive commands",
  "Always check the current git branch before making changes",
  "Prefix all test data with 'staging_' to avoid polluting production",
]

[sandbox]
enabled = true
network = false
EOF
```

```bash
vibecli --profile staging
```

```
vibecli 0.5.1 | Profile: staging | Provider: claude | Model: claude-sonnet-4-6
Approval: auto-approve reads, require writes, block deletes
Sandbox: enabled (network disabled)
Rules: 3 project rules loaded
Type /help for commands, /quit to exit

>
```

The staging profile blocks `delete_file` entirely and enables sandboxed execution with no network access, preventing accidental damage.

## Profile Inheritance

Profiles inherit from the base `~/.vibecli/config.toml`. Only the fields you specify in the profile override the defaults. This means you only need to define what differs:

```toml
# minimal profile -- only changes the model
[provider]
model = "claude-opus-4-6"
```

Everything else (API keys, approval policies, rules) falls through from the base config.

## Demo Recording

```json
{
  "meta": {
    "title": "Profiles & Session Management",
    "description": "Create profiles for different environments, list and resume sessions, and switch contexts without losing state.",
    "duration_seconds": 180,
    "version": "1.0.0"
  },
  "steps": [
    {
      "id": 1,
      "action": "shell",
      "command": "mkdir -p ~/.vibecli/profiles && cat > ~/.vibecli/profiles/work.toml << 'EOF'\n[provider]\nname = \"claude\"\nmodel = \"claude-sonnet-4-6\"\n\n[approval]\nauto_approve = [\"read_file\", \"list_files\", \"search\"]\nrequire_approval = [\"write_file\", \"execute_command\"]\nEOF",
      "description": "Create work profile",
      "delay_ms": 2000
    },
    {
      "id": 2,
      "action": "shell",
      "command": "vibecli --profile work --version",
      "description": "Launch with work profile",
      "expected_output_contains": "0.5.1",
      "delay_ms": 1500
    },
    {
      "id": 3,
      "action": "repl",
      "commands": [
        { "input": "/sessions list", "delay_ms": 3000 },
        { "input": "/resume sess_c3d4", "delay_ms": 4000 },
        { "input": "Add cursor-based pagination to the list endpoint", "delay_ms": 8000 },
        { "input": "/quit", "delay_ms": 500 }
      ],
      "description": "List sessions, resume one, and continue working"
    }
  ]
}
```

## What's Next

- [Demo 48: OpenMemory](../48-open-memory/) -- Persistent cognitive memory engine
- [Demo 50: Warp-Style Features](../50-warp-features/) -- Natural language commands and secret redaction
- [Demo 52: Watch Mode & Sandbox](../52-watch-sandbox/) -- File watching and isolated execution
