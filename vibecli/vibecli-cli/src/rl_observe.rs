//! RL Observability module — comprehensive monitoring for Reinforcement Learning systems.
//!
//! Provides production-grade RL-OS (Reinforcement Learning Operating System) observability:
//! - Reward drift detection (CUSUM, EWMA, KS test)
//! - Observation & action distribution monitoring
//! - Safety constraint monitoring with near-miss detection
//! - Exploration metrics (state coverage, novelty, entropy)
//! - Multi-agent traces with coalition detection
//! - Training health (gradient norms, loss convergence, GPU utilization)
//! - Configurable alert system
//! - Dashboard data provider for VibeUI panels
//! - Policy diff visualization
//! - Trajectory analysis with reward attribution
//! - Cost tracking and compute efficiency
//! - Anomaly detection on metric time series
//! - Automated report generation
//! - Metric aggregation (sliding window, EMA, percentiles)
//! - Retention & downsampling policies
//! - Correlation analysis between metrics

use std::collections::HashMap;

// ── Enums ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DriftType {
    MeanShift,
    VarianceChange,
    DistributionalShift,
    TrendDrift,
    SeasonalDrift,
}

impl DriftType {
    pub fn label(&self) -> &'static str {
        match self {
            Self::MeanShift => "Mean Shift",
            Self::VarianceChange => "Variance Change",
            Self::DistributionalShift => "Distributional Shift",
            Self::TrendDrift => "Trend Drift",
            Self::SeasonalDrift => "Seasonal Drift",
        }
    }

    pub fn severity_weight(&self) -> f64 {
        match self {
            Self::MeanShift => 0.8,
            Self::VarianceChange => 0.6,
            Self::DistributionalShift => 0.9,
            Self::TrendDrift => 0.5,
            Self::SeasonalDrift => 0.3,
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
    Info,
    Warning,
    Critical,
    Emergency,
}

impl AlertSeverity {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Info => "Info",
            Self::Warning => "Warning",
            Self::Critical => "Critical",
            Self::Emergency => "Emergency",
        }
    }

    pub fn priority(&self) -> u32 {
        match self {
            Self::Info => 0,
            Self::Warning => 1,
            Self::Critical => 2,
            Self::Emergency => 3,
        }
    }
}

impl std::fmt::Display for AlertSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AlertType {
    RewardDrop,
    ConstraintViolationSpike,
    DistributionalShift,
    TrainingDivergence,
    ExplorationCollapse,
    GradientExplosion,
    CostOverrun,
    AnomalyDetected,
    SafetyBreach,
    PolicyDegradation,
}

impl AlertType {
    pub fn label(&self) -> &'static str {
        match self {
            Self::RewardDrop => "Reward Drop",
            Self::ConstraintViolationSpike => "Constraint Violation Spike",
            Self::DistributionalShift => "Distributional Shift",
            Self::TrainingDivergence => "Training Divergence",
            Self::ExplorationCollapse => "Exploration Collapse",
            Self::GradientExplosion => "Gradient Explosion",
            Self::CostOverrun => "Cost Overrun",
            Self::AnomalyDetected => "Anomaly Detected",
            Self::SafetyBreach => "Safety Breach",
            Self::PolicyDegradation => "Policy Degradation",
        }
    }

    pub fn default_severity(&self) -> AlertSeverity {
        match self {
            Self::SafetyBreach | Self::GradientExplosion => AlertSeverity::Emergency,
            Self::RewardDrop | Self::TrainingDivergence | Self::ConstraintViolationSpike => {
                AlertSeverity::Critical
            }
            Self::DistributionalShift | Self::ExplorationCollapse | Self::PolicyDegradation => {
                AlertSeverity::Warning
            }
            Self::CostOverrun | Self::AnomalyDetected => AlertSeverity::Info,
        }
    }
}

impl std::fmt::Display for AlertType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MetricKind {
    Reward,
    Observation,
    Action,
    GradientNorm,
    Loss,
    Entropy,
    LearningRate,
    GpuUtilization,
    ConstraintViolation,
    ExplorationRate,
    EpisodeLength,
    Custom(String),
}

impl MetricKind {
    pub fn label(&self) -> &str {
        match self {
            Self::Reward => "Reward",
            Self::Observation => "Observation",
            Self::Action => "Action",
            Self::GradientNorm => "Gradient Norm",
            Self::Loss => "Loss",
            Self::Entropy => "Entropy",
            Self::LearningRate => "Learning Rate",
            Self::GpuUtilization => "GPU Utilization",
            Self::ConstraintViolation => "Constraint Violation",
            Self::ExplorationRate => "Exploration Rate",
            Self::EpisodeLength => "Episode Length",
            Self::Custom(name) => name.as_str(),
        }
    }
}

impl std::fmt::Display for MetricKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AggregationType {
    SlidingWindow,
    ExponentialMovingAverage,
    Percentile,
    Cumulative,
}

impl AggregationType {
    pub fn label(&self) -> &'static str {
        match self {
            Self::SlidingWindow => "Sliding Window",
            Self::ExponentialMovingAverage => "EMA",
            Self::Percentile => "Percentile",
            Self::Cumulative => "Cumulative",
        }
    }
}

impl std::fmt::Display for AggregationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DownsampleStrategy {
    Average,
    Min,
    Max,
    Last,
    Median,
}

impl DownsampleStrategy {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Average => "Average",
            Self::Min => "Min",
            Self::Max => "Max",
            Self::Last => "Last",
            Self::Median => "Median",
        }
    }
}

impl std::fmt::Display for DownsampleStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DashboardWidgetKind {
    TimeSeries,
    Histogram,
    Heatmap,
    Table,
    Gauge,
    ScatterPlot,
    BarChart,
}

impl DashboardWidgetKind {
    pub fn label(&self) -> &'static str {
        match self {
            Self::TimeSeries => "Time Series",
            Self::Histogram => "Histogram",
            Self::Heatmap => "Heatmap",
            Self::Table => "Table",
            Self::Gauge => "Gauge",
            Self::ScatterPlot => "Scatter Plot",
            Self::BarChart => "Bar Chart",
        }
    }
}

impl std::fmt::Display for DashboardWidgetKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ConstraintKind {
    SafetyBound,
    ResourceLimit,
    ActionRestriction,
    StateRestriction,
    RewardThreshold,
}

impl ConstraintKind {
    pub fn label(&self) -> &'static str {
        match self {
            Self::SafetyBound => "Safety Bound",
            Self::ResourceLimit => "Resource Limit",
            Self::ActionRestriction => "Action Restriction",
            Self::StateRestriction => "State Restriction",
            Self::RewardThreshold => "Reward Threshold",
        }
    }
}

impl std::fmt::Display for ConstraintKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

// ── Structs ────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct MetricPoint {
    pub timestamp: u64,
    pub value: f64,
    pub labels: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct MetricSeries {
    pub name: String,
    pub kind: MetricKind,
    pub points: Vec<MetricPoint>,
    pub metadata: HashMap<String, String>,
}

impl MetricSeries {
    pub fn new(name: &str, kind: MetricKind) -> Self {
        Self {
            name: name.to_string(),
            kind,
            points: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    pub fn push(&mut self, timestamp: u64, value: f64) {
        self.points.push(MetricPoint {
            timestamp,
            value,
            labels: HashMap::new(),
        });
    }

    pub fn push_labeled(&mut self, timestamp: u64, value: f64, labels: HashMap<String, String>) {
        self.points.push(MetricPoint {
            timestamp,
            value,
            labels,
        });
    }

    pub fn values(&self) -> Vec<f64> {
        self.points.iter().map(|p| p.value).collect()
    }

    pub fn len(&self) -> usize {
        self.points.len()
    }

    pub fn is_empty(&self) -> bool {
        self.points.is_empty()
    }

    pub fn last_value(&self) -> Option<f64> {
        self.points.last().map(|p| p.value)
    }

    pub fn mean(&self) -> f64 {
        if self.points.is_empty() {
            return 0.0;
        }
        let sum: f64 = self.points.iter().map(|p| p.value).sum();
        sum / self.points.len() as f64
    }

    pub fn variance(&self) -> f64 {
        if self.points.len() < 2 {
            return 0.0;
        }
        let mean = self.mean();
        let sum_sq: f64 = self.points.iter().map(|p| (p.value - mean).powi(2)).sum();
        sum_sq / (self.points.len() - 1) as f64
    }

    pub fn std_dev(&self) -> f64 {
        self.variance().sqrt()
    }

    pub fn min_value(&self) -> Option<f64> {
        self.points
            .iter()
            .map(|p| p.value)
            .fold(None, |acc, v| match acc {
                None => Some(v),
                Some(a) => Some(if v < a { v } else { a }),
            })
    }

    pub fn max_value(&self) -> Option<f64> {
        self.points
            .iter()
            .map(|p| p.value)
            .fold(None, |acc, v| match acc {
                None => Some(v),
                Some(a) => Some(if v > a { v } else { a }),
            })
    }
}

// ── Reward Drift Detection ─────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct CusumDetector {
    pub target_mean: f64,
    pub threshold: f64,
    pub drift_magnitude: f64,
    pub s_pos: f64,
    pub s_neg: f64,
    pub detected: bool,
    pub detection_index: Option<usize>,
}

impl CusumDetector {
    pub fn new(target_mean: f64, threshold: f64, drift_magnitude: f64) -> Self {
        Self {
            target_mean,
            threshold,
            drift_magnitude,
            s_pos: 0.0,
            s_neg: 0.0,
            detected: false,
            detection_index: None,
        }
    }

    pub fn update(&mut self, value: f64, index: usize) -> bool {
        let z = value - self.target_mean;
        self.s_pos = (self.s_pos + z - self.drift_magnitude / 2.0).max(0.0);
        self.s_neg = (self.s_neg - z - self.drift_magnitude / 2.0).max(0.0);
        if !self.detected && (self.s_pos > self.threshold || self.s_neg > self.threshold) {
            self.detected = true;
            self.detection_index = Some(index);
            return true;
        }
        false
    }

    pub fn reset(&mut self) {
        self.s_pos = 0.0;
        self.s_neg = 0.0;
        self.detected = false;
        self.detection_index = None;
    }
}

#[derive(Debug, Clone)]
pub struct EwmaDetector {
    pub lambda: f64,
    pub control_limit: f64,
    pub target_mean: f64,
    pub target_std: f64,
    pub ewma: f64,
    pub ucl: f64,
    pub lcl: f64,
    pub detected: bool,
    pub n: usize,
}

impl EwmaDetector {
    pub fn new(lambda: f64, control_limit: f64, target_mean: f64, target_std: f64) -> Self {
        let factor = control_limit * target_std * (lambda / (2.0 - lambda)).sqrt();
        Self {
            lambda,
            control_limit,
            target_mean,
            target_std,
            ewma: target_mean,
            ucl: target_mean + factor,
            lcl: target_mean - factor,
            detected: false,
            n: 0,
        }
    }

    pub fn update(&mut self, value: f64) -> bool {
        self.n += 1;
        self.ewma = self.lambda * value + (1.0 - self.lambda) * self.ewma;
        let factor = self.control_limit
            * self.target_std
            * (self.lambda / (2.0 - self.lambda)
                * (1.0 - (1.0 - self.lambda).powi(2 * self.n as i32)))
                .sqrt();
        self.ucl = self.target_mean + factor;
        self.lcl = self.target_mean - factor;
        if self.ewma > self.ucl || self.ewma < self.lcl {
            self.detected = true;
            return true;
        }
        false
    }

    pub fn reset(&mut self) {
        self.ewma = self.target_mean;
        self.detected = false;
        self.n = 0;
    }
}

/// Two-sample Kolmogorov-Smirnov test for distributional shift.
pub fn ks_test(sample_a: &[f64], sample_b: &[f64]) -> KsTestResult {
    if sample_a.is_empty() || sample_b.is_empty() {
        return KsTestResult {
            statistic: 0.0,
            p_value: 1.0,
            reject_null: false,
        };
    }
    let mut sorted_a = sample_a.to_vec();
    let mut sorted_b = sample_b.to_vec();
    sorted_a.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    sorted_b.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let n_a = sorted_a.len() as f64;
    let n_b = sorted_b.len() as f64;
    let mut i = 0usize;
    let mut j = 0usize;
    let mut d_max: f64 = 0.0;

    while i < sorted_a.len() && j < sorted_b.len() {
        let cdf_a = (i + 1) as f64 / n_a;
        let cdf_b = (j + 1) as f64 / n_b;
        let d = (cdf_a - cdf_b).abs();
        if d > d_max {
            d_max = d;
        }
        if sorted_a[i] <= sorted_b[j] {
            i += 1;
        } else {
            j += 1;
        }
    }
    // Approximate p-value using Kolmogorov distribution asymptotic form
    let en = (n_a * n_b / (n_a + n_b)).sqrt();
    let lambda_val = (en + 0.12 + 0.11 / en) * d_max;
    let p_value = (-2.0 * lambda_val * lambda_val).exp().max(0.0).min(1.0);

    KsTestResult {
        statistic: d_max,
        p_value,
        reject_null: p_value < 0.05,
    }
}

#[derive(Debug, Clone)]
pub struct KsTestResult {
    pub statistic: f64,
    pub p_value: f64,
    pub reject_null: bool,
}

#[derive(Debug, Clone)]
pub struct RewardDriftReport {
    pub drift_type: DriftType,
    pub detected: bool,
    pub statistic: f64,
    pub threshold: f64,
    pub detection_index: Option<usize>,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct RewardDriftDetector {
    pub baseline_mean: f64,
    pub baseline_std: f64,
    pub baseline_samples: Vec<f64>,
    pub cusum: CusumDetector,
    pub ewma: EwmaDetector,
    pub recent_window: Vec<f64>,
    pub window_size: usize,
}

impl RewardDriftDetector {
    pub fn new(baseline: &[f64], cusum_threshold: f64, ewma_lambda: f64) -> Self {
        let mean = if baseline.is_empty() {
            0.0
        } else {
            baseline.iter().sum::<f64>() / baseline.len() as f64
        };
        let std_dev = if baseline.len() < 2 {
            1.0
        } else {
            let var: f64 = baseline.iter().map(|v| (v - mean).powi(2)).sum::<f64>()
                / (baseline.len() - 1) as f64;
            var.sqrt().max(0.001)
        };
        Self {
            baseline_mean: mean,
            baseline_std: std_dev,
            baseline_samples: baseline.to_vec(),
            cusum: CusumDetector::new(mean, cusum_threshold, std_dev),
            ewma: EwmaDetector::new(ewma_lambda, 3.0, mean, std_dev),
            recent_window: Vec::new(),
            window_size: 100,
        }
    }

    pub fn observe(&mut self, value: f64, index: usize) -> Vec<RewardDriftReport> {
        let mut reports = Vec::new();
        self.recent_window.push(value);
        if self.recent_window.len() > self.window_size {
            self.recent_window.remove(0);
        }

        if self.cusum.update(value, index) {
            reports.push(RewardDriftReport {
                drift_type: DriftType::MeanShift,
                detected: true,
                statistic: self.cusum.s_pos.max(self.cusum.s_neg),
                threshold: self.cusum.threshold,
                detection_index: self.cusum.detection_index,
                message: format!("CUSUM detected mean shift at index {}", index),
            });
        }

        if self.ewma.update(value) {
            reports.push(RewardDriftReport {
                drift_type: DriftType::MeanShift,
                detected: true,
                statistic: self.ewma.ewma,
                threshold: self.ewma.ucl,
                detection_index: Some(index),
                message: format!("EWMA detected drift at index {}", index),
            });
        }

        // Variance check
        if self.recent_window.len() >= 20 {
            let recent_var = {
                let m: f64 =
                    self.recent_window.iter().sum::<f64>() / self.recent_window.len() as f64;
                self.recent_window
                    .iter()
                    .map(|v| (v - m).powi(2))
                    .sum::<f64>()
                    / (self.recent_window.len() - 1) as f64
            };
            let ratio = recent_var / (self.baseline_std.powi(2)).max(0.0001);
            if ratio > 2.0 || ratio < 0.5 {
                reports.push(RewardDriftReport {
                    drift_type: DriftType::VarianceChange,
                    detected: true,
                    statistic: ratio,
                    threshold: 2.0,
                    detection_index: Some(index),
                    message: format!(
                        "Variance ratio {:.3} exceeds threshold (baseline_var={:.3}, recent_var={:.3})",
                        ratio,
                        self.baseline_std.powi(2),
                        recent_var
                    ),
                });
            }
        }

        // Distributional shift via KS test
        if self.recent_window.len() >= 30 && self.baseline_samples.len() >= 30 {
            let ks = ks_test(&self.baseline_samples, &self.recent_window);
            if ks.reject_null {
                reports.push(RewardDriftReport {
                    drift_type: DriftType::DistributionalShift,
                    detected: true,
                    statistic: ks.statistic,
                    threshold: 0.05,
                    detection_index: Some(index),
                    message: format!(
                        "KS test rejects null (D={:.4}, p={:.4})",
                        ks.statistic, ks.p_value
                    ),
                });
            }
        }

        reports
    }

