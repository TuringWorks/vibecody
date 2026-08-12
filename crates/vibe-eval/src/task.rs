//! The task model: what a single evaluation *is*.
//!
//! A task is a pure description — a prompt, the workspace it starts from, and
//! the grader that decides whether the agent's effect on that workspace was
//! correct. Nothing here runs anything; execution lives in [`crate::runner`]
//! and grading in [`crate::grade`]. Keeping the description inert is what lets
//! the same task be replayed against a different surface, a different provider,
//! or a recorded transcript without rewriting it.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

/// What ability a task is trying to measure.
///
/// This is the axis reports aggregate over, so it is deliberately coarse: a
/// capability is only worth its own variant if a regression in it would send
/// you to a different part of the codebase than its neighbours.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    /// Write new code from a specification.
    CodeGeneration,
    /// Fix a defect in existing code so a failing test passes.
    CodeRepair,
    /// Change structure without changing behaviour.
    Refactoring,
    /// Localise a fault from a symptom (stack trace, wrong output).
    Debugging,
    /// Write tests that actually exercise the described behaviour.
    TestAuthoring,
    /// A single coherent change spanning several files.
    MultiFileEdit,
    /// Answer questions about a codebase without editing it.
    CodeComprehension,
    /// Choose and sequence tools correctly.
    ToolUse,
    /// Find the relevant material in a large workspace.
    Retrieval,
    /// Decompose a goal before acting on it.
    Planning,
    /// Hold a goal across many steps without drifting.
    LongHorizon,
    /// Non-coding knowledge work: tickets, mail, calendars, documents.
    WorkTask,
    /// Produce prose for a human audience: summaries, reports, replies.
    Communication,
    /// Read data and draw a defensible conclusion from it.
    DataAnalysis,
    /// The surface itself behaves per contract (routes, auth, schemas).
    SurfaceConformance,
    /// Declining, resisting injection, and handling secrets correctly.
    Safety,
}

impl Capability {
    pub const ALL: &'static [Capability] = &[
        Capability::CodeGeneration,
        Capability::CodeRepair,
        Capability::Refactoring,
        Capability::Debugging,
        Capability::TestAuthoring,
        Capability::MultiFileEdit,
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
    ];

    pub fn slug(&self) -> &'static str {
        match self {
            Capability::CodeGeneration => "code_generation",
            Capability::CodeRepair => "code_repair",
            Capability::Refactoring => "refactoring",
            Capability::Debugging => "debugging",
            Capability::TestAuthoring => "test_authoring",
            Capability::MultiFileEdit => "multi_file_edit",
            Capability::CodeComprehension => "code_comprehension",
            Capability::ToolUse => "tool_use",
            Capability::Retrieval => "retrieval",
            Capability::Planning => "planning",
            Capability::LongHorizon => "long_horizon",
            Capability::WorkTask => "work_task",
            Capability::Communication => "communication",
            Capability::DataAnalysis => "data_analysis",
            Capability::SurfaceConformance => "surface_conformance",
            Capability::Safety => "safety",
        }
    }
}

/// Which VibeCody surface is under test.
///
/// VibeCody is one daemon behind many clients, so most surfaces ultimately
/// exercise the same agent loop. They still get separate variants because the
/// *transport* is where they break: a capability can be perfect in the CLI and
/// unreachable from the watch because a route forgot its bearer token.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Surface {
    /// `vibecli --exec` / `--agent`, run as a subprocess.
    Cli,
    /// The HTTP daemon (`vibecli --serve`) directly.
    Daemon,
    /// Agent Client Protocol over stdio (Zed, JetBrains, Neovim).
    Acp,
    /// Model Context Protocol server over stdio (Claude Desktop et al).
    Mcp,
    /// The TypeScript Agent SDK.
    Sdk,
    // The explicit renames below are not cosmetic. `rename_all = "snake_case"`
    // turns `VsCode` into `vs_code`, which disagrees with `slug()` — and since
    // reports aggregate by slug while suite files name surfaces by their serde
    // form, the mismatch would split one surface into two report rows that
    // each look half-measured. `surface_slugs_are_unique_and_round_trip` pins
    // the agreement.
    /// VS Code extension.
    #[serde(rename = "vscode")]
    VsCode,
    /// JetBrains plugin.
    #[serde(rename = "jetbrains")]
    JetBrains,
    /// Neovim plugin.
    Neovim,
    /// VibeCoder desktop app (Tauri).
    #[serde(rename = "vibecoder")]
    VibeCoder,
    /// VibeDesk desktop app (Tauri).
    #[serde(rename = "vibedesk")]
    VibeDesk,
    /// VibeAIChat desktop app (Tauri).
    #[serde(rename = "vibeaichat")]
    VibeAiChat,
    /// Flutter mobile client.
    Mobile,
    /// watchOS / Wear OS clients.
    Watch,
    /// The daemon's built-in web UI.
    Web,
    /// The GitHub Action.
    GithubAction,
}

