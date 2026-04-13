---
layout: page
title: "Demo 61: Verbatim Drawers & MemPalace"
permalink: /demos/61-memory-drawers/
---


## Overview

OpenMemory's cognitive store compresses information — it summarises, classifies, and embeds. That compression is great for general recall but lossy for exact wording. The **Verbatim Drawer** layer solves this: it stores raw text in 800-character chunks with 100-character overlap, no LLM summarisation at all. When the agent needs to quote a runbook step, reproduce a commit message, or recall a precise error trace, drawers deliver it verbatim.

Inspiration comes from the MemPalace memory architecture, which uses Wing→Room spatial pre-filtering before vector search to achieve 96.6% Recall@5 on LongMemEval benchmarks — roughly 22 percentage points above purely cognitive stores.

This demo covers:
1. Ingesting text and files as verbatim chunks
2. Inspecting Wing/Room spatial organisation
3. Creating cross-project Tunnel waypoints manually and automatically
4. Previewing the 4-layer context the agent receives

**Time to complete:** ~12 minutes

## Prerequisites

- VibeCLI 0.5.1 or later
- Some content to ingest: a runbook, architecture doc, or a few commit messages

---

## Step 1: Ingest text as a verbatim chunk

```bash
vibecli
```

```
> /openmemory chunk "The payment service incident on 2026-04-01 was caused by a missing B-tree index on payments.amount. Peak latency hit 8 s. Fix: add index, deploy migration 0047, restart pods."
```

```
Verbatim ingest complete:
  Chunks created:  1
  Skipped:         0
  Wing (project):  default
  Room (sector):   episodic   (auto-classified)
  Chunk size:      800 chars / 100-char overlap
```

The chunk is stored in `drawers.json` with an FNV-1a hash for exact-duplicate detection. If you run the same command again, the hash matches and the chunk is silently dropped:

```
> /openmemory chunk "The payment service incident on 2026-04-01 was caused by a missing B-tree index on payments.amount. Peak latency hit 8 s. Fix: add index, deploy migration 0047, restart pods."
```

```
Verbatim ingest complete:
  Chunks created:  0
  Skipped:         1  (exact duplicate, FNV-1a match)
```

---

## Step 2: Ingest a file

```
> /openmemory chunk file:docs/deploy-runbook.txt
```

```
Verbatim ingest complete:
  Source:   docs/deploy-runbook.txt
  Chunks:   14
  Skipped:  0
  Wing:     payment-service   (project namespace)
  Room:     procedural        (auto-classified)
```

Long files are split on 800-character boundaries with 100-character overlap so context is never lost at a boundary. Near-duplicate chunks (cosine ≥ 0.85 within a 20-item sliding window) are also dropped.

Ingest a second document:

```
> /openmemory chunk file:docs/architecture.md
```

```
Verbatim ingest complete:
  Source:   docs/architecture.md
  Chunks:   31
  Skipped:  2  (near-duplicate of previous chunks, cosine ≥ 0.85)
```

---

## Step 3: Inspect drawer statistics

```
> /openmemory drawers
```

```
Verbatim Drawer Store
  Total chunks:    46
  Dedup hits:      3   (exact: 1, near-dup: 2)

  Wing distribution:
    payment-service    45 chunks
    default             1 chunk

  Room distribution:
    procedural         23 chunks
    semantic           14 chunks
    episodic            9 chunks
```

The Wing maps to the project namespace (directory name), the Room maps to the auto-classified memory sector. Before running a vector search, the engine filters by Wing+Room — so a query inside `payment-service` never scans unrelated project drawers.

---

## Step 4: See drawers appear in layered context

Now ask for layered context on a topic that's in the drawers:

```
> /openmemory layered "database index migration"
```

```xml
<open-memory>
  <!-- L1: Essential Story (always loaded) -->
  User's primary project is the payment service. Deploy uses Docker + kubectl.

  <!-- L2: Scoped semantic search -->
  [Episodic,   0.79] Payment service incident 2026-04-01 — missing index on payments.amount
  [Procedural, 0.61] Deploy: cargo build --release → docker push → kubectl apply

  <!-- L3: Verbatim chunks (4 raw drawer hits) -->
  [deploy-runbook.txt] Step 6: run migration files in order — psql -f migrations/*.sql
                        Verify with: \d payments and confirm index on payments.amount exists.
  [architecture.md]    The payments table uses a B-tree index on (user_id, amount, created_at)
                        for the monthly billing query. Do not drop this index during maintenance.
  [deploy-runbook.txt] Step 9: after migration 0047, restart the payment-worker pods:
                        kubectl rollout restart deployment/payment-worker
  [incident-2026-04-01 chunk] Missing B-tree index on payments.amount. Peak latency 8s.
                        Fix: add index, deploy migration 0047, restart pods.
</open-memory>
```

L3 verbatim chunks appear when L2 returns fewer than 3 matches, **or** whenever the query strongly matches drawer content.

---

## Step 5: Create a cross-project Tunnel

A Tunnel is a bidirectional waypoint between two memories — even memories in different project stores. When the agent traverses the waypoint graph, a Tunnel link carries the semantic relevance of one project's memories into another.

First, look up the memory ID of the relevant memory:

```
> /openmemory list
```

