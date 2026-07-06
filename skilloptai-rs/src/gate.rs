//! `gate` — the held-out validation gate.
//!
//! A candidate edit is accepted **only if** the held-out score **strictly**
//! improves (`>` not `>=`); otherwise the edit goes to the rejected-edit
//! buffer. This strictness is the anti-degradation guarantee and what makes
//! runs reproducible. [`evaluate`] is the trainer's name for
//! `skilllensai::metrics::target_evolvability` — the two crates share one
//! measurement — reimplemented here so the trainer can account for the tokens
//! the gate spends.

use skilllensai::llm::SkillLlm;
use skilllensai::metrics::eval::EvalTask;
use skilllensai::model::skill::Skill;

use crate::report::approx_tokens;

/// A held-out evaluation result.
pub struct GateScore {
    pub score: f32,
    pub spent_tokens: usize,
}

/// Run the agent on the held-out `val_tasks` with `skill` in context; return
/// the mean graded score in `[0,1]` plus the tokens spent. Empty val set ⇒
/// score `0.0`. Sequential for replayability.
pub async fn evaluate(
    skill: &Skill,
    val_tasks: &[EvalTask],
    llm: &dyn SkillLlm,
) -> anyhow::Result<GateScore> {
    if val_tasks.is_empty() {
        return Ok(GateScore {
            score: 0.0,
            spent_tokens: 0,
        });
    }
    let system = skill.render();
    let mut sum = 0.0f32;
    let mut spent = 0usize;
    for t in val_tasks {
        let resp = llm.chat(&system, &t.prompt).await?;
        spent += approx_tokens(&system, &t.prompt, &resp);
        sum += t.grader.grade(&resp, llm).await?;
    }
    Ok(GateScore {
        score: sum / val_tasks.len() as f32,
        spent_tokens: spent,
    })
}

/// The strict-improvement predicate — `>` not `>=`. Centralised so the rule is
/// auditable in one place.
pub fn strictly_improves(new_val: f32, best_val: f32) -> bool {
    new_val > best_val
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strict_gt_only() {
        assert!(strictly_improves(0.6, 0.5));
        assert!(!strictly_improves(0.5, 0.5)); // equal is NOT accepted
        assert!(!strictly_improves(0.4, 0.5));
    }
}