impl Surface {
    pub const ALL: &'static [Surface] = &[
        Surface::Cli,
        Surface::Daemon,
        Surface::Acp,
        Surface::Mcp,
        Surface::Sdk,
        Surface::VsCode,
        Surface::JetBrains,
        Surface::Neovim,
        Surface::VibeCoder,
        Surface::VibeDesk,
        Surface::VibeAiChat,
        Surface::Mobile,
        Surface::Watch,
        Surface::Web,
        Surface::GithubAction,
    ];

    pub fn slug(&self) -> &'static str {
        match self {
            Surface::Cli => "cli",
            Surface::Daemon => "daemon",
            Surface::Acp => "acp",
            Surface::Mcp => "mcp",
            Surface::Sdk => "sdk",
            Surface::VsCode => "vscode",
            Surface::JetBrains => "jetbrains",
            Surface::Neovim => "neovim",
            Surface::VibeCoder => "vibecoder",
            Surface::VibeDesk => "vibedesk",
            Surface::VibeAiChat => "vibeaichat",
            Surface::Mobile => "mobile",
            Surface::Watch => "watch",
            Surface::Web => "web",
            Surface::GithubAction => "github_action",
        }
    }

    /// Surfaces that reach the agent loop through the HTTP daemon rather than
    /// owning their own copy of it. Capability tasks aimed at these are run
    /// against the daemon, and only their *transport contract* is checked
    /// separately — claiming otherwise would report the daemon's score as if
    /// each client had earned it independently.
    pub fn is_daemon_backed(&self) -> bool {
        match self {
            Surface::Sdk
            | Surface::VsCode
            | Surface::JetBrains
            | Surface::Neovim
            | Surface::VibeCoder
            | Surface::VibeDesk
            | Surface::VibeAiChat
            | Surface::Mobile
            | Surface::Watch
            | Surface::Web => true,
            Surface::Cli
            | Surface::Daemon
            | Surface::Acp
            | Surface::Mcp
            | Surface::GithubAction => false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Difficulty {
    Easy,
    Medium,
    Hard,
}

impl Difficulty {
    pub fn slug(&self) -> &'static str {
        match self {
            Difficulty::Easy => "easy",
            Difficulty::Medium => "medium",
            Difficulty::Hard => "hard",
        }
    }
}

/// Where a task came from — vendored in this repo, or imported from a
/// third-party dataset.
///
/// Imports carry their provenance so a report can state which numbers are
/// comparable to a published leaderboard and which are ours alone. Reporting
/// a mixed run as a single "SWE-bench score" would be a category error.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TaskSource {
    /// Authored in this repository, MIT-licensed with the rest of it.
    Vendored,
    /// Converted from a third-party dataset.
    Imported {
        /// Dataset id as declared in its manifest (e.g. `swebench_verified`).
        dataset: String,
        /// The dataset's own identifier for this row.
        instance_id: String,
        /// SPDX-ish licence string copied from the manifest.
        license: String,
    },
}

impl Default for TaskSource {
    fn default() -> Self {
        TaskSource::Vendored
    }
}

/// A file tree the task starts from.
///
/// `files` is inline content (readable in the suite YAML, good for small
/// fixtures); `dir` copies a directory that sits next to the suite file (good
/// for anything with binary content or more than a few files). Both may be
/// used, with `files` applied second so a suite can override one file of a
/// copied tree.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Fixture {
    /// Relative path → file contents, written verbatim.
    #[serde(default)]
    pub files: BTreeMap<String, String>,
    /// Directory to copy, resolved relative to the suite file's directory.
    #[serde(default)]
    pub dir: Option<PathBuf>,
    /// Run `git init` and commit the fixture, so the agent sees a clean tree
    /// and the grader can diff against it.
    #[serde(default)]
    pub git_init: bool,
    /// Commands to run after materialising, before the agent starts (e.g.
    /// `npm install`). Failure here is a task Error, never a Fail — the agent
    /// has not been asked to do anything yet.
    #[serde(default)]
    pub setup: Vec<crate::grade::CommandStep>,
}

impl Fixture {
    pub fn is_empty(&self) -> bool {
        self.files.is_empty() && self.dir.is_none() && self.setup.is_empty()
    }
}

/// Where the task's working directory comes from.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceMode {
    /// A throwaway directory built from the fixture. The default, and the only
    /// safe choice for anything that lets an agent write.
    #[default]
    Temp,
    /// The VibeCody repository itself, read-only by convention.
    ///
    /// This is what makes cross-surface conformance checkable at all: "every
    /// client sets the bearer header", "all three tauri.conf.json agree on the
    /// macOS floor", "every release artifact has a job in release.yml" are
    /// assertions about the shipped source, not about a model. Tasks using it
    /// must not be given a prompt — the runner refuses to hand an agent a
    /// writable checkout of its own source tree.
    RepoRoot,
}

