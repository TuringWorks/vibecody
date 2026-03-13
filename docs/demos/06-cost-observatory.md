---
layout: page
title: "Demo 6: Cost Observatory"
permalink: /demos/cost-observatory/
nav_order: 6
parent: Demos
---

# Demo 6: Cost Observatory

## Overview

The Cost Observatory gives you full visibility into your AI spending across all providers, sessions, and tasks. It tracks token usage in real time, shows per-provider cost breakdowns, lets you set budget alerts, and helps you optimize model selection by balancing quality against cost.

**Time to complete:** ~8 minutes

## Prerequisites

- VibeCLI installed and configured with one or more cloud AI providers (see [Demo 1: First Run](../01-first-run/))
- Some chat or agent history to generate cost data (run a few commands from [Demo 3](../03-multi-provider-chat/) or [Demo 4](../04-agent-loop/) first)
- Local providers like Ollama are free but still tracked for token counts

## Step-by-Step Walkthrough

### Step 1: View the token usage dashboard

**CLI:**

```bash
vibecli cost
```

```
VibeCody Cost Observatory
=========================

Today's Usage:
  Total tokens:    45,230
  Input tokens:    18,420
  Output tokens:   26,810
  Estimated cost:  $0.1247

This Week:
  Total tokens:    312,500
  Estimated cost:  $0.8934

This Month:
  Total tokens:    1,245,000
  Estimated cost:  $3.4521
```

**VibeUI:** Open the AI panel (`Cmd+J`) and click the "Cost" tab.

<!-- Screenshot placeholder: Cost Observatory dashboard -->
![Cost dashboard](../assets/screenshots/demo-06-dashboard.png)

### Step 2: Per-provider cost breakdown

See exactly how much each provider is costing you:

```bash
vibecli cost --by-provider
```

```
Cost by Provider (This Month)
+------------------+----------+----------+---------+----------+--------+
| Provider         | Input Tk | Output Tk| Total Tk| Cost     | % Total|
+------------------+----------+----------+---------+----------+--------+
| claude           |  245,000 |  312,000 | 557,000 | $1.8200  | 52.7%  |
| openai           |  180,000 |  198,000 | 378,000 | $1.2400  | 35.9%  |
| gemini           |   89,000 |  121,000 | 210,000 | $0.3200  |  9.3%  |
| groq             |   42,000 |   58,000 | 100,000 | $0.0721  |  2.1%  |
| ollama (local)   |   85,000 |  115,000 | 200,000 | $0.0000  |  0.0%  |
+------------------+----------+----------+---------+----------+--------+
| TOTAL            |  641,000 |  804,000 |1,445,000| $3.4521  |100.0%  |
+------------------+----------+----------+---------+----------+--------+
```

**Pricing reference:**

| Provider | Model | Input (per 1M tokens) | Output (per 1M tokens) |
|----------|-------|-----------------------|------------------------|
| Claude | claude-sonnet-4-20250514 | $3.00 | $15.00 |
| OpenAI | gpt-4o | $2.50 | $10.00 |
| Gemini | gemini-2.0-flash | $0.075 | $0.30 |
| Groq | llama-3.3-70b | $0.59 | $0.79 |
| DeepSeek | deepseek-chat | $0.14 | $0.28 |
| Ollama | any | Free | Free |

### Step 3: Per-session and per-task costs

Drill down to individual sessions to understand which tasks cost the most:

```bash
vibecli cost --by-session
```

```
Cost by Session (Last 7 Days)
+----+---------------------+------------------+--------+--------+---------+
| #  | Session             | Provider         | Tokens | Cost   | Duration|
+----+---------------------+------------------+--------+--------+---------+
| 1  | Refactor DB module  | claude           | 89,000 | $0.534 | 12m 30s |
| 2  | Fix auth bug        | claude           | 23,400 | $0.142 |  3m 15s |
| 3  | Write unit tests    | openai           | 45,200 | $0.271 |  8m 00s |
| 4  | Code review PR #42  | claude           | 34,100 | $0.204 |  5m 45s |
| 5  | Explain codebase    | gemini           | 67,800 | $0.095 |  4m 20s |
| 6  | Arena: LCS function | claude+openai    | 12,300 | $0.078 |  1m 10s |
| 7  | Chat: Rust tips     | ollama           | 15,600 | $0.000 |  2m 30s |
+----+---------------------+------------------+--------+--------+---------+
```

View details for a specific session:

```bash
vibecli cost --session "Refactor DB module"
```

