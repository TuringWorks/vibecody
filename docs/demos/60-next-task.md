---
layout: page
title: "Demo 60: Next-Task Prediction"
permalink: /demos/60-next-task/
nav_order: 60
parent: Demos
---


## Overview

VibeCody's NextTask module uses a workflow state machine and intent inference to predict what you should do next based on your recent activity. It observes file edits, test runs, git operations, and REPL commands to build a model of your development workflow, then suggests the most likely next action. The system learns from your accept/reject feedback using a lightweight Q-learning approach, improving its predictions over time. NextTask turns VibeCody from a reactive assistant into a proactive development partner.

**Time to complete:** ~10 minutes

## Prerequisites

- VibeCLI v0.5.1 installed and on your PATH
- At least one AI provider configured
- An active project with recent activity (file edits, git commits, test runs)
- (Optional) VibeUI running with the **NextTask** panel visible

## How It Works

NextTask tracks your workflow through a state machine with these states:

| State         | Triggered By                        | Example Next Actions              |
|---------------|-------------------------------------|-----------------------------------|
| **Editing**   | File save, code change              | Run tests, lint, format           |
| **Testing**   | Test run, coverage check            | Fix failures, commit, refactor    |
| **Debugging** | Error output, breakpoint hit        | Read logs, add prints, fix bug    |
| **Reviewing** | PR open, diff view                  | Comment, approve, request changes |
| **Deploying** | Build success, tag push             | Monitor, rollback, notify         |
| **Planning**  | Issue creation, spec editing        | Break down tasks, assign, start   |
| **Idle**      | No activity for 5+ minutes          | Resume last task, check PRs       |

Transitions between states are weighted by frequency. Accept/reject feedback adjusts the Q-values for each state-action pair.

## Step-by-Step Walkthrough

### Step 1: Get a Task Suggestion

After working on your project for a while, ask NextTask what to do next.

```bash
vibecli
```

```
/nexttask suggest
```

Expected output:

```
Next Task Suggestion

  Current State: Editing (src/server.rs modified 2 min ago)
  Confidence:    87%

  Suggested Action:
    Run tests for the server module

  Reasoning:
    1. You modified src/server.rs (added health endpoint)
    2. Last 5 times you edited server.rs, you ran tests 4 times
    3. No tests have been run since the last edit
    4. Tests for this module take ~8 seconds

  Command: cargo test --lib server

  [A]ccept  [R]eject  [S]kip  [D]etail
```

### Step 2: Accept a Suggestion

Accept the suggestion to execute it and provide positive feedback to the model.

```
/nexttask accept
```

```
Executing: cargo test --lib server

running 12 tests
test server::tests::test_health_endpoint ... ok
test server::tests::test_chat_endpoint ... ok
test server::tests::test_stream_endpoint ... ok
test server::tests::test_auth_middleware ... ok
test server::tests::test_rate_limiter ... ok
test server::tests::test_cors_headers ... ok
test server::tests::test_error_responses ... ok
test server::tests::test_graceful_shutdown ... ok
test server::tests::test_metrics_endpoint ... ok
test server::tests::test_version_header ... ok
test server::tests::test_request_logging ... ok
test server::tests::test_timeout_handling ... ok

test result: ok. 12 passed; 0 failed; 0 ignored

Feedback recorded: accept (Editing -> RunTests)
Q-value updated: 0.72 -> 0.78

Next suggestion available. Run /nexttask suggest again.
```

### Step 3: Reject a Suggestion

When a suggestion is not relevant, reject it to provide negative feedback.

```
/nexttask suggest
```

```
Next Task Suggestion

  Current State: Testing (all tests passed 30s ago)
  Confidence:    74%

  Suggested Action:
    Commit your changes

  Reasoning:
    1. All tests pass
    2. You have 3 modified files
    3. Last 4 times tests passed after edits, you committed 3 times

  Command: git add -A && git commit

  [A]ccept  [R]eject  [S]kip  [D]etail
```

```
/nexttask reject
```

```
Suggestion rejected.
Feedback recorded: reject (Testing -> Commit)
Q-value updated: 0.65 -> 0.58

Why did you reject? (optional, press Enter to skip):
> I want to add more tests first

Noted. Adjusting: Testing -> WriteTests weight increased.

New suggestion:

  Suggested Action:
    Write additional tests for the new health endpoint

  Confidence: 81%
  Command:    vibecli --agent "Write tests for the health endpoint in src/server.rs"

  [A]ccept  [R]eject  [S]kip  [D]etail
```

