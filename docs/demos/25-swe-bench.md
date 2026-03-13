---
layout: page
title: "Demo 25: SWE-bench Benchmarking"
permalink: /demos/swe-bench/
nav_order: 25
parent: Demos
---

# Demo 25: SWE-bench Benchmarking

## Overview

This demo shows how to run SWE-bench benchmarks against your AI provider configuration, compare model performance across runs, and export results as Markdown reports. VibeCody integrates the SWE-bench harness directly into both the CLI and VibeUI so you can evaluate coding ability without leaving your workflow.

**Time to complete:** ~15 minutes (excluding benchmark execution time)

## Prerequisites

- VibeCody installed and configured with at least one AI provider
- A working internet connection (benchmarks download test cases on first run)
- At least 2 GB of free disk space for benchmark data
- For VibeUI: the desktop app running (`npm run tauri dev`)

## Step-by-Step Walkthrough

### Step 1: List available benchmark suites

VibeCody ships with four suite types. Start by listing them:

```bash
vibecli repl
> /benchmark list
```

```
Available SWE-bench Suites:
  verified   300 tasks   Official human-verified subset
  pro        500 tasks   Extended professional-grade problems
  lite       100 tasks   Quick evaluation subset
  custom     —           User-defined task sets
```

Each suite contains real GitHub issues paired with their ground-truth patches, scored using Pass@1 (first-attempt success rate).

### Step 2: Run a benchmark

Run the Lite suite against your current provider to get a quick evaluation:

```bash
> /benchmark run --suite lite
```

```
Starting SWE-bench Lite (100 tasks)...
Provider: claude | Model: claude-sonnet-4-20250514
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━ 12/100
  ✓ django__django-11099   passed  (4.2s)
  ✓ django__django-11283   passed  (6.1s)
  ✗ astropy__astropy-6938   failed (8.3s)
  ...
```

Each task is sent to the agent loop. The agent reads the issue description, explores the repository, writes a patch, and VibeCody scores the result against the ground-truth diff.

You can also run directly from the shell:

```bash
vibecli benchmark run --suite lite --provider openai --model gpt-4o
```

### Step 3: View results

Once a run completes, inspect the results:

```bash
> /benchmark results
```

```
Run #3 — SWE-bench Lite — 2026-03-13T10:42:00Z
Provider: claude (claude-sonnet-4-20250514)

Pass@1: 42/100 (42.0%)

Difficulty Breakdown:
  Easy    28/40  (70.0%)
  Medium  11/35  (31.4%)
  Hard     3/25  (12.0%)

Top categories:
  Django      18/30 (60.0%)
  Flask        6/12 (50.0%)
  Requests     4/8  (50.0%)
  Astropy      2/10 (20.0%)
```

### Step 4: Compare models across runs

Run the same suite with a different provider, then compare:

```bash
> /benchmark run --suite lite --provider openai --model gpt-4o
# ... wait for completion ...

> /benchmark compare --runs 3,4
```

```
Model Comparison — SWE-bench Lite

                     Run #3          Run #4
Provider:            claude          openai
Model:               sonnet-4        gpt-4o
Pass@1:              42.0%           38.0%
Easy:                70.0%           65.0%
Medium:              31.4%           28.6%
Hard:                12.0%            8.0%
Avg Time/Task:       6.2s            7.8s
Total Time:          10m 20s         13m 00s

Tasks solved by #3 only:  8
Tasks solved by #4 only:  4
Tasks solved by both:     34
```

### Step 5: Run the Verified suite

For a comprehensive evaluation, use the Verified suite:

```bash
vibecli benchmark run --suite verified --provider claude
```

This takes longer (300 tasks) but provides the most reliable scoring since every task has been human-verified.

### Step 6: Create a custom suite

Define a custom suite from specific repositories:

```bash
> /benchmark run --suite custom --repos "django/django,pallets/flask" --max-tasks 20
```

Custom suites let you benchmark against codebases relevant to your team.

### Step 7: Export a Markdown report

Generate a shareable report:

```bash
> /benchmark export --run 3 --format markdown --output benchmark-report.md
```

```markdown
# SWE-bench Results — Run #3
Generated: 2026-03-13T10:55:00Z

## Summary
| Metric      | Value   |
|-------------|---------|
| Suite       | Lite    |
| Provider    | claude  |
| Pass@1      | 42.0%   |
| Total Tasks | 100     |
| Duration    | 10m 20s |

## Per-Task Results
| Task ID                  | Status | Time  |
|--------------------------|--------|-------|
| django__django-11099     | PASS   | 4.2s  |
| django__django-11283     | PASS   | 6.1s  |
| astropy__astropy-6938    | FAIL   | 8.3s  |
...
```

### Step 8: Use the SWE-bench panel in VibeUI

Open VibeUI and navigate to the **SWE-bench** panel from the AI sidebar. The panel has three tabs:

1. **Run** -- Select a suite, provider, and model. Click "Start Benchmark" to begin. A progress bar shows completion status with live pass/fail indicators.

2. **Results** -- Browse all completed runs. Click a run to see the full difficulty breakdown, per-task results, and timing data. Failed tasks show the expected vs. actual diff.

3. **Compare** -- Select two or more runs to see a side-by-side comparison table. Bar charts visualize Pass@1 differences across difficulty levels.

## Demo Recording

```json
{
  "meta": {
    "title": "SWE-bench Benchmarking",
    "description": "Run SWE-bench suites, compare model performance, and export reports.",
    "duration_seconds": 300,
    "version": "1.0.0"
  },
  "steps": [
    {
      "id": 1,
      "action": "repl",
      "commands": [
        { "input": "/benchmark list", "delay_ms": 2000 }
      ],
      "description": "List available benchmark suites"
    },
    {
      "id": 2,
      "action": "shell",
      "command": "vibecli benchmark run --suite lite --provider claude",
      "description": "Run SWE-bench Lite against Claude",
      "expected_output_contains": "Pass@1",
      "delay_ms": 60000,
      "typing_speed_ms": 40
    },
    {
      "id": 3,
      "action": "repl",
      "commands": [
        { "input": "/benchmark results", "delay_ms": 3000 }
      ],
      "description": "View benchmark results with difficulty breakdown"
    },
    {
      "id": 4,
      "action": "shell",
      "command": "vibecli benchmark run --suite lite --provider openai --model gpt-4o",
      "description": "Run the same suite against OpenAI for comparison",
      "expected_output_contains": "Pass@1",
      "delay_ms": 60000
    },
    {
      "id": 5,
      "action": "repl",
      "commands": [
        { "input": "/benchmark compare --runs 1,2", "delay_ms": 3000 }
      ],
      "description": "Compare two benchmark runs side by side"
    },
    {
      "id": 6,
      "action": "repl",
      "commands": [
        { "input": "/benchmark export --run 1 --format markdown --output report.md", "delay_ms": 2000 }
      ],
      "description": "Export results as a Markdown report"
    },
    {
      "id": 7,
      "action": "vibeui",
      "panel": "SWEBench",
      "tabs": ["Run", "Results", "Compare"],
      "description": "Navigate the SWE-bench panel in VibeUI: start a run, view results, compare models",
      "delay_ms": 5000
    }
  ]
}
```

## What's Next

- [Demo 26: QA Validation Pipeline](../qa-validation/) -- Validate code quality with 8 specialized QA agents
- [Demo 27: HTTP Playground](../http-playground/) -- Build and test API requests interactively
- [Demo 28: GraphQL Explorer](../graphql/) -- Introspect schemas and build queries
