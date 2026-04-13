---
layout: page
title: "Tutorial: Building a Long-Term Memory Store"
permalink: /tutorials/memory/
---

This tutorial walks through building a rich, well-organised memory store for VibeCLI from scratch — adding memories, fact triples, verbatim documents, and measuring recall quality over time.

**Time:** ~20 minutes  
**Prerequisites:** VibeCLI installed, at least one provider configured

---

## Part 1 — Your First Memories

Start the REPL:

```bash
vibecli
```

Add a handful of memories spanning different sectors:

```
> /openmemory add "I prefer Rust for all backend services — safety + performance"
Added memory [mem_a3f8] sector=Semantic salience=100%

> /openmemory add "Debugged a Tokio runtime deadlock in the auth service today"
Added memory [mem_b7e4] sector=Episodic salience=100%

> /openmemory add "Deploy pipeline: cargo build --release, docker build, push to ECR, kubectl rollout"
Added memory [mem_c2a9] sector=Procedural salience=100%

> /openmemory add "Found the slow CI runs frustrating — 14-minute builds kill flow"
Added memory [mem_d1f6] sector=Emotional salience=100%
```

OpenMemory classifies sector automatically by analysing vocabulary. Check what landed:

```
> /openmemory list
Memories (4 total):

  [Semantic]   sal=100% "I prefer Rust for all backend services — safety + perfor..."
    id=mem_a3f8
  [Episodic]   sal=100% "Debugged a Tokio runtime deadlock in the auth service t..."
    id=mem_b7e4
  [Procedural] sal=100% "Deploy pipeline: cargo build --release, docker build, p..."
    id=mem_c2a9
  [Emotional]  sal=100% "Found the slow CI runs frustrating — 14-minute builds k..."
    id=mem_d1f6
```

---

## Part 2 — Inspecting and Managing Individual Memories

Show a memory in full:

```
> /openmemory show mem_b7e4
Memory mem_b7e4
  Sector:   Episodic
  Salience: 100% 
  Tags:     tokio, deadlock, auth
  Created:  12 secs ago

  Debugged a Tokio runtime deadlock in the auth service today
```

Pin important procedural knowledge so it survives decay:

```
> /openmemory pin mem_c2a9
Pinned memory mem_c2a9 — exempt from decay and purge.
```

Verify the pin flag appears in the list:

```
> /openmemory list
  [Procedural] sal=100% [pinned] "Deploy pipeline: cargo build --release..."
    id=mem_c2a9
```

If you added a memory by mistake, delete it:

```
> /openmemory delete mem_d1f6
Deleted memory mem_d1f6.
```

---

## Part 3 — The Knowledge Graph

Facts are structured subject/predicate/object triples with validity windows. They're good for things that change over time — tech stack decisions, library versions, team assignments.

```
> /openmemory fact project language Rust
> /openmemory fact project database PostgreSQL-16
> /openmemory fact project ci_tool GitHub-Actions
> /openmemory fact auth_service runtime Tokio
```

View all active facts:

```
> /openmemory facts
Current temporal facts (4):

  project language Rust                (conf: 100%)
  project database PostgreSQL-16       (conf: 100%)
  project ci_tool GitHub-Actions       (conf: 100%)
  auth_service runtime Tokio           (conf: 100%)
```

When the team migrates the database, update the fact — the previous entry auto-closes:

```
> /openmemory fact project database CockroachDB
```

```
> /openmemory facts
  project language Rust                (conf: 100%)  ← active
  project database CockroachDB         (conf: 100%)  ← active  (PostgreSQL-16 auto-closed)
  project ci_tool GitHub-Actions       (conf: 100%)  ← active
  auth_service runtime Tokio           (conf: 100%)  ← active
```

---

## Part 4 — Querying with Natural Language

Search uses the composite 5-signal score (similarity + salience + recency + graph + sector):

