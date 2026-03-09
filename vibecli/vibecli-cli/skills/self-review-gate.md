---
name: Agent Self-Review Gate
category: agent
triggers:
  - self-review
  - self review
  - review gate
  - agent review
  - pre-completion check
  - quality gate
  - lint check
  - test before complete
  - security scan
  - auto-review
---

# Agent Self-Review Gate

The self-review gate runs automated quality checks before an agent marks a task complete.

## Check Types

1. **Build** — `cargo check`, `npm build`, `go build` (language-detected)
2. **Lint** — `cargo clippy`, `eslint`, `ruff`, `golangci-lint`
3. **Test** — `cargo test`, `npm test`, `pytest`, `go test`
4. **Security** — Secret scanning (AWS keys, GitHub tokens, private keys), dependency audit
5. **Format** — `cargo fmt --check`, `prettier --check`
6. **TypeCheck** — `tsc --noEmit`, `mypy`
7. **DiffReview** — AI reviews its own diff for quality

## How It Works

1. Agent finishes its task and signals completion
2. Self-review gate runs all configured checks
3. If checks pass → task marked complete
4. If checks fail → feedback injected into agent context, agent iterates
5. After `max_retries` → forced approval with warnings

## Best Practices

1. Enable all four default checks (build, lint, test, security) for comprehensive coverage
2. Set `max_retries = 3` to balance quality with speed
3. Use `min_blocking_severity = "error"` to allow warnings through
4. Enable `fail_on_warning` for critical production code changes
5. The security scanner catches AWS keys, GitHub tokens, private keys, and Slack webhooks
6. Check the self-review report in the VibeUI SelfReview panel for detailed findings
7. Custom checks can be added via the `Custom("name")` check kind
8. Self-review integrates with the existing build double-check in agent.rs
9. Use `/self-review config` to adjust settings without editing config.toml
10. Review the markdown report for audit trail of what the agent fixed

## Configuration

```toml
[agent]
self_review = true
self_review_max_retries = 3
self_review_checks = ["build", "lint", "test", "security"]
self_review_fail_on_warning = false
self_review_min_blocking_severity = "error"
```
