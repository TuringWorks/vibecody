//! # vibe-eval — VibeCody's evaluation harness
//!
//! Measures how good VibeCody actually is: at writing and repairing code, at
//! the multi-step tool work modern agents are judged on, at the knowledge-work
//! tasks people do all day, and — the part general benchmarks cannot cover —
//! at doing all of it through every surface the product ships.
//!
//! ## Shape
//!
//! ```text
//!   evals/suites/*.yaml            authored + imported tasks
//!            │
//!            ▼
//!   suite::load_dir ──► validate ──► TaskRef
//!            │
//!            ▼
//!   workspace::prepare              a temp tree, plus a pristine baseline
//!            │
//!            ▼
//!   Harness::run                    cli │ daemon │ probe
//!            │
//!            ▼
//!   Grader::grade                   command │ files │ transcript │
//!            │                      patch_and_test │ http │ judge
//!            ▼
//!   EvalReport ──► markdown / json ──► gate::compare ──► exit code
//! ```
//!
//! ## The rule the whole crate is built around
//!
//! **Never report a result you did not measure.** Four verdicts, kept
//! separate: `pass`, `fail`, `error` (the harness could not decide), `skipped`
//! (the task did not apply here). Errors and skips stay out of the pass-rate
//! denominator, a rate over zero scored tasks is `None` rather than `0.0`, and
//! [`gate`] treats a task that stopped being measured as its own kind of
//! problem — because otherwise the cheapest way to turn a gate green is to
//! stop running the tasks that fail.
//!
//! ## Using it
//!
//! ```no_run
//! # async fn example() {
//! use std::sync::Arc;
//! use vibe_eval::{
//!     harness::{CliConfig, CliHarness},
//!     runner::{RunConfig, Runner},
//! };
//!
//! let runner = Runner::new()
//!     .with_harness(Arc::new(CliHarness::new(CliConfig::default())));
//!
//! let report = runner.run(&RunConfig::default()).await;
//! println!("{}", report.to_markdown());
//! # }
//! ```

pub mod dataset;
pub mod gate;
pub mod grade;
pub mod harness;
pub mod report;
pub mod runner;
pub mod suite;
pub mod task;
pub mod workspace;

pub use gate::{compare, evaluate, Comparison, GateOutcome, GatePolicy};
pub use grade::{Grader, JudgeModel, Verdict};
pub use harness::{Harness, Preflight, RunOutcome};
pub use report::{EvalReport, Tally, TaskResult};
pub use runner::{Filter, RunConfig, Runner};
pub use suite::{Suite, SuiteError};
pub use task::{Capability, Difficulty, EvalTask, Surface, TaskRef};

/// Default location of the suite tree, relative to the repository root.
pub const DEFAULT_SUITES_DIR: &str = "evals/suites";

/// Where runs are archived: `~/.vibecli/evals/runs/<run-id>/`.
///
/// Under `~/.vibecli` with the rest of VibeCody's state rather than in the
/// repository, so a run does not dirty the working tree it may be evaluating.
pub fn runs_dir() -> Option<std::path::PathBuf> {
    std::env::var_os("HOME")
        .map(std::path::PathBuf::from)
        .map(|home| home.join(".vibecli").join("evals").join("runs"))
}

/// Where downloaded third-party datasets are cached.
pub fn datasets_dir() -> Option<std::path::PathBuf> {
    std::env::var_os("HOME")
        .map(std::path::PathBuf::from)
        .map(|home| home.join(".vibecli").join("evals").join("datasets"))
}
