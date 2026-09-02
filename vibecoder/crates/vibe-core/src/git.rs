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
        .and_then(|h| h.shorthand().ok())
        .unwrap_or("DETACHED")
        .to_string();

    // Get file statuses
    let mut file_statuses = HashMap::new();
    let mut opts = StatusOptions::new();
    opts.include_untracked(true);
    // `include_untracked` alone reproduces `git status -unormal`: an untracked
    // directory is reported as one entry (`newdir/`) and the files inside it
    // are never listed. The Git panel showed the new folder and none of the
    // new files in it. `-uall` is what a change list has to mean.
    opts.recurse_untracked_dirs(true);

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
        let name = author_name.unwrap_or("VibeCoder User");
        let email = author_email.unwrap_or("user@vibecoder.local");
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
    // Use git CLI to leverage the user's configured SSH keys and credential helpers
    let output = vibe_no_window::std_command("git")
        .args(["push", remote, branch])
        .current_dir(repo_path)
        .output()
        .map_err(|e| anyhow::anyhow!("Failed to run git push: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!("git push failed: {}", stderr.trim()));
    }
    Ok(())
}

pub fn pull(repo_path: &Path, remote: &str, branch: &str) -> Result<()> {
    // Use git CLI to leverage the user's configured SSH keys and credential helpers
    let output = vibe_no_window::std_command("git")
        .args(["pull", remote, branch])
        .current_dir(repo_path)
        .output()
        .map_err(|e| anyhow::anyhow!("Failed to run git pull: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!("git pull failed: {}", stderr.trim()));
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

/// The file's content as it stands at `HEAD`, or `None` when `HEAD` has no
/// such file.
///
/// This exists because a unified diff is not enough to rebuild the "before"
/// side of a comparison. A patch carries only the changed hunks with a few
/// lines of context; everything between hunks is absent from it. Reconstructing
/// a file by walking the patch and keeping the `-` and context lines therefore
/// yields a few dozen lines where the real file has thousands — and diffing
/// that against the working copy reports almost the entire file as added.
/// That is what VibeCoder's git diff view did.
///
/// `None` is the honest answer for a file that is new in the working tree, and
/// for an unborn `HEAD` in a repository with no commits yet: in both cases
/// every line really is an addition. It is not used to paper over a lookup
/// that failed for some other reason.
///
/// The path is discovered rather than assumed, so a workspace opened at a
/// subdirectory of a checkout still resolves; `file_path` is relative to the
/// repository root, matching [`get_diff`].
pub fn file_at_head(repo_path: &Path, file_path: &str) -> Result<Option<String>> {
    let root = discover_repo_root(repo_path)
        .ok_or_else(|| anyhow::anyhow!("{} is not inside a git repository", repo_path.display()))?;
    let repo = Repository::open(&root)?;

    // An unborn HEAD (a repo with no commits) is not an error here — there is
    // simply no previous version of anything.
    let Ok(head) = repo.head() else {
        return Ok(None);
    };
    let tree = head.peel_to_tree()?;

    // A binary file is reported rather than lossily decoded. Returning `None`
    // would claim it is new, and `from_utf8_lossy` would fill the pane with
    // replacement characters and call it a diff.
    blob_text(&repo, &tree, file_path)
}

/// One file's text inside a tree, or `None` when the tree has no such path.
///
/// Shared by [`file_at_head`] and [`file_versions_at_commit`] so both agree on
/// what "absent" and "binary" mean.
fn blob_text(repo: &Repository, tree: &git2::Tree<'_>, file_path: &str) -> Result<Option<String>> {
    let Ok(entry) = tree.get_path(Path::new(file_path)) else {
        return Ok(None);
    };
    let blob = repo.find_blob(entry.id())?;
    match std::str::from_utf8(blob.content()) {
        Ok(text) => Ok(Some(text.to_string())),
        Err(_) => Err(anyhow::anyhow!("{file_path} is binary")),
    }
}

/// The two sides of what `rev` did to `file_path`: its content in `rev`'s first
/// parent, and its content in `rev` itself.
///
/// This is what a history entry's diff needs, and it is not what `HEAD` against
/// the working tree gives you. Showing the latter for a file listed under an
/// older commit is wrong in a way that hides: if the working tree happens to
/// match `HEAD` — the usual case while browsing history — the two sides are
/// identical and the viewer renders an empty diff, which reads as "this commit
/// changed nothing".
///
/// Either side may be `None`, and each means something specific:
/// the file was added by this commit (no parent side), deleted by it (no commit
/// side), or the commit is the repository's root and has no parent at all.
pub fn file_versions_at_commit(
    repo_path: &Path,
    rev: &str,
    file_path: &str,
) -> Result<(Option<String>, Option<String>)> {
    let root = discover_repo_root(repo_path)
        .ok_or_else(|| anyhow::anyhow!("{} is not inside a git repository", repo_path.display()))?;
    let repo = Repository::open(&root)?;
    let oid = repo.revparse_single(rev)?.id();
    let commit = repo.find_commit(oid)?;

    let after = blob_text(&repo, &commit.tree()?, file_path)?;
    let before = if commit.parent_count() > 0 {
        blob_text(&repo, &commit.parent(0)?.tree()?, file_path)?
    } else {
        // A root commit adds everything it contains.
        None
    };
    Ok((before, after))
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
        let ref_name = branch_ref
            .get()
            .name()
            .map_err(|_| anyhow::anyhow!("Branch reference has non-UTF-8 name"))?
            .to_string();
        repo.set_head(&ref_name)?;
        repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force()))?;
        return Ok(());
    }

    // Check if it's a remote branch (e.g. "origin/feature")
    if let Ok(remote_branch) = repo.find_branch(branch, git2::BranchType::Remote) {
        // Derive local name by stripping the remote prefix (e.g. "origin/feature" -> "feature")
        let local_name = branch
            .split_once('/')
            .map(|(_, name)| name)
            .unwrap_or(branch);

        // Create a local branch tracking the remote
        let commit = remote_branch.get().peel_to_commit()?;
        let mut new_branch = repo.branch(local_name, &commit, false)?;

        // Set upstream tracking
        let remote_ref = remote_branch
            .get()
            .name()
            .map_err(|_| anyhow::anyhow!("Remote branch reference has non-UTF-8 name"))?;
        new_branch.set_upstream(Some(remote_ref))?;

        let ref_name = new_branch
            .get()
            .name()
            .map_err(|_| anyhow::anyhow!("New branch reference has non-UTF-8 name"))?
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

    let diff = repo.diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), None)?;

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

