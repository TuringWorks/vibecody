# OpenMemory — Cognitive Memory Engine + MemPalace Verbatim Drawers

## Overview

VibeCody's OpenMemory is a bio-inspired cognitive memory engine for AI agents. It provides persistent, structured long-term memory with five cognitive sectors, associative graph linking, temporal knowledge graph, composite scoring, and—since the MemPalace integration—a lossless verbatim drawer layer that achieves 96.6% Recall@5 on LongMemEval benchmarks.

## Comparison with Other Memory Systems

| Feature | Mem0 / Zep | VibeCody OpenMemory |
|---------|-----------|---------------------|
| Sector classification | Flat tags | TF-IDF + 5-sector auto-classifier |
| Associative graph | Single-link | Multi-waypoint (top-5 per node), cross-project Tunnels |
| Vector search | External embedding API | Local TF-IDF + HNSW ANN (zero external deps) |
| Encryption at rest | Not implemented | AES-256-GCM with key derivation |
| Memory consolidation | None | Sleep-cycle merging + reflective generation |
| Temporal graph | Basic tags | Bi-temporal (valid_from/valid_to + recorded_at) |
| Verbatim storage | None | 800-char chunks, FNV-1a dedup, cosine near-dedup |
| Retrieval layers | 1 | L0 identity / L1 essential story / L2 scoped / L3 verbatim |
| Benchmarking | None | LongMemEval recall@K built-in |
| Scoping | user_id only | User + project (Wing) + sector (Room) |

## Five Cognitive Sectors

| Sector | Purpose | Decay rate | Weight |
|--------|---------|------------|--------|
| **Episodic** | Events and experiences | 0.015/day | 1.2 |
| **Semantic** | Facts and knowledge | 0.005/day | 1.0 |
| **Procedural** | How-to and workflows | 0.008/day | 1.1 |
| **Emotional** | Sentiment and reactions | 0.020/day | 1.3 |
| **Reflective** | Meta-cognition, auto-generated | 0.001/day | 0.8 |

Classification is automatic from content. Pinned memories are exempt from decay.

## Composite Retrieval Score

```
score = 0.45 × semantic_similarity   (TF-IDF cosine, HNSW)
      + 0.20 × salience               (effective after decay)
      + 0.15 × recency                (exponential boost)
      + 0.10 × waypoint_graph_score   (1-hop expansion)
      + 0.10 × sector_match_bonus     (sector alignment)
```

## 4-Layer Context Architecture

| Layer | Token budget | Always loaded? | Content |
|-------|-------------|----------------|---------|
| L0 | ~100 | Yes | Identity header (user profile) |
| L1 | ~700 | Yes | Essential story — highest-salience memories across all sectors |
| L2 | query-dependent | When relevant | Wing/Room-scoped semantic search, top-8 |
| L3 | fallback + drawer hits | When L2 < 3 | Full semantic search + verbatim raw chunks |

## REPL Commands

### Core operations
```
/openmemory add <content>            — Store a memory (auto-classifies sector)
/openmemory query <text>             — Semantic search with composite scoring
/openmemory search <text>            — Alias for query
/openmemory list                     — List all memories
/openmemory stats                    — Count by sector, storage size, encryption status
/openmemory health                   — Full health dashboard (diversity, at-risk, staleness)
/openmemory at-risk                  — Memories near purge threshold (salience ≤ 0.15)
/openmemory dedup [threshold]        — Remove near-duplicate memories (default: 0.92)
```

### Temporal knowledge graph
```
/openmemory fact <subj> <pred> <obj> — Add temporal fact (auto-closes previous same-key fact)
/openmemory facts                    — Browse active and closed facts
```

### Memory lifecycle
```
/openmemory decay                    — Run exponential salience decay manually
/openmemory consolidate              — Sleep-cycle: merge weak memories, reinforce accessed
/openmemory reflect                  — Generate one-off reflective memory
/openmemory summary                  — User memory profile (top sectors, dominant tags)
```

### Import / export
```
/openmemory export                   — Dump all memories as markdown
/openmemory import [fmt] <file>      — Import from mem0 / Zep / native JSON (auto-detect)
/openmemory ingest <file>            — Chunk + store document as cognitive memories (400-char)
/openmemory encrypt                  — Enable AES-256-GCM encryption at rest
```

