# Issue Triage

Autonomous issue triage for GitHub and Linear. Automatically classifies incoming issues, applies labels, estimates priority and effort, assigns to appropriate team members, and drafts initial responses with reproduction steps or clarifying questions.

## When to Use
- Automatically labeling and prioritizing new GitHub issues
- Triaging Linear tickets based on content analysis and historical patterns
- Generating initial responses to bug reports with troubleshooting steps
- Routing issues to the right team based on affected code areas
- Identifying duplicate issues and linking them together

## Commands
- `/triage start` — Begin watching for new issues on connected repos
- `/triage stop` — Stop automatic triage
- `/triage run <issue-url>` — Manually triage a specific issue
- `/triage rules` — Show current triage rules and label mappings
- `/triage add-rule <condition> <action>` — Add a custom triage rule
- `/triage stats` — Show triage accuracy and volume statistics
- `/triage review` — Review recent auto-triage decisions for correction
- `/triage config <platform> <token>` — Configure GitHub or Linear connection

## Examples
```
/triage run https://github.com/org/repo/issues/456
# Classification: bug (confidence: 0.94)
# Labels: [bug, P2, area:auth, needs-repro]
# Suggested assignee: @alice (auth module owner)
# Draft response: "Thanks for reporting. Could you share your OS version..."

/triage stats
# Last 30 days: 142 issues triaged, 91% label accuracy
# Avg time to first response: 2.3 min (was 4.2 hours manual)
```

## Best Practices
- Review auto-triage decisions weekly to improve classification accuracy
- Define clear label taxonomies before enabling automatic labeling
- Set confidence thresholds for auto-apply vs needs-review actions
- Use custom rules for project-specific patterns the model might miss
- Keep draft responses concise and always request reproduction steps for bugs
