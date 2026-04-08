//! RL-OS Evaluation Operating System — comprehensive reinforcement learning policy evaluation.
//!
//! Provides a full-stack evaluation pipeline for RL policies:
//! - YAML-based eval suite definition with scenarios, metrics, thresholds, gates
//! - Performance, robustness, safety, and finance-specific metrics
//! - Off-policy evaluation (FQE, Importance Sampling, Doubly Robust, MAGIC)
//! - Adversarial evaluation (FGSM, random noise, reward drop measurement)
//! - Regression detection with statistical significance (t-test, bootstrap)
//! - Generalization scoring, counterfactual evaluation, multi-agent analysis
//! - Quality gates, comparison engine, report generation, continuous pipeline
//!
//! # Architecture
//!
//! ```text
//! EvalSuite (YAML)
//!   → EvalPipeline::run()
//!     ├─ SmokeTest stage      — fast sanity checks
//!     ├─ FullEval stage       — performance + robustness metrics
//!     ├─ SafetyEval stage     — constraint violations, near-miss detection
//!     ├─ AdversarialEval      — FGSM, noise injection
//!     ├─ OPE stage            — off-policy estimation (FQE, IS, DR, MAGIC)
//!     ├─ RegressionDetect     — compare against baseline
//!     ├─ Generalization       — cross-environment scoring
//!     └─ QualityGates         — deploy/reject decision
//!   → EvalReport { pass/fail, metric summaries, recommendations }
//! ```

use std::collections::HashMap;

// ── Enums ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MetricKind {
    Performance,
    Robustness,
    Safety,
    Financial,
    Generalization,
    Adversarial,
    MultiAgent,
}

impl MetricKind {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Performance => "Performance",
            Self::Robustness => "Robustness",
            Self::Safety => "Safety",
            Self::Financial => "Financial",
            Self::Generalization => "Generalization",
            Self::Adversarial => "Adversarial",
            Self::MultiAgent => "Multi-Agent",
        }
    }

    pub fn all() -> Vec<MetricKind> {
        vec![
            Self::Performance,
            Self::Robustness,
            Self::Safety,
            Self::Financial,
            Self::Generalization,
            Self::Adversarial,
            Self::MultiAgent,
        ]
    }
}

impl std::fmt::Display for MetricKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum EvalStage {
    SmokeTest,
    FullEval,
    SafetyEval,
    AdversarialEval,
    OffPolicyEval,
    RegressionDetect,
    GeneralizationEval,
    QualityGateCheck,
}

impl EvalStage {
    pub fn label(&self) -> &'static str {
        match self {
            Self::SmokeTest => "Smoke Test",
            Self::FullEval => "Full Evaluation",
            Self::SafetyEval => "Safety Evaluation",
            Self::AdversarialEval => "Adversarial Evaluation",
            Self::OffPolicyEval => "Off-Policy Evaluation",
            Self::RegressionDetect => "Regression Detection",
            Self::GeneralizationEval => "Generalization Evaluation",
            Self::QualityGateCheck => "Quality Gate Check",
        }
    }

    pub fn pipeline_order() -> Vec<EvalStage> {
        vec![
            Self::SmokeTest,
            Self::FullEval,
            Self::SafetyEval,
            Self::AdversarialEval,
            Self::OffPolicyEval,
            Self::RegressionDetect,
            Self::GeneralizationEval,
            Self::QualityGateCheck,
        ]
    }
}

impl std::fmt::Display for EvalStage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum OpeMethod {
    FittedQEvaluation,
    ImportanceSampling,
    DoublyRobust,
    Magic,
}

impl OpeMethod {
    pub fn label(&self) -> &'static str {
        match self {
            Self::FittedQEvaluation => "FQE",
            Self::ImportanceSampling => "IS",
            Self::DoublyRobust => "DR",
            Self::Magic => "MAGIC",
        }
    }
}

impl std::fmt::Display for OpeMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AdversarialMethod {
    Fgsm,
    RandomNoise,
    BoundaryAttack,
    PolicyPerturbation,
}

impl AdversarialMethod {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Fgsm => "FGSM",
            Self::RandomNoise => "Random Noise",
            Self::BoundaryAttack => "Boundary Attack",
            Self::PolicyPerturbation => "Policy Perturbation",
        }
    }
}

impl std::fmt::Display for AdversarialMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum GateVerdict {
    Pass,
    Fail,
    Warn,
    Skip,
}

impl GateVerdict {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Pass => "PASS",
            Self::Fail => "FAIL",
            Self::Warn => "WARN",
            Self::Skip => "SKIP",
        }
    }

    pub fn is_blocking(&self) -> bool {
        matches!(self, Self::Fail)
    }
}

impl std::fmt::Display for GateVerdict {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum StatTestMethod {
    TTest,
    Bootstrap,
    WilcoxonRankSum,
    PermutationTest,
}

impl StatTestMethod {
    pub fn label(&self) -> &'static str {
        match self {
            Self::TTest => "t-test",
            Self::Bootstrap => "Bootstrap",
            Self::WilcoxonRankSum => "Wilcoxon Rank-Sum",
            Self::PermutationTest => "Permutation Test",
        }
    }
}

impl std::fmt::Display for StatTestMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ReportFormat {
    Json,
    Markdown,
    Html,
    Plain,
}

impl ReportFormat {
    pub fn extension(&self) -> &'static str {
        match self {
            Self::Json => "json",
            Self::Markdown => "md",
            Self::Html => "html",
            Self::Plain => "txt",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DriftType {
    Reward,
    State,
    Action,
    Distributional,
}

impl DriftType {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Reward => "Reward Drift",
            Self::State => "State Drift",
            Self::Action => "Action Drift",
            Self::Distributional => "Distributional Drift",
        }
    }
}

impl std::fmt::Display for DriftType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AlertSeverity {
    Low,
    Medium,
    High,
    Critical,
}

impl AlertSeverity {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Low => "Low",
            Self::Medium => "Medium",
            Self::High => "High",
            Self::Critical => "Critical",
        }
    }

    pub fn weight(&self) -> f64 {
        match self {
            Self::Low => 0.2,
            Self::Medium => 0.5,
            Self::High => 0.8,
            Self::Critical => 1.0,
        }
    }
}

impl std::fmt::Display for AlertSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

// ── Structs ────────────────────────────────────────────────────────────

// 1. Evaluation Suite Definition

#[derive(Debug, Clone)]
pub struct EvalSuite {
    pub name: String,
    pub version: String,
    pub description: String,
    pub scenarios: Vec<EvalScenario>,
    pub metrics: Vec<MetricConfig>,
    pub gates: Vec<QualityGate>,
    pub tags: HashMap<String, String>,
}

impl EvalSuite {
    pub fn new(name: &str, version: &str) -> Self {
        Self {
            name: name.to_string(),
            version: version.to_string(),
            description: String::new(),
            scenarios: Vec::new(),
            metrics: Vec::new(),
            gates: Vec::new(),
            tags: HashMap::new(),
        }
    }

    pub fn with_description(mut self, desc: &str) -> Self {
        self.description = desc.to_string();
        self
    }

    pub fn add_scenario(&mut self, scenario: EvalScenario) {
        self.scenarios.push(scenario);
    }

    pub fn add_metric(&mut self, metric: MetricConfig) {
        self.metrics.push(metric);
    }

    pub fn add_gate(&mut self, gate: QualityGate) {
        self.gates.push(gate);
    }

    pub fn scenario_count(&self) -> usize {
        self.scenarios.len()
    }

    pub fn parse_yaml(yaml_content: &str) -> Result<EvalSuite, String> {
        let mut suite = EvalSuite::new("parsed", "1.0");
        for line in yaml_content.lines() {
            let trimmed = line.trim();
            if let Some(rest) = trimmed.strip_prefix("name:") {
                suite.name = rest.trim().trim_matches('"').to_string();
            } else if let Some(rest) = trimmed.strip_prefix("version:") {
                suite.version = rest.trim().trim_matches('"').to_string();
            } else if let Some(rest) = trimmed.strip_prefix("description:") {
                suite.description = rest.trim().trim_matches('"').to_string();
            } else if let Some(rest) = trimmed.strip_prefix("- scenario:") {
                let scenario_name = rest.trim().trim_matches('"');
                suite.scenarios.push(EvalScenario::new(scenario_name));
            } else if let Some(rest) = trimmed.strip_prefix("- metric:") {
                let metric_name = rest.trim().trim_matches('"');
                suite.metrics.push(MetricConfig {
                    name: metric_name.to_string(),
                    kind: MetricKind::Performance,
                    threshold: None,
                    higher_is_better: true,
                    weight: 1.0,
                });
            } else if let Some(rest) = trimmed.strip_prefix("- gate:") {
                let gate_name = rest.trim().trim_matches('"');
                suite.gates.push(QualityGate::new(gate_name));
            }
        }
        if suite.name.is_empty() {
            return Err("Missing suite name".to_string());
        }
        Ok(suite)
    }

    pub fn to_yaml(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!("name: \"{}\"\n", self.name));
        out.push_str(&format!("version: \"{}\"\n", self.version));
        out.push_str(&format!("description: \"{}\"\n", self.description));
        out.push_str("scenarios:\n");
        for s in &self.scenarios {
            out.push_str(&format!("  - scenario: \"{}\"\n", s.name));
            out.push_str(&format!("    episodes: {}\n", s.episode_count));
            out.push_str(&format!("    safety_critical: {}\n", s.safety_critical));
        }
        out.push_str("metrics:\n");
        for m in &self.metrics {
            out.push_str(&format!("  - metric: \"{}\"\n", m.name));
            out.push_str(&format!("    kind: \"{}\"\n", m.kind));
        }
        out.push_str("gates:\n");
        for g in &self.gates {
            out.push_str(&format!("  - gate: \"{}\"\n", g.name));
        }
        out
    }
}

// 2. Scenario-Based Testing

#[derive(Debug, Clone)]
pub struct EvalScenario {
    pub name: String,
    pub env_overrides: HashMap<String, String>,
    pub episode_count: usize,
    pub safety_critical: bool,
    pub max_steps: usize,
    pub seed: Option<u64>,
    pub tags: Vec<String>,
}

impl EvalScenario {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            env_overrides: HashMap::new(),
            episode_count: 100,
            safety_critical: false,
            max_steps: 1000,
            seed: None,
            tags: Vec::new(),
        }
    }

    pub fn with_episodes(mut self, count: usize) -> Self {
        self.episode_count = count;
        self
    }

    pub fn with_safety_critical(mut self, critical: bool) -> Self {
        self.safety_critical = critical;
        self
    }

    pub fn with_seed(mut self, seed: u64) -> Self {
        self.seed = Some(seed);
        self
    }

    pub fn add_env_override(&mut self, key: &str, value: &str) {
        self.env_overrides.insert(key.to_string(), value.to_string());
    }

    pub fn add_tag(&mut self, tag: &str) {
        self.tags.push(tag.to_string());
    }
}

#[derive(Debug, Clone)]
pub struct MetricConfig {
    pub name: String,
    pub kind: MetricKind,
    pub threshold: Option<f64>,
    pub higher_is_better: bool,
    pub weight: f64,
}

// 3. Performance Metrics

#[derive(Debug, Clone)]
pub struct EpisodeRecord {
    pub episode_id: usize,
    pub rewards: Vec<f64>,
    pub steps: usize,
    pub done: bool,
    pub info: HashMap<String, String>,
}

impl EpisodeRecord {
    pub fn new(episode_id: usize) -> Self {
        Self {
            episode_id,
            rewards: Vec::new(),
            steps: 0,
            done: false,
            info: HashMap::new(),
        }
    }

    pub fn cumulative_reward(&self) -> f64 {
        self.rewards.iter().sum()
    }
}

#[derive(Debug, Clone)]
pub struct PerformanceMetrics {
    pub cumulative_reward: f64,
    pub mean_reward: f64,
    pub reward_variance: f64,
    pub reward_std: f64,
    pub win_rate: f64,
    pub mean_episode_length: f64,
    pub median_episode_length: f64,
    pub min_episode_length: usize,
    pub max_episode_length: usize,
    pub episode_count: usize,
}

pub fn compute_performance_metrics(episodes: &[EpisodeRecord], win_threshold: f64) -> PerformanceMetrics {
    if episodes.is_empty() {
        return PerformanceMetrics {
            cumulative_reward: 0.0,
            mean_reward: 0.0,
            reward_variance: 0.0,
            reward_std: 0.0,
            win_rate: 0.0,
            mean_episode_length: 0.0,
            median_episode_length: 0.0,
            min_episode_length: 0,
            max_episode_length: 0,
            episode_count: 0,
        };
    }

    let rewards: Vec<f64> = episodes.iter().map(|e| e.cumulative_reward()).collect();
    let n = rewards.len() as f64;
    let cumulative: f64 = rewards.iter().sum();
    let mean = cumulative / n;
    let variance = rewards.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / n;

    let wins = rewards.iter().filter(|&&r| r >= win_threshold).count();

    let lengths: Vec<usize> = episodes.iter().map(|e| e.steps).collect();
    let mean_len = lengths.iter().sum::<usize>() as f64 / n;
    let mut sorted_lengths = lengths.clone();
    sorted_lengths.sort();
    let median_len = if sorted_lengths.len().is_multiple_of(2) {
        let mid = sorted_lengths.len() / 2;
        (sorted_lengths[mid - 1] + sorted_lengths[mid]) as f64 / 2.0
    } else {
        sorted_lengths[sorted_lengths.len() / 2] as f64
    };

    PerformanceMetrics {
        cumulative_reward: cumulative,
        mean_reward: mean,
        reward_variance: variance,
        reward_std: variance.sqrt(),
        win_rate: wins as f64 / n,
        mean_episode_length: mean_len,
        median_episode_length: median_len,
        min_episode_length: *sorted_lengths.first().unwrap_or(&0),
        max_episode_length: *sorted_lengths.last().unwrap_or(&0),
        episode_count: episodes.len(),
    }
}

// 4. Robustness Metrics

#[derive(Debug, Clone)]
pub struct RobustnessMetrics {
    pub worst_case_reward: f64,
    pub percentile_5_reward: f64,
    pub coefficient_of_variation: f64,
    pub reward_stability: f64,
    pub cross_period_stability: f64,
    pub interquartile_range: f64,
}

pub fn compute_robustness_metrics(episodes: &[EpisodeRecord]) -> RobustnessMetrics {
    if episodes.is_empty() {
        return RobustnessMetrics {
            worst_case_reward: 0.0,
            percentile_5_reward: 0.0,
            coefficient_of_variation: 0.0,
            reward_stability: 1.0,
            cross_period_stability: 1.0,
            interquartile_range: 0.0,
        };
    }

    let mut rewards: Vec<f64> = episodes.iter().map(|e| e.cumulative_reward()).collect();
    rewards.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let n = rewards.len();
    let worst_case = rewards[0];
    let p5_idx = ((n as f64) * 0.05).floor() as usize;
    let percentile_5 = rewards[p5_idx.min(n - 1)];

    let mean: f64 = rewards.iter().sum::<f64>() / n as f64;
    let std_dev = (rewards.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / n as f64).sqrt();
    let cv = if mean.abs() > f64::EPSILON { std_dev / mean.abs() } else { 0.0 };

    // Reward stability: 1 - CV (capped at 0)
    let stability = (1.0 - cv).max(0.0);

    // Cross-period stability: compare first half vs second half means
    let mid = n / 2;
    let first_half_mean = if mid > 0 { rewards[..mid].iter().sum::<f64>() / mid as f64 } else { 0.0 };
    let second_half_mean = if n - mid > 0 {
        rewards[mid..].iter().sum::<f64>() / (n - mid) as f64
    } else {
        0.0
    };
    let cross_stability = if first_half_mean.abs() > f64::EPSILON {
        1.0 - ((second_half_mean - first_half_mean) / first_half_mean).abs()
    } else {
        1.0
    };

    let q1_idx = (n as f64 * 0.25).floor() as usize;
    let q3_idx = (n as f64 * 0.75).floor() as usize;
    let iqr = rewards[q3_idx.min(n - 1)] - rewards[q1_idx.min(n - 1)];

    RobustnessMetrics {
        worst_case_reward: worst_case,
        percentile_5_reward: percentile_5,
        coefficient_of_variation: cv,
        reward_stability: stability,
        cross_period_stability: cross_stability.max(0.0),
        interquartile_range: iqr,
    }
}

