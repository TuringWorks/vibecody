# Test Impact Analysis

Changed-file → affected-test mapping using symbol-import graph BFS traversal. Runs only the tests that could be affected by a given set of file changes. Language-agnostic (Rust, TypeScript, JavaScript, Python, Go). Matches GitHub Copilot Workspace v2's test impact analysis.

## When to Use
- Reducing CI feedback time by skipping unaffected tests
- Identifying which test files to re-run after a focused change
- Building pre-commit hooks that only run relevant tests
- Understanding test coverage gaps for new modules

## How It Works
1. Build an `ImportGraph` (nodes = files, edges = import relationships)
2. For each changed file, BFS the reverse edges to find all transitive importers
3. Filter importers that contain test code (`has_tests = true`)
4. Return the `ImpactReport` with affected + unaffected test files

## ImpactReport Fields
- `changed_files` — files analysed
- `affected_tests` — test files that need re-running
- `unaffected_tests` — test files that can be skipped
- `reduction_pct()` — % of test suite that can be skipped

## Commands
- `/testimpact analyze <files...>` — compute affected tests for changed files
- `/testimpact run` — run only affected tests
- `/testimpact map` — show the full import→test mapping

## Examples
```
/testimpact analyze src/utils.rs
# Affected: tests/util_test.rs, tests/lib_test.rs
# Unaffected: 12 other test files (86% reduction)

/testimpact run
# Running 2 / 14 test files (86% saved)
# test result: ok. 47 passed; 0 failed
```