/// Discard local changes to one path, tracked or not.
///
/// Two distinct cases used to collapse into the same silent no-op:
///
/// * **Tracked and modified** — restore it from HEAD. `CheckoutBuilder::path`
///   takes a *repo-relative* pathspec, but callers hand us a canonicalised
///   absolute path, which matched nothing; `checkout_head` then returned `Ok`
///   having done nothing at all.
/// * **Untracked** — it is not in HEAD, so there is nothing to restore. The
///   only way to discard it is to remove it, which is what `git clean -fd`
///   does. `checkout_head` can never do this, whatever the pathspec.
///
/// Removal is irreversible — untracked content exists nowhere else — so the
/// path is resolved inside the working tree first and an empty or `.` pathspec
/// is refused rather than treated as "the whole repository".
pub fn discard_changes(repo_path: &Path, file_path: &str) -> Result<()> {
    let repo = Repository::open(repo_path)?;
    let workdir = repo
        .workdir()
        .ok_or_else(|| anyhow::anyhow!("bare repository has no working tree"))?
        .to_path_buf();
    let rel = repo_relative_pathspec(&workdir, file_path)?;

    if is_tracked(&repo, &rel) {
        let mut checkout_builder = git2::build::CheckoutBuilder::new();
        checkout_builder.path(&rel);
        checkout_builder.force();
        checkout_builder.remove_untracked(false);
        repo.checkout_head(Some(&mut checkout_builder))?;
        return Ok(());
    }

    let target = workdir.join(&rel);
    if target.is_dir() {
        std::fs::remove_dir_all(&target)?;
    } else if target.exists() {
        std::fs::remove_file(&target)?;
    }
    Ok(())
}

/// Turn an absolute-or-relative path into a repo-relative pathspec.
///
/// Both sides are canonicalised before stripping: on macOS `/var` reports as
/// `/private/var`, so comparing an OS-reported path against a raw workdir would
/// spuriously look like an escape.
fn repo_relative_pathspec(workdir: &Path, file_path: &str) -> Result<String> {
    let raw = Path::new(file_path);
    let rel = if raw.is_absolute() {
        let canon_workdir = workdir
            .canonicalize()
            .unwrap_or_else(|_| workdir.to_path_buf());
        let canon_target = raw.canonicalize().unwrap_or_else(|_| raw.to_path_buf());
        canon_target
            .strip_prefix(&canon_workdir)
            .map_err(|_| {
                anyhow::anyhow!(
                    "'{file_path}' is outside the repository at {}",
                    workdir.display()
                )
            })?
            .to_path_buf()
    } else {
        raw.to_path_buf()
    };

    // git status reports untracked directories with a trailing slash ("vibeui/").
    let spec = rel.to_string_lossy().trim_end_matches('/').to_string();
    if spec.is_empty() || spec == "." {
        anyhow::bail!("refusing to discard the entire working tree");
    }
    Ok(spec)
}

/// Is `rel` tracked — either an index entry itself, or a directory containing one?
fn is_tracked(repo: &Repository, rel: &str) -> bool {
    let Ok(index) = repo.index() else {
        return false;
    };
    if index.get_path(Path::new(rel), 0).is_some() {
        return true;
    }
    let prefix = format!("{rel}/");
    index
        .iter()
        .any(|entry| String::from_utf8_lossy(&entry.path).starts_with(&prefix))
}

pub fn is_git_repo(path: &Path) -> bool {
    Repository::open(path).is_ok()
}

/// The working root of the repository *containing* `path`, or `None` when
/// `path` is not inside one.
///
/// This is deliberately not `is_git_repo`. That helper calls
/// `Repository::open`, which only succeeds when `path` is *itself* the repo
/// root, so it answers "no repo" for `src/` inside a perfectly ordinary
/// checkout — the same trap already documented at `serve.rs::resolve_repo_root`
/// after it left the Environment inspector and Review diff blank. Anything
/// deciding whether a file is under version control must walk up, because that
/// is what every other git tool does.
///
/// A file path is resolved from its parent directory, so callers can pass the
/// path they just wrote without pre-trimming it. Linked worktrees resolve to
/// their own root (git2's `workdir`), matching `rev-parse --show-toplevel`.
/// Bare repositories have no working tree and yield `None`.
pub fn discover_repo_root(path: &Path) -> Option<std::path::PathBuf> {
    let start = if path.is_file() { path.parent()? } else { path };
    let repo = Repository::discover(start).ok()?;
    repo.workdir().map(|w| w.to_path_buf())
}

