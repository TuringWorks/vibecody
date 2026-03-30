# Codebase Health Score

Analyze and score codebase health across 12 dimensions. Use `/healthscore scan` to get a comprehensive health report with actionable remediations.

## Dimensions
- **Test Coverage** — Percentage of code covered by tests
- **Dependency Freshness** — How up-to-date dependencies are
- **Security Posture** — Known CVEs and vulnerability count
- **Doc Coverage** — Documentation completeness
- **Complexity** — Average cyclomatic complexity
- **Type Safety** — Percentage of typed code
- **Dead Code** — Unreachable or unused code
- **Linter Warnings** — Static analysis issue count
- **Build Time** — Compilation/build duration
- **Bundle Size** — Output artifact size
- **Accessibility** — A11y compliance issues
- **API Coverage** — API documentation completeness

## Commands
- `/healthscore scan [path]` — Scan and score the codebase
- `/healthscore trend` — Show score trends over time
- `/healthscore remediate` — Get prioritized improvement suggestions

## Scoring
Each dimension is scored 0-100. The overall score is a weighted average.
- **80-100** — Healthy (green)
- **60-79** — Warning (amber)
- **0-59** — Critical (red)

## Example
```
/healthscore scan .
/healthscore remediate
```
