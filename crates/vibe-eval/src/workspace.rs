//! Building the directory a task runs in.
//!
//! Every temp-mode task gets a fresh tree and a pristine copy of it. The copy
//! is what makes `unchanged` assertions and post-hoc diffing possible, and
//! taking it *before* the agent runs is the only moment it is available —
//! reconstructing "what the fixture looked like" afterwards from the agent's
//! own account of its edits would be trusting the thing under test.

use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::grade::run_command;
use crate::task::{EvalTask, WorkspaceMode};

/// A prepared working directory, alive for as long as this value is.
#[derive(Debug)]
pub struct PreparedWorkspace {
    /// Where the agent runs.
    pub root: PathBuf,
    /// Pristine copy of the fixture, or `None` for repo-root tasks.
    pub baseline: Option<PathBuf>,
    /// Dropping this removes the temp tree. Held, not ignored: an eval run
    /// over a few hundred tasks otherwise fills the disk with fixtures.
    _guard: Option<tempfile::TempDir>,
}

#[derive(Debug, thiserror::Error)]
pub enum WorkspaceError {
    #[error("cannot create workspace: {0}")]
    Create(String),
    #[error("cannot materialise fixture file {path}: {reason}")]
    Fixture { path: String, reason: String },
    #[error("fixture setup command failed: {0}")]
    Setup(String),
    #[error("repo root not found: {0}")]
    RepoRoot(String),
}

/// Prepare the workspace for one task.
///
/// `repo_root` is required only by [`WorkspaceMode::RepoRoot`] tasks.
pub async fn prepare(
    task: &EvalTask,
    suite_dir: &Path,
    repo_root: Option<&Path>,
) -> Result<PreparedWorkspace, WorkspaceError> {
    match task.workspace {
        WorkspaceMode::RepoRoot => {
            let root = repo_root
                .ok_or_else(|| WorkspaceError::RepoRoot("no repository root configured".into()))?;
            if !root.exists() {
                return Err(WorkspaceError::RepoRoot(format!(
                    "{} does not exist",
                    root.display()
                )));
            }
            Ok(PreparedWorkspace {
                root: root.to_path_buf(),
                // No baseline: this tree is not ours to snapshot, and
                // `unchanged` assertions over it would be meaningless.
                baseline: None,
                _guard: None,
            })
        }
        WorkspaceMode::Temp => prepare_temp(task, suite_dir).await,
    }
}

async fn prepare_temp(
    task: &EvalTask,
    suite_dir: &Path,
) -> Result<PreparedWorkspace, WorkspaceError> {
    let temp = tempfile::Builder::new()
        .prefix("vibe-eval-")
        .tempdir()
        .map_err(|e| WorkspaceError::Create(e.to_string()))?;
    let root = temp.path().join("workspace");
    std::fs::create_dir_all(&root).map_err(|e| WorkspaceError::Create(e.to_string()))?;

    // Directory first, inline files second, so a suite can copy a tree and
    // then override one file of it.
    if let Some(rel) = &task.fixture.dir {
        let src = suite_dir.join(rel);
        if !src.exists() {
            return Err(WorkspaceError::Fixture {
                path: rel.display().to_string(),
                reason: format!("{} does not exist", src.display()),
            });
        }
        copy_tree(&src, &root).map_err(|e| WorkspaceError::Fixture {
            path: rel.display().to_string(),
            reason: e,
        })?;
    }

    for (rel, contents) in &task.fixture.files {
        let dest = safe_join(&root, rel).ok_or_else(|| WorkspaceError::Fixture {
            path: rel.clone(),
            reason: "path escapes the workspace".to_string(),
        })?;
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent).map_err(|e| WorkspaceError::Fixture {
                path: rel.clone(),
                reason: e.to_string(),
            })?;
        }
        std::fs::write(&dest, contents).map_err(|e| WorkspaceError::Fixture {
            path: rel.clone(),
            reason: e.to_string(),
        })?;
    }

    if task.fixture.git_init {
        init_git(&root).await?;
    }

    for step in &task.fixture.setup {
        // Setup failures are environment problems, not agent failures: the
        // agent has not been asked to do anything yet. Surfacing them as a
        // distinct error is what lets the runner mark the task errored rather
        // than failed.
        let run = run_command(step, &root, Duration::from_secs(600))
            .await
            .map_err(WorkspaceError::Setup)?;
        if run.exit_code != Some(step.expect_exit.unwrap_or(0)) {
            return Err(WorkspaceError::Setup(format!(
                "`{}` exited {:?}: {}",
                step.display(),
                run.exit_code,
                run.combined_output().chars().take(600).collect::<String>()
            )));
        }
    }

    // Snapshot after setup so generated files (lockfiles, node_modules) are
    // part of the baseline and do not read as agent edits.
    let baseline = temp.path().join("baseline");
    copy_tree(&root, &baseline).map_err(WorkspaceError::Create)?;

    Ok(PreparedWorkspace {
        root,
        baseline: Some(baseline),
        _guard: Some(temp),
    })
}

