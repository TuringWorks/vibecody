# Slice 5 — Model Hub + Lineage

**Status:** Draft · 2026-04-29
**Depends on:** [01-persistence.md](./01-persistence.md), [02-training-executor.md](./02-training-executor.md), [04-evaluation.md](./04-evaluation.md) (eval results attach to lineage nodes)
**Unblocks:** slice 6 (deployment promotes from the hub), slice 7 (distill needs to register children + parents)
**Disclaimer banner after this slice:** drops "Lineage" and "Rewards" from `covers={[...]}`. `RLModelLineage` shows a real DAG; `RLRewardDecomposition` shows real per-component reward attribution.

---

## Goal

Stop treating runs as the only first-class object. Promote **policies** to a first-class concept with semantic versioning, lineage edges, model cards, and an artifact store on disk that can be browsed independently from any single run.

`rl_model_hub.rs` (3,726 LOC) has the type system; slice 5 wires it.

## Domain model

```text
Run (slice 1)  ──produces──▶  Artifact (slice 1)  ──registered as──▶  Policy (slice 5)
                                                                      │
                                                                      ├── ModelCard (auto-gen)
                                                                      ├── LineageEdges (DAG)
                                                                      └── PromotionStage (slice 6)
```

A `Policy` is a registered, named, semver'd reference to one or more `Artifact`s plus metadata. A run produces artifacts; the user (or auto-rule) registers them as a policy version. This separation matters because:

- A single training run might produce multiple checkpoints worth registering (best-on-eval vs final).
- Distillation produces a policy whose artifacts come from the student run but whose lineage points at the teacher.
- A merged policy (slice 7) has multiple parent runs.

## Schema additions

Add to the migration started in slice 1 (or a follow-on migration if shipping order forbids):

```sql
CREATE TABLE rl_policies (
    policy_id        TEXT PRIMARY KEY,           -- ULID
    name             TEXT NOT NULL,
    version          TEXT NOT NULL,              -- semver
    description      TEXT,
    primary_artifact TEXT NOT NULL REFERENCES rl_artifacts(artifact_id),
    onnx_artifact    TEXT REFERENCES rl_artifacts(artifact_id),
    model_card_md    TEXT NOT NULL,
    framework        TEXT NOT NULL,              -- 'pytorch' | 'onnx' | 'native_candle'
    obs_space_json   TEXT NOT NULL,
    act_space_json   TEXT NOT NULL,
    obs_normalization_json TEXT,
    act_normalization_json TEXT,
    created_at       INTEGER NOT NULL,
    UNIQUE(name, version)
);
CREATE INDEX rl_policies_name_idx ON rl_policies(name, created_at DESC);

CREATE TABLE rl_policy_runs (                    -- many-to-many: policies ↔ runs
    policy_id TEXT NOT NULL REFERENCES rl_policies(policy_id) ON DELETE CASCADE,
    run_id    TEXT NOT NULL REFERENCES rl_runs(run_id),
    role      TEXT NOT NULL,                     -- 'producer' | 'teacher' | 'merge_source' | 'rlhf_base'
    PRIMARY KEY (policy_id, run_id, role)
);

CREATE TABLE rl_reward_components (
    run_id        TEXT NOT NULL REFERENCES rl_runs(run_id) ON DELETE CASCADE,
    episode_idx   INTEGER NOT NULL,
    component     TEXT NOT NULL,                 -- e.g. 'goal_reach' | 'control_cost' | 'safety_violation'
    contribution  REAL NOT NULL,
    PRIMARY KEY (run_id, episode_idx, component)
);
```

`rl_lineage_edges` (created in slice 1) is reused. Distillation, merging, RLHF all add edges here.

## Reward decomposition (the `RLRewardDecomposition` panel)

Today the panel renders mock per-component breakdowns. Real source: the env wrapper. We extend `vibe_rl/envs/wrappers.py:MonitorWrapper` to optionally accept a list of component names and emit per-step contributions whenever the env's `step` returns an `info` dict with `reward_components: {goal: 1.0, ctrl: -0.05, ...}`.

Most Gymnasium envs don't decompose by default. We document the contract:

> If your env's `step` returns `info["reward_components"]: dict[str, float]`, the wrapper aggregates per-episode and the panel surfaces it. Otherwise the panel shows aggregate reward only and a note ("env does not expose components").

This is a pragmatic compromise — we don't try to auto-decompose unstructured rewards. Custom envs and a few flagship Gymnasium envs (the safety-gym subset) get the real treatment.

The wrapper writes per-episode contributions to a separate JSON-Lines stream which the daemon batches into `rl_reward_components`.

## Lineage DAG (the `RLModelLineage` panel)

Edges come from three sources:

1. **Implicit**: `rl_runs.parent_run_id` (resume / continue from checkpoint).
2. **Explicit registered**: `rl_lineage_edges` rows written when slice 7 distill / merge / RLHF runs complete.
3. **Eval attachments**: `rl_eval_results` rows linking a policy back to a suite — surfaced as side-pane info on a node, not a DAG edge.

The DAG endpoint:

```
GET /v1/rl/policies/{policy_id}/lineage?depth=N
→ { nodes: [{policy_id, run_id, kind, ...}], edges: [{from, to, kind, weight}] }
```

depth=N walks both directions. Default 3.

## Auto-generated model card

When a policy is registered, the daemon writes a Markdown card to `<workspace>/.vibecli/rl-artifacts/<run_id>/MODEL_CARD.md` and stores the rendered text in `rl_policies.model_card_md`. Sections:

```markdown
# Policy: <name>@<version>
## Summary
- Algorithm: PPO
- Environment: gym:CartPole-v1:gym-0.29
- Trained for: 1,000,000 timesteps
- Final mean return: 487.3 ± 12.4 (last 100 episodes)

## Configuration
<config_yaml>

## Evaluation
| Suite | Metric | Value | CI | Pass? |
|---|---|---|---|---|
| cartpole-robustness-v1 | mean_return | 451.2 | [438.0, 464.5] | yes |
...

## Artifacts
- Primary (PyTorch): `final.pt` (sha256: ...)
- ONNX: `final.onnx` (sha256: ...)

## Lineage
- Parent run: <run_id>
- Distilled from: (none)

## Reproducibility
- Seed: 42
- Sidecar version: 0.1.0
- Git SHA: <repo HEAD at run time>
```

Cards are static at registration time. If eval results land later (eval-after-register flow), the card is regenerated.

## ONNX export

At policy-registration time, the sidecar runs `python -m vibe_rl export --run <run_id> --format onnx --output <path>`. ONNX export is in-scope for slice 5 because:

- Slice 6 (deployment) has a much easier story if it can rely on ONNX runtime instead of Python at serve time.
- Burn / Candle paths in Phase B/C all start from ONNX import.

Failure to export to ONNX (rare — some custom modules don't trace cleanly) is non-fatal: policy is registered without `onnx_artifact`, model card flags it, deployment falls back to the Python runtime path (slice 6).

## Artifact storage

Workspace-relative under `<workspace>/.vibecli/rl-artifacts/<run_id>/`. Per-policy convenience symlink at `<workspace>/.vibecli/rl-policies/<name>/<version>/` pointing into the run's artifact directory. (On Windows: file copies, not symlinks.)

We considered profile-wide artifact storage (`~/.vibecli/rl-artifacts/`) but rejected: artifacts can be GB-scale, and tying them to the workspace lifecycle (move/copy/delete the workspace, the artifacts come along) is the user-intuitive default. Future opt-in: a per-profile cache for shared base policies.

Garbage collection: on policy delete, optionally cascade to artifact files. UI confirms before delete with the on-disk size displayed.

## HTTP routes

| Method | Path | Body | Returns |
|---|---|---|---|
| `POST` | `/v1/rl/policies` | `{ name, version, run_id, primary_artifact_id, onnx_export: bool }` | `Policy` |
| `GET` | `/v1/rl/policies` | `?name=&framework=` | `Vec<Policy>` |
| `GET` | `/v1/rl/policies/{policy_id}` | — | `Policy` |
| `DELETE` | `/v1/rl/policies/{policy_id}` | `?cascade_artifacts=bool` | `204` |
| `GET` | `/v1/rl/policies/{policy_id}/lineage` | `?depth=` | `LineageGraph` |
| `GET` | `/v1/rl/policies/{policy_id}/card` | — | `text/markdown` |
| `GET` | `/v1/rl/runs/{run_id}/reward-components` | — | `Vec<ComponentRow>` |

## Tauri command rewrites

| Command | After slice 5 |
|---|---|
| `rl_list_policies` (currently mock at `commands.rs:41888`) | `GET /v1/rl/policies` (was deriving fake policies from runs) |
| `rl_get_model_lineage` | `GET /v1/rl/policies/{id}/lineage` |
| `rl_get_reward_decomposition` | `GET /v1/rl/runs/{id}/reward-components` |

New: `rl_register_policy`, `rl_get_policy_card`, `rl_delete_policy`.

## Frontend changes

`RLModelLineage.tsx`:
- Render the DAG with the existing graph component (already used by recap-resume / sandbox-tiers).
- Click node → side-pane with model card + eval summary.
- Filter by env, algorithm, framework.

`RLRewardDecomposition.tsx`:
- Stacked area chart: reward components over time within a run.
- Per-episode breakdown table.
- "This env does not expose components" empty state when nothing was emitted.

The Training Dashboard's run-detail view gets a "Register as policy" button when status=`Succeeded`.

## Definition of done

1. User finishes a training run, clicks "Register as policy", picks a name + version, gets a real Policy row with a model card and ONNX export (if exportable).
2. Lineage DAG correctly shows resume relationships and (by slice 7) distill/merge edges.
3. Reward decomposition shows real per-component breakdowns when the env exposes them, otherwise a clear empty state.
4. Disclaimer banner drops "Lineage" and "Rewards".

## Out of scope for slice 5

- Federated / shared model hub across users. Single-user only.
- Push-to-cloud registry (Hugging Face Hub, MLflow). Future, behind opt-in.
- Diffing two policies' weights (parameter-level). Future.
- Auto-cards beyond the static template. The card is generated; not interactive.
