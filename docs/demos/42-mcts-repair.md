---
layout: page
title: "Demo 42: MCTS Code Repair"
permalink: /demos/42-mcts-repair/
---


## Overview

Traditional AI code repair is linear: the model reads the error, proposes one fix, and hopes it works. VibeCody takes a fundamentally different approach with Monte Carlo Tree Search (MCTS) repair. Inspired by AlphaGo's game-tree exploration, the MCTS repair engine builds a tree of candidate fixes, simulates each path by running the test suite, and selects the branch with the highest win rate. The result is dramatically more reliable fixes at a fraction of the cost -- often under $0.01 per resolved issue.

This demo walks through diagnosing a failing test with MCTS repair, comparing it against linear fix strategies, and reviewing the tree exploration statistics.

**Time to complete:** ~10 minutes

## Prerequisites

- VibeCLI 0.5.1 installed and on your PATH
- At least one AI provider configured (Claude, OpenAI, or Ollama)
- A project with a test suite that can be run via `cargo test`, `pytest`, `npm test`, or similar
- For VibeUI: the desktop app running with the **MctsRepairPanel** visible

## How MCTS Repair Works

The repair engine operates in four phases:

| Phase        | Description                                                        |
|--------------|--------------------------------------------------------------------|
| **Select**   | Walk the tree from root, picking the child with highest UCB1 score |
| **Expand**   | Ask the LLM to generate 2-4 candidate fix variants at the leaf    |
| **Simulate** | Apply each fix in a shadow workspace and run the test suite        |
| **Backprop** | Propagate pass/fail results up the tree, updating win rates        |

The engine repeats this cycle until a fix passes all tests or the exploration budget is exhausted.

## Step-by-Step Walkthrough

### Step 1: Identify a failing test

Suppose you have a Rust project where `test_login` is failing:

```bash
cargo test test_login
```

```
running 1 test
test auth::test_login ... FAILED

failures:

---- auth::test_login stdout ----
thread 'auth::test_login' panicked at 'assertion failed: `(left == right)`
  left: `Err(InvalidCredentials)`,
 right: `Ok(Session { user_id: 42 })`', src/auth.rs:87:9

test result: FAILED. 0 passed; 1 failed; 0 ignored
```

### Step 2: Launch MCTS repair from the REPL

Start VibeCLI and use the `/repair` command with the `mcts` strategy:

```bash
vibecli
```

```
> /repair mcts --issue "test_login fails with InvalidCredentials"
```

```
[MCTS Repair] Starting repair session...
[MCTS Repair] Analyzing test failure: test_login
[MCTS Repair] Identified root file: src/auth.rs
[MCTS Repair] Building search tree...

Phase 1: SELECT — Root node selected (UCB1: 1.414)
Phase 2: EXPAND — Generated 3 candidate fixes:
  ├── Fix A: Update password hash comparison (bcrypt → argon2id)
  ├── Fix B: Fix session token generation (missing salt)
  └── Fix C: Correct credential validation order (check email before password)

Phase 3: SIMULATE — Running test suite on each candidate...
  ├── Fix A: cargo test test_login ... 0/1 passed (hash mismatch persists)
  ├── Fix B: cargo test test_login ... 0/1 passed (token gen not reached)
  └── Fix C: cargo test test_login ... 1/1 passed ✓

Phase 4: BACKPROP — Updating win rates:
  ├── Fix A: 0.00 (0/1)
  ├── Fix B: 0.00 (0/1)
  └── Fix C: 1.00 (1/1) ★

[MCTS Repair] Solution found in 1 iteration!
[MCTS Repair] Applying Fix C to src/auth.rs

── src/auth.rs ──
- if validate_password(&input.password, &user.hash) {
+ if validate_email(&input.email) && validate_password(&input.password, &user.hash) {

[MCTS Repair] Verification: cargo test test_login ... 1/1 passed ✓
[MCTS Repair] Full suite: cargo test ... 47/47 passed ✓

Repair complete.
  Strategy:        MCTS (3 candidates, 1 iteration)
  Nodes explored:  4
  Total LLM calls: 1
  Tokens used:     2,847
  Estimated cost:  $0.008
  Time elapsed:    12.3s
```

### Step 3: Review the exploration tree

View detailed tree statistics:

```
> /repair tree
```

```
MCTS Exploration Tree — Session mcts-2026-03-29-001
════════════════════════════════════════════════════

Root (test_login fails)
├── [A] Update password hash comparison
│   UCB1: 0.000 | Visits: 1 | Wins: 0 | Depth: 1
├── [B] Fix session token generation
│   UCB1: 0.000 | Visits: 1 | Wins: 0 | Depth: 1
└── [C] Correct credential validation order ★ WINNER
    UCB1: 2.414 | Visits: 1 | Wins: 1 | Depth: 1

Tree stats:
  Total nodes:       4 (1 root + 3 leaves)
  Max depth:         1
  Branching factor:  3.0
  Exploration const: 1.414 (√2)
```

### Step 4: Compare MCTS vs linear repair

