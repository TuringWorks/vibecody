# VibeMemory — Local SQLite Vector Memory

## Context

VibeCody currently has:
- `open_memory.rs` — cognitive memory engine with TF-IDF embeddings, sector classification, decay, waypoints
- `vector_db.rs` — abstraction over external vector DBs (Qdrant, Pinecone, etc.)
- `vibe-infer` — in-process inference traits (`Embedder`, `TextGenerator`) with candle backend

Missing: a **local, zero-dependency, in-process vector store** for project and computer contexts. This spec adds it.

---

## Problem Statement

The user wants **OB1-style memory per project and per computer**, using SQLite as the persistence layer. No external services, no API keys, no network. Every project gets its own vector store; every computer gets one global store.

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           VibeCody Daemon (vibecli)                        │
│                                                                             │
│  ┌─────────────┐   ┌──────────────────┐   ┌────────────────────────────┐  │
│  │ open_memory │──▶│ MemoryContextHub │──▶│ vibe-memory (this crate)   │  │
│  │   .rs       │   │                  │   │                            │  │
│  └─────────────┘   └────────┬─────────┘   └───────────┬────────────────┘  │
│                              │                       │                    │
│                    ┌─────────┴─────────┐   ┌────────┴────────┐          │
│                    │  ProjectContext    │   │ GlobalContext   │          │
│                    │  (per-workspace)   │   │  (per-machine)  │          │
│                    └─────────┬─────────┘   └────────┬────────┘          │
│                              │                       │                    │
│                    ┌─────────┴─────────┐   ┌────────┴────────┐          │
│                    │ ProjectMemStore    │   │ GlobalMemStore  │          │
│                    │  sqlite-vec  768d  │   │  sqlite-vec 768d│          │
│                    │ ~/.vibecli/        │   │ ~/.vibecli/      │          │
│                    │ workspaces/{id}/   │   │ memory.db       │          │
│                    │ memory.db           │   │                 │          │
│                    └────────────────────┘   └─────────────────┘          │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Request Flow

```
User query "how does auth work here?"
        │
        ▼
┌─────────────────────────────────┐
│      MemoryContextHub           │
│  (scopes query to project+global)│
└───────────────┬─────────────────┘
                │
        ┌───────┴────────┐
        ▼                ▼
┌──────────────┐  ┌───────────────┐
│ ProjectStore │  │ GlobalStore   │
│  .vibecli/   │  │ ~/.vibecli/   │
│ memory.db    │  │ memory.db     │
└──────┬───────┘  └───────┬───────┘
       │                  │
       ▼                  ▼
   VECTOR SEARCH (top-K cosine)
       │                  │
       └────────┬─────────┘
                ▼
        Merged results ranked by
        salience × recency × sector-weight
                │
                ▼
        <vibe-memory> XML tag in context
```

### Storage Layout

```
~/.vibecli/
├── profile_settings.db       ← encrypted API keys (unchanged)
├── sessions.db                ← session history (unchanged)
└── memory/
    ├── global.db              ← computer-scoped store (all projects share)
    └── workspaces/
        └── {workspace-hash}/
            └── memory.db      ← project-scoped store
```

**Key derivation** mirrors the existing ProfileStore model:
- Global key: `SHA-256("vibememory-global-v1:" + $HOME + ":" + $USER)`
- Project key: `SHA-256("vibememory-project-v1:" + $HOME + ":" + $USER + ":" + workspace_path)`

Both stores use **ChaCha20-Poly1305** for encryption, matching the existing security model.

---

## Module Structure

```
vibe-memory/
├── Cargo.toml
├── src/
│   ├── lib.rs                 ← public API
│   ├── extension.rs           ← SQLite extension selection (vec/vector/lite)
│   ├── project_store.rs       ← ProjectMemStore
│   ├── global_store.rs        ← GlobalMemStore
│   ├── hub.rs                 ← MemoryContextHub (orchestrator)
│   ├── schema.rs              ← table definitions
│   └── error.rs               ← error types
└── tests/
    ├── project_memory.bdd.test.rs
    ├── global_memory.bdd.test.rs
    └── hub_integration.bdd.test.rs
```

---

## BDD Scenarios

### Project Memory Store

