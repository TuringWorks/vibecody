# Slice 3 — Environment Registry

**Status:** Draft · 2026-04-29
**Depends on:** [01-persistence.md](./01-persistence.md), [02-training-executor.md](./02-training-executor.md)
**Unblocks:** slice 4 (eval needs envs), slice 7 (multi-agent needs PettingZoo wiring)
**Disclaimer banner after this slice:** drops "Environments" from `covers={[...]}`. `RLEnvironmentViewer` shows real Gymnasium + custom envs registered in this workspace.

---

## Goal

Replace the hardcoded `vec![CartPole-v1, Franka-Reach, Trading-Sim]` at `commands.rs:41834-41862` with a real registry that:

1. Probes the sidecar for installed Gymnasium / PettingZoo envs.
2. Lets users register custom envs from a Python file in their workspace.
3. Versions envs (a small DAG so eval suites can pin to "the env as it existed when this run was trained").
4. Serves the spec (observation space, action space, reward range, max episode steps) to the panel.

`rl_env_os.rs` (4,202 LOC) already has the type system for this; slice 3 wires it.

## What `rl_env_os.rs` gives us

It's all types — `ObservationSpace::{Box, Discrete, MultiDiscrete, Dict, Tuple}`, `ActionSpace::*`, `EnvSpec`, `EnvVersion`, `EnvRegistry`, `DomainRandomizationConfig`, etc. None of it executes anything. We use these types as the **canonical spec format** in `rl_environments.spec_json` (the SQL column from slice 1) and over the HTTP API. The sidecar serializes `gymnasium.Env.observation_space` into the same JSON shape via a small adapter.

What we do **not** use from `rl_env_os.rs` in slice 3:

- The simulation backend enums (`MuJoCo`, `PhysX`, `Brax`, `Unity`). Concrete sims live in Python via Gymnasium; we don't wrap them in Rust.
- The real-world connector enums (`REST`, `gRPC`, `MQTT`, `WebSocket`). Out of scope for slice 3 — these are post-doc-set work.

## Registry sources

| Source | Discovery | Versioning |
|---|---|---|
| `gymnasium` | `python -m vibe_rl probe-envs --source gymnasium` walks `gymnasium.envs.registry` | env spec ID (e.g. `CartPole-v1`) + `gymnasium.__version__` |
| `pettingzoo` | `python -m vibe_rl probe-envs --source pettingzoo` walks PettingZoo envs (slice 7 leans on this) | env id + `pettingzoo.__version__` |
| `custom_python` | User points at a `.py` file containing a `gymnasium.Env` subclass; sidecar imports it, captures spaces | content-hashed (`sha256` of the file) — version DAG via `parent_env_id` |
| `custom_dsl` | A YAML env spec compatible with `rl_env_os.rs::EnvSpec` (no execution, observation/eval-only) | semver |

Slice 3 ships sources 1, 2, and 3. `custom_dsl` is enumerable but inert until a future slice gives it a runtime.

## Sidecar probe

```text
$ python -m vibe_rl probe-envs --source gymnasium --json
{"source":"gymnasium","sdk_version":"0.29.1","envs":[
  {"id":"CartPole-v1","entry_point":"gymnasium.envs.classic_control.cartpole:CartPoleEnv",
   "observation_space":{"kind":"box","shape":[4],"low":[-4.8,-Infinity,-0.42,-Infinity],"high":[4.8,Infinity,0.42,Infinity],"dtype":"float32"},
   "action_space":{"kind":"discrete","n":2},
   "reward_threshold":475.0,"max_episode_steps":500,"nondeterministic":false},
  ...
]}
```

The Rust side merges this into `rl_environments` (UPSERT by `(name, version, source)`), preserving `env_id`s for runs that already reference them.

## HTTP routes

Added to `serve.rs`:

| Method | Path | Body | Returns |
|---|---|---|---|
| `GET` | `/v1/rl/envs` | `?source=&search=` | `Vec<Environment>` |
| `GET` | `/v1/rl/envs/{env_id}` | — | `Environment` |
| `POST` | `/v1/rl/envs/refresh` | `{source: "gymnasium" \| "pettingzoo"}` | `RefreshReport { added, updated, removed }` |
| `POST` | `/v1/rl/envs/custom` | `{ name, version, file_path }` | `Environment` (file_path is workspace-relative) |
| `DELETE` | `/v1/rl/envs/{env_id}` | — | `204` (only `custom_*` sources can be deleted; gym/pettingzoo refresh-managed) |

