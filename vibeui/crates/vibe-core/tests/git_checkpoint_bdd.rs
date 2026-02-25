//! BDD tests for git checkpoint (stash) management.
//!
//! Run with: `cargo test --test git_checkpoint_bdd`

use cucumber::{given, then, when, World};
use std::process::Command;
use tempfile::TempDir;
use vibe_core::git;

// ── World ─────────────────────────────────────────────────────────────────────

#[derive(Debug, World)]
#[world(init = Self::new)]
pub struct GitWorld {
    dir: Option<TempDir>,
    checkpoints: Vec<git::CheckpointInfo>,
}

impl GitWorld {
    fn new() -> Self {
        Self { dir: None, checkpoints: vec![] }
    }

    fn repo(&self) -> &std::path::Path {
        self.dir.as_ref().expect("repo not initialised").path()
    }

    fn run_git(&self, args: &[&str]) {
        let status = Command::new("git")
            .args(args)
            .current_dir(self.repo())
            .output()
            .unwrap_or_else(|_| panic!("git {:?} failed to spawn", args))
            .status;
        assert!(status.success(), "git {:?} exited with {}", args, status);
    }
}

// ── Background ────────────────────────────────────────────────────────────────

#[given("a fresh git repository with an initial commit")]
fn setup_repo(world: &mut GitWorld) {
    let dir = TempDir::new().expect("tempdir");
    let p = dir.path();

    let run = |args: &[&str]| {
        Command::new("git").args(args).current_dir(p).output().unwrap();
    };
    run(&["init"]);
    run(&["config", "user.email", "bdd@test.com"]);
    run(&["config", "user.name", "BDD Test"]);
    std::fs::write(p.join("README.md"), "init").unwrap();
    run(&["add", "README.md"]);
    run(&["commit", "-m", "init"]);

    world.dir = Some(dir);
}

// ── Given steps ───────────────────────────────────────────────────────────────

#[given(regex = r#"the file "([^"]+)" exists with content "([^"]+)""#)]
fn file_exists(world: &mut GitWorld, filename: String, content: String) {
    std::fs::write(world.repo().join(&filename), &content).unwrap();
    world.run_git(&["add", &filename]);
    world.run_git(&["commit", "-m", &format!("add {filename}")]);
}

#[given(regex = r#"the file "([^"]+)" is modified to contain "([^"]+)""#)]
fn file_modified(world: &mut GitWorld, filename: String, content: String) {
    std::fs::write(world.repo().join(&filename), &content).unwrap();
}

#[given(regex = r#"I create a checkpoint named "([^"]+)""#)]
fn create_checkpoint(world: &mut GitWorld, name: String) {
    git::create_stash(world.repo(), &name).expect("create_stash failed");
}

// ── When steps ────────────────────────────────────────────────────────────────

#[when(regex = r#"I create a checkpoint named "([^"]+)""#)]
fn when_create_checkpoint(world: &mut GitWorld, name: String) {
    git::create_stash(world.repo(), &name).expect("create_stash failed");
}

#[when("I list all checkpoints")]
fn list_checkpoints(world: &mut GitWorld) {
    world.checkpoints = git::list_stashes(world.repo()).expect("list_stashes failed");
}

#[when(regex = r"I restore the checkpoint at index (\d+)")]
fn restore_checkpoint(world: &mut GitWorld, index: usize) {
    git::restore_stash(world.repo(), index).expect("restore_stash failed");
}

#[when(regex = r"I delete the checkpoint at index (\d+)")]
fn delete_checkpoint(world: &mut GitWorld, index: usize) {
    git::drop_stash(world.repo(), index).expect("drop_stash failed");
}

// ── Then steps ────────────────────────────────────────────────────────────────

#[then(regex = r"(\d+) checkpoint(?:s)? exist(?:s)? in the stash list")]
fn checkpoint_count(world: &mut GitWorld, expected: usize) {
    let stashes = git::list_stashes(world.repo()).expect("list_stashes failed");
    assert_eq!(
        stashes.len(),
        expected,
        "expected {expected} stash(es), found {}: {:?}",
        stashes.len(),
        stashes
    );
}

#[then(regex = r#"the checkpoint message contains "([^"]+)""#)]
fn checkpoint_message_contains(world: &mut GitWorld, substring: String) {
    let stashes = git::list_stashes(world.repo()).expect("list_stashes");
    assert!(
        stashes.iter().any(|s| s.message.contains(&substring)),
        "no stash with message containing '{substring}'; got: {:?}",
        stashes.iter().map(|s| &s.message).collect::<Vec<_>>()
    );
}

#[then(regex = r#"the file "([^"]+)" contains "([^"]+)""#)]
fn file_contains(world: &mut GitWorld, filename: String, expected: String) {
    let actual = std::fs::read_to_string(world.repo().join(&filename))
        .unwrap_or_else(|_| panic!("cannot read {filename}"));
    assert_eq!(
        actual.trim(),
        expected.trim(),
        "file '{filename}' content mismatch"
    );
}

// ── Entry point ───────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    GitWorld::run("tests/features/git_checkpoint.feature").await;
}
