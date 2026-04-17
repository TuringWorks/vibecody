//! Prompt version control — save, branch, diff, and restore prompt versions.
//! FIT-GAP v11 Phase 48 — closes gap vs Cody 6.0.

use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// A stored version of a prompt.
#[derive(Debug, Clone)]
pub struct PromptVersion {
    pub id: String,
    pub parent_id: Option<String>,
    pub branch: String,
    pub message: String,
    pub content: String,
    pub timestamp_ms: u64,
    pub tags: Vec<String>,
}

impl PromptVersion {
    pub fn is_tagged(&self, tag: &str) -> bool {
        self.tags.iter().any(|t| t == tag)
    }
}

/// Summary diff between two versions.
#[derive(Debug, Clone)]
pub struct PromptDiff {
    pub from_id: String,
    pub to_id: String,
    pub added_lines: usize,
    pub removed_lines: usize,
    pub unchanged_lines: usize,
    pub hunks: Vec<DiffHunk>,
}

#[derive(Debug, Clone)]
pub struct DiffHunk {
    pub kind: HunkKind,
    pub content: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HunkKind { Added, Removed, Context }

// ---------------------------------------------------------------------------
// Versioned Prompt Store
// ---------------------------------------------------------------------------

/// Stores prompt versions in a branched history.
#[derive(Debug, Default)]
pub struct PromptVcs {
    versions: HashMap<String, PromptVersion>,
    /// branch name → HEAD commit id
    branches: HashMap<String, String>,
    current_branch: String,
    next_id: u64,
}

impl PromptVcs {
    pub fn new() -> Self {
        let mut vcs = Self {
            versions: HashMap::new(),
            branches: HashMap::new(),
            current_branch: "main".to_string(),
            next_id: 1,
        };
        vcs.branches.insert("main".to_string(), String::new());
        vcs
    }

    fn gen_id(&mut self) -> String {
        let id = format!("v{:04}", self.next_id);
        self.next_id += 1;
        id
    }

    /// Commit a new version on the current branch.
    pub fn commit(&mut self, content: impl Into<String>, message: impl Into<String>, ts: u64) -> String {
        let id = self.gen_id();
        let parent_id = self.branches.get(&self.current_branch).and_then(|s| {
            if s.is_empty() { None } else { Some(s.clone()) }
        });
        let version = PromptVersion {
            id: id.clone(),
            parent_id,
            branch: self.current_branch.clone(),
            message: message.into(),
            content: content.into(),
            timestamp_ms: ts,
            tags: Vec::new(),
        };
        self.versions.insert(id.clone(), version);
        self.branches.insert(self.current_branch.clone(), id.clone());
        id
    }

    /// Get a version by id.
    pub fn get(&self, id: &str) -> Option<&PromptVersion> {
        self.versions.get(id)
    }

    /// Get HEAD of a branch.
    pub fn head(&self, branch: &str) -> Option<&PromptVersion> {
        let id = self.branches.get(branch)?;
        if id.is_empty() { return None; }
        self.versions.get(id)
    }

    /// Create a new branch from the current HEAD.
    pub fn create_branch(&mut self, name: impl Into<String>) -> bool {
        let name = name.into();
        if self.branches.contains_key(&name) { return false; }
        let head_id = self.branches.get(&self.current_branch).cloned().unwrap_or_default();
        self.branches.insert(name, head_id);
        true
    }

    /// Switch to an existing branch.
    pub fn checkout(&mut self, branch: &str) -> bool {
        if !self.branches.contains_key(branch) { return false; }
        self.current_branch = branch.to_string();
        true
    }

    /// Tag a version.
    pub fn tag(&mut self, id: &str, tag: impl Into<String>) -> bool {
        if let Some(v) = self.versions.get_mut(id) {
            v.tags.push(tag.into());
            true
        } else {
            false
        }
    }

    /// Find versions by tag.
    pub fn versions_with_tag(&self, tag: &str) -> Vec<&PromptVersion> {
        self.versions.values().filter(|v| v.is_tagged(tag)).collect()
    }

    /// Compute a line-level diff between two versions.
    pub fn diff(&self, from_id: &str, to_id: &str) -> Option<PromptDiff> {
        let from = self.versions.get(from_id)?;
        let to = self.versions.get(to_id)?;
        let from_lines: Vec<&str> = from.content.lines().collect();
        let to_lines: Vec<&str> = to.content.lines().collect();

        let mut hunks = Vec::new();
        let mut added = 0;
        let mut removed = 0;
        let mut unchanged = 0;

        // Simple Myers-like diff: LCS-based line comparison.
        let lcs = compute_lcs(&from_lines, &to_lines);
        let mut fi = 0;
        let mut ti = 0;
        let mut li = 0;

        while fi < from_lines.len() || ti < to_lines.len() {
            if li < lcs.len() {
                let (lf, lt) = lcs[li];
                while fi < lf {
                    hunks.push(DiffHunk { kind: HunkKind::Removed, content: from_lines[fi].to_string() });
                    removed += 1;
                    fi += 1;
                }
                while ti < lt {
                    hunks.push(DiffHunk { kind: HunkKind::Added, content: to_lines[ti].to_string() });
                    added += 1;
                    ti += 1;
                }
                hunks.push(DiffHunk { kind: HunkKind::Context, content: from_lines[fi].to_string() });
                unchanged += 1;
                fi += 1;
                ti += 1;
                li += 1;
            } else {
                if fi < from_lines.len() {
                    hunks.push(DiffHunk { kind: HunkKind::Removed, content: from_lines[fi].to_string() });
                    removed += 1;
                    fi += 1;
                }
                if ti < to_lines.len() {
                    hunks.push(DiffHunk { kind: HunkKind::Added, content: to_lines[ti].to_string() });
                    added += 1;
                    ti += 1;
                }
            }
        }

        Some(PromptDiff {
            from_id: from_id.to_string(),
            to_id: to_id.to_string(),
            added_lines: added,
            removed_lines: removed,
            unchanged_lines: unchanged,
            hunks,
        })
    }

