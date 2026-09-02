---
name: "Developer Onboarding — Commit on Day One"
description: "Developer Onboarding — Commit on Day One: designing and measuring an onboarding path where a new engineer checks in code within their first day. Covers the bootstrap contract, reproducible environments, the access critical path, the starter-task pattern, and honest measurement of time-to-first-commit. Use when the task involves developer onboarding, time to first commit, environment setup, bootstrap script, devcontainer, day one productivity, or new hire ramp."
category: devex
triggers: ["developer onboarding", "time to first commit", "day one", "environment setup", "bootstrap script", "devcontainer", "new hire ramp", "onboarding time", "first pull request"]
tools_allowed: ["read_file", "write_file", "bash"]
---

# Commit on day one

The target: a new engineer's first change is merged on their first day. It is
achievable, and it is a genuinely good forcing function — almost everything
that blocks it also slows every existing engineer, every day.

## Measure the ground first

```
vibecli --devex onboarding --path <repo>
```

Reports bootstrap readiness (which signals are present, and which are missing
by name) and new-contributor activity from git history.

**It deliberately does not report time-to-first-commit.** Git records a
contributor's first commit; it does not record the day they joined. The
interval the target is about has no start in git, and any number printed there
would have invented one. Joining the identity system's start date to the
first-commit date gives the real metric — the scan supplies the second half and
says so. Do not paper over the gap with a plausible default; a fabricated
onboarding metric is the kind that gets quoted in a board deck.

## The critical path, in the order it actually blocks people

Most onboarding programs optimise the repo setup and leave the real blockers
untouched. In order of how often they are the binding constraint:

1. **Accounts and access.** SSO, VPN, repo permissions, cloud roles, the secret
   store, the ticket system. This is the number one cause of a lost first day
   in almost every organisation, and it is an IT/identity problem rather than
   an engineering one — which is precisely why nobody owns fixing it. Own it.
   Target: everything provisioned and verified **before** day one, triggered by
   the offer-accepted event, not the start date.
2. **Hardware and the base toolchain.** Machine shipped, imaged, and tested.
3. **Repository bootstrap.** One command, clone to running.
4. **The first task.** Identified, small, and reviewed the same day.
5. **A named person to ask.** Not a rota — one name.

## The bootstrap contract

Write this down and hold the platform team to it:

> `git clone` → one documented command → the application runs locally and its
> test suite passes, on a clean machine, in under 30 minutes, with no
> undocumented prerequisites.

Mechanics:
- **One entry point**: `make setup`, `./scripts/bootstrap.sh`, `just setup`.
  Not a README with fourteen steps.
- **Reproducible environment**: devcontainer, Nix flake, or a pinned toolchain
  manifest (`.tool-versions`, `rust-toolchain.toml`, `.nvmrc`). Version drift
  between machines is the second-largest source of lost first days.
- **A verification step that fails loudly.** The last thing bootstrap does is
  run a smoke test and print either a success line or the exact next action.
- **Tested in CI on a clean image.** A bootstrap script that is only ever run
  on machines that already work does not stay working. This check is cheap and
  it is the only thing that keeps the path alive.
- **Every prerequisite stated with a version.** OS, runtime, accounts needed.

## The starter task

Prepare these in advance; do not improvise on the morning.

- Small, real, and merged the same day. Not a sandbox exercise — the point is
  to exercise the whole path from clone to production.
- Touches the full loop: code, test, review, CI, merge, deploy.
- Labelled in the tracker (`good-first-task`) and kept stocked. Running out is
  the usual reason a new joiner's first week drifts.
- Has an owner who will review it within two hours.

## Measuring it honestly

| Metric | Source | Note |
|---|---|---|
| Time to first commit | identity start date + git first commit | Both halves, joined; the scan gives one |
| Time to first merged PR | identity start date + PR merge | The better target — merging exercises review and CI too |
| Bootstrap success rate | CI run of the bootstrap on a clean image | Leading indicator; catches rot before a human does |
| Bootstrap duration | same CI run | Watch the trend, not the value |
| Self-reported blockers | week-one survey, three questions | Where the real story is |

Report the distribution, not the mean. A mean of "1.4 days" that contains three
people who took a week is a mean that hides the entire problem.

## Where it lives in VibeCody

- Readiness scan → `/onboarding`, `GET /devex/onboarding`, **Developer
  Excellence** panel, Onboarding tab
- Bootstrap CI check → **CI/CD** panel
- Week-one survey and results → **Project Hub**
- Starter-task queue → **Project Hub** / the tracker integration
