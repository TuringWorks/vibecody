---
triggers: ["incident response", "outage", "RCA", "postmortem", "on-call", "mitigation", "rollback"]
tools_allowed: ["read_file", "write_file", "bash"]
category: workflow
---

# Incident Response

When handling production incidents:

1. **Triage**: assess severity (P0 total outage → P3 minor degradation) — assign incident commander
2. **Communicate**: notify stakeholders — status page update, Slack channel, customer comms
3. **Mitigate**: restore service FIRST — rollback, feature flag off, traffic shift, scale up
4. Don't debug in production during an outage — restore service, then investigate
5. **Investigate**: check dashboards, logs, recent deploys, external dependencies
6. Timeline: document what happened, when, what was done — real-time notes
7. Rollback: if a deploy caused the issue, revert immediately — fix forward later
8. Feature flags: disable suspicious features without deploying new code
9. **Root Cause Analysis**: 5 Whys technique — dig past symptoms to systemic causes
10. **Postmortem**: blameless, focus on process improvements — what will prevent recurrence?
11. Action items: concrete, assigned, time-boxed — track to completion
12. Share learnings: publish postmortem internally — help other teams avoid similar issues
