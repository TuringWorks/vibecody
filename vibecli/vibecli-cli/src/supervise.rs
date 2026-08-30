//! Keep a background loop alive across its own failures.
//!
//! A `tokio::spawn`ed loop is unsupervised by construction: when its future
//! panics or returns, the task is gone and *nothing says so*. The daemon keeps
//! serving, so no client notices — the workflow reaper stops enforcing
//! timeouts, the announcer stops announcing, the index stops refreshing, and
//! the only symptom is a feature that quietly stopped working hours ago.
//!
//! This is the other half of [`crate::serve`]'s `CatchPanicLayer`: that one
//! contains a panic on the request path, this one contains a panic off it.
//! Both need `panic = "unwind"` — with `abort` there is nothing to contain, and
//! the workspace profile says so.
//!
//! ```no_run
//! # use vibecli_cli::supervise::spawn_supervised;
//! spawn_supervised("fluxo-reaper", || async {
//!     loop {
//!         tokio::time::sleep(std::time::Duration::from_secs(15)).await;
//!         // …work that may panic…
//!     }
//! });
//! ```

use std::future::Future;
use std::time::{Duration, Instant};

/// First wait after a failure. Doubles up to [`MAX_BACKOFF`].
const INITIAL_BACKOFF: Duration = Duration::from_secs(1);

/// Ceiling on the wait between restarts. A loop that fails permanently must not
/// spin, and must not give up either — the daemon runs for days and the cause
/// (a locked database, a busy port) is usually temporary.
const MAX_BACKOFF: Duration = Duration::from_secs(60);

/// How long a run must last to count as healthy, resetting the backoff. A task
/// that survives this long and then fails is a new incident, not a retry of the
/// old one.
const HEALTHY_AFTER: Duration = Duration::from_secs(60);

/// Run `make()`'s future, restarting it whenever it panics or returns.
///
/// `name` appears in every log line — the point is that a background failure
/// stops being silent, so an unnamed supervisor would be half the fix.
pub fn spawn_supervised<F, Fut>(name: &'static str, make: F) -> tokio::task::JoinHandle<()>
where
    F: FnMut() -> Fut + Send + 'static,
    Fut: Future<Output = ()> + Send + 'static,
{
    spawn_supervised_with(name, INITIAL_BACKOFF, make)
}

/// [`spawn_supervised`] with the first backoff spelled out, so a test does not
/// have to wait a second to observe a restart.
pub fn spawn_supervised_with<F, Fut>(
    name: &'static str,
    initial_backoff: Duration,
    mut make: F,
) -> tokio::task::JoinHandle<()>
where
    F: FnMut() -> Fut + Send + 'static,
    Fut: Future<Output = ()> + Send + 'static,
{
    tokio::spawn(async move {
        let mut backoff = initial_backoff;
        loop {
            let started = Instant::now();
            // Spawned rather than awaited inline: a panic in a nested task is
            // returned as a `JoinError`, whereas awaiting the future here would
            // unwind this supervisor along with it.
            let outcome = tokio::spawn(make()).await;
            let ran_for = started.elapsed();

            match outcome {
                Err(e) if e.is_cancelled() => {
                    // Someone aborted it deliberately. Restarting would fight
                    // the caller.
                    tracing::info!("supervise: {name} was cancelled; not restarting");
                    return;
                }
                Err(e) => tracing::error!(
                    "supervise: {name} panicked after {:?}; restarting in {:?} ({e})",
                    ran_for,
                    backoff
                ),
                Ok(()) => tracing::warn!(
                    "supervise: {name} returned after {:?}; restarting in {:?}",
                    ran_for,
                    backoff
                ),
            }

            if ran_for >= HEALTHY_AFTER {
                backoff = initial_backoff;
            }
            tokio::time::sleep(backoff).await;
            backoff = (backoff * 2).min(MAX_BACKOFF);
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    #[tokio::test]
    async fn a_panicking_loop_comes_back() {
        // The failure this exists to prevent: a background loop panics once and
        // is gone for the life of the daemon, with nothing on screen and
        // nothing in the log to say the feature stopped.
        let runs = Arc::new(AtomicUsize::new(0));
        let seen = Arc::clone(&runs);
        let (tx, rx) = tokio::sync::oneshot::channel();
        let tx = Arc::new(std::sync::Mutex::new(Some(tx)));

        let handle = spawn_supervised_with("test-loop", Duration::from_millis(1), move || {
            let seen = Arc::clone(&seen);
            let tx = Arc::clone(&tx);
            async move {
                let n = seen.fetch_add(1, Ordering::SeqCst);
                if n < 2 {
                    panic!("attempt {n} fails");
                }
                if let Ok(mut slot) = tx.lock() {
                    if let Some(tx) = slot.take() {
                        let _ = tx.send(n);
                    }
                }
                // Stay alive, like a real loop, so the supervisor is not just
                // observed spinning.
                std::future::pending::<()>().await;
            }
        });

        let reached = tokio::time::timeout(Duration::from_secs(5), rx)
            .await
            .expect("the supervisor must restart past the panics")
            .expect("the third run must report in");
        assert_eq!(reached, 2, "two failures, then a healthy run");
        handle.abort();
    }

    #[tokio::test]
    async fn a_returning_loop_is_restarted_too() {
        // A loop that falls out of its own `while` is as dead as one that
        // panicked, and just as silent.
        let runs = Arc::new(AtomicUsize::new(0));
        let seen = Arc::clone(&runs);
        let handle = spawn_supervised_with("test-return", Duration::from_millis(1), move || {
            let seen = Arc::clone(&seen);
            async move {
                seen.fetch_add(1, Ordering::SeqCst);
            }
        });

        // Poll rather than sleeping a guess: the restarts are milliseconds apart.
        let deadline = Instant::now() + Duration::from_secs(5);
        while runs.load(Ordering::SeqCst) < 3 && Instant::now() < deadline {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        assert!(
            runs.load(Ordering::SeqCst) >= 3,
            "a returning task must be restarted, got {} run(s)",
            runs.load(Ordering::SeqCst)
        );
        handle.abort();
    }
}
