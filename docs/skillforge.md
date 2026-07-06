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
3. **Optimize** — configure `TrainConfig` (epochs / val split / textual LR / patience / seed + env kind `repo`|`static`), launch a train job, watch the **validation curve** (inline SVG sparkline) update as epochs complete, see accepted/rejected counts + a spent-tokens meter, expand the trained `best_skill.md`, then **Promote**.

**Promote is guarded.** It writes `<skill>.opt.md` next to the shipped skill and **never overwrites the shipped file**. Swapping a promoted skill into the live agent is a separate, deliberate action — no silent regressions.

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
| `POST` | `/v1/skillopt/train` | `{skill, env:{kind, tasks?}, config, provider, model}` → `{job_id}` | yes (async) |
| `GET` | `/v1/skillopt/status/:job` | — | no |
| `POST` | `/v1/skillopt/cancel/:job` | — | no |
| `POST` | `/v1/skillopt/promote` | `{skill, content}` | no |

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
| **VibeUI** (desktop) | full | `SkillForgePanel` (Catalog / Lens / Optimize) via 10 Tauri commands that proxy the daemon |
| **VS Code extension** | full | `skilllens{ListSkills,GetSkill,Refresh,Convert,Extract,Score}` + `skillopt{Train,Status,Cancel,Promote}` |
| **Agent SDK** (TypeScript) | full | `agent.skilllens.{list,get,refresh,convert,extract,score}` + `agent.skillopt.{train,status,cancel,promote}` |
| **VibeMobile** (Flutter) | read-only | `skilllensSkills`, `skilllensSkill(name)`, `skilloptStatus(jobId)` |
| **Apple Watch** | read-only | `loadSkilllensSkills`, `loadSkilllensSkill(name)` |
| **Wear OS** | read-only | `skilllensSkills()`, `skilllensSkill(name)` |

Read-only endpoints (catalogue + train-status) fan out to every client; the heavy `score`/`train`/`promote` mutations ship only in the desktop-class clients (VibeUI, VS Code, Agent SDK) — the wrist/mobile form factor doesn't surface a toolbar-selected LLM, so the mutations would have no `provider`/`model` to forward.

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

**Deferred follow-ups** (tracked in `notes/skillforge/06`):
- Per-epoch SSE streaming + true cancellation (needs a callback/cancel token in `skilloptai::trainer::train`).
- `RepoAgentEnv` tasks derived from real VibeCody agent-job history (decision-tracing already exists) instead of the catalog.
- Efficacy-metric substrate: LLM-judge vs embedding-overlap for `extraction_efficacy`.
- Promoted-skill override dir (`<ws>/.vibecli/skills/*.opt.md`) vs in-repo overwrite — leaning per-workspace override so the shipped 710 stay pristine.
- Nightly "sleep" job (offline self-evolution with experience replay).
- External benchmark `Env` impls (SWE-bench / BFCL) behind a `benchmarks` feature.