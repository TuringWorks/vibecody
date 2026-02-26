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

pub fn commit(repo_path: &Path, message: &str, files: Vec<String>) -> Result<()> {
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
    let signature = repo.signature()?;
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
    
    // Get the branch reference
    let branch_ref = repo.find_branch(branch, git2::BranchType::Local)?;
    let reference = branch_ref.get();
    
    // Set HEAD to the branch
    repo.set_head(reference.name().unwrap())?;
    
    // Checkout the branch
    repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force()))?;
    
    Ok(())
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
                worktrees.push(WorktreeInfo {
                    path,
                    branch: current_branch.clone(),
                    is_main: worktrees.is_empty(),
                });
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
