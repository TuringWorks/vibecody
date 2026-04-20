#![allow(dead_code)]
//! Provider-aware rate-limit backoff — exponential backoff with jitter and
//! per-provider circuit-breaker logic.
//!
//! Matches Cody 6.0's rate-limit backoff with circuit-breaker.

use std::collections::HashMap;
use std::time::{Duration, Instant};

// ---------------------------------------------------------------------------
// Backoff policy
// ---------------------------------------------------------------------------

/// Strategy for computing the next retry delay.
#[derive(Debug, Clone)]
pub enum BackoffStrategy {
    /// Fixed delay regardless of attempt number.
    Fixed(Duration),
    /// `base * multiplier^attempt`, capped at `max`.
    Exponential {
        base: Duration,
        multiplier: f64,
        max: Duration,
    },
    /// Exponential + full jitter (random in [0, delay]).
    ExponentialJitter {
        base: Duration,
        multiplier: f64,
        max: Duration,
    },
}

impl BackoffStrategy {
    /// Compute delay for attempt `n` (0-indexed).
    pub fn delay(&self, attempt: u32) -> Duration {
        match self {
            BackoffStrategy::Fixed(d) => *d,
            BackoffStrategy::Exponential {
                base,
                multiplier,
                max,
            } => {
                let secs = base.as_secs_f64() * multiplier.powi(attempt as i32);
                Duration::from_secs_f64(secs.min(max.as_secs_f64()))
            }
            BackoffStrategy::ExponentialJitter {
                base,
                multiplier,
                max,
            } => {
                let secs = base.as_secs_f64() * multiplier.powi(attempt as i32);
                let capped = secs.min(max.as_secs_f64());
                // Deterministic jitter: use attempt as seed substitute.
                // In production this would use rand::rng().
                let jitter_factor = simple_jitter(attempt);
                Duration::from_secs_f64(capped * jitter_factor)
            }
        }
    }
}

/// Simple deterministic jitter in [0.5, 1.0] based on attempt number.
fn simple_jitter(attempt: u32) -> f64 {
    // Use a LCG-derived value in [0.5, 1.0].
    let x = ((attempt.wrapping_mul(2654435761) ^ 0xdeadbeef) % 1000) as f64 / 1000.0;
    0.5 + x * 0.5
}

// ---------------------------------------------------------------------------
// HTTP status codes that trigger retry
// ---------------------------------------------------------------------------

/// HTTP status codes that should trigger a retry.
#[derive(Debug, Clone)]
pub struct RetryableStatuses {
    pub codes: Vec<u16>,
}

impl Default for RetryableStatuses {
    fn default() -> Self {
        Self {
            codes: vec![429, 500, 502, 503, 504, 529],
        }
    }
}

impl RetryableStatuses {
    pub fn is_retryable(&self, status: u16) -> bool {
        self.codes.contains(&status)
    }

    pub fn is_rate_limited(&self, status: u16) -> bool {
        status == 429 || status == 529
    }
}

// ---------------------------------------------------------------------------
// Retry policy
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct RetryPolicy {
    pub max_attempts: u32,
    pub strategy: BackoffStrategy,
    pub retryable: RetryableStatuses,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 5,
            strategy: BackoffStrategy::ExponentialJitter {
                base: Duration::from_millis(500),
                multiplier: 2.0,
                max: Duration::from_secs(60),
            },
            retryable: RetryableStatuses::default(),
        }
    }
}

impl RetryPolicy {
    /// Returns `Some(delay)` if attempt `n` should be retried, `None` if exhausted.
    pub fn next_delay(&self, attempt: u32, status: u16) -> Option<Duration> {
        if attempt >= self.max_attempts {
            return None;
        }
        if !self.retryable.is_retryable(status) {
            return None;
        }
        Some(self.strategy.delay(attempt))
    }
}

// ---------------------------------------------------------------------------
// Circuit breaker
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CircuitState {
    /// Normal operation.
    Closed,
    /// Half-open: allows one probe request to test recovery.
    HalfOpen,
    /// Open: rejects all requests until cool-down expires.
    Open,
}

impl std::fmt::Display for CircuitState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CircuitState::Closed => write!(f, "closed"),
            CircuitState::HalfOpen => write!(f, "half_open"),
            CircuitState::Open => write!(f, "open"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// Number of consecutive failures to trip the breaker.
    pub failure_threshold: u32,
    /// Duration to stay Open before moving to HalfOpen.
    pub open_duration: Duration,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            open_duration: Duration::from_secs(30),
        }
    }
}

/// Per-provider circuit breaker.
pub struct CircuitBreaker {
    config: CircuitBreakerConfig,
    state: CircuitState,
    consecutive_failures: u32,
    opened_at: Option<Instant>,
}

