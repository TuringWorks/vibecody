//! long_session — Token-budgeted long-running autonomous session management.
//! Tracks per-turn token usage, wall-time, and decides when to compact or halt.

/// Lightweight snapshot of one turn's resource usage.
#[derive(Debug, Clone, Default)]
pub struct TurnRecord {
    pub tokens_used: u64,
    pub tool_calls: u32,
}

/// Mutable state for an in-progress session.
#[derive(Debug, Clone, Default)]
pub struct SessionState {
    pub id: String,
    pub started_at: u64,
    pub turns: Vec<TurnRecord>,
    pub total_tokens: u64,
}

impl SessionState {
    pub fn new(id: impl Into<String>, started_at: u64) -> Self {
        Self { id: id.into(), started_at, turns: vec![], total_tokens: 0 }
    }

    pub fn record_turn(&mut self, tokens: u64, tool_calls: u32) {
        self.total_tokens += tokens;
        self.turns.push(TurnRecord { tokens_used: tokens, tool_calls });
    }

    pub fn turn_count(&self) -> usize {
        self.turns.len()
    }
}

/// Remaining capacity reported by `SessionManager::budget_remaining`.
#[derive(Debug, Clone, Default)]
pub struct SessionBudget {
    /// Tokens left before the hard limit.
    pub max_tokens: u64,
    /// Wall-time seconds remaining (0 = unlimited).
    pub wall_secs: u64,
}

/// Decision returned per-turn by `SessionManager::decide`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContinuationDecision {
    /// All budgets healthy — continue normally.
    Continue,
    /// Token usage ≥ 75% — compact context and continue.
    CompactAndContinue,
    /// Hard limit reached — halt with a reason string.
    Halt(String),
}

/// Configuration knobs for the session manager.
#[derive(Debug, Clone)]
pub struct SessionConfig {
    /// Hard token limit per session (default: 2_000_000).
    pub max_tokens: u64,
    /// Token fraction at which compaction is triggered (default: 0.75).
    pub compact_threshold: f64,
    /// Maximum wall-time in seconds (0 = unlimited).
    pub max_wall_secs: u64,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            max_tokens: 2_000_000,
            compact_threshold: 0.75,
            max_wall_secs: 0,
        }
    }
}

/// Evaluates session health and emits continuation decisions.
#[derive(Debug, Clone, Default)]
pub struct SessionManager {
    pub config: SessionConfig,
}

impl SessionManager {
    pub fn with_defaults() -> Self {
        Self { config: SessionConfig::default() }
    }

    pub fn new(config: SessionConfig) -> Self {
        Self { config }
    }

    pub fn decide(&self, state: &SessionState, now: u64) -> ContinuationDecision {
        let usage = state.total_tokens as f64 / self.config.max_tokens as f64;
        if usage >= 1.0 {
            return ContinuationDecision::Halt(format!(
                "token limit reached ({}/{})",
                state.total_tokens, self.config.max_tokens
            ));
        }
        if self.config.max_wall_secs > 0 {
            let elapsed = now.saturating_sub(state.started_at);
            if elapsed >= self.config.max_wall_secs {
                return ContinuationDecision::Halt(format!(
                    "wall-time limit reached ({}s)", elapsed
                ));
            }
        }
        if usage >= self.config.compact_threshold {
            return ContinuationDecision::CompactAndContinue;
        }
        ContinuationDecision::Continue
    }

    pub fn budget_remaining(&self, state: &SessionState, _now: u64) -> SessionBudget {
        let remaining_tokens = self.config.max_tokens.saturating_sub(state.total_tokens);
        SessionBudget { max_tokens: remaining_tokens, wall_secs: 0 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mgr_with_limit(max_tokens: u64) -> SessionManager {
        SessionManager::new(SessionConfig { max_tokens, ..Default::default() })
    }

    #[test]
    fn test_continue_when_under_threshold() {
        let mgr = mgr_with_limit(1_000_000);
        let mut s = SessionState::new("s1", 0);
        s.record_turn(100_000, 5); // 10% usage
        assert_eq!(mgr.decide(&s, 1), ContinuationDecision::Continue);
    }

    #[test]
    fn test_compact_when_over_threshold() {
        let mgr = mgr_with_limit(1_000_000);
        let mut s = SessionState::new("s1", 0);
        s.record_turn(800_000, 5); // 80% usage → > 0.75
        assert_eq!(mgr.decide(&s, 1), ContinuationDecision::CompactAndContinue);
    }

    #[test]
    fn test_halt_when_token_limit_reached() {
        let mgr = mgr_with_limit(1_000);
        let mut s = SessionState::new("s1", 0);
        s.record_turn(1_000, 1);
        let d = mgr.decide(&s, 1);
        assert!(matches!(d, ContinuationDecision::Halt(_)));
    }

    #[test]
    fn test_halt_when_wall_time_exceeded() {
        let mgr = SessionManager::new(SessionConfig {
            max_tokens: 1_000_000,
            max_wall_secs: 60,
            ..Default::default()
        });
        let s = SessionState::new("s1", 0); // started at t=0
        let d = mgr.decide(&s, 120); // now = 120 > 60
        assert!(matches!(d, ContinuationDecision::Halt(_)));
    }

    #[test]
    fn test_no_halt_wall_time_not_reached() {
        let mgr = SessionManager::new(SessionConfig {
            max_tokens: 1_000_000,
            max_wall_secs: 60,
            ..Default::default()
        });
        let s = SessionState::new("s1", 0);
        let d = mgr.decide(&s, 30); // still within limit
        assert_eq!(d, ContinuationDecision::Continue);
    }

    #[test]
    fn test_record_turn_accumulates_tokens() {
        let mut s = SessionState::new("s1", 0);
        s.record_turn(500, 2);
        s.record_turn(300, 1);
        assert_eq!(s.total_tokens, 800);
        assert_eq!(s.turn_count(), 2);
    }

    #[test]
    fn test_budget_remaining_calculates_correctly() {
        let mgr = mgr_with_limit(1_000);
        let mut s = SessionState::new("s1", 0);
        s.record_turn(400, 0);
        let budget = mgr.budget_remaining(&s, 0);
        assert_eq!(budget.max_tokens, 600);
    }

    #[test]
    fn test_zero_wall_secs_means_unlimited() {
        let mgr = SessionManager::with_defaults(); // max_wall_secs = 0
        let s = SessionState::new("s1", 0);
        // even at a very large now, should not halt on wall time
        let d = mgr.decide(&s, u64::MAX);
        assert_eq!(d, ContinuationDecision::Continue);
    }
}
