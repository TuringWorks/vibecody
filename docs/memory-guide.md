---
layout: page
title: Memory Guide
permalink: /memory-guide/
---

Complete reference for all memory systems in VibeCLI and VibeUI — from the simple auto-recording file to the full cognitive engine with verbatim drawers, cross-project tunnels, and recall benchmarking.

---

## Overview — Three Memory Layers

VibeCody provides three distinct memory systems that complement each other:

| System | Where it lives | Best for |
|--------|----------------|----------|
| **Auto-recording** (`memory.md`) | `~/.vibecli/memory.md` | Simple per-session learnings, no extra setup |
| **OpenMemory** (cognitive store) | `~/.local/share/vibecli/openmemory/` | Structured, searchable, long-term memories |
| **Verbatim Drawers** (MemPalace) | Same store, `drawers.json` | Lossless raw-text recall, highest fidelity |

All three feed into the agent context automatically — you never have to manually pass memory to a prompt.

---

## 1 — Auto-Recording (`memory.md`)

The simplest layer. After an agent session with at least N tool-use steps, VibeCLI summarises what it learned and appends it to `~/.vibecli/memory.md`. The file is injected verbatim into every future system prompt.

### Configuration

```toml
# ~/.vibecli/config.toml

[memory]
auto_record = true          # Enable automatic session recording
min_session_steps = 3       # Minimum tool-use steps before a session is recorded
```

### How it works

1. You run `vibecli` and complete a coding task.
2. The agent makes 3+ tool calls (edit, bash, search, …).
3. At session end, a summary like the following is appended:

```markdown
## 2026-04-12 — Session: payment-service refactor
- Moved database connection pooling to a lazy-static OnceLock.
- All integration tests pass; unit tests for PaymentRepo were added.
- User prefers short commit messages with a verb prefix (fix:, feat:, chore:).
```

1. Next session, the entire file loads into the system prompt before the first message.

### Manual management

```bash
# View current memory file
cat ~/.vibecli/memory.md

# Clear memory (keeps the file, empties it)
echo "" > ~/.vibecli/memory.md

# Edit manually — add or remove anything
$EDITOR ~/.vibecli/memory.md
```

> **Tip:** Keep `memory.md` under ~2 000 tokens. Large files slow down each request because the whole file is re-sent every turn. Use OpenMemory for long-lived or large knowledge sets.

---

## 2 — OpenMemory Cognitive Engine

OpenMemory is a bio-inspired, five-sector cognitive memory engine. It stores memories as vector-embedded nodes, links them into an associative graph, decays them over time unless reinforced, and retrieves them with a composite score that weighs similarity, salience, recency, graph position, and sector match.

### The Five Cognitive Sectors

| Sector | Purpose | Decay rate | Example |
|--------|---------|------------|---------|
| **Episodic** | Events and interactions | Fast | "Fixed a race condition in payment service, 2026-04-12" |
| **Semantic** | Facts and knowledge | Slow | "User prefers Rust over Go for backend work" |
| **Procedural** | Workflows and how-tos | Moderate | "Deploy: cargo build --release → docker push → kubectl apply" |
| **Emotional** | Sentiment and preferences | Fast | "User was frustrated by flaky CI timeouts" |
| **Reflective** | Auto-generated meta patterns | Slow | "User frequently debugs async issues on Tokio runtimes" |

Classification is automatic — the engine reads content and picks the best-fit sector.

### Composite Retrieval Score

Every memory is ranked by five weighted factors:

```text
score = 0.45 × semantic_similarity
      + 0.20 × salience
      + 0.15 × recency
      + 0.10 × waypoint_graph_score
      + 0.10 × sector_match_bonus
```

### Storage paths

| Surface | Path |
|---------|------|
| VibeCLI | `~/.local/share/vibecli/openmemory/` |
| VibeUI | `~/.local/share/vibeui/openmemory/` |
| Project-scoped | `<workspace>/.vibecli/openmemory/` |

Each store contains: `memories.json`, `waypoints.json`, `facts.json`, `drawers.json`.

### Configuration

```toml
# ~/.vibecli/config.toml

[openmemory]
enabled = true
auto_inject = true              # Inject context into every agent turn
max_context_tokens = 1200       # Hard cap on injected context
decay_enabled = true            # Run salience decay each session
consolidate_on_exit = false     # Run sleep-cycle consolidation when REPL exits
encryption = false              # AES-256-GCM at rest (see /openmemory encrypt)
```

