//! Agent auto-scaler — adjusts the active agent pool size based on queue depth,
//! utilization metrics, and scaling policies. Matches Devin 2.0's auto-scaling.
//!
//! Scale-up: when queue depth exceeds threshold or utilization > high-watermark
//! Scale-down: when utilization drops below low-watermark for a cooldown period
//! Respects min/max pool sizes from the scaling policy.

use std::collections::VecDeque;
use std::time::{SystemTime, UNIX_EPOCH};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Configuration for the auto-scaler.
#[derive(Debug, Clone)]
pub struct ScalingPolicy {
    pub min_agents: usize,
    pub max_agents: usize,
    /// Scale up when utilization exceeds this fraction.
    pub scale_up_threshold: f64,
    /// Scale down when utilization drops below this fraction.
    pub scale_down_threshold: f64,
    /// Scale up when pending queue depth exceeds this.
    pub queue_depth_threshold: usize,
    /// How many agents to add/remove per scaling event.
    pub scale_step: usize,
    /// Seconds to wait before allowing another scale-down.
    pub cooldown_secs: u64,
    /// Seconds to wait before allowing another scale-up.
    pub scale_up_cooldown_secs: u64,
}

impl Default for ScalingPolicy {
    fn default() -> Self {
        Self {
            min_agents: 1,
            max_agents: 16,
            scale_up_threshold: 0.8,
            scale_down_threshold: 0.2,
            queue_depth_threshold: 5,
            scale_step: 2,
            cooldown_secs: 60,
            scale_up_cooldown_secs: 30,
        }
    }
}

/// A scaling event record.
#[derive(Debug, Clone)]
pub struct ScalingEvent {
    pub kind: ScalingEventKind,
    pub delta: i64, // positive = scale up, negative = scale down
    pub reason: String,
    pub timestamp_ms: u64,
    pub from_size: usize,
    pub to_size: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScalingEventKind {
    ScaleUp,
    ScaleDown,
    NoOp,
}

/// Current pool metrics snapshot.
#[derive(Debug, Clone)]
pub struct PoolMetrics {
    pub active_agents: usize,
    pub busy_agents: usize,
    pub pending_tasks: usize,
    pub timestamp_ms: u64,
}

impl PoolMetrics {
    pub fn new(active: usize, busy: usize, pending: usize) -> Self {
        Self {
            active_agents: active,
            busy_agents: busy,
            pending_tasks: pending,
            timestamp_ms: now_ms(),
        }
    }

    pub fn utilization(&self) -> f64 {
        if self.active_agents == 0 { return 0.0; }
        self.busy_agents as f64 / self.active_agents as f64
    }

    pub fn idle_agents(&self) -> usize {
        self.active_agents.saturating_sub(self.busy_agents)
    }
}

/// Decision made by the auto-scaler.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScalingDecision {
    ScaleUp(usize),   // number to add
    ScaleDown(usize), // number to remove
    Hold,             // no change needed
}

// ---------------------------------------------------------------------------
// Auto-scaler
// ---------------------------------------------------------------------------

pub struct AgentAutoScaler {
    pub policy: ScalingPolicy,
    pub current_size: usize,
    history: VecDeque<ScalingEvent>,
    last_scale_up_ms: u64,
    last_scale_down_ms: u64,
    /// Moving average window for utilization smoothing.
    utilization_window: VecDeque<f64>,
    window_size: usize,
}

impl AgentAutoScaler {
    pub fn new(policy: ScalingPolicy, initial_size: usize) -> Self {
        let initial_size = initial_size.max(policy.min_agents).min(policy.max_agents);
        Self {
            policy,
            current_size: initial_size,
            history: VecDeque::new(),
            last_scale_up_ms: 0,
            last_scale_down_ms: 0,
            utilization_window: VecDeque::new(),
            window_size: 5,
        }
    }

