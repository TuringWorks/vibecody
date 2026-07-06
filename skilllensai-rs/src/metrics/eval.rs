//! LLM-backed evaluation primitives (feature = `llm`).
//!
//! These are the **shared measurement** both crates use: `skilloptai-rs`'s
//! validation gate calls [`target_evolvability`], and [`extraction_efficacy`]
//! scores how well a candidate skill captures a pool's successful behaviour.
//!
//! Design note: `EvalTask` / `Grader` live here (in the lower crate) rather
//! than in `skilloptai-rs` so the shared measurement can call them without a
//! circular dependency. `skilloptai-rs` defines the training-specific `Env`
//! trait (tasks + held-out split) on top of these.

use serde::{Deserialize, Serialize};

use crate::llm::SkillLlm;
use crate::model::experience::ExperiencePool;
use crate::model::skill::Skill;
use crate::model::trajectory::Trajectory;

/// A single gradeable evaluation item. The target model is prompted with
/// `prompt`; its response is scored by [`Grader`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvalTask {
    pub id: String,
    pub prompt: String,
    pub grader: Grader,
}

/// How a response is scored to a `[0.0, 1.0]` scalar.
///
/// Adjacently tagged (`{"kind":"contains","value":"..."}`) so newtype variants
/// round-trip cleanly — internally-tagged enums can't carry string newtypes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum Grader {
    /// `response.trim() == expected` ⇒ 1.0, else 0.0.
    Exact(String),
    /// `response.contains(needle)` ⇒ 1.0, else 0.0.
    Contains(String),
    /// `regex` matches somewhere in the response ⇒ 1.0, else 0.0.
    Regex(String),
    /// The agent exited its tool loop cleanly ⇒ 1.0 (the env/daemon is the
    /// source of truth on whether the run terminated; we treat a recorded
    /// response as a clean exit).
    ToolExit,
    /// A rubric the LLM judges the response against, returning `[0,1]`.
    LlmJudge(String),
}

impl Grader {
    /// Score `response` against this grader. `LlmJudge` invokes `llm`; the
    /// rest are synchronous and never call the model.
    pub async fn grade(&self, response: &str, llm: &dyn SkillLlm) -> anyhow::Result<f32> {
        Ok(match self {
            Grader::Exact(expected) => (response.trim() == expected) as u32 as f32,
            Grader::Contains(needle) => response.contains(needle.as_str()) as u32 as f32,
            Grader::Regex(pattern) => {
                let re = regex::Regex::new(pattern)?;
                re.is_match(response) as u32 as f32
            }
            Grader::ToolExit => 1.0,
            Grader::LlmJudge(rubric) => {
                let system = "You are a strict grader. Score the response against the rubric. \
                              Reply with a single number in [0,1] and nothing else.";
                let user = format!("Rubric:\n{rubric}\n\nResponse:\n{response}");
                let raw = llm.chat(system, &user).await?;
                parse_unit_scalar(&raw)
            }
        })
    }
}

/// How much a *target* model improves when given `skill` in context, measured
/// as the mean score over `tasks`. This is the held-out validation signal the
/// optimizer's strict gate compares against (`>` not `>=`).
///
/// Sequential by construction — the same `tasks` ordering yields the same
/// score across runs (replay-friendly). Empty `tasks` ⇒ `0.0`.
pub async fn target_evolvability(
    skill: &Skill,
    tasks: &[EvalTask],
    llm: &dyn SkillLlm,
) -> anyhow::Result<f32> {
    if tasks.is_empty() {
        return Ok(0.0);
    }
    let system = skill.render();
    let mut sum = 0.0f32;
    for t in tasks {
        let resp = llm.chat(&system, &t.prompt).await?;
        sum += t.grader.grade(&resp, llm).await?;
    }
    Ok(sum / tasks.len() as f32)
}

/// Same as [`target_evolvability`] but with **no** skill in context — the
/// baseline the evolvability delta is measured against.
pub async fn baseline_score(tasks: &[EvalTask], llm: &dyn SkillLlm) -> anyhow::Result<f32> {
    if tasks.is_empty() {
        return Ok(0.0);
    }
    let system = "You are a helpful agent.";
    let mut sum = 0.0f32;
    for t in tasks {
        let resp = llm.chat(system, &t.prompt).await?;
        sum += t.grader.grade(&resp, llm).await?;
    }
    Ok(sum / tasks.len() as f32)
}

