//! long_session — Token-budgeted long-running autonomous session management.
//! Tracks per-turn token usage, wall-time, and decides when to compact or halt.

/// Lightweight snapshot of one turn's resource usage.
#[derive(Debug, Clone, Default)]
pub struct TurnRecord {
    pub tokens_used: u64,
    pub tool_calls: u32,
}

/// Mutable state for an in-progress session.
#[derive(Debug, Clone)]
pub struct SessionState {
    pub id: String,
    pub started_at: u64,
    pub turns: Vec<TurnRecord>,
    pub total_tokens: u64,
}

impl Default for SessionState {
    fn default() -> Self {
        Self { id: String::new(), started_at: 0, turns: vec![], total_tokens: 0 }
    }
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
#[derive(Debug, Clone)]
pub struct SessionManager {
    pub config: SessionConfig,
}

impl Default for SessionManager {
    fn default() -> Self {
        Self { config: SessionConfig::default() }
    }
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
