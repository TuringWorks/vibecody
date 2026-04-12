//! Workspace fingerprinting for deterministic session identity.
//!
//! Claw-code parity Wave 1: computes a stable hash of the workspace state
//! (tracked files, HEAD commit, branch name) so sessions can detect workspace
//! drift and resume from the correct context.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ─── Fingerprint Components ───────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceFingerprint {
    /// Git HEAD commit hash (40 hex chars), or "no-git" if not a repo.
    pub head_commit: String,
    /// Current branch name.
    pub branch: String,
    /// Sorted map of relative path → content hash for all tracked files.
    pub file_hashes: BTreeMap<String, u64>,
    /// Combined fingerprint hash of all components.
    pub hash: u64,
}

impl WorkspaceFingerprint {
    /// Build a fingerprint from explicit components.
    pub fn new(head_commit: impl Into<String>, branch: impl Into<String>, files: BTreeMap<String, u64>) -> Self {
        let mut fp = Self {
            head_commit: head_commit.into(),
            branch: branch.into(),
            file_hashes: files,
            hash: 0,
        };
        fp.hash = fp.compute_hash();
        fp
    }

    fn compute_hash(&self) -> u64 {
        let mut h = fnv1a(self.head_commit.as_bytes());
        h ^= fnv1a(self.branch.as_bytes());
        for (path, fhash) in &self.file_hashes {
            h ^= fnv1a(path.as_bytes());
            h = h.wrapping_add(*fhash);
        }
        h
    }

    /// True if two fingerprints represent the same workspace state.
    pub fn matches(&self, other: &Self) -> bool { self.hash == other.hash }

    /// Compute a diff: which files changed/added/removed.
    pub fn diff(&self, newer: &Self) -> FingerprintDiff {
        let mut added   = Vec::new();
        let mut removed = Vec::new();
        let mut changed = Vec::new();

        for (path, &new_hash) in &newer.file_hashes {
            match self.file_hashes.get(path) {
                None => added.push(path.clone()),
                Some(&old_hash) if old_hash != new_hash => changed.push(path.clone()),
                _ => {}
            }
        }
        for path in self.file_hashes.keys() {
            if !newer.file_hashes.contains_key(path) { removed.push(path.clone()); }
        }
        let branch_changed = self.branch != newer.branch;
        let commit_changed = self.head_commit != newer.head_commit;

        FingerprintDiff { added, removed, changed, branch_changed, commit_changed }
    }

    /// Whether the fingerprint represents a clean state (no tracked files changed from HEAD).
    pub fn is_clean(&self) -> bool { self.file_hashes.is_empty() }

    /// Number of tracked files.
    pub fn file_count(&self) -> usize { self.file_hashes.len() }
}

fn fnv1a(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for &b in bytes { h ^= b as u64; h = h.wrapping_mul(0x100000001b3); }
    h
}

/// Content hash of a file for use in fingerprints.
pub fn hash_content(content: &str) -> u64 { fnv1a(content.as_bytes()) }

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FingerprintDiff {
    pub added: Vec<String>,
    pub removed: Vec<String>,
    pub changed: Vec<String>,
    pub branch_changed: bool,
    pub commit_changed: bool,
}

impl FingerprintDiff {
    pub fn is_empty(&self) -> bool {
        self.added.is_empty() && self.removed.is_empty() && self.changed.is_empty()
        && !self.branch_changed && !self.commit_changed
    }

    pub fn total_changes(&self) -> usize { self.added.len() + self.removed.len() + self.changed.len() }
}

// ─── Fingerprint Store ────────────────────────────────────────────────────────

/// In-memory store of saved fingerprints keyed by session ID.
#[derive(Debug, Default)]
pub struct FingerprintStore {
    entries: BTreeMap<String, WorkspaceFingerprint>,
}

impl FingerprintStore {
    pub fn new() -> Self { Self::default() }

    pub fn save(&mut self, session_id: impl Into<String>, fp: WorkspaceFingerprint) {
        self.entries.insert(session_id.into(), fp);
    }

    pub fn load(&self, session_id: &str) -> Option<&WorkspaceFingerprint> {
        self.entries.get(session_id)
    }

