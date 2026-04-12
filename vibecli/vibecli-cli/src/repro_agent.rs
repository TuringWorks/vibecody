//! Reproduction agent — snapshot, trace, diff, and reproducibility verification.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ─── LockFileType ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LockFileType {
    Cargo,
    Npm,
    Pip,
    Yarn,
    Pnpm,
    Go,
    Other(String),
}

// ─── LockFileEntry ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockFileEntry {
    pub lock_type: LockFileType,
    pub path: String,
    pub content_hash: String,
}

// ─── EnvVarEntry ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvVarEntry {
    pub key: String,
    /// Value is hashed for privacy.
    pub value_hash: String,
}

// ─── ReproSnapshot ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReproSnapshot {
    pub snapshot_id: String,
    pub session_id: String,
    pub lock_files: Vec<LockFileEntry>,
    pub env_vars: Vec<EnvVarEntry>,
    pub os_version: String,
    pub rust_version: Option<String>,
    pub random_seed: u64,
    pub created_at_ms: u64,
}

impl ReproSnapshot {
    /// Produces a stable fingerprint from the snapshot_id and all lock file content hashes.
    pub fn fingerprint(&self) -> String {
        let mut parts = vec![self.snapshot_id.clone()];
        for lf in &self.lock_files {
            parts.push(lf.content_hash.clone());
        }
        simple_hash(&parts.join("|"))
    }
}

// ─── ToolCallRecord ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallRecord {
    pub call_id: String,
    pub tool_name: String,
    pub inputs_json: String,
    pub output_json: String,
    pub is_deterministic: bool,
    pub timestamp_ms: u64,
}

// ─── SessionTrace ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionTrace {
    pub trace_id: String,
    pub session_id: String,
    pub snapshot_id: String,
    pub tool_calls: Vec<ToolCallRecord>,
    pub created_at_ms: u64,
}

impl SessionTrace {
    pub fn add_call(&mut self, call: ToolCallRecord) {
        self.tool_calls.push(call);
    }

    /// Returns references to non-deterministic tool calls.
    pub fn non_deterministic_tools(&self) -> Vec<&ToolCallRecord> {
        self.tool_calls
            .iter()
            .filter(|c| !c.is_deterministic)
            .collect()
    }

    pub fn call_count(&self) -> usize {
        self.tool_calls.len()
    }

    /// Returns a hash of all output_json values concatenated in order.
    pub fn output_hash(&self) -> String {
        let concatenated: String = self
            .tool_calls
            .iter()
            .map(|c| c.output_json.as_str())
            .collect::<Vec<_>>()
            .join("");
        simple_hash(&concatenated)
    }
}

// ─── SessionDiff ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionDiff {
    pub added_calls: Vec<String>,
    pub removed_calls: Vec<String>,
    pub changed_calls: Vec<String>,
    pub output_hash_match: bool,
}

impl SessionDiff {
    /// True when there are no diffs and the output hashes match.
    pub fn is_identical(&self) -> bool {
        self.added_calls.is_empty()
            && self.removed_calls.is_empty()
            && self.changed_calls.is_empty()
            && self.output_hash_match
    }
}

// ─── ReproEngine ─────────────────────────────────────────────────────────────

#[derive(Debug, Default)]
pub struct ReproEngine {
    snapshots: HashMap<String, ReproSnapshot>,
    traces: HashMap<String, SessionTrace>,
}

impl ReproEngine {
    pub fn new() -> Self {
        Self {
            snapshots: HashMap::new(),
            traces: HashMap::new(),
        }
    }

    pub fn store_snapshot(&mut self, snapshot: ReproSnapshot) {
        self.snapshots
            .insert(snapshot.snapshot_id.clone(), snapshot);
    }

    pub fn store_trace(&mut self, trace: SessionTrace) {
        self.traces.insert(trace.trace_id.clone(), trace);
    }

    pub fn get_snapshot(&self, snapshot_id: &str) -> Option<&ReproSnapshot> {
        self.snapshots.get(snapshot_id)
    }

    pub fn get_trace(&self, trace_id: &str) -> Option<&SessionTrace> {
        self.traces.get(trace_id)
    }

