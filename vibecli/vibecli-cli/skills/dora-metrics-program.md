---
name: "DORA Metrics Program"
description: "DORA Metrics Program: standing up the four keys — deployment frequency, lead time for changes, change failure rate, time to restore — as an honest, defensible measurement across many teams. Covers proxy selection, instrumentation, banding, the ways the four keys get gamed, and what they cannot see. Use when the task involves DORA, four keys, deployment frequency, lead time for changes, change failure rate, MTTR, delivery performance, or an engineering metrics baseline."
category: devex
triggers: ["DORA", "four keys", "deployment frequency", "lead time for changes", "change failure rate", "time to restore", "MTTR", "delivery performance", "engineering metrics", "accelerate metrics"]
tools_allowed: ["read_file", "write_file", "bash"]
---

# DORA metrics — a program, not a dashboard

## Run it first, argue about it second

```
vibecli --devex dora --path <repo>                      # 90-day window, tags as deployments
vibecli --devex dora --path <repo> --marker merges --branch main
vibecli --devex dora --path <repo> --window 180 --json
```

Panel: **Cloud & Platform → Developer Excellence**. Route: `GET /devex/dora`.

## What the four keys actually are

| Key | Definition | Interval it measures |
|---|---|---|
| Deployment frequency | How often the org successfully releases to production | Between successful production deployments |
| Lead time for changes | How long a commit takes to reach production | First commit → running in production |
| Change failure rate | Fraction of deployments causing degraded service needing remediation | Per deployment |
| Time to restore service | How long to recover from a production failure | Failure detected → service restored |

Two are throughput. Two are stability. **They are reported as a set of four.**
Any three of them can be improved by damaging the fourth, so a program that
publishes throughput alone will get exactly what it asked for: faster breakage.

## Choosing a proxy, and saying so out loud

Nothing in a git repository knows what "production" means. Every DORA number in
existence is derived from a proxy, and the difference between a credible
program and a discredited one is whether the proxy is stated.

| Signal you have | Proxy | Blind spot to state |
|---|---|---|
| Release tags from the pipeline | tag = one deployment | deploys that ship untagged; multi-service tags |
| Merges to a release branch | merge = one deployment | merges that fail to deploy |
| Deployment events from the CD system | the real thing | none — use this if you have it |
| Nothing | **do not publish a number** | — |

The last row is the one that matters. A team whose deploys are not recorded
gets `deployment_frequency: unmeasured` with the reason spelled out and a
concrete remedy, not `0.0/week`. Zero is a claim about their delivery; absence
is a claim about your instrumentation, and only one of those is true.

## Banding, and its limits

The published DORA bands (elite / high / medium / low) are useful for one
purpose: telling a team roughly where they sit against an industry-wide
distribution. They are not a target and not a ranking.

- **Do not set a band as an OKR.** "Reach elite deployment frequency" is
  satisfied by deploying an empty change hourly.
- **Do compare a team against its own previous quarter.** That comparison holds
  the domain, the risk profile and the regulatory load constant.
- A payments-clearing team at "medium" may be operating better than a marketing
  site at "elite". Band without context is noise.

## The three ways this gets gamed

1. **Batch splitting.** Deployment frequency rises because one release became
   five commits deployed separately. Watch lead time and change failure rate
   together; genuine improvement moves all three the right way.
2. **Failure redefinition.** Change failure rate falls because "degraded" got
   redefined. Freeze the definition in writing, version it, and note the version
   on every chart. If it changes, the series breaks — say so rather than
   splicing.
3. **Restore-clock manipulation.** Time to restore falls because the clock now
   starts at incident declaration instead of at customer impact. Same remedy:
   define the interval endpoints once, in writing, and never move them quietly.

## What DORA cannot see

State these in the same breath as the numbers, every time:

- **Whether the work was worth doing.** DORA measures the pipe, not the water.
- **Developer experience.** A team can be elite on all four keys and miserable.
  That is what SPACE is for — see `space-framework-productivity.md`.
- **Quality of the change.** Deploying bad code fast scores well.
- **Toil, cognitive load, and on-call cost.** Invisible to all four keys.
- **Anything about an individual.** The unit is a team's delivery system.

## Rolling it out across many teams

1. **Baseline everything before you announce anything.** Run the measurement on
   every repo, publish nothing, and read the `unmeasured` distribution. That
   list is your instrumentation roadmap.
2. **Fix instrumentation before fixing performance.** A team cannot improve a
   number they cannot see, and a program that grades teams on a metric they
   cannot compute will be routed around within a quarter.
3. **Give teams their own numbers first, privately, for one full quarter.**
   Trust is the scarce resource. Publish comparatively only after teams have
   had a chance to correct their own instrumentation.
4. **Aggregate at the team boundary and stop.** Write down that you will never
   report per-individual four keys, and hold it when asked.
5. **Report the coverage, not just the value.** "Deployment frequency 3.1/week
   across 40% of services" is honest. The same sentence without the 40% is not.

## Gating on it

```
vibecli --devex gate --path <repo> \
  --require-lead-time high \
  --require-change-failure-rate high
```

Exit codes: `0` met, `1` a required band was missed, `2` usage error, `3` a
required metric was **unmeasurable**. Code 3 is deliberately distinct from 1: a
pipeline that treats "we could not measure it" as "it is bad" blocks releases
for a tooling gap, and one that treats it as "it is fine" ships on an absence.
Choose which your pipeline does — `--unmeasured-is-failure` folds 3 into 1 —
but choose it consciously.