### VibeCLI REPL commands

All memory commands are under the `/openmemory` prefix. Run `/openmemory` with no arguments to show a summary of all available commands and live statistics.

#### Core memory operations

```bash
/openmemory add <content>
```

Store a memory. The engine auto-classifies the sector and builds TF-IDF embeddings.

```bash
/openmemory add "User prefers snake_case for Rust identifiers"
/openmemory add "Refactored auth middleware to remove JWT storage in cookies — compliance requirement"
/openmemory add "Always run cargo clippy --all-targets before committing"
```

---

```bash
/openmemory query <text>
/openmemory search <text>
```

Semantic search. Returns up to 10 results ranked by composite score.

```bash
/openmemory query "deployment workflow"
/openmemory query "what does the user prefer for error handling?"
```

---

```bash
/openmemory list
```

List all memories with sector labels, salience, and tag cloud.

---

```bash
/openmemory stats
```

Show count by sector, storage size, encryption status, association graph density.

---

```bash
/openmemory health
```

Full health dashboard: diversity index, at-risk counts, staleness percentages, decay schedule.

---

```bash
/openmemory at-risk
```

List memories whose salience is near the purge threshold (≤ 0.15). Pin them to save or let them decay.

---

```bash
/openmemory dedup [threshold]
```

Remove near-duplicate memories. Default cosine threshold: 0.92. Prefer the higher-salience copy.

```bash
/openmemory dedup         # use default 0.92
/openmemory dedup 0.85    # broader dedup
```

---

#### Knowledge graph (temporal facts)

Facts are explicit subject-predicate-object triples with validity windows. Adding a new fact for the same subject+predicate automatically closes the previous one.

```bash
/openmemory fact <subject> <predicate> <object>
/openmemory fact user prefers_language Rust
/openmemory fact deploy uses_tool "docker + kubectl"
/openmemory fact payment_service last_debugged "2026-04-12 race condition"
```

```bash
/openmemory facts
```

Browse all active and closed facts. Closed facts show `[CLOSED yyyy-mm-dd]` and the superseding fact ID.

---

#### Memory lifecycle

```bash
/openmemory decay
```

Manually trigger the exponential decay cycle. Memories not accessed since last decay lose salience. Pinned memories are exempt.

```bash
/openmemory consolidate
```

Run the sleep-cycle consolidation pass:

- Merges near-duplicate memories (cosine ≥ 0.92).
- Reinforces frequently accessed memories.
- Generates a new Reflective memory summarising patterns.

```bash
/openmemory reflect
```

Immediately generate a one-off Reflective summary of current memory contents.

```bash
/openmemory summary
```

Show the user memory profile: top sectors, most-accessed memories, dominant tags, inferred preferences.

---

#### Import / Export / Migration

```bash
/openmemory export
```

Export all memories as a markdown file to stdout (redirectable to a file).

```bash
/openmemory export > my-memories.md
```

```bash
/openmemory import <file>
/openmemory import [mem0|zep|openmemory|auto] <file>
```

Import from a JSON file. The `auto` format detector recognises mem0, Zep, and native OpenMemory exports. Duplicates are skipped via FNV-1a hash comparison.

```bash
/openmemory import mem0 exported-memories.json
/openmemory import auto ~/backup/memories-2026-04-01.json
```

---

#### Document ingestion

```bash
/openmemory ingest <file>
```

Chunk a document into 400-character overlapping segments and store each chunk as a Semantic memory. Use this for long documents like architecture specs, runbooks, or design docs.

```bash
/openmemory ingest docs/architecture.md
/openmemory ingest ~/runbooks/incident-response.txt
```

---

#### Encryption

```bash
/openmemory encrypt
```

Enable AES-256-GCM encryption at rest. All existing memories are re-encrypted in place. The key is stored at `~/.local/share/vibecli/openmemory/.key` (mode 0600). To use a passphrase instead:

```bash
VIBECLI_MEMORY_KEY="$(pass show vibecli/memory)" vibecli
```

---

### 4-Layer Context Retrieval

When the agent runs, OpenMemory assembles context in four layers — from cheapest to richest:

