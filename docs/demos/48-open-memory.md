---
layout: page
title: "Demo 48: OpenMemory Cognitive Engine"
permalink: /demos/48-open-memory/
---


## Overview

VibeCody includes OpenMemory, a cognitive memory engine that gives the AI assistant persistent, structured memory across sessions. Unlike simple key-value stores, OpenMemory organizes knowledge into 5 cognitive sectors, builds associative graphs between memories, and uses TF-IDF scoring for intelligent retrieval. Memories decay over time unless reinforced, mimicking how human cognition works. All data stays local with optional AES-256-GCM encryption.

**Time to complete:** ~10 minutes

## Prerequisites

- VibeCLI 0.5.1 or later installed and on your PATH
- At least one AI provider configured in `~/.vibecli/config.toml`
- For VibeUI: the desktop app running with the **OpenMemory** panel visible

## The 5 Cognitive Sectors

OpenMemory classifies every memory into one of five sectors, each with its own decay rate and weight:

| Sector         | Purpose                                          | Example                                   |
|----------------|--------------------------------------------------|--------------------------------------------|
| **Episodic**   | Events and interactions that happened             | "User fixed a race condition on March 15"  |
| **Semantic**   | Facts and general knowledge                       | "User prefers Rust over Go"                |
| **Procedural** | How-to knowledge and workflows                    | "Deploy process: build, test, push, tag"   |
| **Emotional**  | Sentiment and preference signals                  | "User was frustrated by slow CI"           |
| **Reflective** | Meta-observations about patterns (auto-generated) | "User frequently asks about async patterns"|

## Step-by-Step Walkthrough

### Step 1: Add your first memory

Launch the REPL and add a semantic memory:

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
  Created: 2026-03-29T10:15:22Z

OpenMemory classified this as Semantic because it describes a factual preference.
```

### Step 2: Add memories across sectors

```
> /openmemory add "Debugged a deadlock in the payment service yesterday"
```

```
Memory added successfully.
  ID:      mem_b7e4f091
  Sector:  Episodic
  Tags:    [debug, deadlock, payment]
  Weight:  1.00
  Created: 2026-03-29T10:15:48Z
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
  Created: 2026-03-29T10:16:05Z
```

### Step 3: Query memories with natural language

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

  3. [Episodic]   "Debugged a deadlock in the payment service yesterday"
     Score: 0.34 | Tags: debug, deadlock, payment
     Associations: none

Scoring: similarity(0.45) + salience(0.20) + recency(0.15) + waypoint(0.10) + sector(0.10)
```

### Step 4: View the knowledge graph

```
> /openmemory facts
```

```
Knowledge Graph (bi-temporal facts):

  Fact 1: [ACTIVE since 2026-03-29]
    Subject:   user
    Predicate: prefers_language
    Object:    Rust
    Source:     mem_a3f8c1d2

  Fact 2: [ACTIVE since 2026-03-29]
    Subject:   user
    Predicate: works_on
    Object:    payment service
    Source:     mem_b7e4f091

  Fact 3: [ACTIVE since 2026-03-29]
    Subject:   deployment
    Predicate: uses_tool
    Object:    docker, kubernetes
    Source:     mem_c2a9d5e3

Total: 3 active facts, 0 closed facts
```

### Step 5: Run memory consolidation

Consolidation mimics a sleep cycle -- it strengthens frequently accessed memories and generates reflective summaries:

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
    Text:   "User's workflow centers on Rust backend development with
             containerized deployment. Payment service is an active focus area."
    Pinned: true
```

### Step 6: Check memory statistics

```
> /openmemory stats
```

```
OpenMemory Statistics:
  Total memories:  4
  By sector:
    Episodic:    1 (25%)
    Semantic:    1 (25%)
    Procedural:  1 (25%)
    Reflective:  1 (25%)
    Emotional:   0 (0%)
  Knowledge graph: 3 active facts
  Associations:    2 links
  Encryption:      disabled
  Storage:         ~/.local/share/vibecli/openmemory/
  Size on disk:    12.4 KB
```

### Step 7: Enable encryption at rest

```
> /openmemory encrypt
```

```
AES-256-GCM encryption enabled.
  Key stored at: ~/.local/share/vibecli/openmemory/.key
  All memories re-encrypted in place.
  Existing backups are NOT encrypted -- delete them if needed.
```

### Step 8: Agent context injection

When the agent runs, OpenMemory automatically injects relevant context. Try it:

```
> How should I structure the new inventory service?
```

```
Based on your preferences and history, I can see that:
- You prefer Rust for backend services
- Your deployment pipeline uses Docker and Kubernetes
- You recently worked on the payment service

I'd recommend structuring the inventory service similarly to your payment
service, using Rust with Actix-web or Axum. Here's a suggested layout...
```

The agent received context from OpenMemory via `get_agent_context()` without any explicit query.

## VibeUI: OpenMemory Panel

In the desktop app, the **OpenMemory** panel provides 6 tabs:

- **Overview** -- Memory statistics and sector distribution chart
- **Memories** -- Browse, search, pin, unpin, and delete memories
- **Query** -- Natural language search with scored results
- **Facts** -- Browse the bi-temporal knowledge graph
- **Graph** -- Visual associative graph showing memory connections
- **Settings** -- Encryption toggle, decay rate tuning, data export/import

## Demo Recording

```json
{
  "meta": {
    "title": "OpenMemory Cognitive Engine",
    "description": "Add memories across 5 cognitive sectors, query with natural language, view the knowledge graph, and run consolidation.",
    "duration_seconds": 180,
    "version": "1.0.0"
  },
  "steps": [
    {
      "id": 1,
      "action": "repl",
      "commands": [
        { "input": "/openmemory add \"User prefers Rust for backend services\"", "delay_ms": 3000 },
        { "input": "/openmemory add \"Debugged a deadlock in the payment service yesterday\"", "delay_ms": 3000 },
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
        { "input": "/openmemory facts", "delay_ms": 3000 }
      ],
      "description": "View bi-temporal knowledge graph"
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
        { "input": "/openmemory stats", "delay_ms": 2000 },
        { "input": "/quit", "delay_ms": 500 }
      ],
      "description": "View memory statistics and exit"
    }
  ]
}
```

## What's Next

- [Demo 49: Auto-Research](../49-auto-research/) -- Autonomous iterative research agent
- [Demo 51: Profiles & Sessions](../51-profiles-sessions/) -- Profile-based configuration and session resumption
- [Demo 53: Workflow Orchestration](../53-workflow-orchestration/) -- Task tracking with lessons and complexity estimation
