//! `RejectedEditBuffer` — remembers failed edits across epochs so `propose`
//! never re-suggests a known-bad change (training stability).
//!
//! Membership is by canonical JSON key (see [`EditOp`]'s serde form), so two
//! edits that serialise identically are treated as the same proposal. The
//! buffer also remembers **why** an edit was rejected (apply error vs. no val
//! gain) for the training report.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::edit::EditOp;

/// Why an edit landed in the buffer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RejectReason {
    /// The edit failed to apply (e.g. unknown anchor).
    ApplyError(String),
    /// Applied cleanly but the held-out score did not strictly improve.
    NoValGain,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RejectedEntry {
    pub edit: EditOp,
    pub reason: RejectReason,
    pub epoch: usize,
}

/// A set of edits known not to help, keyed by their canonical JSON form.
#[derive(Debug, Clone, Default)]
pub struct RejectedEditBuffer {
    keys: HashSet<String>,
    entries: Vec<RejectedEntry>,
}

impl RejectedEditBuffer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn entries(&self) -> &[RejectedEntry] {
        &self.entries
    }

    /// True when an edit structurally identical to `op` is already rejected.
    pub fn contains(&self, op: &EditOp) -> bool {
        self.keys.contains(&canonical_key(op))
    }

    /// Record a rejected edit. Idempotent — pushing the same edit twice keeps
    /// the first entry.
    pub fn push(&mut self, edit: EditOp, reason: RejectReason, epoch: usize) {
        let key = canonical_key(&edit);
        if self.keys.insert(key) {
            self.entries.push(RejectedEntry {
                edit,
                reason,
                epoch,
            });
        }
    }
}

/// Canonical, stable key for an edit — its compact JSON. Two edits with the
/// same fields produce the same key.
pub fn canonical_key(op: &EditOp) -> String {
    serde_json::to_string(op).unwrap_or_else(|_| String::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dedups_structurally_identical_edits() {
        let mut buf = RejectedEditBuffer::new();
        let e = EditOp::Add {
            after_anchor: None,
            text: "x".into(),
        };
        buf.push(e.clone(), RejectReason::NoValGain, 0);
        buf.push(e.clone(), RejectReason::NoValGain, 1); // dup
        assert_eq!(buf.len(), 1);
        assert!(buf.contains(&e));
    }

    #[test]
    fn distinguishes_different_edits() {
        let mut buf = RejectedEditBuffer::new();
        let a = EditOp::Add {
            after_anchor: None,
            text: "x".into(),
        };
        let b = EditOp::Add {
            after_anchor: None,
            text: "y".into(),
        };
        buf.push(a.clone(), RejectReason::NoValGain, 0);
        assert!(!buf.contains(&b));
        buf.push(b, RejectReason::ApplyError("no anchor".into()), 1);
        assert_eq!(buf.len(), 2);
    }

    #[test]
    fn records_reason_and_epoch() {
        let mut buf = RejectedEditBuffer::new();
        let e = EditOp::Delete {
            anchor: "step 2".into(),
        };
        buf.push(
            e.clone(),
            RejectReason::ApplyError("anchor not found".into()),
            3,
        );
        let entry = &buf.entries()[0];
        assert_eq!(entry.epoch, 3);
        assert!(matches!(entry.reason, RejectReason::ApplyError(_)));
    }
}
