//! `SequentialExtractor` — single-pass baseline extraction over the pool.
//!
//! One LLM call with the concatenated successful trajectories; the response is
//! parsed into one [`Skill`]. Cheap baseline; use [`super::ParallelExtractor`]
//! for the primary method.

use async_trait::async_trait;

use crate::extract::{parse_skill_response, Extractor, EXTRACT_SYSTEM};
use crate::llm::SkillLlm;
use crate::model::experience::ExperiencePool;
use crate::model::skill::Skill;
use crate::model::trajectory::Role;

/// Concatenate the pool's successful trajectories and ask the LLM for one
/// skill. `max_trajectories` bounds the transcript size (cost control).
pub struct SequentialExtractor {
    pub max_trajectories: usize,
}

impl Default for SequentialExtractor {
    fn default() -> Self {
        Self {
            max_trajectories: 16,
        }
    }
}

#[async_trait]
impl Extractor for SequentialExtractor {
    async fn extract(
        &self,
        pool: &ExperiencePool,
        llm: &dyn SkillLlm,
    ) -> anyhow::Result<Vec<Skill>> {
        let transcript = render_transcript(pool, self.max_trajectories);
        if transcript.is_empty() {
            return Ok(Vec::new());
        }
        let raw = llm.chat(EXTRACT_SYSTEM, &transcript).await?;
        Ok(vec![parse_skill_response(&raw, "sequential")])
    }
}

pub(crate) fn render_transcript(pool: &ExperiencePool, max: usize) -> String {
    let mut out = String::new();
    let mut n = 0usize;
    for t in pool.successes() {
        if n >= max {
            break;
        }
        out.push_str(&format!(
            "--- Run {n} (intent: {}) ---\n",
            t.intent().unwrap_or("")
        ));
        for s in &t.steps {
            if matches!(s.role, Role::System) {
                continue;
            }
            let role = match s.role {
                Role::User => "user",
                Role::Assistant => "assistant",
                Role::Tool => "tool",
                Role::System => "system",
            };
            out.push_str(&format!("[{role}] {}\n", s.content));
        }
        out.push('\n');
        n += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::trajectory::{Outcome, Step, Trajectory};

    fn traj(content: &str) -> Trajectory {
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
    fn render_skips_system_and_numbers_runs() {
        let mut pool = ExperiencePool::new();
        pool.push(traj("done"));
        pool.push(traj("done2"));
        let r = render_transcript(&pool, 16);
        assert!(r.contains("Run 0"));
        assert!(r.contains("Run 1"));
        assert!(!r.contains("[system]"));
    }

    #[test]
    fn render_respects_max() {
        let mut pool = ExperiencePool::new();
        for _ in 0..5 {
            pool.push(traj("x"));
        }
        let r = render_transcript(&pool, 2);
        assert!(r.contains("Run 0"));
        assert!(r.contains("Run 1"));
        assert!(!r.contains("Run 2"));
    }
}
