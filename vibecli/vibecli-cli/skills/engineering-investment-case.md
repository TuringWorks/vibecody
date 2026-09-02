---
name: "Engineering Investment Case"
description: "Engineering Investment Case: turning developer-productivity and platform work into a funding decision a finance partner will defend. Covers sizing the opportunity from measured data, the three credible benefit types, cost modelling, what not to claim, and the annual planning cycle. Use when the task involves engineering investment, platform funding, productivity ROI, business case, cost of delay, capitalization, or influencing yearly financial investment decisions for engineering."
category: devex
triggers: ["engineering investment", "platform funding", "productivity ROI", "business case", "cost of delay", "engineering budget", "investment decision", "developer productivity ROI", "capitalization"]
tools_allowed: ["read_file", "write_file", "bash"]
---

# The engineering investment case

Platform and practices work competes for money against features with revenue
attached. It loses that competition when it is argued on principle and wins it
when it is argued on measured cost.

## The three credible benefit types

Use these. They survive a finance review.

1. **Capacity released** — engineer-hours currently spent on toil that the
   investment removes. Credible **only** when the time is measured, not
   estimated. Measure it: instrument the pipeline wait, count the tickets,
   time the manual step, survey with a specific recall question.
2. **Cycle-time reduction** — lead time falling means the same headcount ships
   more increments per year. Convert to value only where the increments have a
   known value; otherwise present it as a throughput claim and stop there.
3. **Risk and incident cost avoided** — change failure rate and time-to-restore
   improvements, priced against the organisation's own incident cost history.
   Use your incident record, not an industry average.

## The one to avoid

**"Developer productivity increased 20%, so we saved 0.2 × headcount × salary."**
It is the most common framing and the least survivable. It assumes released
time converts to output at full efficiency, it cannot be verified after the
fact, and a CFO who has seen it before will discount the entire paper. If a
number cannot be checked a year later, do not put it in the case.

## Sizing from measured data

```
vibecli --devex scorecard --path <repo> --json    # per repo
vibecli --devex report --path <repo>              # the narrative form
```

Roll up across repositories and build the case from the aggregate:

| Input | Where it comes from | Honesty note |
|---|---|---|
| Engineers affected | HR/org data | Segment; a platform change rarely touches everyone |
| Current pipeline wait | CI system | p50 and p95, with the sample described |
| Waits per engineer per day | CI system | Count, do not assume |
| Onboarding days lost | onboarding scan + start dates | Needs both halves — see the onboarding skill |
| Incident cost | incident record | Your own, not a benchmark |
| Coverage | scan | **State it.** A number from 40% of services is a number from 40% of services |

## Cost side, in full

Under-costing is why the second year of funding gets refused.

- Build: engineers × months, fully loaded
- Run: the platform team's steady-state cost, ongoing
- Infrastructure: compute, licences, vendor
- **Migration cost borne by product teams** — the line most often omitted, and
  frequently the largest. Estimate it explicitly with two or three of the teams
  who will pay it.
- Opportunity cost: what the same engineers would otherwise have built

## The paper

One page, in this order. Anything longer gets skimmed to the number and the
number gets challenged without its context.

1. **The problem, in the organisation's own measured numbers.** Two or three.
2. **What is being proposed**, concretely, with scope boundaries.
3. **Benefit**, by type, with the measurement that will confirm it.
4. **Cost**, fully loaded, including migration.
5. **The check** — the specific metric, threshold and date at which this will
   be judged, agreed in advance.
6. **What happens if it is not funded.** Not a threat; the honest counterfactual.
7. **Assumptions and coverage**, stated plainly. This section is what makes
   the rest believable.

## The annual cycle

| When | What | Artifact |
|---|---|---|
| Q1 | Baseline refresh; last year's checks evaluated | Scorecard diff |
| Q2 | Opportunity sizing with domain leaders | Sized backlog |
| Q3 | Cases written and socialised with finance | One-pagers |
| Q4 | Portfolio decision; commitments recorded | Funded roadmap + checks |

**Evaluate last year's cases before writing this year's.** A director who
reports honestly that one of three initiatives missed its check is trusted with
the next round. One who only ever reports wins is not, eventually.

## Where it lives in VibeCody

- Measured inputs → **Cloud & Platform → Developer Excellence** panel
- Cost data → **Billing** panel
- The paper itself → **Project Hub** documents
- Commitments and their check dates → **Goals** panel
