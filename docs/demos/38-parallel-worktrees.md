---
layout: page
title: "Demo 38: Parallel Worktree Agents"
permalink: /demos/38-parallel-worktrees/
---


## Overview

VibeCody can spawn multiple AI agents that work simultaneously in isolated git worktrees. Each agent gets its own working directory backed by a real git worktree, so file changes from one agent never interfere with another. When all agents finish, you review the results and merge the branches you want. This provides lightweight parallelism without the overhead of Docker containers or virtual machines.

**Time to complete:** ~10 minutes

## Prerequisites

- VibeCLI v0.5.1 installed and on your PATH
- A git repository (worktrees require git)
- At least one AI provider configured
- For VibeUI: the desktop app running with the **WorktreePool** panel visible

## Why Worktrees Instead of Containers?

| Feature              | Worktrees                          | Docker Containers             |
|----------------------|------------------------------------|-------------------------------|
| **Setup time**       | < 1 second (git worktree add)      | 5-30 seconds (image pull)     |
| **Disk usage**       | Shared .git objects, minimal       | Full filesystem per container |
| **File isolation**   | Yes (separate working directory)   | Yes (separate filesystem)     |
| **Git integration**  | Native branches, easy merge        | Requires volume mounts        |
| **Dependencies**     | git only                           | Docker daemon required        |
| **Best for**         | Code changes, refactoring, tests   | Untrusted code, system deps   |

## Step-by-Step Walkthrough

### 1. Spawn Parallel Agents

Launch multiple agents, each working on a different task in its own worktree.

**REPL:**

```bash
vibecli
> /worktree spawn "Add input validation to src/api/handlers.rs" "Write unit tests for src/auth/jwt.rs" "Refactor src/db/queries.rs to use prepared statements"
```

Example output:

```
Spawning 3 worktree agents...

  Agent 1: worktree-agent-1a2b
    Branch:   wt/input-validation-1a2b
    Worktree: .git/worktrees/agent-1a2b/
    Task:     Add input validation to src/api/handlers.rs
    Status:   running

  Agent 2: worktree-agent-3c4d
    Branch:   wt/unit-tests-3c4d
    Worktree: .git/worktrees/agent-3c4d/
    Task:     Write unit tests for src/auth/jwt.rs
    Status:   running

  Agent 3: worktree-agent-5e6f
    Branch:   wt/refactor-queries-5e6f
    Worktree: .git/worktrees/agent-5e6f/
    Task:     Refactor src/db/queries.rs to use prepared statements
    Status:   running

All 3 agents running in parallel. Use /worktree list to check progress.
```

Each agent operates on its own branch and worktree directory. Your main working directory remains untouched.

### 2. Monitor Progress

Check the status of all running worktree agents.

**REPL:**

```bash
vibecli
> /worktree list
```

Example output:

```
Worktree Agents:

  ID          Branch                        Status     Duration  Files Changed
  1a2b        wt/input-validation-1a2b      running    0:42      3
  3c4d        wt/unit-tests-3c4d            completed  1:15      2
  5e6f        wt/refactor-queries-5e6f      running    0:42      1

Summary: 2 running, 1 completed, 0 failed
```

### 3. View Agent Output

Inspect what a specific agent has done so far.

**REPL:**

```bash
vibecli
> /worktree show 3c4d
```

Example output:

```
Agent: worktree-agent-3c4d
Branch: wt/unit-tests-3c4d
Status: completed (1m 15s)
Task: Write unit tests for src/auth/jwt.rs

Files changed:
  A  tests/auth/jwt_tests.rs       (+142 lines)
  M  src/auth/jwt.rs               (+3 lines, doc comments)

Commits:
  a7f2e01  Add comprehensive JWT unit tests
  b8c3d12  Add doc comments to public JWT functions

Agent summary:
  Created 8 test cases covering token creation, validation, expiry,
  refresh, invalid signatures, malformed tokens, missing claims, and
  role-based access. All tests pass (cargo test --lib auth::jwt).
```

### 4. Review Diffs Before Merging

Compare the worktree branch against your current branch.

**REPL:**

```bash
vibecli
> /worktree diff 3c4d
```

Example output:

```
Diff: wt/unit-tests-3c4d vs main

--- /dev/null
+++ b/tests/auth/jwt_tests.rs
@@ -0,0 +1,142 @@
+use crate::auth::jwt::{create_token, validate_token, refresh_token};
+use crate::auth::claims::Claims;
+
+#[test]
+fn test_create_token_returns_valid_jwt() {
+    let claims = Claims::new("user-123", vec!["admin"]);
+    let token = create_token(&claims, "secret").unwrap();
+    assert!(token.starts_with("eyJ"));
+    assert_eq!(token.matches('.').count(), 2);
+}
+
+#[test]
+fn test_validate_rejects_expired_token() {
+    let claims = Claims::expired("user-123");
+    let token = create_token(&claims, "secret").unwrap();
+    let result = validate_token(&token, "secret");
+    assert!(result.is_err());
+}
+...
(+140 more lines)
```

