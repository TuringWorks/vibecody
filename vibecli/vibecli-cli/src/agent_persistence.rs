//! Agent persistence — serialize and restore running agent state across restarts.
//! Matches Claude Code 1.x's background agent persistence and Cursor 4.0's
//! session resume feature.
//!
//! Agents checkpoint their state as JSON snapshots. On restart, they can
//! reload the last checkpoint and continue from where they left off.

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// A serializable snapshot of an agent's state at a point in time.
#[derive(Debug, Clone)]
pub struct AgentSnapshot {
    pub agent_id: String,
    pub session_id: String,
    pub checkpoint_id: u64,
    pub timestamp_ms: u64,
    /// Key/value map of arbitrary state fields.
    pub state: HashMap<String, StateValue>,
    /// Pending tool calls that were in flight.
    pub pending_tool_calls: Vec<PendingToolCall>,
    /// Accumulated conversation context (last N messages as JSON-ish strings).
    pub context_summary: Vec<String>,
    pub current_task: Option<String>,
    pub step_count: usize,
    pub metadata: HashMap<String, String>,
}

impl AgentSnapshot {
    pub fn new(agent_id: impl Into<String>, session_id: impl Into<String>) -> Self {
        Self {
            agent_id: agent_id.into(),
            session_id: session_id.into(),
            checkpoint_id: 0,
            timestamp_ms: now_ms(),
            state: HashMap::new(),
            pending_tool_calls: Vec::new(),
            context_summary: Vec::new(),
            current_task: None,
            step_count: 0,
            metadata: HashMap::new(),
        }
    }
}

/// A typed state value (no external JSON dep — minimal type set).
#[derive(Debug, Clone, PartialEq)]
pub enum StateValue {
    Str(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    List(Vec<String>),
    Null,
}

impl StateValue {
    pub fn as_str(&self) -> Option<&str> {
        if let StateValue::Str(s) = self { Some(s) } else { None }
    }
    pub fn as_int(&self) -> Option<i64> {
        if let StateValue::Int(n) = self { Some(*n) } else { None }
    }
    pub fn as_bool(&self) -> Option<bool> {
        if let StateValue::Bool(b) = self { Some(*b) } else { None }
    }
}

/// A tool call that was pending when the snapshot was taken.
#[derive(Debug, Clone)]
pub struct PendingToolCall {
    pub tool_name: String,
    pub args: String,
    pub call_id: String,
}

// ---------------------------------------------------------------------------
// Snapshot Store
// ---------------------------------------------------------------------------

/// In-memory checkpoint store keyed by (agent_id, session_id).
pub struct SnapshotStore {
    /// agent_id → session_id → vec of snapshots (ordered, newest last)
    snapshots: HashMap<String, HashMap<String, Vec<AgentSnapshot>>>,
    /// Max checkpoints to retain per session.
    pub max_checkpoints: usize,
}

impl Default for SnapshotStore {
    fn default() -> Self { Self::new(10) }
}

impl SnapshotStore {
    pub fn new(max_checkpoints: usize) -> Self {
        Self { snapshots: HashMap::new(), max_checkpoints }
    }

    /// Save a snapshot. Assigns the next checkpoint_id.
    pub fn save(&mut self, mut snapshot: AgentSnapshot) -> u64 {
        let sessions = self.snapshots.entry(snapshot.agent_id.clone()).or_default();
        let checkpoints = sessions.entry(snapshot.session_id.clone()).or_default();

        let next_id = checkpoints.last().map(|s| s.checkpoint_id + 1).unwrap_or(1);
        snapshot.checkpoint_id = next_id;
        snapshot.timestamp_ms = now_ms();
        checkpoints.push(snapshot);

        // Trim to max
        while checkpoints.len() > self.max_checkpoints {
            checkpoints.remove(0);
        }
        next_id
    }

    /// Load the latest snapshot for (agent_id, session_id).
    pub fn load_latest(&self, agent_id: &str, session_id: &str) -> Option<&AgentSnapshot> {
        self.snapshots.get(agent_id)?.get(session_id)?.last()
    }

    /// Load a specific checkpoint by ID.
    pub fn load_checkpoint(&self, agent_id: &str, session_id: &str, checkpoint_id: u64) -> Option<&AgentSnapshot> {
        self.snapshots.get(agent_id)?.get(session_id)?
            .iter().find(|s| s.checkpoint_id == checkpoint_id)
    }

    /// List all checkpoint IDs for a session.
    pub fn list_checkpoints(&self, agent_id: &str, session_id: &str) -> Vec<u64> {
        self.snapshots.get(agent_id)
            .and_then(|s| s.get(session_id))
            .map(|cs| cs.iter().map(|s| s.checkpoint_id).collect())
            .unwrap_or_default()
    }

    /// Delete all checkpoints for a session.
    pub fn clear_session(&mut self, agent_id: &str, session_id: &str) {
        if let Some(sessions) = self.snapshots.get_mut(agent_id) {
            sessions.remove(session_id);
        }
    }

    /// Total snapshots stored.
    pub fn total_snapshots(&self) -> usize {
        self.snapshots.values().flat_map(|s| s.values()).map(|v| v.len()).sum()
    }
}

// ---------------------------------------------------------------------------
// Checkpoint builder
// ---------------------------------------------------------------------------

/// Fluent builder for constructing snapshots.
pub struct SnapshotBuilder {
    snapshot: AgentSnapshot,
}

impl SnapshotBuilder {
    pub fn new(agent_id: impl Into<String>, session_id: impl Into<String>) -> Self {
        Self { snapshot: AgentSnapshot::new(agent_id, session_id) }
    }

