---
layout: page
title: "Demo 66: SkillForge — Measure & Train Agent Skills"
permalink: /demos/66-skillforge/
---

## Overview

VibeCody ships ~710 agent-skill files in `vibecli/vibecli-cli/skills/*.md`. They're hand-authored and static. **SkillForge** makes that library measurable and self-improving: [SkillLens](https://github.com/TuringWorks/SkillLens) scores how useful a skill is to a target model, and [SkillOpt](https://github.com/TuringWorks/SkillOpt) trains the skill markdown with bounded edits accepted only when a held-out validation score strictly improves. The output is a `best_skill.md` deployed with **zero inference-time overhead** — the agent's skill loader just reads the improved file.

SkillForge is wired into the daemon the same way the [Code Graph](./41-semantic-index/) is: two standalone Rust crates (`skilllensai-rs`, `skilloptai-rs`), one daemon bridge module (`skillforge_index.rs`), `/v1/skilllens/*` + `/v1/skillopt/*` HTTP routes, and a VibeCoder panel. Every LLM call uses the toolbar-selected provider + model (STRICT — no hard-coded Anthropic).

**Time to complete:** ~10 minutes.

## Prerequisites

- VibeCLI daemon running (`vibecli --serve --port 7878`) — VibeCoder starts this automatically.
- At least one provider configured in the ProfileStore with a resolvable API key, selected in the VibeCoder toolbar.
- For the HTTP steps: the daemon bearer token at `~/.vibecli/daemon.token`.

## SkillForge vs hand-authored skills

| Aspect | Hand-authored skills | SkillForge |
|---|---|---|
| **Measurement** | None — guess if a skill helps | Trigger Coverage (deterministic) + Target Evolvability (LLM-held-out) |
| **Improvement** | Manual edits, no signal | Bounded add/delete/replace edits, accepted only on strict val gain |
| **Regression safety** | Trust the author | Strict held-out gate + rejected-edit buffer + `*.opt.md` (shipped file untouched) |
| **Cost control** | N/A | Hard token budget + live spent-tokens meter + abort on cap |
| **Deploy overhead** | N/A | Zero — the loader reads the improved file, no new model call |
| **Provider lock-in** | N/A | None — any toolbar-selected provider/model |

## Step-by-Step Walkthrough

### 1. Open the SkillForge panel

In VibeCoder, open the **AI/ML** composite and click the **SkillForge** tab. Three views appear: **Catalog**, **Lens**, **Optimize**. The toolbar `selectedProvider` / `selectedModel` drive every LLM call — if either is unset, the Lens/Optimize views show a "select a model" empty state.

### 2. Browse the Catalog

The **Catalog** view lists the ~710 skills as a table (name, category, cached coverage/evolvability, source). No LLM call. Click a row to open that skill in **Lens**.

> Behind the scenes: `GET /v1/skilllens/skills` → daemon reads the bundled `skills/*.md` via the existing `SkillCatalog` (the MCP data layer — no re-parse), returns `{skills:[{name, category, summary, source, trigger_coverage?, …}]}`.

### 3. Score a skill (Lens)

In **Lens**, pick a skill (e.g. `formal-verification`) and click **Score**. Three metric cards appear:

- **Trigger Coverage** — deterministic, no LLM. How often the skill's declared triggers fire on the run set.
- **Target Evolvability** — the held-out lift the skill provides, LLM-graded. This is the headline number.
- **Extraction Efficacy** — `—` until you supply a trajectory pool (Phase 4 follow-up).

> Behind the scenes: `POST /v1/skilllens/score {skill, tasks?, provider, model}`. The daemon builds the LLM from `(provider, model)` via `build_provider_override`, constructs `EvalTask`s from the catalog, runs `skilllensai::metrics::target_evolvability`, caches the report, and returns it.

### 4. Train a skill (Optimize)

In **Optimize**, configure `TrainConfig`:

| Field | Default | Meaning |
|---|---|---|
| Epochs | 8 | Max training epochs |
| Val split | 0.3 | Held-out fraction (strict gate scores against this) |
| Textual LR | 512 | Max chars added/replaced per edit (stability) |
| Patience | 3 | Early-stop after N epochs with no val gain |
| Seed | 0 | Full determinism (seeded splitmix64 — no `Math.random`/wall-clock) |
| Env kind | `repo` | `repo` derives `EvalTask`s from the catalog; `static` takes inline JSONL |

