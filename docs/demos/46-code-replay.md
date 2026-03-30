---
layout: page
title: "Demo 46: Code Replay & Explainable Agent"
permalink: /demos/46-code-replay/
---


## Overview

Every VibeCody agent session is recorded as a trace -- a complete log of the reasoning chain, tool calls, file edits, and decisions the agent made. Code Replay lets you step through past sessions like a debugger, while the Explainable Agent feature surfaces the reasoning behind each decision. Together, these features are invaluable for debugging unexpected agent behavior, auditing code changes for compliance, onboarding new team members, and learning from the agent's problem-solving patterns.

**Time to complete:** ~8 minutes

## Prerequisites

- VibeCLI 0.5.1 installed and on your PATH
- At least one completed agent session (traces stored in `~/.vibecli/traces/`)
- For VibeUI: the desktop app with access to the Traces panel

## How Traces Work

VibeCody records three files per agent session:

| File                      | Contents                                           |
|---------------------------|----------------------------------------------------|
| `<session-id>.jsonl`      | Event stream: each tool call, response, file edit   |
| `<session-id>-messages.json` | Full message history (user + assistant turns)    |
| `<session-id>-context.json`  | Initial context: files read, project state       |

Traces are stored in `~/.vibecli/traces/` and retained for 30 days by default.

## Step-by-Step Walkthrough

### Step 1: List past sessions

Start VibeCLI and view recorded sessions:

```bash
vibecli
```

```
> /replay list
```

```
Agent Session Traces
════════════════════

ID                    │ Date                │ Duration │ Task                          │ Events
──────────────────────┼─────────────────────┼──────────┼───────────────────────────────┼───────
sess-20260329-001     │ 2026-03-29 09:14:02 │ 45s      │ Fix auth test failures        │ 12
sess-20260328-003     │ 2026-03-28 16:42:18 │ 2m 13s   │ Refactor DB connection pool   │ 28
sess-20260328-002     │ 2026-03-28 14:05:33 │ 1m 07s   │ Add pagination to /users API  │ 19
sess-20260328-001     │ 2026-03-28 10:22:45 │ 32s      │ Update README badges          │ 7
sess-20260327-002     │ 2026-03-27 15:38:10 │ 3m 44s   │ Implement rate limiting       │ 34

Showing 5 of 23 sessions. Use `/replay list --all` for full history.
```

### Step 2: Replay a session

Step through a past session event by event:

```
> /replay sess-20260329-001
```

```
Replaying: sess-20260329-001 — "Fix auth test failures"
Started: 2026-03-29 09:14:02 | Duration: 45s | Events: 12

[1/12] 09:14:02  USER_MESSAGE
  "Fix the failing auth tests"

[2/12] 09:14:03  REASONING
  The user wants me to fix failing auth tests. I should first run the
  test suite to identify which tests are failing and examine the error
  messages.

[3/12] 09:14:04  TOOL_CALL  → shell
  Command: cargo test auth:: 2>&1
  Result: 2 tests failed (test_login, test_token_refresh)

[4/12] 09:14:08  REASONING
  Two tests failed: test_login and test_token_refresh. Both show
  InvalidCredentials errors. Let me read the auth module to understand
  the validation logic.

[5/12] 09:14:09  TOOL_CALL  → read_file
  Path: src/auth.rs
  Lines read: 1-120

[6/12] 09:14:10  REASONING
  I see the issue. The validate_user function checks the password
  hash before verifying the email exists. If the email lookup fails,
  it returns InvalidCredentials instead of UserNotFound, and the
  hash comparison panics on None.

[7/12] 09:14:11  TOOL_CALL  → edit_file
  Path: src/auth.rs
  Change: Reorder validation — check email existence first
  Lines modified: 45-52

[8/12] 09:14:12  TOOL_CALL  → edit_file
  Path: src/auth.rs
  Change: Add early return for missing user
  Lines modified: 46-48

[9/12] 09:14:13  TOOL_CALL  → shell
  Command: cargo test auth:: 2>&1
  Result: 4 tests passed, 0 failed ✓

[10/12] 09:14:18  REASONING
  All auth tests pass now. Let me run the full suite to check for
  regressions.

[11/12] 09:14:19  TOOL_CALL  → shell
  Command: cargo test 2>&1
  Result: 47 tests passed, 0 failed ✓

[12/12] 09:14:44  RESPONSE
  Fixed 2 failing auth tests by reordering validation logic in
  src/auth.rs. The email existence check now runs before password
  hash comparison.

Replay complete.
```

### Step 3: Explain agent reasoning

Use `/explain` to get a structured breakdown of the agent's decision-making:

```
> /explain sess-20260329-001
```

