---
name: "Build and Deploy Time Reduction"
description: "Build and Deploy Time Reduction: cutting pipeline latency by a stated percentage and proving the reduction. Covers profiling the pipeline, the win order (queue time, caching, parallelism, incrementality, test selection), what not to do, and how to report a like-for-like improvement that survives scrutiny. Use when the task involves build time, CI time, pipeline speed, deploy time, build caching, test selection, monorepo build performance, or a percentage reduction target."
category: devex
triggers: ["build time", "CI time", "pipeline speed", "deploy time", "build cache", "test selection", "faster builds", "reduce build time", "pipeline optimization", "monorepo build"]
tools_allowed: ["read_file", "write_file", "bash"]
---

# Reducing build and deploy time

The target is usually written as "reduce build and deploy times by X%". Two
things make that target succeed or fail, and neither is technical: **what you
measured before** and **whether the after-measurement is like-for-like**.

## Measure before you touch anything

Record, for a representative sample of at least 50 runs:

- **Queue time** (job created → job started), p50 and p95
- **Execution time** per stage, p50 and p95
- **End-to-end** (push → deployable artifact), p50 and p95
- **Cache hit rate** per cache
- **Failure rate and retry rate** — a pipeline that is fast because it fails
  fast is not faster

Record the shape of the sample too: branch mix, time of day, runner sizes,
whether a release was in flight. That description is what makes the after
comparison honest, and it is the part everyone skips.

**Report p50 and p95 together, always.** A change that improves the mean while
worsening the tail makes the pipeline feel slower, because people remember the
bad runs.

## The win order

Work down this list. It is ordered by return per unit of effort, and the top
two are where most of the available time actually is.

1. **Queue time.** Measure it separately before anything else. In a constrained
   runner pool it is frequently the largest single component of end-to-end, and
   it is almost always the cheapest to fix (more runners, better sizing,
   priority lanes for the trunk). It is also the component teams most often do
   not measure at all — so a quarter gets spent optimising the build while the
   wait in front of it sits untouched. Do not assume a share; read it off your
   own runs.
2. **Caching.** Dependency cache, build cache, container layer cache, test
   fixtures. Measure the hit rate per cache. A hit rate well below what the
   change pattern would predict almost always means the key includes something
   that varies when it should not — a timestamp, an absolute path, a machine
   identifier, a lockfile hash that includes dev-only churn. Diff two
   consecutive keys to find it rather than guessing.
3. **Parallelism.** Split by stage first, then shard the longest stage. Sharding
   only helps once the critical path is genuinely the thing you sharded — check
   the dependency graph rather than assuming.
4. **Incrementality.** Build only what changed. Turborepo, Nx, Bazel, Gradle
   build cache, `cargo` with a warm target dir. High ceiling, high setup cost —
   which is why it belongs below caching, not above it.
5. **Test selection.** Run the tests affected by the change on PRs, everything
   on trunk. Powerful and genuinely risky: an impact analysis that misses a
   dependency ships a regression. Introduce it in shadow mode first — select,
   but still run everything, and compare — until the selection has been right
   for a full quarter.
6. **The build itself.** Compiler flags, linker choice, image size, fewer
   layers. Real wins, usually smaller than the five above.

## What not to do

- **Do not remove tests to hit the number.** It works, it is measurable, and it
  converts a delivery-speed metric into a change-failure-rate problem next
  quarter. If coverage is deliberately reduced, say so in the same report.
- **Do not move work out of the measured window.** Shifting a 6-minute step
  into a nightly job improves the chart and not the engineer's day, unless the
  step genuinely did not need to be in the loop — in which case say that
  explicitly and show where it went.
- **Do not report a best run.** Report the same percentile on a comparable
  sample.
- **Do not compare across a runner-fleet change** without saying so. Doubling
  runner size is a legitimate improvement and a completely different claim from
  making the build more efficient.

## Reporting the reduction

A defensible claim states all five:

> p95 end-to-end fell from 24m10s to 13m40s (−43%) over 200 runs on `main`,
> same runner class, same test scope, measured 1–14 May against 1–14 April.
> Cache hit rate rose 61% → 92%. No tests were removed; test scope is
> unchanged. Queue time accounted for 6m of the 10.5m saved.

Attribution — which change bought which minutes — is what makes the next
investment decision possible. A total with no attribution is a number, not a
finding.

## Deploy time specifically

Usually dominated by things that are not the deploy:

- **Approval wait**, not deploy execution. Measure them separately or the whole
  analysis is wrong from the start.
- **Health-check and soak windows.** Legitimate; tune deliberately rather than
  by default. Halving a soak window is a risk decision, not a performance one.
- **Sequential environment promotion.** Parallelise where the environments are
  genuinely independent.
- **Artifact transfer.** Registry locality and image size.

## Where it lives in VibeCody

- Pipeline runs and stage timing → **CI/CD** panel
- Lead-time effect of the change → `vibecli --devex dora` before and after
- Gate the regression → `vibecli --devex gate --require-lead-time high`
