---
name: Debugging
description: Systematic approach to debugging errors and issues
triggers: ["debug", "error", "bug", "issue", "crash", "fix", "broken", "failing", "stacktrace", "traceback"]
---

When debugging:
1. **Reproduce first**: Understand the exact error. Read the full error message, stack trace, and logs.
2. **Locate the source**: Use `search_files` to find the relevant code. Trace from the error location backwards.
3. **Understand the context**: Read the surrounding code. Check recent git changes (`git log --oneline -10`, `git diff`).
4. **Form a hypothesis**: Use `think` to reason about what could cause this specific error.
5. **Test the hypothesis**: Make a minimal, focused fix. Don't change unrelated code.
6. **Verify the fix**: Run the build/tests to confirm the error is resolved and nothing else broke.
7. **Don't guess**: If the first fix doesn't work, re-read the error and gather more information rather than trying random changes.

Common patterns:
- Off-by-one errors: Check loop bounds and array indices
- Null/None: Trace where the value could be unset
- Race conditions: Look for shared mutable state without synchronization
- Import/dependency errors: Check versions and paths
- Type mismatches: Read both sides of the assignment/call carefully
