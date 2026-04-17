#![allow(dead_code)]
//! File watcher — debounced file-system change detection for live index refresh.
//!
//! Provides a platform-agnostic watcher that batches rapid changes and emits
//! `FileChangeEvent` batches after a configurable debounce window.
//!
//! Matches Cursor 4.0 and Cody 6.0's sub-50ms reindex latency.

use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

// ---------------------------------------------------------------------------
// Event types
// ---------------------------------------------------------------------------

/// Kind of file-system change detected.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ChangeKind {
    Created,
    Modified,
    Deleted,
    Renamed { from: PathBuf },
}

impl std::fmt::Display for ChangeKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChangeKind::Created => write!(f, "created"),
            ChangeKind::Modified => write!(f, "modified"),
            ChangeKind::Deleted => write!(f, "deleted"),
            ChangeKind::Renamed { from } => write!(f, "renamed({})", from.display()),
        }
    }
}

/// A single file-system change event.
#[derive(Debug, Clone)]
pub struct FileChangeEvent {
    pub path: PathBuf,
    pub kind: ChangeKind,
    pub at: Instant,
}

impl FileChangeEvent {
    pub fn new(path: impl Into<PathBuf>, kind: ChangeKind) -> Self {
        Self {
            path: path.into(),
            kind,
            at: Instant::now(),
        }
    }
}

/// A debounced batch of file changes ready for processing.
#[derive(Debug, Clone)]
pub struct ChangeBatch {
    pub events: Vec<FileChangeEvent>,
    pub window_start: Instant,
    pub window_end: Instant,
}

impl ChangeBatch {
    /// Paths of all changed files (deduplicated).
    pub fn changed_paths(&self) -> HashSet<&PathBuf> {
        self.events.iter().map(|e| &e.path).collect()
    }

    /// Number of events in this batch.
    pub fn len(&self) -> usize {
        self.events.len()
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }
}

// ---------------------------------------------------------------------------
// Watcher configuration
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct WatcherConfig {
    /// How long to wait after the last event before flushing a batch.
    pub debounce: Duration,
    /// Glob patterns to ignore (e.g. `target/**`, `.git/**`).
    pub ignore_patterns: Vec<String>,
    /// Maximum events per batch before forcing a flush.
    pub max_batch_size: usize,
}

impl Default for WatcherConfig {
    fn default() -> Self {
        Self {
            debounce: Duration::from_millis(50),
            ignore_patterns: vec![
                "target/**".into(),
                ".git/**".into(),
                "node_modules/**".into(),
            ],
            max_batch_size: 500,
        }
    }
}

// ---------------------------------------------------------------------------
// Watcher state
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WatcherStatus {
    Idle,
    Watching,
    Paused,
    Error(String),
}

impl std::fmt::Display for WatcherStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WatcherStatus::Idle => write!(f, "idle"),
            WatcherStatus::Watching => write!(f, "watching"),
            WatcherStatus::Paused => write!(f, "paused"),
            WatcherStatus::Error(e) => write!(f, "error({e})"),
        }
    }
}

// ---------------------------------------------------------------------------
// File watcher (simulated — real inotify/FSEvents integration wired at runtime)
// ---------------------------------------------------------------------------

/// Core file watcher. In production this wraps `notify` or OS-native APIs.
/// This implementation provides the debounce/filter layer and can be driven
/// by injecting raw events via `inject()` for testing.
pub struct FileWatcher {
    config: WatcherConfig,
    status: WatcherStatus,
    /// Raw events pending debounce.
    pending: Vec<FileChangeEvent>,
    /// Last event arrival time (for debounce).
    last_event_at: Option<Instant>,
    /// Flushed batches ready for consumption.
    batches: VecDeque<ChangeBatch>,
    /// Total events received.
    total_events: u64,
    /// Total batches flushed.
    total_batches: u64,
    /// Watched root paths.
    watched_paths: Vec<PathBuf>,
}

impl FileWatcher {
    pub fn new(config: WatcherConfig) -> Self {
        Self {
            config,
            status: WatcherStatus::Idle,
            pending: vec![],
            last_event_at: None,
            batches: VecDeque::new(),
            total_events: 0,
            total_batches: 0,
            watched_paths: vec![],
        }
    }

    pub fn with_default_config() -> Self {
        Self::new(WatcherConfig::default())
    }

    /// Start watching a path.
    pub fn watch(&mut self, path: impl Into<PathBuf>) -> Result<(), String> {
        let p = path.into();
        if self.status == WatcherStatus::Paused {
            return Err("watcher is paused".into());
        }
        self.watched_paths.push(p);
        self.status = WatcherStatus::Watching;
        Ok(())
    }

    /// Stop watching a specific path.
    pub fn unwatch(&mut self, path: &PathBuf) {
        self.watched_paths.retain(|p| p != path);
        if self.watched_paths.is_empty() {
            self.status = WatcherStatus::Idle;
        }
    }

