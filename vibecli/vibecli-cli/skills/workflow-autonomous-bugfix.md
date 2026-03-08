---
triggers: ["autonomous bug fix", "fix bug autonomously", "auto debug", "fix failing tests", "fix ci", "debug from logs"]
tools_allowed: ["read_file", "write_file", "bash"]
category: workflow
---

# Autonomous Bug Fixing

Fix bugs without hand-holding — zero context switching for the user:

## Process
1. **Read the signal**: logs, error messages, stack traces, failing test output
2. **Reproduce**: run the failing test or trigger the error path
3. **Diagnose**: trace the execution path from error back to root cause
4. **Fix**: apply the minimal, correct fix at the root cause
5. **Verify**: run the test suite, confirm the fix works, check for regressions
6. **Report**: brief summary of what broke, why, and what you changed

## Principles
- Just fix it. Don't ask "what should I do?" — read the error and resolve it
- Point at logs, errors, failing tests — then resolve them
- Go fix failing CI tests without being told how
- No temporary workarounds. Find and fix the root cause
- If the fix touches more than the immediate bug, pause and confirm scope

## CI Failure Protocol
1. Read the CI output: `gh run view --log-failed` or equivalent
2. Identify the failing step and error message
3. Reproduce locally if possible: run the same command
4. Fix the root cause in the source code
5. Run tests locally to confirm
6. Commit and push — don't wait for instructions

## When to Escalate
- The bug is in a third-party dependency you can't modify
- The fix requires a design decision beyond your scope
- You've spent 3+ attempts without progress — re-plan instead