```
Session: Refactor DB module
Provider: claude (claude-sonnet-4-20250514)
Started: 2026-03-13T09:15:00Z
Duration: 12m 30s

Token Breakdown:
  System prompt:      2,100 tokens   ($0.006)
  User messages:      8,200 tokens   ($0.025)
  Tool results:      24,300 tokens   ($0.073)
  AI responses:      54,400 tokens   ($0.816)
  Total:             89,000 tokens   ($0.534)

Agent Iterations: 14
  ReadFile calls:   8  (18,200 tokens in results)
  EditFile calls:   4  (3,100 tokens in results)
  Shell calls:      3  (3,000 tokens in results)
  Search calls:     2  (1,200 tokens in results)
```

### Step 4: Set budget alerts

Configure spending limits and alerts to avoid surprise bills:

```toml
# ~/.vibecli/config.toml

[cost]
daily_budget_usd = 5.00
weekly_budget_usd = 25.00
monthly_budget_usd = 50.00
alert_threshold_percent = 80  # Alert at 80% of budget
action_at_limit = "warn"      # "warn", "confirm", or "block"
```

When you approach a limit:

```
WARNING: You have used $4.12 of your $5.00 daily budget (82.4%).
Remaining: $0.88

To continue, use a cheaper model or switch to Ollama (free).
```

When `action_at_limit = "confirm"`:

```
BUDGET LIMIT: Daily budget of $5.00 reached.
Continue anyway? [y/N]:
```

When `action_at_limit = "block"`:

```
BLOCKED: Daily budget of $5.00 exceeded. No further API calls will be made.
Switch to a local provider (ollama) or increase your budget in ~/.vibecli/config.toml.
```

### Step 5: Historical cost trends

View cost trends over time:

```bash
vibecli cost --trend daily --days 14
```

```
Daily Cost Trend (Last 14 Days)
$2.00 |
      |          *
$1.50 |    *     *
      |    *  *  * *
$1.00 |  * *  *  * *
      |  * *  *  * *  *
$0.50 |* * *  *  * *  *  *     *
      |* * * ** ** ** ** **  * *
$0.00 +--+--+--+--+--+--+--+--+--+--+--+--+--+--
      Mar 1  3  5  7  9  11 13
```

```bash
vibecli cost --trend weekly --weeks 8
```

```
Weekly Cost Trend (Last 8 Weeks)
$12.00|
      |
$10.00|         *
      |      *  *
$ 8.00|   *  *  *
      |   *  *  *  *
$ 6.00|*  *  *  *  *  *
      |*  *  *  *  *  *  *  *
$ 4.00+--+--+--+--+--+--+--+--
      W1 W2 W3 W4 W5 W6 W7 W8
```

### Step 6: Optimize model selection by cost

VibeCody can recommend the most cost-effective model based on your Arena ratings and cost data:

```bash
vibecli cost --optimize
```

```
Model Cost-Effectiveness Analysis
+--------------------------+-------+--------+----------+------------------+
| Model                    | Elo   | $/1K tk| Quality  | Recommendation   |
+--------------------------+-------+--------+----------+------------------+
| gemini:gemini-2.0-flash  | 1198  | $0.0002| Good     | BEST VALUE       |
| groq:llama-3.3-70b       | 1117  | $0.0007| Fair     | Budget option    |
| deepseek:deepseek-chat   | 1156  | $0.0002| Good     | Cost-effective   |
| openai:gpt-4o            | 1245  | $0.0060| Great    | Premium          |
| claude:claude-sonnet-4   | 1284  | $0.0090| Excellent| Premium          |
| ollama:llama3 (local)    | 1050  | $0.0000| Fair     | Free (local GPU) |
+--------------------------+-------+--------+----------+------------------+

Suggestions:
  - For routine tasks: Switch from Claude ($0.009/1K) to Gemini Flash ($0.0002/1K)
    Estimated monthly savings: $2.40
  - For coding tasks: Claude has the highest Elo (1284) -- worth the premium
  - For explanations: GPT-4o offers the best quality-to-cost ratio
  - For exploration: Use Ollama locally at zero cost
```

### Step 7: Cost comparison by task type

```bash
vibecli cost --by-category
```

```
Cost by Task Category (This Month)
+------------------+--------+---------+-------------------+
| Category         | Tasks  | Cost    | Avg Cost/Task     |
+------------------+--------+---------+-------------------+
| Agent (coding)   |     12 | $1.8200 | $0.1517           |
| Chat             |     45 | $0.9800 | $0.0218           |
| Code review      |      8 | $0.4200 | $0.0525           |
| Arena matches    |     15 | $0.1821 | $0.0121           |
| Search           |     20 | $0.0500 | $0.0025           |
+------------------+--------+---------+-------------------+
```

## VibeUI Cost Observatory Panel

In VibeUI, the Cost panel provides an interactive dashboard with:

- **Real-time token counter** -- See tokens accumulate as you chat
- **Pie chart** -- Provider cost distribution at a glance
- **Line chart** -- Daily/weekly/monthly trends with zoom
- **Budget gauge** -- Visual progress toward budget limits
- **Session drill-down** -- Click any session to see per-message costs
- **Export button** -- Download cost reports as CSV or JSON

<!-- Screenshot placeholder: VibeUI Cost Observatory panel -->
![VibeUI Cost Observatory](../assets/screenshots/demo-06-vibeui-cost.png)

<!-- Screenshot placeholder: Cost pie chart by provider -->
![Cost pie chart](../assets/screenshots/demo-06-pie-chart.png)

<!-- Screenshot placeholder: Budget gauge showing usage -->
![Budget gauge](../assets/screenshots/demo-06-budget-gauge.png)

## Demo Recording

```json
{
  "meta": {
    "title": "Cost Observatory",
    "description": "Track AI spending across providers, set budget alerts, analyze cost trends, and optimize model selection.",
    "duration_seconds": 200,
    "version": "1.0.0"
  },
  "steps": [
    {
      "id": 1,
      "action": "shell",
      "command": "vibecli cost",
      "description": "View the cost dashboard summary",
      "delay_ms": 3000
    },
    {
      "id": 2,
      "action": "shell",
      "command": "vibecli cost --by-provider",
      "description": "View per-provider cost breakdown",
      "delay_ms": 3000
    },
    {
      "id": 3,
      "action": "shell",
      "command": "vibecli cost --by-session",
      "description": "View per-session costs",
      "delay_ms": 3000
    },
    {
      "id": 4,
      "action": "shell",
      "command": "vibecli cost --session \"Refactor DB module\"",
      "description": "Drill into a specific session's cost details",
      "delay_ms": 3000
    },
    {
      "id": 5,
      "action": "shell",
      "command": "vibecli cost --trend daily --days 14",
      "description": "View 14-day cost trend chart",
      "delay_ms": 3000
    },
    {
      "id": 6,
      "action": "shell",
      "command": "vibecli cost --optimize",
      "description": "Get cost optimization recommendations",
      "delay_ms": 4000
    },
    {
      "id": 7,
      "action": "shell",
      "command": "vibecli cost --by-category",
      "description": "View costs broken down by task category",
      "delay_ms": 3000
    },
    {
      "id": 8,
      "action": "write_file",
      "path": "~/.vibecli/config.toml",
      "content": "[cost]\ndaily_budget_usd = 5.00\nweekly_budget_usd = 25.00\nmonthly_budget_usd = 50.00\nalert_threshold_percent = 80\naction_at_limit = \"warn\"\n",
      "description": "Configure budget alerts",
      "delay_ms": 1000,
      "append": true
    },
    {
      "id": 9,
      "action": "shell",
      "command": "vibecli cost --export json | head -20",
      "description": "Export cost data as JSON",
      "delay_ms": 2000
    },
    {
      "id": 10,
      "action": "shell",
      "command": "vibecli chat --provider ollama \"This message is free because Ollama runs locally\"",
      "description": "Demonstrate zero-cost local inference",
      "delay_ms": 4000
    },
    {
      "id": 11,
      "action": "shell",
      "command": "vibecli cost",
      "description": "Verify the Ollama message cost was $0.00",
      "delay_ms": 2000
    }
  ]
}
```

## Cost-Saving Tips

1. **Use Ollama for exploration** -- Local models cost nothing. Use them for brainstorming and drafting before switching to a premium model for the final version.

2. **Gemini Flash for bulk tasks** -- At $0.075 per million input tokens, Gemini Flash is 40x cheaper than Claude for tasks where top-tier quality is not critical.

3. **Groq for speed + savings** -- Groq's inference is fast and cheap, making it ideal for quick questions and iterations.

4. **Set daily budgets** -- Even a $5/day budget prevents accidental spending spikes from long agent sessions.

5. **Use the Failover provider** -- Configure your chain from cheap to expensive: `["ollama", "groq", "gemini", "claude"]`. VibeCody tries the cheapest first.

6. **Review agent sessions** -- Agent tasks with many iterations can be expensive. Use `/context` to monitor token usage during long sessions and `/checkpoint rollback` to avoid wasted computation.

7. **Cache results** -- VibeCody caches identical prompts by default. Repeated questions cost nothing.

## What's Next

- [Demo 1: First Run](../01-first-run/) -- Revisit initial setup
- [Demo 3: Multi-Provider Chat](../03-multi-provider-chat/) -- Configure more providers for cost comparison
- [Demo 5: Model Arena](../05-model-arena/) -- Build quality ratings to complement cost data
