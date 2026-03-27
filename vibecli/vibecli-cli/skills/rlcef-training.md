# RLCEF Training

Reinforcement learning from code execution feedback. The agent learns from test results, build outcomes, and runtime behavior to improve its code generation quality over time. Tracks which patterns lead to passing tests and applies that knowledge to future tasks.

## When to Use
- Improving agent code quality by learning from execution outcomes
- Training the agent on project-specific patterns and conventions
- Reducing iteration cycles by avoiding previously failed approaches
- Building a project-specific knowledge base of what works and what breaks
- Analyzing which code patterns correlate with test success or failure

## Commands
- `/rlcef enable` — Enable execution feedback learning for this project
- `/rlcef disable` — Disable feedback learning
- `/rlcef status` — Show learning stats: episodes, success rate, top patterns
- `/rlcef patterns` — List learned positive and negative code patterns
- `/rlcef replay <episode>` — Replay a specific learning episode
- `/rlcef export` — Export learned patterns for sharing across projects
- `/rlcef import <path>` — Import patterns from another project
- `/rlcef reset` — Reset all learned patterns to baseline

## Examples
```
/rlcef enable
# RLCEF enabled. Learning from: test results, build output, runtime errors
# Current episodes: 0 | Baseline success rate: --

/rlcef status
# Episodes: 247 | Success rate: 89% (was 71% at episode 50)
# Top positive patterns:
#   - Always use .map_err() for error conversion (+12% success)
#   - Include boundary tests for parsing functions (+8%)
# Top negative patterns:
#   - Avoid unwrap() in async contexts (caused 23 failures)
#   - Don't assume UTF-8 in file reading (caused 11 failures)

/rlcef patterns
# Positive (31 patterns): error handling, test coverage, type safety...
# Negative (18 patterns): unwrap in async, hardcoded paths, missing bounds...
```

## Best Practices
- Enable RLCEF early in a project to accumulate learning episodes
- Review learned patterns periodically to prune false correlations
- Export patterns from mature projects and import into new ones
- Combine with test coverage data for the strongest learning signal
- Reset patterns when making major architectural changes
