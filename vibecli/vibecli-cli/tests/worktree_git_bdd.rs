//! BDD coverage for GitWorktreePool against real git repos (US-003).

use cucumber::{World, given, then, when};
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;
use vibecli_cli::worktree_git::{GitWorktreePool, MergeOutcome};

#[derive(Default, World)]
pub struct GitWorktreeWorld {
    repo: Option<TempDir>,
    base_dir: Option<TempDir>,
    pool: Option<GitWorktreePool>,
    last_merge: Option<MergeOutcome>,
    last_spawn_error: Option<String>,
}

impl std::fmt::Debug for GitWorktreeWorld {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GitWorktreeWorld")
            .field("repo_path", &self.repo.as_ref().map(|d| d.path().to_owned()))
            .field("base_dir", &self.base_dir.as_ref().map(|d| d.path().to_owned()))
            .field("last_merge", &self.last_merge)
            .field("last_spawn_error", &self.last_spawn_error)
            .finish()
    }
}

fn run_git(cwd: &PathBuf, args: &[&str]) {
    let out = Command::new("git")
        .current_dir(cwd)
        .args(args)
        .output()
        .expect("spawn git");
    assert!(
        out.status.success(),
        "git {:?} failed: {}",
        args,
        String::from_utf8_lossy(&out.stderr)
    );
}

fn repo_path(w: &GitWorktreeWorld) -> PathBuf {
    w.repo.as_ref().expect("repo").path().to_path_buf()
}

fn base_path(w: &GitWorktreeWorld) -> PathBuf {
    w.base_dir.as_ref().expect("base_dir").path().to_path_buf()
}

// ── Given ───────────────────────────────────────────────────────────────────

#[given(regex = r#"^a fresh git repo with a single commit on branch "([^"]+)"$"#)]
fn given_fresh_repo(w: &mut GitWorktreeWorld, branch: String) {
    let repo = tempfile::tempdir().expect("tempdir");
    let base = tempfile::tempdir().expect("base");
    let repo_path = repo.path().to_path_buf();
    run_git(&repo_path, &["init", "-b", &branch]);
    run_git(&repo_path, &["config", "user.name", "bdd"]);
    run_git(&repo_path, &["config", "user.email", "bdd@test"]);
    run_git(&repo_path, &["config", "commit.gpgsign", "false"]);
    run_git(&repo_path, &["config", "tag.gpgsign", "false"]);
    std::fs::write(repo_path.join("hello.txt"), "orig\n").expect("write");
    run_git(&repo_path, &["add", "hello.txt"]);
    run_git(&repo_path, &["commit", "-m", "init"]);
    w.pool = Some(GitWorktreePool::new(
        &repo_path,
        base.path(),
        4,
    ));
    w.repo = Some(repo);
    w.base_dir = Some(base);
}

#[given(regex = r#"^the pool has a max capacity of (\d+)$"#)]
fn given_max_capacity(w: &mut GitWorktreeWorld, max: usize) {
    let repo_p = repo_path(w);
    let base_p = base_path(w);
    w.pool = Some(GitWorktreePool::new(&repo_p, &base_p, max));
}

#[given(regex = r#"^the pool has spawned a worktree "([^"]+)" on branch "([^"]+)"$"#)]
fn given_spawned(w: &mut GitWorktreeWorld, id: String, branch: String) {
    let pool = w.pool.as_mut().expect("pool");
    pool.spawn(&id, &branch).expect("spawn");
}

#[given(regex = r#"^a new file "([^"]+)" with content "([^"]+)" is committed in worktree "([^"]+)"$"#)]
fn given_new_file(w: &mut GitWorktreeWorld, file: String, content: String, id: String) {
    let pool = w.pool.as_ref().expect("pool");
    let handle = pool.get(&id).expect("handle");
    let wt_path = handle.path.clone();
    std::fs::write(wt_path.join(&file), format!("{content}\n")).expect("write");
    run_git(&wt_path, &["add", &file]);
    run_git(&wt_path, &["commit", "-m", &format!("add {file}")]);
}

#[given(regex = r#"^file "([^"]+)" is modified to "([^"]+)" and committed in worktree "([^"]+)"$"#)]
fn given_modified_in_wt(w: &mut GitWorktreeWorld, file: String, content: String, id: String) {
    let pool = w.pool.as_ref().expect("pool");
    let handle = pool.get(&id).expect("handle");
    let wt_path = handle.path.clone();
    std::fs::write(wt_path.join(&file), format!("{content}\n")).expect("write");
    run_git(&wt_path, &["add", &file]);
    run_git(&wt_path, &["commit", "-m", &format!("modify {file} in wt")]);
}

#[given(regex = r#"^file "([^"]+)" is modified to "([^"]+)" and committed on branch "([^"]+)"$"#)]
fn given_modified_on_branch(w: &mut GitWorktreeWorld, file: String, content: String, branch: String) {
    let repo_p = repo_path(w);
    run_git(&repo_p, &["checkout", &branch]);
    std::fs::write(repo_p.join(&file), format!("{content}\n")).expect("write");
    run_git(&repo_p, &["add", &file]);
    run_git(&repo_p, &["commit", "-m", &format!("modify {file} on {branch}")]);
}

