---
layout: page
title: "Demo 43: Cost-Optimized Agent Routing"
permalink: /demos/43-cost-routing/
---


## Overview

Not every task needs the most expensive model. A simple variable rename does not require Claude Opus when Haiku can handle it in milliseconds for a fraction of the cost. VibeCody's cost-optimized routing engine classifies each task by complexity, selects the cheapest model that can handle it reliably, and tracks spending against configurable daily and monthly budgets. This demo shows how to inspect routing decisions, set budgets, and compare model costs across providers.

**Time to complete:** ~8 minutes

## Prerequisites

- VibeCLI 0.5.1 installed and on your PATH
- Two or more AI providers configured (to see routing in action)
- For VibeUI: the desktop app running with the **CostRouterPanel** visible

## How Cost Routing Works

The router classifies tasks into four complexity tiers and maps each to the cheapest viable model:

| Tier       | Examples                              | Default Model          | Cost/1K tokens |
|------------|---------------------------------------|------------------------|-----------------|
| **Trivial**| Rename variable, fix typo             | Claude Haiku / GPT-4o-mini | $0.00025    |
| **Simple** | Write a unit test, explain code       | Claude Sonnet / GPT-4o    | $0.003       |
| **Medium** | Refactor module, debug race condition | Claude Sonnet / GPT-4o    | $0.003       |
| **Complex**| Architect system, multi-file refactor | Claude Opus / GPT-4.5     | $0.015       |

## Step-by-Step Walkthrough

### Step 1: View current routing configuration

Start VibeCLI and inspect the active routing rules:

```bash
vibecli
```

```
> /route cost
```

```
Cost Router Configuration
═════════════════════════

Active strategy: cheapest-viable-model
Budget period:   daily
Daily budget:    $5.00 (default)
Spent today:     $0.00
Remaining:       $5.00

Tier Routing Table:
  Tier     │ Provider  │ Model                │ Input $/1K │ Output $/1K
  ─────────┼───────────┼──────────────────────┼────────────┼────────────
  Trivial  │ claude    │ claude-haiku-3        │ $0.00025   │ $0.00125
  Simple   │ claude    │ claude-sonnet-4       │ $0.00300   │ $0.01500
  Medium   │ claude    │ claude-sonnet-4       │ $0.00300   │ $0.01500
  Complex  │ claude    │ claude-opus-4         │ $0.01500   │ $0.07500

Fallback chain: claude → openai → ollama (free)
Budget enforcement: soft (warn at 80%, block at 100%)
```

### Step 2: Set a daily budget

Configure a daily spending limit:

```
> /route budget set 10.00
```

```
Daily budget updated: $5.00 → $10.00
Monthly projection at current usage: ~$0.00
Budget enforcement: soft (warn at 80% = $8.00, block at 100% = $10.00)
```

Set a strict budget that blocks requests when exceeded:

```
> /route budget enforce strict
```

```
Budget enforcement updated: soft → strict
When daily budget ($10.00) is exhausted, all requests will be blocked
until the next day (resets at midnight UTC).
Tip: Add [cost_router] fallback_to_ollama = true in config.toml
     to fall back to Ollama (free) instead of blocking.
```

### Step 3: Compare model costs across providers

View a side-by-side cost comparison for all configured providers:

```
> /route model compare
```

```
Model Cost Comparison (all configured providers)
═════════════════════════════════════════════════

Provider   │ Model                  │ Input $/1K │ Output $/1K │ Avg task cost │ Quality
───────────┼────────────────────────┼────────────┼─────────────┼───────────────┼─────────
claude     │ claude-haiku-3         │ $0.00025   │ $0.00125    │ $0.002        │ ★★★☆☆
claude     │ claude-sonnet-4        │ $0.00300   │ $0.01500    │ $0.024        │ ★★★★☆
claude     │ claude-opus-4          │ $0.01500   │ $0.07500    │ $0.112        │ ★★★★★
openai     │ gpt-4o-mini            │ $0.00015   │ $0.00060    │ $0.001        │ ★★★☆☆
openai     │ gpt-4o                 │ $0.00250   │ $0.01000    │ $0.016        │ ★★★★☆
ollama     │ llama3                 │ free       │ free        │ free          │ ★★★☆☆
ollama     │ codellama              │ free       │ free        │ free          │ ★★★☆☆
groq       │ llama-3.3-70b          │ $0.00059   │ $0.00079    │ $0.002        │ ★★★☆☆

Cheapest cloud model:  gpt-4o-mini ($0.001/task avg)
Cheapest overall:      ollama/llama3 (free, local)
Best quality/cost:     claude-sonnet-4 ($0.024/task, ★★★★☆)
```