### 4-layer context
```
/openmemory context [query]          — Preview L1+L2+L3 context block the agent receives
/openmemory layered [query]          — Same as context (explicit name)
```

### MemPalace verbatim drawers
```
/openmemory chunk <text>             — Ingest raw text as 800-char verbatim chunks
/openmemory chunk file:<path>        — Ingest file as verbatim chunks
/openmemory drawers                  — Drawer store stats: total, Wing/Room distribution, dedup
/openmemory tunnel <id1> <id2> [w]   — Cross-project waypoint (bidirectional, weight 0–1)
/openmemory auto-tunnel [threshold]  — Auto-detect similar memories across stores, create Tunnels
/openmemory benchmark [k]            — LongMemEval recall@K (default k=5)
```

## Storage

| Surface | Path | Files |
|---------|------|-------|
| VibeCLI global | `~/.local/share/vibecli/openmemory/` | memories.json, waypoints.json, facts.json, drawers.json |
| VibeCLI project | `<workspace>/.vibecli/openmemory/` | same |
| VibeUI | `~/.local/share/vibeui/openmemory/` | same |

## Architecture: Verbatim Drawers (MemPalace Technique)

Drawers store text losslessly — no LLM summarisation. Chunk parameters:

- **Chunk size**: 800 characters
- **Overlap**: 100 characters (no context lost at boundaries)
- **Dedup**: FNV-1a hash for exact duplicates; cosine ≥ 0.85 within 20-item sliding window for near-duplicates
- **Wing**: project namespace (maps to `project_id`)
- **Room**: memory sector (maps to `MemorySector`)
- **Embeddings**: Local TF-IDF — no external API required

Before vector search, Wing+Room filter reduces scan space. This is the primary source of Recall@5 gains over pure cognitive stores.

## Architecture: Cross-Project Tunnels

A Tunnel is a named bidirectional waypoint between two memories in different project stores. When the salience graph traverses the tunnel weight, semantically related memories from a different project can surface. Created manually with `tunnel` or automatically with `auto-tunnel`.

## Agent Integration

Use `store.get_layered_context_default(query)` in the agent loop to get the full 4-layer context block:

```rust
let ctx = store.get_layered_context_default(&task);
// Returns: <open-memory><!-- L1 -->\n...\n<!-- L2 -->\n...\n<!-- L3 -->\n...</open-memory>
```

The block is injected into the system prompt before the first assistant turn. Tune layer budgets:

```rust
store.get_layered_context(query, l1_tokens, l2_limit, l3_threshold)
// l1_tokens:     700   (Essential Story token cap)
// l2_limit:      8     (max L2 semantic results)
// l3_threshold:  3     (L2 results below this triggers L3 fallback)
```

## Examples

### Build memory over a session
```
/openmemory add "User prefers Rust for backend services"
/openmemory add "Deploy: cargo build --release → docker push → kubectl apply"
/openmemory add "Debugged deadlock in payment service 2026-04-12"
```

### Ingest a document verbatim
```
/openmemory chunk file:docs/deploy-runbook.txt
/openmemory chunk file:docs/architecture.md
```

### Track evolving facts
```
/openmemory fact project framework React-17
/openmemory fact project framework React-18
/openmemory facts  # Shows React-18 active; React-17 auto-closed
```

### Preview agent context
```
/openmemory layered "how do I deploy?"
# Shows L1 essential story + L2 scoped results + L3 verbatim chunks
```

### Run recall benchmark
```
/openmemory benchmark      # k=5
/openmemory benchmark 10   # k=10 for deeper recall
```

### Auto-link related projects
```
/openmemory auto-tunnel        # threshold 0.75
/openmemory auto-tunnel 0.85   # stronger links only
```

## LongMemEval Benchmark Interpretation

| Combined Recall@5 | Interpretation |
|-------------------|----------------|
| ≥ 96% | Excellent — drawers well-populated, cognitive store dense |
| 85–95% | Good — typical for a store with 20–100 memories |
| 70–84% | Fair — add more specific memories or ingest key documents |
| < 70% | Poor — store likely sparse; run `/openmemory ingest` on core docs |
