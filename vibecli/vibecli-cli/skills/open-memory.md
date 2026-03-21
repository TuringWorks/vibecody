# OpenMemory — Cognitive Memory Engine

## Overview
VibeCody's OpenMemory is a bio-inspired cognitive memory engine for AI agents, inspired by TuringWorks/OpenMemory but significantly exceeding its capabilities. It provides persistent, structured long-term memory with five cognitive sectors, associative graph linking, temporal knowledge graph, and composite scoring.

## Key Advantages Over OpenMemory

| Feature | OpenMemory | VibeCody |
|---------|-----------|----------|
| Sector classification | Regex patterns | TF-IDF + keyword scoring with online learning |
| Associative graph | Single-link (1 per node) | Multi-waypoint (top-5 per node) |
| Vector search | Brute-force cosine | HNSW approximate nearest neighbor |
| Encryption at rest | Not implemented | AES-256-GCM with key derivation |
| Memory consolidation | None | Sleep-cycle merging of weak memories |
| Embeddings | External API required | Local TF-IDF (zero external deps) |
| Temporal graph | Basic validity windows | Bi-temporal + point-in-time queries |
| Scoping | user_id only | User + project + workspace |
| Integration | VS Code extension | CLI REPL + VibeUI panel + agent loop |

## Five Cognitive Sectors

- **Episodic** — Events and experiences (decay: 0.015/day, weight: 1.2)
- **Semantic** — Facts and knowledge (decay: 0.005/day, weight: 1.0)
- **Procedural** — How-to and processes (decay: 0.008/day, weight: 1.1)
- **Emotional** — Sentiment and reactions (decay: 0.020/day, weight: 1.3)
- **Reflective** — Meta-cognition and insights (decay: 0.001/day, weight: 0.8)

## REPL Commands

```
/openmemory                         — Show help + stats
/openmemory add <content>           — Store memory (auto-classified)
/openmemory query <text>            — Semantic search
/openmemory list                    — List all memories
/openmemory fact <subj> <pred> <obj>— Add temporal fact
/openmemory facts                   — Show current facts
/openmemory decay                   — Run exponential decay
/openmemory consolidate             — Merge similar weak memories
/openmemory export                  — Export as markdown
/openmemory context [query]         — Get agent context injection
/openmemory encrypt                 — Encryption setup info
```

## Architecture

### Composite Scoring
Queries use 5-signal composite scoring (configurable weights):
- **Similarity** (0.45) — TF-IDF cosine similarity
- **Salience** (0.20) — Effective salience after decay
- **Recency** (0.15) — Exponential recency boost
- **Waypoint** (0.10) — 1-hop graph expansion similarity
- **Sector match** (0.10) — Sector alignment with query

### Memory Lifecycle
1. **Add** — Content auto-classified, embedded, waypoints created
2. **Query** — Composite scoring with graph expansion
3. **Reinforce** — Accessed memories get salience boost (+0.1)
4. **Decay** — Exponential decay on last_seen_at interval
5. **Consolidate** — Similar weak memories merged (sleep cycle)
6. **Purge** — Memories below 5% effective salience removed

### Temporal Knowledge Graph
- Facts have `valid_from` / `valid_to` windows
- New facts with same subject+predicate auto-close previous ones
- Point-in-time queries: "What was true on date X?"

### Storage
- CLI: `~/.local/share/vibecli/openmemory/` (memories.json, waypoints.json, temporal_facts.json)
- VibeUI: `~/.local/share/vibeui/openmemory/`
- Encryption: AES-256-GCM with passphrase-derived key

## Agent Integration
Use `store.get_agent_context(query, max)` to inject relevant memories into system prompts:
```xml
<open-memory>
[semantic | sal:85% | score:0.72] The API uses REST with JSON payloads...
[procedural | sal:90% | score:0.68] Deploy using: kubectl apply -f deploy.yaml...
--- temporal facts ---
project uses React 18
database is PostgreSQL 16
</open-memory>
```

## Examples

### Store a procedural memory
```
/openmemory add Step 1: Run cargo build --release. Step 2: Copy binary to /usr/local/bin.
```

### Track evolving facts
```
/openmemory fact project framework React-17
/openmemory fact project framework React-18
/openmemory facts  # Only shows React-18 (17 auto-closed)
```

### Search with composite scoring
```
/openmemory query deployment process for production
```
