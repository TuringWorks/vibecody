//! `trainer::train` — the public entry point: the epoch loop
//! (rollout → reflect/select → propose → validation-gate → meta-update).
//! Deterministic given [`TrainConfig::seed`]. Emits [`TrainingReport`] +
//! the deployable `best_skill.md`. See `notes/skillforge/03` for the full loop.
//!
//! Key fidelity points to upstream SkillOpt:
//! - **Strict** validation gate (`>` not `>=`) — the anti-degradation guarantee.
//! - **Rejected-edit buffer** persists across epochs → no re-proposing a
//!   known-bad edit.
//! - **Textual LR** bounds churn per epoch → stability.
//! - **Held-out split** fixed by `seed` → replayable training.
//! - `evaluate()` ≡ `skilllensai::metrics::target_evolvability` — the two
//!   crates share one measurement.

use skilllensai::llm::SkillLlm;
use skilllensai::metrics::eval::EvalTask;
use skilllensai::model::skill::Skill;

use crate::buffer::{RejectReason, RejectedEditBuffer};
use crate::env::Env;
use crate::gate::{strictly_improves, GateScore};
use crate::propose::propose;
use crate::report::TrainingReport;
use crate::rollout::{rollout, select_failures};

/// Training hyperparameters. All deterministic — no wall-clock / `Math.random`.
#[derive(Debug, Clone)]
pub struct TrainConfig {
    /// Maximum epochs to run.
    pub epochs: usize,
    /// How many train tasks to roll out per epoch.
    pub rollouts_per_epoch: usize,
    /// Per-epoch textual learning rate — max chars of edit churn allowed.
    pub textual_lr: usize,
    /// Held-out fraction in `[0,1]`.
    pub val_split: f32,
    /// Hard cap on the trained skill's token estimate.
    pub max_skill_tokens: usize,
    /// Early-stop after this many consecutive epochs with no val gain.
    pub patience: usize,
    /// How many failing rollouts to hand to `propose` each epoch.
    pub select_k: usize,
    /// Deterministic split + per-epoch sampling seed.
    pub seed: u64,
}

impl Default for TrainConfig {
    fn default() -> Self {
        Self {
            epochs: 8,
            rollouts_per_epoch: 4,
            textual_lr: 512,
            val_split: 0.3,
            max_skill_tokens: 2000,
            patience: 3,
            select_k: 2,
            seed: 0,
        }
    }
}

/// Run the training loop. The seed skill is never mutated; the trained
/// artifact is returned in [`TrainingReport::best_skill_md`].
pub async fn train(
    seed_skill: Skill,
    env: &dyn Env,
    llm: &dyn SkillLlm,
    cfg: &TrainConfig,
) -> anyhow::Result<TrainingReport> {
    let (train_tasks, val_tasks) = env.split(cfg.val_split, cfg.seed);

    let mut skill = seed_skill;
    let baseline = evaluate(&skill, &val_tasks, llm).await?;
    let mut best_val = baseline.score;
    let mut spent_tokens = baseline.spent_tokens;

    let mut rejected = RejectedEditBuffer::new();
    let mut val_curve = Vec::with_capacity(cfg.epochs);
    let mut accepted = 0usize;
    let mut rejected_count = 0usize;
    let mut no_gain = 0usize;
    let mut early_stopped = false;
    let mut epochs_run = 0usize;

    for epoch in 0..cfg.epochs {
        epochs_run = epoch + 1;

        // 1. ROLLOUT — sample train tasks, run the agent with the current skill.
        let sample = sample_tasks(&train_tasks, cfg.rollouts_per_epoch, cfg.seed, epoch);
        let roll = rollout(&skill, &sample, llm).await?;
        spent_tokens += roll.spent_tokens;

        // 2. REFLECT / SELECT — worst-scoring trajectories become targets.
        let targets = select_failures(&roll.trajectories, cfg.select_k);

        // 3. PROPOSE — textual gradient, filtered by the rejected buffer, lr-capped.
        let prop = propose(&skill, &targets, &rejected, cfg.textual_lr, llm).await?;
        spent_tokens += prop.spent_tokens;

        // 4. VALIDATION GATE — apply, re-evaluate on held-out, accept iff strictly better.
        let mut gained_this_epoch = false;
        for edit in prop.edits {
            let cand_body = match edit.apply(&skill.body) {
                Ok(b) => b,
                Err(e) => {
                    rejected.push(edit, RejectReason::ApplyError(e.to_string()), epoch);
                    rejected_count += 1;
                    continue;
                }
            };
            let cand = skill.with_body(&cand_body);
            if cand.token_estimate > cfg.max_skill_tokens {
                rejected.push(
                    edit,
                    RejectReason::ApplyError("exceeds max_skill_tokens".into()),
                    epoch,
                );
                rejected_count += 1;
                continue;
            }
            let GateScore {
                score: cand_val,
                spent_tokens: gate_spent,
            } = evaluate(&cand, &val_tasks, llm).await?;
            spent_tokens += gate_spent;
            if strictly_improves(cand_val, best_val) {
                skill = cand;
                best_val = cand_val;
                accepted += 1;
                gained_this_epoch = true;
            } else {
                rejected.push(edit, RejectReason::NoValGain, epoch);
                rejected_count += 1;
            }
        }

        // 5. META-UPDATE — record the curve, early-stop on patience exhaustion.
        val_curve.push(best_val);
        if gained_this_epoch {
            no_gain = 0;
        } else {
            no_gain += 1;
        }
        if no_gain >= cfg.patience {
            early_stopped = true;
            break;
        }
    }

    Ok(TrainingReport {
        skill_name: skill.name.clone(),
        epochs_run,
        best_val_score: best_val,
        val_curve,
        accepted,
        rejected: rejected_count,
        final_tokens: skill.token_estimate,
        spent_tokens,
        best_skill_md: skill.render(),
        early_stopped,
    })
}

