---
triggers: ["deploy checklist", "pre-flight", "rollback plan", "deployment process", "release checklist"]
tools_allowed: ["read_file", "write_file", "bash"]
category: workflow
---

# Deployment Checklist

When deploying to production:

1. **Pre-flight**: all tests pass, linting clean, no known blockers, changelog updated
2. **Feature flags**: new features behind flags — deploy disabled, enable gradually
3. **Database migrations**: run and verify before deploying application code
4. **Rollback plan**: know how to revert — previous version tagged, rollback tested
5. **Monitoring**: dashboards open — error rates, latency, CPU, memory, queue depth
6. **Deploy**: use CI/CD pipeline — never deploy manually from a laptop
7. **Canary**: route 5% of traffic to new version — watch for errors for 15 minutes
8. **Gradual rollout**: 5% → 25% → 50% → 100% — stop and investigate if errors spike
9. **Smoke test**: hit critical endpoints after deploy — verify core user journeys work
10. **Communication**: notify team in Slack — "Deploying v1.2.3 to production"
11. **Post-deploy**: monitor for 30 minutes — check logs, metrics, customer reports
12. **Document**: record deployment time, version, any issues — update runbook if needed