    pub fn remove(&mut self, session_id: &str) -> bool { self.entries.remove(session_id).is_some() }

    /// Find sessions whose workspace matches the given fingerprint.
    pub fn find_matching(&self, fp: &WorkspaceFingerprint) -> Vec<&str> {
        self.entries.iter().filter(|(_, v)| v.matches(fp)).map(|(k, _)| k.as_str()).collect()
    }
}

// ─── Path-Based Namespace Isolation ──────────────────────────────────────────
//
// These functions implement FNV-1a hashing of workspace *paths* (not content)
// for per-workspace session namespace isolation, preventing cross-workspace
// session collisions when the same agent runs on multiple workspaces
// simultaneously.

/// Raw FNV-1a hash of a string.
///
/// Starts with the FNV offset basis and, for each byte, XORs then multiplies
/// by the FNV prime.
pub fn fnv1a_hash(data: &str) -> u64 {
    const OFFSET_BASIS: u64 = 0xcbf29ce484222325;
    const PRIME: u64 = 0x00000100000001B3;
    let mut h = OFFSET_BASIS;
    for b in data.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(PRIME);
    }
    h
}

/// Produce a 16-character lowercase hex fingerprint for a workspace path.
///
/// Trailing slashes are stripped before hashing so that `/foo/bar` and
/// `/foo/bar/` produce the same fingerprint.
pub fn workspace_fingerprint(workspace_path: &str) -> String {
    let normalized = workspace_path.trim_end_matches('/');
    format!("{:016x}", fnv1a_hash(normalized))
}

/// Build a `"ws-{fingerprint}"` namespace prefix for session IDs.
pub fn session_namespace(workspace_path: &str) -> String {
    format!("ws-{}", workspace_fingerprint(workspace_path))
}