| Layer | Name | Behaviour |
|-------|------|-----------|
| **L0** | Identity | Always included. Fixed user profile header (≤ 100 tokens). |
| **L1** | Essential story | Always included. Top memories by salience across all sectors (≤ 700 tokens). |
| **L2** | Scoped semantic | Query-dependent. Wing/Room-filtered semantic search, top-8 matches. |
| **L3** | Deep fallback + verbatim drawers | Triggered only when L2 returns < 3 matches. Full search + verbatim raw chunks. |

```bash
/openmemory context [query]
/openmemory layered [query]
```

Preview the exact context block the agent would receive for a given query.

```bash
/openmemory context "what is our deploy process?"
/openmemory layered "async rust patterns"
```

Output format:

```xml
<open-memory>
  <!-- L1: Essential Story (salience ≥ 0.60) -->
  User prefers Rust for backend. Deploy uses Docker + kubectl.

  <!-- L2: Scoped (query: deploy process) -->
  [Procedural, 0.91] cargo build --release → docker push → kubectl apply
  [Episodic,   0.72] Debugged deploy pipeline timeout 2026-03-28

  <!-- L3: Verbatim chunks (3 raw drawer hits) -->
  [chunk:runbook-2026-04.txt] Step 4: run smoke tests before promoting to prod ...
</open-memory>
```

---

## 3 — Verbatim Drawers (MemPalace Technique)

Verbatim drawers store raw text in 800-character chunks with 100-character overlap — **no summarisation, no information loss**. They achieve 96.6% Recall@5 on LongMemEval benchmarks compared to ~74% for purely cognitive stores. Use drawers for any text where exact wording matters: runbooks, specs, commit messages, error logs, transcripts.

### Wing / Room spatial scoping

Drawers are organised by **Wing** (project namespace) and **Room** (memory sector). Before running a vector search, the engine filters by Wing+Room, dramatically reducing search space on large stores:

```text
Wing: "payment-service"   →  project namespace
Room: "procedural"        →  sector within the project
```

This maps directly to VibeCLI's concept of a project-scoped store.

### Commands

```bash
/openmemory chunk <text>
/openmemory chunk file:<path>
```

Ingest text as verbatim 800-char chunks. Exact duplicates (FNV-1a hash) are silently dropped. Near-duplicates (cosine ≥ 0.85) within a 20-item sliding window are also skipped.

```bash
/openmemory chunk "The incident on 2026-04-01 was caused by a missing index on payments.amount..."
/openmemory chunk file:docs/architecture.md
/openmemory chunk file:~/runbooks/deploy-runbook.txt
```

---

```bash
/openmemory drawers
```

Show drawer store statistics: total chunks, Wing distribution, Room distribution, dedup hit rate.

---

```bash
/openmemory tunnel <src-memory-id> <dst-memory-id> [weight]
```

Create a cross-project waypoint (Tunnel) between two memories. Tunnels are bidirectional and survive across store reloads. Weight defaults to 0.8.

```bash
/openmemory tunnel mem_a3f8c1d2 mem_b7e4f091 0.9
```

---

```bash
/openmemory auto-tunnel [threshold]
```

Automatically detect semantically similar memories across the default store and the current project-scoped store, and create Tunnel waypoints for pairs above the similarity threshold (default: 0.75).

```bash
/openmemory auto-tunnel
/openmemory auto-tunnel 0.80
```

---

### LongMemEval Benchmark

Measure the recall quality of your current memory store across both cognitive and verbatim layers:

```bash
/openmemory benchmark [k]
```

Runs 20 built-in probe cases across all 5 sectors, reports Recall@K for cognitive (L2), verbatim (L3), and combined layers.

```bash
/openmemory benchmark        # k=5 (default)
/openmemory benchmark 10     # k=10
```

Example output:

```text
LongMemEval Benchmark Results (k=5)
  Total memories: 47   Verbatim drawers: 132   Probe cases: 20

  Recall@5 — Cognitive (L2):  78.0%  ████████████████████████░░░░░░
  Recall@5 — Verbatim  (L3):  90.0%  ██████████████████████████████
  Recall@5 — Combined:        96.0%  ████████████████████████████████

  Per-case breakdown:
    episodic    What was the last project I worked on?         ✓ cognitive  ✓ verbatim
    semantic    What programming languages do I know?          ✓ cognitive  ✓ verbatim
    procedural  How do I run the test suite?                   ✓ cognitive  ✓ verbatim
    preference  What coding style does the user prefer?        ✗ cognitive  ✓ verbatim
    ...
```

---

## VibeUI — Memory Panels

