# Slice 1 — Persistence + Run Lifecycle

**Status:** Draft · 2026-04-29
**Depends on:** none (this is the foundation slice)
**Unblocks:** slices 2–7
**Disclaimer banner after this slice:** still covers all 10 panels; the *list* of runs in `RLTrainingDashboard` becomes durable, but no real metrics yet.

---

## Goal

Replace the in-memory mock state at `vibeui/src-tauri/src/commands.rs:41692-41699` with durable, encrypted storage in `WorkspaceStore` (`<workspace>/.vibecli/workspace.db`). Runs created in the UI must survive process restart. No real training yet — that's slice 2 — but the schema, the `RunLifecycle` state machine, and the daemon HTTP routes go in here so slice 2 only has to plug in the executor.

## Where state lives

Per AGENTS.md storage rules:

- **API keys** (sidecar OpenAI keys for RLHF, etc.) → `ProfileStore` (`~/.vibecli/profile_settings.db`)
- **All RL run state, metrics, artifacts metadata, env definitions, eval suites** → `WorkspaceStore` (`<workspace>/.vibecli/workspace.db`)
- **Artifact files** (model weights, ONNX, checkpoints) → `<workspace>/.vibecli/rl-artifacts/<run_id>/...`
- **Sidecar logs** → `<workspace>/.vibecli/rl-logs/<run_id>.jsonl` (rotated, gzipped after run completes)

Workspace DB is already encrypted (per AGENTS.md). Keys never leak to the artifact tree; if a user moves a workspace, both DB and artifacts move together.

## Schema

DDL goes in a new migration file `vibecli/vibecli-cli/migrations/workspace/NNNN_rl_os.sql` (matching the pattern of existing workspace migrations). All tables namespaced `rl_*` to avoid collisions.