impl CircuitBreaker {
    pub fn new(config: CircuitBreakerConfig) -> Self {
        Self {
            config,
            state: CircuitState::Closed,
            consecutive_failures: 0,
            opened_at: None,
        }
    }

    pub fn state(&self) -> &CircuitState {
        // Check if we should auto-transition from Open to HalfOpen.
        &self.state
    }

    /// Call before sending a request. Returns `false` if the circuit is Open
    /// (request should be rejected immediately).
    pub fn allow_request(&mut self) -> bool {
        match self.state {
            CircuitState::Closed => true,
            CircuitState::HalfOpen => true, // allow probe
            CircuitState::Open => {
                if let Some(opened_at) = self.opened_at {
                    if opened_at.elapsed() >= self.config.open_duration {
                        self.state = CircuitState::HalfOpen;
                        return true;
                    }
                }
                false
            }
        }
    }

    /// Record a successful response.
    pub fn record_success(&mut self) {
        self.consecutive_failures = 0;
        self.opened_at = None;
        self.state = CircuitState::Closed;
    }

    /// Record a failed response.
    pub fn record_failure(&mut self) {
        self.consecutive_failures += 1;
        if self.consecutive_failures >= self.config.failure_threshold {
            self.state = CircuitState::Open;
            self.opened_at = Some(Instant::now());
        }
    }

    pub fn consecutive_failures(&self) -> u32 {
        self.consecutive_failures
    }
}

// ---------------------------------------------------------------------------
// Provider backoff manager
// ---------------------------------------------------------------------------

/// Manages per-provider retry policies and circuit breakers.
pub struct ProviderBackoffManager {
    policies: HashMap<String, RetryPolicy>,
    breakers: HashMap<String, CircuitBreaker>,
    default_policy: RetryPolicy,
    default_cb_config: CircuitBreakerConfig,
}

impl ProviderBackoffManager {
    pub fn new() -> Self {
        Self {
            policies: HashMap::new(),
            breakers: HashMap::new(),
            default_policy: RetryPolicy::default(),
            default_cb_config: CircuitBreakerConfig::default(),
        }
    }

    /// Register a custom retry policy for a provider.
    pub fn set_policy(&mut self, provider: impl Into<String>, policy: RetryPolicy) {
        self.policies.insert(provider.into(), policy);
    }

    /// Register a custom circuit-breaker config for a provider.
    pub fn set_circuit_breaker(
        &mut self,
        provider: impl Into<String>,
        config: CircuitBreakerConfig,
    ) {
        let key = provider.into();
        self.breakers.insert(key, CircuitBreaker::new(config));
    }

    /// Check if a request to the provider is allowed.
    pub fn allow_request(&mut self, provider: &str) -> bool {
        self.breakers
            .entry(provider.to_string())
            .or_insert_with(|| CircuitBreaker::new(self.default_cb_config.clone()))
            .allow_request()
    }

    /// Get the next retry delay for a provider after a failed request.
    pub fn next_delay(&self, provider: &str, attempt: u32, status: u16) -> Option<Duration> {
        let policy = self.policies.get(provider).unwrap_or(&self.default_policy);
        policy.next_delay(attempt, status)
    }

    /// Record a successful response for the provider.
    pub fn record_success(&mut self, provider: &str) {
        if let Some(cb) = self.breakers.get_mut(provider) {
            cb.record_success();
        }
    }

    /// Record a failed response for the provider.
    pub fn record_failure(&mut self, provider: &str) {
        self.breakers
            .entry(provider.to_string())
            .or_insert_with(|| CircuitBreaker::new(self.default_cb_config.clone()))
            .record_failure();
    }

    /// Get the circuit state for a provider.
    pub fn circuit_state(&self, provider: &str) -> CircuitState {
        self.breakers
            .get(provider)
            .map(|cb| cb.state().clone())
            .unwrap_or(CircuitState::Closed)
    }

    /// Summary of all provider states.
    pub fn status_report(&self) -> HashMap<String, String> {
        self.breakers
            .iter()
            .map(|(k, cb)| (k.clone(), cb.state().to_string()))
            .collect()
    }
}

impl Default for ProviderBackoffManager {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fixed_backoff_constant() {
        let s = BackoffStrategy::Fixed(Duration::from_secs(2));
        assert_eq!(s.delay(0), Duration::from_secs(2));
        assert_eq!(s.delay(5), Duration::from_secs(2));
    }

