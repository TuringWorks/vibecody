---
layout: page
title: "Demo 31: Batch Builder"
permalink: /demos/31-batch-builder/
---


## Overview

The Batch Builder enables large-scale autonomous code generation across entire projects. Define a **BatchSpec** describing your target application, and VibeCody orchestrates up to 10 specialized agent roles through an architecture plan, module plan, and code generation pipeline. Runs can target 3M+ lines of output over 8-12 hours, with full pause/resume/cancel support and checkpoint intervals.

## Prerequisites

- VibeCLI installed and on your PATH
- At least one AI provider configured (e.g., `ANTHROPIC_API_KEY` or `OPENAI_API_KEY`)
- For VibeUI: the desktop app running with the **Batch Builder** panel visible

## Step-by-Step Walkthrough

### 1. Create a BatchSpec

Define the scope of your batch run. A BatchSpec includes target modules, estimated line counts, and which agent roles to activate.

**CLI:**

```bash
vibecli --batch create --name "ecommerce-platform" \
  --target-lines 500000 \
  --roles architect,frontend,backend,database,api,auth,testing,devops,docs,qa \
  --checkpoint-interval 30m
```

**VibeUI:**

Open the **Batch Builder** panel and select the **New Run** tab. Fill in the project name, target line count, and toggle the agent roles you want to include. Click **Create BatchSpec**.

### 2. Review the Architecture Plan

Once the BatchSpec is created, the Architect agent produces an architecture plan that maps out the high-level structure: services, data models, APIs, and dependencies.

**CLI:**

```bash
vibecli --batch status ecommerce-platform --show-plan
```

**VibeUI:**

In the **Monitor** tab, expand the **Architecture Plan** section to review the generated service graph and module breakdown.

### 3. Start the Batch Run

Launch the autonomous run. All 10 agent roles work in parallel according to the architecture and module plans.

**CLI:**

```bash
vibecli --batch start ecommerce-platform
```

**VibeUI:**

Click **Start Run** in the **Monitor** tab. The progress bar and per-agent status indicators will update in real time.

### 4. Monitor Progress

Track each agent's output, token usage, and completion percentage during the 8-12 hour run.

**CLI:**

```bash
vibecli --batch status ecommerce-platform
```

Example output:

```
Batch: ecommerce-platform
Status: Running (4h 12m elapsed)
Progress: 47% (235,000 / 500,000 lines)
Checkpoint: #8 (saved 2m ago)

Agent Roles:
  Architect    ████████████████████ 100%
  Frontend     ████████████░░░░░░░░  62%
  Backend      ██████████░░░░░░░░░░  51%
  Database     ████████████████████ 100%
  API          ████████████░░░░░░░░  58%
  Auth         ██████████████░░░░░░  72%
  Testing      ████████░░░░░░░░░░░░  38%
  DevOps       ██████████████████░░  90%
  Docs         ██████░░░░░░░░░░░░░░  30%
  QA           ████░░░░░░░░░░░░░░░░  22%
```

**VibeUI:**

The **Monitor** tab shows a live dashboard with per-agent progress bars, a timeline view of checkpoints, and token consumption graphs.

### 5. Pause and Resume

Interrupt a run without losing progress. The system saves a checkpoint before pausing.

**CLI:**

```bash
vibecli --batch pause ecommerce-platform
# Later...
vibecli --batch resume ecommerce-platform
```

**VibeUI:**

Click **Pause** in the Monitor tab toolbar. The run saves a checkpoint and halts. Click **Resume** when ready to continue.

### 6. QA Review

After the run completes (or at any checkpoint), review the generated code through the integrated QA pipeline.

**CLI:**

```bash
vibecli --batch status ecommerce-platform --qa-summary
```

**VibeUI:**

Switch to the **QA Review** tab to see severity-weighted scores, auto-fix suggestions, and cross-validation confidence ratings for each module.

### 7. Cancel a Run

If a run is no longer needed, cancel it cleanly.

**CLI:**

```bash
vibecli --batch cancel ecommerce-platform
```

### 8. View History

Browse past batch runs, their specs, and outcomes.

**CLI:**

```bash
vibecli --batch status --history
```

**VibeUI:**

The **History** tab lists all previous runs with their specs, durations, line counts, and QA scores.

## The 10 Agent Roles

| Role       | Responsibility                                      |
|------------|-----------------------------------------------------|
| Architect  | High-level design, service boundaries, data models  |
| Frontend   | UI components, pages, styling, client state         |
| Backend    | Business logic, service layer, middleware            |
| Database   | Schema design, migrations, queries, indexes         |
| API        | REST/GraphQL endpoints, request validation           |
| Auth       | Authentication, authorization, RBAC, session mgmt   |
| Testing    | Unit tests, integration tests, E2E tests            |
| DevOps     | CI/CD pipelines, Docker, K8s manifests, IaC         |
| Docs       | API docs, README files, architecture decision records|
| QA         | Code review, quality scoring, auto-fix suggestions  |

## Pipeline Flow

```
BatchSpec → Architect → ArchitecturePlan → ModulePlan → [10 Agents in parallel] → Code Output
                                                              ↓
                                                    Checkpoint saved every N minutes
                                                              ↓
                                                      QA Review & Scoring
```

## Demo Recording JSON

```json
{
  "demo_id": "31-batch-builder",
  "title": "Batch Builder",
  "version": "1.0.0",
  "steps": [
    {
      "action": "cli_command",
      "command": "vibecli --batch create --name demo-app --target-lines 100000 --roles architect,frontend,backend,testing --checkpoint-interval 15m",
      "description": "Create a BatchSpec with 4 agent roles"
    },
    {
      "action": "cli_command",
      "command": "vibecli --batch start demo-app",
      "description": "Start the autonomous batch run"
    },
    {
      "action": "cli_command",
      "command": "vibecli --batch status demo-app",
      "description": "Monitor progress and per-agent status"
    },
    {
      "action": "cli_command",
      "command": "vibecli --batch pause demo-app",
      "description": "Pause the run with a checkpoint"
    },
    {
      "action": "cli_command",
      "command": "vibecli --batch resume demo-app",
      "description": "Resume from the last checkpoint"
    },
    {
      "action": "cli_command",
      "command": "vibecli --batch status demo-app --qa-summary",
      "description": "Review QA scores after completion"
    },
    {
      "action": "vibeui_interaction",
      "panel": "BatchBuilder",
      "tab": "New Run",
      "description": "Create a BatchSpec using the GUI form"
    },
    {
      "action": "vibeui_interaction",
      "panel": "BatchBuilder",
      "tab": "Monitor",
      "description": "Watch live progress dashboard with per-agent bars"
    },
    {
      "action": "vibeui_interaction",
      "panel": "BatchBuilder",
      "tab": "QA Review",
      "description": "Review auto-fix suggestions and quality scores"
    },
    {
      "action": "vibeui_interaction",
      "panel": "BatchBuilder",
      "tab": "History",
      "description": "Browse past batch runs and outcomes"
    }
  ]
}
```

## What's Next

- [Demo 32: Legacy Migration](../32-legacy-migration/) -- Migrate legacy codebases with the batch pipeline
- [Demo 33: App Builder](../33-app-builder/) -- Scaffold full applications from natural language descriptions
- [Demo 34: Usage Metering](../34-usage-metering/) -- Track token usage and budgets across batch runs
