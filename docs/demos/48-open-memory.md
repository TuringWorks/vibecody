---
layout: page
title: "Demo 48: OpenMemory Cognitive Engine"
permalink: /demos/48-open-memory/
---


## Overview

VibeCody includes OpenMemory, a cognitive memory engine that gives the AI assistant persistent, structured memory across sessions. Unlike simple key-value stores, OpenMemory organises knowledge into 5 cognitive sectors, builds associative graphs between memories, and uses TF-IDF scoring with HNSW approximate nearest-neighbour search for intelligent retrieval. Memories decay over time unless reinforced, mimicking human cognition. All data stays local with optional AES-256-GCM encryption.

This demo covers the full memory lifecycle: adding, querying, inspecting the knowledge graph, running consolidation, and using the newer MemPalace verbatim drawer layer for lossless recall. For a deep dive into drawers and recall benchmarking see [Demo 61](../61-memory-drawers/) and [Demo 62](../62-memory-benchmark/).

**Time to complete:** ~15 minutes

## Prerequisites

- VibeCLI 0.5.1 or later installed and on your PATH
- At least one AI provider configured in `~/.vibecli/config.toml`
- For VibeUI: the desktop app running with the **OpenMemory** panel visible

## The 5 Cognitive Sectors

OpenMemory classifies every memory into one of five sectors, each with its own decay rate and weight:

| Sector         | Purpose                                          | Decay | Example                                   |
|----------------|--------------------------------------------------|-------|-------------------------------------------|
| **Episodic**   | Events and interactions that happened            | Fast  | "User fixed a race condition on April 12"  |
| **Semantic**   | Facts and general knowledge                      | Slow  | "User prefers Rust over Go"                |
| **Procedural** | How-to knowledge and workflows                   | Med   | "Deploy: build, test, push, kubectl apply" |
| **Emotional**  | Sentiment and preference signals                 | Fast  | "User was frustrated by slow CI"           |
| **Reflective** | Meta-observations about patterns (auto-generated)| Slow  | "User frequently asks about async patterns"|

## Composite Retrieval Score

Every memory is ranked by five weighted factors:

```
score = 0.45 × semantic_similarity
      + 0.20 × salience
      + 0.15 × recency
      + 0.10 × waypoint_graph_score
      + 0.10 × sector_match_bonus
```

## Step-by-Step Walkthrough

### Step 1: Add memories across sectors

Launch the REPL and add a few memories:

```bash
vibecli
```

```
vibecli 0.5.1 | Provider: claude | Model: claude-sonnet-4-6
Type /help for commands, /quit to exit

> /openmemory add "User prefers Rust for backend services"
```

```
Memory added successfully.
  ID:      mem_a3f8c1d2
  Sector:  Semantic
  Tags:    [rust, backend, preference]
  Weight:  1.00
  Created: 2026-04-12T10:15:22Z
```

```
> /openmemory add "Debugged a deadlock in the payment service today"
```

```
Memory added successfully.
  ID:      mem_b7e4f091
  Sector:  Episodic
  Tags:    [debug, deadlock, payment]
  Weight:  1.00
  Created: 2026-04-12T10:15:48Z
```

```
> /openmemory add "To deploy: cargo build --release, run tests, docker push, kubectl apply"
```

```
Memory added successfully.
  ID:      mem_c2a9d5e3
  Sector:  Procedural
  Tags:    [deploy, cargo, docker, kubernetes]
  Weight:  1.00
  Created: 2026-04-12T10:16:05Z
```

### Step 2: Query memories with natural language

```
> /openmemory query "coding preferences"
```

```
Query Results (3 matches, scored by composite similarity):

  1. [Semantic]   "User prefers Rust for backend services"
     Score: 0.87 | Tags: rust, backend, preference
     Associations: mem_c2a9d5e3 (procedural, 0.42)

  2. [Procedural] "To deploy: cargo build --release, run tests, docker push, kubectl apply"
     Score: 0.61 | Tags: deploy, cargo, docker, kubernetes
     Associations: mem_a3f8c1d2 (semantic, 0.42)

  3. [Episodic]   "Debugged a deadlock in the payment service today"
     Score: 0.34 | Tags: debug, deadlock, payment
     Associations: none

Scoring: similarity(0.45) + salience(0.20) + recency(0.15) + waypoint(0.10) + sector(0.10)
```

### Step 3: Add temporal facts to the knowledge graph

Facts are explicit subject-predicate-object triples. A new fact for the same subject+predicate automatically closes the previous one:

