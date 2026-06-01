#![allow(dead_code)] // Staged wave6 / Phase 53 module — wired up in a later cycle
//! Skills hot-reload watcher — `notify`-backed file-system observer for the
//! skills directory.
//!
//! Companion to the `SkillCatalog` work shipped under B1 (PR #11). A10
//! from the v13 fitgap (Phase 53 P1) calls for two pieces:
//!
//!   1. `notify`-based watcher on the skills directory.
//!   2. Real-time skill execution progress display via the existing
//!      event bus.
//!
//! This module is piece (1). Piece (2) is a UI integration that depends
//! on the daemon's event bus surface and is tracked separately.
//!
//! Behaviour:
//! - Watches a directory non-recursively for create / modify / remove.
//! - Filters to `*.md` only — non-markdown files (READMEs in other
//!   formats, dotfiles, editor swap files) are dropped on the floor.
//! - Debounces rapid changes (default 250 ms): a burst of events
//!   collapses into one [`SkillEvent::SkillsChanged`] frame whose
//!   `paths` field carries every distinct path touched in the window.
//! - Drops the [`SkillWatcher`] to stop watching.
//!
//! Architecture: a single dispatcher thread sits between `notify`'s raw
//! event channel and the public batched channel. It buffers every
//! relevant raw event into a `HashSet<PathBuf>`, restarts a debounce
//! timer, and flushes the set as one [`SkillEvent::SkillsChanged`]
//! frame after the configured idle window. Drop semantics: dropping
//! the [`SkillWatcher`] sets a stop flag and the dispatcher exits its
//! recv loop after the next deadline.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, RecvTimeoutError};
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkillEvent {
    /// One or more skill files changed within the debounce window.
    /// `paths` are unique and sorted for deterministic test assertions.
    SkillsChanged { paths: Vec<PathBuf> },
}

#[derive(Debug, Clone)]
pub struct SkillWatcherConfig {
    /// Window after the last raw event before flushing a batch.
    /// Default 250 ms, balancing latency vs. fsync-storm absorption.
    pub debounce: Duration,
}

impl Default for SkillWatcherConfig {
    fn default() -> Self {
        Self {
            debounce: Duration::from_millis(250),
        }
    }
}