    pub fn reset(&mut self) {
        self.cusum.reset();
        self.ewma.reset();
        self.recent_window.clear();
    }
}

// ── Observation Distribution Monitoring ────────────────────────────────

#[derive(Debug, Clone)]
pub struct ObservationMonitor {
    pub dimensions: usize,
    pub baseline_means: Vec<f64>,
    pub baseline_stds: Vec<f64>,
    pub current_means: Vec<f64>,
    pub current_counts: Vec<u64>,
    pub current_sums: Vec<f64>,
    pub current_sum_sq: Vec<f64>,
    pub drift_per_dim: Vec<bool>,
    pub total_observations: u64,
}

impl ObservationMonitor {
    pub fn new(dimensions: usize) -> Self {
        Self {
            dimensions,
            baseline_means: vec![0.0; dimensions],
            baseline_stds: vec![1.0; dimensions],
            current_means: vec![0.0; dimensions],
            current_counts: vec![0; dimensions],
            current_sums: vec![0.0; dimensions],
            current_sum_sq: vec![0.0; dimensions],
            drift_per_dim: vec![false; dimensions],
            total_observations: 0,
        }
    }

    pub fn set_baseline(&mut self, means: Vec<f64>, stds: Vec<f64>) {
        self.baseline_means = means;
        self.baseline_stds = stds;
    }

    pub fn observe(&mut self, observation: &[f64]) {
        self.total_observations += 1;
        for (i, &val) in observation.iter().enumerate().take(self.dimensions) {
            self.current_counts[i] += 1;
            self.current_sums[i] += val;
            self.current_sum_sq[i] += val * val;
            let n = self.current_counts[i] as f64;
            self.current_means[i] = self.current_sums[i] / n;

            // Z-test for covariate shift per dimension
            if self.current_counts[i] >= 30 {
                let std = self.baseline_stds[i].max(0.001);
                let z = (self.current_means[i] - self.baseline_means[i]) / (std / n.sqrt());
                self.drift_per_dim[i] = z.abs() > 2.576; // 99% CI
            }
        }
    }

    pub fn drifted_dimensions(&self) -> Vec<usize> {
        self.drift_per_dim
            .iter()
            .enumerate()
            .filter(|(_, &d)| d)
            .map(|(i, _)| i)
            .collect()
    }

    pub fn coverage_ratio(&self) -> f64 {
        if self.dimensions == 0 {
            return 0.0;
        }
        let active = self.current_counts.iter().filter(|&&c| c > 0).count();
        active as f64 / self.dimensions as f64
    }

    pub fn per_dimension_stats(&self) -> Vec<DimensionStats> {
        (0..self.dimensions)
            .map(|i| {
                let n = self.current_counts[i] as f64;
                let mean = if n > 0.0 {
                    self.current_sums[i] / n
                } else {
                    0.0
                };
                let variance = if n > 1.0 {
                    (self.current_sum_sq[i] / n - mean * mean).max(0.0)
                } else {
                    0.0
                };
                DimensionStats {
                    dimension: i,
                    count: self.current_counts[i],
                    mean,
                    variance,
                    drift_detected: self.drift_per_dim[i],
                }
            })
            .collect()
    }

    pub fn reset(&mut self) {
        self.current_counts = vec![0; self.dimensions];
        self.current_sums = vec![0.0; self.dimensions];
        self.current_sum_sq = vec![0.0; self.dimensions];
        self.current_means = vec![0.0; self.dimensions];
        self.drift_per_dim = vec![false; self.dimensions];
        self.total_observations = 0;
    }
}

#[derive(Debug, Clone)]
pub struct DimensionStats {
    pub dimension: usize,
    pub count: u64,
    pub mean: f64,
    pub variance: f64,
    pub drift_detected: bool,
}

// ── Action Distribution Monitoring ─────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ActionMonitor {
    pub action_space_size: usize,
    pub action_counts: Vec<u64>,
    pub total_actions: u64,
    pub baseline_distribution: Vec<f64>,
    pub entropy_history: Vec<f64>,
    pub window_counts: Vec<u64>,
    pub window_size: u64,
    pub window_total: u64,
}

impl ActionMonitor {
    pub fn new(action_space_size: usize) -> Self {
        Self {
            action_space_size,
            action_counts: vec![0; action_space_size],
            total_actions: 0,
            baseline_distribution: vec![1.0 / action_space_size as f64; action_space_size],
            entropy_history: Vec::new(),
            window_counts: vec![0; action_space_size],
            window_size: 1000,
            window_total: 0,
        }
    }

    pub fn set_baseline(&mut self, dist: Vec<f64>) {
        self.baseline_distribution = dist;
    }

    pub fn observe(&mut self, action: usize) {
        if action < self.action_space_size {
            self.action_counts[action] += 1;
            self.total_actions += 1;
            self.window_counts[action] += 1;
            self.window_total += 1;

            if self.window_total >= self.window_size {
                let ent = self.window_entropy();
                self.entropy_history.push(ent);
                self.window_counts = vec![0; self.action_space_size];
                self.window_total = 0;
            }
        }
    }

    pub fn distribution(&self) -> Vec<f64> {
        if self.total_actions == 0 {
            return vec![0.0; self.action_space_size];
        }
        self.action_counts
            .iter()
            .map(|&c| c as f64 / self.total_actions as f64)
            .collect()
    }

    pub fn entropy(&self) -> f64 {
        let dist = self.distribution();
        compute_entropy(&dist)
    }

    fn window_entropy(&self) -> f64 {
        if self.window_total == 0 {
            return 0.0;
        }
        let dist: Vec<f64> = self
            .window_counts
            .iter()
            .map(|&c| c as f64 / self.window_total as f64)
            .collect();
        compute_entropy(&dist)
    }

    pub fn max_entropy(&self) -> f64 {
        (self.action_space_size as f64).ln()
    }

    pub fn kl_divergence_from_baseline(&self) -> f64 {
        let current = self.distribution();
        kl_divergence(&current, &self.baseline_distribution)
    }

    pub fn behavioral_change_detected(&self, kl_threshold: f64) -> bool {
        self.kl_divergence_from_baseline() > kl_threshold
    }

    pub fn most_frequent_action(&self) -> Option<usize> {
        self.action_counts
            .iter()
            .enumerate()
            .max_by_key(|(_, &c)| c)
            .map(|(i, _)| i)
    }

    pub fn least_frequent_action(&self) -> Option<usize> {
        if self.total_actions == 0 {
            return None;
        }
        self.action_counts
            .iter()
            .enumerate()
            .min_by_key(|(_, &c)| c)
            .map(|(i, _)| i)
    }

    pub fn reset(&mut self) {
        self.action_counts = vec![0; self.action_space_size];
        self.total_actions = 0;
        self.entropy_history.clear();
        self.window_counts = vec![0; self.action_space_size];
        self.window_total = 0;
    }
}

fn compute_entropy(dist: &[f64]) -> f64 {
    let mut h = 0.0;
    for &p in dist {
        if p > 0.0 {
            h -= p * p.ln();
        }
    }
    h
}

fn kl_divergence(p: &[f64], q: &[f64]) -> f64 {
    let mut kl = 0.0;
    for (i, &pi) in p.iter().enumerate() {
        let qi = if i < q.len() { q[i] } else { 0.0 };
        if pi > 0.0 && qi > 0.0 {
            kl += pi * (pi / qi).ln();
        }
    }
    kl
}

// ── Safety Constraint Monitoring ───────────────────────────────────────

#[derive(Debug, Clone)]
pub struct SafetyConstraint {
    pub id: String,
    pub name: String,
    pub kind: ConstraintKind,
    pub lower_bound: Option<f64>,
    pub upper_bound: Option<f64>,
    pub near_miss_pct: f64,
}

impl SafetyConstraint {
    pub fn new_upper(id: &str, name: &str, kind: ConstraintKind, bound: f64) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            kind,
            lower_bound: None,
            upper_bound: Some(bound),
            near_miss_pct: 0.1,
        }
    }

    pub fn new_range(
        id: &str,
        name: &str,
        kind: ConstraintKind,
        lower: f64,
        upper: f64,
    ) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            kind,
            lower_bound: Some(lower),
            upper_bound: Some(upper),
            near_miss_pct: 0.1,
        }
    }

    pub fn check(&self, value: f64) -> ConstraintCheckResult {
        let mut violated = false;
        let mut near_miss = false;

        if let Some(upper) = self.upper_bound {
            if value > upper {
                violated = true;
            } else {
                let margin = upper * self.near_miss_pct;
                if value > upper - margin {
                    near_miss = true;
                }
            }
        }
        if let Some(lower) = self.lower_bound {
            if value < lower {
                violated = true;
            } else {
                let margin = lower.abs() * self.near_miss_pct;
                if value < lower + margin {
                    near_miss = true;
                }
            }
        }

        ConstraintCheckResult {
            constraint_id: self.id.clone(),
            value,
            violated,
            near_miss,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ConstraintCheckResult {
    pub constraint_id: String,
    pub value: f64,
    pub violated: bool,
    pub near_miss: bool,
}

#[derive(Debug, Clone)]
pub struct SafetyMonitor {
    pub constraints: Vec<SafetyConstraint>,
    pub violation_counts: HashMap<String, u64>,
    pub near_miss_counts: HashMap<String, u64>,
    pub violation_history: Vec<ConstraintViolationEvent>,
    pub total_checks: u64,
}

impl SafetyMonitor {
    pub fn new() -> Self {
        Self {
            constraints: Vec::new(),
            violation_counts: HashMap::new(),
            near_miss_counts: HashMap::new(),
            violation_history: Vec::new(),
            total_checks: 0,
        }
    }

    pub fn add_constraint(&mut self, constraint: SafetyConstraint) {
        self.violation_counts
            .insert(constraint.id.clone(), 0);
        self.near_miss_counts
            .insert(constraint.id.clone(), 0);
        self.constraints.push(constraint);
    }

    pub fn check_all(&mut self, values: &HashMap<String, f64>, timestamp: u64) -> Vec<ConstraintCheckResult> {
        let mut results = Vec::new();
        self.total_checks += 1;
        for constraint in &self.constraints {
            if let Some(&val) = values.get(&constraint.id) {
                let result = constraint.check(val);
                if result.violated {
                    *self.violation_counts.entry(constraint.id.clone()).or_insert(0) += 1;
                    self.violation_history.push(ConstraintViolationEvent {
                        constraint_id: constraint.id.clone(),
                        timestamp,
                        value: val,
                        near_miss: false,
                    });
                }
                if result.near_miss {
                    *self.near_miss_counts.entry(constraint.id.clone()).or_insert(0) += 1;
                }
                results.push(result);
            }
        }
        results
    }

    pub fn violation_rate(&self, constraint_id: &str) -> f64 {
        if self.total_checks == 0 {
            return 0.0;
        }
        let count = self.violation_counts.get(constraint_id).copied().unwrap_or(0);
        count as f64 / self.total_checks as f64
    }

    pub fn total_violations(&self) -> u64 {
        self.violation_counts.values().sum()
    }

    pub fn total_near_misses(&self) -> u64 {
        self.near_miss_counts.values().sum()
    }

    pub fn safety_score(&self) -> f64 {
        if self.total_checks == 0 {
            return 1.0;
        }
        let total_v: u64 = self.violation_counts.values().sum();
        let max_possible = self.total_checks * self.constraints.len() as u64;
        if max_possible == 0 {
            return 1.0;
        }
        1.0 - (total_v as f64 / max_possible as f64)
    }

    pub fn recent_violation_trend(&self, last_n: usize) -> f64 {
        if self.violation_history.len() < 2 {
            return 0.0;
        }
        let start = if self.violation_history.len() > last_n {
            self.violation_history.len() - last_n
        } else {
            0
        };
        let recent = &self.violation_history[start..];
        recent.len() as f64 / last_n.max(1) as f64
    }
}

#[derive(Debug, Clone)]
pub struct ConstraintViolationEvent {
    pub constraint_id: String,
    pub timestamp: u64,
    pub value: f64,
    pub near_miss: bool,
}

// ── Exploration Metrics ────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ExplorationTracker {
    pub state_visits: HashMap<String, u64>,
    pub total_visits: u64,
    pub novelty_scores: Vec<f64>,
    pub grid_resolution: usize,
    pub heatmap: Vec<Vec<u64>>,
    pub heatmap_rows: usize,
    pub heatmap_cols: usize,
}

impl ExplorationTracker {
    pub fn new(heatmap_rows: usize, heatmap_cols: usize) -> Self {
        Self {
            state_visits: HashMap::new(),
            total_visits: 0,
            novelty_scores: Vec::new(),
            grid_resolution: 100,
            heatmap: vec![vec![0u64; heatmap_cols]; heatmap_rows],
            heatmap_rows,
            heatmap_cols,
        }
    }

    pub fn visit_state(&mut self, state_key: &str) {
        self.total_visits += 1;
        let count = self.state_visits.entry(state_key.to_string()).or_insert(0);
        *count += 1;
        let novelty = 1.0 / (*count as f64);
        self.novelty_scores.push(novelty);
    }

    pub fn visit_grid(&mut self, row: usize, col: usize) {
        if row < self.heatmap_rows && col < self.heatmap_cols {
            self.heatmap[row][col] += 1;
        }
        self.total_visits += 1;
    }

    pub fn state_coverage(&self) -> usize {
        self.state_visits.len()
    }

    pub fn state_coverage_entropy(&self) -> f64 {
        if self.total_visits == 0 {
            return 0.0;
        }
        let mut h = 0.0;
        for &count in self.state_visits.values() {
            let p = count as f64 / self.total_visits as f64;
            if p > 0.0 {
                h -= p * p.ln();
            }
        }
        h
    }

    pub fn max_possible_entropy(&self) -> f64 {
        if self.state_visits.is_empty() {
            return 0.0;
        }
        (self.state_visits.len() as f64).ln()
    }

    pub fn exploration_efficiency(&self) -> f64 {
        let max_ent = self.max_possible_entropy();
        if max_ent == 0.0 {
            return 0.0;
        }
        self.state_coverage_entropy() / max_ent
    }

    pub fn mean_novelty(&self) -> f64 {
        if self.novelty_scores.is_empty() {
            return 0.0;
        }
        self.novelty_scores.iter().sum::<f64>() / self.novelty_scores.len() as f64
    }

    pub fn heatmap_coverage(&self) -> f64 {
        let total_cells = self.heatmap_rows * self.heatmap_cols;
        if total_cells == 0 {
            return 0.0;
        }
        let visited = self
            .heatmap
            .iter()
            .flat_map(|row| row.iter())
            .filter(|&&c| c > 0)
            .count();
        visited as f64 / total_cells as f64
    }

    pub fn heatmap_data(&self) -> &Vec<Vec<u64>> {
        &self.heatmap
    }

    pub fn top_visited_states(&self, n: usize) -> Vec<(String, u64)> {
        let mut entries: Vec<_> = self.state_visits.iter().map(|(k, &v)| (k.clone(), v)).collect();
        entries.sort_by(|a, b| b.1.cmp(&a.1));
        entries.truncate(n);
        entries
    }

    pub fn reset(&mut self) {
        self.state_visits.clear();
        self.total_visits = 0;
        self.novelty_scores.clear();
        self.heatmap = vec![vec![0u64; self.heatmap_cols]; self.heatmap_rows];
    }
}

// ── Multi-Agent Traces ─────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct AgentTrace {
    pub agent_id: String,
    pub rewards: Vec<f64>,
    pub actions: Vec<usize>,
    pub messages_sent: u64,
    pub messages_received: u64,
    pub coalition_id: Option<String>,
}

