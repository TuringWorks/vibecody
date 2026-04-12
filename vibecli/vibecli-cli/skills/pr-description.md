# PR Description Generator

Generates PR titles and bodies from diff context and commit history.
Matches Claude Code 1.x, Cursor 4.0, and Copilot Workspace v2.

## Generated Sections
- **Title** — ≤70 chars, from first feat/fix commit or branch name
- **Summary** — bullet points from notable commits (max 5)
- **Changes** — files changed, +lines, −lines
- **Test plan** — checklist with security/size-aware hints
- **Labels** — enhancement, bug, breaking-change, large-pr, security

## Reviewer Hints
- Large PR (>20 files or >500 lines changed): review by area
- Touches auth/security files: security review required
- Breaking change: verify downstream compatibility

## Commands
- `/pr describe` — generate description for current branch
- `/pr title` — generate title only
- `/pr labels` — infer labels from commits
