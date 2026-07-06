//! `TrainingReport` (epochs, val-curve, accepted/rejected, spent tokens) +
//! the `best_skill.md` writer — the deployable artifact.
//!
//! The report is the surface the VibeUI panel renders (val-curve chart, accept
//! counts, spent-token meter). `best_skill.md` is **never** auto-overwritten onto
//! a shipped `skills/*.md` — the panel writes `*.opt.md` and requires an explicit
//! human "promote" action (see `notes/skillforge/06`).

use std::path::Path;

use serde::{Deserialize, Serialize};

use skilllensai::model::skill::Skill;

/// The outcome of a training run.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrainingReport {
    pub skill_name: String,
    pub epochs_run: usize,
    pub best_val_score: f32,
    /// Per-epoch held-out score (one entry per epoch actually run).
    pub val_curve: Vec<f32>,
    pub accepted: usize,
    pub rejected: usize,
    pub final_tokens: usize,
    /// Approximate tokens spent on LLM calls (rollouts + propose + merges).
    pub spent_tokens: usize,
    /// The deployable artifact — the trained skill markdown.
    pub best_skill_md: String,
    /// Whether early-stop fired (`patience` epochs without val gain).
    pub early_stopped: bool,
}

impl TrainingReport {
    /// Render a compact human-readable summary.
    pub fn to_markdown(&self) -> String {
        let curve = self
            .val_curve
            .iter()
            .map(|v| format!("{v:.3}"))
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            "# TrainingReport — {name}\n\n\
             - epochs run: {epochs}\n\
             - best val score: {best:.3}\n\
             - val curve: [{curve}]\n\
             - accepted / rejected: {acc} / {rej}\n\
             - final tokens: {ftok}\n\
             - spent tokens (LLM): {spent}\n\
             - early stopped: {es}\n\n\
             ## best_skill.md\n\n```markdown\n{md}\n```\n",
            name = self.skill_name,
            epochs = self.epochs_run,
            best = self.best_val_score,
            acc = self.accepted,
            rej = self.rejected,
            ftok = self.final_tokens,
            spent = self.spent_tokens,
            es = self.early_stopped,
            md = self.best_skill_md.trim_end(),
        )
    }

    /// Write the deployable `best_skill.md` to `path`. **Never** point this at a
    /// shipped `skills/*.md` — the operator promotes `*.opt.md` explicitly.
    pub fn write_best_skill(&self, path: &Path) -> std::io::Result<()> {
        std::fs::write(path, &self.best_skill_md)
    }
}

/// Approximate token cost of a single chat turn (system + user + response),
/// using the ~4-chars/token heuristic shared with `skilllensai::Skill`.
pub fn approx_tokens(system: &str, user: &str, response: &str) -> usize {
    (system.chars().count() + user.chars().count() + response.chars().count()) / 4
}

/// Render a skill body to the deployable markdown (frontmatter + body). Wraps
/// [`Skill::render`] so the trainer doesn't depend on the model module path.
pub fn render_skill(skill: &Skill) -> String {
    skill.render()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_markdown_includes_curve_and_counts() {
        let r = TrainingReport {
            skill_name: "formal-verification".into(),
            epochs_run: 2,
            best_val_score: 0.75,
            val_curve: vec![0.5, 0.75],
            accepted: 1,
            rejected: 2,
            final_tokens: 220,
            spent_tokens: 10_000,
            best_skill_md: "---\ntriggers: [\"x\"]\ncategory: c\n---\nbody\n".into(),
            early_stopped: false,
        };
        let md = r.to_markdown();
        assert!(md.contains("formal-verification"));
        assert!(md.contains("0.500, 0.750"));
        assert!(md.contains("accepted / rejected: 1 / 2"));
        assert!(md.contains("best_skill.md"));
    }

    #[test]
    fn approx_tokens_uses_chars_per_four() {
        // 40 chars total → 10 tokens.
        assert_eq!(approx_tokens("aaaa", "bbbb", "cccc"), 3);
        assert_eq!(approx_tokens("a".repeat(40).as_str(), "", ""), 10);
    }
}