```gherkin
Feature: Project Memory Store

  Scenario: Store memory entry with vector embedding
    Given a fresh project workspace at "/tmp/test-project"
    And the SQLite extension "sqlite-vec" is available
    When I store a memory entry with content "Rust ownership prevents data races"
    Then the entry exists in the project store with a valid 768-dim vector
    And the sector is classified as "procedural"
    And the entry ID is a hex timestamp with random suffix

  Scenario: Query project memories by semantic similarity
    Given a project with 3 stored memories about "Rust ownership", "Go concurrency", "Python GIL"
    When I query for "memory safety in systems programming"
    Then the top result is about Rust ownership (highest cosine similarity)
    And results are ranked by salience × recency × sector-weight

  Scenario: Project memory isolated from other projects
    Given project A with memory "Auth service uses JWT"
    And project B with memory "Auth service uses OAuth"
    When I query project A's store for "authentication"
    Then the result is about JWT, not OAuth
    And project B's store is not touched

  Scenario: Encrypted at rest with machine-bound key
    Given a project store at "/tmp/test-project"
    When I store a memory entry
    Then the raw SQLite file contains encrypted vectors
    And decrypting with a wrong key produces garbage
    And decrypting with the correct key restores the entry

  Scenario: Delete memory entry by ID
    Given a project with 5 stored memories
    When I delete one memory by ID
    Then the store contains 4 entries
    And the deleted ID returns None on lookup

  Scenario: List all project memories with metadata
    Given a project with mixed sector memories
    When I list all memories with metadata
    Then each entry includes id, content, sector, salience, created_at, tags

  Scenario: Salience decay over time
    Given a memory entry with salience 1.0 created 7 days ago
    When I calculate the current salience
    Then the value is less than 1.0 due to sector-specific decay

  Scenario: Pin memory prevents decay and purge
    Given a pinned memory with low salience
    When decay is applied
    Then the pinned memory retains its original salience
    And the pinned memory is excluded from purge operations
```

### Global Memory Store

```gherkin
Feature: Global (Computer) Memory Store

  Scenario: Store computer-level context memory
    Given a fresh global store
    When I store a memory about "User prefers dark mode and Rust"
    Then the memory is available to all projects on this machine
    And the sector is classified as "emotional"

  Scenario: Cross-project context retrieval
    Given project A stored "Python dicts are slow"
    And project B stored "Use dataclasses for performance"
    When I query global store for "python performance"
    Then results include both project A and B entries
    And each result includes the source project ID

  Scenario: Global store not affected by project deletion
    Given project A with global memory references
    When project A is deleted
    Then global store entries from project A are preserved
    And other projects can still access global memories

  Scenario: Global memory merges with project memory
    Given project has 2 memories and global has 3 memories
    When I fetch layered context for a query
    Then the results include project memories (higher priority)
    And global memories fill remaining context budget
    And sector weights are applied to final ranking

  Scenario: Computer-level encryption key derivation
    Given I open the global store
    When I write a memory entry
    Then the SQLite file uses the machine-derived key
    And the key differs from any project's store key
    And the key is consistent across VibeCody restarts
```

### Memory Context Hub

```gherkin
Feature: Memory Context Hub (Orchestrator)

  Scenario: Layered context from project + global stores
    Given project store has 3 memories about "API design"
    And global store has 2 memories about "best practices"
    When I query for "REST API design patterns"
    Then I receive merged results with project entries weighted higher
    And sector weights (emotional=1.3, episodic=1.2, procedural=1.1) are applied
    And recency and salience boost are factored

  Scenario: Budget-aware context assembly
    Given 20 relevant memories (total ~8K tokens)
    And context budget is 4K tokens
    When I assemble context for a query
    Then only the top ~4K tokens of memories are included
    And memories are sorted by composite score before truncation

  Scenario: Empty stores return empty context
    Given both project and global stores are empty
    When I assemble context for any query
    Then the result is an empty <vibe-memory> tag
    And no error is raised

  Scenario: Single store populated (project only)
    Given project store has memories
    And global store is empty
    When I assemble context
    Then only project memories are returned
    And no error is raised

  Scenario: Vector search with top-K and min_score filter
    Given a store with 100 memories
    When I search with top_k=5 and min_score=0.75
    Then I receive at most 5 results
    And all results have cosine similarity >= 0.75
    And results are sorted by score descending

  Scenario: Query routing to correct stores
    Given project path "/tmp/myproject"
    When I route a query through the hub
    Then the project store path is derived from "/tmp/myproject"
    And the global store path is ~/.vibecli/memory/global.db
    And both stores are queried in parallel

  Scenario: Hub exposes /api/memory route
    Given the daemon is running
    When GET /api/memory?query=rust+ownership&workspace=/tmp/project
    Then I receive JSON with matched memories and scores
    And the response includes project and global results separately
```