```
> /openmemory fact user prefers_language Rust
> /openmemory fact deploy uses_tool "docker + kubectl"
> /openmemory fact payment_service last_debugged "2026-04-12 deadlock"
```

```
> /openmemory facts
```

```
Knowledge Graph (bi-temporal facts):

  Fact 1: [ACTIVE since 2026-04-12]
    Subject:   user
    Predicate: prefers_language
    Object:    Rust
    Source:    mem_a3f8c1d2

  Fact 2: [ACTIVE since 2026-04-12]
    Subject:   deploy
    Predicate: uses_tool
    Object:    docker + kubectl
    Source:    mem_c2a9d5e3

  Fact 3: [ACTIVE since 2026-04-12]
    Subject:   payment_service
    Predicate: last_debugged
    Object:    2026-04-12 deadlock
    Source:    mem_b7e4f091

Total: 3 active facts, 0 closed facts
```

### Step 4: Run memory consolidation

Consolidation mimics a sleep cycle — it strengthens frequently accessed memories and generates reflective summaries:

```
> /openmemory consolidate
```

```
Consolidation complete (sleep-cycle mode):
  Memories processed:  3
  Decayed:             0 (all recent, no decay applied)
  Reinforced:          1 (mem_a3f8c1d2 accessed 2x)
  Reflections created: 1

  New reflection:
    ID:     mem_r1f0a8b4
    Sector: Reflective
    Text:   "User's workflow centres on Rust backend development with
             containerised deployment. Payment service is an active focus area."
    Pinned: true
```

### Step 5: Check the health dashboard

```
> /openmemory health
```

```
OpenMemory Health Dashboard
  Total memories:   4        Waypoints:   2
  Verbatim drawers: 0        Facts:       3
  Encryption:       disabled

  Sector distribution:
    Episodic    ████░░░░  1 (25%)   avg salience 0.82
    Semantic    ████░░░░  1 (25%)   avg salience 0.95  ← pinned
    Procedural  ████░░░░  1 (25%)   avg salience 0.78
    Reflective  ████░░░░  1 (25%)   avg salience 0.90  ← pinned
    Emotional   ░░░░░░░░  0

  Diversity index: 0.82 / 1.00   (4 sectors covered)
  At-risk memories (salience ≤ 0.15): 0
  Staleness (not accessed > 30 days): 0
```

### Step 6: Preview what the agent sees (4-layer context)

Before asking the agent anything, you can preview exactly what context it will receive:

```
> /openmemory layered "how should I structure the new inventory service?"
```

```xml
<open-memory>
  <!-- L1: Essential Story (always loaded, salience ≥ 0.60) -->
  User prefers Rust for backend services.
  Deploy process: cargo build --release → docker push → kubectl apply.
  Pattern: User focuses on Rust backend + containerised deployment (Reflective).

  <!-- L2: Scoped semantic search (query: inventory service) -->
  [Semantic,   0.87] User prefers Rust for backend services
  [Procedural, 0.72] cargo build --release, docker push, kubectl apply
  [Episodic,   0.51] Debugged deadlock in payment service 2026-04-12

  <!-- L3: Verbatim drawers — none yet (0 drawers ingested) -->
</open-memory>
```

Now ask the agent — it uses this context automatically without any extra prompt:

```
> How should I structure the new inventory service?
```

```
Based on your preferences and history, I can see that:
- You prefer Rust for backend services
- Your deployment pipeline uses Docker and Kubernetes
- You recently worked on the payment service

I'd recommend structuring the inventory service similarly to your payment
service, using Rust with Actix-web or Axum...
```

### Step 7: Ingest a document as verbatim chunks

For lossless recall of a runbook or spec, use verbatim drawers instead of (or in addition to) cognitive memories. Drawers store raw 800-character chunks with no summarisation:

```
> /openmemory chunk file:docs/deploy-runbook.txt
```

```
Verbatim ingest complete:
  Source:   docs/deploy-runbook.txt
  Chunks:   14
  Skipped:  0 (no duplicates detected)
  Wing:     payment-service   (project namespace)
  Room:     procedural        (auto-classified sector)
```

```
> /openmemory drawers
```

```
Verbatim Drawer Store
  Total chunks: 14
  Dedup hits:   0   (FNV-1a hash + cosine 0.85 threshold)

  Wing distribution:
    payment-service   14 chunks

  Room distribution:
    procedural        11 chunks
    episodic           3 chunks
```

### Step 8: Enable encryption at rest

```
> /openmemory encrypt
```

