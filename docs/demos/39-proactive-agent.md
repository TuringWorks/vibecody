---
layout: page
title: "Demo 39: Proactive Agent Intelligence"
permalink: /demos/39-proactive-agent/
---


## Overview

VibeCody's proactive agent continuously monitors your codebase in the background and surfaces actionable suggestions without being asked. It detects potential bugs, performance issues, missing tests, outdated dependencies, and code quality improvements. You train its learning store by accepting or rejecting suggestions, so it adapts to your preferences over time and avoids repeating noise.

**Time to complete:** ~8 minutes

## Prerequisites

- VibeCLI v0.5.1 installed and on your PATH
- A git repository with source code
- At least one AI provider configured
- For VibeUI: the desktop app running with the **Proactive** panel visible

## How It Works

The proactive agent runs a background scan at configurable intervals (default: every 10 minutes while the REPL or TUI is active). It analyzes recent file changes, git diffs, and project structure to generate suggestions. Each suggestion includes a severity, category, file location, and a concrete fix.

The learning store records your accept/reject decisions and adjusts future suggestions accordingly. After enough feedback, the agent stops suggesting patterns you consistently reject.

## Step-by-Step Walkthrough

### 1. Run a Proactive Scan

Trigger a manual scan to see what the agent finds.

**REPL:**

```bash
vibecli
> /proactive scan
```

Example output:

```
Proactive Scan Results (scanned 47 files in 3.2s):

  #1 [HIGH] Missing error handling
     File: src/api/handlers.rs:34
     The unwrap() on line 34 will panic if the database query fails.
     Suggestion: Replace with proper error propagation using ?.
     Category: bug-prevention

  #2 [MEDIUM] Unused dependency
     File: Cargo.toml
     The crate `chrono` is listed as a dependency but not imported
     anywhere in the project.
     Suggestion: Remove chrono from [dependencies].
     Category: cleanup

  #3 [MEDIUM] Missing test coverage
     File: src/auth/jwt.rs
     The function validate_token (42 lines) has no corresponding test.
     Suggestion: Add unit tests for valid, expired, and malformed tokens.
     Category: testing

  #4 [LOW] Inconsistent naming
     File: src/models/user.rs:12
     Field `userName` uses camelCase while all other fields use snake_case.
     Suggestion: Rename to user_name for consistency.
     Category: style

  #5 [LOW] TODO comment (stale)
     File: src/db/migrations.rs:88
     TODO comment dated 2025-11-02: "fix this later" — 4 months old.
     Suggestion: Resolve or remove the stale TODO.
     Category: cleanup

Found 5 suggestions (1 high, 2 medium, 2 low).
Use /proactive accept <id> or /proactive reject <id> to train the learning store.
```

### 2. Accept a Suggestion

Accept a suggestion to apply the fix and teach the learning store.

**REPL:**

```bash
vibecli
> /proactive accept 1
```

Example output:

```
Applying suggestion #1: Missing error handling

Before (src/api/handlers.rs:34):
  let user = db.query_user(id).unwrap();

After (src/api/handlers.rs:34):
  let user = db.query_user(id)
      .map_err(|e| ApiError::Database(e))?;

File saved. Learning store updated:
  Pattern: unwrap-on-fallible → preference: fix (confidence: 0.6)
```

### 3. Reject a Suggestion

Reject a suggestion you disagree with. The learning store records your preference.

**REPL:**

```bash
vibecli
> /proactive reject 4
```

Example output:

```
Suggestion #4 rejected: Inconsistent naming

Learning store updated:
  Pattern: camelCase-field-rename → preference: ignore (confidence: 0.6)

Note: After 3 rejections of the same pattern, the agent will stop
suggesting it. Current rejections for this pattern: 1/3.
```

### 4. Configure Scan Cadence

Set how often the background scanner runs.

**REPL:**

```bash
vibecli
> /proactive config --interval 5m --severity medium --categories bug-prevention,testing,security
```

Example output:

```
Proactive Agent Configuration Updated:
  Scan interval:      5 minutes (was: 10 minutes)
  Minimum severity:   medium (was: low)
  Enabled categories: bug-prevention, testing, security
  Disabled:           cleanup, style, performance

Background scanning active. Next scan in 5m.
```

Available categories:

| Category           | What It Detects                                     |
|--------------------|-----------------------------------------------------|
| `bug-prevention`   | Panics, unwraps, unchecked errors, null risks       |
| `security`         | Hardcoded secrets, injection risks, unsafe blocks   |
| `testing`          | Missing tests, low coverage, untested edge cases    |
| `performance`      | Unnecessary allocations, N+1 queries, blocking I/O  |
| `cleanup`          | Unused deps, dead code, stale TODOs                 |
| `style`            | Naming inconsistencies, formatting, doc comments    |

### 5. View the Learning Store

Inspect what the agent has learned from your accept/reject history.

**REPL:**

```bash
vibecli
> /proactive learn
```

Example output:

