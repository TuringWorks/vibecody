//! Antifragility primitives — jitter, provider health tracking, failure journaling,
//! circuit breaker recovery, and resilience configuration.
//!
//! These primitives make VibeCody *stronger* from stress rather than merely surviving it:
//! - **Jitter**: Randomized backoff prevents thundering herd on retry storms.
//! - **ProviderHealthTracker**: Sliding-window health scoring enables dynamic failover reordering.
//! - **RecoveryPolicy**: Half-open circuit breaker probing recovers from transient degradation.
//! - **FailureJournal**: Structured failure persistence enables pattern detection and learning.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

// ── Jitter ──────────────────────────────────────────────────────────────────

/// Add ±25% random jitter to a backoff duration to prevent thundering herd.
///
/// Uses time-seeded hashing (no external `rand` dependency).
pub fn add_jitter(backoff_ms: u64) -> u64 {
    if backoff_ms == 0 {
        return 1;
    }
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos();
    let mut hasher = DefaultHasher::new();
    nanos.hash(&mut hasher);
    // Mix in the backoff value itself so consecutive calls with different
    // values don't collide even within the same nanosecond bucket.
    backoff_ms.hash(&mut hasher);
    let hash = hasher.finish();
    let jitter_range = backoff_ms / 4; // 25%
    if jitter_range == 0 {
        return backoff_ms.max(1);
    }
    let jitter = (hash % (jitter_range * 2 + 1)) as i64 - jitter_range as i64;
    (backoff_ms as i64 + jitter).max(1) as u64
}

// ── Failure Categories ──────────────────────────────────────────────────────

/// Categories of failures for pattern analysis and health scoring.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FailureCategory {
    RateLimit,
    Timeout,
    ServerError,
    AuthError,
    NetworkError,
    InvalidResponse,
    StreamInterrupted,
    Unknown,
}

impl std::fmt::Display for FailureCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RateLimit => write!(f, "RateLimit"),
            Self::Timeout => write!(f, "Timeout"),
            Self::ServerError => write!(f, "ServerError"),
            Self::AuthError => write!(f, "AuthError"),
            Self::NetworkError => write!(f, "NetworkError"),
            Self::InvalidResponse => write!(f, "InvalidResponse"),
            Self::StreamInterrupted => write!(f, "StreamInterrupted"),
            Self::Unknown => write!(f, "Unknown"),
        }
    }
}

/// Classify an error string into a FailureCategory.
pub fn classify_error(error: &str) -> FailureCategory {
    let lower = error.to_lowercase();
    if lower.contains("rate limit") || lower.contains("429") || lower.contains("too many") {
        FailureCategory::RateLimit
    } else if lower.contains("timeout") || lower.contains("timed out") || lower.contains("deadline") {
        FailureCategory::Timeout
    } else if lower.contains("503") || lower.contains("502") || lower.contains("504")
        || lower.contains("529") || lower.contains("overloaded") || lower.contains("internal server")
    {
        FailureCategory::ServerError
    } else if lower.contains("401") || lower.contains("403") || lower.contains("auth")
        || lower.contains("api key") || lower.contains("forbidden")
    {
        FailureCategory::AuthError
    } else if lower.contains("network") || lower.contains("connection") || lower.contains("dns")
        || lower.contains("broken pipe") || lower.contains("reset")
    {
        FailureCategory::NetworkError
    } else if lower.contains("stream") || lower.contains("eof") || lower.contains("incomplete") {
        FailureCategory::StreamInterrupted
    } else if lower.contains("invalid") || lower.contains("unexpected") || lower.contains("decode")
        || lower.contains("parse") || lower.contains("malformed")
    {
        FailureCategory::InvalidResponse
    } else {
        FailureCategory::Unknown
    }
}

// ── Provider Health Tracking ────────────────────────────────────────────────

/// Outcome of a single provider call.
#[derive(Debug, Clone)]
pub struct ProviderCallOutcome {
    pub provider_name: String,
    pub success: bool,
    pub latency: Duration,
    pub timestamp: Instant,
    pub error_category: Option<FailureCategory>,
}

/// Per-provider health state computed from a sliding window.
#[derive(Debug, Clone, Serialize)]
pub struct ProviderHealth {
    pub name: String,
    /// Overall health score: 0.0 (dead) to 1.0 (healthy).
    pub score: f64,
    pub success_rate: f64,
    pub avg_latency_ms: f64,
    pub total_calls: usize,
    pub recent_failures: usize,
}

