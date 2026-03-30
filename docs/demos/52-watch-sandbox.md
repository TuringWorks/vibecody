---
layout: page
title: "Demo 52: Watch Mode & Sandbox Isolation"
permalink: /demos/52-watch-sandbox/
---


## Overview

VibeCody provides two powerful operational modes: Watch Mode re-runs an agent task automatically whenever files change, and Sandbox Isolation runs agent tasks inside an OS-level container with restricted filesystem, network, and process access. These can be combined to create safe, continuous development loops where the AI reacts to your edits in a locked-down environment.

**Time to complete:** ~12 minutes

## Prerequisites

- VibeCLI 0.5.1 or later installed and on your PATH
- At least one AI provider configured
- For sandbox mode: Docker or Podman installed (or OpenSandbox for macOS native)
- A project with source files and tests

## Step-by-Step Walkthrough

### Step 1: Basic watch mode

Watch mode monitors files and re-runs an agent task when changes are detected:

```bash
vibecli --watch --agent "Run the test suite and report failures"
```

```
vibecli 0.5.1 | Watch mode enabled
Watching: . (all files)
Agent task: "Run the test suite and report failures"

[Watch] Initial run...

[Agent] Running test suite...
[Tool: execute_command] cargo test
  Running 2,473 tests
  test result: ok. 2,473 passed; 0 failed

[Agent] All 2,473 tests pass. Watching for changes...

[Watch] Waiting for file changes... (Ctrl+C to stop)
```

Now edit a file in another terminal. The agent re-runs automatically:

```
[Watch] Changed: src/inventory.rs (modified)
[Watch] Re-running agent task...

[Agent] Detected change in src/inventory.rs. Running tests...
[Tool: execute_command] cargo test --lib inventory

  test inventory::tests::test_create ... ok
  test inventory::tests::test_list ... FAILED
  test inventory::tests::test_pagination ... ok

  failures:
    test_list expected 5 items but got 4

  test result: FAILED. 2 passed; 1 failed

[Agent] 1 test failure in inventory::tests::test_list.
  The test expects 5 items but gets 4. This is likely caused by
  the filter change on line 42 of src/inventory.rs where you added
  `status != "archived"`, which excludes one test fixture item.

  Fix: update the test to expect 4 items, or add a non-archived
  fixture item to maintain the count of 5.

[Watch] Waiting for file changes...
```

### Step 2: Watch with glob filters

Limit which files trigger re-runs using `--watch-glob`:

```bash
vibecli --watch --agent "Run Rust tests and clippy" --watch-glob "**/*.rs"
```

```
vibecli 0.5.1 | Watch mode enabled
Watching: **/*.rs (Rust files only)
Agent task: "Run Rust tests and clippy"

[Watch] Initial run...
[Agent] Running cargo test and cargo clippy...
[Tool: execute_command] cargo test
  test result: ok. 2,473 passed; 0 failed
[Tool: execute_command] cargo clippy -- -D warnings
  Finished: 0 warnings, 0 errors

[Watch] Waiting for file changes... (ignoring non-Rust files)
```

Changes to `.md`, `.toml`, `.tsx`, or other non-Rust files are silently ignored.

### Step 3: Sandbox mode basics

Run an agent task inside an isolated container:

```bash
vibecli --sandbox --agent "Analyze the project structure and suggest improvements"
```

```
vibecli 0.5.1 | Sandbox: enabled (Docker)
Container: vibecli-sandbox-f8a2c1d3
Filesystem: project mounted read-only at /workspace
Network: enabled (outbound only)
Timeout: 300s

[Agent] Analyzing project structure from sandbox...
[Tool: execute_command] find /workspace -type f -name "*.rs" | head -30
[Tool: read_file] /workspace/Cargo.toml
[Tool: read_file] /workspace/vibecli/vibecli-cli/src/main.rs

[Agent] Project structure analysis:
  - Workspace with 6 crates (well-organized monorepo)
  - 178+ Rust modules in vibecli-cli
  - Good separation: vibe-core, vibe-ai, vibe-lsp, vibe-extensions
  - Suggestion: Consider splitting vibecli-cli/src/ into feature-gated
    sub-crates to reduce compile times

[Sandbox] Container removed. Duration: 12s.
```

### Step 4: Network-disabled sandbox

For maximum isolation, disable network access:

```bash
vibecli --sandbox --no-network --agent "Review src/auth.rs for security issues"
```

```
vibecli 0.5.1 | Sandbox: enabled (Docker, network disabled)
Container: vibecli-sandbox-b4e7d290
Filesystem: project mounted read-only at /workspace
Network: disabled (no outbound connections)
Timeout: 300s

[Agent] Reviewing src/auth.rs in isolated sandbox...
[Tool: read_file] /workspace/vibecli/vibecli-cli/src/auth.rs

[Agent] Security review of auth.rs:

  1. [OK] Token hashing uses SHA-256 with salt
  2. [OK] Rate limiting on login attempts (5/minute)
  3. [WARN] Line 84: Token expiry is set to 30 days — consider 24h
     with refresh tokens for sensitive operations
  4. [WARN] Line 112: Error message distinguishes "user not found"
     from "wrong password" — this leaks username existence
  5. [OK] CORS headers properly restricted to allowed origins

  2 warnings, 0 critical issues found.

[Sandbox] Container removed. Duration: 8s.
```

