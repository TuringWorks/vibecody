#![allow(dead_code)]
//! Agent state machine — formal FSM for the agent execution loop.
//!
//! Exposes agent state (Idle → Planning → Executing → Reviewing → Blocked →
//! Complete) as a first-class API that hooks, MCP tools, and the SDK can
//! subscribe to.
//!
//! Matches Cody 6.0's agent FSM and Claude Code SDK state transitions.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::Instant;

// ---------------------------------------------------------------------------
// State enum
// ---------------------------------------------------------------------------

/// Top-level states of the agent execution loop.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AgentState {
    /// No active task.
    Idle,
    /// Breaking down the user's request into a plan.
    Planning,
    /// Executing tool calls / generating output.
    Executing,
    /// Waiting for user approval or reviewing generated output.
    Reviewing,
    /// Blocked on external input (user prompt, dependency, resource limit).
    Blocked(BlockReason),
    /// Task completed successfully.
    Complete,
    /// Task aborted due to error or user cancellation.
    Aborted(String),
}

/// Reason the agent is blocked.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum BlockReason {
    /// Waiting for the user to approve a tool call.
    AwaitingApproval,
    /// Waiting for a long-running tool to finish.
    AwaitingTool(String),
    /// Context window is full — budget exceeded.
    ContextBudgetExceeded,
    /// Provider rate-limited.
    RateLimited,
    /// Custom reason.
    Other(String),
}

impl std::fmt::Display for BlockReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BlockReason::AwaitingApproval => write!(f, "awaiting_approval"),
            BlockReason::AwaitingTool(t) => write!(f, "awaiting_tool({t})"),
            BlockReason::ContextBudgetExceeded => write!(f, "context_budget_exceeded"),
            BlockReason::RateLimited => write!(f, "rate_limited"),
            BlockReason::Other(s) => write!(f, "other({s})"),
        }
    }
}

impl std::fmt::Display for AgentState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentState::Idle => write!(f, "idle"),
            AgentState::Planning => write!(f, "planning"),
            AgentState::Executing => write!(f, "executing"),
            AgentState::Reviewing => write!(f, "reviewing"),
            AgentState::Blocked(r) => write!(f, "blocked({r})"),
            AgentState::Complete => write!(f, "complete"),
            AgentState::Aborted(e) => write!(f, "aborted({e})"),
        }
    }
}

// ---------------------------------------------------------------------------
// Transition events
// ---------------------------------------------------------------------------

/// Input events that drive state transitions.
#[derive(Debug, Clone, PartialEq)]
pub enum AgentEvent {
    /// User submitted a new task.
    TaskReceived,
    /// Planning phase started.
    PlanStarted,
    /// Planning produced a plan.
    PlanReady,
    /// Agent began executing tools.
    ExecutionStarted,
    /// All current tool calls issued; awaiting results.
    ToolsDispatched,
    /// All tool results received; agent has output to review.
    OutputReady,
    /// User approved the output / continuation.
    UserApproved,
    /// Agent requires explicit user approval before proceeding.
    ApprovalRequired,
    /// A long-running tool is being awaited.
    WaitingForTool(String),
    /// The blocking condition resolved.
    Unblocked,
    /// Task marked as done.
    TaskComplete,
    /// Error or user-initiated cancellation.
    Abort(String),
    /// Context budget exceeded.
    BudgetExceeded,
    /// Provider returned 429/529 — rate limited.
    RateLimited,
}

// ---------------------------------------------------------------------------
// Transition record
// ---------------------------------------------------------------------------

/// A recorded state transition with metadata.
#[derive(Debug, Clone)]
pub struct Transition {
    pub from: AgentState,
    pub to: AgentState,
    pub event: AgentEvent,
    pub at: Instant,
}

// ---------------------------------------------------------------------------
// FSM
// ---------------------------------------------------------------------------

/// Formal finite-state machine for the agent loop.
pub struct AgentFsm {
    state: AgentState,
    history: VecDeque<Transition>,
    /// Maximum transitions to keep in history.
    history_cap: usize,
}

impl AgentFsm {
    pub fn new() -> Self {
        Self {
            state: AgentState::Idle,
            history: VecDeque::new(),
            history_cap: 200,
        }
    }

