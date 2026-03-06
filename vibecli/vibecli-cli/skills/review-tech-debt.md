---
triggers: ["tech debt", "technical debt", "code quality", "debt assessment", "risk scoring", "code smell"]
tools_allowed: ["read_file", "write_file", "bash"]
category: review
---

# Technical Debt Assessment

When assessing and managing technical debt:

1. Categorize debt: design debt, code debt, test debt, documentation debt, infrastructure debt
2. Risk scoring: impact (1-5) x likelihood (1-5) = priority score — attack high scores first
3. Code smells indicating debt: long methods, deep nesting, duplicate code, god classes
4. Test debt indicators: low coverage on critical paths, flaky tests, missing integration tests
5. Measure: cyclomatic complexity, coupling, code churn, bug density per module
6. Track in backlog: create tickets for tech debt with business impact justification
7. Allocate capacity: dedicate 15-20% of sprint to tech debt reduction
8. Quick wins: address debt during feature work in the same area (Boy Scout Rule)
9. Avoid gold plating: fix debt that causes problems, not theoretical imperfections
10. Communication: explain debt in business terms — "This will cause X outage risk" not "Bad code"
11. Debt freeze: before major features, pay down debt in the affected area
12. Metrics over time: track complexity trends, build times, test suite duration, deployment frequency