`refresh` is debounced — once per profile session unless force=true. Probing Gymnasium takes ~200 ms; PettingZoo is heavier (~1 s) since some envs import-side-effect.

## Tauri command rewrites

| Command | After slice 3 |
|---|---|
| `rl_list_environments` (currently mock at `commands.rs:41863`) | `GET /v1/rl/envs` |
| `rl_get_environment` (currently mock at `commands.rs:41869`) | `GET /v1/rl/envs/{id}` |
| `rl_deploy_environment` (currently a no-op at `commands.rs:41877`) | **Renamed** to `rl_register_custom_environment` → `POST /v1/rl/envs/custom`. Old name kept as deprecated alias for one release. |

New command: `rl_refresh_environments` → `POST /v1/rl/envs/refresh`. Surfaced in `RLEnvironmentViewer.tsx` as a refresh icon.

## Frontend changes

`RLEnvironmentViewer.tsx`:

1. Replace the hardcoded list assumption with the registry response.
2. Add a "Register custom env" dialog with a file picker scoped to the workspace.
3. Add per-source filter (Gymnasium / PettingZoo / Custom) and a search box.
4. Show the spec JSON pretty-printed with the syntax-highlighter from the design system.
5. Surface the version DAG (parent → children) for custom envs that have been edited and re-registered.

The wizard step in `RLTrainingDashboard.tsx` for environment selection switches from a static dropdown to `rl_list_environments()`. This is where slice 3's value compounds — slice 2 trains *something*, slice 3 lets the user pick what.

## Domain randomization

`rl_env_os.rs::DomainRandomizationConfig` exists. We expose it via a per-run override field in `CreateRunRequest`:

```yaml
environment_id: gym:CartPole-v1:gym-0.29
domain_randomization:
  enabled: true
  parameters:
    - name: gravity
      kind: uniform
      low: 9.0
      high: 11.0
    - name: pole_length
      kind: gaussian
      mean: 0.5
      std: 0.05
```

The sidecar wraps the env in a `DomainRandomizationWrapper` that applies overrides per episode. This is one Python file (`vibe_rl/envs/wrappers.py:DomainRandomizationWrapper`) implementing the spec.

## Environment lineage

When a user registers an updated version of a custom env (same `name`, new content hash → new `env_id`), we record an edge to the prior version via `parent_env_id`. This DAG feeds slice 5 (model lineage shows "this policy was trained on env v3 of frozen-lake-custom").

## Out of scope for slice 3

- **Real-world connectors** (REST/gRPC/MQTT envs from `rl_env_os.rs`). These need broker integration (sandbox-tiers egress broker) — separate workstream.
- **Vector envs configured per-run.** Vec envs work in slice 2 as a single-host implementation detail; surfacing the vec count as a tunable comes here only if it's trivial. Otherwise punt.
- **MuJoCo license/install management.** Documented as a manual `uv pip install gymnasium[mujoco]` step inside the sidecar venv. Not auto-installed.
- **Hybrid sim+real** (`rl_env_os.rs::HybridConfig`). Future.

## Tests

- Probe Gymnasium fixture: spawn the sidecar against a venv with only `gymnasium`, assert at least `CartPole-v1`, `Pendulum-v1`, `MountainCar-v0`, `LunarLander-v2` come back with valid spaces.
- Custom env happy path: write a `.py` file with a 2-state random env, register, confirm spec round-trips.
- Custom env reject path: file with import-time exception → 400 with traceback excerpt.
- Re-register same name with edited file → new `env_id`, prior linked as parent.
- Refresh idempotency: two `refresh` calls in a row return identical state.

## Definition of done

1. `RLEnvironmentViewer` lists everything Gymnasium ships with (~30 envs in classic + box2d + atari subsets) + any custom registered in this workspace.
2. Custom env registration accepts a workspace-relative `.py`, validates it imports, persists, and is selectable in the training wizard.
3. The training wizard's env dropdown is no longer static.
4. Disclaimer banner drops "Environments" from `covers={[...]}`.
