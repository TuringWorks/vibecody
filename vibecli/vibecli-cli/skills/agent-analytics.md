# Agent Analytics

Enterprise usage analytics with per-user, per-team, and per-project dashboards. Track token consumption, task completion rates, time savings, ROI metrics, and model performance to optimize AI investment.

## When to Use
- Measuring ROI of AI coding assistant adoption across teams
- Tracking per-user and per-team token consumption and costs
- Identifying which tasks benefit most from AI assistance
- Generating executive reports on AI productivity impact
- Detecting usage anomalies or potential misuse patterns

## Commands
- `/analytics dashboard` — Open the analytics dashboard summary
- `/analytics user <username>` — Show usage stats for a specific user
- `/analytics team <team>` — Show aggregate stats for a team
- `/analytics project <name>` — Show stats scoped to a project
- `/analytics export <format>` — Export data as CSV, JSON, or PDF
- `/analytics roi` — Calculate estimated ROI based on time savings
- `/analytics trends <period>` — Show usage trends over a time period
- `/analytics alerts` — Configure usage alerts and thresholds

## Examples
```
/analytics dashboard
# Organization: Acme Corp (42 users, 8 teams)
# This month: 2.4M tokens, $342 cost, 1,847 tasks completed
# Estimated time saved: 1,240 hours ($186,000 at avg eng rate)
# ROI: 543x

/analytics team backend
# Team: Backend (12 users)
# Top users: alice (312 tasks), bob (287), carol (245)
# Most common tasks: code review (34%), bug fix (28%), refactor (18%)
# Avg task completion: 3.2 min (manual baseline: 28 min)

/analytics export pdf
# Exported: analytics-2026-03-report.pdf (12 pages)
```

## Best Practices
- Set up weekly automated reports for engineering leadership
- Track trends over months to identify adoption patterns
- Use project-level analytics to justify AI investment per product
- Configure alerts for unusual token spikes that may indicate misuse
- Compare ROI across teams to identify training and adoption opportunities
