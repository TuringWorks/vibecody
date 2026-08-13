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

## One run is weak evidence

Agent runs are not deterministic, and the spread is not small. The greenfield
e-commerce build, same task and same model, five attempts:

| attempt | outcome | time |
|---|---|---:|
| 1 | timed out | 3600s |
| 2 | **8/8 rungs, complete** | 65.6s |
| 3–5 | timed out, **no `server.py` written at all** | 903s each |

One run in five built the entire thing in about a minute; the rest produced
nothing. Quoting either end as "how VibeCody performs" would be reporting a
coin toss.

```bash
vibecli --eval run --suite greenfield --samples 5
```

`--samples` repeats each task and the report gains an **Unstable tasks**
section listing anything whose samples disagreed, worst-first. Row keys keep
their single-sample form for the first repetition, so turning sampling on does
not invalidate an existing baseline.

Timeouts are diagnosable too: the CLI harness streams the child's output into
capped buffers as it arrives, so a killed run still reports its last output.
Collecting it with `Command::output()` — the obvious implementation — loses
every byte when the timeout cancels the future, which is how three consecutive
15-minute failures produced no explanation at all.

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
| `greenfield` | building a whole working application from a spec | `python3` |
| `brownfield` | changing an existing multi-module service | `python3` |
| `migrations` | Python 2→3, CommonJS→ESM, retiring a deprecated API | `python3`, `node` |
| `vibecoder-panels` | every panel's commands exist and honour the toolbar provider | `python3` |
| `continuity` | the conversation survives compaction, hand-off and being killed | `python3` |

All of it runs **offline**. Missing toolchains skip rather than fail.

### Greenfield builds — and why they are scored as a ladder

Every other suite hands the agent a file and asks for a change. `greenfield`
hands it a specification and an empty directory, which is the thing agents are
actually sold on and the hardest thing to grade.

Two decisions make it work:

- **Black-box grading.** The agent picks its own structure and names; the
  grader starts the server it produced and drives it over HTTP. Asserting on
  file layout would measure obedience to a structure we invented rather than
  whether the thing works.
- **A ladder, not a verdict.** Eight milestones — boots, catalogue, lookup,
  carts, add-items, totals, checkout-and-stock, edge cases — are separate
  children of `all`, so nothing short-circuits and the mean is a real
  *completion depth*. An agent that serves a catalogue but never gets checkout
  working scores 0.38 and the grade tree shows exactly where it stopped. For a
  build that can take an hour, throwing that away for one pass/fail bit is
  wasteful, and "0%" would say the same thing about an empty directory as
  about a nearly-finished store.

The task is validated in both directions: a reference implementation of the
spec scores 8/8, and a deliberately half-finished one scores 3/8. An eval
nobody can pass is worthless, and one everybody passes measures nothing.

These runs are slow, so `greenfield` is deliberately **not** tagged `offline` —
a quick `--tag offline` run stays quick. Opt in with `--suite greenfield`.

### Brownfield — changing code you did not write

`evals/fixtures/orders-service/` is a small real service: four modules across
two packages, a seam between them, and a passing test suite. Tasks add a
feature across three modules, fix a stock leak whose cause sits two modules
from its symptom, and translate an error at a package boundary.

Every task pairs "the new behaviour works" with "the existing behaviour still
works", and asserts `tests/test_existing.py` is `unchanged` — editing the
regression suite is the fastest way to make a change look successful.

### Migrations — the long, boring, high-risk kind of work

Upgrades are where agents most often quietly half-finish: convert four files
out of six, leave the shim in place, or "fix" a call site by deleting the
feature. Each grader checks three things rather than one:

1. the new world works,
2. the old world is **gone** — no surviving `require`, no leftover `legacy.py`,
3. behaviour is unchanged.

The traps are deliberate. In the Python 2→3 port, `mean` uses `/` on two ints:
a mechanical port silently turns 2 into 2.5 and every caller downstream
drifts, so the grader asserts the result is still `2` **and** still an `int`.

### VibeCoder panels — the check nothing else performs

`invoke('some_command')` is a string. TypeScript cannot check it, Rust never
sees it, and the failure is a runtime rejection inside whichever panel nobody
opened during review. `tsc --noEmit` passes, `vitest` passes, and the tab
throws.

`vibecoder-panels` connects the two halves: every command a panel invokes must
appear in that shell's `generate_handler!`, every LLM panel must honour the
toolbar provider (AGENTS.md → Provider-Agnostic Panels — STRICT), and no panel
may reach a protected daemon route with a bare `fetch`. All static, all free,
all in CI.

### Continuity — a property of *when* the code writes

Three things routinely destroy an agent's history mid-task: compaction rewrites
the middle of the conversation, a circuit-breaker hand-off clears it outright,
and the process can simply be killed. The greenfield build hit the third on
four consecutive runs, and **one of those had finished the entire application**
before it was cut off.

What made that expensive is where the writes were. `save_messages` had two
callers — an explicit `/fork` and the end of a run — and the headless `--exec`
path had *neither*: 35 step-traces in `~/.vibecli/traces` with not one
conversation transcript beside them. Nothing run headlessly was ever resumable.

The agent loop now checkpoints at 80% of the context budget (**before**
`prune_middle` can replace the history with a summary), before a hand-off
retires the context, and once more at the end of every run.

These tasks are static checks over the source, because continuity is a property
of *ordering*: a test can assert a checkpoint exists, but only reading the code
can assert it happens before the thing that destroys what it was meant to save.
Reproducing genuine context pressure end-to-end costs an hour of tokens per
sample and is non-deterministic on top.

> A note on writing these: two of the five checks initially reported false
> positives — comment prose inside `generate_handler!` read as command names,
> and handlers re-exported from a sibling crate read as unimplemented. Both
> were fixed before shipping. **An eval that cries wolf is worse than no eval**,
> because it trains people to ignore the one that matters.

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