### Step 4: Watch routing decisions in real time

Send a few tasks and observe which model the router selects:

```
> Rename the variable `x` to `count` in main.rs
```

```
[CostRouter] Task classified: Trivial
[CostRouter] Selected: claude/claude-haiku-3 ($0.00025/1K input)
[CostRouter] Reason: Simple rename, no semantic analysis needed

✓ Renamed `x` → `count` in src/main.rs (1 occurrence)

Cost: $0.001 | Budget remaining: $9.999
```

```
> Refactor the auth module to use async/await throughout
```

```
[CostRouter] Task classified: Medium
[CostRouter] Selected: claude/claude-sonnet-4 ($0.003/1K input)
[CostRouter] Reason: Multi-function refactor requiring semantic understanding

✓ Refactored 4 functions in src/auth.rs to async/await
  - validate_user() → async
  - create_session() → async
  - refresh_token() → async
  - revoke_session() → async

Cost: $0.031 | Budget remaining: $9.968
```

```
> Design a new microservice architecture for the payment system
```

```
[CostRouter] Task classified: Complex
[CostRouter] Selected: claude/claude-opus-4 ($0.015/1K input)
[CostRouter] Reason: Architecture design requiring broad reasoning

✓ Generated architecture document: docs/payment-service-architecture.md
  - 4 services: Gateway, Processor, Ledger, Notifications
  - Event-driven with Kafka
  - PostgreSQL per-service, Redis for caching

Cost: $0.142 | Budget remaining: $9.826
```

### Step 5: View spending summary

```
> /route spend
```

```
Spending Summary — 2026-03-29
═════════════════════════════

Total spent today:  $0.174 / $10.00 (1.7%)
Requests today:     3

By tier:
  Trivial:  1 request  │ $0.001  │ 0.6%
  Simple:   0 requests │ $0.000  │ 0.0%
  Medium:   1 request  │ $0.031  │ 17.8%
  Complex:  1 request  │ $0.142  │ 81.6%

By provider:
  claude (haiku):   $0.001
  claude (sonnet):  $0.031
  claude (opus):    $0.142

Projected daily spend at current rate: $0.174
Projected monthly spend: ~$5.22
```

### Step 6: Override routing for a specific task

Force a specific model when the router's choice is not what you want:

```
> /route override opus
> Explain what this regex does: ^(?:[a-z0-9!#$%&'*+/=?^_`{|}~-]+)@[a-z0-9-]+\.{1}[a-z]{2,}$
```

```
[CostRouter] Override active: forcing claude/claude-opus-4
[CostRouter] Original classification: Simple (would have used Sonnet)

This regex validates email addresses...
[detailed explanation]

Cost: $0.089 | Budget remaining: $9.737
Note: Override saved $0.065 less than default routing would have.
```

Clear the override:

```
> /route override clear
```

```
Override cleared. Resuming automatic cheapest-viable-model routing.
```

### Step 7: Use cost routing in VibeUI

In the VibeUI desktop app, open the **CostRouterPanel** from the AI sidebar. The panel provides:

- **Dashboard** -- Real-time spending chart with budget threshold lines
- **Routing Log** -- Every request with its tier classification, model selected, and cost
- **Budget Settings** -- Configure daily/monthly limits and enforcement mode
- **Model Comparison** -- Interactive table with sorting by cost, quality, and speed

## Demo Recording

```json
{
  "meta": {
    "title": "Cost-Optimized Agent Routing",
    "description": "Route tasks to the cheapest viable model and manage spending budgets.",
    "duration_seconds": 150,
    "version": "0.5.1"
  },
  "steps": [
    {
      "id": 1,
      "action": "repl",
      "commands": [
        { "input": "/route cost", "delay_ms": 2000 },
        { "input": "/route budget set 10.00", "delay_ms": 1500 },
        { "input": "/route model compare", "delay_ms": 3000 },
        { "input": "Rename the variable `x` to `count` in main.rs", "delay_ms": 5000 },
        { "input": "Refactor the auth module to use async/await", "delay_ms": 8000 },
        { "input": "/route spend", "delay_ms": 2000 },
        { "input": "/quit", "delay_ms": 500 }
      ],
      "description": "Configure routing, send tasks at different tiers, review spending"
    }
  ]
}
```

## What's Next

- [Demo 42: MCTS Code Repair](../42-mcts-repair/) -- Fix bugs with tree-search exploration
- [Demo 44: Visual Verification](../44-visual-verify/) -- Screenshot-based design compliance checking
- [Demo 34: Usage Metering](../34-usage-metering/) -- Detailed per-user and per-project credit tracking