Click **Train**. A job launches on the daemon and the panel polls `GET /v1/skillopt/status/:job` every 1.5 s. The **validation curve** renders as an inline SVG sparkline — one point per epoch — alongside accepted/rejected counts and a spent-tokens meter. Early-stop fires when patience is exceeded.

> Behind the scenes: `POST /v1/skillopt/train {skill, env:{kind, tasks?}, config, provider, model}` → spawns a tokio task, returns `{job_id}`. Each epoch: rollout → bounded edit proposal → strict held-out gate (`>` not `>=`) → accept/reject + rejected-edit buffer. Cancel is best-effort (`POST /v1/skillopt/cancel/:job`).

### 5. Promote

When the job reaches `done`, expand the trained **best_skill.md** to review it, then click **Promote**. A confirm banner explains that promoting writes `<skill>.opt.md` next to the shipped skill and **never overwrites the shipped file**. Swapping a promoted skill into the live agent is a separate, deliberate action — no silent regressions.

> Behind the scenes: `POST /v1/skillopt/promote {skill, content}` writes the file. The shipped 710 skills stay pristine.

### 6. Drive it over HTTP directly

The daemon is the single source of truth, so any client works. With the bearer token:

```bash
TOKEN=$(cat ~/.vibecli/daemon.token)
BASE=http://localhost:7878

# Catalogue (no LLM)
curl -s -H "Authorization: Bearer $TOKEN" $BASE/v1/skilllens/skills | jq '.skills | length'

# Score a skill against the toolbar-selected model
curl -s -H "Authorization: Bearer $TOKEN" -H "Content-Type: application/json" \
  $BASE/v1/skilllens/score \
  -d '{"skill":"formal-verification","provider":"anthropic","model":"claude-sonnet-5"}' | jq '.report'

# Launch a train job
JOB=$(curl -s -H "Authorization: Bearer $TOKEN" -H "Content-Type: application/json" \
  $BASE/v1/skillopt/train \
  -d '{"skill":"formal-verification","env":{"kind":"repo"},"config":{"epochs":4,"seed":1},"provider":"anthropic","model":"claude-sonnet-5"}' \
  | jq -r .job_id)

# Poll status
curl -s -H "Authorization: Bearer $TOKEN" $BASE/v1/skillopt/status/$JOB | jq '{state, val_curve: .report.val_curve, spent_tokens: .report.spent_tokens}'

# Promote (writes *.opt.md, shipped skill untouched)
curl -s -H "Authorization: Bearer $TOKEN" -H "Content-Type: application/json" \
  $BASE/v1/skillopt/promote \
  -d '{"skill":"formal-verification","content":"# formal-verification\n…trained body…"}'
```

## Client fan-out

| Client | What it ships |
|---|---|
| **VibeCoder** | `SkillForgePanel` (Catalog / Lens / Optimize) — full surface |
| **VS Code extension** | `skilllens{List,Get,Refresh,Convert,Extract,Score}` + `skillopt{Train,Status,Cancel,Promote}` — full surface |
| **Agent SDK** | `agent.skilllens.*` + `agent.skillopt.*` — full surface |
| **VibeMobile (Flutter)** | `skilllensSkills`, `skilllensSkill`, `skilloptStatus` — read-only |
| **Apple Watch / Wear OS** | `loadSkilllensSkills` / `skilllensSkills` + one-line skill detail — read-only |

Read-only endpoints fan out to every client; the heavy `score`/`train`/`promote` mutations ship only on desktop-class clients (the wrist/mobile form factor doesn't surface a toolbar-selected LLM).

## Configuration Reference

`TrainConfig` (POST `/v1/skillopt/train` `config` body) — all optional, sensible defaults:

| Field | Type | Default | Notes |
|---|---|---|---|
| `epochs` | u32 | 8 | Max epochs |
| `val_split` | f32 | 0.3 | Held-out fraction `(0,1)` |
| `textual_lr` | u32 | 512 | Max chars per edit |
| `patience` | u32 | 3 | Early-stop window |
| `seed` | u64 | 0 | Full determinism |

A hard token budget caps cost; the panel's spent-tokens meter shows live spend and the run aborts on cap.

## What's Next

- [SkillForge reference](../skillforge/) — full HTTP surface + standalone crates
- [Demo 41: Deep Semantic Code Index](./41-semantic-index/) — the kodegraph integration SkillForge mirrors
- [Plugin Development](../plugin-development/) — authoring skills by hand (the input SkillForge optimises)