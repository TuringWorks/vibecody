---
triggers: ["batch build", "bulk generation", "batch code", "autonomous build", "hands-off development", "outsource development", "batch run", "bulk code generation", "system 2", "batch agent"]
tools_allowed: ["read_file", "write_file", "bash"]
category: workflow
---

# Batch Builder — Autonomous Bulk Code Generation

When performing large-scale autonomous code generation:

1. Start by creating a BatchSpec from the user's natural language description — extract title, requirements, user stories, API endpoints, data models, and UI components before generating any code.
2. Estimate complexity before starting: use estimated_complexity() to gauge effort (1-100 scale), recommend agent count, and predict duration. Warn the user if the spec is too vague or too complex.
3. Generate an ArchitecturePlan first: system overview, module decomposition, dependency graph, deployment strategy, database design, API design, and security approach. Get user approval before proceeding.
4. Use topological ordering for module generation: build dependency-free modules first, then progressively build dependent modules. This prevents circular dependency issues.
5. Spawn specialized agents per role: Architect, Backend, Frontend, Database, Infrastructure, Testing, Documentation, Security, Performance, Integration. Each agent works on its assigned modules independently.
6. Monitor progress in real-time: track lines/hour, files/hour, compile pass rate, test pass rate. Use checkpoint intervals (default 30min) to save progress for resume capability.
7. Run multi-QA validation after generation: CompileChecker, TestRunner, SecurityAuditor, StyleEnforcer, DocValidator, PerformanceAnalyzer, DependencyAuditor, IntegrationTester — multiple rounds until passing threshold.
8. Support pause/resume/cancel: long-running batch builds (8-12 hours) should be interruptible and resumable without losing progress.