    /// Compares two traces by call_id sets and output hashes.
    pub fn diff_traces(
        &self,
        trace_a_id: &str,
        trace_b_id: &str,
    ) -> Result<SessionDiff, String> {
        let a = self
            .traces
            .get(trace_a_id)
            .ok_or_else(|| format!("trace '{}' not found", trace_a_id))?;
        let b = self
            .traces
            .get(trace_b_id)
            .ok_or_else(|| format!("trace '{}' not found", trace_b_id))?;

        let a_ids: HashMap<&str, &ToolCallRecord> = a
            .tool_calls
            .iter()
            .map(|c| (c.call_id.as_str(), c))
            .collect();
        let b_ids: HashMap<&str, &ToolCallRecord> = b
            .tool_calls
            .iter()
            .map(|c| (c.call_id.as_str(), c))
            .collect();

        let added_calls: Vec<String> = b_ids
            .keys()
            .filter(|id| !a_ids.contains_key(*id))
            .map(|s| s.to_string())
            .collect();

        let removed_calls: Vec<String> = a_ids
            .keys()
            .filter(|id| !b_ids.contains_key(*id))
            .map(|s| s.to_string())
            .collect();

        let changed_calls: Vec<String> = a_ids
            .iter()
            .filter(|(id, a_call)| {
                b_ids
                    .get(*id)
                    .map(|b_call| b_call.output_json != a_call.output_json)
                    .unwrap_or(false)
            })
            .map(|(id, _)| id.to_string())
            .collect();

        let output_hash_match = a.output_hash() == b.output_hash();

        Ok(SessionDiff {
            added_calls,
            removed_calls,
            changed_calls,
            output_hash_match,
        })
    }

    /// Returns true if the trace's output_hash matches the reference hash.
    pub fn verify_reproducibility(&self, trace_id: &str, reference_hash: &str) -> bool {
        match self.traces.get(trace_id) {
            Some(trace) => trace.output_hash() == reference_hash,
            None => false,
        }
    }

    /// Returns the tool names of all non-deterministic calls in the trace.
    pub fn non_determinism_report(&self, trace_id: &str) -> Vec<String> {
        match self.traces.get(trace_id) {
            Some(trace) => trace
                .non_deterministic_tools()
                .iter()
                .map(|c| c.tool_name.clone())
                .collect(),
            None => vec![],
        }
    }

    pub fn snapshot_count(&self) -> usize {
        self.snapshots.len()
    }

