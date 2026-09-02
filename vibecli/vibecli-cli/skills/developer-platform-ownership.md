---
name: "Developer Platform Ownership"
description: "Developer Platform Ownership: operating CI/CD, observability and the internal developer platform as products with users, SLOs, a roadmap and a deprecation policy rather than as infrastructure ticket queues. Covers platform SLOs, golden paths, self-service boundaries, adoption metrics, funding, and the failure modes of central platform teams. Use when the task involves internal developer platform, IDP, platform team, golden path, self-service infrastructure, CI/CD ownership, observability platform, or platform as a product."
category: devex
triggers: ["internal developer platform", "IDP", "platform as a product", "platform team", "golden path", "self-service infrastructure", "CI/CD ownership", "observability platform", "developer platform", "backstage", "paved road"]
tools_allowed: ["read_file", "write_file", "bash"]
---

# Owning the developer platform as a product

The distinction that decides everything: **infrastructure is something teams
file tickets against; a platform is something teams choose.** If your platform
cannot be declined, you will never learn whether it is good.

## Product mechanics, applied to a platform

| Product concept | Platform equivalent | Concretely |
|---|---|---|
| Users | Engineers in product teams | Named, segmented, talked to weekly |
| Value proposition | Time from idea to production | Stated as a number, measured |
| Roadmap | Public, voted on | In the repo, not in a director's deck |
| SLO | Platform availability and latency | Pipeline queue time, portal uptime |
| Support | Office hours + channel + docs | Response time measured |
| Adoption metric | % of services on the golden path | Reported monthly |
| Deprecation policy | Written, with timelines | N-2 support, 12 months notice |

## Platform SLOs that matter to users

Pick these, not resource utilisation:

- **Pipeline queue time, p95** — how long before a build *starts*. Queue time
  is felt as "CI is slow" even when build time is fine, and it is usually the
  cheaper of the two to fix.
- **Pipeline end-to-end, p50 and p95** — publish both. The p50 is the
  experience; the p95 is why they complain.
- **Environment provisioning time, p95** — from request to usable.
- **Portal / catalog availability** — if the IDP is down, every self-service
  path reverts to tickets.
- **Time-to-first-successful-build for a new service** — the golden path's own
  latency.

Every one of these is a number a product engineer feels. None of them is a
number about your cluster.

## Golden paths

A golden path is the opinionated, well-supported route through a common task.
It is the primary adoption mechanism for every standard the practices program
publishes, so it belongs to the platform team.

Define one for each of: create a service, add CI/CD, add observability, connect
a datastore, handle secrets, deploy to production, run a load test.

Rules:
- **Opinionated.** One supported way. Five documented alternatives is a
  reference manual, not a golden path.
- **Complete.** A template that scaffolds a service without CI, tests,
  dashboards and alerts has moved the work rather than removed it.
- **Tested quarterly.** Run it end to end on a clean machine as a real task.
  Golden paths rot silently — nothing fails when they do, teams just quietly
  stop using them.
- **Deviation is possible, not default.** Teams off the path get less support,
  not a policy violation. Make the support difference explicit and honest.

## Where self-service stops

Self-service without a boundary becomes an unbounded cost centre. Draw the line
in advance:

- **Self-service, guardrails only**: standard-size databases, caches, queues,
  DNS, certs, non-prod environments, dashboards.
- **Self-service with automatic cost attribution**: anything with per-hour
  spend. Attribution, not approval — a team that can see its bill regulates
  itself far better than a team that must request permission.
- **Requires review**: production data access, cross-region replication,
  anything touching regulated data, and anything that would exceed a
  pre-agreed budget envelope.

Enforce the first two with policy-as-code so they are guardrails, not gates.

## Measuring the platform

```
vibecli --devex dora --path <repo>          # does the platform make delivery faster?
vibecli --devex onboarding --path <repo>    # does a newcomer get productive?
vibecli --devex scorecard --path <repo>     # both, in one view
```

Panel: **Cloud & Platform → Developer Excellence**. Also relevant:
**CI/CD** panel for pipeline health, **Observability** panel for telemetry.

The platform's own success metric is **the delta it produces in its users'
numbers**, not its own uptime. A platform with 99.99% availability that nobody
adopted has failed. Report: adoption %, and the DORA delta between services on
the golden path and services off it. That second number is the funding case,
and it is the one platform teams most often neglect to compute.

## The four ways central platform teams fail

1. **Building what is interesting rather than what is blocking.** Remedy: the
   support-theme list. Every fortnight, the top three themes from the support
   channel go on the roadmap or get an explicit written "not now".
2. **Mandate before value.** A platform that teams are forced onto never gets
   the feedback that would have made it good. Earn adoption for the first
   third of the org before making anything mandatory.
3. **No deprecation policy.** Every version supported forever; the team's whole
   capacity goes to maintenance within three years. Write the policy on day one,
   when it costs nothing.
4. **Invisible cost.** Nobody outside the team can say what the platform costs
   or what it saves, so it is the first thing cut in a lean year. See
   `engineering-investment-case.md`.