// ── When ────────────────────────────────────────────────────────────────────

#[when(regex = r#"^the pool spawns a worktree "([^"]+)" on branch "([^"]+)"$"#)]
fn when_spawn(w: &mut GitWorktreeWorld, id: String, branch: String) {
    let pool = w.pool.as_mut().expect("pool");
    pool.spawn(&id, &branch).expect("spawn");
}

#[when(regex = r#"^the pool removes worktree "([^"]+)"$"#)]
fn when_remove(w: &mut GitWorktreeWorld, id: String) {
    let pool = w.pool.as_mut().expect("pool");
    pool.remove(&id).expect("remove");
}

#[when(regex = r#"^the pool merges worktree "([^"]+)" into branch "([^"]+)"$"#)]
fn when_merge(w: &mut GitWorktreeWorld, id: String, target: String) {
    let pool = w.pool.as_mut().expect("pool");
    let outcome = pool.merge_into(&id, &target).expect("merge call");
    w.last_merge = Some(outcome);
}

#[when(regex = r#"^the pool attempts to spawn worktree "([^"]+)" on branch "([^"]+)"$"#)]
fn when_spawn_attempt(w: &mut GitWorktreeWorld, id: String, branch: String) {
    let pool = w.pool.as_mut().expect("pool");
    match pool.spawn(&id, &branch) {
        Ok(_) => w.last_spawn_error = None,
        Err(e) => w.last_spawn_error = Some(e),
    }
}

// ── Then ────────────────────────────────────────────────────────────────────

#[then(regex = r#"^the path for worktree "([^"]+)" exists on disk$"#)]
fn then_path_exists(w: &mut GitWorktreeWorld, id: String) {
    let pool = w.pool.as_ref().expect("pool");
    let handle = pool.get(&id).expect("handle");
    assert!(handle.path.is_dir(), "path {:?} does not exist", handle.path);
}

#[then(regex = r#"^the worktree HEAD is on branch "([^"]+)"$"#)]
fn then_head_on_branch(w: &mut GitWorktreeWorld, branch: String) {
    let pool = w.pool.as_ref().expect("pool");
    let handle = pool.list().into_iter().next().expect("at least one");
    let out = Command::new("git")
        .current_dir(&handle.path)
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .expect("git");
    let head = String::from_utf8_lossy(&out.stdout).trim().to_string();
    assert_eq!(head, branch);
}

#[then(regex = r#"^the path for worktree "([^"]+)" no longer exists$"#)]
fn then_path_gone(w: &mut GitWorktreeWorld, id: String) {
    let base = base_path(w);
    let expected = base.join(&id);
    assert!(!expected.exists(), "worktree dir {expected:?} still present");
    let pool = w.pool.as_ref().expect("pool");
    assert!(pool.get(&id).is_none(), "handle for {id} still tracked");
}

#[then(regex = r#"^the merge succeeds with no conflicts$"#)]
fn then_merge_clean(w: &mut GitWorktreeWorld) {
    let m = w.last_merge.as_ref().expect("merge outcome");
    assert!(m.merged, "merge failed: {:?}", m);
    assert!(m.conflicts.is_empty(), "conflicts: {:?}", m.conflicts);
}

#[then(regex = r#"^branch "([^"]+)" contains file "([^"]+)"$"#)]
fn then_branch_has_file(w: &mut GitWorktreeWorld, branch: String, file: String) {
    let repo_p = repo_path(w);
    run_git(&repo_p, &["checkout", &branch]);
    assert!(repo_p.join(&file).exists(), "file {file} not present on {branch}");
}

#[then(regex = r#"^the merge reports conflicts in "([^"]+)"$"#)]
fn then_merge_conflicts(w: &mut GitWorktreeWorld, file: String) {
    let m = w.last_merge.as_ref().expect("merge outcome");
    assert!(!m.merged, "expected failed merge, got success");
    assert!(
        m.conflicts.iter().any(|c| c.contains(&file)),
        "conflicts {:?} missing {file}",
        m.conflicts
    );
}

#[then(regex = r#"^the source repo working tree is clean$"#)]
fn then_source_clean(w: &mut GitWorktreeWorld) {
    let repo_p = repo_path(w);
    let out = Command::new("git")
        .current_dir(&repo_p)
        .args(["status", "--porcelain"])
        .output()
        .expect("git");
    let status = String::from_utf8_lossy(&out.stdout);
    assert!(
        status.trim().is_empty(),
        "source repo not clean: {status}"
    );
}

#[then(regex = r#"^the spawn returns an error mentioning "([^"]+)"$"#)]
fn then_spawn_error(w: &mut GitWorktreeWorld, needle: String) {
    let err = w.last_spawn_error.as_ref().expect("spawn error");
    assert!(
        err.to_lowercase().contains(&needle.to_lowercase()),
        "error {err:?} missing {needle}"
    );
}

fn main() {
    futures::executor::block_on(
        GitWorktreeWorld::run("tests/features/worktree_git.feature"),
    );
}
