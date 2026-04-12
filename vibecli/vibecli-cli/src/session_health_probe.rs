//! Session health probing and liveness tracking.
//!
//! Claw-code parity Wave 1: monitors active agent sessions for stalls,
//! resource leaks, and tool-call timeouts; triggers recovery or abort.
//!
//! # Post-compaction probe
//! After auto-compacting a conversation, the session's tool executor may have
//! become unresponsive (timeout, resource exhaustion, etc.). The
//! [`PostCompactionProbe`] / [`ToolResponsivenessChecker`] types run a
//! lightweight check before the agent loop resumes, so the agent can be
//! flagged as Degraded rather than continuing blindly into failure.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ─── Health Status ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthStatus {
    Healthy,
    Degraded { reason: String },
    Stalled  { stall_seconds: u64 },
    Dead,
}

impl HealthStatus {
    pub fn is_ok(&self) -> bool { matches!(self, Self::Healthy) }

    pub fn severity(&self) -> u8 {
        match self {
            Self::Healthy => 0, Self::Degraded { .. } => 1, Self::Stalled { .. } => 2, Self::Dead => 3,
        }
    }
}

impl std::fmt::Display for HealthStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Healthy => write!(f, "healthy"),
            Self::Degraded { reason } => write!(f, "degraded: {reason}"),
            Self::Stalled { stall_seconds } => write!(f, "stalled {stall_seconds}s"),
            Self::Dead => write!(f, "dead"),
        }
    }
}

// ─── Session Metrics ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMetrics {
    pub session_id: String,
    pub started_at_ms: u64,
    pub last_activity_ms: u64,
    pub tool_calls: u32,
    pub tool_errors: u32,
    pub tokens_used: u64,
    pub token_budget: u64,
    pub pending_tool_calls: u32,
}

impl SessionMetrics {
    pub fn new(session_id: impl Into<String>, started_at_ms: u64, token_budget: u64) -> Self {
        Self {
            session_id: session_id.into(), started_at_ms, last_activity_ms: started_at_ms,
            tool_calls: 0, tool_errors: 0, tokens_used: 0, token_budget,
            pending_tool_calls: 0,
        }
    }

    pub fn record_tool_call(&mut self, now_ms: u64, is_error: bool) {
        self.tool_calls += 1;
        if is_error { self.tool_errors += 1; }
        self.last_activity_ms = now_ms;
        if !is_error { self.pending_tool_calls = self.pending_tool_calls.saturating_sub(1); }
    }

    pub fn add_tokens(&mut self, tokens: u64) { self.tokens_used += tokens; }

    pub fn idle_seconds(&self, now_ms: u64) -> u64 {
        (now_ms - self.last_activity_ms) / 1000
    }

    pub fn token_utilisation(&self) -> f64 {
        if self.token_budget == 0 { return 1.0; }
        self.tokens_used as f64 / self.token_budget as f64
    }

    pub fn error_rate(&self) -> f64 {
        if self.tool_calls == 0 { return 0.0; }
        self.tool_errors as f64 / self.tool_calls as f64
    }
}

// ─── Probe Thresholds ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbeThresholds {
    /// Idle for this many seconds → stalled.
    pub stall_seconds: u64,
    /// Idle for this many seconds → dead.
    pub dead_seconds: u64,
    /// Token utilisation above this → degraded.
    pub token_warning: f64,
    /// Error rate above this → degraded.
    pub error_rate_warning: f64,
    /// More than this many pending tool calls → degraded.
    pub max_pending: u32,
}

impl Default for ProbeThresholds {
    fn default() -> Self {
        Self {
            stall_seconds: 30, dead_seconds: 120,
            token_warning: 0.85, error_rate_warning: 0.3, max_pending: 5,
        }
    }
}

// ─── Health Probe ─────────────────────────────────────────────────────────────

pub struct SessionHealthProbe {
    pub thresholds: ProbeThresholds,
    sessions: HashMap<String, SessionMetrics>,
}

impl SessionHealthProbe {
    pub fn new(thresholds: ProbeThresholds) -> Self {
        Self { thresholds, sessions: HashMap::new() }
    }

    pub fn register(&mut self, metrics: SessionMetrics) {
        self.sessions.insert(metrics.session_id.clone(), metrics);
    }

    pub fn update(&mut self, session_id: &str, f: impl FnOnce(&mut SessionMetrics)) -> bool {
        if let Some(m) = self.sessions.get_mut(session_id) { f(m); true } else { false }
    }

