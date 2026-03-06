---
triggers: ["pull request", "PR description", "PR workflow", "review process", "CI gate", "merge strategy"]
tools_allowed: ["read_file", "write_file", "bash"]
category: review
---

# Pull Request Best Practices

When creating and managing pull requests:

1. Keep PRs small: 200-400 lines max — large PRs get rubber-stamped, small PRs get thorough review
2. Title: imperative mood, under 70 chars — "Add user authentication" not "Added auth stuff"
3. Description: What changed, Why, How to test, Screenshots for UI changes
4. One concern per PR: don't mix features with refactoring — separate them
5. Self-review before requesting: re-read your own diff, remove debug code, check TODOs
6. CI gates: require passing tests, linting, type checking, and security scans before merge
7. Use draft PRs for work-in-progress — request review only when ready
8. Link related issues: "Fixes #123" auto-closes the issue on merge
9. Rebase on main before merge — keep a clean, linear history
10. Respond to all review comments — even if just "Done" or explaining why you disagree
11. Use merge commits for feature branches, squash for single-concern PRs
12. Delete branches after merge — keep the repository clean
