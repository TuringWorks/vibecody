//! One shared `reqwest::Client` per timeout, for the whole process.
//!
//! A `reqwest::Client` is not a handle — it *owns* a connection pool, a DNS
//! resolver with its own cache, and a TLS `ClientConfig`. Building one per
//! request therefore does more than allocate: it throws away keep-alive, so
//! every call pays a fresh TCP (and TLS) handshake, and it re-resolves a name
//! the last client had already cached.
//!
//! That is not hypothetical here. VibeCoder's session pollers ran on a 2-second
//! interval and built a client each time — 3,600 clients an hour, and 3,600
//! handshakes to a daemon on localhost that a single client would have made
//! once. `Client` is internally `Arc`-based, so cloning one out of this pool is
//! cheap and sharing it across tasks is the intended usage.
//!
//! ```no_run
//! let client = vibe_http_pool::client(10);
//! // ... use it; do not store it, just call again next time.
//! ```
//!
//! Keyed by timeout because that is the only per-caller knob in practice, and
//! almost every caller wants the default — so this is usually one client for
//! the entire process. A caller needing genuinely different configuration
//! (proxies, custom roots, redirect policy) should build and hold its own.

use std::sync::{OnceLock, RwLock};

/// One client per distinct timeout.
fn pool() -> &'static RwLock<Vec<(u64, reqwest::Client)>> {
    static POOL: OnceLock<RwLock<Vec<(u64, reqwest::Client)>>> = OnceLock::new();
    POOL.get_or_init(|| RwLock::new(Vec::new()))
}

/// The shared client for `timeout_secs`, creating it on first use.
///
/// Never fails: a client that cannot be built with the requested timeout falls
/// back to a default one rather than propagating, because every caller's
/// alternative is to make the request anyway.
pub fn client(timeout_secs: u64) -> reqwest::Client {
    let pool = pool();
    // Read path first — the steady state is a hit.
    if let Ok(guard) = pool.read() {
        if let Some((_, c)) = guard.iter().find(|(t, _)| *t == timeout_secs) {
            return c.clone();
        }
    }
    let built = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(timeout_secs))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());
    if let Ok(mut guard) = pool.write() {
        // Another thread may have inserted while we built; prefer theirs so the
        // pool stays one-client-per-timeout rather than growing a duplicate.
        if let Some((_, c)) = guard.iter().find(|(t, _)| *t == timeout_secs) {
            return c.clone();
        }
        guard.push((timeout_secs, built.clone()));
    }
    built
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The property the whole crate exists for: asking twice must not build a
    /// second client, or the connection pool is discarded between calls.
    #[test]
    fn the_same_timeout_reuses_one_pooled_client() {
        let _ = client(4242);
        let _ = client(4242);
        let guard = pool().read().expect("pool readable");
        assert_eq!(
            guard.iter().filter(|(t, _)| *t == 4242).count(),
            1,
            "a second call built a second client"
        );
    }

    #[test]
    fn distinct_timeouts_get_distinct_entries() {
        let _ = client(4243);
        let _ = client(4244);
        let guard = pool().read().expect("pool readable");
        assert!(guard.iter().any(|(t, _)| *t == 4243));
        assert!(guard.iter().any(|(t, _)| *t == 4244));
    }

    #[test]
    fn repeated_calls_do_not_grow_the_pool() {
        for _ in 0..50 {
            let _ = client(4245);
        }
        let guard = pool().read().expect("pool readable");
        assert_eq!(guard.iter().filter(|(t, _)| *t == 4245).count(), 1);
    }
}