/// Whether `path` lives inside a git working tree.
pub fn is_inside_repo(path: &Path) -> bool {
    discover_repo_root(path).is_some()
}

/// The user's home directory, canonicalised so it compares equal to a path
/// that reached us through a symlink (`/var` → `/private/var` on macOS).
fn home_dir() -> Option<std::path::PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .filter(|h| !h.is_empty())
        .map(std::path::PathBuf::from)
        .and_then(|h| h.canonicalize().ok())
}

/// Why `dir` is a bad place to create a repository, or `None` if it is fine.
///
/// Shared by `init_repo` and by the "should we suggest this?" decision, so a
/// location we would refuse to initialise is never one we offer to initialise.
/// Says nothing about whether `dir` is already inside a repo — that is a
/// separate question with a separate answer for each caller.
pub fn repo_location_objection(dir: &Path) -> Option<String> {
    if !dir.is_dir() {
        return Some(format!("{} is not a directory", dir.display()));
    }
    let canonical = match dir.canonicalize() {
        Ok(c) => c,
        Err(e) => return Some(format!("cannot resolve {}: {e}", dir.display())),
    };
    if canonical.parent().is_none() {
        return Some("refusing to create a git repository at the filesystem root".to_string());
    }
    if home_dir().is_some_and(|home| home == canonical) {
        return Some(
            "refusing to create a git repository over your entire home directory".to_string(),
        );
    }
    None
}

/// Create a new repository at `dir` with `main` as the initial branch.
///
/// Refuses paths that would put a working tree around a user's entire home
/// directory or filesystem root — an accidental `git init` there is tedious to
/// notice and unpleasant to undo. Refuses as well when `dir` is already inside
/// a repository, since a nested checkout shadows the outer one rather than
/// adding anything.
pub fn init_repo(dir: &Path) -> Result<std::path::PathBuf> {
    if let Some(objection) = repo_location_objection(dir) {
        anyhow::bail!(objection);
    }
    let canonical = dir.canonicalize()?;

    if let Some(existing) = discover_repo_root(&canonical) {
        anyhow::bail!(
            "{} is already inside the git repository at {}",
            canonical.display(),
            existing.display()
        );
    }

    let mut opts = git2::RepositoryInitOptions::new();
    opts.initial_head("main");
    let repo = Repository::init_opts(&canonical, &opts)?;
    repo.workdir()
        .map(|w| w.to_path_buf())
        .ok_or_else(|| anyhow::anyhow!("initialized repository has no working tree"))
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
    let message = format!("vibecoder-pre-ai: {}", name);
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
    let output = vibe_no_window::std_command("git")
        .args([
            "worktree",
            "add",
            "-b",
            branch,
            &worktree_path.to_string_lossy(),
        ])
        .current_dir(repo_path)
        .output()?;
    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("git worktree add failed: {}", err);
    }
    Ok(())
}

