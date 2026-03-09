---
name: Event-Driven Automations
category: automation
triggers:
  - automation
  - automations
  - event-driven
  - webhook trigger
  - github webhook
  - slack automation
  - pagerduty alert
  - linear automation
  - spawn agent
  - external trigger
  - event handler
---

# Event-Driven Automations

Set up automation rules that spawn agent tasks from external events.

## Supported Triggers

1. **GitHub**: push, pull_request, issues, release, workflow_run
2. **Slack**: app_mention, message, reaction_added
3. **Linear**: issue created/updated/completed
4. **PagerDuty**: incident triggered/acknowledged/resolved
5. **Cron**: time-based schedules (cron expressions)
6. **File Watch**: glob-pattern file system changes
7. **Webhook**: generic HTTP webhook from any service

## Best Practices

1. Use specific event filters to avoid noisy automation fires
2. Set `max_turns` to prevent runaway agent sessions
3. Enable sandbox mode for automations that modify code
4. Use `{{variable}}` placeholders in prompt templates for dynamic context
5. Verify webhook signatures (HMAC-SHA256) for security
6. Start with conservative filters, then widen as you gain confidence
7. Monitor the task log to spot failed or slow automations
8. Use the `/automation test` command to dry-run rules with sample payloads
9. Combine with self-review gate to ensure automated changes pass quality checks
10. Keep prompt templates focused — one clear goal per automation rule

## REPL Commands

- `/automation list` — List all automation rules
- `/automation add` — Create a new automation rule (interactive wizard)
- `/automation remove <id>` — Remove a rule
- `/automation enable <id>` — Enable a disabled rule
- `/automation disable <id>` — Disable a rule
- `/automation test <id>` — Dry-run with a sample event
- `/automation stats` — Show automation statistics
