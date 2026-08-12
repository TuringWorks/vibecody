//! The shipped suites must load, validate, and stay internally consistent.
//!
//! A suite file is data, so nothing else in the build checks it: a typo in a
//! grader key, a task id used twice, an assertion naming a fixture file that
//! does not exist — all of it would surface as a confusing mid-run error, or
//! worse, as a task that quietly stops testing anything. This runs on every
//! `cargo test`, so the suites are covered the way code is.

use std::collections::BTreeSet;
use std::path::PathBuf;

use vibe_eval::grade::{FileAssertion, Grader};
use vibe_eval::suite::{self, Suite};
use vibe_eval::task::{Capability, Surface, WorkspaceMode};

fn repo_root() -> PathBuf {
    // crates/vibe-eval → repo root
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

fn suites_dir() -> PathBuf {
    repo_root().join(vibe_eval::DEFAULT_SUITES_DIR)
}

fn load() -> Vec<Suite> {
    let dir = suites_dir();
    assert!(
        dir.exists(),
        "{} is missing — the vendored suites are part of the product, not an optional extra",
        dir.display()
    );
    let (suites, errors) = suite::load_dir(&dir);
    assert!(
        errors.is_empty(),
        "suites failed to load:\n{}",
        errors
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("\n")
    );
    assert!(!suites.is_empty(), "no suites found in {}", dir.display());
    suites
}

#[test]
fn every_shipped_suite_loads_and_validates() {
    let suites = load();
    let total: usize = suites.iter().map(|s| s.tasks.len()).sum();
    assert!(
        total >= 25,
        "only {} tasks across {} suites — too thin to characterise an agent",
        total,
        suites.len()
    );
}

#[test]
fn suite_ids_are_unique() {
    let suites = load();
    let mut seen = BTreeSet::new();
    for s in &suites {
        assert!(seen.insert(s.id.clone()), "duplicate suite id `{}`", s.id);
    }
}

#[test]
fn task_keys_are_globally_unique() {
    // Report rows and baseline lookups are keyed on `<suite>/<task>`, so a
    // collision across suites would silently merge two measurements.
    let suites = load();
    let mut seen = BTreeSet::new();
    for s in &suites {
        for t in s.task_refs() {
            assert!(seen.insert(t.key()), "duplicate task key `{}`", t.key());
        }
    }
}

#[test]
fn file_assertions_reference_files_the_task_could_plausibly_have() {
    // An `unchanged` assertion needs a fixture baseline containing that file.
    // Naming a path the fixture never created makes the assertion error at
    // run time — which is honest, but is a bug we should catch at build time.
    let suites = load();
    for s in &suites {
        for t in &s.tasks {
            if t.workspace != WorkspaceMode::Temp || t.fixture.dir.is_some() {
                // Repo-root tasks assert over the checkout, and `dir` fixtures
                // are copied from disk; neither can be checked from the YAML.
                continue;
            }
            for path in unchanged_paths(&t.grader) {
                assert!(
                    t.fixture.files.contains_key(&path),
                    "{}/{}: `unchanged: {}` names a file the fixture never creates",
                    s.id,
                    t.id,
                    path
                );
            }
        }
    }
}

fn unchanged_paths(grader: &Grader) -> Vec<String> {
    match grader {
        Grader::Files { assertions } => assertions
            .iter()
            .filter_map(|a| match a {
                FileAssertion::Unchanged { path } => Some(path.clone()),
                _ => None,
            })
            .collect(),
        Grader::All { of } | Grader::Any { of } => of.iter().flat_map(unchanged_paths).collect(),
        _ => Vec::new(),
    }
}

#[test]
fn tasks_that_run_toolchains_declare_them() {
    // A task whose grader shells out to python3 without declaring
    // `requires: [python3]` fails on a machine without it, instead of
    // skipping — and a false failure is worse than a missing measurement.
    let suites = load();
    for s in &suites {
        for t in &s.tasks {
            let commands = grader_commands(&t.grader);
            for tool in ["python3", "node", "cargo", "make", "git"] {
                if commands.iter().any(|c| c == tool) {
                    assert!(
                        t.requires.iter().any(|r| r == tool),
                        "{}/{}: grader runs `{}` but does not declare it in `requires`",
                        s.id,
                        t.id,
                        tool
                    );
                }
            }
        }
    }
}

fn grader_commands(grader: &Grader) -> Vec<String> {
    match grader {
        Grader::Command { steps } => steps.iter().map(|s| s.cmd.clone()).collect(),
        Grader::PatchAndTest { runner, .. } => vec![runner.cmd.clone()],
        Grader::All { of } | Grader::Any { of } => of.iter().flat_map(grader_commands).collect(),
        _ => Vec::new(),
    }
}

#[test]
fn conformance_tasks_never_carry_a_prompt() {
    // Enforced by `Suite::validate` too; asserted here so the shipped tree is
    // covered rather than only the validator.
    let suites = load();
    for s in &suites {
        for t in &s.tasks {
            if t.workspace == WorkspaceMode::RepoRoot {
                assert!(
                    t.prompt.trim().is_empty(),
                    "{}/{} would point an agent at the repository checkout",
                    s.id,
                    t.id
                );
            }
        }
    }
}

#[test]
fn the_suites_cover_the_capabilities_they_claim() {
    // The headline claim is "coding agent plus modern work tasks across every
    // surface". These are the capabilities that claim cashes out to; a gap
    // here means the report would render `n/a` for something we say we test.
    let suites = load();
    let covered: BTreeSet<Capability> = suites
        .iter()
        .flat_map(|s| s.tasks.iter().map(|t| t.capability))
        .collect();

    for required in [
        Capability::CodeGeneration,
        Capability::CodeRepair,
        Capability::Debugging,
        Capability::Refactoring,
        Capability::MultiFileEdit,
        Capability::TestAuthoring,
        Capability::CodeComprehension,
        Capability::ToolUse,
        Capability::Retrieval,
        Capability::Planning,
        Capability::LongHorizon,
        Capability::WorkTask,
        Capability::Communication,
        Capability::DataAnalysis,
        Capability::SurfaceConformance,
        Capability::Safety,
    ] {
        assert!(
            covered.contains(&required),
            "no task measures `{}` — the suites do not cover what the harness claims",
            required.slug()
        );
    }
}

#[test]
fn conformance_covers_every_shipped_surface() {
    // The whole reason surface conformance exists: a capability score says
    // nothing about whether the watch can reach the feature. If a surface has
    // no conformance task, nothing in this harness would notice it going
    // unreachable.
    let suites = load();
    let covered: BTreeSet<Surface> = suites
        .iter()
        .flat_map(|s| s.task_refs())
        .filter(|t| t.task.capability == Capability::SurfaceConformance)
        .flat_map(|t| t.task.surfaces.clone())
        .collect();

    let uncovered: Vec<&str> = Surface::ALL
        .iter()
        .filter(|s| !covered.contains(s))
        .map(|s| s.slug())
        .collect();

    assert!(
        uncovered.is_empty(),
        "these surfaces have no conformance task: {}. Add one to \
         evals/suites/surfaces.yaml, or remove the surface if it no longer ships.",
        uncovered.join(", ")
    );
}