/// Create a worktree for an **existing** branch (no new branch created). Used by
/// the restore path to re-materialize a worktree from a kept/preserved branch.
///
/// Equivalent to: `git worktree add <worktree_path> <branch>`
pub fn add_worktree_existing_branch(
    repo_path: &Path,
    branch: &str,
    worktree_path: &Path,
) -> Result<()> {
    let output = vibe_no_window::std_command("git")
        .args(["worktree", "add", &worktree_path.to_string_lossy(), branch])
        .current_dir(repo_path)
        .output()?;
    if !output.status.success() {
        anyhow::bail!(
            "git worktree add (existing branch) failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

/// Create a branch pointing at an arbitrary ref (e.g. recover a deleted task
/// branch from its preserved `refs/trash/<id>`). No-op if the branch exists.
///
/// Equivalent to: `git branch <branch> <source_ref>`
pub fn create_branch_from_ref(repo_path: &Path, branch: &str, source_ref: &str) -> Result<()> {
    if branch_exists(repo_path, branch) {
        return Ok(());
    }
    let output = vibe_no_window::std_command("git")
        .args(["branch", branch, source_ref])
        .current_dir(repo_path)
        .output()?;
    if !output.status.success() {
        anyhow::bail!(
            "git branch {branch} {source_ref} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

/// Whether a ref (any namespace, e.g. `refs/trash/<id>`) exists.
pub fn ref_exists(repo_path: &Path, full_ref: &str) -> bool {
    vibe_no_window::std_command("git")
        .args(["show-ref", "--verify", "--quiet", full_ref])
        .current_dir(repo_path)
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Remove a git worktree at `worktree_path`.
///
/// Equivalent to: `git worktree remove --force <worktree_path>`
pub fn remove_worktree(repo_path: &Path, worktree_path: &Path) -> Result<()> {
    let output = vibe_no_window::std_command("git")
        .args([
            "worktree",
            "remove",
            "--force",
            &worktree_path.to_string_lossy(),
        ])
        .current_dir(repo_path)
        .output()?;
    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("git worktree remove failed: {}", err);
    }
    Ok(())
}

/// Whether `branch` is fully merged into the repo's current HEAD — i.e. its tip
/// is an ancestor of HEAD, so it holds no unique commits and is safe to delete.
///
/// Equivalent to: `git merge-base --is-ancestor refs/heads/<branch> HEAD`
/// (exit 0 = merged, exit 1 = not merged). Used by the worktree reaper to decide
/// whether a branch can be deleted or must be preserved.
pub fn is_branch_merged(repo_path: &Path, branch: &str) -> Result<bool> {
    let status = vibe_no_window::std_command("git")
        .args([
            "merge-base",
            "--is-ancestor",
            &format!("refs/heads/{branch}"),
            "HEAD",
        ])
        .current_dir(repo_path)
        .status()?;
    Ok(status.success())
}

/// Whether a local branch ref exists.
pub fn branch_exists(repo_path: &Path, branch: &str) -> bool {
    vibe_no_window::std_command("git")
        .args([
            "show-ref",
            "--verify",
            "--quiet",
            &format!("refs/heads/{branch}"),
        ])
        .current_dir(repo_path)
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Commit any uncommitted changes in a worktree onto its branch, so removing the
/// worktree directory can never discard work. Returns `Ok(true)` if a commit was
/// made, `Ok(false)` if the tree was already clean (nothing to commit).
///
/// Runs `git -C <worktree> add -A` then `git -C <worktree> commit -m <message>`.
pub fn commit_all_in_worktree(worktree_path: &Path, message: &str) -> Result<bool> {
    // Fast path: clean tree → nothing to do.
    let status = vibe_no_window::std_command("git")
        .args(["status", "--porcelain"])
        .current_dir(worktree_path)
        .output()?;
    if status.stdout.is_empty() {
        return Ok(false);
    }
    let add = vibe_no_window::std_command("git")
        .args(["add", "-A"])
        .current_dir(worktree_path)
        .output()?;
    if !add.status.success() {
        anyhow::bail!(
            "git add -A failed: {}",
            String::from_utf8_lossy(&add.stderr)
        );
    }
    let commit = vibe_no_window::std_command("git")
        .args(["commit", "--no-verify", "-m", message])
        .current_dir(worktree_path)
        .output()?;
    if !commit.status.success() {
        anyhow::bail!(
            "git commit failed: {}",
            String::from_utf8_lossy(&commit.stderr)
        );
    }
    Ok(true)
}

/// Preserve a branch's tip under `refs/trash/<id>` before the branch itself is
/// deleted, so unmerged work stays recoverable (hidden from `git branch`, but
/// reachable via `git update-ref` / reflog). No-op if the branch doesn't exist.
///
/// Runs `git update-ref refs/trash/<id> refs/heads/<branch>`.
pub fn preserve_branch_ref(repo_path: &Path, branch: &str, id: &str) -> Result<()> {
    if !branch_exists(repo_path, branch) {
        return Ok(());
    }
    let output = vibe_no_window::std_command("git")
        .args([
            "update-ref",
            &format!("refs/trash/{id}"),
            &format!("refs/heads/{branch}"),
        ])
        .current_dir(repo_path)
        .output()?;
    if !output.status.success() {
        anyhow::bail!(
            "git update-ref refs/trash/{id} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

/// Delete a local branch. `force` selects `-D` (unconditional) over `-d` (only
/// if merged). No-op (Ok) if the branch is already gone.
pub fn delete_branch(repo_path: &Path, branch: &str, force: bool) -> Result<()> {
    if !branch_exists(repo_path, branch) {
        return Ok(());
    }
    let flag = if force { "-D" } else { "-d" };
    let output = vibe_no_window::std_command("git")
        .args(["branch", flag, branch])
        .current_dir(repo_path)
        .output()?;
    if !output.status.success() {
        anyhow::bail!(
            "git branch {flag} {branch} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

/// Prune stale administrative worktree metadata (`.git/worktrees/*`) for
/// directories that no longer exist on disk.
///
/// Equivalent to: `git worktree prune`
pub fn prune_worktrees(repo_path: &Path) -> Result<()> {
    let output = vibe_no_window::std_command("git")
        .args(["worktree", "prune"])
        .current_dir(repo_path)
        .output()?;
    if !output.status.success() {
        anyhow::bail!(
            "git worktree prune failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

/// List all worktrees for the repository.
pub fn list_worktrees(repo_path: &Path) -> Result<Vec<WorktreeInfo>> {
    let output = vibe_no_window::std_command("git")
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
            current_path = Some(std::path::PathBuf::from(
                line.trim_start_matches("worktree "),
            ));
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
    let output = vibe_no_window::std_command("git")
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

    // ── get_status ────────────────────────────────────────────────────────────

    /// Regression: the Git panel listed a brand-new folder but none of the
    /// files inside it. libgit2 defaults to `git status -unormal`, which
    /// collapses an untracked directory into a single `dir/` entry, so every
    /// file created under a new folder was invisible to the change list.
    #[test]
    fn status_lists_files_inside_an_untracked_directory() {
        let dir = make_git_repo();
        let nested = dir.path().join("newdir").join("deeper");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::write(nested.join("added.rs"), "fn main() {}").unwrap();
        std::fs::write(dir.path().join("newdir").join("top.rs"), "// top").unwrap();

        let status = get_status(dir.path()).expect("status");
        let paths: Vec<&str> = status.file_statuses.keys().map(String::as_str).collect();

        assert!(
            paths.contains(&"newdir/deeper/added.rs"),
            "nested untracked file missing from {paths:?}"
        );
        assert!(
            paths.contains(&"newdir/top.rs"),
            "untracked file missing from {paths:?}"
        );
        assert_eq!(
            status.file_statuses.get("newdir/top.rs"),
            Some(&FileStatus::New)
        );
        // The collapsed directory entry must not be reported alongside them.
        assert!(
            !paths.contains(&"newdir/"),
            "collapsed dir entry in {paths:?}"
        );
    }

    /// A `.gitignore`d path stays out of the change list even though untracked
    /// directories are now walked — recursion must not defeat the ignore file.
    #[test]
    fn status_still_honours_gitignore_when_recursing() {
        let dir = make_git_repo();
        std::fs::write(dir.path().join(".gitignore"), "build/\n").unwrap();
        std::fs::create_dir_all(dir.path().join("build")).unwrap();
        std::fs::write(dir.path().join("build").join("out.o"), "bin").unwrap();

        let status = get_status(dir.path()).expect("status");
        let paths: Vec<&str> = status.file_statuses.keys().map(String::as_str).collect();

        assert!(
            !paths.iter().any(|p| p.starts_with("build/")),
            "ignored file leaked into {paths:?}"
        );
    }

    // ── discard_changes ───────────────────────────────────────────────────────

    /// Regression: callers pass a canonicalised *absolute* path, but a
    /// CheckoutBuilder pathspec is repo-relative — so it matched nothing and
    /// checkout_head reported success having restored nothing.
    #[test]
    fn discard_restores_a_tracked_file_given_an_absolute_path() {
        let dir = make_git_repo();
        let file = dir.path().join("README.md");
        std::fs::write(&file, "locally modified").unwrap();

        discard_changes(dir.path(), &file.to_string_lossy()).unwrap();

        assert_eq!(std::fs::read_to_string(&file).unwrap(), "init");
    }

    #[test]
    fn discard_restores_a_tracked_file_given_a_relative_path() {
        let dir = make_git_repo();
        let file = dir.path().join("README.md");
        std::fs::write(&file, "locally modified").unwrap();

        discard_changes(dir.path(), "README.md").unwrap();

        assert_eq!(std::fs::read_to_string(&file).unwrap(), "init");
    }

    #[test]
    fn discard_restores_a_deleted_tracked_file() {
        let dir = make_git_repo();
        let file = dir.path().join("README.md");
        std::fs::remove_file(&file).unwrap();

        discard_changes(dir.path(), "README.md").unwrap();

        assert_eq!(std::fs::read_to_string(&file).unwrap(), "init");
    }

    /// Regression: an untracked file is not in HEAD, so checkout_head could
    /// never discard it — the UI reported success while the entry stayed put.
    #[test]
    fn discard_removes_an_untracked_file() {
        let dir = make_git_repo();
        let stray = dir.path().join("stray.txt");
        std::fs::write(&stray, "not committed").unwrap();

        discard_changes(dir.path(), "stray.txt").unwrap();

        assert!(!stray.exists(), "untracked file survived discard");
    }

    /// The reported case: git status names untracked directories "vibeui/".
    #[test]
    fn discard_removes_an_untracked_directory_with_a_trailing_slash() {
        let dir = make_git_repo();
        let nested = dir.path().join("vibeui/node_modules/pkg");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::write(nested.join("index.js"), "x").unwrap();

        discard_changes(dir.path(), "vibeui/").unwrap();

        assert!(
            !dir.path().join("vibeui").exists(),
            "untracked directory survived discard"
        );
    }

    #[test]
    fn discard_leaves_tracked_siblings_alone() {
        let dir = make_git_repo();
        let stray = dir.path().join("stray.txt");
        std::fs::write(&stray, "not committed").unwrap();

        discard_changes(dir.path(), "stray.txt").unwrap();

        assert!(!stray.exists());
        assert_eq!(
            std::fs::read_to_string(dir.path().join("README.md")).unwrap(),
            "init",
            "discarding an untracked path must not touch tracked files"
        );
    }

    /// Removal is irreversible, so an empty or "." pathspec must never be read
    /// as "everything".
    #[test]
    fn discard_refuses_to_wipe_the_whole_working_tree() {
        let dir = make_git_repo();

        for spec in ["", ".", "/"] {
            assert!(
                discard_changes(dir.path(), spec).is_err(),
                "pathspec {spec:?} should be refused"
            );
        }
        assert!(dir.path().join("README.md").exists());
    }

    #[test]
    fn discard_refuses_a_path_outside_the_repository() {
        let dir = make_git_repo();
        let outside = TempDir::new().unwrap();
        let victim = outside.path().join("keep.txt");
        std::fs::write(&victim, "important").unwrap();

        assert!(discard_changes(dir.path(), &victim.to_string_lossy()).is_err());
        assert!(victim.exists(), "file outside the repo must not be removed");
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
        assert!(
            branch == "main" || branch == "master",
            "unexpected branch: {branch}"
        );
    }

    // ── get_status ────────────────────────────────────────────────────────────

    #[test]
    fn get_status_clean_repo_has_no_modified_files() {
        let dir = make_git_repo();
        let status = get_status(dir.path()).unwrap();
        let modified: Vec<_> = status
            .file_statuses
            .values()
            .filter(|s| **s == FileStatus::Modified)
            .collect();
        assert!(
            modified.is_empty(),
            "clean repo should have no modified files"
        );
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
            status
                .file_statuses
                .values()
                .any(|s| *s == FileStatus::Modified),
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
        assert!(
            list_stashes(dir.path()).unwrap().is_empty(),
            "stash should be gone after drop"
        );
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
        assert!(
            result.is_err(),
            "dropping nonexistent stash index should fail"
        );
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
            Command::new("git")
                .args(args)
                .current_dir(dir.path())
                .output()
                .unwrap();
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
            Command::new("git")
                .args(args)
                .current_dir(dir.path())
                .output()
                .unwrap();
        };
        run(&["add", "new.txt"]);

        commit(
            dir.path(),
            "add new file",
            vec!["new.txt".to_string()],
            None,
            None,
        )
        .unwrap();

        let history = get_history(dir.path(), 1).unwrap();
        assert!(history[0].message.contains("add new file"));
    }

    // ── switch_branch ──────────────────────────────────────────────────────

    #[test]
    fn switch_branch_changes_head() {
        let dir = make_git_repo();
        // Create a new branch
        let run = |args: &[&str]| {
            Command::new("git")
                .args(args)
                .current_dir(dir.path())
                .output()
                .unwrap();
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
        assert_eq!(
            status.file_statuses.get("src/lib.rs"),
            Some(&FileStatus::Modified)
        );
        assert_eq!(
            status.file_statuses.get("new_file.txt"),
            Some(&FileStatus::New)
        );
        assert_eq!(
            status.file_statuses.get("old.rs"),
            Some(&FileStatus::Deleted)
        );
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

    // ── Repo discovery + init ────────────────────────────────────────────────
    //
    // TempDir hands back /var/... on macOS, which is a symlink to /private/var,
    // and git2 reports the resolved form — so every comparison here canonicalises
    // both sides rather than trusting the path we were given.

    #[test]
    fn discover_repo_root_finds_the_enclosing_repo_from_a_subdirectory() {
        let dir = TempDir::new().unwrap();
        let root = init_repo(dir.path()).unwrap();
        let nested = dir.path().join("src").join("deeply").join("nested");
        std::fs::create_dir_all(&nested).unwrap();

        // The regression this whole helper exists for: `is_git_repo` answers
        // "no" here, which would make us offer to create a repo inside a repo.
        assert!(!is_git_repo(&nested));
        assert!(is_inside_repo(&nested));
        assert_eq!(
            discover_repo_root(&nested).unwrap().canonicalize().unwrap(),
            root.canonicalize().unwrap()
        );
    }

    #[test]
    fn discover_repo_root_resolves_a_file_path_via_its_parent() {
        let dir = TempDir::new().unwrap();
        let root = init_repo(dir.path()).unwrap();
        let sub = dir.path().join("src");
        std::fs::create_dir_all(&sub).unwrap();
        let file = sub.join("main.rs");
        std::fs::write(&file, "fn main() {}").unwrap();

        assert_eq!(
            discover_repo_root(&file).unwrap().canonicalize().unwrap(),
            root.canonicalize().unwrap()
        );
    }

    #[test]
    fn discover_repo_root_is_none_outside_any_repo() {
        // A TempDir can sit under a directory that is itself in a repo on a
        // developer machine, so assert against a path we know is not: discovery
        // from the filesystem root has nowhere above it to find a .git.
        assert!(discover_repo_root(Path::new("/")).is_none());
    }

    #[test]
    fn discover_repo_root_is_none_for_a_nonexistent_path() {
        assert!(discover_repo_root(Path::new("/nonexistent/xyz/abc")).is_none());
    }

    /// What `git rev-parse --show-toplevel` reports for `dir`, or None.
    fn cli_toplevel(dir: &Path) -> Option<std::path::PathBuf> {
        let out = std::process::Command::new("git")
            .arg("-C")
            .arg(dir)
            .args(["rev-parse", "--show-toplevel"])
            .output()
            .ok()?;
        if !out.status.success() {
            return None;
        }
        let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
        (!s.is_empty()).then(|| std::path::PathBuf::from(s))
    }

    /// Pins the contract the hand-rolled `rev-parse --show-toplevel` callers
    /// relied on before they were consolidated onto `discover_repo_root`:
    /// the two must agree, including for a linked worktree, which resolves to
    /// its own root rather than the main checkout's.
    #[test]
    fn discover_repo_root_agrees_with_rev_parse_show_toplevel() {
        let dir = TempDir::new().unwrap();
        init_repo(dir.path()).unwrap();
        let nested = dir.path().join("src").join("inner");
        std::fs::create_dir_all(&nested).unwrap();

        for probe in [dir.path(), nested.as_path()] {
            let ours = discover_repo_root(probe).map(|p| p.canonicalize().unwrap());
            let theirs = cli_toplevel(probe).map(|p| p.canonicalize().unwrap());
            assert_eq!(ours, theirs, "disagreed about {}", probe.display());
        }

        // Outside any repo both must decline. "/" has no enclosing repo and is
        // not inside a TempDir that might itself sit in one.
        assert_eq!(discover_repo_root(Path::new("/")), None);
        assert_eq!(cli_toplevel(Path::new("/")), None);
    }

    #[test]
    fn discover_repo_root_resolves_a_linked_worktree_to_its_own_root() {
        let dir = TempDir::new().unwrap();
        init_repo(dir.path()).unwrap();
        // A worktree needs a commit to branch from.
        let file = dir.path().join("seed.txt");
        std::fs::write(&file, "seed").unwrap();
        let git = |args: &[&str]| {
            std::process::Command::new("git")
                .arg("-C")
                .arg(dir.path())
                .args(args)
                .output()
                .expect("git runs in tests")
        };
        git(&["config", "user.email", "t@example.com"]);
        git(&["config", "user.name", "t"]);
        git(&["add", "-A"]);
        git(&["commit", "-m", "seed", "--no-verify"]);

        let wt = dir.path().join("wt");
        let out = git(&["worktree", "add", wt.to_str().unwrap(), "-b", "side"]);
        assert!(out.status.success(), "worktree add failed");

        let ours = discover_repo_root(&wt).map(|p| p.canonicalize().unwrap());
        assert_eq!(
            ours,
            cli_toplevel(&wt).map(|p| p.canonicalize().unwrap()),
            "a linked worktree must resolve to its own root, not the main checkout"
        );
        assert_eq!(ours, Some(wt.canonicalize().unwrap()));
    }

    #[test]
    fn init_repo_creates_a_working_tree_on_main() {
        let dir = TempDir::new().unwrap();
        let root = init_repo(dir.path()).unwrap();

        assert!(dir.path().join(".git").exists());
        assert_eq!(
            root.canonicalize().unwrap(),
            dir.path().canonicalize().unwrap()
        );
        // A fresh repo has no commits, so HEAD is unborn and `get_current_branch`
        // cannot resolve it; read the symbolic target instead.
        let repo = Repository::open(dir.path()).unwrap();
        let head_ref = repo.find_reference("HEAD").unwrap();
        assert_eq!(head_ref.symbolic_target(), Ok(Some("refs/heads/main")));
    }

    #[test]
    fn init_repo_refuses_a_directory_already_inside_a_repo() {
        let dir = TempDir::new().unwrap();
        init_repo(dir.path()).unwrap();
        let nested = dir.path().join("packages").join("app");
        std::fs::create_dir_all(&nested).unwrap();

        let err = init_repo(&nested).unwrap_err().to_string();
        assert!(
            err.contains("already inside the git repository"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn init_repo_refuses_a_path_that_is_not_a_directory() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("notes.txt");
        std::fs::write(&file, "hi").unwrap();

        let err = init_repo(&file).unwrap_err().to_string();
        assert!(err.contains("not a directory"), "unexpected error: {err}");
    }

    #[test]
    fn init_repo_refuses_the_filesystem_root() {
        let err = init_repo(Path::new("/")).unwrap_err().to_string();
        assert!(err.contains("filesystem root"), "unexpected error: {err}");
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

#[cfg(test)]
mod file_at_head_tests {
    use super::*;
    use std::fs;

    /// A repo with one commit containing `path`, plus a modified working copy.
    fn repo_with(path: &str, committed: &str) -> (tempfile::TempDir, std::path::PathBuf) {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path().to_path_buf();
        let repo = Repository::init(&root).expect("init");
        let file = root.join(path);
        if let Some(parent) = file.parent() {
            fs::create_dir_all(parent).expect("mkdir");
        }
        fs::write(&file, committed).expect("write");

        let mut index = repo.index().expect("index");
        index.add_path(Path::new(path)).expect("add");
        index.write().expect("index write");
        let tree_id = index.write_tree().expect("write_tree");
        let tree = repo.find_tree(tree_id).expect("find_tree");
        let sig = git2::Signature::now("t", "t@example.com").expect("sig");
        repo.commit(Some("HEAD"), &sig, &sig, "init", &tree, &[])
            .expect("commit");
        drop(tree);
        (dir, root)
    }

    /// The regression. A unified diff carries only the changed hunks, so a
    /// "before" side rebuilt from one is a few lines where the file is many —
    /// and the comparison then reports nearly every line as added. The whole
    /// file has to come from the object store.
    #[test]
    fn returns_the_whole_committed_file_not_just_the_changed_part() {
        let body: String = (1..=500).map(|i| format!("line {i}\n")).collect();
        let (_dir, root) = repo_with("src/cli.ts", &body);
        // One line edited in the working tree.
        fs::write(
            root.join("src/cli.ts"),
            body.replace("line 250\n", "line 250 edited\n"),
        )
        .expect("write");

        let head = file_at_head(&root, "src/cli.ts")
            .expect("ok")
            .expect("tracked");
        assert_eq!(head, body);
        assert_eq!(head.lines().count(), 500);
    }

    /// A file that is new in the working tree has no previous version, and
    /// saying so is what makes "every line is an addition" correct rather than
    /// a bug.
    #[test]
    fn an_untracked_file_has_no_head_version() {
        let (_dir, root) = repo_with("a.txt", "a\n");
        fs::write(root.join("new.txt"), "hello\n").expect("write");
        assert_eq!(file_at_head(&root, "new.txt").expect("ok"), None);
    }

    /// Opened at a subdirectory, the repository is still discovered — the
    /// mistake `discover_repo_root` exists to prevent.
    #[test]
    fn resolves_from_a_subdirectory_of_the_checkout() {
        let (_dir, root) = repo_with("src/cli.ts", "x\n");
        let head = file_at_head(&root.join("src"), "src/cli.ts").expect("ok");
        assert_eq!(head.as_deref(), Some("x\n"));
    }

    /// A repository with no commits yet: nothing has a previous version.
    #[test]
    fn an_unborn_head_is_not_an_error() {
        let dir = tempfile::tempdir().expect("tempdir");
        Repository::init(dir.path()).expect("init");
        fs::write(dir.path().join("a.txt"), "a\n").expect("write");
        assert_eq!(file_at_head(dir.path(), "a.txt").expect("ok"), None);
    }

    /// Reported, not lossily decoded: `None` would claim the file is new, and
    /// a lossy decode would fill the pane with replacement characters.
    #[test]
    fn a_binary_file_is_an_error_rather_than_a_wrong_answer() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path().to_path_buf();
        let repo = Repository::init(&root).expect("init");
        fs::write(root.join("bin"), [0xff_u8, 0xfe, 0x00, 0x01]).expect("write");
        let mut index = repo.index().expect("index");
        index.add_path(Path::new("bin")).expect("add");
        index.write().expect("index write");
        let tree_id = index.write_tree().expect("write_tree");
        let tree = repo.find_tree(tree_id).expect("tree");
        let sig = git2::Signature::now("t", "t@example.com").expect("sig");
        repo.commit(Some("HEAD"), &sig, &sig, "init", &tree, &[])
            .expect("commit");
        drop(tree);

        assert!(file_at_head(&root, "bin").is_err());
    }

    #[test]
    fn a_path_outside_any_repository_is_an_error() {
        let dir = tempfile::tempdir().expect("tempdir");
        assert!(file_at_head(dir.path(), "a.txt").is_err());
    }
}

#[cfg(test)]
mod file_versions_at_commit_tests {
    use super::*;
    use std::fs;

    /// Commit `files` on top of whatever is there, returning the new hash.
    fn commit_all(root: &Path, files: &[(&str, &str)], msg: &str) -> String {
        let repo = Repository::open(root).expect("open");
        for (path, body) in files {
            let full = root.join(path);
            if let Some(parent) = full.parent() {
                fs::create_dir_all(parent).expect("mkdir");
            }
            fs::write(&full, body).expect("write");
        }
        let mut index = repo.index().expect("index");
        index
            .add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)
            .expect("add_all");
        index.write().expect("index write");
        let tree_id = index.write_tree().expect("write_tree");
        let tree = repo.find_tree(tree_id).expect("tree");
        let sig = git2::Signature::now("t", "t@example.com").expect("sig");
        let parents: Vec<git2::Commit<'_>> =
            match repo.head().ok().and_then(|h| h.peel_to_commit().ok()) {
                Some(c) => vec![c],
                None => vec![],
            };
        let parent_refs: Vec<&git2::Commit<'_>> = parents.iter().collect();
        let oid = repo
            .commit(Some("HEAD"), &sig, &sig, msg, &tree, &parent_refs)
            .expect("commit");
        oid.to_string()
    }

    fn repo() -> (tempfile::TempDir, std::path::PathBuf) {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path().to_path_buf();
        Repository::init(&root).expect("init");
        (dir, root)
    }

    /// The regression this exists for. Browsing history, the working tree
    /// usually matches HEAD — so comparing HEAD against the working tree gives
    /// two identical sides and the viewer shows an empty diff, as though the
    /// commit changed nothing. The two sides must come from the commit and its
    /// parent.
    #[test]
    fn gives_the_commit_and_its_parent_not_head() {
        let (_dir, root) = repo();
        commit_all(&root, &[("a.txt", "one\n")], "first");
        let second = commit_all(&root, &[("a.txt", "two\n")], "second");
        // A third commit, so HEAD is past the one being inspected.
        commit_all(&root, &[("a.txt", "three\n")], "third");

        let (before, after) = file_versions_at_commit(&root, &second, "a.txt").expect("ok");
        assert_eq!(before.as_deref(), Some("one\n"));
        assert_eq!(after.as_deref(), Some("two\n"));
    }

    #[test]
    fn a_file_added_by_the_commit_has_no_parent_side() {
        let (_dir, root) = repo();
        commit_all(&root, &[("a.txt", "a\n")], "first");
        let second = commit_all(&root, &[("b.txt", "b\n")], "adds b");

        let (before, after) = file_versions_at_commit(&root, &second, "b.txt").expect("ok");
        assert_eq!(before, None);
        assert_eq!(after.as_deref(), Some("b\n"));
    }

    #[test]
    fn a_file_deleted_by_the_commit_has_no_commit_side() {
        let (_dir, root) = repo();
        commit_all(&root, &[("a.txt", "a\n"), ("b.txt", "b\n")], "first");
        fs::remove_file(root.join("b.txt")).expect("rm");
        let second = commit_all(&root, &[], "deletes b");

        let (before, after) = file_versions_at_commit(&root, &second, "b.txt").expect("ok");
        assert_eq!(before.as_deref(), Some("b\n"));
        assert_eq!(after, None);
    }

    /// A root commit adds everything it contains — there is no parent to read.
    #[test]
    fn a_root_commit_has_no_parent_side() {
        let (_dir, root) = repo();
        let first = commit_all(&root, &[("a.txt", "a\n")], "first");

        let (before, after) = file_versions_at_commit(&root, &first, "a.txt").expect("ok");
        assert_eq!(before, None);
        assert_eq!(after.as_deref(), Some("a\n"));
    }

    /// A file untouched by the commit reads the same on both sides, which is
    /// the honest answer — the viewer then shows no change, correctly.
    #[test]
    fn a_file_the_commit_did_not_touch_is_identical_on_both_sides() {
        let (_dir, root) = repo();
        commit_all(&root, &[("a.txt", "a\n"), ("b.txt", "b\n")], "first");
        let second = commit_all(&root, &[("a.txt", "a2\n")], "touches a only");

        let (before, after) = file_versions_at_commit(&root, &second, "b.txt").expect("ok");
        assert_eq!(before.as_deref(), Some("b\n"));
        assert_eq!(after.as_deref(), Some("b\n"));
    }

    #[test]
    fn an_unknown_revision_is_an_error() {
        let (_dir, root) = repo();
        commit_all(&root, &[("a.txt", "a\n")], "first");
        assert!(file_versions_at_commit(&root, "nosuchrev", "a.txt").is_err());
    }
}
