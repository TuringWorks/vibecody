# Specialized Sub-Agent Roles

Spawn typed sub-agents with domain-specific expertise for focused tasks.

## Triggers
- "sub-agent", "spawn agent", "code reviewer agent", "test writer agent"
- "security reviewer", "debugger agent", "architect agent"

## Usage
```
/subagent spawn code_reviewer src/main.rs
/subagent spawn test_writer src/lib.rs
/subagent spawn security_reviewer src/auth.rs
/subagent spawn debugger --extra "Fix the timeout issue"
/subagent list
/subagent results
/subagent findings
```

## Available Roles
- **CodeReviewer** — Code correctness, readability, best practices (5 turns)
- **TestWriter** — Unit tests, integration tests, edge cases (15 turns)
- **SecurityReviewer** — OWASP Top 10, secrets, injection flaws (8 turns)
- **Refactorer** — Code structure, duplication, design patterns
- **DocumentationWriter** — API docs, README sections, inline comments
- **Debugger** — Error analysis, stack traces, root cause (20 turns)
- **Architect** — System design, trade-offs, module boundaries
- **PerformanceOptimizer** — Bottlenecks, algorithms, memory usage
- **DependencyManager** — Vulnerability audit, updates, conflict resolution
- **MigrationSpecialist** — Data migrations, schema changes, API upgrades

## Features
- Role-specific system prompts with domain expertise
- Configurable tools and max turns per role
- Auto-spawn triggers (e.g., spawn SecurityReviewer on security-sensitive changes)
- AgentFinding with 4 severity levels (Error, Warning, Info, Hint)
- Results aggregation by role, findings across all agents
