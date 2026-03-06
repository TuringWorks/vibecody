---
triggers: ["standup", "status report", "progress tracking", "daily update", "blockers report", "sprint update"]
tools_allowed: ["read_file", "write_file", "bash"]
category: workflow
---

# Standup & Status Reporting

When preparing status updates and progress reports:

1. Three questions: What did I do? What will I do? Any blockers?
2. Be specific: "Fixed auth token expiry bug (#234)" not "Worked on auth stuff"
3. Mention impact: "Reduced API latency by 40% by adding Redis cache to /users endpoint"
4. Flag blockers early: "Blocked on API spec from team X — need by Wednesday for sprint goal"
5. Use git log for evidence: `git log --since="yesterday" --author="me" --oneline`
6. Track sprint progress: tasks completed vs. remaining — flag risks to sprint goal
7. Async standups: write in Slack/channel for remote teams — time zones make sync hard
8. Keep it brief: 2-3 sentences per question — save details for 1:1s or threads
9. Include metrics: "Tests: 94% coverage (+2%), Build: 45s (-10s), 0 open P0 bugs"
10. Weekly summary: aggregate daily standups into a weekly progress report
11. Sprint retrospective inputs: note what went well and what was painful throughout the sprint
12. Use tools: GitHub Activity, JIRA dashboard, or git-standup for automated summaries
