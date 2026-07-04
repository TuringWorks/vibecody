# kodegraph

> A tree-sitter + LSP **code knowledge graph** for token-efficient codebase navigation.

`kodegraph` turns a source tree into a queryable graph of symbols and their
relationships — calls, imports, implementations, extensions, containment,
references, dependencies. Instead of dumping whole files into an LLM context
window, an agent traverses a bounded subgraph: **a few hundred tokens instead of
tens of thousands.**

Every edge carries a **provenance** — *source* (tree-sitter / LSP / SCIP),
*tag* (`extracted` / `inferred` / `ambiguous`), and a *confidence* — so a
consumer always knows what was measured vs guessed. (This schema is borrowed from
[Graphify](https://graphify.net/); `kodegraph` is the Rust-native, zero-config,
MCP-friendly take.)

## Why a graph beats flat retrieval

A codebase plus its relationships rarely fits in a prompt. Vector RAG loses
structure (`DigestAuth → Response` is meaningful whether or not the files share
vocabulary). A code graph preserves that structure, so the agent follows edges
instead of retrieving flat chunks — and the per-query cost stays roughly flat as
the codebase grows instead of exploding.

## Install

```bash
# library
cargo add kodegraph

# CLI + MCP server
cargo install kodegraph --features cli
```

## Quickstart (CLI)

```bash
# 1. Build a graph from a source dir (persists to ./kodegraph-out/codegraph.db
#    and writes GRAPH_REPORT.md, graph.mmd, graph.json).
kodegraph build ./src

# 2. Read the high-level report first (the "where do I start" view).
kodegraph report

# 3. Pull a focused subgraph under a token budget.
kodegraph query "build_temp_provider" --budget 1000

# 4. Change-impact for a symbol (callers + callees within 2 hops).
kodegraph explain build_temp_provider

# 5. Render the dependency graph.
kodegraph viz --mermaid

# 6. Expose the graph to an external agent over MCP (stdio).
kodegraph serve
```

## Quickstart (library)

```rust
use kodegraph::builder::CodeGraphBuilder;
use kodegraph::query;

let (graph, _hashes) = CodeGraphBuilder::new().scan_dir("src")?.build()?;

// Who calls build_temp_provider? — a few hundred tokens, not whole files.
for edge in graph.callers("build_temp_provider") {
    println!("{} -> {}  [conf {:.2}]", edge.caller, edge.callee, edge.provenance.confidence);
}

// Change impact: everything reachable within 2 hops (both directions).
let br = query::blast_radius(&graph, "build_temp_provider", 2);
println!("affected: {}", br.affected());
# Ok::<(), anyhow::Error>(())
```

## Architecture (two-tier)

- **Tier 1 — tree-sitter backbone** (default, zero-config): always works, no language
  server needed. Edges are AST-inferred at confidence `0.7`, tagged `inferred`.
- **Tier 2 — LSP enrichment** (`lsp` feature): upgrades call-graph + type-hierarchy
  edges to confidence `0.95`, tagged `extracted`, by asking a real language server
  (rust-analyzer, pyright, gopls, tsserver) for `callHierarchy` and `typeHierarchy`.
  Skipped gracefully when no server is installed.

`kodegraph` ships its **own** minimal stdio LSP client so it stays independent of
any editor's LSP wrapper. Consumers may instead implement the `EdgeProvider` trait
on top of their own LSP client (or a SCIP reader) and feed `kodegraph`.

## Features

| Feature | Default | Purpose |
|---|---|---|
| `tree-sitter` | yes | Zero-config AST backbone (Rust / TS / Python / Go) |
| `sqlite` | yes | Persistent graph + incremental cache |
| `viz` | yes | Mermaid / DOT rendering |
| `lsp` | no | Optional LSP enrichment tier |
| `mcp` | no | stdio MCP server |
| `cli` | no | `kodegraph` binary (implies `mcp` + `sqlite`) |

```toml
[dependencies]
kodegraph = { version = "0.1", features = ["tree-sitter", "sqlite", "lsp"] }
```

## What you get

- **Graph model** — symbols + 7 edge kinds + Graphify provenance + hyperedges, on a
  `petgraph` backbone with cycle detection, coupling (fan-in/out), transitive deps.
- **Analytics** — god nodes (highest-degree keystones), Leiden-style community
  detection (topology-only, no embeddings), "surprising" cross-file edges, and
  `blast_radius` — the token-reduction primitive.
- **Query API** — `query_graph`, `get_node`, `get_neighbors`, `shortest_path`,
  `blast_radius`, `communities`, `god_nodes` (Graphify-compatible MCP tool names).
- **Reporting** — `GRAPH_REPORT.md`, Mermaid, DOT.
- **Incremental** — SHA256 content-hash cache; re-runs only touch changed files.
- **Persistence** — SQLite (bundled, zero external dep).

## How it compares

| Tool | Lang | Reduction | Storage | MCP | Notes |
|---|---|---|---|---|---|
| **kodegraph** | Rust | — | SQLite | ✅ | Native Rust, two-tier (tree-sitter + LSP), provenance tags |
| [Graphify](https://graphify.net/) | Python | 71.5× | NetworkX | ✅ | Richest schema, multi-modal (docs/PDF/image/audio) |
| [LeanKG](https://github.com/FreePeak/LeanKG) | Rust | ~98% | local | ✅ | Smallest per-query cost, session-hook auto-injection |
| [NeuralMind](https://dfrostar.github.io/neuralmind/) | ? | 40–70× | local | ✅ | Hebbian co-edit memory layer |
| [Contexly](https://github.com/Mynksri/Contexly) | Python | ~95% | local | ✅ | Logic skeletons + change-impact risk tiers |
| [code-nexus](https://github.com/snagrecha/code-nexus) | Python | >80% | SQLite | ✅ | Strong temporal / git-history support |
| [GitCortex](https://github.com/bharath03-a/gitcortex) | Rust | ~7.7% | KuzuDB | ✅ | Branch-aware, auto-indexes on commit |

`kodegraph` borrows Graphify's provenance/hyperedge/community schema and LeanKG's
native-Rust + session-injection philosophy, and adds the two-tier (tree-sitter +
LSP) confidence model from the SCIP/tree-sitter pipeline pattern.

## License

MIT. See [`LICENSE`](LICENSE).