    /// Walk history from a version back to the root.
    pub fn history(&self, from_id: &str) -> Vec<&PromptVersion> {
        let mut result = Vec::new();
        let mut current = from_id;
        while let Some(v) = self.versions.get(current) {
            result.push(v);
            match &v.parent_id {
                Some(p) => current = p.as_str(),
                None => break,
            }
        }
        result
    }

    pub fn version_count(&self) -> usize { self.versions.len() }
    pub fn branch_count(&self) -> usize { self.branches.len() }
    pub fn current_branch(&self) -> &str { &self.current_branch }
}

/// Compute LCS of two string slices, returning pairs of matching indices.
fn compute_lcs<'a>(a: &[&'a str], b: &[&'a str]) -> Vec<(usize, usize)> {
    let m = a.len();
    let n = b.len();
    let mut dp = vec![vec![0usize; n + 1]; m + 1];
    for i in 1..=m {
        for j in 1..=n {
            if a[i - 1] == b[j - 1] {
                dp[i][j] = dp[i - 1][j - 1] + 1;
            } else {
                dp[i][j] = dp[i - 1][j].max(dp[i][j - 1]);
            }
        }
    }
    // Backtrack
    let mut result = Vec::new();
    let (mut i, mut j) = (m, n);
    while i > 0 && j > 0 {
        if a[i - 1] == b[j - 1] {
            result.push((i - 1, j - 1));
            i -= 1;
            j -= 1;
        } else if dp[i - 1][j] > dp[i][j - 1] {
            i -= 1;
        } else {
            j -= 1;
        }
    }
    result.reverse();
    result
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_commit_and_get() {
        let mut vcs = PromptVcs::new();
        let id = vcs.commit("Hello world", "initial", 1000);
        let v = vcs.get(&id).unwrap();
        assert_eq!(v.content, "Hello world");
        assert_eq!(v.message, "initial");
    }

    #[test]
    fn test_head() {
        let mut vcs = PromptVcs::new();
        vcs.commit("v1", "first", 1);
        let id2 = vcs.commit("v2", "second", 2);
        let head = vcs.head("main").unwrap();
        assert_eq!(head.id, id2);
    }

    #[test]
    fn test_parent_chain() {
        let mut vcs = PromptVcs::new();
        let id1 = vcs.commit("v1", "first", 1);
        let id2 = vcs.commit("v2", "second", 2);
        let v2 = vcs.get(&id2).unwrap();
        assert_eq!(v2.parent_id, Some(id1));
    }

    #[test]
    fn test_create_branch() {
        let mut vcs = PromptVcs::new();
        vcs.commit("base", "initial", 1);
        assert!(vcs.create_branch("feature"));
        assert_eq!(vcs.branch_count(), 2);
    }

    #[test]
    fn test_checkout_and_commit_on_branch() {
        let mut vcs = PromptVcs::new();
        vcs.commit("base", "initial", 1);
        vcs.create_branch("feature");
        vcs.checkout("feature");
        assert_eq!(vcs.current_branch(), "feature");
        let id = vcs.commit("feature content", "feat: new prompt", 2);
        let v = vcs.get(&id).unwrap();
        assert_eq!(v.branch, "feature");
    }

    #[test]
    fn test_checkout_nonexistent_branch() {
        let mut vcs = PromptVcs::new();
        assert!(!vcs.checkout("nonexistent"));
    }

    #[test]
    fn test_tag_and_find() {
        let mut vcs = PromptVcs::new();
        let id = vcs.commit("content", "msg", 1);
        vcs.tag(&id, "golden");
        let tagged = vcs.versions_with_tag("golden");
        assert_eq!(tagged.len(), 1);
    }

    #[test]
    fn test_diff_added_removed() {
        let mut vcs = PromptVcs::new();
        let id1 = vcs.commit("line one\nline two\nline three", "v1", 1);
        let id2 = vcs.commit("line one\nline TWO\nline three\nline four", "v2", 2);
        let diff = vcs.diff(&id1, &id2).unwrap();
        assert!(diff.added_lines > 0);
        assert!(diff.removed_lines > 0);
    }

    #[test]
    fn test_diff_identical() {
        let mut vcs = PromptVcs::new();
        let id1 = vcs.commit("same content", "v1", 1);
        let id2 = vcs.commit("same content", "v2", 2);
        let diff = vcs.diff(&id1, &id2).unwrap();
        assert_eq!(diff.added_lines, 0);
        assert_eq!(diff.removed_lines, 0);
    }

    #[test]
    fn test_history_walk() {
        let mut vcs = PromptVcs::new();
        vcs.commit("v1", "first", 1);
        vcs.commit("v2", "second", 2);
        let id3 = vcs.commit("v3", "third", 3);
        let hist = vcs.history(&id3);
        assert_eq!(hist.len(), 3);
    }

    #[test]
    fn test_version_count() {
        let mut vcs = PromptVcs::new();
        vcs.commit("a", "m1", 1);
        vcs.commit("b", "m2", 2);
        assert_eq!(vcs.version_count(), 2);
    }

    #[test]
    fn test_create_branch_duplicate_fails() {
        let mut vcs = PromptVcs::new();
        assert!(!vcs.create_branch("main")); // already exists
    }
}
