//! `propose` — the optimizer LLM turns failing trajectories into candidate
//! [`EditOp`]s (the textual gradient), excluding anything in the
//! rejected-edit buffer and bounded by the textual learning rate.
//!
//! The system prompt carries the marker `PROPOSE_EDITS` so a mock `SkillLlm`
//! can branch on it deterministically (see `tests/`). The model is asked to
//! return a JSON array of edit ops; we parse leniently, drop anything already
//! in [`RejectedEditBuffer`], and truncate to the `textual_lr` budget via
//! [`within_budget`](crate::edit::within_budget).

use skilllensai::llm::SkillLlm;
use skilllensai::model::skill::Skill;
use skilllensai::model::trajectory::Trajectory;

use crate::buffer::RejectedEditBuffer;
use crate::edit::{within_budget, EditOp};
use crate::report::approx_tokens;

/// The result of one propose pass.
pub struct ProposeResult {
    /// Edits after rejected-buffer filtering and lr truncation.
    pub edits: Vec<EditOp>,
    /// Edits the model emitted that were dropped because they're in the
    /// rejected buffer (reported, not re-applied).
    pub dropped_as_rejected: usize,
    pub raw_response: String,
    pub spent_tokens: usize,
}

/// System prompt marker — a mock `SkillLlm` can detect propose calls by it.
pub const PROPOSE_MARKER: &str = "PROPOSE_EDITS";

const PROPOSE_SYSTEM: &str = "\
[PROPOSE_EDITS] You are a textual-gradient optimizer. The agent skill below is the \
trainable state of a frozen agent. Given the failing rollouts, propose ONE OR MORE \
bounded edits (Add / Delete / Replace) that would make the skill produce better, \
correct behaviour on those tasks. Output ONLY a JSON array of edit objects, no prose. \
Edit object shapes: \
{\"op\":\"add\",\"after_anchor\":null,\"text\":\"...\"} (after_anchor null ⇒ prepend), \
{\"op\":\"delete\",\"anchor\":\"<substring of an existing line>\"}, \
{\"op\":\"replace\",\"anchor\":\"<substring>\",\"text\":\"...\"}. Anchors must be \
substrings of lines currently in the skill body.";

pub async fn propose(
    skill: &Skill,
    targets: &[Trajectory],
    rejected: &RejectedEditBuffer,
    lr: usize,
    llm: &dyn SkillLlm,
) -> anyhow::Result<ProposeResult> {
    let user = build_user(skill, targets);
    let raw = llm.chat(PROPOSE_SYSTEM, &user).await?;
    let spent = approx_tokens(PROPOSE_SYSTEM, &user, &raw);

    let parsed = parse_edits(&raw);
    let mut edits = Vec::with_capacity(parsed.len());
    let mut dropped = 0usize;
    for e in parsed {
        if rejected.contains(&e) {
            dropped += 1;
        } else {
            edits.push(e);
        }
    }
    edits = within_budget(edits, lr);
    Ok(ProposeResult {
        edits,
        dropped_as_rejected: dropped,
        raw_response: raw,
        spent_tokens: spent,
    })
}

fn build_user(skill: &Skill, targets: &[Trajectory]) -> String {
    let mut out = String::new();
    out.push_str("### Current skill\n```markdown\n");
    out.push_str(&skill.render());
    out.push_str("\n```\n\n### Failing rollouts\n");
    if targets.is_empty() {
        out.push_str("(no failing rollouts this epoch — propose no edits: [])");
        return out;
    }
    for (i, t) in targets.iter().enumerate() {
        out.push_str(&format!(
            "--- Failure {i} (task {}, score {:.2}) ---\n",
            t.task_id,
            t.score_value()
        ));
        for s in &t.steps {
            let role = match s.role {
                skilllensai::model::trajectory::Role::User => "user",
                skilllensai::model::trajectory::Role::Assistant => "assistant",
                skilllensai::model::trajectory::Role::Tool => "tool",
                skilllensai::model::trajectory::Role::System => "system",
            };
            out.push_str(&format!("[{role}] {}\n", s.content));
        }
        out.push('\n');
    }
    out
}

/// Parse a JSON array of [`EditOp`] from a model reply, leniently: tolerate
/// surrounding prose / code fences by extracting the first balanced `[...]`.
pub fn parse_edits(raw: &str) -> Vec<EditOp> {
    let slice = extract_json_array(raw).unwrap_or(raw);
    serde_json::from_str::<Vec<EditOp>>(slice).unwrap_or_default()
}

/// Extract the first balanced `[ ... ]` substring, accounting for nested
/// brackets inside JSON strings. Returns `None` if no array is found.
fn extract_json_array(s: &str) -> Option<&str> {
    let bytes = s.as_bytes();
    let start = bytes.iter().position(|&b| b == b'[')?;
    let mut depth = 0i32;
    let mut in_str = false;
    let mut esc = false;
    for (i, &b) in bytes.iter().enumerate().skip(start) {
        if in_str {
            if esc {
                esc = false;
            } else if b == b'\\' {
                esc = true;
            } else if b == b'"' {
                in_str = false;
            }
            continue;
        }
        match b {
            b'"' => in_str = true,
            b'[' => depth += 1,
            b']' => {
                depth -= 1;
                if depth == 0 {
                    return Some(&s[start..=i]);
                }
            }
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_edits_strict_json() {
        let raw = r#"[{"op":"add","after_anchor":null,"text":"always output 42"}]"#;
        let edits = parse_edits(raw);
        assert_eq!(edits.len(), 1);
        assert!(matches!(edits[0], EditOp::Add { .. }));
    }

    #[test]
    fn parse_edits_tolerates_prose_and_fence() {
        let raw = "Here are the edits:\n```json\n[{\"op\":\"delete\",\"anchor\":\"step 2\"}]\n```\nThanks.";
        let edits = parse_edits(raw);
        assert_eq!(edits.len(), 1);
        assert!(matches!(edits[0], EditOp::Delete { .. }));
    }

    #[test]
    fn parse_edits_multiple() {
        let raw = r#"[
            {"op":"add","after_anchor":null,"text":"a"},
            {"op":"replace","anchor":"old","text":"new"}
        ]"#;
        assert_eq!(parse_edits(raw).len(), 2);
    }

    #[test]
    fn parse_edits_garbage_is_empty() {
        assert!(parse_edits("no array here").is_empty());
        assert!(parse_edits("[ not valid json ]").is_empty());
    }

    #[test]
    fn extract_array_handles_nested_brackets_in_strings() {
        let raw = r#"prefix [{"op":"replace","anchor":"a[b]","text":"c"}] suffix"#;
        let slice = extract_json_array(raw).unwrap();
        let edits: Vec<EditOp> = serde_json::from_str(slice).unwrap();
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn build_user_with_no_targets_says_so() {
        let s = Skill::from_str_named("t", "---\ntriggers: [\"x\"]\n---\nbody");
        let u = build_user(&s, &[]);
        assert!(u.contains("no failing rollouts"));
    }
}
