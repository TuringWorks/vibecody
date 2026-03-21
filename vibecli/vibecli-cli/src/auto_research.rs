//! AutoResearch — Autonomous iterative research agent
//!
//! Inspired by TuringWorks/autoresearch but significantly more capable:
//! - Multi-file editing (not limited to a single file)
//! - Multi-metric evaluation with weighted scoring
//! - Parallel experiments via git worktrees
//! - Search strategies: greedy, beam search, genetic, combinatorial
//! - Cross-run learning with persistent hypothesis memory
//! - Domain-agnostic (ML training, API optimization, compiler flags, etc.)
//! - Safety rails: resource limits, NaN detection, timeout enforcement
//! - Structured hypothesis tracking with research methodology

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

// ── Enums ──────────────────────────────────────────────────────────────────────

/// Research domain determines default metrics, editable files, and evaluation strategy.
#[derive(Debug, Clone, PartialEq)]
pub enum ResearchDomain {
    /// ML model training (val_bpb, loss, accuracy, perplexity)
    MlTraining,
    /// API/server performance (latency_p99, throughput, error_rate)
    ApiPerformance,
    /// Compiler/build optimization (build_time, binary_size, test_pass_rate)
    BuildOptimization,
    /// Algorithm benchmarking (execution_time, memory_peak, correctness)
    AlgorithmBench,
    /// Database query tuning (query_time, rows_scanned, index_usage)
    DatabaseTuning,
    /// Frontend performance (bundle_size, FCP, LCP, CLS)
    FrontendPerf,
    /// Custom domain with user-defined metrics
    Custom(String),
}

impl ResearchDomain {
    pub fn default_metrics(&self) -> Vec<MetricDef> {
        match self {
            Self::MlTraining => vec![
                MetricDef::new("val_bpb", "Validation bits-per-byte", MetricDirection::Lower, 1.0),
                MetricDef::new("train_loss", "Training loss", MetricDirection::Lower, 0.3),
                MetricDef::new("gpu_util", "GPU utilization %", MetricDirection::Higher, 0.2),
                MetricDef::new("throughput", "Tokens/second", MetricDirection::Higher, 0.2),
            ],
            Self::ApiPerformance => vec![
                MetricDef::new("p99_ms", "P99 latency (ms)", MetricDirection::Lower, 1.0),
                MetricDef::new("throughput_rps", "Requests/second", MetricDirection::Higher, 0.8),
                MetricDef::new("error_rate", "Error rate %", MetricDirection::Lower, 0.5),
                MetricDef::new("memory_mb", "Memory usage (MB)", MetricDirection::Lower, 0.3),
            ],
            Self::BuildOptimization => vec![
                MetricDef::new("build_time_s", "Build time (seconds)", MetricDirection::Lower, 1.0),
                MetricDef::new("binary_size_kb", "Binary size (KB)", MetricDirection::Lower, 0.5),
                MetricDef::new("test_pass_rate", "Test pass rate %", MetricDirection::Higher, 0.8),
            ],
            Self::AlgorithmBench => vec![
                MetricDef::new("exec_time_ms", "Execution time (ms)", MetricDirection::Lower, 1.0),
                MetricDef::new("memory_peak_kb", "Peak memory (KB)", MetricDirection::Lower, 0.5),
                MetricDef::new("correctness", "Correctness score", MetricDirection::Higher, 0.9),
            ],
            Self::DatabaseTuning => vec![
                MetricDef::new("query_time_ms", "Query time (ms)", MetricDirection::Lower, 1.0),
                MetricDef::new("rows_scanned", "Rows scanned", MetricDirection::Lower, 0.6),
                MetricDef::new("index_usage", "Index usage %", MetricDirection::Higher, 0.4),
            ],
            Self::FrontendPerf => vec![
                MetricDef::new("bundle_size_kb", "Bundle size (KB)", MetricDirection::Lower, 0.8),
                MetricDef::new("fcp_ms", "First Contentful Paint (ms)", MetricDirection::Lower, 1.0),
                MetricDef::new("lcp_ms", "Largest Contentful Paint (ms)", MetricDirection::Lower, 0.9),
                MetricDef::new("cls", "Cumulative Layout Shift", MetricDirection::Lower, 0.5),
            ],
            Self::Custom(_) => vec![
                MetricDef::new("score", "Primary score", MetricDirection::Higher, 1.0),
            ],
        }
    }
}

/// Which direction is "better" for a metric.
#[derive(Debug, Clone, PartialEq)]
pub enum MetricDirection {
    Higher,
    Lower,
}

/// Search strategy for exploring the experiment space.
#[derive(Debug, Clone, PartialEq)]
pub enum SearchStrategy {
    /// Keep/discard each experiment independently (like autoresearch)
    Greedy,
    /// Maintain top-K candidates and branch from the best ones
    BeamSearch { beam_width: usize },
    /// Evolutionary: mutate, crossover, select from population
    Genetic { population_size: usize, mutation_rate: f64 },
    /// Try combining pairs of individually-discarded changes
    Combinatorial { max_combinations: usize },
    /// Bayesian optimization with surrogate model
    Bayesian { exploration_weight: f64 },
}

/// Status of a single experiment run.
#[derive(Debug, Clone, PartialEq)]
pub enum ExperimentStatus {
    Pending,
    Running,
    Completed,
    Failed(String),
    Timeout,
    Crashed,
    Kept,
    Discarded,
}

/// Status of the overall research session.
#[derive(Debug, Clone, PartialEq)]
pub enum SessionStatus {
    Idle,
    Running,
    Paused,
    Completed,
    Aborted(String),
}

/// Hypothesis confidence level.
#[derive(Debug, Clone, PartialEq)]
pub enum Confidence {
    Speculative,
    Low,
    Medium,
    High,
    Validated,
}

/// Safety check result.
#[derive(Debug, Clone, PartialEq)]
pub enum SafetyViolation {
    Timeout { elapsed: Duration, limit: Duration },
    MemoryExceeded { used_mb: u64, limit_mb: u64 },
    NaNDetected { metric: String },
    ProcessCrash { exit_code: i32, stderr: String },
    DiskSpaceExceeded { used_mb: u64, limit_mb: u64 },
    ResourceLeak { description: String },
}

// ── Core Structures ────────────────────────────────────────────────────────────

/// Definition of a metric to track and optimize.
#[derive(Debug, Clone)]
pub struct MetricDef {
    pub name: String,
    pub description: String,
    pub direction: MetricDirection,
    /// Weight for composite scoring (higher = more important)
    pub weight: f64,
}

impl MetricDef {
    pub fn new(name: &str, description: &str, direction: MetricDirection, weight: f64) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            direction,
            weight,
        }
    }
}

/// Recorded metric value from an experiment run.
#[derive(Debug, Clone)]
pub struct MetricValue {
    pub name: String,
    pub value: f64,
    pub unit: Option<String>,
}

/// A structured hypothesis for what an experiment will test.
#[derive(Debug, Clone)]
pub struct Hypothesis {
    pub id: String,
    pub description: String,
    pub rationale: String,
    pub confidence: Confidence,
    pub predicted_impact: HashMap<String, f64>,
    pub tags: Vec<String>,
    pub parent_hypothesis: Option<String>,
    pub created_at: SystemTime,
}

impl Hypothesis {
    pub fn new(id: &str, description: &str, rationale: &str) -> Self {
        Self {
            id: id.to_string(),
            description: description.to_string(),
            rationale: rationale.to_string(),
            confidence: Confidence::Speculative,
            predicted_impact: HashMap::new(),
            tags: Vec::new(),
            parent_hypothesis: None,
            created_at: SystemTime::now(),
        }
    }
}

/// A single file modification made during an experiment.
#[derive(Debug, Clone)]
pub struct FileChange {
    pub path: PathBuf,
    pub diff: String,
    pub lines_added: usize,
    pub lines_removed: usize,
}

/// A single experiment run within a research session.
#[derive(Debug, Clone)]
pub struct Experiment {
    pub id: String,
    pub session_id: String,
    pub hypothesis: Hypothesis,
    pub git_commit: Option<String>,
    pub git_branch: Option<String>,
    pub file_changes: Vec<FileChange>,
    pub command: String,
    pub metrics: Vec<MetricValue>,
    pub composite_score: f64,
    pub baseline_score: f64,
    pub delta: f64,
    pub status: ExperimentStatus,
    pub duration: Duration,
    pub started_at: SystemTime,
    pub completed_at: Option<SystemTime>,
    pub log_output: String,
    pub safety_violations: Vec<SafetyViolation>,
    pub parent_experiment: Option<String>,
}

impl Experiment {
    pub fn new(id: &str, session_id: &str, hypothesis: Hypothesis, command: &str) -> Self {
        Self {
            id: id.to_string(),
            session_id: session_id.to_string(),
            hypothesis,
            git_commit: None,
            git_branch: None,
            file_changes: Vec::new(),
            command: command.to_string(),
            metrics: Vec::new(),
            composite_score: 0.0,
            baseline_score: 0.0,
            delta: 0.0,
            status: ExperimentStatus::Pending,
            duration: Duration::ZERO,
            started_at: SystemTime::now(),
            completed_at: None,
            log_output: String::new(),
            safety_violations: Vec::new(),
            parent_experiment: None,
        }
    }

    pub fn is_improvement(&self) -> bool {
        self.delta > 0.0 && self.safety_violations.is_empty()
    }
}

/// Resource limits for safety enforcement.
#[derive(Debug, Clone)]
pub struct ResourceLimits {
    pub max_duration: Duration,
    pub max_memory_mb: u64,
    pub max_disk_mb: u64,
    pub max_cpu_percent: f64,
    pub kill_on_timeout: bool,
    pub max_file_changes: usize,
    pub forbidden_paths: Vec<PathBuf>,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_duration: Duration::from_secs(300), // 5 minutes
            max_memory_mb: 16384,                   // 16 GB
            max_disk_mb: 10240,                     // 10 GB
            max_cpu_percent: 100.0,
            kill_on_timeout: true,
            max_file_changes: 20,
            forbidden_paths: vec![
                PathBuf::from("/etc"),
                PathBuf::from("/usr"),
                PathBuf::from("/var"),
            ],
        }
    }
}

/// Configuration for a research session.
#[derive(Debug, Clone)]
pub struct ResearchConfig {
    pub domain: ResearchDomain,
    pub strategy: SearchStrategy,
    pub metrics: Vec<MetricDef>,
    pub editable_files: Vec<PathBuf>,
    pub read_only_files: Vec<PathBuf>,
    pub run_command: String,
    pub eval_command: Option<String>,
    pub metric_extract_pattern: Option<String>,
    pub resource_limits: ResourceLimits,
    pub max_experiments: usize,
    pub auto_revert_on_failure: bool,
    pub parallel_workers: usize,
    pub git_branch_prefix: String,
    pub workspace_dir: PathBuf,
    pub results_file: PathBuf,
    pub checkpoint_interval: usize,
}

impl Default for ResearchConfig {
    fn default() -> Self {
        Self {
            domain: ResearchDomain::Custom("default".into()),
            strategy: SearchStrategy::Greedy,
            metrics: vec![MetricDef::new("score", "Primary score", MetricDirection::Higher, 1.0)],
            editable_files: Vec::new(),
            read_only_files: Vec::new(),
            run_command: String::new(),
            eval_command: None,
            metric_extract_pattern: None,
            resource_limits: ResourceLimits::default(),
            max_experiments: 100,
            auto_revert_on_failure: true,
            parallel_workers: 1,
            git_branch_prefix: "autoresearch".to_string(),
            workspace_dir: PathBuf::from("."),
            results_file: PathBuf::from("results.tsv"),
            checkpoint_interval: 5,
        }
    }
}

/// A persistent lesson learned from past experiments.
#[derive(Debug, Clone)]
pub struct ResearchLesson {
    pub id: String,
    pub description: String,
    pub evidence: Vec<String>,
    pub confidence: Confidence,
    pub domain: String,
    pub tags: Vec<String>,
    pub created_at: SystemTime,
}

/// Cross-run memory for learning from past research.
#[derive(Debug, Clone)]
pub struct ResearchMemory {
    pub lessons: Vec<ResearchLesson>,
    pub successful_patterns: Vec<String>,
    pub failed_patterns: Vec<String>,
    pub metric_baselines: HashMap<String, f64>,
    pub total_experiments: usize,
    pub total_improvements: usize,
    pub total_sessions: usize,
}