/// Tracks health of all providers using a sliding window of outcomes.
///
/// Thread-safe via `Mutex`. Designed for moderate call rates (dozens/minute).
pub struct ProviderHealthTracker {
    windows: Mutex<HashMap<String, Vec<ProviderCallOutcome>>>,
    window_size: usize,
    window_duration: Duration,
}

impl ProviderHealthTracker {
    pub fn new(window_size: usize, window_duration: Duration) -> Self {
        Self {
            windows: Mutex::new(HashMap::new()),
            window_size,
            window_duration,
        }
    }

    /// Record a call outcome for a provider.
    pub fn record(&self, outcome: ProviderCallOutcome) {
        let mut windows = self.windows.lock().unwrap_or_else(|e| e.into_inner());
        let entries = windows.entry(outcome.provider_name.clone()).or_default();
        entries.push(outcome);

        // Prune old entries by time
        let cutoff = Instant::now() - self.window_duration;
        entries.retain(|e| e.timestamp > cutoff);

        // Enforce window size
        while entries.len() > self.window_size {
            entries.remove(0);
        }
    }

    /// Compute health for a single provider.
    pub fn health(&self, provider_name: &str) -> ProviderHealth {
        let windows = self.windows.lock().unwrap_or_else(|e| e.into_inner());
        match windows.get(provider_name) {
            None => ProviderHealth {
                name: provider_name.to_string(),
                score: 0.5, // unknown — neutral
                success_rate: 0.0,
                avg_latency_ms: 0.0,
                total_calls: 0,
                recent_failures: 0,
            },
            Some(entries) if entries.is_empty() => ProviderHealth {
                name: provider_name.to_string(),
                score: 0.5,
                success_rate: 0.0,
                avg_latency_ms: 0.0,
                total_calls: 0,
                recent_failures: 0,
            },
            Some(entries) => {
                let total = entries.len();
                let successes = entries.iter().filter(|e| e.success).count();
                let failures = total - successes;
                let success_rate = successes as f64 / total as f64;

                let avg_latency_ms = if total > 0 {
                    entries.iter().map(|e| e.latency.as_millis() as f64).sum::<f64>() / total as f64
                } else {
                    0.0
                };

                // Latency factor: 1.0 at 0ms, 0.0 at 30s+
                let latency_factor = 1.0 - (avg_latency_ms / 30_000.0).min(1.0);

                let score = (success_rate * 0.7 + latency_factor * 0.3).clamp(0.0, 1.0);

                ProviderHealth {
                    name: provider_name.to_string(),
                    score,
                    success_rate,
                    avg_latency_ms,
                    total_calls: total,
                    recent_failures: failures,
                }
            }
        }
    }

    /// Get health for all tracked providers, sorted by score descending.
    pub fn all_health(&self) -> Vec<ProviderHealth> {
        let names = self.tracked_names();
        let mut healths: Vec<ProviderHealth> = names
            .iter()
            .map(|name| self.health(name))
            .collect();
        healths.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        healths
    }

    /// Get all provider names currently tracked.
    pub fn tracked_names(&self) -> Vec<String> {
        let windows = self.windows.lock().unwrap_or_else(|e| e.into_inner());
        windows.keys().cloned().collect()
    }

    /// Reorder a list of provider names by health score (highest first).
    /// Providers not yet tracked get a neutral 0.5 score.
    pub fn ranked_providers(&self, names: &[String]) -> Vec<String> {
        let mut scored: Vec<(String, f64)> = names
            .iter()
            .map(|n| {
                let h = self.health(n);
                (n.clone(), h.score)
            })
            .collect();
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.into_iter().map(|(n, _)| n).collect()
    }
}


// ── Recovery Policy ─────────────────────────────────────────────────────────

/// Time-based recovery for circuit breaker half-open state.
///
/// After a cooldown period, the circuit breaker enters a "probing" state where
/// it allows calls through. If `required_successes` consecutive successes occur,
/// the circuit promotes back to Progress. Any failure during probing re-escalates.
#[derive(Debug, Clone)]
pub struct RecoveryPolicy {
    /// Cooldown before trying to recover from a non-Progress state.
    pub cooldown: Duration,
    /// Number of consecutive successes needed to return to Progress.
    pub required_successes: u32,
    /// Current consecutive success count during recovery probing.
    pub consecutive_successes: u32,
    /// Whether we are in a half-open probing state.
    pub probing: bool,
}

