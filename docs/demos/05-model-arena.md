---
layout: page
title: "Demo 5: Model Arena"
permalink: /demos/model-arena/
nav_order: 5
parent: Demos
---


## Overview

The Model Arena lets you compare AI models head-to-head by sending the same prompt to multiple models simultaneously and evaluating the results side by side. It includes an Elo ranking system that learns your preferences over time, helping you choose the best model for each task type.

**Time to complete:** ~10 minutes

## Prerequisites

- VibeCLI installed and configured (see [Demo 1: First Run](../first-run/))
- API keys for at least two AI providers (see [Demo 3: Multi-Provider Chat](../multi-provider-chat/))
- VibeUI installed for the graphical Arena panel (optional -- CLI arena also available)

## Step-by-Step Walkthrough

### Step 1: Open the Arena in VibeUI

Launch VibeUI and navigate to the Arena panel:

1. Open VibeUI: `cd vibeui && npm run tauri dev`
2. Press `Cmd+J` to open the AI sidebar
3. Click the "Arena" tab in the panel tab bar

<!-- Screenshot placeholder: Arena panel initial view -->

Alternatively, use the CLI:

```bash
vibecli arena
```

### Step 2: Select models for comparison

In the Arena panel, choose which models to compare. You can select 2-4 models at a time.

**VibeUI:** Use the model selector dropdowns at the top of the Arena panel.

**CLI:**

```bash
vibecli arena --models claude:claude-sonnet-4-20250514,openai:gpt-4o,gemini:gemini-2.0-flash,groq:llama-3.3-70b-versatile
```

Example configurations:

```bash
# Compare flagship models
vibecli arena --models claude:claude-sonnet-4-20250514,openai:gpt-4o,gemini:gemini-2.0-pro

# Compare fast/cheap models
vibecli arena --models groq:llama-3.3-70b-versatile,cerebras:llama3.1-70b,ollama:llama3

# Compare coding models
vibecli arena --models claude:claude-sonnet-4-20250514,deepseek:deepseek-coder,mistral:codestral-latest
```

<!-- Screenshot placeholder: Model selector with 3 models chosen -->

### Step 3: Send a prompt to all models

Type your prompt in the Arena input field. The same prompt is sent to all selected models simultaneously.

**VibeUI:** Type in the input field and click "Send to All" or press `Cmd+Enter`.

**CLI:**

```bash
vibecli arena --models claude:claude-sonnet-4-20250514,openai:gpt-4o \
  --prompt "Write a Rust function that finds the longest common subsequence of two strings"
```

All models receive identical prompts with identical system messages to ensure a fair comparison.

```
Sending to 3 models...
  [claude]  Streaming... (1.2s)
  [openai]  Streaming... (0.9s)
  [gemini]  Streaming... (1.4s)
```

### Step 4: View responses side by side

Responses appear in parallel columns (VibeUI) or sequentially labeled sections (CLI).

**VibeUI layout:**

```
+-------------------+-------------------+-------------------+
| Claude Sonnet 4   | GPT-4o            | Gemini 2.0 Flash  |
|                   |                   |                   |
| fn lcs(a: &str,   | fn longest_common | pub fn lcs(       |
|     b: &str)      |     _subsequence( |     s1: &str,     |
|     -> String {   |     s1: &str,     |     s2: &str,     |
|   let m = a.len();|     s2: &str,     | ) -> String {     |
|   ...             |     ...           |     ...           |
|                   |                   |                   |
| Time: 2.3s        | Time: 1.8s        | Time: 2.1s        |
| Tokens: 342       | Tokens: 289       | Tokens: 315       |
| Cost: $0.0034     | Cost: $0.0029     | Cost: $0.0008     |
+-------------------+-------------------+-------------------+
```

<!-- Screenshot placeholder: Side-by-side responses in Arena -->

Each response card shows:

- The model name and provider
- Response time (time to first token and total)
- Token count (input + output)
- Estimated cost
- The full response with syntax highlighting

### Step 5: Rate responses

After reviewing the responses, rate them to build your preference profile.