async fn init_git(root: &Path) -> Result<(), WorkspaceError> {
    let steps: Vec<Vec<&str>> = vec![
        vec!["init", "--quiet"],
        // Identity is set locally so the commit works on a machine with no
        // global git config — a CI container, typically.
        vec!["config", "user.email", "eval@vibecody.local"],
        vec!["config", "user.name", "VibeCody Eval"],
        vec!["add", "-A"],
        vec!["commit", "--quiet", "-m", "fixture"],
    ];
    for args in steps {
        let step = crate::grade::CommandStep {
            cmd: "git".to_string(),
            args: args.iter().map(|s| s.to_string()).collect(),
            cwd: None,
            env: Default::default(),
            expect_exit: None,
            stdout_contains: None,
            stdout_not_contains: None,
            timeout_secs: Some(60),
        };
        let run = run_command(&step, root, Duration::from_secs(60))
            .await
            .map_err(WorkspaceError::Setup)?;
        if run.exit_code != Some(0) {
            return Err(WorkspaceError::Setup(format!(
                "git {}: {}",
                args.join(" "),
                run.combined_output().chars().take(400).collect::<String>()
            )));
        }
    }
    Ok(())
}

/// Join `rel` under `root`, refusing anything that would escape.
///
/// Fixture paths come from YAML, and `../../.ssh/authorized_keys` is a
/// perfectly valid YAML string.
fn safe_join(root: &Path, rel: &str) -> Option<PathBuf> {
    let candidate = Path::new(rel);
    if candidate.is_absolute() {
        return None;
    }
    let mut depth: i32 = 0;
    for component in candidate.components() {
        match component {
            std::path::Component::ParentDir => {
                depth -= 1;
                if depth < 0 {
                    return None;
                }
            }
            std::path::Component::Normal(_) => depth += 1,
            std::path::Component::CurDir => {}
            std::path::Component::RootDir | std::path::Component::Prefix(_) => return None,
        }
    }
    Some(root.join(candidate))
}

