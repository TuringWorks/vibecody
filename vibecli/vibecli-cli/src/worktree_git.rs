//! Real `git worktree` pool (US-003).
//!
//! [`worktree_pool`] provides the in-memory data model (agents, tasks,
//! metrics, branch-name generator) that powers the VibeUI panel. This module
//! provides the actual disk-side implementation: shelling out to the `git`
//! CLI to create worktrees, merge branches, and clean up on completion.
//!
//! The split lets the in-memory pool keep its pure business logic (task
//! splitting, parallelism estimation, branch naming) while delegating every
//! "touches the filesystem or git state" call to this module.
//!
//! Typical lifecycle for one unit of work:
//! 1. [`GitWorktreePool::spawn`] runs `git -C <source> worktree add -b <branch> <path>`
//! 2. Caller drives the agent in the worktree directory (writes files, commits).
//! 3. [`GitWorktreePool::merge_into`] checks out the target branch in the source
//!    repo, runs `git merge --no-ff <branch>`, and aborts cleanly on conflict.
//! 4. [`GitWorktreePool::remove`] runs `git worktree remove --force <path>`
//!    and drops the in-memory handle.
//!
//! Errors returned are human-readable strings containing stderr from git.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

/// A single live worktree tracked by the pool.
#[derive(Debug, Clone)]
pub struct WorktreeHandle {
    pub id: String,
    pub path: PathBuf,
    pub branch: String,
}

/// Result of a merge attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergeOutcome {
    pub merged: bool,
    pub conflicts: Vec<String>,
    pub stdout: String,
    pub stderr: String,
}

/// Real git-backed worktree pool.
#[derive(Debug)]
pub struct GitWorktreePool {
    source_repo: PathBuf,
    base_dir: PathBuf,
    max_worktrees: usize,
    worktrees: HashMap<String, WorktreeHandle>,
}

impl GitWorktreePool {
    pub fn new(source_repo: impl AsRef<Path>, base_dir: impl AsRef<Path>, max_worktrees: usize) -> Self {
        Self {
            source_repo: source_repo.as_ref().to_path_buf(),
            base_dir: base_dir.as_ref().to_path_buf(),
            max_worktrees,
            worktrees: HashMap::new(),
        }
    }

    pub fn source_repo(&self) -> &Path {
        &self.source_repo
    }

    pub fn base_dir(&self) -> &Path {
        &self.base_dir
    }

    pub fn max_worktrees(&self) -> usize {
        self.max_worktrees
    }

    pub fn active_count(&self) -> usize {
        self.worktrees.len()
    }

    pub fn get(&self, id: &str) -> Option<&WorktreeHandle> {
        self.worktrees.get(id)
    }

    pub fn list(&self) -> Vec<&WorktreeHandle> {
        let mut v: Vec<&WorktreeHandle> = self.worktrees.values().collect();
        v.sort_by(|a, b| a.id.cmp(&b.id));
        v
    }

