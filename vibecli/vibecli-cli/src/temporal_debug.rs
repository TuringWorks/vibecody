//! Context versioning and temporal debugging ("time travel" debugging).
//!
//! GAP-v9-016: rivals GitHub Copilot History, Cursor Timeline, Zed Time Travel.
//! - Snapshot context at each agent step with diff-aware comparison
//! - "What changed between commits A and B?" prompt injection
//! - Variable state timeline: track value changes across agent turns
//! - Temporal bisect: binary-search for the step that introduced a bug
//! - Context version store: JSONL append-only log with random-access replay
//! - Side-by-side diff between any two context snapshots

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ─── Context Snapshot ─────────────────────────────────────────────────────────

/// A point-in-time snapshot of agent context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextSnapshot {
    pub id: String,
    pub step: u64,
    pub timestamp_ms: u64,
    pub commit_hash: Option<String>,
    pub variables: HashMap<String, VarValue>,
    pub files_changed: Vec<String>,
    pub agent_action: String,
    pub parent_id: Option<String>,
}

impl ContextSnapshot {
    pub fn new(id: &str, step: u64, ts: u64, action: &str) -> Self {
        Self {
            id: id.to_string(), step, timestamp_ms: ts,
            commit_hash: None, variables: HashMap::new(),
            files_changed: Vec::new(), agent_action: action.to_string(),
            parent_id: None,
        }
    }

    pub fn with_commit(mut self, hash: &str) -> Self { self.commit_hash = Some(hash.to_string()); self }
    pub fn with_parent(mut self, parent: &str) -> Self { self.parent_id = Some(parent.to_string()); self }
    pub fn add_var(&mut self, name: &str, val: VarValue) { self.variables.insert(name.to_string(), val); }
    pub fn add_file(&mut self, path: &str) { self.files_changed.push(path.to_string()); }
}

/// A variable value at a point in time.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum VarValue {
    Int(i64),
    Float(f64),
    Bool(bool),
    Text(String),
    Null,
    List(Vec<VarValue>),
}

impl std::fmt::Display for VarValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Int(n)   => write!(f, "{n}"),
            Self::Float(v) => write!(f, "{v:.3}"),
            Self::Bool(b)  => write!(f, "{b}"),
            Self::Text(s)  => write!(f, "{s}"),
            Self::Null     => write!(f, "null"),
            Self::List(v)  => write!(f, "[{} items]", v.len()),
        }
    }
}

// ─── Context Diff ─────────────────────────────────────────────────────────────

/// Type of change between two context snapshots.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ContextChange {
    VarAdded     { name: String, value: String },
    VarRemoved   { name: String },
    VarChanged   { name: String, from: String, to: String },
    FileAdded    { path: String },
    FileRemoved  { path: String },
    ActionChanged { from: String, to: String },
}

/// Diff between two context snapshots.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextDiff {
    pub from_id: String,
    pub to_id: String,
    pub from_step: u64,
    pub to_step: u64,
    pub changes: Vec<ContextChange>,
}

impl ContextDiff {
    pub fn is_empty(&self) -> bool { self.changes.is_empty() }
    pub fn has_var_changes(&self) -> bool {
        self.changes.iter().any(|c| matches!(c, ContextChange::VarChanged { .. }))
    }
    pub fn has_file_changes(&self) -> bool {
        self.changes.iter().any(|c| matches!(c, ContextChange::FileAdded { .. } | ContextChange::FileRemoved { .. }))
    }
}

// ─── Variable Timeline ────────────────────────────────────────────────────────

/// A single entry in a variable's history.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VarHistoryEntry {
    pub step: u64,
    pub snapshot_id: String,
    pub value: VarValue,
}

/// Timeline of a variable's changes across steps.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VarTimeline {
    pub name: String,
    pub entries: Vec<VarHistoryEntry>,
}

impl VarTimeline {
    pub fn new(name: &str) -> Self { Self { name: name.to_string(), entries: Vec::new() } }

    pub fn push(&mut self, step: u64, snap_id: &str, val: VarValue) {
        self.entries.push(VarHistoryEntry { step, snapshot_id: snap_id.to_string(), value: val });
    }

