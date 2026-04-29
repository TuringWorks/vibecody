# Slice 4 — Evaluation Suites

**Status:** Draft · 2026-04-29
**Depends on:** [01-persistence.md](./01-persistence.md), [02-training-executor.md](./02-training-executor.md), [03-environments.md](./03-environments.md)
**Unblocks:** slices 5 (lineage stores eval results), 6 (deployment gates on eval thresholds), 7 (RLHF eval needs this surface)
**Disclaimer banner after this slice:** drops "Evaluation" and "Compare" from `covers={[...]}`. `RLEvalResults` and `RLPolicyComparison` show real numbers.

---

## Goal

Run a trained policy against a defined set of environments, persist statistical results (mean return, success rate, confidence intervals, off-policy estimates), and let users define + version eval suites in YAML.

Reuse the same Python sidecar from slice 2 — eval is a special "run" with no learning.

## Suite definition

YAML, validated against the schema implied by `rl_eval_os.rs::EvalSuite`. Stored in `rl_eval_suites.config_yaml`.

```yaml
# Example: a "robustness" suite for a CartPole policy
suite:
  name: cartpole-robustness-v1
  description: CartPole-v1 with progressively perturbed dynamics
  metrics:
    - mean_return
    - success_rate
    - reward_std
  rollouts_per_env: 100
  seed_strategy: deterministic   # | random | sweep
  envs:
    - env_id: gym:CartPole-v1:gym-0.29
      label: nominal
    - env_id: gym:CartPole-v1:gym-0.29
      label: heavy_pole
      domain_randomization:
        parameters:
          - { name: pole_mass, kind: fixed, value: 0.5 }
    - env_id: gym:CartPole-v1:gym-0.29
      label: high_gravity
      domain_randomization:
        parameters:
          - { name: gravity, kind: fixed, value: 15.0 }
  off_policy_estimators:
    - kind: fqe       # fitted Q evaluation
    - kind: is        # importance sampling
    - kind: dr        # doubly-robust
  quality_gates:
    - { metric: mean_return, env_label: nominal, op: ">=", value: 450.0 }
    - { metric: success_rate, env_label: high_gravity, op: ">=", value: 0.6 }
```

`quality_gates` are evaluated post-rollout; failures are surfaced in the panel and (slice 6) block promotion to deployment.

## Eval rollout

A new `kind='eval'` row in `rl_runs`. The sidecar entry point is `python -m vibe_rl eval --policy <artifact_id> --suite <suite_id> --run-id <id>`. It:

1. Loads the policy from the artifact path (slice 5 will formalize this; slice 4 reads the artifact row directly).
2. For each env in the suite, runs `rollouts_per_env` episodes with no exploration (deterministic policy if available; else mean-action sampling for stochastic).
3. Logs every step into a per-rollout buffer for off-policy estimators.
4. Emits `episode` JSON-Lines exactly like a training run — `RunStore::append_episodes` already handles them.
5. After all rollouts, computes:
   - **Mean return** + 95% CI (bootstrap, 10k resamples)
   - **Success rate** + Wilson interval (when env defines success)
   - **Reward std**
   - **Off-policy estimators** if `off_policy_estimators` is non-empty
6. Writes a row per `(suite, env_label, metric)` into `rl_eval_results`.
7. Evaluates `quality_gates` and writes a `payload.gates` summary into the run row.

## Off-policy evaluators (OPE)

Implemented in `vibe_rl/eval.py`. For slice 4, ship three:

| Estimator | What it answers | When to trust |
|---|---|---|
| **FQE** (Fitted Q-Evaluation) | Expected return under target policy from logged data | Stationary env, plenty of data |
| **IS** (Importance Sampling) | Same, with explicit reweighting | Behavior policy known; high-variance |
| **DR** (Doubly-Robust) | IS + FQE control variate | Best of both when both work |

These are textbook implementations (≤ 200 LOC each) — no separate library dependency required. `rl_eval_os.rs` has the result types (`OPEEstimate { kind, value, ci_low, ci_high, ess, n }`); we serialize Python output into that shape.

Adversarial eval (FGSM, noise injection) and regression detection (t-test, bootstrap-CI difference) are listed in `rl_eval_os.rs` but **deferred to a follow-up slice**. They're plumbing-light but library-heavy and not on the disclaimer-removal critical path.

## HTTP routes

Added to `serve.rs`:

| Method | Path | Body | Returns |
|---|---|---|---|
| `POST` | `/v1/rl/eval/suites` | `EvalSuiteCreateRequest` | `EvalSuite` |
| `GET` | `/v1/rl/eval/suites` | — | `Vec<EvalSuite>` |
| `GET` | `/v1/rl/eval/suites/{suite_id}` | — | `EvalSuite` |
| `PUT` | `/v1/rl/eval/suites/{suite_id}` | `EvalSuiteUpdateRequest` | `EvalSuite` (creates a new version row, old one preserved) |
| `POST` | `/v1/rl/eval/runs` | `{ suite_id, policy_artifact_id }` | `Run` (kind=eval) |
| `GET` | `/v1/rl/eval/results` | `?run_id=` or `?suite_id=&policy=` | `Vec<EvalResultRow>` |
| `POST` | `/v1/rl/eval/compare` | `{ run_ids: [...] }` | `ComparisonReport` |

`/eval/runs` reuses the existing `/v1/rl/runs/{id}/start` lifecycle once the row is created — no second executor pathway.

## Tauri command rewrites

| Command | After slice 4 |
|---|---|
| `rl_list_eval_suites` | `GET /v1/rl/eval/suites` |
| `rl_get_eval_results` | `GET /v1/rl/eval/results?run_id=` |
| `rl_compare_policies` | `POST /v1/rl/eval/compare` |

New: `rl_create_eval_suite`, `rl_update_eval_suite`, `rl_run_eval` — wired in panels.

## Frontend changes

`RLEvalResults.tsx`:
- Replace mock metrics with `rl_get_eval_results({run_id})`.
- Render per-env breakdown (table) + a violin/box plot for return distribution.
- Show OPE estimates with a tooltip explaining each method.
- Quality-gate badges: green pass, red fail, with the threshold inline.

`RLPolicyComparison.tsx`:
- Replace mock comparison with `rl_compare_policies({run_ids})`.
- Per-metric paired comparison: Cohen's d effect size, paired bootstrap CI, "is the difference significant" call.
- Show the same envs (suite intersection) for fair comparison.

A new "Run eval" affordance in the Training Dashboard's run detail view: lets a user attach a finished training run to a suite without leaving the panel.

## Definition of done

1. User defines a suite in YAML, runs it against a finished training run, gets real per-env metrics with CIs.
2. OPE estimates populate when historical trajectories are available.
3. Quality gates surface as pass/fail badges.
4. Comparing two policies on the same suite shows real statistical differences.
5. Disclaimer banner drops "Evaluation" and "Compare".

## Out of scope for slice 4

- Adversarial robustness (FGSM, attack budgets). Deferred.
- Regression detection across runs (auto-flag suite drift). Deferred.
- Generalization metrics (held-out env splits). Deferred but trivial — a suite already encodes this manually.
