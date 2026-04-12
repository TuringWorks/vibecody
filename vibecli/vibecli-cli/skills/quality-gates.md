# Quality Gates

Enforce configurable pass/fail criteria (tests, coverage, clippy, security, compilation) before marking a task complete. Supports blocking vs advisory gates and a GreenContract hierarchical merge-readiness system.

## When to Use
- Blocking task completion until all CI criteria pass
- Enforcing coverage thresholds before merging
- Checking clippy cleanliness and security findings
- Defining per-project quality contracts
- Building merge-readiness gates (TargetedTests → Package → Workspace → MergeReady)

## Gate Criteria
| Criterion | Passes when |
|---|---|
| `TestsPass` | 0 test failures (skipped if no tests) |
| `CoverageAbove { min_pct }` | Coverage ≥ threshold |
| `ClippyClean` | 0 clippy errors |
| `NoSecurityFindings` | 0 high/critical findings |
| `Compiles` | Build succeeds |
| `LintWarningsBelow { max }` | Warnings ≤ max |
| `Custom { name }` | External gate result |

## GreenContract Levels (ordered)
`TargetedTests < Package < Workspace < MergeReady`

## Commands
- `/gates check` — Evaluate all gates against current evidence
- `/gates status` — Show pass/fail/skipped for each criterion
- `/gates preset rust` — Apply the rust_project_gate preset
- `/gates add <criterion>` — Add a criterion to the current gate
- `/gates advisory` — Run gates in advisory mode (non-blocking)
- `/gates evidence` — Show the current TaskEvidence snapshot

## Examples
```
/gates check
# ✓ compiles  ✓ tests_pass  ✓ clippy_clean  ✗ coverage>=70% (62.1%)

/gates preset rust
# Applies: Compiles + TestsPass + ClippyClean + Coverage≥70%

/gates add coverage 80
# Requires coverage ≥ 80% to complete task
```
