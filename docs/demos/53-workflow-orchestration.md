---
layout: page
title: "Demo 53: Workflow Orchestration"
permalink: /demos/53-workflow-orchestration/
---


## Overview

VibeCody's Workflow Orchestration module brings structured project management into the AI assistant. It maintains a lessons store (capturing what worked and what failed), a todo tracker (with complexity estimation), and a verification system that checks whether tasks are truly complete. These persist in Markdown files (`tasks/lessons.md` and `tasks/todo.md`) and are automatically injected into the agent's context so the AI learns from your project history.

**Time to complete:** ~10 minutes

## Prerequisites

- VibeCLI 0.5.1 or later installed and on your PATH
- At least one AI provider configured
- A project directory (orchestration files are created in `./tasks/`)
- For VibeUI: the desktop app running with the **Orchestration** panel visible

## Step-by-Step Walkthrough

### Step 1: Check orchestration status

Launch the REPL and view the current state:

```bash
vibecli
```

```
vibecli 0.5.1 | Provider: claude | Model: claude-sonnet-4-6
Type /help for commands, /quit to exit

> /orchestrate status
```

```
Workflow Orchestration Status:

  Lessons:  0 entries (tasks/lessons.md not found -- will create on first use)
  Todos:    0 entries (tasks/todo.md not found -- will create on first use)
  Context:  Orchestration context will be injected into agent prompts

  Run /orchestrate todo to add tasks, /orchestrate lessons to view learnings.
```

### Step 2: Add todo items with complexity estimation

```
> /orchestrate todo add "Implement pagination for the inventory API"
```

```
Todo added:
  ID:         todo_1
  Task:       Implement pagination for the inventory API
  Complexity: Medium (estimated)
  Status:     Pending
  Created:    2026-03-29

  Complexity breakdown:
    Files to change:  ~3 (handler, model, tests)
    Estimated effort: 30-60 minutes
    Risk:             Low (additive change, no breaking modifications)

  Written to: tasks/todo.md
```

```
> /orchestrate todo add "Migrate database from SQLite to PostgreSQL"
```

```
Todo added:
  ID:         todo_2
  Task:       Migrate database from SQLite to PostgreSQL
  Complexity: High (estimated)
  Status:     Pending
  Created:    2026-03-29

  Complexity breakdown:
    Files to change:  ~8 (connection, queries, migrations, config, tests)
    Estimated effort: 2-4 hours
    Risk:             High (data migration required, connection pooling changes)
    Dependencies:     PostgreSQL server, connection string config
```

```
> /orchestrate todo add "Update README with API examples"
```

```
Todo added:
  ID:         todo_3
  Task:       Update README with API examples
  Complexity: Low (estimated)
  Status:     Pending
  Created:    2026-03-29

  Complexity breakdown:
    Files to change:  ~1 (README.md)
    Estimated effort: 10-20 minutes
    Risk:             None
```

### Step 3: View the todo list

```
> /orchestrate todo
```

```
Todo List (tasks/todo.md):

  ID       Status    Complexity  Task
  ─────────────────────────────────────────────────────────────────
  todo_1   Pending   Medium      Implement pagination for the inventory API
  todo_2   Pending   High        Migrate database from SQLite to PostgreSQL
  todo_3   Pending   Low         Update README with API examples

  3 tasks: 3 pending, 0 in progress, 0 done
  Estimated total effort: 3-5 hours
```

### Step 4: Work on a task and record a lesson

Have the agent work on the first task:

```
> Implement pagination for the inventory API (todo_1)
```

```
[Agent] Working on todo_1: Implement pagination for the inventory API...
[Tool: read_file] src/inventory.rs
[Tool: read_file] src/models.rs
[Tool: write_file] src/models.rs — adding PaginationParams and CursorPage
[Tool: write_file] src/inventory.rs — updating list handler with cursor logic
[Tool: write_file] tests/inventory_test.rs — adding pagination tests
[Tool: execute_command] cargo test --lib inventory
  test result: ok. 21 passed; 0 failed

[Agent] Pagination implemented. Marking todo_1 as done.

Todo todo_1 marked as Done.

Lesson recorded automatically:
  "Cursor-based pagination is cleaner than offset pagination for the
   inventory API because items can be inserted/deleted between requests.
   The cursor uses the primary key (id) which is already indexed."
```

### Step 5: View lessons

```
> /orchestrate lessons
```

```
Lessons Store (tasks/lessons.md):

  Lesson 1 [2026-03-29] (from todo_1):
    "Cursor-based pagination is cleaner than offset pagination for the
     inventory API because items can be inserted/deleted between requests.
     The cursor uses the primary key (id) which is already indexed."
    Tags: pagination, api, database
    Outcome: Success

No failure lessons yet. Failures are tracked to prevent repeating mistakes.
```

### Step 6: Add a manual lesson

```
> /orchestrate lesson add "SQLite does not support concurrent writes well -- use WAL mode or switch to PostgreSQL for multi-writer workloads"
```

```
Lesson added:
  Lesson 2 [2026-03-29]:
    "SQLite does not support concurrent writes well -- use WAL mode or
     switch to PostgreSQL for multi-writer workloads"
    Tags: sqlite, postgresql, concurrency
    Outcome: Learning

  Written to: tasks/lessons.md
```

