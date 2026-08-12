//! Reports: what a run produced, and what to do about it.
//!
//! The report is the product. A pass rate on its own is nearly useless for
//! deciding what to change, so everything here is built to answer "where is
//! VibeCody weak, and which part of the codebase does that point at" — the
//! capability × surface matrix is the main instrument, and the failure list
//! carries the evidence needed to act without re-running anything.
//!
//! One rule runs through all of it: **a rate over zero scored tasks is `None`,
//! not `0.0`.** A suite that skipped every task because no toolchain was
//! installed has measured nothing, and rendering that as 0% turns a broken
//! machine into a capability regression — the single easiest way for this
//! harness to send someone to fix code that was never broken.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::grade::{GradeResult, Verdict};
use crate::task::{Capability, Difficulty, Surface, TaskSource};

/// Outcome for one task on one surface.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskResult {
    /// `<suite>/<task-id>` — stable across runs and machines.
    pub key: String,
    pub suite: String,
    pub task_id: String,
    pub title: String,
    pub capability: Capability,
    pub difficulty: Difficulty,
    pub surface: Surface,
    pub verdict: Verdict,
    /// Partial credit where the grader expressed one. `None` means no numeric
    /// judgement was reached — never treat it as zero.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub score: Option<f64>,
    /// Full grader tree, kept so a failure can be diagnosed from the report
    /// alone rather than by re-running an expensive task.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grade: Option<GradeResult>,
    pub duration_ms: u64,
    /// Which configuration produced this — binary, provider, model.
    pub harness: String,
    #[serde(default)]
    pub source: TaskSource,
}

impl TaskResult {
    /// A report row identity that includes the surface, because the same task
    /// legitimately has different outcomes on different surfaces.
    pub fn row_key(&self) -> String {
        format!("{}@{}", self.key, self.surface.slug())
    }
}

/// Counts, kept as counts rather than a single rate.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tally {
    pub passed: usize,
    pub failed: usize,
    pub errored: usize,
    pub skipped: usize,
}

impl Tally {
    pub fn add(&mut self, verdict: &Verdict) {
        match verdict {
            Verdict::Pass => self.passed += 1,
            Verdict::Fail { .. } => self.failed += 1,
            Verdict::Error { .. } => self.errored += 1,
            Verdict::Skipped { .. } => self.skipped += 1,
        }
    }

    /// Tasks that actually produced a verdict about the agent.
    pub fn scored(&self) -> usize {
        self.passed + self.failed
    }

    pub fn total(&self) -> usize {
        self.passed + self.failed + self.errored + self.skipped
    }

    /// Pass rate over scored tasks, or `None` when nothing was scored.
    ///
    /// Returning `None` rather than `0.0` is deliberate and load-bearing: the
    /// difference between "failed everything" and "measured nothing" is the
    /// difference between a real regression and an unset API key.
    pub fn pass_rate(&self) -> Option<f64> {
        match self.scored() {
            0 => None,
            n => Some(self.passed as f64 / n as f64),
        }
    }

    /// Fraction of tasks that never reached a verdict. High values mean the
    /// run's headline number rests on a small sample and should be read as
    /// such.
    pub fn coverage(&self) -> Option<f64> {
        match self.total() {
            0 => None,
            n => Some(self.scored() as f64 / n as f64),
        }
    }
}

fn fmt_rate(rate: Option<f64>) -> String {
    match rate {
        Some(r) => format!("{:.0}%", r * 100.0),
        // Not "0%", and not "-". The reader must be able to tell that the
        // harness declined to produce a number here.
        None => "n/a".to_string(),
    }
}

