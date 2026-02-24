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