/// Return `true` when two paths refer to the same workspace after stripping
/// trailing slashes.
pub fn is_same_workspace(path_a: &str, path_b: &str) -> bool {
    workspace_fingerprint(path_a) == workspace_fingerprint(path_b)
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn fp(commit: &str, branch: &str, files: &[(&str, &str)]) -> WorkspaceFingerprint {
        let map = files.iter().map(|(p, c)| (p.to_string(), hash_content(c))).collect();
        WorkspaceFingerprint::new(commit, branch, map)
    }

    #[test]
    fn test_same_state_matches() {
        let a = fp("abc123", "main", &[("src/lib.rs", "fn foo() {}")]);
        let b = fp("abc123", "main", &[("src/lib.rs", "fn foo() {}")]);
        assert!(a.matches(&b));
    }

    #[test]
    fn test_different_commit_no_match() {
        let a = fp("abc123", "main", &[]);
        let b = fp("def456", "main", &[]);
        assert!(!a.matches(&b));
    }

    #[test]
    fn test_different_branch_no_match() {
        let a = fp("abc", "main", &[]);
        let b = fp("abc", "feature", &[]);
        assert!(!a.matches(&b));
    }

    #[test]
    fn test_different_content_no_match() {
        let a = fp("abc", "main", &[("f.rs", "hello")]);
        let b = fp("abc", "main", &[("f.rs", "world")]);
        assert!(!a.matches(&b));
    }

    #[test]
    fn test_diff_file_added() {
        let a = fp("abc", "main", &[]);
        let b = fp("abc", "main", &[("new.rs", "content")]);
        let diff = a.diff(&b);
        assert_eq!(diff.added, vec!["new.rs"]);
        assert!(diff.removed.is_empty());
    }

    #[test]
    fn test_diff_file_removed() {
        let a = fp("abc", "main", &[("old.rs", "x")]);
        let b = fp("abc", "main", &[]);
        let diff = a.diff(&b);
        assert_eq!(diff.removed, vec!["old.rs"]);
    }

    #[test]
    fn test_diff_file_changed() {
        let a = fp("abc", "main", &[("x.rs", "v1")]);
        let b = fp("abc", "main", &[("x.rs", "v2")]);
        let diff = a.diff(&b);
        assert_eq!(diff.changed, vec!["x.rs"]);
    }

    #[test]
    fn test_diff_branch_changed() {
        let a = fp("abc", "main", &[]);
        let b = fp("abc", "feature", &[]);
        let diff = a.diff(&b);
        assert!(diff.branch_changed);
    }

    #[test]
    fn test_diff_empty_when_same() {
        let a = fp("abc", "main", &[("f.rs", "x")]);
        let b = fp("abc", "main", &[("f.rs", "x")]);
        let diff = a.diff(&b);
        assert!(diff.is_empty());
    }

    #[test]
    fn test_diff_total_changes() {
        let a = fp("abc", "main", &[("old.rs", "x")]);
        let b = fp("abc", "main", &[("new.rs", "y"), ("other.rs", "z")]);
        let diff = a.diff(&b);
        assert_eq!(diff.total_changes(), 3); // 2 added + 1 removed
    }

    #[test]
    fn test_is_clean_empty_files() {
        let fp = WorkspaceFingerprint::new("abc", "main", BTreeMap::new());
        assert!(fp.is_clean());
    }

    #[test]
    fn test_file_count() {
        let f = fp("abc", "main", &[("a.rs", "1"), ("b.rs", "2")]);
        assert_eq!(f.file_count(), 2);
    }

    #[test]
    fn test_hash_content_stable() {
        assert_eq!(hash_content("hello"), hash_content("hello"));
        assert_ne!(hash_content("hello"), hash_content("world"));
    }

    #[test]
    fn test_store_save_load() {
        let mut store = FingerprintStore::new();
        let f = fp("abc", "main", &[]);
        store.save("sess-1", f.clone());
        assert!(store.load("sess-1").unwrap().matches(&f));
    }

    #[test]
    fn test_store_remove() {
        let mut store = FingerprintStore::new();
        store.save("sess-1", fp("abc", "main", &[]));
        assert!(store.remove("sess-1"));
        assert!(store.load("sess-1").is_none());
    }

    #[test]
    fn test_store_find_matching() {
        let mut store = FingerprintStore::new();
        let f = fp("abc", "main", &[("x.rs", "y")]);
        store.save("s1", f.clone());
        store.save("s2", fp("xyz", "dev", &[]));
        let matches = store.find_matching(&f);
        assert_eq!(matches, vec!["s1"]);
    }

    #[test]
    fn test_fingerprint_hash_deterministic() {
        let f1 = fp("abc", "main", &[("a.rs", "code")]);
        let f2 = fp("abc", "main", &[("a.rs", "code")]);
        assert_eq!(f1.hash, f2.hash);
    }

    // ── Path-namespace isolation tests ────────────────────────────────────────

    #[test]
    fn fnv1a_empty_string_returns_offset_basis() {
        assert_eq!(fnv1a_hash(""), 0xcbf29ce484222325u64);
    }

    #[test]
    fn fnv1a_deterministic_for_same_input() {
        assert_eq!(fnv1a_hash("/home/user/project"), fnv1a_hash("/home/user/project"));
    }

    #[test]
    fn fnv1a_differs_for_different_input() {
        assert_ne!(fnv1a_hash("/home/user/alpha"), fnv1a_hash("/home/user/beta"));
    }

    #[test]
    fn fingerprint_produces_16_char_hex() {
        let fp = workspace_fingerprint("/home/user/project");
        assert_eq!(fp.len(), 16);
        assert!(fp.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn fingerprint_deterministic() {
        assert_eq!(
            workspace_fingerprint("/home/user/project"),
            workspace_fingerprint("/home/user/project")
        );
    }

    #[test]
    fn fingerprint_differs_for_different_paths() {
        assert_ne!(
            workspace_fingerprint("/home/user/alpha"),
            workspace_fingerprint("/home/user/beta")
        );
    }

    #[test]
    fn session_namespace_format() {
        let ns = session_namespace("/home/user/project");
        assert!(ns.starts_with("ws-"));
        assert_eq!(ns.len(), 3 + 16); // "ws-" + 16 hex chars
    }

    #[test]
    fn same_workspace_normalizes_trailing_slash() {
        assert!(is_same_workspace("/home/user/project", "/home/user/project/"));
        assert!(is_same_workspace("/home/user/project/", "/home/user/project"));
    }
}