/// The configuration a run was performed under, recorded so a number is never
/// quoted without the setup that produced it.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct RunConfigSummary {
    pub provider: String,
    pub model: String,
    pub surfaces: Vec<Surface>,
    pub suites: Vec<String>,
    pub concurrency: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub judge: Option<String>,
    #[serde(default)]
    pub harnesses: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvalReport {
    pub run_id: String,
    pub started_at_unix: u64,
    pub finished_at_unix: u64,
    pub config: RunConfigSummary,
    pub results: Vec<TaskResult>,
    /// Suites that failed to load, surfaced in the report rather than only on
    /// stderr — a run missing half its suites should say so in its artefact.
    #[serde(default)]
    pub load_errors: Vec<String>,
}

impl EvalReport {
    pub fn overall(&self) -> Tally {
        self.results.iter().fold(Tally::default(), |mut t, r| {
            t.add(&r.verdict);
            t
        })
    }

    fn tally_by<K: Ord, F: Fn(&TaskResult) -> K>(&self, key: F) -> BTreeMap<K, Tally> {
        self.results.iter().fold(BTreeMap::new(), |mut acc, r| {
            acc.entry(key(r)).or_default().add(&r.verdict);
            acc
        })
    }

    pub fn by_capability(&self) -> BTreeMap<Capability, Tally> {
        self.tally_by(|r| r.capability)
    }

    pub fn by_surface(&self) -> BTreeMap<Surface, Tally> {
        self.tally_by(|r| r.surface)
    }

    pub fn by_suite(&self) -> BTreeMap<String, Tally> {
        self.tally_by(|r| r.suite.clone())
    }

    pub fn by_difficulty(&self) -> BTreeMap<Difficulty, Tally> {
        self.tally_by(|r| r.difficulty)
    }

    /// The capability × surface matrix — the view that answers "is this a
    /// model problem or a transport problem".
    pub fn matrix(&self) -> BTreeMap<(Capability, Surface), Tally> {
        self.tally_by(|r| (r.capability, r.surface))
    }

    pub fn failures(&self) -> Vec<&TaskResult> {
        self.results
            .iter()
            .filter(|r| matches!(r.verdict, Verdict::Fail { .. }))
            .collect()
    }

    pub fn errors(&self) -> Vec<&TaskResult> {
        self.results
            .iter()
            .filter(|r| matches!(r.verdict, Verdict::Error { .. }))
            .collect()
    }

    /// Capabilities ranked worst-first, ignoring ones with no scored tasks.
    ///
    /// This is the "what should I fix next" list, and it deliberately excludes
    /// unmeasured capabilities instead of ranking them at the bottom: an
    /// unmeasured capability is a gap in the *suite*, reported separately.
    pub fn weakest_capabilities(&self) -> Vec<(Capability, Tally, f64)> {
        let mut ranked: Vec<(Capability, Tally, f64)> = self
            .by_capability()
            .into_iter()
            .filter_map(|(cap, tally)| tally.pass_rate().map(|rate| (cap, tally, rate)))
            .collect();
        ranked.sort_by(|a, b| {
            a.2.partial_cmp(&b.2)
                .unwrap_or(std::cmp::Ordering::Equal)
                // Break ties by sample size: a 50% over 20 tasks is a stronger
                // signal than a 50% over 2.
                .then(b.1.scored().cmp(&a.1.scored()))
        });
        ranked
    }