The VibeUI desktop app exposes memory through four dedicated panels.

### OpenMemory Panel

The primary memory management UI. Access via the **AI** sidebar → **OpenMemory** tab.

| Tab | Contents |
|-----|----------|
| **Overview** | Memory counts, sector distribution bar chart, 4-column stats (memories / waypoints / facts / drawers) |
| **Memories** | Paginated list with sector filter, salience bar, pin/unpin/delete actions |
| **Query** | Natural language semantic search with scored result cards |
| **Facts** | Temporal knowledge graph — active and closed facts with validity windows |
| **Graph** | D3 force-directed graph of memory associations and waypoints |
| **Drawers** | Verbatim drawer stats by Wing/Room, 4-layer context preview, LongMemEval benchmark runner |
| **Settings** | Encryption toggle, decay rate sliders, import/export, clear all |

#### Drawers tab — Benchmark Runner

The Drawers tab includes a live benchmark panel:

1. Set **k** (recall depth, 1–20).
2. Click **Run**.
3. Three recall-percentage gauges appear instantly: Cognitive, Verbatim, Combined.
4. Scroll down for the per-case hit/miss table — green **✓** for a hit, grey **✗** for a miss.

### ChatMemoryPanel

Visible inside the Chat panel as a collapsible sidebar. Shows facts extracted from the current conversation in real time: pin facts to the long-term store, edit wording, or delete before they're persisted.

### SessionMemoryPanel

In the Session panel header. Tracks memory health for the current session: token budget consumed by context injection, staleness warnings, and a sparkline of salience over time.

### MemoryPanel

A standalone rules-and-facts editor. Add, edit, or delete persistent user rules (always use `tracing::` not `println!`, always run clippy before commit, etc.). Rules are injected into every agent system prompt ahead of memory context.

---

## Agent Context Integration

All three memory systems feed into agent context automatically. You can tune the injection behaviour per-session:

### VibeCLI flags

```bash
# Disable memory injection for this session
vibecli --no-memory

# Use a specific project store (project-scoped Wing)
vibecli --memory-scope ./payment-service

# Show what context would be injected without running
vibecli --dry-run-memory "what is our testing strategy?"
```

### In the REPL

```bash
/openmemory context <your question>
```

Preview exactly what the agent will see before you ask.

---

## Quick-Reference Cheat Sheet

```bash
# ── Core ──────────────────────────────────────────────────────────────
/openmemory add <text>              Store a memory (auto-classifies sector)
/openmemory query <text>            Semantic search
/openmemory list                    List all memories
/openmemory stats                   Counts, storage, encryption status
/openmemory health                  Full health dashboard
/openmemory at-risk                 Memories near purge threshold
/openmemory dedup [thresh]          Remove near-duplicate memories

# ── Knowledge Graph ───────────────────────────────────────────────────
/openmemory fact <s> <p> <o>        Add temporal fact (closes previous)
/openmemory facts                   Browse active + closed facts

# ── Lifecycle ─────────────────────────────────────────────────────────
/openmemory decay                   Run salience decay manually
/openmemory consolidate             Sleep-cycle consolidation
/openmemory reflect                 One-off reflective summary
/openmemory summary                 User memory profile

# ── Import / Export ───────────────────────────────────────────────────
/openmemory export                  Dump as markdown
/openmemory import [fmt] <file>     Import from mem0 / Zep / native JSON
/openmemory ingest <file>           Chunk & store document (cognitive)
/openmemory encrypt                 Enable AES-256-GCM encryption

# ── 4-Layer Context ───────────────────────────────────────────────────
/openmemory context [query]         Preview agent context (L1+L2+L3)
/openmemory layered [query]         Same as context

# ── MemPalace (Verbatim Drawers) ──────────────────────────────────────
/openmemory chunk <text|file:path>  Verbatim 800-char ingest
/openmemory drawers                 Drawer stats (Wing/Room distribution)
/openmemory tunnel <id1> <id2> [w]  Cross-project waypoint
/openmemory auto-tunnel [thresh]    Auto-detect and create tunnels

# ── Benchmark ─────────────────────────────────────────────────────────
/openmemory benchmark [k]           LongMemEval recall@K
```

---

## Operations

### Where memory lives on disk

