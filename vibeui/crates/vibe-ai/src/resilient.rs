//! Resilience utilities: retry with exponential backoff for HTTP and AI providers.
//!
//! Two complementary layers:
//!
//! 1. **`retry_async`** — generic retry wrapper for any `async fn() -> Result<T>`.
//!    Use this for one-off HTTP calls (JIRA, GitHub, OAuth, sandbox, etc.).
//!
//! 2. **`ResilientProvider`** — decorator that wraps any `AIProvider`, adding
//!    automatic retry with exponential backoff to `chat`, `stream_chat`, `complete`,
//!    `chat_response`, and `chat_with_images`.

use crate::provider::{
    AIProvider, CodeContext, CompletionResponse, CompletionStream, ImageAttachment, Message,
};
use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;

// ── Retry Configuration ──────────────────────────────────────────────────────

/// Configuration for retry behaviour on transient errors.
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of attempts (including the initial one).
    pub max_attempts: u32,
    /// Initial backoff duration in milliseconds.
    pub initial_backoff_ms: u64,
    /// Maximum backoff duration in milliseconds.
    pub max_backoff_ms: u64,
    /// Multiplier applied to backoff after each failed attempt.
    pub backoff_multiplier: f64,
    /// Add random jitter (0–25% of backoff) to avoid thundering herd.
    pub jitter: bool,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 4,
            initial_backoff_ms: 1_000,
            max_backoff_ms: 30_000,
            backoff_multiplier: 2.0,
            jitter: true,
        }
    }
}

impl RetryConfig {
    /// Calculate backoff duration for the given attempt (0-indexed).
    fn backoff_ms(&self, attempt: u32) -> u64 {
        let base = self.initial_backoff_ms as f64 * self.backoff_multiplier.powi(attempt as i32);
        let capped = (base as u64).min(self.max_backoff_ms);
        if self.jitter {
            // Add 0–25% jitter
            let jitter = (capped as f64 * 0.25 * rand_f64()) as u64;
            capped + jitter
        } else {
            capped
        }
    }
}

/// Simple pseudo-random f64 in [0, 1) using thread-local state.
/// Not cryptographic, just enough for jitter.
fn rand_f64() -> f64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    use std::time::SystemTime;

    let mut h = DefaultHasher::new();
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
        .hash(&mut h);
    std::thread::current().id().hash(&mut h);
    (h.finish() % 10_000) as f64 / 10_000.0
}

/// Classify an error string as retryable (transient) or permanent.
pub fn is_retryable(error: &str) -> bool {
    let lower = error.to_lowercase();
    lower.contains("rate limit")
        || lower.contains("429")
        || lower.contains("503")
        || lower.contains("529")
        || lower.contains("502")
        || lower.contains("504")
        || lower.contains("overloaded")
        || lower.contains("timeout")
        || lower.contains("timed out")
        || lower.contains("connection reset")
        || lower.contains("connection refused")
        || lower.contains("connection closed")
        || lower.contains("broken pipe")
        || lower.contains("error decoding")
        || lower.contains("unexpected eof")
        || lower.contains("incomplete message")
        || lower.contains("network")
        || lower.contains("dns")
        || lower.contains("temporarily unavailable")
        || lower.contains("bad gateway")
        || lower.contains("gateway timeout")
        || lower.contains("hyper")
        || lower.contains("h2 protocol error")
        || lower.contains("stream closed")
}

// ── Generic Retry Wrapper ────────────────────────────────────────────────────

/// Retry an async operation with exponential backoff.
///
/// ```rust,ignore
/// let body = retry_async(&RetryConfig::default(), "fetch issue", || async {
///     client.get(url).send().await?.text().await.map_err(Into::into)
/// }).await?;
/// ```
pub async fn retry_async<F, Fut, T>(config: &RetryConfig, label: &str, mut f: F) -> Result<T>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T>>,
{
    let mut last_err: Option<anyhow::Error> = None;

    for attempt in 0..config.max_attempts {
        if attempt > 0 {
            let backoff = config.backoff_ms(attempt - 1);
            tracing::warn!(
                label,
                attempt = attempt + 1,
                max = config.max_attempts,
                backoff_ms = backoff,
                "Retrying after transient error"
            );
            tokio::time::sleep(std::time::Duration::from_millis(backoff)).await;
        }

        match f().await {
            Ok(val) => return Ok(val),
            Err(e) => {
                let err_str = e.to_string();
                if is_retryable(&err_str) && attempt + 1 < config.max_attempts {
                    tracing::warn!(
                        label,
                        attempt = attempt + 1,
                        error = %err_str,
                        "Transient error, will retry"
                    );
                    last_err = Some(e);
                    continue;
                }
                // Non-retryable or exhausted attempts
                return Err(e);
            }
        }
    }

    Err(last_err.unwrap_or_else(|| anyhow::anyhow!("retry_async: exhausted all attempts")))
}

