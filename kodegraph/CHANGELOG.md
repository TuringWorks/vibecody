# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2026-07-03

### Added
- `CodeGraph` data model consolidating symbol / call / import / type graphs onto a
  `petgraph` backbone with Graphify-style edge provenance (`Extracted` / `Inferred` /
  `Ambiguous` + confidence + source).
- Two-tier parsing: `TreeSitterParser` (Rust / TypeScript / Python / Go, zero-config,
  confidence 0.7) + optional `LspEdgeProvider` (own stdio JSON-RPC client,
  `callHierarchy` + `typeHierarchy`, confidence 0.95).
- Analytics: Leiden-style label-propagation community detection, god-node
  identification, cross-file "surprising edge" scoring, and `blast_radius` (the
  token-reduction primitive — reachable set within N hops, both directions).
- Graphify-compatible query API: `query_graph`, `get_node`, `get_neighbors`,
  `shortest_path`, `blast_radius`, `communities`, `god_nodes`.
- Reporting: `GRAPH_REPORT.md` (god nodes + communities + surprising edges +
  suggested questions), Mermaid + DOT renderers.
- Incremental SHA256 content-hash cache; re-runs only re-parse changed files.
- SQLite persistence (`SQLiteStore`, bundled rusqlite) for graph + hashes.
- stdio MCP server (`mcp` feature) exposing the query tools to external agents.
- `kodegraph` CLI (`cli` feature): `build`, `query`, `path`, `explain`, `viz`,
  `report`, `serve`.
- Hyperedges (3+ node groups) for relationships pairwise edges can't express.
- Examples: `build_graph`, `mcp_server`.

[Unreleased]: https://github.com/ravituringworks/vibecody/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/ravituringworks/vibecody/releases/tag/v0.1.0