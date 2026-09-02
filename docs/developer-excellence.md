---
layout: page
title: "Developer Excellence"
permalink: /developer-excellence/
---

# Developer Excellence — measured, not asserted

Engineering-metrics tooling has one recurring failure: it reports numbers
nobody measured. A dashboard that shows `0` for a metric no team instrumented,
or a scorecard that awards a service 8/10 for deploy frequency from a constant,
does more damage than no dashboard at all — because it will be quoted in a
funding conversation and it cannot be defended.

VibeCody's Developer Excellence tooling is built the other way round. Every
number states the proxy it came from and the size of the sample it came from,
and **a metric that cannot be computed is returned as `unmeasured`, with the
reason and the concrete change that would make it measurable** — never as zero.

- **Panel:** Cloud & Platform → Developer Excellence (VibeCoder)
- **Routes:** `/devex/*` on the VibeCLI daemon (bearer auth required)
- **CLI:** `vibecli --devex …`
- **Skills:** the `devex` category, starting with
  `devex-director-operating-system.md`
- **Source:** `vibecli/vibecli-cli/src/devex_metrics.rs`,
  `vibecli/vibecli-cli/src/devex_routes.rs`,
  `vibecli/vibecli-cli/src/devex_cmd.rs`

---

## What it measures

### DORA — the four keys

Computed from git history. Nothing in a repository knows what "production"
means, so every key is derived from a **declared proxy**, and the proxy travels
with the number.

| Key | Proxy | Blind spot |
|---|---|---|
| Deployment frequency | version-like tags, or merges on a release branch | deploys that ship without a tag |
| Lead time for changes | commit author-time → time of the release that first contained it | time queued before the first commit |
| Change failure rate | a deployment followed by a revert / hotfix / rollback commit before the next deployment | failures fixed by a config change or flag flip |
| Time to restore | that deployment → its remediation commit | incidents with no code remediation |

Values are banded elite / high / medium / low using the thresholds published in
the DORA reports. Those thresholds are the only externally-sourced numbers in
the subsystem, and the source travels in the payload as `band_source`.

**`fix:` commits are not counted as remediation.** Most of them fix a bug found
in development; counting them would push change failure rate toward 100% for
every healthy team. Only language that means *something shipped and had to be
taken back* — `Revert`, `hotfix`, `rollback` — counts.

### Practice maturity

Ten practices, each evidenced by up to three signals looked for at the
workspace root. Reported as a **detected** level, `0` absent → `3` defined.

**It never reports level 4.** A file proves a practice is present; it cannot
prove the practice is followed, reviewed, or improving — which is exactly what
separates the top level of any maturity model worth using from the one below
it. Level 4 is attested by people. A maturity score that a `touch` could earn
is decorative.

Practices that are known to under-detect say so in a `detection_caveat` that is
rendered next to the misses, not in a footnote: `automated-testing` cannot see
Rust `#[cfg(test)]` or Go `_test.go` tests, and `golden-path` reads "missing"
for a monorepo whose lint conventions live one level down.

### SPACE — the experience half

DORA measures the delivery system; SPACE measures the people running it and what
it costs them. **Most of SPACE is not in a git repository, and the tool says so
rather than approximating it.**

| Dimension | Measured here | From what |
|---|---|---|
| Satisfaction & wellbeing | — | needs a survey; `vibecli --devex survey` prints the instrument |
| Performance | ✅ | *references* DORA's stability pair rather than restating it |
| Activity | ✅ | commits, distinct authors, deployments — counts, never names |
| Communication & collaboration | ✅ partly | files touched by more than one author; `Co-authored-by` share. **Review latency is not here** — it lives in the forge's PR API, and a merge commit records when a branch landed, not when anyone first looked at it |
| Efficiency & flow | — | pipeline queue/run time from CI; focus hours from the calendar |

Two rules are enforced in the shape of the data, not left to the reader:

- **There is no aggregate SPACE score.** Summing a survey response with a commit
  count produces a number that cannot be wrong and therefore cannot be useful.
  The payload has no field for one.
- **Volume is never reported without an outcome signal.** When Performance has
  no measure, `outcome_signal` is false and every renderer must say so: Activity
  and Collaboration describe how much happened and in what shape, and read
  without an outcome they are not a picture of productivity.

  This began as an `activity_only` flag and a test proved it could never fire —
  any repository with one commit gets a `Co-authored-by` percentage, so
  Collaboration always had a measure. A flag that cannot fire is a reassurance
  nobody earned, so the predicate became one that is both reachable and worth
  warning about.

Relatedly, a window with a **single author** does not get a multi-author file
share: with one author it is 0 by arithmetic rather than by how the team works,
so it is reported as a named gap instead of a measurement of the formula.

Nothing in the SPACE report is per-individual. Author *counts* are activity;
author *names* would be surveillance, and the shape has no field for them.

### Onboarding

Bootstrap readiness (one-command setup, reproducible environment, a
getting-started guide) plus first-time contributors from git history.

**Time-to-first-commit is deliberately absent.** Git records a contributor's
first commit but not the day they joined, so the interval the day-one target is
about has no start. The report says so and names the system that holds the
missing half, rather than printing a number built on an invented start date.

---

## CLI

```bash
vibecli --devex dora        --path <repo> [--window 90] [--marker tags|merges] [--branch main]
vibecli --devex practices   --path <repo>
vibecli --devex onboarding  --path <repo> [--window 90]
vibecli --devex space       --path <repo> [--window 90] [--markdown]
vibecli --devex survey                             # the quarterly survey instrument
vibecli --devex scorecard   --path <repo>          # delivery + practices in one view
vibecli --devex report      --path <repo>          # the scorecard as markdown, on stdout
vibecli --devex gate        --path <repo> --require-lead-time high
```