The AI could not phone home, exfiltrate code, or reach any external service during this run.

### Step 5: Combine watch and sandbox

The most powerful configuration -- continuous testing in an isolated environment:

```bash
vibecli --watch --sandbox --agent "Run tests" --watch-glob "**/*.rs"
```

```
vibecli 0.5.1 | Watch mode + Sandbox enabled
Watching: **/*.rs
Sandbox: Docker (network enabled, filesystem read-only)

[Watch] Initial run (spinning up sandbox)...
[Sandbox] Container: vibecli-sandbox-c9f1a3b5
[Agent] Running cargo test in sandbox...
[Tool: execute_command] cargo test
  test result: ok. 2,473 passed; 0 failed
[Sandbox] Container paused (kept warm for fast re-runs).

[Watch] Waiting for file changes...

  --- (you edit src/models.rs) ---

[Watch] Changed: src/models.rs (modified)
[Sandbox] Container resumed.
[Agent] Running affected tests...
[Tool: execute_command] cargo test --lib models
  test result: ok. 18 passed; 0 failed
[Sandbox] Container paused.

[Watch] Waiting for file changes...
```

The sandbox container is kept warm between runs for faster iteration. It is fully destroyed when you press Ctrl+C.

### Step 6: Sandbox configuration

Fine-tune sandbox behavior in `~/.vibecli/config.toml`:

```toml
[sandbox]
runtime = "docker"          # docker, podman, or opensandbox
timeout_seconds = 300
network = true
mount_mode = "read-only"    # read-only or read-write
memory_limit = "2g"
cpu_limit = 2
allowed_paths = ["/workspace", "/tmp"]
blocked_commands = ["rm -rf /", "curl", "wget"]
```

Or per-profile in `~/.vibecli/profiles/secure.toml`:

```toml
[sandbox]
runtime = "docker"
network = false
mount_mode = "read-only"
memory_limit = "1g"
```

### Step 7: Sandbox with write access

When you need the agent to actually modify files:

```bash
vibecli --sandbox --agent "Fix the failing test in src/models.rs"
```

```
vibecli 0.5.1 | Sandbox: enabled (Docker)
Filesystem: project mounted read-write at /workspace
Network: enabled

[Agent] Investigating test failure...
[Tool: execute_command] cargo test --lib models 2>&1
  test models::tests::test_serialize ... FAILED
  assertion: left == right
    left:  {"name":"item","qty":0}
    right: {"name":"item","quantity":0}

[Tool: read_file] /workspace/src/models.rs
[Agent] Found it -- the Serialize rename attribute uses "qty" but the
test expects "quantity".

[Tool: write_file] /workspace/src/models.rs (line 15: rename = "quantity")
[Tool: execute_command] cargo test --lib models
  test result: ok. 18 passed; 0 failed

[Agent] Fixed. The serde rename on line 15 was changed from "qty" to
"quantity" to match the expected JSON output.

[Sandbox] Container removed. Changes written to project directory.
```

## How Sandbox Isolation Works

VibeCody uses the `ContainerRuntime` trait to abstract across 3 runtimes:

| Runtime        | Platform       | Isolation Level | Setup Required       |
|----------------|----------------|-----------------|----------------------|
| **Docker**     | All            | Full container  | Docker installed     |
| **Podman**     | Linux          | Rootless        | Podman installed     |
| **OpenSandbox**| macOS          | App Sandbox     | Xcode tools          |

The runtime provides 16 async methods including container creation, filesystem mounting, network policy, process limits, and cleanup. All agent tool calls are proxied through the container.

## Demo Recording

```json
{
  "meta": {
    "title": "Watch Mode & Sandbox Isolation",
    "description": "Automatic file watching with agent re-runs and OS-level sandbox isolation for safe AI execution.",
    "duration_seconds": 200,
    "version": "1.0.0"
  },
  "steps": [
    {
      "id": 1,
      "action": "shell",
      "command": "vibecli --watch --agent \"Run tests\" --watch-glob \"**/*.rs\" &",
      "description": "Start watch mode for Rust files",
      "delay_ms": 5000
    },
    {
      "id": 2,
      "action": "shell",
      "command": "echo '// trigger' >> src/lib.rs",
      "description": "Trigger a file change",
      "delay_ms": 8000
    },
    {
      "id": 3,
      "action": "shell",
      "command": "kill %1",
      "description": "Stop watch mode",
      "delay_ms": 1000
    },
    {
      "id": 4,
      "action": "shell",
      "command": "vibecli --sandbox --no-network --agent \"Review src/auth.rs for security issues\"",
      "description": "Run a sandboxed security review with no network",
      "delay_ms": 15000
    },
    {
      "id": 5,
      "action": "shell",
      "command": "vibecli --sandbox --agent \"Fix the failing test in src/models.rs\"",
      "description": "Run a sandboxed agent that writes fixes",
      "delay_ms": 12000
    }
  ]
}
```

## What's Next

- [Demo 51: Profiles & Sessions](../51-profiles-sessions/) -- Profile-based configuration and session resumption
- [Demo 53: Workflow Orchestration](../53-workflow-orchestration/) -- Task tracking with lessons and complexity estimation
- [Demo 48: OpenMemory](../48-open-memory/) -- Persistent cognitive memory across sessions
