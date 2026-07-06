//! `SkillReport` → markdown / JSON. Per-skill utility summary.
//!
//! Phase 1 populates `trigger_coverage` + `token_cost` deterministically; the
//! LLM-backed metrics stay `None` until measured in Phase 2 (honest reporting —
//! `None` means "not measured", not "zero").

use serde::{Deserialize, Serialize};

use crate::model::skill::Skill;

/// The measured utility of one skill.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SkillReport {
    pub skill: String,
    pub category: String,
    pub token_cost: usize,
    pub trigger_coverage: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extraction_efficacy: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_evolvability: Option<f32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
}

impl SkillReport {
    /// Build the deterministic (no-LLM) portion of a report for `skill` against
    /// a set of observed `intents`.
    pub fn measure_static(skill: &Skill, intents: &[String]) -> Self {
        SkillReport {
            skill: skill.name.clone(),
            category: skill.category.clone(),
            token_cost: skill.token_estimate,
            trigger_coverage: crate::metrics::trigger_coverage(skill, intents),
            extraction_efficacy: None,
            target_evolvability: None,
            notes: Vec::new(),
        }
    }

    pub fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or(serde_json::Value::Null)
    }

    /// A single markdown table row (no header).
    pub fn to_markdown_row(&self) -> String {
        let fmt = |o: Option<f32>| o.map(|v| format!("{v:.2}")).unwrap_or_else(|| "—".into());
        format!(
            "| {} | {} | {} | {:.2} | {} | {} |",
            self.skill,
            self.category,
            self.token_cost,
            self.trigger_coverage,
            fmt(self.extraction_efficacy),
            fmt(self.target_evolvability),
        )
    }
}

/// Render a portfolio of reports as a markdown table.
pub fn portfolio_markdown(reports: &[SkillReport]) -> String {
    let mut out = String::from(
        "| skill | category | tokens | coverage | efficacy | evolvability |\n|---|---|---|---|---|---|\n",
    );
    for r in reports {
        out.push_str(&r.to_markdown_row());
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn static_report_has_coverage_and_none_llm_metrics() {
        let s = Skill::from_str_named("t", "---\ntriggers: [\"foo\"]\ncategory: c\n---\nb");
        let r = SkillReport::measure_static(&s, &["do foo now".to_string()]);
        assert_eq!(r.skill, "t");
        assert_eq!(r.trigger_coverage, 1.0);
        assert!(r.extraction_efficacy.is_none());
        // `None` metrics are omitted from JSON.
        let j = r.to_json();
        assert!(j.get("extraction_efficacy").is_none());
        assert!(portfolio_markdown(&[r]).contains("| t |"));
    }
}
