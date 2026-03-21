---
layout: page
title: "Demo 34: Usage Metering"
permalink: /demos/34-usage-metering/
---


## Overview

Usage Metering provides full visibility into token consumption across agents, tasks, users, and providers. Set credit budgets with automatic alerts at 80%, 95%, and 100% thresholds. Generate reports by provider, model, or task type, and produce chargeback summaries for departments. Available through both CLI commands and the 4-tab VibeUI panel.

## Prerequisites

- VibeCLI installed and on your PATH
- At least one AI provider configured
- For team/project budgets: VibeCody running in multi-user mode (optional)
- For VibeUI: the desktop app running with the **Usage Metering** panel visible

## Step-by-Step Walkthrough

### 1. Check Current Usage Status

View a summary of token consumption and budget utilization.

**CLI:**

```bash
vibecli metering status
```

Example output:

```
Usage Metering Status
Period: March 2026 (Monthly)

Token Usage:
  Input tokens:    1,245,890
  Output tokens:     892,340
  Total tokens:    2,138,230

Budget: 5,000,000 tokens
Used:   2,138,230 (42.8%)
  [████████████████░░░░░░░░░░░░░░░░░░░░░░░░] 42.8%

Top Consumers:
  1. batch-ecommerce    892,000 tokens (41.7%)
  2. agent-refactor     456,200 tokens (21.3%)
  3. code-review        312,400 tokens (14.6%)

Alerts: None active
```

**VibeUI:**

Open the **Usage Metering** panel and select the **Dashboard** tab. The dashboard shows real-time token consumption, budget utilization bars, and a top-consumers ranking.

### 2. Configure Budgets

Set credit budgets scoped to a user, team, project, or globally. Specify the budget period and alert thresholds.

**CLI:**

```bash
vibecli metering budget set \
  --owner user:alice \
  --limit 2000000 \
  --period monthly \
  --alert-warning 80 \
  --alert-critical 95 \
  --alert-limit 100
```

Example output:

```
Budget configured:
  Owner:    user:alice
  Limit:    2,000,000 tokens
  Period:   Monthly (resets March 1)
  Alerts:
    Warning       at 80%  (1,600,000 tokens)
    Critical      at 95%  (1,900,000 tokens)
    LimitReached  at 100% (2,000,000 tokens)
```

**CLI (list all budgets):**

```bash
vibecli metering budget list
```

Example output:

```
Active Budgets:
  Owner             Limit        Period     Used      %
  user:alice        2,000,000    Monthly    892,000   44.6%
  team:platform     10,000,000   Monthly    3,200,000 32.0%
  project:ecommerce 5,000,000    Quarterly  2,138,230 42.8%
  global            50,000,000   Monthly    8,450,000 16.9%
```

**VibeUI:**

Switch to the **Budgets** tab. Click **New Budget** to create one. Select the owner type (User, Team, Project, Global), set the token limit, choose the period (Daily, Weekly, Monthly, Quarterly), and configure alert thresholds.

### 3. Budget Periods

Budgets support four period types:

| Period      | Reset Cycle                     |
|-------------|----------------------------------|
| **Daily**   | Resets at midnight UTC           |
| **Weekly**  | Resets every Monday at midnight  |
| **Monthly** | Resets on the 1st of each month  |
| **Quarterly** | Resets Jan 1, Apr 1, Jul 1, Oct 1 |

### 4. Alert Thresholds

Three alert levels trigger notifications when budget usage crosses a threshold:

| Alert Level      | Default Threshold | Behavior                                    |
|------------------|-------------------|---------------------------------------------|
| **Warning**      | 80%               | Notification sent; operations continue       |
| **Critical**     | 95%               | Urgent notification; review recommended      |
| **LimitReached** | 100%              | Operations paused; manual override available |

**CLI (view active alerts):**

```bash
vibecli metering alerts
```

Example output:

```
Active Alerts:
  [WARNING]  user:alice at 82.1% of monthly budget (1,642,000 / 2,000,000)
  [CRITICAL] project:legacy-migration at 96.3% of quarterly budget
```

**VibeUI:**

The **Alerts** tab shows all active and recent alerts with timestamps, owner details, and quick actions (increase budget, acknowledge, snooze).

### 5. Generate Reports

