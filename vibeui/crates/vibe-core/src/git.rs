use anyhow::Result;
use git2::{Repository, StatusOptions};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FileStatus {
    Modified,
    New,
    Deleted,
    Renamed,
    Ignored,
    Conflicted,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitStatus {
    pub branch: String,
    pub file_statuses: HashMap<String, FileStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitInfo {
    pub hash: String,
    pub author: String,
    pub message: String,
    pub timestamp: i64,
}

pub fn get_status(root_path: &Path) -> Result<GitStatus> {
    let repo = Repository::open(root_path)?;

    // Get branch name
    let head = repo.head().ok();
    let branch = head
        .as_ref()
        .and_then(|h| h.shorthand())
        .unwrap_or("DETACHED")
        .to_string();

    // Get file statuses
    let mut file_statuses = HashMap::new();
    let mut opts = StatusOptions::new();
    opts.include_untracked(true);

    for entry in repo.statuses(Some(&mut opts))?.iter() {
        let path = entry.path().unwrap_or("").to_string();
        let status = entry.status();

        let file_status = if status.is_wt_new() || status.is_index_new() {
            FileStatus::New
        } else if status.is_wt_modified() || status.is_index_modified() {
            FileStatus::Modified
        } else if status.is_wt_deleted() || status.is_index_deleted() {
            FileStatus::Deleted
        } else if status.is_wt_renamed() || status.is_index_renamed() {
            FileStatus::Renamed
        } else if status.is_ignored() {
            FileStatus::Ignored
        } else if status.is_conflicted() {
            FileStatus::Conflicted
        } else {
            FileStatus::Unknown
        };

        file_statuses.insert(path, file_status);
    }

    Ok(GitStatus {
        branch,
        file_statuses,
    })
}

pub fn commit(
    repo_path: &Path,
    message: &str,
    files: Vec<String>,
    author_name: Option<&str>,
    author_email: Option<&str>,
) -> Result<()> {
    let repo = Repository::open(repo_path)?;
    let mut index = repo.index()?;

    // Stage files
    for file in files {
        index.add_path(Path::new(&file))?;
    }
    index.write()?;

    // Create commit
    let tree_id = index.write_tree()?;
    let tree = repo.find_tree(tree_id)?;
    let signature = repo.signature().or_else(|_| {
        // Use provided author info, or fall back to defaults
        let name = author_name.unwrap_or("VibeUI User");
        let email = author_email.unwrap_or("user@vibeui.local");
        git2::Signature::now(name, email)
    })?;
    let parent_commit = repo.head()?.peel_to_commit()?;

    repo.commit(
        Some("HEAD"),
        &signature,
        &signature,
        message,
        &tree,
        &[&parent_commit],
    )?;

    Ok(())
}

pub fn push(repo_path: &Path, remote: &str, branch: &str) -> Result<()> {
    let repo = Repository::open(repo_path)?;
    let mut remote = repo.find_remote(remote)?;
    
    remote.push(
        &[&format!("refs/heads/{}", branch)],
        None,
    )?;
    
    Ok(())
}

pub fn pull(repo_path: &Path, remote: &str, branch: &str) -> Result<()> {
    let repo = Repository::open(repo_path)?;
    let mut remote = repo.find_remote(remote)?;
    
    // Fetch
    remote.fetch(&[branch], None, None)?;
    
    // Merge
    let fetch_head = repo.find_reference("FETCH_HEAD")?;
    let fetch_commit = repo.reference_to_annotated_commit(&fetch_head)?;
    
    let analysis = repo.merge_analysis(&[&fetch_commit])?;
    
    if analysis.0.is_up_to_date() {
        return Ok(());
    } else if analysis.0.is_fast_forward() {
        let refname = format!("refs/heads/{}", branch);
        let mut reference = repo.find_reference(&refname)?;
        reference.set_target(fetch_commit.id(), "Fast-Forward")?;
        repo.set_head(&refname)?;
        repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force()))?;
    }
    
    Ok(())
}

pub fn get_diff(repo_path: &Path, file_path: &str) -> Result<String> {
    let repo = Repository::open(repo_path)?;
    let head = repo.head()?.peel_to_tree()?;
    
    let mut diff_opts = git2::DiffOptions::new();
    diff_opts.pathspec(file_path);
    
    let diff = repo.diff_tree_to_workdir_with_index(Some(&head), Some(&mut diff_opts))?;
    
    let mut diff_text = String::new();
    diff.print(git2::DiffFormat::Patch, |_delta, _hunk, line| {
        let content = String::from_utf8_lossy(line.content());
        diff_text.push_str(&content);
        true
    })?;
    
    Ok(diff_text)
}

