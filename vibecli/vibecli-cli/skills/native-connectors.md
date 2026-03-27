# Native Connectors

Pre-built integrations for 20+ popular services including Slack, Jira, Confluence, Notion, PagerDuty, Datadog, Sentry, and more. Pull context from external tools directly into the agent without custom MCP servers.

## When to Use
- Pulling Jira ticket details into coding context automatically
- Posting status updates to Slack channels from the agent
- Fetching Sentry error details to debug production issues
- Querying Datadog metrics to inform performance optimization
- Syncing Notion docs or Confluence pages as project context

## Commands
- `/connect add <service>` — Add a new service connector
- `/connect remove <service>` — Remove a service connector
- `/connect list` — List all configured connectors and their status
- `/connect test <service>` — Test connectivity to a service
- `/connect pull <service> <query>` — Pull data from a connected service
- `/connect push <service> <data>` — Push data to a connected service
- `/connect sync <service>` — Sync latest data from a service into context
- `/connect catalog` — Show all available connector types

## Examples
```
/connect catalog
# Available: Slack, Jira, Confluence, Notion, GitHub, GitLab,
# PagerDuty, Datadog, Sentry, Linear, Figma, Asana, Trello,
# Monday, Airtable, Supabase, Firebase, Vercel, Netlify, AWS, GCP

/connect add jira
# Configure Jira: URL? https://team.atlassian.net
# API token? [stored in keychain]
# Connected! 3 projects visible.

/connect pull jira "PROJECT-142"
# PROJECT-142: Fix login timeout on mobile
# Status: In Progress | Priority: High | Sprint: Sprint 23
# Description: Users report 30s timeout on iOS Safari...
```

## Best Practices
- Store all credentials in the system keychain, never in config files
- Test connectors after setup to verify permissions are correct
- Use pull for on-demand context and sync for continuous background updates
- Limit sync frequency to avoid API rate limits on external services
- Review connector permissions to follow least-privilege principles
