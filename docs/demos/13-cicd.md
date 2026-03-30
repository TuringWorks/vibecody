---
layout: page
title: "Demo 13: CI/CD Pipeline"
permalink: /demos/13-cicd/
nav_order: 13
parent: Demos
---


## Overview

This demo covers VibeCody's CI/CD integration, which brings pipeline visibility directly into your development workflow. You will learn how to monitor GitHub Actions pipelines, view build logs, trigger workflows, and use the GH Actions agent for automated workflow debugging -- all from the CLI and VibeUI.

**Time to complete:** ~15 minutes

## Prerequisites

- VibeCLI installed and configured ([Demo 1](../first-run/))
- A GitHub repository with at least one GitHub Actions workflow
- A GitHub personal access token with `repo` and `workflow` scopes
- (Optional) VibeUI for the desktop panel experience

### GitHub Token Setup

```bash
# Set your GitHub token
export GITHUB_TOKEN="ghp_..."

# Or add to config.toml
cat >> ~/.vibecli/config.toml << 'EOF'

[github]
token = "ghp_..."
EOF
```

## Step-by-Step Walkthrough

### Step 1: Check pipeline status

Use the `/cicd status` REPL command to view the current state of all pipelines in your repository.

```bash
vibecli
> /cicd status
```

Expected output:

```
CI/CD Pipeline Status (myorg/myrepo)
=====================================
  #142  build-and-test   main     completed   success    2m 34s   3 min ago
  #141  deploy-staging   main     completed   success    4m 12s   15 min ago
  #140  build-and-test   fix/auth in_progress running    1m 02s   now
  #139  lint-check       fix/auth completed   failure    0m 48s   22 min ago

Active: 1 | Passed: 2 | Failed: 1 | Total runs shown: 4
```

### Step 2: View build logs

Drill into a specific run to see detailed logs.

```bash
> /cicd logs 139
```

Expected output:

```
Run #139: lint-check (fix/auth)
================================
Status: failure
Trigger: push
Duration: 48s

Job: lint (ubuntu-latest)
  Step 1: Checkout            passed   2s
  Step 2: Setup Rust          passed   15s
  Step 3: cargo clippy        FAILED   31s

--- Error Output ---
error: unused variable `config`
  --> src/auth.rs:42:9
   |
42 |     let config = load_config()?;
   |         ^^^^^^ help: if this is intentional, prefix with underscore: `_config`

error: could not compile `myproject` (lib) due to 1 previous error
```

### Step 3: Trigger a workflow manually

Re-run a failed workflow or trigger a workflow dispatch.

```bash
# Re-run a specific workflow
> /cicd trigger --rerun 139

# Trigger a workflow_dispatch event
> /cicd trigger --workflow deploy.yml --ref main --input environment=staging
```

Expected output:

```
Triggered workflow run #143: lint-check on fix/auth
Watch progress: /cicd logs 143 --follow
```

### Step 4: View pipeline history

Review recent CI/CD activity for a branch or the entire repository.

```bash
# History for the current branch
> /cicd history

# History for a specific branch
> /cicd history --branch main --limit 20

# Filter by status
> /cicd history --status failure --limit 10
```

Expected output:

```
Pipeline History (last 10 runs, branch: main)
==============================================
  #142  build-and-test   success   2m 34s   2026-03-13 09:41
  #141  deploy-staging   success   4m 12s   2026-03-13 09:26
  #138  build-and-test   success   2m 28s   2026-03-13 08:55
  #135  build-and-test   failure   1m 52s   2026-03-12 17:30
  #134  deploy-staging   success   4m 05s   2026-03-12 17:15
  ...

Success rate: 80% (8/10)  |  Avg duration: 2m 48s
```

### Step 5: Monitor pipelines with alerts

Enable real-time monitoring that notifies you when builds finish or fail.

```bash
# Watch a running pipeline
> /cicd logs 140 --follow

# Enable desktop notifications for failures
> /cicd monitor --notify-on failure
```

When a pipeline fails, VibeCody sends a system notification and displays a summary in the REPL:

```
[CI ALERT] Run #144 build-and-test FAILED on fix/auth (1m 22s)
  cargo test: 2 tests failed (test_login_flow, test_token_refresh)
  Use /cicd logs 144 to see details
```

### Step 6: GH Actions agent for automated debugging

When a workflow fails, ask the GH Actions agent to analyze and suggest fixes.

```bash
> /cicd debug 139
```

The agent reads the workflow YAML, inspects the error logs, and proposes a fix:

```
Analyzing run #139 (lint-check)...

Root cause: Unused variable `config` in src/auth.rs:42
The variable is assigned but never used in the function body.

Suggested fix (src/auth.rs):
  - let config = load_config()?;
  + let _config = load_config()?;

Or, if the variable should be used, check that subsequent code
references `config` as intended.

Apply fix? [y/n]:
```

You can also ask the agent to fix workflow YAML issues:

```bash
> /cicd debug-workflow .github/workflows/ci.yml
```

```
Analyzing .github/workflows/ci.yml...

Issues found:
  1. Line 23: uses: actions/checkout@v2 -- outdated, recommend v4
  2. Line 31: Missing `cache: true` on actions/setup-node -- builds are slow
  3. Line 45: `continue-on-error: true` suppresses real failures

Suggested .github/workflows/ci.yml patch:
  - uses: actions/checkout@v2
  + uses: actions/checkout@v4
  ...

Apply patch? [y/n]:
```

### Step 7: CI review bot for PR checks

The CI review bot automatically comments on pull requests when checks fail, providing actionable summaries.

```bash
# Enable CI review bot for a repository
> /cicd review-bot enable --repo myorg/myrepo

# View bot configuration
> /cicd review-bot config
```

```
CI Review Bot Configuration
============================
Repository:     myorg/myrepo
Status:         enabled
Auto-comment:   on failure
Analyze depth:  full (logs + code)
Notify:         PR author + reviewers
```

When a PR check fails, the bot posts a GitHub comment like:

```markdown
## CI Failure Analysis

**Run:** #145 build-and-test | **Status:** failure | **Duration:** 2m 10s

### Failed Step: cargo test

2 tests failed:
- `test_login_flow`: assertion failed at auth.rs:87 — expected `Ok`, got `Err(InvalidToken)`
- `test_token_refresh`: timeout after 30s — likely missing mock for token endpoint

### Suggested Fix
The `InvalidToken` error suggests the test fixture token expired.
Update `tests/fixtures/test_token.json` with a fresh token or use
a time-independent mock.
```

### Step 8: Use the CI/CD panel in VibeUI

Open VibeUI and navigate to the **CI/CD** panel in the left sidebar.

```bash
cd vibeui && npm run tauri dev
```

The CI/CD panel provides four views:

1. **Pipeline Status** -- Live view of all workflow runs with status badges, durations, and quick-action buttons (re-run, view logs, cancel).

2. **Logs Viewer** -- Streaming log viewer with ANSI color support, search, and jump-to-error. Click any failed step to see its output.

3. **History** -- Timeline chart of pipeline runs with success/failure trends, average duration, and flakiness detection.

4. **Actions Agent** -- Interactive debugging panel where you paste a run URL or ID and the GH Actions agent analyzes the failure, suggests fixes, and can apply patches directly.

### Step 9: Configure pipeline alerts in config.toml

```toml
[cicd]
github_token = "ghp_..."
poll_interval_seconds = 30
notify_on = ["failure", "cancelled"]

[cicd.review_bot]
enabled = true
auto_comment = true
analyze_depth = "full"

[cicd.alerts]
desktop_notifications = true
slack_webhook = "https://hooks.slack.com/services/T.../B.../..."
```

## Demo Recording