pub fn list_branches(repo_path: &Path) -> Result<Vec<String>> {
    let repo = Repository::open(repo_path)?;
    let branches = repo.branches(None)?;
    
    let mut branch_names = Vec::new();
    for branch in branches {
        let (branch, _) = branch?;
        if let Some(name) = branch.name()? {
            branch_names.push(name.to_string());
        }
    }
    
    Ok(branch_names)
}

pub fn switch_branch(repo_path: &Path, branch: &str) -> Result<()> {
    let repo = Repository::open(repo_path)?;

    // Try local branch first
    if let Ok(branch_ref) = repo.find_branch(branch, git2::BranchType::Local) {
        let ref_name = branch_ref.get().name()
            .ok_or_else(|| anyhow::anyhow!("Branch reference has non-UTF-8 name"))?
            .to_string();
        repo.set_head(&ref_name)?;
        repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force()))?;
        return Ok(());
    }

    // Check if it's a remote branch (e.g. "origin/feature")
    if let Ok(remote_branch) = repo.find_branch(branch, git2::BranchType::Remote) {
        // Derive local name by stripping the remote prefix (e.g. "origin/feature" -> "feature")
        let local_name = branch.split_once('/')
            .map(|(_, name)| name)
            .unwrap_or(branch);

        // Create a local branch tracking the remote
        let commit = remote_branch.get().peel_to_commit()?;
        let mut new_branch = repo.branch(local_name, &commit, false)?;

        // Set upstream tracking
        let remote_ref = remote_branch.get().name()
            .ok_or_else(|| anyhow::anyhow!("Remote branch reference has non-UTF-8 name"))?;
        new_branch.set_upstream(Some(remote_ref))?;

        let ref_name = new_branch.get().name()
            .ok_or_else(|| anyhow::anyhow!("New branch reference has non-UTF-8 name"))?
            .to_string();
        repo.set_head(&ref_name)?;
        repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force()))?;
        return Ok(());
    }

    anyhow::bail!("Branch '{}' not found (checked local and remote)", branch)
}

pub fn get_history(repo_path: &Path, limit: usize) -> Result<Vec<CommitInfo>> {
    let repo = Repository::open(repo_path)?;
    let mut revwalk = repo.revwalk()?;
    revwalk.push_head()?;
    
    let mut commits = Vec::new();
    for (i, oid) in revwalk.enumerate() {
        if i >= limit {
            break;
        }
        
        let oid = oid?;
        let commit = repo.find_commit(oid)?;
        
        commits.push(CommitInfo {
            hash: oid.to_string(),
            author: commit.author().name().unwrap_or("Unknown").to_string(),
            message: commit.message().unwrap_or("").to_string(),
            timestamp: commit.time().seconds(),
        });
    }
    
    Ok(commits)
}

/// Return the list of files changed in a given commit (by SHA hash string).
///
/// Each entry is a relative path string.  For merge commits the diff is taken
/// against the first parent; for the root commit the diff is taken against an
/// empty tree.
pub fn get_commit_files(repo_path: &Path, hash: &str) -> Result<Vec<String>> {
    let repo = Repository::open(repo_path)?;
    let oid = repo.revparse_single(hash)?.id();
    let commit = repo.find_commit(oid)?;

    let tree = commit.tree()?;
    let parent_tree = if commit.parent_count() > 0 {
        Some(commit.parent(0)?.tree()?)
    } else {
        None
    };

    let diff = repo.diff_tree_to_tree(
        parent_tree.as_ref(),
        Some(&tree),
        None,
    )?;

    let mut files = Vec::new();
    diff.foreach(
        &mut |delta, _progress| {
            if let Some(path) = delta.new_file().path() {
                files.push(path.to_string_lossy().into_owned());
            } else if let Some(path) = delta.old_file().path() {
                // deleted file — report old path
                files.push(path.to_string_lossy().into_owned());
            }
            true
        },
        None,
        None,
        None,
    )?;

    files.sort();
    files.dedup();
    Ok(files)
}

pub fn discard_changes(repo_path: &Path, file_path: &str) -> Result<()> {
    let repo = Repository::open(repo_path)?;
    
    // Checkout the file from HEAD
    let mut checkout_builder = git2::build::CheckoutBuilder::new();
    checkout_builder.path(file_path);
    checkout_builder.force();
    
    repo.checkout_head(Some(&mut checkout_builder))?;
    
    Ok(())
}