### 5. Merge Results

Merge completed worktree branches into your current branch.

**REPL:**

```bash
vibecli
> /worktree merge 3c4d
```

Example output:

```
Merging wt/unit-tests-3c4d into main...

  Merge strategy: fast-forward
  Commits merged: 2
  Files added:    1 (tests/auth/jwt_tests.rs)
  Files modified: 1 (src/auth/jwt.rs)
  Conflicts:      0

Merge complete. Worktree cleaned up.
Branch wt/unit-tests-3c4d deleted.
```

To merge all completed agents at once:

```bash
vibecli
> /worktree merge --all
```

Example output:

```
Merging 3 completed branches into main...

  wt/input-validation-1a2b   merged (fast-forward, 0 conflicts)
  wt/unit-tests-3c4d         merged (fast-forward, 0 conflicts)
  wt/refactor-queries-5e6f   merged (3-way merge, 0 conflicts)

All 3 branches merged. Worktrees cleaned up.
```

### 6. Handle Merge Conflicts

If a merge has conflicts, VibeCLI reports them and optionally uses the AI to resolve.

```bash
vibecli
> /worktree merge 5e6f
```

Example output with conflicts:

```
Merging wt/refactor-queries-5e6f into main...

  Conflicts detected in 1 file:
    src/db/queries.rs (lines 42-58)

Options:
  1. /worktree resolve 5e6f --auto    AI-assisted conflict resolution
  2. /worktree resolve 5e6f --manual  Open in editor
  3. /worktree merge 5e6f --abort     Cancel the merge

> /worktree resolve 5e6f --auto

AI resolved 1 conflict in src/db/queries.rs:
  Kept prepared statement refactor, preserved new validation logic
  from the input-validation branch.

Merge complete. Worktree cleaned up.
```

### 7. Cancel or Clean Up

Abort a running agent or remove stale worktrees.

**REPL:**

```bash
vibecli
> /worktree cancel 1a2b
```

```
Agent worktree-agent-1a2b cancelled.
Worktree .git/worktrees/agent-1a2b/ removed.
Branch wt/input-validation-1a2b deleted.
```

Clean up all finished worktrees:

```bash
vibecli
> /worktree clean
```

```
Cleaned 2 worktrees (0 running agents remain).
```

### 8. VibeUI WorktreePool Panel

Open the **WorktreePool** panel in VibeUI to see:

- **Spawn** tab: enter tasks and launch parallel agents visually
- **Monitor** tab: live progress bars, file change counts, agent logs
- **Diff** tab: side-by-side diff viewer for each worktree branch
- **Merge** tab: one-click merge with conflict resolution preview

## Configuration

Add worktree settings to `~/.vibecli/config.toml`:

```toml
[worktree]
max_parallel = 5
auto_cleanup = true
branch_prefix = "wt/"
default_provider = "claude"
```

## Demo Recording JSON

```json
{
  "meta": {
    "title": "Parallel Worktree Agents",
    "description": "Spawn AI agents in isolated git worktrees for parallel task execution.",
    "duration_seconds": 200,
    "version": "0.5.1"
  },
  "steps": [
    {
      "id": 1,
      "action": "repl",
      "commands": [
        { "input": "/worktree spawn \"Add input validation\" \"Write unit tests\" \"Refactor queries\"", "delay_ms": 5000 }
      ],
      "description": "Spawn 3 parallel worktree agents"
    },
    {
      "id": 2,
      "action": "repl",
      "commands": [
        { "input": "/worktree list", "delay_ms": 3000 }
      ],
      "description": "Monitor running agents"
    },
    {
      "id": 3,
      "action": "repl",
      "commands": [
        { "input": "/worktree show 3c4d", "delay_ms": 3000 }
      ],
      "description": "View completed agent details"
    },
    {
      "id": 4,
      "action": "repl",
      "commands": [
        { "input": "/worktree diff 3c4d", "delay_ms": 3000 }
      ],
      "description": "Review diff before merging"
    },
    {
      "id": 5,
      "action": "repl",
      "commands": [
        { "input": "/worktree merge --all", "delay_ms": 5000 }
      ],
      "description": "Merge all completed branches"
    },
    {
      "id": 6,
      "action": "vibeui_interaction",
      "panel": "WorktreePool",
      "tab": "Monitor",
      "description": "View live agent progress in VibeUI"
    },
    {
      "id": 7,
      "action": "vibeui_interaction",
      "panel": "WorktreePool",
      "tab": "Diff",
      "description": "Side-by-side diff viewer for worktree branches"
    }
  ]
}
```

## What's Next

- [Demo 37: A2A Protocol](../37-a2a-protocol/) -- Delegate tasks to remote agents
- [Demo 20: Agent Teams](../20-agent-teams/) -- Multi-agent collaboration with defined roles
- [Demo 4: Agent Loop](../04-agent-loop/) -- Single-agent autonomous coding
