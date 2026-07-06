//! `rollout` — run the agent on sampled tasks with the current skill, producing
//! [`skilllensai::Trajectory`] values for the reflect/propose step.
//!
//! A rollout is one LLM call per task: the skill document is the system prompt,
//! the task prompt is the user message, the response is graded by the task's
//! [`Grader`]. Sequential for replayability — the same `tasks` ordering yields
//! the same trajectories across runs.

use skilllensai::llm::SkillLlm;
use skilllensai::metrics::eval::EvalTask;
use skilllensai::model::skill::Skill;
use skilllensai::model::trajectory::{Outcome, Role, Step, Trajectory};

use crate::report::approx_tokens;

/// The result of a rollout pass.
pub struct RolloutResult {
    pub trajectories: Vec<Trajectory>,
    pub spent_tokens: usize,
}

/// Run the agent on `tasks` with `skill` in context. Each task becomes one
/// [`Trajectory`] graded by its [`Grader`]. `spent_tokens` is the approximate
/// LLM token cost of the whole pass.
pub async fn rollout(
    skill: &Skill,
    tasks: &[EvalTask],
    llm: &dyn SkillLlm,
) -> anyhow::Result<RolloutResult> {
    let system = skill.render();
    let descriptor = llm.descriptor();
    let mut trajectories = Vec::with_capacity(tasks.len());
    let mut spent = 0usize;

    for (i, t) in tasks.iter().enumerate() {
        let response = llm.chat(&system, &t.prompt).await?;
        spent += approx_tokens(&system, &t.prompt, &response);
        let score = t.grader.grade(&response, llm).await?;
        let outcome = if score >= 1.0 {
            Outcome::Success
        } else if score <= 0.0 {
            Outcome::Failure
        } else {
            Outcome::Partial { score }
        };
        trajectories.push(Trajectory {
            id: format!("{}-{i}", t.id),
            task_id: t.id.clone(),
            steps: vec![
                Step {
                    role: Role::User,
                    content: t.prompt.clone(),
                    tool: None,
                },
                Step {
                    role: Role::Assistant,
                    content: response,
                    tool: None,
                },
            ],
            outcome,
            score: Some(score),
            meta: serde_json::json!({
                "provider": descriptor.provider,
                "model": descriptor.model,
                "grader": format!("{:?}", t.grader),
            }),
        });
    }

    Ok(RolloutResult {
        trajectories,
        spent_tokens: spent,
    })
}

/// Select the lowest-scoring `k` trajectories — the improvement targets the
/// `propose` step tries to fix. Sorted ascending by [`Trajectory::score_value`]
/// and truncated. Empty input ⇒ empty output.
pub fn select_failures(trajectories: &[Trajectory], k: usize) -> Vec<Trajectory> {
    if trajectories.is_empty() || k == 0 {
        return Vec::new();
    }
    let mut sorted: Vec<&Trajectory> = trajectories.iter().collect();
    sorted.sort_by(|a, b| {
        a.score_value()
            .partial_cmp(&b.score_value())
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    sorted.into_iter().take(k).cloned().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn traj(id: &str, score: f32) -> Trajectory {
        Trajectory {
            id: id.into(),
            task_id: id.into(),
            steps: Vec::new(),
            outcome: if score >= 1.0 {
                Outcome::Success
            } else if score <= 0.0 {
                Outcome::Failure
            } else {
                Outcome::Partial { score }
            },
            score: Some(score),
            meta: serde_json::Value::Null,
        }
    }

    #[test]
    fn select_failures_picks_lowest_k() {
        let trajs = vec![
            traj("a", 0.9),
            traj("b", 0.1),
            traj("c", 0.5),
            traj("d", 0.2),
        ];
        let worst = select_failures(&trajs, 2);
        assert_eq!(worst.len(), 2);
        assert_eq!(worst[0].id, "b"); // 0.1
        assert_eq!(worst[1].id, "d"); // 0.2
    }

    #[test]
    fn select_failures_empty_or_zero_k() {
        let trajs = vec![traj("a", 0.5)];
        assert!(select_failures(&trajs, 0).is_empty());
        assert!(select_failures(&[], 3).is_empty());
    }
}
