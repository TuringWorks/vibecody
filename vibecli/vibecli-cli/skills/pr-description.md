# PR Description Generator

Diff-aware pull request title and body generation. Matches Claude Code 1.x, Cursor 4.0, Copilot Workspace v2, and Devin 2.0.

## When to Use
- Generating a PR title and body from commits before pushing
- Summarising which files changed and why
- Including a structured test plan in the PR description
- Customising description by dominant commit type

## Commands
- `/pr-desc generate [--base <branch>]` — Generate PR description from commits
- `/pr-desc title` — Generate title only
- `/pr-desc preview` — Preview markdown body
- `/pr-desc write` — Write to `.pr-description.md`
- `/pr-desc config` — Show/edit generator config

## Output Sections
1. **Summary** — commit count + dominant type with emoji
2. **Changes** — per-commit bullet list (capped at max_commits_in_body)
3. **Files Changed** — deduplicated, sorted file list
4. **Test Plan** — checkboxes for unit tests, manual smoke, regression check

## Config
```
base_branch: main
include_file_list: true
max_commits_in_body: 10
include_test_plan: true
```
