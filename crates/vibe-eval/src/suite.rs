//! Loading suites from disk, and refusing to run ones that would lie.
//!
//! Suites are YAML files under `evals/suites/`. Loading validates them, and
//! validation is not cosmetic: a task with an empty grader, or a duplicate id
//! that silently overwrites its twin in the report, produces a number that
//! looks like a result and is not one. [`Suite::validate`] rejects those at
//! load time rather than letting them reach a leaderboard.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::grade::Grader;
use crate::task::{EvalTask, Limits, Surface, TaskRef, WorkspaceMode};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Suite {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub description: String,
    /// Surfaces a task runs against when it names none itself.
    #[serde(default)]
    pub default_surfaces: Vec<Surface>,
    /// Limits a task inherits when it sets none.
    #[serde(default)]
    pub defaults: Limits,
    pub tasks: Vec<EvalTask>,
    /// Directory the suite was loaded from. Fixture `dir` paths resolve
    /// against it, so a suite is relocatable as a unit.
    #[serde(skip)]
    pub base_dir: PathBuf,
}

#[derive(Debug, thiserror::Error)]
pub enum SuiteError {
    #[error("cannot read {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("{path}: {source}")]
    Parse {
        path: PathBuf,
        #[source]
        source: serde_yaml::Error,
    },
    #[error("{path} is not a valid suite:\n  - {}", problems.join("\n  - "))]
    Invalid {
        path: PathBuf,
        problems: Vec<String>,
    },
}

impl Suite {
    pub fn from_yaml(text: &str, path: &Path) -> Result<Self, SuiteError> {
        let mut suite: Suite = serde_yaml::from_str(text).map_err(|source| SuiteError::Parse {
            path: path.to_path_buf(),
            source,
        })?;
        suite.base_dir = path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));
        let problems = suite.validate();
        if !problems.is_empty() {
            return Err(SuiteError::Invalid {
                path: path.to_path_buf(),
                problems,
            });
        }
        Ok(suite)
    }

    pub fn load(path: &Path) -> Result<Self, SuiteError> {
        let text = std::fs::read_to_string(path).map_err(|source| SuiteError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        Self::from_yaml(&text, path)
    }

    /// Everything wrong with this suite, stated in full rather than one at a
    /// time — an author fixing a suite should see the whole list.
    pub fn validate(&self) -> Vec<String> {
        let mut problems = Vec::new();
        if self.id.trim().is_empty() {
            problems.push("suite id is empty".to_string());
        }
        if self.tasks.is_empty() {
            problems.push("suite has no tasks".to_string());
        }

        let mut seen: std::collections::BTreeSet<&str> = std::collections::BTreeSet::new();
        for task in &self.tasks {
            if task.id.trim().is_empty() {
                problems.push("a task has an empty id".to_string());
            } else if !seen.insert(task.id.as_str()) {
                // Two tasks with one id collapse into a single report row, so
                // one of them silently stops being measured.
                problems.push(format!("duplicate task id `{}`", task.id));
            }
            problems.extend(
                validate_grader(&task.grader)
                    .into_iter()
                    .map(|p| format!("task `{}`: {}", task.id, p)),
            );
            match task.workspace {
                WorkspaceMode::RepoRoot => {
                    if !task.prompt.trim().is_empty() {
                        // A repo-root task hands the working directory to
                        // whatever runs; combining that with a prompt would
                        // point an autonomous agent at its own source tree.
                        problems.push(format!(
                            "task `{}`: repo_root tasks must not carry a prompt \
                             (they are static checks, not agent runs)",
                            task.id
                        ));
                    }
                    if !task.fixture.is_empty() {
                        problems.push(format!(
                            "task `{}`: repo_root tasks cannot also declare a fixture",
                            task.id
                        ));
                    }
                }
                WorkspaceMode::Temp => {
                    if task.prompt.trim().is_empty() && !is_probe_only(&task.grader) {
                        problems.push(format!(
                            "task `{}`: no prompt, so the agent would be asked nothing \
                             while the grader still judged it",
                            task.id
                        ));
                    }
                }
            }
        }
        problems
    }

    /// Tasks paired with this suite's id, with suite defaults resolved.
    pub fn task_refs(&self) -> Vec<TaskRef> {
        self.tasks
            .iter()
            .map(|task| {
                let mut task = task.clone();
                if task.surfaces.is_empty() {
                    task.surfaces = self.default_surfaces.clone();
                }
                task.limits = Limits {
                    timeout_secs: task.limits.timeout_secs.or(self.defaults.timeout_secs),
                    max_turns: task.limits.max_turns.or(self.defaults.max_turns),
                    no_network: task.limits.no_network.or(self.defaults.no_network),
                };
                TaskRef {
                    suite: self.id.clone(),
                    task,
                }
            })
            .collect()
    }
}