impl AgentTrace {
    pub fn new(agent_id: &str) -> Self {
        Self {
            agent_id: agent_id.to_string(),
            rewards: Vec::new(),
            actions: Vec::new(),
            messages_sent: 0,
            messages_received: 0,
            coalition_id: None,
        }
    }

    pub fn total_reward(&self) -> f64 {
        self.rewards.iter().sum()
    }

    pub fn mean_reward(&self) -> f64 {
        if self.rewards.is_empty() {
            return 0.0;
        }
        self.total_reward() / self.rewards.len() as f64
    }

    pub fn communication_ratio(&self) -> f64 {
        let total = self.messages_sent + self.messages_received;
        if total == 0 {
            return 0.0;
        }
        self.messages_sent as f64 / total as f64
    }
}

#[derive(Debug, Clone)]
pub struct MultiAgentTracer {
    pub agents: HashMap<String, AgentTrace>,
    pub communication_graph: HashMap<(String, String), u64>,
    pub global_rewards: Vec<f64>,
}

impl MultiAgentTracer {
    pub fn new() -> Self {
        Self {
            agents: HashMap::new(),
            communication_graph: HashMap::new(),
            global_rewards: Vec::new(),
        }
    }

    pub fn register_agent(&mut self, agent_id: &str) {
        self.agents
            .entry(agent_id.to_string())
            .or_insert_with(|| AgentTrace::new(agent_id));
    }

    pub fn record_reward(&mut self, agent_id: &str, reward: f64) {
        if let Some(trace) = self.agents.get_mut(agent_id) {
            trace.rewards.push(reward);
        }
    }

    pub fn record_action(&mut self, agent_id: &str, action: usize) {
        if let Some(trace) = self.agents.get_mut(agent_id) {
            trace.actions.push(action);
        }
    }

    pub fn record_communication(&mut self, from: &str, to: &str) {
        let key = (from.to_string(), to.to_string());
        *self.communication_graph.entry(key).or_insert(0) += 1;
        if let Some(trace) = self.agents.get_mut(from) {
            trace.messages_sent += 1;
        }
        if let Some(trace) = self.agents.get_mut(to) {
            trace.messages_received += 1;
        }
    }

    pub fn record_global_reward(&mut self, reward: f64) {
        self.global_rewards.push(reward);
    }

    pub fn set_coalition(&mut self, agent_id: &str, coalition_id: &str) {
        if let Some(trace) = self.agents.get_mut(agent_id) {
            trace.coalition_id = Some(coalition_id.to_string());
        }
    }

    pub fn social_welfare(&self) -> f64 {
        self.agents.values().map(|t| t.total_reward()).sum()
    }

    pub fn gini_coefficient(&self) -> f64 {
        let rewards: Vec<f64> = self.agents.values().map(|t| t.total_reward()).collect();
        compute_gini(&rewards)
    }

    pub fn coalition_rewards(&self) -> HashMap<String, f64> {
        let mut result: HashMap<String, f64> = HashMap::new();
        for trace in self.agents.values() {
            let coalition = trace
                .coalition_id
                .as_deref()
                .unwrap_or("unassigned")
                .to_string();
            *result.entry(coalition).or_insert(0.0) += trace.total_reward();
        }
        result
    }

    pub fn top_communicators(&self, n: usize) -> Vec<(String, u64)> {
        let mut comm: Vec<_> = self
            .agents
            .iter()
            .map(|(id, t)| (id.clone(), t.messages_sent + t.messages_received))
            .collect();
        comm.sort_by(|a, b| b.1.cmp(&a.1));
        comm.truncate(n);
        comm
    }

    pub fn communication_density(&self) -> f64 {
        let n = self.agents.len();
        if n < 2 {
            return 0.0;
        }
        let max_edges = n * (n - 1);
        let actual_edges = self.communication_graph.len();
        actual_edges as f64 / max_edges as f64
    }

    pub fn per_agent_reward_decomposition(&self) -> Vec<(String, f64, f64)> {
        let sw = self.social_welfare();
        self.agents
            .iter()
            .map(|(id, trace)| {
                let total = trace.total_reward();
                let pct = if sw != 0.0 { total / sw * 100.0 } else { 0.0 };
                (id.clone(), total, pct)
            })
            .collect()
    }
}

fn compute_gini(values: &[f64]) -> f64 {
    let n = values.len();
    if n == 0 {
        return 0.0;
    }
    let mean = values.iter().sum::<f64>() / n as f64;
    if mean == 0.0 {
        return 0.0;
    }
    let mut sum_diff = 0.0;
    for i in 0..n {
        for j in 0..n {
            sum_diff += (values[i] - values[j]).abs();
        }
    }
    sum_diff / (2.0 * n as f64 * n as f64 * mean)
}

// ── Training Health Metrics ────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct TrainingHealthMonitor {
    pub gradient_norms: Vec<f64>,
    pub losses: Vec<f64>,
    pub learning_rates: Vec<f64>,
    pub gpu_utilizations: Vec<f64>,
    pub epoch_times: Vec<f64>,
    pub convergence_window: usize,
    pub gradient_explosion_threshold: f64,
    pub gradient_vanishing_threshold: f64,
}

impl TrainingHealthMonitor {
    pub fn new() -> Self {
        Self {
            gradient_norms: Vec::new(),
            losses: Vec::new(),
            learning_rates: Vec::new(),
            gpu_utilizations: Vec::new(),
            epoch_times: Vec::new(),
            convergence_window: 50,
            gradient_explosion_threshold: 100.0,
            gradient_vanishing_threshold: 1e-7,
        }
    }

    pub fn record_gradient_norm(&mut self, norm: f64) {
        self.gradient_norms.push(norm);
    }

    pub fn record_loss(&mut self, loss: f64) {
        self.losses.push(loss);
    }

    pub fn record_learning_rate(&mut self, lr: f64) {
        self.learning_rates.push(lr);
    }

    pub fn record_gpu_utilization(&mut self, util: f64) {
        self.gpu_utilizations.push(util);
    }

    pub fn record_epoch_time(&mut self, seconds: f64) {
        self.epoch_times.push(seconds);
    }

    pub fn is_gradient_exploding(&self) -> bool {
        self.gradient_norms
            .last()
            .map(|&n| n > self.gradient_explosion_threshold)
            .unwrap_or(false)
    }

    pub fn is_gradient_vanishing(&self) -> bool {
        self.gradient_norms
            .last()
            .map(|&n| n < self.gradient_vanishing_threshold)
            .unwrap_or(false)
    }

    pub fn is_loss_converged(&self) -> bool {
        if self.losses.len() < self.convergence_window {
            return false;
        }
        let window = &self.losses[self.losses.len() - self.convergence_window..];
        let mean = window.iter().sum::<f64>() / window.len() as f64;
        if mean == 0.0 {
            return true;
        }
        let variance = window.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / window.len() as f64;
        let cv = variance.sqrt() / mean.abs();
        cv < 0.01
    }

    pub fn is_loss_diverging(&self) -> bool {
        if self.losses.len() < 10 {
            return false;
        }
        let recent = &self.losses[self.losses.len() - 10..];
        let first_half: f64 = recent[..5].iter().sum::<f64>() / 5.0;
        let second_half: f64 = recent[5..].iter().sum::<f64>() / 5.0;
        second_half > first_half * 1.5 || second_half.is_nan() || second_half.is_infinite()
    }

    pub fn mean_gpu_utilization(&self) -> f64 {
        if self.gpu_utilizations.is_empty() {
            return 0.0;
        }
        self.gpu_utilizations.iter().sum::<f64>() / self.gpu_utilizations.len() as f64
    }

    pub fn mean_epoch_time(&self) -> f64 {
        if self.epoch_times.is_empty() {
            return 0.0;
        }
        self.epoch_times.iter().sum::<f64>() / self.epoch_times.len() as f64
    }

    pub fn latest_loss(&self) -> Option<f64> {
        self.losses.last().copied()
    }

    pub fn latest_gradient_norm(&self) -> Option<f64> {
        self.gradient_norms.last().copied()
    }

    pub fn training_progress_pct(&self) -> f64 {
        if self.losses.len() < 2 {
            return 0.0;
        }
        let initial = self.losses[0];
        let current = self.losses.last().copied().unwrap_or(initial);
        if initial == 0.0 {
            return 100.0;
        }
        ((initial - current) / initial * 100.0).max(0.0).min(100.0)
    }