    pub fn changed_at(&self) -> Vec<u64> {
        let mut steps = Vec::new();
        for i in 1..self.entries.len() {
            if self.entries[i].value != self.entries[i - 1].value {
                steps.push(self.entries[i].step);
            }
        }
        steps
    }

    pub fn value_at(&self, step: u64) -> Option<&VarValue> {
        self.entries.iter().rev().find(|e| e.step <= step).map(|e| &e.value)
    }

    pub fn total_changes(&self) -> usize { self.changed_at().len() }
}

// ─── Temporal Bisect ──────────────────────────────────────────────────────────

/// Bisect result locating the step introducing a bug.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BisectResult {
    pub culprit_step: u64,
    pub culprit_snapshot_id: String,
    pub iterations: usize,
    pub good_step: u64,
    pub bad_step: u64,
}

// ─── Temporal Debugger ────────────────────────────────────────────────────────

/// Core temporal debugging engine.
pub struct TemporalDebugger {
    snapshots: Vec<ContextSnapshot>,
    id_counter: u32,
}

impl TemporalDebugger {
    pub fn new() -> Self { Self { snapshots: Vec::new(), id_counter: 0 } }

    fn next_id(&mut self) -> String {
        self.id_counter += 1;
        format!("snap-{:05}", self.id_counter)
    }

    /// Record a new context snapshot.
    pub fn record(&mut self, mut snap: ContextSnapshot) -> &ContextSnapshot {
        if snap.id.is_empty() { snap.id = self.next_id(); }
        // Auto-link to previous snapshot as parent
        if snap.parent_id.is_none() {
            if let Some(prev) = self.snapshots.last() {
                snap.parent_id = Some(prev.id.clone());
            }
        }
        self.snapshots.push(snap);
        self.snapshots.last().unwrap()
    }

    /// Diff two snapshots by ID.
    pub fn diff(&self, from_id: &str, to_id: &str) -> Option<ContextDiff> {
        let from = self.snapshots.iter().find(|s| s.id == from_id)?;
        let to   = self.snapshots.iter().find(|s| s.id == to_id)?;
        let mut changes = Vec::new();

        // Variable changes
        for (k, v) in &to.variables {
            match from.variables.get(k) {
                None => changes.push(ContextChange::VarAdded { name: k.clone(), value: v.to_string() }),
                Some(old) if old != v => changes.push(ContextChange::VarChanged { name: k.clone(), from: old.to_string(), to: v.to_string() }),
                _ => {}
            }
        }
        for k in from.variables.keys() {
            if !to.variables.contains_key(k) {
                changes.push(ContextChange::VarRemoved { name: k.clone() });
            }
        }
        // File changes
        for f in &to.files_changed {
            if !from.files_changed.contains(f) {
                changes.push(ContextChange::FileAdded { path: f.clone() });
            }
        }
        for f in &from.files_changed {
            if !to.files_changed.contains(f) {
                changes.push(ContextChange::FileRemoved { path: f.clone() });
            }
        }
        // Action change
        if from.agent_action != to.agent_action {
            changes.push(ContextChange::ActionChanged { from: from.agent_action.clone(), to: to.agent_action.clone() });
        }

        Some(ContextDiff {
            from_id: from_id.to_string(), to_id: to_id.to_string(),
            from_step: from.step, to_step: to.step, changes,
        })
    }

    /// Get snapshot by step number.
    pub fn at_step(&self, step: u64) -> Option<&ContextSnapshot> {
        self.snapshots.iter().find(|s| s.step == step)
    }

    /// Replay snapshots from `from_step` to `to_step` in order.
    pub fn replay(&self, from_step: u64, to_step: u64) -> Vec<&ContextSnapshot> {
        self.snapshots.iter()
            .filter(|s| s.step >= from_step && s.step <= to_step)
            .collect()
    }

    /// Build a variable timeline across all snapshots.
    pub fn var_timeline(&self, var_name: &str) -> VarTimeline {
        let mut timeline = VarTimeline::new(var_name);
        for snap in &self.snapshots {
            if let Some(val) = snap.variables.get(var_name) {
                timeline.push(snap.step, &snap.id, val.clone());
            }
        }
        timeline
    }

