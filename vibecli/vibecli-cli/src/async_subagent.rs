//! Async subagent state machine — long-running, check-back-later
//! subagents distinct from the synchronous-oversight pool in
//! `nested_agents.rs`.
//!
//! Phase 53 P1 (A5 from v13 fitgap, Cursor 3.2). Async subagents
//! persist their state across the parent agent's idle periods so the
//! parent can `/agents resume <id>` after the user closes their tab.
//! This module ships the state machine half: `register` to create,
//! `mark_running` / `mark_completed` / `mark_failed` / `cancel` to
//! drive transitions, `poll` to read state.
//!
//! Persistence wiring (SQLite-backed log of state transitions) is the
//! follow-up that consumes this state machine — keeping them split
//! lets the state-machine logic stay pure-Rust + unit-testable while
//! the persistence half tracks the broader session_store work.
//!
//! Transitions:
//!   Pending  → Running | Cancelled
//!   Running  → Completed | Failed | Cancelled
//!   *terminal*  → no further transitions allowed

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Lifecycle states. Transitions are constrained — see the BDD
/// scenarios for the full diagram.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubagentState {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

impl SubagentState {
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            SubagentState::Completed | SubagentState::Failed | SubagentState::Cancelled
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubagentRecord {
    pub id: String,
    pub task: String,
    pub state: SubagentState,
    pub created_at_unix: u64,
    pub updated_at_unix: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TransitionError {
    Unknown(String),
    InvalidTransition {
        id: String,
        from: SubagentState,
        to: SubagentState,
    },
}

impl std::fmt::Display for TransitionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TransitionError::Unknown(id) => write!(f, "unknown subagent id: {id}"),
            TransitionError::InvalidTransition { id, from, to } => write!(
                f,
                "invalid transition for {id}: {from:?} → {to:?} not allowed"
            ),
        }
    }
}

impl std::error::Error for TransitionError {}

#[derive(Debug, Clone, Default)]
pub struct AsyncSubagentRegistry {
    state: Arc<Mutex<RegistryState>>,
}

#[derive(Debug, Default)]
struct RegistryState {
    next_seq: u64,
    records: BTreeMap<String, SubagentRecord>,
}

