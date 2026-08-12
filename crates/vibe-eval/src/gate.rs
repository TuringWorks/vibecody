//! Comparing runs, and gating on the comparison.
//!
//! A pass rate is a weak signal on its own — it moves with sample size,
//! provider latency, and which tasks happened to be skipped. What is worth
//! gating on is *change at the task level*: this specific task passed last
//! week and fails today.
//!
//! The subtle failure mode this module exists to prevent: the cheapest way to
//! make a gate green is to stop measuring. A task that goes from `pass` to
//! `skipped` raises no regression under naive comparison, and its removal
//! from the denominator can even push the headline rate *up*. So coverage loss
//! is tracked as its own category and can fail the gate on its own.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::grade::Verdict;
use crate::report::EvalReport;

/// One task's verdict changing between two runs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Change {
    /// `<suite>/<task>@<surface>`.
    pub row: String,
    pub title: String,
    pub before: String,
    pub after: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Comparison {
    pub baseline_run: String,
    pub current_run: String,
    /// Passed before, fails now. The thing gates exist for.
    pub regressions: Vec<Change>,
    /// Failed before, passes now.
    pub fixes: Vec<Change>,
    /// Produced a verdict before, produces none now — skipped or errored.
    /// Not a regression and emphatically not a fix: it is the measurement
    /// disappearing, which is why it is gated separately.
    pub coverage_losses: Vec<Change>,
    /// Was unmeasured before and is measured now.
    pub coverage_gains: Vec<Change>,
    pub added_rows: Vec<String>,
    pub removed_rows: Vec<String>,
    /// `None` when either side scored nothing — a delta against an unmeasured
    /// baseline is not a number.
    pub pass_rate_before: Option<f64>,
    pub pass_rate_after: Option<f64>,
}

impl Comparison {
    pub fn pass_rate_delta(&self) -> Option<f64> {
        match (self.pass_rate_before, self.pass_rate_after) {
            (Some(before), Some(after)) => Some(after - before),
            _ => None,
        }
    }

    pub fn is_clean(&self) -> bool {
        self.regressions.is_empty() && self.coverage_losses.is_empty()
    }
}

/// Compare a current report against a baseline.
pub fn compare(baseline: &EvalReport, current: &EvalReport) -> Comparison {
    let index = |report: &EvalReport| -> BTreeMap<String, (Verdict, String)> {
        report
            .results
            .iter()
            .map(|r| (r.row_key(), (r.verdict.clone(), r.title.clone())))
            .collect()
    };
    let before = index(baseline);
    let after = index(current);

    let mut regressions = Vec::new();
    let mut fixes = Vec::new();
    let mut coverage_losses = Vec::new();
    let mut coverage_gains = Vec::new();

    for (row, (new_verdict, title)) in &after {
        let Some((old_verdict, _)) = before.get(row) else {
            continue;
        };
        let change = |reason: Option<&str>| Change {
            row: row.clone(),
            title: title.clone(),
            before: old_verdict.label().to_string(),
            after: new_verdict.label().to_string(),
            reason: reason.map(str::to_string),
        };
        match (old_verdict.is_scored(), new_verdict.is_scored()) {
            (true, true) => match (old_verdict.is_pass(), new_verdict.is_pass()) {
                (true, false) => regressions.push(change(new_verdict.reason())),
                (false, true) => fixes.push(change(None)),
                _ => {}
            },
            // Measured before, unmeasured now. The gate's blind spot if it is
            // not tracked: making a task skip is indistinguishable from fixing
            // it, in every metric except this one.
            (true, false) => coverage_losses.push(change(new_verdict.reason())),
            (false, true) => coverage_gains.push(change(None)),
            (false, false) => {}
        }
    }

    let added_rows = after
        .keys()
        .filter(|k| !before.contains_key(*k))
        .cloned()
        .collect();
    let removed_rows = before
        .keys()
        .filter(|k| !after.contains_key(*k))
        .cloned()
        .collect();

    regressions.sort_by(|a, b| a.row.cmp(&b.row));
    fixes.sort_by(|a, b| a.row.cmp(&b.row));
    coverage_losses.sort_by(|a, b| a.row.cmp(&b.row));
    coverage_gains.sort_by(|a, b| a.row.cmp(&b.row));

    Comparison {
        baseline_run: baseline.run_id.clone(),
        current_run: current.run_id.clone(),
        regressions,
        fixes,
        coverage_losses,
        coverage_gains,
        added_rows,
        removed_rows,
        pass_rate_before: baseline.overall().pass_rate(),
        pass_rate_after: current.overall().pass_rate(),
    }
}