```
> /openmemory query "how do I deploy?"
Found 2 memories:

  1. [Procedural] score=0.891 sal=100% "Deploy pipeline: cargo build --release, docker build..."
  2. [Semantic]   score=0.312 sal=100% "I prefer Rust for all backend services..."
```

The procedural memory ranks first because it has high semantic similarity *and* a sector match bonus for procedural content.

---

## Part 5 — Verbatim Drawers for Exact Recall

The cognitive store compresses information for relevance. For text where exact wording matters — runbooks, architecture decisions, error logs — use verbatim drawers instead.

Ingest a runbook:

```
> /openmemory chunk file:docs/deploy-runbook.txt
Stored 14 verbatim drawer(s) from "docs/deploy-runbook.txt" (no summarization — raw recall).
  Total drawers: 14  |  Use '/openmemory drawers' to inspect
```

Inspect Wing/Room organisation:

```
> /openmemory drawers
Verbatim Drawer Store (14 total)

  Chunk size: 800 chars / 100 overlap  |  Near-dedup threshold: 85%

  Wings (projects):
    my-project           14 drawers

  Rooms (sectors):
    procedural           11 drawers
    episodic              3 drawers

  Use '/openmemory context <query>' to see L1+L2+L3 layered retrieval.
```

Also ingest the architecture document:

```
> /openmemory chunk file:docs/architecture.md
Stored 31 verbatim drawer(s) from "docs/architecture.md" (no summarization — raw recall).
  Total drawers: 45  |  Use '/openmemory drawers' to inspect
```

---

## Part 6 — Previewing What the Agent Sees

Before trusting the agent, preview exactly what context it will receive for a query:

```
> /openmemory layered "deploy to production"
```

```xml
<open-memory>
  <!-- L1: Essential Story (always loaded, salience ≥ 0.60) -->
  I prefer Rust for backend services.
  Deploy pipeline: cargo build --release → docker build → push ECR → kubectl rollout. [PINNED]

  <!-- L2: Scoped semantic search (query: deploy to production) -->
  [Procedural, 0.89] Deploy pipeline: cargo build --release, docker build, push to ECR, kubectl rollout
  [Episodic,   0.54] Debugged a Tokio runtime deadlock in the auth service

  <!-- L3: Verbatim chunks (3 raw drawer hits) -->
  [deploy-runbook.txt] Step 4: Before rolling out, run the smoke-test suite:
                        make smoke CLUSTER=prod. If any test fails, do NOT promote.
  [deploy-runbook.txt] Step 7: Monitor rollout with: kubectl rollout status deployment/api-server
                        Wait for "successfully rolled out" before closing the ticket.
  [architecture.md]    All deployments go through the staging environment first. Production
                        deploys require approval from two engineers via the deploy-approvals channel.
</open-memory>
```

The L3 verbatim chunks contain exact runbook steps — no information was lost to summarisation.

Now ask the agent naturally:

```
> How do I safely deploy the API service today?
```

The agent responds with precise runbook steps because L3 injected the exact text.

---

## Part 7 — Running the Recall Benchmark

Measure how well your store can answer questions:

```
> /openmemory benchmark
LongMemEval Benchmark (k=5)
  Total memories: 3   Verbatim drawers: 45   Probe cases: 20

  Recall@5 — Cognitive  (L2):  55.0%  ████████████████░░░░░░░░░░░░░░
  Recall@5 — Verbatim   (L3):  85.0%  █████████████████████████░░░░░
  Recall@5 — Combined:         91.0%  ███████████████████████████░░░
```

The cognitive store is sparse (only 3 memories) — verbatim drawers are doing most of the work. Add more memories to bring cognitive recall up:

```
> /openmemory add "Auth service uses Tokio 1.36 with async-trait for handler traits"
> /openmemory add "Database migrations use sqlx migrate run — never touch production manually"
> /openmemory add "CI pipeline: lint → unit tests → integration tests → docker build → deploy to staging"
```

Re-run:

```
> /openmemory benchmark
  Recall@5 — Cognitive:   70.0%  █████████████████████░░░░░░░░░
  Recall@5 — Verbatim:    85.0%  █████████████████████████░░░░░
  Recall@5 — Combined:    95.5%  ████████████████████████████░░
```

---

## Part 8 — Memory Lifecycle

### Decay and consolidation

Over time, memories that aren't accessed lose salience. Pinned memories are exempt. Run decay manually to see what's fading:

```
> /openmemory decay
Decay complete: 0 memories purged, 6 remaining
```

After many sessions, run consolidation to merge similar weak memories and generate a reflective summary:

```
> /openmemory consolidate
Consolidated 1 groups of similar memories.
```

```
> /openmemory list
  ...
  [Reflective] sal=90%  [pinned] "User's workflow centres on Rust backend development
                                  with Tokio async runtime. Deployment uses Docker +
                                  Kubernetes with strict smoke testing before production."
```

### Health check

```
> /openmemory health
OpenMemory Health Dashboard
  Total memories:   7        Waypoints:    4
  Verbatim drawers: 45       Facts:        4
  Encryption:       disabled

  Sector distribution:
    Semantic    ████████░░  2 (29%)   avg salience 92%
    Episodic    ████░░░░░░  1 (14%)   avg salience 82%
    Procedural  ████░░░░░░  1 (14%)   avg salience 100%  ← pinned
    Reflective  ████░░░░░░  1 (14%)   avg salience 90%   ← pinned
    Emotional   ░░░░░░░░░░  0

  Diversity index: 0.71 / 1.00
  At-risk memories (salience ≤ 0.15): 0
  Staleness (not accessed > 30 days): 0
```

---

## Part 9 — Export, Import, and Encryption

Export your whole store as a markdown snapshot:

```
> /openmemory export > memory-backup-2026-04-12.md
```

To restore on another machine:

```
> /openmemory import auto memory-backup-2026-04-12.md
Import complete: 7 memories, 4 facts imported. 0 duplicates skipped.
```

Enable encryption at rest:

```
> /openmemory encrypt
AES-256-GCM encryption enabled.
  Key stored at: ~/.local/share/vibecli/openmemory/.key
  All memories re-encrypted in place.
```

---

## Part 10 — Cross-Project Tunnels

If you work on multiple related services, link their memories with Tunnels so context from one project surfaces in another.

First check IDs in the current project store:

```
> /openmemory list
  [Episodic] id=mem_b7e4 "Debugged Tokio deadlock in auth service"
```

Create a tunnel to an inventory-service memory:

```
> /openmemory tunnel mem_b7e4 mem_inv_8a3c 0.9
Tunnel created: mem_b7e4 ↔ mem_inv_8a3c  weight=0.90  cross-project=true
```

Or let VibeCLI find them automatically:

```
> /openmemory auto-tunnel 0.80
Auto-tunnel scan (threshold: 0.80):
  Tunnels created: 2
    mem_b7e4 ↔ mem_inv_8a3c  (cosine 0.83)
    mem_c2a9 ↔ mem_inv_deploy (cosine 0.81)
```

---

## Summary

After completing this tutorial your store has:

| Layer | Contents |
|-------|----------|
| Cognitive memories | 7+ memories spanning 4 sectors |
| Temporal facts | 4 active facts, 1 auto-closed |
| Verbatim drawers | 45+ raw chunks from 2 documents |
| Tunnels | Cross-project waypoints to related service |
| Benchmark | Combined Recall@5 ≥ 90% |

The agent now has persistent, structured memory of your preferences, your stack, your deployment process, and the exact wording of your runbooks — without any manual prompt engineering.

---

## What's Next

- [Memory Guide](../memory-guide/) — Complete reference for all three memory layers
- [Demo 61: Verbatim Drawers](../demos/61-memory-drawers/) — Deep dive into MemPalace chunk ingestion
- [Demo 62: Memory Benchmarking](../demos/62-memory-benchmark/) — Improve recall quality over time
- [Configuration Guide](../configuration/) — Tune `[openmemory]` decay rates and encryption
