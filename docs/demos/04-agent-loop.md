---
layout: page
title: "Demo 4: Agent Loop & Tool Execution"
permalink: /demos/agent-loop/
nav_order: 4
parent: Demos
---

# Demo 4: Agent Loop & Tool Execution

## Overview

The agent loop is VibeCody's most powerful feature. Instead of just chatting, the AI can read files, edit code, run shell commands, search your codebase, and interact with your LSP server -- all in an autonomous loop with human oversight. This demo walks through a complete agent session, from task definition to code changes with checkpointing and rollback.

**Time to complete:** ~12 minutes

## Prerequisites

- VibeCLI installed and configured with an AI provider that supports tool use (Claude, OpenAI, Gemini)
- A code project to work with (any language)
- Familiarity with [Demo 1: First Run](../01-first-run/)

## How the Agent Loop Works

The agent follows a think-act-observe cycle:

```
User Task
    |
    v
[1. Think] -- AI reasons about what to do next
    |
    v
[2. Tool Call] -- AI invokes a tool (ReadFile, EditFile, Shell, etc.)
    |
    v
[3. Observe] -- Tool result is fed back to the AI
    |
    v
[4. Decide] -- Continue (go to step 1) or finish
    |
    v
Final Response
```

The AI uses XML-based tool calling that works across all providers, regardless of whether they support native function calling.

## Step-by-Step Walkthrough

### Step 1: Start an agent session

```bash
vibecli agent "Fix the login bug in src/auth.rs -- users are getting a 401 even with valid tokens"
```

Or for an interactive agent session where you can guide the process:

```bash
vibecli agent --interactive "Refactor the database module"
```

<!-- Screenshot placeholder: Agent session starting -->
![Agent session start](../assets/screenshots/demo-04-agent-start.png)

### Step 2: Watch the agent think

The agent first analyzes the task and plans its approach. You will see the thinking output:

```
[Thinking] I need to understand the login flow. Let me start by reading the auth module
to understand how token validation works.

I'll:
1. Read src/auth.rs to understand the current implementation
2. Look for the token validation logic
3. Identify the bug
4. Fix it with proper error handling
```

### Step 3: Tool types

The agent has access to these built-in tools:

| Tool | Description | Example |
|------|-------------|---------|
| `ReadFile` | Read file contents | `ReadFile { path: "src/auth.rs" }` |
| `WriteFile` | Create or overwrite a file | `WriteFile { path: "new.rs", content: "..." }` |
| `EditFile` | Apply targeted edits to a file | `EditFile { path: "src/auth.rs", old: "...", new: "..." }` |
| `Shell` | Run a shell command | `Shell { command: "cargo test" }` |
| `Search` | Grep the codebase | `Search { pattern: "validate_token", path: "src/" }` |
| `LSP` | Query the language server | `LSP { action: "definition", symbol: "AuthError" }` |
| `ListDir` | List directory contents | `ListDir { path: "src/" }` |
| `WebSearch` | Search the web | `WebSearch { query: "JWT RS256 validation" }` |
| `WebFetch` | Fetch a URL | `WebFetch { url: "https://docs.rs/..." }` |

### Step 4: Observe the agent reading files

```
[Tool Call] ReadFile { path: "src/auth.rs" }

[Result]
 1  use jsonwebtoken::{decode, DecodingKey, Validation, Algorithm};
 2  use crate::error::AuthError;
 3
 4  pub fn validate_token(token: &str) -> Result<Claims, AuthError> {
 5      let key = DecodingKey::from_secret(b"secret");
 6      let validation = Validation::new(Algorithm::HS256);
 7      let token_data = decode::<Claims>(token, &key, &validation)?;
 8      Ok(token_data.claims)
 9  }
```

### Step 5: Watch the agent make edits

The agent identifies the issue and applies a fix:

```
[Thinking] I see the problem. The validation is using HS256 but the tokens are signed
with RS256 according to the config. Also, the secret key is hardcoded instead of
being read from the environment.

[Tool Call] EditFile {
    path: "src/auth.rs",
    old: "    let key = DecodingKey::from_secret(b\"secret\");\n    let validation = Validation::new(Algorithm::HS256);",
    new: "    let secret = std::env::var(\"JWT_SECRET\").map_err(|_| AuthError::MissingSecret)?;\n    let key = DecodingKey::from_secret(secret.as_bytes());\n    let validation = Validation::new(Algorithm::RS256);"
}

[Result] File edited successfully. Changes:
  - src/auth.rs:5-6 (2 lines changed)
```

