//! Worktree reaper — the single owner of destructive git for task worktrees.
//!
//! See `docs/design/worktree-lifecycle/`. The daemon creates one git worktree
//! per task under `.vibecli/worktrees/<id>`. Deleting/archiving a task is a
//! reversible *state change* (`trashed_at` / `archived_at` on the row); the
//! physical reclaim of the directory + branch is deferred to this reaper, which
//! runs once on boot and periodically thereafter.
//!
//! Safety invariants, enforced before any directory is removed:
//!   1. Never touch a live task (status running/reviewing) — the store queries
//!      already exclude those.
//!   2. Commit before remove — dirty worktrees are committed onto their branch
//!      so work is never discarded.
//!   3. Preserve before delete — a branch is `-d`-deleted only when fully merged;
//!      otherwise its tip is moved to `refs/trash/<id>` and kept.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::task_store::{TaskRow, TaskStore};

/// Reaper tuning knobs. Defaults match `docs/design/worktree-lifecycle/`.
#[derive(Debug, Clone)]
pub struct ReaperPolicy {
    /// How long a trashed task is retained (and restorable) before its worktree
    /// is reclaimed. Default 14 days.
    pub trash_grace_secs: i64,
}

impl Default for ReaperPolicy {
    fn default() -> Self {
        Self {
            trash_grace_secs: 14 * 24 * 60 * 60,
        }
    }
}

/// What a sweep did. Logged by the daemon; returned for tests.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct SweepReport {
    /// Trashed tasks whose worktrees were reclaimed after the grace window.
    pub trashed_reaped: usize,
    /// Archived tasks whose worktree directory was freed (branch kept).
    pub archived_reclaimed: usize,
    /// On-disk worktrees with no owning row that were removed.
    pub orphans_removed: usize,
    /// Branches preserved at `refs/trash/<id>` because they held unmerged work.
    pub branches_preserved: usize,
    /// Non-fatal problems (a single bad worktree never aborts the sweep).
    pub errors: Vec<String>,
}

impl SweepReport {
    fn merge(&mut self, other: SweepReport) {
        self.trashed_reaped += other.trashed_reaped;
        self.archived_reclaimed += other.archived_reclaimed;
        self.orphans_removed += other.orphans_removed;
        self.branches_preserved += other.branches_preserved;
        self.errors.extend(other.errors);
    }

    /// True if the sweep changed anything (used to decide whether to log).
    pub fn did_work(&self) -> bool {
        self.trashed_reaped > 0
            || self.archived_reclaimed > 0
            || self.orphans_removed > 0
            || self.branches_preserved > 0
    }
}

/// Run a full sweep: reap post-grace trashed tasks, reclaim archived
/// directories, and reconcile each repo's `.vibecli/worktrees/` against the DB.
/// `extra_repos` lets the caller add repos not yet referenced by any task row
/// (e.g. the daemon's `workspace_root`).
pub fn sweep(
    store: &TaskStore,
    extra_repos: &[PathBuf],
    policy: &ReaperPolicy,
    now: i64,
) -> SweepReport {
    let mut report = SweepReport::default();

    // 1. Trashed-past-grace → reclaim dir + dispose branch.
    let cutoff = now.saturating_sub(policy.trash_grace_secs);
    match store.reapable_trashed(cutoff) {
        Ok(rows) => {
            for row in rows {
                match reap_trashed_row(store, &row, now) {
                    Ok(preserved) => {
                        report.trashed_reaped += 1;
                        if preserved {
                            report.branches_preserved += 1;
                        }
                    }
                    Err(e) => report
                        .errors
                        .push(format!("reap trashed {}: {e}", row.id)),
                }
            }
        }
        Err(e) => report.errors.push(format!("query reapable_trashed: {e}")),
    }

    // 2. Archived with a live worktree dir → free disk, keep branch.
    match store.archived_with_worktree() {
        Ok(rows) => {
            for row in rows {
                match reclaim_archived_row(store, &row, now) {
                    Ok(()) => report.archived_reclaimed += 1,
                    Err(e) => report
                        .errors
                        .push(format!("reclaim archived {}: {e}", row.id)),
                }
            }
        }
        Err(e) => report
            .errors
            .push(format!("query archived_with_worktree: {e}")),
    }

    // 3. Orphan reconciliation across every known repo.
    let mut repos: Vec<PathBuf> = extra_repos.to_vec();
    if let Ok(paths) = store.distinct_project_paths() {
        repos.extend(paths.into_iter().map(PathBuf::from));
    }
    dedup_paths(&mut repos);

    let known = known_worktree_set(store);
    for repo in &repos {
        report.merge(reconcile_orphans(repo, &known));
    }

    report
}

