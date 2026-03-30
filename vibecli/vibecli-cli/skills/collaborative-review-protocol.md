# Collaborative Review Protocol

Multi-round code review system with structured comment threads, approval workflows, and quality metrics tracking. Measures review precision to distinguish real issues from false positives.

## Features
- **Multi-round reviews** — AI review, human review, and final approval rounds
- **Inline comments** — File:line targeted review comments with threading
- **Comment lifecycle** — Open, resolved, won't-fix statuses
- **Quality metrics** — Precision tracking (real issues vs false positives)
- **Checklists** — Configurable review checklists per session
- **Agent pushback** — AI can disagree with human corrections

## Commands
- `/creview start <title>` — Start a new review session
- `/creview comment <file:line> <msg>` — Add an inline comment
- `/creview resolve <id>` — Mark a comment as resolved
- `/creview approve` — Approve the current review round
- `/creview reject` — Request changes
- `/creview list` — List active review sessions
- `/creview stats` — Show review quality metrics

## Quality Metrics
- **Total Comments** — All comments across sessions
- **Resolved** — Comments marked as resolved
- **Real Issues** — Comments that identified actual bugs
- **False Positives** — Comments that were incorrect
- **Precision** — Real issues / (Real issues + False positives)

## Example
```
/creview start "Auth refactor review"
/creview comment src/auth.rs:42 "Potential SQL injection here"
/creview approve
/creview stats
```