pub fn is_git_repo(path: &Path) -> bool {
    Repository::open(path).is_ok()
}

pub fn get_current_branch(path: &Path) -> Result<String> {
    let repo = Repository::open(path)?;
    let head = repo.head()?;
    Ok(head.shorthand().unwrap_or("DETACHED").to_string())
}

/// Create a named git stash of all current changes. Returns the stash ref name.
pub fn create_stash(repo_path: &Path, name: &str) -> Result<String> {
    let mut repo = Repository::open(repo_path)?;
    let sig = repo.signature()?;
    let message = format!("vibeui-pre-ai: {}", name);
    let stash_oid = repo.stash_save(&sig, &message, None)?;
    Ok(stash_oid.to_string())
}

/// Pop (apply + drop) the most recent git stash.
pub fn pop_stash(repo_path: &Path) -> Result<()> {
    let mut repo = Repository::open(repo_path)?;
    let mut stash_opts = git2::StashApplyOptions::new();
    repo.stash_apply(0, Some(&mut stash_opts))?;
    repo.stash_drop(0)?;
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointInfo {
    pub index: usize,
    pub message: String,
    pub oid: String,
}

/// List all stashes as checkpoint info (newest first).
pub fn list_stashes(repo_path: &Path) -> Result<Vec<CheckpointInfo>> {
    let mut repo = Repository::open(repo_path)?;
    let mut stashes = Vec::new();
    repo.stash_foreach(|index, message, oid| {
        stashes.push(CheckpointInfo {
            index,
            message: message.to_string(),
            oid: oid.to_string(),
        });
        true
    })?;
    Ok(stashes)
}

/// Drop (delete) a stash at `index` permanently.
pub fn drop_stash(repo_path: &Path, index: usize) -> Result<()> {
    let mut repo = Repository::open(repo_path)?;
    repo.stash_drop(index)?;
    Ok(())
}

/// Apply a stash at `index` without dropping it (allows repeated restore).
pub fn restore_stash(repo_path: &Path, index: usize) -> Result<()> {
    let mut repo = Repository::open(repo_path)?;
    let mut opts = git2::StashApplyOptions::new();
    repo.stash_apply(index, Some(&mut opts))?;
    Ok(())
}

// ── Git Worktree Management ───────────────────────────────────────────────────

/// Information about a git worktree.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorktreeInfo {
    /// Branch checked out in this worktree.
    pub branch: String,
    /// Absolute path to the worktree directory.
    pub path: std::path::PathBuf,
    /// Whether this is the main (primary) worktree.
    pub is_main: bool,
}

/// Create a new git worktree at `worktree_path` on a new branch named `branch`.
///
/// Equivalent to: `git worktree add <worktree_path> -b <branch>`
pub fn create_worktree(repo_path: &Path, branch: &str, worktree_path: &Path) -> Result<()> {
    let output = std::process::Command::new("git")
        .args(["worktree", "add", "-b", branch, &worktree_path.to_string_lossy()])
        .current_dir(repo_path)
        .output()?;
    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("git worktree add failed: {}", err);
    }
    Ok(())
}

/// Remove a git worktree at `worktree_path`.
///
/// Equivalent to: `git worktree remove --force <worktree_path>`
pub fn remove_worktree(repo_path: &Path, worktree_path: &Path) -> Result<()> {
    let output = std::process::Command::new("git")
        .args(["worktree", "remove", "--force", &worktree_path.to_string_lossy()])
        .current_dir(repo_path)
        .output()?;
    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("git worktree remove failed: {}", err);
    }
    Ok(())
}