**VibeUI:** Click the thumbs up/down buttons on each response card, or select a winner.

**CLI:**

```bash
# After responses are displayed:
Rate the responses:
  [1] Claude Sonnet 4   - Vote: (w)in / (l)ose / (t)ie / (s)kip
  [2] GPT-4o            - Vote: (w)in / (l)ose / (t)ie / (s)kip
  [3] Gemini 2.0 Flash  - Vote: (w)in / (l)ose / (t)ie / (s)kip

> 1w 2l 3t
Recorded: Claude wins, GPT-4o loses, Gemini ties.
```

You can also rate on specific criteria:

```bash
> /rate accuracy 1>2>3
> /rate speed 3>1>2
> /rate style 2>1>3
```

<!-- Screenshot placeholder: Rating interface with thumbs up/down -->

### Step 6: Elo ranking system

VibeCody maintains an Elo rating for each model based on your votes. View the leaderboard:

```bash
vibecli arena --leaderboard
```

```
Model Arena Leaderboard (47 matches)
+----+---------------------------+------+------+------+------+--------+
| #  | Model                     | Elo  | Wins | Loss | Ties | Win %  |
+----+---------------------------+------+------+------+------+--------+
| 1  | claude:claude-sonnet-4    | 1284 |   18 |    6 |    4 | 64.3%  |
| 2  | openai:gpt-4o             | 1245 |   15 |    8 |    5 | 53.6%  |
| 3  | gemini:gemini-2.0-pro     | 1198 |   12 |   10 |    6 | 42.9%  |
| 4  | deepseek:deepseek-coder   | 1156 |    9 |   12 |    3 | 37.5%  |
| 5  | groq:llama-3.3-70b        | 1117 |    7 |   14 |    2 | 30.4%  |
+----+---------------------------+------+------+------+------+--------+
```

The Elo system uses the standard chess rating algorithm:

- New models start at 1200
- Winning against a higher-rated model gives more points
- Ratings converge after ~30 matches per model

### Step 7: Filter by task type

Arena tracks performance by category. View category-specific rankings:

```bash
vibecli arena --leaderboard --category coding
vibecli arena --leaderboard --category explanation
vibecli arena --leaderboard --category creative
vibecli arena --leaderboard --category analysis
```

```
Coding Leaderboard (23 matches)
+----+---------------------------+------+--------+
| #  | Model                     | Elo  | Win %  |
+----+---------------------------+------+--------+
| 1  | claude:claude-sonnet-4    | 1312 | 72.0%  |
| 2  | deepseek:deepseek-coder   | 1267 | 58.3%  |
| 3  | openai:gpt-4o             | 1201 | 45.0%  |
+----+---------------------------+------+--------+
```

### Step 8: Blind mode

For unbiased evaluation, enable blind mode where model names are hidden until after voting:

```bash
vibecli arena --blind --models claude:claude-sonnet-4-20250514,openai:gpt-4o
```

```
Response A:
  fn lcs(a: &str, b: &str) -> String { ... }

Response B:
  fn longest_common_subsequence(s1: &str, s2: &str) -> String { ... }

Which is better? [A/B/tie/skip]: A
Revealed: A = Claude Sonnet 4, B = GPT-4o
```

### Step 9: Export comparison results

Export arena results for sharing or analysis:

```bash
# Export as JSON
vibecli arena --export json > arena_results.json

# Export as CSV
vibecli arena --export csv > arena_results.csv

# Export as Markdown table
vibecli arena --export markdown > arena_results.md
```

Example JSON export:

```json
{
  "matches": [
    {
      "id": "match_001",
      "timestamp": "2026-03-13T10:30:00Z",
      "prompt": "Write a Rust function for LCS",
      "category": "coding",
      "responses": [
        {
          "model": "claude:claude-sonnet-4-20250514",
          "tokens": 342,
          "time_ms": 2300,
          "cost_usd": 0.0034,
          "rating": "win"
        },
        {
          "model": "openai:gpt-4o",
          "tokens": 289,
          "time_ms": 1800,
          "cost_usd": 0.0029,
          "rating": "lose"
        }
      ]
    }
  ],
  "leaderboard": {
    "claude:claude-sonnet-4-20250514": { "elo": 1284, "matches": 28 },
    "openai:gpt-4o": { "elo": 1245, "matches": 28 }
  }
}
```