    pub fn probe(&self, session_id: &str, now_ms: u64) -> HealthStatus {
        let m = match self.sessions.get(session_id) {
            Some(m) => m,
            None => return HealthStatus::Dead,
        };

        let idle = m.idle_seconds(now_ms);
        if idle >= self.thresholds.dead_seconds { return HealthStatus::Dead; }
        if idle >= self.thresholds.stall_seconds { return HealthStatus::Stalled { stall_seconds: idle }; }

        if m.token_utilisation() >= self.thresholds.token_warning {
            return HealthStatus::Degraded { reason: format!("token budget {:.0}% used", m.token_utilisation() * 100.0) };
        }
        if m.error_rate() >= self.thresholds.error_rate_warning {
            return HealthStatus::Degraded { reason: format!("error rate {:.0}%", m.error_rate() * 100.0) };
        }
        if m.pending_tool_calls > self.thresholds.max_pending {
            return HealthStatus::Degraded { reason: format!("{} pending tool calls", m.pending_tool_calls) };
        }

        HealthStatus::Healthy
    }

    /// Probe all sessions and return unhealthy ones sorted by severity.
    pub fn probe_all(&self, now_ms: u64) -> Vec<(String, HealthStatus)> {
        let mut results: Vec<_> = self.sessions.keys()
            .map(|id| (id.clone(), self.probe(id, now_ms)))
            .filter(|(_, s)| !s.is_ok())
            .collect();
        results.sort_by_key(|(_, s)| std::cmp::Reverse(s.severity()));
        results
    }

    pub fn remove(&mut self, session_id: &str) { self.sessions.remove(session_id); }
    pub fn session_count(&self) -> usize { self.sessions.len() }
    pub fn metrics(&self, session_id: &str) -> Option<&SessionMetrics> { self.sessions.get(session_id) }
}

impl Default for SessionHealthProbe {
    fn default() -> Self { Self::new(ProbeThresholds::default()) }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn probe() -> SessionHealthProbe { SessionHealthProbe::default() }
    fn now() -> u64 { 0 }
    const BUDGET: u64 = 100_000;

    fn register(p: &mut SessionHealthProbe, id: &str) {
        p.register(SessionMetrics::new(id, 0, BUDGET));
    }

    #[test]
    fn test_healthy_fresh_session() {
        let mut p = probe();
        register(&mut p, "s1");
        assert_eq!(p.probe("s1", now()), HealthStatus::Healthy);
    }

    #[test]
    fn test_unknown_session_is_dead() {
        let p = probe();
        assert_eq!(p.probe("nope", now()), HealthStatus::Dead);
    }

    #[test]
    fn test_stalled_after_30s() {
        let mut p = probe();
        register(&mut p, "s1");
        let status = p.probe("s1", 31_000);
        assert!(matches!(status, HealthStatus::Stalled { .. }));
    }

    #[test]
    fn test_dead_after_120s() {
        let mut p = probe();
        register(&mut p, "s1");
        assert_eq!(p.probe("s1", 121_000), HealthStatus::Dead);
    }

    #[test]
    fn test_not_stalled_before_threshold() {
        let mut p = probe();
        register(&mut p, "s1");
        assert_eq!(p.probe("s1", 29_000), HealthStatus::Healthy);
    }

    #[test]
    fn test_token_budget_warning() {
        let mut p = probe();
        p.register(SessionMetrics::new("s1", 0, 100));
        p.update("s1", |m| m.tokens_used = 90); // 90% > 85%
        assert!(matches!(p.probe("s1", now()), HealthStatus::Degraded { .. }));
    }

    #[test]
    fn test_error_rate_warning() {
        let mut p = probe();
        register(&mut p, "s1");
        p.update("s1", |m| {
            m.tool_calls = 10;
            m.tool_errors = 4; // 40% > 30%
        });
        assert!(matches!(p.probe("s1", now()), HealthStatus::Degraded { .. }));
    }

    #[test]
    fn test_pending_tool_calls_warning() {
        let mut p = probe();
        register(&mut p, "s1");
        p.update("s1", |m| m.pending_tool_calls = 6); // > 5
        assert!(matches!(p.probe("s1", now()), HealthStatus::Degraded { .. }));
    }