/// Owning handle for a running skill watcher. Drop it to stop watching.
/// The channel returned by [`SkillWatcher::start`] receives debounced
/// [`SkillEvent`] frames.
pub struct SkillWatcher {
    /// Background dispatcher thread join handle. Detached on drop —
    /// the notify watcher's close drives the dispatcher's recv loop
    /// to return.
    _dispatcher: std::thread::JoinHandle<()>,
    /// Underlying notify watcher; dropped after `_dispatcher` per
    /// declaration order, but the stop flag short-circuits the
    /// dispatcher first so neither order produces a deadlock.
    _inner: RecommendedWatcher,
    /// Signal the dispatcher to exit (set in [`Drop`]).
    stop_flag: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl SkillWatcher {
    /// Begin watching `dir` for `*.md` changes. Returns the watcher
    /// (caller holds for its lifetime) and a receiver for batched
    /// events.
    pub fn start(dir: &Path, config: SkillWatcherConfig) -> Result<(Self, Receiver<SkillEvent>)> {
        let (raw_tx, raw_rx) = channel::<notify::Result<Event>>();
        let mut watcher = notify::recommended_watcher(move |res| {
            // notify guarantees the channel survives the watcher
            // dropping it; ignore send errors which only happen after
            // the dispatcher exits.
            let _ = raw_tx.send(res);
        })
        .context("notify::recommended_watcher")?;
        watcher
            .watch(dir, RecursiveMode::NonRecursive)
            .with_context(|| format!("watch {}", dir.display()))?;

        let (out_tx, out_rx) = channel::<SkillEvent>();
        let stop_flag = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let stop_flag_thread = stop_flag.clone();
        let debounce = config.debounce;

        let dispatcher = std::thread::Builder::new()
            .name("skill-watcher-dispatcher".to_string())
            .spawn(move || {
                run_dispatcher(raw_rx, out_tx, debounce, stop_flag_thread);
            })
            .context("spawn skill-watcher dispatcher thread")?;

        Ok((
            Self {
                _dispatcher: dispatcher,
                _inner: watcher,
                stop_flag,
            },
            out_rx,
        ))
    }
}

/// Pull raw notify events, keep a rolling debounce deadline, and flush
/// one [`SkillEvent::SkillsChanged`] when the channel idles for the
/// configured window.
fn run_dispatcher(
    raw_rx: std::sync::mpsc::Receiver<notify::Result<Event>>,
    out_tx: std::sync::mpsc::Sender<SkillEvent>,
    debounce: Duration,
    stop_flag: std::sync::Arc<std::sync::atomic::AtomicBool>,
) {
    let mut pending: BTreeSet<PathBuf> = BTreeSet::new();
    let mut deadline: Option<Instant> = None;

    loop {
        if stop_flag.load(std::sync::atomic::Ordering::SeqCst) {
            return;
        }

        let timeout = match deadline {
            Some(d) => d
                .checked_duration_since(Instant::now())
                .unwrap_or(Duration::from_millis(0)),
            None => Duration::from_millis(100),
        };

        match raw_rx.recv_timeout(timeout) {
            Ok(Ok(event)) => {
                if !is_relevant(&event) {
                    continue;
                }
                for p in event.paths {
                    if is_markdown(&p) {
                        pending.insert(p);
                    }
                }
                if !pending.is_empty() {
                    deadline = Some(Instant::now() + debounce);
                }
            }
            Ok(Err(_)) => {
                // notify error: keep pumping. Permanent failures
                // (e.g. inotify limit hit) surface as a closed
                // channel; the next iteration's Disconnected branch
                // exits cleanly.
                continue;
            }
            Err(RecvTimeoutError::Timeout) => {
                // Either the debounce window elapsed (pending is
                // non-empty → flush) or there's nothing to do (pending
                // is empty → spin-wait for the stop flag). The
                // 100ms idle timeout above bounds the latter.
                if !pending.is_empty() && deadline.map(|d| Instant::now() >= d).unwrap_or(true) {
                    let paths = std::mem::take(&mut pending).into_iter().collect();
                    if out_tx.send(SkillEvent::SkillsChanged { paths }).is_err() {
                        return;
                    }
                    deadline = None;
                }
            }
            Err(RecvTimeoutError::Disconnected) => return,
        }
    }
}

/// Filter notify events to the kinds that change skill content.
/// Access (read-only stat) and metadata-only events don't matter for
/// reload semantics; they would otherwise produce noisy frames.
fn is_relevant(event: &Event) -> bool {
    matches!(
        event.kind,
        EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
    )
}

fn is_markdown(p: &Path) -> bool {
    p.extension().and_then(|e| e.to_str()) == Some("md")
}

impl Drop for SkillWatcher {
    fn drop(&mut self) {
        self.stop_flag
            .store(true, std::sync::atomic::Ordering::SeqCst);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;
    use std::sync::mpsc::RecvTimeoutError;
    use tempfile::tempdir;

    /// Recv timeout = debounce + a margin for FS events to propagate
    /// through the kernel + notify's internal queue. CI on slow runners
    /// occasionally hits ~1.5s on macOS; 2s is safe but not so long
    /// that a green run blocks longer than necessary.
    const RECV_TIMEOUT: Duration = Duration::from_secs(2);

    fn quick_config() -> SkillWatcherConfig {
        SkillWatcherConfig {
            debounce: Duration::from_millis(150),
        }
    }

    fn write_skill(dir: &Path, name: &str, body: &str) -> PathBuf {
        let p = dir.join(format!("{name}.md"));
        fs::write(&p, body).unwrap();
        p
    }

    // ── Scenario 1: new .md file emits SkillsChanged ─────────────────────────

    #[test]
    fn watcher_emits_skills_changed_when_new_md_is_created() {
        let dir = tempdir().unwrap();
        let (_w, rx) = SkillWatcher::start(dir.path(), quick_config()).unwrap();

        let p = write_skill(dir.path(), "agent-loops", "# Agent\n");
        let ev = rx
            .recv_timeout(RECV_TIMEOUT)
            .expect("expected SkillsChanged within timeout");
        match ev {
            SkillEvent::SkillsChanged { paths } => {
                assert!(
                    paths.iter().any(|q| q == &p),
                    "expected {} in paths {:?}",
                    p.display(),
                    paths
                );
            }
        }
    }

    // ── Scenario 2: rapid changes debounce into one batch ────────────────────

    #[test]
    fn watcher_debounces_rapid_changes_into_one_batch() {
        let dir = tempdir().unwrap();
        let (_w, rx) = SkillWatcher::start(dir.path(), quick_config()).unwrap();

        // Three writes within the debounce window — expect one frame
        // with all three paths.
        let p1 = write_skill(dir.path(), "a", "1");
        let p2 = write_skill(dir.path(), "b", "1");
        let p3 = write_skill(dir.path(), "c", "1");

        let ev = rx
            .recv_timeout(RECV_TIMEOUT)
            .expect("expected one batched SkillsChanged");
        let SkillEvent::SkillsChanged { paths } = ev;
        assert!(paths.iter().any(|q| q == &p1));
        assert!(paths.iter().any(|q| q == &p2));
        assert!(paths.iter().any(|q| q == &p3));

        // No second batch should arrive within a debounce window of
        // silence — confirms debounce, not just "one event per write".
        let next = rx.recv_timeout(Duration::from_millis(400));
        assert!(
            matches!(next, Err(RecvTimeoutError::Timeout)),
            "expected no second batch; got {:?}",
            next
        );
    }

    // ── Scenario 3: non-md files are ignored ─────────────────────────────────

    #[test]
    fn watcher_ignores_non_md_files() {
        let dir = tempdir().unwrap();
        let (_w, rx) = SkillWatcher::start(dir.path(), quick_config()).unwrap();

        fs::write(dir.path().join("README.txt"), "ignored").unwrap();
        fs::write(dir.path().join(".DS_Store"), "ignored").unwrap();

        let next = rx.recv_timeout(Duration::from_millis(500));
        assert!(
            matches!(next, Err(RecvTimeoutError::Timeout)),
            "non-md activity must not emit; got {:?}",
            next
        );
    }

    // ── Scenario 4: modifying an existing .md emits SkillsChanged ───────────

    #[test]
    fn watcher_emits_on_modify_existing_md() {
        let dir = tempdir().unwrap();
        let p = write_skill(dir.path(), "design", "# Original\n");
        let (_w, rx) = SkillWatcher::start(dir.path(), quick_config()).unwrap();

        // Drain any startup noise (some platforms emit on watch start).
        let _ = rx.recv_timeout(Duration::from_millis(200));

        fs::write(&p, "# Edited\n").unwrap();
        let ev = rx
            .recv_timeout(RECV_TIMEOUT)
            .expect("expected SkillsChanged after modify");
        let SkillEvent::SkillsChanged { paths } = ev;
        assert!(paths.iter().any(|q| q == &p));
    }

    // ── Scenario 5: removing an .md emits SkillsChanged ─────────────────────

    #[test]
    fn watcher_emits_on_remove_md() {
        let dir = tempdir().unwrap();
        let p = write_skill(dir.path(), "stale", "# Stale\n");
        let (_w, rx) = SkillWatcher::start(dir.path(), quick_config()).unwrap();

        let _ = rx.recv_timeout(Duration::from_millis(200));

        fs::remove_file(&p).unwrap();
        let ev = rx
            .recv_timeout(RECV_TIMEOUT)
            .expect("expected SkillsChanged after remove");
        let SkillEvent::SkillsChanged { paths } = ev;
        assert!(paths.iter().any(|q| q == &p));
    }

    // ── Scenario 6: dropping the watcher closes the channel ─────────────────

    #[test]
    fn dropping_watcher_closes_channel() {
        let dir = tempdir().unwrap();
        let (w, rx) = SkillWatcher::start(dir.path(), quick_config()).unwrap();
        drop(w);
        // After drop, the dispatcher thread exits and the channel is
        // either disconnected or returns Timeout very quickly. Either
        // outcome is acceptable; what we care about is that no further
        // SkillEvent::SkillsChanged ever arrives.
        let result = rx.recv_timeout(Duration::from_millis(500));
        assert!(
            matches!(
                result,
                Err(RecvTimeoutError::Disconnected) | Err(RecvTimeoutError::Timeout)
            ),
            "after drop expected disconnect or timeout; got {:?}",
            result
        );
    }
}
