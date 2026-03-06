---
triggers: ["feature development", "spec to code", "implementation pipeline", "feature workflow", "plan implement test"]
tools_allowed: ["read_file", "write_file", "bash"]
category: workflow
---

# Feature Development Workflow

When implementing a new feature (inspired by fire-flow):

1. **Spec**: define requirements, acceptance criteria, edge cases — write them down
2. **Plan**: design approach, identify files to change, estimate scope — get alignment
3. **Branch**: create feature branch from main — `feat/descriptive-name`
4. **Implement**: write code in small, testable increments — commit often
5. **Test**: write tests alongside code — unit tests for logic, integration for flows
6. **Lint**: run formatters and linters — fix all issues before review
7. **Review**: self-review diff → request peer review → address feedback
8. **Verify**: run full test suite, check in staging/preview environment
9. **Merge**: squash or merge to main — clean commit message summarizing the feature
10. **Monitor**: watch metrics after deploy — error rates, latency, user behavior
11. Document: update README, API docs, changelog as part of the PR
12. Retrospect: what went well? what to improve? — apply learnings to next feature