impl Default for RecoveryPolicy {
    fn default() -> Self {
        Self {
            cooldown: Duration::from_secs(30),
            required_successes: 2,
            consecutive_successes: 0,
            probing: false,
        }
    }
}

impl RecoveryPolicy {
    /// Check if we should enter probing state (cooldown elapsed).
    pub fn should_probe(&self, last_state_change: Instant) -> bool {
        !self.probing && last_state_change.elapsed() >= self.cooldown
    }

    /// Record the result of a probe attempt.
    /// Returns `Some(true)` to promote to Progress, `Some(false)` to re-escalate, `None` to keep probing.
    pub fn record_probe_result(&mut self, success: bool) -> Option<bool> {
        if !self.probing {
            self.probing = true;
            self.consecutive_successes = 0;
        }

        if success {
            self.consecutive_successes += 1;
            if self.consecutive_successes >= self.required_successes {
                self.reset();
                Some(true) // promote to Progress
            } else {
                None // keep probing
            }
        } else {
            self.reset();
            Some(false) // re-escalate
        }
    }

    /// Reset probing state.
    pub fn reset(&mut self) {
        self.probing = false;
        self.consecutive_successes = 0;
    }
}

// ── Failure Journal ─────────────────────────────────────────────────────────

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// A structured failure record for pattern analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailureRecord {
    pub timestamp_ms: u64,
    pub category: FailureCategory,
    pub provider: Option<String>,
    pub error_message: String,
    pub context: Option<String>,
}

impl FailureRecord {
    pub fn new(
        category: FailureCategory,
        provider: Option<String>,
        error_message: String,
        context: Option<String>,
    ) -> Self {
        Self {
            timestamp_ms: now_ms(),
            category,
            provider,
            error_message,
            context,
        }
    }
}

/// A recurring failure pattern detected by the journal.
#[derive(Debug, Clone, Serialize)]
pub struct FailurePattern {
    pub category: FailureCategory,
    pub count: usize,
    pub provider: Option<String>,
    pub first_seen_ms: u64,
    pub last_seen_ms: u64,
    /// Appeared in multiple distinct time windows.
    pub is_recurring: bool,
}

/// Summary statistics from the failure journal.
#[derive(Debug, Clone, Serialize)]
pub struct FailureJournalSummary {
    pub total_failures: usize,
    pub by_category: HashMap<FailureCategory, usize>,
    pub by_provider: HashMap<String, usize>,
    pub patterns: Vec<FailurePattern>,
    pub last_24h_count: usize,
}

/// Append-only failure log with pattern detection.
pub struct FailureJournal {
    records: Vec<FailureRecord>,
    max_records: usize,
    file_path: Option<PathBuf>,
}

impl FailureJournal {
    pub fn new(file_path: Option<PathBuf>, max_records: usize) -> Self {
        Self {
            records: Vec::new(),
            max_records,
            file_path,
        }
    }