impl ResearchMemory {
    pub fn new() -> Self {
        Self {
            lessons: Vec::new(),
            successful_patterns: Vec::new(),
            failed_patterns: Vec::new(),
            metric_baselines: HashMap::new(),
            total_experiments: 0,
            total_improvements: 0,
            total_sessions: 0,
        }
    }

    pub fn acceptance_rate(&self) -> f64 {
        if self.total_experiments == 0 {
            0.0
        } else {
            self.total_improvements as f64 / self.total_experiments as f64
        }
    }

    pub fn add_lesson(&mut self, lesson: ResearchLesson) {
        self.lessons.push(lesson);
    }

    pub fn record_outcome(&mut self, kept: bool, pattern: &str) {
        self.total_experiments += 1;
        if kept {
            self.total_improvements += 1;
            if !self.successful_patterns.contains(&pattern.to_string()) {
                self.successful_patterns.push(pattern.to_string());
            }
        } else if !self.failed_patterns.contains(&pattern.to_string()) {
            self.failed_patterns.push(pattern.to_string());
        }
    }
}

/// Result analysis for a set of experiments.
#[derive(Debug, Clone)]
pub struct ResearchAnalysis {
    pub total_experiments: usize,
    pub kept_count: usize,
    pub discarded_count: usize,
    pub failed_count: usize,
    pub acceptance_rate: f64,
    pub best_score: f64,
    pub baseline_score: f64,
    pub improvement_pct: f64,
    pub best_experiment_id: Option<String>,
    pub top_changes: Vec<(String, f64)>,
    pub metric_trends: HashMap<String, Vec<f64>>,
    pub total_duration: Duration,
    pub avg_experiment_duration: Duration,
}

/// Beam candidate for beam search strategy.
#[derive(Debug, Clone)]
pub struct BeamCandidate {
    pub experiment_id: String,
    pub score: f64,
    pub git_ref: String,
    pub depth: usize,
}

/// Population member for genetic search strategy.
#[derive(Debug, Clone)]
pub struct GeneticIndividual {
    pub id: String,
    pub genome: Vec<String>,
    pub fitness: f64,
    pub generation: usize,
    pub parents: Vec<String>,
}

/// An ongoing research session.
#[derive(Debug, Clone)]
pub struct ResearchSession {
    pub id: String,
    pub name: String,
    pub config: ResearchConfig,
    pub status: SessionStatus,
    pub experiments: Vec<Experiment>,
    pub current_best_score: f64,
    pub baseline_score: f64,
    pub beam_candidates: Vec<BeamCandidate>,
    pub population: Vec<GeneticIndividual>,
    pub started_at: SystemTime,
    pub updated_at: SystemTime,
    pub memory: ResearchMemory,
}

impl ResearchSession {
    pub fn new(id: &str, name: &str, config: ResearchConfig) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            config,
            status: SessionStatus::Idle,
            experiments: Vec::new(),
            current_best_score: 0.0,
            baseline_score: 0.0,
            beam_candidates: Vec::new(),
            population: Vec::new(),
            started_at: SystemTime::now(),
            updated_at: SystemTime::now(),
            memory: ResearchMemory::new(),
        }
    }

    pub fn kept_experiments(&self) -> Vec<&Experiment> {
        self.experiments.iter().filter(|e| e.status == ExperimentStatus::Kept).collect()
    }

    pub fn discarded_experiments(&self) -> Vec<&Experiment> {
        self.experiments.iter().filter(|e| e.status == ExperimentStatus::Discarded).collect()
    }

    pub fn failed_experiments(&self) -> Vec<&Experiment> {
        self.experiments.iter().filter(|e| matches!(e.status, ExperimentStatus::Failed(_) | ExperimentStatus::Crashed | ExperimentStatus::Timeout)).collect()
    }

    pub fn acceptance_rate(&self) -> f64 {
        let completed = self.experiments.iter().filter(|e| matches!(e.status, ExperimentStatus::Kept | ExperimentStatus::Discarded)).count();
        if completed == 0 {
            return 0.0;
        }
        self.kept_experiments().len() as f64 / completed as f64
    }

    pub fn improvement_pct(&self) -> f64 {
        if self.baseline_score == 0.0 {
            return 0.0;
        }
        ((self.current_best_score - self.baseline_score) / self.baseline_score.abs()) * 100.0
    }

    pub fn total_duration(&self) -> Duration {
        self.experiments.iter().map(|e| e.duration).sum()
    }
}

// ── ResearchEngine ─────────────────────────────────────────────────────────────

/// Main engine that orchestrates the research loop.
pub struct ResearchEngine {
    pub sessions: Vec<ResearchSession>,
    pub global_memory: ResearchMemory,
    pub active_session: Option<String>,
}

impl ResearchEngine {
    pub fn new() -> Self {
        Self {
            sessions: Vec::new(),
            global_memory: ResearchMemory::new(),
            active_session: None,
        }
    }

    pub fn create_session(&mut self, name: &str, config: ResearchConfig) -> String {
        let id = format!("rs_{}", self.sessions.len() + 1);
        let session = ResearchSession::new(&id, name, config);
        self.sessions.push(session);
        self.active_session = Some(id.clone());
        id
    }

    pub fn get_session(&self, id: &str) -> Option<&ResearchSession> {
        self.sessions.iter().find(|s| s.id == id)
    }

    pub fn get_session_mut(&mut self, id: &str) -> Option<&mut ResearchSession> {
        self.sessions.iter_mut().find(|s| s.id == id)
    }

    pub fn active(&self) -> Option<&ResearchSession> {
        self.active_session.as_ref().and_then(|id| self.get_session(id))
    }

    pub fn active_mut(&mut self) -> Option<&mut ResearchSession> {
        let id = self.active_session.clone();
        id.and_then(move |id| self.get_session_mut(&id))
    }

    /// Compute composite score from multiple metrics using weighted formula.
    pub fn compute_composite_score(metrics: &[MetricValue], defs: &[MetricDef]) -> f64 {
        let mut score = 0.0;
        let mut total_weight = 0.0;
        for def in defs {
            if let Some(mv) = metrics.iter().find(|m| m.name == def.name) {
                let normalized = match def.direction {
                    MetricDirection::Higher => mv.value,
                    MetricDirection::Lower => {
                        if mv.value == 0.0 { f64::MAX } else { 1.0 / mv.value }
                    }
                };
                score += normalized * def.weight;
                total_weight += def.weight;
            }
        }
        if total_weight > 0.0 { score / total_weight } else { 0.0 }
    }

    /// Check if an experiment's metrics contain NaN values.
    pub fn check_nan(metrics: &[MetricValue]) -> Vec<SafetyViolation> {
        metrics
            .iter()
            .filter(|m| m.value.is_nan() || m.value.is_infinite())
            .map(|m| SafetyViolation::NaNDetected { metric: m.name.clone() })
            .collect()
    }

    /// Validate that file changes respect the config constraints.
    pub fn validate_file_changes(changes: &[FileChange], config: &ResearchConfig) -> Vec<String> {
        let mut violations = Vec::new();
        if changes.len() > config.resource_limits.max_file_changes {
            violations.push(format!(
                "Too many file changes: {} (limit {})",
                changes.len(),
                config.resource_limits.max_file_changes
            ));
        }
        for change in changes {
            for forbidden in &config.resource_limits.forbidden_paths {
                if change.path.starts_with(forbidden) {
                    violations.push(format!(
                        "File change in forbidden path: {}",
                        change.path.display()
                    ));
                }
            }
            if !config.editable_files.is_empty()
                && !config.editable_files.iter().any(|e| change.path.ends_with(e) || change.path == *e)
            {
                violations.push(format!(
                    "File not in editable list: {}",
                    change.path.display()
                ));
            }
        }
        violations
    }

    /// Record an experiment result and update session state.
    pub fn record_experiment(&mut self, session_id: &str, mut experiment: Experiment) -> Result<(), String> {
        let session = self.get_session_mut(session_id)
            .ok_or_else(|| format!("Session not found: {}", session_id))?;

        // Compute composite score
        experiment.composite_score = Self::compute_composite_score(&experiment.metrics, &session.config.metrics);
        experiment.baseline_score = session.current_best_score;
        experiment.delta = experiment.composite_score - session.current_best_score;

        // Check NaN
        let nan_violations = Self::check_nan(&experiment.metrics);
        experiment.safety_violations.extend(nan_violations);

        // Determine keep/discard
        if experiment.is_improvement() {
            experiment.status = ExperimentStatus::Kept;
            session.current_best_score = experiment.composite_score;
            session.memory.record_outcome(true, &experiment.hypothesis.description);
        } else if experiment.safety_violations.is_empty() {
            experiment.status = ExperimentStatus::Discarded;
            session.memory.record_outcome(false, &experiment.hypothesis.description);
        }

        experiment.completed_at = Some(SystemTime::now());
        session.experiments.push(experiment);
        session.updated_at = SystemTime::now();

        Ok(())
    }

    /// Generate analysis of a research session.
    pub fn analyze_session(&self, session_id: &str) -> Result<ResearchAnalysis, String> {
        let session = self.get_session(session_id)
            .ok_or_else(|| format!("Session not found: {}", session_id))?;

        let kept = session.kept_experiments();
        let discarded = session.discarded_experiments();
        let failed = session.failed_experiments();
        let total = kept.len() + discarded.len() + failed.len();

        let mut metric_trends: HashMap<String, Vec<f64>> = HashMap::new();
        for exp in &session.experiments {
            for mv in &exp.metrics {
                metric_trends.entry(mv.name.clone()).or_default().push(mv.value);
            }
        }

        let mut top_changes: Vec<(String, f64)> = session.experiments
            .iter()
            .filter(|e| e.status == ExperimentStatus::Kept)
            .map(|e| (e.hypothesis.description.clone(), e.delta))
            .collect();
        top_changes.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let total_duration = session.total_duration();
        let avg_duration = if total > 0 {
            Duration::from_secs(total_duration.as_secs() / total as u64)
        } else {
            Duration::ZERO
        };

        Ok(ResearchAnalysis {
            total_experiments: total,
            kept_count: kept.len(),
            discarded_count: discarded.len(),
            failed_count: failed.len(),
            acceptance_rate: session.acceptance_rate(),
            best_score: session.current_best_score,
            baseline_score: session.baseline_score,
            improvement_pct: session.improvement_pct(),
            best_experiment_id: kept.last().map(|e| e.id.clone()),
            top_changes,
            metric_trends,
            total_duration,
            avg_experiment_duration: avg_duration,
        })
    }

    /// Select next experiments based on search strategy.
    pub fn suggest_next(&self, session_id: &str) -> Result<Vec<String>, String> {
        let session = self.get_session(session_id)
            .ok_or_else(|| format!("Session not found: {}", session_id))?;

        let mut suggestions = Vec::new();

        match &session.config.strategy {
            SearchStrategy::Greedy => {
                suggestions.push("Try a single focused change to the highest-impact parameter".to_string());
                if !session.memory.successful_patterns.is_empty() {
                    suggestions.push(format!(
                        "Patterns that worked: {}",
                        session.memory.successful_patterns.join(", ")
                    ));
                }
                if !session.memory.failed_patterns.is_empty() {
                    suggestions.push(format!(
                        "Avoid patterns: {}",
                        session.memory.failed_patterns.iter().take(5).cloned().collect::<Vec<_>>().join(", ")
                    ));
                }
            }
            SearchStrategy::BeamSearch { beam_width } => {
                suggestions.push(format!("Beam width: {} — branch from top candidates", beam_width));
                for candidate in session.beam_candidates.iter().take(*beam_width) {
                    suggestions.push(format!(
                        "Branch from {} (score: {:.4}, depth: {})",
                        candidate.experiment_id, candidate.score, candidate.depth
                    ));
                }
            }
            SearchStrategy::Genetic { population_size, mutation_rate } => {
                suggestions.push(format!(
                    "Population: {}, mutation rate: {:.2}",
                    population_size, mutation_rate
                ));
                let top: Vec<_> = session.population.iter()
                    .take(3)
                    .map(|g| format!("{} (fitness: {:.4})", g.id, g.fitness))
                    .collect();
                if !top.is_empty() {
                    suggestions.push(format!("Top individuals: {}", top.join(", ")));
                }
            }
            SearchStrategy::Combinatorial { max_combinations } => {
                let discarded = session.discarded_experiments();
                let n = discarded.len().min(*max_combinations);
                suggestions.push(format!("Try combining {} discarded changes", n));
                for pair in discarded.windows(2).take(n) {
                    suggestions.push(format!(
                        "Combine: {} + {}",
                        pair[0].hypothesis.description,
                        pair[1].hypothesis.description,
                    ));
                }
            }
            SearchStrategy::Bayesian { exploration_weight } => {
                suggestions.push(format!(
                    "Bayesian optimization (exploration weight: {:.2})",
                    exploration_weight
                ));
                suggestions.push("Explore undersampled regions of the parameter space".to_string());
            }
        }

        Ok(suggestions)
    }

