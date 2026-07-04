# kodegraph schema

## Nodes

A graph node (`NodeData`) is one of:

| Variant | Payload | Granularity |
|---|---|---|
| `Symbol` | `Symbol { name, kind, qualified_name, file_path, line_start, line_end, signature, doc_comment, visibility, language }` | fine |
| `Module` | `{ name, file_path }` | coarse |
| `File` | `{ path }` | coarse |

`SymbolKind`: `function`, `method`, `class`, `struct`, `enum`, `interface`, `trait`,
`module`, `constant`, `variable`, `type_alias`, `macro`.

`Symbol::node_key()` = `"{file_path}:{qualified_name}"` — the dedup key.

## Edges

### Backbone edge (`EdgeData`)

| Field | Type |
|---|---|
| `kind` | `EdgeKind` |
| `provenance` | `Provenance` |

### `EdgeKind` (seven first-class relationships)

`calls`, `imports`, `implements`, `extends`, `contains`, `references`, `depends_on`.

### `Provenance` (Graphify-style)

| Field | Type | Notes |
|---|---|---|
| `source` | `EdgeSource` | `tree_sitter` / `lsp` / `scip` / `regex` / `inferred` |
| `tag` | `ProvenanceTag` | `extracted` / `inferred` / `ambiguous` |
| `confidence` | `f32` | `[0, 1]` |

Default confidence by source: `scip` 1.0, `lsp` 0.95, `tree_sitter` 0.7, `regex` 0.5,
`inferred` 0.4. Default tag: `lsp`/`scip` → `extracted`; `tree_sitter`/`regex`/`inferred`
→ `inferred`.

### Rich typed edge views (parallel to the backbone)

- `CallEdge { caller, callee, file, line, call_type, provenance }` — `call_graph: Vec<CallEdge>`
- `ImportEdge { source_file, target, imported_symbols, import_type, provenance }` — `import_graph`
- `TypeRelation { parent, child, relation, provenance }` — `type_hierarchy`
- `ApiContract { params, return_type, error_types, is_async }` — `api_contracts: HashMap`

### Hyperedges

`Hyperedge { label, kind, members: Vec<node_key> }` with `HyperedgeKind`:
`implements_group`, `flow`, `community`, `module`, `custom`. A hyperedge with ≥ 3
members is "hyper" (`is_hyper()`).

## Persistence (SQLite)

`SQLiteStore` (bundled `rusqlite`) — two tables:

- `graph(id INTEGER PRIMARY KEY, payload TEXT, updated_at TEXT)` — single row holding
  the serialized `CodeGraph` JSON. (Finer-grained relational schema is a follow-up.)
- `file_hashes(path TEXT PRIMARY KEY, hash TEXT)` — incremental cache (stored as a
  JSON blob under the reserved path `__cache_blob__`).

## MCP tools (`mcp` feature)

| Tool | Args | Returns |
|---|---|---|
| `query_graph` | `{ query: string, budget?: int }` | focused subgraph (text) + `estTokens` |
| `get_node` | `{ name }` | node payload |
| `get_neighbors` | `{ name }` | adjacent node labels |
| `shortest_path` | `{ from, to }` | hop count + path |
| `blast_radius` | `{ name, max_hops? }` | reachable nodes by hop + `affected` |

Protocol: JSON-RPC 2.0 over stdio with `Content-Length` framing. Supports
`initialize`, `tools/list`, `tools/call`.

## CLI (`cli` feature)

```
kodegraph build <path> [--out kodegraph-out] [--quiet] [--db PATH]
kodegraph query "<q>" [--budget N] [--db PATH]
kodegraph path <from> <to> [--db PATH]
kodegraph explain <name> [--db PATH]
kodegraph viz [--mermaid] [--dot] [--db PATH]
kodegraph report [--db PATH]
kodegraph serve [--db PATH]
```

`build` writes `GRAPH_REPORT.md`, `graph.mmd`, `graph.json` to `--out` and persists
the graph + file-hash cache to `--db`. Subsequent `build` runs are incremental.

## Language coverage (v0.1)

| Language | tree-sitter (Tier 1) | LSP enrichment (Tier 2) |
|---|---|---|
| Rust | ✅ | ✅ (rust-analyzer) |
| TypeScript / JavaScript | ✅ | ✅ (tsserver) |
| Python | ✅ | ✅ (pyright) |
| Go | ✅ | ✅ (gopls) |

Other languages fall back to `Language::Unknown` and are not parsed in v0.1.