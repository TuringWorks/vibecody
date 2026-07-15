//! B3 daemon glue — the always-on security-review loop.
//!
//! Ties the tested pure controller in [`crate::security_review_watch`] to the
//! live daemon:
//!
//! * a real `notify` filesystem watcher feeds OS change events into a debouncing
//!   [`SharedFileWatcher`] (the "real inotify/FSEvents integration wired at
//!   runtime" the file-watcher module leaves as a seam);
//! * an interval task polls that watcher and runs
//!   [`crate::security_review_watch::poll_and_review`] with a provider-backed
//!   [`SecurityReviewer`];
//! * findings land in a bounded, cloneable [`SecurityFindingsQueue`] the daemon's
//!   `/v1/security-review/findings` route snapshots for the ReviewPanel.
//!
//! The §18.B3 posture is preserved: the loop only *starts* when the caller passes
//! an **enabled** [`SecurityReviewConfig`] (opt-in / default-OFF), and it only
//! ever *produces* [`Finding`]s — it never mutates files or auto-applies fixes.
//! The reviewer is provider-agnostic (any [`AIProvider`], never hard-coded).

use crate::file_watcher::{ChangeKind, FileChangeEvent, SharedFileWatcher, WatcherConfig};
use crate::security_review_watch::{poll_and_review, SecurityReviewConfig, SecurityReviewer};
use crate::self_review::Finding;
use notify::{EventKind, RecursiveMode, Watcher};
use std::collections::VecDeque;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use vibe_ai::provider::{AIProvider, Message, MessageRole};

/// Max findings retained in the in-memory sink (newest-wins past the cap so a
/// long-running daemon can't grow unbounded).
const FINDINGS_CAP: usize = 500;

/// Bounded, cloneable in-memory sink for the findings the watcher loop produces.
/// The `/v1/security-review/findings` route snapshots it; the loop pushes into
/// it. Poisoned-lock-safe (recovers the guard rather than panicking) per the
/// no-`unwrap`-in-daemon-paths rule.
#[derive(Clone, Default)]
pub struct SecurityFindingsQueue {
    inner: Arc<Mutex<VecDeque<Finding>>>,
}

impl SecurityFindingsQueue {
    pub fn new() -> Self {
        Self::default()
    }

    /// Append findings, evicting the oldest past [`FINDINGS_CAP`].
    pub fn push_all(&self, findings: Vec<Finding>) {
        if findings.is_empty() {
            return;
        }
        let mut q = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        for f in findings {
            q.push_back(f);
        }
        while q.len() > FINDINGS_CAP {
            q.pop_front();
        }
    }

    /// Current findings, oldest-first.
    pub fn snapshot(&self) -> Vec<Finding> {
        self.inner
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .iter()
            .cloned()
            .collect()
    }