/// Permanently remove a single task *now* (the explicit purge path), going
/// through the same safe teardown as the reaper: commit WIP, remove the
/// directory, dispose the branch (preserve-if-unmerged), then hard-delete the
/// row. Returns whether the branch was preserved. Unlike the old
/// `--force` delete, this can never silently discard unmerged work.
pub fn purge_task(store: &TaskStore, row: &TaskRow, _now: i64) -> anyhow::Result<bool> {
    let repo = PathBuf::from(&row.project_path);
    reclaim_worktree_dir(&repo, &row.worktree_path)?;
    let preserved = dispose_branch(&repo, &row.branch, &row.id)?;
    store.delete(&row.id)?;
    Ok(preserved)
}

/// Restore a trashed/archived task to Active, re-materializing its worktree if
/// the directory was already reclaimed. Recovers the branch from
/// `refs/trash/<id>` if it had been preserved. Returns the worktree path the
/// task is now checked out at (empty if the project isn't a git repo). Updates
/// the row's lifecycle flags and `worktree_path`.
pub fn restore_task(store: &TaskStore, row: &TaskRow, now: i64) -> anyhow::Result<String> {
    let repo = PathBuf::from(&row.project_path);

    // If the directory still exists (restored within the grace window before the
    // reaper ran), just clear the flags.
    if !row.worktree_path.is_empty() && Path::new(&row.worktree_path).exists() {
        store.restore(&row.id, now)?;
        return Ok(row.worktree_path.clone());
    }

    // Otherwise re-materialize. Nothing to do if it isn't a git repo or has no
    // branch recorded.
    if row.branch.is_empty() || !vibe_core::git::is_git_repo(&repo) {
        store.restore(&row.id, now)?;
        return Ok(String::new());
    }

    // Recover the branch from its preserved tip if it was deleted.
    if !vibe_core::git::branch_exists(&repo, &row.branch) {
        let trash_ref = format!("refs/trash/{}", row.id);
        if vibe_core::git::ref_exists(&repo, &trash_ref) {
            vibe_core::git::create_branch_from_ref(&repo, &row.branch, &trash_ref)?;
        } else {
            // Branch and preserved ref both gone — restore the row but leave it
            // worktree-less; the caller can re-run the task.
            store.restore(&row.id, now)?;
            return Ok(String::new());
        }
    }

    let wt = repo.join(".vibecli").join("worktrees").join(&row.id);
    if !wt.exists() {
        vibe_core::git::add_worktree_existing_branch(&repo, &row.branch, &wt)?;
    }
    let path = wt.to_string_lossy().into_owned();
    store.restore(&row.id, now)?;
    store.set_worktree(&row.id, &row.branch, &path, now)?;
    Ok(path)
}

/// Reap one trashed row: commit WIP, remove the worktree dir, dispose its
/// branch (preserve-if-unmerged), and mark the row reaped. Returns whether the
/// branch was preserved.
fn reap_trashed_row(store: &TaskStore, row: &TaskRow, now: i64) -> anyhow::Result<bool> {
    let repo = PathBuf::from(&row.project_path);
    reclaim_worktree_dir(&repo, &row.worktree_path)?;
    let preserved = dispose_branch(&repo, &row.branch, &row.id)?;
    store.mark_reaped(&row.id, now)?;
    Ok(preserved)
}

/// Reclaim an archived row's directory while keeping its branch forever (restore
/// re-creates the worktree from it). Marks the row reaped (dir gone) but leaves
/// `archived_at` set so it still shows in the Archive view.
fn reclaim_archived_row(store: &TaskStore, row: &TaskRow, now: i64) -> anyhow::Result<()> {
    let repo = PathBuf::from(&row.project_path);
    reclaim_worktree_dir(&repo, &row.worktree_path)?;
    store.mark_reaped(&row.id, now)?;
    Ok(())
}

