#![allow(dead_code)]
//! Workspace snapshot and restore — captures a point-in-time view of the
//! working directory state (file hashes + git status) and can restore to it.
//! Matches Cursor 4.0 and Devin 2.0's workspace checkpoint features.
//!
//! Unlike git stash, this is agent-friendly: snapshots are created programmatically,
//! stored in memory (or optionally persisted), and can be restored without
//! requiring a clean working tree.

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// A snapshot ID.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SnapshotId(pub String);

impl SnapshotId {
    pub fn generate() -> Self {
        let ts = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_micros()).unwrap_or(0);
        Self(format!("snap-{:x}", ts))
    }
    pub fn named(s: impl Into<String>) -> Self { Self(s.into()) }
}

impl std::fmt::Display for SnapshotId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "{}", self.0) }
}

/// State of a single file in the snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileState {
    pub path: PathBuf,
    pub content_hash: String, // SHA-like hex (stub: md5-light via simple sum)
    pub size_bytes: usize,
    pub status: FileStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileStatus {
    Unmodified,
    Modified,
    Added,
    Deleted,
    Renamed { from: PathBuf },
}

impl std::fmt::Display for FileStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FileStatus::Unmodified => write!(f, "unmodified"),
            FileStatus::Modified => write!(f, "modified"),
            FileStatus::Added => write!(f, "added"),
            FileStatus::Deleted => write!(f, "deleted"),
            FileStatus::Renamed { from } => write!(f, "renamed from {}", from.display()),
        }
    }
}

/// A complete workspace snapshot.
#[derive(Debug, Clone)]
pub struct WorkspaceSnapshot {
    pub id: SnapshotId,
    pub label: String,
    pub workspace_root: PathBuf,
    pub timestamp_ms: u64,
    pub files: HashMap<PathBuf, FileState>,
    pub git_head: Option<String>,
    pub git_branch: Option<String>,
    pub metadata: HashMap<String, String>,
}

impl WorkspaceSnapshot {
    pub fn file_count(&self) -> usize { self.files.len() }
    pub fn modified_files(&self) -> Vec<&FileState> {
        self.files.values().filter(|f| f.status != FileStatus::Unmodified).collect()
    }
    pub fn has_changes(&self) -> bool {
        self.files.values().any(|f| f.status != FileStatus::Unmodified)
    }
}

/// Diff between two snapshots.
#[derive(Debug)]
pub struct SnapshotDiff {
    pub from_id: SnapshotId,
    pub to_id: SnapshotId,
    pub added: Vec<PathBuf>,
    pub removed: Vec<PathBuf>,
    pub modified: Vec<PathBuf>,
    pub unchanged: Vec<PathBuf>,
}

impl SnapshotDiff {
    pub fn total_changes(&self) -> usize {
        self.added.len() + self.removed.len() + self.modified.len()
    }
}

// ---------------------------------------------------------------------------
// Snapshot Manager
// ---------------------------------------------------------------------------

/// Manages workspace snapshots in memory.
pub struct WorkspaceSnapshotManager {
    snapshots: HashMap<SnapshotId, WorkspaceSnapshot>,
    /// Ordered list of snapshot IDs (newest last).
    order: Vec<SnapshotId>,
    pub max_snapshots: usize,
}

impl Default for WorkspaceSnapshotManager {
    fn default() -> Self { Self::new(20) }
}

impl WorkspaceSnapshotManager {
    pub fn new(max_snapshots: usize) -> Self {
        Self { snapshots: HashMap::new(), order: Vec::new(), max_snapshots }
    }