```
Explainable Agent — sess-20260329-001
═════════════════════════════════════

Task: Fix auth test failures

Decision Chain:
  1. DIAGNOSE → Ran test suite to identify failures
     Rationale: Need to see exact errors before reading code
     Alternatives considered: Read code first (rejected — test output is more targeted)

  2. INVESTIGATE → Read src/auth.rs
     Rationale: Both failures point to validation logic in the auth module
     Files considered: src/auth.rs, src/models/user.rs (auth.rs chosen — errors originate there)

  3. ROOT CAUSE → Validation order bug
     Finding: Password hash checked before email existence verified
     Confidence: High (error message matches code path)

  4. FIX → Reorder validation + add early return
     Approach: Minimal change — only reorder existing checks
     Alternatives: Add separate email validation function (rejected — over-engineering)

  5. VERIFY → Ran full test suite
     Result: 47/47 passed, no regressions
     Rationale: Full suite needed to catch side effects of validation order change

Risk Assessment:
  Regression risk: Low (validation order change is isolated)
  Behavioral change: UserNotFound error now returned instead of InvalidCredentials
                     when email is missing (more correct behavior)

Cost: $0.012 | Tokens: 3,241 | Tool calls: 5
```

### Step 4: Filter and search traces

Search for sessions involving specific files or patterns:

```
> /replay search --file src/auth.rs
```

```
Sessions involving src/auth.rs:
  sess-20260329-001  │ Fix auth test failures        │ edited lines 45-52
  sess-20260325-004  │ Add OAuth2 support             │ edited lines 88-134
  sess-20260320-002  │ Implement JWT refresh          │ edited lines 60-75

3 sessions found.
```

```
> /replay search --tool edit_file --since 7d
```

```
Sessions with edit_file calls (last 7 days):
  14 sessions, 42 total file edits across 18 unique files.
  Most edited: src/auth.rs (3 sessions), src/api/routes.rs (2 sessions)
```

### Step 5: Export a trace for sharing

Export a session as a standalone report:

```
> /replay export sess-20260329-001 --format markdown
```

```
Exported: .vibecli/traces/exports/sess-20260329-001-report.md

Contents:
  - Task summary
  - Full reasoning chain
  - All file diffs
  - Test results before/after
  - Cost breakdown
```

### Step 6: Compare two sessions

Useful for understanding why the agent took different approaches on similar tasks:

```
> /replay compare sess-20260329-001 sess-20260325-004
```

```
Session Comparison
══════════════════

                     │ sess-20260329-001         │ sess-20260325-004
─────────────────────┼───────────────────────────┼──────────────────────────
Task                 │ Fix auth test failures    │ Add OAuth2 support
Duration             │ 45s                       │ 3m 22s
Tool calls           │ 5                         │ 14
Files edited         │ 1                         │ 4
Lines changed        │ +6 -3                     │ +89 -12
Tests before         │ 45/47 (2 failing)         │ 47/47 (all passing)
Tests after          │ 47/47                     │ 52/52 (5 new)
Cost                 │ $0.012                    │ $0.067
Strategy             │ Diagnose → Fix → Verify   │ Plan → Implement → Test → Verify

Common files: src/auth.rs
Approach difference: First session was a targeted bug fix (minimal change).
Second session was feature addition (new code paths, new tests).
```

### Step 7: Use code replay in VibeUI

In the VibeUI desktop app, the **Traces** panel provides a visual replay experience:

- **Timeline View** -- Horizontal timeline with event markers, click to jump to any point
- **Diff Viewer** -- Side-by-side file diffs at each edit step
- **Reasoning Panel** -- Agent's internal reasoning displayed alongside each action
- **Cost Graph** -- Cumulative cost chart showing spend per tool call
- **Search** -- Full-text search across all traces

## Demo Recording

```json
{
  "meta": {
    "title": "Code Replay & Explainable Agent",
    "description": "Replay past agent sessions and inspect reasoning chains.",
    "duration_seconds": 150,
    "version": "0.5.1"
  },
  "steps": [
    {
      "id": 1,
      "action": "repl",
      "commands": [
        { "input": "/replay list", "delay_ms": 2000 },
        { "input": "/replay sess-20260329-001", "delay_ms": 8000 },
        { "input": "/explain sess-20260329-001", "delay_ms": 4000 },
        { "input": "/replay search --file src/auth.rs", "delay_ms": 2000 },
        { "input": "/replay export sess-20260329-001 --format markdown", "delay_ms": 2000 },
        { "input": "/quit", "delay_ms": 500 }
      ],
      "description": "List sessions, replay one, explain reasoning, search, and export"
    }
  ]
}
```

## What's Next

- [Demo 47: Multi-LLM Deliberation](../47-counsel-superbrain/) -- Structured debates and consensus across providers
- [Demo 42: MCTS Code Repair](../42-mcts-repair/) -- Fix bugs with tree-search exploration
- [Demo 4: Agent Loop](../agent-loop/) -- Learn how the agent loop works from scratch
