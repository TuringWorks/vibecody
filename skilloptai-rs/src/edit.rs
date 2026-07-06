//! `EditOp` — bounded Add / Delete / Replace operations on a skill document.
//!
//! Edits are anchor-based and operate on the skill **body** (the numbered-step
//! markdown). The optimizer's `propose` step emits these as JSON; the trainer
//! applies each within the `textual_lr` per-epoch budget and the
//! `max_skill_tokens` hard cap.
//!
//! Apply semantics are line-oriented:
//! - `Add { after_anchor: None, text }` — prepend `text` as a new line at the
//!   top of the body.
//! - `Add { after_anchor: Some(a), text }` — insert `text` as a new line
//!   immediately after the first line containing `a`.
//! - `Delete { anchor }` — remove every line containing `anchor`.
//! - `Replace { anchor, text }` — replace the first line containing `anchor`
//!   with `text`.
//!
//! An unknown anchor is an error (`Err`); the trainer treats a failed apply as
//! a rejected edit rather than aborting the epoch.

use serde::{Deserialize, Serialize};

/// One bounded textual edit to a skill body.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "lowercase")]
pub enum EditOp {
    /// Insert `text`. Prepended when `after_anchor` is absent; otherwise
    /// inserted after the first line matching the anchor.
    Add {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        after_anchor: Option<String>,
        text: String,
    },
    /// Remove every line containing `anchor`.
    Delete { anchor: String },
    /// Replace the first line containing `anchor` with `text`.
    Replace { anchor: String, text: String },
}

impl EditOp {
    /// A rough magnitude-of-change estimate used to enforce the per-epoch
    /// textual learning rate. Counts characters added (Add/Replace) or the
    /// anchor length (Delete) — a proxy for "how much text churn this edit
    /// causes", sufficient for budgeting.
    pub fn char_cost(&self) -> usize {
        match self {
            EditOp::Add { text, .. } | EditOp::Replace { text, .. } => text.chars().count(),
            EditOp::Delete { anchor } => anchor.chars().count(),
        }
    }

    /// Apply the edit to `body`, returning the new body. `Err` when an anchor
    /// is required but not found.
    pub fn apply(&self, body: &str) -> anyhow::Result<String> {
        Ok(match self {
            EditOp::Add {
                after_anchor: None,
                text,
            } => prepend_line(body, text),
            EditOp::Add {
                after_anchor: Some(anchor),
                text,
            } => insert_after_first_match(body, anchor, text)?,
            EditOp::Delete { anchor } => remove_matching_lines(body, anchor),
            EditOp::Replace { anchor, text } => replace_first_match(body, anchor, text)?,
        })
    }

    /// A short, human-readable label for reports / the rejected-edit buffer.
    pub fn label(&self) -> String {
        match self {
            EditOp::Add {
                after_anchor: None,
                text,
            } => {
                format!("Add(prepend, {} chars)", text.chars().count())
            }
            EditOp::Add {
                after_anchor: Some(a),
                text,
            } => format!(
                "Add(after {:?}, {} chars)",
                truncate_for_log(a, 24),
                text.chars().count()
            ),
            EditOp::Delete { anchor } => format!("Delete({:?})", truncate_for_log(anchor, 24)),
            EditOp::Replace { anchor, text } => format!(
                "Replace({:?} → {} chars)",
                truncate_for_log(anchor, 24),
                text.chars().count()
            ),
        }
    }
}

/// Truncate a list of edits so their total [`EditOp::char_cost`] fits `lr`.
/// Greedy, order-preserving — the first edits that fit win.
pub fn within_budget(edits: Vec<EditOp>, lr: usize) -> Vec<EditOp> {
    let mut budget = lr;
    let mut out = Vec::with_capacity(edits.len());
    for e in edits {
        let cost = e.char_cost();
        if cost > budget {
            continue;
        }
        budget -= cost;
        out.push(e);
    }
    out
}

fn truncate_for_log(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let mut out: String = s.chars().take(max.saturating_sub(1)).collect();
        out.push('…');
        out
    }
}

fn prepend_line(body: &str, text: &str) -> String {
    let mut out = String::with_capacity(body.len() + text.len() + 2);
    out.push_str(text);
    out.push('\n');
    out.push_str(body);
    out
}