/// How completely `skill` captures the successful behaviours in `pool`
/// (LLM-judge, `[0,1]`). Samples up to `max_sample` successes to bound cost.
pub async fn extraction_efficacy(
    skill: &Skill,
    pool: &ExperiencePool,
    llm: &dyn SkillLlm,
    max_sample: usize,
) -> anyhow::Result<f32> {
    let successes: Vec<&Trajectory> = pool.successes().take(max_sample).collect();
    if successes.is_empty() {
        return Ok(0.0);
    }
    let mut transcripts = String::new();
    for (i, t) in successes.iter().enumerate() {
        transcripts.push_str(&format!(
            "--- Run {i} (intent: {}) ---\n{}\n\n",
            t.intent().unwrap_or(""),
            render_steps(t)
        ));
    }
    let system = "You are a skill-evaluation judge. Rate how completely the given skill \
                  captures the successful behaviours shown below. Reply with a single \
                  number in [0,1] and nothing else.";
    let user = format!(
        "Skill:\n{}\n\nObserved successful behaviours:\n{transcripts}",
        skill.render()
    );
    let raw = llm.chat(system, &user).await?;
    Ok(parse_unit_scalar(&raw))
}

fn render_steps(t: &Trajectory) -> String {
    let mut out = String::new();
    for s in &t.steps {
        let role = match s.role {
            crate::model::trajectory::Role::System => "system",
            crate::model::trajectory::Role::User => "user",
            crate::model::trajectory::Role::Assistant => "assistant",
            crate::model::trajectory::Role::Tool => "tool",
        };
        out.push_str(&format!("[{role}] {}\n", s.content));
    }
    out
}

/// Parse a `[0,1]` scalar from a model reply, clamped and defaulted to `0.0`.
fn parse_unit_scalar(raw: &str) -> f32 {
    let trimmed = raw.trim();
    // Tolerate a leading sentinel like "0.8" or "Score: 0.8".
    let candidate = trimmed
        .split(|c: char| !c.is_ascii_digit() && c != '.' && c != '-')
        .find(|s| !s.is_empty() && s.parse::<f32>().is_ok())
        .and_then(|s| s.parse::<f32>().ok());
    candidate.unwrap_or(0.0).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::NullLlm;
    use crate::model::experience::ExperiencePool;
    use crate::model::trajectory::{Outcome, Role, Step, Trajectory};

    #[tokio::test]
    async fn grader_variants_sync_paths() {
        let llm = NullLlm;
        assert_eq!(
            Grader::Exact("ok".into())
                .grade("  ok  ", &llm)
                .await
                .unwrap(),
            1.0
        );
        assert_eq!(
            Grader::Exact("ok".into()).grade("no", &llm).await.unwrap(),
            0.0
        );
        assert_eq!(
            Grader::Contains("ANSWER".into())
                .grade("the ANSWER is 42", &llm)
                .await
                .unwrap(),
            1.0
        );
        assert_eq!(
            Grader::Contains("nope".into())
                .grade("the ANSWER is 42", &llm)
                .await
                .unwrap(),
            0.0
        );
        assert_eq!(
            Grader::Regex(r"ANSWER:\s*\d+".into())
                .grade("ANSWER: 42", &llm)
                .await
                .unwrap(),
            1.0
        );
        assert_eq!(
            Grader::Regex(r"ANSWER:\s*\d+".into())
                .grade("no match", &llm)
                .await
                .unwrap(),
            0.0
        );
        assert_eq!(Grader::ToolExit.grade("anything", &llm).await.unwrap(), 1.0);
    }

    #[tokio::test]
    async fn target_evolvability_empty_is_zero() {
        let llm = NullLlm;
        let s = Skill::from_str_named("t", "---\ntriggers: [\"x\"]\n---\nbody");
        assert_eq!(target_evolvability(&s, &[], &llm).await.unwrap(), 0.0);
    }

    #[tokio::test]
    async fn parse_scalar_tolerant() {
        assert_eq!(parse_unit_scalar("0.8"), 0.8);
        assert_eq!(parse_unit_scalar("Score: 0.5\n"), 0.5);
        assert_eq!(parse_unit_scalar("garbage"), 0.0);
        assert_eq!(parse_unit_scalar("1.5"), 1.0);
        assert_eq!(parse_unit_scalar("-0.2"), 0.0);
    }

    #[tokio::test]
    async fn extraction_efficacy_empty_pool_is_zero() {
        let llm = NullLlm;
        let s = Skill::from_str_named("t", "---\ntriggers: [\"x\"]\n---\nbody");
        let pool = ExperiencePool::new();
        assert_eq!(extraction_efficacy(&s, &pool, &llm, 8).await.unwrap(), 0.0);
    }

    fn traj_success(content: &str) -> Trajectory {
        Trajectory {
            id: "r".into(),
            task_id: "t".into(),
            steps: vec![
                Step {
                    role: Role::User,
                    content: "do thing".into(),
                    tool: None,
                },
                Step {
                    role: Role::Assistant,
                    content: content.into(),
                    tool: None,
                },
            ],
            outcome: Outcome::Success,
            score: None,
            meta: serde_json::Value::Null,
        }
    }

    #[test]
    fn render_steps_includes_roles() {
        let t = traj_success("done");
        assert!(render_steps(&t).contains("[user]"));
        assert!(render_steps(&t).contains("[assistant]"));
    }
}
