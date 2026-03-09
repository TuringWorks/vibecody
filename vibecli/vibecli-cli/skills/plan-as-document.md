# Plan-as-Document with Feedback

Create structured plan documents with human review loops before execution.

## Triggers
- "plan document", "create plan", "review plan", "plan feedback"
- "plan approval", "plan markdown", "step-by-step plan"

## Usage
```
/plan create "Refactor auth module"
/plan step "Update schema" --type task
/plan comment step-1 "Why this approach?" --type question
/plan review bob                  # Submit for review
/plan approve                     # Approve plan
/plan reject "Needs more detail"  # Reject with reason
/plan revise                      # Bump version, back to draft
/plan export                      # Export as markdown
```

## Features
- Plan lifecycle: Draft -> InReview -> Approved/Rejected, version bumping on revision
- PlanStep with 8 status types, file change tracking, dependency references
- 5 comment types: Approval, Rejection, Question, Suggestion, Note
- 4 feedback actions: Approve, Reject, RequestChanges, AskQuestion
- Markdown export with step badges, file changes, dependencies, inline comments
- Markdown import for round-trip editing (from_markdown)
- Progress percentage tracking (completed + skipped steps)
- Total estimated lines across all steps
- Unresolved comment tracking (plan-level + step-level)