    /// Binary-search for the snapshot where a predicate first becomes true.
    /// Returns the culprit snapshot. `good_predicate` returns true for a "good" snapshot.
    pub fn bisect<F>(&self, good_predicate: F) -> Option<BisectResult>
    where F: Fn(&ContextSnapshot) -> bool
    {
        if self.snapshots.is_empty() { return None; }
        let mut lo = 0usize;
        let mut hi = self.snapshots.len() - 1;
        if !good_predicate(&self.snapshots[lo]) { return None; } // first is already bad
        if good_predicate(&self.snapshots[hi]) { return None; }  // last is still good → no regression

        let mut iterations = 0;
        while lo + 1 < hi {
            iterations += 1;
            let mid = (lo + hi) / 2;
            if good_predicate(&self.snapshots[mid]) { lo = mid; } else { hi = mid; }
        }
        let culprit = &self.snapshots[hi];
        Some(BisectResult {
            culprit_step: culprit.step,
            culprit_snapshot_id: culprit.id.clone(),
            iterations,
            good_step: self.snapshots[lo].step,
            bad_step: culprit.step,
        })
    }

    /// Export snapshot log as JSONL.
    pub fn export_jsonl(&self) -> Vec<String> {
        self.snapshots.iter()
            .map(|s| serde_json::to_string(s).unwrap_or_default())
            .collect()
    }

    /// Generate a diff-aware prompt injection string for LLM context.
    pub fn diff_prompt(&self, from_id: &str, to_id: &str) -> Option<String> {
        let diff = self.diff(from_id, to_id)?;
        if diff.is_empty() { return Some("No changes detected between snapshots.".into()); }
        let mut prompt = format!("Changes from step {} to step {}:\n", diff.from_step, diff.to_step);
        for c in &diff.changes {
            let line = match c {
                ContextChange::VarChanged { name, from, to } => format!("  • Variable `{name}` changed: {from} → {to}"),
                ContextChange::VarAdded { name, value } => format!("  • Variable `{name}` added with value: {value}"),
                ContextChange::VarRemoved { name } => format!("  • Variable `{name}` removed"),
                ContextChange::FileAdded { path } => format!("  • File added: {path}"),
                ContextChange::FileRemoved { path } => format!("  • File removed: {path}"),
                ContextChange::ActionChanged { from, to } => format!("  • Action changed: {from} → {to}"),
            };
            prompt.push_str(&line);
            prompt.push('\n');
        }
        Some(prompt)
    }

    pub fn snapshots(&self) -> &[ContextSnapshot] { &self.snapshots }
    pub fn snapshot_count(&self) -> usize { self.snapshots.len() }
}