    /// Generate a results TSV string for a session (compatible with autoresearch format).
    pub fn export_results_tsv(&self, session_id: &str) -> Result<String, String> {
        let session = self.get_session(session_id)
            .ok_or_else(|| format!("Session not found: {}", session_id))?;

        let mut lines = vec!["experiment_id\tcommit\tstatus\tcomposite_score\tdelta\tduration_s\tdescription".to_string()];

        for exp in &session.experiments {
            let status_str = match &exp.status {
                ExperimentStatus::Kept => "KEEP",
                ExperimentStatus::Discarded => "DISCARD",
                ExperimentStatus::Failed(msg) => msg.as_str(),
                ExperimentStatus::Timeout => "TIMEOUT",
                ExperimentStatus::Crashed => "CRASH",
                _ => "PENDING",
            };
            lines.push(format!(
                "{}\t{}\t{}\t{:.6}\t{:.6}\t{}\t{}",
                exp.id,
                exp.git_commit.as_deref().unwrap_or("-"),
                status_str,
                exp.composite_score,
                exp.delta,
                exp.duration.as_secs(),
                exp.hypothesis.description,
            ));
        }

        Ok(lines.join("\n"))
    }

    /// List all sessions.
    pub fn list_sessions(&self) -> Vec<(&str, &str, &SessionStatus, usize, f64)> {
        self.sessions
            .iter()
            .map(|s| (s.id.as_str(), s.name.as_str(), &s.status, s.experiments.len(), s.acceptance_rate()))
            .collect()
    }
}

// ── MetricExtractor ────────────────────────────────────────────────────────────

/// Extracts metric values from command output using configurable strategies.
#[derive(Debug, Clone)]
pub enum ExtractionStrategy {
    /// Extract using a regex with named capture groups: `(?P<name>...)`.
    Regex(String),
    /// Extract from JSON output using dot-notation path (e.g. "metrics.val_bpb").
    JsonPath(String),
    /// Extract key=value pairs from log lines (e.g. "val_bpb: 1.08").
    KeyValue { separator: String },
    /// Extract the last numeric value on a line matching a prefix.
    LastLine { prefix: String },
}

/// Parses command output and extracts metric values.
pub struct MetricExtractor;

impl MetricExtractor {
    /// Extract metrics from raw output text using a given strategy.
    pub fn extract(output: &str, strategy: &ExtractionStrategy) -> Vec<MetricValue> {
        match strategy {
            ExtractionStrategy::Regex(pattern) => Self::extract_regex(output, pattern),
            ExtractionStrategy::JsonPath(path) => Self::extract_json(output, path),
            ExtractionStrategy::KeyValue { separator } => Self::extract_key_value(output, separator),
            ExtractionStrategy::LastLine { prefix } => Self::extract_last_line(output, prefix),
        }
    }

    fn extract_regex(output: &str, pattern: &str) -> Vec<MetricValue> {
        let mut results = Vec::new();
        if let Ok(re) = regex::Regex::new(pattern) {
            for caps in re.captures_iter(output) {
                // Try named groups first
                for name in re.capture_names().flatten() {
                    if let Some(m) = caps.name(name) {
                        if let Ok(val) = m.as_str().parse::<f64>() {
                            results.push(MetricValue { name: name.to_string(), value: val, unit: None });
                        }
                    }
                }
                // Fallback: if no named groups, use group 1 as "metric" and group 2 as value
                if results.is_empty() && caps.len() >= 3 {
                    if let (Some(n), Some(v)) = (caps.get(1), caps.get(2)) {
                        if let Ok(val) = v.as_str().parse::<f64>() {
                            results.push(MetricValue { name: n.as_str().to_string(), value: val, unit: None });
                        }
                    }
                }
            }
        }
        results
    }

    fn extract_json(output: &str, path: &str) -> Vec<MetricValue> {
        let mut results = Vec::new();
        // Find JSON object in output (may be surrounded by other text)
        let json_start = output.find('{');
        let json_end = output.rfind('}');
        if let (Some(start), Some(end)) = (json_start, json_end) {
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&output[start..=end]) {
                let parts: Vec<&str> = path.split('.').collect();
                let mut current = &parsed;
                for part in &parts {
                    if let Some(next) = current.get(part) {
                        current = next;
                    } else {
                        return results;
                    }
                }
                if let Some(val) = current.as_f64() {
                    let name = parts.last().copied().unwrap_or("value");
                    results.push(MetricValue { name: name.to_string(), value: val, unit: None });
                } else if let Some(obj) = current.as_object() {
                    for (k, v) in obj {
                        if let Some(val) = v.as_f64() {
                            results.push(MetricValue { name: k.clone(), value: val, unit: None });
                        }
                    }
                }
            }
        }
        results
    }

    fn extract_key_value(output: &str, separator: &str) -> Vec<MetricValue> {
        let mut results = Vec::new();
        for line in output.lines() {
            let line = line.trim();
            if let Some(idx) = line.find(separator) {
                let key = line[..idx].trim().to_string();
                let val_str = line[idx + separator.len()..].trim();
                // Strip any trailing non-numeric characters (units, etc.)
                let numeric: String = val_str.chars().take_while(|c| c.is_ascii_digit() || *c == '.' || *c == '-' || *c == 'e' || *c == 'E' || *c == '+').collect();
                if let Ok(val) = numeric.parse::<f64>() {
                    results.push(MetricValue { name: key, value: val, unit: None });
                }
            }
        }
        results
    }

    fn extract_last_line(output: &str, prefix: &str) -> Vec<MetricValue> {
        let mut results = Vec::new();
        for line in output.lines().rev() {
            let line = line.trim();
            if line.starts_with(prefix) {
                let rest = line[prefix.len()..].trim();
                let numeric: String = rest.chars().take_while(|c| c.is_ascii_digit() || *c == '.' || *c == '-' || *c == 'e' || *c == 'E' || *c == '+').collect();
                if let Ok(val) = numeric.parse::<f64>() {
                    results.push(MetricValue { name: prefix.trim_end_matches(&[':', '=', ' '][..]).to_string(), value: val, unit: None });
                    break;
                }
            }
        }
        results
    }
}

// ── StatisticalValidator ───────────────────────────────────────────────────────

/// Statistical significance testing for experiment results.
/// Prevents keeping noise as "improvements".
pub struct StatisticalValidator;

impl StatisticalValidator {
    /// Run Welch's t-test to determine if the difference between two sets of
    /// measurements is statistically significant.
    /// Returns (t_statistic, p_value_approximate, is_significant).
    pub fn welch_t_test(sample_a: &[f64], sample_b: &[f64], alpha: f64) -> (f64, f64, bool) {
        if sample_a.len() < 2 || sample_b.len() < 2 {
            return (0.0, 1.0, false);
        }
        let n_a = sample_a.len() as f64;
        let n_b = sample_b.len() as f64;
        let mean_a = sample_a.iter().sum::<f64>() / n_a;
        let mean_b = sample_b.iter().sum::<f64>() / n_b;
        let var_a = sample_a.iter().map(|x| (x - mean_a).powi(2)).sum::<f64>() / (n_a - 1.0);
        let var_b = sample_b.iter().map(|x| (x - mean_b).powi(2)).sum::<f64>() / (n_b - 1.0);

        let se = (var_a / n_a + var_b / n_b).sqrt();
        if se == 0.0 {
            return (0.0, 1.0, false);
        }
        let t = (mean_a - mean_b) / se;

        // Welch-Satterthwaite degrees of freedom
        let num = (var_a / n_a + var_b / n_b).powi(2);
        let denom = (var_a / n_a).powi(2) / (n_a - 1.0) + (var_b / n_b).powi(2) / (n_b - 1.0);
        let df = if denom > 0.0 { num / denom } else { 1.0 };

        // Approximate p-value using normal approximation for large df
        let p_approx = Self::approx_p_value(t.abs(), df);

        (t, p_approx, p_approx < alpha)
    }

    /// Approximate two-tailed p-value from t-statistic and degrees of freedom.
    /// Uses the approximation: p ≈ 2 * (1 - Φ(|t| * sqrt(df / (df - 2)))) for df > 2.
    fn approx_p_value(t_abs: f64, df: f64) -> f64 {
        if df <= 2.0 {
            return 1.0; // Not enough data
        }
        // Normal approximation
        let z = t_abs * (df / (df - 2.0)).sqrt().recip();
        // Approximation of 2 * (1 - Φ(z)) using logistic function
        let p = 2.0 * (1.0 / (1.0 + (1.7 * z).exp()));
        p.min(1.0).max(0.0)
    }

    /// Bootstrap confidence interval for the mean difference.
    /// Returns (lower_bound, upper_bound, mean_diff).
    pub fn bootstrap_ci(sample_a: &[f64], sample_b: &[f64], confidence: f64, n_bootstrap: usize) -> (f64, f64, f64) {
        if sample_a.is_empty() || sample_b.is_empty() {
            return (0.0, 0.0, 0.0);
        }
        let mean_a: f64 = sample_a.iter().sum::<f64>() / sample_a.len() as f64;
        let mean_b: f64 = sample_b.iter().sum::<f64>() / sample_b.len() as f64;
        let observed_diff = mean_a - mean_b;

        // Simple deterministic bootstrap approximation using jackknife
        let mut diffs = Vec::with_capacity(n_bootstrap.min(sample_a.len() + sample_b.len()));

        // Jackknife resampling on sample_a
        for i in 0..sample_a.len() {
            let jack_mean: f64 = sample_a.iter().enumerate()
                .filter(|(j, _)| *j != i)
                .map(|(_, v)| v)
                .sum::<f64>() / (sample_a.len() - 1).max(1) as f64;
            diffs.push(jack_mean - mean_b);
        }
        // Jackknife resampling on sample_b
        for i in 0..sample_b.len() {
            let jack_mean: f64 = sample_b.iter().enumerate()
                .filter(|(j, _)| *j != i)
                .map(|(_, v)| v)
                .sum::<f64>() / (sample_b.len() - 1).max(1) as f64;
            diffs.push(mean_a - jack_mean);
        }

        diffs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let alpha = (1.0 - confidence) / 2.0;
        let lower_idx = (alpha * diffs.len() as f64).floor() as usize;
        let upper_idx = ((1.0 - alpha) * diffs.len() as f64).ceil() as usize;

        let lower = diffs.get(lower_idx).copied().unwrap_or(observed_diff);
        let upper = diffs.get(upper_idx.min(diffs.len() - 1)).copied().unwrap_or(observed_diff);

        (lower, upper, observed_diff)
    }

    /// Check if an improvement is statistically significant given multiple runs.
    pub fn is_significant_improvement(
        baseline_runs: &[f64],
        experiment_runs: &[f64],
        direction: &MetricDirection,
        alpha: f64,
    ) -> bool {
        if baseline_runs.len() < 2 || experiment_runs.len() < 2 {
            // Not enough data for statistical test — fall back to mean comparison
            let mean_b = baseline_runs.iter().sum::<f64>() / baseline_runs.len().max(1) as f64;
            let mean_e = experiment_runs.iter().sum::<f64>() / experiment_runs.len().max(1) as f64;
            return match direction {
                MetricDirection::Higher => mean_e > mean_b,
                MetricDirection::Lower => mean_e < mean_b,
            };
        }

        let (ordered_a, ordered_b) = match direction {
            MetricDirection::Higher => (experiment_runs, baseline_runs),
            MetricDirection::Lower => (baseline_runs, experiment_runs),
        };

        let (_, _, significant) = Self::welch_t_test(ordered_a, ordered_b, alpha);
        significant
    }