    /// Number of retained findings.
    pub fn len(&self) -> usize {
        self.inner.lock().unwrap_or_else(|e| e.into_inner()).len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Provider-backed [`SecurityReviewer`]: reviews a file by sending the prompt as
/// a single user turn to any [`AIProvider`]. Provider-agnostic by construction —
/// the caller injects whichever provider the config/selection resolved.
pub struct ProviderReviewer {
    provider: Arc<dyn AIProvider>,
}

impl ProviderReviewer {
    pub fn new(provider: Arc<dyn AIProvider>) -> Self {
        Self { provider }
    }
}

#[async_trait::async_trait]
impl SecurityReviewer for ProviderReviewer {
    async fn review(&self, prompt: String) -> Result<String, String> {
        let messages = vec![Message {
            role: MessageRole::User,
            content: prompt,
        }];
        self.provider
            .chat(&messages, None)
            .await
            .map_err(|e| e.to_string())
    }
}

/// Owning handle for a running security-review watcher. **Drop it to stop
/// watching** — the loop task is aborted and the notify watcher it owns closes.
/// The daemon keeps it alive for the process lifetime (`std::mem::forget`).
pub struct SecurityWatchHandle {
    task: tokio::task::JoinHandle<()>,
}

impl Drop for SecurityWatchHandle {
    fn drop(&mut self) {
        self.task.abort();
    }
}

/// Map a notify event kind to our [`ChangeKind`]. Access/other events are
/// dropped (`None`) so read-only stats don't trigger reviews.
fn map_kind(kind: &EventKind) -> Option<ChangeKind> {
    match kind {
        EventKind::Create(_) => Some(ChangeKind::Created),
        EventKind::Modify(_) => Some(ChangeKind::Modified),
        EventKind::Remove(_) => Some(ChangeKind::Deleted),
        _ => None,
    }
}

/// Start the always-on security-review loop over `workspace_root`.
///
/// Returns `None` (a no-op) when `cfg.enabled` is false — the opt-in gate — or
/// when the OS watcher can't be created. On success the returned handle owns the
/// notify watcher + the loop task; the daemon `std::mem::forget`s it so both run
/// for the process lifetime, while tests drop it to stop.
///
/// Generic over the reviewer so tests can inject a mock; the daemon passes a
/// [`ProviderReviewer`].
pub fn spawn<R>(
    workspace_root: &Path,
    cfg: SecurityReviewConfig,
    reviewer: R,
    findings: SecurityFindingsQueue,
    interval: Duration,
) -> Option<SecurityWatchHandle>
where
    R: SecurityReviewer + Send + Sync + 'static,
{
    if !cfg.enabled {
        return None;
    }

    // The debouncing filter layer; the notify callback injects into it and the
    // loop polls it. Its default ignore_patterns already drop target/.git/etc.
    let shared = SharedFileWatcher::new(WatcherConfig::default());
    shared.with(|w| {
        let _ = w.watch(workspace_root.to_path_buf());
    });

    // notify → inject bridge: raw OS events become FileChangeEvents.
    let inject_target = shared.clone_handle();
    let mut watcher = match notify::recommended_watcher(
        move |res: notify::Result<notify::Event>| {
            if let Ok(event) = res {
                if let Some(kind) = map_kind(&event.kind) {
                    for path in event.paths {
                        inject_target
                            .with(|w| w.inject(FileChangeEvent::new(path.clone(), kind.clone())));
                    }
                }
            }
        },
    ) {
        Ok(w) => w,
        Err(_) => return None,
    };
    if watcher
        .watch(workspace_root, RecursiveMode::Recursive)
        .is_err()
    {
        return None;
    }

    let poll_target = shared.clone_handle();
    let task = tokio::spawn(async move {
        // Keep the notify watcher alive for the task's lifetime; dropping it
        // would stop the OS subscription.
        let _watcher = watcher;
        let mut ticker = tokio::time::interval(interval);
        ticker.tick().await; // consume the immediate first tick
        loop {
            ticker.tick().await;
            let found = poll_and_review(&cfg, &poll_target, &reviewer).await;
            findings.push_all(found);
        }
    });

    Some(SecurityWatchHandle { task })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::self_review::Severity;

    /// Mock reviewer returning a canned finding line for every file.
    struct MockReviewer(String);

    #[async_trait::async_trait]
    impl SecurityReviewer for MockReviewer {
        async fn review(&self, _prompt: String) -> Result<String, String> {
            Ok(self.0.clone())
        }
    }

    fn finding(msg: &str) -> Finding {
        Finding::new(crate::self_review::CheckKind::Security, Severity::Error, msg)
    }

    #[test]
    fn findings_queue_push_snapshot_and_cap() {
        let q = SecurityFindingsQueue::new();
        assert!(q.is_empty());
        q.push_all(vec![finding("a"), finding("b")]);
        assert_eq!(q.len(), 2);
        assert_eq!(q.snapshot().len(), 2);
        // Overflow the cap → oldest evicted, newest retained.
        let flood: Vec<Finding> = (0..FINDINGS_CAP + 10).map(|i| finding(&format!("f{i}"))).collect();
        q.push_all(flood);
        assert_eq!(q.len(), FINDINGS_CAP);
        // The very last pushed finding is still present.
        let snap = q.snapshot();
        assert_eq!(snap.last().unwrap().message, format!("f{}", FINDINGS_CAP + 9));
    }

    #[test]
    fn push_all_empty_is_noop() {
        let q = SecurityFindingsQueue::new();
        q.push_all(vec![]);
        assert!(q.is_empty());
    }

    #[test]
    fn map_kind_covers_create_modify_remove() {
        use notify::event::{CreateKind, ModifyKind, RemoveKind};
        assert_eq!(
            map_kind(&EventKind::Create(CreateKind::File)),
            Some(ChangeKind::Created)
        );
        assert_eq!(
            map_kind(&EventKind::Modify(ModifyKind::Any)),
            Some(ChangeKind::Modified)
        );
        assert_eq!(
            map_kind(&EventKind::Remove(RemoveKind::File)),
            Some(ChangeKind::Deleted)
        );
        assert_eq!(map_kind(&EventKind::Access(notify::event::AccessKind::Any)), None);
    }

    #[test]
    fn spawn_disabled_returns_none() {
        let q = SecurityFindingsQueue::new();
        let handle = spawn(
            Path::new("."),
            SecurityReviewConfig::default(), // disabled
            MockReviewer("critical|1|x|y".into()),
            q,
            Duration::from_millis(50),
        );
        assert!(handle.is_none());
    }

    #[tokio::test]
    async fn spawn_reviews_a_real_file_change_end_to_end() {
        let dir = tempfile::tempdir().unwrap();
        let q = SecurityFindingsQueue::new();
        let cfg = SecurityReviewConfig {
            enabled: true,
            watched_suffixes: vec![".rs".into()],
            min_severity: Severity::Info,
        };
        let reviewer = MockReviewer("critical|7|hardcoded secret|use a vault".into());
        let handle = spawn(
            dir.path(),
            cfg,
            reviewer,
            q.clone(),
            Duration::from_millis(100),
        )
        .expect("watcher should start when enabled");

        // Trigger a real filesystem change the watcher must pick up.
        std::fs::write(dir.path().join("vuln.rs"), "let k = \"secret\";\n").unwrap();

        // Poll for the finding to propagate (notify + debounce + interval).
        let mut got = false;
        for _ in 0..40 {
            if !q.is_empty() {
                got = true;
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        drop(handle); // stop watching
        assert!(got, "expected a finding from the reviewed file change");
        assert_eq!(q.snapshot()[0].message, "hardcoded secret");
    }
}
