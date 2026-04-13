---
layout: page
title: "Demo 62: Memory Benchmarking"
permalink: /demos/62-memory-benchmark/
---


## Overview

How good is your memory store? The built-in LongMemEval-style benchmark measures Recall@K across all retrieval layers so you can answer that question with numbers instead of guesses.

The benchmark runs a fixed set of probe questions against your live memory store and checks whether the expected answer appears in the top-K retrieved results. It reports three recall figures:

| Metric | Layer | Typical range |
|--------|-------|---------------|
| **Recall@K — Cognitive** | L2 semantic search (HNSW + TF-IDF) | 55–80% for sparse stores |
| **Recall@K — Verbatim** | L3 raw drawer chunks | 80–97% after document ingestion |
| **Recall@K — Combined** | Either layer returns a hit | 90–99% |

The combined figure is the number that matters for agent quality. If it is below 80%, add more content or lower your dedup thresholds.

**Time to complete:** ~8 minutes

## Prerequisites

- VibeCLI 0.5.1 or later
- A populated memory store (at least a few memories; more content gives more meaningful numbers)
- Optionally: some verbatim chunks ingested (see [Demo 61](../61-memory-drawers/))

---

## Step 1: Run the benchmark at k=5

```bash
vibecli
```

```
> /openmemory benchmark
```

```
LongMemEval Benchmark (k=5)
  Total memories:   47   Verbatim drawers: 132   Probe cases: 20

  Recall@5 — Cognitive  (L2):  75.0%  ██████████████████████░░░░░░░░
  Recall@5 — Verbatim   (L3):  90.0%  ███████████████████████████░░░
  Recall@5 — Combined:         96.0%  ████████████████████████████████

  Per-case breakdown:
    sector       query                                           cognitive  verbatim
    ─────────────────────────────────────────────────────────────────────────────────
    episodic     What was the last project I worked on?          ✓          ✓
    semantic     What programming languages do I know?           ✓          ✓
    procedural   How do I run the test suite for this project?   ✓          ✓
    preference   What coding style does the user prefer?         ✗          ✓
    identity     What is the user's primary role?                ✗          ✓
    episodic     What was the most recent error we debugged?     ✓          ✓
    semantic     What AI providers are configured?               ✓          ✓
    procedural   What is the deployment pipeline?                ✓          ✓
    preference   How does the user prefer to name variables?     ✗          ✓
    emotional    Was the user frustrated with anything?          ✓          ✗
    semantic     What databases does this project use?           ✓          ✓
    procedural   How do I set up a new dev environment?          ✗          ✓
    episodic     When was the last incident in production?       ✓          ✓
    reflective   What patterns does the user repeat?             ✓          ✗
    semantic     What is the main programming language?          ✓          ✓
    procedural   How do I add a new API endpoint?                ✓          ✓
    preference   What test framework does the user prefer?       ✗          ✗
    episodic     What was discussed in the last session?         ✓          ✓
    semantic     What version of Rust is required?               ✗          ✓
    procedural   How do I run a database migration?              ✓          ✓

  Summary: 15/20 cognitive · 18/20 verbatim · 19/20 combined
```

---

## Step 2: Increase k to check depth

```
> /openmemory benchmark 10
```

```
LongMemEval Benchmark (k=10)
  Recall@10 — Cognitive:   85.0%  ████████████████████████████░░
  Recall@10 — Verbatim:    95.0%  ██████████████████████████████
  Recall@10 — Combined:    98.0%  ████████████████████████████████
```

At k=10, the cognitive layer has more chances to surface a relevant memory — useful when a specific answer is ranked 6th–10th rather than in the top 5.

---

## Step 3: Improve recall by adding missing content

Two probe cases failed both layers:
- **"What test framework does the user prefer?"** — no matching memory or drawer

Add the missing memory:

```
> /openmemory add "User prefers Nextest (cargo-nextest) for running Rust tests; faster parallelism than cargo test"
```

And optionally ingest the relevant config:

```
> /openmemory chunk file:.config/nextest.toml
```

Rerun the benchmark:

```
> /openmemory benchmark
```

```
  Recall@5 — Cognitive:   80.0%  ████████████████████████░░░░░░
  Recall@5 — Verbatim:    95.0%  ████████████████████████████████
  Recall@5 — Combined:   100.0%  ████████████████████████████████

  preference   What test framework does the user prefer?       ✓          ✓   ← was ✗ ✗
```