    /// Create a new worktree at `<base_dir>/<id>` on a fresh branch.
    ///
    /// Runs `git -C <source_repo> worktree add -b <branch> <path>` and records
    /// the handle on success. Fails if the pool is at capacity, if the id is
    /// already tracked, or if git returns a non-zero exit status.
    pub fn spawn(&mut self, id: &str, branch: &str) -> Result<WorktreeHandle, String> {
        if self.worktrees.len() >= self.max_worktrees {
            return Err(format!(
                "pool at capacity ({} / {})",
                self.worktrees.len(),
                self.max_worktrees
            ));
        }
        if self.worktrees.contains_key(id) {
            return Err(format!("worktree id '{id}' already tracked"));
        }
        let path = self.base_dir.join(id);
        if path.exists() {
            return Err(format!("target path {path:?} already exists"));
        }

        let path_str = path.to_string_lossy().into_owned();
        let out = Command::new("git")
            .current_dir(&self.source_repo)
            .args(["worktree", "add", "-b", branch, &path_str])
            .output()
            .map_err(|e| format!("spawn git: {e}"))?;
        if !out.status.success() {
            return Err(format!(
                "git worktree add failed: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            ));
        }

        let handle = WorktreeHandle {
            id: id.to_string(),
            path,
            branch: branch.to_string(),
        };
        self.worktrees.insert(id.to_string(), handle.clone());
        Ok(handle)
    }

    /// Remove a tracked worktree from disk and delete its branch.
    ///
    /// Runs `git -C <source_repo> worktree remove --force <path>` then
    /// `git -C <source_repo> branch -D <branch>` so the checkout and the branch
    /// are both cleaned up.
    pub fn remove(&mut self, id: &str) -> Result<(), String> {
        let handle = self
            .worktrees
            .get(id)
            .ok_or_else(|| format!("worktree '{id}' not tracked"))?
            .clone();

        let path_str = handle.path.to_string_lossy().into_owned();
        let out = Command::new("git")
            .current_dir(&self.source_repo)
            .args(["worktree", "remove", "--force", &path_str])
            .output()
            .map_err(|e| format!("spawn git: {e}"))?;
        if !out.status.success() {
            return Err(format!(
                "git worktree remove failed: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            ));
        }

        // Best-effort branch delete — if the branch has already been merged
        // somewhere else, -D still succeeds; if someone else is on it we log
        // but keep the handle drop.
        let _ = Command::new("git")
            .current_dir(&self.source_repo)
            .args(["branch", "-D", &handle.branch])
            .output();

        self.worktrees.remove(id);
        Ok(())
    }

    /// Merge a worktree's branch into `target_branch` on the source repo.
    ///
    /// On conflict, runs `git merge --abort` so the source repo is left on a
    /// clean HEAD and the list of conflicted files is returned in
    /// [`MergeOutcome::conflicts`].
    pub fn merge_into(&self, id: &str, target_branch: &str) -> Result<MergeOutcome, String> {
        let handle = self
            .worktrees
            .get(id)
            .ok_or_else(|| format!("worktree '{id}' not tracked"))?;

        // Checkout the target branch in the source repo.
        let checkout = Command::new("git")
            .current_dir(&self.source_repo)
            .args(["checkout", target_branch])
            .output()
            .map_err(|e| format!("spawn git: {e}"))?;
        if !checkout.status.success() {
            return Err(format!(
                "git checkout {target_branch} failed: {}",
                String::from_utf8_lossy(&checkout.stderr).trim()
            ));
        }

        // Attempt the merge. git returns non-zero on conflict; we still want
        // to read the status to collect file names.
        let merge = Command::new("git")
            .current_dir(&self.source_repo)
            .args(["merge", "--no-ff", "--no-edit", &handle.branch])
            .output()
            .map_err(|e| format!("spawn git: {e}"))?;
        let stdout = String::from_utf8_lossy(&merge.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&merge.stderr).into_owned();

        if merge.status.success() {
            return Ok(MergeOutcome {
                merged: true,
                conflicts: Vec::new(),
                stdout,
                stderr,
            });
        }

        // Conflict path: enumerate the files, abort the merge.
        let conflicts = collect_conflicts(&self.source_repo);
        let _ = Command::new("git")
            .current_dir(&self.source_repo)
            .args(["merge", "--abort"])
            .output();

        Ok(MergeOutcome {
            merged: false,
            conflicts,
            stdout,
            stderr,
        })
    }
}

/// Parse `git status --porcelain` output for files in a conflicted state
/// (codes like `UU`, `AA`, `DU`, `UD`, `AU`, `UA`).
fn collect_conflicts(repo: &Path) -> Vec<String> {
    let out = match Command::new("git")
        .current_dir(repo)
        .args(["status", "--porcelain"])
        .output()
    {
        Ok(o) => o,
        Err(_) => return Vec::new(),
    };
    let text = String::from_utf8_lossy(&out.stdout);
    let mut conflicts = Vec::new();
    for line in text.lines() {
        if line.len() < 3 {
            continue;
        }
        let code = &line[..2];
        let is_conflict = matches!(code, "UU" | "AA" | "DD" | "DU" | "UD" | "AU" | "UA");
        if is_conflict {
            let path = line[3..].trim().to_string();
            conflicts.push(path);
        }
    }
    conflicts
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn run(cwd: &Path, args: &[&str]) {
        let out = Command::new("git")
            .current_dir(cwd)
            .args(args)
            .output()
            .expect("git");
        assert!(
            out.status.success(),
            "git {args:?} failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }

    fn init_repo() -> (TempDir, TempDir) {
        let repo = tempfile::tempdir().unwrap();
        let base = tempfile::tempdir().unwrap();
        let p = repo.path();
        run(p, &["init", "-b", "main"]);
        run(p, &["config", "user.name", "t"]);
        run(p, &["config", "user.email", "t@test"]);
        run(p, &["config", "commit.gpgsign", "false"]);
        run(p, &["config", "tag.gpgsign", "false"]);
        std::fs::write(p.join("hello.txt"), "orig\n").unwrap();
        run(p, &["add", "hello.txt"]);
        run(p, &["commit", "-m", "init"]);
        (repo, base)
    }

    #[test]
    fn spawn_creates_worktree_with_new_branch() {
        let (repo, base) = init_repo();
        let mut pool = GitWorktreePool::new(repo.path(), base.path(), 4);
        let h = pool.spawn("wt-1", "feat/one").expect("spawn");
        assert!(h.path.is_dir());
        assert_eq!(h.branch, "feat/one");
        assert_eq!(pool.active_count(), 1);
    }

    #[test]
    fn spawn_fails_at_capacity() {
        let (repo, base) = init_repo();
        let mut pool = GitWorktreePool::new(repo.path(), base.path(), 1);
        pool.spawn("wt-1", "feat/a").expect("first");
        let err = pool.spawn("wt-2", "feat/b").unwrap_err();
        assert!(err.to_lowercase().contains("capacity"), "{err}");
    }

    #[test]
    fn remove_deletes_directory() {
        let (repo, base) = init_repo();
        let mut pool = GitWorktreePool::new(repo.path(), base.path(), 4);
        let h = pool.spawn("wt-1", "feat/two").expect("spawn");
        let path = h.path.clone();
        pool.remove("wt-1").expect("remove");
        assert!(!path.exists(), "worktree dir still exists");
        assert!(pool.get("wt-1").is_none());
    }

    #[test]
    fn clean_merge_brings_changes_in() {
        let (repo, base) = init_repo();
        let mut pool = GitWorktreePool::new(repo.path(), base.path(), 4);
        let h = pool.spawn("wt-1", "feat/clean").expect("spawn");
        std::fs::write(h.path.join("added.txt"), "added\n").unwrap();
        run(&h.path, &["add", "added.txt"]);
        run(&h.path, &["commit", "-m", "add"]);
        let outcome = pool.merge_into("wt-1", "main").expect("merge");
        assert!(outcome.merged, "{outcome:?}");
        assert!(outcome.conflicts.is_empty());
        assert!(repo.path().join("added.txt").exists());
    }

    #[test]
    fn conflicting_merge_aborts_and_reports_conflict() {
        let (repo, base) = init_repo();
        let mut pool = GitWorktreePool::new(repo.path(), base.path(), 4);
        let h = pool.spawn("wt-1", "feat/x").expect("spawn");
        // branch change
        std::fs::write(h.path.join("hello.txt"), "A\n").unwrap();
        run(&h.path, &["add", "hello.txt"]);
        run(&h.path, &["commit", "-m", "A"]);
        // main change
        run(repo.path(), &["checkout", "main"]);
        std::fs::write(repo.path().join("hello.txt"), "B\n").unwrap();
        run(repo.path(), &["add", "hello.txt"]);
        run(repo.path(), &["commit", "-m", "B"]);

        let outcome = pool.merge_into("wt-1", "main").expect("merge call");
        assert!(!outcome.merged);
        assert!(
            outcome.conflicts.iter().any(|c| c == "hello.txt"),
            "conflicts: {:?}",
            outcome.conflicts
        );
        // source repo clean after abort
        let status = Command::new("git")
            .current_dir(repo.path())
            .args(["status", "--porcelain"])
            .output()
            .unwrap();
        assert!(
            String::from_utf8_lossy(&status.stdout).trim().is_empty(),
            "source not clean after abort"
        );
    }
}