/// What the gate is allowed to tolerate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GatePolicy {
    /// Tasks allowed to go pass → fail.
    pub max_regressions: usize,
    /// Tasks allowed to go measured → unmeasured.
    pub max_coverage_losses: usize,
    /// Absolute floor on the current run's pass rate.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_pass_rate: Option<f64>,
    /// Minimum fraction of tasks that must reach a verdict. Guards against a
    /// run that is green because almost nothing ran.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_coverage: Option<f64>,
    /// Whether a run that scored nothing at all fails.
    pub fail_on_empty_run: bool,
}

impl Default for GatePolicy {
    fn default() -> Self {
        Self {
            // Zero by default: a regression is the whole reason to have a gate.
            max_regressions: 0,
            max_coverage_losses: 0,
            min_pass_rate: None,
            min_coverage: None,
            // A run that measured nothing must never be reported as a pass.
            fail_on_empty_run: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GateOutcome {
    pub passed: bool,
    /// Every reason the gate failed, not just the first.
    pub violations: Vec<String>,
    pub notes: Vec<String>,
}

impl GateOutcome {
    /// `0` when the gate passes, `1` when it fails. Distinct from the
    /// harness's own error codes so CI can tell "the evals regressed" from
    /// "the eval run itself broke".
    pub fn exit_code(&self) -> i32 {
        if self.passed {
            0
        } else {
            1
        }
    }
}

/// Apply a policy to a report, optionally against a baseline.
pub fn evaluate(
    current: &EvalReport,
    comparison: Option<&Comparison>,
    policy: &GatePolicy,
) -> GateOutcome {
    let overall = current.overall();
    let mut violations = Vec::new();
    let mut notes = Vec::new();

    if policy.fail_on_empty_run && overall.scored() == 0 {
        violations.push(format!(
            "no task produced a verdict ({} skipped, {} errored) — this run measured nothing",
            overall.skipped, overall.errored
        ));
    }

    match (policy.min_pass_rate, overall.pass_rate()) {
        (Some(floor), Some(rate)) if rate < floor => violations.push(format!(
            "pass rate {:.1}% is below the {:.1}% floor",
            rate * 100.0,
            floor * 100.0
        )),
        (Some(floor), None) => notes.push(format!(
            "pass-rate floor of {:.1}% could not be checked: nothing was scored",
            floor * 100.0
        )),
        _ => {}
    }

    match (policy.min_coverage, overall.coverage()) {
        (Some(floor), Some(cov)) if cov < floor => violations.push(format!(
            "only {:.1}% of tasks were scored, below the {:.1}% coverage floor",
            cov * 100.0,
            floor * 100.0
        )),
        _ => {}
    }

    if let Some(cmp) = comparison {
        if cmp.regressions.len() > policy.max_regressions {
            violations.push(format!(
                "{} regression(s), more than the {} allowed: {}",
                cmp.regressions.len(),
                policy.max_regressions,
                cmp.regressions
                    .iter()
                    .take(10)
                    .map(|c| c.row.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        if cmp.coverage_losses.len() > policy.max_coverage_losses {
            violations.push(format!(
                "{} task(s) stopped being measured, more than the {} allowed: {}. \
                 Silently skipping a task is not the same as fixing it.",
                cmp.coverage_losses.len(),
                policy.max_coverage_losses,
                cmp.coverage_losses
                    .iter()
                    .take(10)
                    .map(|c| c.row.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        if !cmp.fixes.is_empty() {
            notes.push(format!("{} task(s) newly pass", cmp.fixes.len()));
        }
        if !cmp.added_rows.is_empty() {
            notes.push(format!(
                "{} new task row(s) not in the baseline",
                cmp.added_rows.len()
            ));
        }
    } else {
        notes.push("no baseline supplied — only absolute thresholds were checked".to_string());
    }

    GateOutcome {
        passed: violations.is_empty(),
        violations,
        notes,
    }
}

/// Render a comparison for a human or a CI log.
pub fn comparison_markdown(cmp: &Comparison) -> String {
    let mut md = format!(
        "# Eval comparison — `{}` → `{}`\n\n",
        cmp.baseline_run, cmp.current_run
    );
    let rate = |r: Option<f64>| match r {
        Some(v) => format!("{:.1}%", v * 100.0),
        None => "n/a".to_string(),
    };
    md.push_str(&format!(
        "- **Pass rate:** {} → {}{}\n",
        rate(cmp.pass_rate_before),
        rate(cmp.pass_rate_after),
        match cmp.pass_rate_delta() {
            Some(d) => format!(" ({:+.1} pts)", d * 100.0),
            None => String::new(),
        }
    ));
    md.push_str(&format!("- **Regressions:** {}\n", cmp.regressions.len()));
    md.push_str(&format!("- **Fixes:** {}\n", cmp.fixes.len()));
    md.push_str(&format!(
        "- **Stopped being measured:** {}\n\n",
        cmp.coverage_losses.len()
    ));

    let table = |title: &str, changes: &[Change]| -> String {
        if changes.is_empty() {
            return String::new();
        }
        let mut s = format!(
            "## {} ({})\n\n| task | before → after | why |\n|---|---|---|\n",
            title,
            changes.len()
        );
        for c in changes {
            s.push_str(&format!(
                "| `{}` | {} → {} | {} |\n",
                c.row,
                c.before,
                c.after,
                c.reason.as_deref().unwrap_or("")
            ));
        }
        s.push('\n');
        s
    };
    md.push_str(&table("Regressions", &cmp.regressions));
    md.push_str(&table("Stopped being measured", &cmp.coverage_losses));
    md.push_str(&table("Fixes", &cmp.fixes));
    md.push_str(&table("Newly measured", &cmp.coverage_gains));
    md
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::report::{RunConfigSummary, TaskResult};
    use crate::task::{Capability, Difficulty, Surface, TaskSource};

    fn row(key: &str, verdict: Verdict) -> TaskResult {
        TaskResult {
            key: key.to_string(),
            suite: "s".into(),
            task_id: key.into(),
            title: format!("title of {}", key),
            capability: Capability::CodeRepair,
            difficulty: Difficulty::Easy,
            surface: Surface::Cli,
            verdict,
            score: None,
            grade: None,
            duration_ms: 1,
            harness: "h".into(),
            source: TaskSource::Vendored,
        }
    }

    fn report(id: &str, results: Vec<TaskResult>) -> EvalReport {
        EvalReport {
            run_id: id.to_string(),
            started_at_unix: 0,
            finished_at_unix: 1,
            config: RunConfigSummary::default(),
            results,
            load_errors: vec![],
        }
    }

    #[test]
    fn a_pass_becoming_a_fail_is_a_regression() {
        let base = report("base", vec![row("a", Verdict::Pass)]);
        let now = report(
            "now",
            vec![row(
                "a",
                Verdict::Fail {
                    reason: "broke".into(),
                },
            )],
        );
        let cmp = compare(&base, &now);
        assert_eq!(cmp.regressions.len(), 1);
        assert_eq!(cmp.regressions[0].reason.as_deref(), Some("broke"));
        assert!(!cmp.is_clean());
    }

    #[test]
    fn a_pass_becoming_a_skip_is_coverage_loss_not_a_fix() {
        // The gate's central blind spot. Making a task skip must never look
        // like progress.
        let base = report("base", vec![row("a", Verdict::Pass)]);
        let now = report(
            "now",
            vec![row(
                "a",
                Verdict::Skipped {
                    reason: "no cargo".into(),
                },
            )],
        );
        let cmp = compare(&base, &now);
        assert!(cmp.regressions.is_empty());
        assert!(cmp.fixes.is_empty());
        assert_eq!(cmp.coverage_losses.len(), 1);
        assert!(!cmp.is_clean(), "coverage loss must not be clean");
    }

    #[test]
    fn skipping_every_failure_cannot_produce_a_green_gate() {
        // Baseline: one pass, one fail → 50%.
        let base = report(
            "base",
            vec![
                row("a", Verdict::Pass),
                row("b", Verdict::Fail { reason: "x".into() }),
            ],
        );
        // Now: the failing task is skipped → headline jumps to 100%.
        let now = report(
            "now",
            vec![
                row("a", Verdict::Pass),
                row("b", Verdict::Skipped { reason: "x".into() }),
            ],
        );
        let cmp = compare(&base, &now);
        assert_eq!(cmp.pass_rate_before, Some(0.5));
        assert_eq!(cmp.pass_rate_after, Some(1.0));
        assert!(cmp.pass_rate_delta().unwrap_or(0.0) > 0.0, "rate went up");

        // …and the gate still fails, because the improvement is fake.
        let outcome = evaluate(&now, Some(&cmp), &GatePolicy::default());
        assert!(!outcome.passed);
        assert!(
            outcome
                .violations
                .iter()
                .any(|v| v.contains("stopped being measured")),
            "{:?}",
            outcome.violations
        );
    }

    #[test]
    fn a_fail_becoming_a_pass_is_a_fix() {
        let base = report("base", vec![row("a", Verdict::Fail { reason: "x".into() })]);
        let now = report("now", vec![row("a", Verdict::Pass)]);
        let cmp = compare(&base, &now);
        assert_eq!(cmp.fixes.len(), 1);
        assert!(cmp.is_clean());
        assert!(evaluate(&now, Some(&cmp), &GatePolicy::default()).passed);
    }

    #[test]
    fn new_and_removed_rows_are_tracked_but_do_not_regress() {
        let base = report("base", vec![row("a", Verdict::Pass)]);
        let now = report("now", vec![row("b", Verdict::Pass)]);
        let cmp = compare(&base, &now);
        assert_eq!(cmp.added_rows.len(), 1);
        assert_eq!(cmp.removed_rows.len(), 1);
        assert!(cmp.regressions.is_empty());
    }

    #[test]
    fn an_empty_run_fails_the_gate_by_default() {
        let now = report(
            "now",
            vec![row("a", Verdict::Skipped { reason: "x".into() })],
        );
        let outcome = evaluate(&now, None, &GatePolicy::default());
        assert!(!outcome.passed);
        assert!(outcome.violations[0].contains("measured nothing"));
        assert_eq!(outcome.exit_code(), 1);
    }

    #[test]
    fn a_pass_rate_floor_that_cannot_be_checked_is_a_note_not_a_silent_pass() {
        let now = report(
            "now",
            vec![row("a", Verdict::Skipped { reason: "x".into() })],
        );
        let policy = GatePolicy {
            min_pass_rate: Some(0.8),
            fail_on_empty_run: false,
            ..GatePolicy::default()
        };
        let outcome = evaluate(&now, None, &policy);
        // It does not fail on the floor (nothing to compare), but it says so
        // rather than reporting a clean pass against an unchecked threshold.
        assert!(outcome
            .notes
            .iter()
            .any(|n| n.contains("could not be checked")));
    }

    #[test]
    fn a_coverage_floor_catches_a_mostly_skipped_run() {
        let mut results = vec![row("a", Verdict::Pass)];
        for i in 0..9 {
            results.push(row(
                &format!("s{}", i),
                Verdict::Skipped { reason: "x".into() },
            ));
        }
        let now = report("now", results);
        let policy = GatePolicy {
            min_coverage: Some(0.5),
            ..GatePolicy::default()
        };
        let outcome = evaluate(&now, None, &policy);
        assert!(!outcome.passed);
        assert!(outcome
            .violations
            .iter()
            .any(|v| v.contains("coverage floor")));
    }

    #[test]
    fn all_violations_are_reported_not_just_the_first() {
        let now = report("now", vec![row("a", Verdict::Fail { reason: "x".into() })]);
        let base = report("base", vec![row("a", Verdict::Pass)]);
        let cmp = compare(&base, &now);
        let policy = GatePolicy {
            min_pass_rate: Some(0.9),
            ..GatePolicy::default()
        };
        let outcome = evaluate(&now, Some(&cmp), &policy);
        assert!(outcome.violations.len() >= 2, "{:?}", outcome.violations);
    }

    #[test]
    fn comparison_markdown_mentions_the_categories() {
        let base = report("base", vec![row("a", Verdict::Pass)]);
        let now = report(
            "now",
            vec![row(
                "a",
                Verdict::Fail {
                    reason: "broke".into(),
                },
            )],
        );
        let md = comparison_markdown(&compare(&base, &now));
        assert!(md.contains("Regressions"));
        assert!(md.contains("broke"));
    }
}