    /// Load existing journal from a JSONL file.
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let mut records = Vec::new();
        for line in content.lines() {
            if let Ok(record) = serde_json::from_str::<FailureRecord>(line) {
                records.push(record);
            }
        }
        Ok(Self {
            records,
            max_records: 10_000,
            file_path: Some(path.to_path_buf()),
        })
    }

    /// Record a failure and persist to disk.
    pub fn record(&mut self, entry: FailureRecord) {
        self.records.push(entry);
        // Enforce max records (drop oldest)
        while self.records.len() > self.max_records {
            self.records.remove(0);
        }
        // Best-effort persist
        let _ = self.flush();
    }

    /// Write all records to disk as JSONL.
    pub fn flush(&self) -> anyhow::Result<()> {
        if let Some(path) = &self.file_path {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut content = String::new();
            for record in &self.records {
                if let Ok(line) = serde_json::to_string(record) {
                    content.push_str(&line);
                    content.push('\n');
                }
            }
            std::fs::write(path, content)?;
        }
        Ok(())
    }

    /// Total records in journal.
    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// Whether the journal is empty.
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// Detect recurring failure patterns within a time window.
    pub fn detect_patterns(&self, window: Duration) -> Vec<FailurePattern> {
        if self.records.is_empty() {
            return Vec::new();
        }

        let window_ms = window.as_millis() as u64;
        let now = now_ms();
        let cutoff = now.saturating_sub(window_ms);

        // Group recent failures by (category, provider)
        let mut groups: HashMap<(FailureCategory, Option<String>), Vec<u64>> = HashMap::new();
        for record in &self.records {
            if record.timestamp_ms >= cutoff {
                groups
                    .entry((record.category.clone(), record.provider.clone()))
                    .or_default()
                    .push(record.timestamp_ms);
            }
        }

        let mut patterns = Vec::new();
        for ((category, provider), timestamps) in groups {
            if timestamps.len() < 2 {
                continue;
            }
            let first = *timestamps.iter().min().unwrap_or(&0);
            let last = *timestamps.iter().max().unwrap_or(&0);

            // Check if pattern spans multiple sub-windows (recurring vs burst)
            let sub_window = window_ms / 3;
            let windows_hit: std::collections::HashSet<u64> = timestamps
                .iter()
                .map(|t| (t - cutoff) / sub_window.max(1))
                .collect();
            let is_recurring = windows_hit.len() > 1;

            patterns.push(FailurePattern {
                category,
                count: timestamps.len(),
                provider,
                first_seen_ms: first,
                last_seen_ms: last,
                is_recurring,
            });
        }

        patterns.sort_by(|a, b| b.count.cmp(&a.count));
        patterns
    }

    /// Generate a summary of the failure journal.
    pub fn summary(&self) -> FailureJournalSummary {
        let mut by_category: HashMap<FailureCategory, usize> = HashMap::new();
        let mut by_provider: HashMap<String, usize> = HashMap::new();

        let now = now_ms();
        let day_ms = 24 * 60 * 60 * 1000;
        let mut last_24h = 0;

        for record in &self.records {
            *by_category.entry(record.category.clone()).or_default() += 1;
            if let Some(ref provider) = record.provider {
                *by_provider.entry(provider.clone()).or_default() += 1;
            }
            if record.timestamp_ms >= now.saturating_sub(day_ms) {
                last_24h += 1;
            }
        }

        let patterns = self.detect_patterns(Duration::from_secs(3600)); // 1hr window

        FailureJournalSummary {
            total_failures: self.records.len(),
            by_category,
            by_provider,
            patterns,
            last_24h_count: last_24h,
        }
    }
}

// ── Resilience Configuration ────────────────────────────────────────────────

/// Configuration for the `[resilience]` section in config.toml.
///
/// All fields are optional — `None` means "use the default".
///
/// ```toml
/// [resilience]
/// retry_max_attempts = 5
/// retry_jitter_enabled = true
/// cb_recovery_cooldown_secs = 30
/// health_aware_failover = true
/// failure_journal_enabled = true
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ResilienceConfig {
    // ── Retry settings ──
    pub retry_max_attempts: Option<u32>,
    pub retry_initial_backoff_ms: Option<u64>,
    pub retry_max_backoff_ms: Option<u64>,
    pub retry_multiplier: Option<f64>,
    pub retry_jitter_enabled: Option<bool>,

    // ── Circuit breaker settings ──
    pub cb_stall_threshold: Option<u32>,
    pub cb_spin_threshold: Option<u32>,
    pub cb_degradation_pct: Option<f64>,
    pub cb_max_rotations: Option<u32>,
    pub cb_recovery_cooldown_secs: Option<u64>,
    pub cb_recovery_required_successes: Option<u32>,

    // ── Provider health settings ──
    pub health_window_size: Option<usize>,
    pub health_window_duration_secs: Option<u64>,
    pub health_aware_failover: Option<bool>,

    // ── Failure journal settings ──
    pub failure_journal_enabled: Option<bool>,
    pub failure_journal_max_records: Option<usize>,
}

