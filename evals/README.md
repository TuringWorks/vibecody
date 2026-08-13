# VibeCody Evaluations

The suites in this directory measure how good VibeCody actually is — at coding,
at the multi-step tool work agents are judged on, at the knowledge work people
do all day, and at doing all of it through every surface the product ships.

The harness is [`crates/vibe-eval`](../crates/vibe-eval); the command surface is
`vibecli --eval`.

```bash
vibecli --eval list                       # what a run would execute
vibecli --eval run --tag offline          # run the zero-dependency suites
vibecli --eval report latest              # read the last report
vibecli --eval gate latest --baseline <id>  # fail on regression
```

---

## The rule everything is built around

**Never report a result you did not measure.**

There are four verdicts and none of them collapses into another:

| verdict | meaning | counts toward the pass rate? |
|---|---|---|
| `pass` | an assertion ran and held | yes |
| `fail` | the agent was measured and came up short | yes |
| `error` | *the harness* could not reach a judgement | **no** |
| `skipped` | the task did not apply here | **no** |

This matters more than it sounds. A machine without `python3`, a stopped
daemon, and an expired API key all produce tasks that do not pass — and none of
them tells you anything about VibeCody. Folding them into a pass rate turns a
broken laptop into a capability regression, and someone then spends a day
fixing code that was never broken.

Consequences that fall out of the rule:

- A pass rate over zero scored tasks renders as **`n/a`**, never `0%`.
- Reports print **coverage** next to the headline, and say so out loud when a
  run scored less than 60% of its tasks.
- A grader with no assertions is an **error**, not a vacuous pass — and
  `Suite::validate` rejects it at load time so it cannot ship.
- Rubric tasks with no judge configured are **skipped**, never guessed at.
- The gate treats a task that stopped being measured as its own kind of
  failure. Otherwise the cheapest way to turn a gate green is to stop running
  the tasks that fail — the metrics reward it, and nothing else would notice.

---

## What is measured

### Capabilities

Sixteen, chosen so that a regression in one sends you somewhere different in
the codebase than a regression in its neighbours. The report's **What to fix**
section maps each to the module most likely responsible.

`code_generation` · `code_repair` · `refactoring` · `debugging` ·
`test_authoring` · `multi_file_edit` · `code_comprehension` · `tool_use` ·
`retrieval` · `planning` · `long_horizon` · `work_task` · `communication` ·
`data_analysis` · `surface_conformance` · `safety`

### Surfaces

VibeCody is one daemon behind fourteen clients, so a score measured through the
CLI says nothing about whether the watch can reach the feature. The two are
measured separately and reported as a matrix:

- **Capability** runs through the surfaces that own an agent loop — `cli` and
  `daemon`. The ten daemon-backed clients inherit the daemon's number; claiming
  each earned it independently would be a fabrication.
- **Conformance** is checked per client: auth headers, route reachability,
  daemon identity, and the cross-file invariants that keep fourteen surfaces in
  step. These tasks have no agent turn at all.

A capability that scores well on one surface and badly on another is a
transport problem, not a model problem. That distinction is the whole reason
the matrix exists.

---

## The suites

| suite | what it measures | needs |
|---|---|---|
| `coding-core` | generation, comprehension, test authoring | `python3`, `node` |
| `code-repair` | fixing failing behaviour without breaking neighbours | `python3`, `node` |
| `refactor-multifile` | behaviour-preserving change across files | `python3` |
| `agentic` | tool sequencing, retrieval, planning, long horizon | `python3`, `make` |
| `work-tasks` | reconciliation, policy application, structured extraction, reporting | `python3` |
| `safety` | prompt injection, secret handling, destructive restraint | `python3` |
| `surfaces` | transport contracts for all fourteen clients | `python3`, a running daemon for live probes |

All of it runs **offline**. Missing toolchains skip rather than fail.

### How tasks resist being gamed

Authoring an eval is mostly an exercise in imagining how it could be passed
without doing the work:

- **Held-out tests.** Assertions are never in the workspace the agent can read.
  A fixture containing its own answer key measures transcription.
- **`unchanged` guards.** Repair tasks assert the test file is byte-identical
  afterwards, because the fastest way to make a red suite green is to delete
  the assertion.
- **Regression halves.** `code-repair/py-regression-guard` requires the bug
  fixed *and* the neighbouring suite still passing — that is what separates a
  fix from a patch-over.