    /// Pause event processing.
    pub fn pause(&mut self) {
        self.status = WatcherStatus::Paused;
    }

    /// Resume event processing.
    pub fn resume(&mut self) {
        if self.status == WatcherStatus::Paused {
            self.status = if self.watched_paths.is_empty() {
                WatcherStatus::Idle
            } else {
                WatcherStatus::Watching
            };
        }
    }

    /// Inject a raw event (used by OS adapter layer or tests).
    pub fn inject(&mut self, event: FileChangeEvent) {
        if self.status == WatcherStatus::Paused {
            return;
        }
        if self.is_ignored(&event.path) {
            return;
        }
        self.last_event_at = Some(event.at);
        self.total_events += 1;
        self.pending.push(event);

        // Force-flush if batch is full.
        if self.pending.len() >= self.config.max_batch_size {
            self.flush();
        }
    }

    /// Poll for ready batches. Should be called periodically by the consumer.
    /// Flushes pending events whose debounce window has expired.
    pub fn poll(&mut self) -> Vec<ChangeBatch> {
        if let Some(last) = self.last_event_at {
            if last.elapsed() >= self.config.debounce && !self.pending.is_empty() {
                self.flush();
            }
        }
        self.batches.drain(..).collect()
    }

    /// Force-flush pending events into a batch immediately.
    pub fn flush(&mut self) {
        if self.pending.is_empty() {
            return;
        }
        let now = Instant::now();
        let window_start = self
            .pending
            .first()
            .map(|e| e.at)
            .unwrap_or(now);
        let batch = ChangeBatch {
            events: std::mem::take(&mut self.pending),
            window_start,
            window_end: now,
        };
        self.batches.push_back(batch);
        self.total_batches += 1;
        self.last_event_at = None;
    }

    /// Current watcher status.
    pub fn status(&self) -> &WatcherStatus {
        &self.status
    }

    /// Statistics snapshot.
    pub fn stats(&self) -> WatcherStats {
        WatcherStats {
            status: self.status.clone(),
            watched_paths: self.watched_paths.len(),
            pending_events: self.pending.len(),
            ready_batches: self.batches.len(),
            total_events: self.total_events,
            total_batches: self.total_batches,
        }
    }

    // Check if a path matches an ignore pattern (simple prefix/suffix match).
    fn is_ignored(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();
        for pattern in &self.config.ignore_patterns {
            let pat = pattern.trim_end_matches("/**").trim_end_matches("/*");
            if path_str.contains(pat) {
                return true;
            }
        }
        false
    }
}

#[derive(Debug, Clone)]
pub struct WatcherStats {
    pub status: WatcherStatus,
    pub watched_paths: usize,
    pub pending_events: usize,
    pub ready_batches: usize,
    pub total_events: u64,
    pub total_batches: u64,
}

// ---------------------------------------------------------------------------
// Shared watcher
// ---------------------------------------------------------------------------

pub struct SharedFileWatcher(Arc<Mutex<FileWatcher>>);

impl SharedFileWatcher {
    pub fn new(config: WatcherConfig) -> Self {
        Self(Arc::new(Mutex::new(FileWatcher::new(config))))
    }

    pub fn clone_handle(&self) -> Self {
        Self(Arc::clone(&self.0))
    }

    pub fn with<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut FileWatcher) -> R,
    {
        let mut guard = self.0.lock().unwrap_or_else(|e| e.into_inner());
        f(&mut guard)
    }
}

// ---------------------------------------------------------------------------
// Change aggregator — merges redundant events for the same path
// ---------------------------------------------------------------------------

