---
triggers: ["knowledge graph", "cross-repo", "code graph", "symbol graph", "dependency graph", "callers", "callees", "implementors", "cross-repository", "code intelligence", "semantic graph"]
tools_allowed: ["read_file", "write_file", "bash", "search_files"]
category: code-intelligence
---

# Cross-Repository Knowledge Graph

When building or querying a cross-repo knowledge graph:

1. **Register Repositories** — Add repositories to the graph with `add_repo(name, path)`. Each repo is scanned for symbols (functions, structs, traits, classes, interfaces, modules) using language-aware regex extraction. Supported languages: Rust (`fn`, `struct`, `enum`, `trait`, `impl`, `mod`), TypeScript/JavaScript (`function`, `class`, `interface`, `export`), Python (`def`, `class`, `import`), Go (`func`, `type`, `interface`). The graph automatically indexes all source files, skipping hidden directories, `node_modules`, `target/`, and `.git/`.

2. **Understand Edge Types** — The graph tracks seven relationship types: `Calls` (function A invokes function B), `Implements` (struct implements trait/interface), `Extends` (class extends base class), `Imports` (file imports symbol from another file), `Contains` (module contains a symbol), `References` (code references a type or constant), `DependsOn` (file-level dependency). Edges are inferred from import statements, use declarations, function call patterns, and impl blocks.

3. **Query Callers and Callees** — Use `query_callers("symbol_name")` to find all code locations that call a given function or method. Use `query_callees("symbol_name")` to find everything a function calls. These queries work across repository boundaries, so you can trace how a shared library function is used across all registered repos.

4. **Find Implementors** — `query_implementors("TraitName")` returns all types that implement a given trait or interface across all registered repos. This is essential for understanding polymorphism, plugin systems, and trait-based architectures where implementations are scattered across crates or packages.

5. **Analyze Dependencies** — `query_dependencies(file)` returns what a file depends on (imports, uses). `query_dependents(file)` returns everything that depends on that file. Use these for impact analysis before refactoring — know exactly what will break if you change a type signature or remove a function.

6. **Cross-Repo References** — `cross_repo_references(repo)` reveals all edges that connect symbols in one repo to symbols in another. This surfaces coupling between repositories, shared interfaces, and integration points. Use this to understand how microservices, shared libraries, and monorepo packages interact.

7. **Find Paths Between Symbols** — `shortest_path("SymbolA", "SymbolB")` uses BFS to find the shortest chain of relationships connecting two symbols. This helps answer questions like "how does the HTTP handler reach the database?" or "what's the call chain from the CLI entry point to the encryption module?"

8. **Extract Subgraphs** — `subgraph("symbol", depth)` extracts the neighborhood around a symbol up to N hops. Depth 1 gives direct connections; depth 2-3 gives the broader context. Export subgraphs to DOT format for visualization with Graphviz.

9. **Monitor Graph Stats** — `stats()` provides node and edge counts per repo, most-connected symbols (highest in-degree and out-degree), and orphan symbols (no edges). Use stats to identify tightly-coupled modules, hub symbols that many things depend on, and dead code (symbols with zero references).

10. **Persist and Share** — Save the graph to JSON with `save(path)` and reload with `load(path)`. This avoids re-scanning large codebases. The graph can be shared across team members or committed to the repo for CI-based dependency analysis.
