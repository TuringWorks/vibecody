//! Webhook delivery with exponential-backoff retry and dead-letter — M5.
//!
//! Background jobs that carry a `webhook_url` fire a terminal-state
//! callback once the job reaches `complete`, `failed`, or `cancelled`.
//! Earlier milestones fired a single best-effort POST with no retry;
//! M5 adds bounded exponential backoff plus durable outcome tracking so
//! operators can tell which webhooks made it and which landed in the
//! dead-letter state.
//!
//! The retry loop is intentionally generic over the "send" closure so
//! tests can drive deterministic failure sequences without running a
//! real HTTP server.

use std::time::Duration;

/// Retry policy. Defaults match what production code uses; tests override
/// `base_delay` to keep the suite fast.
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Max attempts, including the first. `1` = no retry.
    pub max_attempts: u32,
    /// Backoff base — delay between attempt N and N+1 is
    /// `base_delay * 2^(N-1)`, clamped to `max_delay`.
    pub base_delay: Duration,
    /// Upper bound on any single sleep.
    pub max_delay: Duration,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 5,
            base_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(30),
        }
    }
}

impl RetryConfig {
    /// Deterministic test profile — zero sleeps.
    pub fn instant_for_tests(max_attempts: u32) -> Self {
        Self {
            max_attempts,
            base_delay: Duration::from_millis(0),
            max_delay: Duration::from_millis(0),
        }
    }

    fn delay_for_attempt(&self, prev_attempt: u32) -> Duration {
        if self.base_delay.is_zero() {
            return Duration::from_millis(0);
        }
        let shift = prev_attempt.saturating_sub(1).min(20);
        let scaled = self.base_delay.saturating_mul(1u32 << shift);
        scaled.min(self.max_delay)
    }
}

/// One delivery attempt's outcome.
#[derive(Debug, Clone)]
pub enum AttemptOutcome {
    /// 2xx response — stop retrying.
    Success(u16),
    /// Transport error or non-2xx — retry if budget remains.
    Transient(String),
}

/// Final outcome after retries are exhausted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WebhookOutcome {
    Delivered {
        status: u16,
        attempts: u32,
    },
    DeadLetter {
        attempts: u32,
        last_error: String,
    },
}

impl WebhookOutcome {
    pub fn status_str(&self) -> &'static str {
        match self {
            WebhookOutcome::Delivered { .. } => "delivered",
            WebhookOutcome::DeadLetter { .. } => "dead_letter",
        }
    }

    pub fn is_delivered(&self) -> bool {
        matches!(self, WebhookOutcome::Delivered { .. })
    }
}

/// Drive the retry loop against a caller-supplied send closure. The caller
/// owns the HTTP client; this module only orchestrates retry + backoff so
/// it can be unit-tested without a real network.
pub async fn deliver_with_retry<F, Fut>(
    cfg: &RetryConfig,
    mut send: F,
) -> WebhookOutcome
where
    F: FnMut(u32) -> Fut,
    Fut: std::future::Future<Output = AttemptOutcome>,
{
    let mut last_error = String::from("no attempts");
    for attempt in 1..=cfg.max_attempts {
        match send(attempt).await {
            AttemptOutcome::Success(status) => {
                return WebhookOutcome::Delivered { status, attempts: attempt };
            }
            AttemptOutcome::Transient(err) => {
                last_error = err;
                if attempt < cfg.max_attempts {
                    let delay = cfg.delay_for_attempt(attempt);
                    if !delay.is_zero() {
                        tokio::time::sleep(delay).await;
                    }
                }
            }
        }
    }
    WebhookOutcome::DeadLetter {
        attempts: cfg.max_attempts,
        last_error,
    }
}

/// Convenience wrapper that does the reqwest-based POST behind the retry
/// loop. Classifies any non-2xx status as `Transient`.
pub async fn deliver_json(
    client: &reqwest::Client,
    url: &str,
    payload: &serde_json::Value,
    cfg: &RetryConfig,
    per_attempt_timeout: Duration,
) -> WebhookOutcome {
    deliver_with_retry(cfg, |_attempt| async {
        let resp = client
            .post(url)
            .json(payload)
            .timeout(per_attempt_timeout)
            .send()
            .await;
        match resp {
            Ok(r) => {
                let status = r.status().as_u16();
                if r.status().is_success() {
                    AttemptOutcome::Success(status)
                } else {
                    AttemptOutcome::Transient(format!("HTTP {status}"))
                }
            }
            Err(e) => AttemptOutcome::Transient(format!("send error: {e}")),
        }
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    #[tokio::test]
    async fn first_attempt_success_records_one_attempt() {
        let cfg = RetryConfig::instant_for_tests(5);
        let out = deliver_with_retry(&cfg, |_| async { AttemptOutcome::Success(200) }).await;
        assert_eq!(out, WebhookOutcome::Delivered { status: 200, attempts: 1 });
    }

    #[tokio::test]
    async fn retries_until_success() {
        let cfg = RetryConfig::instant_for_tests(5);
        let counter = Arc::new(AtomicU32::new(0));
        let c = counter.clone();
        let out = deliver_with_retry(&cfg, move |_| {
            let c = c.clone();
            async move {
                let n = c.fetch_add(1, Ordering::SeqCst) + 1;
                if n < 3 {
                    AttemptOutcome::Transient(format!("HTTP 503 (try {n})"))
                } else {
                    AttemptOutcome::Success(202)
                }
            }
        })
        .await;
        assert_eq!(out, WebhookOutcome::Delivered { status: 202, attempts: 3 });
        assert_eq!(counter.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn exhausts_attempts_and_becomes_dead_letter() {
        let cfg = RetryConfig::instant_for_tests(4);
        let counter = Arc::new(AtomicU32::new(0));
        let c = counter.clone();
        let out = deliver_with_retry(&cfg, move |_| {
            let c = c.clone();
            async move {
                let n = c.fetch_add(1, Ordering::SeqCst) + 1;
                AttemptOutcome::Transient(format!("boom {n}"))
            }
        })
        .await;
        match out {
            WebhookOutcome::DeadLetter { attempts, last_error } => {
                assert_eq!(attempts, 4);
                assert!(last_error.contains("boom 4"), "got {last_error:?}");
            }
            other => panic!("expected DeadLetter, got {other:?}"),
        }
        assert_eq!(counter.load(Ordering::SeqCst), 4);
    }

    #[test]
    fn delay_schedule_doubles_and_clamps() {
        let cfg = RetryConfig {
            max_attempts: 10,
            base_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(8),
        };
        assert_eq!(cfg.delay_for_attempt(1), Duration::from_secs(1));
        assert_eq!(cfg.delay_for_attempt(2), Duration::from_secs(2));
        assert_eq!(cfg.delay_for_attempt(3), Duration::from_secs(4));
        assert_eq!(cfg.delay_for_attempt(4), Duration::from_secs(8));
        // Clamped at max_delay, not doubled further.
        assert_eq!(cfg.delay_for_attempt(5), Duration::from_secs(8));
        assert_eq!(cfg.delay_for_attempt(15), Duration::from_secs(8));
    }

    #[test]
    fn instant_profile_has_zero_sleeps() {
        let cfg = RetryConfig::instant_for_tests(5);
        for n in 1..=5 {
            assert_eq!(cfg.delay_for_attempt(n), Duration::from_millis(0));
        }
    }
}