    pub fn state(mut self, key: impl Into<String>, val: StateValue) -> Self {
        self.snapshot.state.insert(key.into(), val);
        self
    }

    pub fn task(mut self, task: impl Into<String>) -> Self {
        self.snapshot.current_task = Some(task.into());
        self
    }

    pub fn steps(mut self, n: usize) -> Self {
        self.snapshot.step_count = n;
        self
    }

    pub fn context(mut self, lines: Vec<String>) -> Self {
        self.snapshot.context_summary = lines;
        self
    }

    pub fn pending_call(mut self, tool: impl Into<String>, args: impl Into<String>, id: impl Into<String>) -> Self {
        self.snapshot.pending_tool_calls.push(PendingToolCall {
            tool_name: tool.into(),
            args: args.into(),
            call_id: id.into(),
        });
        self
    }

    pub fn meta(mut self, k: impl Into<String>, v: impl Into<String>) -> Self {
        self.snapshot.metadata.insert(k.into(), v.into());
        self
    }

    pub fn build(self) -> AgentSnapshot { self.snapshot }
}

fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn snap(agent: &str, session: &str) -> AgentSnapshot {
        SnapshotBuilder::new(agent, session)
            .state("phase", StateValue::Str("planning".into()))
            .state("step", StateValue::Int(3))
            .task("fix the bug")
            .steps(3)
            .build()
    }

    #[test]
    fn test_save_and_load_latest() {
        let mut store = SnapshotStore::new(5);
        store.save(snap("a1", "sess-1"));
        let loaded = store.load_latest("a1", "sess-1").unwrap();
        assert_eq!(loaded.agent_id, "a1");
        assert_eq!(loaded.checkpoint_id, 1);
    }

    #[test]
    fn test_checkpoint_ids_increment() {
        let mut store = SnapshotStore::new(5);
        let id1 = store.save(snap("a1", "s1"));
        let id2 = store.save(snap("a1", "s1"));
        assert_eq!(id1, 1);
        assert_eq!(id2, 2);
    }

    #[test]
    fn test_load_specific_checkpoint() {
        let mut store = SnapshotStore::new(5);
        store.save(snap("a1", "s1"));
        store.save(snap("a1", "s1"));
        let cp = store.load_checkpoint("a1", "s1", 1).unwrap();
        assert_eq!(cp.checkpoint_id, 1);
    }

    #[test]
    fn test_max_checkpoints_eviction() {
        let mut store = SnapshotStore::new(3);
        for _ in 0..5 {
            store.save(snap("a1", "s1"));
        }
        assert_eq!(store.list_checkpoints("a1", "s1").len(), 3);
        // Oldest should be evicted (id 1 and 2 gone)
        assert_eq!(store.list_checkpoints("a1", "s1"), vec![3, 4, 5]);
    }

    #[test]
    fn test_clear_session() {
        let mut store = SnapshotStore::new(5);
        store.save(snap("a1", "s1"));
        store.clear_session("a1", "s1");
        assert!(store.load_latest("a1", "s1").is_none());
    }

    #[test]
    fn test_state_values() {
        let snap = SnapshotBuilder::new("a", "s")
            .state("s", StateValue::Str("hello".into()))
            .state("n", StateValue::Int(42))
            .state("b", StateValue::Bool(true))
            .build();
        assert_eq!(snap.state["s"].as_str(), Some("hello"));
        assert_eq!(snap.state["n"].as_int(), Some(42));
        assert_eq!(snap.state["b"].as_bool(), Some(true));
    }

    #[test]
    fn test_pending_tool_calls() {
        let snap = SnapshotBuilder::new("a", "s")
            .pending_call("read_file", "main.rs", "call-1")
            .build();
        assert_eq!(snap.pending_tool_calls.len(), 1);
        assert_eq!(snap.pending_tool_calls[0].tool_name, "read_file");
    }

    #[test]
    fn test_multiple_sessions_independent() {
        let mut store = SnapshotStore::new(5);
        store.save(snap("a1", "s1"));
        store.save(snap("a1", "s2"));
        assert_eq!(store.load_latest("a1", "s1").unwrap().checkpoint_id, 1);
        assert_eq!(store.load_latest("a1", "s2").unwrap().checkpoint_id, 1);
    }

    #[test]
    fn test_total_snapshots() {
        let mut store = SnapshotStore::new(10);
        store.save(snap("a1", "s1"));
        store.save(snap("a1", "s1"));
        store.save(snap("a2", "s1"));
        assert_eq!(store.total_snapshots(), 3);
    }

    #[test]
    fn test_load_nonexistent_returns_none() {
        let store = SnapshotStore::new(5);
        assert!(store.load_latest("ghost", "session").is_none());
    }

    #[test]
    fn test_context_summary() {
        let snap = SnapshotBuilder::new("a", "s")
            .context(vec!["user: fix bug".into(), "assistant: reading...".into()])
            .build();
        assert_eq!(snap.context_summary.len(), 2);
    }

    #[test]
    fn test_metadata() {
        let snap = SnapshotBuilder::new("a", "s").meta("region", "us-east").build();
        assert_eq!(snap.metadata.get("region").unwrap(), "us-east");
    }
}