/// Commit any uncommitted work onto the branch, then remove the worktree
/// directory. No-op if the path is empty or already gone.
fn reclaim_worktree_dir(repo: &Path, worktree_path: &str) -> anyhow::Result<()> {
    if worktree_path.is_empty() {
        return Ok(());
    }
    let wt = PathBuf::from(worktree_path);
    if !wt.exists() {
        return Ok(());
    }
    // Best-effort commit so nothing is discarded; a failure here must not block
    // reclaim (e.g. detached/odd state) — the branch dispose step still runs.
    if let Err(e) = vibe_core::git::commit_all_in_worktree(&wt, "vibecli: preserve WIP before worktree reclaim")
    {
        tracing::warn!("commit-before-reclaim failed for {worktree_path}: {e}");
    }
    vibe_core::git::remove_worktree(repo, &wt)
}

/// Dispose a branch: delete it if merged, else preserve its tip at
/// `refs/trash/<id>` and force-delete the branch. Returns whether it was
/// preserved. No-op (Ok(false)) for an empty/absent branch.
fn dispose_branch(repo: &Path, branch: &str, id: &str) -> anyhow::Result<bool> {
    if branch.is_empty() || !vibe_core::git::branch_exists(repo, branch) {
        return Ok(false);
    }
    if vibe_core::git::is_branch_merged(repo, branch).unwrap_or(false) {
        vibe_core::git::delete_branch(repo, branch, false)?;
        Ok(false)
    } else {
        vibe_core::git::preserve_branch_ref(repo, branch, id)?;
        vibe_core::git::delete_branch(repo, branch, true)?;
        Ok(true)
    }
}

