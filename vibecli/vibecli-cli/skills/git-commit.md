---
name: Git Commit
description: Best practices for creating git commits
triggers: ["commit", "git commit", "stage", "check in"]
---

When creating a git commit:
1. Run `git status` and `git diff` to see all changes
2. Review the changes to understand what was modified and why
3. Stage specific files (prefer `git add <file>` over `git add -A`)
4. Write a concise commit message that focuses on "why" not "what"
5. Never skip hooks (--no-verify) unless explicitly asked
6. Prefer creating NEW commits over amending existing ones
7. Never commit files that likely contain secrets (.env, credentials)
8. Do not use `git add -i` or `git rebase -i` (interactive modes not supported)