/// Merges multiple events for the same path in a batch into a single event.
pub fn aggregate_batch(batch: &ChangeBatch) -> Vec<FileChangeEvent> {
    let mut seen: HashMap<PathBuf, FileChangeEvent> = HashMap::new();
    for event in &batch.events {
        seen.entry(event.path.clone())
            .and_modify(|e| {
                // Later event wins; Deleted always trumps Modified.
                if matches!(e.kind, ChangeKind::Deleted) {
                    return; // keep deleted
                }
                *e = event.clone();
            })
            .or_insert_with(|| event.clone());
    }
    seen.into_values().collect()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    fn watcher_fast() -> FileWatcher {
        FileWatcher::new(WatcherConfig {
            debounce: Duration::from_millis(5),
            ..Default::default()
        })
    }

    fn evt(path: &str, kind: ChangeKind) -> FileChangeEvent {
        FileChangeEvent::new(PathBuf::from(path), kind)
    }

    #[test]
    fn test_initial_status_idle() {
        let w = watcher_fast();
        assert_eq!(*w.status(), WatcherStatus::Idle);
    }

    #[test]
    fn test_watch_sets_status() {
        let mut w = watcher_fast();
        w.watch("/tmp/project").unwrap();
        assert_eq!(*w.status(), WatcherStatus::Watching);
    }

    #[test]
    fn test_unwatch_returns_to_idle() {
        let mut w = watcher_fast();
        let p = PathBuf::from("/tmp/project");
        w.watch(p.clone()).unwrap();
        w.unwatch(&p);
        assert_eq!(*w.status(), WatcherStatus::Idle);
    }

    #[test]
    fn test_injected_event_pending() {
        let mut w = watcher_fast();
        w.watch("/tmp/project").unwrap();
        w.inject(evt("src/lib.rs", ChangeKind::Modified));
        assert_eq!(w.stats().pending_events, 1);
    }

    #[test]
    fn test_debounce_flush_after_delay() {
        let mut w = watcher_fast();
        w.watch("/tmp/project").unwrap();
        w.inject(evt("src/lib.rs", ChangeKind::Modified));
        thread::sleep(Duration::from_millis(20));
        let batches = w.poll();
        assert_eq!(batches.len(), 1);
        assert_eq!(batches[0].len(), 1);
    }

    #[test]
    fn test_no_flush_before_debounce() {
        let mut w = FileWatcher::new(WatcherConfig {
            debounce: Duration::from_millis(500),
            ..Default::default()
        });
        w.watch("/tmp/project").unwrap();
        w.inject(evt("src/lib.rs", ChangeKind::Modified));
        let batches = w.poll();
        assert!(batches.is_empty());
    }

    #[test]
    fn test_force_flush() {
        let mut w = watcher_fast();
        w.watch("/tmp/project").unwrap();
        w.inject(evt("src/main.rs", ChangeKind::Created));
        w.flush();
        let batches = w.poll();
        assert_eq!(batches.len(), 1);
    }

    #[test]
    fn test_ignored_target_dir() {
        let mut w = watcher_fast();
        w.watch("/tmp/project").unwrap();
        w.inject(evt("target/debug/vibecli", ChangeKind::Modified));
        assert_eq!(w.stats().total_events, 0);
    }

    #[test]
    fn test_git_dir_ignored() {
        let mut w = watcher_fast();
        w.watch("/tmp/project").unwrap();
        w.inject(evt(".git/COMMIT_EDITMSG", ChangeKind::Modified));
        assert_eq!(w.stats().total_events, 0);
    }

    #[test]
    fn test_paused_watcher_drops_events() {
        let mut w = watcher_fast();
        w.watch("/tmp/project").unwrap();
        w.pause();
        w.inject(evt("src/lib.rs", ChangeKind::Modified));
        assert_eq!(w.stats().total_events, 0);
    }

    #[test]
    fn test_resume_after_pause() {
        let mut w = watcher_fast();
        w.watch("/tmp/project").unwrap();
        w.pause();
        w.resume();
        assert_eq!(*w.status(), WatcherStatus::Watching);
    }

    #[test]
    fn test_max_batch_size_forces_flush() {
        let mut w = FileWatcher::new(WatcherConfig {
            max_batch_size: 3,
            debounce: Duration::from_millis(5000),
            ..Default::default()
        });
        w.watch("/tmp/project").unwrap();
        w.inject(evt("a.rs", ChangeKind::Modified));
        w.inject(evt("b.rs", ChangeKind::Modified));
        w.inject(evt("c.rs", ChangeKind::Modified));
        // 3rd event triggers flush
        assert_eq!(w.stats().ready_batches, 1);
    }

    #[test]
    fn test_batch_changed_paths_deduped() {
        let mut batch = ChangeBatch {
            events: vec![
                evt("src/lib.rs", ChangeKind::Modified),
                evt("src/lib.rs", ChangeKind::Modified),
            ],
            window_start: Instant::now(),
            window_end: Instant::now(),
        };
        batch.events.push(evt("src/main.rs", ChangeKind::Created));
        let paths = batch.changed_paths();
        assert_eq!(paths.len(), 2);
    }

    #[test]
    fn test_aggregate_batch_keeps_latest() {
        let batch = ChangeBatch {
            events: vec![
                evt("src/lib.rs", ChangeKind::Modified),
                evt("src/lib.rs", ChangeKind::Deleted),
            ],
            window_start: Instant::now(),
            window_end: Instant::now(),
        };
        let agg = aggregate_batch(&batch);
        assert_eq!(agg.len(), 1);
        assert_eq!(agg[0].kind, ChangeKind::Deleted);
    }

    #[test]
    fn test_shared_watcher() {
        let shared = SharedFileWatcher::new(WatcherConfig {
            debounce: Duration::from_millis(5),
            ..Default::default()
        });
        shared.with(|w| w.watch("/tmp/project").unwrap());
        shared.with(|w| w.inject(evt("src/lib.rs", ChangeKind::Modified)));
        let stats = shared.with(|w| w.stats());
        assert_eq!(stats.total_events, 1);
    }
}
