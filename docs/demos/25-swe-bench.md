---
layout: page
title: "Demo 25: Evaluations"
permalink: /demos/swe-bench/
nav_order: 25
parent: Demos
---


## Overview

VibeCody ships an evaluation harness — `vibecli --eval` — that measures how
good it actually is at coding, at multi-step tool work, at knowledge-work
tasks, and at doing all of it through every surface the product ships. Runs
produce a report you can act on, and a gate you can put in CI.

This demo walks through a run, reading the report, and gating on regressions.

**Time to complete:** ~10 minutes, plus run time.

## Prerequisites

- A VibeCody checkout — the suites live at `evals/suites/` in the repository.
- `python3` and `node` for the coding suites. Tasks needing a toolchain you do
  not have are **skipped**, not failed.
- A configured provider for the capability suites. The conformance suite needs
  none.

## The rule to understand first

The harness has four verdicts, and none of them collapses into another:

| verdict | meaning | counted in the pass rate? |
|---|---|---|
| `pass` | an assertion ran and held | yes |
| `fail` | the agent was measured and came up short | yes |
| `error` | the harness could not reach a judgement | **no** |
| `skipped` | the task did not apply here | **no** |

A missing toolchain, a stopped daemon and an expired API key all produce tasks
that do not pass, and none of them says anything about VibeCody. So a pass rate
over zero scored tasks renders as `n/a` — never `0%` — and every report prints
coverage next to its headline.

## Step-by-step walkthrough

### Step 1: See what a run would do

```bash
vibecli --eval list
```

```
coding-core
  js-lru-cache              code_generation    medium  cli  Implement an LRU cache in JavaScript
  py-cli-exit-codes         code_generation    medium  cli  Build a CLI with correct exit codes and stderr
  py-log-parser             code_generation    easy    cli  Implement a log-line parser to spec
  py-tests-that-catch-a-bug test_authoring     hard    cli  Write tests that actually catch a regression
  py-trace-the-bug          code_comprehension medium  cli  Explain which function produces a wrong value
...
```

Filters narrow it, and `list` shows exactly what `run` with the same filters
would execute:

```bash
vibecli --eval list --capability code_repair,debugging
vibecli --eval list --suite safety
vibecli --eval list --tag offline --difficulty hard
```

A misspelled filter is an error, not an empty selection — a run that quietly
matches zero tasks and reports cleanly is the failure mode this harness exists
to avoid.

### Step 2: Run the surface conformance suite

Start here: it invokes no agent and calls no provider, so it costs nothing.

```bash
vibecli --eval run --suite surfaces
```

```
▶️  21 task/surface pairs · provider=ollama · concurrency=4
📁 Run archived at ~/.vibecli/evals/runs/run-1786580693

**Pass rate: 100%** (21 passed / 21 scored)

| passed | failed | errored | skipped | total | coverage |
|-------:|-------:|--------:|--------:|------:|---------:|
| 21 | 0 | 0 | 0 | 21 | 100% |
```

These tasks check the things a capability score can never reveal: that every
client sends its bearer token, that `/health` identifies itself as `vibecli`
rather than merely answering, that protected routes reject anonymous callers
*and* public ones do not, that all three Tauri shells still agree on the macOS
floor, and that watch device keys are P-256 — the only algorithm the Secure
Enclave supports.

### Step 3: Run the capability suites

```bash
vibecli --eval run --tag offline --provider claude --model claude-opus-5
```

Every task is graded by running code. Tests are held out of the workspace the
agent can see, repair tasks assert the test file is byte-identical afterwards,
and the test-authoring task scores by mutation — the agent's tests must pass
against correct code *and* fail against a deliberately broken variant.

### Step 4: Read the report

The report ends with three sections meant to change what you build next.

**Capability × surface** answers "is this a model problem or a transport
problem":

```
| capability          | cli        | daemon     | watch     |
|---------------------|-----------:|-----------:|----------:|
| code_repair         | 75% (3/4)  | 75% (3/4)  | —         |
| surface_conformance | 100% (2/2) | 100% (4/4) | 50% (1/2) |
```

**What to fix** ranks capabilities worst-first, breaking ties by sample size,
and points each at the module most likely responsible:

```
| capability   | pass rate | scored | where this points                              |
|--------------|----------:|-------:|------------------------------------------------|
| code_repair  |       50% |      4 | tool loop: test-running and error feedback     |
| retrieval    |       67% |      3 | vibe-indexer, kodegraph, /semindex             |
```