```
Learning Store (12 patterns recorded):

  Pattern                       Preference  Confidence  Decisions
  unwrap-on-fallible            fix         0.92        8 accept, 1 reject
  missing-test-coverage         fix         0.85        6 accept, 1 reject
  stale-todo                    fix         0.78        4 accept, 1 reject
  unused-dependency             fix         0.71        3 accept, 1 reject
  camelCase-field-rename        ignore      0.88        1 accept, 5 reject
  clippy-pedantic-lint          ignore      0.75        0 accept, 3 reject
  add-doc-comment               neutral     0.50        2 accept, 2 reject
  ...

Suppressed patterns (auto-muted after 3+ rejections):
  camelCase-field-rename
  clippy-pedantic-lint
```

### 6. Reset Learning for a Pattern

If you change your mind about a previously rejected pattern, reset it.

**REPL:**

```bash
vibecli
> /proactive learn --reset camelCase-field-rename
```

Example output:

```
Pattern camelCase-field-rename reset.
  Previous: ignore (confidence: 0.88, 5 rejections)
  Current:  neutral (confidence: 0.50, 0 decisions)

The agent will start suggesting this pattern again.
```

### 7. View Suggestion History

See all past suggestions and your decisions.

**REPL:**

```bash
vibecli
> /proactive history --last 10
```

Example output:

```
Suggestion History (last 10):

  Date        Severity  Category         File                     Decision
  2026-03-29  HIGH      bug-prevention   src/api/handlers.rs:34   accepted
  2026-03-29  MEDIUM    cleanup          Cargo.toml               pending
  2026-03-29  MEDIUM    testing          src/auth/jwt.rs          pending
  2026-03-29  LOW       style            src/models/user.rs:12    rejected
  2026-03-29  LOW       cleanup          src/db/migrations.rs:88  pending
  2026-03-28  HIGH      security         src/config.rs:22         accepted
  2026-03-28  MEDIUM    performance      src/db/queries.rs:56     accepted
  2026-03-28  MEDIUM    testing          src/api/routes.rs        accepted
  2026-03-28  LOW       style            src/models/post.rs:8     rejected
  2026-03-27  HIGH      bug-prevention   src/main.rs:112          accepted
```

### 8. VibeUI Proactive Panel

Open the **Proactive** panel in VibeUI to see:

- **Suggestions** tab: card-based view of current suggestions with accept/reject buttons, file links, and inline diffs
- **Config** tab: toggle categories, set severity threshold, adjust scan interval with a slider
- **Learning** tab: visual breakdown of pattern preferences with confidence bars
- **History** tab: filterable table of all past suggestions and decisions

## Configuration Reference

Add proactive settings to `~/.vibecli/config.toml`:

```toml
[proactive]
enabled = true
interval = "10m"
min_severity = "low"
categories = ["bug-prevention", "security", "testing", "performance", "cleanup", "style"]
max_suggestions_per_scan = 10
learning_store = "~/.vibecli/proactive-learning.json"
```

## Demo Recording JSON

```json
{
  "meta": {
    "title": "Proactive Agent Intelligence",
    "description": "Background code scanning with AI-powered suggestions and a learning feedback loop.",
    "duration_seconds": 150,
    "version": "0.5.1"
  },
  "steps": [
    {
      "id": 1,
      "action": "repl",
      "commands": [
        { "input": "/proactive scan", "delay_ms": 6000 }
      ],
      "description": "Run a proactive scan and review suggestions"
    },
    {
      "id": 2,
      "action": "repl",
      "commands": [
        { "input": "/proactive accept 1", "delay_ms": 3000 }
      ],
      "description": "Accept a suggestion and apply the fix"
    },
    {
      "id": 3,
      "action": "repl",
      "commands": [
        { "input": "/proactive reject 4", "delay_ms": 2000 }
      ],
      "description": "Reject a suggestion and train the learning store"
    },
    {
      "id": 4,
      "action": "repl",
      "commands": [
        { "input": "/proactive config --interval 5m --severity medium --categories bug-prevention,testing,security", "delay_ms": 2000 }
      ],
      "description": "Configure scan cadence and categories"
    },
    {
      "id": 5,
      "action": "repl",
      "commands": [
        { "input": "/proactive learn", "delay_ms": 2000 }
      ],
      "description": "View the learning store"
    },
    {
      "id": 6,
      "action": "repl",
      "commands": [
        { "input": "/proactive history --last 10", "delay_ms": 2000 }
      ],
      "description": "Review suggestion history"
    },
    {
      "id": 7,
      "action": "vibeui_interaction",
      "panel": "Proactive",
      "tab": "Suggestions",
      "description": "Browse and act on suggestions in VibeUI"
    },
    {
      "id": 8,
      "action": "vibeui_interaction",
      "panel": "Proactive",
      "tab": "Learning",
      "description": "View pattern preferences and confidence levels"
    }
  ]
}
```

## What's Next

- [Demo 9: Autofix & Diagnostics](../09-autofix/) -- Automated bug detection and one-click fixes
- [Demo 40: Web Search Grounding](../40-web-grounding/) -- Enrich agent answers with live web data
- [Demo 23: Test Runner & Coverage](../test-coverage/) -- Verify test coverage for flagged files