fn insert_after_first_match(body: &str, anchor: &str, text: &str) -> anyhow::Result<String> {
    let mut out = String::with_capacity(body.len() + text.len() + 2);
    let mut matched = false;
    for line in body.split_inclusive('\n') {
        out.push_str(line);
        if !matched && line.contains(anchor) {
            out.push_str(text);
            out.push('\n');
            matched = true;
        }
    }
    if !matched {
        anyhow::bail!("anchor not found: {anchor:?}");
    }
    Ok(out)
}

fn remove_matching_lines(body: &str, anchor: &str) -> String {
    body.split_inclusive('\n')
        .filter(|line| !line.contains(anchor))
        .collect()
}

fn replace_first_match(body: &str, anchor: &str, text: &str) -> anyhow::Result<String> {
    let mut out = String::with_capacity(body.len());
    let mut matched = false;
    for line in body.split_inclusive('\n') {
        if !matched && line.contains(anchor) {
            out.push_str(text);
            out.push('\n');
            matched = true;
        } else {
            out.push_str(line);
        }
    }
    if !matched {
        anyhow::bail!("anchor not found: {anchor:?}");
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_prepend_and_after_anchor() {
        let body = "line one\nline two\n";
        assert_eq!(
            EditOp::Add {
                after_anchor: None,
                text: "TOP".into()
            }
            .apply(body)
            .unwrap(),
            "TOP\nline one\nline two\n"
        );
        let got = EditOp::Add {
            after_anchor: Some("two".into()),
            text: "INSERTED".into(),
        }
        .apply(body)
        .unwrap();
        assert_eq!(got, "line one\nline two\nINSERTED\n");
    }

    #[test]
    fn delete_removes_all_matching_lines() {
        let body = "keep\nremove me\nkeep\nremove me too\n";
        let got = EditOp::Delete {
            anchor: "remove".into(),
        }
        .apply(body)
        .unwrap();
        assert_eq!(got, "keep\nkeep\n");
    }

    #[test]
    fn replace_first_match_only() {
        let body = "a\nb\na\n";
        let got = EditOp::Replace {
            anchor: "a".into(),
            text: "Z".into(),
        }
        .apply(body)
        .unwrap();
        assert_eq!(got, "Z\nb\na\n");
    }

    #[test]
    fn unknown_anchor_errors() {
        let body = "only line\n";
        assert!(EditOp::Replace {
            anchor: "missing".into(),
            text: "x".into()
        }
        .apply(body)
        .is_err());
        assert!(EditOp::Add {
            after_anchor: Some("missing".into()),
            text: "x".into()
        }
        .apply(body)
        .is_err());
    }

    #[test]
    fn char_cost_counts_text_or_anchor() {
        assert_eq!(
            EditOp::Add {
                after_anchor: None,
                text: "abc".into()
            }
            .char_cost(),
            3
        );
        assert_eq!(
            EditOp::Replace {
                anchor: "x".into(),
                text: "abcd".into()
            }
            .char_cost(),
            4
        );
        assert_eq!(
            EditOp::Delete {
                anchor: "xy".into()
            }
            .char_cost(),
            2
        );
    }

    #[test]
    fn within_budget_greedy_order_preserving() {
        let edits = vec![
            EditOp::Add {
                after_anchor: None,
                text: "aa".into(),
            }, // cost 2
            EditOp::Add {
                after_anchor: None,
                text: "bbbb".into(),
            }, // cost 4
            EditOp::Add {
                after_anchor: None,
                text: "c".into(),
            }, // cost 1
        ];
        let fit = within_budget(edits, 3);
        assert_eq!(fit.len(), 2);
        assert_eq!(fit[0].char_cost(), 2);
        assert_eq!(fit[1].char_cost(), 1);
    }

    #[test]
    fn serde_roundtrip_json() {
        let op = EditOp::Replace {
            anchor: "step 1".into(),
            text: "step 1 revised".into(),
        };
        let j = serde_json::to_string(&op).unwrap();
        assert!(j.contains("\"op\":\"replace\""));
        let back: EditOp = serde_json::from_str(&j).unwrap();
        assert_eq!(op, back);
    }

    #[test]
    fn label_is_short() {
        let op = EditOp::Add {
            after_anchor: Some("very long anchor text here".into()),
            text: "x".into(),
        };
        let l = op.label();
        assert!(l.contains("Add(after"));
        assert!(l.contains("…"));
    }
}