```
AES-256-GCM encryption enabled.
  Key stored at: ~/.local/share/vibecli/openmemory/.key
  All memories re-encrypted in place.
  Existing backups are NOT encrypted — delete them if needed.
```

### Step 9: Export and import

```
> /openmemory export > my-memories-2026-04-12.md
```

```
Exported 4 memories, 3 facts → my-memories-2026-04-12.md (2.1 KB)
```

To restore on another machine or project:

```
> /openmemory import auto my-memories-2026-04-12.md
```

```
Import complete: 4 memories, 3 facts imported. 0 duplicates skipped.
```

## VibeUI: OpenMemory Panel

In the desktop app, the **OpenMemory** panel provides 7 tabs:

| Tab | What you can do |
|-----|----------------|
| **Overview** | Memory statistics, sector distribution chart, 4-column stats card |
| **Memories** | Browse, search, pin/unpin, delete memories; filter by sector |
| **Query** | Natural language search with scored result cards |
| **Facts** | Browse the bi-temporal knowledge graph; active and closed facts |
| **Graph** | D3 force-directed visualisation of memory associations |
| **Drawers** | Verbatim chunk stats by Wing/Room, 4-layer context preview, LongMemEval benchmark runner |
| **Settings** | Encryption toggle, decay rate tuning, data export/import, clear all |

## Demo Recording

```json
{
  "meta": {
    "title": "OpenMemory Cognitive Engine",
    "description": "Full memory lifecycle: add across 5 sectors, query, build knowledge graph, consolidate, ingest verbatim chunks, and preview 4-layer agent context.",
    "duration_seconds": 300,
    "version": "2.0.0"
  },
  "steps": [
    {
      "id": 1,
      "action": "repl",
      "commands": [
        { "input": "/openmemory add \"User prefers Rust for backend services\"", "delay_ms": 3000 },
        { "input": "/openmemory add \"Debugged a deadlock in the payment service today\"", "delay_ms": 3000 },
        { "input": "/openmemory add \"To deploy: cargo build --release, run tests, docker push, kubectl apply\"", "delay_ms": 3000 }
      ],
      "description": "Add memories across Semantic, Episodic, and Procedural sectors"
    },
    {
      "id": 2,
      "action": "repl",
      "commands": [
        { "input": "/openmemory query \"coding preferences\"", "delay_ms": 4000 }
      ],
      "description": "Query memories with natural language"
    },
    {
      "id": 3,
      "action": "repl",
      "commands": [
        { "input": "/openmemory fact user prefers_language Rust", "delay_ms": 2000 },
        { "input": "/openmemory fact deploy uses_tool \"docker + kubectl\"", "delay_ms": 2000 },
        { "input": "/openmemory facts", "delay_ms": 3000 }
      ],
      "description": "Add temporal facts and view the knowledge graph"
    },
    {
      "id": 4,
      "action": "repl",
      "commands": [
        { "input": "/openmemory consolidate", "delay_ms": 4000 }
      ],
      "description": "Run sleep-cycle memory consolidation"
    },
    {
      "id": 5,
      "action": "repl",
      "commands": [
        { "input": "/openmemory layered \"new inventory service\"", "delay_ms": 3000 }
      ],
      "description": "Preview 4-layer agent context (L1 essential story + L2 scoped + L3 drawers)"
    },
    {
      "id": 6,
      "action": "repl",
      "commands": [
        { "input": "/openmemory chunk file:docs/deploy-runbook.txt", "delay_ms": 3000 },
        { "input": "/openmemory drawers", "delay_ms": 2000 }
      ],
      "description": "Ingest a document as verbatim chunks (MemPalace technique)"
    },
    {
      "id": 7,
      "action": "repl",
      "commands": [
        { "input": "/openmemory health", "delay_ms": 3000 },
        { "input": "/openmemory stats", "delay_ms": 2000 },
        { "input": "/quit", "delay_ms": 500 }
      ],
      "description": "View health dashboard and exit"
    }
  ]
}
```

## What's Next

- [Demo 61: Verbatim Drawers & MemPalace](../61-memory-drawers/) — Deep dive into lossless chunk ingestion, dedup, and cross-project tunnels
- [Demo 62: Memory Benchmarking](../62-memory-benchmark/) — Run LongMemEval recall@K in the CLI and VibeUI panel
- [Demo 49: Auto-Research](../49-auto-research/) — Autonomous iterative research agent that uses memory for cross-run learning
- [Memory Guide](../../memory-guide/) — Full reference for all three memory layers