/// List all worktrees for the repository.
pub fn list_worktrees(repo_path: &Path) -> Result<Vec<WorktreeInfo>> {
    let output = std::process::Command::new("git")
        .args(["worktree", "list", "--porcelain"])
        .current_dir(repo_path)
        .output()?;
    if !output.status.success() {
        anyhow::bail!("git worktree list failed");
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut worktrees = Vec::new();
    let mut current_path = None::<std::path::PathBuf>;
    let mut current_branch = String::new();
    let mut is_bare = false;

    for line in stdout.lines() {
        if line.starts_with("worktree ") {
            // Save previous entry
            if let Some(path) = current_path.take() {
                if !is_bare {
                    worktrees.push(WorktreeInfo {
                        path,
                        branch: current_branch.clone(),
                        is_main: worktrees.is_empty(),
                    });
                }
            }
            current_path = Some(std::path::PathBuf::from(line.trim_start_matches("worktree ")));
            current_branch.clear();
            is_bare = false;
        } else if line.starts_with("branch ") {
            current_branch = line.trim_start_matches("branch refs/heads/").to_string();
        } else if line == "bare" {
            is_bare = true;
        }
    }
    // Last entry
    if let Some(path) = current_path {
        if !is_bare {
            worktrees.push(WorktreeInfo {
                path,
                branch: current_branch,
                is_main: worktrees.is_empty(),
            });
        }
    }
    Ok(worktrees)
}

/// The result of merging a worktree branch back into the current branch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergeResult {
    pub success: bool,
    pub message: String,
    pub conflicts: Vec<String>,
}

