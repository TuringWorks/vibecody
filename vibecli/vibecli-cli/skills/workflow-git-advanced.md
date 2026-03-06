---
triggers: ["git rebase", "git bisect", "git worktree", "git submodule", "sparse checkout", "git advanced", "interactive rebase"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["git"]
category: workflow
---

# Advanced Git Workflows

When using advanced git features:

1. Interactive rebase: `git rebase -i HEAD~5` — squash, reorder, edit, fixup commits
2. Bisect: `git bisect start` → `git bisect bad` → `git bisect good v1.0` — binary search for bugs
3. Worktrees: `git worktree add ../feature-branch feature` — work on multiple branches simultaneously
4. Cherry-pick: `git cherry-pick abc123` — apply specific commits to another branch
5. Stash: `git stash push -m "description"` — save work-in-progress without committing
6. Reflog: `git reflog` — recover lost commits, find previous HEAD positions
7. Sparse checkout: clone only needed directories — `git sparse-checkout set src/ tests/`
8. Submodules: `git submodule add url path` — pin external repos at specific commits
9. Blame: `git blame -L 10,20 file.rs` — find who changed specific lines and when
10. Log: `git log --oneline --graph --all` for visual history; `git log -S "search"` for code changes
11. Reset: `--soft` (keep staged), `--mixed` (keep unstaged), `--hard` (discard all) — know the difference
12. Hooks: `pre-commit` for linting, `commit-msg` for format validation, `pre-push` for tests