impl Default for TemporalDebugger { fn default() -> Self { Self::new() } }

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn snap(id: &str, step: u64, action: &str) -> ContextSnapshot {
        ContextSnapshot::new(id, step, step * 1000, action)
    }

    // ── VarValue ──────────────────────────────────────────────────────────

    #[test]
    fn test_var_value_display_int() { assert_eq!(VarValue::Int(42).to_string(), "42"); }
    #[test]
    fn test_var_value_display_bool() { assert_eq!(VarValue::Bool(true).to_string(), "true"); }
    #[test]
    fn test_var_value_display_null() { assert_eq!(VarValue::Null.to_string(), "null"); }
    #[test]
    fn test_var_value_equality() { assert_eq!(VarValue::Int(1), VarValue::Int(1)); }
    #[test]
    fn test_var_value_inequality() { assert_ne!(VarValue::Int(1), VarValue::Int(2)); }

    // ── ContextSnapshot ───────────────────────────────────────────────────

    #[test]
    fn test_snapshot_add_var() {
        let mut s = snap("s1", 1, "init");
        s.add_var("x", VarValue::Int(10));
        assert!(s.variables.contains_key("x"));
    }

    #[test]
    fn test_snapshot_add_file() {
        let mut s = snap("s1", 1, "edit");
        s.add_file("src/lib.rs");
        assert_eq!(s.files_changed, vec!["src/lib.rs"]);
    }

    #[test]
    fn test_snapshot_with_commit() {
        let s = snap("s1", 1, "act").with_commit("abc123");
        assert_eq!(s.commit_hash.as_deref(), Some("abc123"));
    }

    // ── TemporalDebugger — record ─────────────────────────────────────────

    #[test]
    fn test_debugger_record_increments_count() {
        let mut d = TemporalDebugger::new();
        d.record(snap("", 1, "a"));
        d.record(snap("", 2, "b"));
        assert_eq!(d.snapshot_count(), 2);
    }

    #[test]
    fn test_debugger_auto_parent_link() {
        let mut d = TemporalDebugger::new();
        d.record(snap("s1", 1, "a"));
        d.record(snap("s2", 2, "b"));
        assert_eq!(d.snapshots()[1].parent_id.as_deref(), Some("s1"));
    }

    #[test]
    fn test_debugger_at_step() {
        let mut d = TemporalDebugger::new();
        d.record(snap("s1", 5, "act"));
        assert!(d.at_step(5).is_some());
        assert!(d.at_step(9).is_none());
    }

    // ── diff ──────────────────────────────────────────────────────────────

    #[test]
    fn test_diff_var_changed() {
        let mut d = TemporalDebugger::new();
        let mut s1 = snap("s1", 1, "init");
        s1.add_var("x", VarValue::Int(1));
        let mut s2 = snap("s2", 2, "update");
        s2.add_var("x", VarValue::Int(99));
        d.record(s1); d.record(s2);
        let diff = d.diff("s1", "s2").unwrap();
        assert!(diff.has_var_changes());
        assert!(diff.changes.iter().any(|c| matches!(c, ContextChange::VarChanged { name, .. } if name == "x")));
    }

    #[test]
    fn test_diff_var_added() {
        let mut d = TemporalDebugger::new();
        d.record(snap("s1", 1, "init"));
        let mut s2 = snap("s2", 2, "add");
        s2.add_var("new_var", VarValue::Bool(true));
        d.record(s2);
        let diff = d.diff("s1", "s2").unwrap();
        assert!(diff.changes.iter().any(|c| matches!(c, ContextChange::VarAdded { name, .. } if name == "new_var")));
    }

    #[test]
    fn test_diff_var_removed() {
        let mut d = TemporalDebugger::new();
        let mut s1 = snap("s1", 1, "init");
        s1.add_var("tmp", VarValue::Null);
        d.record(s1);
        d.record(snap("s2", 2, "clean"));
        let diff = d.diff("s1", "s2").unwrap();
        assert!(diff.changes.iter().any(|c| matches!(c, ContextChange::VarRemoved { name } if name == "tmp")));
    }

    #[test]
    fn test_diff_file_added() {
        let mut d = TemporalDebugger::new();
        d.record(snap("s1", 1, "a"));
        let mut s2 = snap("s2", 2, "b");
        s2.add_file("new.rs");
        d.record(s2);
        let diff = d.diff("s1", "s2").unwrap();
        assert!(diff.has_file_changes());
    }

    #[test]
    fn test_diff_empty_when_same() {
        let mut d = TemporalDebugger::new();
        d.record(snap("s1", 1, "same"));
        d.record(snap("s2", 2, "same"));
        let diff = d.diff("s1", "s2").unwrap();
        assert!(diff.is_empty());
    }

    #[test]
    fn test_diff_unknown_id_returns_none() {
        let d = TemporalDebugger::new();
        assert!(d.diff("x", "y").is_none());
    }

    // ── replay ────────────────────────────────────────────────────────────

    #[test]
    fn test_replay_range() {
        let mut d = TemporalDebugger::new();
        for i in 1..=5 { d.record(snap(&format!("s{i}"), i, "a")); }
        let replayed = d.replay(2, 4);
        assert_eq!(replayed.len(), 3);
        assert_eq!(replayed[0].step, 2);
        assert_eq!(replayed[2].step, 4);
    }

    #[test]
    fn test_replay_single_step() {
        let mut d = TemporalDebugger::new();
        d.record(snap("s1", 1, "a"));
        d.record(snap("s2", 2, "b"));
        let replayed = d.replay(2, 2);
        assert_eq!(replayed.len(), 1);
    }

    // ── VarTimeline ───────────────────────────────────────────────────────

    #[test]
    fn test_var_timeline_changed_at() {
        let mut d = TemporalDebugger::new();
        let mut s1 = snap("s1", 1, "a"); s1.add_var("count", VarValue::Int(0)); d.record(s1);
        let mut s2 = snap("s2", 2, "b"); s2.add_var("count", VarValue::Int(1)); d.record(s2);
        let mut s3 = snap("s3", 3, "c"); s3.add_var("count", VarValue::Int(1)); d.record(s3);
        let tl = d.var_timeline("count");
        assert_eq!(tl.changed_at(), vec![2]); // only step 2 changed
    }

    #[test]
    fn test_var_timeline_value_at_step() {
        let mut d = TemporalDebugger::new();
        let mut s1 = snap("s1", 1, "a"); s1.add_var("x", VarValue::Int(10)); d.record(s1);
        let mut s2 = snap("s2", 3, "b"); s2.add_var("x", VarValue::Int(20)); d.record(s2);
        let tl = d.var_timeline("x");
        // At step 2 (between 1 and 3), value should be the step-1 value
        assert_eq!(tl.value_at(2), Some(&VarValue::Int(10)));
        assert_eq!(tl.value_at(3), Some(&VarValue::Int(20)));
    }

    #[test]
    fn test_var_timeline_total_changes() {
        let mut tl = VarTimeline::new("v");
        tl.push(1, "s1", VarValue::Int(0));
        tl.push(2, "s2", VarValue::Int(1));
        tl.push(3, "s3", VarValue::Int(1)); // same, no change
        tl.push(4, "s4", VarValue::Int(2));
        assert_eq!(tl.total_changes(), 2);
    }

    // ── bisect ────────────────────────────────────────────────────────────

    #[test]
    fn test_bisect_finds_culprit() {
        let mut d = TemporalDebugger::new();
        for i in 0..8u64 { d.record(snap(&format!("s{i}"), i, "step")); }
        // Steps 0-4 are "good" (step < 5), steps 5+ are "bad"
        let result = d.bisect(|s| s.step < 5).unwrap();
        assert_eq!(result.culprit_step, 5);
    }

    #[test]
    fn test_bisect_returns_none_if_first_bad() {
        let mut d = TemporalDebugger::new();
        for i in 0..4u64 { d.record(snap(&format!("s{i}"), i, "s")); }
        // all steps fail predicate
        let result = d.bisect(|_| false);
        assert!(result.is_none());
    }

    #[test]
    fn test_bisect_returns_none_if_all_good() {
        let mut d = TemporalDebugger::new();
        for i in 0..4u64 { d.record(snap(&format!("s{i}"), i, "s")); }
        let result = d.bisect(|_| true);
        assert!(result.is_none());
    }

    // ── export & prompt ───────────────────────────────────────────────────

    #[test]
    fn test_export_jsonl_count() {
        let mut d = TemporalDebugger::new();
        d.record(snap("s1", 1, "a"));
        d.record(snap("s2", 2, "b"));
        assert_eq!(d.export_jsonl().len(), 2);
    }

    #[test]
    fn test_diff_prompt_has_changes() {
        let mut d = TemporalDebugger::new();
        let mut s1 = snap("s1", 1, "init"); s1.add_var("y", VarValue::Int(0)); d.record(s1);
        let mut s2 = snap("s2", 2, "run"); s2.add_var("y", VarValue::Int(42)); d.record(s2);
        let prompt = d.diff_prompt("s1", "s2").unwrap();
        assert!(prompt.contains("y"));
        assert!(prompt.contains("42"));
    }

    #[test]
    fn test_diff_prompt_no_changes_message() {
        let mut d = TemporalDebugger::new();
        d.record(snap("s1", 1, "same"));
        d.record(snap("s2", 2, "same"));
        let prompt = d.diff_prompt("s1", "s2").unwrap();
        assert!(prompt.contains("No changes"));
    }
}
