---
layout: page
title: SkillForge
permalink: /skillforge/
---

> **One-liner.** SkillForge makes VibeCody's shipped skill library **measurable** (SkillLens) and **self-improving** (SkillOpt) — turning each skill markdown doc into trainable state that improves against held-out validation, with **zero inference-time overhead** at deploy.

VibeCody ships ~710 skill files in `vibecli/vibecli-cli/skills/*.md`. Today they are hand-authored and static. SkillForge is a Rust port of [TuringWorks/SkillLens](https://github.com/TuringWorks/SkillLens) + [TuringWorks/SkillOpt](https://github.com/TuringWorks/SkillOpt), wired into the daemon the same way the [Code Graph](./demos/41-semantic-index/) (kodegraph) is — one standalone crate, one daemon bridge module, HTTP routes, and a VibeUI panel.

This page documents the user-facing surface. For the full design, decisions, and roadmap, see the **`notes/skillforge/`** vault (start at `SkillForge — MOC.md`). For a runnable walkthrough, see [Demo 66: SkillForge](./demos/66-skillforge/).

## What it does

### SkillLens — analyse

Normalises agent runs into a unified **Trajectory** schema, extracts candidate skills (sequential baseline / parallel mode-merge), and scores skill utility:

- **Trigger Coverage** — deterministic, no LLM. How often the skill's declared triggers fire on the run set.
- **Extraction Efficacy** — does extracting skills from a pool beat a no-skill baseline? (Needs a trajectory pool.)
- **Target Evolvability** — the held-out lift a skill provides. LLM-graded against `EvalTask`s.

### SkillOpt — train

Treats a skill markdown doc as the trainable state of a frozen agent. Scored **rollouts** drive bounded **add/delete/replace** edits, accepted **only when a held-out validation score strictly improves** — epoch after epoch, with a rejected-edit buffer and a textual learning rate for stability. Output is a `best_skill.md` deployed with **zero inference-time overhead** (no new model call at inference time — the loader just reads the improved file).

## The VibeUI panel

Open the **SkillForge** tab in the AI/ML composite. Three views:

1. **Catalog** — the ~710 skills as a table (name, category, cached coverage/evolvability, source). No LLM call. Click a row to open it in Lens.
2. **Lens** — pick a skill → **Score** against the toolbar-selected model → three metric cards (Trigger Coverage, Target Evolvability, Extraction Efficacy) with progress bars.
3. **Optimize** — configure `TrainConfig` (epochs / val split / textual LR / patience / seed + env kind `repo`|`static`|`history`), launch a train job, watch the **validation curve** (inline SVG sparkline) update as epochs complete, see accepted/rejected counts + a spent-tokens meter, expand the trained `best_skill.md`, then **Promote**.

**Promote is guarded.** It writes `<skill>.opt.md` to the per-workspace override dir (`<workspace>/.vibecli/skills/`, or `~/.vibecli/skills/` when no workspace is resolved) and **never overwrites the shipped `skills/*.md`** — the 710 shipped skills stay pristine. The catalogue JSON surfaces `has_promoted_override` on each skill and `promoted_override` (path) on the detail view, so a UI can badge overridden skills. Swapping a promoted skill into the live agent is a separate, deliberate action — no silent regressions.

## Provider-agnostic (STRICT)

Every LLM call — SkillLens scoring/extraction and SkillOpt training — uses the provider and model selected in the VibeUI toolbar dropdown (`selectedProvider` / `selectedModel`). No panel, daemon route, Tauri command, or client method hard-codes Anthropic or any single provider. If the toolbar selection is empty, the panel shows a "select a model" empty state rather than silently calling a default. See [AGENTS.md → Provider-Agnostic Panels](../AGENTS.md).

## HTTP surface

All routes live on the VibeCLI daemon (`vibecli --serve --port 7878`). Shapes are daemon-owned; responses are JSON.

### `/v1/skilllens/*` — analyse

| Method | Path | Body | LLM? |
|---|---|---|---|
| `GET` | `/v1/skilllens/skills` | — | no |
| `GET` | `/v1/skilllens/skills/:name` | — | no |
| `POST` | `/v1/skilllens/refresh` | — | no |
| `POST` | `/v1/skilllens/convert` | `{runs}` | no |
| `POST` | `/v1/skilllens/extract` | `{pool, method, provider, model}` | yes |
| `POST` | `/v1/skilllens/score` | `{skill, tasks?, provider, model}` | yes |

### `/v1/skillopt/*` — train

| Method | Path | Body | LLM? |
|---|---|---|---|
| `POST` | `/v1/skillopt/train` | `{skill, env:{kind:"repo"\|"static"\|"history", tasks?, grader?}, config, provider, model}` → `{job_id}` | yes (async) |
| `POST` | `/v1/skillopt/train/stream` | same body → SSE: `job` → `epoch`* → `done` | yes (async) |
| `GET` | `/v1/skillopt/status/:job` | — | no |
| `POST` | `/v1/skillopt/cancel/:job` | — | no |
| `POST` | `/v1/skillopt/promote` | `{skill, content}` | no |

`/v1/skillopt/train/stream` is the streaming variant of `/train`: it spawns the same job (registered in the shared job map, so `/status` and `/cancel` work identically) and emits Server-Sent Events — one `job` event carrying `{job_id, status, llm}`, one `epoch` event per completed epoch (the `EpochEvent` JSON: `epoch`, `best_val`, `accepted`, `rejected`, `spent_tokens`, `early_stopped`), and a terminal `done` event with the final `TrainJob` JSON (state `done` / `cancelled` / `failed`). On launch failure it emits a single `error` event. Keep-alive pings every 15s. Cancel with `POST /v1/skillopt/cancel/:job` flips a live cancel token observed at the next epoch boundary — the run stops promptly and a final `done` event carries the `cancelled` state.

`/health` reports `skillforge: {status, skills, cached_reports, toolchain}`; the startup banner prints `skillforge: ready (N skills)`.

### Watch surface (curated)

The watch form factor gets two compact, read-only routes (never `/v1/*`):

| Method | Path | Shape |
|---|---|---|
| `GET` | `/watch/skilllens/skills` | `{count, top5:[{name, category, summary}]}` |
| `GET` | `/watch/skilllens/skills/:name` | `{name, category, summary}` |

## Client fan-out

The daemon is the single source of truth; clients proxy it.

| Client | Surface | Methods |
|---|---|---|
| **VibeCLI REPL** | full (drive) | `/skillforge list/show/refresh/score/train/status/cancel/promote/health` — calls the bridge in-process; `score`/`train` use the REPL `active_provider`/`active_model` (STRICT) |
| **VibeCLI TUI** | read-only browse | `/skillforge` opens a Ratatui screen — catalogue (cov/evolvability) + train-jobs pane + `/health` footer; `j`/`k`/`r` navigate |
| **VibeUI** (desktop) | full | `SkillForgePanel` (Catalog / Lens / Optimize) via 10 Tauri commands that proxy the daemon |
| **VibeApp** (companion) | backend surface | 10 Tauri proxy commands registered in `vibeapp/src-tauri` (the bespoke UI has no panel; reachable via `invoke()`) |
| **VS Code extension** | full | `skilllens{ListSkills,GetSkill,Refresh,Convert,Extract,Score}` + `skillopt{Train,StreamTrain,Status,Cancel,Promote}` |
| **Agent SDK** (TypeScript) | full | `agent.skilllens.{list,get,refresh,convert,extract,score}` + `agent.skillopt.{train,streamTrain,status,cancel,promote}` |
| **VibeMobile** (Flutter) | read-only | `SkillforgeScreen` (8th "Skills" tab) — catalogue across paired machines + detail + train-status lookup |
| **Apple Watch** | read-only | `SkillforgeView` (6th "Skills" tab) — `top5` catalogue → one-line detail |
| **Wear OS** | read-only | `SkillforgeScreen` + `SkillforgeTileService` — `top5` catalogue → detail |
| **Agent system prompt** | auto | a compact `## Skill Health` line (`N skills, M scored, top evolvability X`) auto-injected when `cached_reports > 0` (G3) |

Read-only endpoints (catalogue + train-status) fan out to every client; the heavy `score`/`train`/`promote` mutations ship only in the desktop-class clients (VibeCLI REPL, VibeUI, VS Code, Agent SDK) — the wrist/mobile form factor doesn't surface a toolbar-selected LLM, so the mutations would have no `provider`/`model` to forward.

## The standalone crates

SkillForge is two MIT-licensed standalone crates + a facade, independent of `vibecli`/`vibe-ai`:

| Crate | Role |
|---|---|
| [`skilllensai-rs`](https://github.com/TuringWorks/vibecody/tree/main/skilllensai-rs) | analyse: trajectory → extract → score |
| [`skilloptai-rs`](https://github.com/TuringWorks/vibecody/tree/main/skilloptai-rs) | train: rollout → bounded edit → strict held-out gate → epoch |
| [`skillforgeai-rs`](https://github.com/TuringWorks/vibecody/tree/main/skillforgeai-rs) | facade: `use skillforgeai::{lens, opt};` |

Provider-agnostic via a crate-local `SkillLlm` trait; the daemon bridge adapts it onto `vibe_ai::AIProvider`. The standalone CLIs (`skilllensai`, `skilloptai`) ship an OpenAI-compatible `SkillLlm` so the crates work without VibeCody.

## Status & follow-ups

**Phases 0–5 done** (2026-07-06): crates, daemon bridge + routes, watch mirror, VibeUI panel, full client fan-out, docs. `cargo test` green (23 + 1 golden, 30 tests); `cargo check` + `tsc --noEmit` + `dart analyze` clean.

**Gap-closure pass done** (2026-07-06): the seven post-Phase-5 visibility gaps from `notes/skillforge/07` are all closed — VibeCLI REPL `/skillforge` command + TUI browse screen + agent system-prompt `## Skill Health` line (auto-gated on `cached_reports > 0`), VibeApp Tauri surface, Flutter "Skills" screen, Apple Watch "Skills" view + Wear OS `SkillforgeScreen`/`SkillforgeTileService`. The daemon's own client no longer needs `curl`; every client surface now renders the catalogue. See `notes/skillforge/07 — Client Visibility & UX Gaps.md`.

**Per-epoch streaming + true cancellation done** (2026-07-07): `skilloptai::trainer::train_with_signals` threads a dependency-free `CancelToken` (checked between epochs) and an optional per-epoch `EpochEvent` channel; the plain `train()` entry point is now a thin wrapper with empty signals. The daemon surfaces this as `POST /v1/skillopt/train/stream` (SSE: `job` → `epoch`* → `done`/`error`), sharing the same job map as the poll-based `/train` so `/status` and `/cancel` work on both. `cancel/:job` now flips the live token so the run stops at the next epoch boundary instead of running to completion. 30 existing skilloptai tests + 3 new (cancel-before-first-epoch, cancel-observed-between-epochs, one-event-per-epoch) green; 13 bridge tests green (2 new cancel-token cases). The VS Code extension (`VibeCLIClient.skilloptStreamTrain`) and the Agent SDK (`agent.skillopt.streamTrain`) consume the stream as an `AsyncGenerator<SkilloptTrainEvent>` (`{type:'job'|'epoch'|'done'|'error', …}`) via a small typed-SSE parser (`readSseTypedEvents`) layered over the existing `data:`-only helpers; 4 new Agent SDK vitest cases (job→epoch*→done ordering, error-stop, non-2xx throw, request-body shape) + `tsc --noEmit` clean for both clients. The poll-based `/status` surface remains for clients that prefer it (Flutter/Watch/Wear).

**Promoted-skill override dir done** (2026-07-08): `promote` now writes `<skill>.opt.md` to the per-workspace override dir (`<workspace>/.vibecli/skills/`, falling back to `~/.vibecli/skills/` when no workspace is resolved), so the 710 shipped `skills/*.md` stay pristine — no in-repo overwrite. The catalogue list surfaces `has_promoted_override` per skill and the detail view surfaces `promoted_override` (path or null); the bridge scans the override dir at init + refresh so overrides are picked up without a restart. 4 new bridge tests cover the dir resolver, the write helper, the stem-keyed scan, and a missing-dir empty result; 17 bridge tests + `cargo check` + `tsc --noEmit` clean. The shipped-skill path resolution is unchanged (`SkillCatalog` still wins over same-named plugins).

**Real agent-job history env done** (2026-07-08): a third env kind, `history`, derives `EvalTask`s from actual agent runs instead of the catalog. The CLI writes a lightweight per-session `SkillEvalRecord` (`<session_id>-eval.json`) at the end of every agent run — `{session_id, timestamp, prompt (first user msg), final_answer (last assistant prose), tool_success_rate, steps, completed}` — alongside the existing `<session_id>.jsonl` trace (secrets scrubbed). `env.kind=history` scans `~/.vibecli/traces/` (or an `env.tasks` override dir) for those records and builds one task per run: the session's prompt becomes the task prompt, and the grader is `LlmJudge` (default — rubric cites the reference final answer + tool-success rate + completion; one extra LLM call per task per epoch) or `Contains` (free, weak — a phrase from the reference answer), selected via `env.grader="llm_judge"|"contains"`. Records with an empty prompt or final answer (errored/truncated runs) are skipped; an empty trace dir returns a user-facing "no agent-job history found" error. The catalog-derived `repo` env is unchanged. New: `SkillEvalRecord` + `TraceWriter::save_eval_record` + `load_eval_records` in `vibe-ai::trace` (re-exported); `EnvKind::History` + `EnvSpec.grader` + `RepoAgentEnv::from_history` + `history_trace_dir` + `parse_history_grader` in the bridge; the VS Code extension + Agent SDK `train`/`streamTrain` accept `'history'` + an optional `envGrader`. 9 new bridge tests + 4 new trace tests green; `cargo check` + `tsc --noEmit` + 42 Agent SDK vitest clean.

**Deferred follow-ups** (tracked in `notes/skillforge/06`):
- Efficacy-metric substrate: LLM-judge vs embedding-overlap for `extraction_efficacy`.
- Nightly "sleep" job (offline self-evolution with experience replay).
- External benchmark `Env` impls (SWE-bench / BFCL) behind a `benchmarks` feature.