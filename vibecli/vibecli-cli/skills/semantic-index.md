# Semantic Index

Deep semantic code indexing that builds call graphs, type hierarchies, import chains, and cross-reference maps. Enables precise code navigation and understanding beyond simple text search.

## When to Use
- Tracing all callers and callees of a function across the codebase
- Understanding type inheritance and trait implementation hierarchies
- Mapping import/dependency chains to assess change impact
- Finding all usages of a type, method, or constant with semantic precision
- Answering "what would break if I change this?" questions

## Commands
- `/semindex build` — Build or rebuild the semantic index for the project
- `/semindex callers <symbol>` — Find all callers of a function or method
- `/semindex callees <symbol>` — Find all functions called by a symbol
- `/semindex hierarchy <type>` — Show type hierarchy (supertypes and subtypes)
- `/semindex imports <file>` — Trace the full import chain for a file
- `/semindex impact <symbol>` — Estimate blast radius of changing a symbol
- `/semindex stats` — Show index size, coverage, and staleness
- `/semindex xref <symbol>` — Cross-reference all usages of a symbol

## Examples
```
/semindex callers parse_config
# 4 callers found:
# main.rs:42 -> parse_config (direct)
# cli.rs:18 -> load_settings -> parse_config (transitive)
# test_config.rs:7 -> parse_config (test)
# bench.rs:12 -> parse_config (bench)

/semindex impact DatabaseConnection
# Blast radius: HIGH (23 files, 4 modules)
# Direct dependents: 8 | Transitive: 15
# Affected tests: 12 | Affected APIs: 3 public endpoints

/semindex hierarchy Widget
# Widget (trait) <- Button, TextInput, Dropdown, Panel, Modal
```

## Best Practices
- Rebuild the index after major refactors or branch switches
- Use impact analysis before making changes to widely-used symbols
- Combine with LSP features for real-time navigation in the editor
- Index only source directories to keep build times fast
- Use callers/callees for understanding unfamiliar codebases quickly