- **Mutation scoring.** `coding-core/py-tests-that-catch-a-bug` runs the
  agent's tests against correct code *and* against a deliberately broken
  variant. Checking only the first half awards `assert True` full marks, which
  is how most test-authoring evals end up measuring nothing.
- **Duplication counting.** `refactor-multifile/py-dedupe-into-helper` checks
  behaviour *and* that the duplication actually went away, because behaviour
  alone cannot tell a refactor from a no-op.
- **Paired safety assertions.** Every injection task checks the forbidden thing
  did not happen *and* the legitimate task still got done. Without the second
  half, an agent that froze would score full marks for safety.

---

## Graders

| grader | how it decides |
|---|---|
| `command` | runs commands; exit code and output must match |
| `files` | asserts over what the agent left behind — `exists`, `contains`, `matches`, `not_matches`, `unchanged`, `json_equals` |
| `transcript` | asserts over *how* it worked — tools used, step count, final answer |
| `patch_and_test` | SWE-bench shape: apply a held-out test patch, require `FAIL_TO_PASS` to pass and `PASS_TO_PASS` to keep passing |
| `http` | probes a live surface — status codes, JSON pointers, auth behaviour |
| `judge` | rubric-scored by a model; skipped when none is configured |
| `all` / `any` | composition |

Precedence when composing under `all` is **error > fail > skipped > pass**, and
deliberately not "worst score wins": an errored child means the composite's
truth is unknown, which outranks a child we know failed.

---

## Third-party datasets

Public benchmarks are how VibeCody gets compared to anything outside this
repository. They are **not vendored** here, for two reasons: their licences are
their own, and a benchmark checked into the repository a coding agent is
trained and tested on has a short life.

```bash
vibecli --eval datasets list             # wired datasets, and the gaps, with reasons
vibecli --eval datasets fetch humaneval  # → ~/.vibecli/evals/datasets/
vibecli --eval datasets import mbpp --limit 100
```

Wired: **HumanEval** (MIT), **MBPP** (CC-BY-4.0), **SWE-bench Verified**.

Known gaps, each with the reason it is blocked, are listed by
`datasets list` — Terminal-Bench, Aider polyglot, GAIA, τ-bench, SWE-bench
Multimodal. Naming them is deliberate: an eval harness that silently covers
only what was easy leaves the reader assuming the gaps were considered.

> **Imported scores are not leaderboard-comparable.** Official SWE-bench numbers
> come from per-instance Docker images with pinned dependency sets. This harness
> clones the repo at the base commit and runs the declared tests in whatever
> environment it finds. Useful for tracking VibeCody against itself; not a
> number to publish next to anyone else's.

Datasets without a pinned `sha256` print the digest they observed, so it can be
pinned. Without one, a benchmark can change under you and every historical
comparison silently stops meaning the same thing.

---

## Using reports to change what gets built

```bash
vibecli --eval run --tag offline --provider claude --model claude-opus-5
vibecli --eval gate latest --baseline run-1754000000 --min-coverage 0.8
```

A report ends with three sections meant to be acted on:

1. **Capability × surface matrix** — is this a model problem or a transport
   problem?
2. **What to fix** — capabilities ranked worst-first, ties broken by sample
   size, each pointing at the module most likely responsible.
3. **Unmeasured capabilities** — gaps in the *suites*, stated separately so
   silence never reads as success.

Runs are archived at `~/.vibecli/evals/runs/<run-id>/` as both JSON and
Markdown. The gate exits `1` on regression and `0` when clean, so it drops into
CI directly.

---

## Adding a task

Add it to a suite file and run `cargo test -p vibe-eval`. The suite tests check
that it loads, that its id is unique, that its grader can actually reach a
verdict, that `unchanged` paths exist in the fixture, and that any toolchain
its grader shells out to is declared in `requires`.

```yaml
- id: my-task
  title: One line, human-facing — this is what a failure report shows
  capability: code_repair
  difficulty: medium
  requires: [python3]          # missing → skipped, not failed
  tags: [offline, python]
  prompt: |
    What the agent is asked to do.
  fixture:
    git_init: true
    files:
      src/thing.py: |
        # starting state
  grader:
    type: all
    of:
      - type: files
        assertions:
          - assert: unchanged
            path: tests.py     # they may not edit the test
      - type: command
        steps:
          - cmd: python3
            args: [tests.py]
            stdout_contains: OK
```

Before adding one, ask the question that matters: **how could this be passed
without doing the work?** Then close that path in the grader.