/// A grader that performs its own probing and needs no agent turn.
fn is_probe_only(grader: &Grader) -> bool {
    match grader {
        Grader::Http { .. } | Grader::Command { .. } | Grader::Files { .. } => true,
        Grader::All { of } | Grader::Any { of } => of.iter().all(is_probe_only),
        Grader::Transcript { .. } | Grader::PatchAndTest { .. } | Grader::Judge { .. } => false,
        Grader::AlwaysSkip { .. } => true,
    }
}

/// Graders that cannot reach a verdict, caught before they reach a report.
fn validate_grader(grader: &Grader) -> Vec<String> {
    match grader {
        // An `all` over zero assertions is vacuously true under naive
        // semantics. `GradeResult::reduce_all` already reports it as an error
        // at runtime; rejecting it at load time means it never ships.
        Grader::Command { steps } if steps.is_empty() => {
            vec!["command grader has no steps".to_string()]
        }
        Grader::Files { assertions } if assertions.is_empty() => {
            vec!["files grader has no assertions".to_string()]
        }
        Grader::Transcript { assertions } if assertions.is_empty() => {
            vec!["transcript grader has no assertions".to_string()]
        }
        Grader::Http { probes } if probes.is_empty() => {
            vec!["http grader has no probes".to_string()]
        }
        Grader::All { of } | Grader::Any { of } if of.is_empty() => {
            vec!["composite grader has no children".to_string()]
        }
        Grader::All { of } | Grader::Any { of } => of.iter().flat_map(validate_grader).collect(),
        Grader::PatchAndTest(spec)
            if spec.fail_to_pass.is_empty() && spec.pass_to_pass.is_empty() =>
        {
            vec!["patch_and_test declares no tests, so it would verify nothing".to_string()]
        }
        Grader::Judge { threshold, .. } if !(0.0..=1.0).contains(threshold) => {
            vec![format!(
                "judge threshold {} is outside 0.0..=1.0",
                threshold
            )]
        }
        _ => Vec::new(),
    }
}

/// Load every `*.yaml` / `*.yml` suite under `dir`, recursively.
///
/// Returns the suites it could load *and* the errors it hit, rather than
/// failing the whole run on one bad file: a broken suite should be reported as
/// broken while the rest still produce a number.
pub fn load_dir(dir: &Path) -> (Vec<Suite>, Vec<SuiteError>) {
    let mut suites = Vec::new();
    let mut errors = Vec::new();
    if !dir.exists() {
        return (suites, errors);
    }
    let entries = walkdir::WalkDir::new(dir)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
        .filter(|e| {
            e.path()
                .extension()
                .and_then(|s| s.to_str())
                .is_some_and(|ext| ext == "yaml" || ext == "yml")
        });
    for entry in entries {
        match Suite::load(entry.path()) {
            Ok(suite) => suites.push(suite),
            Err(e) => errors.push(e),
        }
    }
    suites.sort_by(|a, b| a.id.cmp(&b.id));
    (suites, errors)
}

#[cfg(test)]
mod tests {
    use super::*;

    const MINIMAL: &str = r#"
id: demo
title: Demo suite
default_surfaces: [cli]
tasks:
  - id: t1
    title: Add a function
    capability: code_generation
    difficulty: easy
    prompt: Write add() in src/lib.rs
    grader:
      type: files
      assertions:
        - assert: exists
          path: src/lib.rs
"#;

    #[test]
    fn loads_a_minimal_suite() {
        let suite = Suite::from_yaml(MINIMAL, Path::new("/tmp/demo.yaml")).expect("load");
        assert_eq!(suite.id, "demo");
        assert_eq!(suite.tasks.len(), 1);
        assert_eq!(suite.base_dir, Path::new("/tmp"));
    }

    #[test]
    fn task_refs_inherit_suite_defaults() {
        let yaml = format!("{}\ndefaults:\n  timeout_secs: 600\n", MINIMAL);
        let suite = Suite::from_yaml(&yaml, Path::new("/tmp/demo.yaml")).expect("load");
        let refs = suite.task_refs();
        assert_eq!(refs[0].task.limits.timeout_secs, Some(600));
        assert_eq!(refs[0].task.surfaces, vec![Surface::Cli]);
        assert_eq!(refs[0].key(), "demo/t1");
    }