fn copy_tree(src: &Path, dest: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dest).map_err(|e| e.to_string())?;
    for entry in walkdir::WalkDir::new(src).follow_links(false) {
        let entry = entry.map_err(|e| e.to_string())?;
        let rel = entry.path().strip_prefix(src).map_err(|e| e.to_string())?;
        if rel.as_os_str().is_empty() {
            continue;
        }
        let target = dest.join(rel);
        if entry.file_type().is_dir() {
            std::fs::create_dir_all(&target).map_err(|e| e.to_string())?;
        } else if entry.file_type().is_file() {
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            std::fs::copy(entry.path(), &target).map_err(|e| e.to_string())?;
        }
        // Symlinks are skipped rather than followed: a fixture that links out
        // of the tree would give an agent a writable path outside its sandbox.
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::{Capability, Difficulty, Fixture, Limits, TaskSource};
    use std::collections::BTreeMap;

    fn task_with(fixture: Fixture) -> EvalTask {
        EvalTask {
            id: "t".into(),
            title: "t".into(),
            capability: Capability::CodeGeneration,
            difficulty: Difficulty::Easy,
            surfaces: vec![],
            prompt: "do a thing".into(),
            fixture,
            grader: crate::grade::Grader::AlwaysSkip {
                reason: "test".into(),
            },
            limits: Limits::default(),
            tags: vec![],
            source: TaskSource::Vendored,
            requires: vec![],
            workspace: WorkspaceMode::Temp,
        }
    }

    #[tokio::test]
    async fn materialises_inline_files_and_snapshots_a_baseline() {
        let mut files = BTreeMap::new();
        files.insert("src/main.rs".to_string(), "fn main() {}\n".to_string());
        let ws = prepare(
            &task_with(Fixture {
                files,
                ..Fixture::default()
            }),
            Path::new("."),
            None,
        )
        .await
        .expect("prepare");
        assert_eq!(
            std::fs::read_to_string(ws.root.join("src/main.rs")).expect("read"),
            "fn main() {}\n"
        );
        let baseline = ws.baseline.clone().expect("baseline");
        assert!(baseline.join("src/main.rs").exists());

        // Edits after preparation must not reach the baseline — that is the
        // whole point of taking the copy first.
        std::fs::write(ws.root.join("src/main.rs"), "edited").expect("write");
        assert_eq!(
            std::fs::read_to_string(baseline.join("src/main.rs")).expect("read"),
            "fn main() {}\n"
        );
    }

    #[tokio::test]
    async fn workspace_is_removed_when_dropped() {
        let mut files = BTreeMap::new();
        files.insert("a.txt".to_string(), "x".to_string());
        let root = {
            let ws = prepare(
                &task_with(Fixture {
                    files,
                    ..Fixture::default()
                }),
                Path::new("."),
                None,
            )
            .await
            .expect("prepare");
            ws.root.clone()
        };
        assert!(!root.exists(), "temp workspace should be cleaned up");
    }

    #[test]
    fn fixture_paths_cannot_escape_the_workspace() {
        let root = Path::new("/tmp/ws");
        assert!(safe_join(root, "src/main.rs").is_some());
        assert!(safe_join(root, "./a/../b.txt").is_some());
        // These are the ones that matter.
        assert!(safe_join(root, "../outside.txt").is_none());
        assert!(safe_join(root, "a/../../outside.txt").is_none());
        assert!(safe_join(root, "/etc/passwd").is_none());
    }

    #[tokio::test]
    async fn an_escaping_fixture_path_is_rejected() {
        let mut files = BTreeMap::new();
        files.insert("../escaped.txt".to_string(), "nope".to_string());
        let err = prepare(
            &task_with(Fixture {
                files,
                ..Fixture::default()
            }),
            Path::new("."),
            None,
        )
        .await
        .expect_err("should refuse");
        assert!(err.to_string().contains("escapes"), "{}", err);
    }

    #[tokio::test]
    async fn git_init_produces_a_clean_tree() {
        let mut files = BTreeMap::new();
        files.insert("a.txt".to_string(), "hello\n".to_string());
        let ws = prepare(
            &task_with(Fixture {
                files,
                git_init: true,
                ..Fixture::default()
            }),
            Path::new("."),
            None,
        )
        .await
        .expect("prepare");
        let step = crate::grade::CommandStep {
            cmd: "git".into(),
            args: vec!["status".into(), "--porcelain".into()],
            cwd: None,
            env: Default::default(),
            expect_exit: None,
            stdout_contains: None,
            stdout_not_contains: None,
            timeout_secs: Some(30),
        };
        let run = run_command(&step, &ws.root, Duration::from_secs(30))
            .await
            .expect("git status");
        assert!(
            run.stdout.trim().is_empty(),
            "fixture should be committed, got: {}",
            run.stdout
        );
    }

    #[tokio::test]
    async fn a_failing_setup_command_is_a_setup_error_not_a_task_failure() {
        let fixture = Fixture {
            setup: vec![crate::grade::CommandStep {
                cmd: "sh".into(),
                args: vec!["-c".into(), "exit 7".into()],
                cwd: None,
                env: Default::default(),
                expect_exit: None,
                stdout_contains: None,
                stdout_not_contains: None,
                timeout_secs: Some(30),
            }],
            ..Fixture::default()
        };
        let err = prepare(&task_with(fixture), Path::new("."), None)
            .await
            .expect_err("setup should fail");
        assert!(matches!(err, WorkspaceError::Setup(_)), "{:?}", err);
    }

    #[tokio::test]
    async fn repo_root_mode_uses_the_repo_and_takes_no_baseline() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut task = task_with(Fixture::default());
        task.workspace = WorkspaceMode::RepoRoot;
        task.prompt = String::new();
        let ws = prepare(&task, Path::new("."), Some(dir.path()))
            .await
            .expect("prepare");
        assert_eq!(ws.root, dir.path());
        assert!(ws.baseline.is_none());
    }

    #[tokio::test]
    async fn repo_root_mode_without_a_repo_errors() {
        let mut task = task_with(Fixture::default());
        task.workspace = WorkspaceMode::RepoRoot;
        task.prompt = String::new();
        let err = prepare(&task, Path::new("."), None)
            .await
            .expect_err("should error");
        assert!(matches!(err, WorkspaceError::RepoRoot(_)));
    }
}