/// Per-task limits. Absent means "use the suite default"; the runner resolves
/// them, and the resolved value is what the report records.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Limits {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_secs: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_turns: Option<usize>,
    /// Deny the agent network access for this task.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub no_network: Option<bool>,
}

/// A single evaluation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvalTask {
    /// Unique within a suite; `<suite>/<id>` is unique globally.
    pub id: String,
    /// One line, human-facing — this is what shows up in a failure report.
    pub title: String,
    pub capability: Capability,
    pub difficulty: Difficulty,
    /// Surfaces this task should run against. Empty means "the suite default".
    #[serde(default)]
    pub surfaces: Vec<Surface>,
    /// What the agent is asked to do.
    ///
    /// Defaults to empty because conformance tasks have no agent turn at all.
    /// [`crate::suite::Suite::validate`] rejects an empty prompt on any task
    /// whose grader *does* need one, so the default cannot quietly produce a
    /// task that asks nothing and grades something.
    #[serde(default)]
    pub prompt: String,
    /// Workspace the agent starts in.
    #[serde(default, skip_serializing_if = "Fixture::is_empty")]
    pub fixture: Fixture,
    /// How the result is judged.
    pub grader: crate::grade::Grader,
    #[serde(default)]
    pub limits: Limits,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(default)]
    pub source: TaskSource,
    /// External tools the task needs on PATH (`cargo`, `python3`, `node`).
    /// Missing ones make the task *skipped*, not failed — a machine without a
    /// Rust toolchain has not told us anything about the agent.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub requires: Vec<String>,
    #[serde(default)]
    pub workspace: WorkspaceMode,
}

/// A task paired with the suite it came from, which is how the runner and the
/// report refer to it. Carrying the suite id inline keeps `EvalTask` free of a
/// field that only makes sense once it has been loaded from somewhere.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskRef {
    pub suite: String,
    pub task: EvalTask,
}

impl TaskRef {
    /// Globally unique identifier, stable across runs. Report keys and
    /// baseline lookups both use this, so it must not embed a timestamp,
    /// a path, or anything else that varies between machines.
    pub fn key(&self) -> String {
        format!("{}/{}", self.suite, self.task.id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capability_slugs_are_unique_and_round_trip() {
        let mut seen = std::collections::BTreeSet::new();
        for cap in Capability::ALL {
            assert!(seen.insert(cap.slug()), "duplicate slug {}", cap.slug());
            // The slug and the serde representation must agree, or a report
            // aggregated by slug will not match a suite file that names the
            // capability by its serde name.
            let json = serde_json::to_string(cap).unwrap_or_default();
            assert_eq!(json, format!("\"{}\"", cap.slug()));
        }
        assert_eq!(seen.len(), Capability::ALL.len());
    }

    #[test]
    fn surface_slugs_are_unique_and_round_trip() {
        let mut seen = std::collections::BTreeSet::new();
        for s in Surface::ALL {
            assert!(seen.insert(s.slug()), "duplicate slug {}", s.slug());
            let json = serde_json::to_string(s).unwrap_or_default();
            assert_eq!(json, format!("\"{}\"", s.slug()));
        }
    }

    #[test]
    fn every_surface_is_classified_as_daemon_backed_or_not() {
        // The point of this test is that adding a Surface variant without
        // deciding how it reaches the agent loop fails to compile in
        // `is_daemon_backed`, and this asserts the classification is total.
        let backed = Surface::ALL.iter().filter(|s| s.is_daemon_backed()).count();
        let direct = Surface::ALL
            .iter()
            .filter(|s| !s.is_daemon_backed())
            .count();
        assert_eq!(backed + direct, Surface::ALL.len());
        assert!(backed > 0 && direct > 0);
    }

    #[test]
    fn task_key_is_suite_qualified() {
        let t = TaskRef {
            suite: "coding-core".to_string(),
            task: EvalTask {
                id: "borrow-fix".to_string(),
                title: "t".to_string(),
                capability: Capability::CodeRepair,
                difficulty: Difficulty::Easy,
                surfaces: vec![],
                prompt: "p".to_string(),
                fixture: Fixture::default(),
                grader: crate::grade::Grader::AlwaysSkip {
                    reason: "test".to_string(),
                },
                limits: Limits::default(),
                tags: vec![],
                source: TaskSource::Vendored,
                requires: vec![],
                workspace: WorkspaceMode::Temp,
            },
        };
        assert_eq!(t.key(), "coding-core/borrow-fix");
    }
}
