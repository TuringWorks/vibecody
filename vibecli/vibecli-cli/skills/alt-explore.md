# Alt Explore

Alternative exploration tournament — score N agent candidates on a task by test pass rate, diff size, and compile success, then select the best.

## When to Use
- Running multiple agent attempts in parallel and picking the strongest result
- Comparing model outputs on the same coding task with an objective scoring rubric
- Enforcing a minimum bar (compilation required) before any candidate can win
- Tuning scoring weights to bias toward fewer changed lines or higher test coverage

## Commands
- `/explore run --candidates 4 "<task>"` — Run 4 agent candidates and print the tournament result
- `/explore score <file>` — Score a single candidate output file against the current weights
- `/explore weights` — Show the current ScoringWeights (test/diff/compile)
- `/explore weights set test 0.7 diff 0.2 compile 0.1` — Update scoring weights
- `/explore disqualify-mode on|off` — Toggle min_compile_required

## Examples
```
/explore run --candidates 4 "Implement binary search"
# winner: candidate-2  score: 0.91
#   test_pass_rate: 1.0  diff_lines: 12  compile: true
# runner-up: candidate-1  score: 0.78

/explore weights set test 0.8 diff 0.1 compile 0.1
# Weights updated: test=0.8 diff=0.1 compile=0.1

/explore disqualify-mode on
# min_compile_required = true (non-compiling candidates disqualified)
```