```sql
-- Runs: one row per training/eval/distill execution
CREATE TABLE rl_runs (
    run_id           TEXT    PRIMARY KEY,        -- ULID
    name             TEXT    NOT NULL,
    kind             TEXT    NOT NULL,           -- 'train' | 'eval' | 'distill' | 'rlhf'
    status           TEXT    NOT NULL,           -- see RunLifecycle below
    algorithm        TEXT    NOT NULL,           -- 'PPO' | 'SAC' | ...
    environment_id   TEXT    NOT NULL,           -- FK rl_environments.env_id
    parent_run_id    TEXT    REFERENCES rl_runs(run_id),  -- distill/resume parent
    config_yaml      TEXT    NOT NULL,           -- full TrainingConfig serialized
    seed             INTEGER NOT NULL,
    sidecar_version  TEXT    NOT NULL,           -- vibe-rl-py version pin
    created_at       INTEGER NOT NULL,           -- unix ms
    started_at       INTEGER,
    finished_at      INTEGER,
    total_timesteps  INTEGER NOT NULL,
    elapsed_steps    INTEGER NOT NULL DEFAULT 0,
    last_reward_mean REAL,
    error_message    TEXT,
    workspace_path   TEXT    NOT NULL            -- so artifact paths are reconstructable
);
CREATE INDEX rl_runs_status_idx     ON rl_runs(status, created_at DESC);
CREATE INDEX rl_runs_kind_idx       ON rl_runs(kind, created_at DESC);
CREATE INDEX rl_runs_parent_idx     ON rl_runs(parent_run_id);

-- Episode-level metrics (one row per training episode)
CREATE TABLE rl_episodes (
    run_id       TEXT    NOT NULL REFERENCES rl_runs(run_id) ON DELETE CASCADE,
    episode_idx  INTEGER NOT NULL,
    timestep     INTEGER NOT NULL,
    reward_sum   REAL    NOT NULL,
    length       INTEGER NOT NULL,
    success      INTEGER,                        -- nullable boolean (env-dependent)
    duration_ms  INTEGER NOT NULL,
    PRIMARY KEY (run_id, episode_idx)
);

-- Time-series of training-loop metrics (one row per logging tick, e.g. every N updates)
CREATE TABLE rl_metrics (
    run_id      TEXT    NOT NULL REFERENCES rl_runs(run_id) ON DELETE CASCADE,
    tick        INTEGER NOT NULL,                -- monotonic counter
    timestep    INTEGER NOT NULL,
    wall_time   INTEGER NOT NULL,                -- unix ms
    payload     TEXT    NOT NULL,                -- JSON: {policy_loss, value_loss, entropy, lr, kl, ...}
    PRIMARY KEY (run_id, tick)
);

-- Checkpoints + final artifacts (file pointers — files live under .vibecli/rl-artifacts/)
CREATE TABLE rl_artifacts (
    artifact_id   TEXT PRIMARY KEY,              -- ULID
    run_id        TEXT NOT NULL REFERENCES rl_runs(run_id) ON DELETE CASCADE,
    kind          TEXT NOT NULL,                 -- 'checkpoint' | 'final' | 'onnx' | 'replay_buffer' | 'model_card'
    timestep      INTEGER,                       -- nullable (final has none)
    rel_path      TEXT NOT NULL,                 -- path relative to workspace, e.g. '.vibecli/rl-artifacts/<run_id>/ckpt-100k.pt'
    sha256        TEXT NOT NULL,
    size_bytes    INTEGER NOT NULL,
    created_at    INTEGER NOT NULL,
    metadata_json TEXT                           -- arbitrary kv (framework, dtype, ...)
);
CREATE INDEX rl_artifacts_run_idx ON rl_artifacts(run_id, kind, timestep);

-- Environments registered in this workspace
CREATE TABLE rl_environments (
    env_id           TEXT PRIMARY KEY,            -- '<source>:<name>:<version>' e.g. 'gym:CartPole-v1:gym-0.29'
    name             TEXT NOT NULL,
    version          TEXT NOT NULL,
    source           TEXT NOT NULL,               -- 'gymnasium' | 'pettingzoo' | 'custom_python' | 'custom_dsl'
    spec_json        TEXT NOT NULL,               -- observation_space, action_space, reward_range, max_episode_steps
    entry_point      TEXT,                        -- module:class for python sources
    file_path        TEXT,                        -- for custom_python registered from a workspace file
    parent_env_id    TEXT REFERENCES rl_environments(env_id),  -- version DAG
    created_at       INTEGER NOT NULL,
    UNIQUE(name, version, source)
);

-- Eval suites (definitions only — runs go in rl_runs with kind='eval')
CREATE TABLE rl_eval_suites (
    suite_id     TEXT PRIMARY KEY,
    name         TEXT NOT NULL UNIQUE,
    description  TEXT,
    config_yaml  TEXT NOT NULL,
    created_at   INTEGER NOT NULL
);

-- Per-suite results, one row per (eval_run, suite, episode group)
CREATE TABLE rl_eval_results (
    run_id        TEXT NOT NULL REFERENCES rl_runs(run_id) ON DELETE CASCADE,
    suite_id      TEXT NOT NULL REFERENCES rl_eval_suites(suite_id),
    metric_name   TEXT NOT NULL,                 -- 'mean_return' | 'success_rate' | 'fqe_estimate' | ...
    value         REAL NOT NULL,
    ci_low        REAL,
    ci_high       REAL,
    n_episodes    INTEGER NOT NULL,
    extra_json    TEXT,
    PRIMARY KEY (run_id, suite_id, metric_name)
);

-- Deployments (slice 6 — table created here so the FK is stable)
CREATE TABLE rl_deployments (
    deployment_id     TEXT PRIMARY KEY,
    name              TEXT NOT NULL,
    artifact_id       TEXT NOT NULL REFERENCES rl_artifacts(artifact_id),
    runtime           TEXT NOT NULL,             -- 'onnx' | 'python' | 'native_candle'
    status            TEXT NOT NULL,             -- 'staging' | 'canary' | 'production' | 'rolled_back' | 'stopped'
    traffic_pct       REAL NOT NULL DEFAULT 0.0,
    config_json       TEXT NOT NULL,
    created_at        INTEGER NOT NULL,
    promoted_at       INTEGER,
    rolled_back_at    INTEGER,
    rollback_reason   TEXT
);

-- Lineage edges that aren't already implied by parent_run_id (e.g. multi-teacher distill)
CREATE TABLE rl_lineage_edges (
    child_run_id   TEXT NOT NULL REFERENCES rl_runs(run_id) ON DELETE CASCADE,
    parent_run_id  TEXT NOT NULL REFERENCES rl_runs(run_id),
    edge_kind      TEXT NOT NULL,                -- 'distill_teacher' | 'rlhf_base' | 'merge_source'
    weight         REAL,                          -- e.g. teacher mix weight
    PRIMARY KEY (child_run_id, parent_run_id, edge_kind)
);
```

## RunLifecycle state machine

```text
created ──start──▶ queued ──executor_pickup──▶ running ─┬─ checkpoint ──▶ running
                       │                                │
                       │                                ├─ stop_request ──▶ stopping ──▶ stopped
                       │                                │
                       │                                ├─ error ──▶ failed
                       │                                │
                       │                                └─ done ──▶ succeeded
                       │
                       └─ cancel ──▶ cancelled
```

State enum lives in a new module `vibecli/vibecli-cli/src/rl_runs.rs` (alongside the orphaned `rl_*_os.rs` modules — this one is the wiring module). Transitions are validated server-side; the UI never sets `status` directly, only sends intent (`start`, `stop`, `cancel`).

```rust
// vibecli/vibecli-cli/src/rl_runs.rs
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum RunStatus {
    Created, Queued, Running, Stopping, Stopped, Cancelled, Succeeded, Failed,
}

pub struct RunStore { /* wraps WorkspaceStore handle */ }

impl RunStore {
    pub fn create(&self, req: CreateRunRequest) -> Result<Run> { ... }
    pub fn get(&self, run_id: &str) -> Result<Option<Run>> { ... }
    pub fn list(&self, filter: RunFilter) -> Result<Vec<Run>> { ... }
    pub fn transition(&self, run_id: &str, to: RunStatus) -> Result<Run> { ... }
    pub fn append_metrics(&self, run_id: &str, batch: &[MetricTick]) -> Result<()> { ... }
    pub fn append_episodes(&self, run_id: &str, batch: &[EpisodeRow]) -> Result<()> { ... }
    pub fn record_artifact(&self, run_id: &str, art: ArtifactRecord) -> Result<Artifact> { ... }
}
```