```
  mem_a3f8c1d2  [Semantic]    "User prefers Rust for backend services"      salience 0.95
  mem_b7e4f091  [Episodic]    "Debugged a deadlock in the payment service"   salience 0.82
  mem_c2a9d5e3  [Procedural]  "cargo build --release → docker push → kubectl" salience 0.78
```

Now create a Tunnel between the episodic memory in this project and a memory in a related project. Use a weight of 0.9 to make the link strong:

```
> /openmemory tunnel mem_b7e4f091 mem_inventory_d4e5f6 0.9
```

```
Tunnel created:
  Source: mem_b7e4f091  (payment-service / episodic)
  Target: mem_inventory_d4e5f6  (inventory-service / semantic)
  Weight: 0.90
  Cross-project: true
  Bidirectional: yes
```

The next time you query the inventory service store and salience traverses this tunnel, the payment service deadlock context can surface — useful when the two services share a database schema.

---

## Step 6: Auto-tunnel across projects

Instead of creating tunnels by hand, let the engine find semantically similar memories across stores automatically:

```
> /openmemory auto-tunnel
```

```
Auto-tunnel scan (threshold: 0.75):
  Stores compared: default ↔ payment-service
  Pairs evaluated: 47
  Tunnels created: 3

  New tunnels:
    mem_a3f8c1d2 ↔ mem_inventory_lang_pref  (cosine 0.84) — "Rust preference"
    mem_c2a9d5e3 ↔ mem_inventory_deploy     (cosine 0.79) — "deployment pattern"
    mem_b7e4f091 ↔ mem_shared_incident_db   (cosine 0.76) — "database incident"
```

Use a higher threshold to create only the strongest links:

```
> /openmemory auto-tunnel 0.85
```

```
Auto-tunnel scan (threshold: 0.85):
  Tunnels created: 1  (mem_a3f8c1d2 ↔ mem_inventory_lang_pref, cosine 0.84)
```

---

## Step 7: Ingest a conversation directly

After completing a coding session you can chunk the entire conversation log as verbatim source for future recall:

```
> /openmemory chunk file:~/.vibecli/sessions/session-2026-04-12.txt
```

This is done automatically at the end of every agent session when `auto_record = true` in your config. Each session gets a unique source ID so chunks can be traced back to the originating conversation.

---

## VibeUI: Drawers Tab

In the **OpenMemory** panel, select the **Drawers** tab:

**Stats cards (top row)**
- Total drawers
- Unique Wings (projects)
- Unique Rooms (sectors)
- Dedup hit rate

**Wing distribution** — horizontal bar chart, one bar per project namespace.

**Room distribution** — colour-coded bars matching sector colours (blue=episodic, green=semantic, gold=procedural, rose=emotional, purple=reflective).

**Context preview** — enter any query and click **Preview**. The panel shows the full 4-layer context block the agent would receive, with colour-coded sections for L1 (yellow), L2 (blue), and L3 verbatim chunks (purple).

**LongMemEval Benchmark** (bottom of tab) — see [Demo 62](../62-memory-benchmark/) for a full walkthrough of the benchmark runner.

---

## Demo Recording

```json
{
  "meta": {
    "title": "Verbatim Drawers & MemPalace",
    "description": "Lossless chunk ingestion, Wing/Room spatial organisation, deduplication, cross-project Tunnel waypoints, and 4-layer agent context preview.",
    "duration_seconds": 240,
    "version": "1.0.0"
  },
  "steps": [
    {
      "id": 1,
      "action": "repl",
      "commands": [
        { "input": "/openmemory chunk \"The payment service incident on 2026-04-01 was caused by a missing B-tree index on payments.amount. Peak latency hit 8 s. Fix: add index, deploy migration 0047, restart pods.\"", "delay_ms": 3000 },
        { "input": "/openmemory chunk file:docs/deploy-runbook.txt", "delay_ms": 3000 },
        { "input": "/openmemory chunk file:docs/architecture.md", "delay_ms": 3000 }
      ],
      "description": "Ingest text and files as verbatim chunks"
    },
    {
      "id": 2,
      "action": "repl",
      "commands": [
        { "input": "/openmemory drawers", "delay_ms": 2000 }
      ],
      "description": "Inspect Wing/Room distribution and dedup stats"
    },
    {
      "id": 3,
      "action": "repl",
      "commands": [
        { "input": "/openmemory layered \"database index migration\"", "delay_ms": 4000 }
      ],
      "description": "Preview 4-layer context — L3 verbatim chunks appear"
    },
    {
      "id": 4,
      "action": "repl",
      "commands": [
        { "input": "/openmemory auto-tunnel", "delay_ms": 4000 }
      ],
      "description": "Auto-detect and create cross-project Tunnel waypoints"
    },
    {
      "id": 5,
      "action": "repl",
      "commands": [
        { "input": "/openmemory drawers", "delay_ms": 2000 },
        { "input": "/quit", "delay_ms": 500 }
      ],
      "description": "Confirm updated drawer stats and exit"
    }
  ]
}
```

## What's Next

- [Demo 62: Memory Benchmarking](../62-memory-benchmark/) — Measure Recall@K across cognitive and verbatim layers
- [Demo 48: OpenMemory Engine](../48-open-memory/) — Core cognitive memory (sectors, consolidation, facts)
- [Memory Guide](../../memory-guide/) — Full reference for all three memory layers
