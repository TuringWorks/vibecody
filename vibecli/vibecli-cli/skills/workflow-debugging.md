---
triggers: ["debugging", "reproduce bug", "root cause", "debug workflow", "fire-debug", "bisect debug"]
tools_allowed: ["read_file", "write_file", "bash"]
category: workflow
---

# Debugging Workflow

When debugging issues (inspired by fire-flow /fire-debug):

1. **Reproduce**: get a reliable reproduction — specific input, environment, steps
2. **Isolate**: narrow down to the smallest failing case — remove unrelated complexity
3. **Hypothesize**: form a theory about the cause before reading code
4. **Verify**: test your hypothesis — add logging, use debugger, check data
5. **Trace**: follow the execution path — input → transformation → output
6. Read error messages carefully — they usually tell you exactly what's wrong
7. Use `git bisect` to find the commit that introduced the bug
8. Check recent changes: `git log --oneline -20` — recent code is most likely source
9. Binary search: comment out half the code, see if bug persists, repeat
10. Rubber duck debugging: explain the problem out loud — often reveals the answer
11. **Fix**: write a test that reproduces the bug FIRST, then fix until test passes
12. **Verify**: run full test suite — ensure fix doesn't break other things
