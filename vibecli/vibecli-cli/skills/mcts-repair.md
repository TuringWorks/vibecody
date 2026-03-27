# MCTS Code Repair

Monte Carlo tree search for autonomous bug fixing. Explores multiple repair strategies in parallel, evaluates each via test execution, and selects the highest-confidence fix. Handles complex multi-file bugs that simple single-shot prompting misses.

## When to Use
- Fixing bugs where the root cause is unclear or spans multiple files
- Repairing test failures that resist simple one-shot fixes
- Exploring multiple fix strategies to find the most robust solution
- Handling complex type errors or lifetime issues in Rust/TypeScript
- Debugging flaky tests by systematically exploring failure conditions

## Commands
- `/mcts repair <test-or-error>` — Start MCTS repair for a failing test or error
- `/mcts status` — Show search tree progress and current best candidate
- `/mcts candidates` — List top-ranked repair candidates with confidence scores
- `/mcts apply <id>` — Apply a specific repair candidate
- `/mcts config depth <n>` — Set maximum search depth (default: 5)
- `/mcts config iterations <n>` — Set maximum iterations (default: 100)
- `/mcts config timeout <seconds>` — Set search timeout
- `/mcts explain <id>` — Explain the reasoning behind a repair candidate

## Examples
```
/mcts repair "cargo test test_parse_config -- --exact"
# Starting MCTS repair. Failing test: test_parse_config
# Iteration 12/100: Found 3 candidates
#   [1] Fix off-by-one in line 42 (confidence: 0.91, tests: 48/48 pass)
#   [2] Add missing null check at line 38 (confidence: 0.73, tests: 46/48)
#   [3] Restructure match arms (confidence: 0.65, tests: 45/48)
# Best: candidate 1 — applying...

/mcts explain 1
# Root cause: parse_config splits on newline but doesn't handle
# trailing newline, causing an empty last element. Fix adds
# .filter(|s| !s.is_empty()) after split.
```

## Best Practices
- Provide a specific failing test for the most targeted repair search
- Start with lower iteration counts and increase if no fix is found
- Review the explain output before accepting any automated fix
- Use MCTS for bugs that persist after 2-3 manual fix attempts
- Ensure the test suite is deterministic before running MCTS repair