/// Reconcile one repo's `.vibecli/worktrees/` against the set of worktree paths
/// the store still tracks. Any directory not tracked is an orphan: its WIP is
/// committed, its branch disposed (preserve-if-unmerged), and the directory
/// removed. Finishes with `git worktree prune` to clear stale admin metadata.
fn reconcile_orphans(repo: &Path, known: &HashSet<String>) -> SweepReport {
    let mut report = SweepReport::default();
    let base = repo.join(".vibecli").join("worktrees");
    if !base.is_dir() {
        return report;
    }

    // Map registered-worktree path → branch, so we can dispose orphan branches.
    let registered: std::collections::HashMap<String, String> =
        match vibe_core::git::list_worktrees(repo) {
            Ok(wts) => wts
                .into_iter()
                .map(|w| (canon(&w.path), w.branch))
                .collect(),
            Err(e) => {
                report.errors.push(format!("list_worktrees {repo:?}: {e}"));
                std::collections::HashMap::new()
            }
        };

    let entries = match std::fs::read_dir(&base) {
        Ok(e) => e,
        Err(e) => {
            report.errors.push(format!("read_dir {base:?}: {e}"));
            return report;
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let key = canon(&path);
        if known.contains(&key) {
            continue; // tracked by a live/trashed/archived row — leave it
        }

        // Orphan. The directory name is the task id (worktree pool uses
        // `<base>/<id>`); use it for the preserve ref.
        let id = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let branch = registered.get(&key).cloned().unwrap_or_default();

        let result: anyhow::Result<bool> = (|| {
            reclaim_worktree_dir(repo, &path.to_string_lossy())?;
            // If git didn't recognise it as a worktree (no branch / leftover
            // dir), remove the bare directory so it doesn't linger.
            if path.exists() {
                std::fs::remove_dir_all(&path)?;
            }
            dispose_branch(repo, &branch, &id)
        })();

        match result {
            Ok(preserved) => {
                report.orphans_removed += 1;
                if preserved {
                    report.branches_preserved += 1;
                }
            }
            Err(e) => report.errors.push(format!("orphan {key}: {e}")),
        }
    }

    if let Err(e) = vibe_core::git::prune_worktrees(repo) {
        report.errors.push(format!("prune {repo:?}: {e}"));
    }
    report
}

/// Every worktree path the store tracks, canonicalized where possible plus the
/// raw form, so a scanned directory can be matched regardless of symlink/`..`
/// normalization.
fn known_worktree_set(store: &TaskStore) -> HashSet<String> {
    let mut set = HashSet::new();
    if let Ok(paths) = store.known_worktree_paths() {
        for p in paths {
            set.insert(canon(Path::new(&p)));
            set.insert(p);
        }
    }
    set
}

/// Canonicalize a path to a comparable string, falling back to the lossy form
/// when the path doesn't exist (already removed) or can't be resolved.
fn canon(p: &Path) -> String {
    std::fs::canonicalize(p)
        .map(|c| c.to_string_lossy().into_owned())
        .unwrap_or_else(|_| p.to_string_lossy().into_owned())
}

fn dedup_paths(paths: &mut Vec<PathBuf>) {
    let mut seen = HashSet::new();
    paths.retain(|p| seen.insert(canon(p)));
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;
    use tempfile::TempDir;

    fn git(repo: &Path, args: &[&str]) {
        let st = Command::new("git")
            .args(args)
            .current_dir(repo)
            .output()
            .expect("git")
            .status;
        assert!(st.success(), "git {args:?} failed");
    }

    fn make_repo() -> TempDir {
        let dir = TempDir::new().unwrap();
        let p = dir.path();
        git(p, &["init", "-b", "main"]);
        git(p, &["config", "user.email", "t@t.com"]);
        git(p, &["config", "user.name", "T"]);
        std::fs::write(p.join("README.md"), "init").unwrap();
        git(p, &["add", "-A"]);
        git(p, &["commit", "-m", "init"]);
        dir
    }

    fn add_worktree(repo: &Path, id: &str, branch: &str) -> PathBuf {
        let wt = repo.join(".vibecli").join("worktrees").join(id);
        std::fs::create_dir_all(wt.parent().unwrap()).unwrap();
        git(
            repo,
            &["worktree", "add", "-b", branch, &wt.to_string_lossy()],
        );
        wt
    }

    #[test]
    fn orphan_with_merged_branch_is_fully_removed() {
        let dir = make_repo();
        let repo = dir.path();
        let wt = add_worktree(repo, "abc123", "task/abc123-hi");
        assert!(wt.exists());

        // No row tracks it → orphan. Branch has no unique commits → merged.
        let report = reconcile_orphans(repo, &HashSet::new());
        assert_eq!(report.orphans_removed, 1, "{report:?}");
        assert_eq!(report.branches_preserved, 0);
        assert!(!wt.exists(), "worktree dir should be gone");
        assert!(
            !vibe_core::git::branch_exists(repo, "task/abc123-hi"),
            "merged branch should be deleted"
        );
    }

    #[test]
    fn orphan_with_unmerged_work_is_preserved() {
        let dir = make_repo();
        let repo = dir.path();
        let wt = add_worktree(repo, "def456", "task/def456-work");
        // Commit a unique change inside the worktree → unmerged.
        std::fs::write(wt.join("new.txt"), "important").unwrap();
        git(&wt, &["add", "-A"]);
        git(&wt, &["commit", "-m", "wip"]);

        let report = reconcile_orphans(repo, &HashSet::new());
        assert_eq!(report.orphans_removed, 1, "{report:?}");
        assert_eq!(report.branches_preserved, 1, "{report:?}");
        assert!(!wt.exists());
        // Branch deleted, but the tip is preserved under refs/trash/<id>.
        assert!(!vibe_core::git::branch_exists(repo, "task/def456-work"));
        let show = Command::new("git")
            .args(["rev-parse", "--verify", "refs/trash/def456"])
            .current_dir(repo)
            .output()
            .unwrap();
        assert!(show.status.success(), "refs/trash/def456 should exist");
    }

    #[test]
    fn tracked_worktree_is_left_alone() {
        let dir = make_repo();
        let repo = dir.path();
        let wt = add_worktree(repo, "keep01", "task/keep01-hi");
        let mut known = HashSet::new();
        known.insert(canon(&wt));

        let report = reconcile_orphans(repo, &known);
        assert_eq!(report.orphans_removed, 0, "{report:?}");
        assert!(wt.exists(), "tracked worktree must survive");
    }

    #[test]
    fn dirty_orphan_commits_before_removal_then_preserves() {
        let dir = make_repo();
        let repo = dir.path();
        let wt = add_worktree(repo, "dirty1", "task/dirty1-x");
        // Uncommitted change — must be committed (not discarded) then preserved.
        std::fs::write(wt.join("scratch.txt"), "unsaved work").unwrap();

        let report = reconcile_orphans(repo, &HashSet::new());
        assert_eq!(report.orphans_removed, 1, "{report:?}");
        assert_eq!(report.branches_preserved, 1, "{report:?}");
        // The committed WIP is reachable from the preserved ref.
        let files = Command::new("git")
            .args(["ls-tree", "--name-only", "refs/trash/dirty1"])
            .current_dir(repo)
            .output()
            .unwrap();
        let listing = String::from_utf8_lossy(&files.stdout);
        assert!(
            listing.contains("scratch.txt"),
            "WIP should be committed onto the preserved ref, got: {listing}"
        );
    }
}