### Step 4: View Prediction Details

Use the Detail option to see the full scoring breakdown.

```
/nexttask suggest
```

Then press `D` for details:

```
/nexttask detail
```

```
Prediction Detail

  State: Testing
  History depth: 47 transitions analyzed

  Candidate Actions (ranked):

  Rank  Action          Q-Value  Frequency  Recency  Score
  1     WriteTests      0.81     12/47      2 min    0.84
  2     Commit          0.58     14/47      --       0.62
  3     Refactor        0.55     8/47       --       0.48
  4     OpenPR          0.42     5/47       --       0.38
  5     SwitchBranch    0.30     3/47       --       0.25

  Score formula:
    score = (Q-value * 0.5) + (frequency * 0.3) + (recency * 0.2)

  Recent transitions:
    Idle -> Editing -> Testing -> [current]
    Last session: Editing -> Testing -> Commit -> Reviewing -> Idle
```

### Step 5: View Learning Statistics

Check how the prediction model has evolved over time.

```
/nexttask stats
```

```
NextTask Learning Statistics

  Sessions tracked:      23
  Total transitions:     347
  Accept rate:           78% (156/200 suggestions)
  Prediction accuracy:   82% (last 50 suggestions)

  Top Learned Patterns:
    Editing -> RunTests         Q=0.89  (strongest signal)
    Testing (pass) -> Commit    Q=0.73
    Testing (fail) -> Debug     Q=0.85
    Debugging -> Editing        Q=0.77
    Reviewing -> Comment        Q=0.68

  Accuracy Trend (last 5 sessions):
    Session 19:  72%
    Session 20:  76%
    Session 21:  80%
    Session 22:  84%
    Session 23:  85%

  The model is improving. Accuracy has increased 13% over the last
  5 sessions.
```

### Step 6: Reset the Model

If the model has learned bad patterns (e.g., from a different project type), you can reset it.

```
/nexttask reset
```

```
Reset NextTask Model?
  This will clear 347 transitions and all Q-values.
  The model will start learning from scratch.

  [y/n]: y

Model reset. NextTask will start with default heuristics and learn
from your new activity.
```

### Step 7: View in VibeUI

Open VibeUI and navigate to the **NextTask** panel. The panel provides:

- **Suggestion** -- Current suggestion with accept/reject buttons and confidence gauge
- **Timeline** -- Visual timeline of recent state transitions
- **Patterns** -- Learned workflow patterns with Q-values displayed as a heatmap
- **Stats** -- Accuracy trend chart and accept/reject history
- **Settings** -- Configure suggestion frequency, auto-execute threshold, and model parameters

## Demo Recording JSON

```json
{
  "meta": {
    "title": "Next-Task Prediction",
    "description": "AI-powered workflow prediction with Q-learning feedback loop.",
    "duration_seconds": 240,
    "version": "0.5.1"
  },
  "steps": [
    {
      "id": 1,
      "action": "repl",
      "commands": [
        { "input": "/nexttask suggest", "delay_ms": 3000 },
        { "input": "/nexttask accept", "delay_ms": 5000 },
        { "input": "/nexttask suggest", "delay_ms": 3000 },
        { "input": "/nexttask reject", "delay_ms": 2000 },
        { "input": "/nexttask suggest", "delay_ms": 3000 },
        { "input": "/nexttask detail", "delay_ms": 3000 },
        { "input": "/nexttask stats", "delay_ms": 3000 },
        { "input": "/quit", "delay_ms": 500 }
      ],
      "description": "Full NextTask workflow: suggest, accept, reject, detail, stats"
    },
    {
      "id": 2,
      "action": "vibeui_interaction",
      "panel": "NextTask",
      "tab": "Patterns",
      "description": "View learned workflow patterns as a heatmap"
    },
    {
      "id": 3,
      "action": "vibeui_interaction",
      "panel": "NextTask",
      "tab": "Stats",
      "description": "View accuracy trend and learning progress"
    }
  ]
}
```

## What's Next

- [Demo 04: Agent Loop & Tool Execution](../agent-loop/) -- Let the AI execute suggested actions
- [Demo 06: Cost Observatory](../cost-observatory/) -- Track costs of executed suggestions
- [Demo 58: Living Documentation Sync](../58-doc-sync/) -- Keep docs updated as tasks complete