impl ResilienceConfig {
    pub fn retry_max_attempts(&self) -> u32 {
        self.retry_max_attempts.unwrap_or(5)
    }
    pub fn retry_initial_backoff_ms(&self) -> u64 {
        self.retry_initial_backoff_ms.unwrap_or(1_000)
    }
    pub fn retry_max_backoff_ms(&self) -> u64 {
        self.retry_max_backoff_ms.unwrap_or(60_000)
    }
    pub fn retry_multiplier(&self) -> f64 {
        self.retry_multiplier.unwrap_or(2.0)
    }
    pub fn retry_jitter_enabled(&self) -> bool {
        self.retry_jitter_enabled.unwrap_or(true)
    }
    pub fn cb_stall_threshold(&self) -> u32 {
        self.cb_stall_threshold.unwrap_or(5)
    }
    pub fn cb_spin_threshold(&self) -> u32 {
        self.cb_spin_threshold.unwrap_or(3)
    }
    pub fn cb_degradation_pct(&self) -> f64 {
        self.cb_degradation_pct.unwrap_or(50.0)
    }
    pub fn cb_max_rotations(&self) -> u32 {
        self.cb_max_rotations.unwrap_or(3)
    }
    pub fn cb_recovery_cooldown(&self) -> Duration {
        Duration::from_secs(self.cb_recovery_cooldown_secs.unwrap_or(30))
    }
    pub fn cb_recovery_required_successes(&self) -> u32 {
        self.cb_recovery_required_successes.unwrap_or(2)
    }
    pub fn health_window_size(&self) -> usize {
        self.health_window_size.unwrap_or(50)
    }
    pub fn health_window_duration(&self) -> Duration {
        Duration::from_secs(self.health_window_duration_secs.unwrap_or(600))
    }
    pub fn health_aware_failover(&self) -> bool {
        self.health_aware_failover.unwrap_or(true)
    }
    pub fn failure_journal_enabled(&self) -> bool {
        self.failure_journal_enabled.unwrap_or(true)
    }
    pub fn failure_journal_max_records(&self) -> usize {
        self.failure_journal_max_records.unwrap_or(10_000)
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Jitter tests ────────────────────────────────────────────────────────

    #[test]
    fn jitter_stays_within_bounds() {
        for _ in 0..100 {
            let result = add_jitter(1000);
            assert!(result >= 750, "jitter too low: {result}");
            assert!(result <= 1250, "jitter too high: {result}");
        }
    }

    #[test]
    fn jitter_of_zero_returns_positive() {
        let result = add_jitter(0);
        assert!(result >= 1);
    }

    #[test]
    fn jitter_of_small_value() {
        for _ in 0..50 {
            let result = add_jitter(2);
            assert!(result >= 1);
        }
    }

    #[test]
    fn jitter_large_value() {
        for _ in 0..50 {
            let result = add_jitter(60_000);
            assert!(result >= 45_000);
            assert!(result <= 75_000);
        }
    }

    // ── classify_error tests ────────────────────────────────────────────────

    #[test]
    fn classify_rate_limit() {
        assert_eq!(classify_error("Rate limit exceeded"), FailureCategory::RateLimit);
        assert_eq!(classify_error("HTTP 429"), FailureCategory::RateLimit);
    }

    #[test]
    fn classify_timeout() {
        assert_eq!(classify_error("connection timed out"), FailureCategory::Timeout);
        assert_eq!(classify_error("request timeout"), FailureCategory::Timeout);
    }

    #[test]
    fn classify_server_error() {
        assert_eq!(classify_error("HTTP 503 Service Unavailable"), FailureCategory::ServerError);
        assert_eq!(classify_error("API overloaded"), FailureCategory::ServerError);
    }

    #[test]
    fn classify_auth() {
        assert_eq!(classify_error("HTTP 401 Unauthorized"), FailureCategory::AuthError);
        assert_eq!(classify_error("Invalid API key"), FailureCategory::AuthError);
    }

    #[test]
    fn classify_network() {
        assert_eq!(classify_error("connection reset by peer"), FailureCategory::NetworkError);
        assert_eq!(classify_error("DNS resolution failed"), FailureCategory::NetworkError);
    }

    #[test]
    fn classify_invalid_response() {
        assert_eq!(classify_error("invalid JSON in response"), FailureCategory::InvalidResponse);
        assert_eq!(classify_error("failed to decode body"), FailureCategory::InvalidResponse);
    }

    #[test]
    fn classify_stream() {
        assert_eq!(classify_error("unexpected EOF during stream"), FailureCategory::StreamInterrupted);
        assert_eq!(classify_error("incomplete message received"), FailureCategory::StreamInterrupted);
    }

    #[test]
    fn classify_unknown() {
        assert_eq!(classify_error("something went wrong"), FailureCategory::Unknown);
    }

    // ── ProviderHealthTracker tests ─────────────────────────────────────────

    #[test]
    fn new_tracker_unknown_provider_returns_neutral() {
        let tracker = ProviderHealthTracker::new(50, Duration::from_secs(600));
        let health = tracker.health("nonexistent");
        assert_eq!(health.score, 0.5);
        assert_eq!(health.total_calls, 0);
    }

    #[test]
    fn single_success_high_score() {
        let tracker = ProviderHealthTracker::new(50, Duration::from_secs(600));
        tracker.record(ProviderCallOutcome {
            provider_name: "claude".to_string(),
            success: true,
            latency: Duration::from_millis(200),
            timestamp: Instant::now(),
            error_category: None,
        });
        let health = tracker.health("claude");
        assert!(health.score > 0.9, "score should be high: {}", health.score);
        assert_eq!(health.total_calls, 1);
        assert_eq!(health.recent_failures, 0);
    }

    #[test]
    fn single_failure_low_score() {
        let tracker = ProviderHealthTracker::new(50, Duration::from_secs(600));
        tracker.record(ProviderCallOutcome {
            provider_name: "claude".to_string(),
            success: false,
            latency: Duration::from_millis(5000),
            timestamp: Instant::now(),
            error_category: Some(FailureCategory::ServerError),
        });
        let health = tracker.health("claude");
        assert!(health.score < 0.3, "score should be low: {}", health.score);
        assert_eq!(health.recent_failures, 1);
    }

    #[test]
    fn mixed_outcomes_weighted() {
        let tracker = ProviderHealthTracker::new(50, Duration::from_secs(600));
        // 7 successes, 3 failures = 70% success rate
        for i in 0..10 {
            tracker.record(ProviderCallOutcome {
                provider_name: "openai".to_string(),
                success: i < 7,
                latency: Duration::from_millis(500),
                timestamp: Instant::now(),
                error_category: if i >= 7 { Some(FailureCategory::ServerError) } else { None },
            });
        }
        let health = tracker.health("openai");
        assert!((health.success_rate - 0.7).abs() < 0.01);
        assert_eq!(health.total_calls, 10);
        assert_eq!(health.recent_failures, 3);
    }

    #[test]
    fn window_size_enforced() {
        let tracker = ProviderHealthTracker::new(5, Duration::from_secs(600));
        for _ in 0..10 {
            tracker.record(ProviderCallOutcome {
                provider_name: "test".to_string(),
                success: true,
                latency: Duration::from_millis(100),
                timestamp: Instant::now(),
                error_category: None,
            });
        }
        let health = tracker.health("test");
        assert_eq!(health.total_calls, 5);
    }

    #[test]
    fn ranked_providers_sorted_by_score() {
        let tracker = ProviderHealthTracker::new(50, Duration::from_secs(600));
        // "good" provider: all successes
        for _ in 0..5 {
            tracker.record(ProviderCallOutcome {
                provider_name: "good".to_string(),
                success: true,
                latency: Duration::from_millis(100),
                timestamp: Instant::now(),
                error_category: None,
            });
        }
        // "bad" provider: all failures
        for _ in 0..5 {
            tracker.record(ProviderCallOutcome {
                provider_name: "bad".to_string(),
                success: false,
                latency: Duration::from_millis(10000),
                timestamp: Instant::now(),
                error_category: Some(FailureCategory::ServerError),
            });
        }
        let ranked = tracker.ranked_providers(&[
            "bad".to_string(),
            "good".to_string(),
        ]);
        assert_eq!(ranked[0], "good");
        assert_eq!(ranked[1], "bad");
    }

    #[test]
    fn health_score_clamped_0_to_1() {
        let tracker = ProviderHealthTracker::new(50, Duration::from_secs(600));
        tracker.record(ProviderCallOutcome {
            provider_name: "fast".to_string(),
            success: true,
            latency: Duration::from_millis(1),
            timestamp: Instant::now(),
            error_category: None,
        });
        let health = tracker.health("fast");
        assert!(health.score >= 0.0 && health.score <= 1.0);
    }

    #[test]
    fn tracked_names() {
        let tracker = ProviderHealthTracker::new(50, Duration::from_secs(600));
        tracker.record(ProviderCallOutcome {
            provider_name: "a".to_string(),
            success: true,
            latency: Duration::from_millis(100),
            timestamp: Instant::now(),
            error_category: None,
        });
        tracker.record(ProviderCallOutcome {
            provider_name: "b".to_string(),
            success: true,
            latency: Duration::from_millis(100),
            timestamp: Instant::now(),
            error_category: None,
        });
        let names = tracker.tracked_names();
        assert_eq!(names.len(), 2);
        assert!(names.contains(&"a".to_string()));
        assert!(names.contains(&"b".to_string()));
    }

    // ── RecoveryPolicy tests ────────────────────────────────────────────────

    #[test]
    fn should_not_probe_before_cooldown() {
        let policy = RecoveryPolicy::default();
        let just_now = Instant::now();
        assert!(!policy.should_probe(just_now));
    }

    #[test]
    fn should_probe_after_cooldown() {
        let policy = RecoveryPolicy {
            cooldown: Duration::from_millis(1),
            ..Default::default()
        };
        let past = Instant::now() - Duration::from_millis(10);
        assert!(policy.should_probe(past));
    }

    #[test]
    fn probe_success_promotes_after_required() {
        let mut policy = RecoveryPolicy {
            required_successes: 2,
            ..Default::default()
        };
        assert_eq!(policy.record_probe_result(true), None); // 1st success, keep probing
        assert_eq!(policy.record_probe_result(true), Some(true)); // 2nd success, promote
        assert!(!policy.probing); // reset after promotion
    }

    #[test]
    fn probe_failure_re_escalates() {
        let mut policy = RecoveryPolicy::default();
        let _ = policy.record_probe_result(true); // 1st success
        assert_eq!(policy.record_probe_result(false), Some(false)); // failure re-escalates
        assert!(!policy.probing);
        assert_eq!(policy.consecutive_successes, 0);
    }

    #[test]
    fn reset_clears_state() {
        let mut policy = RecoveryPolicy::default();
        let _ = policy.record_probe_result(true);
        assert!(policy.probing);
        policy.reset();
        assert!(!policy.probing);
        assert_eq!(policy.consecutive_successes, 0);
    }

    // ── FailureJournal tests ────────────────────────────────────────────────

    #[test]
    fn new_journal_is_empty() {
        let journal = FailureJournal::new(None, 100);
        assert!(journal.is_empty());
        assert_eq!(journal.len(), 0);
    }

    #[test]
    fn record_adds_entry() {
        let mut journal = FailureJournal::new(None, 100);
        journal.record(FailureRecord::new(
            FailureCategory::Timeout,
            Some("claude".to_string()),
            "timed out".to_string(),
            None,
        ));
        assert_eq!(journal.len(), 1);
    }

    #[test]
    fn max_records_enforced() {
        let mut journal = FailureJournal::new(None, 5);
        for i in 0..10 {
            journal.record(FailureRecord::new(
                FailureCategory::Unknown,
                None,
                format!("error {i}"),
                None,
            ));
        }
        assert_eq!(journal.len(), 5);
    }

    #[test]
    fn detect_patterns_empty() {
        let journal = FailureJournal::new(None, 100);
        let patterns = journal.detect_patterns(Duration::from_secs(3600));
        assert!(patterns.is_empty());
    }

    #[test]
    fn detect_patterns_finds_recurring() {
        let mut journal = FailureJournal::new(None, 100);
        // Add 5 timeout errors for claude
        for _ in 0..5 {
            journal.record(FailureRecord::new(
                FailureCategory::Timeout,
                Some("claude".to_string()),
                "timed out".to_string(),
                None,
            ));
        }
        let patterns = journal.detect_patterns(Duration::from_secs(3600));
        assert!(!patterns.is_empty());
        assert_eq!(patterns[0].category, FailureCategory::Timeout);
        assert_eq!(patterns[0].count, 5);
        assert_eq!(patterns[0].provider, Some("claude".to_string()));
    }

    #[test]
    fn summary_counts_by_category() {
        let mut journal = FailureJournal::new(None, 100);
        journal.record(FailureRecord::new(FailureCategory::Timeout, None, "t1".into(), None));
        journal.record(FailureRecord::new(FailureCategory::Timeout, None, "t2".into(), None));
        journal.record(FailureRecord::new(FailureCategory::AuthError, None, "a1".into(), None));
        let summary = journal.summary();
        assert_eq!(summary.total_failures, 3);
        assert_eq!(*summary.by_category.get(&FailureCategory::Timeout).unwrap(), 2);
        assert_eq!(*summary.by_category.get(&FailureCategory::AuthError).unwrap(), 1);
    }

    #[test]
    fn summary_counts_by_provider() {
        let mut journal = FailureJournal::new(None, 100);
        journal.record(FailureRecord::new(FailureCategory::Timeout, Some("claude".into()), "t".into(), None));
        journal.record(FailureRecord::new(FailureCategory::Timeout, Some("openai".into()), "t".into(), None));
        journal.record(FailureRecord::new(FailureCategory::Timeout, Some("claude".into()), "t".into(), None));
        let summary = journal.summary();
        assert_eq!(*summary.by_provider.get("claude").unwrap(), 2);
        assert_eq!(*summary.by_provider.get("openai").unwrap(), 1);
    }

    #[test]
    fn journal_flush_and_load_roundtrip() {
        let dir = std::env::temp_dir().join("vibecli_test_journal");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("test_journal.jsonl");

        let mut journal = FailureJournal::new(Some(path.clone()), 100);
        journal.record(FailureRecord::new(
            FailureCategory::RateLimit,
            Some("groq".into()),
            "429 too many requests".into(),
            Some("agent step 3".into()),
        ));
        journal.flush().unwrap();

        let loaded = FailureJournal::load(&path).unwrap();
        assert_eq!(loaded.len(), 1);

        // Cleanup
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_dir(&dir);
    }

    // ── ResilienceConfig tests ──────────────────────────────────────────────

    #[test]
    fn default_config_all_none() {
        let config = ResilienceConfig::default();
        assert!(config.retry_max_attempts.is_none());
        assert!(config.cb_stall_threshold.is_none());
        assert!(config.health_aware_failover.is_none());
        assert!(config.failure_journal_enabled.is_none());
    }

    #[test]
    fn config_accessors_use_defaults() {
        let config = ResilienceConfig::default();
        assert_eq!(config.retry_max_attempts(), 5);
        assert_eq!(config.retry_initial_backoff_ms(), 1_000);
        assert_eq!(config.retry_max_backoff_ms(), 60_000);
        assert_eq!(config.retry_multiplier(), 2.0);
        assert!(config.retry_jitter_enabled());
        assert_eq!(config.cb_stall_threshold(), 5);
        assert_eq!(config.cb_spin_threshold(), 3);
        assert_eq!(config.cb_degradation_pct(), 50.0);
        assert_eq!(config.cb_max_rotations(), 3);
        assert_eq!(config.cb_recovery_cooldown(), Duration::from_secs(30));
        assert_eq!(config.health_window_size(), 50);
        assert!(config.health_aware_failover());
        assert!(config.failure_journal_enabled());
        assert_eq!(config.failure_journal_max_records(), 10_000);
    }

    #[test]
    fn config_serde_roundtrip() {
        let config = ResilienceConfig {
            retry_max_attempts: Some(3),
            retry_jitter_enabled: Some(false),
            cb_recovery_cooldown_secs: Some(60),
            health_aware_failover: Some(false),
            ..Default::default()
        };
        let json = serde_json::to_string(&config).unwrap();
        let parsed: ResilienceConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.retry_max_attempts(), 3);
        assert!(!parsed.retry_jitter_enabled());
        assert_eq!(parsed.cb_recovery_cooldown(), Duration::from_secs(60));
        assert!(!parsed.health_aware_failover());
    }

    #[test]
    fn config_toml_roundtrip() {
        let config = ResilienceConfig {
            retry_max_attempts: Some(7),
            failure_journal_max_records: Some(5000),
            ..Default::default()
        };
        let toml_str = toml::to_string(&config).unwrap();
        let parsed: ResilienceConfig = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.retry_max_attempts(), 7);
        assert_eq!(parsed.failure_journal_max_records(), 5000);
    }

    // ── FailureCategory Display ─────────────────────────────────────────────

    #[test]
    fn failure_category_display() {
        assert_eq!(format!("{}", FailureCategory::RateLimit), "RateLimit");
        assert_eq!(format!("{}", FailureCategory::StreamInterrupted), "StreamInterrupted");
    }
}
