---
layout: page
title: "Demo 26: QA Validation Pipeline"
permalink: /demos/qa-validation/
nav_order: 26
parent: Demos
---

# Demo 26: QA Validation Pipeline

## Overview

This demo walks you through VibeCody's QA validation pipeline, which uses 8 specialized QA agent types to validate code changes through multiple rounds of automated review. Each agent focuses on a different quality dimension, and cross-validation between agents produces a confidence score with severity-weighted results and auto-fix suggestions.

**Time to complete:** ~10 minutes

## Prerequisites

- VibeCody installed and configured with an AI provider
- A project directory with source code to validate
- For VibeUI: the desktop app running (`npm run tauri dev`)

## Step-by-Step Walkthrough

### Step 1: Understand the 8 QA agent types

VibeCody's QA pipeline deploys these specialized agents:

| Agent | Focus | What It Checks |
|-------|-------|-----------------|
| **Linter** | Style & formatting | Code style violations, naming conventions, unused imports |
| **Type Checker** | Type safety | Type mismatches, missing annotations, unsafe casts |
| **Test Runner** | Test coverage | Failing tests, uncovered branches, missing edge cases |
| **Security** | Vulnerabilities | SQL injection, XSS, path traversal, hardcoded secrets |
| **Performance** | Efficiency | O(n^2) loops, unnecessary allocations, blocking I/O |
| **API Contract** | Interface stability | Breaking changes, missing docs, backward compatibility |
| **Dependency** | Supply chain | Outdated packages, known CVEs, license conflicts |
| **Architecture** | Design quality | Circular dependencies, layer violations, coupling metrics |

### Step 2: Run a basic QA validation

Start the REPL and validate the current project:

```bash
vibecli repl
> /qavalidate run
```

```
QA Validation Pipeline — Starting
Target: ./src/ (47 files, 3,200 lines)
Provider: claude

Round 1/3 ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━ 8/8 agents
  ✓ Linter         3 issues (2 low, 1 medium)
  ✓ Type Checker   1 issue  (1 medium)
  ✓ Test Runner    2 issues (1 high, 1 medium)
  ✓ Security       1 issue  (1 high)
  ✓ Performance    0 issues
  ✓ API Contract   0 issues
  ✓ Dependency     2 issues (1 medium, 1 low)
  ✓ Architecture   1 issue  (1 low)

Round 2/3 — Cross-validation...
  Confirming findings across agents...

Round 3/3 — Generating fixes...

Summary:
  Total Issues: 10
  Confidence:   87.3%
  Severity:     2 high, 3 medium, 5 low
  Auto-fixable: 6/10
```

### Step 3: Inspect findings by severity

View detailed findings sorted by severity:

```bash
> /qavalidate results --sort severity
```

```
HIGH SEVERITY (2):
  [SEC-001] Hardcoded API key in src/config.rs:42
    Agent: Security | Confidence: 94%
    Fix: Move to environment variable or secrets manager
    Auto-fix available: Yes

  [TST-001] Uncovered error path in src/handler.rs:88
    Agent: Test Runner | Confidence: 91%
    Fix: Add test for ConnectionError branch
    Auto-fix available: Yes

MEDIUM SEVERITY (3):
  [TYP-001] Implicit any type in src/utils.ts:15
    Agent: Type Checker | Confidence: 89%
    ...

  [LNT-001] Unused import in src/main.rs:3
    Agent: Linter | Confidence: 98%
    ...

  [DEP-001] Outdated dependency: serde 1.0.150 → 1.0.197
    Agent: Dependency | Confidence: 95%
    ...

LOW SEVERITY (5):
  ...
```

### Step 4: Apply auto-fixes

Let VibeCody fix the issues it can resolve automatically:

```bash
> /qavalidate fix --severity high,medium
```

```
Applying auto-fixes...
  ✓ [SEC-001] Moved API key to env var in src/config.rs
  ✓ [TST-001] Generated test for error path in tests/handler_test.rs
  ✓ [TYP-001] Added explicit type annotation in src/utils.ts
  ✓ [LNT-001] Removed unused import in src/main.rs
  ✗ [DEP-001] Skipped — dependency update requires manual review

Fixed: 4/5 issues
```

