//! SWE-bench benchmarking harness.
//!
//! Built-in benchmark runner for evaluating agent performance on SWE-bench
//! and custom benchmark suites. Tracks runs, results, and generates reports.

use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum BenchmarkSuite {
    SWEBenchVerified,
    SWEBenchPro,
    SWEBenchLite,
    Custom(String),
}

impl BenchmarkSuite {
    pub fn name(&self) -> String {
        match self {
            BenchmarkSuite::SWEBenchVerified => "SWE-bench Verified".to_string(),
            BenchmarkSuite::SWEBenchPro => "SWE-bench Pro".to_string(),
            BenchmarkSuite::SWEBenchLite => "SWE-bench Lite".to_string(),
            BenchmarkSuite::Custom(name) => name.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Difficulty {
    Easy,
    Medium,
    Hard,
}

impl Difficulty {
    pub fn label(&self) -> &str {
        match self {
            Difficulty::Easy => "easy",
            Difficulty::Medium => "medium",
            Difficulty::Hard => "hard",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BenchmarkTask {
    pub id: String,
    pub repo: String,
    pub instance_id: String,
    pub problem_statement: String,
    pub expected_patch: String,
    pub test_patch: String,
    pub difficulty: Difficulty,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RunStatus {
    Pending,
    Running,
    Completed,
    Failed(String),
    Cancelled,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TaskResult {
    pub task_id: String,
    pub passed: bool,
    pub patch_generated: String,
    pub error: Option<String>,
    pub duration_ms: u64,
    pub tokens_used: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BenchmarkConfig {
    pub max_turns: usize,
    pub timeout_per_task_secs: u64,
    pub parallel_tasks: usize,
    pub model: String,
    pub provider: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BenchmarkRun {
    pub id: String,
    pub suite: BenchmarkSuite,
    pub provider: String,
    pub model: String,
    pub started_at: u64,
    pub completed_at: Option<u64>,
    pub status: RunStatus,
    pub results: Vec<TaskResult>,
    pub config: BenchmarkConfig,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BenchmarkReport {
    pub run_id: String,
    pub suite: String,
    pub model: String,
    pub pass_at_1: f64,
    pub total_tasks: usize,
    pub passed_tasks: usize,
    pub failed_tasks: usize,
    pub avg_duration_ms: u64,
    pub total_tokens: u64,
    pub cost_estimate_usd: f64,
    pub difficulty_breakdown: HashMap<String, (usize, usize)>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ComparisonReport {
    pub runs: Vec<BenchmarkReport>,
    pub best_run_id: String,
    pub best_pass_rate: f64,
}

#[derive(Debug, Clone)]
pub struct BenchmarkRunner {
    pub runs: Vec<BenchmarkRun>,
    pub tasks: HashMap<String, Vec<BenchmarkTask>>,
    next_run_id: u64,
}

impl BenchmarkRunner {
    pub fn new() -> Self {
        Self {
            runs: Vec::new(),
            tasks: HashMap::new(),
            next_run_id: 1,
        }
    }

    pub fn load_suite(&mut self, suite: &BenchmarkSuite, tasks: Vec<BenchmarkTask>) {
        self.tasks.insert(suite.name(), tasks);
    }

    pub fn create_run(&mut self, suite: BenchmarkSuite, config: BenchmarkConfig) -> String {
        let id = format!("run-{}", self.next_run_id);
        self.next_run_id += 1;
        let run = BenchmarkRun {
            id: id.clone(),
            provider: config.provider.clone(),
            model: config.model.clone(),
            suite,
            started_at: 0,
            completed_at: None,
            status: RunStatus::Pending,
            results: Vec::new(),
            config,
        };
        self.runs.push(run);
        id
    }

    pub fn start_run(&mut self, run_id: &str) -> Result<(), String> {
        let run = self
            .runs
            .iter_mut()
            .find(|r| r.id == run_id)
            .ok_or_else(|| format!("Run not found: {}", run_id))?;
        match &run.status {
            RunStatus::Pending => {
                run.status = RunStatus::Running;
                run.started_at = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                Ok(())
            }
            _ => Err(format!("Run {} is not in Pending state", run_id)),
        }
    }

    pub fn record_result(&mut self, run_id: &str, result: TaskResult) -> Result<(), String> {
        let run = self
            .runs
            .iter_mut()
            .find(|r| r.id == run_id)
            .ok_or_else(|| format!("Run not found: {}", run_id))?;
        match &run.status {
            RunStatus::Running => {
                run.results.push(result);
                Ok(())
            }
            _ => Err(format!("Run {} is not in Running state", run_id)),
        }
    }

    pub fn complete_run(&mut self, run_id: &str) -> Result<BenchmarkReport, String> {
        let run = self
            .runs
            .iter_mut()
            .find(|r| r.id == run_id)
            .ok_or_else(|| format!("Run not found: {}", run_id))?;
        match &run.status {
            RunStatus::Running => {
                run.status = RunStatus::Completed;
                run.completed_at = Some(
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs(),
                );
                Ok(Self::build_report(run, &self.tasks))
            }
            _ => Err(format!("Run {} is not in Running state", run_id)),
        }
    }

    pub fn cancel_run(&mut self, run_id: &str) -> Result<(), String> {
        let run = self
            .runs
            .iter_mut()
            .find(|r| r.id == run_id)
            .ok_or_else(|| format!("Run not found: {}", run_id))?;
        match &run.status {
            RunStatus::Pending | RunStatus::Running => {
                run.status = RunStatus::Cancelled;
                Ok(())
            }
            _ => Err(format!("Run {} cannot be cancelled in current state", run_id)),
        }
    }

    pub fn get_report(&self, run_id: &str) -> Option<BenchmarkReport> {
        let run = self.runs.iter().find(|r| r.id == run_id)?;
        if run.status != RunStatus::Completed {
            return None;
        }
        Some(Self::build_report(run, &self.tasks))
    }

    pub fn compare_runs(&self, run_ids: &[String]) -> ComparisonReport {
        let mut reports = Vec::new();
        let mut best_run_id = String::new();
        let mut best_pass_rate: f64 = -1.0;

        for rid in run_ids {
            if let Some(report) = self.get_report(rid) {
                if report.pass_at_1 > best_pass_rate {
                    best_pass_rate = report.pass_at_1;
                    best_run_id = report.run_id.clone();
                }
                reports.push(report);
            }
        }

        if best_pass_rate < 0.0 {
            best_pass_rate = 0.0;
        }

        ComparisonReport {
            runs: reports,
            best_run_id,
            best_pass_rate,
        }
    }

    pub fn list_runs(&self) -> Vec<&BenchmarkRun> {
        self.runs.iter().collect()
    }

    pub fn export_report_markdown(report: &BenchmarkReport) -> String {
        let mut md = String::with_capacity(1024);
        md.push_str(&format!("# Benchmark Report: {}\n\n", report.run_id));
        md.push_str(&format!("- **Suite:** {}\n", report.suite));
        md.push_str(&format!("- **Model:** {}\n", report.model));
        md.push_str(&format!("- **Pass@1:** {:.1}%\n", report.pass_at_1 * 100.0));
        md.push_str(&format!(
            "- **Tasks:** {} passed / {} total\n",
            report.passed_tasks, report.total_tasks
        ));
        md.push_str(&format!(
            "- **Avg Duration:** {}ms\n",
            report.avg_duration_ms
        ));
        md.push_str(&format!("- **Total Tokens:** {}\n", report.total_tokens));
        md.push_str(&format!(
            "- **Est. Cost:** ${:.4}\n\n",
            report.cost_estimate_usd
        ));

        if !report.difficulty_breakdown.is_empty() {
            md.push_str("## Difficulty Breakdown\n\n");
            md.push_str("| Difficulty | Passed | Total | Rate |\n");
            md.push_str("|-----------|--------|-------|------|\n");
            let mut keys: Vec<&String> = report.difficulty_breakdown.keys().collect();
            keys.sort();
            for key in keys {
                let (passed, total) = report.difficulty_breakdown[key];
                let rate = if total > 0 {
                    (passed as f64 / total as f64) * 100.0
                } else {
                    0.0
                };
                md.push_str(&format!(
                    "| {} | {} | {} | {:.1}% |\n",
                    key, passed, total, rate
                ));
            }
        }

        md
    }

    fn build_report(
        run: &BenchmarkRun,
        all_tasks: &HashMap<String, Vec<BenchmarkTask>>,
    ) -> BenchmarkReport {
        let total = run.results.len();
        let passed = run.results.iter().filter(|r| r.passed).count();
        let failed = total - passed;
        let pass_at_1 = if total > 0 {
            passed as f64 / total as f64
        } else {
            0.0
        };
        let total_duration: u64 = run.results.iter().map(|r| r.duration_ms).sum();
        let avg_duration_ms = if total > 0 {
            total_duration / total as u64
        } else {
            0
        };
        let total_tokens: u64 = run.results.iter().map(|r| r.tokens_used).sum();
        // Rough cost estimate: $0.003 per 1K tokens
        let cost_estimate_usd = (total_tokens as f64 / 1000.0) * 0.003;

        let mut difficulty_breakdown: HashMap<String, (usize, usize)> = HashMap::new();
        let suite_name = run.suite.name();
        if let Some(tasks) = all_tasks.get(&suite_name) {
            for result in &run.results {
                if let Some(task) = tasks.iter().find(|t| t.id == result.task_id) {
                    let label = task.difficulty.label().to_string();
                    let entry = difficulty_breakdown.entry(label).or_insert((0, 0));
                    entry.1 += 1;
                    if result.passed {
                        entry.0 += 1;
                    }
                }
            }
        }

        BenchmarkReport {
            run_id: run.id.clone(),
            suite: suite_name,
            model: run.model.clone(),
            pass_at_1,
            total_tasks: total,
            passed_tasks: passed,
            failed_tasks: failed,
            avg_duration_ms,
            total_tokens,
            cost_estimate_usd,
            difficulty_breakdown,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_tasks() -> Vec<BenchmarkTask> {
        vec![
            BenchmarkTask {
                id: "task-1".to_string(),
                repo: "django/django".to_string(),
                instance_id: "django__django-12345".to_string(),
                problem_statement: "Fix ORM query issue".to_string(),
                expected_patch: "--- a/foo\n+++ b/foo".to_string(),
                test_patch: "def test_fix(): assert True".to_string(),
                difficulty: Difficulty::Easy,
            },
            BenchmarkTask {
                id: "task-2".to_string(),
                repo: "django/django".to_string(),
                instance_id: "django__django-67890".to_string(),
                problem_statement: "Fix migration bug".to_string(),
                expected_patch: "--- a/bar\n+++ b/bar".to_string(),
                test_patch: "def test_migration(): assert True".to_string(),
                difficulty: Difficulty::Medium,
            },
            BenchmarkTask {
                id: "task-3".to_string(),
                repo: "scikit-learn/scikit-learn".to_string(),
                instance_id: "sklearn__sklearn-11111".to_string(),
                problem_statement: "Fix estimator serialization".to_string(),
                expected_patch: "--- a/baz\n+++ b/baz".to_string(),
                test_patch: "def test_serialize(): assert True".to_string(),
                difficulty: Difficulty::Hard,
            },
        ]
    }

    fn sample_config() -> BenchmarkConfig {
        BenchmarkConfig {
            max_turns: 10,
            timeout_per_task_secs: 300,
            parallel_tasks: 2,
            model: "claude-opus-4-20250514".to_string(),
            provider: "anthropic".to_string(),
        }
    }

    #[test]
    fn test_new_runner_is_empty() {
        let runner = BenchmarkRunner::new();
        assert!(runner.runs.is_empty());
        assert!(runner.tasks.is_empty());
    }

    #[test]
    fn test_load_suite() {
        let mut runner = BenchmarkRunner::new();
        let suite = BenchmarkSuite::SWEBenchLite;
        runner.load_suite(&suite, sample_tasks());
        assert_eq!(runner.tasks.len(), 1);
        assert_eq!(runner.tasks["SWE-bench Lite"].len(), 3);
    }

    #[test]
    fn test_create_run_returns_unique_ids() {
        let mut runner = BenchmarkRunner::new();
        let id1 = runner.create_run(BenchmarkSuite::SWEBenchLite, sample_config());
        let id2 = runner.create_run(BenchmarkSuite::SWEBenchPro, sample_config());
        assert_ne!(id1, id2);
        assert_eq!(runner.runs.len(), 2);
    }

    #[test]
    fn test_create_run_status_is_pending() {
        let mut runner = BenchmarkRunner::new();
        let id = runner.create_run(BenchmarkSuite::SWEBenchVerified, sample_config());
        let run = runner.runs.iter().find(|r| r.id == id).unwrap();
        assert_eq!(run.status, RunStatus::Pending);
    }

    #[test]
    fn test_start_run() {
        let mut runner = BenchmarkRunner::new();
        let id = runner.create_run(BenchmarkSuite::SWEBenchLite, sample_config());
        assert!(runner.start_run(&id).is_ok());
        let run = runner.runs.iter().find(|r| r.id == id).unwrap();
        assert_eq!(run.status, RunStatus::Running);
        assert!(run.started_at > 0);
    }

    #[test]
    fn test_start_run_not_found() {
        let mut runner = BenchmarkRunner::new();
        assert!(runner.start_run("nonexistent").is_err());
    }

    #[test]
    fn test_start_run_already_running() {
        let mut runner = BenchmarkRunner::new();
        let id = runner.create_run(BenchmarkSuite::SWEBenchLite, sample_config());
        runner.start_run(&id).unwrap();
        assert!(runner.start_run(&id).is_err());
    }

    #[test]
    fn test_record_result() {
        let mut runner = BenchmarkRunner::new();
        let id = runner.create_run(BenchmarkSuite::SWEBenchLite, sample_config());
        runner.start_run(&id).unwrap();
        let result = TaskResult {
            task_id: "task-1".to_string(),
            passed: true,
            patch_generated: "some patch".to_string(),
            error: None,
            duration_ms: 5000,
            tokens_used: 1500,
        };
        assert!(runner.record_result(&id, result).is_ok());
        let run = runner.runs.iter().find(|r| r.id == id).unwrap();
        assert_eq!(run.results.len(), 1);
    }

    #[test]
    fn test_record_result_on_pending_fails() {
        let mut runner = BenchmarkRunner::new();
        let id = runner.create_run(BenchmarkSuite::SWEBenchLite, sample_config());
        let result = TaskResult {
            task_id: "task-1".to_string(),
            passed: true,
            patch_generated: String::new(),
            error: None,
            duration_ms: 100,
            tokens_used: 50,
        };
        assert!(runner.record_result(&id, result).is_err());
    }

    #[test]
    fn test_complete_run() {
        let mut runner = BenchmarkRunner::new();
        let suite = BenchmarkSuite::SWEBenchLite;
        runner.load_suite(&suite, sample_tasks());
        let id = runner.create_run(suite, sample_config());
        runner.start_run(&id).unwrap();
        runner
            .record_result(
                &id,
                TaskResult {
                    task_id: "task-1".to_string(),
                    passed: true,
                    patch_generated: "p".to_string(),
                    error: None,
                    duration_ms: 1000,
                    tokens_used: 500,
                },
            )
            .unwrap();
        runner
            .record_result(
                &id,
                TaskResult {
                    task_id: "task-2".to_string(),
                    passed: false,
                    patch_generated: String::new(),
                    error: Some("timeout".to_string()),
                    duration_ms: 3000,
                    tokens_used: 800,
                },
            )
            .unwrap();
        let report = runner.complete_run(&id).unwrap();
        assert_eq!(report.total_tasks, 2);
        assert_eq!(report.passed_tasks, 1);
        assert_eq!(report.failed_tasks, 1);
        assert!((report.pass_at_1 - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_complete_run_not_running_fails() {
        let mut runner = BenchmarkRunner::new();
        let id = runner.create_run(BenchmarkSuite::SWEBenchLite, sample_config());
        assert!(runner.complete_run(&id).is_err());
    }

    #[test]
    fn test_cancel_run_from_pending() {
        let mut runner = BenchmarkRunner::new();
        let id = runner.create_run(BenchmarkSuite::SWEBenchLite, sample_config());
        assert!(runner.cancel_run(&id).is_ok());
        let run = runner.runs.iter().find(|r| r.id == id).unwrap();
        assert_eq!(run.status, RunStatus::Cancelled);
    }

    #[test]
    fn test_cancel_run_from_running() {
        let mut runner = BenchmarkRunner::new();
        let id = runner.create_run(BenchmarkSuite::SWEBenchLite, sample_config());
        runner.start_run(&id).unwrap();
        assert!(runner.cancel_run(&id).is_ok());
    }

    #[test]
    fn test_cancel_completed_run_fails() {
        let mut runner = BenchmarkRunner::new();
        let id = runner.create_run(BenchmarkSuite::SWEBenchLite, sample_config());
        runner.start_run(&id).unwrap();
        runner.complete_run(&id).unwrap();
        assert!(runner.cancel_run(&id).is_err());
    }

    #[test]
    fn test_get_report_completed() {
        let mut runner = BenchmarkRunner::new();
        let id = runner.create_run(BenchmarkSuite::SWEBenchLite, sample_config());
        runner.start_run(&id).unwrap();
        runner.complete_run(&id).unwrap();
        assert!(runner.get_report(&id).is_some());
    }

    #[test]
    fn test_get_report_not_completed() {
        let mut runner = BenchmarkRunner::new();
        let id = runner.create_run(BenchmarkSuite::SWEBenchLite, sample_config());
        assert!(runner.get_report(&id).is_none());
    }

    #[test]
    fn test_compare_runs() {
        let mut runner = BenchmarkRunner::new();
        let id1 = runner.create_run(BenchmarkSuite::SWEBenchLite, sample_config());
        runner.start_run(&id1).unwrap();
        runner
            .record_result(
                &id1,
                TaskResult {
                    task_id: "t1".to_string(),
                    passed: true,
                    patch_generated: String::new(),
                    error: None,
                    duration_ms: 100,
                    tokens_used: 50,
                },
            )
            .unwrap();
        runner.complete_run(&id1).unwrap();

        let id2 = runner.create_run(BenchmarkSuite::SWEBenchLite, sample_config());
        runner.start_run(&id2).unwrap();
        runner
            .record_result(
                &id2,
                TaskResult {
                    task_id: "t1".to_string(),
                    passed: false,
                    patch_generated: String::new(),
                    error: None,
                    duration_ms: 200,
                    tokens_used: 100,
                },
            )
            .unwrap();
        runner.complete_run(&id2).unwrap();

        let cmp = runner.compare_runs(&[id1.clone(), id2.clone()]);
        assert_eq!(cmp.runs.len(), 2);
        assert_eq!(cmp.best_run_id, id1);
        assert!((cmp.best_pass_rate - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_compare_runs_empty() {
        let runner = BenchmarkRunner::new();
        let cmp = runner.compare_runs(&["nope".to_string()]);
        assert!(cmp.runs.is_empty());
        assert_eq!(cmp.best_pass_rate, 0.0);
    }

    #[test]
    fn test_list_runs() {
        let mut runner = BenchmarkRunner::new();
        runner.create_run(BenchmarkSuite::SWEBenchLite, sample_config());
        runner.create_run(BenchmarkSuite::SWEBenchPro, sample_config());
        assert_eq!(runner.list_runs().len(), 2);
    }

    #[test]
    fn test_export_report_markdown() {
        let mut breakdown = HashMap::new();
        breakdown.insert("easy".to_string(), (3, 4));
        breakdown.insert("hard".to_string(), (1, 2));
        let report = BenchmarkReport {
            run_id: "run-1".to_string(),
            suite: "SWE-bench Lite".to_string(),
            model: "claude-opus-4-20250514".to_string(),
            pass_at_1: 0.75,
            total_tasks: 6,
            passed_tasks: 4,
            failed_tasks: 2,
            avg_duration_ms: 2500,
            total_tokens: 10000,
            cost_estimate_usd: 0.03,
            difficulty_breakdown: breakdown,
        };
        let md = BenchmarkRunner::export_report_markdown(&report);
        assert!(md.contains("# Benchmark Report: run-1"));
        assert!(md.contains("75.0%"));
        assert!(md.contains("claude-opus-4-20250514"));
        assert!(md.contains("Difficulty Breakdown"));
        assert!(md.contains("easy"));
    }

    #[test]
    fn test_difficulty_breakdown_in_report() {
        let mut runner = BenchmarkRunner::new();
        let suite = BenchmarkSuite::SWEBenchLite;
        runner.load_suite(&suite, sample_tasks());
        let id = runner.create_run(suite, sample_config());
        runner.start_run(&id).unwrap();
        runner
            .record_result(
                &id,
                TaskResult {
                    task_id: "task-1".to_string(),
                    passed: true,
                    patch_generated: "p".to_string(),
                    error: None,
                    duration_ms: 100,
                    tokens_used: 50,
                },
            )
            .unwrap();
        runner
            .record_result(
                &id,
                TaskResult {
                    task_id: "task-3".to_string(),
                    passed: false,
                    patch_generated: String::new(),
                    error: None,
                    duration_ms: 200,
                    tokens_used: 100,
                },
            )
            .unwrap();
        let report = runner.complete_run(&id).unwrap();
        assert_eq!(report.difficulty_breakdown.get("easy"), Some(&(1, 1)));
        assert_eq!(report.difficulty_breakdown.get("hard"), Some(&(0, 1)));
    }

    #[test]
    fn test_suite_name() {
        assert_eq!(BenchmarkSuite::SWEBenchVerified.name(), "SWE-bench Verified");
        assert_eq!(BenchmarkSuite::SWEBenchPro.name(), "SWE-bench Pro");
        assert_eq!(BenchmarkSuite::SWEBenchLite.name(), "SWE-bench Lite");
        assert_eq!(
            BenchmarkSuite::Custom("MyBench".to_string()).name(),
            "MyBench"
        );
    }

    #[test]
    fn test_difficulty_labels() {
        assert_eq!(Difficulty::Easy.label(), "easy");
        assert_eq!(Difficulty::Medium.label(), "medium");
        assert_eq!(Difficulty::Hard.label(), "hard");
    }

    #[test]
    fn test_cost_estimate() {
        let mut runner = BenchmarkRunner::new();
        let id = runner.create_run(BenchmarkSuite::SWEBenchLite, sample_config());
        runner.start_run(&id).unwrap();
        runner
            .record_result(
                &id,
                TaskResult {
                    task_id: "t".to_string(),
                    passed: true,
                    patch_generated: String::new(),
                    error: None,
                    duration_ms: 100,
                    tokens_used: 10000,
                },
            )
            .unwrap();
        let report = runner.complete_run(&id).unwrap();
        // 10000 tokens / 1000 * 0.003 = 0.03
        assert!((report.cost_estimate_usd - 0.03).abs() < 0.001);
    }

    #[test]
    fn test_avg_duration() {
        let mut runner = BenchmarkRunner::new();
        let id = runner.create_run(BenchmarkSuite::SWEBenchLite, sample_config());
        runner.start_run(&id).unwrap();
        for dur in [100u64, 200, 300] {
            runner
                .record_result(
                    &id,
                    TaskResult {
                        task_id: format!("t-{}", dur),
                        passed: true,
                        patch_generated: String::new(),
                        error: None,
                        duration_ms: dur,
                        tokens_used: 10,
                    },
                )
                .unwrap();
        }
        let report = runner.complete_run(&id).unwrap();
        assert_eq!(report.avg_duration_ms, 200);
    }

    #[test]
    fn test_empty_run_report() {
        let mut runner = BenchmarkRunner::new();
        let id = runner.create_run(BenchmarkSuite::SWEBenchLite, sample_config());
        runner.start_run(&id).unwrap();
        let report = runner.complete_run(&id).unwrap();
        assert_eq!(report.total_tasks, 0);
        assert_eq!(report.pass_at_1, 0.0);
        assert_eq!(report.avg_duration_ms, 0);
    }

    #[test]
    fn test_multiple_results_same_run() {
        let mut runner = BenchmarkRunner::new();
        let id = runner.create_run(BenchmarkSuite::SWEBenchLite, sample_config());
        runner.start_run(&id).unwrap();
        for i in 0..5 {
            runner
                .record_result(
                    &id,
                    TaskResult {
                        task_id: format!("t-{}", i),
                        passed: i % 2 == 0,
                        patch_generated: String::new(),
                        error: None,
                        duration_ms: 100,
                        tokens_used: 10,
                    },
                )
                .unwrap();
        }
        let report = runner.complete_run(&id).unwrap();
        assert_eq!(report.passed_tasks, 3);
        assert_eq!(report.failed_tasks, 2);
    }

    #[test]
    fn test_run_model_and_provider_stored() {
        let mut runner = BenchmarkRunner::new();
        let config = BenchmarkConfig {
            max_turns: 5,
            timeout_per_task_secs: 60,
            parallel_tasks: 1,
            model: "gpt-4o".to_string(),
            provider: "openai".to_string(),
        };
        let id = runner.create_run(BenchmarkSuite::SWEBenchPro, config);
        let run = runner.runs.iter().find(|r| r.id == id).unwrap();
        assert_eq!(run.model, "gpt-4o");
        assert_eq!(run.provider, "openai");
    }
}