The `RunStore` is the only thing the executor (slice 2) writes to. It's the only thing the HTTP routes read from. No state in `OnceLock<Mutex<...>>`.

## Daemon HTTP routes

Added to `vibecli/vibecli-cli/src/serve.rs`:

| Method | Path | Body | Returns |
|---|---|---|---|
| `POST` | `/v1/rl/runs` | `CreateRunRequest` | `Run` |
| `GET` | `/v1/rl/runs` | — | `Vec<Run>` (filtered via query) |
| `GET` | `/v1/rl/runs/{run_id}` | — | `Run` |
| `POST` | `/v1/rl/runs/{run_id}/start` | — | `Run` |
| `POST` | `/v1/rl/runs/{run_id}/stop` | — | `Run` |
| `POST` | `/v1/rl/runs/{run_id}/cancel` | — | `Run` |
| `DELETE` | `/v1/rl/runs/{run_id}` | — | `204` (only if status ∈ {Cancelled, Failed, Succeeded, Stopped}) |
| `GET` | `/v1/rl/runs/{run_id}/metrics` | — | `MetricsSnapshot` (batch) |
| `GET` | `/v1/rl/runs/{run_id}/metrics/stream` | — | SSE stream of `MetricTick` |
| `GET` | `/v1/rl/runs/{run_id}/episodes` | `?since=N&limit=K` | `Vec<EpisodeRow>` |
| `GET` | `/v1/rl/runs/{run_id}/artifacts` | — | `Vec<Artifact>` |

Auth uses the existing daemon bearer-token mechanism (per CLAUDE.md). RL routes are not exposed unauthenticated.

## Tauri command rewrites

The 5 commands rewritten in this slice:

| Command | Today | After slice 1 |
|---|---|---|
| `rl_create_training_run` | Pushes JSON to `OnceLock<Vec>` | `POST /v1/rl/runs` → returns `Run` row with status=`Created` |
| `rl_list_training_runs` | Returns `OnceLock<Vec>` | `GET /v1/rl/runs?kind=train` |
| `rl_get_training_metrics` | Synthesizes deterministic noise | `GET /v1/rl/runs/{id}/metrics` — returns persisted batch (empty until slice 2 wires the executor; UI shows "Run created — start to see metrics") |
| `rl_start_training` | Mutates in-memory status | `POST /v1/rl/runs/{id}/start` (transitions to `Queued`; until slice 2, an executor sentinel job that just records a "no-executor" failure and transitions to `Failed`) |
| `rl_stop_training` | Mutates in-memory status | `POST /v1/rl/runs/{id}/stop` |

The other 15 commands stay on mocks until their respective slices. Each still-mocked command gets a `#[deprecated_mock = "slice N"]` attribute (custom helper macro) so we can grep for the remaining mocks.

## Frontend changes

`RLTrainingDashboard.tsx` (32.8 KB) needs three small changes:

1. Empty-state when a run has no episodes yet ("Start the run to see metrics").
2. Loading state distinct from empty state (skeleton rows while `rl_get_training_metrics` is in-flight).
3. Error state if the daemon returns 5xx.

`RLOSComposite.tsx:19` keeps the disclaimer; this slice doesn't change which panels are illustrative.

## Tests

Mandatory before merge:

- `cargo test -p vibecli rl_runs` — schema migration applies cleanly to a fresh DB and a DB with prior migrations.
- `cargo test -p vibecli rl_runs::lifecycle` — every illegal transition is rejected; every legal transition persists.
- `cargo test -p vibecli rl_runs::concurrency` — two parallel `transition` calls don't both succeed (row lock).
- `cargo test -p vibecli rl_runs::artifact_path` — artifact paths sit inside the workspace and are correctly relativized.
- One BDD harness in `vibecli/vibecli-cli/tests/` exercising the full create→start→stop→delete flow over HTTP.

Use `WorkspaceStore::open_with(path, key)` per CLAUDE.md test guidance — never the production DB.

## Out of scope for slice 1

- Any real metric production. Slice 2 owns this.
- Any artifact files actually being written. Slice 2 owns this.
- Eval-suite execution. Slice 4.
- Lineage UI. Slice 5.

## Definition of done

1. Migration applies on a fresh workspace and on an existing workspace with no RL data.
2. All 5 listed Tauri commands hit the daemon, persist, and survive a daemon restart.
3. Creating a run, restarting the daemon, and listing runs returns the run with status=`Created`.
4. The 15 still-mocked commands compile-time mark themselves so we can grep them down.
5. `RLTrainingDashboard` shows a real empty state with a "Start" button that issues `POST /start` and gets a real 200 (which then transitions to `Failed` because no executor — that's slice 2's problem).