### Step 5: Cross-validation details

See how agents corroborated each other's findings:

```bash
> /qavalidate cross-validation
```

```
Cross-Validation Matrix:
  SEC-001 (Security) confirmed by: Linter (pattern match), Architecture (data flow)
  TST-001 (Test Runner) confirmed by: Security (error handling gap)
  TYP-001 (Type Checker) confirmed by: Linter (implicit any rule)

Confidence adjustments:
  SEC-001: 88% → 94% (+6% from 2 confirmations)
  TST-001: 85% → 91% (+6% from 1 confirmation)
  False positive rate: 2.1%
```

### Step 6: Run batch QA for a large codebase

For larger projects, use batch mode with parallel agent execution:

```bash
vibecli qavalidate --batch --path ./monorepo --parallel 4
```

```
Batch QA — 12 packages, 847 files
Running 4 agents in parallel per package...

Package            Issues  High  Medium  Low   Score
────────────────────────────────────────────────────
api-server          8       1     3       4    72.5
web-frontend        5       0     2       3    85.0
shared-lib          2       0     0       2    95.0
auth-service       11       3     4       4    58.2
...

Overall Score: 76.4 (weighted by severity)
```

### Step 7: Configure validation rules

Customize which agents run and their sensitivity:

```bash
> /qavalidate config
```

Edit the validation section in `~/.vibecli/config.toml`:

```toml
[qa]
rounds = 3
agents = ["linter", "type_checker", "test_runner", "security", "performance"]
min_confidence = 80
auto_fix = true
severity_weights = { high = 10, medium = 5, low = 1 }
```

### Step 8: Use the QA panel in VibeUI

Open VibeUI and navigate to the **QA Validation** panel. The workflow is:

1. Select the target directory or changed files from the file tree.
2. Choose which QA agents to enable using the agent toggles.
3. Click **Run Validation** to start the pipeline.
4. Results appear in a sortable table grouped by severity. Click any finding to see the full agent report, cross-validation data, and suggested fix.
5. Use the **Auto-Fix** button to apply all safe fixes with one click. A diff preview shows exactly what will change before you confirm.

## Demo Recording

```json
{
  "meta": {
    "title": "QA Validation Pipeline",
    "description": "Validate code with 8 QA agents, cross-validate findings, and apply auto-fixes.",
    "duration_seconds": 240,
    "version": "1.0.0"
  },
  "steps": [
    {
      "id": 1,
      "action": "repl",
      "commands": [
        { "input": "/qavalidate run", "delay_ms": 15000 }
      ],
      "description": "Run full QA validation pipeline with all 8 agents"
    },
    {
      "id": 2,
      "action": "repl",
      "commands": [
        { "input": "/qavalidate results --sort severity", "delay_ms": 3000 }
      ],
      "description": "View findings sorted by severity"
    },
    {
      "id": 3,
      "action": "repl",
      "commands": [
        { "input": "/qavalidate cross-validation", "delay_ms": 2000 }
      ],
      "description": "Inspect cross-validation confirmations between agents"
    },
    {
      "id": 4,
      "action": "repl",
      "commands": [
        { "input": "/qavalidate fix --severity high,medium", "delay_ms": 5000 }
      ],
      "description": "Apply auto-fixes for high and medium severity issues"
    },
    {
      "id": 5,
      "action": "shell",
      "command": "vibecli qavalidate --batch --path . --parallel 4",
      "description": "Run batch QA across the entire project",
      "expected_output_contains": "Overall Score",
      "delay_ms": 30000
    },
    {
      "id": 6,
      "action": "vibeui",
      "panel": "QAValidation",
      "actions": ["select_agents", "run_validation", "view_results", "apply_fixes"],
      "description": "Use the QA Validation panel in VibeUI to run agents and review findings",
      "delay_ms": 5000
    }
  ]
}
```

## What's Next

- [Demo 27: HTTP Playground](../http-playground/) -- Build and test API requests interactively
- [Demo 28: GraphQL Explorer](../graphql/) -- Introspect schemas and build queries
- [Demo 29: Regex & Encoding Tools](../regex-encoding/) -- Pattern testing, JWT decoding, and data conversion