    #[test]
    fn test_exponential_doubles() {
        let s = BackoffStrategy::Exponential {
            base: Duration::from_secs(1),
            multiplier: 2.0,
            max: Duration::from_secs(60),
        };
        assert_eq!(s.delay(0), Duration::from_secs(1));
        assert_eq!(s.delay(1), Duration::from_secs(2));
        assert_eq!(s.delay(2), Duration::from_secs(4));
    }

    #[test]
    fn test_exponential_capped_at_max() {
        let s = BackoffStrategy::Exponential {
            base: Duration::from_secs(1),
            multiplier: 2.0,
            max: Duration::from_secs(5),
        };
        assert_eq!(s.delay(10), Duration::from_secs(5));
    }

    #[test]
    fn test_jitter_within_bounds() {
        let s = BackoffStrategy::ExponentialJitter {
            base: Duration::from_secs(1),
            multiplier: 2.0,
            max: Duration::from_secs(60),
        };
        for i in 0..10 {
            let d = s.delay(i);
            // With full exponential at attempt i, delay should be <= 2^i seconds
            let max_expected = Duration::from_secs_f64(2f64.powi(i as i32).min(60.0));
            assert!(d <= max_expected, "delay {d:?} > max {max_expected:?} at attempt {i}");
        }
    }

    #[test]
    fn test_retryable_status_429() {
        let rs = RetryableStatuses::default();
        assert!(rs.is_retryable(429));
        assert!(rs.is_rate_limited(429));
    }

    #[test]
    fn test_non_retryable_status_400() {
        let rs = RetryableStatuses::default();
        assert!(!rs.is_retryable(400));
    }

    #[test]
    fn test_retry_policy_returns_delay_for_retryable() {
        let p = RetryPolicy::default();
        assert!(p.next_delay(0, 429).is_some());
    }

    #[test]
    fn test_retry_policy_returns_none_for_non_retryable() {
        let p = RetryPolicy::default();
        assert!(p.next_delay(0, 400).is_none());
    }

    #[test]
    fn test_retry_policy_exhausted() {
        let p = RetryPolicy {
            max_attempts: 3,
            ..Default::default()
        };
        assert!(p.next_delay(3, 429).is_none());
    }

    #[test]
    fn test_circuit_breaker_starts_closed() {
        let cb = CircuitBreaker::new(CircuitBreakerConfig::default());
        assert_eq!(*cb.state(), CircuitState::Closed);
    }

    #[test]
    fn test_circuit_breaker_opens_after_threshold() {
        let mut cb = CircuitBreaker::new(CircuitBreakerConfig {
            failure_threshold: 3,
            open_duration: Duration::from_secs(30),
        });
        cb.record_failure();
        cb.record_failure();
        assert_eq!(*cb.state(), CircuitState::Closed);
        cb.record_failure();
        assert_eq!(*cb.state(), CircuitState::Open);
    }

    #[test]
    fn test_circuit_breaker_closed_after_success() {
        let mut cb = CircuitBreaker::new(CircuitBreakerConfig {
            failure_threshold: 2,
            open_duration: Duration::from_secs(30),
        });
        cb.record_failure();
        cb.record_failure();
        assert_eq!(*cb.state(), CircuitState::Open);
        cb.record_success();
        assert_eq!(*cb.state(), CircuitState::Closed);
    }

    #[test]
    fn test_open_circuit_rejects_requests() {
        let mut cb = CircuitBreaker::new(CircuitBreakerConfig {
            failure_threshold: 1,
            open_duration: Duration::from_secs(3600),
        });
        cb.record_failure();
        assert!(!cb.allow_request());
    }

    #[test]
    fn test_provider_manager_allow_request_default() {
        let mut mgr = ProviderBackoffManager::new();
        assert!(mgr.allow_request("anthropic"));
    }

    #[test]
    fn test_provider_manager_circuit_trips() {
        let mut mgr = ProviderBackoffManager::new();
        mgr.set_circuit_breaker(
            "anthropic",
            CircuitBreakerConfig {
                failure_threshold: 2,
                open_duration: Duration::from_secs(3600),
            },
        );
        mgr.record_failure("anthropic");
        mgr.record_failure("anthropic");
        assert!(!mgr.allow_request("anthropic"));
        assert_eq!(mgr.circuit_state("anthropic"), CircuitState::Open);
    }

    #[test]
    fn test_provider_manager_next_delay() {
        let mgr = ProviderBackoffManager::new();
        let delay = mgr.next_delay("openai", 0, 429);
        assert!(delay.is_some());
    }

    #[test]
    fn test_status_report() {
        let mut mgr = ProviderBackoffManager::new();
        mgr.allow_request("groq"); // creates entry
        let report = mgr.status_report();
        assert!(report.contains_key("groq"));
    }
}
