//! `ParallelExtractor` — the primary/recommended extraction method.
//!
//! Two phases:
//! 1. **Per-trajectory mode extraction** — one LLM call per successful
//!    trajectory, fanned out under a semaphore bound (`concurrency`). Each
//!    yields a candidate "mode" skill.
//! 2. **Hierarchical pairwise merge** — reduce the mode skills to a single
//!    skill via ceil(log2(N)) merge rounds; each merge is one LLM call that
//!    deduplicates two skills into one, guided by extraction efficacy
//!    (lower-value steps dropped).
//!
//! Sequential within a phase for replayability; only the per-trajectory
//! fan-out is concurrent (semaphore-bounded, like kodegraph's build fan-out).

use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::Semaphore;

use crate::extract::{parse_skill_response, Extractor, EXTRACT_SYSTEM};
use crate::llm::SkillLlm;
use crate::model::experience::ExperiencePool;
use crate::model::skill::Skill;
use crate::model::trajectory::Role;

/// Per-trajectory mode extraction + hierarchical merge.
pub struct ParallelExtractor {
    /// Max successful trajectories to mine (cost control).
    pub max_trajectories: usize,
    /// Max concurrent per-trajectory LLM calls.
    pub concurrency: usize,
}

impl Default for ParallelExtractor {
    fn default() -> Self {
        Self {
            max_trajectories: 16,
            concurrency: 4,
        }
    }
}

const MERGE_SYSTEM: &str = "\
You merge two candidate agent-skill documents into ONE deduplicated skill. Keep the \
highest-value steps, drop redundancy, and union the triggers/tools. Output ONLY the \
merged skill markdown (YAML frontmatter `triggers`/`tools_allowed`/`category` + a \
numbered-step body). No commentary.";

#[async_trait]
impl Extractor for ParallelExtractor {
    async fn extract(
        &self,
        pool: &ExperiencePool,
        llm: &dyn SkillLlm,
    ) -> anyhow::Result<Vec<Skill>> {
        // 1. Per-trajectory mode extraction, semaphore-bounded.
        let targets: Vec<_> = pool.successes().take(self.max_trajectories).collect();
        if targets.is_empty() {
            return Ok(Vec::new());
        }
        let sem = Arc::new(Semaphore::new(self.concurrency));
        let mode_futs = targets
            .iter()
            .enumerate()
            .map(|(i, t)| {
                let sem = sem.clone();
                async move {
                    let _permit = sem.acquire().await.map_err(|e| anyhow::anyhow!("{e}"))?;
                    let transcript = render_single(t);
                    let raw = llm.chat(EXTRACT_SYSTEM, &transcript).await?;
                    Ok::<Skill, anyhow::Error>(parse_skill_response(&raw, &format!("mode-{i}")))
                }
            })
            .collect::<Vec<_>>();
        let modes: Vec<Skill> = futures::future::join_all(mode_futs)
            .await
            .into_iter()
            .collect::<Result<_, _>>()?;

        // 2. Hierarchical pairwise merge.
        let mut current = modes;
        while current.len() > 1 {
            let mut next = Vec::with_capacity(current.len().div_ceil(2));
            for pair in current.chunks(2) {
                if pair.len() == 2 {
                    next.push(merge_skills(&pair[0], &pair[1], llm).await?);
                } else {
                    next.push(pair[0].clone());
                }
            }
            current = next;
        }
        Ok(current)
    }
}

async fn merge_skills(a: &Skill, b: &Skill, llm: &dyn SkillLlm) -> anyhow::Result<Skill> {
    let user = format!(
        "--- Skill A ---\n{}\n\n--- Skill B ---\n{}",
        a.render(),
        b.render()
    );
    let raw = llm.chat(MERGE_SYSTEM, &user).await?;
    let name = format!("{}-{}", a.name, b.name);
    Ok(parse_skill_response(&raw, &name))
}

fn render_single(t: &crate::model::trajectory::Trajectory) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "--- Run (intent: {}) ---\n",
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
    fn render_single_skips_system() {
        let t = traj("done");
        let r = render_single(&t);
        assert!(r.contains("[user]"));
        assert!(r.contains("[assistant]"));
        assert!(!r.contains("[system]"));
    }
}