    /// Compute effect size (Cohen's d).
    pub fn cohens_d(sample_a: &[f64], sample_b: &[f64]) -> f64 {
        if sample_a.len() < 2 || sample_b.len() < 2 {
            return 0.0;
        }
        let n_a = sample_a.len() as f64;
        let n_b = sample_b.len() as f64;
        let mean_a = sample_a.iter().sum::<f64>() / n_a;
        let mean_b = sample_b.iter().sum::<f64>() / n_b;
        let var_a = sample_a.iter().map(|x| (x - mean_a).powi(2)).sum::<f64>() / (n_a - 1.0);
        let var_b = sample_b.iter().map(|x| (x - mean_b).powi(2)).sum::<f64>() / (n_b - 1.0);
        let pooled_std = (((n_a - 1.0) * var_a + (n_b - 1.0) * var_b) / (n_a + n_b - 2.0)).sqrt();
        if pooled_std == 0.0 { 0.0 } else { (mean_a - mean_b) / pooled_std }
    }
}

// ── ExperimentGraph ────────────────────────────────────────────────────────────

/// Tracks parent-child relationships between experiments, enabling
/// rollback to any branch point (not just the latest).
#[derive(Debug, Clone)]
pub struct ExperimentGraph {
    /// experiment_id -> parent_experiment_id
    edges: HashMap<String, String>,
    /// experiment_id -> list of children
    children: HashMap<String, Vec<String>>,
    /// Root experiments (no parent)
    roots: Vec<String>,
}

impl ExperimentGraph {
    pub fn new() -> Self {
        Self { edges: HashMap::new(), children: HashMap::new(), roots: Vec::new() }
    }

    pub fn add_experiment(&mut self, id: &str, parent: Option<&str>) {
        if let Some(p) = parent {
            self.edges.insert(id.to_string(), p.to_string());
            self.children.entry(p.to_string()).or_default().push(id.to_string());
        } else {
            self.roots.push(id.to_string());
        }
    }

    /// Get the ancestry chain from an experiment back to the root.
    pub fn ancestry(&self, id: &str) -> Vec<String> {
        let mut chain = vec![id.to_string()];
        let mut current = id;
        while let Some(parent) = self.edges.get(current) {
            chain.push(parent.clone());
            current = parent;
        }
        chain.reverse();
        chain
    }

    /// Get all children of an experiment.
    pub fn children_of(&self, id: &str) -> Vec<&str> {
        self.children.get(id).map(|c| c.iter().map(|s| s.as_str()).collect()).unwrap_or_default()
    }

    /// Get depth of an experiment in the tree.
    pub fn depth(&self, id: &str) -> usize {
        self.ancestry(id).len() - 1
    }

    /// Find all leaf experiments (no children).
    pub fn leaves(&self) -> Vec<&str> {
        let mut all_ids: Vec<&str> = self.edges.keys().map(|s| s.as_str()).collect();
        all_ids.extend(self.roots.iter().map(|s| s.as_str()));
        all_ids.into_iter().filter(|id| !self.children.contains_key(*id)).collect()
    }

    /// Find best branch point: the experiment with the most kept descendants.
    pub fn best_branch_point(&self, kept_ids: &[&str]) -> Option<String> {
        let mut scores: HashMap<&str, usize> = HashMap::new();
        for kid in kept_ids {
            for ancestor in self.ancestry(kid) {
                *scores.entry(Box::leak(ancestor.into_boxed_str())).or_insert(0) += 1;
            }
        }
        scores.into_iter().max_by_key(|(_, v)| *v).map(|(k, _)| k.to_string())
    }
}

// ── HypothesisGenerator ────────────────────────────────────────────────────────

/// Generates hypotheses based on past experiment results and domain knowledge.
pub struct HypothesisGenerator;

impl HypothesisGenerator {
    /// Analyze past results and suggest hypotheses for the next experiment.
    pub fn generate(session: &ResearchSession) -> Vec<Hypothesis> {
        let mut hypotheses = Vec::new();
        let exp_count = session.experiments.len();

        // Strategy 1: Exploit — variations of what worked
        for exp in session.kept_experiments().iter().rev().take(3) {
            let h = Hypothesis::new(
                &format!("h_var_{}", exp_count + hypotheses.len()),
                &format!("Variation of: {}", exp.hypothesis.description),
                &format!("Based on kept experiment {} (delta: {:.4})", exp.id, exp.delta),
            );
            hypotheses.push(h);
        }

        // Strategy 2: Explore — opposite of what failed
        for exp in session.discarded_experiments().iter().rev().take(2) {
            let mut h = Hypothesis::new(
                &format!("h_opp_{}", exp_count + hypotheses.len()),
                &format!("Opposite of: {}", exp.hypothesis.description),
                &format!("Discarded experiment {} suggests trying the inverse approach", exp.id),
            );
            h.confidence = Confidence::Low;
            hypotheses.push(h);
        }

        // Strategy 3: Combine — merge two successful changes
        let kept: Vec<_> = session.kept_experiments();
        if kept.len() >= 2 {
            let a = &kept[kept.len() - 1];
            let b = &kept[kept.len() - 2];
            let mut h = Hypothesis::new(
                &format!("h_combo_{}", exp_count + hypotheses.len()),
                &format!("Combine: {} + {}", a.hypothesis.description, b.hypothesis.description),
                "Two individually-beneficial changes may compound",
            );
            h.confidence = Confidence::Medium;
            h.tags = vec!["combinatorial".to_string()];
            hypotheses.push(h);
        }

        // Strategy 4: Metric-driven — focus on the weakest metric
        if let Some(last) = session.experiments.last() {
            if let Some(weakest) = Self::find_weakest_metric(&last.metrics, &session.config.metrics) {
                let mut h = Hypothesis::new(
                    &format!("h_weak_{}", exp_count + hypotheses.len()),
                    &format!("Improve weakest metric: {}", weakest),
                    &format!("{} has the most room for improvement relative to its weight", weakest),
                );
                h.tags = vec!["metric-driven".to_string()];
                hypotheses.push(h);
            }
        }

        // Strategy 5: Novelty — try something not yet attempted
        if session.memory.failed_patterns.len() + session.memory.successful_patterns.len() > 5 {
            let mut h = Hypothesis::new(
                &format!("h_novel_{}", exp_count + hypotheses.len()),
                "Try a fundamentally different approach",
                "Many incremental changes have been tried; a larger structural change may unlock new gains",
            );
            h.confidence = Confidence::Speculative;
            h.tags = vec!["exploration".to_string()];
            hypotheses.push(h);
        }

        hypotheses
    }

    /// Find the metric with the worst relative performance.
    fn find_weakest_metric(metrics: &[MetricValue], defs: &[MetricDef]) -> Option<String> {
        let mut worst_name = None;
        let mut worst_score = f64::MAX;
        for def in defs {
            if let Some(mv) = metrics.iter().find(|m| m.name == def.name) {
                let normalized = match def.direction {
                    MetricDirection::Higher => mv.value,
                    MetricDirection::Lower => if mv.value == 0.0 { f64::MAX } else { 1.0 / mv.value },
                };
                let weighted = normalized * def.weight;
                if weighted < worst_score {
                    worst_score = weighted;
                    worst_name = Some(def.name.clone());
                }
            }
        }
        worst_name
    }
}

// ── WorktreeRunner ─────────────────────────────────────────────────────────────

/// Manages parallel experiment execution using git worktrees.
#[derive(Debug, Clone)]
pub struct WorktreeSlot {
    pub id: String,
    pub worktree_path: PathBuf,
    pub branch_name: String,
    pub experiment_id: Option<String>,
    pub status: WorktreeStatus,
}

#[derive(Debug, Clone, PartialEq)]
pub enum WorktreeStatus {
    Available,
    Running,
    Completed,
    Failed,
}

pub struct WorktreeRunner {
    pub slots: Vec<WorktreeSlot>,
    pub base_dir: PathBuf,
    pub max_parallel: usize,
}

impl WorktreeRunner {
    pub fn new(base_dir: PathBuf, max_parallel: usize) -> Self {
        let slots = (0..max_parallel).map(|i| WorktreeSlot {
            id: format!("wt_{}", i),
            worktree_path: base_dir.join(format!(".autoresearch-wt-{}", i)),
            branch_name: format!("autoresearch/worker-{}", i),
            experiment_id: None,
            status: WorktreeStatus::Available,
        }).collect();
        Self { slots, base_dir, max_parallel }
    }

    /// Find an available worktree slot.
    pub fn available_slot(&self) -> Option<&WorktreeSlot> {
        self.slots.iter().find(|s| s.status == WorktreeStatus::Available)
    }

    /// Count running experiments.
    pub fn running_count(&self) -> usize {
        self.slots.iter().filter(|s| s.status == WorktreeStatus::Running).count()
    }

    /// Get the git commands needed to create a worktree.
    pub fn create_worktree_commands(slot: &WorktreeSlot, base_ref: &str) -> Vec<String> {
        vec![
            format!("git worktree add -b {} {} {}", slot.branch_name, slot.worktree_path.display(), base_ref),
        ]
    }

    /// Get the git commands needed to clean up a worktree.
    pub fn cleanup_worktree_commands(slot: &WorktreeSlot) -> Vec<String> {
        vec![
            format!("git worktree remove --force {}", slot.worktree_path.display()),
            format!("git branch -D {}", slot.branch_name),
        ]
    }

    /// Mark a slot as running with an experiment.
    pub fn assign_experiment(&mut self, slot_id: &str, experiment_id: &str) -> bool {
        if let Some(slot) = self.slots.iter_mut().find(|s| s.id == slot_id && s.status == WorktreeStatus::Available) {
            slot.experiment_id = Some(experiment_id.to_string());
            slot.status = WorktreeStatus::Running;
            true
        } else {
            false
        }
    }

    /// Mark a slot as completed and available.
    pub fn release_slot(&mut self, slot_id: &str) {
        if let Some(slot) = self.slots.iter_mut().find(|s| s.id == slot_id) {
            slot.experiment_id = None;
            slot.status = WorktreeStatus::Available;
        }
    }
}

// ── WarmStarter ────────────────────────────────────────────────────────────────

/// Loads lessons and baselines from prior sessions to seed new research.
pub struct WarmStarter;

impl WarmStarter {
    /// Merge lessons from past sessions into a new session's memory.
    pub fn warm_start(session: &mut ResearchSession, past_sessions: &[ResearchSession]) {
        for past in past_sessions {
            // Only import lessons from the same domain
            if past.config.domain != session.config.domain {
                continue;
            }
            for lesson in &past.memory.lessons {
                if !session.memory.lessons.iter().any(|l| l.description == lesson.description) {
                    session.memory.lessons.push(lesson.clone());
                }
            }
            for pattern in &past.memory.successful_patterns {
                if !session.memory.successful_patterns.contains(pattern) {
                    session.memory.successful_patterns.push(pattern.clone());
                }
            }
            for pattern in &past.memory.failed_patterns {
                if !session.memory.failed_patterns.contains(pattern) {
                    session.memory.failed_patterns.push(pattern.clone());
                }
            }
            // Import baseline metrics
            for (name, val) in &past.memory.metric_baselines {
                session.memory.metric_baselines.entry(name.clone()).or_insert(*val);
            }
        }
        session.memory.total_sessions += past_sessions.len();
    }