/// Evaluate the held-out score for `skill` (the shared measurement).
async fn evaluate(
    skill: &Skill,
    val_tasks: &[EvalTask],
    llm: &dyn SkillLlm,
) -> anyhow::Result<GateScore> {
    crate::gate::evaluate(skill, val_tasks, llm).await
}

/// Deterministically sample `k` tasks from `pool` for `epoch`. Same
/// `(seed, epoch)` ⇒ same sample, replayable. Returns all of `pool` when
/// `k >= pool.len()`.
fn sample_tasks(pool: &[EvalTask], k: usize, seed: u64, epoch: usize) -> Vec<EvalTask> {
    if k == 0 || pool.is_empty() {
        return Vec::new();
    }
    let mut tasks: Vec<EvalTask> = pool.to_vec();
    crate::env::seeded_shuffle(&mut tasks, seed.wrapping_add(epoch as u64));
    tasks.into_iter().take(k).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::env::StaticEnv;
    use async_trait::async_trait;
    use skilllensai::llm::{LlmDescriptor, SkillLlm};
    use skilllensai::metrics::eval::{EvalTask, Grader};

    /// A mock LLM with a "magic marker" rule: it answers correctly iff the
    /// skill in context contains the marker; on propose calls it always emits
    /// one edit that adds the marker. Fully deterministic — no randomness.
    struct MockLlm {
        marker: String,
        answer: String,
        propose_response: String,
    }

    impl MockLlm {
        fn new(marker: &str, answer: &str) -> Self {
            // The propose response adds the marker as a prepended line.
            let propose_response =
                format!(r#"[{{"op":"add","after_anchor":null,"text":"{marker}"}}]"#);
            Self {
                marker: marker.to_string(),
                answer: answer.to_string(),
                propose_response,
            }
        }
    }

    #[async_trait]
    impl SkillLlm for MockLlm {
        fn descriptor(&self) -> LlmDescriptor {
            LlmDescriptor {
                provider: "mock".into(),
                model: "mock-1".into(),
            }
        }

        async fn chat(&self, system: &str, _user: &str) -> anyhow::Result<String> {
            if system.contains(crate::propose::PROPOSE_MARKER) {
                return Ok(self.propose_response.clone());
            }
            // Rollout / eval — system is the rendered skill.
            if system.contains(self.marker.as_str()) {
                Ok(self.answer.clone())
            } else {
                Ok("I don't know".to_string())
            }
        }
    }

    fn tasks(n: usize, answer: &str) -> Vec<EvalTask> {
        (0..n)
            .map(|i| EvalTask {
                id: format!("t{i}"),
                prompt: "What is 2+2?".into(),
                grader: Grader::Contains(answer.into()),
            })
            .collect()
    }

    #[tokio::test]
    async fn trainer_accepts_improving_edit_then_converges() {
        let marker = "ALWAYS ANSWER: 42";
        let answer = "ANSWER: 42";
        let llm = MockLlm::new(marker, answer);
        let env = StaticEnv::new(tasks(6, answer));
        let seed = Skill::from_str_named(
            "math",
            "---\ntriggers: [\"math\"]\ncategory: math\n---\n# Math skill\n1. do the math\n",
        );

        let cfg = TrainConfig {
            epochs: 3,
            rollouts_per_epoch: 3,
            textual_lr: 100,
            val_split: 0.5,
            max_skill_tokens: 2000,
            patience: 2,
            select_k: 3,
            seed: 1,
        };

        let report = train(seed, &env, &llm, &cfg).await.unwrap();

        // Epoch 0: baseline 0 → add marker → val 1.0 (accepted).
        // Epoch 1: re-propose same edit → no gain (rejected).
        // Epoch 2: propose filtered by rejected buffer → no edits → no gain → early stop.
        assert_eq!(report.epochs_run, 3);
        assert!(report.early_stopped, "should early-stop");
        assert_eq!(report.accepted, 1, "exactly one accepted edit");
        assert_eq!(report.rejected, 1, "one rejected (re-propose, no gain)");
        assert_eq!(report.best_val_score, 1.0);
        assert_eq!(report.val_curve, vec![1.0, 1.0, 1.0]);
        assert!(report.best_skill_md.contains(marker));
        assert!(report.spent_tokens > 0);
    }

    #[tokio::test]
    async fn trainer_is_deterministic_under_fixed_seed() {
        let marker = "HINT: answer foo";
        let answer = "foo";
        let env = StaticEnv::new(tasks(8, answer));
        let cfg = TrainConfig {
            epochs: 4,
            rollouts_per_epoch: 4,
            textual_lr: 200,
            val_split: 0.5,
            max_skill_tokens: 2000,
            patience: 3,
            select_k: 4,
            seed: 7,
        };
        let seed_skill = Skill::from_str_named("s", "---\ntriggers: [\"x\"]\n---\nbody\n");

        let r1 = train(
            seed_skill.clone(),
            &env,
            &MockLlm::new(marker, answer),
            &cfg,
        )
        .await
        .unwrap();
        let r2 = train(seed_skill, &env, &MockLlm::new(marker, answer), &cfg)
            .await
            .unwrap();
        assert_eq!(r1.val_curve, r2.val_curve);
        assert_eq!(r1.accepted, r2.accepted);
        assert_eq!(r1.rejected, r2.rejected);
        assert_eq!(r1.best_skill_md, r2.best_skill_md);
    }

    #[tokio::test]
    async fn trainer_rejects_degrading_edit() {
        // A mock that proposes an edit, but the "answer" only appears when the
        // marker is ABSENT — so adding the marker *degrades* val score. The
        // strict gate must reject it (no improvement) and never accept.
        struct DegradingMock;
        #[async_trait]
        impl SkillLlm for DegradingMock {
            fn descriptor(&self) -> LlmDescriptor {
                LlmDescriptor {
                    provider: "mock".into(),
                    model: "deg".into(),
                }
            }
            async fn chat(&self, system: &str, _user: &str) -> anyhow::Result<String> {
                if system.contains(crate::propose::PROPOSE_MARKER) {
                    return Ok(
                        r#"[{"op":"add","after_anchor":null,"text":"BAD-HINT"}]"#.to_string()
                    );
                }
                // Correct answer when BAD-HINT is absent; wrong when present.
                if system.contains("BAD-HINT") {
                    Ok("wrong".to_string())
                } else {
                    Ok("correct".to_string())
                }
            }
        }
        let env = StaticEnv::new(
            (0..4)
                .map(|i| EvalTask {
                    id: format!("t{i}"),
                    prompt: "q".into(),
                    grader: Grader::Contains("correct".into()),
                })
                .collect(),
        );
        let seed_skill = Skill::from_str_named("s", "---\ntriggers: [\"x\"]\n---\nbody\n");
        let cfg = TrainConfig {
            epochs: 2,
            rollouts_per_epoch: 2,
            textual_lr: 100,
            val_split: 0.5,
            max_skill_tokens: 2000,
            patience: 5, // don't early-stop; just assert no acceptance
            select_k: 2,
            seed: 3,
        };
        let r = train(seed_skill, &env, &DegradingMock, &cfg).await.unwrap();
        assert_eq!(r.accepted, 0, "degrading edit must not be accepted");
        assert!(r.rejected >= 1);
        assert_eq!(r.best_val_score, 1.0, "baseline already perfect; unchanged");
    }
}
