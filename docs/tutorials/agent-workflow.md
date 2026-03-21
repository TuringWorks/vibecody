---
layout: page
title: "Tutorial: Agent Workflow"
permalink: /tutorials/agent-workflow/
---

# Using the Agent to Fix Bugs and Add Features

The VibeCody agent is an autonomous loop that reads your codebase, plans changes, writes code, and runs commands -- all guided by a natural language task description. This tutorial walks you through three real-world scenarios.

**Prerequisites:**
- VibeCody installed with a working provider (see [First Provider Tutorial](./first-provider/))
- A Git repository to work in

---

## What Is the Agent Loop?

When you give VibeCody an agent task, it enters a loop:

```
1. Read the task description
2. Plan what to do (which files to read, what to change)
3. Use tools: read files, write files, run commands, search code
4. Check its work (run tests, verify builds)
5. Present results for your approval
```

The agent has access to your filesystem and shell within the project directory. It can read any file, write changes, and execute commands like `cargo build` or `npm test`.

---

## Approval Policies

Before diving in, understand the three approval modes:

| Policy | Flag | File Edits | Shell Commands | Best For |
|--------|------|-----------|----------------|----------|
| **Suggest** | *(default)* | Ask first | Ask first | Interactive development |
| **Auto-edit** | `--auto-edit` | Auto-apply | Ask first | Trusted file changes, cautious commands |
| **Full-auto** | `--full-auto` | Auto-apply | Auto-execute | CI/CD, sandboxed environments |

Start with the default (suggest) until you are comfortable with how the agent works. You can always combine `--full-auto` with `--sandbox` for safe autonomous execution.

---

## Example 1: Fix a Bug

You have a Rust project where `auth.rs` panics on invalid input because of an `unwrap()` on a network call.

### From the Command Line

```bash
vibecli --exec "fix the unwrap panic in src/auth.rs -- use proper error handling with anyhow"
```

### From the REPL

```bash
vibecli
```

```
vibecli> /agent fix the unwrap panic in src/auth.rs -- use proper error handling with anyhow
```

### What the Agent Does

The agent streams its reasoning and shows each tool step with a checkmark:

```
 Agent   fix the unwrap panic in src/auth.rs -- use proper error handling with anyhow
  Policy: suggest (ask before every action)  |  Press Ctrl+C to stop

I'll read the file first to understand the current error handling patterns.

 ✓ Reading src/auth.rs

I can see 3 unwrap() calls that need to be replaced with proper error handling.

  apply_patch  Patching src/auth.rs (3 hunks)
   Approve? (y/n/a=approve-all): y

 ✓ Patching src/auth.rs (3 hunks)

Now let me verify the build still passes.

  bash  Running: cargo check
   Approve? (y/n/a=approve-all): y

 ✓ Running: cargo check

Agent complete: Replaced 3 unwrap() calls with ? operator and anyhow context.
   Files modified: src/auth.rs
   Commands run: 1
   Steps: 3/3 succeeded
   Trace saved: ~/.vibecli/traces/1711234567.jsonl
   Resume with: vibecli --resume 1711234567
```

The diff shows exactly what changed:

```diff
-    let resp = reqwest::get(url).await.unwrap();
+    let resp = reqwest::get(url).await.context("failed to reach auth server")?;

-    let data: AuthResponse = serde_json::from_str(&body).unwrap();
+    let data: AuthResponse = serde_json::from_str(&body).context("invalid auth response JSON")?;

-    let rows = db.query(&sql).unwrap();
+    let rows = db.query(&sql).context("database query failed")?;
```

In `suggest` mode, the agent asks before each destructive action (`y` to approve, `n` to reject, `a` to approve all remaining steps).

---

## Example 2: Add a Feature

You want to add rate limiting to your API server.

```bash
vibecli --exec "add rate limiting to the API server in src/server.rs -- use a token bucket algorithm, limit to 100 requests per minute per IP"
```

### What the Agent Does

```
 Agent   add rate limiting to the API server in src/server.rs
  Policy: auto-edit (auto-apply files, ask for commands)  |  Press Ctrl+C to stop

 ✓ Reading src/server.rs
 ✓ Reading Cargo.toml
 ✓ Searching: "middleware"
 ✓ Patching Cargo.toml (1 hunks)
 ✓ Writing src/rate_limit.rs (45 lines)
 ✓ Patching src/server.rs (2 hunks)

  bash  Running: cargo check
   Approve? (y/n/a=approve-all): y

 ✓ Running: cargo check

  bash  Running: cargo test
   Approve? (y/n/a=approve-all): a

 ✓ Running: cargo test

Agent complete: Added token bucket rate limiter with per-IP tracking.
   Files modified: Cargo.toml, src/rate_limit.rs, src/server.rs
   Commands run: 2
   Steps: 8/8 succeeded
```