    #[test]
    fn a_task_specific_limit_beats_the_suite_default() {
        let yaml = r#"
id: demo
title: Demo
defaults:
  timeout_secs: 600
tasks:
  - id: t1
    title: t
    capability: code_generation
    difficulty: easy
    prompt: p
    limits:
      timeout_secs: 30
    grader:
      type: files
      assertions:
        - assert: exists
          path: a
"#;
        let suite = Suite::from_yaml(yaml, Path::new("/tmp/d.yaml")).expect("load");
        assert_eq!(suite.task_refs()[0].task.limits.timeout_secs, Some(30));
    }

    #[test]
    fn duplicate_task_ids_are_rejected() {
        let yaml = r#"
id: demo
title: Demo
tasks:
  - id: same
    title: a
    capability: code_generation
    difficulty: easy
    prompt: p
    grader: {type: files, assertions: [{assert: exists, path: a}]}
  - id: same
    title: b
    capability: code_generation
    difficulty: easy
    prompt: p
    grader: {type: files, assertions: [{assert: exists, path: b}]}
"#;
        let err = Suite::from_yaml(yaml, Path::new("/tmp/d.yaml")).expect_err("should reject");
        assert!(err.to_string().contains("duplicate task id"), "{}", err);
    }

    #[test]
    fn an_empty_grader_is_rejected_at_load_time() {
        // This is the vacuous-pass bug in its authored form.
        let yaml = r#"
id: demo
title: Demo
tasks:
  - id: t
    title: t
    capability: code_generation
    difficulty: easy
    prompt: p
    grader:
      type: files
      assertions: []
"#;
        let err = Suite::from_yaml(yaml, Path::new("/tmp/d.yaml")).expect_err("should reject");
        assert!(err.to_string().contains("no assertions"), "{}", err);
    }

    #[test]
    fn a_repo_root_task_may_not_carry_a_prompt() {
        let yaml = r#"
id: demo
title: Demo
tasks:
  - id: t
    title: t
    capability: surface_conformance
    difficulty: easy
    workspace: repo_root
    prompt: go wild in my source tree
    grader: {type: files, assertions: [{assert: exists, path: Cargo.toml}]}
"#;
        let err = Suite::from_yaml(yaml, Path::new("/tmp/d.yaml")).expect_err("should reject");
        assert!(
            err.to_string().contains("must not carry a prompt"),
            "{}",
            err
        );
    }

    #[test]
    fn a_prompted_task_that_asks_nothing_is_rejected() {
        let yaml = r#"
id: demo
title: Demo
tasks:
  - id: t
    title: t
    capability: code_generation
    difficulty: easy
    prompt: "   "
    grader:
      type: transcript
      assertions:
        - assert: used_tool
          tool: bash
"#;
        let err = Suite::from_yaml(yaml, Path::new("/tmp/d.yaml")).expect_err("should reject");
        assert!(err.to_string().contains("asked nothing"), "{}", err);
    }

    #[test]
    fn a_probe_only_task_needs_no_prompt() {
        let yaml = r#"
id: demo
title: Demo
tasks:
  - id: t
    title: health responds
    capability: surface_conformance
    difficulty: easy
    grader:
      type: http
      probes:
        - url: "{daemon}/health"
          expect_status: [200]
"#;
        Suite::from_yaml(yaml, Path::new("/tmp/d.yaml")).expect("probe-only task is valid");
    }

    #[test]
    fn patch_and_test_without_tests_is_rejected() {
        let yaml = r#"
id: demo
title: Demo
tasks:
  - id: t
    title: t
    capability: code_repair
    difficulty: hard
    prompt: fix it
    grader:
      type: patch_and_test
      test_patch: ""
      runner:
        cmd: pytest
"#;
        let err = Suite::from_yaml(yaml, Path::new("/tmp/d.yaml")).expect_err("should reject");
        assert!(err.to_string().contains("verify nothing"), "{}", err);
    }

    #[test]
    fn load_dir_reports_broken_suites_without_losing_good_ones() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("good.yaml"), MINIMAL).expect("write");
        std::fs::write(dir.path().join("bad.yaml"), "id: x\ntitle: y\ntasks: []\n").expect("write");
        let (suites, errors) = load_dir(dir.path());
        assert_eq!(suites.len(), 1, "the good suite should still load");
        assert_eq!(errors.len(), 1, "the broken one should be reported");
    }
}