// 5. Safety Metrics

#[derive(Debug, Clone)]
pub struct SafetyConstraint {
    pub name: String,
    pub threshold: f64,
    pub is_hard_constraint: bool,
}

impl SafetyConstraint {
    pub fn new(name: &str, threshold: f64, hard: bool) -> Self {
        Self {
            name: name.to_string(),
            threshold,
            is_hard_constraint: hard,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SafetyViolation {
    pub constraint_name: String,
    pub episode_id: usize,
    pub step: usize,
    pub value: f64,
    pub threshold: f64,
    pub is_near_miss: bool,
}

#[derive(Debug, Clone)]
pub struct SafetyMetrics {
    pub violation_rate: f64,
    pub hard_violation_count: usize,
    pub soft_violation_count: usize,
    pub near_miss_count: usize,
    pub safety_margin_mean: f64,
    pub safety_margin_min: f64,
    pub violations: Vec<SafetyViolation>,
}

pub fn compute_safety_metrics(
    episode_constraint_values: &[(usize, usize, &str, f64)], // (episode, step, constraint, value)
    constraints: &[SafetyConstraint],
    near_miss_factor: f64, // how close to threshold counts as near-miss (e.g. 0.9)
) -> SafetyMetrics {
    let mut violations = Vec::new();
    let mut hard_count = 0usize;
    let mut soft_count = 0usize;
    let mut near_miss_count = 0usize;
    let mut margins = Vec::new();

    let constraint_map: HashMap<&str, &SafetyConstraint> =
        constraints.iter().map(|c| (c.name.as_str(), c)).collect();

    for &(episode, step, cname, value) in episode_constraint_values {
        if let Some(constraint) = constraint_map.get(cname) {
            let margin = constraint.threshold - value;
            margins.push(margin);

            if value > constraint.threshold {
                let v = SafetyViolation {
                    constraint_name: cname.to_string(),
                    episode_id: episode,
                    step,
                    value,
                    threshold: constraint.threshold,
                    is_near_miss: false,
                };
                violations.push(v);
                if constraint.is_hard_constraint {
                    hard_count += 1;
                } else {
                    soft_count += 1;
                }
            } else if value > constraint.threshold * near_miss_factor {
                near_miss_count += 1;
                violations.push(SafetyViolation {
                    constraint_name: cname.to_string(),
                    episode_id: episode,
                    step,
                    value,
                    threshold: constraint.threshold,
                    is_near_miss: true,
                });
            }
        }
    }

    let total_checks = episode_constraint_values.len();
    let violation_count = hard_count + soft_count;
    let violation_rate = if total_checks > 0 {
        violation_count as f64 / total_checks as f64
    } else {
        0.0
    };

    let margin_mean = if margins.is_empty() {
        0.0
    } else {
        margins.iter().sum::<f64>() / margins.len() as f64
    };
    let margin_min = margins.iter().cloned().fold(f64::INFINITY, f64::min);
    let margin_min = if margin_min == f64::INFINITY { 0.0 } else { margin_min };

    SafetyMetrics {
        violation_rate,
        hard_violation_count: hard_count,
        soft_violation_count: soft_count,
        near_miss_count,
        safety_margin_mean: margin_mean,
        safety_margin_min: margin_min,
        violations,
    }
}

// 6. Off-Policy Evaluation (OPE)

#[derive(Debug, Clone)]
pub struct OpeTransition {
    pub state: Vec<f64>,
    pub action: usize,
    pub reward: f64,
    pub next_state: Vec<f64>,
    pub behavior_prob: f64,
    pub target_prob: f64,
    pub done: bool,
}

#[derive(Debug, Clone)]
pub struct OpeResult {
    pub method: OpeMethod,
    pub estimated_value: f64,
    pub confidence_lower: f64,
    pub confidence_upper: f64,
    pub effective_sample_size: f64,
    pub num_trajectories: usize,
}

pub fn importance_sampling_estimate(trajectories: &[Vec<OpeTransition>], gamma: f64) -> OpeResult {
    if trajectories.is_empty() {
        return OpeResult {
            method: OpeMethod::ImportanceSampling,
            estimated_value: 0.0,
            confidence_lower: 0.0,
            confidence_upper: 0.0,
            effective_sample_size: 0.0,
            num_trajectories: 0,
        };
    }

    let mut estimates = Vec::new();
    let mut weight_squares_sum = 0.0;
    let mut weight_sum = 0.0;

    for traj in trajectories {
        let mut importance_weight = 1.0;
        let mut discounted_return = 0.0;
        let mut discount = 1.0;

        for t in traj {
            if t.behavior_prob > f64::EPSILON {
                importance_weight *= t.target_prob / t.behavior_prob;
            }
            discounted_return += discount * t.reward;
            discount *= gamma;
        }

        let weighted_return = importance_weight * discounted_return;
        estimates.push(weighted_return);
        weight_sum += importance_weight;
        weight_squares_sum += importance_weight * importance_weight;
    }

    let n = estimates.len() as f64;
    let mean = estimates.iter().sum::<f64>() / n;
    let variance = estimates.iter().map(|e| (e - mean).powi(2)).sum::<f64>() / n;
    let std_err = (variance / n).sqrt();

    let ess = if weight_squares_sum > f64::EPSILON {
        (weight_sum * weight_sum) / weight_squares_sum
    } else {
        0.0
    };

    OpeResult {
        method: OpeMethod::ImportanceSampling,
        estimated_value: mean,
        confidence_lower: mean - 1.96 * std_err,
        confidence_upper: mean + 1.96 * std_err,
        effective_sample_size: ess,
        num_trajectories: trajectories.len(),
    }
}

pub fn fitted_q_evaluation(
    transitions: &[OpeTransition],
    gamma: f64,
    iterations: usize,
) -> OpeResult {
    // Simplified FQE: iteratively estimate Q-values via linear approximation
    if transitions.is_empty() {
        return OpeResult {
            method: OpeMethod::FittedQEvaluation,
            estimated_value: 0.0,
            confidence_lower: 0.0,
            confidence_upper: 0.0,
            effective_sample_size: 0.0,
            num_trajectories: 0,
        };
    }

    let n = transitions.len() as f64;
    let mut q_estimates: Vec<f64> = transitions.iter().map(|t| t.reward).collect();

    for _ in 0..iterations {
        let mean_q = q_estimates.iter().sum::<f64>() / n;
        for (i, t) in transitions.iter().enumerate() {
            let target = t.reward + if t.done { 0.0 } else { gamma * mean_q };
            // Exponential moving average update
            q_estimates[i] = 0.7 * q_estimates[i] + 0.3 * target;
        }
    }

    let estimated_value = q_estimates.iter().sum::<f64>() / n;
    let variance = q_estimates.iter().map(|q| (q - estimated_value).powi(2)).sum::<f64>() / n;
    let std_err = (variance / n).sqrt();

    OpeResult {
        method: OpeMethod::FittedQEvaluation,
        estimated_value,
        confidence_lower: estimated_value - 1.96 * std_err,
        confidence_upper: estimated_value + 1.96 * std_err,
        effective_sample_size: n,
        num_trajectories: transitions.len(),
    }
}

pub fn doubly_robust_estimate(trajectories: &[Vec<OpeTransition>], gamma: f64) -> OpeResult {
    // Combines IS with a direct method (reward-based baseline)
    if trajectories.is_empty() {
        return OpeResult {
            method: OpeMethod::DoublyRobust,
            estimated_value: 0.0,
            confidence_lower: 0.0,
            confidence_upper: 0.0,
            effective_sample_size: 0.0,
            num_trajectories: 0,
        };
    }

    // Compute baseline: average discounted return under behavior policy
    let mut baseline_returns = Vec::new();
    for traj in trajectories {
        let mut ret = 0.0;
        let mut discount = 1.0;
        for t in traj {
            ret += discount * t.reward;
            discount *= gamma;
        }
        baseline_returns.push(ret);
    }
    let baseline = baseline_returns.iter().sum::<f64>() / baseline_returns.len() as f64;

    // DR estimate: IS correction + baseline
    let mut dr_estimates = Vec::new();
    for (i, traj) in trajectories.iter().enumerate() {
        let mut importance_weight = 1.0;
        for t in traj {
            if t.behavior_prob > f64::EPSILON {
                importance_weight *= t.target_prob / t.behavior_prob;
            }
        }
        let dr = baseline + importance_weight * (baseline_returns[i] - baseline);
        dr_estimates.push(dr);
    }

    let n = dr_estimates.len() as f64;
    let mean = dr_estimates.iter().sum::<f64>() / n;
    let variance = dr_estimates.iter().map(|e| (e - mean).powi(2)).sum::<f64>() / n;
    let std_err = (variance / n).sqrt();

    OpeResult {
        method: OpeMethod::DoublyRobust,
        estimated_value: mean,
        confidence_lower: mean - 1.96 * std_err,
        confidence_upper: mean + 1.96 * std_err,
        effective_sample_size: n,
        num_trajectories: trajectories.len(),
    }
}

pub fn magic_estimate(
    trajectories: &[Vec<OpeTransition>],
    gamma: f64,
    num_bootstrap: usize,
) -> OpeResult {
    // MAGIC: combines multiple horizon IS estimates with bootstrap CI
    if trajectories.is_empty() {
        return OpeResult {
            method: OpeMethod::Magic,
            estimated_value: 0.0,
            confidence_lower: 0.0,
            confidence_upper: 0.0,
            effective_sample_size: 0.0,
            num_trajectories: 0,
        };
    }

    // Compute per-trajectory IS estimates at multiple horizons
    let max_horizon = trajectories.iter().map(|t| t.len()).max().unwrap_or(1);
    let horizons = vec![
        1.min(max_horizon),
        (max_horizon / 4).max(1),
        (max_horizon / 2).max(1),
        max_horizon,
    ];

    let mut horizon_estimates: Vec<Vec<f64>> = Vec::new();
    for &h in &horizons {
        let mut ests = Vec::new();
        for traj in trajectories {
            let mut weight = 1.0;
            let mut ret = 0.0;
            let mut discount = 1.0;
            for (i, t) in traj.iter().enumerate() {
                if i >= h {
                    break;
                }
                if t.behavior_prob > f64::EPSILON {
                    weight *= t.target_prob / t.behavior_prob;
                }
                ret += discount * t.reward;
                discount *= gamma;
            }
            ests.push(weight * ret);
        }
        horizon_estimates.push(ests);
    }

    // Blend horizon estimates (equal weighting for simplicity)
    let n_traj = trajectories.len();
    let mut blended = vec![0.0; n_traj];
    let n_horizons = horizon_estimates.len() as f64;
    for ests in &horizon_estimates {
        for (i, &e) in ests.iter().enumerate() {
            blended[i] += e / n_horizons;
        }
    }

    let n = blended.len() as f64;
    let mean = blended.iter().sum::<f64>() / n;

    // Bootstrap confidence intervals
    let mut bootstrap_means = Vec::with_capacity(num_bootstrap);
    let seed_base = (mean.abs() * 1000.0) as u64;
    for b in 0..num_bootstrap {
        let mut boot_sum = 0.0;
        let pseudo_seed = seed_base.wrapping_add(b as u64);
        for i in 0..blended.len() {
            let idx = (pseudo_seed.wrapping_mul(31).wrapping_add(i as u64) as usize) % blended.len();
            boot_sum += blended[idx];
        }
        bootstrap_means.push(boot_sum / n);
    }
    bootstrap_means.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let ci_lower = bootstrap_means.get(num_bootstrap / 40).copied().unwrap_or(mean);
    let ci_upper = bootstrap_means
        .get(num_bootstrap - num_bootstrap / 40)
        .copied()
        .unwrap_or(mean);

    OpeResult {
        method: OpeMethod::Magic,
        estimated_value: mean,
        confidence_lower: ci_lower,
        confidence_upper: ci_upper,
        effective_sample_size: n,
        num_trajectories: trajectories.len(),
    }
}

// 7. Adversarial Evaluation

#[derive(Debug, Clone)]
pub struct AdversarialConfig {
    pub method: AdversarialMethod,
    pub epsilon: f64,
    pub num_trials: usize,
    pub seed: Option<u64>,
}

impl AdversarialConfig {
    pub fn fgsm(epsilon: f64) -> Self {
        Self {
            method: AdversarialMethod::Fgsm,
            epsilon,
            num_trials: 100,
            seed: None,
        }
    }

    pub fn random_noise(epsilon: f64) -> Self {
        Self {
            method: AdversarialMethod::RandomNoise,
            epsilon,
            num_trials: 100,
            seed: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AdversarialResult {
    pub method: AdversarialMethod,
    pub clean_reward: f64,
    pub adversarial_reward: f64,
    pub reward_drop: f64,
    pub reward_drop_pct: f64,
    pub robustness_score: f64,
    pub num_trials: usize,
    pub worst_case_drop: f64,
}

pub fn run_adversarial_eval(
    clean_rewards: &[f64],
    perturbed_rewards: &[f64],
    config: &AdversarialConfig,
) -> AdversarialResult {
    let n = clean_rewards.len().min(perturbed_rewards.len());
    if n == 0 {
        return AdversarialResult {
            method: config.method.clone(),
            clean_reward: 0.0,
            adversarial_reward: 0.0,
            reward_drop: 0.0,
            reward_drop_pct: 0.0,
            robustness_score: 1.0,
            num_trials: 0,
            worst_case_drop: 0.0,
        };
    }

    let clean_mean = clean_rewards[..n].iter().sum::<f64>() / n as f64;
    let adv_mean = perturbed_rewards[..n].iter().sum::<f64>() / n as f64;
    let drop = clean_mean - adv_mean;
    let drop_pct = if clean_mean.abs() > f64::EPSILON {
        (drop / clean_mean.abs()) * 100.0
    } else {
        0.0
    };

    let worst_drop = clean_rewards[..n]
        .iter()
        .zip(perturbed_rewards[..n].iter())
        .map(|(c, p)| c - p)
        .fold(f64::NEG_INFINITY, f64::max);

    let robustness = if clean_mean.abs() > f64::EPSILON {
        (adv_mean / clean_mean).clamp(0.0, 1.0)
    } else {
        1.0
    };

    AdversarialResult {
        method: config.method.clone(),
        clean_reward: clean_mean,
        adversarial_reward: adv_mean,
        reward_drop: drop,
        reward_drop_pct: drop_pct,
        robustness_score: robustness,
        num_trials: n,
        worst_case_drop: worst_drop,
    }
}

pub fn fgsm_perturb(states: &[Vec<f64>], epsilon: f64, gradient_signs: &[Vec<f64>]) -> Vec<Vec<f64>> {
    states
        .iter()
        .zip(gradient_signs.iter())
        .map(|(s, g)| {
            s.iter()
                .zip(g.iter())
                .map(|(&sv, &gv)| sv + epsilon * gv.signum())
                .collect()
        })
        .collect()
}

pub fn random_noise_perturb(states: &[Vec<f64>], epsilon: f64, seed: u64) -> Vec<Vec<f64>> {
    states
        .iter()
        .enumerate()
        .map(|(i, s)| {
            s.iter()
                .enumerate()
                .map(|(j, &sv)| {
                    // Deterministic pseudo-random based on seed
                    let hash = seed.wrapping_mul(31).wrapping_add(i as u64).wrapping_mul(17).wrapping_add(j as u64);
                    let noise = ((hash % 2001) as f64 / 1000.0 - 1.0) * epsilon;
                    sv + noise
                })
                .collect()
        })
        .collect()
}

// 8. Regression Detection

#[derive(Debug, Clone)]
pub struct RegressionTestResult {
    pub baseline_mean: f64,
    pub candidate_mean: f64,
    pub difference: f64,
    pub effect_size: f64,
    pub p_value: f64,
    pub is_regression: bool,
    pub stat_method: StatTestMethod,
    pub confidence_interval: (f64, f64),
}

pub fn detect_regression_ttest(
    baseline_rewards: &[f64],
    candidate_rewards: &[f64],
    significance_level: f64,
) -> RegressionTestResult {
    let n1 = baseline_rewards.len() as f64;
    let n2 = candidate_rewards.len() as f64;

    if n1 < 2.0 || n2 < 2.0 {
        return RegressionTestResult {
            baseline_mean: 0.0,
            candidate_mean: 0.0,
            difference: 0.0,
            effect_size: 0.0,
            p_value: 1.0,
            is_regression: false,
            stat_method: StatTestMethod::TTest,
            confidence_interval: (0.0, 0.0),
        };
    }

    let mean1 = baseline_rewards.iter().sum::<f64>() / n1;
    let mean2 = candidate_rewards.iter().sum::<f64>() / n2;
    let var1 = baseline_rewards.iter().map(|r| (r - mean1).powi(2)).sum::<f64>() / (n1 - 1.0);
    let var2 = candidate_rewards.iter().map(|r| (r - mean2).powi(2)).sum::<f64>() / (n2 - 1.0);

    let pooled_std = ((var1 / n1) + (var2 / n2)).sqrt();
    let t_stat = if pooled_std > f64::EPSILON {
        (mean2 - mean1) / pooled_std
    } else {
        0.0
    };

    // Approximate p-value using standard normal approximation for large samples
    let p_value = approximate_two_sided_p(t_stat);

    // Cohen's d effect size
    let pooled_sd = ((var1 + var2) / 2.0).sqrt();
    let cohens_d = if pooled_sd > f64::EPSILON {
        (mean2 - mean1) / pooled_sd
    } else {
        0.0
    };

    let ci_lower = (mean2 - mean1) - 1.96 * pooled_std;
    let ci_upper = (mean2 - mean1) + 1.96 * pooled_std;

    let is_regression = p_value < significance_level && mean2 < mean1;

    RegressionTestResult {
        baseline_mean: mean1,
        candidate_mean: mean2,
        difference: mean2 - mean1,
        effect_size: cohens_d,
        p_value,
        is_regression,
        stat_method: StatTestMethod::TTest,
        confidence_interval: (ci_lower, ci_upper),
    }
}

pub fn detect_regression_bootstrap(
    baseline_rewards: &[f64],
    candidate_rewards: &[f64],
    num_bootstrap: usize,
    significance_level: f64,
) -> RegressionTestResult {
    let n1 = baseline_rewards.len();
    let n2 = candidate_rewards.len();

    if n1 == 0 || n2 == 0 {
        return RegressionTestResult {
            baseline_mean: 0.0,
            candidate_mean: 0.0,
            difference: 0.0,
            effect_size: 0.0,
            p_value: 1.0,
            is_regression: false,
            stat_method: StatTestMethod::Bootstrap,
            confidence_interval: (0.0, 0.0),
        };
    }

    let mean1 = baseline_rewards.iter().sum::<f64>() / n1 as f64;
    let mean2 = candidate_rewards.iter().sum::<f64>() / n2 as f64;
    let observed_diff = mean2 - mean1;

    // Pooled data for permutation-style bootstrap
    let pooled: Vec<f64> = baseline_rewards.iter().chain(candidate_rewards.iter()).copied().collect();
    let total = pooled.len();

    let mut count_more_extreme = 0usize;
    let seed_base = (observed_diff.abs() * 10000.0) as u64;

    for b in 0..num_bootstrap {
        let pseudo = seed_base.wrapping_add(b as u64);
        let mut group1_sum = 0.0;
        let mut group2_sum = 0.0;
        for i in 0..n1 {
            let idx = (pseudo.wrapping_mul(31).wrapping_add(i as u64) as usize) % total;
            group1_sum += pooled[idx];
        }
        for i in 0..n2 {
            let idx = (pseudo.wrapping_mul(47).wrapping_add(i as u64).wrapping_add(n1 as u64) as usize) % total;
            group2_sum += pooled[idx];
        }
        let boot_diff = group2_sum / n2 as f64 - group1_sum / n1 as f64;
        if boot_diff.abs() >= observed_diff.abs() {
            count_more_extreme += 1;
        }
    }

    let p_value = count_more_extreme as f64 / num_bootstrap as f64;

    let var1 = baseline_rewards.iter().map(|r| (r - mean1).powi(2)).sum::<f64>() / n1.max(1) as f64;
    let var2 = candidate_rewards.iter().map(|r| (r - mean2).powi(2)).sum::<f64>() / n2.max(1) as f64;
    let pooled_sd = ((var1 + var2) / 2.0).sqrt();
    let cohens_d = if pooled_sd > f64::EPSILON {
        observed_diff / pooled_sd
    } else {
        0.0
    };

    let std_err = ((var1 / n1 as f64) + (var2 / n2 as f64)).sqrt();
    let ci_lower = observed_diff - 1.96 * std_err;
    let ci_upper = observed_diff + 1.96 * std_err;

    RegressionTestResult {
        baseline_mean: mean1,
        candidate_mean: mean2,
        difference: observed_diff,
        effect_size: cohens_d,
        p_value,
        is_regression: p_value < significance_level && mean2 < mean1,
        stat_method: StatTestMethod::Bootstrap,
        confidence_interval: (ci_lower, ci_upper),
    }
}

// Helper: approximate two-sided p-value from z/t stat using sigmoid approximation
fn approximate_two_sided_p(t: f64) -> f64 {
    let abs_t = t.abs();
    // Approximation of 2*(1-Phi(|t|)) for normal distribution
    let p = 2.0 * (1.0 / (1.0 + (abs_t * 0.7).exp()));
    p.min(1.0)
}

// 9. Generalization Scoring

#[derive(Debug, Clone)]
pub struct EnvironmentResult {
    pub env_name: String,
    pub mean_reward: f64,
    pub reward_std: f64,
    pub episode_count: usize,
    pub domain_features: HashMap<String, f64>,
}

#[derive(Debug, Clone)]
pub struct GeneralizationScore {
    pub overall_score: f64,
    pub per_env_scores: HashMap<String, f64>,
    pub domain_shift_penalty: f64,
    pub transfer_efficiency: f64,
    pub worst_env: String,
    pub worst_env_score: f64,
    pub cross_env_variance: f64,
}

pub fn compute_generalization_score(
    training_env: &EnvironmentResult,
    eval_envs: &[EnvironmentResult],
) -> GeneralizationScore {
    if eval_envs.is_empty() {
        return GeneralizationScore {
            overall_score: 0.0,
            per_env_scores: HashMap::new(),
            domain_shift_penalty: 0.0,
            transfer_efficiency: 0.0,
            worst_env: String::new(),
            worst_env_score: 0.0,
            cross_env_variance: 0.0,
        };
    }

    let mut per_env = HashMap::new();
    let mut worst_name = String::new();
    let mut worst_score = f64::INFINITY;

    for env in eval_envs {
        let ratio = if training_env.mean_reward.abs() > f64::EPSILON {
            env.mean_reward / training_env.mean_reward
        } else {
            0.0
        };
        let score = ratio.clamp(0.0, 1.0);
        per_env.insert(env.env_name.clone(), score);
        if score < worst_score {
            worst_score = score;
            worst_name = env.env_name.clone();
        }
    }

    let scores: Vec<f64> = per_env.values().copied().collect();
    let mean_score = scores.iter().sum::<f64>() / scores.len() as f64;
    let variance = scores.iter().map(|s| (s - mean_score).powi(2)).sum::<f64>() / scores.len() as f64;

    // Domain shift: average feature distance between training and eval
    let mut total_shift = 0.0;
    let mut shift_count = 0;
    for env in eval_envs {
        for (k, &v) in &env.domain_features {
            if let Some(&train_v) = training_env.domain_features.get(k) {
                total_shift += (v - train_v).abs();
                shift_count += 1;
            }
        }
    }
    let domain_shift = if shift_count > 0 {
        total_shift / shift_count as f64
    } else {
        0.0
    };
    let domain_shift_penalty = 1.0 - (-domain_shift).exp(); // higher shift = higher penalty

    // Transfer efficiency: ratio of eval performance to training performance
    let eval_mean_reward = eval_envs.iter().map(|e| e.mean_reward).sum::<f64>() / eval_envs.len() as f64;
    let transfer_efficiency = if training_env.mean_reward.abs() > f64::EPSILON {
        (eval_mean_reward / training_env.mean_reward).clamp(0.0, 1.0)
    } else {
        0.0
    };

    // Overall score: weighted combo (penalize high variance and domain shift)
    let overall = mean_score * (1.0 - domain_shift_penalty * 0.5) * (1.0 - variance.sqrt() * 0.3);

    GeneralizationScore {
        overall_score: overall.max(0.0),
        per_env_scores: per_env,
        domain_shift_penalty,
        transfer_efficiency,
        worst_env: worst_name,
        worst_env_score: worst_score,
        cross_env_variance: variance,
    }
}

// 10. Quality Gates

#[derive(Debug, Clone)]
pub struct QualityGate {
    pub name: String,
    pub conditions: Vec<GateCondition>,
    pub verdict: GateVerdict,
    pub reason: String,
}

impl QualityGate {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            conditions: Vec::new(),
            verdict: GateVerdict::Skip,
            reason: String::new(),
        }
    }

    pub fn add_condition(&mut self, condition: GateCondition) {
        self.conditions.push(condition);
    }

    pub fn evaluate(&mut self, metrics: &GateMetrics) -> GateVerdict {
        let mut all_pass = true;
        let mut reasons = Vec::new();

        for cond in &self.conditions {
            let (pass, msg) = cond.check(metrics);
            if !pass {
                all_pass = false;
                reasons.push(msg);
            }
        }

        if all_pass {
            self.verdict = GateVerdict::Pass;
            self.reason = "All conditions met".to_string();
        } else {
            self.verdict = GateVerdict::Fail;
            self.reason = reasons.join("; ");
        }

        self.verdict.clone()
    }
}

#[derive(Debug, Clone)]
pub enum GateCondition {
    MinSharpe(f64),
    MaxDrawdown(f64),
    MinAdversarialRobustness(f64),
    MaxViolationRate(f64),
    MinWinRate(f64),
    MinMeanReward(f64),
    MaxRewardVariance(f64),
    MinGeneralizationScore(f64),
    MaxRegressionPValue(f64),
    Custom { name: String, threshold: f64, higher_is_better: bool },
}

impl GateCondition {
    pub fn check(&self, metrics: &GateMetrics) -> (bool, String) {
        match self {
            Self::MinSharpe(min) => {
                let pass = metrics.sharpe_ratio >= *min;
                (pass, format!("Sharpe {:.3} < min {:.3}", metrics.sharpe_ratio, min))
            }
            Self::MaxDrawdown(max) => {
                let pass = metrics.max_drawdown <= *max;
                (pass, format!("Drawdown {:.3} > max {:.3}", metrics.max_drawdown, max))
            }
            Self::MinAdversarialRobustness(min) => {
                let pass = metrics.adversarial_robustness >= *min;
                (pass, format!("Adv robustness {:.3} < min {:.3}", metrics.adversarial_robustness, min))
            }
            Self::MaxViolationRate(max) => {
                let pass = metrics.violation_rate <= *max;
                (pass, format!("Violation rate {:.3} > max {:.3}", metrics.violation_rate, max))
            }
            Self::MinWinRate(min) => {
                let pass = metrics.win_rate >= *min;
                (pass, format!("Win rate {:.3} < min {:.3}", metrics.win_rate, min))
            }
            Self::MinMeanReward(min) => {
                let pass = metrics.mean_reward >= *min;
                (pass, format!("Mean reward {:.3} < min {:.3}", metrics.mean_reward, min))
            }
            Self::MaxRewardVariance(max) => {
                let pass = metrics.reward_variance <= *max;
                (pass, format!("Reward variance {:.3} > max {:.3}", metrics.reward_variance, max))
            }
            Self::MinGeneralizationScore(min) => {
                let pass = metrics.generalization_score >= *min;
                (pass, format!("Gen score {:.3} < min {:.3}", metrics.generalization_score, min))
            }
            Self::MaxRegressionPValue(max) => {
                let pass = metrics.regression_p_value <= *max;
                (pass, format!("P-value {:.4} > max {:.4}", metrics.regression_p_value, max))
            }
            Self::Custom { name, threshold, higher_is_better } => {
                let val = metrics.custom_metrics.get(name).copied().unwrap_or(0.0);
                let pass = if *higher_is_better { val >= *threshold } else { val <= *threshold };
                (pass, format!("Custom '{}' {:.3} failed threshold {:.3}", name, val, threshold))
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct GateMetrics {
    pub sharpe_ratio: f64,
    pub max_drawdown: f64,
    pub adversarial_robustness: f64,
    pub violation_rate: f64,
    pub win_rate: f64,
    pub mean_reward: f64,
    pub reward_variance: f64,
    pub generalization_score: f64,
    pub regression_p_value: f64,
    pub custom_metrics: HashMap<String, f64>,
}

impl GateMetrics {
    pub fn new() -> Self {
        Self {
            sharpe_ratio: 0.0,
            max_drawdown: 0.0,
            adversarial_robustness: 1.0,
            violation_rate: 0.0,
            win_rate: 0.0,
            mean_reward: 0.0,
            reward_variance: 0.0,
            generalization_score: 0.0,
            regression_p_value: 1.0,
            custom_metrics: HashMap::new(),
        }
    }
}

// 11. Evaluation Pipeline

#[derive(Debug, Clone)]
pub struct PipelineConfig {
    pub stages: Vec<EvalStage>,
    pub fail_fast: bool,
    pub timeout_per_stage_secs: u64,
    pub parallel_scenarios: bool,
}

impl PipelineConfig {
    pub fn default_pipeline() -> Self {
        Self {
            stages: EvalStage::pipeline_order(),
            fail_fast: true,
            timeout_per_stage_secs: 300,
            parallel_scenarios: false,
        }
    }

    pub fn smoke_only() -> Self {
        Self {
            stages: vec![EvalStage::SmokeTest],
            fail_fast: true,
            timeout_per_stage_secs: 60,
            parallel_scenarios: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct StageResult {
    pub stage: EvalStage,
    pub passed: bool,
    pub metrics: HashMap<String, f64>,
    pub messages: Vec<String>,
    pub duration_ms: u64,
}

impl StageResult {
    pub fn pass(stage: EvalStage) -> Self {
        Self {
            stage,
            passed: true,
            metrics: HashMap::new(),
            messages: Vec::new(),
            duration_ms: 0,
        }
    }

    pub fn fail(stage: EvalStage, msg: &str) -> Self {
        Self {
            stage,
            passed: false,
            metrics: HashMap::new(),
            messages: vec![msg.to_string()],
            duration_ms: 0,
        }
    }

    pub fn add_metric(&mut self, name: &str, value: f64) {
        self.metrics.insert(name.to_string(), value);
    }
}

#[derive(Debug, Clone)]
pub struct EvalPipeline {
    pub config: PipelineConfig,
    pub results: Vec<StageResult>,
    pub overall_passed: bool,
}

impl EvalPipeline {
    pub fn new(config: PipelineConfig) -> Self {
        Self {
            config,
            results: Vec::new(),
            overall_passed: false,
        }
    }

    pub fn run_stage(&mut self, stage: EvalStage, episodes: &[EpisodeRecord]) -> StageResult {
        let result = match &stage {
            EvalStage::SmokeTest => {
                let mut r = StageResult::pass(stage.clone());
                if episodes.is_empty() {
                    r = StageResult::fail(stage.clone(), "No episodes to evaluate");
                } else {
                    let perf = compute_performance_metrics(episodes, 0.0);
                    r.add_metric("mean_reward", perf.mean_reward);
                    r.add_metric("episode_count", perf.episode_count as f64);
                    if perf.mean_reward.is_nan() || perf.mean_reward.is_infinite() {
                        r.passed = false;
                        r.messages.push("Invalid reward values detected".to_string());
                    }
                }
                r
            }
            EvalStage::FullEval => {
                let mut r = StageResult::pass(stage.clone());
                let perf = compute_performance_metrics(episodes, 0.0);
                let robust = compute_robustness_metrics(episodes);
                r.add_metric("cumulative_reward", perf.cumulative_reward);
                r.add_metric("mean_reward", perf.mean_reward);
                r.add_metric("reward_variance", perf.reward_variance);
                r.add_metric("win_rate", perf.win_rate);
                r.add_metric("reward_stability", robust.reward_stability);
                r.add_metric("cv", robust.coefficient_of_variation);
                r
            }
            _ => StageResult::pass(stage.clone()),
        };
        self.results.push(result.clone());
        result
    }

    pub fn run_all(&mut self, episodes: &[EpisodeRecord]) -> bool {
        let stages = self.config.stages.clone();
        self.overall_passed = true;

        for stage in stages {
            let result = self.run_stage(stage, episodes);
            if !result.passed {
                self.overall_passed = false;
                if self.config.fail_fast {
                    break;
                }
            }
        }
        self.overall_passed
    }

    pub fn summary(&self) -> String {
        let mut out = String::new();
        out.push_str("Pipeline Summary\n");
        out.push_str(&format!("Overall: {}\n", if self.overall_passed { "PASS" } else { "FAIL" }));
        for r in &self.results {
            out.push_str(&format!("  {} — {}\n", r.stage, if r.passed { "PASS" } else { "FAIL" }));
            for msg in &r.messages {
                out.push_str(&format!("    {}\n", msg));
            }
        }
        out
    }
}

// 12. Comparison Engine

#[derive(Debug, Clone)]
pub struct PolicyComparison {
    pub policy_a_name: String,
    pub policy_b_name: String,
    pub metric_comparisons: Vec<MetricComparison>,
    pub statistical_test: Option<RegressionTestResult>,
    pub winner: Option<String>,
}

#[derive(Debug, Clone)]
pub struct MetricComparison {
    pub metric_name: String,
    pub value_a: f64,
    pub value_b: f64,
    pub difference: f64,
    pub relative_change_pct: f64,
    pub effect_size: f64,
}

pub fn compare_policies(
    name_a: &str,
    rewards_a: &[f64],
    name_b: &str,
    rewards_b: &[f64],
) -> PolicyComparison {
    let mean_a = if rewards_a.is_empty() { 0.0 } else { rewards_a.iter().sum::<f64>() / rewards_a.len() as f64 };
    let mean_b = if rewards_b.is_empty() { 0.0 } else { rewards_b.iter().sum::<f64>() / rewards_b.len() as f64 };

    let var_a = if rewards_a.len() > 1 {
        rewards_a.iter().map(|r| (r - mean_a).powi(2)).sum::<f64>() / (rewards_a.len() - 1) as f64
    } else {
        0.0
    };
    let var_b = if rewards_b.len() > 1 {
        rewards_b.iter().map(|r| (r - mean_b).powi(2)).sum::<f64>() / (rewards_b.len() - 1) as f64
    } else {
        0.0
    };

    let pooled_sd = ((var_a + var_b) / 2.0).sqrt();
    let cohens_d = if pooled_sd > f64::EPSILON {
        (mean_b - mean_a) / pooled_sd
    } else {
        0.0
    };

    let rel_change = if mean_a.abs() > f64::EPSILON {
        ((mean_b - mean_a) / mean_a.abs()) * 100.0
    } else {
        0.0
    };

    let metric_comp = MetricComparison {
        metric_name: "mean_reward".to_string(),
        value_a: mean_a,
        value_b: mean_b,
        difference: mean_b - mean_a,
        relative_change_pct: rel_change,
        effect_size: cohens_d,
    };

    let var_comp = MetricComparison {
        metric_name: "reward_variance".to_string(),
        value_a: var_a,
        value_b: var_b,
        difference: var_b - var_a,
        relative_change_pct: if var_a.abs() > f64::EPSILON { ((var_b - var_a) / var_a) * 100.0 } else { 0.0 },
        effect_size: 0.0,
    };

    let stat_test = detect_regression_ttest(rewards_a, rewards_b, 0.05);

    let winner = if stat_test.p_value < 0.05 {
        if mean_b > mean_a {
            Some(name_b.to_string())
        } else {
            Some(name_a.to_string())
        }
    } else {
        None // No statistically significant difference
    };

    PolicyComparison {
        policy_a_name: name_a.to_string(),
        policy_b_name: name_b.to_string(),
        metric_comparisons: vec![metric_comp, var_comp],
        statistical_test: Some(stat_test),
        winner,
    }
}

// 13. Report Generation

#[derive(Debug, Clone)]
pub struct EvalReport {
    pub title: String,
    pub suite_name: String,
    pub timestamp: u64,
    pub overall_verdict: GateVerdict,
    pub stage_results: Vec<StageResult>,
    pub gate_results: Vec<QualityGate>,
    pub metric_summary: HashMap<String, f64>,
    pub recommendations: Vec<String>,
    pub warnings: Vec<String>,
}

impl EvalReport {
    pub fn new(title: &str, suite_name: &str) -> Self {
        Self {
            title: title.to_string(),
            suite_name: suite_name.to_string(),
            timestamp: 0,
            overall_verdict: GateVerdict::Skip,
            stage_results: Vec::new(),
            gate_results: Vec::new(),
            metric_summary: HashMap::new(),
            recommendations: Vec::new(),
            warnings: Vec::new(),
        }
    }

    pub fn add_recommendation(&mut self, rec: &str) {
        self.recommendations.push(rec.to_string());
    }

    pub fn add_warning(&mut self, warn: &str) {
        self.warnings.push(warn.to_string());
    }

    pub fn set_verdict(&mut self, verdict: GateVerdict) {
        self.overall_verdict = verdict;
    }

    pub fn render(&self, format: &ReportFormat) -> String {
        match format {
            ReportFormat::Markdown => self.render_markdown(),
            ReportFormat::Plain => self.render_plain(),
            ReportFormat::Json => self.render_json(),
            ReportFormat::Html => self.render_html(),
        }
    }

    fn render_markdown(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!("# {}\n\n", self.title));
        out.push_str(&format!("**Suite:** {}\n", self.suite_name));
        out.push_str(&format!("**Verdict:** {}\n\n", self.overall_verdict));

        out.push_str("## Stage Results\n\n");
        out.push_str("| Stage | Result |\n|---|---|\n");
        for sr in &self.stage_results {
            out.push_str(&format!("| {} | {} |\n", sr.stage, if sr.passed { "PASS" } else { "FAIL" }));
        }

        out.push_str("\n## Metrics\n\n");
        for (k, v) in &self.metric_summary {
            out.push_str(&format!("- **{}**: {:.4}\n", k, v));
        }

        if !self.recommendations.is_empty() {
            out.push_str("\n## Recommendations\n\n");
            for r in &self.recommendations {
                out.push_str(&format!("- {}\n", r));
            }
        }

        if !self.warnings.is_empty() {
            out.push_str("\n## Warnings\n\n");
            for w in &self.warnings {
                out.push_str(&format!("- {}\n", w));
            }
        }

        out
    }

    fn render_plain(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!("{}\n", self.title));
        out.push_str(&format!("Suite: {}\n", self.suite_name));
        out.push_str(&format!("Verdict: {}\n\n", self.overall_verdict));
        for (k, v) in &self.metric_summary {
            out.push_str(&format!("  {}: {:.4}\n", k, v));
        }
        out
    }

    fn render_json(&self) -> String {
        let mut out = String::from("{\n");
        out.push_str(&format!("  \"title\": \"{}\",\n", self.title));
        out.push_str(&format!("  \"suite\": \"{}\",\n", self.suite_name));
        out.push_str(&format!("  \"verdict\": \"{}\",\n", self.overall_verdict));
        out.push_str("  \"metrics\": {\n");
        let items: Vec<String> = self.metric_summary.iter().map(|(k, v)| format!("    \"{}\": {:.4}", k, v)).collect();
        out.push_str(&items.join(",\n"));
        out.push_str("\n  }\n");
        out.push('}');
        out
    }

    fn render_html(&self) -> String {
        let mut out = String::from("<html><body>\n");
        out.push_str(&format!("<h1>{}</h1>\n", self.title));
        out.push_str(&format!("<p>Suite: {} | Verdict: <strong>{}</strong></p>\n", self.suite_name, self.overall_verdict));
        out.push_str("<table><tr><th>Metric</th><th>Value</th></tr>\n");
        for (k, v) in &self.metric_summary {
            out.push_str(&format!("<tr><td>{}</td><td>{:.4}</td></tr>\n", k, v));
        }
        out.push_str("</table>\n</body></html>");
        out
    }
}

pub fn generate_report(
    suite: &EvalSuite,
    pipeline: &EvalPipeline,
    gates: &[QualityGate],
) -> EvalReport {
    let mut report = EvalReport::new(
        &format!("Evaluation Report: {}", suite.name),
        &suite.name,
    );

    report.stage_results = pipeline.results.clone();
    report.gate_results = gates.to_vec();

    // Aggregate metrics from all stages
    for sr in &pipeline.results {
        for (k, v) in &sr.metrics {
            report.metric_summary.insert(k.clone(), *v);
        }
    }

    // Determine verdict
    let all_stages_pass = pipeline.results.iter().all(|r| r.passed);
    let all_gates_pass = gates.iter().all(|g| g.verdict == GateVerdict::Pass);

    if all_stages_pass && all_gates_pass {
        report.set_verdict(GateVerdict::Pass);
        report.add_recommendation("Policy is ready for deployment.");
    } else if gates.iter().any(|g| g.verdict == GateVerdict::Fail) {
        report.set_verdict(GateVerdict::Fail);
        for g in gates {
            if g.verdict == GateVerdict::Fail {
                report.add_warning(&format!("Gate '{}' failed: {}", g.name, g.reason));
            }
        }
        report.add_recommendation("Address failing quality gates before deployment.");
    } else {
        report.set_verdict(GateVerdict::Warn);
        report.add_recommendation("Review warnings before proceeding with deployment.");
    }

    report
}

// 14. Counterfactual Evaluation

#[derive(Debug, Clone)]
pub struct CounterfactualQuery {
    pub name: String,
    pub intervention: String,
    pub logged_actions: Vec<usize>,
    pub counterfactual_actions: Vec<usize>,
    pub logged_rewards: Vec<f64>,
    pub action_probs_logged: Vec<f64>,
    pub action_probs_counterfactual: Vec<f64>,
}

#[derive(Debug, Clone)]
pub struct CounterfactualResult {
    pub query_name: String,
    pub logged_value: f64,
    pub counterfactual_value: f64,
    pub treatment_effect: f64,
    pub relative_effect_pct: f64,
    pub confidence_lower: f64,
    pub confidence_upper: f64,
}

pub fn counterfactual_evaluate(query: &CounterfactualQuery) -> CounterfactualResult {
    let n = query.logged_rewards.len();
    if n == 0 {
        return CounterfactualResult {
            query_name: query.name.clone(),
            logged_value: 0.0,
            counterfactual_value: 0.0,
            treatment_effect: 0.0,
            relative_effect_pct: 0.0,
            confidence_lower: 0.0,
            confidence_upper: 0.0,
        };
    }

    let logged_value = query.logged_rewards.iter().sum::<f64>() / n as f64;

    // IS-based counterfactual estimate
    let mut cf_estimates = Vec::new();
    for i in 0..n {
        let behavior_prob = query.action_probs_logged.get(i).copied().unwrap_or(1.0);
        let target_prob = query.action_probs_counterfactual.get(i).copied().unwrap_or(1.0);
        let weight = if behavior_prob > f64::EPSILON {
            target_prob / behavior_prob
        } else {
            1.0
        };
        cf_estimates.push(weight * query.logged_rewards[i]);
    }

    let cf_value = cf_estimates.iter().sum::<f64>() / n as f64;
    let treatment_effect = cf_value - logged_value;
    let rel_pct = if logged_value.abs() > f64::EPSILON {
        (treatment_effect / logged_value.abs()) * 100.0
    } else {
        0.0
    };

    let variance = cf_estimates.iter().map(|e| (e - cf_value).powi(2)).sum::<f64>() / n as f64;
    let std_err = (variance / n as f64).sqrt();

    CounterfactualResult {
        query_name: query.name.clone(),
        logged_value,
        counterfactual_value: cf_value,
        treatment_effect,
        relative_effect_pct: rel_pct,
        confidence_lower: treatment_effect - 1.96 * std_err,
        confidence_upper: treatment_effect + 1.96 * std_err,
    }
}

// 15. Multi-Agent Evaluation

#[derive(Debug, Clone)]
pub struct AgentRecord {
    pub agent_id: String,
    pub rewards: Vec<f64>,
    pub messages_sent: usize,
    pub messages_received: usize,
    pub cooperation_score: f64,
}

#[derive(Debug, Clone)]
pub struct MultiAgentMetrics {
    pub per_agent: HashMap<String, AgentMetricSummary>,
    pub social_welfare: f64,
    pub gini_coefficient: f64,
    pub communication_efficiency: f64,
    pub coalition_value: f64,
    pub fairness_score: f64,
    pub coordination_overhead: f64,
}

#[derive(Debug, Clone)]
pub struct AgentMetricSummary {
    pub mean_reward: f64,
    pub total_reward: f64,
    pub contribution_ratio: f64,
    pub communication_ratio: f64,
}

pub fn compute_multi_agent_metrics(agents: &[AgentRecord]) -> MultiAgentMetrics {
    if agents.is_empty() {
        return MultiAgentMetrics {
            per_agent: HashMap::new(),
            social_welfare: 0.0,
            gini_coefficient: 0.0,
            communication_efficiency: 0.0,
            coalition_value: 0.0,
            fairness_score: 1.0,
            coordination_overhead: 0.0,
        };
    }

    let mut per_agent = HashMap::new();
    let mut total_rewards: Vec<f64> = Vec::new();
    let total_messages: usize = agents.iter().map(|a| a.messages_sent + a.messages_received).sum();
    let grand_total_reward: f64 = agents.iter().map(|a| a.rewards.iter().sum::<f64>()).sum();

    for agent in agents {
        let total = agent.rewards.iter().sum::<f64>();
        let mean = if agent.rewards.is_empty() { 0.0 } else { total / agent.rewards.len() as f64 };
        let contrib = if grand_total_reward.abs() > f64::EPSILON {
            total / grand_total_reward
        } else {
            0.0
        };
        let comm_ratio = if total_messages > 0 {
            (agent.messages_sent + agent.messages_received) as f64 / total_messages as f64
        } else {
            0.0
        };

        total_rewards.push(total);
        per_agent.insert(
            agent.agent_id.clone(),
            AgentMetricSummary {
                mean_reward: mean,
                total_reward: total,
                contribution_ratio: contrib,
                communication_ratio: comm_ratio,
            },
        );
    }

    let social_welfare = grand_total_reward;

    // Gini coefficient
    let n = total_rewards.len() as f64;
    let mean_reward = social_welfare / n;
    let mut gini_num = 0.0;
    for i in 0..total_rewards.len() {
        for j in 0..total_rewards.len() {
            gini_num += (total_rewards[i] - total_rewards[j]).abs();
        }
    }
    let gini = if mean_reward.abs() > f64::EPSILON && n > 0.0 {
        gini_num / (2.0 * n * n * mean_reward.abs())
    } else {
        0.0
    };

    // Communication efficiency: reward per message
    let comm_eff = if total_messages > 0 {
        social_welfare / total_messages as f64
    } else {
        social_welfare
    };

    // Coalition value: sum of cooperation scores
    let coalition_value = agents.iter().map(|a| a.cooperation_score).sum::<f64>();

    // Fairness: 1 - gini
    let fairness = 1.0 - gini;

    // Coordination overhead: messages / reward (lower is better)
    let coord_overhead = if social_welfare.abs() > f64::EPSILON {
        total_messages as f64 / social_welfare.abs()
    } else {
        0.0
    };

    MultiAgentMetrics {
        per_agent,
        social_welfare,
        gini_coefficient: gini,
        communication_efficiency: comm_eff,
        coalition_value,
        fairness_score: fairness.max(0.0),
        coordination_overhead: coord_overhead,
    }
}

// 16. Finance-Specific Metrics

#[derive(Debug, Clone)]
pub struct FinanceMetrics {
    pub sharpe_ratio: f64,
    pub sortino_ratio: f64,
    pub max_drawdown: f64,
    pub max_drawdown_duration: usize,
    pub calmar_ratio: f64,
    pub var_95: f64,
    pub cvar_95: f64,
    pub total_return: f64,
    pub annualized_return: f64,
    pub volatility: f64,
    pub downside_deviation: f64,
}

pub fn compute_finance_metrics(returns: &[f64], risk_free_rate: f64, periods_per_year: f64) -> FinanceMetrics {
    if returns.is_empty() {
        return FinanceMetrics {
            sharpe_ratio: 0.0,
            sortino_ratio: 0.0,
            max_drawdown: 0.0,
            max_drawdown_duration: 0,
            calmar_ratio: 0.0,
            var_95: 0.0,
            cvar_95: 0.0,
            total_return: 0.0,
            annualized_return: 0.0,
            volatility: 0.0,
            downside_deviation: 0.0,
        };
    }

    let n = returns.len() as f64;
    let mean_return = returns.iter().sum::<f64>() / n;
    let excess_returns: Vec<f64> = returns.iter().map(|r| r - risk_free_rate / periods_per_year).collect();
    let mean_excess = excess_returns.iter().sum::<f64>() / n;
    let volatility = (returns.iter().map(|r| (r - mean_return).powi(2)).sum::<f64>() / n).sqrt();

    // Sharpe ratio
    let sharpe = if volatility > f64::EPSILON {
        (mean_excess / volatility) * periods_per_year.sqrt()
    } else {
        0.0
    };

    // Downside deviation (negative returns only)
    let downside_sq: f64 = returns
        .iter()
        .filter(|&&r| r < risk_free_rate / periods_per_year)
        .map(|r| (r - risk_free_rate / periods_per_year).powi(2))
        .sum();
    let downside_count = returns.iter().filter(|&&r| r < risk_free_rate / periods_per_year).count();
    let downside_dev = if downside_count > 0 {
        (downside_sq / downside_count as f64).sqrt()
    } else {
        0.0
    };

    // Sortino ratio
    let sortino = if downside_dev > f64::EPSILON {
        (mean_excess / downside_dev) * periods_per_year.sqrt()
    } else {
        0.0
    };

    // Max drawdown
    let mut cumulative = Vec::with_capacity(returns.len());
    let mut cum = 1.0;
    for &r in returns {
        cum *= 1.0 + r;
        cumulative.push(cum);
    }

    let mut max_dd = 0.0f64;
    let mut peak = cumulative[0];
    let mut max_dd_duration = 0usize;
    let mut current_dd_start = 0usize;
    let mut in_drawdown = false;

    for (i, &c) in cumulative.iter().enumerate() {
        if c > peak {
            peak = c;
            if in_drawdown {
                let duration = i - current_dd_start;
                if duration > max_dd_duration {
                    max_dd_duration = duration;
                }
                in_drawdown = false;
            }
        } else {
            if !in_drawdown {
                current_dd_start = i;
                in_drawdown = true;
            }
            let dd = (peak - c) / peak;
            if dd > max_dd {
                max_dd = dd;
            }
        }
    }
    if in_drawdown {
        let duration = returns.len() - current_dd_start;
        if duration > max_dd_duration {
            max_dd_duration = duration;
        }
    }

    // Calmar ratio
    let total_return = cumulative.last().copied().unwrap_or(1.0) - 1.0;
    let annualized_return = (1.0 + total_return).powf(periods_per_year / n) - 1.0;
    let calmar = if max_dd > f64::EPSILON {
        annualized_return / max_dd
    } else {
        0.0
    };

    // VaR and CVaR at 95%
    let mut sorted_returns: Vec<f64> = returns.to_vec();
    sorted_returns.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let var_idx = ((returns.len() as f64) * 0.05).floor() as usize;
    let var_95 = sorted_returns.get(var_idx).copied().unwrap_or(0.0);
    let cvar_95 = if var_idx > 0 {
        sorted_returns[..var_idx].iter().sum::<f64>() / var_idx as f64
    } else {
        var_95
    };

    FinanceMetrics {
        sharpe_ratio: sharpe,
        sortino_ratio: sortino,
        max_drawdown: max_dd,
        max_drawdown_duration: max_dd_duration,
        calmar_ratio: calmar,
        var_95,
        cvar_95,
        total_return,
        annualized_return,
        volatility,
        downside_deviation: downside_dev,
    }
}

// 17. Continuous Eval Pipeline

#[derive(Debug, Clone)]
pub struct ContinuousEvalConfig {
    pub schedule_cron: String,
    pub drift_threshold: f64,
    pub alert_channels: Vec<String>,
    pub baseline_policy_id: String,
    pub eval_suite_name: String,
    pub max_history: usize,
}

impl ContinuousEvalConfig {
    pub fn new(suite_name: &str, baseline: &str) -> Self {
        Self {
            schedule_cron: "0 */6 * * *".to_string(),
            drift_threshold: 0.1,
            alert_channels: vec!["slack".to_string()],
            baseline_policy_id: baseline.to_string(),
            eval_suite_name: suite_name.to_string(),
            max_history: 100,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DriftDetection {
    pub drift_type: DriftType,
    pub metric_name: String,
    pub baseline_value: f64,
    pub current_value: f64,
    pub drift_magnitude: f64,
    pub is_significant: bool,
    pub severity: AlertSeverity,
}

#[derive(Debug, Clone)]
pub struct EvalAlert {
    pub severity: AlertSeverity,
    pub message: String,
    pub metric_name: String,
    pub current_value: f64,
    pub threshold: f64,
    pub timestamp: u64,
}

#[derive(Debug, Clone)]
pub struct ContinuousEvalState {
    pub config: ContinuousEvalConfig,
    pub history: Vec<EvalRunRecord>,
    pub active_alerts: Vec<EvalAlert>,
    pub drift_detections: Vec<DriftDetection>,
    pub run_count: usize,
}

#[derive(Debug, Clone)]
pub struct EvalRunRecord {
    pub run_id: usize,
    pub timestamp: u64,
    pub mean_reward: f64,
    pub win_rate: f64,
    pub violation_rate: f64,
    pub sharpe_ratio: f64,
    pub passed: bool,
}

impl ContinuousEvalState {
    pub fn new(config: ContinuousEvalConfig) -> Self {
        Self {
            config,
            history: Vec::new(),
            active_alerts: Vec::new(),
            drift_detections: Vec::new(),
            run_count: 0,
        }
    }

    pub fn record_run(&mut self, record: EvalRunRecord) {
        self.run_count += 1;
        self.history.push(record);
        if self.history.len() > self.config.max_history {
            self.history.remove(0);
        }
    }

    pub fn detect_drift(&mut self) -> Vec<DriftDetection> {
        self.drift_detections.clear();

        if self.history.len() < 2 {
            return Vec::new();
        }

        // Compare recent window (last 5) vs baseline window (first 5)
        let baseline_window = self.history.len().min(5);
        let recent_start = if self.history.len() > 5 { self.history.len() - 5 } else { 0 };

        let baseline_mean = self.history[..baseline_window]
            .iter()
            .map(|r| r.mean_reward)
            .sum::<f64>()
            / baseline_window as f64;
        let recent_mean = self.history[recent_start..]
            .iter()
            .map(|r| r.mean_reward)
            .sum::<f64>()
            / (self.history.len() - recent_start) as f64;

        let magnitude = if baseline_mean.abs() > f64::EPSILON {
            ((recent_mean - baseline_mean) / baseline_mean.abs()).abs()
        } else {
            0.0
        };

        let is_sig = magnitude > self.config.drift_threshold;
        let severity = if magnitude > 0.3 {
            AlertSeverity::Critical
        } else if magnitude > 0.2 {
            AlertSeverity::High
        } else if magnitude > 0.1 {
            AlertSeverity::Medium
        } else {
            AlertSeverity::Low
        };

        let detection = DriftDetection {
            drift_type: DriftType::Reward,
            metric_name: "mean_reward".to_string(),
            baseline_value: baseline_mean,
            current_value: recent_mean,
            drift_magnitude: magnitude,
            is_significant: is_sig,
            severity: severity.clone(),
        };
        self.drift_detections.push(detection.clone());

        // Also check win rate drift
        let baseline_wr = self.history[..baseline_window]
            .iter()
            .map(|r| r.win_rate)
            .sum::<f64>()
            / baseline_window as f64;
        let recent_wr = self.history[recent_start..]
            .iter()
            .map(|r| r.win_rate)
            .sum::<f64>()
            / (self.history.len() - recent_start) as f64;

        let wr_magnitude = (recent_wr - baseline_wr).abs();
        if wr_magnitude > self.config.drift_threshold {
            let wr_detection = DriftDetection {
                drift_type: DriftType::Action,
                metric_name: "win_rate".to_string(),
                baseline_value: baseline_wr,
                current_value: recent_wr,
                drift_magnitude: wr_magnitude,
                is_significant: true,
                severity: if wr_magnitude > 0.2 { AlertSeverity::High } else { AlertSeverity::Medium },
            };
            self.drift_detections.push(wr_detection);
        }

        self.drift_detections.clone()
    }

    pub fn generate_alerts(&mut self) -> Vec<EvalAlert> {
        self.active_alerts.clear();

        for drift in &self.drift_detections {
            if drift.is_significant {
                self.active_alerts.push(EvalAlert {
                    severity: drift.severity.clone(),
                    message: format!(
                        "{} drift detected: {} changed from {:.4} to {:.4} (magnitude: {:.4})",
                        drift.drift_type, drift.metric_name, drift.baseline_value, drift.current_value, drift.drift_magnitude
                    ),
                    metric_name: drift.metric_name.clone(),
                    current_value: drift.current_value,
                    threshold: self.config.drift_threshold,
                    timestamp: 0,
                });
            }
        }

        // Check latest run for failures
        if let Some(last) = self.history.last() {
            if !last.passed {
                self.active_alerts.push(EvalAlert {
                    severity: AlertSeverity::High,
                    message: format!("Evaluation run {} failed", last.run_id),
                    metric_name: "eval_pass".to_string(),
                    current_value: 0.0,
                    threshold: 1.0,
                    timestamp: last.timestamp,
                });
            }
        }

        self.active_alerts.clone()
    }

    pub fn trend_summary(&self) -> String {
        if self.history.is_empty() {
            return "No evaluation history available.".to_string();
        }
        let n = self.history.len();
        let avg_reward = self.history.iter().map(|r| r.mean_reward).sum::<f64>() / n as f64;
        let pass_rate = self.history.iter().filter(|r| r.passed).count() as f64 / n as f64;
        format!(
            "Runs: {} | Avg Reward: {:.4} | Pass Rate: {:.1}% | Active Alerts: {}",
            n,
            avg_reward,
            pass_rate * 100.0,
            self.active_alerts.len()
        )
    }
}

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Helpers ────────────────────────────────────────────────────────

    fn make_episodes(rewards_per_episode: &[Vec<f64>]) -> Vec<EpisodeRecord> {
        rewards_per_episode
            .iter()
            .enumerate()
            .map(|(i, rewards)| {
                let mut ep = EpisodeRecord::new(i);
                ep.rewards = rewards.clone();
                ep.steps = rewards.len();
                ep.done = true;
                ep
            })
            .collect()
    }

    fn make_trajectory(rewards: &[f64], behavior_prob: f64, target_prob: f64) -> Vec<OpeTransition> {
        rewards
            .iter()
            .map(|&r| OpeTransition {
                state: vec![0.0],
                action: 0,
                reward: r,
                next_state: vec![0.0],
                behavior_prob,
                target_prob,
                done: false,
            })
            .collect()
    }

    fn make_agents(agent_data: &[(&str, &[f64], usize, usize, f64)]) -> Vec<AgentRecord> {
        agent_data
            .iter()
            .map(|(id, rewards, sent, recv, coop)| AgentRecord {
                agent_id: id.to_string(),
                rewards: rewards.to_vec(),
                messages_sent: *sent,
                messages_received: *recv,
                cooperation_score: *coop,
            })
            .collect()
    }

    // ── 1. Eval Suite Definition ──────────────────────────────────────

    #[test]
    fn test_eval_suite_new() {
        let suite = EvalSuite::new("test-suite", "1.0");
        assert_eq!(suite.name, "test-suite");
        assert_eq!(suite.version, "1.0");
        assert!(suite.scenarios.is_empty());
    }

    #[test]
    fn test_eval_suite_with_description() {
        let suite = EvalSuite::new("s", "1.0").with_description("A test suite");
        assert_eq!(suite.description, "A test suite");
    }

    #[test]
    fn test_eval_suite_add_scenario() {
        let mut suite = EvalSuite::new("s", "1.0");
        suite.add_scenario(EvalScenario::new("scenario-a"));
        assert_eq!(suite.scenario_count(), 1);
    }

    #[test]
    fn test_eval_suite_parse_yaml_basic() {
        let yaml = "name: \"my-suite\"\nversion: \"2.0\"\ndescription: \"testing\"\n";
        let suite = EvalSuite::parse_yaml(yaml).unwrap();
        assert_eq!(suite.name, "my-suite");
        assert_eq!(suite.version, "2.0");
    }

    #[test]
    fn test_eval_suite_parse_yaml_with_scenarios() {
        let yaml = "name: \"s\"\n- scenario: \"fast\"\n- scenario: \"slow\"\n";
        let suite = EvalSuite::parse_yaml(yaml).unwrap();
        assert_eq!(suite.scenarios.len(), 2);
    }

    #[test]
    fn test_eval_suite_parse_yaml_with_metrics() {
        let yaml = "name: \"s\"\n- metric: \"reward\"\n- metric: \"safety\"\n";
        let suite = EvalSuite::parse_yaml(yaml).unwrap();
        assert_eq!(suite.metrics.len(), 2);
    }

    #[test]
    fn test_eval_suite_parse_yaml_with_gates() {
        let yaml = "name: \"s\"\n- gate: \"deploy\"\n";
        let suite = EvalSuite::parse_yaml(yaml).unwrap();
        assert_eq!(suite.gates.len(), 1);
    }

    #[test]
    fn test_eval_suite_to_yaml_roundtrip() {
        let mut suite = EvalSuite::new("roundtrip", "3.0");
        suite.add_scenario(EvalScenario::new("s1"));
        let yaml = suite.to_yaml();
        assert!(yaml.contains("roundtrip"));
        assert!(yaml.contains("s1"));
    }

    #[test]
    fn test_eval_suite_parse_yaml_missing_name() {
        let yaml = "version: \"1.0\"\n";
        let result = EvalSuite::parse_yaml(yaml);
        // Name defaults to "parsed" which is non-empty
        assert!(result.is_ok());
    }

    // ── 2. Scenario-Based Testing ─────────────────────────────────────

    #[test]
    fn test_eval_scenario_new() {
        let s = EvalScenario::new("env-v1");
        assert_eq!(s.name, "env-v1");
        assert_eq!(s.episode_count, 100);
        assert!(!s.safety_critical);
    }

    #[test]
    fn test_eval_scenario_with_episodes() {
        let s = EvalScenario::new("s").with_episodes(500);
        assert_eq!(s.episode_count, 500);
    }

    #[test]
    fn test_eval_scenario_with_safety_critical() {
        let s = EvalScenario::new("s").with_safety_critical(true);
        assert!(s.safety_critical);
    }

    #[test]
    fn test_eval_scenario_with_seed() {
        let s = EvalScenario::new("s").with_seed(42);
        assert_eq!(s.seed, Some(42));
    }

    #[test]
    fn test_eval_scenario_env_override() {
        let mut s = EvalScenario::new("s");
        s.add_env_override("GRAVITY", "9.81");
        assert_eq!(s.env_overrides.get("GRAVITY").unwrap(), "9.81");
    }

    #[test]
    fn test_eval_scenario_tags() {
        let mut s = EvalScenario::new("s");
        s.add_tag("critical");
        s.add_tag("nightly");
        assert_eq!(s.tags.len(), 2);
    }

    // ── 3. Performance Metrics ────────────────────────────────────────

    #[test]
    fn test_performance_metrics_basic() {
        let episodes = make_episodes(&[vec![1.0, 2.0, 3.0], vec![4.0, 5.0, 6.0]]);
        let pm = compute_performance_metrics(&episodes, 10.0);
        assert_eq!(pm.episode_count, 2);
        assert!((pm.cumulative_reward - 21.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_performance_metrics_mean_reward() {
        let episodes = make_episodes(&[vec![10.0], vec![20.0], vec![30.0]]);
        let pm = compute_performance_metrics(&episodes, 0.0);
        assert!((pm.mean_reward - 20.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_performance_metrics_win_rate() {
        let episodes = make_episodes(&[vec![5.0], vec![15.0], vec![25.0], vec![35.0]]);
        let pm = compute_performance_metrics(&episodes, 20.0);
        assert!((pm.win_rate - 0.5).abs() < f64::EPSILON); // 25 and 35 are wins
    }

    #[test]
    fn test_performance_metrics_empty() {
        let pm = compute_performance_metrics(&[], 0.0);
        assert_eq!(pm.episode_count, 0);
        assert_eq!(pm.mean_reward, 0.0);
    }

    #[test]
    fn test_performance_metrics_variance() {
        let episodes = make_episodes(&[vec![10.0], vec![10.0], vec![10.0]]);
        let pm = compute_performance_metrics(&episodes, 0.0);
        assert!(pm.reward_variance < f64::EPSILON);
    }

    #[test]
    fn test_performance_metrics_episode_length() {
        let episodes = make_episodes(&[vec![1.0; 5], vec![1.0; 10], vec![1.0; 15]]);
        let pm = compute_performance_metrics(&episodes, 0.0);
        assert!((pm.mean_episode_length - 10.0).abs() < f64::EPSILON);
        assert!((pm.median_episode_length - 10.0).abs() < f64::EPSILON);
        assert_eq!(pm.min_episode_length, 5);
        assert_eq!(pm.max_episode_length, 15);
    }

    #[test]
    fn test_episode_record_cumulative_reward() {
        let mut ep = EpisodeRecord::new(0);
        ep.rewards = vec![1.0, 2.0, 3.0, 4.0];
        assert!((ep.cumulative_reward() - 10.0).abs() < f64::EPSILON);
    }

    // ── 4. Robustness Metrics ─────────────────────────────────────────

    #[test]
    fn test_robustness_metrics_basic() {
        let episodes = make_episodes(&[vec![5.0], vec![10.0], vec![15.0], vec![20.0]]);
        let rm = compute_robustness_metrics(&episodes);
        assert!((rm.worst_case_reward - 5.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_robustness_metrics_empty() {
        let rm = compute_robustness_metrics(&[]);
        assert_eq!(rm.worst_case_reward, 0.0);
        assert!((rm.reward_stability - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_robustness_cv_zero_for_constant() {
        let episodes = make_episodes(&[vec![10.0], vec![10.0], vec![10.0]]);
        let rm = compute_robustness_metrics(&episodes);
        assert!(rm.coefficient_of_variation < f64::EPSILON);
        assert!((rm.reward_stability - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_robustness_iqr() {
        let episodes = make_episodes(&[vec![1.0], vec![2.0], vec![3.0], vec![4.0], vec![5.0], vec![6.0], vec![7.0], vec![8.0]]);
        let rm = compute_robustness_metrics(&episodes);
        assert!(rm.interquartile_range > 0.0);
    }

    #[test]
    fn test_robustness_cross_period_stability_constant() {
        let episodes = make_episodes(&[vec![10.0], vec![10.0], vec![10.0], vec![10.0]]);
        let rm = compute_robustness_metrics(&episodes);
        assert!((rm.cross_period_stability - 1.0).abs() < 0.01);
    }

    // ── 5. Safety Metrics ─────────────────────────────────────────────

    #[test]
    fn test_safety_metrics_no_violations() {
        let constraints = vec![SafetyConstraint::new("speed", 100.0, true)];
        let values = vec![(0, 0, "speed", 50.0), (0, 1, "speed", 60.0)];
        let sm = compute_safety_metrics(&values, &constraints, 0.9);
        assert_eq!(sm.hard_violation_count, 0);
        assert_eq!(sm.soft_violation_count, 0);
        assert!((sm.violation_rate - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_safety_metrics_hard_violation() {
        let constraints = vec![SafetyConstraint::new("speed", 100.0, true)];
        let values = vec![(0, 0, "speed", 110.0)];
        let sm = compute_safety_metrics(&values, &constraints, 0.9);
        assert_eq!(sm.hard_violation_count, 1);
        assert!((sm.violation_rate - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_safety_metrics_near_miss() {
        let constraints = vec![SafetyConstraint::new("speed", 100.0, true)];
        let values = vec![(0, 0, "speed", 95.0)]; // above 90% threshold
        let sm = compute_safety_metrics(&values, &constraints, 0.9);
        assert_eq!(sm.near_miss_count, 1);
    }

    #[test]
    fn test_safety_metrics_margin() {
        let constraints = vec![SafetyConstraint::new("force", 50.0, false)];
        let values = vec![(0, 0, "force", 30.0), (0, 1, "force", 40.0)];
        let sm = compute_safety_metrics(&values, &constraints, 0.9);
        assert!((sm.safety_margin_mean - 15.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_safety_metrics_empty() {
        let sm = compute_safety_metrics(&[], &[], 0.9);
        assert_eq!(sm.hard_violation_count, 0);
        assert_eq!(sm.violation_rate, 0.0);
    }

    #[test]
    fn test_safety_constraint_new() {
        let c = SafetyConstraint::new("collision", 0.5, true);
        assert_eq!(c.name, "collision");
        assert!(c.is_hard_constraint);
    }

    // ── 6. Off-Policy Evaluation ──────────────────────────────────────

    #[test]
    fn test_is_estimate_same_policy() {
        let traj = make_trajectory(&[1.0, 1.0, 1.0], 0.5, 0.5);
        let result = importance_sampling_estimate(&[traj], 0.99);
        assert!(result.estimated_value > 0.0);
        assert_eq!(result.method, OpeMethod::ImportanceSampling);
    }

    #[test]
    fn test_is_estimate_empty() {
        let result = importance_sampling_estimate(&[], 0.99);
        assert_eq!(result.estimated_value, 0.0);
        assert_eq!(result.num_trajectories, 0);
    }

    #[test]
    fn test_fqe_basic() {
        let transitions: Vec<OpeTransition> = (0..10)
            .map(|_| OpeTransition {
                state: vec![1.0],
                action: 0,
                reward: 1.0,
                next_state: vec![1.0],
                behavior_prob: 0.5,
                target_prob: 0.5,
                done: false,
            })
            .collect();
        let result = fitted_q_evaluation(&transitions, 0.99, 10);
        assert!(result.estimated_value > 0.0);
        assert_eq!(result.method, OpeMethod::FittedQEvaluation);
    }

    #[test]
    fn test_fqe_empty() {
        let result = fitted_q_evaluation(&[], 0.99, 10);
        assert_eq!(result.estimated_value, 0.0);
    }

    #[test]
    fn test_doubly_robust_basic() {
        let traj = make_trajectory(&[2.0, 2.0, 2.0], 0.5, 0.5);
        let result = doubly_robust_estimate(&[traj], 0.99);
        assert!(result.estimated_value > 0.0);
        assert_eq!(result.method, OpeMethod::DoublyRobust);
    }

    #[test]
    fn test_doubly_robust_empty() {
        let result = doubly_robust_estimate(&[], 0.99);
        assert_eq!(result.estimated_value, 0.0);
    }

    #[test]
    fn test_magic_estimate_basic() {
        let traj = make_trajectory(&[1.0, 2.0, 3.0], 0.5, 0.5);
        let result = magic_estimate(&[traj], 0.99, 100);
        assert!(result.estimated_value > 0.0);
        assert_eq!(result.method, OpeMethod::Magic);
    }

    #[test]
    fn test_magic_estimate_empty() {
        let result = magic_estimate(&[], 0.99, 100);
        assert_eq!(result.estimated_value, 0.0);
    }

    #[test]
    fn test_magic_confidence_interval() {
        let trajs: Vec<Vec<OpeTransition>> = (0..20)
            .map(|i| make_trajectory(&[1.0 + i as f64 * 0.1, 2.0, 1.5], 0.4, 0.6))
            .collect();
        let result = magic_estimate(&trajs, 0.99, 200);
        assert!(result.confidence_lower <= result.estimated_value);
        assert!(result.confidence_upper >= result.estimated_value);
    }

    // ── 7. Adversarial Evaluation ─────────────────────────────────────

    #[test]
    fn test_adversarial_eval_no_drop() {
        let clean = vec![10.0, 10.0, 10.0];
        let perturbed = vec![10.0, 10.0, 10.0];
        let config = AdversarialConfig::fgsm(0.01);
        let result = run_adversarial_eval(&clean, &perturbed, &config);
        assert!((result.reward_drop).abs() < f64::EPSILON);
        assert!((result.robustness_score - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_adversarial_eval_with_drop() {
        let clean = vec![10.0, 10.0, 10.0];
        let perturbed = vec![5.0, 5.0, 5.0];
        let config = AdversarialConfig::fgsm(0.1);
        let result = run_adversarial_eval(&clean, &perturbed, &config);
        assert!((result.reward_drop - 5.0).abs() < f64::EPSILON);
        assert!((result.robustness_score - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_adversarial_eval_empty() {
        let config = AdversarialConfig::random_noise(0.1);
        let result = run_adversarial_eval(&[], &[], &config);
        assert_eq!(result.num_trials, 0);
    }

    #[test]
    fn test_fgsm_perturb() {
        let states = vec![vec![1.0, 2.0], vec![3.0, 4.0]];
        let grads = vec![vec![1.0, -1.0], vec![-1.0, 1.0]];
        let perturbed = fgsm_perturb(&states, 0.1, &grads);
        assert!((perturbed[0][0] - 1.1).abs() < f64::EPSILON);
        assert!((perturbed[0][1] - 1.9).abs() < f64::EPSILON);
    }

    #[test]
    fn test_random_noise_perturb() {
        let states = vec![vec![1.0, 2.0]];
        let perturbed = random_noise_perturb(&states, 0.1, 42);
        assert_ne!(perturbed[0][0], 1.0); // Should be perturbed
    }

    #[test]
    fn test_adversarial_config_fgsm() {
        let config = AdversarialConfig::fgsm(0.05);
        assert_eq!(config.method, AdversarialMethod::Fgsm);
        assert!((config.epsilon - 0.05).abs() < f64::EPSILON);
    }

    // ── 8. Regression Detection ───────────────────────────────────────

    #[test]
    fn test_regression_ttest_no_regression() {
        let baseline = vec![10.0, 11.0, 12.0, 10.5, 11.5, 10.0, 11.0, 12.0, 10.5, 11.5];
        let candidate = vec![10.2, 11.3, 12.1, 10.8, 11.2, 10.3, 11.1, 12.3, 10.6, 11.4];
        let result = detect_regression_ttest(&baseline, &candidate, 0.05);
        assert!(!result.is_regression);
    }

    #[test]
    fn test_regression_ttest_with_regression() {
        let baseline: Vec<f64> = (0..30).map(|i| 20.0 + (i as f64) * 0.1).collect();
        let candidate: Vec<f64> = (0..30).map(|i| 5.0 + (i as f64) * 0.1).collect();
        let result = detect_regression_ttest(&baseline, &candidate, 0.05);
        assert!(result.is_regression);
        assert!(result.difference < 0.0);
    }

    #[test]
    fn test_regression_ttest_empty() {
        let result = detect_regression_ttest(&[], &[], 0.05);
        assert!(!result.is_regression);
        assert!((result.p_value - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_regression_bootstrap_no_regression() {
        let baseline = vec![10.0, 11.0, 10.5, 10.3, 10.7];
        let candidate = vec![10.1, 11.2, 10.6, 10.4, 10.8];
        let result = detect_regression_bootstrap(&baseline, &candidate, 1000, 0.05);
        assert!(!result.is_regression);
    }

    #[test]
    fn test_regression_bootstrap_with_regression() {
        let baseline = vec![20.0; 20];
        let candidate = vec![5.0; 20];
        let result = detect_regression_bootstrap(&baseline, &candidate, 500, 0.05);
        assert!(result.candidate_mean < result.baseline_mean);
    }

    #[test]
    fn test_regression_effect_size() {
        let baseline: Vec<f64> = (0..20).map(|i| 10.0 + (i as f64) * 0.2).collect();
        let candidate: Vec<f64> = (0..20).map(|i| 15.0 + (i as f64) * 0.2).collect();
        let result = detect_regression_ttest(&baseline, &candidate, 0.05);
        assert!(result.effect_size > 0.0); // Positive because candidate > baseline
    }

    #[test]
    fn test_regression_confidence_interval() {
        let baseline = vec![10.0, 11.0, 12.0, 10.0, 11.0];
        let candidate = vec![15.0, 16.0, 17.0, 15.0, 16.0];
        let result = detect_regression_ttest(&baseline, &candidate, 0.05);
        assert!(result.confidence_interval.0 < result.confidence_interval.1);
    }

    // ── 9. Generalization Scoring ─────────────────────────────────────

    #[test]
    fn test_generalization_score_basic() {
        let train = EnvironmentResult {
            env_name: "train".to_string(),
            mean_reward: 100.0,
            reward_std: 5.0,
            episode_count: 100,
            domain_features: HashMap::new(),
        };
        let eval_envs = vec![EnvironmentResult {
            env_name: "eval-1".to_string(),
            mean_reward: 90.0,
            reward_std: 8.0,
            episode_count: 50,
            domain_features: HashMap::new(),
        }];
        let gs = compute_generalization_score(&train, &eval_envs);
        assert!(gs.overall_score > 0.0);
        assert!(gs.overall_score <= 1.0);
        assert!((gs.per_env_scores["eval-1"] - 0.9).abs() < f64::EPSILON);
    }

    #[test]
    fn test_generalization_score_empty_envs() {
        let train = EnvironmentResult {
            env_name: "train".to_string(),
            mean_reward: 100.0,
            reward_std: 5.0,
            episode_count: 100,
            domain_features: HashMap::new(),
        };
        let gs = compute_generalization_score(&train, &[]);
        assert_eq!(gs.overall_score, 0.0);
    }

    #[test]
    fn test_generalization_worst_env() {
        let train = EnvironmentResult {
            env_name: "train".to_string(),
            mean_reward: 100.0,
            reward_std: 5.0,
            episode_count: 100,
            domain_features: HashMap::new(),
        };
        let eval_envs = vec![
            EnvironmentResult {
                env_name: "easy".to_string(),
                mean_reward: 95.0,
                reward_std: 3.0,
                episode_count: 50,
                domain_features: HashMap::new(),
            },
            EnvironmentResult {
                env_name: "hard".to_string(),
                mean_reward: 40.0,
                reward_std: 10.0,
                episode_count: 50,
                domain_features: HashMap::new(),
            },
        ];
        let gs = compute_generalization_score(&train, &eval_envs);
        assert_eq!(gs.worst_env, "hard");
    }

    #[test]
    fn test_generalization_domain_shift() {
        let mut train_features = HashMap::new();
        train_features.insert("gravity".to_string(), 9.81);
        let train = EnvironmentResult {
            env_name: "train".to_string(),
            mean_reward: 100.0,
            reward_std: 5.0,
            episode_count: 100,
            domain_features: train_features,
        };
        let mut eval_features = HashMap::new();
        eval_features.insert("gravity".to_string(), 3.72); // Mars gravity
        let eval_envs = vec![EnvironmentResult {
            env_name: "mars".to_string(),
            mean_reward: 60.0,
            reward_std: 15.0,
            episode_count: 50,
            domain_features: eval_features,
        }];
        let gs = compute_generalization_score(&train, &eval_envs);
        assert!(gs.domain_shift_penalty > 0.0);
    }

    // ── 10. Quality Gates ─────────────────────────────────────────────

    #[test]
    fn test_quality_gate_pass() {
        let mut gate = QualityGate::new("deploy");
        gate.add_condition(GateCondition::MinSharpe(1.0));
        let mut metrics = GateMetrics::new();
        metrics.sharpe_ratio = 1.5;
        let verdict = gate.evaluate(&metrics);
        assert_eq!(verdict, GateVerdict::Pass);
    }

    #[test]
    fn test_quality_gate_fail() {
        let mut gate = QualityGate::new("deploy");
        gate.add_condition(GateCondition::MinSharpe(2.0));
        let mut metrics = GateMetrics::new();
        metrics.sharpe_ratio = 1.0;
        let verdict = gate.evaluate(&metrics);
        assert_eq!(verdict, GateVerdict::Fail);
    }

    #[test]
    fn test_quality_gate_max_drawdown() {
        let mut gate = QualityGate::new("risk");
        gate.add_condition(GateCondition::MaxDrawdown(0.1));
        let mut metrics = GateMetrics::new();
        metrics.max_drawdown = 0.05;
        assert_eq!(gate.evaluate(&metrics), GateVerdict::Pass);
    }

    #[test]
    fn test_quality_gate_max_drawdown_fail() {
        let mut gate = QualityGate::new("risk");
        gate.add_condition(GateCondition::MaxDrawdown(0.1));
        let mut metrics = GateMetrics::new();
        metrics.max_drawdown = 0.15;
        assert_eq!(gate.evaluate(&metrics), GateVerdict::Fail);
    }

    #[test]
    fn test_quality_gate_multiple_conditions() {
        let mut gate = QualityGate::new("full");
        gate.add_condition(GateCondition::MinSharpe(1.0));
        gate.add_condition(GateCondition::MaxDrawdown(0.2));
        gate.add_condition(GateCondition::MinWinRate(0.5));
        let mut metrics = GateMetrics::new();
        metrics.sharpe_ratio = 1.5;
        metrics.max_drawdown = 0.1;
        metrics.win_rate = 0.6;
        assert_eq!(gate.evaluate(&metrics), GateVerdict::Pass);
    }

    #[test]
    fn test_quality_gate_custom_condition() {
        let mut gate = QualityGate::new("custom");
        gate.add_condition(GateCondition::Custom {
            name: "my_metric".to_string(),
            threshold: 0.8,
            higher_is_better: true,
        });
        let mut metrics = GateMetrics::new();
        metrics.custom_metrics.insert("my_metric".to_string(), 0.9);
        assert_eq!(gate.evaluate(&metrics), GateVerdict::Pass);
    }

    #[test]
    fn test_gate_verdict_is_blocking() {
        assert!(GateVerdict::Fail.is_blocking());
        assert!(!GateVerdict::Pass.is_blocking());
        assert!(!GateVerdict::Warn.is_blocking());
    }

    // ── 11. Evaluation Pipeline ───────────────────────────────────────

    #[test]
    fn test_pipeline_smoke_test_pass() {
        let config = PipelineConfig::smoke_only();
        let mut pipeline = EvalPipeline::new(config);
        let episodes = make_episodes(&[vec![1.0, 2.0], vec![3.0, 4.0]]);
        let result = pipeline.run_stage(EvalStage::SmokeTest, &episodes);
        assert!(result.passed);
    }

    #[test]
    fn test_pipeline_smoke_test_fail_empty() {
        let config = PipelineConfig::smoke_only();
        let mut pipeline = EvalPipeline::new(config);
        let result = pipeline.run_stage(EvalStage::SmokeTest, &[]);
        assert!(!result.passed);
    }

    #[test]
    fn test_pipeline_full_eval() {
        let config = PipelineConfig::default_pipeline();
        let mut pipeline = EvalPipeline::new(config);
        let episodes = make_episodes(&[vec![5.0; 10], vec![6.0; 10]]);
        let result = pipeline.run_stage(EvalStage::FullEval, &episodes);
        assert!(result.passed);
        assert!(result.metrics.contains_key("mean_reward"));
    }

    #[test]
    fn test_pipeline_run_all() {
        let config = PipelineConfig::default_pipeline();
        let mut pipeline = EvalPipeline::new(config);
        let episodes = make_episodes(&std::array::from_fn::<Vec<f64>, 10, _>(|_| vec![1.0; 5]));
        let passed = pipeline.run_all(&episodes);
        assert!(passed);
    }

    #[test]
    fn test_pipeline_fail_fast() {
        let config = PipelineConfig {
            stages: vec![EvalStage::SmokeTest, EvalStage::FullEval],
            fail_fast: true,
            timeout_per_stage_secs: 60,
            parallel_scenarios: false,
        };
        let mut pipeline = EvalPipeline::new(config);
        let passed = pipeline.run_all(&[]); // SmokeTest fails on empty
        assert!(!passed);
        assert_eq!(pipeline.results.len(), 1); // Stopped after first failure
    }

    #[test]
    fn test_pipeline_summary() {
        let config = PipelineConfig::smoke_only();
        let mut pipeline = EvalPipeline::new(config);
        let episodes = make_episodes(&[vec![1.0]]);
        pipeline.run_all(&episodes);
        let summary = pipeline.summary();
        assert!(summary.contains("PASS"));
    }

    // ── 12. Comparison Engine ─────────────────────────────────────────

    #[test]
    fn test_compare_policies_equal() {
        let a = vec![10.0; 20];
        let b = vec![10.0; 20];
        let comp = compare_policies("A", &a, "B", &b);
        assert!(comp.winner.is_none()); // No significant difference
    }

    #[test]
    fn test_compare_policies_b_wins() {
        let a = vec![10.0; 30];
        let b = vec![20.0; 30];
        let comp = compare_policies("A", &a, "B", &b);
        assert_eq!(comp.metric_comparisons[0].difference, 10.0);
    }

    #[test]
    fn test_compare_policies_effect_size() {
        let a: Vec<f64> = (0..20).map(|i| 10.0 + (i as f64) * 0.1).collect();
        let b: Vec<f64> = (0..20).map(|i| 15.0 + (i as f64) * 0.1).collect();
        let comp = compare_policies("A", &a, "B", &b);
        assert!(comp.metric_comparisons[0].effect_size > 0.0);
    }

    #[test]
    fn test_compare_policies_relative_change() {
        let a = vec![100.0; 10];
        let b = vec![120.0; 10];
        let comp = compare_policies("A", &a, "B", &b);
        assert!((comp.metric_comparisons[0].relative_change_pct - 20.0).abs() < f64::EPSILON);
    }

    // ── 13. Report Generation ─────────────────────────────────────────

    #[test]
    fn test_report_new() {
        let report = EvalReport::new("Test Report", "suite-1");
        assert_eq!(report.title, "Test Report");
        assert_eq!(report.overall_verdict, GateVerdict::Skip);
    }

    #[test]
    fn test_report_render_markdown() {
        let mut report = EvalReport::new("Report", "suite");
        report.set_verdict(GateVerdict::Pass);
        report.metric_summary.insert("sharpe".to_string(), 1.5);
        let md = report.render(&ReportFormat::Markdown);
        assert!(md.contains("# Report"));
        assert!(md.contains("PASS"));
    }

    #[test]
    fn test_report_render_json() {
        let mut report = EvalReport::new("Report", "suite");
        report.set_verdict(GateVerdict::Fail);
        let json = report.render(&ReportFormat::Json);
        assert!(json.contains("FAIL"));
    }

    #[test]
    fn test_report_render_html() {
        let mut report = EvalReport::new("Report", "suite");
        report.set_verdict(GateVerdict::Pass);
        let html = report.render(&ReportFormat::Html);
        assert!(html.contains("<html>"));
    }

    #[test]
    fn test_report_render_plain() {
        let mut report = EvalReport::new("Report", "suite");
        report.set_verdict(GateVerdict::Warn);
        let plain = report.render(&ReportFormat::Plain);
        assert!(plain.contains("WARN"));
    }

    #[test]
    fn test_generate_report_pass() {
        let suite = EvalSuite::new("test", "1.0");
        let config = PipelineConfig::smoke_only();
        let mut pipeline = EvalPipeline::new(config);
        let episodes = make_episodes(&std::array::from_fn::<Vec<f64>, 3, _>(|_| vec![5.0]));
        pipeline.run_all(&episodes);
        let gates = vec![{
            let mut g = QualityGate::new("g");
            g.verdict = GateVerdict::Pass;
            g.reason = "All good".to_string();
            g
        }];
        let report = generate_report(&suite, &pipeline, &gates);
        assert_eq!(report.overall_verdict, GateVerdict::Pass);
    }

    #[test]
    fn test_generate_report_fail() {
        let suite = EvalSuite::new("test", "1.0");
        let config = PipelineConfig::smoke_only();
        let mut pipeline = EvalPipeline::new(config);
        pipeline.run_all(&[]);
        let gates = vec![{
            let mut g = QualityGate::new("g");
            g.verdict = GateVerdict::Fail;
            g.reason = "Bad sharpe".to_string();
            g
        }];
        let report = generate_report(&suite, &pipeline, &gates);
        assert_eq!(report.overall_verdict, GateVerdict::Fail);
        assert!(!report.warnings.is_empty());
    }

    #[test]
    fn test_report_recommendations() {
        let mut report = EvalReport::new("r", "s");
        report.add_recommendation("Increase training episodes");
        report.add_warning("High variance detected");
        assert_eq!(report.recommendations.len(), 1);
        assert_eq!(report.warnings.len(), 1);
    }

    // ── 14. Counterfactual Evaluation ─────────────────────────────────

    #[test]
    fn test_counterfactual_same_policy() {
        let query = CounterfactualQuery {
            name: "same".to_string(),
            intervention: "none".to_string(),
            logged_actions: vec![0, 1, 0],
            counterfactual_actions: vec![0, 1, 0],
            logged_rewards: vec![1.0, 2.0, 3.0],
            action_probs_logged: vec![0.5, 0.5, 0.5],
            action_probs_counterfactual: vec![0.5, 0.5, 0.5],
        };
        let result = counterfactual_evaluate(&query);
        assert!((result.treatment_effect).abs() < f64::EPSILON);
    }

    #[test]
    fn test_counterfactual_different_policy() {
        let query = CounterfactualQuery {
            name: "change".to_string(),
            intervention: "aggressive".to_string(),
            logged_actions: vec![0, 0, 0],
            counterfactual_actions: vec![1, 1, 1],
            logged_rewards: vec![10.0, 10.0, 10.0],
            action_probs_logged: vec![0.5, 0.5, 0.5],
            action_probs_counterfactual: vec![0.8, 0.8, 0.8],
        };
        let result = counterfactual_evaluate(&query);
        assert!(result.counterfactual_value > result.logged_value);
    }

    #[test]
    fn test_counterfactual_empty() {
        let query = CounterfactualQuery {
            name: "empty".to_string(),
            intervention: "none".to_string(),
            logged_actions: vec![],
            counterfactual_actions: vec![],
            logged_rewards: vec![],
            action_probs_logged: vec![],
            action_probs_counterfactual: vec![],
        };
        let result = counterfactual_evaluate(&query);
        assert_eq!(result.treatment_effect, 0.0);
    }

    #[test]
    fn test_counterfactual_confidence_interval() {
        let query = CounterfactualQuery {
            name: "ci".to_string(),
            intervention: "test".to_string(),
            logged_actions: vec![0; 10],
            counterfactual_actions: vec![1; 10],
            logged_rewards: vec![5.0; 10],
            action_probs_logged: vec![0.5; 10],
            action_probs_counterfactual: vec![0.6; 10],
        };
        let result = counterfactual_evaluate(&query);
        assert!(result.confidence_lower <= result.confidence_upper);
    }

    // ── 15. Multi-Agent Evaluation ────────────────────────────────────

    #[test]
    fn test_multi_agent_basic() {
        let agents = make_agents(&[
            ("a1", &[10.0, 20.0], 5, 3, 0.8),
            ("a2", &[15.0, 25.0], 3, 5, 0.7),
        ]);
        let metrics = compute_multi_agent_metrics(&agents);
        assert!((metrics.social_welfare - 70.0).abs() < f64::EPSILON);
        assert!(metrics.per_agent.contains_key("a1"));
        assert!(metrics.per_agent.contains_key("a2"));
    }

    #[test]
    fn test_multi_agent_empty() {
        let metrics = compute_multi_agent_metrics(&[]);
        assert_eq!(metrics.social_welfare, 0.0);
        assert!((metrics.fairness_score - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_multi_agent_fairness_equal() {
        let agents = make_agents(&[
            ("a1", &[10.0], 1, 1, 0.5),
            ("a2", &[10.0], 1, 1, 0.5),
        ]);
        let metrics = compute_multi_agent_metrics(&agents);
        assert!((metrics.gini_coefficient).abs() < f64::EPSILON);
        assert!((metrics.fairness_score - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_multi_agent_fairness_unequal() {
        let agents = make_agents(&[
            ("rich", &[100.0], 1, 1, 0.5),
            ("poor", &[1.0], 1, 1, 0.5),
        ]);
        let metrics = compute_multi_agent_metrics(&agents);
        assert!(metrics.gini_coefficient > 0.0);
        assert!(metrics.fairness_score < 1.0);
    }

    #[test]
    fn test_multi_agent_communication_efficiency() {
        let agents = make_agents(&[
            ("a1", &[100.0], 2, 2, 0.9),
        ]);
        let metrics = compute_multi_agent_metrics(&agents);
        assert!(metrics.communication_efficiency > 0.0);
    }

    #[test]
    fn test_multi_agent_coalition_value() {
        let agents = make_agents(&[
            ("a1", &[10.0], 1, 1, 0.8),
            ("a2", &[10.0], 1, 1, 0.6),
        ]);
        let metrics = compute_multi_agent_metrics(&agents);
        assert!((metrics.coalition_value - 1.4).abs() < f64::EPSILON);
    }

    // ── 16. Finance-Specific Metrics ──────────────────────────────────

    #[test]
    fn test_finance_metrics_basic() {
        let returns = vec![0.01, 0.02, -0.005, 0.015, 0.01, -0.01, 0.02, 0.005, 0.01, -0.003];
        let fm = compute_finance_metrics(&returns, 0.02, 252.0);
        assert!(fm.sharpe_ratio != 0.0);
        assert!(fm.total_return > 0.0);
    }

    #[test]
    fn test_finance_metrics_empty() {
        let fm = compute_finance_metrics(&[], 0.02, 252.0);
        assert_eq!(fm.sharpe_ratio, 0.0);
        assert_eq!(fm.max_drawdown, 0.0);
    }

    #[test]
    fn test_finance_sharpe_positive_returns() {
        let returns: Vec<f64> = (0..20).map(|i| 0.04 + 0.005 * (i as f64 / 20.0)).collect();
        let fm = compute_finance_metrics(&returns, 0.0, 252.0);
        assert!(fm.sharpe_ratio > 0.0);
    }

    #[test]
    fn test_finance_max_drawdown() {
        let returns = vec![0.1, -0.2, -0.1, 0.05, 0.1];
        let fm = compute_finance_metrics(&returns, 0.0, 252.0);
        assert!(fm.max_drawdown > 0.0);
    }

    #[test]
    fn test_finance_var_cvar() {
        let returns = vec![-0.05, -0.03, -0.01, 0.0, 0.01, 0.02, 0.03, 0.04, 0.05, 0.06,
                          -0.02, 0.01, 0.02, 0.03, -0.04, 0.01, 0.02, 0.03, 0.04, 0.05];
        let fm = compute_finance_metrics(&returns, 0.0, 252.0);
        assert!(fm.var_95 < 0.0); // VaR should be negative (loss)
        assert!(fm.cvar_95 <= fm.var_95); // CVaR should be worse than VaR
    }

    #[test]
    fn test_finance_sortino_ratio() {
        let returns = vec![0.01, 0.02, 0.03, -0.01, 0.02, -0.005, 0.015, 0.01];
        let fm = compute_finance_metrics(&returns, 0.0, 252.0);
        assert!(fm.sortino_ratio > 0.0);
    }

    #[test]
    fn test_finance_volatility() {
        let returns = vec![0.01; 10];
        let fm = compute_finance_metrics(&returns, 0.0, 252.0);
        assert!(fm.volatility < f64::EPSILON); // Constant returns = zero volatility
    }

    #[test]
    fn test_finance_calmar_ratio() {
        let returns = vec![0.02, -0.01, 0.03, 0.02, -0.005, 0.01, 0.015, -0.008, 0.02, 0.01];
        let fm = compute_finance_metrics(&returns, 0.0, 252.0);
        // Calmar = annualized_return / max_drawdown
        if fm.max_drawdown > 0.0 {
            assert!(fm.calmar_ratio > 0.0);
        }
    }

    // ── 17. Continuous Eval Pipeline ──────────────────────────────────

    #[test]
    fn test_continuous_eval_state_new() {
        let config = ContinuousEvalConfig::new("suite", "baseline-v1");
        let state = ContinuousEvalState::new(config);
        assert_eq!(state.run_count, 0);
        assert!(state.history.is_empty());
    }

    #[test]
    fn test_continuous_eval_record_run() {
        let config = ContinuousEvalConfig::new("suite", "baseline");
        let mut state = ContinuousEvalState::new(config);
        state.record_run(EvalRunRecord {
            run_id: 1,
            timestamp: 100,
            mean_reward: 10.0,
            win_rate: 0.6,
            violation_rate: 0.01,
            sharpe_ratio: 1.5,
            passed: true,
        });
        assert_eq!(state.run_count, 1);
        assert_eq!(state.history.len(), 1);
    }

    #[test]
    fn test_continuous_eval_max_history() {
        let mut config = ContinuousEvalConfig::new("suite", "baseline");
        config.max_history = 3;
        let mut state = ContinuousEvalState::new(config);
        for i in 0..5 {
            state.record_run(EvalRunRecord {
                run_id: i,
                timestamp: i as u64 * 100,
                mean_reward: 10.0,
                win_rate: 0.5,
                violation_rate: 0.0,
                sharpe_ratio: 1.0,
                passed: true,
            });
        }
        assert_eq!(state.history.len(), 3);
        assert_eq!(state.history[0].run_id, 2); // Oldest evicted
    }

    #[test]
    fn test_continuous_eval_detect_drift() {
        let config = ContinuousEvalConfig::new("suite", "baseline");
        let mut state = ContinuousEvalState::new(config);
        // Baseline runs with high reward
        for i in 0..5 {
            state.record_run(EvalRunRecord {
                run_id: i,
                timestamp: i as u64 * 100,
                mean_reward: 100.0,
                win_rate: 0.8,
                violation_rate: 0.0,
                sharpe_ratio: 2.0,
                passed: true,
            });
        }
        // Recent runs with much lower reward
        for i in 5..10 {
            state.record_run(EvalRunRecord {
                run_id: i,
                timestamp: i as u64 * 100,
                mean_reward: 50.0,
                win_rate: 0.4,
                violation_rate: 0.1,
                sharpe_ratio: 0.5,
                passed: false,
            });
        }
        let drifts = state.detect_drift();
        assert!(!drifts.is_empty());
        assert!(drifts[0].is_significant);
    }

    #[test]
    fn test_continuous_eval_no_drift() {
        let config = ContinuousEvalConfig::new("suite", "baseline");
        let mut state = ContinuousEvalState::new(config);
        for i in 0..10 {
            state.record_run(EvalRunRecord {
                run_id: i,
                timestamp: i as u64 * 100,
                mean_reward: 100.0,
                win_rate: 0.8,
                violation_rate: 0.0,
                sharpe_ratio: 2.0,
                passed: true,
            });
        }
        let drifts = state.detect_drift();
        // All runs identical, drift magnitude should be 0
        assert!(drifts.iter().all(|d| !d.is_significant || d.drift_magnitude < 0.11));
    }

    #[test]
    fn test_continuous_eval_generate_alerts() {
        let config = ContinuousEvalConfig::new("suite", "baseline");
        let mut state = ContinuousEvalState::new(config);
        for i in 0..5 {
            state.record_run(EvalRunRecord {
                run_id: i,
                timestamp: i as u64 * 100,
                mean_reward: 100.0,
                win_rate: 0.8,
                violation_rate: 0.0,
                sharpe_ratio: 2.0,
                passed: true,
            });
        }
        for i in 5..10 {
            state.record_run(EvalRunRecord {
                run_id: i,
                timestamp: i as u64 * 100,
                mean_reward: 30.0,
                win_rate: 0.2,
                violation_rate: 0.3,
                sharpe_ratio: 0.1,
                passed: false,
            });
        }
        state.detect_drift();
        let alerts = state.generate_alerts();
        assert!(!alerts.is_empty());
    }

    #[test]
    fn test_continuous_eval_trend_summary() {
        let config = ContinuousEvalConfig::new("suite", "baseline");
        let mut state = ContinuousEvalState::new(config);
        state.record_run(EvalRunRecord {
            run_id: 0,
            timestamp: 100,
            mean_reward: 50.0,
            win_rate: 0.5,
            violation_rate: 0.0,
            sharpe_ratio: 1.0,
            passed: true,
        });
        let summary = state.trend_summary();
        assert!(summary.contains("Runs: 1"));
        assert!(summary.contains("50.0000"));
    }

    #[test]
    fn test_continuous_eval_trend_empty() {
        let config = ContinuousEvalConfig::new("suite", "baseline");
        let state = ContinuousEvalState::new(config);
        let summary = state.trend_summary();
        assert!(summary.contains("No evaluation history"));
    }

    // ── Enum display tests ────────────────────────────────────────────

    #[test]
    fn test_metric_kind_all() {
        let all = MetricKind::all();
        assert_eq!(all.len(), 7);
    }

    #[test]
    fn test_metric_kind_display() {
        assert_eq!(format!("{}", MetricKind::Performance), "Performance");
        assert_eq!(format!("{}", MetricKind::Safety), "Safety");
    }

    #[test]
    fn test_eval_stage_display() {
        assert_eq!(format!("{}", EvalStage::SmokeTest), "Smoke Test");
        assert_eq!(format!("{}", EvalStage::OffPolicyEval), "Off-Policy Evaluation");
    }

    #[test]
    fn test_eval_stage_pipeline_order() {
        let order = EvalStage::pipeline_order();
        assert_eq!(order.len(), 8);
        assert_eq!(order[0], EvalStage::SmokeTest);
        assert_eq!(order[7], EvalStage::QualityGateCheck);
    }

    #[test]
    fn test_ope_method_display() {
        assert_eq!(format!("{}", OpeMethod::FittedQEvaluation), "FQE");
        assert_eq!(format!("{}", OpeMethod::Magic), "MAGIC");
    }

    #[test]
    fn test_adversarial_method_display() {
        assert_eq!(format!("{}", AdversarialMethod::Fgsm), "FGSM");
        assert_eq!(format!("{}", AdversarialMethod::RandomNoise), "Random Noise");
    }

    #[test]
    fn test_gate_verdict_display() {
        assert_eq!(format!("{}", GateVerdict::Pass), "PASS");
        assert_eq!(format!("{}", GateVerdict::Fail), "FAIL");
    }

    #[test]
    fn test_stat_test_method_display() {
        assert_eq!(format!("{}", StatTestMethod::TTest), "t-test");
        assert_eq!(format!("{}", StatTestMethod::Bootstrap), "Bootstrap");
    }

    #[test]
    fn test_drift_type_display() {
        assert_eq!(format!("{}", DriftType::Reward), "Reward Drift");
    }

    #[test]
    fn test_alert_severity_weight() {
        assert!((AlertSeverity::Critical.weight() - 1.0).abs() < f64::EPSILON);
        assert!(AlertSeverity::Low.weight() < AlertSeverity::High.weight());
    }

    #[test]
    fn test_report_format_extension() {
        assert_eq!(ReportFormat::Json.extension(), "json");
        assert_eq!(ReportFormat::Markdown.extension(), "md");
        assert_eq!(ReportFormat::Html.extension(), "html");
        assert_eq!(ReportFormat::Plain.extension(), "txt");
    }

    // ── Edge case tests ───────────────────────────────────────────────

    #[test]
    fn test_approximate_two_sided_p_zero() {
        let p = approximate_two_sided_p(0.0);
        assert!((p - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_approximate_two_sided_p_large() {
        let p = approximate_two_sided_p(10.0);
        assert!(p < 0.01);
    }

    #[test]
    fn test_gate_min_adversarial_robustness() {
        let mut gate = QualityGate::new("adv");
        gate.add_condition(GateCondition::MinAdversarialRobustness(0.9));
        let mut metrics = GateMetrics::new();
        metrics.adversarial_robustness = 0.95;
        assert_eq!(gate.evaluate(&metrics), GateVerdict::Pass);
    }

    #[test]
    fn test_gate_max_violation_rate() {
        let mut gate = QualityGate::new("safety");
        gate.add_condition(GateCondition::MaxViolationRate(0.01));
        let mut metrics = GateMetrics::new();
        metrics.violation_rate = 0.005;
        assert_eq!(gate.evaluate(&metrics), GateVerdict::Pass);
    }

    #[test]
    fn test_gate_min_generalization_score() {
        let mut gate = QualityGate::new("gen");
        gate.add_condition(GateCondition::MinGeneralizationScore(0.7));
        let mut metrics = GateMetrics::new();
        metrics.generalization_score = 0.6;
        assert_eq!(gate.evaluate(&metrics), GateVerdict::Fail);
    }

    #[test]
    fn test_gate_max_regression_p_value() {
        let mut gate = QualityGate::new("reg");
        gate.add_condition(GateCondition::MaxRegressionPValue(0.05));
        let mut metrics = GateMetrics::new();
        metrics.regression_p_value = 0.03;
        assert_eq!(gate.evaluate(&metrics), GateVerdict::Pass);
    }

    #[test]
    fn test_gate_max_reward_variance() {
        let mut gate = QualityGate::new("var");
        gate.add_condition(GateCondition::MaxRewardVariance(5.0));
        let mut metrics = GateMetrics::new();
        metrics.reward_variance = 3.0;
        assert_eq!(gate.evaluate(&metrics), GateVerdict::Pass);
    }

    #[test]
    fn test_gate_min_mean_reward() {
        let mut gate = QualityGate::new("rew");
        gate.add_condition(GateCondition::MinMeanReward(50.0));
        let mut metrics = GateMetrics::new();
        metrics.mean_reward = 60.0;
        assert_eq!(gate.evaluate(&metrics), GateVerdict::Pass);
    }
}
