# Proactive Agent

Background intelligence that continuously scans your codebase for issues, improvements, and opportunities. Detects bugs, performance problems, security risks, and stale dependencies without being asked, and surfaces actionable suggestions.

## When to Use
- Enabling always-on background code analysis during development
- Catching bugs and anti-patterns before they reach code review
- Monitoring for newly disclosed CVEs affecting your dependencies
- Detecting performance regressions from recent commits
- Getting proactive refactoring suggestions based on code smells

## Commands
- `/proactive start` — Enable background scanning on the current project
- `/proactive stop` — Disable background scanning
- `/proactive status` — Show scan status, queue depth, and findings count
- `/proactive findings` — List all current findings by severity
- `/proactive dismiss <id>` — Dismiss a finding as not applicable
- `/proactive config <sensitivity>` — Set sensitivity (low, medium, high, paranoid)
- `/proactive schedule <cron>` — Set custom scan schedule instead of continuous
- `/proactive focus <area>` — Focus scanning on specific areas (security, perf, style)

## Examples
```
/proactive start
# Background scanning enabled. Sensitivity: medium, watching 342 files.

/proactive findings
# [P1] Unbounded allocation in parse_input() - src/parser.rs:142
# [P2] SQL query missing parameterization - src/db/queries.rs:89
# [P2] Stale lock file: Cargo.lock 45 days behind Cargo.toml
# [P3] Dead code: fn legacy_handler() never called - src/routes.rs:203

/proactive focus security
# Scan focus set to: security. Queued 342 files for re-scan.
```

## Best Practices
- Start with medium sensitivity and adjust based on noise level
- Review and dismiss false positives to train the detection heuristics
- Use focus mode during security-critical development phases
- Schedule deep scans for off-hours to avoid CPU contention
- Integrate findings into your PR workflow for automated review comments
