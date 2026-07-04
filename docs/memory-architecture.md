---
layout: page
title: Memory & Context Architecture
permalink: /memory-architecture/
---

This document describes the complete memory and context architecture of VibeCody — from encrypted storage through context assembly to cross-client recap/resume.

**Last Updated:** 2026-05-08  
**Scope:** vibecli, vibeui, vibemobile, vibewatch, plugins, Agent SDK  
**Author:** System Architecture

---

## Table of Contents

1. [Executive Summary](#1-executive-summary)
2. [The Five Memory Stores](#2-the-five-memory-stores)
3. [Storage Security Model](#3-storage-security-model)
4. [Context Assembler](#4-context-assembler)
5. [Memory Projections](#5-memory-projections)
6. [Recap & Resume Architecture](#6-recap--resume-architecture)
7. [Data Flow Diagrams](#7-data-flow-diagrams)
8. [Cross-Client Surface](#8-cross-client-surface)
9. [Configuration Reference](#9-configuration-reference)

---

## 1. Executive Summary

VibeCody's memory system is **operational infrastructure** for three pressures:

1. **Large codebases** — can't fit in context → need retrieval (`vibe-indexer`)
2. **Large context** — finite window → need budgeting + semantic recall
3. **Long-running agents** — durable working state across restarts/pauses

The architecture consists of **five specialized retrievers** (not duplicates), a **Context Assembler** that applies policy-driven budget allocation, and a **Recap/Resume** system for cross-client session continuity.

```text
┌─────────────────────────────────────────────────────────────────────────────┐
│                    MEMORY ARCHITECTURE OVERVIEW                             │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│   ┌──────────────┐     ┌──────────────┐     ┌──────────────┐                │
│   │   Static     │     │    Code      │     │  Extracted   │                │
│   │    Docs      │     │   Symbols    │     │    Facts     │                │
│   │(memory/*.md) │     │(vibe-indexer)│     │(open_memory) │                │
│   └──────┬───────┘     └──────┬───────┘     └──────┬───────┘                │
│          │                    │                    │                        │
│          ▼                    ▼                    ▼                        │
│   ┌─────────────────────────────────────────────────────────────┐           │
│   │                  CONTEXT ASSEMBLER                          │           │
│   │   (policy-driven budget allocation, dedup at assembly)      │           │
│   └─────────────────────────────────────────────────────────────┘           │
│                              │                                              │
│                              ▼                                              │
│   ┌─────────────────────────────────────────────────────────────┐           │
│   │            AGENT / CHAT CONTEXT WINDOW                      │           │
│   └─────────────────────────────────────────────────────────────┘           │
│                                                                             │
│   ┌──────────────┐     ┌────────────────┐                                   │
│   │ Transcripts  │     │     Skills     │                                   │
│   │(sessions.db) │     │(self_improving)│                                   │
│   └──────────────┘     └────────────────┘                                   │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 2. The Five Memory Stores

Each store is specialized for a different content type. They are **not duplicates** — consolidation would lose capability.

### 2.1 Store Matrix

| Store | Content Type | Location | Encryption | Role |
|-------|-------------|----------|------------|------|
| **ProjectMemory** | Static docs (`CLAUDE.md`, `AGENTS.md`, `VIBECLI.md`) | Hierarchical (system/user/project/dir) | None | Constant context floor |
| **OpenMemory** | Extracted facts, preferences, user context | `~/.vibecli/openmemory/` + project-scoped | AES-256-GCM | Fact retriever (TF-IDF + sectors) |
| **SessionStore** | Conversation transcripts | `~/.vibecli/sessions.db` | None (file-perm) | History retriever (FTS5) |
| **JobManager** | Agent job state, scratchpad | `~/.vibecli/jobs.db` | ChaCha20-Poly1305 | Job retriever (durable working state) |
| **vibe-indexer** | Code + symbols | Project `.vibecli/index/` | None | Code retriever (HNSW embeddings) |

### 2.2 Store Architecture Diagram

```text
┌─────────────────────────────────────────────────────────────────────────────┐
│                        FIVE MEMORY STORES                                   │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│   ┌─────────────────────────────────────────────────────────────────────┐   │
│   │                    PROJECT MEMORY (Static)                          │   │
│   │  ┌───────────┐  ┌───────────┐  ┌───────────┐  ┌──────────┐          │   │
│   │  │  System   │─▶│   User    │─▶│  Project  │─▶│ Directory│          │   │
│   │  │/etc/...   │  │~/.vibecli/│  │<git-root> │  │   cwd    │          │   │
│   │  │VIBECLI.md │  │VIBECLI.md │  │CLAUDE.md  │  │VIBECLI.md│          │   │
│   │  └───────────┘  └───────────┘  └───────────┘  └──────────┘          │   │
│   │                    │                                                │   │
│   │                    ▼                                                │   │
│   │        Combined() ─▶ system prompt injection                        │   │
│   └─────────────────────────────────────────────────────────────────────┘   │
│                              │                                              │
│                              ▼                                              │
│   ┌─────────────────────────────────────────────────────────────────────┐   │
│   │                    OPEN MEMORY (Cognitive)                          │   │
│   │                                                                     │   │
│   │  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐             │   │
│   │  │ Episodic │  │ Semantic │  │Procedural│  │Emotional │             │   │
│   │  │ (fast    │  │ (slow    │  │(moderate)│  │ (fast    │             │   │
│   │  │  decay)  │  │  decay)  │  │          │  │  decay)  │             │   │
│   │  └──────────┘  └──────────┘  └──────────┘  └──────────┘             │   │
│   │  ┌──────────────────────────────────────────────────────────┐       │   │
│   │  │                      Reflective                          │       │   │
│   │  │                    (slowest decay)                       │       │   │
│   │  └──────────────────────────────────────────────────────────┘       │   │
│   │                                                                     │   │
│   │  TF-IDF embeddings + HNSW index (TurboQuant ~3 bits/dim)            │   │
│   │  Waypoint graph for associative recall                              │   │
│   │  Temporal facts (S-P-O triples with validity windows)               │   │
│   └─────────────────────────────────────────────────────────────────────┘   │
│                              │                                              │
│                              ▼                                              │
│   ┌─────────────────────────────────────────────────────────────────────┐   │
│   │                   SESSION STORE (History)                           │   │
│   │                                                                     │   │
│   │  SQLite: ~/.vibecli/sessions.db                                     │   │
│   │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐                  │   │
│   │  │  sessions   │  │  messages   │  │    steps    │                  │   │
│   │  │    table    │  │    table    │  │    table    │                  │   │
│   │  │             │  │   + FTS5    │  │             │                  │   │
│   │  │ id, task,   │  │             │  │ tool_name,  │                  │   │
│   │  │ status,     │  │ role,       │  │ input,      │                  │   │
│   │  │ parent_id,  │  │ content,    │  │ output      │                  │   │
│   │  │ depth,      │  │ created_at  │  │             │                  │   │
│   │  │ project_path│  │             │  │             │                  │   │
│   │  └─────────────┘  └─────────────┘  └─────────────┘                  │   │
│   │                                                                     │   │
│   │  Full-text search via FTS5 (Porter tokenizer)                       │   │
│   │  Recaps table for resume (new)                                      │   │
│   └─────────────────────────────────────────────────────────────────────┘   │
│                              │                                              │
│                              ▼                                              │
│   ┌─────────────────────────────────────────────────────────────────────┐   │
│   │                  JOB MANAGER (Durable State)                        │   │
│   │                                                                     │   │
│   │  SQLite: ~/.vibecli/jobs.db (ChaCha20-Poly1305 encrypted)           │   │
│   │  ┌─────────────┐  ┌───────────────┐                                 │   │
│   │  │    jobs     │  │job_scratchpad │                                 │   │
│   │  │    table    │  │    table      │                                 │   │
│   │  │             │  │               │                                 │   │
│   │  │ session_id, │  │ session_id,   │                                 │   │
│   │  │ status,     │  │ key,          │                                 │   │
│   │  │ task,       │  │ encrypted_    │                                 │   │
│   │  │ summary,    │  │ value,        │                                 │   │
│   │  │ priority,   │  │ updated_at    │                                 │   │
│   │  │ steps,      │  │               │                                 │   │
│   │  │ tokens      │  │ (agent working memory)                          │   │
│   │  └─────────────┘  └───────────────┘                                 │   │
│   │                                                                     │   │
│   │  Agent scratchpad: persist tool calls, errors, completions          │   │
│   │  across pause/resume cycles                                         │   │
│   └─────────────────────────────────────────────────────────────────────┘   │
│                              │                                              │
│                              ▼                                              │
│   ┌─────────────────────────────────────────────────────────────────────┐   │
│   │                 VIBE-INDEXER (Code/Symbols)                         │   │
│   │                                                                     │   │
│   │  HNSW vector index with Tree-sitter symbol extraction               │   │
│   │  Embeddings: Local (Ollama) or Cloud (OpenAI)                       │   │
│   │  Incremental updates on file changes                                │   │
│   │                                                                     │   │
│   │  Project-scoped: <workspace>/.vibecli/index/                        │   │
│   └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 3. Storage Security Model

### 3.1 Key Derivation

```text
┌────────────────────────────────────────────────────────────────────────────┐
│                       KEY DERIVATION CHAIN                                 │
├────────────────────────────────────────────────────────────────────────────┤
│                                                                            │
│   ┌─────────────────────────────────────────────────────────────────────┐  │
│   │  Profile Key (machine-bound)                                        │  │
│   │  ───────────────────────────                                        │  │
│   │  SHA-256("vibecli-profile-store-v1:" + $HOME + ":" + $USER)         │  │
│   │                                                                     │  │
│   │  Used for:                                                          │  │
│   │  - profile_settings.db (API keys, panel settings, global config)    │  │
│   │  - jobs.db (encrypted columns: task, summary, webhook_url)          │  │
│   └─────────────────────────────────────────────────────────────────────┘  │
│                                  │                                         │
│                                  ▼                                         │
│   ┌─────────────────────────────────────────────────────────────────────┐  │
│   │  Workspace Key (machine + project-bound)                            │  │
│   │  ────────────────────────────────────────                           │  │
│   │  SHA-256("vibecli-workspace-store-v1:" + $HOME + ":" + $USER + ":" +│  │
│   │         <workspace_path>)                                           │  │
│   │                                                                     │  │
│   │  Used for:                                                          │  │
│   │  - workspace.db (project settings, project secrets)                 │  │
│   │  - openmemory store (when project-scoped)                           │  │
│   └─────────────────────────────────────────────────────────────────────┘  │
│                                                                            │
│   Nonces: 12-byte random per-value, prepended to ciphertext                │
│   Algorithm: ChaCha20-Poly1305 (authenticated encryption)                  │
│                                                                            │
└────────────────────────────────────────────────────────────────────────────┘
```

### 3.2 Storage Hierarchy

```bash
~/.vibecli/
├── profile_settings.db      <- encrypted: API keys, panel settings, global config, master keys
├── company.db               <- company orchestration data (unencrypted)
├── sessions.db              <- agent session history, messages, steps, recaps (unencrypted)
├── jobs.db                  <- encrypted: async job records + scratchpad
├── openmemory/              <- cognitive memory store (encrypted at rest option)
│   ├── memories.json
│   ├── waypoints.json
│   ├── facts.json
│   └── drawers.json
└── config.toml              <- CLI feature flags, provider enable/disable (no keys)

<workspace>/
└── .vibecli/
    ├── workspace.db         <- encrypted: project settings + project secrets
    ├── MEMORY.md            <- auto-generated from OpenMemory (project tier)
    ├── openmemory/          <- project-scoped memory (optional)
    └── index/               <- vibe-indexer HNSW data
```

---

## 4. Context Assembler

The Context Assembler is the single entry point that builds system context from the various memory subsystems under a policy-driven budget.

### 4.1 Assembler Architecture

```text
┌─────────────────────────────────────────────────────────────────────────────┐
│                     CONTEXT ASSEMBLER PIPELINE                              │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│   Input: (workspace, policy, budget, toggles)                               │
│                              │                                              │
│                              ▼                                              │
│   ┌──────────────────────────────────────────────────────────────────────┐  │
│   │                     CONTEXT POLICY                                   │  │
│   │  ┌────────┐  ┌──────────────┐  ┌──────────────┐  ┌──────────┐        │  │
│   │  │  Chat  │  │ CodingAgent  │  │ResearchAgent │  │Background│        │  │
│   │  │        │  │              │  │              │  │   Job    │        │  │
│   │  │project_│  │agent_        │  │agent_        │  │agent_    │        │  │
│   │  │memory +│  │scratchpad +  │  │scratchpad +  │  │scratch-  │        │  │
│   │  │orches- │  │project_      │  │project_      │  │pad       │        │  │
│   │  │tration │  │profile +     │  │profile +     │  │(dominant)│        │  │
│   │  │        │  │task_files +  │  │open_memory   │  │          │        │  │
│   │  │        │  │open_memory   │  │(dominant)    │  │          │        │  │
│   │  └────────┘  └──────────────┘  └──────────────┘  └──────────┘        │  │
│   │                                                                      │  │
│   │  Budget allocation (per-agent-kind defaults):                        │  │
│   │  ┌───────────────┬────────────┬────────────────────────────────────┐ │  │
│   │  │ AgentKind     │ Total Chars│ Section Caps                       │ │  │
│   │  ├───────────────┼────────────┼────────────────────────────────────┤ │  │
│   │  │ Chat          │ 32,000     │ memory: 16k, orch: 8k              │ │  │
│   │  │ CodingAgent   │ 128,000    │ scratchpad: 16k, profile: 8k,      │ │  │
│   │  │               │            │ task_files: 96k, open_memory: 16k. │ │  │
│   │  │ ResearchAgent │ 128,000    │ open_memory: 64k (dominant)        │ │  │
│   │  │ BackgroundJob │ 48,000     │ scratchpad: 40k (dominant)         │ │  │
│   │  └───────────────┴────────────┴────────────────────────────────────┘ │  │ 
│   └──────────────────────────────────────────────────────────────────────┘  │
│                              │                                              │
│                              ▼                                              │
│   ┌─────────────────────────────────────────────────────────────────────┐   │
│   │                    SECTION COLLECTION                               │   │
│   │                                                                     │   │
│   │   For Chat policy:                                                  │   │
│   │   1. Load ProjectMemory (CLAUDE.md hierarchy)                       │   │
│   │   2. Load orchestration lessons + todo state                        │   │
│   │                                                                     │   │
│   │   For Agent policy:                                                 │   │
│   │   1. Load project_profile (project-level memory files)              │   │
│   │   2. Extract task_files (relevant files for task)                   │   │
│   │   3. Query OpenMemory (when toggles.openmemory_enabled)             │   │
│   │   4. Load agent_scratchpad from jobs.db (when job_id provided)      │   │
│   │                                                                     │   │
│   └─────────────────────────────────────────────────────────────────────┘   │
│                              │                                              │
│                              ▼                                              │
│   ┌─────────────────────────────────────────────────────────────────────┐   │
│   │                    BUDGET APPLICATION                               │   │
│   │                                                                     │   │
│   │   1. Sort sections by priority (lower = more important)             │   │
│   │   2. Apply per-section caps first                                   │   │
│   │   3. Apply total cap                                                │   │
│   │   4. Truncate at UTF-8 char boundaries                              │   │
│   │   5. Mark truncated sections                                        │   │
│   │                                                                     │   │
│   └─────────────────────────────────────────────────────────────────────┘   │
│                              │                                              │
│                              ▼                                              │
│   Output: AssembledContext { sections[], total_chars }                      │
│                                                                             │
│   Sections emitted (KNOWN_SECTION_NAMES):                                   │
│   - "project_memory" -- VIBECLI.md hierarchy                                │
│   - "orchestration" -- workflow lessons + active task                       │
│   - "project_profile" -- project-level memory                               │
│   - "task_files" -- relevant file previews                                  │
│   - "open_memory" -- TF-IDF retrieved facts                                 │
│   - "agent_scratchpad" -- durable job state from jobs.db                    │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 4.2 Assembler API

```rust
// Entry point
pub fn assemble_context(
    workspace: &Path,
    policy: &ContextPolicy,
    budget: &ContextBudget,
    toggles: &MemoryToggles,
) -> AssembledContext

// Policies
pub enum ContextPolicy {
    Chat,
    Agent { task: String, job_id: Option<String> },
}

// Agent kinds (affect budget allocation)
pub enum AgentKind {
    Chat,           // 32k total, memory-dominant
    CodingAgent,    // 128k total, task_files-dominant (96k)
    ResearchAgent,  // 128k total, open_memory-dominant (64k)
    BackgroundJob,  // 48k total, scratchpad-dominant (40k)
}

// Budget with per-section overrides
pub struct ContextBudget {
    pub max_total_chars: usize,
    pub max_section_chars: usize,
    pub section_caps: Vec<(&'static str, usize)>,
}

// Toggles (subset of MemoryConfig)
pub struct MemoryToggles {
    pub openmemory_enabled: bool,
    pub openmemory_auto_inject: bool,
    pub jobs_db_path: Option<PathBuf>,  // for scratchpad access
}

// Output
pub struct AssembledContext {
    pub sections: Vec<ContextSection>,
    pub total_chars: usize,
}

impl AssembledContext {
    pub fn combined(&self) -> Option<String>;  // join with "\n\n---\n\n"
    pub fn get(&self, name: &str) -> Option<&str>;
}
```

---

## 5. Memory Projections

Memory projections are user-visible read-side windows into OpenMemory, rendered as markdown files.

### 5.1 Projection Architecture

```text
┌───────────────────────────────────────────────────────────────────────────┐
│                      MEMORY PROJECTIONS                                   │
├───────────────────────────────────────────────────────────────────────────┤
│                                                                           │
│   OpenMemory Store (SQLite + JSON)                                        │
│          │                                                                │
│          ▼                                                                │
│   ┌─────────────────────────────────────────────────────────────────────┐ │
│   │              memory_projections::render_markdown()                  │ │
│   │                                                                     │ │
│   │   Groups memories by:                                               │ │
│   │   1. Pinned memories (highest priority)                             │ │
│   │   2. Sector buckets (episodic, semantic, procedural, etc.)          │ │
│   │                                                                     │ │
│   │   Filters by project_id so USER.md and MEMORY.md stay disjoint      │ │
│   │   (both tiers share same store, different project_id)               │ │
│   │                                                                     │ │
│   │   Char-boundary-safe truncation with "..." markers                  │ │
│   │   Deterministic ordering (sorted by created_at, then id)            │ │
│   │                                                                     │ │
│   └─────────────────────────────────────────────────────────────────────┘ │
│          │                                                                │
│          ▼                                                                │
│   ┌─────────────────────────┐  ┌───────────────────────────┐              │
│   │       USER.md           │  │      MEMORY.md            │              │
│   │       (user tier)       │  │      (project tier)       │              │
│   │                         │  │                           │              │
│   │  ~/.vibecli/USER.md     │  │  <workspace>/.vibecli/    │              │
│   │                         │  │          MEMORY.md        │              │
│   │  - Global user prefs    │  │                           │              │
│   │  - Cross-project facts  │  │  - Project-specific rules │              │
│   │  - Personal patterns    │  │  - Team conventions       │              │
│   │  - Pinned global mem    │  │  - Project context        │              │
│   │                         │  │  - Pinned project mem     │              │
│   └─────────────────────────┘  └───────────────────────────┘              │
│                                                                           │
│   Auto-refresh:                                                           │
│   - OpenMemoryStore::enable_projection_refresh() opt-in                   │
│   - Every save() calls write_projections() best-effort                    │
│   - REPL /openmemory project command for manual refresh                   │
│                                                                           │
└───────────────────────────────────────────────────────────────────────────┘
```

### 5.2 Projection Schema

```markdown
# VibeCody Memory -- Project: vibecody

## Pinned Memories

- User prefers snake_case for Rust identifiers (semantic, 0.95 salience)
- Always run cargo clippy --all-targets before committing (procedural, 0.92 salience)

---

## Episodic Memories

- 2026-05-01: Refactored auth middleware to use P-256 ECDSA keys...
- 2026-04-28: Debugged race condition in session store...

---

## Semantic Memories

- User prefers Rust over Go for backend work...
- Definition of Context Assembler is policy-driven budget allocation...

---

## Procedural Memories

- Deploy workflow: cargo build --release -> docker push -> kubectl apply...

---

*Auto-generated by VibeCody. Edit to update; changes sync on next session.*
```

---

## 6. Recap & Resume Architecture

Recap & Resume provide cross-client session continuity. The daemon owns recap generation and storage; clients render what the daemon emits.

### 6.1 Three Scopes

| Scope | Unit of Work | Storage | Doc |
|-------|--------------|---------|-----|
| **Session** | Chat/agent conversation | `sessions.db` -> `recaps` table | `design/recap-resume/01-session.md` |
| **Job** | Background agent run | `jobs.db` -> `recaps` table | `design/recap-resume/02-job.md` |
| **DiffChain** | Diffcomplete refinement chain | `workspace.db` -> `diff_chain_recaps` | `design/recap-resume/03-diffcomplete.md` |

### 6.2 Recap Data Model

```rust
pub struct Recap {
    pub id: RecapId,                       // ULID
    pub kind: RecapKind,                   // Session | Job | DiffChain
    pub subject_id: String,                // session_id | job_id | diff_chain_id
    pub workspace: Option<PathBuf>,
    pub generated_at: DateTime<Utc>,
    pub generator: RecapGenerator,         // Heuristic | Llm { provider, model } | UserEdited
    pub headline: String,                  // <= 80 chars, single line
    pub bullets: Vec<String>,              // 3-7 short bullets
    pub next_actions: Vec<String>,         // 0-3 imperative follow-ups
    pub artifacts: Vec<RecapArtifact>,     // files touched, diffs, jobs spawned
    pub resume_hint: Option<ResumeHint>,   // structured handoff
    pub token_usage: Option<TokenUsage>,
    pub schema_version: u16,
}

pub struct ResumeHint {
    pub target: ResumeTarget,               // Session(id) | Job(id) | DiffChain(id)
    pub from_message: Option<MessageId>,    // session resume cursor
    pub from_step: Option<u32>,           // job step cursor
    pub from_diff_index: Option<u32>,     // diff-chain cursor
    pub seed_instruction: Option<String>, // pre-fill next prompt
}
```

### 6.3 Recap Architecture Diagram

```text
┌─────────────────────────────────────────────────────────────────────────────┐
│                    RECAP & RESUME ARCHITECTURE                              │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│   ┌─────────────────────────────────────────────────────────────────────┐   │
│   │                    RECAP GENERATION TRIGGERS                        │   │
│   │                                                                     │   │
│   │   ┌────────────┐  ┌────────────┐  ┌────────────┐  ┌────────────┐    │   │
│   │   │ Agent task │  │User /recap │  │ Tab close  │  │Idle 30min  │    │   │
│   │   │ completes  │  │ command    │  │ (vibeui)   │  │(optional)  │    │   │
│   │   └──────┬─────┘  └──────┬─────┘  └──────┬─────┘  └──────┬─────┘    │   │
│   │          │               │               │               │          │   │
│   │          └───────────────┴───────────────┴───────────────┘          │   │
│   │                            │                                        │   │
│   │                            ▼                                        │   │
│   │   ┌───────────────────────────────────────────────────────────────┐ │   │
│   │   │              RECAP GENERATOR (daemon-side)                    │ │   │
│   │   │                                                               │ │   │
│   │   │   Heuristic (default):                                        │ │   │
│   │   │   - Headline: first user message (<=80 chars)                 │ │   │
│   │   │   - Bullets: tool call counts, files touched, status          │ │   │
│   │   │   - next_actions: parse last assistant messages for TODOs     │ │   │
│   │   │   - artifacts: unique file paths from steps                   │ │   │
│   │   │                                                               │ │   │
│   │   │   LLM (optional, richer):                                     │ │   │
│   │   │   - Summarize with provider-selected model                    │ │   │
│   │   │   - Structured extraction of bullets + next_actions           │ │   │
│   │   │                                                               │ │   │
│   │   └───────────────────────────────────────────────────────────────┘ │   │
│   │                            │                                        │   │
│   │                            ▼                                        │   │
│   │   ┌───────────────────────────────────────────────────────────────┐ │   │
│   │   │                    STORAGE (scope-specific)                   │ │   │
│   │   │                                                               │ │   │
│   │   │   Session -> sessions.db/recaps (plaintext)                   │ │   │
│   │   │   Job     -> jobs.db/recaps (ChaCha20 encrypted)              │ │   │
│   │   │   DiffChain -> workspace.db/diff_chain_recaps (ChaCha20)      │ │   │
│   │   │                                                               │ │   │
│   │   └───────────────────────────────────────────────────────────────┘ │   │
│   │                            │                                        │   │
│   └────────────────────────────┴────────────────────────────────────────┘   │
│                                  │                                          │
│                                  ▼                                          │
│   ┌─────────────────────────────────────────────────────────────────────┐   │
│   │                         RESUME FLOW                                 │   │
│   │                                                                     │   │
│   │   Client POST /v1/resume { from_recap_id, overrides... }            │   │
│   │                                                                     │   │
│   │                    │                                                │   │
│   │                    ▼                                                │   │
│   │   ┌────────────────────────────────────────────────────────────┐    │   │
│   │   │  Daemon:                                                   │    │   │
│   │   │  1. Load recap by ID                                       │    │   │
│   │   │  2. Determine target session/job/diffchain                 │    │   │
│   │   │  3. Apply resume_hint (message cursor, seed instruction)   │    │   │
│   │   │  4. If branch=true: fork new session_id                    │    │   │
│   │   │  5. Return resume handle for polling                       │    │   │
│   │   └────────────────────────────────────────────────────────────┘    │   │
│   │                                                                     │   │
│   │   Client GET /v1/resume/:handle -> poll until ready=true            │   │
│   │                                                                     │   │
│   │   Ready -> Client opens chat/agent with primed context              │   │
│   │                                                                     │   │ 
│   └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 6.4 Cross-Client Resume Surface

```text
┌─────────────────────────────────────────────────────────────────────────────┐
│                    RECAP CONSUMPTION BY CLIENT                              │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│   ┌───────────┐  ┌───────────┐  ┌───────────┐  ┌───────────────────┐        │
│   │  vibecli  │  │  vibeui   │  │ vibemobile│  │ vibewatch         │        │
│   │  REPL/TUI │  │  (Tauri)  │  │ (Flutter) │  │ (SwiftUI/Compose) │        │
│   └─────┬─────┘  └─────┬─────┘  └─────┬─────┘  └────────┬──────────┘        │
│         │              │              │                 │                   │
│         ▼              ▼              ▼                 ▼                   │
│   ┌─────────────────────────────────────────────────────────────────────┐   │
│   │                     /v1/recap ROUTES                                │   │
│   │  POST /v1/recap              -> Generate/fetch recap                │   │ 
│   │  GET  /v1/recap/:id          -> Fetch stored recap                  │   │ 
│   │  GET  /v1/recap?kind=&subject_id= -> List recaps                    │   │ 
│   │  POST /v1/resume             -> Begin resume                        │   │
│   │  GET  /v1/resume/:handle       -> Poll readiness                    │   │
│   └─────────────────────────────────────────────────────────────────────┘   │ 
│                                                                             │
│   Per-client UX:                                                            │
│   - vibecli: /recap command, --resume flag, end-of-agent print              │
│   - vibeui: Recap card at top of restored tab, History panel                │
│   - vibemobile: ChatScreen header with recap card before transcript         │
│   - vibewatch: RecapView (3 bullets max), "Continue on phone" button        │
│                                                                             │
│   Watch constraint: read-only. Watches never generate recaps.               │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 7. Data Flow Diagrams

### 7.1 Complete Request Flow

```text
┌─────────────────────────────────────────────────────────────────────────────┐
│                  COMPLETE REQUEST FLOW: Chat Message                        │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│   User types message in vibeui                                              │
│          │                                                                  │
│          ▼                                                                  │
│   ┌─────────────────────────────────────────────────────────────────────┐   │
│   │  vibeui: AIChat.tsx                                                 │   │ 
│   │  - Read selectedProvider, selectedModel from toolbar                │   │
│   │  - invoke("ai_chat", { provider, model, messages })                 │   │
│   └─────────────────────────────────────────────────────────────────────┘   │
│          │                                                                  │
│          ▼                                                                  │
│   ┌─────────────────────────────────────────────────────────────────────┐   │
│   │  vibeui Tauri: commands.rs                                          │   │
│   │  - build_temp_provider(provider, model)                             │   │
│   │  - Call Context Assembler for project_memory                        │   │
│   └────────────────────────────────────────────────────┬────────────────┘   │
│                                                        │                    │
│                                                        ▼                    │
│   ┌─────────────────────────────────────────────────────────────────────┐   │
│   │  Context Assembler (context_assembler.rs)                           │   │
│   │                                                                     │   │
│   │  Policy: Chat                                                       │   │
│   │  ┌───────────────┐  ┌───────────────┐                               │   │
│   │  │ ProjectMemory │  │ Orchestration │                               │   │
│   │  │    load()     │  │    Lessons    │                               │   │
│   │  │               │  │               │                               │   │
│   │  │ CLAUDE.md     │  │ workflow_     │                               │   │
│   │  │ AGENTS.md     │  │ orchestration │                               │   │
│   │  │ VIBECLI.md    │  │ lessons store │                               │   │
│   │  └───────────────┘  └───────────────┘                               │   │
│   │                                                                     │   │
│   │  Assemble -> [project_memory section, orchestration section]        │   │
│   └───────────────────────────────────────┬─────────────────────────────┘   │
│                                           │                                 │
│                                           ▼                                 │
│   ┌─────────────────────────────────────────────────────────────────────┐   │
│   │  vibe-ai: ChatEngine                                                │   │
│   │  - Prepend assembled context as system message(s)                   │   │
│   │  - Add user message                                                 │   │
│   │  - HTTP POST to provider API                                        │   │
│   └─────────────────────────────────────────────────────────────────────┘   │
│          │                                                                  │
│          ▼                                                                  │
│   ┌─────────────────────────────────────────────────────────────────────┐   │
│   │  Provider API (Claude/OpenAI/etc)                                   │   │
│   │  - Process request                                                  │   │
│   │  - Return streaming response                                        │   │
│   └─────────────────────────────────────────────────────────────────────┘   │
│          │                                                                  │
│          ▼                                                                  │
│   ┌─────────────────────────────────────────────────────────────────────┐   │
│   │  Stream chunks back through:                                        │   │ 
│   │  Provider -> ChatEngine -> Tauri command -> AIChat.tsx -> UI render │   │
│   └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│   Side effects (async):                                                     │
│   - SessionStore: insert message row                                        │
│   - SessionStore: update messages_fts index                                 │
│   - If tool calls: insert step rows                                         │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 7.2 Agent Loop Flow (with Scratchpad)

```text
┌─────────────────────────────────────────────────────────────────────────────┐
│                  BACKGROUND JOB AGENT FLOW                                  │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│   User submits background job via vibeui BackgroundJobsPanel                │
│          │                                                                  │
│          ▼                                                                  │
│   ┌─────────────────────────────────────────────────────────────────────┐   │
│   │  JobManager::submit()                                               │   │
│   │  - Insert job record (queued)                                       │   │
│   │  - Spawn tokio task                                                 │   │
│   └─────────────────────────────────────────────────────────────────────┘   │
│          │                                                                  │
│          ▼                                                                  │
│   ┌─────────────────────────────────────────────────────────────────────┐   │
│   │  run_real_worker_agent_loop()                                       │   │
│   │                                                                     │   │
│   │  1. Build AgentContext:                                             │   │
│   │     - ContextPolicy::Agent { task, job_id: Some(id) }               │   │
│   │     - AgentKind::BackgroundJob (budget: 48k, scratchpad-dominant)   │   │
│   │     - MemoryToggles { jobs_db_path: Some(default_db_path()) }       │   │
│   │                                                                     │   │
│   │  2. assemble_context() -> includes agent_scratchpad section         │   │
│   │     - Query job_scratchpad table for this session_id                │   │
│   │     - Sort by updated_at, format as context                         │   │
│   │                                                                     │   │
│   │  3. Agent loop (plan->act->observe)                                 │   │
│   │                                                                     │   │
│   │  4. On each turn boundary:                                          │   │
│   │     persist_to_scratchpad(db_path, session_id, key, value)          │   │
│   │     - ToolCallExecuted -> step_{step_num:04}_{tool_name}            │   │
│   │     - Complete -> terminal_complete                                 │   │
│   │     - Error -> terminal_error                                       │   │
│   │                                                                     │   │
│   │  5. Agent completes -> generate recap -> store in jobs.db/recaps    │   │
│   │                                                                     │   │
│   └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│   Resume flow (if user clicks "Resume from here"):                          │
│   - POST /v1/resume { from_recap_id: ... }                                  │
│   - Load recap -> extract resume_hint -> prime context                      │
│   - scratchpad entries reloaded via Context Assembler                       │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 8. Cross-Client Surface

### 8.1 HTTP Route Summary

| Route | Method | Description | Clients |
|-------|--------|-------------|---------|
| `/health` | GET | Daemon status, memory block | All |
| `/health/memory` | GET | Memory subsystem health | All |
| `/memory/*` | Various | Full memory API | CLI, SDK, Plugins |
| `/v1/recap` | POST | Generate/fetch recap | CLI, UI, Mobile |
| `/v1/recap/:id` | GET | Fetch stored recap | CLI, UI, Mobile |
| `/v1/resume` | POST | Begin resume | CLI, UI, Mobile |
| `/v1/resume/:handle` | GET | Poll resume readiness | CLI, UI, Mobile |
| `/v1/capabilities` | GET | List context kinds, sections | All |
| `/v1/agent` | POST | Start agent with context negotiation | UI, SDK |
| `/mobile/sessions` | Various | Mobile-specific session endpoints | Mobile |
| `/watch/sessions` | Various | Watch-optimized session endpoints | Watch |

### 8.2 Tauri Commands (VibeUI)

| Command | Description |
|---------|-------------|
| `assemble_context` | Get assembled context for any panel |
| `profile_api_key_get/set/delete` | API key management |
| `workspace_secret_get/set` | Project secret management |
| `panel_settings_get/set` | UI persistence |
| `open_memory_query` | Semantic search |
| `session_list` | List sessions |
| `session_recap_get` | Get session recap |
| `job_list` | List background jobs |
| `job_recap_get` | Get job recap |

---

## 9. Configuration Reference

### 9.1 Config.toml Sections

```toml
# ~/.vibecli/config.toml

[memory]
auto_record = true              # Auto-append to ~/.vibecli/memory.md
min_session_steps = 3          # Minimum tool uses before recording

[openmemory]
enabled = true
auto_inject = true               # Inject into agent context
max_context_tokens = 1200        # Cap on injected context
decay_enabled = true             # Run salience decay
consolidate_on_exit = false      # Run sleep-cycle consolidation
encryption = false               # AES-256-GCM at rest

[context]
# Per-agent-kind budgets (overrides defaults)
chat_total_chars = 32000
coding_total_chars = 128000
research_total_chars = 128000
background_total_chars = 48000

[recap]
auto_generate = true             # Auto-recap on session end
generator = "heuristic"          # "heuristic" | "llm" | "auto"
idle_timeout_minutes = 30        # Auto-recap on idle
on_tab_close = true              # Auto-recap when tab closed
```

### 9.2 Environment Variables

| Variable | Purpose |
|----------|---------|
| `VIBE_MEMORY_DIM` | Embedding dimensions (default: 768) |
| `VIBECLI_MEMORY_KEY` | Passphrase for encrypted stores |
| `RUST_BACKTRACE` | Debug backtraces |

---

## See Also

- [`memory-guide.md`](./memory-guide.md) -- User-facing memory commands and usage
- [`AGENTS.md`](../AGENTS.md) -- Agent guidelines, storage patterns, security rules
- [`design/recap-resume/README.md`](./design/recap-resume/README.md) -- Recap/resume design docs
- [`architecture.md`](./architecture.md) -- General system architecture

---

*Document version: 1.0.0 | Last updated: 2026-05-08*