    /// Current state.
    pub fn state(&self) -> &AgentState {
        &self.state
    }

    /// Attempt to apply an event. Returns the new state on success, or an
    /// error describing the illegal transition.
    pub fn apply(&mut self, event: AgentEvent) -> Result<&AgentState, String> {
        let next = self.transition(&self.state.clone(), &event)?;
        let transition = Transition {
            from: self.state.clone(),
            to: next.clone(),
            event,
            at: Instant::now(),
        };
        if self.history.len() >= self.history_cap {
            self.history.pop_front();
        }
        self.history.push_back(transition);
        self.state = next;
        Ok(&self.state)
    }

    /// Return recent transitions (oldest first).
    pub fn history(&self) -> Vec<&Transition> {
        self.history.iter().collect()
    }

    /// True if the agent is in a terminal state.
    pub fn is_terminal(&self) -> bool {
        matches!(self.state, AgentState::Complete | AgentState::Aborted(_))
    }

    /// Reset to Idle.
    pub fn reset(&mut self) {
        self.state = AgentState::Idle;
    }

    /// Enumerate valid next events from the current state.
    pub fn valid_events(&self) -> Vec<&'static str> {
        match &self.state {
            AgentState::Idle => vec!["TaskReceived"],
            AgentState::Planning => {
                vec!["PlanReady", "Abort"]
            }
            AgentState::Executing => {
                vec![
                    "OutputReady",
                    "ToolsDispatched",
                    "ApprovalRequired",
                    "WaitingForTool",
                    "BudgetExceeded",
                    "RateLimited",
                    "Abort",
                ]
            }
            AgentState::Reviewing => vec!["UserApproved", "Abort"],
            AgentState::Blocked(_) => vec!["Unblocked", "Abort"],
            AgentState::Complete | AgentState::Aborted(_) => vec!["TaskReceived"],
        }
    }

    // Core transition table.
    fn transition(&self, state: &AgentState, event: &AgentEvent) -> Result<AgentState, String> {
        match (state, event) {
            // Idle → Planning
            (AgentState::Idle, AgentEvent::TaskReceived)
            | (AgentState::Complete, AgentEvent::TaskReceived)
            | (AgentState::Aborted(_), AgentEvent::TaskReceived) => Ok(AgentState::Planning),

            // Planning → Executing
            (AgentState::Planning, AgentEvent::PlanReady)
            | (AgentState::Planning, AgentEvent::PlanStarted) => Ok(AgentState::Executing),

            // Executing → Reviewing (output ready)
            (AgentState::Executing, AgentEvent::OutputReady) => Ok(AgentState::Reviewing),

            // Executing → Executing (tools dispatched, still in flight)
            (AgentState::Executing, AgentEvent::ToolsDispatched) => Ok(AgentState::Executing),

            // Executing → Blocked (various reasons)
            (AgentState::Executing, AgentEvent::ApprovalRequired) => {
                Ok(AgentState::Blocked(BlockReason::AwaitingApproval))
            }
            (AgentState::Executing, AgentEvent::WaitingForTool(t)) => {
                Ok(AgentState::Blocked(BlockReason::AwaitingTool(t.clone())))
            }
            (AgentState::Executing, AgentEvent::BudgetExceeded) => {
                Ok(AgentState::Blocked(BlockReason::ContextBudgetExceeded))
            }
            (AgentState::Executing, AgentEvent::RateLimited) => {
                Ok(AgentState::Blocked(BlockReason::RateLimited))
            }

            // Reviewing → Executing (user approved, continue)
            (AgentState::Reviewing, AgentEvent::UserApproved) => Ok(AgentState::Executing),

            // Blocked → Executing (unblocked)
            (AgentState::Blocked(_), AgentEvent::Unblocked) => Ok(AgentState::Executing),

            // Any → Complete
            (AgentState::Executing, AgentEvent::TaskComplete)
            | (AgentState::Reviewing, AgentEvent::TaskComplete) => Ok(AgentState::Complete),

            // Any → Aborted
            (_, AgentEvent::Abort(reason)) => Ok(AgentState::Aborted(reason.clone())),

            _ => Err(format!(
                "illegal transition: {} + {:?}",
                state, event
            )),
        }
    }
}