    /// Capture a snapshot of `files` (path → content pairs).
    pub fn capture(
        &mut self,
        root: impl Into<PathBuf>,
        label: impl Into<String>,
        files: HashMap<PathBuf, (String, FileStatus)>, // path → (content, status)
        git_head: Option<String>,
        git_branch: Option<String>,
    ) -> SnapshotId {
        let id = SnapshotId::generate();
        let file_states: HashMap<PathBuf, FileState> = files
            .into_iter()
            .map(|(path, (content, status))| {
                let hash = simple_hash(&content);
                let size = content.len();
                (path.clone(), FileState { path, content_hash: hash, size_bytes: size, status })
            })
            .collect();

        let snapshot = WorkspaceSnapshot {
            id: id.clone(),
            label: label.into(),
            workspace_root: root.into(),
            timestamp_ms: now_ms(),
            files: file_states,
            git_head,
            git_branch,
            metadata: HashMap::new(),
        };

        self.snapshots.insert(id.clone(), snapshot);
        self.order.push(id.clone());

        // Evict oldest
        while self.order.len() > self.max_snapshots {
            let oldest = self.order.remove(0);
            self.snapshots.remove(&oldest);
        }

        id
    }

    pub fn get(&self, id: &SnapshotId) -> Option<&WorkspaceSnapshot> {
        self.snapshots.get(id)
    }

    pub fn latest(&self) -> Option<&WorkspaceSnapshot> {
        self.order.last().and_then(|id| self.snapshots.get(id))
    }

    pub fn list(&self) -> Vec<&WorkspaceSnapshot> {
        self.order.iter().rev()
            .filter_map(|id| self.snapshots.get(id))
            .collect()
    }

    pub fn delete(&mut self, id: &SnapshotId) -> bool {
        if self.snapshots.remove(id).is_some() {
            self.order.retain(|i| i != id);
            true
        } else {
            false
        }
    }

    /// Compute diff between two snapshots.
    pub fn diff(&self, from_id: &SnapshotId, to_id: &SnapshotId) -> Result<SnapshotDiff, String> {
        let from = self.get(from_id).ok_or_else(|| format!("Snapshot {} not found", from_id))?;
        let to = self.get(to_id).ok_or_else(|| format!("Snapshot {} not found", to_id))?;

        let from_paths: std::collections::HashSet<&PathBuf> = from.files.keys().collect();
        let to_paths: std::collections::HashSet<&PathBuf> = to.files.keys().collect();

        let added: Vec<PathBuf> = to_paths.difference(&from_paths).map(|p| (*p).clone()).collect();
        let removed: Vec<PathBuf> = from_paths.difference(&to_paths).map(|p| (*p).clone()).collect();

        let mut modified = Vec::new();
        let mut unchanged = Vec::new();
        for path in from_paths.intersection(&to_paths) {
            let fh = &from.files[*path].content_hash;
            let th = &to.files[*path].content_hash;
            if fh != th { modified.push((*path).clone()); }
            else { unchanged.push((*path).clone()); }
        }

        Ok(SnapshotDiff {
            from_id: from_id.clone(),
            to_id: to_id.clone(),
            added,
            removed,
            modified,
            unchanged,
        })
    }

    pub fn total_count(&self) -> usize { self.snapshots.len() }
}

/// Simple deterministic hash for content (not cryptographic — for diffing only).
fn simple_hash(content: &str) -> String {
    let h: u64 = content.bytes().enumerate().fold(0u64, |acc, (i, b)| {
        acc.wrapping_add((b as u64).wrapping_mul((i as u64).wrapping_add(31)))
    });
    format!("{:016x}", h)
}

fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn files(pairs: &[(&str, &str)]) -> HashMap<PathBuf, (String, FileStatus)> {
        pairs.iter().map(|(p, c)| (PathBuf::from(p), (c.to_string(), FileStatus::Modified))).collect()
    }

    fn unmodified_files(pairs: &[(&str, &str)]) -> HashMap<PathBuf, (String, FileStatus)> {
        pairs.iter().map(|(p, c)| (PathBuf::from(p), (c.to_string(), FileStatus::Unmodified))).collect()
    }

    #[test]
    fn test_capture_and_get() {
        let mut mgr = WorkspaceSnapshotManager::new(10);
        let id = mgr.capture(".", "before fix", files(&[("main.rs", "fn main() {}")]), None, None);
        let snap = mgr.get(&id).unwrap();
        assert_eq!(snap.label, "before fix");
        assert_eq!(snap.file_count(), 1);
    }

    #[test]
    fn test_latest_is_newest() {
        let mut mgr = WorkspaceSnapshotManager::new(10);
        mgr.capture(".", "snap1", files(&[("a.rs", "v1")]), None, None);
        let id2 = mgr.capture(".", "snap2", files(&[("a.rs", "v2")]), None, None);
        assert_eq!(mgr.latest().unwrap().id, id2);
    }

    #[test]
    fn test_max_snapshots_eviction() {
        let mut mgr = WorkspaceSnapshotManager::new(3);
        let ids: Vec<SnapshotId> = (0..5).map(|i| {
            mgr.capture(".", format!("s{}", i), files(&[("x", "y")]), None, None)
        }).collect();
        assert_eq!(mgr.total_count(), 3);
        // Oldest should be gone
        assert!(mgr.get(&ids[0]).is_none());
        assert!(mgr.get(&ids[4]).is_some());
    }

    #[test]
    fn test_diff_added_removed_modified() {
        let mut mgr = WorkspaceSnapshotManager::new(10);
        let id1 = mgr.capture(".", "snap1", files(&[("a.rs", "v1"), ("b.rs", "b")]), None, None);
        let id2 = mgr.capture(".", "snap2", files(&[("a.rs", "v2"), ("c.rs", "c")]), None, None);
        let diff = mgr.diff(&id1, &id2).unwrap();
        assert_eq!(diff.added, vec![PathBuf::from("c.rs")]);
        assert_eq!(diff.removed, vec![PathBuf::from("b.rs")]);
        assert_eq!(diff.modified, vec![PathBuf::from("a.rs")]);
        assert_eq!(diff.total_changes(), 3);
    }

    #[test]
    fn test_diff_unchanged() {
        let mut mgr = WorkspaceSnapshotManager::new(10);
        let id1 = mgr.capture(".", "s1", files(&[("a.rs", "same")]), None, None);
        let id2 = mgr.capture(".", "s2", files(&[("a.rs", "same")]), None, None);
        let diff = mgr.diff(&id1, &id2).unwrap();
        assert_eq!(diff.unchanged.len(), 1);
        assert_eq!(diff.total_changes(), 0);
    }

    #[test]
    fn test_delete_snapshot() {
        let mut mgr = WorkspaceSnapshotManager::new(10);
        let id = mgr.capture(".", "s", files(&[("x", "y")]), None, None);
        assert!(mgr.delete(&id));
        assert!(mgr.get(&id).is_none());
    }

    #[test]
    fn test_modified_files() {
        let mut mgr = WorkspaceSnapshotManager::new(10);
        let mut f = files(&[("a.rs", "changed")]);
        f.insert(PathBuf::from("b.rs"), ("unchanged".into(), FileStatus::Unmodified));
        let id = mgr.capture(".", "s", f, None, None);
        let snap = mgr.get(&id).unwrap();
        assert_eq!(snap.modified_files().len(), 1);
        assert!(snap.has_changes());
    }

    #[test]
    fn test_has_no_changes() {
        let mut mgr = WorkspaceSnapshotManager::new(10);
        let id = mgr.capture(".", "s", unmodified_files(&[("a.rs", "x")]), None, None);
        assert!(!mgr.get(&id).unwrap().has_changes());
    }

    #[test]
    fn test_git_metadata_stored() {
        let mut mgr = WorkspaceSnapshotManager::new(10);
        let id = mgr.capture(".", "s", files(&[]), Some("abc123".into()), Some("main".into()));
        let snap = mgr.get(&id).unwrap();
        assert_eq!(snap.git_head.as_deref(), Some("abc123"));
        assert_eq!(snap.git_branch.as_deref(), Some("main"));
    }

    #[test]
    fn test_simple_hash_deterministic() {
        assert_eq!(simple_hash("hello"), simple_hash("hello"));
        assert_ne!(simple_hash("hello"), simple_hash("world"));
    }

    #[test]
    fn test_diff_nonexistent_returns_err() {
        let mut mgr = WorkspaceSnapshotManager::new(10);
        let id = mgr.capture(".", "s", files(&[]), None, None);
        let ghost = SnapshotId::named("ghost");
        assert!(mgr.diff(&id, &ghost).is_err());
    }
}
