---
name: "SPACE Framework for Developer Productivity"
description: "SPACE Framework for Developer Productivity: measuring the human and social dimensions of engineering performance — satisfaction and wellbeing, performance, activity, communication and collaboration, efficiency and flow. Covers survey design, metric selection per dimension, sampling, and how to combine SPACE with DORA without double counting. Use when the task involves SPACE framework, developer satisfaction, developer productivity measurement, engineering survey, flow, cognitive load, or team health metrics."
category: devex
triggers: ["SPACE framework", "developer satisfaction", "developer productivity", "engineering survey", "developer experience survey", "flow state", "cognitive load", "team health", "developer wellbeing"]
tools_allowed: ["read_file", "write_file", "bash"]
---

# SPACE — the half of productivity that DORA cannot see

## Run the frame first

```
vibecli --devex space  --path <repo>            # the five dimensions, and the gaps
vibecli --devex space  --path <repo> --json
vibecli --devex survey                          # the quarterly instrument, on stdout
```

Panel: **Cloud & Platform → Developer Excellence → Experience (SPACE)**.
Route: `GET /devex/space`, `GET /devex/survey.md`.

**The gaps are the point.** On a first run the tool fills Performance (by
reference to DORA's stability pair), Activity, and part of Collaboration, and
names the system that holds the rest — the survey, the forge's PR API, CI, the
calendar. That list is your instrumentation roadmap, and it is more useful than
any number the tool could have invented to fill it.

Two things it will not do, by construction:

- **Produce an aggregate SPACE score.** Summing a survey response with a commit
  count yields something that cannot be wrong and therefore cannot be useful.
- **Report volume without an outcome.** When Performance has no measure the
  payload clears `outcome_signal` and every surface says so: Activity and
  Collaboration describe how much happened and in what shape, and neither says
  whether what shipped worked.



DORA measures the delivery system. SPACE measures the people running it and
what it costs them. A program with only DORA optimises a pipeline until the
engineers leave; a program with only SPACE has opinions and no delivery signal.
Run both.

## The five dimensions

| Dimension | What it asks | Example measures |
|---|---|---|
| **S**atisfaction & wellbeing | Would they recommend this as a place to build software? | eNPS for tooling, burnout indicator, retention intent |
| **P**erformance | Did the outcome happen? | change failure rate, reliability against SLO, customer-reported defects |
| **A**ctivity | What volume of work occurred? | PRs, deployments, documents, incidents handled |
| **C**ommunication & collaboration | How well does work move between people? | review latency, review depth, cross-team PR share, onboarding time |
| **E**fficiency & flow | Can they do the work without interruption? | uninterrupted focus hours, handoff count, wait time in the pipeline |

**The rule that makes SPACE work: pick at least three dimensions, and never
report Activity alone.** Activity by itself is the metric that turns into commit
counting, which is the metric that destroys the program's credibility. It is in
the framework because it is a useful *component* of a triangulated picture, and
for no other reason.

## Survey design that yields a decision

Most engineering surveys fail because they produce a number nobody can act on.

- **Ask about the last two weeks, not "in general".** Recall beyond a sprint is
  reconstruction, not memory.
- **Ask about specific frictions, not global feelings.** "How long did you wait
  for CI on your last change?" beats "Rate your developer experience 1–5".
- **Pair every Likert item with one free-text "what would you change".** The
  free text is where the roadmap comes from; the Likert is only for tracking.
- **Keep it under 5 minutes.** Response rate falls off a cliff past that, and a
  20% response rate is a self-selected sample of the most annoyed.
- **Publish what changed because of the last survey before running the next
  one.** Response rates are a function of whether the previous round visibly
  did anything.
- **Sample quarterly, not continuously.** Continuous pulse surveys train people
  to click through.

## Metric selection, per dimension

Choose one or two per dimension. More is not better; more is a dashboard nobody
opens.

- **Satisfaction** — a single tooling-satisfaction question, tracked over time,
  segmented by team. Segment, because a mean across 1,000 engineers hides the
  three teams whose environment is broken.
- **Performance** — reuse DORA's stability pair rather than inventing a new
  measure. Do not double count throughput here.
- **Activity** — deployments and reviews completed. Never lines of code, never
  commits per person, never story points.
- **Collaboration** — **review latency** (PR opened → first substantive review)
  is the highest-yield single number in this dimension. It is a direct measure
  of how long people wait on each other, it is cheap to compute, and reducing
  it improves flow and satisfaction at once.
- **Efficiency** — pipeline wait time and meeting-free block hours. Both are
  measurable without asking anyone.

## Combining with DORA without double counting

| Signal | Owned by | Do not also report as |
|---|---|---|
| Deployment frequency | DORA throughput | SPACE Activity |
| Lead time | DORA throughput | SPACE Efficiency |
| Change failure rate | DORA stability | SPACE Performance (reference it, don't restate it) |
| Review latency | SPACE Collaboration | — |
| Focus hours | SPACE Efficiency | — |
| Tooling satisfaction | SPACE Satisfaction | — |

The combined scorecard is then: four DORA keys + review latency + focus hours +
tooling satisfaction. Seven numbers. That is a scorecard a director can hold in
their head and defend in a funding conversation — which is the actual test.

## The ethics line, and where to draw it

Write this down and circulate it before the first survey ships:

1. SPACE is reported at team level and above. There is no individual view, and
   the tooling will not build one.
2. Survey responses are anonymous, and teams smaller than five are aggregated
   upward rather than reported.
3. No SPACE measure is an input to performance review, compensation, or
   staffing decisions about named individuals.
4. Raw response data has a stated retention period and is deleted on schedule.

You will be asked to break at least one of these, usually in the second year,
usually by someone with good intentions. The written commitment is what makes
declining a policy rather than an argument.

## Where it lives in VibeCody

- Survey instrument and results → **Project Hub** panel (documents + memory)
- Review latency and pipeline wait → **Version Control** / **CI/CD** panels
- Combined scorecard → **Cloud & Platform → Developer Excellence** panel
- Quarterly narrative → `/devex-plan`