// ── Resilient Provider Wrapper ───────────────────────────────────────────────

/// Wraps any `AIProvider` with automatic retry + exponential backoff.
///
/// ```rust,ignore
/// let provider = ResilientProvider::wrap(my_provider);
/// // All calls now auto-retry on transient failures:
/// provider.chat(&messages, None).await?;
/// ```
pub struct ResilientProvider {
    inner: Arc<dyn AIProvider>,
    config: RetryConfig,
}

impl ResilientProvider {
    /// Wrap a provider with default retry config (4 attempts, 1s initial backoff).
    pub fn wrap(inner: Arc<dyn AIProvider>) -> Arc<Self> {
        Arc::new(Self {
            inner,
            config: RetryConfig::default(),
        })
    }

    /// Wrap with custom retry config.
    pub fn wrap_with_config(inner: Arc<dyn AIProvider>, config: RetryConfig) -> Arc<Self> {
        Arc::new(Self { inner, config })
    }
}

#[async_trait]
impl AIProvider for ResilientProvider {
    fn name(&self) -> &str {
        self.inner.name()
    }

    async fn is_available(&self) -> bool {
        self.inner.is_available().await
    }

    async fn complete(&self, context: &CodeContext) -> Result<CompletionResponse> {
        let ctx = context.clone();
        retry_async(&self.config, &format!("{}/complete", self.name()), || {
            let c = ctx.clone();
            let inner = self.inner.clone();
            async move { inner.complete(&c).await }
        })
        .await
    }

    async fn stream_complete(&self, context: &CodeContext) -> Result<CompletionStream> {
        let ctx = context.clone();
        retry_async(
            &self.config,
            &format!("{}/stream_complete", self.name()),
            || {
                let c = ctx.clone();
                let inner = self.inner.clone();
                async move { inner.stream_complete(&c).await }
            },
        )
        .await
    }

    async fn chat(&self, messages: &[Message], context: Option<String>) -> Result<String> {
        let msgs = messages.to_vec();
        let ctx = context.clone();
        retry_async(&self.config, &format!("{}/chat", self.name()), || {
            let m = msgs.clone();
            let c = ctx.clone();
            let inner = self.inner.clone();
            async move { inner.chat(&m, c).await }
        })
        .await
    }

    async fn stream_chat(&self, messages: &[Message]) -> Result<CompletionStream> {
        let msgs = messages.to_vec();
        retry_async(
            &self.config,
            &format!("{}/stream_chat", self.name()),
            || {
                let m = msgs.clone();
                let inner = self.inner.clone();
                async move { inner.stream_chat(&m).await }
            },
        )
        .await
    }

    async fn chat_response(
        &self,
        messages: &[Message],
        context: Option<String>,
    ) -> Result<CompletionResponse> {
        let msgs = messages.to_vec();
        let ctx = context.clone();
        retry_async(
            &self.config,
            &format!("{}/chat_response", self.name()),
            || {
                let m = msgs.clone();
                let c = ctx.clone();
                let inner = self.inner.clone();
                async move { inner.chat_response(&m, c).await }
            },
        )
        .await
    }

    async fn chat_with_images(
        &self,
        messages: &[Message],
        images: &[ImageAttachment],
        context: Option<String>,
    ) -> Result<String> {
        let msgs = messages.to_vec();
        let imgs = images.to_vec();
        let ctx = context.clone();
        retry_async(
            &self.config,
            &format!("{}/chat_with_images", self.name()),
            || {
                let m = msgs.clone();
                let i = imgs.clone();
                let c = ctx.clone();
                let inner = self.inner.clone();
                async move { inner.chat_with_images(&m, &i, c).await }
            },
        )
        .await
    }