impl AsyncSubagentRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&self, task: &str) -> Result<String> {
        let mut state = self
            .state
            .lock()
            .map_err(|e| anyhow::anyhow!("registry lock poisoned: {e}"))?;
        state.next_seq += 1;
        let id = format!("subagent-{:08x}", state.next_seq);
        let now = now_unix();
        state.records.insert(
            id.clone(),
            SubagentRecord {
                id: id.clone(),
                task: task.to_string(),
                state: SubagentState::Pending,
                created_at_unix: now,
                updated_at_unix: now,
                result: None,
                error: None,
            },
        );
        Ok(id)
    }

    pub fn mark_running(&self, id: &str) -> std::result::Result<(), TransitionError> {
        self.transition(id, |rec| {
            if rec.state == SubagentState::Pending {
                rec.state = SubagentState::Running;
                Ok(())
            } else {
                Err(SubagentState::Running)
            }
        })
    }

    pub fn mark_completed(
        &self,
        id: &str,
        result: String,
    ) -> std::result::Result<(), TransitionError> {
        self.transition(id, |rec| {
            if rec.state.is_terminal() {
                Err(SubagentState::Completed)
            } else {
                rec.state = SubagentState::Completed;
                rec.result = Some(result);
                Ok(())
            }
        })
    }

    pub fn mark_failed(
        &self,
        id: &str,
        error: String,
    ) -> std::result::Result<(), TransitionError> {
        self.transition(id, |rec| {
            if rec.state.is_terminal() {
                Err(SubagentState::Failed)
            } else {
                rec.state = SubagentState::Failed;
                rec.error = Some(error);
                Ok(())
            }
        })
    }

    pub fn cancel(&self, id: &str) -> std::result::Result<(), TransitionError> {
        self.transition(id, |rec| {
            if rec.state.is_terminal() {
                Err(SubagentState::Cancelled)
            } else {
                rec.state = SubagentState::Cancelled;
                Ok(())
            }
        })
    }

    pub fn poll(&self, id: &str) -> Option<SubagentRecord> {
        self.state
            .lock()
            .ok()
            .and_then(|s| s.records.get(id).cloned())
    }

    pub fn pending_or_running(&self) -> Vec<SubagentRecord> {
        self.state
            .lock()
            .map(|s| {
                s.records
                    .values()
                    .filter(|r| !r.state.is_terminal())
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    fn transition<F>(&self, id: &str, mutator: F) -> std::result::Result<(), TransitionError>
    where
        F: FnOnce(&mut SubagentRecord) -> std::result::Result<(), SubagentState>,
    {
        let mut state = self
            .state
            .lock()
            .map_err(|e| TransitionError::Unknown(format!("lock poisoned: {e}")))?;
        let rec = state
            .records
            .get_mut(id)
            .ok_or_else(|| TransitionError::Unknown(id.to_string()))?;
        let from = rec.state;
        match mutator(rec) {
            Ok(()) => {
                rec.updated_at_unix = now_unix();
                Ok(())
            }
            Err(to) => Err(TransitionError::InvalidTransition {
                id: id.to_string(),
                from,
                to,
            }),
        }
    }
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_creates_pending_record_with_unique_id() {
        let r = AsyncSubagentRegistry::new();
        let id1 = r.register("task one").unwrap();
        let id2 = r.register("task two").unwrap();
        assert_ne!(id1, id2);
        assert_eq!(r.poll(&id1).unwrap().state, SubagentState::Pending);
        assert_eq!(r.poll(&id2).unwrap().task, "task two");
    }

    #[test]
    fn pending_to_running_to_completed_is_allowed() {
        let r = AsyncSubagentRegistry::new();
        let id = r.register("t").unwrap();
        r.mark_running(&id).unwrap();
        assert_eq!(r.poll(&id).unwrap().state, SubagentState::Running);
        r.mark_completed(&id, "done".into()).unwrap();
        let rec = r.poll(&id).unwrap();
        assert_eq!(rec.state, SubagentState::Completed);
        assert_eq!(rec.result.as_deref(), Some("done"));
    }

    #[test]
    fn cannot_transition_terminal_state() {
        let r = AsyncSubagentRegistry::new();
        let id = r.register("t").unwrap();
        r.mark_running(&id).unwrap();
        r.mark_completed(&id, "ok".into()).unwrap();
        let err = r.mark_failed(&id, "oops".into()).unwrap_err();
        assert!(matches!(err, TransitionError::InvalidTransition { .. }));
        let err = r.cancel(&id).unwrap_err();
        assert!(matches!(err, TransitionError::InvalidTransition { .. }));
    }

    #[test]
    fn cancel_works_from_pending_or_running() {
        let r = AsyncSubagentRegistry::new();
        let id1 = r.register("a").unwrap();
        r.cancel(&id1).unwrap();
        assert_eq!(r.poll(&id1).unwrap().state, SubagentState::Cancelled);

        let id2 = r.register("b").unwrap();
        r.mark_running(&id2).unwrap();
        r.cancel(&id2).unwrap();
        assert_eq!(r.poll(&id2).unwrap().state, SubagentState::Cancelled);
    }

    #[test]
    fn pending_or_running_filters_terminal() {
        let r = AsyncSubagentRegistry::new();
        let pending = r.register("pending").unwrap();
        let running = r.register("running").unwrap();
        r.mark_running(&running).unwrap();
        let done = r.register("done").unwrap();
        r.mark_running(&done).unwrap();
        r.mark_completed(&done, "ok".into()).unwrap();
        let cancelled = r.register("cancelled").unwrap();
        r.cancel(&cancelled).unwrap();

        let active = r.pending_or_running();
        let active_ids: Vec<&str> = active.iter().map(|x| x.id.as_str()).collect();
        assert!(active_ids.contains(&pending.as_str()));
        assert!(active_ids.contains(&running.as_str()));
        assert!(!active_ids.contains(&done.as_str()));
        assert!(!active_ids.contains(&cancelled.as_str()));
    }

    #[test]
    fn unknown_id_returns_unknown_error() {
        let r = AsyncSubagentRegistry::new();
        let err = r.mark_running("does-not-exist").unwrap_err();
        match err {
            TransitionError::Unknown(id) => assert_eq!(id, "does-not-exist"),
            other => panic!("expected Unknown, got {other:?}"),
        }
        assert!(r.poll("does-not-exist").is_none());
    }
}