## VibeUI Arena Panel Features

The VibeUI Arena panel provides additional graphical features:

- **Diff view** -- Toggle a diff between two responses to see exactly where they diverge
- **Syntax highlighting** -- Code blocks are highlighted by language
- **Copy buttons** -- Copy individual responses or the winning response
- **History sidebar** -- Browse all past arena matches with filters
- **Chart view** -- Elo rating trends over time as a line chart
- **Quick rematch** -- Re-run the same prompt with updated models

<!-- Screenshot placeholder: VibeUI Arena with charts -->

## Demo Recording

```json
{
  "meta": {
    "title": "Model Arena",
    "description": "Compare AI models head-to-head with the Model Arena, rate responses, and build an Elo leaderboard.",
    "duration_seconds": 240,
    "version": "1.0.0"
  },
  "steps": [
    {
      "id": 1,
      "action": "shell",
      "command": "vibecli arena --models claude:claude-sonnet-4-20250514,openai:gpt-4o --prompt \"Write a Rust function to check if a string is a palindrome\"",
      "description": "Start an arena match between Claude and GPT-4o",
      "delay_ms": 10000,
      "typing_speed_ms": 30
    },
    {
      "id": 2,
      "action": "type",
      "text": "1w 2l",
      "description": "Rate Claude as winner, GPT-4o as loser",
      "delay_ms": 2000
    },
    {
      "id": 3,
      "action": "keypress",
      "keys": ["enter"],
      "description": "Submit rating",
      "delay_ms": 1500
    },
    {
      "id": 4,
      "action": "shell",
      "command": "vibecli arena --models claude:claude-sonnet-4-20250514,openai:gpt-4o,groq:llama-3.3-70b-versatile --prompt \"Explain the borrow checker in Rust to a JavaScript developer\"",
      "description": "Three-way arena match with an explanation prompt",
      "delay_ms": 12000,
      "typing_speed_ms": 30
    },
    {
      "id": 5,
      "action": "type",
      "text": "1t 2w 3l",
      "description": "Rate GPT-4o as winner for explanation quality",
      "delay_ms": 2000
    },
    {
      "id": 6,
      "action": "keypress",
      "keys": ["enter"],
      "description": "Submit rating",
      "delay_ms": 1500
    },
    {
      "id": 7,
      "action": "shell",
      "command": "vibecli arena --blind --models claude:claude-sonnet-4-20250514,openai:gpt-4o --prompt \"Write a SQL query to find duplicate emails in a users table\"",
      "description": "Blind mode arena match",
      "delay_ms": 8000
    },
    {
      "id": 8,
      "action": "type",
      "text": "A",
      "description": "Pick response A as winner (blind mode)",
      "delay_ms": 2000
    },
    {
      "id": 9,
      "action": "keypress",
      "keys": ["enter"],
      "description": "Submit blind rating and reveal models",
      "delay_ms": 2000
    },
    {
      "id": 10,
      "action": "shell",
      "command": "vibecli arena --leaderboard",
      "description": "View the Elo leaderboard",
      "delay_ms": 3000
    },
    {
      "id": 11,
      "action": "shell",
      "command": "vibecli arena --leaderboard --category coding",
      "description": "View coding-specific leaderboard",
      "delay_ms": 2000
    },
    {
      "id": 12,
      "action": "shell",
      "command": "vibecli arena --export json | head -30",
      "description": "Export arena results as JSON",
      "delay_ms": 2000
    }
  ]
}
```

## What's Next

- [Demo 6: Cost Observatory](../cost-observatory/) -- Understand the cost of each model you tested
- [Demo 3: Multi-Provider Chat](../multi-provider-chat/) -- Deep dive into provider configuration
- [Demo 4: Agent Loop](../agent-loop/) -- Compare how different models perform in agent tasks