---

## Step 4: Watch combined recall grow over time

The benchmark is most useful when you run it regularly. A common workflow:

```bash
# At the end of each significant coding session
vibecli -e '/openmemory benchmark >> ~/.vibecli/recall-log.txt'
```

Plot the log to see recall trend over time as your store grows.

---

## Step 5: Interpret low scores

| Pattern | Likely cause | Fix |
|---------|-------------|-----|
| Cognitive low, Verbatim high | Memories too short or abstract | Add more specific memories with `/openmemory add` |
| Verbatim low, Cognitive high | Not enough raw documents ingested | Use `/openmemory chunk file:...` for key docs |
| Both low | Store nearly empty | Add memories, run `/openmemory consolidate` |
| Combined 100% but only 2 drawers | Store too small to be meaningful | Ingest more documents, add more memories |
| Preference sector always fails | Preference-type info lives in runbooks, not memories | Ingest chat logs with `/openmemory chunk file:session.txt` |

---

## VibeUI: Benchmark Runner (Drawers Tab)

The VibeUI **OpenMemory** panel → **Drawers** tab includes a live benchmark runner at the bottom.

### Running the benchmark

1. Open the **OpenMemory** panel (AI sidebar).
2. Click the **Drawers** tab.
3. Scroll to **LongMemEval Benchmark**.
4. Set `k` using the number input (default: 5).
5. Click **Run**.

The results appear in seconds:

**Three recall gauges** — animated bars for Cognitive, Verbatim, and Combined.

```
Cognitive (L2)        78%  ████████████████████░░░░░
Verbatim  (L3)        90%  ███████████████████████░░
Combined              96%  ████████████████████████████████
```

**Summary line** — `15/20 cognitive hits · 18/20 verbatim hits · 47 memories · 132 drawers`

**Per-case table** — each probe case shows sector, truncated query, and a green **✓ cognitive** / **✓ verbatim** or grey **✗** for each layer. Hover over a query for the full text.

### Iterating in the UI

After running the benchmark, if the score is below your target:

1. Stay in the Drawers tab, scroll to the **Context Preview** section.
2. Type the failing probe query and click **Preview**.
3. Inspect whether the expected answer appears in L2 scoped or L3 verbatim.
4. If absent from both: switch to the **Memories** tab and add a targeted memory.
5. Re-run the benchmark to confirm improvement.

---

## Demo Recording

```json
{
  "meta": {
    "title": "Memory Benchmarking — LongMemEval Recall@K",
    "description": "Run the built-in LongMemEval benchmark in CLI and VibeUI, interpret results, and iterate to reach 100% combined recall.",
    "duration_seconds": 180,
    "version": "1.0.0"
  },
  "steps": [
    {
      "id": 1,
      "action": "repl",
      "commands": [
        { "input": "/openmemory benchmark", "delay_ms": 5000 }
      ],
      "description": "Run recall@5 benchmark — see per-case hit/miss table"
    },
    {
      "id": 2,
      "action": "repl",
      "commands": [
        { "input": "/openmemory benchmark 10", "delay_ms": 4000 }
      ],
      "description": "Increase k to 10 — cognitive recall improves"
    },
    {
      "id": 3,
      "action": "repl",
      "commands": [
        { "input": "/openmemory add \"User prefers Nextest (cargo-nextest) for Rust tests\"", "delay_ms": 3000 },
        { "input": "/openmemory chunk file:.config/nextest.toml", "delay_ms": 3000 },
        { "input": "/openmemory benchmark", "delay_ms": 5000 }
      ],
      "description": "Fix a failing probe case and rerun — combined recall reaches 100%"
    },
    {
      "id": 4,
      "action": "vibeui",
      "panel": "OpenMemory",
      "tab": "Drawers",
      "description": "Open the Drawers tab in VibeUI — same benchmark runner with animated gauges"
    }
  ]
}
```

## What's Next

- [Demo 61: Verbatim Drawers & MemPalace](../61-memory-drawers/) — Learn how drawers are ingested and organised
- [Demo 48: OpenMemory Engine](../48-open-memory/) — Core cognitive memory: sectors, consolidation, knowledge graph
- [Memory Guide](../../memory-guide/) — Full reference for all three memory layers
