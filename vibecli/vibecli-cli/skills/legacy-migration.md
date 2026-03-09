---
triggers: ["legacy migration", "code migration", "cobol migration", "fortran migration", "legacy modernization", "language migration", "modernize codebase", "rewrite legacy", "strangler fig", "legacy refactor"]
tools_allowed: ["read_file", "write_file", "bash"]
category: workflow
---

# Legacy Code Migration

When migrating legacy codebases to modern languages:

1. Analyze the legacy codebase first: identify all components (modules, classes, functions, copybooks, stored procedures), calculate cyclomatic complexity, map dependencies, and assess risk levels. Never start translating without a complete analysis.
2. Choose the right migration strategy: Direct Translation (1:1 mapping, fast but may carry anti-patterns), Rewrite (clean-room reimplementation, highest quality but most effort), Strangler Fig (incremental replacement behind facade, safest for production systems), Big Bang (replace everything at once, only for small codebases), Incremental (module-by-module replacement), Hybrid Bridge (translate core, rewrite UI/API layer).
3. Build a dependency graph and use topological ordering: migrate leaf nodes (no dependencies) first, then work inward. This ensures each translated component's dependencies are already available.
4. Apply translation rules systematically: each source→target language pair has predefined rules (e.g., COBOL PERFORM→Rust fn call, Fortran COMMON→Rust struct). Flag low-confidence translations (score <0.7) for manual review.
5. Identify service boundaries automatically: group related components into microservice candidates based on data access patterns, call frequency, and business domain. Generate API surface definitions for each boundary.
6. Preserve business rules: extract business logic comments and inline documentation. Add migration markers (// MIGRATED: original_file:line) to maintain traceability.
7. Generate comprehensive tests alongside translations: every translated component should have equivalent test coverage. Target 80%+ coverage of the translated code.
8. Produce a migration report: components migrated vs total, source→target line counts, confidence scores, manual reviews needed, service boundary map, and estimated remaining effort.