    /// Capabilities the suite set never actually measured. A gap here is a
    /// task-authoring bug, not a capability finding, and the report says so
    /// rather than letting silence read as success.
    pub fn unmeasured_capabilities(&self) -> Vec<Capability> {
        let measured = self.by_capability();
        Capability::ALL
            .iter()
            .copied()
            .filter(|cap| measured.get(cap).and_then(Tally::pass_rate).is_none())
            .collect()
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    pub fn to_markdown(&self) -> String {
        let overall = self.overall();
        let mut md = String::with_capacity(8192);

        md.push_str(&format!("# VibeCody Eval Report — `{}`\n\n", self.run_id));
        md.push_str(&format!(
            "- **Provider/model:** {} / {}\n",
            blank_as_unset(&self.config.provider),
            blank_as_unset(&self.config.model)
        ));
        md.push_str(&format!(
            "- **Suites:** {}\n",
            if self.config.suites.is_empty() {
                "(none)".to_string()
            } else {
                self.config.suites.join(", ")
            }
        ));
        md.push_str(&format!(
            "- **Surfaces:** {}\n",
            self.config
                .surfaces
                .iter()
                .map(|s| s.slug())
                .collect::<Vec<_>>()
                .join(", ")
        ));
        md.push_str(&format!(
            "- **Judge:** {}\n",
            self.config
                .judge
                .as_deref()
                .unwrap_or("none (rubric tasks skipped)")
        ));
        md.push_str(&format!(
            "- **Duration:** {}s\n\n",
            self.finished_at_unix.saturating_sub(self.started_at_unix)
        ));

        md.push_str("## Headline\n\n");
        md.push_str(&format!(
            "**Pass rate: {}** ({} passed / {} scored)\n\n",
            fmt_rate(overall.pass_rate()),
            overall.passed,
            overall.scored()
        ));
        md.push_str(&format!(
            "| passed | failed | errored | skipped | total | coverage |\n\
             |-------:|-------:|--------:|--------:|------:|---------:|\n\
             | {} | {} | {} | {} | {} | {} |\n\n",
            overall.passed,
            overall.failed,
            overall.errored,
            overall.skipped,
            overall.total(),
            fmt_rate(overall.coverage())
        ));

        if overall.scored() == 0 {
            md.push_str(
                "> **No task produced a verdict.** This run measured nothing about \
                 VibeCody — it is an environment result, not a capability result. \
                 See the errors and skips below before reading anything else.\n\n",
            );
        } else if overall.coverage().is_some_and(|c| c < 0.6) {
            md.push_str(&format!(
                "> **Only {} of tasks were scored.** The headline rests on a partial \
                 sample; the skipped and errored tasks below say why.\n\n",
                fmt_rate(overall.coverage())
            ));
        }

        md.push_str(&section_table("By capability", self.by_capability(), |c| {
            c.slug().to_string()
        }));
        md.push_str(&section_table("By surface", self.by_surface(), |s| {
            s.slug().to_string()
        }));
        md.push_str(&section_table("By suite", self.by_suite(), |s| s));
        md.push_str(&section_table("By difficulty", self.by_difficulty(), |d| {
            d.slug().to_string()
        }));

        md.push_str(&self.matrix_markdown());
        md.push_str(&self.action_markdown());
        md.push_str(&self.failure_markdown());

        if !self.load_errors.is_empty() {
            md.push_str("## Suites that failed to load\n\n");
            for e in &self.load_errors {
                md.push_str(&format!("- {}\n", e));
            }
            md.push('\n');
        }

        md
    }

    fn matrix_markdown(&self) -> String {
        let matrix = self.matrix();
        if matrix.is_empty() {
            return String::new();
        }
        let surfaces: Vec<Surface> = {
            let mut s: Vec<Surface> = matrix.keys().map(|(_, surface)| *surface).collect();
            s.sort();
            s.dedup();
            s
        };
        let capabilities: Vec<Capability> = {
            let mut c: Vec<Capability> = matrix.keys().map(|(cap, _)| *cap).collect();
            c.sort();
            c.dedup();
            c
        };

        let mut md = String::from("## Capability × surface\n\n");
        md.push_str(
            "A capability that scores well on one surface and badly on another is a \
             transport problem, not a model problem. `n/a` means no task was scored \
             in that cell.\n\n",
        );
        md.push_str("| capability |");
        for s in &surfaces {
            md.push_str(&format!(" {} |", s.slug()));
        }
        md.push_str("\n|---|");
        for _ in &surfaces {
            md.push_str("---:|");
        }
        md.push('\n');
        for cap in &capabilities {
            md.push_str(&format!("| {} |", cap.slug()));
            for surface in &surfaces {
                let cell = matrix
                    .get(&(*cap, *surface))
                    .map(|t| format!("{} ({}/{})", fmt_rate(t.pass_rate()), t.passed, t.scored()))
                    .unwrap_or_else(|| "—".to_string());
                md.push_str(&format!(" {} |", cell));
            }
            md.push('\n');
        }
        md.push('\n');
        md
    }

    /// The part of the report that is meant to change what gets built next.
    fn action_markdown(&self) -> String {
        let mut md = String::from("## What to fix\n\n");
        let weakest = self.weakest_capabilities();
        if weakest.is_empty() {
            md.push_str("No capability produced a scored result, so there is nothing to rank.\n\n");
        } else {
            md.push_str("| capability | pass rate | scored | where this points |\n");
            md.push_str("|---|---:|---:|---|\n");
            for (cap, tally, rate) in weakest.iter().take(6) {
                md.push_str(&format!(
                    "| {} | {} | {} | {} |\n",
                    cap.slug(),
                    fmt_rate(Some(*rate)),
                    tally.scored(),
                    remediation_hint(*cap)
                ));
            }
            md.push('\n');
        }

        let unmeasured = self.unmeasured_capabilities();
        if !unmeasured.is_empty() {
            md.push_str(&format!(
                "**Unmeasured capabilities ({}):** {}. These are gaps in the suites, \
                 not results — no claim about them can be read out of this run.\n\n",
                unmeasured.len(),
                unmeasured
                    .iter()
                    .map(|c| c.slug())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        md
    }

    fn failure_markdown(&self) -> String {
        let failures = self.failures();
        let errors = self.errors();
        let mut md = String::new();

        if !failures.is_empty() {
            md.push_str(&format!("## Failures ({})\n\n", failures.len()));
            for r in failures.iter().take(50) {
                md.push_str(&format!(
                    "### `{}` on {}\n\n{}\n\n",
                    r.key,
                    r.surface.slug(),
                    r.title
                ));
                if let Some(reason) = r.verdict.reason() {
                    md.push_str(&format!("**Why:** {}\n\n", reason));
                }
                if let Some(grade) = &r.grade {
                    md.push_str(&grade_tree_markdown(grade, 0));
                    md.push('\n');
                }
            }
            if failures.len() > 50 {
                md.push_str(&format!(
                    "_…and {} more; the full list is in the JSON report._\n\n",
                    failures.len() - 50
                ));
            }
        }

        if !errors.is_empty() {
            md.push_str(&format!("## Errors ({})\n\n", errors.len()));
            md.push_str(
                "These tasks reached no verdict. They are excluded from the pass rate \
                 because they say nothing about the agent.\n\n",
            );
            for r in errors.iter().take(50) {
                md.push_str(&format!(
                    "- `{}` on {} — {}\n",
                    r.key,
                    r.surface.slug(),
                    r.verdict.reason().unwrap_or("(no reason recorded)")
                ));
            }
            md.push('\n');
        }
        md
    }
}

fn blank_as_unset(s: &str) -> &str {
    if s.trim().is_empty() {
        "(unset)"
    } else {
        s
    }
}

fn section_table<K: Ord, F: Fn(K) -> String>(
    title: &str,
    tallies: BTreeMap<K, Tally>,
    label: F,
) -> String {
    if tallies.is_empty() {
        return String::new();
    }
    let mut md = format!("## {}\n\n", title);
    md.push_str("| | pass rate | passed | failed | errored | skipped |\n");
    md.push_str("|---|---:|---:|---:|---:|---:|\n");
    for (key, t) in tallies {
        md.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} |\n",
            label(key),
            fmt_rate(t.pass_rate()),
            t.passed,
            t.failed,
            t.errored,
            t.skipped
        ));
    }
    md.push('\n');
    md
}

fn grade_tree_markdown(node: &GradeResult, depth: usize) -> String {
    let indent = "  ".repeat(depth);
    let icon = match node.verdict {
        Verdict::Pass => "✓",
        Verdict::Fail { .. } => "✗",
        Verdict::Error { .. } => "!",
        Verdict::Skipped { .. } => "–",
    };
    let mut md = format!("{}- {} `{}`", indent, icon, node.label);
    if let Some(reason) = node.verdict.reason() {
        md.push_str(&format!(" — {}", reason));
    }
    md.push('\n');
    if let Some(evidence) = &node.evidence {
        // Only failing evidence is worth the space; a passing command's output
        // is noise in a failure report.
        if !node.verdict.is_pass() {
            md.push_str(&format!("{}  ```\n{}\n{}  ```\n", indent, evidence, indent));
        }
    }
    for child in &node.children {
        md.push_str(&grade_tree_markdown(child, depth + 1));
    }
    md
}

/// Where a weak capability most likely points in this codebase.
///
/// A hint, explicitly not a diagnosis — it exists so the report ends with a
/// next step rather than a number.
fn remediation_hint(cap: Capability) -> &'static str {
    match cap {
        Capability::CodeGeneration | Capability::CodeComprehension => {
            "system prompt + context assembly (`vibe-ai` prompts, `context_assembler.rs`)"
        }
        Capability::CodeRepair | Capability::Debugging => {
            "tool loop: test-running and error feedback in `vibe-ai::agent`"
        }
        Capability::Refactoring | Capability::MultiFileEdit => {
            "edit tooling — `ast_edit.rs`, multi-file patch application"
        }
        Capability::TestAuthoring => "test-authoring prompts and skill files under `skills/`",
        Capability::ToolUse => "tool schemas and descriptions in `vibe-ai::tools`",
        Capability::Retrieval => "indexing and search — `vibe-indexer`, `kodegraph`, `/semindex`",
        Capability::Planning | Capability::LongHorizon => {
            "plan mode and goal tracking (`--plan`, `exec_goal_repl.rs`, `/v1/goals`)"
        }
        Capability::WorkTask | Capability::Communication => {
            "REPL integrations (`/email`, `/cal`, `/jira`, `/linear`) and their prompts"
        }
        Capability::DataAnalysis => "bash/file tooling and output truncation limits",
        Capability::SurfaceConformance => {
            "the client transport — auth headers, route wiring, daemon bootstrap"
        }
        Capability::Safety => "approval policy, tainted-data gates, `sandbox_policy.rs`",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn result(key: &str, cap: Capability, surface: Surface, verdict: Verdict) -> TaskResult {
        TaskResult {
            key: key.to_string(),
            suite: "s".to_string(),
            task_id: key.to_string(),
            title: "t".to_string(),
            capability: cap,
            difficulty: Difficulty::Easy,
            surface,
            verdict,
            score: None,
            grade: None,
            duration_ms: 1,
            harness: "test".to_string(),
            source: TaskSource::Vendored,
        }
    }

    fn report(results: Vec<TaskResult>) -> EvalReport {
        EvalReport {
            run_id: "run-1".to_string(),
            started_at_unix: 100,
            finished_at_unix: 200,
            config: RunConfigSummary::default(),
            results,
            load_errors: vec![],
        }
    }

    #[test]
    fn pass_rate_over_nothing_is_none_not_zero() {
        // The single most important assertion in this file.
        let t = Tally {
            skipped: 10,
            ..Tally::default()
        };
        assert_eq!(t.pass_rate(), None);
        assert_eq!(t.total(), 10);
        assert_eq!(t.scored(), 0);
    }

    #[test]
    fn errors_and_skips_stay_out_of_the_denominator() {
        let t = Tally {
            passed: 3,
            failed: 1,
            errored: 5,
            skipped: 11,
        };
        assert_eq!(t.scored(), 4);
        assert_eq!(t.pass_rate(), Some(0.75));
        assert_eq!(t.coverage(), Some(0.2));
    }

    #[test]
    fn a_measured_nothing_run_says_so_instead_of_showing_zero_percent() {
        let r = report(vec![result(
            "a",
            Capability::CodeRepair,
            Surface::Cli,
            Verdict::Skipped {
                reason: "no cargo".into(),
            },
        )]);
        let md = r.to_markdown();
        assert!(md.contains("n/a"), "should not render a rate");
        assert!(!md.contains("Pass rate: 0%"));
        assert!(md.contains("measured nothing"), "{}", md);
    }

    #[test]
    fn weakest_capabilities_ranks_worst_first_and_skips_unmeasured() {
        let r = report(vec![
            result(
                "a",
                Capability::CodeRepair,
                Surface::Cli,
                Verdict::Fail { reason: "x".into() },
            ),
            result(
                "b",
                Capability::CodeRepair,
                Surface::Cli,
                Verdict::Fail { reason: "x".into() },
            ),
            result("c", Capability::ToolUse, Surface::Cli, Verdict::Pass),
            // Only skipped → unmeasured, must not appear in the ranking.
            result(
                "d",
                Capability::Safety,
                Surface::Cli,
                Verdict::Skipped { reason: "x".into() },
            ),
        ]);
        let ranked = r.weakest_capabilities();
        assert_eq!(ranked[0].0, Capability::CodeRepair);
        assert_eq!(ranked[0].2, 0.0);
        assert!(!ranked.iter().any(|(c, _, _)| *c == Capability::Safety));
        assert!(r.unmeasured_capabilities().contains(&Capability::Safety));
    }

    #[test]
    fn ties_are_broken_by_sample_size() {
        let mut results = vec![];
        // ToolUse: 1 pass 1 fail over 2 tasks.
        results.push(result(
            "a",
            Capability::ToolUse,
            Surface::Cli,
            Verdict::Pass,
        ));
        results.push(result(
            "b",
            Capability::ToolUse,
            Surface::Cli,
            Verdict::Fail { reason: "x".into() },
        ));
        // CodeRepair: 2 pass 2 fail over 4 tasks — same rate, stronger signal.
        for i in 0..2 {
            results.push(result(
                &format!("c{}", i),
                Capability::CodeRepair,
                Surface::Cli,
                Verdict::Pass,
            ));
            results.push(result(
                &format!("d{}", i),
                Capability::CodeRepair,
                Surface::Cli,
                Verdict::Fail { reason: "x".into() },
            ));
        }
        let r = report(results);
        let ranked = r.weakest_capabilities();
        assert_eq!(
            ranked[0].0,
            Capability::CodeRepair,
            "larger sample first at equal rate"
        );
    }

    #[test]
    fn matrix_separates_surfaces() {
        let r = report(vec![
            result("a", Capability::CodeRepair, Surface::Cli, Verdict::Pass),
            result(
                "a",
                Capability::CodeRepair,
                Surface::Daemon,
                Verdict::Fail { reason: "x".into() },
            ),
        ]);
        let m = r.matrix();
        assert_eq!(
            m[&(Capability::CodeRepair, Surface::Cli)].pass_rate(),
            Some(1.0)
        );
        assert_eq!(
            m[&(Capability::CodeRepair, Surface::Daemon)].pass_rate(),
            Some(0.0)
        );
        let md = r.to_markdown();
        assert!(md.contains("Capability × surface"));
    }

    #[test]
    fn report_round_trips_through_json() {
        let r = report(vec![result(
            "a",
            Capability::ToolUse,
            Surface::Cli,
            Verdict::Pass,
        )]);
        let json = r.to_json().expect("serialize");
        let back: EvalReport = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, r);
    }

    #[test]
    fn low_coverage_is_called_out() {
        let mut results = vec![result(
            "a",
            Capability::ToolUse,
            Surface::Cli,
            Verdict::Pass,
        )];
        for i in 0..9 {
            results.push(result(
                &format!("s{}", i),
                Capability::ToolUse,
                Surface::Cli,
                Verdict::Skipped {
                    reason: "no toolchain".into(),
                },
            ));
        }
        let md = report(results).to_markdown();
        assert!(
            md.contains("partial \nsample") || md.contains("partial"),
            "{}",
            md
        );
    }
}