    pub fn health_summary(&self) -> TrainingHealthSummary {
        TrainingHealthSummary {
            total_epochs: self.losses.len(),
            latest_loss: self.latest_loss(),
            latest_gradient_norm: self.latest_gradient_norm(),
            is_converged: self.is_loss_converged(),
            is_diverging: self.is_loss_diverging(),
            gradient_exploding: self.is_gradient_exploding(),
            gradient_vanishing: self.is_gradient_vanishing(),
            mean_gpu_util: self.mean_gpu_utilization(),
            progress_pct: self.training_progress_pct(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TrainingHealthSummary {
    pub total_epochs: usize,
    pub latest_loss: Option<f64>,
    pub latest_gradient_norm: Option<f64>,
    pub is_converged: bool,
    pub is_diverging: bool,
    pub gradient_exploding: bool,
    pub gradient_vanishing: bool,
    pub mean_gpu_util: f64,
    pub progress_pct: f64,
}

// ── Alert System ───────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct AlertRule {
    pub id: String,
    pub alert_type: AlertType,
    pub severity: AlertSeverity,
    pub threshold: f64,
    pub cooldown_secs: u64,
    pub enabled: bool,
    pub description: String,
    pub last_triggered: Option<u64>,
}

impl AlertRule {
    pub fn new(id: &str, alert_type: AlertType, threshold: f64) -> Self {
        let severity = alert_type.default_severity();
        Self {
            id: id.to_string(),
            alert_type,
            severity,
            threshold,
            cooldown_secs: 300,
            enabled: true,
            description: String::new(),
            last_triggered: None,
        }
    }

    pub fn with_severity(mut self, severity: AlertSeverity) -> Self {
        self.severity = severity;
        self
    }

    pub fn with_cooldown(mut self, secs: u64) -> Self {
        self.cooldown_secs = secs;
        self
    }

    pub fn with_description(mut self, desc: &str) -> Self {
        self.description = desc.to_string();
        self
    }

    pub fn can_fire(&self, now: u64) -> bool {
        if !self.enabled {
            return false;
        }
        match self.last_triggered {
            None => true,
            Some(last) => now.saturating_sub(last) >= self.cooldown_secs,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Alert {
    pub rule_id: String,
    pub alert_type: AlertType,
    pub severity: AlertSeverity,
    pub timestamp: u64,
    pub value: f64,
    pub threshold: f64,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct AlertManager {
    pub rules: Vec<AlertRule>,
    pub alerts: Vec<Alert>,
    pub max_alerts: usize,
}

impl AlertManager {
    pub fn new() -> Self {
        Self {
            rules: Vec::new(),
            alerts: Vec::new(),
            max_alerts: 10000,
        }
    }

    pub fn add_rule(&mut self, rule: AlertRule) {
        self.rules.push(rule);
    }

    pub fn check_and_fire(
        &mut self,
        alert_type: &AlertType,
        value: f64,
        timestamp: u64,
    ) -> Vec<Alert> {
        let mut fired = Vec::new();
        for rule in &mut self.rules {
            if rule.alert_type == *alert_type && rule.can_fire(timestamp) {
                let should_fire = match alert_type {
                    AlertType::RewardDrop
                    | AlertType::PolicyDegradation
                    | AlertType::ExplorationCollapse => value < -rule.threshold || value.abs() > rule.threshold,
                    _ => value > rule.threshold,
                };
                if should_fire {
                    let alert = Alert {
                        rule_id: rule.id.clone(),
                        alert_type: rule.alert_type.clone(),
                        severity: rule.severity.clone(),
                        timestamp,
                        value,
                        threshold: rule.threshold,
                        message: format!(
                            "{}: value={:.4} exceeds threshold={:.4}",
                            rule.alert_type.label(),
                            value,
                            rule.threshold
                        ),
                    };
                    rule.last_triggered = Some(timestamp);
                    fired.push(alert);
                }
            }
        }
        for alert in &fired {
            self.alerts.push(alert.clone());
        }
        // Enforce retention
        if self.alerts.len() > self.max_alerts {
            let drain_count = self.alerts.len() - self.max_alerts;
            self.alerts.drain(0..drain_count);
        }
        fired
    }

    pub fn recent_alerts(&self, n: usize) -> &[Alert] {
        let start = if self.alerts.len() > n {
            self.alerts.len() - n
        } else {
            0
        };
        &self.alerts[start..]
    }

    pub fn alerts_by_severity(&self, severity: &AlertSeverity) -> Vec<&Alert> {
        self.alerts
            .iter()
            .filter(|a| a.severity == *severity)
            .collect()
    }

    pub fn alert_count(&self) -> usize {
        self.alerts.len()
    }

    pub fn clear(&mut self) {
        self.alerts.clear();
    }
}

// ── Dashboard Data Provider ────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct DashboardWidget {
    pub id: String,
    pub title: String,
    pub kind: DashboardWidgetKind,
    pub data: DashboardData,
}

#[derive(Debug, Clone)]
pub enum DashboardData {
    TimeSeries(Vec<(u64, f64)>),
    Histogram(Vec<(String, f64)>),
    Heatmap {
        rows: usize,
        cols: usize,
        values: Vec<Vec<f64>>,
    },
    Table {
        headers: Vec<String>,
        rows: Vec<Vec<String>>,
    },
    Gauge {
        value: f64,
        min: f64,
        max: f64,
    },
}

#[derive(Debug, Clone)]
pub struct DashboardProvider {
    pub widgets: Vec<DashboardWidget>,
}

impl DashboardProvider {
    pub fn new() -> Self {
        Self {
            widgets: Vec::new(),
        }
    }

    pub fn add_time_series(&mut self, id: &str, title: &str, data: Vec<(u64, f64)>) {
        self.widgets.push(DashboardWidget {
            id: id.to_string(),
            title: title.to_string(),
            kind: DashboardWidgetKind::TimeSeries,
            data: DashboardData::TimeSeries(data),
        });
    }

    pub fn add_histogram(&mut self, id: &str, title: &str, data: Vec<(String, f64)>) {
        self.widgets.push(DashboardWidget {
            id: id.to_string(),
            title: title.to_string(),
            kind: DashboardWidgetKind::Histogram,
            data: DashboardData::Histogram(data),
        });
    }

    pub fn add_heatmap(
        &mut self,
        id: &str,
        title: &str,
        rows: usize,
        cols: usize,
        values: Vec<Vec<f64>>,
    ) {
        self.widgets.push(DashboardWidget {
            id: id.to_string(),
            title: title.to_string(),
            kind: DashboardWidgetKind::Heatmap,
            data: DashboardData::Heatmap { rows, cols, values },
        });
    }

    pub fn add_table(&mut self, id: &str, title: &str, headers: Vec<String>, rows: Vec<Vec<String>>) {
        self.widgets.push(DashboardWidget {
            id: id.to_string(),
            title: title.to_string(),
            kind: DashboardWidgetKind::Table,
            data: DashboardData::Table { headers, rows },
        });
    }

    pub fn add_gauge(&mut self, id: &str, title: &str, value: f64, min: f64, max: f64) {
        self.widgets.push(DashboardWidget {
            id: id.to_string(),
            title: title.to_string(),
            kind: DashboardWidgetKind::Gauge,
            data: DashboardData::Gauge { value, min, max },
        });
    }

    pub fn widget_count(&self) -> usize {
        self.widgets.len()
    }

    pub fn get_widget(&self, id: &str) -> Option<&DashboardWidget> {
        self.widgets.iter().find(|w| w.id == id)
    }

    pub fn from_metric_series(series: &MetricSeries) -> DashboardWidget {
        let data: Vec<(u64, f64)> = series.points.iter().map(|p| (p.timestamp, p.value)).collect();
        DashboardWidget {
            id: series.name.clone(),
            title: series.name.clone(),
            kind: DashboardWidgetKind::TimeSeries,
            data: DashboardData::TimeSeries(data),
        }
    }

    pub fn from_exploration_heatmap(tracker: &ExplorationTracker) -> DashboardWidget {
        let values: Vec<Vec<f64>> = tracker
            .heatmap
            .iter()
            .map(|row| row.iter().map(|&v| v as f64).collect())
            .collect();
        DashboardWidget {
            id: "exploration_heatmap".to_string(),
            title: "State Exploration Heatmap".to_string(),
            kind: DashboardWidgetKind::Heatmap,
            data: DashboardData::Heatmap {
                rows: tracker.heatmap_rows,
                cols: tracker.heatmap_cols,
                values,
            },
        }
    }
}

// ── Policy Diff Visualization ──────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct PolicySnapshot {
    pub version: String,
    pub timestamp: u64,
    pub action_distribution: Vec<f64>,
    pub state_value_samples: HashMap<String, f64>,
    pub metadata: HashMap<String, String>,
}

impl PolicySnapshot {
    pub fn new(version: &str, timestamp: u64, distribution: Vec<f64>) -> Self {
        Self {
            version: version.to_string(),
            timestamp,
            action_distribution: distribution,
            state_value_samples: HashMap::new(),
            metadata: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PolicyDiff {
    pub version_a: String,
    pub version_b: String,
    pub action_distribution_diff: Vec<f64>,
    pub kl_divergence: f64,
    pub max_action_change: f64,
    pub changed_states: Vec<(String, f64, f64)>,
}

pub fn compute_policy_diff(a: &PolicySnapshot, b: &PolicySnapshot) -> PolicyDiff {
    let diff: Vec<f64> = a
        .action_distribution
        .iter()
        .zip(b.action_distribution.iter())
        .map(|(x, y)| y - x)
        .collect();
    let max_change = diff.iter().map(|d| d.abs()).fold(0.0f64, f64::max);
    let kl = kl_divergence(&b.action_distribution, &a.action_distribution);

    let mut changed_states = Vec::new();
    for (state, &val_a) in &a.state_value_samples {
        if let Some(&val_b) = b.state_value_samples.get(state) {
            if (val_a - val_b).abs() > 0.01 {
                changed_states.push((state.clone(), val_a, val_b));
            }
        }
    }

    PolicyDiff {
        version_a: a.version.clone(),
        version_b: b.version.clone(),
        action_distribution_diff: diff,
        kl_divergence: kl,
        max_action_change: max_change,
        changed_states,
    }
}

// ── Trajectory Analysis ────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct TrajectoryStep {
    pub timestep: usize,
    pub state_key: String,
    pub action: usize,
    pub reward: f64,
    pub cumulative_reward: f64,
    pub info: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct Trajectory {
    pub episode_id: String,
    pub steps: Vec<TrajectoryStep>,
    pub total_reward: f64,
    pub terminal: bool,
}

impl Trajectory {
    pub fn new(episode_id: &str) -> Self {
        Self {
            episode_id: episode_id.to_string(),
            steps: Vec::new(),
            total_reward: 0.0,
            terminal: false,
        }
    }

    pub fn add_step(&mut self, state_key: &str, action: usize, reward: f64) {
        self.total_reward += reward;
        self.steps.push(TrajectoryStep {
            timestep: self.steps.len(),
            state_key: state_key.to_string(),
            action,
            reward,
            cumulative_reward: self.total_reward,
            info: HashMap::new(),
        });
    }

    pub fn finish(&mut self) {
        self.terminal = true;
    }

    pub fn length(&self) -> usize {
        self.steps.len()
    }

    pub fn reward_attribution(&self) -> Vec<f64> {
        self.steps.iter().map(|s| s.reward).collect()
    }

    pub fn critical_decision_points(&self, threshold: f64) -> Vec<usize> {
        let mut points = Vec::new();
        for (i, step) in self.steps.iter().enumerate() {
            if step.reward.abs() > threshold {
                points.push(i);
            }
        }
        points
    }

    pub fn reward_variance(&self) -> f64 {
        if self.steps.len() < 2 {
            return 0.0;
        }
        let mean = self.total_reward / self.steps.len() as f64;
        let sum_sq: f64 = self.steps.iter().map(|s| (s.reward - mean).powi(2)).sum();
        sum_sq / (self.steps.len() - 1) as f64
    }

    pub fn max_reward_step(&self) -> Option<usize> {
        self.steps
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| {
                a.reward
                    .partial_cmp(&b.reward)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(i, _)| i)
    }

    pub fn min_reward_step(&self) -> Option<usize> {
        self.steps
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| {
                a.reward
                    .partial_cmp(&b.reward)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(i, _)| i)
    }
}

#[derive(Debug, Clone)]
pub struct TrajectoryAnalyzer {
    pub trajectories: Vec<Trajectory>,
}

impl TrajectoryAnalyzer {
    pub fn new() -> Self {
        Self {
            trajectories: Vec::new(),
        }
    }

    pub fn add_trajectory(&mut self, traj: Trajectory) {
        self.trajectories.push(traj);
    }

    pub fn mean_episode_length(&self) -> f64 {
        if self.trajectories.is_empty() {
            return 0.0;
        }
        self.trajectories.iter().map(|t| t.length() as f64).sum::<f64>()
            / self.trajectories.len() as f64
    }

    pub fn mean_episode_reward(&self) -> f64 {
        if self.trajectories.is_empty() {
            return 0.0;
        }
        self.trajectories.iter().map(|t| t.total_reward).sum::<f64>()
            / self.trajectories.len() as f64
    }

    pub fn best_episode(&self) -> Option<&Trajectory> {
        self.trajectories
            .iter()
            .max_by(|a, b| {
                a.total_reward
                    .partial_cmp(&b.total_reward)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    }

    pub fn worst_episode(&self) -> Option<&Trajectory> {
        self.trajectories
            .iter()
            .min_by(|a, b| {
                a.total_reward
                    .partial_cmp(&b.total_reward)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    }

    pub fn reward_percentile(&self, pct: f64) -> f64 {
        percentile_value(
            &self
                .trajectories
                .iter()
                .map(|t| t.total_reward)
                .collect::<Vec<_>>(),
            pct,
        )
    }

    pub fn episode_count(&self) -> usize {
        self.trajectories.len()
    }
}

// ── Cost Tracking ──────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct CostEntry {
    pub timestamp: u64,
    pub experiment_id: String,
    pub gpu_hours: f64,
    pub cost_usd: f64,
    pub reward_improvement: f64,
    pub description: String,
}

#[derive(Debug, Clone)]
pub struct CostTracker {
    pub entries: Vec<CostEntry>,
    pub cost_per_gpu_hour: f64,
}

impl CostTracker {
    pub fn new(cost_per_gpu_hour: f64) -> Self {
        Self {
            entries: Vec::new(),
            cost_per_gpu_hour,
        }
    }

    pub fn record(
        &mut self,
        experiment_id: &str,
        gpu_hours: f64,
        reward_improvement: f64,
        timestamp: u64,
    ) {
        self.entries.push(CostEntry {
            timestamp,
            experiment_id: experiment_id.to_string(),
            gpu_hours,
            cost_usd: gpu_hours * self.cost_per_gpu_hour,
            reward_improvement,
            description: String::new(),
        });
    }

    pub fn total_gpu_hours(&self) -> f64 {
        self.entries.iter().map(|e| e.gpu_hours).sum()
    }

    pub fn total_cost_usd(&self) -> f64 {
        self.entries.iter().map(|e| e.cost_usd).sum()
    }

    pub fn total_reward_improvement(&self) -> f64 {
        self.entries.iter().map(|e| e.reward_improvement).sum()
    }

    pub fn cost_per_reward_unit(&self) -> f64 {
        let total_improvement = self.total_reward_improvement();
        if total_improvement == 0.0 {
            return f64::INFINITY;
        }
        self.total_cost_usd() / total_improvement
    }

    pub fn compute_efficiency(&self) -> f64 {
        let total_hours = self.total_gpu_hours();
        if total_hours == 0.0 {
            return 0.0;
        }
        self.total_reward_improvement() / total_hours
    }

    pub fn cost_by_experiment(&self) -> HashMap<String, f64> {
        let mut map: HashMap<String, f64> = HashMap::new();
        for entry in &self.entries {
            *map.entry(entry.experiment_id.clone()).or_insert(0.0) += entry.cost_usd;
        }
        map
    }

    pub fn most_expensive_experiment(&self) -> Option<(String, f64)> {
        self.cost_by_experiment()
            .into_iter()
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
    }

    pub fn most_efficient_experiment(&self) -> Option<(String, f64)> {
        let mut exp_hours: HashMap<String, f64> = HashMap::new();
        let mut exp_reward: HashMap<String, f64> = HashMap::new();
        for entry in &self.entries {
            *exp_hours.entry(entry.experiment_id.clone()).or_insert(0.0) += entry.gpu_hours;
            *exp_reward.entry(entry.experiment_id.clone()).or_insert(0.0) +=
                entry.reward_improvement;
        }
        exp_hours
            .into_iter()
            .filter_map(|(id, hours)| {
                let reward = exp_reward.get(&id).copied().unwrap_or(0.0);
                if hours > 0.0 {
                    Some((id, reward / hours))
                } else {
                    None
                }
            })
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
    }
}

// ── Anomaly Detection ──────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct AnomalyResult {
    pub index: usize,
    pub value: f64,
    pub score: f64,
    pub is_anomaly: bool,
    pub method: String,
}

/// Statistical anomaly detection using z-score method.
pub fn detect_anomalies_zscore(values: &[f64], z_threshold: f64) -> Vec<AnomalyResult> {
    if values.len() < 3 {
        return Vec::new();
    }
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    let std_dev = {
        let var = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / (values.len() - 1) as f64;
        var.sqrt()
    };
    if std_dev == 0.0 {
        return Vec::new();
    }
    values
        .iter()
        .enumerate()
        .map(|(i, &v)| {
            let z = (v - mean).abs() / std_dev;
            AnomalyResult {
                index: i,
                value: v,
                score: z,
                is_anomaly: z > z_threshold,
                method: "z-score".to_string(),
            }
        })
        .collect()
}

/// Isolation-forest-inspired anomaly detection using path length heuristic.
/// Simplified for single-dimensional time series: uses random subsampling
/// and measures how quickly a point can be isolated.
pub fn detect_anomalies_isolation(values: &[f64], contamination: f64) -> Vec<AnomalyResult> {
    if values.len() < 5 {
        return Vec::new();
    }
    let n = values.len();
    let avg_path_length = (2.0 * (n as f64 - 1.0).ln() + 0.5772) - 2.0 * (n as f64 - 1.0) / n as f64;

    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median = sorted[n / 2];
    let mad = {
        let mut abs_devs: Vec<f64> = values.iter().map(|v| (v - median).abs()).collect();
        abs_devs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        abs_devs[abs_devs.len() / 2]
    };

    let scores: Vec<f64> = values
        .iter()
        .map(|v| {
            if mad == 0.0 {
                return 0.5;
            }
            let deviation = (v - median).abs() / mad;
            let path_est = avg_path_length / (1.0 + deviation);
            let score = 2.0_f64.powf(-path_est / avg_path_length);
            score
        })
        .collect();

    let mut sorted_scores: Vec<f64> = scores.clone();
    sorted_scores.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
    let threshold_idx = ((n as f64 * contamination).ceil() as usize).min(n - 1);
    let threshold = sorted_scores[threshold_idx];

    values
        .iter()
        .enumerate()
        .map(|(i, &v)| AnomalyResult {
            index: i,
            value: v,
            score: scores[i],
            is_anomaly: scores[i] >= threshold,
            method: "isolation".to_string(),
        })
        .collect()
}

/// Moving average anomaly detection: flags points deviating from local window average.
pub fn detect_anomalies_moving_avg(
    values: &[f64],
    window: usize,
    deviation_factor: f64,
) -> Vec<AnomalyResult> {
    if values.len() < window + 1 {
        return Vec::new();
    }
    let mut results = Vec::new();
    for i in window..values.len() {
        let w = &values[i - window..i];
        let mean = w.iter().sum::<f64>() / w.len() as f64;
        let std = {
            let var = w.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / w.len() as f64;
            var.sqrt()
        };
        let dev = if std > 0.0 {
            (values[i] - mean).abs() / std
        } else {
            0.0
        };
        results.push(AnomalyResult {
            index: i,
            value: values[i],
            score: dev,
            is_anomaly: dev > deviation_factor,
            method: "moving_average".to_string(),
        });
    }
    results
}

// ── Report Generation ──────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ObservabilityReport {
    pub title: String,
    pub generated_at: u64,
    pub sections: Vec<ReportSection>,
}

#[derive(Debug, Clone)]
pub struct ReportSection {
    pub heading: String,
    pub content: String,
    pub metrics: HashMap<String, f64>,
    pub status: ReportStatus,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ReportStatus {
    Healthy,
    Warning,
    Critical,
    Unknown,
}

impl ReportStatus {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Healthy => "Healthy",
            Self::Warning => "Warning",
            Self::Critical => "Critical",
            Self::Unknown => "Unknown",
        }
    }
}

impl std::fmt::Display for ReportStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

pub struct ReportGenerator;

impl ReportGenerator {
    pub fn generate(
        training: &TrainingHealthMonitor,
        safety: &SafetyMonitor,
        cost: &CostTracker,
        alerts: &AlertManager,
        timestamp: u64,
    ) -> ObservabilityReport {
        let mut sections = Vec::new();

        // Training health
        let health = training.health_summary();
        let mut train_metrics = HashMap::new();
        if let Some(loss) = health.latest_loss {
            train_metrics.insert("latest_loss".to_string(), loss);
        }
        if let Some(grad) = health.latest_gradient_norm {
            train_metrics.insert("latest_gradient_norm".to_string(), grad);
        }
        train_metrics.insert("mean_gpu_utilization".to_string(), health.mean_gpu_util);
        train_metrics.insert("progress_pct".to_string(), health.progress_pct);
        let train_status = if health.is_diverging || health.gradient_exploding {
            ReportStatus::Critical
        } else if health.gradient_vanishing {
            ReportStatus::Warning
        } else if health.is_converged {
            ReportStatus::Healthy
        } else {
            ReportStatus::Unknown
        };
        sections.push(ReportSection {
            heading: "Training Health".to_string(),
            content: format!(
                "Epochs: {}, Converged: {}, Diverging: {}, GPU: {:.1}%",
                health.total_epochs, health.is_converged, health.is_diverging, health.mean_gpu_util
            ),
            metrics: train_metrics,
            status: train_status,
        });

        // Safety compliance
        let mut safety_metrics = HashMap::new();
        safety_metrics.insert("total_violations".to_string(), safety.total_violations() as f64);
        safety_metrics.insert("total_near_misses".to_string(), safety.total_near_misses() as f64);
        safety_metrics.insert("safety_score".to_string(), safety.safety_score());
        let safety_status = if safety.total_violations() > 0 {
            ReportStatus::Critical
        } else if safety.total_near_misses() > 0 {
            ReportStatus::Warning
        } else {
            ReportStatus::Healthy
        };
        sections.push(ReportSection {
            heading: "Safety Compliance".to_string(),
            content: format!(
                "Violations: {}, Near-misses: {}, Score: {:.3}",
                safety.total_violations(),
                safety.total_near_misses(),
                safety.safety_score()
            ),
            metrics: safety_metrics,
            status: safety_status,
        });

        // Cost breakdown
        let mut cost_metrics = HashMap::new();
        cost_metrics.insert("total_gpu_hours".to_string(), cost.total_gpu_hours());
        cost_metrics.insert("total_cost_usd".to_string(), cost.total_cost_usd());
        cost_metrics.insert("cost_per_reward_unit".to_string(), cost.cost_per_reward_unit());
        cost_metrics.insert("compute_efficiency".to_string(), cost.compute_efficiency());
        sections.push(ReportSection {
            heading: "Cost Breakdown".to_string(),
            content: format!(
                "GPU-hours: {:.1}, Total cost: ${:.2}, Efficiency: {:.4}",
                cost.total_gpu_hours(),
                cost.total_cost_usd(),
                cost.compute_efficiency()
            ),
            metrics: cost_metrics,
            status: ReportStatus::Healthy,
        });

        // Alert summary
        let mut alert_metrics = HashMap::new();
        alert_metrics.insert("total_alerts".to_string(), alerts.alert_count() as f64);
        let critical_count = alerts.alerts_by_severity(&AlertSeverity::Critical).len();
        let emergency_count = alerts.alerts_by_severity(&AlertSeverity::Emergency).len();
        alert_metrics.insert("critical_alerts".to_string(), critical_count as f64);
        alert_metrics.insert("emergency_alerts".to_string(), emergency_count as f64);
        let alert_status = if emergency_count > 0 {
            ReportStatus::Critical
        } else if critical_count > 0 {
            ReportStatus::Warning
        } else {
            ReportStatus::Healthy
        };
        sections.push(ReportSection {
            heading: "Alert Summary".to_string(),
            content: format!(
                "Total: {}, Critical: {}, Emergency: {}",
                alerts.alert_count(),
                critical_count,
                emergency_count
            ),
            metrics: alert_metrics,
            status: alert_status,
        });

        ObservabilityReport {
            title: "RL Observability Report".to_string(),
            generated_at: timestamp,
            sections,
        }
    }

    pub fn to_markdown(report: &ObservabilityReport) -> String {
        let mut md = String::with_capacity(2048);
        md.push_str(&format!("# {}\n\n", report.title));
        md.push_str(&format!("Generated at: {}\n\n", report.generated_at));
        for section in &report.sections {
            md.push_str(&format!("## {} [{}]\n\n", section.heading, section.status));
            md.push_str(&format!("{}\n\n", section.content));
            if !section.metrics.is_empty() {
                md.push_str("| Metric | Value |\n|--------|-------|\n");
                let mut keys: Vec<_> = section.metrics.keys().collect();
                keys.sort();
                for key in keys {
                    md.push_str(&format!(
                        "| {} | {:.4} |\n",
                        key,
                        section.metrics[key]
                    ));
                }
                md.push('\n');
            }
        }
        md
    }
}

// ── Metric Aggregation ─────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct SlidingWindowAggregator {
    pub window_size: usize,
    pub values: Vec<f64>,
}

impl SlidingWindowAggregator {
    pub fn new(window_size: usize) -> Self {
        Self {
            window_size,
            values: Vec::new(),
        }
    }

    pub fn push(&mut self, value: f64) {
        self.values.push(value);
        if self.values.len() > self.window_size {
            self.values.remove(0);
        }
    }

    pub fn mean(&self) -> f64 {
        if self.values.is_empty() {
            return 0.0;
        }
        self.values.iter().sum::<f64>() / self.values.len() as f64
    }

    pub fn min(&self) -> Option<f64> {
        self.values
            .iter()
            .copied()
            .fold(None, |acc: Option<f64>, v| match acc {
                None => Some(v),
                Some(a) => Some(if v < a { v } else { a }),
            })
    }

    pub fn max(&self) -> Option<f64> {
        self.values
            .iter()
            .copied()
            .fold(None, |acc: Option<f64>, v| match acc {
                None => Some(v),
                Some(a) => Some(if v > a { v } else { a }),
            })
    }

    pub fn std_dev(&self) -> f64 {
        if self.values.len() < 2 {
            return 0.0;
        }
        let m = self.mean();
        let var =
            self.values.iter().map(|v| (v - m).powi(2)).sum::<f64>() / (self.values.len() - 1) as f64;
        var.sqrt()
    }

    pub fn is_full(&self) -> bool {
        self.values.len() >= self.window_size
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
}

#[derive(Debug, Clone)]
pub struct EmaAggregator {
    pub alpha: f64,
    pub value: Option<f64>,
    pub count: u64,
}

impl EmaAggregator {
    pub fn new(alpha: f64) -> Self {
        Self {
            alpha,
            value: None,
            count: 0,
        }
    }

    pub fn push(&mut self, v: f64) {
        self.count += 1;
        self.value = Some(match self.value {
            None => v,
            Some(prev) => self.alpha * v + (1.0 - self.alpha) * prev,
        });
    }

    pub fn current(&self) -> Option<f64> {
        self.value
    }

    pub fn reset(&mut self) {
        self.value = None;
        self.count = 0;
    }
}

#[derive(Debug, Clone)]
pub struct PercentileTracker {
    pub values: Vec<f64>,
    pub sorted: bool,
}

impl PercentileTracker {
    pub fn new() -> Self {
        Self {
            values: Vec::new(),
            sorted: false,
        }
    }

    pub fn push(&mut self, v: f64) {
        self.values.push(v);
        self.sorted = false;
    }

    fn ensure_sorted(&mut self) {
        if !self.sorted {
            self.values
                .sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            self.sorted = true;
        }
    }

    pub fn percentile(&mut self, pct: f64) -> f64 {
        self.ensure_sorted();
        percentile_value(&self.values, pct)
    }

    pub fn p50(&mut self) -> f64 {
        self.percentile(50.0)
    }

    pub fn p95(&mut self) -> f64 {
        self.percentile(95.0)
    }

    pub fn p99(&mut self) -> f64 {
        self.percentile(99.0)
    }

    pub fn count(&self) -> usize {
        self.values.len()
    }
}

fn percentile_value(sorted_values: &[f64], pct: f64) -> f64 {
    if sorted_values.is_empty() {
        return 0.0;
    }
    let mut v = sorted_values.to_vec();
    v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let idx = ((pct / 100.0) * (v.len() as f64 - 1.0))
        .round()
        .max(0.0) as usize;
    v[idx.min(v.len() - 1)]
}

// ── Retention & Downsampling ───────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct RetentionPolicy {
    pub raw_retention_secs: u64,
    pub downsample_interval_secs: u64,
    pub strategy: DownsampleStrategy,
}

impl Default for RetentionPolicy {
    fn default() -> Self {
        Self {
            raw_retention_secs: 86400,       // 24 hours
            downsample_interval_secs: 3600,  // 1 hour
            strategy: DownsampleStrategy::Average,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RetentionManager {
    pub policy: RetentionPolicy,
}

impl RetentionManager {
    pub fn new(policy: RetentionPolicy) -> Self {
        Self { policy }
    }

    pub fn apply(&self, series: &MetricSeries, now: u64) -> MetricSeries {
        let cutoff = now.saturating_sub(self.policy.raw_retention_secs);
        let recent: Vec<&MetricPoint> = series.points.iter().filter(|p| p.timestamp >= cutoff).collect();
        let old: Vec<&MetricPoint> = series.points.iter().filter(|p| p.timestamp < cutoff).collect();

        let mut result = MetricSeries::new(&series.name, series.kind.clone());
        result.metadata = series.metadata.clone();

        // Downsample old data
        if !old.is_empty() {
            let mut buckets: HashMap<u64, Vec<f64>> = HashMap::new();
            for p in &old {
                let bucket = p.timestamp / self.policy.downsample_interval_secs
                    * self.policy.downsample_interval_secs;
                buckets.entry(bucket).or_default().push(p.value);
            }
            let mut bucket_keys: Vec<u64> = buckets.keys().copied().collect();
            bucket_keys.sort();
            for key in bucket_keys {
                let vals = &buckets[&key];
                let value = self.aggregate(vals);
                result.push(key, value);
            }
        }

        // Keep recent raw
        for p in recent {
            result.push(p.timestamp, p.value);
        }

        result
    }

    fn aggregate(&self, values: &[f64]) -> f64 {
        if values.is_empty() {
            return 0.0;
        }
        match self.policy.strategy {
            DownsampleStrategy::Average => values.iter().sum::<f64>() / values.len() as f64,
            DownsampleStrategy::Min => values
                .iter()
                .copied()
                .fold(f64::INFINITY, f64::min),
            DownsampleStrategy::Max => values
                .iter()
                .copied()
                .fold(f64::NEG_INFINITY, f64::max),
            DownsampleStrategy::Last => *values.last().unwrap(),
            DownsampleStrategy::Median => {
                let mut sorted = values.to_vec();
                sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                sorted[sorted.len() / 2]
            }
        }
    }

    pub fn should_downsample(&self, oldest_timestamp: u64, now: u64) -> bool {
        now.saturating_sub(oldest_timestamp) > self.policy.raw_retention_secs
    }
}

// ── Correlation Analysis ───────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct CorrelationResult {
    pub metric_a: String,
    pub metric_b: String,
    pub pearson_r: f64,
    pub p_value_approx: f64,
    pub significant: bool,
    pub relationship: String,
}

/// Compute Pearson correlation between two aligned metric series.
pub fn pearson_correlation(a: &[f64], b: &[f64]) -> f64 {
    let n = a.len().min(b.len());
    if n < 3 {
        return 0.0;
    }
    let mean_a = a[..n].iter().sum::<f64>() / n as f64;
    let mean_b = b[..n].iter().sum::<f64>() / n as f64;
    let mut cov = 0.0;
    let mut var_a = 0.0;
    let mut var_b = 0.0;
    for i in 0..n {
        let da = a[i] - mean_a;
        let db = b[i] - mean_b;
        cov += da * db;
        var_a += da * da;
        var_b += db * db;
    }
    let denom = (var_a * var_b).sqrt();
    if denom == 0.0 {
        return 0.0;
    }
    cov / denom
}

/// Approximate p-value for Pearson r using t-distribution approximation.
fn approx_p_value(r: f64, n: usize) -> f64 {
    if n < 4 {
        return 1.0;
    }
    let t = r * ((n as f64 - 2.0) / (1.0 - r * r).max(1e-15)).sqrt();
    // Rough approximation using normal distribution tail
    let z = t.abs();
    let p = (-0.5 * z * z).exp() * 0.3989; // quick approx
    (2.0 * p).min(1.0)
}

#[derive(Debug, Clone)]
pub struct CorrelationAnalyzer {
    pub series: HashMap<String, Vec<f64>>,
    pub results: Vec<CorrelationResult>,
}

impl CorrelationAnalyzer {
    pub fn new() -> Self {
        Self {
            series: HashMap::new(),
            results: Vec::new(),
        }
    }

    pub fn add_series(&mut self, name: &str, values: Vec<f64>) {
        self.series.insert(name.to_string(), values);
    }

    pub fn compute_all(&mut self) {
        self.results.clear();
        let keys: Vec<String> = self.series.keys().cloned().collect();
        for i in 0..keys.len() {
            for j in (i + 1)..keys.len() {
                let a = &self.series[&keys[i]];
                let b = &self.series[&keys[j]];
                let n = a.len().min(b.len());
                let r = pearson_correlation(a, b);
                let p = approx_p_value(r, n);
                let relationship = if r > 0.7 {
                    "Strong positive".to_string()
                } else if r > 0.3 {
                    "Moderate positive".to_string()
                } else if r > -0.3 {
                    "Weak/none".to_string()
                } else if r > -0.7 {
                    "Moderate negative".to_string()
                } else {
                    "Strong negative".to_string()
                };
                self.results.push(CorrelationResult {
                    metric_a: keys[i].clone(),
                    metric_b: keys[j].clone(),
                    pearson_r: r,
                    p_value_approx: p,
                    significant: p < 0.05,
                    relationship,
                });
            }
        }
    }

    pub fn significant_correlations(&self) -> Vec<&CorrelationResult> {
        self.results.iter().filter(|r| r.significant).collect()
    }

    pub fn strongest_correlation(&self) -> Option<&CorrelationResult> {
        self.results
            .iter()
            .max_by(|a, b| {
                a.pearson_r
                    .abs()
                    .partial_cmp(&b.pearson_r.abs())
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    }

    pub fn correlations_for(&self, metric: &str) -> Vec<&CorrelationResult> {
        self.results
            .iter()
            .filter(|r| r.metric_a == metric || r.metric_b == metric)
            .collect()
    }
}

// ── Top-level Observability Engine ─────────────────────────────────────

#[derive(Debug, Clone)]
pub struct RlObservabilityConfig {
    pub reward_drift_enabled: bool,
    pub observation_monitoring_enabled: bool,
    pub action_monitoring_enabled: bool,
    pub safety_monitoring_enabled: bool,
    pub exploration_tracking_enabled: bool,
    pub multi_agent_tracing_enabled: bool,
    pub training_health_enabled: bool,
    pub alert_system_enabled: bool,
    pub cost_tracking_enabled: bool,
    pub anomaly_detection_enabled: bool,
    pub correlation_analysis_enabled: bool,
    pub retention_policy: RetentionPolicy,
}

impl Default for RlObservabilityConfig {
    fn default() -> Self {
        Self {
            reward_drift_enabled: true,
            observation_monitoring_enabled: true,
            action_monitoring_enabled: true,
            safety_monitoring_enabled: true,
            exploration_tracking_enabled: true,
            multi_agent_tracing_enabled: true,
            training_health_enabled: true,
            alert_system_enabled: true,
            cost_tracking_enabled: true,
            anomaly_detection_enabled: true,
            correlation_analysis_enabled: true,
            retention_policy: RetentionPolicy::default(),
        }
    }
}

#[derive(Debug)]
pub struct RlObservabilityEngine {
    pub config: RlObservabilityConfig,
    pub metric_store: HashMap<String, MetricSeries>,
    pub alert_manager: AlertManager,
    pub dashboard: DashboardProvider,
    pub training_health: TrainingHealthMonitor,
    pub safety_monitor: SafetyMonitor,
    pub cost_tracker: CostTracker,
    pub exploration_tracker: ExplorationTracker,
    pub trajectory_analyzer: TrajectoryAnalyzer,
    pub correlation_analyzer: CorrelationAnalyzer,
    pub retention_manager: RetentionManager,
}

impl RlObservabilityEngine {
    pub fn new(config: RlObservabilityConfig) -> Self {
        let retention_manager = RetentionManager::new(config.retention_policy.clone());
        Self {
            config,
            metric_store: HashMap::new(),
            alert_manager: AlertManager::new(),
            dashboard: DashboardProvider::new(),
            training_health: TrainingHealthMonitor::new(),
            safety_monitor: SafetyMonitor::new(),
            cost_tracker: CostTracker::new(2.50),
            exploration_tracker: ExplorationTracker::new(10, 10),
            trajectory_analyzer: TrajectoryAnalyzer::new(),
            correlation_analyzer: CorrelationAnalyzer::new(),
            retention_manager,
        }
    }

    pub fn record_metric(&mut self, name: &str, kind: MetricKind, timestamp: u64, value: f64) {
        let series = self
            .metric_store
            .entry(name.to_string())
            .or_insert_with(|| MetricSeries::new(name, kind));
        series.push(timestamp, value);
    }

    pub fn get_metric(&self, name: &str) -> Option<&MetricSeries> {
        self.metric_store.get(name)
    }

    pub fn generate_report(&self, timestamp: u64) -> ObservabilityReport {
        ReportGenerator::generate(
            &self.training_health,
            &self.safety_monitor,
            &self.cost_tracker,
            &self.alert_manager,
            timestamp,
        )
    }

    pub fn apply_retention(&mut self, now: u64) {
        let names: Vec<String> = self.metric_store.keys().cloned().collect();
        for name in names {
            if let Some(series) = self.metric_store.get(&name) {
                let new_series = self.retention_manager.apply(series, now);
                self.metric_store.insert(name, new_series);
            }
        }
    }

    pub fn run_correlation_analysis(&mut self) {
        self.correlation_analyzer.series.clear();
        for (name, series) in &self.metric_store {
            self.correlation_analyzer
                .add_series(name, series.values());
        }
        self.correlation_analyzer.compute_all();
    }

    pub fn metric_count(&self) -> usize {
        self.metric_store.len()
    }

    pub fn total_data_points(&self) -> usize {
        self.metric_store.values().map(|s| s.len()).sum()
    }
}

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // -- Enum tests --

    #[test]
    fn test_drift_type_label() {
        assert_eq!(DriftType::MeanShift.label(), "Mean Shift");
        assert_eq!(DriftType::VarianceChange.label(), "Variance Change");
        assert_eq!(DriftType::DistributionalShift.label(), "Distributional Shift");
        assert_eq!(DriftType::TrendDrift.label(), "Trend Drift");
        assert_eq!(DriftType::SeasonalDrift.label(), "Seasonal Drift");
    }

    #[test]
    fn test_drift_type_severity_weight() {
        assert!(DriftType::DistributionalShift.severity_weight() > DriftType::SeasonalDrift.severity_weight());
        assert_eq!(DriftType::MeanShift.severity_weight(), 0.8);
    }

    #[test]
    fn test_drift_type_display() {
        assert_eq!(format!("{}", DriftType::MeanShift), "Mean Shift");
    }

    #[test]
    fn test_alert_severity_priority() {
        assert!(AlertSeverity::Emergency.priority() > AlertSeverity::Critical.priority());
        assert!(AlertSeverity::Critical.priority() > AlertSeverity::Warning.priority());
        assert!(AlertSeverity::Warning.priority() > AlertSeverity::Info.priority());
    }

    #[test]
    fn test_alert_severity_display() {
        assert_eq!(format!("{}", AlertSeverity::Critical), "Critical");
    }

    #[test]
    fn test_alert_type_labels() {
        assert_eq!(AlertType::RewardDrop.label(), "Reward Drop");
        assert_eq!(AlertType::SafetyBreach.label(), "Safety Breach");
        assert_eq!(AlertType::GradientExplosion.label(), "Gradient Explosion");
    }

    #[test]
    fn test_alert_type_default_severity() {
        assert_eq!(AlertType::SafetyBreach.default_severity(), AlertSeverity::Emergency);
        assert_eq!(AlertType::RewardDrop.default_severity(), AlertSeverity::Critical);
        assert_eq!(AlertType::CostOverrun.default_severity(), AlertSeverity::Info);
    }

    #[test]
    fn test_metric_kind_label() {
        assert_eq!(MetricKind::Reward.label(), "Reward");
        assert_eq!(MetricKind::GpuUtilization.label(), "GPU Utilization");
        assert_eq!(MetricKind::Custom("MyMetric".to_string()).label(), "MyMetric");
    }

    #[test]
    fn test_aggregation_type_label() {
        assert_eq!(AggregationType::SlidingWindow.label(), "Sliding Window");
        assert_eq!(AggregationType::ExponentialMovingAverage.label(), "EMA");
    }

    #[test]
    fn test_downsample_strategy_display() {
        assert_eq!(format!("{}", DownsampleStrategy::Average), "Average");
        assert_eq!(format!("{}", DownsampleStrategy::Median), "Median");
    }

    #[test]
    fn test_dashboard_widget_kind_label() {
        assert_eq!(DashboardWidgetKind::TimeSeries.label(), "Time Series");
        assert_eq!(DashboardWidgetKind::Heatmap.label(), "Heatmap");
    }

    #[test]
    fn test_constraint_kind_label() {
        assert_eq!(ConstraintKind::SafetyBound.label(), "Safety Bound");
        assert_eq!(ConstraintKind::ResourceLimit.label(), "Resource Limit");
    }

    // -- MetricSeries tests --

    #[test]
    fn test_metric_series_basic() {
        let mut s = MetricSeries::new("reward", MetricKind::Reward);
        assert!(s.is_empty());
        s.push(1, 10.0);
        s.push(2, 20.0);
        s.push(3, 30.0);
        assert_eq!(s.len(), 3);
        assert_eq!(s.mean(), 20.0);
        assert_eq!(s.last_value(), Some(30.0));
    }

    #[test]
    fn test_metric_series_variance() {
        let mut s = MetricSeries::new("test", MetricKind::Reward);
        for v in &[2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0] {
            s.push(0, *v);
        }
        let var = s.variance();
        assert!((var - 4.571).abs() < 0.01);
    }

    #[test]
    fn test_metric_series_min_max() {
        let mut s = MetricSeries::new("test", MetricKind::Reward);
        s.push(0, 5.0);
        s.push(1, 2.0);
        s.push(2, 8.0);
        assert_eq!(s.min_value(), Some(2.0));
        assert_eq!(s.max_value(), Some(8.0));
    }

    #[test]
    fn test_metric_series_empty() {
        let s = MetricSeries::new("empty", MetricKind::Reward);
        assert_eq!(s.mean(), 0.0);
        assert_eq!(s.variance(), 0.0);
        assert_eq!(s.min_value(), None);
        assert_eq!(s.max_value(), None);
    }

    #[test]
    fn test_metric_series_labeled() {
        let mut s = MetricSeries::new("test", MetricKind::Reward);
        let mut labels = HashMap::new();
        labels.insert("agent".to_string(), "a1".to_string());
        s.push_labeled(1, 5.0, labels);
        assert_eq!(s.points[0].labels.get("agent"), Some(&"a1".to_string()));
    }

    // -- CUSUM tests --

    #[test]
    fn test_cusum_no_drift() {
        let mut cusum = CusumDetector::new(0.0, 5.0, 1.0);
        for i in 0..10 {
            assert!(!cusum.update(0.1, i));
        }
        assert!(!cusum.detected);
    }

    #[test]
    fn test_cusum_detects_drift() {
        let mut cusum = CusumDetector::new(0.0, 3.0, 1.0);
        let mut detected = false;
        for i in 0..50 {
            if cusum.update(2.0, i) {
                detected = true;
                break;
            }
        }
        assert!(detected);
        assert!(cusum.detected);
        assert!(cusum.detection_index.is_some());
    }

    #[test]
    fn test_cusum_reset() {
        let mut cusum = CusumDetector::new(0.0, 3.0, 1.0);
        for i in 0..50 {
            cusum.update(2.0, i);
        }
        cusum.reset();
        assert!(!cusum.detected);
        assert_eq!(cusum.s_pos, 0.0);
        assert_eq!(cusum.s_neg, 0.0);
    }

    // -- EWMA tests --

    #[test]
    fn test_ewma_no_drift() {
        let mut ewma = EwmaDetector::new(0.2, 3.0, 10.0, 1.0);
        for _ in 0..20 {
            assert!(!ewma.update(10.05));
        }
    }

    #[test]
    fn test_ewma_detects_drift() {
        let mut ewma = EwmaDetector::new(0.2, 3.0, 0.0, 1.0);
        let mut detected = false;
        for _ in 0..100 {
            if ewma.update(5.0) {
                detected = true;
                break;
            }
        }
        assert!(detected);
    }

    #[test]
    fn test_ewma_reset() {
        let mut ewma = EwmaDetector::new(0.2, 3.0, 10.0, 1.0);
        ewma.update(15.0);
        ewma.reset();
        assert_eq!(ewma.ewma, 10.0);
        assert_eq!(ewma.n, 0);
    }

    // -- KS test --

    #[test]
    fn test_ks_test_same_distribution() {
        let a: Vec<f64> = (0..100).map(|i| i as f64 * 0.01).collect();
        let b: Vec<f64> = (0..100).map(|i| i as f64 * 0.01 + 0.001).collect();
        let result = ks_test(&a, &b);
        assert!(!result.reject_null);
    }

    #[test]
    fn test_ks_test_different_distribution() {
        let a: Vec<f64> = (0..100).map(|i| i as f64 * 0.01).collect();
        let b: Vec<f64> = (0..100).map(|i| i as f64 * 0.01 + 0.5).collect();
        let result = ks_test(&a, &b);
        assert!(result.statistic > 0.0);
    }

    #[test]
    fn test_ks_test_empty() {
        let result = ks_test(&[], &[1.0, 2.0]);
        assert_eq!(result.statistic, 0.0);
        assert_eq!(result.p_value, 1.0);
    }

    // -- Reward Drift Detector --

    #[test]
    fn test_reward_drift_detector_stable() {
        let baseline: Vec<f64> = (0..50).map(|i| 1.0 + (i as f64 % 5.0) * 0.02).collect();
        let mut detector = RewardDriftDetector::new(&baseline, 5.0, 0.2);
        for i in 0..30 {
            let reports = detector.observe(1.04, i);
            // Should not flag mean shift for tiny deviation
            let mean_shifts: Vec<_> = reports
                .iter()
                .filter(|r| r.drift_type == DriftType::MeanShift)
                .collect();
            assert!(mean_shifts.is_empty(), "Unexpected mean shift at i={}", i);
        }
    }

    #[test]
    fn test_reward_drift_detector_detects_shift() {
        let baseline: Vec<f64> = (0..50).map(|_| 0.0).collect();
        let mut detector = RewardDriftDetector::new(&baseline, 3.0, 0.2);
        let mut detected = false;
        for i in 0..200 {
            let reports = detector.observe(5.0, i);
            if !reports.is_empty() {
                detected = true;
                break;
            }
        }
        assert!(detected);
    }

    #[test]
    fn test_reward_drift_detector_reset() {
        let baseline: Vec<f64> = vec![1.0; 50];
        let mut detector = RewardDriftDetector::new(&baseline, 5.0, 0.2);
        detector.observe(10.0, 0);
        detector.reset();
        assert!(detector.recent_window.is_empty());
        assert!(!detector.cusum.detected);
    }

    // -- Observation Monitor --

    #[test]
    fn test_observation_monitor_basic() {
        let mut mon = ObservationMonitor::new(3);
        mon.set_baseline(vec![0.0, 0.0, 0.0], vec![1.0, 1.0, 1.0]);
        mon.observe(&[0.1, 0.2, 0.3]);
        assert_eq!(mon.total_observations, 1);
        assert_eq!(mon.coverage_ratio(), 1.0);
    }

    #[test]
    fn test_observation_monitor_drift_detection() {
        let mut mon = ObservationMonitor::new(2);
        mon.set_baseline(vec![0.0, 0.0], vec![1.0, 1.0]);
        for _ in 0..50 {
            mon.observe(&[5.0, 0.0]);
        }
        let drifted = mon.drifted_dimensions();
        assert!(drifted.contains(&0));
        assert!(!drifted.contains(&1));
    }

    #[test]
    fn test_observation_monitor_per_dim_stats() {
        let mut mon = ObservationMonitor::new(2);
        mon.observe(&[3.0, 7.0]);
        let stats = mon.per_dimension_stats();
        assert_eq!(stats.len(), 2);
        assert_eq!(stats[0].mean, 3.0);
        assert_eq!(stats[1].mean, 7.0);
    }

    #[test]
    fn test_observation_monitor_reset() {
        let mut mon = ObservationMonitor::new(2);
        mon.observe(&[1.0, 2.0]);
        mon.reset();
        assert_eq!(mon.total_observations, 0);
        assert_eq!(mon.coverage_ratio(), 0.0);
    }

    // -- Action Monitor --

    #[test]
    fn test_action_monitor_basic() {
        let mut am = ActionMonitor::new(4);
        am.observe(0);
        am.observe(1);
        am.observe(2);
        am.observe(3);
        assert_eq!(am.total_actions, 4);
        let dist = am.distribution();
        assert_eq!(dist, vec![0.25, 0.25, 0.25, 0.25]);
    }

    #[test]
    fn test_action_monitor_entropy() {
        let mut am = ActionMonitor::new(4);
        for _ in 0..100 {
            am.observe(0);
            am.observe(1);
            am.observe(2);
            am.observe(3);
        }
        let ent = am.entropy();
        let max_ent = am.max_entropy();
        assert!((ent - max_ent).abs() < 0.01);
    }

    #[test]
    fn test_action_monitor_skewed() {
        let mut am = ActionMonitor::new(3);
        for _ in 0..100 {
            am.observe(0);
        }
        am.observe(1);
        let dist = am.distribution();
        assert!(dist[0] > 0.9);
        assert!(am.entropy() < am.max_entropy());
    }

    #[test]
    fn test_action_monitor_most_least_frequent() {
        let mut am = ActionMonitor::new(3);
        am.observe(0);
        am.observe(0);
        am.observe(1);
        assert_eq!(am.most_frequent_action(), Some(0));
        assert_eq!(am.least_frequent_action(), Some(2));
    }

    #[test]
    fn test_action_monitor_kl_divergence() {
        let mut am = ActionMonitor::new(2);
        am.set_baseline(vec![0.5, 0.5]);
        for _ in 0..100 {
            am.observe(0);
        }
        assert!(am.kl_divergence_from_baseline() > 0.0);
        assert!(am.behavioral_change_detected(0.1));
    }

    #[test]
    fn test_action_monitor_reset() {
        let mut am = ActionMonitor::new(3);
        am.observe(0);
        am.reset();
        assert_eq!(am.total_actions, 0);
    }

    // -- Safety Monitor --

    #[test]
    fn test_safety_constraint_upper_bound() {
        let c = SafetyConstraint::new_upper("c1", "Max Speed", ConstraintKind::SafetyBound, 100.0);
        let result = c.check(50.0);
        assert!(!result.violated);
        assert!(!result.near_miss);

        let result = c.check(95.0);
        assert!(!result.violated);
        assert!(result.near_miss);

        let result = c.check(105.0);
        assert!(result.violated);
    }

    #[test]
    fn test_safety_constraint_range() {
        let c = SafetyConstraint::new_range("c2", "Temp", ConstraintKind::SafetyBound, -10.0, 50.0);
        let result = c.check(25.0);
        assert!(!result.violated);

        let result = c.check(-15.0);
        assert!(result.violated);

        let result = c.check(55.0);
        assert!(result.violated);
    }

    #[test]
    fn test_safety_monitor_full_flow() {
        let mut sm = SafetyMonitor::new();
        sm.add_constraint(SafetyConstraint::new_upper(
            "speed",
            "Max Speed",
            ConstraintKind::SafetyBound,
            100.0,
        ));
        let mut vals = HashMap::new();
        vals.insert("speed".to_string(), 50.0);
        let results = sm.check_all(&vals, 1);
        assert!(!results[0].violated);
        assert_eq!(sm.total_violations(), 0);

        vals.insert("speed".to_string(), 110.0);
        let results = sm.check_all(&vals, 2);
        assert!(results[0].violated);
        assert_eq!(sm.total_violations(), 1);
    }

    #[test]
    fn test_safety_monitor_safety_score() {
        let mut sm = SafetyMonitor::new();
        sm.add_constraint(SafetyConstraint::new_upper(
            "x",
            "X",
            ConstraintKind::SafetyBound,
            10.0,
        ));
        let mut vals = HashMap::new();
        vals.insert("x".to_string(), 5.0);
        sm.check_all(&vals, 1);
        assert_eq!(sm.safety_score(), 1.0);
    }

    #[test]
    fn test_safety_monitor_violation_rate() {
        let mut sm = SafetyMonitor::new();
        sm.add_constraint(SafetyConstraint::new_upper(
            "x",
            "X",
            ConstraintKind::SafetyBound,
            10.0,
        ));
        let mut vals = HashMap::new();
        vals.insert("x".to_string(), 15.0);
        sm.check_all(&vals, 1);
        sm.check_all(&vals, 2);
        assert_eq!(sm.violation_rate("x"), 1.0);
    }

    #[test]
    fn test_safety_monitor_near_misses() {
        let mut sm = SafetyMonitor::new();
        sm.add_constraint(SafetyConstraint::new_upper(
            "y",
            "Y",
            ConstraintKind::SafetyBound,
            100.0,
        ));
        let mut vals = HashMap::new();
        vals.insert("y".to_string(), 95.0);
        sm.check_all(&vals, 1);
        assert_eq!(sm.total_near_misses(), 1);
    }

    // -- Exploration Tracker --

    #[test]
    fn test_exploration_basic() {
        let mut et = ExplorationTracker::new(5, 5);
        et.visit_state("s1");
        et.visit_state("s2");
        et.visit_state("s1");
        assert_eq!(et.state_coverage(), 2);
        assert_eq!(et.total_visits, 3);
    }

    #[test]
    fn test_exploration_entropy() {
        let mut et = ExplorationTracker::new(5, 5);
        et.visit_state("a");
        et.visit_state("b");
        et.visit_state("c");
        et.visit_state("d");
        let ent = et.state_coverage_entropy();
        // Uniform visits => max entropy for 4 states
        assert!((ent - (4.0_f64).ln()).abs() < 0.01);
    }

    #[test]
    fn test_exploration_efficiency() {
        let mut et = ExplorationTracker::new(5, 5);
        // All equal visits => efficiency = 1.0
        for _ in 0..10 {
            et.visit_state("a");
            et.visit_state("b");
        }
        assert!((et.exploration_efficiency() - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_exploration_novelty() {
        let mut et = ExplorationTracker::new(5, 5);
        et.visit_state("new");
        assert_eq!(et.mean_novelty(), 1.0);
        et.visit_state("new");
        assert!(et.mean_novelty() < 1.0);
    }

    #[test]
    fn test_exploration_heatmap() {
        let mut et = ExplorationTracker::new(3, 3);
        et.visit_grid(0, 0);
        et.visit_grid(1, 1);
        et.visit_grid(2, 2);
        assert!((et.heatmap_coverage() - 3.0 / 9.0).abs() < 0.01);
    }

    #[test]
    fn test_exploration_top_visited() {
        let mut et = ExplorationTracker::new(5, 5);
        for _ in 0..10 {
            et.visit_state("popular");
        }
        et.visit_state("rare");
        let top = et.top_visited_states(1);
        assert_eq!(top[0].0, "popular");
    }

    #[test]
    fn test_exploration_reset() {
        let mut et = ExplorationTracker::new(3, 3);
        et.visit_state("s1");
        et.visit_grid(0, 0);
        et.reset();
        assert_eq!(et.state_coverage(), 0);
        assert_eq!(et.total_visits, 0);
    }

    // -- Multi-Agent Tracer --

    #[test]
    fn test_multi_agent_basic() {
        let mut mat = MultiAgentTracer::new();
        mat.register_agent("a1");
        mat.register_agent("a2");
        mat.record_reward("a1", 10.0);
        mat.record_reward("a2", 20.0);
        assert_eq!(mat.social_welfare(), 30.0);
    }

    #[test]
    fn test_multi_agent_gini() {
        let mut mat = MultiAgentTracer::new();
        mat.register_agent("a1");
        mat.register_agent("a2");
        mat.record_reward("a1", 10.0);
        mat.record_reward("a2", 10.0);
        assert!((mat.gini_coefficient() - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_multi_agent_communication() {
        let mut mat = MultiAgentTracer::new();
        mat.register_agent("a1");
        mat.register_agent("a2");
        mat.record_communication("a1", "a2");
        mat.record_communication("a1", "a2");
        assert_eq!(mat.agents["a1"].messages_sent, 2);
        assert_eq!(mat.agents["a2"].messages_received, 2);
    }

    #[test]
    fn test_multi_agent_coalition() {
        let mut mat = MultiAgentTracer::new();
        mat.register_agent("a1");
        mat.register_agent("a2");
        mat.set_coalition("a1", "team1");
        mat.set_coalition("a2", "team1");
        mat.record_reward("a1", 5.0);
        mat.record_reward("a2", 7.0);
        let cr = mat.coalition_rewards();
        assert_eq!(*cr.get("team1").unwrap(), 12.0);
    }

    #[test]
    fn test_multi_agent_communication_density() {
        let mut mat = MultiAgentTracer::new();
        mat.register_agent("a");
        mat.register_agent("b");
        mat.record_communication("a", "b");
        // 2 agents, 2 possible directed edges, 1 used
        assert!((mat.communication_density() - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_multi_agent_top_communicators() {
        let mut mat = MultiAgentTracer::new();
        mat.register_agent("a");
        mat.register_agent("b");
        mat.register_agent("c");
        mat.record_communication("a", "b");
        mat.record_communication("a", "b");
        mat.record_communication("a", "b");
        mat.record_communication("a", "c");
        mat.record_communication("a", "c");
        // "a" has 5 sent + 0 received = 5, "b" has 0+3=3, "c" has 0+2=2
        let top = mat.top_communicators(1);
        assert_eq!(top[0].0, "a");
    }

    #[test]
    fn test_multi_agent_reward_decomposition() {
        let mut mat = MultiAgentTracer::new();
        mat.register_agent("x");
        mat.register_agent("y");
        mat.record_reward("x", 30.0);
        mat.record_reward("y", 70.0);
        let decomp = mat.per_agent_reward_decomposition();
        assert_eq!(decomp.len(), 2);
    }

    // -- Training Health --

    #[test]
    fn test_training_health_basic() {
        let mut th = TrainingHealthMonitor::new();
        th.record_loss(1.0);
        th.record_loss(0.5);
        th.record_gradient_norm(0.1);
        assert_eq!(th.latest_loss(), Some(0.5));
        assert_eq!(th.latest_gradient_norm(), Some(0.1));
    }

    #[test]
    fn test_training_health_gradient_explosion() {
        let mut th = TrainingHealthMonitor::new();
        th.record_gradient_norm(200.0);
        assert!(th.is_gradient_exploding());
    }

    #[test]
    fn test_training_health_gradient_vanishing() {
        let mut th = TrainingHealthMonitor::new();
        th.record_gradient_norm(1e-10);
        assert!(th.is_gradient_vanishing());
    }

    #[test]
    fn test_training_health_loss_converged() {
        let mut th = TrainingHealthMonitor::new();
        th.convergence_window = 10;
        for _ in 0..20 {
            th.record_loss(0.5);
        }
        assert!(th.is_loss_converged());
    }

    #[test]
    fn test_training_health_loss_diverging() {
        let mut th = TrainingHealthMonitor::new();
        for i in 0..10 {
            th.record_loss(1.0 + (i as f64) * 2.0);
        }
        assert!(th.is_loss_diverging());
    }

    #[test]
    fn test_training_health_gpu_utilization() {
        let mut th = TrainingHealthMonitor::new();
        th.record_gpu_utilization(80.0);
        th.record_gpu_utilization(90.0);
        assert_eq!(th.mean_gpu_utilization(), 85.0);
    }

    #[test]
    fn test_training_health_progress() {
        let mut th = TrainingHealthMonitor::new();
        th.record_loss(1.0);
        th.record_loss(0.5);
        assert!((th.training_progress_pct() - 50.0).abs() < 0.01);
    }

    #[test]
    fn test_training_health_summary() {
        let mut th = TrainingHealthMonitor::new();
        th.record_loss(1.0);
        th.record_gradient_norm(0.1);
        th.record_gpu_utilization(75.0);
        let summary = th.health_summary();
        assert_eq!(summary.total_epochs, 1);
        assert!(!summary.gradient_exploding);
    }

    // -- Alert System --

    #[test]
    fn test_alert_rule_creation() {
        let rule = AlertRule::new("r1", AlertType::RewardDrop, 0.5)
            .with_severity(AlertSeverity::Critical)
            .with_cooldown(60)
            .with_description("Reward dropped");
        assert_eq!(rule.id, "r1");
        assert_eq!(rule.threshold, 0.5);
        assert_eq!(rule.cooldown_secs, 60);
    }

    #[test]
    fn test_alert_rule_can_fire() {
        let mut rule = AlertRule::new("r1", AlertType::RewardDrop, 0.5);
        assert!(rule.can_fire(100));
        rule.last_triggered = Some(100);
        rule.cooldown_secs = 300;
        assert!(!rule.can_fire(200));
        assert!(rule.can_fire(500));
    }

    #[test]
    fn test_alert_manager_fires() {
        let mut am = AlertManager::new();
        am.add_rule(AlertRule::new("r1", AlertType::GradientExplosion, 50.0));
        let alerts = am.check_and_fire(&AlertType::GradientExplosion, 100.0, 1);
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].value, 100.0);
    }

    #[test]
    fn test_alert_manager_cooldown() {
        let mut am = AlertManager::new();
        am.add_rule(AlertRule::new("r1", AlertType::GradientExplosion, 50.0).with_cooldown(100));
        am.check_and_fire(&AlertType::GradientExplosion, 100.0, 1);
        let alerts = am.check_and_fire(&AlertType::GradientExplosion, 100.0, 50);
        assert!(alerts.is_empty());
    }

    #[test]
    fn test_alert_manager_recent() {
        let mut am = AlertManager::new();
        am.add_rule(AlertRule::new("r1", AlertType::GradientExplosion, 50.0).with_cooldown(0));
        am.check_and_fire(&AlertType::GradientExplosion, 100.0, 1);
        am.check_and_fire(&AlertType::GradientExplosion, 200.0, 2);
        let recent = am.recent_alerts(1);
        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].value, 200.0);
    }

    #[test]
    fn test_alert_manager_by_severity() {
        let mut am = AlertManager::new();
        am.add_rule(
            AlertRule::new("r1", AlertType::GradientExplosion, 50.0)
                .with_severity(AlertSeverity::Emergency),
        );
        am.check_and_fire(&AlertType::GradientExplosion, 100.0, 1);
        assert_eq!(am.alerts_by_severity(&AlertSeverity::Emergency).len(), 1);
        assert_eq!(am.alerts_by_severity(&AlertSeverity::Info).len(), 0);
    }

    #[test]
    fn test_alert_manager_clear() {
        let mut am = AlertManager::new();
        am.add_rule(AlertRule::new("r1", AlertType::GradientExplosion, 50.0));
        am.check_and_fire(&AlertType::GradientExplosion, 100.0, 1);
        am.clear();
        assert_eq!(am.alert_count(), 0);
    }

    // -- Dashboard --

    #[test]
    fn test_dashboard_add_widgets() {
        let mut dp = DashboardProvider::new();
        dp.add_time_series("ts1", "Loss", vec![(1, 0.5), (2, 0.3)]);
        dp.add_histogram("h1", "Actions", vec![("a0".to_string(), 50.0)]);
        dp.add_gauge("g1", "Safety", 0.95, 0.0, 1.0);
        assert_eq!(dp.widget_count(), 3);
    }

    #[test]
    fn test_dashboard_get_widget() {
        let mut dp = DashboardProvider::new();
        dp.add_time_series("ts1", "Loss", vec![(1, 0.5)]);
        assert!(dp.get_widget("ts1").is_some());
        assert!(dp.get_widget("nonexistent").is_none());
    }

    #[test]
    fn test_dashboard_from_metric_series() {
        let mut s = MetricSeries::new("reward", MetricKind::Reward);
        s.push(1, 10.0);
        s.push(2, 20.0);
        let widget = DashboardProvider::from_metric_series(&s);
        assert_eq!(widget.id, "reward");
    }

    #[test]
    fn test_dashboard_from_exploration() {
        let mut et = ExplorationTracker::new(3, 3);
        et.visit_grid(0, 0);
        let widget = DashboardProvider::from_exploration_heatmap(&et);
        assert_eq!(widget.title, "State Exploration Heatmap");
    }

    #[test]
    fn test_dashboard_add_table() {
        let mut dp = DashboardProvider::new();
        dp.add_table(
            "t1",
            "Agents",
            vec!["Name".to_string(), "Reward".to_string()],
            vec![vec!["a1".to_string(), "10.0".to_string()]],
        );
        assert_eq!(dp.widget_count(), 1);
    }

    #[test]
    fn test_dashboard_add_heatmap() {
        let mut dp = DashboardProvider::new();
        dp.add_heatmap("hm1", "Coverage", 2, 2, vec![vec![1.0, 2.0], vec![3.0, 4.0]]);
        let w = dp.get_widget("hm1").unwrap();
        assert_eq!(w.kind, DashboardWidgetKind::Heatmap);
    }

    // -- Policy Diff --

    #[test]
    fn test_policy_diff_identical() {
        let a = PolicySnapshot::new("v1", 1, vec![0.5, 0.5]);
        let b = PolicySnapshot::new("v2", 2, vec![0.5, 0.5]);
        let diff = compute_policy_diff(&a, &b);
        assert_eq!(diff.max_action_change, 0.0);
        assert!(diff.kl_divergence < 0.001);
    }

    #[test]
    fn test_policy_diff_changed() {
        let a = PolicySnapshot::new("v1", 1, vec![0.9, 0.1]);
        let b = PolicySnapshot::new("v2", 2, vec![0.1, 0.9]);
        let diff = compute_policy_diff(&a, &b);
        assert!(diff.max_action_change > 0.5);
        assert!(diff.kl_divergence > 0.0);
    }

    #[test]
    fn test_policy_diff_changed_states() {
        let mut a = PolicySnapshot::new("v1", 1, vec![0.5, 0.5]);
        a.state_value_samples.insert("s1".to_string(), 1.0);
        let mut b = PolicySnapshot::new("v2", 2, vec![0.5, 0.5]);
        b.state_value_samples.insert("s1".to_string(), 5.0);
        let diff = compute_policy_diff(&a, &b);
        assert_eq!(diff.changed_states.len(), 1);
    }

    // -- Trajectory Analysis --

    #[test]
    fn test_trajectory_basic() {
        let mut traj = Trajectory::new("ep1");
        traj.add_step("s0", 0, 1.0);
        traj.add_step("s1", 1, 2.0);
        traj.add_step("s2", 0, -0.5);
        traj.finish();
        assert_eq!(traj.length(), 3);
        assert_eq!(traj.total_reward, 2.5);
        assert!(traj.terminal);
    }

    #[test]
    fn test_trajectory_critical_points() {
        let mut traj = Trajectory::new("ep1");
        traj.add_step("s0", 0, 0.1);
        traj.add_step("s1", 1, 10.0);
        traj.add_step("s2", 0, 0.2);
        let critical = traj.critical_decision_points(5.0);
        assert_eq!(critical, vec![1]);
    }

    #[test]
    fn test_trajectory_reward_attribution() {
        let mut traj = Trajectory::new("ep1");
        traj.add_step("s0", 0, 1.0);
        traj.add_step("s1", 1, 2.0);
        assert_eq!(traj.reward_attribution(), vec![1.0, 2.0]);
    }

    #[test]
    fn test_trajectory_max_min_step() {
        let mut traj = Trajectory::new("ep1");
        traj.add_step("s0", 0, 1.0);
        traj.add_step("s1", 1, 5.0);
        traj.add_step("s2", 2, -3.0);
        assert_eq!(traj.max_reward_step(), Some(1));
        assert_eq!(traj.min_reward_step(), Some(2));
    }

    #[test]
    fn test_trajectory_analyzer() {
        let mut analyzer = TrajectoryAnalyzer::new();
        let mut t1 = Trajectory::new("e1");
        t1.add_step("s0", 0, 10.0);
        let mut t2 = Trajectory::new("e2");
        t2.add_step("s0", 0, 5.0);
        t2.add_step("s1", 1, 5.0);
        analyzer.add_trajectory(t1);
        analyzer.add_trajectory(t2);
        assert_eq!(analyzer.episode_count(), 2);
        assert_eq!(analyzer.mean_episode_reward(), 10.0);
        assert_eq!(analyzer.mean_episode_length(), 1.5);
    }

    #[test]
    fn test_trajectory_analyzer_best_worst() {
        let mut analyzer = TrajectoryAnalyzer::new();
        let mut t1 = Trajectory::new("e1");
        t1.add_step("s0", 0, 100.0);
        let mut t2 = Trajectory::new("e2");
        t2.add_step("s0", 0, 1.0);
        analyzer.add_trajectory(t1);
        analyzer.add_trajectory(t2);
        assert_eq!(analyzer.best_episode().unwrap().episode_id, "e1");
        assert_eq!(analyzer.worst_episode().unwrap().episode_id, "e2");
    }

    // -- Cost Tracking --

    #[test]
    fn test_cost_tracker_basic() {
        let mut ct = CostTracker::new(2.0);
        ct.record("exp1", 10.0, 5.0, 1);
        assert_eq!(ct.total_gpu_hours(), 10.0);
        assert_eq!(ct.total_cost_usd(), 20.0);
        assert_eq!(ct.total_reward_improvement(), 5.0);
    }

    #[test]
    fn test_cost_per_reward_unit() {
        let mut ct = CostTracker::new(1.0);
        ct.record("exp1", 10.0, 2.0, 1);
        assert_eq!(ct.cost_per_reward_unit(), 5.0);
    }

    #[test]
    fn test_cost_compute_efficiency() {
        let mut ct = CostTracker::new(1.0);
        ct.record("exp1", 4.0, 8.0, 1);
        assert_eq!(ct.compute_efficiency(), 2.0);
    }

    #[test]
    fn test_cost_by_experiment() {
        let mut ct = CostTracker::new(1.0);
        ct.record("exp1", 5.0, 1.0, 1);
        ct.record("exp2", 10.0, 2.0, 2);
        ct.record("exp1", 3.0, 1.0, 3);
        let by_exp = ct.cost_by_experiment();
        assert_eq!(*by_exp.get("exp1").unwrap(), 8.0);
        assert_eq!(*by_exp.get("exp2").unwrap(), 10.0);
    }

    #[test]
    fn test_cost_most_expensive() {
        let mut ct = CostTracker::new(1.0);
        ct.record("cheap", 1.0, 1.0, 1);
        ct.record("expensive", 100.0, 1.0, 2);
        let (id, _) = ct.most_expensive_experiment().unwrap();
        assert_eq!(id, "expensive");
    }

    #[test]
    fn test_cost_most_efficient() {
        let mut ct = CostTracker::new(1.0);
        ct.record("efficient", 1.0, 100.0, 1);
        ct.record("wasteful", 100.0, 1.0, 2);
        let (id, _) = ct.most_efficient_experiment().unwrap();
        assert_eq!(id, "efficient");
    }

    // -- Anomaly Detection --

    #[test]
    fn test_anomaly_zscore_no_anomalies() {
        let values: Vec<f64> = vec![1.0, 1.1, 0.9, 1.0, 1.05, 0.95, 1.02, 0.98];
        let results = detect_anomalies_zscore(&values, 3.0);
        let anomalies: Vec<_> = results.iter().filter(|r| r.is_anomaly).collect();
        assert!(anomalies.is_empty());
    }

    #[test]
    fn test_anomaly_zscore_with_outlier() {
        let mut values: Vec<f64> = vec![1.0; 20];
        values.push(100.0);
        let results = detect_anomalies_zscore(&values, 3.0);
        let anomalies: Vec<_> = results.iter().filter(|r| r.is_anomaly).collect();
        assert!(!anomalies.is_empty());
        assert_eq!(anomalies.last().unwrap().index, 20);
    }

    #[test]
    fn test_anomaly_isolation() {
        let mut values: Vec<f64> = vec![1.0; 20];
        values.push(100.0);
        let results = detect_anomalies_isolation(&values, 0.1);
        let anomalies: Vec<_> = results.iter().filter(|r| r.is_anomaly).collect();
        assert!(!anomalies.is_empty());
    }

    #[test]
    fn test_anomaly_moving_avg() {
        let mut values: Vec<f64> = (0..20).map(|i| 1.0 + (i as f64) * 0.01).collect();
        values.push(50.0);
        let results = detect_anomalies_moving_avg(&values, 10, 3.0);
        let anomalies: Vec<_> = results.iter().filter(|r| r.is_anomaly).collect();
        assert!(!anomalies.is_empty());
    }

    #[test]
    fn test_anomaly_zscore_empty() {
        let results = detect_anomalies_zscore(&[], 3.0);
        assert!(results.is_empty());
    }

    #[test]
    fn test_anomaly_isolation_small() {
        let results = detect_anomalies_isolation(&[1.0], 0.1);
        assert!(results.is_empty());
    }

    // -- Report Generation --

    #[test]
    fn test_report_generation() {
        let th = TrainingHealthMonitor::new();
        let sm = SafetyMonitor::new();
        let ct = CostTracker::new(1.0);
        let am = AlertManager::new();
        let report = ReportGenerator::generate(&th, &sm, &ct, &am, 1000);
        assert_eq!(report.sections.len(), 4);
        assert_eq!(report.title, "RL Observability Report");
    }

    #[test]
    fn test_report_markdown() {
        let th = TrainingHealthMonitor::new();
        let sm = SafetyMonitor::new();
        let ct = CostTracker::new(1.0);
        let am = AlertManager::new();
        let report = ReportGenerator::generate(&th, &sm, &ct, &am, 1000);
        let md = ReportGenerator::to_markdown(&report);
        assert!(md.contains("# RL Observability Report"));
        assert!(md.contains("Training Health"));
        assert!(md.contains("Safety Compliance"));
    }

    #[test]
    fn test_report_status_display() {
        assert_eq!(format!("{}", ReportStatus::Healthy), "Healthy");
        assert_eq!(format!("{}", ReportStatus::Critical), "Critical");
    }

    // -- Metric Aggregation --

    #[test]
    fn test_sliding_window_mean() {
        let mut sw = SlidingWindowAggregator::new(3);
        sw.push(1.0);
        sw.push(2.0);
        sw.push(3.0);
        assert_eq!(sw.mean(), 2.0);
        sw.push(4.0);
        assert_eq!(sw.mean(), 3.0);
        assert_eq!(sw.len(), 3);
    }

    #[test]
    fn test_sliding_window_min_max() {
        let mut sw = SlidingWindowAggregator::new(5);
        sw.push(3.0);
        sw.push(1.0);
        sw.push(5.0);
        assert_eq!(sw.min(), Some(1.0));
        assert_eq!(sw.max(), Some(5.0));
    }

    #[test]
    fn test_sliding_window_std_dev() {
        let mut sw = SlidingWindowAggregator::new(5);
        sw.push(2.0);
        sw.push(4.0);
        assert!(sw.std_dev() > 0.0);
    }

    #[test]
    fn test_sliding_window_empty() {
        let sw = SlidingWindowAggregator::new(5);
        assert_eq!(sw.mean(), 0.0);
        assert!(sw.is_empty());
        assert_eq!(sw.min(), None);
    }

    #[test]
    fn test_ema_aggregator() {
        let mut ema = EmaAggregator::new(0.5);
        ema.push(10.0);
        assert_eq!(ema.current(), Some(10.0));
        ema.push(20.0);
        assert_eq!(ema.current(), Some(15.0));
    }

    #[test]
    fn test_ema_reset() {
        let mut ema = EmaAggregator::new(0.5);
        ema.push(10.0);
        ema.reset();
        assert_eq!(ema.current(), None);
        assert_eq!(ema.count, 0);
    }

    #[test]
    fn test_percentile_tracker() {
        let mut pt = PercentileTracker::new();
        for i in 1..=100 {
            pt.push(i as f64);
        }
        assert!((pt.p50() - 50.5).abs() < 2.0);
        assert!((pt.p95() - 95.0).abs() < 2.0);
        assert!((pt.p99() - 99.0).abs() < 2.0);
    }

    #[test]
    fn test_percentile_empty() {
        let mut pt = PercentileTracker::new();
        assert_eq!(pt.p50(), 0.0);
    }

    // -- Retention & Downsampling --

    #[test]
    fn test_retention_default() {
        let policy = RetentionPolicy::default();
        assert_eq!(policy.raw_retention_secs, 86400);
        assert_eq!(policy.downsample_interval_secs, 3600);
    }

    #[test]
    fn test_retention_manager_keeps_recent() {
        let policy = RetentionPolicy {
            raw_retention_secs: 100,
            downsample_interval_secs: 50,
            strategy: DownsampleStrategy::Average,
        };
        let rm = RetentionManager::new(policy);
        let mut series = MetricSeries::new("test", MetricKind::Reward);
        series.push(990, 1.0);
        series.push(995, 2.0);
        series.push(1000, 3.0);
        let result = rm.apply(&series, 1000);
        assert!(result.len() >= 2); // recent data kept
    }

    #[test]
    fn test_retention_manager_downsamples_old() {
        let policy = RetentionPolicy {
            raw_retention_secs: 10,
            downsample_interval_secs: 5,
            strategy: DownsampleStrategy::Average,
        };
        let rm = RetentionManager::new(policy);
        let mut series = MetricSeries::new("test", MetricKind::Reward);
        // Old data
        series.push(1, 10.0);
        series.push(2, 20.0);
        series.push(3, 30.0);
        // Recent data
        series.push(95, 5.0);
        series.push(100, 6.0);
        let result = rm.apply(&series, 100);
        // Old data should be downsampled into bucket(s), recent kept raw
        assert!(result.len() < series.len());
    }

    #[test]
    fn test_retention_should_downsample() {
        let rm = RetentionManager::new(RetentionPolicy::default());
        assert!(rm.should_downsample(0, 100000));
        assert!(!rm.should_downsample(99999, 100000));
    }

    #[test]
    fn test_downsample_strategies() {
        let rm_avg = RetentionManager::new(RetentionPolicy {
            raw_retention_secs: 0,
            downsample_interval_secs: 100,
            strategy: DownsampleStrategy::Average,
        });
        assert_eq!(rm_avg.aggregate(&[1.0, 2.0, 3.0]), 2.0);

        let rm_min = RetentionManager::new(RetentionPolicy {
            raw_retention_secs: 0,
            downsample_interval_secs: 100,
            strategy: DownsampleStrategy::Min,
        });
        assert_eq!(rm_min.aggregate(&[1.0, 2.0, 3.0]), 1.0);

        let rm_max = RetentionManager::new(RetentionPolicy {
            raw_retention_secs: 0,
            downsample_interval_secs: 100,
            strategy: DownsampleStrategy::Max,
        });
        assert_eq!(rm_max.aggregate(&[1.0, 2.0, 3.0]), 3.0);

        let rm_last = RetentionManager::new(RetentionPolicy {
            raw_retention_secs: 0,
            downsample_interval_secs: 100,
            strategy: DownsampleStrategy::Last,
        });
        assert_eq!(rm_last.aggregate(&[1.0, 2.0, 3.0]), 3.0);

        let rm_med = RetentionManager::new(RetentionPolicy {
            raw_retention_secs: 0,
            downsample_interval_secs: 100,
            strategy: DownsampleStrategy::Median,
        });
        assert_eq!(rm_med.aggregate(&[1.0, 2.0, 3.0]), 2.0);
    }

    // -- Correlation Analysis --

    #[test]
    fn test_pearson_perfect_positive() {
        let a = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let b = vec![2.0, 4.0, 6.0, 8.0, 10.0];
        let r = pearson_correlation(&a, &b);
        assert!((r - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_pearson_perfect_negative() {
        let a = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let b = vec![10.0, 8.0, 6.0, 4.0, 2.0];
        let r = pearson_correlation(&a, &b);
        assert!((r - (-1.0)).abs() < 0.001);
    }

    #[test]
    fn test_pearson_no_correlation() {
        let a = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let b = vec![3.0, 1.0, 4.0, 2.0, 5.0];
        let r = pearson_correlation(&a, &b);
        assert!(r.abs() < 0.9); // Not perfectly correlated
    }

    #[test]
    fn test_pearson_empty() {
        assert_eq!(pearson_correlation(&[], &[]), 0.0);
    }

    #[test]
    fn test_correlation_analyzer() {
        let mut ca = CorrelationAnalyzer::new();
        ca.add_series("reward", vec![1.0, 2.0, 3.0, 4.0, 5.0]);
        ca.add_series("loss", vec![5.0, 4.0, 3.0, 2.0, 1.0]);
        ca.compute_all();
        assert_eq!(ca.results.len(), 1);
        assert!((ca.results[0].pearson_r - (-1.0)).abs() < 0.001);
    }

    #[test]
    fn test_correlation_strongest() {
        let mut ca = CorrelationAnalyzer::new();
        ca.add_series("a", vec![1.0, 2.0, 3.0, 4.0, 5.0]);
        ca.add_series("b", vec![2.0, 4.0, 6.0, 8.0, 10.0]);
        ca.add_series("c", vec![3.0, 1.0, 4.0, 2.0, 5.0]);
        ca.compute_all();
        let strongest = ca.strongest_correlation().unwrap();
        assert!(strongest.pearson_r.abs() > 0.9);
    }

    #[test]
    fn test_correlation_for_metric() {
        let mut ca = CorrelationAnalyzer::new();
        ca.add_series("a", vec![1.0, 2.0, 3.0]);
        ca.add_series("b", vec![1.0, 2.0, 3.0]);
        ca.add_series("c", vec![1.0, 2.0, 3.0]);
        ca.compute_all();
        let for_a = ca.correlations_for("a");
        assert_eq!(for_a.len(), 2);
    }

    // -- Engine Integration --

    #[test]
    fn test_engine_creation() {
        let engine = RlObservabilityEngine::new(RlObservabilityConfig::default());
        assert_eq!(engine.metric_count(), 0);
        assert_eq!(engine.total_data_points(), 0);
    }

    #[test]
    fn test_engine_record_metric() {
        let mut engine = RlObservabilityEngine::new(RlObservabilityConfig::default());
        engine.record_metric("reward", MetricKind::Reward, 1, 10.0);
        engine.record_metric("reward", MetricKind::Reward, 2, 20.0);
        assert_eq!(engine.metric_count(), 1);
        assert_eq!(engine.total_data_points(), 2);
    }

    #[test]
    fn test_engine_get_metric() {
        let mut engine = RlObservabilityEngine::new(RlObservabilityConfig::default());
        engine.record_metric("loss", MetricKind::Loss, 1, 0.5);
        let series = engine.get_metric("loss").unwrap();
        assert_eq!(series.len(), 1);
        assert_eq!(series.last_value(), Some(0.5));
    }

    #[test]
    fn test_engine_generate_report() {
        let engine = RlObservabilityEngine::new(RlObservabilityConfig::default());
        let report = engine.generate_report(1000);
        assert_eq!(report.sections.len(), 4);
    }

    #[test]
    fn test_engine_correlation_analysis() {
        let mut engine = RlObservabilityEngine::new(RlObservabilityConfig::default());
        for i in 0..10 {
            engine.record_metric("a", MetricKind::Reward, i, i as f64);
            engine.record_metric("b", MetricKind::Loss, i, (10 - i) as f64);
        }
        engine.run_correlation_analysis();
        assert!(!engine.correlation_analyzer.results.is_empty());
    }

    #[test]
    fn test_engine_retention() {
        let mut engine = RlObservabilityEngine::new(RlObservabilityConfig::default());
        for i in 0..100 {
            engine.record_metric("m", MetricKind::Reward, i, i as f64);
        }
        engine.apply_retention(100000);
        // After retention, old points should be downsampled
        let series = engine.get_metric("m").unwrap();
        assert!(series.len() <= 100);
    }

    #[test]
    fn test_engine_config_defaults() {
        let config = RlObservabilityConfig::default();
        assert!(config.reward_drift_enabled);
        assert!(config.safety_monitoring_enabled);
        assert!(config.anomaly_detection_enabled);
    }

    // -- Helper function tests --

    #[test]
    fn test_compute_entropy_uniform() {
        let dist = vec![0.25, 0.25, 0.25, 0.25];
        let h = compute_entropy(&dist);
        assert!((h - (4.0_f64).ln()).abs() < 0.001);
    }

    #[test]
    fn test_compute_entropy_degenerate() {
        let dist = vec![1.0, 0.0, 0.0];
        let h = compute_entropy(&dist);
        assert!((h - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_kl_divergence_same() {
        let p = vec![0.5, 0.5];
        let kl = kl_divergence(&p, &p);
        assert!((kl - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_kl_divergence_different() {
        let p = vec![0.9, 0.1];
        let q = vec![0.5, 0.5];
        let kl = kl_divergence(&p, &q);
        assert!(kl > 0.0);
    }

    #[test]
    fn test_compute_gini_equal() {
        let values = vec![10.0, 10.0, 10.0];
        assert!((compute_gini(&values) - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_compute_gini_unequal() {
        let values = vec![0.0, 0.0, 100.0];
        let g = compute_gini(&values);
        assert!(g > 0.5);
    }

    #[test]
    fn test_percentile_value_basic() {
        let vals = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        assert!((percentile_value(&vals, 50.0) - 3.0).abs() < 0.01);
    }
}