`--json` on any command emits the full payload. `--path` defaults to the
working directory.

### Gating in CI

```bash
vibecli --devex gate --path . \
  --require-deploy-frequency high \
  --require-lead-time high \
  --require-change-failure-rate high
```

| Exit code | Meaning |
|---|---|
| `0` | Every required band was met |
| `1` | A required band was missed, or the command failed |
| `2` | Usage error — unknown subcommand, missing argument, bad value |
| `3` | A required metric could not be **measured** here |

Code `3` is deliberately distinct from `1`. "We could not measure lead time" is
not "lead time is bad": a pipeline that conflates them either blocks releases
for a tooling gap or ships on an absence. Pass `--unmeasured-is-failure` to
fold `3` into `1` — but choose it consciously.

A gate with no `--require-*` threshold is refused. The cheapest route to a
green gate must not be to remove its criteria.

### In CI

```yaml
# .github/workflows/devex.yml
name: developer-excellence
on:
  schedule: [{ cron: "0 6 * * 1" }]   # Monday morning, not on every push
  workflow_dispatch:

jobs:
  measure:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0        # DORA needs history; a shallow clone measures nothing
          fetch-tags: true      # and tags are the default deployment proxy

      - name: Scorecard
        run: vibecli --devex report --path . >> "$GITHUB_STEP_SUMMARY"

      - name: Gate
        run: |
          vibecli --devex gate --path . \
            --require-lead-time high \
            --require-change-failure-rate high
```

`fetch-depth: 0` is not optional. A shallow clone has no history and no tags, so
every key comes back `unmeasured` and the gate exits `3` — correctly, but for a
reason that is about the checkout rather than the team.

Run it on a schedule rather than on every push. These are trailing indicators
over a 90-day window; measuring them per commit produces noise and trains people
to ignore the job.

Locally: `make devex`, `make devex-report`, `make devex-gate`.

---

## HTTP

All `/devex/*` endpoints require authentication
(`Authorization: Bearer $VIBECLI_TOKEN`) and are read-only.

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/devex/dora` | The four keys, plus `unmeasured` for the ones that could not be computed |
| `GET` | `/devex/practices` | Practice maturity, per signal, with detection caveats |
| `GET` | `/devex/onboarding` | Bootstrap readiness and first-time contributors |
| `GET` | `/devex/scorecard` | Delivery and practices in one payload, with `dora_coverage` |
| `GET` | `/devex/scorecard.md` | The same, rendered as a markdown briefing (`text/markdown`) |
| `GET` | `/devex/space` | The SPACE frame: measures per dimension, and the system holding each one this repository cannot answer |
| `GET` | `/devex/survey.md` | The quarterly experience-survey instrument (`text/markdown`) |

**Query parameters** (all endpoints): `path` (**required**), `window` (days,
1–1825, default 90), `marker` (`tags` \| `merges`), `branch` (with
`marker=merges`).

`path` is required and never inferred. A daemon that fell back to its own
working directory would measure an unrelated tree and label the answer with the
caller's repository. An unknown `marker` and an out-of-range `window` are `400`
with the expected values named, rather than being silently defaulted or
clamped — a clamped value rendered back as if it were the requested one is a
number the caller never asked for.

```bash
curl -s -H "Authorization: Bearer $VIBECLI_TOKEN" \
  "http://localhost:7878/devex/scorecard?path=/src/myrepo&window=90" | jq .
```

---

## Slash commands

In VibeCoder chat. Each names its skill file with the extension — `get_skill`
accepts either spelling — so the source of the guidance is one click away.

| Command | What it does |
|---|---|
| `/devex` | Scorecard for the workspace, with the `unmeasured` block reported in full |
| `/dora` | The four keys, each with its band, sample size and proxy |
| `/practices` | Practice maturity with the missing signals named |
| `/onboarding` | Bootstrap readiness and first-time contributors |
| `/space` | The five SPACE dimensions, and what this repository cannot answer |
| `/devex-plan` | A sequenced improvement plan, instrumentation gaps before performance work |

---

## Skills

The `devex` category. Load the router first, then the pillar skill.

| Skill | Pillar |
|---|---|
| `devex-director-operating-system.md` | routing — the three pillars, the cadence, the traps |
| `dora-metrics-program.md` | Global Practices Program |
| `space-framework-productivity.md` | Global Practices Program |
| `engineering-practices-program.md` | Global Practices Program |
| `engineering-productivity-dashboards.md` | Global Practices Program |
| `developer-platform-ownership.md` | Strategic Developers' Platform Ownership |
| `developer-onboarding-day-one.md` | Strategic Developers' Platform Ownership |
| `build-deploy-acceleration.md` | Strategic Developers' Platform Ownership |
| `engineering-investment-case.md` | Engineering Leadership |
| `principal-engineering-community.md` | Engineering Leadership |

---

## What this tooling will not do

- **Report a metric per individual.** DORA and SPACE are team and system
  measures. Aggregation stops at the team boundary, and the SPACE payload has no
  per-author field to add one to.
- **Produce an aggregate SPACE score.** There deliberately is not one.
- **Substitute a default for a missing value.** Absent stays absent.
- **Claim a maturity level it cannot observe.**
- **Guess which directory you meant.**

The IDP service scorecard (Cloud & Platform → IDP) grades a service's **catalog
metadata** and says so. It used to also award four fabricated DORA scores; they
have been removed rather than replaced, because a `repo_url` is not a checkout
and DORA needs history. That scorecard now carries its own `unmeasured` block
pointing here.
