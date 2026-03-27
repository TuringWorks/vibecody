# Next Task Prediction

Workflow-level prediction that suggests what to do next based on your current context, recent actions, project state, and team patterns. Learns from your habits to surface the right task at the right time.

## When to Use
- Getting suggestions for what to work on next after completing a task
- Identifying forgotten follow-up tasks (tests, docs, migrations)
- Discovering tasks implied by recent code changes (update types, fix imports)
- Surfacing blocked or stale work items that need attention
- Reducing context-switching overhead by predicting logical next steps

## Commands
- `/nexttask` — Get the top 3 predicted next tasks with confidence scores
- `/nexttask accept <id>` — Accept and start working on a predicted task
- `/nexttask dismiss <id>` — Dismiss a prediction as not relevant
- `/nexttask history` — Show prediction history and accuracy metrics
- `/nexttask learn` — Retrain predictions from recent activity patterns
- `/nexttask context` — Show the signals used for current predictions
- `/nexttask config <source>` — Enable/disable prediction sources (git, issues, tests)

## Examples
```
/nexttask
# Based on your recent changes to auth.rs:
# [1] Write tests for new OAuth flow (confidence: 0.92)
#     Reason: 3 new public functions with 0% test coverage
# [2] Update API docs for /auth/callback (confidence: 0.85)
#     Reason: Endpoint signature changed, docs are stale
# [3] Fix failing CI: test_token_refresh (confidence: 0.78)
#     Reason: Related test broke 2 commits ago

/nexttask history
# Last 50 predictions: 72% accepted, 18% dismissed, 10% expired
# Most accurate source: test coverage gaps (89% acceptance)
```

## Best Practices
- Dismiss irrelevant predictions to improve future accuracy
- Enable all context sources for the most informed predictions
- Review predictions at the start of each work session
- Use accept to build a task trail for session summaries
- Retrain periodically after workflow changes or team reorganizations
