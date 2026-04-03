---
name: Simplify
description: Review code for reuse, quality, and efficiency then fix issues
triggers: ["simplify", "cleanup", "refactor", "clean up", "optimize", "improve code"]
---

When simplifying code:
1. Read the changed files and understand what they do
2. Look for:
   - Duplicated code that could be extracted into a shared function
   - Overly complex logic that could be simplified
   - Unused imports, variables, or dead code
   - Inefficient patterns (N+1 queries, unnecessary allocations, redundant loops)
   - Missing error handling at system boundaries
3. Make focused changes — don't refactor everything at once
4. Preserve the existing API/interface unless the user asks to change it
5. Run tests after each change to ensure correctness
6. Prefer readability over cleverness