/// Merge the branch from a worktree back into HEAD.
///
/// Equivalent to: `git merge <branch> --no-ff -m "Merge worktree <branch>"`
pub fn merge_worktree_branch(repo_path: &Path, branch: &str) -> Result<MergeResult> {
    let output = std::process::Command::new("git")
        .args([
            "merge",
            branch,
            "--no-ff",
            "-m",
            &format!("Merge worktree branch '{}'", branch),
        ])
        .current_dir(repo_path)
        .output()?;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if output.status.success() {
        Ok(MergeResult {
            success: true,
            message: stdout,
            conflicts: vec![],
        })
    } else {
        // Parse conflict file names from stderr/stdout
        let conflicts: Vec<String> = stdout
            .lines()
            .chain(stderr.lines())
            .filter(|l| l.contains("CONFLICT") || l.contains("conflict"))
            .map(|l| l.to_string())
            .collect();
        Ok(MergeResult {
            success: false,
            message: stderr,
            conflicts,
        })
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;
    use tempfile::TempDir;

    /// Create a minimal git repo with one commit so stash operations work.
    fn make_git_repo() -> TempDir {
        let dir = TempDir::new().expect("tempdir");
        let path = dir.path();

        let run = |args: &[&str]| {
            let status = Command::new("git")
                .args(args)
                .current_dir(path)
                .output()
                .expect("git command")
                .status;
            assert!(status.success(), "git {:?} failed", args);
        };

        run(&["init"]);
        run(&["config", "user.email", "test@test.com"]);
        run(&["config", "user.name", "Test"]);
        // Create + commit a file so HEAD exists
        std::fs::write(path.join("README.md"), "init").unwrap();
        run(&["add", "README.md"]);
        run(&["commit", "-m", "init"]);
        dir
    }

    // ── is_git_repo ───────────────────────────────────────────────────────────

    #[test]
    fn is_git_repo_true_for_real_repo() {
        let dir = make_git_repo();
        assert!(is_git_repo(dir.path()));
    }

    #[test]
    fn is_git_repo_false_for_plain_dir() {
        let dir = TempDir::new().unwrap();
        assert!(!is_git_repo(dir.path()));
    }

    // ── get_current_branch ────────────────────────────────────────────────────

    #[test]
    fn get_current_branch_returns_main_or_master() {
        let dir = make_git_repo();
        let branch = get_current_branch(dir.path()).unwrap();
        assert!(branch == "main" || branch == "master",
            "unexpected branch: {branch}");
    }

    // ── get_status ────────────────────────────────────────────────────────────

    #[test]
    fn get_status_clean_repo_has_no_modified_files() {
        let dir = make_git_repo();
        let status = get_status(dir.path()).unwrap();
        let modified: Vec<_> = status.file_statuses
            .values()
            .filter(|s| **s == FileStatus::Modified)
            .collect();
        assert!(modified.is_empty(), "clean repo should have no modified files");
    }

    #[test]
    fn get_status_detects_new_untracked_file() {
        let dir = make_git_repo();
        std::fs::write(dir.path().join("new.txt"), "hello").unwrap();
        let status = get_status(dir.path()).unwrap();
        assert!(
            status.file_statuses.values().any(|s| *s == FileStatus::New),
            "untracked file should appear as New"
        );
    }

    #[test]
    fn get_status_detects_modified_file() {
        let dir = make_git_repo();
        std::fs::write(dir.path().join("README.md"), "changed").unwrap();
        let status = get_status(dir.path()).unwrap();
        assert!(
            status.file_statuses.values().any(|s| *s == FileStatus::Modified),
            "changed file should appear as Modified"
        );
    }

    // ── stash operations ──────────────────────────────────────────────────────

    #[test]
    fn list_stashes_empty_on_clean_repo() {
        let dir = make_git_repo();
        let stashes = list_stashes(dir.path()).unwrap();
        assert!(stashes.is_empty(), "clean repo has no stashes");
    }

    #[test]
    fn create_and_list_stash() {
        let dir = make_git_repo();
        // Dirty the working tree so stash has something to save
        std::fs::write(dir.path().join("README.md"), "dirty").unwrap();
        create_stash(dir.path(), "before-test").unwrap();
        let stashes = list_stashes(dir.path()).unwrap();
        assert_eq!(stashes.len(), 1);
        assert!(stashes[0].message.contains("before-test"));
    }

    #[test]
    fn drop_stash_removes_entry() {
        let dir = make_git_repo();
        std::fs::write(dir.path().join("README.md"), "dirty").unwrap();
        create_stash(dir.path(), "to-drop").unwrap();
        assert_eq!(list_stashes(dir.path()).unwrap().len(), 1);
        drop_stash(dir.path(), 0).unwrap();
        assert!(list_stashes(dir.path()).unwrap().is_empty(),
            "stash should be gone after drop");
    }

    #[test]
    fn restore_stash_reapplies_changes() {
        let dir = make_git_repo();
        let file = dir.path().join("README.md");
        std::fs::write(&file, "modified content").unwrap();
        create_stash(dir.path(), "checkpoint").unwrap();
        // After stash, file is back to original
        let after_stash = std::fs::read_to_string(&file).unwrap();
        assert_eq!(after_stash, "init");
        // Restore brings back the modification
        restore_stash(dir.path(), 0).unwrap();
        let after_restore = std::fs::read_to_string(&file).unwrap();
        assert_eq!(after_restore, "modified content");
    }

    #[test]
    fn drop_stash_on_wrong_index_returns_error() {
        let dir = make_git_repo();
        let result = drop_stash(dir.path(), 99);
        assert!(result.is_err(), "dropping nonexistent stash index should fail");
    }

    // ── list_branches ──────────────────────────────────────────────────────

    #[test]
    fn list_branches_includes_default() {
        let dir = make_git_repo();
        let branches = list_branches(dir.path()).unwrap();
        assert!(!branches.is_empty(), "should have at least one branch");
        assert!(
            branches.iter().any(|b| b == "main" || b == "master"),
            "default branch should be listed"
        );
    }

    // ── get_history ────────────────────────────────────────────────────────

    #[test]
    fn get_history_returns_initial_commit() {
        let dir = make_git_repo();
        let history = get_history(dir.path(), 10).unwrap();
        assert!(!history.is_empty());
        assert!(history[0].message.contains("init"));
    }

    #[test]
    fn get_history_respects_limit() {
        let dir = make_git_repo();
        // Make a second commit
        std::fs::write(dir.path().join("second.txt"), "second").unwrap();
        let run = |args: &[&str]| {
            Command::new("git").args(args).current_dir(dir.path()).output().unwrap();
        };
        run(&["add", "second.txt"]);
        run(&["commit", "-m", "second"]);

        let history = get_history(dir.path(), 1).unwrap();
        assert_eq!(history.len(), 1);
        assert!(history[0].message.contains("second"));
    }

    // ── get_commit_files ───────────────────────────────────────────────────

    #[test]
    fn get_commit_files_lists_changed_files() {
        let dir = make_git_repo();
        // The initial commit should have README.md
        let history = get_history(dir.path(), 1).unwrap();
        let hash = &history[0].hash;
        let files = get_commit_files(dir.path(), hash).unwrap();
        assert!(files.contains(&"README.md".to_string()));
    }

    // ── get_diff ───────────────────────────────────────────────────────────

    #[test]
    fn get_diff_shows_changes() {
        let dir = make_git_repo();
        std::fs::write(dir.path().join("README.md"), "changed content").unwrap();
        let diff = get_diff(dir.path(), "README.md").unwrap();
        assert!(!diff.is_empty(), "diff should show changes");
        assert!(diff.contains("changed content") || diff.contains("README"));
    }

    #[test]
    fn get_diff_unchanged_file_empty() {
        let dir = make_git_repo();
        let diff = get_diff(dir.path(), "README.md").unwrap();
        assert!(diff.is_empty(), "unchanged file should have empty diff");
    }

    // ── get_repo_diff ──────────────────────────────────────────────────────

    #[test]
    fn get_repo_diff_on_clean_repo_is_empty() {
        let dir = make_git_repo();
        let diff = get_repo_diff(dir.path()).unwrap();
        assert!(diff.is_empty());
    }

    #[test]
    fn get_repo_diff_shows_all_changes() {
        let dir = make_git_repo();
        std::fs::write(dir.path().join("README.md"), "changed").unwrap();
        std::fs::write(dir.path().join("new.txt"), "new file").unwrap();
        let diff = get_repo_diff(dir.path()).unwrap();
        assert!(!diff.is_empty());
    }

    // ── discard_changes ────────────────────────────────────────────────────

    #[test]
    fn discard_changes_restores_file() {
        let dir = make_git_repo();
        let file = dir.path().join("README.md");
        std::fs::write(&file, "dirty content").unwrap();
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "dirty content");

        discard_changes(dir.path(), "README.md").unwrap();
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "init");
    }

    // ── commit ─────────────────────────────────────────────────────────────

    #[test]
    fn commit_creates_new_commit() {
        let dir = make_git_repo();
        std::fs::write(dir.path().join("new.txt"), "hello").unwrap();
        let run = |args: &[&str]| {
            Command::new("git").args(args).current_dir(dir.path()).output().unwrap();
        };
        run(&["add", "new.txt"]);

        commit(dir.path(), "add new file", vec!["new.txt".to_string()], None, None).unwrap();

        let history = get_history(dir.path(), 1).unwrap();
        assert!(history[0].message.contains("add new file"));
    }

    // ── switch_branch ──────────────────────────────────────────────────────

    #[test]
    fn switch_branch_changes_head() {
        let dir = make_git_repo();
        // Create a new branch
        let run = |args: &[&str]| {
            Command::new("git").args(args).current_dir(dir.path()).output().unwrap();
        };
        run(&["branch", "feature-x"]);

        switch_branch(dir.path(), "feature-x").unwrap();
        let branch = get_current_branch(dir.path()).unwrap();
        assert_eq!(branch, "feature-x");
    }

    // ── FileStatus / GitStatus serde ───────────────────────────────────────

    #[test]
    fn file_status_serde_roundtrip() {
        let status = FileStatus::Modified;
        let json = serde_json::to_string(&status).unwrap();
        let back: FileStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(back, FileStatus::Modified);
    }

    #[test]
    fn commit_info_serde_roundtrip() {
        let ci = CommitInfo {
            hash: "abc123".to_string(),
            author: "Test".to_string(),
            message: "init".to_string(),
            timestamp: 1234567890,
        };
        let json = serde_json::to_string(&ci).unwrap();
        let back: CommitInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(back.hash, "abc123");
        assert_eq!(back.timestamp, 1234567890);
    }

    #[test]
    fn checkpoint_info_serde_roundtrip() {
        let cp = CheckpointInfo {
            index: 0,
            message: "before test".to_string(),
            oid: "def456".to_string(),
        };
        let json = serde_json::to_string(&cp).unwrap();
        let back: CheckpointInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(back.message, "before test");
    }

    #[test]
    fn worktree_info_serde_roundtrip() {
        let wt = WorktreeInfo {
            branch: "main".to_string(),
            path: std::path::PathBuf::from("/tmp/wt"),
            is_main: true,
        };
        let json = serde_json::to_string(&wt).unwrap();
        let back: WorktreeInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(back.branch, "main");
        assert!(back.is_main);
    }

    #[test]
    fn merge_result_serde_roundtrip() {
        let mr = MergeResult {
            success: true,
            message: "Merged".to_string(),
            conflicts: vec![],
        };
        let json = serde_json::to_string(&mr).unwrap();
        let back: MergeResult = serde_json::from_str(&json).unwrap();
        assert!(back.success);
        assert!(back.conflicts.is_empty());
    }

    // ── is_git_repo on non-repo returns false ──────────────────────────────

    #[test]
    fn not_a_git_repo_for_nonexistent_path() {
        assert!(!is_git_repo(Path::new("/nonexistent/path/xyz")));
    }

    // ── pop_stash ──────────────────────────────────────────────────────────

    #[test]
    fn pop_stash_restores_and_removes() {
        let dir = make_git_repo();
        let file = dir.path().join("README.md");
        std::fs::write(&file, "stashed content").unwrap();
        create_stash(dir.path(), "to-pop").unwrap();
        // After stash, file reverts
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "init");

        pop_stash(dir.path()).unwrap();
        // File restored
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "stashed content");
        // Stash should be gone
        assert!(list_stashes(dir.path()).unwrap().is_empty());
    }

    // ── FileStatus enum coverage ──────────────────────────────────────────────

    #[test]
    fn file_status_all_variants_are_distinct() {
        let variants = vec![
            FileStatus::Modified,
            FileStatus::New,
            FileStatus::Deleted,
            FileStatus::Renamed,
            FileStatus::Ignored,
            FileStatus::Conflicted,
            FileStatus::Unknown,
        ];
        // Every variant should be equal to itself
        for v in &variants {
            assert_eq!(v, v);
        }
        // Adjacent variants should differ
        for pair in variants.windows(2) {
            assert_ne!(&pair[0], &pair[1]);
        }
    }

    #[test]
    fn file_status_serde_all_variants() {
        let variants = vec![
            FileStatus::Modified,
            FileStatus::New,
            FileStatus::Deleted,
            FileStatus::Renamed,
            FileStatus::Ignored,
            FileStatus::Conflicted,
            FileStatus::Unknown,
        ];
        for v in variants {
            let json = serde_json::to_string(&v).unwrap();
            let back: FileStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(back, v, "roundtrip failed for {:?}", v);
        }
    }

    #[test]
    fn file_status_debug_format() {
        let s = format!("{:?}", FileStatus::Conflicted);
        assert_eq!(s, "Conflicted");
    }

    // ── GitStatus construction ────────────────────────────────────────────────

    #[test]
    fn git_status_empty_file_statuses() {
        let status = GitStatus {
            branch: "main".to_string(),
            file_statuses: HashMap::new(),
        };
        assert_eq!(status.branch, "main");
        assert!(status.file_statuses.is_empty());
    }

    #[test]
    fn git_status_multiple_files() {
        let mut files = HashMap::new();
        files.insert("src/lib.rs".to_string(), FileStatus::Modified);
        files.insert("new_file.txt".to_string(), FileStatus::New);
        files.insert("old.rs".to_string(), FileStatus::Deleted);
        let status = GitStatus {
            branch: "feature/test".to_string(),
            file_statuses: files,
        };
        assert_eq!(status.file_statuses.len(), 3);
        assert_eq!(status.file_statuses.get("src/lib.rs"), Some(&FileStatus::Modified));
        assert_eq!(status.file_statuses.get("new_file.txt"), Some(&FileStatus::New));
        assert_eq!(status.file_statuses.get("old.rs"), Some(&FileStatus::Deleted));
    }

    #[test]
    fn git_status_serde_roundtrip() {
        let mut files = HashMap::new();
        files.insert("a.rs".to_string(), FileStatus::Modified);
        let status = GitStatus {
            branch: "dev".to_string(),
            file_statuses: files,
        };
        let json = serde_json::to_string(&status).unwrap();
        let back: GitStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(back.branch, "dev");
        assert_eq!(back.file_statuses.get("a.rs"), Some(&FileStatus::Modified));
    }

    // ── CommitInfo construction and edge cases ────────────────────────────────

    #[test]
    fn commit_info_empty_message() {
        let ci = CommitInfo {
            hash: "0000000".to_string(),
            author: "Bot".to_string(),
            message: "".to_string(),
            timestamp: 0,
        };
        assert!(ci.message.is_empty());
        assert_eq!(ci.timestamp, 0);
    }

    #[test]
    fn commit_info_long_hash() {
        let ci = CommitInfo {
            hash: "a".repeat(40),
            author: "Test User".to_string(),
            message: "initial commit".to_string(),
            timestamp: 1709827200,
        };
        assert_eq!(ci.hash.len(), 40);
    }

    #[test]
    fn commit_info_multiline_message() {
        let ci = CommitInfo {
            hash: "abc".to_string(),
            author: "Dev".to_string(),
            message: "feat: add feature\n\nDetailed description here.".to_string(),
            timestamp: 1000,
        };
        assert!(ci.message.contains('\n'));
        let first_line = ci.message.lines().next().unwrap();
        assert_eq!(first_line, "feat: add feature");
    }

    #[test]
    fn commit_info_negative_timestamp() {
        // Pre-epoch timestamps are valid in git
        let ci = CommitInfo {
            hash: "def".to_string(),
            author: "Ancient".to_string(),
            message: "before epoch".to_string(),
            timestamp: -100,
        };
        assert!(ci.timestamp < 0);
    }

    // ── MergeResult construction ──────────────────────────────────────────────

    #[test]
    fn merge_result_with_conflicts() {
        let mr = MergeResult {
            success: false,
            message: "Automatic merge failed".to_string(),
            conflicts: vec![
                "CONFLICT (content): Merge conflict in src/main.rs".to_string(),
                "CONFLICT (content): Merge conflict in README.md".to_string(),
            ],
        };
        assert!(!mr.success);
        assert_eq!(mr.conflicts.len(), 2);
        assert!(mr.conflicts[0].contains("main.rs"));
    }

    #[test]
    fn merge_result_successful_no_conflicts() {
        let mr = MergeResult {
            success: true,
            message: "Already up to date.".to_string(),
            conflicts: vec![],
        };
        assert!(mr.success);
        assert!(mr.conflicts.is_empty());
    }

    #[test]
    fn merge_result_serde_with_conflicts() {
        let mr = MergeResult {
            success: false,
            message: "conflict".to_string(),
            conflicts: vec!["file.rs".to_string()],
        };
        let json = serde_json::to_string(&mr).unwrap();
        let back: MergeResult = serde_json::from_str(&json).unwrap();
        assert!(!back.success);
        assert_eq!(back.conflicts.len(), 1);
    }

    // ── WorktreeInfo construction ─────────────────────────────────────────────

    #[test]
    fn worktree_info_non_main() {
        let wt = WorktreeInfo {
            branch: "feature/branch".to_string(),
            path: std::path::PathBuf::from("/home/user/project-wt"),
            is_main: false,
        };
        assert!(!wt.is_main);
        assert!(wt.branch.contains('/'));
    }

    #[test]
    fn worktree_info_clone() {
        let wt = WorktreeInfo {
            branch: "main".to_string(),
            path: std::path::PathBuf::from("/tmp/wt"),
            is_main: true,
        };
        let cloned = wt.clone();
        assert_eq!(cloned.branch, wt.branch);
        assert_eq!(cloned.path, wt.path);
        assert_eq!(cloned.is_main, wt.is_main);
    }

    // ── CheckpointInfo construction ───────────────────────────────────────────

    #[test]
    fn checkpoint_info_various_indices() {
        for i in [0, 1, 5, 100] {
            let cp = CheckpointInfo {
                index: i,
                message: format!("checkpoint-{}", i),
                oid: format!("{:040x}", i),
            };
            assert_eq!(cp.index, i);
        }
    }

    #[test]
    fn checkpoint_info_debug_format() {
        let cp = CheckpointInfo {
            index: 0,
            message: "test".to_string(),
            oid: "abc".to_string(),
        };
        let debug = format!("{:?}", cp);
        assert!(debug.contains("CheckpointInfo"));
        assert!(debug.contains("test"));
    }

    // ── Non-existent repo error handling ──────────────────────────────────────

    #[test]
    fn get_status_fails_for_nonexistent_path() {
        let result = get_status(Path::new("/nonexistent/repo/path/xyz"));
        assert!(result.is_err());
    }

    #[test]
    fn get_current_branch_fails_for_non_repo() {
        let dir = TempDir::new().unwrap();
        let result = get_current_branch(dir.path());
        assert!(result.is_err());
    }

    #[test]
    fn list_branches_fails_for_non_repo() {
        let dir = TempDir::new().unwrap();
        let result = list_branches(dir.path());
        assert!(result.is_err());
    }

    #[test]
    fn get_history_fails_for_non_repo() {
        let result = get_history(Path::new("/nonexistent/xyz"), 10);
        assert!(result.is_err());
    }

    #[test]
    fn get_diff_fails_for_non_repo() {
        let dir = TempDir::new().unwrap();
        let result = get_diff(dir.path(), "file.txt");
        assert!(result.is_err());
    }
}

pub fn get_repo_diff(repo_path: &Path) -> Result<String> {
    let repo = Repository::open(repo_path)?;
    // Check if HEAD exists, if not (empty repo), return empty diff
    if repo.head().is_err() {
        return Ok(String::new());
    }
    
    let head = repo.head()?.peel_to_tree()?;
    let diff = repo.diff_tree_to_workdir_with_index(Some(&head), None)?;
    
    let mut diff_text = String::new();
    diff.print(git2::DiffFormat::Patch, |_delta, _hunk, line| {
        let content = String::from_utf8_lossy(line.content());
        diff_text.push_str(&content);
        true
    })?;
    
    Ok(diff_text)
}