---

## API Design

### Public API (`lib.rs`)

```rust
use std::path::Path;

// ── Stores ─────────────────────────────────────────────────────────────────────

/// Open (or create) the project-scoped memory store.
/// The store is encrypted with a key derived from machine + workspace path.
pub fn open_project_store(workspace: &Path) -> Result<ProjectMemStore>;

/// Open (or create) the global (computer-scoped) memory store.
/// The store is encrypted with a key derived from machine identity.
pub fn open_global_store() -> Result<GlobalMemStore>;

/// The orchestrator — queries both stores and merges results.
pub fn new_memory_hub() -> MemoryContextHub;

// ── Core operations ──────────────────────────────────────────────────────────

/// Store a memory with auto-generated embedding and sector classification.
pub fn store_memory(...)
    -> impl Future<Output = Result<MemoryEntry>> + Send

/// Search by semantic similarity, returns top-K merged results.
pub fn search_context(...)
    -> impl Future<Output = Vec<ContextResult>> + Send

/// Assemble layered context string for injection into LLM prompt.
pub fn assemble_context(...)
    -> impl Future<Output = String> + Send  // returns <vibe-memory>...</vibe-memory>

/// Delete a memory entry.
pub fn delete_memory(...)
    -> impl Future<Output = Result<()>> + Send

/// List all memories with optional sector filter.
pub fn list_memories(...)
    -> impl Future<Output = Vec<MemoryEntry>> + Send

/// Pin/unpin a memory (immune to decay/purge).
pub fn set_pinned(...)
    -> impl Future<Output = Result<()>> + Send

/// Apply decay to all memories and purge salience-below-threshold entries.
pub fn consolidate_memories(...)
    -> impl Future<Output = PurgeReport> + Send
```

---

## SQLite Extension Selection

The crate supports three vector extensions, selected at runtime:

| Extension | Selection criteria | Dimensions | License |
|-----------|-------------------|------------|---------|
| **sqlite-vec** | Default, most portable, WASM-compatible | 1–4096 | BSD-2 |
| **sqlite-vector** | SIMD available (`target_arch = "x86_64"`) | 1–4096 | Apache-2 |
| **vectorlite** | Large-scale (>100K entries) | 1–4096 | MIT |

The `extension.rs` module detects the environment and picks the best option.

---

## Security Model

- **Encryption**: ChaCha20-Poly1305 with per-store random nonces (12 bytes)
- **Key derivation**: SHA-256 of context string (see above)
- **Isolation**: Project store key includes workspace path; global store key is machine-only
- **No plaintext storage**: Vectors and content are encrypted at rest
- **Key storage**: Keys themselves are derived at runtime, never stored

---

## Default Configuration

| Setting | Default | Override via |
|---------|---------|--------------|
| Vector dimensions | 768 | env `VIBE_MEMORY_DIM` |
| Embedding batch size | 32 | env `VIBE_MEMORY_BATCH` |
| Top-K results | 8 | env `VIBE_MEMORY_TOP_K` |
| Decay interval | 24h | env `VIBE_MEMORY_DECAY_HOURS` |
| Purge threshold | 0.1 | env `VIBE_MEMORY_PURGE_THRESHOLD` |
| Context token budget | 4096 | env `VIBE_MEMORY_CONTEXT_BUDGET` |

---

## Change-Surface Checklist

When adding this crate:

| Also touch | Why |
|-----------|-----|
| `vibecli/vibecli-cli/Cargo.toml` | Add workspace member |
| `vibecli/vibecli-cli/src/open_memory.rs` | Integrate via MemoryContextHub |
| `vibecli/vibecli-cli/src/serve.rs` | Add `/api/memory` route |
| `docs/FEATURE-MATRIX.md` | Mark memory feature as implemented |
| `docs/FEATURE-REFERENCE.md` | Document the new API |
| `vibeui/src-tauri/src/commands.rs` | Expose Tauri commands for memory ops |

---

## Out of Scope

- External vector DB integration (already in `vector_db.rs`)
- Cross-machine sync (future work)
- Multi-user shared memory (future work)
- Embedding provider external API (uses local TF-IDF via `vibe-infer`)
