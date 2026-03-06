---
triggers: ["cloud cost", "cost optimization", "right-sizing", "spot instance", "cloud spending", "FinOps"]
tools_allowed: ["read_file", "write_file", "bash"]
category: workflow
---

# Cloud Cost Optimization

When optimizing cloud spending:

1. **Visibility first**: tag all resources by team/project/environment — you can't optimize what you can't see
2. **Right-sizing**: analyze CPU/memory utilization — downsize over-provisioned instances
3. **Reserved/Savings Plans**: commit to 1-3 year terms for steady workloads — 30-60% savings
4. **Spot/Preemptible**: use for batch processing, CI/CD, fault-tolerant workloads — 60-90% savings
5. **Auto-scaling**: scale down during off-hours — schedule-based for predictable patterns
6. **Storage tiering**: move infrequently accessed data to cheaper tiers (S3 Glacier, Archive)
7. **Unused resources**: terminate stopped instances, delete unattached EBS volumes, old snapshots
8. **Database optimization**: use serverless/on-demand for variable workloads, reserved for steady
9. **CDN/caching**: reduce origin requests — lower compute and bandwidth costs
10. **Architecture**: use serverless (Lambda/Cloud Functions) for spiky, low-volume workloads
11. **Monitoring**: set up billing alerts at 50%, 80%, 100% of budget thresholds
12. **Regular reviews**: monthly cost review meetings — compare actuals vs. budget