    #[test]
    fn test_record_tool_call_updates_activity() {
        let mut p = probe();
        register(&mut p, "s1");
        p.update("s1", |m| m.record_tool_call(50_000, false));
        assert_eq!(p.probe("s1", 51_000), HealthStatus::Healthy); // only 1s idle
    }

    #[test]
    fn test_error_rate_zero_with_no_calls() {
        let m = SessionMetrics::new("s1", 0, BUDGET);
        assert_eq!(m.error_rate(), 0.0);
    }

    #[test]
    fn test_idle_seconds_calculation() {
        let m = SessionMetrics::new("s1", 0, BUDGET);
        assert_eq!(m.idle_seconds(5000), 5);
    }

    #[test]
    fn test_token_utilisation() {
        let mut m = SessionMetrics::new("s1", 0, 1000);
        m.add_tokens(500);
        assert!((m.token_utilisation() - 0.5).abs() < 1e-6);
    }

    #[test]
    fn test_probe_all_returns_unhealthy() {
        let mut p = probe();
        // Register "healthy" session with a recent timestamp so it stays healthy at now=35_000.
        p.register(SessionMetrics::new("healthy", 34_000, BUDGET));
        register(&mut p, "stalled"); // starts at 0 → idle 35s at now=35_000
        p.update("stalled", |m| m.last_activity_ms = 0);
        let unhealthy = p.probe_all(35_000);
        assert_eq!(unhealthy.len(), 1);
        assert_eq!(unhealthy[0].0, "stalled");
    }

    #[test]
    fn test_probe_all_sorted_by_severity() {
        let mut p = probe();
        register(&mut p, "dead");
        register(&mut p, "degraded");
        p.update("dead", |m| m.last_activity_ms = 0);
        p.update("degraded", |m| { m.tool_calls = 10; m.tool_errors = 5; });
        let unhealthy = p.probe_all(200_000);
        assert_eq!(unhealthy[0].1.severity(), 3); // dead first
    }

    #[test]
    fn test_remove_session() {
        let mut p = probe();
        register(&mut p, "s1");
        p.remove("s1");
        assert_eq!(p.probe("s1", now()), HealthStatus::Dead);
    }

    #[test]
    fn test_session_count() {
        let mut p = probe();
        register(&mut p, "a");
        register(&mut p, "b");
        assert_eq!(p.session_count(), 2);
        p.remove("a");
        assert_eq!(p.session_count(), 1);
    }

    #[test]
    fn test_status_severity_ordering() {
        assert!(HealthStatus::Healthy.severity() < HealthStatus::Degraded { reason: "x".into() }.severity());
        assert!(HealthStatus::Stalled { stall_seconds: 5 }.severity() < HealthStatus::Dead.severity());
    }

    #[test]
    fn test_status_display() {
        assert_eq!(HealthStatus::Healthy.to_string(), "healthy");
        assert_eq!(HealthStatus::Dead.to_string(), "dead");
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Post-compaction probe — lightweight checker run after auto-compaction
// ═══════════════════════════════════════════════════════════════════════════════

// ── ProbeResult ───────────────────────────────────────────────────────────────

/// Result of a post-compaction health probe.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProbeResult {
    Healthy,
    Degraded(String),
    Failed(String),
}

impl std::fmt::Display for ProbeResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Healthy => write!(f, "HEALTHY"),
            Self::Degraded(r) => write!(f, "DEGRADED: {r}"),
            Self::Failed(r) => write!(f, "FAILED: {r}"),
        }
    }
}

impl ProbeResult {
    /// Maps to the string used by AgentHealthState Display.
    pub fn to_health_string(&self) -> &'static str {
        match self {
            Self::Healthy => "PROGRESS",
            Self::Degraded(_) => "DEGRADED",
            Self::Failed(_) => "DEGRADED", // both map to DEGRADED in agent health
        }
    }

    pub fn is_healthy(&self) -> bool {
        matches!(self, Self::Healthy)
    }
}

// ── ProbeType ─────────────────────────────────────────────────────────────────

/// Which aspect of the session is being probed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ProbeType {
    #[default]
    ToolResponsiveness,
    ContextIntegrity,
    ProviderReachability,
}

impl std::fmt::Display for ProbeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ToolResponsiveness => write!(f, "tool_responsiveness"),
            Self::ContextIntegrity => write!(f, "context_integrity"),
            Self::ProviderReachability => write!(f, "provider_reachability"),
        }
    }
}