```json
{
  "meta": {
    "title": "CI/CD Pipeline Integration",
    "description": "Monitor GitHub Actions pipelines, view logs, debug failures, and use the CI review bot from VibeCLI and VibeUI.",
    "duration_seconds": 300,
    "version": "1.0.0"
  },
  "steps": [
    {
      "id": 1,
      "action": "shell",
      "command": "export GITHUB_TOKEN=\"ghp_demo_token\"",
      "description": "Set GitHub token for CI/CD access",
      "delay_ms": 500
    },
    {
      "id": 2,
      "action": "repl",
      "commands": [
        { "input": "/cicd status", "delay_ms": 3000 }
      ],
      "description": "View current pipeline status for all workflows"
    },
    {
      "id": 3,
      "action": "repl",
      "commands": [
        { "input": "/cicd logs 139", "delay_ms": 4000 }
      ],
      "description": "Inspect logs for a failed lint-check run"
    },
    {
      "id": 4,
      "action": "repl",
      "commands": [
        { "input": "/cicd history --branch main --limit 10", "delay_ms": 3000 }
      ],
      "description": "Review pipeline history for the main branch"
    },
    {
      "id": 5,
      "action": "repl",
      "commands": [
        { "input": "/cicd trigger --rerun 139", "delay_ms": 2000 }
      ],
      "description": "Re-trigger a failed workflow run"
    },
    {
      "id": 6,
      "action": "repl",
      "commands": [
        { "input": "/cicd debug 139", "delay_ms": 5000 }
      ],
      "description": "Use the GH Actions agent to analyze the failure and suggest a fix"
    },
    {
      "id": 7,
      "action": "repl",
      "commands": [
        { "input": "/cicd review-bot enable --repo myorg/myrepo", "delay_ms": 2000 },
        { "input": "/cicd review-bot config", "delay_ms": 1500 }
      ],
      "description": "Enable and configure the CI review bot for automatic PR comments"
    },
    {
      "id": 8,
      "action": "shell",
      "command": "cd vibeui && npm run tauri dev",
      "description": "Launch VibeUI to explore the CI/CD panel",
      "delay_ms": 8000
    },
    {
      "id": 9,
      "action": "Navigate",
      "target": "panel://cicd",
      "description": "Open the CI/CD panel in VibeUI"
    },
    {
      "id": 10,
      "action": "Click",
      "target": ".pipeline-run[data-id='139']",
      "description": "Click a failed run to see detailed log output"
    },
    {
      "id": 11,
      "action": "Screenshot",
      "label": "cicd-panel-logs",
      "description": "Capture the CI/CD panel showing streamed build logs"
    },
    {
      "id": 12,
      "action": "Click",
      "target": ".tab-actions-agent",
      "description": "Switch to the Actions Agent tab for interactive debugging"
    },
    {
      "id": 13,
      "action": "Type",
      "target": "#run-id-input",
      "value": "139",
      "description": "Enter the run ID for the agent to analyze"
    },
    {
      "id": 14,
      "action": "Click",
      "target": ".btn-analyze",
      "description": "Start the GH Actions agent analysis"
    },
    {
      "id": 15,
      "action": "Wait",
      "duration_ms": 5000,
      "description": "Wait for the agent to complete its analysis"
    },
    {
      "id": 16,
      "action": "Screenshot",
      "label": "cicd-agent-fix",
      "description": "Capture the agent's suggested fix with Apply button"
    }
  ]
}
```

## Troubleshooting

| Problem | Solution |
|---------|----------|
| `GitHub token missing` | Set `GITHUB_TOKEN` env var or add to `[github]` in config.toml |
| `403 Forbidden` on workflow trigger | Your token needs the `workflow` scope -- regenerate with correct permissions |
| No runs shown | Ensure you are in a directory with a `.git` remote pointing to GitHub |
| Review bot not commenting | Check that the token has `pull_requests: write` permission |
| Logs truncated | GitHub retains logs for 90 days -- older runs may not have logs available |

## What's Next

- [Demo 14: Cloud Provider Integration](../14-cloud-providers/) -- Scan for AWS/GCP/Azure usage and generate IaC
- [Demo 12: Kubernetes Operations](../12-kubernetes/) -- Deploy and monitor K8s workloads
- [Demo 9: Autofix & Diagnostics](../09-autofix/) -- Automatically fix code issues detected by CI