Run the comparison to see how MCTS stacks up against a standard linear fix approach:

```
> /repair compare
```

```
Repair Strategy Comparison — test_login
════════════════════════════════════════

                    │ Linear    │ MCTS
────────────────────┼───────────┼──────────
Attempts            │ 3         │ 1 iter
LLM calls           │ 3         │ 1
Tokens used         │ 8,412     │ 2,847
Estimated cost      │ $0.025    │ $0.008
Time elapsed        │ 34.1s     │ 12.3s
Fix found           │ Yes (3rd) │ Yes (1st)
Full suite pass     │ Yes       │ Yes
Regression risk     │ Medium    │ Low

MCTS advantage: 68% fewer tokens, 64% faster, 3× fewer attempts.

At scale (1,000 issues/month):
  Linear:  ~$25.00/month
  MCTS:    ~$8.00/month  (save $17.00/month)
  Per-issue average: $0.008 — under $0.01/issue ✓
```

### Step 5: Repair a harder bug with deeper exploration

For more complex bugs, MCTS explores deeper trees:

```
> /repair mcts --issue "test_concurrent_writes deadlocks" --max-depth 4 --budget 20
```

```
[MCTS Repair] Starting repair session...
[MCTS Repair] Analyzing test failure: test_concurrent_writes
[MCTS Repair] Identified root files: src/db/pool.rs, src/db/transaction.rs
[MCTS Repair] Building search tree (max depth: 4, budget: 20 nodes)...

Iteration 1: 3 candidates generated, 0 passing
Iteration 2: 2 candidates generated (expanding Fix A), 0 passing
Iteration 3: 2 candidates generated (expanding Fix A.2), 1 passing ✓

[MCTS Repair] Solution found in 3 iterations!

── Tree Summary ──
  Nodes explored:  8 / 20 budget
  Max depth used:  3 / 4 limit
  LLM calls:       3
  Tokens used:     6,203
  Estimated cost:  $0.018
  Winning path:    Root → A (lock ordering) → A.2 (Arc<Mutex>) → A.2.1 (try_lock with retry)

Repair complete. 2 files modified:
  src/db/pool.rs        (+4 -2 lines)
  src/db/transaction.rs (+7 -3 lines)
```

### Step 6: View repair history

```
> /repair history
```

```
MCTS Repair History
═══════════════════

ID                       │ Issue                         │ Nodes │ Cost    │ Result
─────────────────────────┼───────────────────────────────┼───────┼─────────┼────────
mcts-2026-03-29-001      │ test_login fails              │ 4     │ $0.008  │ Fixed ✓
mcts-2026-03-29-002      │ test_concurrent_writes        │ 8     │ $0.018  │ Fixed ✓

Total issues resolved: 2
Average cost per issue:  $0.013
Average nodes explored:  6.0
```

### Step 7: Use MCTS repair in VibeUI

In the VibeUI desktop app, open the **MctsRepairPanel** from the AI sidebar. The panel provides:

- **Tree Visualizer** -- Interactive tree diagram showing explored nodes, win rates, and the winning path highlighted in green
- **Live Simulation** -- Watch test results stream in as each candidate fix is evaluated
- **Cost Tracker** -- Running cost estimate that updates with each LLM call
- **History Tab** -- Browse past repair sessions with full tree replay

## Demo Recording

```json
{
  "meta": {
    "title": "MCTS Code Repair",
    "description": "Use Monte Carlo Tree Search to find optimal code fixes with tree exploration.",
    "duration_seconds": 180,
    "version": "0.5.1"
  },
  "steps": [
    {
      "id": 1,
      "action": "shell",
      "command": "cargo test test_login",
      "description": "Show the failing test",
      "expected_output_contains": "FAILED",
      "delay_ms": 3000
    },
    {
      "id": 2,
      "action": "repl",
      "commands": [
        { "input": "/repair mcts --issue \"test_login fails with InvalidCredentials\"", "delay_ms": 15000 },
        { "input": "/repair tree", "delay_ms": 3000 },
        { "input": "/repair compare", "delay_ms": 3000 },
        { "input": "/repair history", "delay_ms": 2000 },
        { "input": "/quit", "delay_ms": 500 }
      ],
      "description": "Run MCTS repair, view tree, compare strategies, check history"
    },
    {
      "id": 3,
      "action": "shell",
      "command": "cargo test test_login",
      "description": "Verify the fix passes",
      "expected_output_contains": "1 passed",
      "delay_ms": 3000
    },
    {
      "id": 4,
      "action": "shell",
      "command": "cargo test",
      "description": "Verify no regressions in full suite",
      "expected_output_contains": "0 failed",
      "delay_ms": 5000
    }
  ]
}
```

## What's Next

- [Demo 43: Cost-Optimized Agent Routing](../43-cost-routing/) -- Route tasks to the cheapest viable model automatically
- [Demo 44: Visual Verification](../44-visual-verify/) -- Screenshot-based design compliance checking
- [Demo 46: Code Replay](../46-code-replay/) -- Replay past agent sessions for debugging and auditing