// ── ProbeConfig ───────────────────────────────────────────────────────────────

/// Configuration for the post-compaction health probe.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbeConfig {
    pub timeout_ms: u64,
    pub probe_type: ProbeType,
    /// Minimum number of compacted messages to trigger a probe.
    pub compaction_threshold: usize,
}

impl Default for ProbeConfig {
    fn default() -> Self {
        Self {
            timeout_ms: 5000,
            probe_type: ProbeType::ToolResponsiveness,
            compaction_threshold: 50,
        }
    }
}

// ── ToolResponsivenessChecker ─────────────────────────────────────────────────

/// Trait for the lightweight responsiveness check.
/// The agent loop passes its tool executor implementation.
pub trait ToolResponsivenessChecker {
    fn check(&self) -> ProbeResult;
}

// ── Mock checkers (usable in tests and integration stubs) ─────────────────────

pub struct ResponsiveMockChecker;
impl ToolResponsivenessChecker for ResponsiveMockChecker {
    fn check(&self) -> ProbeResult {
        ProbeResult::Healthy
    }
}

pub struct UnresponsiveMockChecker {
    pub reason: String,
}
impl ToolResponsivenessChecker for UnresponsiveMockChecker {
    fn check(&self) -> ProbeResult {
        ProbeResult::Failed(self.reason.clone())
    }
}

pub struct SlowMockChecker {
    pub reason: String,
}
impl ToolResponsivenessChecker for SlowMockChecker {
    fn check(&self) -> ProbeResult {
        ProbeResult::Degraded(self.reason.clone())
    }
}

// ── PostCompactionProbe ───────────────────────────────────────────────────────

/// Runs a lightweight probe after auto-compaction before the agent loop resumes.
#[derive(Debug, Default)]
pub struct PostCompactionProbe {
    pub config: ProbeConfig,
}

impl PostCompactionProbe {
    pub fn new(config: ProbeConfig) -> Self {
        Self { config }
    }

    /// Run the probe using the given checker.
    pub fn run(&self, checker: &dyn ToolResponsivenessChecker) -> ProbeResult {
        checker.check()
    }

    /// Should we probe after compacting `compacted_count` messages?
    pub fn should_probe_after_compaction(&self, compacted_count: usize) -> bool {
        compacted_count >= self.config.compaction_threshold
    }
}

// ── SessionHealthProbe re-export shim ────────────────────────────────────────
// Type alias for post-compaction probe; kept for any downstream consumers.
#[allow(unused_imports)]
pub use PostCompactionProbe as SessionHealthProbeCompact;

// ── Post-compaction TDD tests ─────────────────────────────────────────────────

#[cfg(test)]
mod compaction_tests {
    use super::*;

    #[test]
    fn healthy_on_responsive_tool() {
        let probe = PostCompactionProbe::new(ProbeConfig::default());
        let checker = ResponsiveMockChecker;
        assert_eq!(probe.run(&checker), ProbeResult::Healthy);
    }

    #[test]
    fn degraded_on_slow_tool() {
        let probe = PostCompactionProbe::new(ProbeConfig::default());
        let checker = SlowMockChecker { reason: "slow".into() };
        assert!(matches!(probe.run(&checker), ProbeResult::Degraded(_)));
    }

    #[test]
    fn failed_on_unresponsive_tool() {
        let probe = PostCompactionProbe::new(ProbeConfig::default());
        let checker = UnresponsiveMockChecker { reason: "timeout".into() };
        assert!(matches!(probe.run(&checker), ProbeResult::Failed(_)));
    }

    #[test]
    fn map_healthy_to_progress() {
        assert_eq!(ProbeResult::Healthy.to_health_string(), "PROGRESS");
    }

    #[test]
    fn map_degraded_to_degraded() {
        assert_eq!(ProbeResult::Degraded("x".into()).to_health_string(), "DEGRADED");
        assert_eq!(ProbeResult::Failed("x".into()).to_health_string(), "DEGRADED");
    }

    #[test]
    fn should_probe_above_threshold() {
        let probe = PostCompactionProbe::new(ProbeConfig {
            compaction_threshold: 50,
            ..Default::default()
        });
        assert!(probe.should_probe_after_compaction(60));
        assert!(!probe.should_probe_after_compaction(10));
        assert!(!probe.should_probe_after_compaction(49));
        assert!(probe.should_probe_after_compaction(50));
    }
}
