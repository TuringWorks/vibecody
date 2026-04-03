---
name: Pull Request Creation
description: Best practices for creating pull requests
triggers: ["pull request", "PR", "gh pr", "create pr"]
---

When creating a pull request:
1. Check `git status` and `git log` to understand all commits on the branch
2. Look at `git diff main...HEAD` to see the full changeset
3. Push the branch with `git push -u origin <branch>`
4. Create PR with a short title (under 70 characters) and detailed body
5. PR body should include: Summary (1-3 bullets), Test plan (checklist)
6. Never force-push to main/master
7. If tests fail, fix them before creating the PR