### Step 7: Verify a completed task

```
> /orchestrate verify todo_1
```

```
Verifying todo_1: "Implement pagination for the inventory API"

  Checks:
    [PASS] Files modified: src/models.rs, src/inventory.rs, tests/inventory_test.rs
    [PASS] Tests exist for pagination: 3 test functions found
    [PASS] Tests pass: cargo test --lib inventory → 21 passed, 0 failed
    [PASS] No TODO/FIXME markers left in changed files
    [PASS] Clippy clean on changed files

  Verification: PASSED
  Confidence: High (all 5 checks passed)
```

### Step 8: View the orchestration context

The lessons and todos are automatically injected into the agent's system prompt. See what the agent receives:

```
> /orchestrate status --verbose
```

```
Workflow Orchestration Status:

  Lessons:  2 entries
  Todos:    3 entries (1 done, 2 pending)

  Context injected into agent prompts:
  ┌─────────────────────────────────────────────────────────────┐
  │ ## Project Lessons                                          │
  │ - Cursor-based pagination preferred over offset for APIs    │
  │ - SQLite: use WAL mode or PostgreSQL for concurrent writes  │
  │                                                             │
  │ ## Current Tasks                                            │
  │ - [DONE] Implement pagination for the inventory API         │
  │ - [PENDING/High] Migrate database from SQLite to PostgreSQL │
  │ - [PENDING/Low] Update README with API examples             │
  │                                                             │
  │ ## Complexity Notes                                         │
  │ - DB migration is High complexity (8 files, 2-4 hours)      │
  │ - README update is Low complexity (1 file, 10-20 min)       │
  └─────────────────────────────────────────────────────────────┘

  The agent uses this context to:
    1. Avoid repeating past mistakes
    2. Apply lessons learned to new tasks
    3. Understand remaining work and priorities
```

### Step 9: Reset orchestration (optional)

```
> /orchestrate reset
```

```
Are you sure you want to reset orchestration data? This will delete:
  - tasks/lessons.md (2 lessons)
  - tasks/todo.md (3 todos)

Type 'yes' to confirm: yes

Orchestration data reset.
  tasks/lessons.md deleted
  tasks/todo.md deleted
```

## The tasks/ Directory

Orchestration data lives in plain Markdown files that you can version-control:

```
project/
  tasks/
    lessons.md    # Lessons learned (auto-updated by agent)
    todo.md       # Task list with complexity and status
```

Both files use standard Markdown, so they render nicely on GitHub and can be edited manually.

## VibeUI: Orchestration Panel

The **Orchestration** panel in VibeUI provides a visual interface with tabs for:

- **Status** -- Overview of lessons count, todo progress, and context preview
- **Todos** -- Add, edit, reorder, and mark tasks complete with drag-and-drop
- **Lessons** -- Browse lessons with tag filtering and search
- **Verify** -- Run verification checks on completed tasks
- **History** -- Timeline of task completions and lessons learned

## Demo Recording

```json
{
  "meta": {
    "title": "Workflow Orchestration",
    "description": "Track tasks with complexity estimation, record lessons learned, verify completions, and inject context into agent prompts.",
    "duration_seconds": 200,
    "version": "1.0.0"
  },
  "steps": [
    {
      "id": 1,
      "action": "repl",
      "commands": [
        { "input": "/orchestrate status", "delay_ms": 2000 }
      ],
      "description": "Check initial orchestration status"
    },
    {
      "id": 2,
      "action": "repl",
      "commands": [
        { "input": "/orchestrate todo add \"Implement pagination for the inventory API\"", "delay_ms": 3000 },
        { "input": "/orchestrate todo add \"Migrate database from SQLite to PostgreSQL\"", "delay_ms": 3000 },
        { "input": "/orchestrate todo add \"Update README with API examples\"", "delay_ms": 3000 }
      ],
      "description": "Add three tasks with automatic complexity estimation"
    },
    {
      "id": 3,
      "action": "repl",
      "commands": [
        { "input": "/orchestrate todo", "delay_ms": 2000 }
      ],
      "description": "View the todo list"
    },
    {
      "id": 4,
      "action": "repl",
      "commands": [
        { "input": "/orchestrate lessons", "delay_ms": 2000 },
        { "input": "/orchestrate lesson add \"SQLite does not support concurrent writes well -- use WAL mode\"", "delay_ms": 3000 }
      ],
      "description": "View and add lessons"
    },
    {
      "id": 5,
      "action": "repl",
      "commands": [
        { "input": "/orchestrate verify todo_1", "delay_ms": 4000 },
        { "input": "/orchestrate status --verbose", "delay_ms": 3000 },
        { "input": "/quit", "delay_ms": 500 }
      ],
      "description": "Verify a task and view injected context"
    }
  ]
}
```

## What's Next

- [Demo 48: OpenMemory](../48-open-memory/) -- Persistent cognitive memory engine
- [Demo 49: Auto-Research](../49-auto-research/) -- Autonomous iterative research agent
- [Demo 52: Watch Mode & Sandbox](../52-watch-sandbox/) -- File watching and isolated execution
