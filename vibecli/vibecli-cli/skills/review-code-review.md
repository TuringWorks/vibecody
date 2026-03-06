---
triggers: ["code review", "review checklist", "review feedback", "PR review", "review severity"]
tools_allowed: ["read_file", "write_file", "bash"]
category: review
---

# Code Review Best Practices

When reviewing code (inspired by Claude Code/fire-flow review patterns):

1. Use a structured checklist: correctness, security, performance, readability, tests, docs
2. Severity levels: Critical (must fix), Major (should fix), Minor (nice to have), Nit (style)
3. Be specific: "This SQL query is vulnerable to injection on line 42" not "Security issue"
4. Suggest fixes: include code snippets showing the recommended change
5. Review for correctness first: does the code do what it claims? Edge cases? Error handling?
6. Security review: input validation, auth checks, SQL injection, XSS, secrets in code
7. Performance review: N+1 queries, unnecessary allocations, missing indexes, sync I/O in async
8. Readability: clear naming, appropriate abstraction level, commented "why" not "what"
9. Test coverage: are new code paths tested? Are edge cases covered? Happy AND sad paths?
10. Approve with minor nits — don't block PRs for style preferences
11. Focus on the diff — review what changed, not the entire file
12. Ask questions when intent is unclear — "What happens if X is null here?"