    /// Feed a metrics snapshot and get a scaling decision.
    pub fn evaluate(&mut self, metrics: &PoolMetrics) -> ScalingDecision {
        // Update smoothed utilization
        self.utilization_window.push_back(metrics.utilization());
        if self.utilization_window.len() > self.window_size {
            self.utilization_window.pop_front();
        }
        let smooth_util = self.smooth_utilization();
        let now = now_ms();

        // Scale up: queue overflow
        if metrics.pending_tasks >= self.policy.queue_depth_threshold
            && self.current_size < self.policy.max_agents
        {
            let elapsed = now.saturating_sub(self.last_scale_up_ms) / 1000;
            if elapsed >= self.policy.scale_up_cooldown_secs {
                let add = self.policy.scale_step.min(self.policy.max_agents - self.current_size);
                return ScalingDecision::ScaleUp(add);
            }
        }

        // Scale up: high utilization
        if smooth_util > self.policy.scale_up_threshold
            && self.current_size < self.policy.max_agents
        {
            let elapsed = now.saturating_sub(self.last_scale_up_ms) / 1000;
            if elapsed >= self.policy.scale_up_cooldown_secs {
                let add = self.policy.scale_step.min(self.policy.max_agents - self.current_size);
                return ScalingDecision::ScaleUp(add);
            }
        }

        // Scale down: low utilization, no pending tasks
        if smooth_util < self.policy.scale_down_threshold
            && metrics.pending_tasks == 0
            && self.current_size > self.policy.min_agents
        {
            let elapsed = now.saturating_sub(self.last_scale_down_ms) / 1000;
            if elapsed >= self.policy.cooldown_secs {
                let remove = self.policy.scale_step.min(self.current_size - self.policy.min_agents);
                return ScalingDecision::ScaleDown(remove);
            }
        }

        ScalingDecision::Hold
    }

    /// Apply a scaling decision (updates internal size + records event).
    pub fn apply(&mut self, decision: &ScalingDecision, reason: impl Into<String>) -> Option<ScalingEvent> {
        let reason_str = reason.into();
        let from_size = self.current_size;
        let event = match decision {
            ScalingDecision::ScaleUp(n) => {
                self.current_size = (self.current_size + n).min(self.policy.max_agents);
                self.last_scale_up_ms = now_ms();
                ScalingEvent {
                    kind: ScalingEventKind::ScaleUp,
                    delta: *n as i64,
                    reason: reason_str,
                    timestamp_ms: now_ms(),
                    from_size,
                    to_size: self.current_size,
                }
            }
            ScalingDecision::ScaleDown(n) => {
                self.current_size = self.current_size.saturating_sub(*n).max(self.policy.min_agents);
                self.last_scale_down_ms = now_ms();
                ScalingEvent {
                    kind: ScalingEventKind::ScaleDown,
                    delta: -(*n as i64),
                    reason: reason_str,
                    timestamp_ms: now_ms(),
                    from_size,
                    to_size: self.current_size,
                }
            }
            ScalingDecision::Hold => return None,
        };
        self.history.push_back(event.clone());
        if self.history.len() > 100 {
            self.history.pop_front();
        }
        Some(event)
    }

    fn smooth_utilization(&self) -> f64 {
        if self.utilization_window.is_empty() { return 0.0; }
        self.utilization_window.iter().sum::<f64>() / self.utilization_window.len() as f64
    }

    pub fn history(&self) -> &VecDeque<ScalingEvent> { &self.history }

    pub fn scale_up_count(&self) -> usize {
        self.history.iter().filter(|e| e.kind == ScalingEventKind::ScaleUp).count()
    }

    pub fn scale_down_count(&self) -> usize {
        self.history.iter().filter(|e| e.kind == ScalingEventKind::ScaleDown).count()
    }

    /// Force-set the current size (for testing / external control).
    pub fn set_size(&mut self, size: usize) {
        self.current_size = size.max(self.policy.min_agents).min(self.policy.max_agents);
    }

    /// Override last scale times for testing cooldown bypass.
    pub fn reset_cooldowns(&mut self) {
        self.last_scale_up_ms = 0;
        self.last_scale_down_ms = 0;
    }
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

    fn policy() -> ScalingPolicy {
        ScalingPolicy {
            min_agents: 1,
            max_agents: 8,
            scale_up_threshold: 0.8,
            scale_down_threshold: 0.2,
            queue_depth_threshold: 3,
            scale_step: 2,
            cooldown_secs: 0, // no cooldown in tests
            scale_up_cooldown_secs: 0,
        }
    }

    #[test]
    fn test_scale_up_on_high_utilization() {
        let mut scaler = AgentAutoScaler::new(policy(), 2);
        scaler.reset_cooldowns();
        let metrics = PoolMetrics::new(2, 2, 0); // 100% utilization
        let decision = scaler.evaluate(&metrics);
        assert!(matches!(decision, ScalingDecision::ScaleUp(_)));
    }

