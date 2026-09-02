---
name: "Developer Excellence Director Operating System"
description: "Developer Excellence Director Operating System: the routing skill for a Technology Director of Developer Excellence / Enterprise Engineering Practices & Platforms. Maps the three pillars of the role — a global practices program, strategic developer-platform ownership, and engineering leadership — onto the VibeCody skills, commands and panels that do the work. Use when the task involves developer excellence, engineering practices program, developer productivity, internal developer platform, DORA, SPACE, engineering standards, platform ownership, or a director-level engineering operating cadence."
category: devex
triggers: ["developer excellence", "engineering practices program", "technology director", "director of engineering practices", "enterprise engineering", "developer productivity", "engineering standards", "internal developer platform", "IDP", "platform ownership", "engineering effectiveness", "DORA", "SPACE"]
tools_allowed: ["read_file", "write_file", "bash"]
---

# Developer Excellence — the director's operating system

This is the **router** for the role. It does not do the work; it says which
skill, command and panel does, and in what order. Load it first, then load the
one or two skills the task actually needs.

The role has three pillars. Everything below hangs off one of them.

| Pillar | The question it answers | Where the evidence lives |
|---|---|---|
| **Global Practices Program** | Are we building the same way, well, everywhere? | `vibecli --devex practices`, Code Analysis panel, CI/CD panel |
| **Strategic Developers' Platform Ownership** | Does the platform make the right way the fast way? | `vibecli --devex dora`, `vibecli --devex onboarding`, Cloud & Platform panel |
| **Engineering Leadership** | Do the people with the most influence agree, and are they carrying it? | Engagement panel, Company panel, Project Hub |

## The first hour on any new org

Run this before writing a strategy. A strategy written before a measurement is
a preference.

```
vibecli --devex scorecard --path <repo>       # what the code says about delivery
vibecli --devex practices --path <repo>       # what standards are actually present
vibecli --devex space      --path <repo>      # the experience half, and its gaps
vibecli --devex onboarding --path <repo>      # what a new joiner walks into
vibecli --devex report --path <repo> > devex-baseline.md
```

Read the `unmeasured` block before the numbers. On a first run it is usually
longer than the measured block, and it is the more valuable half: it names
every place the organisation cannot currently see itself. "We cannot measure
lead time because releases are not tagged" is a finding you can act on this
week. A lead-time number derived from a guess is not.

## Slash commands

| Command | What it does |
|---|---|
| `/devex` | Scorecard for the current workspace: DORA + practice maturity |
| `/dora` | The four keys only, with the proxy each was derived from |
| `/practices` | Practice-by-practice maturity with the missing signals named |
| `/space` | The five SPACE dimensions, and what this repository cannot answer |
| `/onboarding` | Bootstrap readiness and new-contributor activity |
| `/devex-plan` | Turn a scorecard into a sequenced improvement plan with owners |

## Skill map

Load the pillar skill, not this one, once you know which pillar the task is in.

**Global Practices Program**
- `dora-metrics-program.md` — the four keys: what they mean, how to measure them honestly, how to avoid the three ways they get gamed
- `space-framework-productivity.md` — the human half: satisfaction, performance, activity, collaboration, efficiency; survey design that yields a decision
- `engineering-practices-program.md` — maturity model, adoption mechanics, the practice council, how a standard becomes real
- `engineering-productivity-dashboards.md` — what to put on a dashboard for engineers, for directors, and for the CFO, and why they are three different dashboards

**Strategic Developers' Platform Ownership**
- `developer-platform-ownership.md` — running CI/CD, observability and the IDP as products with users, SLOs and a roadmap
- `developer-onboarding-day-one.md` — the commit-on-day-one target: what it takes, how to measure it, what usually blocks it
- `build-deploy-acceleration.md` — reducing build and deploy time by a stated percentage, and proving the reduction

**Engineering Leadership**
- `engineering-investment-case.md` — turning platform work into a funding decision the finance partner can defend
- `principal-engineering-community.md` — influencing without authority: architecture councils, RFCs, communities of practice

**Adjacent skills already in the catalogue** — prefer these over rewriting them:
`platform-engineering.md`, `devex-developer-experience.md`,
`leadership-people-management.md`, `observability-metrics.md`,
`observability-tracing.md`.

## The cadence

A practices program that only exists at planning time is a document. The
cadence is what makes it an operating system.

| Rhythm | What happens | Artifact |
|---|---|---|
| Weekly | Platform team reviews its own SLOs and top support themes | Support-theme list |
| Fortnightly | Practice council reviews one standard and its adoption data | Standard revision or a recorded decision not to |
| Monthly | Scorecard refresh per domain; regressions get an owner | `vibecli --devex report` diffed against last month |
| Quarterly | SPACE survey; investment reallocation; roadmap re-cut | Survey + funding brief |
| Annually | Investment case for the following year | `engineering-investment-case.md` |

## The four traps in this role

1. **Reporting a number nobody measured.** The most common failure mode of an
   engineering-metrics program, and the fastest way to lose the room. If a
   metric cannot be computed here, it is `unmeasured` with a reason — never
   zero, never a plausible default. Every tool in this skill set is built that
   way and will refuse to fabricate; do not work around it.
2. **Measuring individuals.** DORA and SPACE are team and system metrics. The
   moment a four-keys dashboard is filtered per person it stops measuring
   delivery and starts measuring who commits at 5pm. Aggregate at the team
   boundary and refuse requests to go below it, in writing, once.
3. **Standards without a golden path.** A standard that makes the compliant
   route slower than the non-compliant one loses, every time, in every org.
   Ship the paved road first, then the policy.
4. **A platform with no product manager.** CI/CD and observability owned as
   infrastructure decay into ticket queues. Owned as products — users, SLOs,
   roadmap, adoption metric, deprecation policy — they compound.

## What good looks like at the end of year one

- Every domain has a scorecard that is refreshed without anyone being asked.
- The `unmeasured` list is shorter than it was, and each removal was a real
  instrumentation change rather than a loosened definition.
- A new engineer's first commit lands on day one, and the number is measured
  from the identity system's start date rather than asserted.
- At least one standard has been *withdrawn* because the adoption data said it
  was not worth its cost. A program that only ever adds is not a program.
