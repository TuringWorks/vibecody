# AI-Assisted Code Review

Automated code review engine that analyzes PRs and files for bugs, security issues, complexity, style violations, test gaps, and architecture concerns. Matches and exceeds Qodo Merge, CodeRabbit, and Bito capabilities.

## Features
- **7 Pattern Detectors**: Security (OWASP Top 10), complexity, style, documentation, testing, duplication, architecture
- **Multi-Linter Aggregation**: clippy, eslint, pylint, golint, rubocop, shellcheck, hadolint, markdownlint with false-positive filtering
- **Natural Language Quality Gates**: Define merge requirements in plain English
- **Learning Loop**: Tracks accepted/rejected findings, calculates precision/recall/F1
- **PR Summary & Diagrams**: Auto-generate markdown summaries and Mermaid architecture diagrams from diffs
- **Breaking Change Detection**: Identifies public API changes with migration hints
- **Test Generation Hints**: Suggests missing unit/integration/edge case tests from diffs

## Severity Levels
Info, Warning, Error, Critical, Security

## Categories
Bug, Security, Performance, Style, Documentation, Testing, Architecture, Complexity, Duplication, Accessibility, BreakingChange, MergeConflictRisk

## Commands
- `/aireview diff` — Review current diff
- `/aireview file <path>` — Review a specific file
- `/aireview gates` — List quality gates
- `/aireview suggest-tests` — Suggest tests for current changes
- `/aireview breaking` — Detect breaking changes
- `/aireview summary` — Generate PR summary
- `/aireview diagram` — Generate architecture diagram from changes
- `/aireview learn` — Show learning statistics
- `/aireview linters` — Run all linters

## Quality Gate Examples
```
- "No function longer than 50 lines"
- "All public APIs must have docstrings"
- "Test coverage must not decrease"
- "No hardcoded secrets or API keys"
- "Cyclomatic complexity under 10"
```

## Example
```
/aireview diff
/aireview gates
/aireview suggest-tests
/aireview summary
```