The diff is displayed inline:

```diff
- let key = DecodingKey::from_secret(b"secret");
- let validation = Validation::new(Algorithm::HS256);
+ let secret = std::env::var("JWT_SECRET").map_err(|_| AuthError::MissingSecret)?;
+ let key = DecodingKey::from_secret(secret.as_bytes());
+ let validation = Validation::new(Algorithm::RS256);
```

<!-- Screenshot placeholder: Agent showing diff of edits -->
![Agent edit diff](../assets/screenshots/demo-04-agent-edit.png)

### Step 6: Shell command execution

The agent runs tests to verify the fix:

```
[Tool Call] Shell { command: "cargo test auth::tests" }

[Result]
running 5 tests
test auth::tests::test_valid_token ... ok
test auth::tests::test_expired_token ... ok
test auth::tests::test_invalid_signature ... ok
test auth::tests::test_missing_secret ... ok
test auth::tests::test_malformed_token ... ok

test result: ok. 5 passed; 0 failed; 0 ignored
```

### Step 7: Sandboxing

Shell commands run in a sandboxed environment by default. Configure sandbox rules in `~/.vibecli/config.toml`:

```toml
[sandbox]
enabled = true
allow_read = ["/home/user/project/**"]
allow_write = ["/home/user/project/src/**", "/home/user/project/tests/**"]
blocked_commands = ["rm -rf /", "sudo", "curl | bash"]
network = false  # Disable network access in sandbox
max_runtime_seconds = 30
```

When the agent tries a blocked command:

```
[Tool Call] Shell { command: "sudo rm -rf /tmp/test" }

[Result] BLOCKED: Command "sudo" is in the blocked commands list.
The agent will be informed and asked to use an alternative approach.
```

### Step 8: Agent hooks

Hooks let you run custom scripts before and after agent actions. They receive JSON on stdin and output JSON on stdout.

```toml
# ~/.vibecli/config.toml

[[hooks]]
event = "pre_edit"
command = "python3 ~/.vibecli/hooks/check_edit.py"
# Exit 0 = allow, exit 2 = block

[[hooks]]
event = "post_edit"
command = "cargo fmt -- --check"
# Auto-format after every edit

[[hooks]]
event = "pre_command"
command = "python3 ~/.vibecli/hooks/audit_command.py"
# Log all commands for audit
```

Example hook script (`check_edit.py`):

```python
#!/usr/bin/env python3
import json, sys

edit = json.load(sys.stdin)
path = edit.get("path", "")

# Block edits to production config files
if "prod" in path and "config" in path:
    print(json.dumps({"blocked": True, "reason": "Cannot edit production configs"}))
    sys.exit(2)

print(json.dumps({"blocked": False}))
sys.exit(0)
```

### Step 9: Checkpointing and rollback

The agent automatically creates checkpoints before making changes. You can roll back at any time.

```bash
vibecli repl
> /agent "Refactor the database module to use connection pooling"

# Agent makes several file changes...

> /checkpoint list
Checkpoints for session abc123:
  [1] 2026-03-13T10:00:00Z - Before editing src/db/mod.rs
  [2] 2026-03-13T10:00:15Z - Before editing src/db/pool.rs
  [3] 2026-03-13T10:00:30Z - Before editing src/db/config.rs

> /checkpoint rollback 2
Rolled back to checkpoint 2. Files restored:
  - src/db/config.rs (reverted)
```

In interactive mode, the agent asks for confirmation before each action:

```
[Agent] I want to edit src/db/mod.rs to add connection pooling.
        Proceed? [Y/n/diff]
> diff
[Shows the proposed diff]
> y
[Edit applied]
```

### Step 10: Context management and token budgets

The agent manages context automatically, but you can configure token budgets:

```toml
# ~/.vibecli/config.toml

[agent]
max_tokens = 100000          # Max context window usage
max_iterations = 50           # Max think-act cycles
context_pruning = true        # Auto-prune old context
summary_threshold = 50000     # Summarize context above this
```

Monitor context usage during an agent session:

```bash
> /context
Context usage: 34,521 / 100,000 tokens (34.5%)
  System prompt:     2,100 tokens
  Conversation:     18,421 tokens
  Tool results:     12,800 tokens
  Files in context:  1,200 tokens
```