| Layer | Path |
|---|---|
| Auto-recording | `~/.vibecli/memory.md` |
| OpenMemory store | `~/.local/share/vibecli/openmemory/` (Linux) / `~/Library/Application Support/vibecli/openmemory/` (macOS) / `%APPDATA%\vibecli\openmemory\` (Windows) |
| Verbatim drawers | nested under the OpenMemory store directory |
| Encryption passphrase | `~/.vibecli/profile_settings.db` (encrypted ProfileStore — never plaintext) |

The OpenMemory directory contains JSON serializations of memories, facts, lineage edges, and the embedding index. It's safe to commit to a private backup, but **do not commit the ProfileStore** — that file is machine-bound by design.

### Readiness signal — `/health.memory`

The daemon's `/health` endpoint exposes a canonical memory readiness block. Use this from monitoring dashboards, scripts, or feature gates instead of probing each panel:

```bash
curl http://127.0.0.1:7878/health | jq '.memory'
```

```json
{
  "enabled": true,
  "store_path": "/Users/me/Library/Application Support/vibecli/openmemory",
  "total_memories": 1247,
  "drawer_count": 38,
  "encryption_enabled": true
}
```

A feature that depends on "memory has data" should check `memory.enabled && memory.total_memories > N` rather than calling `/memory/list` directly.

### Backup and restore

The OpenMemory store is plain JSON files — copy them anywhere:

```bash
# Backup
tar czf vibecli-memory-$(date +%F).tar.gz "$HOME/Library/Application Support/vibecli/openmemory"

# Restore (overwrites existing memories)
tar xzf vibecli-memory-2026-05-01.tar.gz -C "$HOME/Library/Application Support/vibecli/"
```

If the store is encrypted, you'll also need the passphrase. The passphrase lives in the encrypted ProfileStore; back it up via:

```bash
vibecli list-keys                                 # confirm openmemory_passphrase is set
# Re-set the same passphrase on a new machine:
vibecli set-key openmemory_passphrase '<your-passphrase>'
```

### Migration between machines

```bash
# On the old machine:
curl http://127.0.0.1:7878/memory/export > memories.md       # human-readable
curl http://127.0.0.1:7878/memory/list   > memories.json     # full snapshot

# On the new machine — re-set the encryption passphrase first if any:
vibecli set-key openmemory_passphrase '<your-passphrase>'
# Then import:
curl -X POST http://127.0.0.1:7878/memory/import \
     -H 'content-type: application/json' \
     -d "{\"format\":\"openmemory\",\"content\":$(cat memories.json | jq -Rs .)}"
```

### Export formats supported

The `/memory/import` route accepts four formats. Use `format: "auto"` to detect:

- **openmemory** — VibeCody's native JSON. Round-trip safe; preserves salience, pin state, encryption flags.
- **mem0** — popular open-source memory format. Lossy — sector classification re-runs on import.
- **zep** — temporal-knowledge-graph format. Imports as facts.
- **markdown** — paste any markdown; each `## ` heading becomes a memory.

---

## Troubleshooting

### "Memories I added aren't surfacing in agent context"

The 4-layer retrieval combines L1 (working set), L2 (recent + reinforced), L3 (semantic recall), L4 (episodic). For a memory to surface in L3, it needs:
- A non-zero similarity score to the query (run `/openmemory query <text>` to verify)
- Effective salience above the recall threshold (default 0.1 — see `[memory.recall]` config)

If the memory isn't in the query results: check the sector. Memories in `procedural` and `semantic` rank higher for technical questions; `episodic` ranks higher for "what did we do last week?".

If the memory IS in query results but not in agent context: check the layer caps — L3 defaults to 5 memories. Increase `max_l3` in `[memory.context]` config.

### "Layered context returns empty even with memories present"

Check `/health.memory.enabled`. If `false`, the store didn't load — see permissions on the store directory.

```bash
curl -s http://127.0.0.1:7878/health | jq '.memory'
```

If `enabled: true` but `total_memories: 0`, you're looking at the wrong store. The store path is shown in `store_path` — check that it matches what your agent is reading.

### "LongMemEval recall@K is lower than expected"

Recall@K depends on the embedding backend, the corpus size, and the salience distribution. Common causes:

- **Cold cache** — the first query after restart loads the index from disk. Run a few queries first, then benchmark.
- **TurboQuant compression artifacts** — high compression ratios (>10×) can degrade recall by 1-3 percentage points. Fall back to `embedding_backend: "f32"` in config if accuracy matters more than RAM.
- **Insufficient training memories** — recall@K below 0.6 with <50 memories is normal; the index needs density to discriminate.

