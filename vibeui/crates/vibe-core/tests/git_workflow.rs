//! Integration tests for vibe-core git operations.
//!
//! These tests exercise multiple git functions together to verify end-to-end
//! workflows rather than individual functions in isolation.

use std::process::Command;
use tempfile::TempDir;
use vibe_core::git;

// ── helpers ───────────────────────────────────────────────────────────────────

fn make_repo() -> TempDir {
    let dir = TempDir::new().unwrap();
    let p = dir.path();
    let run = |args: &[&str]| {
        Command::new("git").args(args).current_dir(p).output().unwrap();
    };
    run(&["init"]);
    run(&["config", "user.email", "it@test.com"]);
    run(&["config", "user.name", "Integration Test"]);
    std::fs::write(p.join("README.md"), "initial").unwrap();
    run(&["add", "."]);
    run(&["commit", "-m", "initial commit"]);
    dir
}

fn add_and_commit(repo: &TempDir, filename: &str, content: &str, message: &str) {
    let p = repo.path();
    std::fs::write(p.join(filename), content).unwrap();
    Command::new("git").args(["add", filename]).current_dir(p).output().unwrap();
    Command::new("git").args(["commit", "-m", message]).current_dir(p).output().unwrap();
}

// ── full stash lifecycle ──────────────────────────────────────────────────────

/// Verifies the complete checkpoint lifecycle:
/// modify → create_stash → list → verify clean → restore → verify content → drop
#[test]
fn full_stash_lifecycle() {
    let repo = make_repo();
    let path = repo.path();
    let file = path.join("README.md");

    // 1. Dirty the working tree
    std::fs::write(&file, "modified content").unwrap();
    assert_eq!(std::fs::read_to_string(&file).unwrap(), "modified content");

    // 2. Create checkpoint
    let oid = git::create_stash(path, "lifecycle-test").unwrap();
    assert!(!oid.is_empty(), "stash OID should be non-empty");

    // 3. Working tree is clean after stash
    let after_stash = std::fs::read_to_string(&file).unwrap();
    assert_eq!(after_stash, "initial", "file should be restored to HEAD after stash");

    // 4. List shows one entry
    let stashes = git::list_stashes(path).unwrap();
    assert_eq!(stashes.len(), 1);
    assert!(stashes[0].message.contains("lifecycle-test"));

    // 5. Restore brings changes back
    git::restore_stash(path, 0).unwrap();
    let after_restore = std::fs::read_to_string(&file).unwrap();
    assert_eq!(after_restore, "modified content");

    // 6. Drop cleans up
    git::drop_stash(path, 0).unwrap();
    assert!(git::list_stashes(path).unwrap().is_empty());
}

// ── status + diff workflow ────────────────────────────────────────────────────

/// Verifies that get_status and get_diff are consistent after file modification.
#[test]
fn status_and_diff_workflow() {
    let repo = make_repo();
    let path = repo.path();

    // Modify the committed file
    std::fs::write(path.join("README.md"), "updated content").unwrap();

    let status = git::get_status(path).unwrap();
    assert!(
        status.file_statuses.contains_key("README.md"),
        "README.md should appear in status"
    );
    assert_eq!(
        status.file_statuses["README.md"],
        git::FileStatus::Modified
    );

    let diff = git::get_diff(path, "README.md").unwrap();
    assert!(diff.contains("updated content"), "diff should show the new content");
    assert!(diff.contains("initial"), "diff should show removed original content");
}

// ── branch operations workflow ────────────────────────────────────────────────

/// Creates a branch, adds a commit, then switches back to the original branch.
#[test]
fn branch_operations_workflow() {
    let repo = make_repo();
    let path = repo.path();
    let initial_branch = git::get_current_branch(path).unwrap();

    // Create a second branch via git CLI (switch_branch operates on existing branches)
    Command::new("git")
        .args(["checkout", "-b", "feature-branch"])
        .current_dir(path)
        .output()
        .unwrap();

    add_and_commit(&repo, "feature.txt", "feature content", "add feature");

    // List branches — should include both
    let branches = git::list_branches(path).unwrap();
    assert!(branches.iter().any(|b| b.contains("feature-branch")));
    assert!(branches.iter().any(|b| b.contains(&initial_branch)));

    // Switch back
    git::switch_branch(path, &initial_branch).unwrap();
    let back = git::get_current_branch(path).unwrap();
    assert_eq!(back, initial_branch);

    // feature.txt should not be present on the main branch
    assert!(!path.join("feature.txt").exists(),
        "feature.txt should not exist on {initial_branch}");
}

// ── git history workflow ──────────────────────────────────────────────────────

/// Adds several commits and verifies history ordering and content.
#[test]
fn history_workflow() {
    let repo = make_repo();

    add_and_commit(&repo, "a.txt", "a", "commit a");
    add_and_commit(&repo, "b.txt", "b", "commit b");
    add_and_commit(&repo, "c.txt", "c", "commit c");

    let history = git::get_history(repo.path(), 10).unwrap();
    assert!(history.len() >= 4, "should have initial + 3 commits");

    // Most recent first
    assert!(history[0].message.contains("commit c"));
    assert!(history[1].message.contains("commit b"));
    assert!(history[2].message.contains("commit a"));

    // Limiting works
    let limited = git::get_history(repo.path(), 2).unwrap();
    assert_eq!(limited.len(), 2);
}

// ── discard changes workflow ──────────────────────────────────────────────────

/// Verifies that discard_changes reverts uncommitted modifications.
#[test]
fn discard_changes_workflow() {
    let repo = make_repo();
    let path = repo.path();
    let file = path.join("README.md");

    std::fs::write(&file, "dirty modification").unwrap();
    assert_eq!(std::fs::read_to_string(&file).unwrap(), "dirty modification");

    git::discard_changes(path, "README.md").unwrap();

    let after = std::fs::read_to_string(&file).unwrap();
    assert_eq!(after, "initial", "file should be reverted to HEAD content");
}

// ── new + deleted file statuses ───────────────────────────────────────────────

/// Verifies that untracked (New) and deleted files appear correctly in status.
#[test]
fn new_and_deleted_file_statuses() {
    let repo = make_repo();
    let path = repo.path();

    // Untracked file → New
    std::fs::write(path.join("new_file.txt"), "hello").unwrap();
    let status = git::get_status(path).unwrap();
    assert_eq!(
        status.file_statuses.get("new_file.txt").unwrap(),
        &git::FileStatus::New
    );

    // Stage + commit the new file, then delete it from disk
    Command::new("git").args(["add", "new_file.txt"]).current_dir(path).output().unwrap();
    Command::new("git").args(["commit", "-m", "add file"]).current_dir(path).output().unwrap();
    std::fs::remove_file(path.join("new_file.txt")).unwrap();

    let status2 = git::get_status(path).unwrap();
    assert_eq!(
        status2.file_statuses.get("new_file.txt").unwrap(),
        &git::FileStatus::Deleted
    );
}
