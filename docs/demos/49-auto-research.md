---
layout: page
title: "Demo 49: Autonomous Research Agent"
permalink: /demos/49-auto-research/
---


## Overview

VibeCody's AutoResearch module is an autonomous iterative research agent that systematically explores parameter spaces, tracks experiments, and learns across runs. It supports 5 search strategies, 7 research domains, composite scoring with NaN detection, and cross-run learning through a persistent ResearchMemory. Safety rails prevent runaway experiments from consuming disk, memory, or time.

**Time to complete:** ~15 minutes

## Prerequisites

- VibeCLI 0.5.1 or later installed and on your PATH
- At least one AI provider configured
- A project directory with code to optimize (examples use a database-backed service)
- For VibeUI: the desktop app running with the **AutoResearch** panel visible

## Research Strategies

AutoResearch offers 5 strategies for exploring the experiment space:

| Strategy        | Best For                                    | How It Works                                            |
|-----------------|---------------------------------------------|---------------------------------------------------------|
| **Greedy**      | Quick wins, single-variable tuning          | Always picks the next experiment that improves the best known score |
| **BeamSearch**  | Moderate exploration with pruning           | Maintains top-K candidates, expands each, prunes worst  |
| **Genetic**     | Large parameter spaces                      | Mutation and crossover of top-performing configurations  |
| **Combinatorial** | Exhaustive small spaces                  | Tries every combination of parameters                   |
| **Bayesian**    | Expensive evaluations, sample-efficient     | Builds a surrogate model, picks points with highest expected improvement |

## Step-by-Step Walkthrough

### Step 1: Create a new research session

```bash
vibecli
```

```
vibecli 0.5.1 | Provider: claude | Model: claude-sonnet-4-6
Type /help for commands, /quit to exit

> /autoresearch new "optimize database queries"
```

```
Research session created.
  Session ID: rs_48d2a1c7
  Domain:     DatabaseTuning
  Strategy:   Greedy (default, change with /autoresearch config strategy <name>)
  Metrics:    [query_time_ms, throughput_qps, memory_mb]
  Status:     Ready

Hypothesis generated:
  H1: "Adding composite indexes on frequently joined columns will reduce
       query time by 30-50%"
  Confidence: Medium
  Predicted impact: High
```

### Step 2: Configure the research strategy

Switch to Bayesian for sample-efficient exploration:

```
> /autoresearch config strategy bayesian
```

```
Strategy updated: Bayesian
  Surrogate model:    Gaussian Process
  Acquisition:        Expected Improvement
  Initial samples:    5 (random)
  Max experiments:    50
  Timeout per run:    300s
```

### Step 3: Start the research run

```
> /autoresearch start rs_48d2a1c7
```

```
Starting research session rs_48d2a1c7...

[Experiment 1/50] Random initial sample
  Parameters: { index_type: "btree", pool_size: 10, cache_mb: 256 }
  Results:    query_time_ms=45.2, throughput_qps=1820, memory_mb=312
  Score:      0.62

[Experiment 2/50] Random initial sample
  Parameters: { index_type: "hash", pool_size: 20, cache_mb: 512 }
  Results:    query_time_ms=38.1, throughput_qps=2140, memory_mb=548
  Score:      0.71

[Experiment 3/50] Random initial sample
  Parameters: { index_type: "btree", pool_size: 5, cache_mb: 128 }
  Results:    query_time_ms=62.8, throughput_qps=1240, memory_mb=189
  Score:      0.44

[Experiment 4/50] Bayesian acquisition (EI=0.18)
  Parameters: { index_type: "btree", pool_size: 25, cache_mb: 512 }
  Results:    query_time_ms=31.4, throughput_qps=2580, memory_mb=540
  Score:      0.79 (new best)

[Experiment 5/50] Bayesian acquisition (EI=0.12)
  Parameters: { index_type: "btree", pool_size: 30, cache_mb: 768 }
  Results:    query_time_ms=28.7, throughput_qps=2820, memory_mb=798
  Score:      0.83 (new best)

...

Session paused at experiment 12/50 (Ctrl+C or /autoresearch pause).
  Best score:   0.89 (experiment 9)
  Best params:  { index_type: "btree", pool_size: 28, cache_mb: 640 }
  Improvement:  43.6% over baseline
```

### Step 4: Check session status

```
> /autoresearch status rs_48d2a1c7
```

```
Session: rs_48d2a1c7 — "optimize database queries"
  Domain:      DatabaseTuning
  Strategy:    Bayesian
  Status:      Paused (experiment 12/50)
  Duration:    4m 38s
  Best score:  0.89

  Metric trends (last 5 experiments):
    query_time_ms:  45.2 → 28.7 → 24.1 → 22.8 → 23.5
    throughput_qps: 1820 → 2820 → 3100 → 3240 → 3180
    memory_mb:      312  → 798  → 640  → 648  → 652

  Hypotheses:
    H1: "Composite indexes reduce query time" — CONFIRMED (38% reduction)
    H2: "Pool size 25-30 is optimal"          — CONFIRMED (plateau at 28)
    H3: "Cache beyond 640MB has diminishing returns" — CONFIRMED

  Safety violations: 0
```

### Step 5: Analyze results