    #[test]
    fn test_scale_up_on_queue_overflow() {
        let mut scaler = AgentAutoScaler::new(policy(), 2);
        scaler.reset_cooldowns();
        let metrics = PoolMetrics::new(2, 1, 5); // 5 pending > threshold 3
        let decision = scaler.evaluate(&metrics);
        assert!(matches!(decision, ScalingDecision::ScaleUp(_)));
    }

    #[test]
    fn test_scale_down_on_low_utilization() {
        let mut scaler = AgentAutoScaler::new(policy(), 4);
        scaler.reset_cooldowns();
        // Feed 5 low-utilization samples to smooth
        for _ in 0..5 {
            scaler.evaluate(&PoolMetrics::new(4, 0, 0)); // 0%
        }
        let decision = scaler.evaluate(&PoolMetrics::new(4, 0, 0));
        assert!(matches!(decision, ScalingDecision::ScaleDown(_)));
    }

    #[test]
    fn test_hold_at_balanced_utilization() {
        let mut scaler = AgentAutoScaler::new(policy(), 4);
        scaler.reset_cooldowns();
        for _ in 0..5 {
            scaler.evaluate(&PoolMetrics::new(4, 2, 0)); // 50%
        }
        let decision = scaler.evaluate(&PoolMetrics::new(4, 2, 0));
        assert_eq!(decision, ScalingDecision::Hold);
    }

    #[test]
    fn test_respects_max_limit() {
        let p = ScalingPolicy { max_agents: 4, scale_step: 10, ..policy() };
        let mut scaler = AgentAutoScaler::new(p, 4);
        scaler.reset_cooldowns();
        let decision = scaler.evaluate(&PoolMetrics::new(4, 4, 0));
        assert_eq!(decision, ScalingDecision::Hold); // already at max
    }

    #[test]
    fn test_respects_min_limit() {
        let mut scaler = AgentAutoScaler::new(policy(), 1); // already at min
        scaler.reset_cooldowns();
        for _ in 0..5 {
            scaler.evaluate(&PoolMetrics::new(1, 0, 0));
        }
        let decision = scaler.evaluate(&PoolMetrics::new(1, 0, 0));
        assert_eq!(decision, ScalingDecision::Hold); // can't go below min
    }

    #[test]
    fn test_apply_scale_up() {
        let mut scaler = AgentAutoScaler::new(policy(), 2);
        let event = scaler.apply(&ScalingDecision::ScaleUp(2), "test").unwrap();
        assert_eq!(scaler.current_size, 4);
        assert_eq!(event.delta, 2);
    }

    #[test]
    fn test_apply_scale_down() {
        let mut scaler = AgentAutoScaler::new(policy(), 4);
        let event = scaler.apply(&ScalingDecision::ScaleDown(2), "test").unwrap();
        assert_eq!(scaler.current_size, 2);
        assert_eq!(event.delta, -2);
    }

    #[test]
    fn test_apply_hold_returns_none() {
        let mut scaler = AgentAutoScaler::new(policy(), 2);
        let result = scaler.apply(&ScalingDecision::Hold, "test");
        assert!(result.is_none());
    }

    #[test]
    fn test_history_records_events() {
        let mut scaler = AgentAutoScaler::new(policy(), 2);
        scaler.apply(&ScalingDecision::ScaleUp(2), "up");
        scaler.apply(&ScalingDecision::ScaleDown(1), "down");
        assert_eq!(scaler.scale_up_count(), 1);
        assert_eq!(scaler.scale_down_count(), 1);
    }

    #[test]
    fn test_pool_metrics_utilization() {
        let m = PoolMetrics::new(4, 3, 0);
        assert!((m.utilization() - 0.75).abs() < 1e-9);
        assert_eq!(m.idle_agents(), 1);
    }

    #[test]
    fn test_utilization_zero_agents() {
        let m = PoolMetrics::new(0, 0, 0);
        assert_eq!(m.utilization(), 0.0);
    }

    #[test]
    fn test_initial_size_clamped_to_policy() {
        let p = ScalingPolicy { min_agents: 2, max_agents: 8, ..policy() };
        let scaler = AgentAutoScaler::new(p, 0); // 0 < min
        assert_eq!(scaler.current_size, 2);
    }
}