    pub fn trace_count(&self) -> usize {
        self.traces.len()
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// Deterministic string hash using FNV-1a (no external deps).
fn simple_hash(input: &str) -> String {
    let mut hash: u64 = 14695981039346656037;
    for byte in input.bytes() {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(1099511628211);
    }
    format!("{:016x}", hash)
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_snapshot(id: &str, hashes: Vec<&str>) -> ReproSnapshot {
        ReproSnapshot {
            snapshot_id: id.to_string(),
            session_id: "sess1".into(),
            lock_files: hashes
                .into_iter()
                .enumerate()
                .map(|(i, h)| LockFileEntry {
                    lock_type: LockFileType::Cargo,
                    path: format!("Cargo.lock.{}", i),
                    content_hash: h.to_string(),
                })
                .collect(),
            env_vars: vec![],
            os_version: "macOS 15".into(),
            rust_version: Some("1.77.0".into()),
            random_seed: 42,
            created_at_ms: 1000,
        }
    }

    fn make_trace(id: &str, calls: Vec<(&str, &str, bool)>) -> SessionTrace {
        SessionTrace {
            trace_id: id.to_string(),
            session_id: "sess1".into(),
            snapshot_id: "snap1".into(),
            tool_calls: calls
                .into_iter()
                .map(|(call_id, output, det)| ToolCallRecord {
                    call_id: call_id.to_string(),
                    tool_name: format!("tool_{}", call_id),
                    inputs_json: "{}".into(),
                    output_json: output.to_string(),
                    is_deterministic: det,
                    timestamp_ms: 0,
                })
                .collect(),
            created_at_ms: 0,
        }
    }

    // ── ReproSnapshot ─────────────────────────────────────────────────────

    #[test]
    fn test_snapshot_fingerprint_not_empty() {
        let s = make_snapshot("s1", vec!["abc", "def"]);
        assert!(!s.fingerprint().is_empty());
    }

    #[test]
    fn test_snapshot_fingerprint_different_for_different_hashes() {
        let s1 = make_snapshot("s1", vec!["abc"]);
        let s2 = make_snapshot("s1", vec!["xyz"]);
        assert_ne!(s1.fingerprint(), s2.fingerprint());
    }

    #[test]
    fn test_snapshot_fingerprint_same_for_same_data() {
        let s1 = make_snapshot("s1", vec!["abc", "def"]);
        let s2 = make_snapshot("s1", vec!["abc", "def"]);
        assert_eq!(s1.fingerprint(), s2.fingerprint());
    }

    #[test]
    fn test_snapshot_fingerprint_changes_with_id() {
        let s1 = make_snapshot("s1", vec!["abc"]);
        let s2 = make_snapshot("s2", vec!["abc"]);
        assert_ne!(s1.fingerprint(), s2.fingerprint());
    }

    #[test]
    fn test_snapshot_no_lock_files() {
        let mut s = make_snapshot("s1", vec![]);
        s.lock_files = vec![];
        assert!(!s.fingerprint().is_empty());
    }

    // ── SessionTrace ──────────────────────────────────────────────────────

    #[test]
    fn test_trace_add_call() {
        let mut t = make_trace("t1", vec![]);
        assert_eq!(t.call_count(), 0);
        t.add_call(ToolCallRecord {
            call_id: "c1".into(),
            tool_name: "read_file".into(),
            inputs_json: "{}".into(),
            output_json: "data".into(),
            is_deterministic: true,
            timestamp_ms: 0,
        });
        assert_eq!(t.call_count(), 1);
    }

    #[test]
    fn test_trace_non_deterministic_tools() {
        let t = make_trace("t1", vec![("c1", "out1", true), ("c2", "out2", false)]);
        let nd = t.non_deterministic_tools();
        assert_eq!(nd.len(), 1);
        assert_eq!(nd[0].call_id, "c2");
    }

    #[test]
    fn test_trace_all_deterministic() {
        let t = make_trace("t1", vec![("c1", "out1", true), ("c2", "out2", true)]);
        assert!(t.non_deterministic_tools().is_empty());
    }

    #[test]
    fn test_trace_call_count() {
        let t = make_trace("t1", vec![("c1", "o1", true), ("c2", "o2", false), ("c3", "o3", true)]);
        assert_eq!(t.call_count(), 3);
    }

    #[test]
    fn test_trace_output_hash_not_empty() {
        let t = make_trace("t1", vec![("c1", "output1", true)]);
        assert!(!t.output_hash().is_empty());
    }

    #[test]
    fn test_trace_output_hash_deterministic() {
        let t1 = make_trace("t1", vec![("c1", "output1", true)]);
        let t2 = make_trace("t1", vec![("c1", "output1", true)]);
        assert_eq!(t1.output_hash(), t2.output_hash());
    }

    #[test]
    fn test_trace_output_hash_changes_with_output() {
        let t1 = make_trace("t1", vec![("c1", "output_A", true)]);
        let t2 = make_trace("t1", vec![("c1", "output_B", true)]);
        assert_ne!(t1.output_hash(), t2.output_hash());
    }

    #[test]
    fn test_trace_empty_output_hash() {
        let t = make_trace("t1", vec![]);
        assert!(!t.output_hash().is_empty());
    }

    // ── SessionDiff ───────────────────────────────────────────────────────

    #[test]
    fn test_session_diff_is_identical() {
        let diff = SessionDiff {
            added_calls: vec![],
            removed_calls: vec![],
            changed_calls: vec![],
            output_hash_match: true,
        };
        assert!(diff.is_identical());
    }

    #[test]
    fn test_session_diff_not_identical_added() {
        let diff = SessionDiff {
            added_calls: vec!["c1".into()],
            removed_calls: vec![],
            changed_calls: vec![],
            output_hash_match: true,
        };
        assert!(!diff.is_identical());
    }

    #[test]
    fn test_session_diff_not_identical_removed() {
        let diff = SessionDiff {
            added_calls: vec![],
            removed_calls: vec!["c1".into()],
            changed_calls: vec![],
            output_hash_match: true,
        };
        assert!(!diff.is_identical());
    }

    #[test]
    fn test_session_diff_not_identical_hash_mismatch() {
        let diff = SessionDiff {
            added_calls: vec![],
            removed_calls: vec![],
            changed_calls: vec![],
            output_hash_match: false,
        };
        assert!(!diff.is_identical());
    }

    // ── ReproEngine ───────────────────────────────────────────────────────

    #[test]
    fn test_engine_new_empty() {
        let e = ReproEngine::new();
        assert_eq!(e.snapshot_count(), 0);
        assert_eq!(e.trace_count(), 0);
    }

    #[test]
    fn test_engine_store_snapshot() {
        let mut e = ReproEngine::new();
        e.store_snapshot(make_snapshot("s1", vec!["abc"]));
        assert_eq!(e.snapshot_count(), 1);
    }

    #[test]
    fn test_engine_store_trace() {
        let mut e = ReproEngine::new();
        e.store_trace(make_trace("t1", vec![]));
        assert_eq!(e.trace_count(), 1);
    }

    #[test]
    fn test_engine_get_snapshot_existing() {
        let mut e = ReproEngine::new();
        e.store_snapshot(make_snapshot("s1", vec!["x"]));
        assert!(e.get_snapshot("s1").is_some());
    }

    #[test]
    fn test_engine_get_snapshot_missing() {
        let e = ReproEngine::new();
        assert!(e.get_snapshot("nope").is_none());
    }

    #[test]
    fn test_engine_get_trace_existing() {
        let mut e = ReproEngine::new();
        e.store_trace(make_trace("t1", vec![]));
        assert!(e.get_trace("t1").is_some());
    }

    #[test]
    fn test_engine_get_trace_missing() {
        let e = ReproEngine::new();
        assert!(e.get_trace("nope").is_none());
    }

    #[test]
    fn test_engine_diff_traces_identical() {
        let mut e = ReproEngine::new();
        e.store_trace(make_trace("t1", vec![("c1", "out1", true)]));
        e.store_trace(make_trace("t2", vec![("c1", "out1", true)]));
        let diff = e.diff_traces("t1", "t2").unwrap();
        assert!(diff.is_identical());
    }

    #[test]
    fn test_engine_diff_traces_added_call() {
        let mut e = ReproEngine::new();
        e.store_trace(make_trace("t1", vec![]));
        e.store_trace(make_trace("t2", vec![("c1", "out1", true)]));
        let diff = e.diff_traces("t1", "t2").unwrap();
        assert!(diff.added_calls.contains(&"c1".to_string()));
    }

    #[test]
    fn test_engine_diff_traces_removed_call() {
        let mut e = ReproEngine::new();
        e.store_trace(make_trace("t1", vec![("c1", "out1", true)]));
        e.store_trace(make_trace("t2", vec![]));
        let diff = e.diff_traces("t1", "t2").unwrap();
        assert!(diff.removed_calls.contains(&"c1".to_string()));
    }

    #[test]
    fn test_engine_diff_traces_changed_call() {
        let mut e = ReproEngine::new();
        e.store_trace(make_trace("t1", vec![("c1", "output_A", true)]));
        e.store_trace(make_trace("t2", vec![("c1", "output_B", true)]));
        let diff = e.diff_traces("t1", "t2").unwrap();
        assert!(diff.changed_calls.contains(&"c1".to_string()));
    }

    #[test]
    fn test_engine_diff_traces_invalid_trace_a() {
        let mut e = ReproEngine::new();
        e.store_trace(make_trace("t2", vec![]));
        let result = e.diff_traces("bad", "t2");
        assert!(result.is_err());
    }

    #[test]
    fn test_engine_diff_traces_invalid_trace_b() {
        let mut e = ReproEngine::new();
        e.store_trace(make_trace("t1", vec![]));
        let result = e.diff_traces("t1", "bad");
        assert!(result.is_err());
    }

    #[test]
    fn test_engine_verify_reproducibility_true() {
        let mut e = ReproEngine::new();
        let t = make_trace("t1", vec![("c1", "output1", true)]);
        let expected_hash = t.output_hash();
        e.store_trace(t);
        assert!(e.verify_reproducibility("t1", &expected_hash));
    }

    #[test]
    fn test_engine_verify_reproducibility_false() {
        let mut e = ReproEngine::new();
        e.store_trace(make_trace("t1", vec![("c1", "output1", true)]));
        assert!(!e.verify_reproducibility("t1", "wrong-hash"));
    }

    #[test]
    fn test_engine_verify_reproducibility_missing_trace() {
        let e = ReproEngine::new();
        assert!(!e.verify_reproducibility("nope", "any"));
    }

    #[test]
    fn test_engine_non_determinism_report() {
        let mut e = ReproEngine::new();
        e.store_trace(make_trace(
            "t1",
            vec![
                ("c1", "o1", true),
                ("c2", "o2", false),
                ("c3", "o3", false),
            ],
        ));
        let report = e.non_determinism_report("t1");
        assert_eq!(report.len(), 2);
    }

    #[test]
    fn test_engine_non_determinism_report_empty_for_missing() {
        let e = ReproEngine::new();
        let report = e.non_determinism_report("nope");
        assert!(report.is_empty());
    }

    #[test]
    fn test_engine_non_determinism_report_contains_tool_names() {
        let mut e = ReproEngine::new();
        let mut trace = make_trace("t1", vec![]);
        trace.add_call(ToolCallRecord {
            call_id: "c1".into(),
            tool_name: "random_picker".into(),
            inputs_json: "{}".into(),
            output_json: "result".into(),
            is_deterministic: false,
            timestamp_ms: 0,
        });
        e.store_trace(trace);
        let report = e.non_determinism_report("t1");
        assert!(report.contains(&"random_picker".to_string()));
    }
}
