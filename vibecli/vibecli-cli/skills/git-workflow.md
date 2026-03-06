---
triggers: ["git commit", "branch", "merge", "rebase", "git workflow"]
tools_allowed: ["bash"]
requires_bins: ["git"]
category: devops
---

# Git Workflow

1. Branch naming: `feat/description`, `fix/description`, `chore/description`
2. Commits: imperative mood, <72 chars title, blank line, body with "why"
3. Prefer rebase for linear history on feature branches
4. Use merge commits when merging feature branches to main
5. Never force-push to main/master
6. Sign commits with GPG when required
7. Use `.gitignore` for build artifacts, dependencies, secrets, IDE files
8. Tag releases with semantic versioning: `v1.2.3`
9. Use `git stash` for WIP, not half-finished commits
10. Review your own diff before committing: `git diff --staged`