---

## Example 3: Refactor Code

Refactoring is one of the agent's strongest use cases because it can read the entire context, plan coordinated changes across files, and verify the result compiles.

```bash
vibecli --exec "refactor the database module to use a connection pool -- extract the connection logic from src/db.rs into a new src/pool.rs, update all callers"
```

### What the Agent Does

In `--exec` (full-auto) mode, the agent runs without prompting:

```
 Agent   refactor the database module to use a connection pool
  Policy: full-auto (execute everything)  |  Press Ctrl+C to stop

 ✓ Reading src/db.rs
 ✓ Searching: "db::connect"
 ✓ Reading src/main.rs
 ✓ Reading src/handlers/auth.rs
 ✓ Reading src/handlers/users.rs
 ✓ Writing src/pool.rs (62 lines)
 ✓ Patching src/db.rs (4 hunks)
 ✓ Patching src/main.rs (2 hunks)
 ✓ Patching src/handlers/auth.rs (1 hunks)
 ✓ Patching src/handlers/users.rs (1 hunks)
 ✓ Running: cargo check
 ✓ Running: cargo test

Agent complete: Extracted connection pool into src/pool.rs, updated 4 callers.
   Files modified: src/pool.rs, src/db.rs, src/main.rs, src/handlers/auth.rs, src/handlers/users.rs
   Commands run: 2
   Steps: 12/12 succeeded
```

---

## Understanding Tool Calls

The agent works by making "tool calls" -- structured actions that read or modify your project:

| Tool | What It Does | Needs Approval |
|------|-------------|----------------|
| **read_file** | Read a file's contents | No (read-only) |
| **write_file** | Write or overwrite a file | Yes (in suggest mode) |
| **apply_patch** | Apply a unified diff patch to a file | Yes (in suggest mode) |
| **bash** | Run a shell command (build, test, git, etc.) | Yes (in suggest/auto-edit modes) |
| **search_files** | Search across the codebase by pattern | No (read-only) |
| **list_directory** | List files in a directory | No (read-only) |
| **think** | Internal reasoning step (free, no side effects) | No |
| **web_search** | Search the web via DuckDuckGo | No |
| **fetch_url** | Fetch a web page (SSRF-protected) | No |
| **spawn_agent** | Delegate a sub-task to a child agent | Yes |
| **task_complete** | Signal that the task is done | No |

In `suggest` mode, you see each tool call before it executes and can approve (`y`), deny (`n`), or approve all remaining (`a`). In `auto-edit` mode, file operations run automatically but shell commands still require approval.

---

## Reviewing Changes

After each step, the agent shows what it did with a checkmark (success) or cross (failure). At completion, you get a summary:

```
Agent complete: <summary>
   Files modified: file1.rs, file2.rs
   Commands run: 2
   Steps: 5/5 succeeded
   Trace saved: ~/.vibecli/traces/<id>.jsonl
   Resume with: vibecli --resume <id>
```

You can resume any previous session with `/resume <id>` to continue where you left off.
| `diff` | Show the full diff of all changes |

If you are using Git (recommended), you can also accept and then review with `git diff` afterward. If anything looks wrong, `git checkout .` reverts everything.

---

## Session Resume

Every agent run creates a session trace. If a task is interrupted or you want to continue where you left off:

### List Recent Sessions

```
vibecli> /sessions
```

```
  ID        | Started            | Task                          | Steps
  a1b2c3d4  | 2026-03-20 14:32   | fix unwrap panic in auth.rs   | 8
  e5f6g7h8  | 2026-03-20 13:10   | add rate limiting to API      | 12
```

### Resume a Session

```
vibecli> /resume a1b2c3d4
```

Or from the command line:

```bash
vibecli --resume a1b2c3d4
```

The agent picks up with full context of what it already did, what files it read, and what changes it made.

---

## Tips for Better Agent Results

1. **Be specific.** "Fix the bug" is vague. "Fix the unwrap panic on line 34 of auth.rs" is actionable.

2. **Mention the file.** The agent can search, but telling it where to look saves time: "in src/server.rs".

3. **State the approach.** If you have a preference, say it: "use the ? operator" or "add a retry with exponential backoff".

4. **Start with suggest mode.** Watch what the agent does for your first few tasks. You will learn its patterns and gain confidence to use auto-edit later.

5. **Use Git.** Always work in a Git repo so you can easily review and revert changes.

6. **Scope the task.** One focused task per agent run works better than "refactor everything". Break large tasks into steps.

---

## Next Steps

- [AI-Powered Code Review](./code-review/) -- review code before committing
- [Setting Up Your First Provider](./first-provider/) -- try a different model
- [Tutorials Index](./) -- browse all tutorials
