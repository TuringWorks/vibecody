//! Skill-utility metrics — implemented once here, reused by `skilloptai-rs`.
//!
//! - [`trigger_coverage`] — deterministic, no LLM (Phase 1).
//! - [`eval`] (`extraction_efficacy` / `target_evolvability` + `EvalTask` /
//!   `Grader`) — LLM-backed (Phase 2, behind the `llm` feature). This is the
//!   shared measurement `skilloptai-rs`'s validation gate calls into.

use crate::model::skill::Skill;
use crate::model::trajectory::Trajectory;

#[cfg(feature = "llm")]
pub mod eval;

/// Fraction of `intents` that match at least one of the skill's triggers
/// (case-insensitive substring). Returns `0.0` for an empty intent set.
///
/// This is the deterministic proxy for "does this skill fire on the intents we
/// actually observe" — no model call, fully reproducible.
pub fn trigger_coverage(skill: &Skill, intents: &[String]) -> f32 {
    if intents.is_empty() {
        return 0.0;
    }
    let triggers: Vec<String> = skill
        .triggers
        .iter()
        .map(|t| t.to_lowercase())
        .filter(|t| !t.is_empty())
        .collect();
    if triggers.is_empty() {
        return 0.0;
    }
    let hits = intents
        .iter()
        .filter(|intent| {
            let lc = intent.to_lowercase();
            triggers.iter().any(|t| lc.contains(t.as_str()))
        })
        .count();
    hits as f32 / intents.len() as f32
}

/// Collect the observed intent (first user step) of each trajectory.
pub fn intents_from_trajectories(trajectories: &[Trajectory]) -> Vec<String> {
    trajectories
        .iter()
        .filter_map(|t| t.intent().map(str::to_string))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::skill::Skill;

    #[test]
    fn coverage_counts_substring_hits() {
        let s = Skill::from_str_named(
            "sec",
            "---\ntriggers: [\"security\", \"owasp\"]\ncategory: x\n---\nbody",
        );
        let intents = vec![
            "do a security review".to_string(),
            "unrelated request".to_string(),
            "OWASP top 10".to_string(),
        ];
        let cov = trigger_coverage(&s, &intents);
        assert!((cov - 2.0 / 3.0).abs() < 1e-6, "cov = {cov}");
    }

    #[test]
    fn empty_inputs_are_zero() {
        let s = Skill::from_str_named("t", "---\ntriggers: [\"x\"]\n---\nb");
        assert_eq!(trigger_coverage(&s, &[]), 0.0);
        let no_triggers = Skill::from_str_named("t", "# no fm\n");
        assert_eq!(trigger_coverage(&no_triggers, &["x".to_string()]), 0.0);
    }
}