    /// Estimate how many experiments a session needs based on past acceptance rates.
    pub fn estimate_experiments_needed(past_sessions: &[ResearchSession], target_improvement_pct: f64) -> usize {
        if past_sessions.is_empty() {
            return 50; // default
        }
        let avg_acceptance = past_sessions.iter()
            .map(|s| s.acceptance_rate())
            .sum::<f64>() / past_sessions.len() as f64;
        let avg_delta_per_kept = past_sessions.iter()
            .flat_map(|s| s.kept_experiments())
            .map(|e| e.delta)
            .sum::<f64>() / past_sessions.iter().map(|s| s.kept_experiments().len()).sum::<usize>().max(1) as f64;

        if avg_acceptance == 0.0 || avg_delta_per_kept <= 0.0 {
            return 100;
        }
        let needed_improvements = (target_improvement_pct / (avg_delta_per_kept * 100.0)).ceil() as usize;
        let needed_experiments = (needed_improvements as f64 / avg_acceptance).ceil() as usize;
        needed_experiments.max(5).min(1000)
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_research_domain_default_metrics() {
        let ml = ResearchDomain::MlTraining;
        let metrics = ml.default_metrics();
        assert_eq!(metrics.len(), 4);
        assert_eq!(metrics[0].name, "val_bpb");
        assert_eq!(metrics[0].direction, MetricDirection::Lower);

        let api = ResearchDomain::ApiPerformance;
        let metrics = api.default_metrics();
        assert_eq!(metrics.len(), 4);
        assert_eq!(metrics[0].name, "p99_ms");

        let build = ResearchDomain::BuildOptimization;
        assert_eq!(build.default_metrics().len(), 3);

        let algo = ResearchDomain::AlgorithmBench;
        assert_eq!(algo.default_metrics().len(), 3);

        let db = ResearchDomain::DatabaseTuning;
        assert_eq!(db.default_metrics().len(), 3);

        let fe = ResearchDomain::FrontendPerf;
        assert_eq!(fe.default_metrics().len(), 4);

        let custom = ResearchDomain::Custom("test".into());
        assert_eq!(custom.default_metrics().len(), 1);
    }

    #[test]
    fn test_metric_def_creation() {
        let m = MetricDef::new("loss", "Training loss", MetricDirection::Lower, 0.8);
        assert_eq!(m.name, "loss");
        assert_eq!(m.weight, 0.8);
        assert_eq!(m.direction, MetricDirection::Lower);
    }

    #[test]
    fn test_hypothesis_creation() {
        let h = Hypothesis::new("h1", "Increase LR", "Higher LR may converge faster");
        assert_eq!(h.id, "h1");
        assert_eq!(h.confidence, Confidence::Speculative);
        assert!(h.tags.is_empty());
    }

    #[test]
    fn test_experiment_creation() {
        let h = Hypothesis::new("h1", "Test", "Reason");
        let exp = Experiment::new("e1", "s1", h, "python train.py");
        assert_eq!(exp.id, "e1");
        assert_eq!(exp.status, ExperimentStatus::Pending);
        assert!(!exp.is_improvement());
    }

    #[test]
    fn test_experiment_is_improvement() {
        let h = Hypothesis::new("h1", "Test", "Reason");
        let mut exp = Experiment::new("e1", "s1", h, "cmd");
        exp.delta = 0.1;
        assert!(exp.is_improvement());

        exp.delta = -0.1;
        assert!(!exp.is_improvement());

        exp.delta = 0.1;
        exp.safety_violations.push(SafetyViolation::NaNDetected { metric: "loss".into() });
        assert!(!exp.is_improvement());
    }

    #[test]
    fn test_resource_limits_default() {
        let limits = ResourceLimits::default();
        assert_eq!(limits.max_duration, Duration::from_secs(300));
        assert_eq!(limits.max_memory_mb, 16384);
        assert!(limits.kill_on_timeout);
        assert_eq!(limits.max_file_changes, 20);
    }

    #[test]
    fn test_research_config_default() {
        let config = ResearchConfig::default();
        assert_eq!(config.strategy, SearchStrategy::Greedy);
        assert_eq!(config.max_experiments, 100);
        assert_eq!(config.parallel_workers, 1);
        assert!(config.auto_revert_on_failure);
    }

    #[test]
    fn test_research_memory() {
        let mut mem = ResearchMemory::new();
        assert_eq!(mem.acceptance_rate(), 0.0);

        mem.record_outcome(true, "increased lr");
        mem.record_outcome(false, "decreased batch size");
        mem.record_outcome(true, "added layer norm");

        assert_eq!(mem.total_experiments, 3);
        assert_eq!(mem.total_improvements, 2);
        assert!((mem.acceptance_rate() - 0.6667).abs() < 0.01);
        assert!(mem.successful_patterns.contains(&"increased lr".to_string()));
        assert!(mem.failed_patterns.contains(&"decreased batch size".to_string()));
    }

    #[test]
    fn test_research_memory_no_duplicates() {
        let mut mem = ResearchMemory::new();
        mem.record_outcome(true, "pattern A");
        mem.record_outcome(true, "pattern A");
        assert_eq!(mem.successful_patterns.len(), 1);
    }

    #[test]
    fn test_research_memory_add_lesson() {
        let mut mem = ResearchMemory::new();
        let lesson = ResearchLesson {
            id: "l1".into(),
            description: "RoPE helps".into(),
            evidence: vec!["exp_1".into()],
            confidence: Confidence::High,
            domain: "ml".into(),
            tags: vec!["attention".into()],
            created_at: SystemTime::now(),
        };
        mem.add_lesson(lesson);
        assert_eq!(mem.lessons.len(), 1);
    }

    #[test]
    fn test_research_engine_create_session() {
        let mut engine = ResearchEngine::new();
        let config = ResearchConfig::default();
        let id = engine.create_session("test run", config);
        assert_eq!(id, "rs_1");
        assert_eq!(engine.sessions.len(), 1);
        assert_eq!(engine.active_session, Some("rs_1".into()));
    }

    #[test]
    fn test_research_engine_get_session() {
        let mut engine = ResearchEngine::new();
        let config = ResearchConfig::default();
        let id = engine.create_session("test", config);
        assert!(engine.get_session(&id).is_some());
        assert!(engine.get_session("nonexistent").is_none());
    }

    #[test]
    fn test_research_engine_active() {
        let mut engine = ResearchEngine::new();
        assert!(engine.active().is_none());

        let config = ResearchConfig::default();
        engine.create_session("test", config);
        assert!(engine.active().is_some());
        assert_eq!(engine.active().unwrap().name, "test");
    }

    #[test]
    fn test_composite_score_higher_is_better() {
        let metrics = vec![
            MetricValue { name: "throughput".into(), value: 100.0, unit: None },
        ];
        let defs = vec![
            MetricDef::new("throughput", "T", MetricDirection::Higher, 1.0),
        ];
        let score = ResearchEngine::compute_composite_score(&metrics, &defs);
        assert_eq!(score, 100.0);
    }

    #[test]
    fn test_composite_score_lower_is_better() {
        let metrics = vec![
            MetricValue { name: "latency".into(), value: 10.0, unit: None },
        ];
        let defs = vec![
            MetricDef::new("latency", "L", MetricDirection::Lower, 1.0),
        ];
        let score = ResearchEngine::compute_composite_score(&metrics, &defs);
        assert_eq!(score, 0.1); // 1/10
    }

    #[test]
    fn test_composite_score_weighted() {
        let metrics = vec![
            MetricValue { name: "a".into(), value: 100.0, unit: None },
            MetricValue { name: "b".into(), value: 50.0, unit: None },
        ];
        let defs = vec![
            MetricDef::new("a", "A", MetricDirection::Higher, 2.0),
            MetricDef::new("b", "B", MetricDirection::Higher, 1.0),
        ];
        let score = ResearchEngine::compute_composite_score(&metrics, &defs);
        // (100*2 + 50*1) / (2+1) = 250/3 = 83.333...
        assert!((score - 83.333).abs() < 0.01);
    }

    #[test]
    fn test_composite_score_empty() {
        let score = ResearchEngine::compute_composite_score(&[], &[]);
        assert_eq!(score, 0.0);
    }

    #[test]
    fn test_check_nan() {
        let metrics = vec![
            MetricValue { name: "ok".into(), value: 1.0, unit: None },
            MetricValue { name: "bad".into(), value: f64::NAN, unit: None },
            MetricValue { name: "inf".into(), value: f64::INFINITY, unit: None },
        ];
        let violations = ResearchEngine::check_nan(&metrics);
        assert_eq!(violations.len(), 2);
    }

    #[test]
    fn test_validate_file_changes_ok() {
        let config = ResearchConfig {
            editable_files: vec![PathBuf::from("train.py")],
            ..Default::default()
        };
        let changes = vec![FileChange {
            path: PathBuf::from("train.py"),
            diff: "+x".into(),
            lines_added: 1,
            lines_removed: 0,
        }];
        let v = ResearchEngine::validate_file_changes(&changes, &config);
        assert!(v.is_empty());
    }

    #[test]
    fn test_validate_file_changes_forbidden_path() {
        let config = ResearchConfig::default();
        let changes = vec![FileChange {
            path: PathBuf::from("/etc/passwd"),
            diff: "+x".into(),
            lines_added: 1,
            lines_removed: 0,
        }];
        let v = ResearchEngine::validate_file_changes(&changes, &config);
        assert!(!v.is_empty());
    }

    #[test]
    fn test_validate_file_changes_too_many() {
        let config = ResearchConfig {
            resource_limits: ResourceLimits { max_file_changes: 1, ..Default::default() },
            ..Default::default()
        };
        let changes = vec![
            FileChange { path: "a.py".into(), diff: "+".into(), lines_added: 1, lines_removed: 0 },
            FileChange { path: "b.py".into(), diff: "+".into(), lines_added: 1, lines_removed: 0 },
        ];
        let v = ResearchEngine::validate_file_changes(&changes, &config);
        assert!(v.iter().any(|s| s.contains("Too many")));
    }

    #[test]
    fn test_validate_file_not_in_editable() {
        let config = ResearchConfig {
            editable_files: vec![PathBuf::from("train.py")],
            ..Default::default()
        };
        let changes = vec![FileChange {
            path: PathBuf::from("prepare.py"),
            diff: "+x".into(),
            lines_added: 1,
            lines_removed: 0,
        }];
        let v = ResearchEngine::validate_file_changes(&changes, &config);
        assert!(v.iter().any(|s| s.contains("not in editable")));
    }

    #[test]
    fn test_record_experiment_keep() {
        let mut engine = ResearchEngine::new();
        let config = ResearchConfig::default();
        let sid = engine.create_session("test", config);

        let h = Hypothesis::new("h1", "Better LR", "Reason");
        let mut exp = Experiment::new("e1", &sid, h, "python train.py");
        exp.metrics = vec![MetricValue { name: "score".into(), value: 10.0, unit: None }];

        engine.record_experiment(&sid, exp).unwrap();

        let session = engine.get_session(&sid).unwrap();
        assert_eq!(session.experiments.len(), 1);
        assert_eq!(session.experiments[0].status, ExperimentStatus::Kept);
        assert!(session.current_best_score > 0.0);
    }

    #[test]
    fn test_record_experiment_discard() {
        let mut engine = ResearchEngine::new();
        let mut config = ResearchConfig::default();
        config.metrics = vec![MetricDef::new("score", "S", MetricDirection::Higher, 1.0)];
        let sid = engine.create_session("test", config);

        // First experiment sets baseline
        let h1 = Hypothesis::new("h1", "Baseline", "Reason");
        let mut exp1 = Experiment::new("e1", &sid, h1, "cmd");
        exp1.metrics = vec![MetricValue { name: "score".into(), value: 10.0, unit: None }];
        engine.record_experiment(&sid, exp1).unwrap();

        // Second experiment is worse
        let h2 = Hypothesis::new("h2", "Worse", "Reason");
        let mut exp2 = Experiment::new("e2", &sid, h2, "cmd");
        exp2.metrics = vec![MetricValue { name: "score".into(), value: 5.0, unit: None }];
        engine.record_experiment(&sid, exp2).unwrap();

        let session = engine.get_session(&sid).unwrap();
        assert_eq!(session.experiments[1].status, ExperimentStatus::Discarded);
    }

    #[test]
    fn test_record_experiment_nan_rejection() {
        let mut engine = ResearchEngine::new();
        let config = ResearchConfig::default();
        let sid = engine.create_session("test", config);

        let h = Hypothesis::new("h1", "NaN producer", "Reason");
        let mut exp = Experiment::new("e1", &sid, h, "cmd");
        exp.metrics = vec![MetricValue { name: "score".into(), value: f64::NAN, unit: None }];

        engine.record_experiment(&sid, exp).unwrap();

        let session = engine.get_session(&sid).unwrap();
        assert!(!session.experiments[0].safety_violations.is_empty());
    }

    #[test]
    fn test_analyze_session() {
        let mut engine = ResearchEngine::new();
        let config = ResearchConfig::default();
        let sid = engine.create_session("test", config);

        for i in 0..5 {
            let h = Hypothesis::new(&format!("h{}", i), &format!("exp {}", i), "Reason");
            let mut exp = Experiment::new(&format!("e{}", i), &sid, h, "cmd");
            exp.metrics = vec![MetricValue { name: "score".into(), value: (i as f64 + 1.0) * 2.0, unit: None }];
            exp.duration = Duration::from_secs(60);
            engine.record_experiment(&sid, exp).unwrap();
        }

        let analysis = engine.analyze_session(&sid).unwrap();
        assert_eq!(analysis.total_experiments, 5);
        assert_eq!(analysis.kept_count, 5); // All improving
        assert_eq!(analysis.discarded_count, 0);
        assert!(analysis.best_score > 0.0);
    }

    #[test]
    fn test_analyze_session_not_found() {
        let engine = ResearchEngine::new();
        assert!(engine.analyze_session("nonexistent").is_err());
    }

    #[test]
    fn test_suggest_next_greedy() {
        let mut engine = ResearchEngine::new();
        let config = ResearchConfig { strategy: SearchStrategy::Greedy, ..Default::default() };
        let sid = engine.create_session("test", config);
        let suggestions = engine.suggest_next(&sid).unwrap();
        assert!(!suggestions.is_empty());
    }

    #[test]
    fn test_suggest_next_beam() {
        let mut engine = ResearchEngine::new();
        let config = ResearchConfig {
            strategy: SearchStrategy::BeamSearch { beam_width: 3 },
            ..Default::default()
        };
        let sid = engine.create_session("test", config);
        let suggestions = engine.suggest_next(&sid).unwrap();
        assert!(suggestions.iter().any(|s| s.contains("Beam width")));
    }

    #[test]
    fn test_suggest_next_genetic() {
        let mut engine = ResearchEngine::new();
        let config = ResearchConfig {
            strategy: SearchStrategy::Genetic { population_size: 20, mutation_rate: 0.1 },
            ..Default::default()
        };
        let sid = engine.create_session("test", config);
        let suggestions = engine.suggest_next(&sid).unwrap();
        assert!(suggestions.iter().any(|s| s.contains("Population")));
    }

    #[test]
    fn test_suggest_next_combinatorial() {
        let mut engine = ResearchEngine::new();
        let config = ResearchConfig {
            strategy: SearchStrategy::Combinatorial { max_combinations: 5 },
            ..Default::default()
        };
        let sid = engine.create_session("test", config);
        let suggestions = engine.suggest_next(&sid).unwrap();
        assert!(suggestions.iter().any(|s| s.contains("Combine") || s.contains("combining")));
    }

    #[test]
    fn test_suggest_next_bayesian() {
        let mut engine = ResearchEngine::new();
        let config = ResearchConfig {
            strategy: SearchStrategy::Bayesian { exploration_weight: 0.5 },
            ..Default::default()
        };
        let sid = engine.create_session("test", config);
        let suggestions = engine.suggest_next(&sid).unwrap();
        assert!(suggestions.iter().any(|s| s.contains("Bayesian")));
    }

    #[test]
    fn test_suggest_next_not_found() {
        let engine = ResearchEngine::new();
        assert!(engine.suggest_next("nonexistent").is_err());
    }

    #[test]
    fn test_export_results_tsv() {
        let mut engine = ResearchEngine::new();
        let config = ResearchConfig::default();
        let sid = engine.create_session("test", config);

        let h = Hypothesis::new("h1", "Test change", "Reason");
        let mut exp = Experiment::new("e1", &sid, h, "python train.py");
        exp.metrics = vec![MetricValue { name: "score".into(), value: 5.0, unit: None }];
        exp.git_commit = Some("abc123".into());
        exp.duration = Duration::from_secs(120);
        engine.record_experiment(&sid, exp).unwrap();

        let tsv = engine.export_results_tsv(&sid).unwrap();
        assert!(tsv.contains("experiment_id"));
        assert!(tsv.contains("e1"));
        assert!(tsv.contains("abc123"));
        assert!(tsv.contains("KEEP"));
    }

    #[test]
    fn test_export_results_not_found() {
        let engine = ResearchEngine::new();
        assert!(engine.export_results_tsv("nonexistent").is_err());
    }

    #[test]
    fn test_list_sessions() {
        let mut engine = ResearchEngine::new();
        engine.create_session("run1", ResearchConfig::default());
        engine.create_session("run2", ResearchConfig::default());
        let list = engine.list_sessions();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].1, "run1");
        assert_eq!(list[1].1, "run2");
    }

    #[test]
    fn test_session_acceptance_rate() {
        let mut session = ResearchSession::new("s1", "test", ResearchConfig::default());
        assert_eq!(session.acceptance_rate(), 0.0);

        session.experiments.push({
            let h = Hypothesis::new("h1", "A", "R");
            let mut e = Experiment::new("e1", "s1", h, "cmd");
            e.status = ExperimentStatus::Kept;
            e
        });
        session.experiments.push({
            let h = Hypothesis::new("h2", "B", "R");
            let mut e = Experiment::new("e2", "s1", h, "cmd");
            e.status = ExperimentStatus::Discarded;
            e
        });
        assert!((session.acceptance_rate() - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_session_improvement_pct() {
        let mut session = ResearchSession::new("s1", "test", ResearchConfig::default());
        session.baseline_score = 100.0;
        session.current_best_score = 120.0;
        assert!((session.improvement_pct() - 20.0).abs() < 0.001);

        session.baseline_score = 0.0;
        assert_eq!(session.improvement_pct(), 0.0);
    }

    #[test]
    fn test_session_total_duration() {
        let mut session = ResearchSession::new("s1", "test", ResearchConfig::default());
        let h1 = Hypothesis::new("h1", "A", "R");
        let mut e1 = Experiment::new("e1", "s1", h1, "cmd");
        e1.duration = Duration::from_secs(60);
        session.experiments.push(e1);

        let h2 = Hypothesis::new("h2", "B", "R");
        let mut e2 = Experiment::new("e2", "s1", h2, "cmd");
        e2.duration = Duration::from_secs(90);
        session.experiments.push(e2);

        assert_eq!(session.total_duration(), Duration::from_secs(150));
    }

    #[test]
    fn test_session_kept_discarded_failed() {
        let mut session = ResearchSession::new("s1", "test", ResearchConfig::default());
        let mk = |id: &str, status: ExperimentStatus| {
            let h = Hypothesis::new(id, id, "R");
            let mut e = Experiment::new(id, "s1", h, "cmd");
            e.status = status;
            e
        };
        session.experiments.push(mk("e1", ExperimentStatus::Kept));
        session.experiments.push(mk("e2", ExperimentStatus::Discarded));
        session.experiments.push(mk("e3", ExperimentStatus::Failed("err".into())));
        session.experiments.push(mk("e4", ExperimentStatus::Timeout));
        session.experiments.push(mk("e5", ExperimentStatus::Crashed));

        assert_eq!(session.kept_experiments().len(), 1);
        assert_eq!(session.discarded_experiments().len(), 1);
        assert_eq!(session.failed_experiments().len(), 3);
    }

    #[test]
    fn test_search_strategy_equality() {
        assert_eq!(SearchStrategy::Greedy, SearchStrategy::Greedy);
        assert_ne!(SearchStrategy::Greedy, SearchStrategy::BeamSearch { beam_width: 3 });
        assert_eq!(
            SearchStrategy::BeamSearch { beam_width: 5 },
            SearchStrategy::BeamSearch { beam_width: 5 },
        );
        assert_ne!(
            SearchStrategy::Genetic { population_size: 10, mutation_rate: 0.1 },
            SearchStrategy::Genetic { population_size: 20, mutation_rate: 0.1 },
        );
    }

    #[test]
    fn test_experiment_status_variants() {
        let statuses = vec![
            ExperimentStatus::Pending,
            ExperimentStatus::Running,
            ExperimentStatus::Completed,
            ExperimentStatus::Failed("err".into()),
            ExperimentStatus::Timeout,
            ExperimentStatus::Crashed,
            ExperimentStatus::Kept,
            ExperimentStatus::Discarded,
        ];
        assert_eq!(statuses.len(), 8);
        assert_ne!(statuses[0], statuses[1]);
    }

    #[test]
    fn test_safety_violation_variants() {
        let v1 = SafetyViolation::Timeout {
            elapsed: Duration::from_secs(600),
            limit: Duration::from_secs(300),
        };
        let v2 = SafetyViolation::MemoryExceeded { used_mb: 32000, limit_mb: 16384 };
        let v3 = SafetyViolation::NaNDetected { metric: "loss".into() };
        let v4 = SafetyViolation::ProcessCrash { exit_code: 1, stderr: "segfault".into() };
        let v5 = SafetyViolation::DiskSpaceExceeded { used_mb: 20000, limit_mb: 10240 };
        let v6 = SafetyViolation::ResourceLeak { description: "open files".into() };
        assert_ne!(v1, v2);
        assert_ne!(v3, v4);
        assert_ne!(v5, v6);
    }

    #[test]
    fn test_confidence_levels() {
        let levels = vec![
            Confidence::Speculative,
            Confidence::Low,
            Confidence::Medium,
            Confidence::High,
            Confidence::Validated,
        ];
        assert_eq!(levels.len(), 5);
        assert_ne!(levels[0], levels[4]);
    }

    #[test]
    fn test_session_status_variants() {
        let statuses = vec![
            SessionStatus::Idle,
            SessionStatus::Running,
            SessionStatus::Paused,
            SessionStatus::Completed,
            SessionStatus::Aborted("reason".into()),
        ];
        assert_eq!(statuses.len(), 5);
    }

    #[test]
    fn test_multiple_sessions() {
        let mut engine = ResearchEngine::new();
        let id1 = engine.create_session("ML run", ResearchConfig {
            domain: ResearchDomain::MlTraining,
            ..Default::default()
        });
        let id2 = engine.create_session("API bench", ResearchConfig {
            domain: ResearchDomain::ApiPerformance,
            ..Default::default()
        });

        assert_eq!(engine.sessions.len(), 2);
        assert_eq!(engine.active_session, Some(id2.clone())); // Last created is active

        let s1 = engine.get_session(&id1).unwrap();
        assert_eq!(s1.name, "ML run");

        let s2 = engine.get_session(&id2).unwrap();
        assert_eq!(s2.name, "API bench");
    }

    #[test]
    fn test_beam_candidate() {
        let c = BeamCandidate {
            experiment_id: "e1".into(),
            score: 0.95,
            git_ref: "abc123".into(),
            depth: 3,
        };
        assert_eq!(c.depth, 3);
        assert_eq!(c.score, 0.95);
    }

    #[test]
    fn test_genetic_individual() {
        let ind = GeneticIndividual {
            id: "g1".into(),
            genome: vec!["lr=0.01".into(), "layers=6".into()],
            fitness: 0.85,
            generation: 2,
            parents: vec!["g0_1".into(), "g0_3".into()],
        };
        assert_eq!(ind.generation, 2);
        assert_eq!(ind.genome.len(), 2);
    }

    #[test]
    fn test_file_change() {
        let fc = FileChange {
            path: PathBuf::from("src/model.py"),
            diff: "+new_line\n-old_line".into(),
            lines_added: 1,
            lines_removed: 1,
        };
        assert_eq!(fc.lines_added, 1);
    }

    #[test]
    fn test_research_lesson() {
        let lesson = ResearchLesson {
            id: "l1".into(),
            description: "Layer norm before attention is better".into(),
            evidence: vec!["e1".into(), "e5".into()],
            confidence: Confidence::High,
            domain: "ml_training".into(),
            tags: vec!["normalization".into(), "attention".into()],
            created_at: SystemTime::now(),
        };
        assert_eq!(lesson.evidence.len(), 2);
        assert_eq!(lesson.tags.len(), 2);
    }

    #[test]
    fn test_composite_score_missing_metric() {
        let metrics = vec![
            MetricValue { name: "a".into(), value: 10.0, unit: None },
        ];
        let defs = vec![
            MetricDef::new("a", "A", MetricDirection::Higher, 1.0),
            MetricDef::new("b", "B", MetricDirection::Higher, 1.0),
        ];
        // Only "a" is present, "b" is skipped — score based only on "a"
        let score = ResearchEngine::compute_composite_score(&metrics, &defs);
        assert_eq!(score, 10.0);
    }

    #[test]
    fn test_composite_score_lower_zero() {
        let metrics = vec![
            MetricValue { name: "loss".into(), value: 0.0, unit: None },
        ];
        let defs = vec![
            MetricDef::new("loss", "L", MetricDirection::Lower, 1.0),
        ];
        let score = ResearchEngine::compute_composite_score(&metrics, &defs);
        assert_eq!(score, f64::MAX);
    }

    #[test]
    fn test_greedy_with_memory() {
        let mut engine = ResearchEngine::new();
        let config = ResearchConfig::default();
        let sid = engine.create_session("test", config);

        // Record some outcomes to populate memory
        {
            let session = engine.get_session_mut(&sid).unwrap();
            session.memory.record_outcome(true, "increased learning rate");
            session.memory.record_outcome(false, "removed dropout");
        }

        let suggestions = engine.suggest_next(&sid).unwrap();
        assert!(suggestions.iter().any(|s| s.contains("increased learning rate")));
        assert!(suggestions.iter().any(|s| s.contains("removed dropout")));
    }

    #[test]
    fn test_record_experiment_session_not_found() {
        let mut engine = ResearchEngine::new();
        let h = Hypothesis::new("h1", "Test", "Reason");
        let exp = Experiment::new("e1", "nonexistent", h, "cmd");
        assert!(engine.record_experiment("nonexistent", exp).is_err());
    }

    #[test]
    fn test_active_mut() {
        let mut engine = ResearchEngine::new();
        assert!(engine.active_mut().is_none());

        engine.create_session("test", ResearchConfig::default());
        let session = engine.active_mut().unwrap();
        session.status = SessionStatus::Running;

        assert_eq!(engine.active().unwrap().status, SessionStatus::Running);
    }

    #[test]
    fn test_research_analysis_fields() {
        let analysis = ResearchAnalysis {
            total_experiments: 10,
            kept_count: 4,
            discarded_count: 5,
            failed_count: 1,
            acceptance_rate: 0.4444,
            best_score: 0.95,
            baseline_score: 0.80,
            improvement_pct: 18.75,
            best_experiment_id: Some("e7".into()),
            top_changes: vec![("change1".into(), 0.05), ("change2".into(), 0.03)],
            metric_trends: HashMap::new(),
            total_duration: Duration::from_secs(600),
            avg_experiment_duration: Duration::from_secs(60),
        };
        assert_eq!(analysis.kept_count, 4);
        assert_eq!(analysis.top_changes.len(), 2);
    }

    #[test]
    fn test_metric_value() {
        let mv = MetricValue {
            name: "loss".into(),
            value: 0.42,
            unit: Some("bpb".into()),
        };
        assert_eq!(mv.value, 0.42);
        assert_eq!(mv.unit, Some("bpb".into()));
    }

    #[test]
    fn test_hypothesis_with_fields() {
        let mut h = Hypothesis::new("h1", "Test RoPE", "Rotary embeddings may help");
        h.confidence = Confidence::Medium;
        h.predicted_impact.insert("val_bpb".into(), -0.05);
        h.tags = vec!["attention".into(), "embeddings".into()];
        h.parent_hypothesis = Some("h0".into());

        assert_eq!(h.confidence, Confidence::Medium);
        assert_eq!(h.predicted_impact.len(), 1);
        assert_eq!(h.tags.len(), 2);
        assert_eq!(h.parent_hypothesis, Some("h0".into()));
    }

    #[test]
    fn test_experiment_with_changes() {
        let h = Hypothesis::new("h1", "Test", "Reason");
        let mut exp = Experiment::new("e1", "s1", h, "cmd");
        exp.file_changes = vec![
            FileChange { path: "a.py".into(), diff: "+1".into(), lines_added: 1, lines_removed: 0 },
            FileChange { path: "b.py".into(), diff: "-1".into(), lines_added: 0, lines_removed: 1 },
        ];
        exp.git_commit = Some("deadbeef".into());
        exp.git_branch = Some("autoresearch/run1".into());
        exp.parent_experiment = Some("e0".into());

        assert_eq!(exp.file_changes.len(), 2);
        assert_eq!(exp.git_commit, Some("deadbeef".into()));
    }

    #[test]
    fn test_validate_empty_editable_allows_all() {
        let config = ResearchConfig {
            editable_files: vec![], // empty = no restriction
            ..Default::default()
        };
        let changes = vec![FileChange {
            path: PathBuf::from("anything.py"),
            diff: "+x".into(),
            lines_added: 1,
            lines_removed: 0,
        }];
        let v = ResearchEngine::validate_file_changes(&changes, &config);
        // No forbidden path violation for "anything.py"
        assert!(v.is_empty());
    }

    #[test]
    fn test_export_tsv_multiple_experiments() {
        let mut engine = ResearchEngine::new();
        let config = ResearchConfig::default();
        let sid = engine.create_session("test", config);

        for i in 0..3 {
            let h = Hypothesis::new(&format!("h{}", i), &format!("change {}", i), "R");
            let mut exp = Experiment::new(&format!("e{}", i), &sid, h, "cmd");
            exp.metrics = vec![MetricValue { name: "score".into(), value: (i + 1) as f64, unit: None }];
            exp.duration = Duration::from_secs(30);
            engine.record_experiment(&sid, exp).unwrap();
        }

        let tsv = engine.export_results_tsv(&sid).unwrap();
        let lines: Vec<&str> = tsv.lines().collect();
        assert_eq!(lines.len(), 4); // header + 3 experiments
    }

    // ── MetricExtractor tests ────────────────────────────────────────────────

    #[test]
    fn test_extract_key_value() {
        let output = "val_bpb: 1.08\ntrain_loss: 2.1\ngpu_util: 85.3%\n";
        let metrics = MetricExtractor::extract(output, &ExtractionStrategy::KeyValue { separator: ": ".into() });
        assert_eq!(metrics.len(), 3);
        assert_eq!(metrics[0].name, "val_bpb");
        assert!((metrics[0].value - 1.08).abs() < 0.001);
        assert_eq!(metrics[1].name, "train_loss");
        assert!((metrics[1].value - 2.1).abs() < 0.001);
    }

    #[test]
    fn test_extract_key_value_equals() {
        let output = "throughput=1250.5\nlatency=42\n";
        let metrics = MetricExtractor::extract(output, &ExtractionStrategy::KeyValue { separator: "=".into() });
        assert_eq!(metrics.len(), 2);
        assert!((metrics[0].value - 1250.5).abs() < 0.1);
    }

    #[test]
    fn test_extract_json() {
        let output = r#"some log output
{"metrics": {"val_bpb": 1.08, "loss": 2.1}}
more output"#;
        let metrics = MetricExtractor::extract(output, &ExtractionStrategy::JsonPath("metrics".into()));
        assert_eq!(metrics.len(), 2);
        assert!(metrics.iter().any(|m| m.name == "val_bpb"));
        assert!(metrics.iter().any(|m| m.name == "loss"));
    }

    #[test]
    fn test_extract_json_nested() {
        let output = r#"{"results": {"score": 0.95}}"#;
        let metrics = MetricExtractor::extract(output, &ExtractionStrategy::JsonPath("results.score".into()));
        assert_eq!(metrics.len(), 1);
        assert_eq!(metrics[0].name, "score");
        assert!((metrics[0].value - 0.95).abs() < 0.001);
    }

    #[test]
    fn test_extract_json_no_json() {
        let output = "plain text output with no JSON";
        let metrics = MetricExtractor::extract(output, &ExtractionStrategy::JsonPath("score".into()));
        assert!(metrics.is_empty());
    }

    #[test]
    fn test_extract_last_line() {
        let output = "epoch 1: val_bpb: 1.20\nepoch 2: val_bpb: 1.15\nval_bpb: 1.08\n";
        let metrics = MetricExtractor::extract(output, &ExtractionStrategy::LastLine { prefix: "val_bpb: ".into() });
        assert_eq!(metrics.len(), 1);
        assert!((metrics[0].value - 1.08).abs() < 0.001);
    }

    #[test]
    fn test_extract_last_line_not_found() {
        let output = "no matching prefix here\n";
        let metrics = MetricExtractor::extract(output, &ExtractionStrategy::LastLine { prefix: "val_bpb: ".into() });
        assert!(metrics.is_empty());
    }

    #[test]
    fn test_extract_regex_named_groups() {
        let output = "step 1000 | val_bpb=1.08 | loss=2.1";
        let metrics = MetricExtractor::extract(
            output,
            &ExtractionStrategy::Regex(r"(?P<val_bpb>[\d.]+).*(?P<loss>[\d.]+)$".into()),
        );
        // Named groups should capture values
        assert!(!metrics.is_empty());
    }

    #[test]
    fn test_extract_regex_invalid() {
        let output = "some output";
        let metrics = MetricExtractor::extract(output, &ExtractionStrategy::Regex(r"[invalid".into()));
        assert!(metrics.is_empty());
    }

    #[test]
    fn test_extract_key_value_negative() {
        let output = "delta: -0.05\n";
        let metrics = MetricExtractor::extract(output, &ExtractionStrategy::KeyValue { separator: ": ".into() });
        assert_eq!(metrics.len(), 1);
        assert!((metrics[0].value - (-0.05)).abs() < 0.001);
    }

    #[test]
    fn test_extract_key_value_scientific() {
        let output = "lr: 3e-4\n";
        let metrics = MetricExtractor::extract(output, &ExtractionStrategy::KeyValue { separator: ": ".into() });
        assert_eq!(metrics.len(), 1);
        assert!((metrics[0].value - 0.0003).abs() < 0.0001);
    }

    // ── StatisticalValidator tests ───────────────────────────────────────────

    #[test]
    fn test_welch_t_test_identical() {
        let a = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let b = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let (t, _, significant) = StatisticalValidator::welch_t_test(&a, &b, 0.05);
        assert!((t - 0.0).abs() < 0.001);
        assert!(!significant);
    }

    #[test]
    fn test_welch_t_test_different() {
        let a = vec![10.0, 11.0, 12.0, 13.0, 14.0];
        let b = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let (t, _, significant) = StatisticalValidator::welch_t_test(&a, &b, 0.05);
        assert!(t > 0.0);
        assert!(significant);
    }

    #[test]
    fn test_welch_t_test_insufficient_data() {
        let a = vec![1.0];
        let b = vec![2.0];
        let (_, p, significant) = StatisticalValidator::welch_t_test(&a, &b, 0.05);
        assert_eq!(p, 1.0);
        assert!(!significant);
    }

    #[test]
    fn test_welch_t_test_zero_variance() {
        let a = vec![5.0, 5.0, 5.0];
        let b = vec![5.0, 5.0, 5.0];
        let (t, _, significant) = StatisticalValidator::welch_t_test(&a, &b, 0.05);
        assert_eq!(t, 0.0);
        assert!(!significant);
    }

    #[test]
    fn test_bootstrap_ci_identical() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![1.0, 2.0, 3.0];
        let (lower, upper, mean_diff) = StatisticalValidator::bootstrap_ci(&a, &b, 0.95, 100);
        assert!((mean_diff - 0.0).abs() < 0.001);
        assert!(lower <= mean_diff);
        assert!(upper >= mean_diff);
    }

    #[test]
    fn test_bootstrap_ci_different() {
        let a = vec![10.0, 11.0, 12.0];
        let b = vec![1.0, 2.0, 3.0];
        let (lower, upper, mean_diff) = StatisticalValidator::bootstrap_ci(&a, &b, 0.95, 100);
        assert!(mean_diff > 0.0);
        assert!(lower > 0.0); // CI should not cross zero for large difference
        assert!(upper > lower);
    }

    #[test]
    fn test_bootstrap_ci_empty() {
        let (l, u, d) = StatisticalValidator::bootstrap_ci(&[], &[1.0], 0.95, 100);
        assert_eq!(l, 0.0);
        assert_eq!(u, 0.0);
        assert_eq!(d, 0.0);
    }

    #[test]
    fn test_cohens_d_large() {
        let a = vec![10.0, 11.0, 12.0, 13.0];
        let b = vec![1.0, 2.0, 3.0, 4.0];
        let d = StatisticalValidator::cohens_d(&a, &b);
        assert!(d > 2.0); // Very large effect
    }

    #[test]
    fn test_cohens_d_zero() {
        let a = vec![5.0, 5.0, 5.0];
        let b = vec![5.0, 5.0, 5.0];
        let d = StatisticalValidator::cohens_d(&a, &b);
        assert_eq!(d, 0.0);
    }

    #[test]
    fn test_cohens_d_insufficient() {
        assert_eq!(StatisticalValidator::cohens_d(&[1.0], &[2.0]), 0.0);
    }

    #[test]
    fn test_is_significant_higher() {
        let baseline = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let experiment = vec![10.0, 11.0, 12.0, 13.0, 14.0];
        assert!(StatisticalValidator::is_significant_improvement(
            &baseline, &experiment, &MetricDirection::Higher, 0.05
        ));
    }

    #[test]
    fn test_is_significant_lower() {
        let baseline = vec![10.0, 11.0, 12.0, 13.0];
        let experiment = vec![1.0, 2.0, 3.0, 4.0];
        assert!(StatisticalValidator::is_significant_improvement(
            &baseline, &experiment, &MetricDirection::Lower, 0.05
        ));
    }

    #[test]
    fn test_is_significant_insufficient_data() {
        // Falls back to mean comparison
        let baseline = vec![5.0];
        let experiment = vec![10.0];
        assert!(StatisticalValidator::is_significant_improvement(
            &baseline, &experiment, &MetricDirection::Higher, 0.05
        ));
    }

    // ── ExperimentGraph tests ────────────────────────────────────────────────

    #[test]
    fn test_graph_new() {
        let g = ExperimentGraph::new();
        assert!(g.roots.is_empty());
        assert!(g.edges.is_empty());
    }

    #[test]
    fn test_graph_add_root() {
        let mut g = ExperimentGraph::new();
        g.add_experiment("e1", None);
        assert_eq!(g.roots, vec!["e1"]);
    }

    #[test]
    fn test_graph_add_child() {
        let mut g = ExperimentGraph::new();
        g.add_experiment("e1", None);
        g.add_experiment("e2", Some("e1"));
        assert_eq!(g.children_of("e1"), vec!["e2"]);
    }

    #[test]
    fn test_graph_ancestry() {
        let mut g = ExperimentGraph::new();
        g.add_experiment("e1", None);
        g.add_experiment("e2", Some("e1"));
        g.add_experiment("e3", Some("e2"));
        assert_eq!(g.ancestry("e3"), vec!["e1", "e2", "e3"]);
    }

    #[test]
    fn test_graph_depth() {
        let mut g = ExperimentGraph::new();
        g.add_experiment("e1", None);
        g.add_experiment("e2", Some("e1"));
        g.add_experiment("e3", Some("e2"));
        assert_eq!(g.depth("e1"), 0);
        assert_eq!(g.depth("e2"), 1);
        assert_eq!(g.depth("e3"), 2);
    }

    #[test]
    fn test_graph_leaves() {
        let mut g = ExperimentGraph::new();
        g.add_experiment("e1", None);
        g.add_experiment("e2", Some("e1"));
        g.add_experiment("e3", Some("e1"));
        let leaves = g.leaves();
        assert!(leaves.contains(&"e2"));
        assert!(leaves.contains(&"e3"));
        assert!(!leaves.contains(&"e1"));
    }

    #[test]
    fn test_graph_best_branch_point() {
        let mut g = ExperimentGraph::new();
        g.add_experiment("e1", None);
        g.add_experiment("e2", Some("e1"));
        g.add_experiment("e3", Some("e1"));
        g.add_experiment("e4", Some("e2"));
        // e1 is ancestor of all, e2 is ancestor of e4
        let best = g.best_branch_point(&["e2", "e3", "e4"]);
        assert!(best.is_some());
    }

    #[test]
    fn test_graph_children_empty() {
        let g = ExperimentGraph::new();
        assert!(g.children_of("nonexistent").is_empty());
    }

    // ── HypothesisGenerator tests ────────────────────────────────────────────

    #[test]
    fn test_hypothesis_gen_empty_session() {
        let session = ResearchSession::new("s1", "test", ResearchConfig::default());
        let hypotheses = HypothesisGenerator::generate(&session);
        // No experiments = no hypotheses
        assert!(hypotheses.is_empty());
    }

    #[test]
    fn test_hypothesis_gen_with_kept() {
        let mut session = ResearchSession::new("s1", "test", ResearchConfig::default());
        let h = Hypothesis::new("h1", "Increased LR", "Reason");
        let mut e = Experiment::new("e1", "s1", h, "cmd");
        e.status = ExperimentStatus::Kept;
        e.delta = 0.05;
        session.experiments.push(e);

        let hypotheses = HypothesisGenerator::generate(&session);
        assert!(!hypotheses.is_empty());
        assert!(hypotheses.iter().any(|h| h.description.contains("Variation")));
    }

    #[test]
    fn test_hypothesis_gen_with_discarded() {
        let mut session = ResearchSession::new("s1", "test", ResearchConfig::default());
        let h = Hypothesis::new("h1", "Removed dropout", "Reason");
        let mut e = Experiment::new("e1", "s1", h, "cmd");
        e.status = ExperimentStatus::Discarded;
        session.experiments.push(e);

        let hypotheses = HypothesisGenerator::generate(&session);
        assert!(hypotheses.iter().any(|h| h.description.contains("Opposite")));
    }

    #[test]
    fn test_hypothesis_gen_combination() {
        let mut session = ResearchSession::new("s1", "test", ResearchConfig::default());
        for i in 0..3 {
            let h = Hypothesis::new(&format!("h{}", i), &format!("change {}", i), "R");
            let mut e = Experiment::new(&format!("e{}", i), "s1", h, "cmd");
            e.status = ExperimentStatus::Kept;
            e.delta = 0.01;
            session.experiments.push(e);
        }

        let hypotheses = HypothesisGenerator::generate(&session);
        assert!(hypotheses.iter().any(|h| h.description.contains("Combine")));
    }

    #[test]
    fn test_hypothesis_gen_metric_driven() {
        let mut session = ResearchSession::new("s1", "test", ResearchConfig {
            metrics: vec![
                MetricDef::new("a", "A", MetricDirection::Higher, 1.0),
                MetricDef::new("b", "B", MetricDirection::Higher, 0.5),
            ],
            ..Default::default()
        });
        let h = Hypothesis::new("h1", "Test", "R");
        let mut e = Experiment::new("e1", "s1", h, "cmd");
        e.metrics = vec![
            MetricValue { name: "a".into(), value: 100.0, unit: None },
            MetricValue { name: "b".into(), value: 1.0, unit: None },
        ];
        e.status = ExperimentStatus::Kept;
        session.experiments.push(e);

        let hypotheses = HypothesisGenerator::generate(&session);
        assert!(hypotheses.iter().any(|h| h.description.contains("weakest")));
    }

    #[test]
    fn test_hypothesis_gen_novelty() {
        let mut session = ResearchSession::new("s1", "test", ResearchConfig::default());
        session.memory.successful_patterns = vec!["a".into(), "b".into(), "c".into()];
        session.memory.failed_patterns = vec!["d".into(), "e".into(), "f".into()];

        let hypotheses = HypothesisGenerator::generate(&session);
        assert!(hypotheses.iter().any(|h| h.tags.contains(&"exploration".to_string())));
    }

    // ── WorktreeRunner tests ─────────────────────────────────────────────────

    #[test]
    fn test_worktree_runner_new() {
        let runner = WorktreeRunner::new(PathBuf::from("/tmp/repo"), 3);
        assert_eq!(runner.slots.len(), 3);
        assert_eq!(runner.max_parallel, 3);
    }

    #[test]
    fn test_worktree_available_slot() {
        let runner = WorktreeRunner::new(PathBuf::from("/tmp"), 2);
        assert!(runner.available_slot().is_some());
        assert_eq!(runner.running_count(), 0);
    }

    #[test]
    fn test_worktree_assign_and_release() {
        let mut runner = WorktreeRunner::new(PathBuf::from("/tmp"), 2);
        assert!(runner.assign_experiment("wt_0", "e1"));
        assert_eq!(runner.running_count(), 1);

        // Can't assign to same slot
        assert!(!runner.assign_experiment("wt_0", "e2"));

        runner.release_slot("wt_0");
        assert_eq!(runner.running_count(), 0);
        assert!(runner.available_slot().is_some());
    }

    #[test]
    fn test_worktree_all_busy() {
        let mut runner = WorktreeRunner::new(PathBuf::from("/tmp"), 1);
        runner.assign_experiment("wt_0", "e1");
        assert!(runner.available_slot().is_none());
    }

    #[test]
    fn test_worktree_commands() {
        let runner = WorktreeRunner::new(PathBuf::from("/tmp"), 1);
        let slot = &runner.slots[0];
        let create_cmds = WorktreeRunner::create_worktree_commands(slot, "HEAD");
        assert_eq!(create_cmds.len(), 1);
        assert!(create_cmds[0].contains("git worktree add"));

        let cleanup_cmds = WorktreeRunner::cleanup_worktree_commands(slot);
        assert_eq!(cleanup_cmds.len(), 2);
        assert!(cleanup_cmds[0].contains("worktree remove"));
        assert!(cleanup_cmds[1].contains("branch -D"));
    }

    // ── WarmStarter tests ────────────────────────────────────────────────────

    #[test]
    fn test_warm_start_same_domain() {
        let mut new_session = ResearchSession::new("s2", "new", ResearchConfig {
            domain: ResearchDomain::MlTraining,
            ..Default::default()
        });
        let mut past = ResearchSession::new("s1", "past", ResearchConfig {
            domain: ResearchDomain::MlTraining,
            ..Default::default()
        });
        past.memory.successful_patterns = vec!["RoPE".into()];
        past.memory.failed_patterns = vec!["no dropout".into()];
        past.memory.lessons.push(ResearchLesson {
            id: "l1".into(), description: "RoPE helps".into(),
            evidence: vec![], confidence: Confidence::High,
            domain: "ml".into(), tags: vec![], created_at: SystemTime::now(),
        });

        WarmStarter::warm_start(&mut new_session, &[past]);
        assert_eq!(new_session.memory.successful_patterns.len(), 1);
        assert_eq!(new_session.memory.failed_patterns.len(), 1);
        assert_eq!(new_session.memory.lessons.len(), 1);
        assert_eq!(new_session.memory.total_sessions, 1);
    }

    #[test]
    fn test_warm_start_different_domain() {
        let mut new_session = ResearchSession::new("s2", "new", ResearchConfig {
            domain: ResearchDomain::ApiPerformance,
            ..Default::default()
        });
        let mut past = ResearchSession::new("s1", "past", ResearchConfig {
            domain: ResearchDomain::MlTraining,
            ..Default::default()
        });
        past.memory.successful_patterns = vec!["RoPE".into()];

        WarmStarter::warm_start(&mut new_session, &[past]);
        // Different domain — should not import patterns
        assert!(new_session.memory.successful_patterns.is_empty());
    }

    #[test]
    fn test_warm_start_no_duplicates() {
        let mut new_session = ResearchSession::new("s2", "new", ResearchConfig {
            domain: ResearchDomain::MlTraining,
            ..Default::default()
        });
        new_session.memory.successful_patterns = vec!["RoPE".into()];

        let mut past = ResearchSession::new("s1", "past", ResearchConfig {
            domain: ResearchDomain::MlTraining,
            ..Default::default()
        });
        past.memory.successful_patterns = vec!["RoPE".into(), "bigger LR".into()];

        WarmStarter::warm_start(&mut new_session, &[past]);
        assert_eq!(new_session.memory.successful_patterns.len(), 2); // RoPE not duplicated
    }

    #[test]
    fn test_estimate_experiments_needed() {
        let mut past = ResearchSession::new("s1", "past", ResearchConfig::default());
        // 10 experiments, 4 kept, avg delta 0.02
        for i in 0..10 {
            let h = Hypothesis::new(&format!("h{}", i), &format!("e{}", i), "R");
            let mut e = Experiment::new(&format!("e{}", i), "s1", h, "cmd");
            if i < 4 {
                e.status = ExperimentStatus::Kept;
                e.delta = 0.02;
            } else {
                e.status = ExperimentStatus::Discarded;
                e.delta = -0.01;
            }
            past.experiments.push(e);
        }

        let needed = WarmStarter::estimate_experiments_needed(&[past], 10.0);
        assert!(needed >= 5);
        assert!(needed <= 1000);
    }

    #[test]
    fn test_estimate_experiments_no_history() {
        let needed = WarmStarter::estimate_experiments_needed(&[], 10.0);
        assert_eq!(needed, 50);
    }

    #[test]
    fn test_estimate_experiments_zero_acceptance() {
        let mut past = ResearchSession::new("s1", "past", ResearchConfig::default());
        let h = Hypothesis::new("h1", "e1", "R");
        let mut e = Experiment::new("e1", "s1", h, "cmd");
        e.status = ExperimentStatus::Discarded;
        e.delta = -0.01;
        past.experiments.push(e);

        let needed = WarmStarter::estimate_experiments_needed(&[past], 10.0);
        assert_eq!(needed, 100); // fallback
    }
}
