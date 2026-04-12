# AI Semantic Merge

AI-assisted three-way merge conflict resolution that classifies conflicts by type and auto-resolves trivial cases. Matches GitHub Copilot Workspace v2's semantic merge.

## Conflict Types
| Type | Auto-Resolved | Strategy |
|---|---|---|
| Whitespace | ✓ | Take `ours` |
| ImportOrder | ✓ | Merge + sort |
| NonOverlapping | ✓ | Append both |
| Rename | Manual | Flag for review |
| Logic | Manual | Preserve markers |
| Structural | Manual | Preserve markers |

## Key Types
- **ConflictHunk** — parsed `<<<<<<<` … `>>>>>>>` block with classification
- **SemanticMergeResolver** — resolves hunks by kind
- **MergeSummary** — auto_resolved / needs_review counts + auto_resolve_rate

## Commands
- `/merge resolve <file>` — parse and auto-resolve a file with conflicts
- `/merge status` — show conflict summary for a file
- `/merge diff <file>` — show which hunks need review

## Examples
```
/merge resolve src/config.rs
# 5 conflicts found: 3 auto-resolved (whitespace, imports), 2 need review (logic)
# Auto-resolve rate: 60%
```