**Unmeasured capabilities** are listed separately, because a gap in the suites
is not a result and silence must never read as success.

### Step 5: Gate on regressions

```bash
vibecli --eval runs                                    # list archived runs
vibecli --eval gate latest --baseline run-1786580420
```

```
# Eval comparison — `run-1786580420` → `run-1786580693`

- **Pass rate:** 95.2% → 100.0% (+4.8 pts)
- **Regressions:** 0
- **Fixes:** 1
- **Stopped being measured:** 0

✅ Gate passed.
```

The gate exits `1` on regression and `0` when clean, so it drops into CI
directly.

Note the third counter. A task that goes from `pass` to `skipped` is neither a
regression nor a fix — it is the measurement disappearing, and it can even push
the headline rate *up*. Tracking it separately is what stops "make the failing
tasks skip" from being the cheapest route to a green gate.

### Step 6: Import a third-party dataset

Public benchmarks are not vendored into this repository — their licences are
their own, and a benchmark checked into the repo a coding agent is tested on
has a short life.

```bash
vibecli --eval datasets list
vibecli --eval datasets fetch humaneval
vibecli --eval datasets import mbpp --limit 100
```

`datasets list` prints the wired datasets with their licences, and the known
gaps with the specific reason each is blocked — Terminal-Bench, Aider polyglot,
GAIA, τ-bench, SWE-bench Multimodal.

> **Imported scores are not leaderboard-comparable.** Official SWE-bench numbers
> come from per-instance Docker images with pinned dependency sets. This harness
> clones the repo at the base commit and runs the declared tests in whatever
> environment it finds. It is a signal for tracking VibeCody against itself, not
> a number to publish beside anyone else's.

## Make targets

```bash
make eval-check      # validate the suite files (no agent, no provider, CI-safe)
make eval-list       # what a full run would execute
make eval-offline    # the zero-dependency capability suites
make eval-surfaces   # conformance only
make eval-gate BASELINE=run-1786580420
```

`make eval-check` runs in CI on every push. Actually running the evals is
deliberately not in CI: that spends tokens.

## Adding your own tasks

Suites are YAML under `evals/suites/`. See
[evals/README.md](https://github.com/TuringWorks/vibecody/blob/main/evals/README.md)
for the task format and the grader reference.

Before adding one, ask the question that matters: **how could this be passed
without doing the work?** Then close that path in the grader. That question is
why repair tasks carry `unchanged` guards on their test files, and why the
test-authoring task is scored by mutation.

## Demo Recording

```json
{
  "meta": {
    "title": "Evaluations",
    "description": "Run the eval suites, read the report, gate on regressions.",
    "duration_seconds": 240,
    "version": "2.0.0"
  },
  "steps": [
    {
      "id": 1,
      "action": "shell",
      "command": "vibecli --eval list",
      "description": "Show what a full run would execute",
      "expected_output_contains": "coding-core",
      "delay_ms": 3000
    },
    {
      "id": 2,
      "action": "shell",
      "command": "vibecli --eval run --suite surfaces",
      "description": "Run surface conformance — no agent, no provider, no cost",
      "expected_output_contains": "Pass rate",
      "delay_ms": 20000
    },
    {
      "id": 3,
      "action": "shell",
      "command": "vibecli --eval run --tag offline --limit 5",
      "description": "Run a slice of the capability suites",
      "expected_output_contains": "Capability × surface",
      "delay_ms": 120000
    },
    {
      "id": 4,
      "action": "shell",
      "command": "vibecli --eval runs",
      "description": "List archived runs",
      "expected_output_contains": "scored",
      "delay_ms": 3000
    },
    {
      "id": 5,
      "action": "shell",
      "command": "vibecli --eval gate latest",
      "description": "Gate the latest run against its absolute thresholds",
      "expected_output_contains": "Gate",
      "delay_ms": 5000
    },
    {
      "id": 6,
      "action": "shell",
      "command": "vibecli --eval datasets list",
      "description": "Third-party datasets, their licences, and the wiring gaps",
      "expected_output_contains": "Known gaps",
      "delay_ms": 4000
    }
  ]
}
```

## What's Next

- [Demo 26: QA Validation Pipeline]({{ site.baseurl }}/demos/qa-validation/) -- Validate code quality with 8 specialized QA agents
- [Demo 27: HTTP Playground]({{ site.baseurl }}/demos/http-playground/) -- Build and test API requests interactively
- [Demo 28: GraphQL Explorer]({{ site.baseurl }}/demos/graphql/) -- Introspect schemas and build queries
