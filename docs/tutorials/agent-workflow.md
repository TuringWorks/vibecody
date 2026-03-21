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

```
[agent] Planning task: fix the unwrap panic in src/auth.rs
[agent] Reading file: src/auth.rs
[agent] Found 3 unwrap() calls on lines 34, 67, 91
[agent] Line 34: reqwest::get(url).await.unwrap() — network call
[agent] Line 67: serde_json::from_str(&body).unwrap() — parse
[agent] Line 91: db.query(&sql).unwrap() — database call
[agent] Writing changes to: src/auth.rs
  + Line 34: reqwest::get(url).await? → replaced with ? operator
  + Line 67: serde_json::from_str(&body)? → replaced with ? operator
  + Line 91: db.query(&sql).context("database query failed")? → added context
  + Updated function signature to return Result<AuthResponse>
[agent] Running: cargo check
[agent] Build succeeded.

Accept changes? [y/n/diff]:
```

Type `diff` to see the exact changes:

```diff
-    let resp = reqwest::get(url).await.unwrap();
+    let resp = reqwest::get(url).await.context("failed to reach auth server")?;

-    let data: AuthResponse = serde_json::from_str(&body).unwrap();
+    let data: AuthResponse = serde_json::from_str(&body).context("invalid auth response JSON")?;

-    let rows = db.query(&sql).unwrap();
+    let rows = db.query(&sql).context("database query failed")?;
```

Type `y` to accept.

---

## Example 2: Add a Feature

You want to add rate limiting to your API server.

```bash
vibecli --exec "add rate limiting to the API server in src/server.rs -- use a token bucket algorithm, limit to 100 requests per minute per IP"
```

### What the Agent Does

```
[agent] Planning task: add rate limiting to src/server.rs
[agent] Reading file: src/server.rs
[agent] Reading file: Cargo.toml (checking existing dependencies)
[agent] Plan:
  1. Add a TokenBucket struct with per-IP tracking
  2. Add middleware that checks the bucket before handling requests
  3. Return 429 Too Many Requests when limit exceeded
  4. Add governor crate to Cargo.toml for production-ready implementation
[agent] Writing changes to: Cargo.toml
  + Added: governor = "0.6"
[agent] Writing changes to: src/rate_limit.rs (new file)
  + TokenBucket struct with HashMap<IpAddr, Bucket>
  + check_rate_limit() middleware function
[agent] Writing changes to: src/server.rs
  + Added rate_limit middleware to router
[agent] Running: cargo check
[agent] Build succeeded.
[agent] Running: cargo test
[agent] All 14 tests passed.

Accept changes? [y/n/diff]:
```

---

## Example 3: Refactor Code

Refactoring is one of the agent's strongest use cases because it can read the entire context, plan coordinated changes across files, and verify the result compiles.

```bash
vibecli --exec "refactor the database module to use a connection pool -- extract the connection logic from src/db.rs into a new src/pool.rs, update all callers"
```

### What the Agent Does

```
[agent] Planning task: refactor database module
[agent] Reading: src/db.rs, src/main.rs, src/handlers/*.rs
[agent] Found 7 files that directly call db::connect()
[agent] Plan:
  1. Create src/pool.rs with a ConnectionPool struct
  2. Move connection logic from db.rs to pool.rs
  3. Update db.rs to use the pool
  4. Update 7 caller files to use shared pool reference
  5. Verify build and tests
[agent] Writing 9 files...
[agent] Running: cargo check
[agent] Build succeeded.
[agent] Running: cargo test
[agent] 23/23 tests passed.

Accept changes? [y/n/diff]:
```

---

## Understanding Tool Calls

The agent works by making "tool calls" -- structured actions that read or modify your project. The main tools are:

| Tool | What It Does |
|------|-------------|
| **read_file** | Read a file's contents |
| **write_file** | Write or overwrite a file |
| **edit_file** | Apply a targeted edit to part of a file |
| **bash** | Run a shell command (build, test, git, etc.) |
| **search** | Search across the codebase by pattern |
| **list_files** | List files in a directory |

In suggest mode, you see each tool call before it executes and can approve or deny it. In auto-edit mode, file operations run automatically but shell commands still require approval.

---

## Reviewing Changes Before Accepting

When the agent finishes, you always have the option to review before accepting. The prompt supports:

| Response | Effect |
|----------|--------|
| `y` or `yes` | Accept all changes |
| `n` or `no` | Reject all changes (files revert to original) |
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