```
> /autoresearch analyze rs_48d2a1c7
```

```
Analysis for session rs_48d2a1c7:

  Top 3 configurations:
    Rank 1: score=0.89 | query=22.8ms, qps=3240, mem=648MB
            { index_type: "btree", pool_size: 28, cache_mb: 640 }
    Rank 2: score=0.87 | query=24.1ms, qps=3100, mem=640MB
            { index_type: "btree", pool_size: 25, cache_mb: 640 }
    Rank 3: score=0.83 | query=28.7ms, qps=2820, mem=798MB
            { index_type: "btree", pool_size: 30, cache_mb: 768 }

  Parameter sensitivity:
    cache_mb:    HIGH  (0.72 correlation with score)
    pool_size:   MEDIUM (0.54 correlation)
    index_type:  LOW   (btree consistently wins)

  Recommendations:
    1. Use btree indexes (hash showed no advantage for this workload)
    2. Set connection pool to 28 connections
    3. Allocate 640MB query cache (beyond this, memory cost exceeds benefit)
    4. Expected production improvement: ~50% latency reduction
```

### Step 6: View cross-run learning

```
> /autoresearch lessons
```

```
Research Memory (cross-run lessons):

  Lesson 1: [DatabaseTuning] "btree indexes outperform hash for mixed
             read/write workloads" (from rs_48d2a1c7, confidence: high)

  Lesson 2: [DatabaseTuning] "connection pool sweet spot is 25-30 for
             PostgreSQL on 8-core machines" (from rs_48d2a1c7, confidence: high)

  Lesson 3: [DatabaseTuning] "cache memory has diminishing returns beyond
             ~640MB for datasets under 10GB" (from rs_48d2a1c7, confidence: medium)

  Success patterns: 3 recorded
  Failure patterns: 1 recorded (hash index + small pool → low throughput)
  Metric baselines: query_time_ms < 30ms, throughput_qps > 3000
```

### Step 7: Export results

```
> /autoresearch export rs_48d2a1c7
```

```
Exported to: ./research_rs_48d2a1c7.tsv

Format: TSV (compatible with autoresearch tooling)
Columns: experiment_id, timestamp, index_type, pool_size, cache_mb,
         query_time_ms, throughput_qps, memory_mb, composite_score

12 rows written.
```

### Step 8: Get AI suggestions for next research

```
> /autoresearch suggest rs_48d2a1c7
```

```
Suggested next experiments (based on Bayesian model + lessons):

  1. Test prepared statements with optimal config
     Parameters: { ..., prepared_statements: true }
     Expected improvement: 8-15% latency reduction
     Rationale: Common optimization not yet explored

  2. Test read replicas for read-heavy queries
     Parameters: { ..., read_replicas: 2 }
     Expected improvement: 40-60% throughput increase
     Rationale: Scale-out strategy for read workloads

  3. Test query plan caching
     Parameters: { ..., plan_cache: true }
     Expected improvement: 5-10% for repeated query patterns
```

## VibeUI: AutoResearch Panel

The **AutoResearch** panel in VibeUI provides 5 tabs:

- **Setup** -- Create sessions, select domain and strategy, define metrics
- **Experiments** -- Live experiment tracking with progress bar and metric charts
- **Analysis** -- Parameter sensitivity heatmaps and configuration rankings
- **Memory** -- Cross-run lessons, success/failure patterns, baselines
- **Export** -- Download TSV files and generate summary reports

## Demo Recording

```json
{
  "meta": {
    "title": "Autonomous Research Agent",
    "description": "Create a research session, run Bayesian optimization experiments, and analyze results with cross-run learning.",
    "duration_seconds": 240,
    "version": "1.0.0"
  },
  "steps": [
    {
      "id": 1,
      "action": "repl",
      "commands": [
        { "input": "/autoresearch new \"optimize database queries\"", "delay_ms": 3000 },
        { "input": "/autoresearch config strategy bayesian", "delay_ms": 2000 }
      ],
      "description": "Create session and configure Bayesian strategy"
    },
    {
      "id": 2,
      "action": "repl",
      "commands": [
        { "input": "/autoresearch start rs_48d2a1c7", "delay_ms": 15000 }
      ],
      "description": "Run research experiments"
    },
    {
      "id": 3,
      "action": "repl",
      "commands": [
        { "input": "/autoresearch status rs_48d2a1c7", "delay_ms": 3000 },
        { "input": "/autoresearch analyze rs_48d2a1c7", "delay_ms": 4000 }
      ],
      "description": "Check status and analyze results"
    },
    {
      "id": 4,
      "action": "repl",
      "commands": [
        { "input": "/autoresearch lessons", "delay_ms": 3000 },
        { "input": "/autoresearch export rs_48d2a1c7", "delay_ms": 2000 },
        { "input": "/quit", "delay_ms": 500 }
      ],
      "description": "View lessons, export results, and exit"
    }
  ]
}
```

## What's Next

- [Demo 48: OpenMemory](../48-open-memory/) -- Persistent cognitive memory across sessions
- [Demo 50: Warp-Style Features](../50-warp-features/) -- Natural language commands and secret redaction
- [Demo 52: Watch Mode & Sandbox](../52-watch-sandbox/) -- File watching and isolated execution
