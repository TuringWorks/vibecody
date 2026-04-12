# Inline Diff Accept/Reject

Hunk-level patch application with partial acceptance. Matches Claude Code 1.x, Cursor 4.0, and Copilot inline diff UI.

## Workflow
1. Create `InlineDiffSession` with original file content
2. `propose(start_line, original_len, replacement_lines)` — add hunks
3. `accept(id)` / `reject(id)` — decide per hunk
4. `apply()` — apply accepted hunks (bottom-up to avoid offset shift)

## Key Types
- **InlineDiffSession** — manages hunks + decisions for one file
- **ProposedHunk** — start_line, original_lines, replacement_lines, decision
- **HunkDecision** — Pending | Accepted | Rejected
- **ApplicationResult** — new_content, accepted/rejected/pending counts, line_delta

## Commands
- `/diff accept <hunk-id>` — accept a proposed change
- `/diff reject <hunk-id>` — reject a proposed change
- `/diff accept-all` — accept all pending hunks
- `/diff preview <hunk-id>` — preview a single hunk applied