### "Encryption was enabled but reads return garbage"

The passphrase changed since the store was last written. Recover by:

1. Re-set the original passphrase: `vibecli set-key openmemory_passphrase '<original>'`
2. Restart the daemon so `load_memory_store()` re-applies the passphrase.
3. If the original passphrase is genuinely lost: the store is unrecoverable. Restore from backup, or delete the openmemory directory to start fresh.

### "Permission denied on the store directory"

The daemon couldn't write to `~/.local/share/vibecli/openmemory/` (Linux) or the equivalent on your platform. Fix:

```bash
# Linux
chmod -R u+rw "$HOME/.local/share/vibecli"
# macOS
chmod -R u+rw "$HOME/Library/Application Support/vibecli"
```

If you're running the daemon under a different user (e.g. a systemd service), make sure the data directory is writable by that user.

### "Disk full"

Run `/openmemory decay` to purge low-salience memories. If that's not enough, export and re-import with a more aggressive salience threshold:

```bash
curl http://127.0.0.1:7878/memory/export > backup.md
rm -rf "$HOME/Library/Application Support/vibecli/openmemory"
# Re-import only the salient bits manually from backup.md.
```

---

## FAQ

**How long do memories persist?**
Indefinitely, until you delete them or the salience-decay loop reclaims them. Decay runs once a day by default and only purges memories with `effective_salience < 0.05` that haven't been reinforced in 30+ days. Pinned memories are exempt.

**Can I delete a specific memory?**
Yes — via the OpenMemory panel's per-row Delete button, or `/openmemory delete <id>` in the REPL, or `POST /memory/delete` with `{ "id": "..." }`. Deletion is immediate and irreversible.

**What's the encryption overhead?**
The current implementation uses XOR-based stream encryption (lightweight, ~5% throughput overhead). AES-256-GCM is on the roadmap — when it lands, expect ~10-15% throughput overhead and no measurable recall impact.

**Is memory available on mobile / watch?**
No — by design. Memory authoring is a desktop / CLI workflow. Mobile and watch clients consume *generated* artifacts (recaps, sessions) but don't surface the memory store directly. The HTTP routes at `/memory/*` are available to any client that wants them, including IDE plugins and the Agent SDK.

**How do I share memories across teammates?**
The store is single-user by design. To share, export specific memories as markdown and check them into your repo as `docs/team-memory.md`. The auto-recording layer reads any `*.md` file in the workspace at agent-context build time.

**Will my memories sync across devices?**
Not automatically. The store is local-only (Zero-Config First — no cloud dependency). To sync manually, use the export/import flow above with a private file-share (Dropbox, iCloud, etc.) or a private git repo.

**What happens if the JSON store gets corrupted?**
On load failure, `load_memory_store()` falls back to a fresh empty store rather than crashing the daemon. You'll see `enabled: true, total_memories: 0` in `/health` even though you previously had memories. Restore from backup, or accept the loss and move on.

---

## Cross-client scope

| Client | Memory access |
|---|---|
| **VibeUI (desktop, Tauri)** | Full UI: OpenMemoryPanel, ChatMemoryPanel, SessionMemoryPanel, MemoryProjectionsPanel, MemoryPanel |
| **VibeCLI** | Full REPL: `/openmemory *` commands |
| **VibeMobile** | None by current design |
| **VibeWatch (watchOS / Wear OS)** | None by current design |
| **VS Code / JetBrains / Neovim plugins** | HTTP via `/memory/*` routes (not surfaced in plugin UIs today) |
| **Agent SDK** | HTTP via `/memory/*` routes |

If you need a memory UI on a client where it's currently absent, file an issue — the daemon-side API is stable and complete; only the client-side surface is missing.

---

## See Also

- [Demo 48: OpenMemory Cognitive Engine](../demos/48-open-memory/) — Basic walkthrough
- [Demo 61: Verbatim Drawers & MemPalace](../demos/61-memory-drawers/) — Lossless chunk ingestion and cross-project tunnels
- [Demo 62: Memory Benchmarking](../demos/62-memory-benchmark/) — LongMemEval recall@K in CLI and VibeUI
- [Configuration Reference](../configuration/) — `[memory]` and `[openmemory]` config tables
- [API Reference](../api-reference/) — Tauri commands for frontend integration