    fn supports_vision(&self) -> bool {
        self.inner.supports_vision()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_retryable() {
        assert!(is_retryable("rate limit exceeded"));
        assert!(is_retryable("HTTP 429 Too Many Requests"));
        assert!(is_retryable("Service temporarily unavailable (503)"));
        assert!(is_retryable("connection reset by peer"));
        assert!(is_retryable("error decoding response body"));
        assert!(is_retryable("request timeout after 30s"));
        assert!(is_retryable("DNS resolution failed"));
        assert!(is_retryable("502 Bad Gateway"));
        assert!(is_retryable("504 Gateway Timeout"));
        assert!(is_retryable("h2 protocol error: stream closed"));
        // Non-retryable
        assert!(!is_retryable("401 Unauthorized"));
        assert!(!is_retryable("Invalid API key"));
        assert!(!is_retryable("Model not found"));
        assert!(!is_retryable("Content policy violation"));
    }

    #[test]
    fn test_backoff_calculation() {
        let config = RetryConfig {
            initial_backoff_ms: 1000,
            max_backoff_ms: 30_000,
            backoff_multiplier: 2.0,
            jitter: false,
            ..Default::default()
        };
        assert_eq!(config.backoff_ms(0), 1000);
        assert_eq!(config.backoff_ms(1), 2000);
        assert_eq!(config.backoff_ms(2), 4000);
        assert_eq!(config.backoff_ms(3), 8000);
        assert_eq!(config.backoff_ms(4), 16000);
        assert_eq!(config.backoff_ms(5), 30000); // capped
    }

    #[test]
    fn test_backoff_with_jitter() {
        let config = RetryConfig {
            initial_backoff_ms: 1000,
            max_backoff_ms: 30_000,
            backoff_multiplier: 2.0,
            jitter: true,
            ..Default::default()
        };
        let b = config.backoff_ms(0);
        // Should be between 1000 and 1250 (1000 + 25% jitter)
        assert!(b >= 1000 && b <= 1250, "backoff was {}", b);
    }

    #[tokio::test]
    async fn test_retry_async_succeeds_first_try() {
        let config = RetryConfig { max_attempts: 3, jitter: false, ..Default::default() };
        let result = retry_async(&config, "test", || async { Ok::<_, anyhow::Error>(42) }).await;
        assert_eq!(result.unwrap(), 42);
    }

    #[tokio::test]
    async fn test_retry_async_non_retryable_fails_immediately() {
        let config = RetryConfig {
            max_attempts: 3,
            initial_backoff_ms: 1, // tiny for test speed
            jitter: false,
            ..Default::default()
        };
        let counter = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
        let c = counter.clone();
        let result = retry_async(&config, "test", || {
            let c = c.clone();
            async move {
                c.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                Err::<i32, _>(anyhow::anyhow!("401 Unauthorized"))
            }
        })
        .await;
        assert!(result.is_err());
        // Should only try once (non-retryable error)
        assert_eq!(counter.load(std::sync::atomic::Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_retry_async_retries_on_transient_error() {
        let config = RetryConfig {
            max_attempts: 3,
            initial_backoff_ms: 1,
            jitter: false,
            ..Default::default()
        };
        let counter = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
        let c = counter.clone();
        let result = retry_async(&config, "test", || {
            let c = c.clone();
            async move {
                let n = c.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                if n < 2 {
                    Err::<i32, _>(anyhow::anyhow!("connection reset by peer"))
                } else {
                    Ok(99)
                }
            }
        })
        .await;
        assert_eq!(result.unwrap(), 99);
        assert_eq!(counter.load(std::sync::atomic::Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn test_retry_async_exhausts_attempts() {
        let config = RetryConfig {
            max_attempts: 2,
            initial_backoff_ms: 1,
            jitter: false,
            ..Default::default()
        };
        let counter = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
        let c = counter.clone();
        let result = retry_async(&config, "test", || {
            let c = c.clone();
            async move {
                c.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                Err::<i32, _>(anyhow::anyhow!("503 service unavailable"))
            }
        })
        .await;
        assert!(result.is_err());
        assert_eq!(counter.load(std::sync::atomic::Ordering::SeqCst), 2);
    }
}