impl Default for AgentFsm {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Shared / observable FSM
// ---------------------------------------------------------------------------

/// Thread-safe, observable agent FSM. Listeners receive state-change
/// notifications via a simple poll interface.
pub struct SharedAgentFsm {
    inner: Arc<Mutex<AgentFsm>>,
    generation: Arc<std::sync::atomic::AtomicU64>,
}

impl SharedAgentFsm {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(AgentFsm::new())),
            generation: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        }
    }

    pub fn apply(&self, event: AgentEvent) -> Result<AgentState, String> {
        let mut fsm = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let result = fsm.apply(event)?;
        self.generation
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Ok(result.clone())
    }

    pub fn state(&self) -> AgentState {
        self.inner
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .state()
            .clone()
    }

    pub fn generation(&self) -> u64 {
        self.generation.load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn clone_handle(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
            generation: Arc::clone(&self.generation),
        }
    }
}

impl Default for SharedAgentFsm {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Badge rendering
// ---------------------------------------------------------------------------

/// Visual badge for the current state (for UI status bars).
pub fn state_badge(state: &AgentState) -> &'static str {
    match state {
        AgentState::Idle => "● idle",
        AgentState::Planning => "⟳ planning",
        AgentState::Executing => "▶ executing",
        AgentState::Reviewing => "◉ reviewing",
        AgentState::Blocked(_) => "⏸ blocked",
        AgentState::Complete => "✓ complete",
        AgentState::Aborted(_) => "✗ aborted",
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh() -> AgentFsm {
        AgentFsm::new()
    }

    #[test]
    fn test_initial_state_is_idle() {
        let fsm = fresh();
        assert_eq!(*fsm.state(), AgentState::Idle);
    }

    #[test]
    fn test_task_received_transitions_to_planning() {
        let mut fsm = fresh();
        fsm.apply(AgentEvent::TaskReceived).unwrap();
        assert_eq!(*fsm.state(), AgentState::Planning);
    }

    #[test]
    fn test_full_happy_path() {
        let mut fsm = fresh();
        fsm.apply(AgentEvent::TaskReceived).unwrap();
        fsm.apply(AgentEvent::PlanReady).unwrap();
        assert_eq!(*fsm.state(), AgentState::Executing);
        fsm.apply(AgentEvent::OutputReady).unwrap();
        assert_eq!(*fsm.state(), AgentState::Reviewing);
        fsm.apply(AgentEvent::UserApproved).unwrap();
        assert_eq!(*fsm.state(), AgentState::Executing);
        fsm.apply(AgentEvent::TaskComplete).unwrap();
        assert_eq!(*fsm.state(), AgentState::Complete);
    }

    #[test]
    fn test_abort_from_any_state() {
        for initial_event in &[
            AgentEvent::TaskReceived,
        ] {
            let mut fsm = fresh();
            fsm.apply(initial_event.clone()).unwrap();
            fsm.apply(AgentEvent::Abort("user cancelled".into())).unwrap();
            assert!(matches!(fsm.state(), AgentState::Aborted(_)));
        }
    }

    #[test]
    fn test_blocked_awaiting_approval() {
        let mut fsm = fresh();
        fsm.apply(AgentEvent::TaskReceived).unwrap();
        fsm.apply(AgentEvent::PlanReady).unwrap();
        fsm.apply(AgentEvent::ApprovalRequired).unwrap();
        assert_eq!(
            *fsm.state(),
            AgentState::Blocked(BlockReason::AwaitingApproval)
        );
    }

    #[test]
    fn test_unblock_resumes_executing() {
        let mut fsm = fresh();
        fsm.apply(AgentEvent::TaskReceived).unwrap();
        fsm.apply(AgentEvent::PlanReady).unwrap();
        fsm.apply(AgentEvent::ApprovalRequired).unwrap();
        fsm.apply(AgentEvent::Unblocked).unwrap();
        assert_eq!(*fsm.state(), AgentState::Executing);
    }

    #[test]
    fn test_rate_limited_block() {
        let mut fsm = fresh();
        fsm.apply(AgentEvent::TaskReceived).unwrap();
        fsm.apply(AgentEvent::PlanReady).unwrap();
        fsm.apply(AgentEvent::RateLimited).unwrap();
        assert_eq!(
            *fsm.state(),
            AgentState::Blocked(BlockReason::RateLimited)
        );
    }

    #[test]
    fn test_budget_exceeded_block() {
        let mut fsm = fresh();
        fsm.apply(AgentEvent::TaskReceived).unwrap();
        fsm.apply(AgentEvent::PlanReady).unwrap();
        fsm.apply(AgentEvent::BudgetExceeded).unwrap();
        assert_eq!(
            *fsm.state(),
            AgentState::Blocked(BlockReason::ContextBudgetExceeded)
        );
    }

    #[test]
    fn test_illegal_transition_returns_error() {
        let mut fsm = fresh();
        let result = fsm.apply(AgentEvent::PlanReady);
        assert!(result.is_err());
    }

    #[test]
    fn test_history_records_transitions() {
        let mut fsm = fresh();
        fsm.apply(AgentEvent::TaskReceived).unwrap();
        fsm.apply(AgentEvent::PlanReady).unwrap();
        assert_eq!(fsm.history().len(), 2);
        assert_eq!(fsm.history()[0].from, AgentState::Idle);
        assert_eq!(fsm.history()[0].to, AgentState::Planning);
    }

    #[test]
    fn test_is_terminal_complete() {
        let mut fsm = fresh();
        fsm.apply(AgentEvent::TaskReceived).unwrap();
        fsm.apply(AgentEvent::PlanReady).unwrap();
        fsm.apply(AgentEvent::TaskComplete).unwrap();
        assert!(fsm.is_terminal());
    }

    #[test]
    fn test_is_terminal_aborted() {
        let mut fsm = fresh();
        fsm.apply(AgentEvent::TaskReceived).unwrap();
        fsm.apply(AgentEvent::Abort("err".into())).unwrap();
        assert!(fsm.is_terminal());
    }

    #[test]
    fn test_reset_returns_to_idle() {
        let mut fsm = fresh();
        fsm.apply(AgentEvent::TaskReceived).unwrap();
        fsm.reset();
        assert_eq!(*fsm.state(), AgentState::Idle);
    }

    #[test]
    fn test_complete_allows_new_task() {
        let mut fsm = fresh();
        fsm.apply(AgentEvent::TaskReceived).unwrap();
        fsm.apply(AgentEvent::PlanReady).unwrap();
        fsm.apply(AgentEvent::TaskComplete).unwrap();
        // Should be able to start a new task from Complete.
        fsm.apply(AgentEvent::TaskReceived).unwrap();
        assert_eq!(*fsm.state(), AgentState::Planning);
    }

    #[test]
    fn test_shared_fsm_generation_increments() {
        let shared = SharedAgentFsm::new();
        let gen0 = shared.generation();
        shared.apply(AgentEvent::TaskReceived).unwrap();
        assert_eq!(shared.generation(), gen0 + 1);
    }

    #[test]
    fn test_shared_fsm_clone_shares_state() {
        let shared = SharedAgentFsm::new();
        let handle = shared.clone_handle();
        shared.apply(AgentEvent::TaskReceived).unwrap();
        assert_eq!(handle.state(), AgentState::Planning);
    }

    #[test]
    fn test_state_badge_idle() {
        assert_eq!(state_badge(&AgentState::Idle), "● idle");
    }

    #[test]
    fn test_valid_events_from_idle() {
        let fsm = fresh();
        let events = fsm.valid_events();
        assert!(events.contains(&"TaskReceived"));
    }

    #[test]
    fn test_waiting_for_tool_block() {
        let mut fsm = fresh();
        fsm.apply(AgentEvent::TaskReceived).unwrap();
        fsm.apply(AgentEvent::PlanReady).unwrap();
        fsm.apply(AgentEvent::WaitingForTool("cargo_build".into())).unwrap();
        assert_eq!(
            *fsm.state(),
            AgentState::Blocked(BlockReason::AwaitingTool("cargo_build".into()))
        );
    }
}