Produce detailed reports broken down by provider, model, task type, or time period.

**CLI:**

```bash
vibecli metering report --period 2026-03 --group-by provider
```

Example output:

```
Usage Report: March 2026 (by Provider)
  Provider     Input Tokens   Output Tokens  Total       Cost Est.
  Anthropic    845,200        612,400        1,457,600   $21.86
  OpenAI       312,000        198,340        510,340     $5.10
  Ollama       88,690         81,600         170,290     $0.00 (local)
  Total        1,245,890      892,340        2,138,230   $26.96
```

**CLI (report by model):**

```bash
vibecli metering report --period 2026-03 --group-by model
```

**CLI (report by task type):**

```bash
vibecli metering report --period 2026-03 --group-by task-type
```

**VibeUI:**

In the **Reports** tab, select the date range and grouping dimension. Click **Generate Report** to view tables and charts. Export as CSV or PDF.

### 6. Chargeback Generation

Generate chargeback reports for departmental billing.

**CLI:**

```bash
vibecli metering report --period 2026-03 --chargeback --group-by team
```

Example output:

```
Chargeback Report: March 2026
  Team          Tokens Used   Estimated Cost  Budget Remaining
  Platform      3,200,000     $48.00          6,800,000
  Data Eng      2,100,000     $31.50          7,900,000
  Mobile        1,450,000     $21.75          8,550,000
  DevOps          700,000     $10.50          9,300,000
  Total         7,450,000     $111.75
```

### 7. Per-Agent and Per-Task Tracking

Drill down into individual agent runs or tasks to see their exact token consumption.

**CLI:**

```bash
vibecli metering status --agent batch-ecommerce --detail
```

Example output:

```
Agent: batch-ecommerce
  Tasks completed: 42
  Total tokens: 892,000
    Architect:  124,000 (13.9%)
    Frontend:   198,000 (22.2%)
    Backend:    234,000 (26.2%)
    Testing:    156,000 (17.5%)
    Other:      180,000 (20.2%)
```

## Demo Recording JSON

```json
{
  "demo_id": "34-usage-metering",
  "title": "Usage Metering",
  "version": "1.0.0",
  "steps": [
    {
      "action": "cli_command",
      "command": "vibecli metering status",
      "description": "View current token usage and budget utilization"
    },
    {
      "action": "cli_command",
      "command": "vibecli metering budget set --owner user:alice --limit 2000000 --period monthly --alert-warning 80 --alert-critical 95 --alert-limit 100",
      "description": "Configure a monthly budget with alert thresholds"
    },
    {
      "action": "cli_command",
      "command": "vibecli metering budget list",
      "description": "List all active budgets"
    },
    {
      "action": "cli_command",
      "command": "vibecli metering alerts",
      "description": "View active budget alerts"
    },
    {
      "action": "cli_command",
      "command": "vibecli metering report --period 2026-03 --group-by provider",
      "description": "Generate a usage report grouped by provider"
    },
    {
      "action": "cli_command",
      "command": "vibecli metering report --period 2026-03 --chargeback --group-by team",
      "description": "Generate a chargeback report for departmental billing"
    },
    {
      "action": "cli_command",
      "command": "vibecli metering status --agent batch-ecommerce --detail",
      "description": "Drill into per-agent token consumption"
    },
    {
      "action": "vibeui_interaction",
      "panel": "UsageMetering",
      "tab": "Dashboard",
      "description": "View real-time usage dashboard with top consumers"
    },
    {
      "action": "vibeui_interaction",
      "panel": "UsageMetering",
      "tab": "Budgets",
      "description": "Create and manage credit budgets by owner"
    },
    {
      "action": "vibeui_interaction",
      "panel": "UsageMetering",
      "tab": "Reports",
      "description": "Generate and export usage reports"
    },
    {
      "action": "vibeui_interaction",
      "panel": "UsageMetering",
      "tab": "Alerts",
      "description": "Review and manage budget alerts"
    }
  ]
}
```

## What's Next

- [Demo 31: Batch Builder](../31-batch-builder/) -- See how batch runs consume tokens at scale
- [Demo 33: App Builder](../33-app-builder/) -- Track scaffolding costs with per-project metering
- [Demo 35: Compliance & Audit](../35-compliance/) -- Audit token usage for SOC 2 compliance