## VibeUI Agent Panel

In VibeUI, the Agent panel (`Cmd+J` then "Agent" tab) provides a visual representation of the agent loop:

1. Each tool call is shown as a collapsible card
2. File edits show inline diffs in the Monaco editor
3. Shell commands show output in an embedded terminal
4. A progress bar indicates the agent's iteration count
5. Checkpoint controls are available in the toolbar

<!-- Screenshot placeholder: VibeUI agent panel -->
![VibeUI agent panel](../assets/screenshots/demo-04-vibeui-agent.png)

## Demo Recording

```json
{
  "meta": {
    "title": "Agent Loop & Tool Execution",
    "description": "Watch the VibeCody agent autonomously read files, identify a bug, apply a fix, and verify with tests.",
    "duration_seconds": 300,
    "version": "1.0.0"
  },
  "steps": [
    {
      "id": 1,
      "action": "shell",
      "command": "mkdir -p /tmp/vibecody-demo/src && cat > /tmp/vibecody-demo/src/auth.rs << 'RUST'\nuse jsonwebtoken::{decode, DecodingKey, Validation, Algorithm};\n\npub struct Claims {\n    pub sub: String,\n    pub exp: usize,\n}\n\npub fn validate_token(token: &str) -> Result<Claims, String> {\n    let key = DecodingKey::from_secret(b\"secret\");\n    let validation = Validation::new(Algorithm::HS256);\n    let token_data = decode::<Claims>(token, &key, &validation)\n        .map_err(|e| format!(\"Invalid token: {}\", e))?;\n    Ok(token_data.claims)\n}\nRUST",
      "description": "Create sample project with a buggy auth module",
      "delay_ms": 1000
    },
    {
      "id": 2,
      "action": "shell",
      "command": "cd /tmp/vibecody-demo && vibecli agent \"The validate_token function in src/auth.rs has a hardcoded secret and wrong algorithm. Fix it to read JWT_SECRET from the environment and use RS256.\"",
      "description": "Start agent to fix the auth bug",
      "delay_ms": 15000
    },
    {
      "id": 3,
      "action": "shell",
      "command": "cat /tmp/vibecody-demo/src/auth.rs",
      "description": "Verify the agent's changes",
      "delay_ms": 2000
    },
    {
      "id": 4,
      "action": "repl",
      "commands": [
        { "input": "/checkpoint list", "delay_ms": 2000, "description": "List available checkpoints" },
        { "input": "/context", "delay_ms": 1500, "description": "Check context token usage" },
        { "input": "/quit", "delay_ms": 500 }
      ],
      "description": "Inspect checkpoints and context usage"
    },
    {
      "id": 5,
      "action": "shell",
      "command": "cd /tmp/vibecody-demo && vibecli agent --interactive \"Add unit tests for the validate_token function\"",
      "description": "Start interactive agent session to add tests",
      "delay_ms": 20000
    },
    {
      "id": 6,
      "action": "repl",
      "commands": [
        {
          "input": "/agent \"Search the codebase for any other hardcoded secrets\"",
          "delay_ms": 10000,
          "description": "Agent searches for more hardcoded secrets"
        },
        { "input": "/quit", "delay_ms": 500 }
      ],
      "description": "Use agent in REPL to audit for more issues"
    },
    {
      "id": 7,
      "action": "shell",
      "command": "rm -rf /tmp/vibecody-demo",
      "description": "Clean up demo project",
      "delay_ms": 500
    }
  ]
}
```

## Tips for Effective Agent Use

1. **Be specific** -- "Fix the login bug in src/auth.rs" works better than "Fix my code"
2. **Use interactive mode** for sensitive changes -- `vibecli agent --interactive`
3. **Set up hooks** to enforce code style and block dangerous operations
4. **Review checkpoints** before accepting large changes
5. **Monitor token usage** with `/context` to avoid hitting limits
6. **Use sandboxing** to prevent unintended side effects from shell commands

## What's Next

- [Demo 5: Model Arena](../05-model-arena/) -- Compare how different models handle agent tasks
- [Demo 6: Cost Observatory](../06-cost-observatory/) -- Track agent session costs
- [Demo 1: First Run](../01-first-run/) -- Revisit setup if you need to configure additional providers
