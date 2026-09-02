---
name: "Engineering Productivity Dashboards"
description: "Engineering Productivity Dashboards: designing the three distinct dashboards an engineering organisation needs — for teams, for engineering leadership, and for finance — without letting any of them become a surveillance tool. Covers audience-specific metric selection, coverage reporting, distribution over averages, refresh cadence, and the metrics never to display. Use when the task involves productivity dashboard, engineering metrics dashboard, delivery dashboard, executive engineering reporting, or engineering scorecards."
category: devex
triggers: ["productivity dashboard", "engineering metrics dashboard", "delivery dashboard", "engineering scorecard", "executive reporting", "engineering KPIs", "team dashboard"]
tools_allowed: ["read_file", "write_file", "bash"]
---

# Three dashboards, three audiences

The most common mistake is building one dashboard for everyone. Teams need
diagnosis, leadership needs distribution, finance needs cost and outcome. One
board serving all three serves none, and usually becomes the surveillance
artifact that ends the program's credibility.

## Dashboard 1 — the team's own

**Audience**: the team. **Purpose**: diagnose their own delivery system.
**Access**: the team, by default. Not aggregated upward without their knowing.

| Panel | Metric | Why |
|---|---|---|
| Flow | Lead time p50/p95, by stage | Shows where the time goes, not just how much |
| Stability | Change failure rate, time to restore | Paired with flow, never shown apart from it |
| Waiting | PR review latency, pipeline queue | The two waits engineers feel most |
| Health | Flaky-test rate, build success rate | Leading indicators of a bad month |
| Their own actions | Open items from the last retro | Makes the board a working surface |

Refresh: continuously. Retention: 13 months, so year-over-year is possible.

## Dashboard 2 — engineering leadership

**Audience**: directors and above. **Purpose**: allocate attention and money.
**Access**: leadership, and — non-negotiably — the teams shown on it.

| Panel | Metric | Why |
|---|---|---|
| Coverage | % of services instrumented, per metric | **First panel, always.** Everything else is unreadable without it |
| Distribution | Histogram of teams per DORA band | Not a mean. A mean of "high" hides the four teams at low |
| Movement | Teams that improved / regressed, counted | The actionable number |
| Practice maturity | Distribution per practice | From `vibecli --devex practices` |
| Platform SLOs | Queue time, provisioning time, portal uptime | The platform's own promises |
| Experience | Quarterly SPACE satisfaction, by domain | The half delivery metrics cannot see |

Refresh: monthly. A leadership dashboard refreshed hourly invites intervention
at a cadence faster than the underlying system can respond to, which is how
metrics programs turn into micromanagement.

**Show distributions, not averages.** This is the single most important design
rule at this level. Every mean here hides the thing you are supposed to act on.

## Dashboard 3 — finance and executive

**Audience**: CFO, CTO, the investment forum. **Purpose**: decide funding.
**Access**: broad.

| Panel | Metric | Why |
|---|---|---|
| Delivery trend | Lead time and deployment frequency, 12-month trend | The throughput story |
| Reliability trend | Change failure rate, time to restore | The risk story |
| Platform cost | Total cost of the platform, per engineer served | The denominator matters |
| Adoption | % of services on the golden path | Whether the investment reached anyone |
| Committed checks | Last year's cases and whether they met their check | Credibility |

Refresh: quarterly. Four numbers and a trend line beat forty and a legend.

## Never display

- Any metric filtered to a named individual.
- Lines of code, commits, or story points, in any dashboard, ever.
- A ranking of teams. Distribution, yes; league table, no — it produces gaming
  within one quarter and nothing else.
- A metric whose definition is not one click away.
- A number without its coverage and its sample size.

## Coverage, sample size, and absence

Three display rules that keep a dashboard honest:

1. **Every metric carries its coverage.** "Lead time 38h (61% of services)".
2. **Every value carries its sample size** where the sample can be small.
3. **An unmeasurable metric renders as "not measured", with the reason on
   hover — never as zero, never as a dash that looks like zero.** A tile that
   shows `0` for a metric nobody instrumented is a lie the dashboard tells on
   your behalf, and it will be quoted back to you.

The VibeCody `/devex` tooling enforces the third rule at the data layer: an
unmeasurable metric is absent from the payload and present in `unmeasured` with
a reason and a remedy. Render both. A client that drops the `unmeasured` block
has reintroduced the problem the API went out of its way to prevent.

## Where it lives in VibeCody

- **Cloud & Platform → Developer Excellence** — the scorecard, per workspace
- **CI/CD** panel — pipeline health and stage timing
- **Code Analysis** panel — repository-level quality signals
- **Billing** panel — platform cost
- `GET /devex/scorecard`, `GET /devex/scorecard.md` — the data behind them
