//! RL Model Hub — policy registry, lineage tracking, artifact management, and deployment for RL-OS.
//!
//! Production-grade reinforcement learning model hub providing:
//! - Policy registration with semantic versioning and quality gates
//! - RL-specific metadata: observation/action spaces, normalization stats, hyperparameters
//! - Full lineage DAG: parent policy, distillation source, env version, reward hash
//! - Evaluation results storage with per-scenario metrics and reward curves
//! - Artifact management: PyTorch, ONNX, TorchScript, WASM, TFLite with SHA-256 checksums
//! - Cross-framework export with embedded RL metadata for reproducibility
//! - Deployment metadata: serving config, latency profiles, rollback references
//! - Comparison engine, search/discovery, retention policies, audit trail
//! - Promotion workflow: staging → canary → production with approval gates
//! - Auto-generated model cards
//!
//! # Architecture
//!
//! ```text
//! PolicyRecord (versioned, with lineage + metadata)
//!   → ModelHub::register()
//!     ├─ QualityGate::evaluate() — block if thresholds not met
//!     ├─ LineageTracker::record_ancestry()
//!     ├─ ArtifactStore::store_weights()
//!     ├─ AuditTrail::log(Register)
//!     └─ RetentionPolicy::enforce()
//!   → PolicyId { name, version }
//! ```

use std::collections::HashMap;

// ── Enums ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RlAlgorithm {
    PPO,
    SAC,
    DQN,
    A2C,
    TD3,
    DDPG,
    TRPO,
    IMPALA,
    Dreamer,
    MuZero,
    Custom(String),
}

impl RlAlgorithm {
    pub fn label(&self) -> &str {
        match self {
            Self::PPO => "PPO",
            Self::SAC => "SAC",
            Self::DQN => "DQN",
            Self::A2C => "A2C",
            Self::TD3 => "TD3",
            Self::DDPG => "DDPG",
            Self::TRPO => "TRPO",
            Self::IMPALA => "IMPALA",
            Self::Dreamer => "Dreamer",
            Self::MuZero => "MuZero",
            Self::Custom(s) => s.as_str(),
        }
    }

    pub fn is_on_policy(&self) -> bool {
        matches!(self, Self::PPO | Self::A2C | Self::TRPO | Self::IMPALA)
    }

    pub fn is_off_policy(&self) -> bool {
        matches!(self, Self::SAC | Self::DQN | Self::TD3 | Self::DDPG)
    }

    pub fn is_model_based(&self) -> bool {
        matches!(self, Self::Dreamer | Self::MuZero)
    }
}

impl std::fmt::Display for RlAlgorithm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ArtifactFormat {
    PyTorch,
    ONNX,
    TorchScript,
    WASM,
    TFLite,
    CustomRuntime(String),
}

impl ArtifactFormat {
    pub fn label(&self) -> &str {
        match self {
            Self::PyTorch => "pytorch",
            Self::ONNX => "onnx",
            Self::TorchScript => "torchscript",
            Self::WASM => "wasm",
            Self::TFLite => "tflite",
            Self::CustomRuntime(s) => s.as_str(),
        }
    }

    pub fn file_extension(&self) -> &str {
        match self {
            Self::PyTorch => ".pt",
            Self::ONNX => ".onnx",
            Self::TorchScript => ".pt",
            Self::WASM => ".wasm",
            Self::TFLite => ".tflite",
            Self::CustomRuntime(_) => ".bin",
        }
    }
}

impl std::fmt::Display for ArtifactFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PromotionStage {
    Development,
    Staging,
    Canary,
    Production,
    Deprecated,
    Archived,
}

impl PromotionStage {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Development => "development",
            Self::Staging => "staging",
            Self::Canary => "canary",
            Self::Production => "production",
            Self::Deprecated => "deprecated",
            Self::Archived => "archived",
        }
    }

    pub fn can_promote_to(&self, target: &PromotionStage) -> bool {
        match (self, target) {
            (Self::Development, Self::Staging) => true,
            (Self::Staging, Self::Canary) => true,
            (Self::Canary, Self::Production) => true,
            (Self::Production, Self::Deprecated) => true,
            (Self::Deprecated, Self::Archived) => true,
            _ => false,
        }
    }

    pub fn ordinal(&self) -> u8 {
        match self {
            Self::Development => 0,
            Self::Staging => 1,
            Self::Canary => 2,
            Self::Production => 3,
            Self::Deprecated => 4,
            Self::Archived => 5,
        }
    }
}

impl std::fmt::Display for PromotionStage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SpaceType {
    Discrete(u64),
    Continuous { dims: usize },
    MultiDiscrete(Vec<u64>),
    MultiBinary(usize),
    Dict(Vec<(String, SpaceType)>),
    Tuple(Vec<SpaceType>),
    Image { height: u32, width: u32, channels: u32 },
}

impl SpaceType {
    pub fn label(&self) -> &str {
        match self {
            Self::Discrete(_) => "Discrete",
            Self::Continuous { .. } => "Continuous",
            Self::MultiDiscrete(_) => "MultiDiscrete",
            Self::MultiBinary(_) => "MultiBinary",
            Self::Dict(_) => "Dict",
            Self::Tuple(_) => "Tuple",
            Self::Image { .. } => "Image",
        }
    }

    pub fn flat_size(&self) -> usize {
        match self {
            Self::Discrete(_) => 1,
            Self::Continuous { dims } => *dims,
            Self::MultiDiscrete(v) => v.len(),
            Self::MultiBinary(n) => *n,
            Self::Dict(fields) => fields.iter().map(|(_, s)| s.flat_size()).sum(),
            Self::Tuple(elems) => elems.iter().map(|s| s.flat_size()).sum(),
            Self::Image { height, width, channels } => (*height as usize) * (*width as usize) * (*channels as usize),
        }
    }
}

impl std::fmt::Display for SpaceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Discrete(n) => write!(f, "Discrete({})", n),
            Self::Continuous { dims } => write!(f, "Box({})", dims),
            Self::MultiDiscrete(v) => write!(f, "MultiDiscrete({:?})", v),
            Self::MultiBinary(n) => write!(f, "MultiBinary({})", n),
            Self::Dict(fields) => {
                let keys: Vec<&str> = fields.iter().map(|(k, _)| k.as_str()).collect();
                write!(f, "Dict({})", keys.join(", "))
            }
            Self::Tuple(elems) => write!(f, "Tuple({})", elems.len()),
            Self::Image { height, width, channels } => write!(f, "Image({}x{}x{})", height, width, channels),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum AuditAction {
    Register,
    Update,
    Promote { from: PromotionStage, to: PromotionStage },
    Deprecate,
    Delete,
    ArtifactUpload { format: ArtifactFormat },
    ArtifactDownload { format: ArtifactFormat },
    EvalAdded,
    TagAdded(String),
    TagRemoved(String),
    RetentionCleanup,
    Import,
    Export,
    QualityGatePass,
    QualityGateFail(String),
}

impl AuditAction {
    pub fn label(&self) -> String {
        match self {
            Self::Register => "register".to_string(),
            Self::Update => "update".to_string(),
            Self::Promote { from, to } => format!("promote:{}->{}", from, to),
            Self::Deprecate => "deprecate".to_string(),
            Self::Delete => "delete".to_string(),
            Self::ArtifactUpload { format } => format!("artifact_upload:{}", format),
            Self::ArtifactDownload { format } => format!("artifact_download:{}", format),
            Self::EvalAdded => "eval_added".to_string(),
            Self::TagAdded(t) => format!("tag_added:{}", t),
            Self::TagRemoved(t) => format!("tag_removed:{}", t),
            Self::RetentionCleanup => "retention_cleanup".to_string(),
            Self::Import => "import".to_string(),
            Self::Export => "export".to_string(),
            Self::QualityGatePass => "quality_gate_pass".to_string(),
            Self::QualityGateFail(reason) => format!("quality_gate_fail:{}", reason),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum HubError {
    PolicyNotFound(String),
    VersionExists(String),
    QualityGateFailed(Vec<String>),
    InvalidPromotion { from: PromotionStage, to: PromotionStage },
    ArtifactNotFound { policy_id: String, format: ArtifactFormat },
    ChecksumMismatch { expected: String, actual: String },
    InvalidVersion(String),
    RetentionViolation(String),
    ImportError(String),
    ApprovalRequired(String),
    DuplicateTag(String),
}

impl std::fmt::Display for HubError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PolicyNotFound(id) => write!(f, "policy not found: {}", id),
            Self::VersionExists(v) => write!(f, "version already exists: {}", v),
            Self::QualityGateFailed(reasons) => write!(f, "quality gate failed: {}", reasons.join("; ")),
            Self::InvalidPromotion { from, to } => write!(f, "invalid promotion: {} -> {}", from, to),
            Self::ArtifactNotFound { policy_id, format } => write!(f, "artifact not found: {} ({})", policy_id, format),
            Self::ChecksumMismatch { expected, actual } => write!(f, "checksum mismatch: expected {}, got {}", expected, actual),
            Self::InvalidVersion(v) => write!(f, "invalid semantic version: {}", v),
            Self::RetentionViolation(msg) => write!(f, "retention violation: {}", msg),
            Self::ImportError(msg) => write!(f, "import error: {}", msg),
            Self::ApprovalRequired(msg) => write!(f, "approval required: {}", msg),
            Self::DuplicateTag(t) => write!(f, "duplicate tag: {}", t),
        }
    }
}

// ── Core Data Structures ───────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct SemanticVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
    pub pre_release: Option<String>,
}

impl SemanticVersion {
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self { major, minor, patch, pre_release: None }
    }

    pub fn with_pre_release(mut self, pre: &str) -> Self {
        self.pre_release = Some(pre.to_string());
        self
    }

    pub fn parse(s: &str) -> Result<Self, HubError> {
        let (version_part, pre) = if let Some(idx) = s.find('-') {
            (&s[..idx], Some(s[idx + 1..].to_string()))
        } else {
            (s, None)
        };
        let parts: Vec<&str> = version_part.split('.').collect();
        if parts.len() != 3 {
            return Err(HubError::InvalidVersion(s.to_string()));
        }
        let major = parts[0].parse::<u32>().map_err(|_| HubError::InvalidVersion(s.to_string()))?;
        let minor = parts[1].parse::<u32>().map_err(|_| HubError::InvalidVersion(s.to_string()))?;
        let patch = parts[2].parse::<u32>().map_err(|_| HubError::InvalidVersion(s.to_string()))?;
        Ok(Self { major, minor, patch, pre_release: pre })
    }

    pub fn to_string_repr(&self) -> String {
        let base = format!("{}.{}.{}", self.major, self.minor, self.patch);
        match &self.pre_release {
            Some(pre) => format!("{}-{}", base, pre),
            None => base,
        }
    }

    pub fn bump_patch(&self) -> Self {
        Self::new(self.major, self.minor, self.patch + 1)
    }

    pub fn bump_minor(&self) -> Self {
        Self::new(self.major, self.minor + 1, 0)
    }

    pub fn bump_major(&self) -> Self {
        Self::new(self.major + 1, 0, 0)
    }

    pub fn is_compatible_with(&self, other: &Self) -> bool {
        self.major == other.major
    }

    pub fn compare(&self, other: &Self) -> std::cmp::Ordering {
        self.major.cmp(&other.major)
            .then(self.minor.cmp(&other.minor))
            .then(self.patch.cmp(&other.patch))
    }
}

impl std::fmt::Display for SemanticVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_string_repr())
    }
}

#[derive(Debug, Clone)]
pub struct NormalizationStats {
    pub running_mean: Vec<f64>,
    pub running_std: Vec<f64>,
    pub count: u64,
    pub clip_range: Option<f64>,
}

impl NormalizationStats {
    pub fn new(dims: usize) -> Self {
        Self {
            running_mean: vec![0.0; dims],
            running_std: vec![1.0; dims],
            count: 0,
            clip_range: None,
        }
    }

    pub fn with_clip_range(mut self, range: f64) -> Self {
        self.clip_range = Some(range);
        self
    }

    pub fn update(&mut self, observation: &[f64]) {
        if observation.len() != self.running_mean.len() {
            return;
        }
        self.count += 1;
        let n = self.count as f64;
        for i in 0..observation.len() {
            let delta = observation[i] - self.running_mean[i];
            self.running_mean[i] += delta / n;
            let delta2 = observation[i] - self.running_mean[i];
            let new_var = ((n - 1.0) * self.running_std[i].powi(2) + delta * delta2) / n;
            self.running_std[i] = new_var.max(1e-8).sqrt();
        }
    }

    pub fn normalize(&self, observation: &[f64]) -> Vec<f64> {
        observation.iter().enumerate().map(|(i, &v)| {
            let mean = self.running_mean.get(i).copied().unwrap_or(0.0);
            let std = self.running_std.get(i).copied().unwrap_or(1.0);
            let normalized = (v - mean) / std.max(1e-8);
            if let Some(clip) = self.clip_range {
                normalized.clamp(-clip, clip)
            } else {
                normalized
            }
        }).collect()
    }

    pub fn dims(&self) -> usize {
        self.running_mean.len()
    }
}

#[derive(Debug, Clone)]
pub struct Hyperparameters {
    pub learning_rate: f64,
    pub gamma: f64,
    pub gae_lambda: Option<f64>,
    pub entropy_coeff: f64,
    pub value_loss_coeff: f64,
    pub max_grad_norm: f64,
    pub batch_size: usize,
    pub n_epochs: usize,
    pub n_steps: usize,
    pub clip_range: Option<f64>,
    pub target_kl: Option<f64>,
    pub tau: Option<f64>,
    pub buffer_size: Option<usize>,
    pub custom: HashMap<String, String>,
}

impl Hyperparameters {
    pub fn default_ppo() -> Self {
        Self {
            learning_rate: 3e-4,
            gamma: 0.99,
            gae_lambda: Some(0.95),
            entropy_coeff: 0.01,
            value_loss_coeff: 0.5,
            max_grad_norm: 0.5,
            batch_size: 64,
            n_epochs: 10,
            n_steps: 2048,
            clip_range: Some(0.2),
            target_kl: Some(0.01),
            tau: None,
            buffer_size: None,
            custom: HashMap::new(),
        }
    }

    pub fn default_sac() -> Self {
        Self {
            learning_rate: 3e-4,
            gamma: 0.99,
            gae_lambda: None,
            entropy_coeff: 0.2,
            value_loss_coeff: 0.5,
            max_grad_norm: 1.0,
            batch_size: 256,
            n_epochs: 1,
            n_steps: 1,
            clip_range: None,
            target_kl: None,
            tau: Some(0.005),
            buffer_size: Some(1_000_000),
            custom: HashMap::new(),
        }
    }

    pub fn default_dqn() -> Self {
        Self {
            learning_rate: 1e-4,
            gamma: 0.99,
            gae_lambda: None,
            entropy_coeff: 0.0,
            value_loss_coeff: 1.0,
            max_grad_norm: 10.0,
            batch_size: 32,
            n_epochs: 1,
            n_steps: 1,
            clip_range: None,
            target_kl: None,
            tau: Some(1.0),
            buffer_size: Some(50_000),
            custom: HashMap::new(),
        }
    }

    pub fn set_custom(&mut self, key: &str, value: &str) {
        self.custom.insert(key.to_string(), value.to_string());
    }

    pub fn get_custom(&self, key: &str) -> Option<&str> {
        self.custom.get(key).map(|s| s.as_str())
    }

    pub fn diff(&self, other: &Hyperparameters) -> Vec<(String, String, String)> {
        let mut diffs = Vec::new();
        if (self.learning_rate - other.learning_rate).abs() > 1e-12 {
            diffs.push(("learning_rate".into(), format!("{}", self.learning_rate), format!("{}", other.learning_rate)));
        }
        if (self.gamma - other.gamma).abs() > 1e-12 {
            diffs.push(("gamma".into(), format!("{}", self.gamma), format!("{}", other.gamma)));
        }
        if self.batch_size != other.batch_size {
            diffs.push(("batch_size".into(), format!("{}", self.batch_size), format!("{}", other.batch_size)));
        }
        if self.n_epochs != other.n_epochs {
            diffs.push(("n_epochs".into(), format!("{}", self.n_epochs), format!("{}", other.n_epochs)));
        }
        if self.n_steps != other.n_steps {
            diffs.push(("n_steps".into(), format!("{}", self.n_steps), format!("{}", other.n_steps)));
        }
        if (self.entropy_coeff - other.entropy_coeff).abs() > 1e-12 {
            diffs.push(("entropy_coeff".into(), format!("{}", self.entropy_coeff), format!("{}", other.entropy_coeff)));
        }
        if self.clip_range != other.clip_range {
            diffs.push(("clip_range".into(), format!("{:?}", self.clip_range), format!("{:?}", other.clip_range)));
        }
        if self.target_kl != other.target_kl {
            diffs.push(("target_kl".into(), format!("{:?}", self.target_kl), format!("{:?}", other.target_kl)));
        }
        if self.tau != other.tau {
            diffs.push(("tau".into(), format!("{:?}", self.tau), format!("{:?}", other.tau)));
        }
        if self.buffer_size != other.buffer_size {
            diffs.push(("buffer_size".into(), format!("{:?}", self.buffer_size), format!("{:?}", other.buffer_size)));
        }
        diffs
    }
}

#[derive(Debug, Clone)]
pub struct PolicyLineage {
    pub parent_policy: Option<String>,
    pub distilled_from: Option<String>,
    pub env_version: String,
    pub reward_function_hash: String,
    pub training_config_ref: String,
    pub training_seed: Option<u64>,
    pub training_steps: u64,
    pub training_wall_time_secs: f64,
    pub training_env_count: u32,
    pub data_source: Option<String>,
}

impl PolicyLineage {
    pub fn new(env_version: &str, reward_hash: &str, config_ref: &str) -> Self {
        Self {
            parent_policy: None,
            distilled_from: None,
            env_version: env_version.to_string(),
            reward_function_hash: reward_hash.to_string(),
            training_config_ref: config_ref.to_string(),
            training_seed: None,
            training_steps: 0,
            training_wall_time_secs: 0.0,
            training_env_count: 1,
            data_source: None,
        }
    }

    pub fn with_parent(mut self, parent: &str) -> Self {
        self.parent_policy = Some(parent.to_string());
        self
    }

    pub fn with_distilled_from(mut self, source: &str) -> Self {
        self.distilled_from = Some(source.to_string());
        self
    }

    pub fn with_seed(mut self, seed: u64) -> Self {
        self.training_seed = Some(seed);
        self
    }

    pub fn with_steps(mut self, steps: u64) -> Self {
        self.training_steps = steps;
        self
    }

    pub fn with_wall_time(mut self, secs: f64) -> Self {
        self.training_wall_time_secs = secs;
        self
    }

    pub fn with_env_count(mut self, count: u32) -> Self {
        self.training_env_count = count;
        self
    }

    pub fn steps_per_second(&self) -> f64 {
        if self.training_wall_time_secs > 0.0 {
            self.training_steps as f64 / self.training_wall_time_secs
        } else {
            0.0
        }
    }

    pub fn is_fine_tuned(&self) -> bool {
        self.parent_policy.is_some()
    }

    pub fn is_distilled(&self) -> bool {
        self.distilled_from.is_some()
    }

    pub fn shares_env_with(&self, other: &PolicyLineage) -> bool {
        self.env_version == other.env_version
    }

    pub fn shares_reward_with(&self, other: &PolicyLineage) -> bool {
        self.reward_function_hash == other.reward_function_hash
    }
}

#[derive(Debug, Clone)]
pub struct ScenarioResult {
    pub scenario_name: String,
    pub mean_reward: f64,
    pub std_reward: f64,
    pub min_reward: f64,
    pub max_reward: f64,
    pub episodes: u32,
    pub success_rate: f64,
    pub mean_episode_length: f64,
    pub custom_metrics: HashMap<String, f64>,
}

impl ScenarioResult {
    pub fn new(name: &str, mean_reward: f64, std_reward: f64, episodes: u32) -> Self {
        Self {
            scenario_name: name.to_string(),
            mean_reward,
            std_reward,
            min_reward: mean_reward - 2.0 * std_reward,
            max_reward: mean_reward + 2.0 * std_reward,
            episodes,
            success_rate: 0.0,
            mean_episode_length: 0.0,
            custom_metrics: HashMap::new(),
        }
    }

    pub fn with_success_rate(mut self, rate: f64) -> Self {
        self.success_rate = rate;
        self
    }

    pub fn with_episode_length(mut self, length: f64) -> Self {
        self.mean_episode_length = length;
        self
    }

    pub fn with_min_max(mut self, min: f64, max: f64) -> Self {
        self.min_reward = min;
        self.max_reward = max;
        self
    }

    pub fn set_custom_metric(&mut self, key: &str, value: f64) {
        self.custom_metrics.insert(key.to_string(), value);
    }

    pub fn reward_range(&self) -> f64 {
        self.max_reward - self.min_reward
    }

    pub fn coefficient_of_variation(&self) -> f64 {
        if self.mean_reward.abs() > 1e-12 {
            self.std_reward / self.mean_reward.abs()
        } else {
            0.0
        }
    }
}

#[derive(Debug, Clone)]
pub struct SafetyReport {
    pub passed: bool,
    pub constraint_violations: Vec<String>,
    pub max_constraint_violation: f64,
    pub safety_score: f64,
    pub tested_scenarios: u32,
}

impl SafetyReport {
    pub fn passing(score: f64, scenarios: u32) -> Self {
        Self {
            passed: true,
            constraint_violations: Vec::new(),
            max_constraint_violation: 0.0,
            safety_score: score,
            tested_scenarios: scenarios,
        }
    }

    pub fn failing(violations: Vec<String>, max_violation: f64, scenarios: u32) -> Self {
        Self {
            passed: false,
            constraint_violations: violations,
            max_constraint_violation: max_violation,
            safety_score: 0.0,
            tested_scenarios: scenarios,
        }
    }
}

#[derive(Debug, Clone)]
pub struct EvaluationResults {
    pub eval_id: String,
    pub timestamp: u64,
    pub scenario_results: Vec<ScenarioResult>,
    pub reward_curve: Vec<f64>,
    pub safety_report: Option<SafetyReport>,
    pub overall_mean_reward: f64,
    pub overall_std_reward: f64,
    pub max_drawdown: f64,
    pub eval_episodes: u32,
    pub eval_steps: u64,
    pub eval_wall_time_secs: f64,
}

impl EvaluationResults {
    pub fn new(eval_id: &str, timestamp: u64) -> Self {
        Self {
            eval_id: eval_id.to_string(),
            timestamp,
            scenario_results: Vec::new(),
            reward_curve: Vec::new(),
            safety_report: None,
            overall_mean_reward: 0.0,
            overall_std_reward: 0.0,
            max_drawdown: 0.0,
            eval_episodes: 0,
            eval_steps: 0,
            eval_wall_time_secs: 0.0,
        }
    }

    pub fn add_scenario(&mut self, result: ScenarioResult) {
        self.eval_episodes += result.episodes;
        self.scenario_results.push(result);
        self.recompute_aggregates();
    }

    pub fn set_reward_curve(&mut self, curve: Vec<f64>) {
        self.max_drawdown = compute_max_drawdown(&curve);
        self.reward_curve = curve;
    }

    pub fn set_safety_report(&mut self, report: SafetyReport) {
        self.safety_report = Some(report);
    }

    fn recompute_aggregates(&mut self) {
        if self.scenario_results.is_empty() {
            return;
        }
        let n = self.scenario_results.len() as f64;
        let sum: f64 = self.scenario_results.iter().map(|s| s.mean_reward).sum();
        self.overall_mean_reward = sum / n;
        let variance: f64 = self.scenario_results.iter()
            .map(|s| (s.mean_reward - self.overall_mean_reward).powi(2))
            .sum::<f64>() / n;
        self.overall_std_reward = variance.sqrt();
    }

    pub fn safety_passed(&self) -> bool {
        self.safety_report.as_ref().map_or(true, |r| r.passed)
    }

    pub fn scenario_count(&self) -> usize {
        self.scenario_results.len()
    }

    pub fn best_scenario(&self) -> Option<&ScenarioResult> {
        self.scenario_results.iter().max_by(|a, b| a.mean_reward.partial_cmp(&b.mean_reward).unwrap_or(std::cmp::Ordering::Equal))
    }

    pub fn worst_scenario(&self) -> Option<&ScenarioResult> {
        self.scenario_results.iter().min_by(|a, b| a.mean_reward.partial_cmp(&b.mean_reward).unwrap_or(std::cmp::Ordering::Equal))
    }
}

fn compute_max_drawdown(curve: &[f64]) -> f64 {
    if curve.is_empty() {
        return 0.0;
    }
    let mut peak = curve[0];
    let mut max_dd = 0.0;
    for &v in curve.iter() {
        if v > peak {
            peak = v;
        }
        let dd = peak - v;
        if dd > max_dd {
            max_dd = dd;
        }
    }
    max_dd
}

#[derive(Debug, Clone)]
pub struct Artifact {
    pub format: ArtifactFormat,
    pub file_path: String,
    pub checksum_sha256: String,
    pub file_size_bytes: u64,
    pub created_at: u64,
    pub metadata: HashMap<String, String>,
}

impl Artifact {
    pub fn new(format: ArtifactFormat, path: &str, checksum: &str, size: u64, created_at: u64) -> Self {
        Self {
            format,
            file_path: path.to_string(),
            checksum_sha256: checksum.to_string(),
            file_size_bytes: size,
            created_at,
            metadata: HashMap::new(),
        }
    }

    pub fn with_metadata(mut self, key: &str, value: &str) -> Self {
        self.metadata.insert(key.to_string(), value.to_string());
        self
    }

    pub fn file_size_mb(&self) -> f64 {
        self.file_size_bytes as f64 / (1024.0 * 1024.0)
    }

    pub fn verify_checksum(&self, actual: &str) -> Result<(), HubError> {
        if self.checksum_sha256 == actual {
            Ok(())
        } else {
            Err(HubError::ChecksumMismatch {
                expected: self.checksum_sha256.clone(),
                actual: actual.to_string(),
            })
        }
    }
}

#[derive(Debug, Clone)]
pub struct DeploymentMetadata {
    pub serving_framework: String,
    pub hardware_target: String,
    pub latency_p50_ms: f64,
    pub latency_p99_ms: f64,
    pub throughput_rps: f64,
    pub memory_usage_mb: f64,
    pub rollback_policy_id: Option<String>,
    pub min_replicas: u32,
    pub max_replicas: u32,
    pub autoscale_metric: Option<String>,
    pub custom_config: HashMap<String, String>,
}

impl DeploymentMetadata {
    pub fn new(framework: &str, hardware: &str) -> Self {
        Self {
            serving_framework: framework.to_string(),
            hardware_target: hardware.to_string(),
            latency_p50_ms: 0.0,
            latency_p99_ms: 0.0,
            throughput_rps: 0.0,
            memory_usage_mb: 0.0,
            rollback_policy_id: None,
            min_replicas: 1,
            max_replicas: 1,
            autoscale_metric: None,
            custom_config: HashMap::new(),
        }
    }

    pub fn with_latency(mut self, p50: f64, p99: f64) -> Self {
        self.latency_p50_ms = p50;
        self.latency_p99_ms = p99;
        self
    }

    pub fn with_throughput(mut self, rps: f64) -> Self {
        self.throughput_rps = rps;
        self
    }

    pub fn with_memory(mut self, mb: f64) -> Self {
        self.memory_usage_mb = mb;
        self
    }

    pub fn with_rollback(mut self, policy_id: &str) -> Self {
        self.rollback_policy_id = Some(policy_id.to_string());
        self
    }

    pub fn with_autoscale(mut self, min: u32, max: u32, metric: &str) -> Self {
        self.min_replicas = min;
        self.max_replicas = max;
        self.autoscale_metric = Some(metric.to_string());
        self
    }

    pub fn latency_ratio(&self) -> f64 {
        if self.latency_p50_ms > 0.0 {
            self.latency_p99_ms / self.latency_p50_ms
        } else {
            0.0
        }
    }
}

// ── Quality Gates ──────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct QualityGate {
    pub min_mean_reward: Option<f64>,
    pub max_drawdown: Option<f64>,
    pub min_success_rate: Option<f64>,
    pub require_safety_pass: bool,
    pub min_eval_episodes: Option<u32>,
    pub max_reward_std: Option<f64>,
    pub min_scenarios: Option<usize>,
    pub custom_checks: Vec<(String, f64, QualityComparator)>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum QualityComparator {
    GreaterThan,
    LessThan,
    GreaterThanOrEqual,
    LessThanOrEqual,
}

impl QualityGate {
    pub fn strict() -> Self {
        Self {
            min_mean_reward: Some(0.0),
            max_drawdown: Some(100.0),
            min_success_rate: Some(0.8),
            require_safety_pass: true,
            min_eval_episodes: Some(100),
            max_reward_std: None,
            min_scenarios: Some(3),
            custom_checks: Vec::new(),
        }
    }

    pub fn permissive() -> Self {
        Self {
            min_mean_reward: None,
            max_drawdown: None,
            min_success_rate: None,
            require_safety_pass: false,
            min_eval_episodes: None,
            max_reward_std: None,
            min_scenarios: None,
            custom_checks: Vec::new(),
        }
    }

    pub fn with_min_reward(mut self, reward: f64) -> Self {
        self.min_mean_reward = Some(reward);
        self
    }

    pub fn with_max_drawdown(mut self, drawdown: f64) -> Self {
        self.max_drawdown = Some(drawdown);
        self
    }

    pub fn with_min_success_rate(mut self, rate: f64) -> Self {
        self.min_success_rate = Some(rate);
        self
    }

    pub fn with_min_episodes(mut self, episodes: u32) -> Self {
        self.min_eval_episodes = Some(episodes);
        self
    }

    pub fn add_custom_check(&mut self, metric: &str, threshold: f64, cmp: QualityComparator) {
        self.custom_checks.push((metric.to_string(), threshold, cmp));
    }

    pub fn evaluate(&self, eval: &EvaluationResults) -> Result<(), Vec<String>> {
        let mut failures = Vec::new();

        if let Some(min_reward) = self.min_mean_reward {
            if eval.overall_mean_reward < min_reward {
                failures.push(format!(
                    "mean reward {:.4} < min {:.4}",
                    eval.overall_mean_reward, min_reward
                ));
            }
        }

        if let Some(max_dd) = self.max_drawdown {
            if eval.max_drawdown > max_dd {
                failures.push(format!(
                    "max drawdown {:.4} > threshold {:.4}",
                    eval.max_drawdown, max_dd
                ));
            }
        }

        if let Some(min_sr) = self.min_success_rate {
            for s in &eval.scenario_results {
                if s.success_rate < min_sr {
                    failures.push(format!(
                        "scenario '{}' success rate {:.4} < min {:.4}",
                        s.scenario_name, s.success_rate, min_sr
                    ));
                }
            }
        }

        if self.require_safety_pass {
            if let Some(ref report) = eval.safety_report {
                if !report.passed {
                    failures.push(format!(
                        "safety check failed: {} violations",
                        report.constraint_violations.len()
                    ));
                }
            } else {
                failures.push("safety report required but not provided".to_string());
            }
        }

        if let Some(min_ep) = self.min_eval_episodes {
            if eval.eval_episodes < min_ep {
                failures.push(format!(
                    "eval episodes {} < min {}",
                    eval.eval_episodes, min_ep
                ));
            }
        }

        if let Some(max_std) = self.max_reward_std {
            if eval.overall_std_reward > max_std {
                failures.push(format!(
                    "reward std {:.4} > max {:.4}",
                    eval.overall_std_reward, max_std
                ));
            }
        }

        if let Some(min_sc) = self.min_scenarios {
            if eval.scenario_results.len() < min_sc {
                failures.push(format!(
                    "scenarios {} < min {}",
                    eval.scenario_results.len(), min_sc
                ));
            }
        }

        for (metric, threshold, cmp) in &self.custom_checks {
            for s in &eval.scenario_results {
                if let Some(&val) = s.custom_metrics.get(metric) {
                    let failed = match cmp {
                        QualityComparator::GreaterThan => val <= *threshold,
                        QualityComparator::LessThan => val >= *threshold,
                        QualityComparator::GreaterThanOrEqual => val < *threshold,
                        QualityComparator::LessThanOrEqual => val > *threshold,
                    };
                    if failed {
                        failures.push(format!(
                            "custom check '{}' failed for scenario '{}': {:.4}",
                            metric, s.scenario_name, val
                        ));
                    }
                }
            }
        }

        if failures.is_empty() {
            Ok(())
        } else {
            Err(failures)
        }
    }
}

// ── Audit Trail ────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct AuditEntry {
    pub timestamp: u64,
    pub policy_id: String,
    pub version: String,
    pub actor: String,
    pub action: AuditAction,
    pub details: Option<String>,
}

impl AuditEntry {
    pub fn new(timestamp: u64, policy_id: &str, version: &str, actor: &str, action: AuditAction) -> Self {
        Self {
            timestamp,
            policy_id: policy_id.to_string(),
            version: version.to_string(),
            actor: actor.to_string(),
            action,
            details: None,
        }
    }

    pub fn with_details(mut self, details: &str) -> Self {
        self.details = Some(details.to_string());
        self
    }

    pub fn summary(&self) -> String {
        let base = format!(
            "[{}] {} {} v{} by {}",
            self.timestamp,
            self.action.label(),
            self.policy_id,
            self.version,
            self.actor,
        );
        match &self.details {
            Some(d) => format!("{} — {}", base, d),
            None => base,
        }
    }
}

// ── Retention Policy ───────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct RetentionPolicy {
    pub keep_latest_n: usize,
    pub keep_tagged: bool,
    pub keep_promoted: bool,
    pub keep_min_reward_above: Option<f64>,
    pub max_age_days: Option<u64>,
}

impl RetentionPolicy {
    pub fn default_policy() -> Self {
        Self {
            keep_latest_n: 10,
            keep_tagged: true,
            keep_promoted: true,
            keep_min_reward_above: None,
            max_age_days: None,
        }
    }

    pub fn aggressive() -> Self {
        Self {
            keep_latest_n: 3,
            keep_tagged: true,
            keep_promoted: true,
            keep_min_reward_above: None,
            max_age_days: Some(30),
        }
    }

    pub fn should_keep(
        &self,
        index_from_latest: usize,
        is_tagged: bool,
        is_promoted: bool,
        mean_reward: f64,
        _age_days: u64,
    ) -> bool {
        if index_from_latest < self.keep_latest_n {
            return true;
        }
        if self.keep_tagged && is_tagged {
            return true;
        }
        if self.keep_promoted && is_promoted {
            return true;
        }
        if let Some(min_r) = self.keep_min_reward_above {
            if mean_reward >= min_r {
                return true;
            }
        }
        if let Some(max_age) = self.max_age_days {
            if _age_days <= max_age {
                return true;
            }
        }
        false
    }
}

// ── Export Bundle ───────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ExportBundle {
    pub format: ArtifactFormat,
    pub policy_name: String,
    pub version: String,
    pub observation_space: SpaceType,
    pub action_space: SpaceType,
    pub normalization_stats: Option<NormalizationStats>,
    pub reward_function_hash: String,
    pub env_version: String,
    pub hyperparameters: Hyperparameters,
    pub artifact_checksum: String,
    pub artifact_size_bytes: u64,
    pub export_metadata: HashMap<String, String>,
}

impl ExportBundle {
    pub fn new(
        format: ArtifactFormat,
        policy_name: &str,
        version: &str,
        obs_space: SpaceType,
        act_space: SpaceType,
        reward_hash: &str,
        env_version: &str,
        hyperparams: Hyperparameters,
        checksum: &str,
        size: u64,
    ) -> Self {
        Self {
            format,
            policy_name: policy_name.to_string(),
            version: version.to_string(),
            observation_space: obs_space,
            action_space: act_space,
            normalization_stats: None,
            reward_function_hash: reward_hash.to_string(),
            env_version: env_version.to_string(),
            hyperparameters: hyperparams,
            artifact_checksum: checksum.to_string(),
            artifact_size_bytes: size,
            export_metadata: HashMap::new(),
        }
    }

    pub fn with_normalization(mut self, stats: NormalizationStats) -> Self {
        self.normalization_stats = Some(stats);
        self
    }

    pub fn set_metadata(&mut self, key: &str, value: &str) {
        self.export_metadata.insert(key.to_string(), value.to_string());
    }

    pub fn is_reproducible(&self) -> bool {
        !self.reward_function_hash.is_empty()
            && !self.env_version.is_empty()
            && !self.artifact_checksum.is_empty()
    }

    pub fn summary(&self) -> String {
        format!(
            "ExportBundle({} v{}, {}, obs={}, act={}, {})",
            self.policy_name,
            self.version,
            self.format,
            self.observation_space,
            self.action_space,
            if self.normalization_stats.is_some() { "normalized" } else { "raw" },
        )
    }
}

// ── Model Card ─────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ModelCard {
    pub policy_name: String,
    pub version: String,
    pub algorithm: RlAlgorithm,
    pub description: String,
    pub author: String,
    pub created_at: u64,
    pub env_name: String,
    pub env_version: String,
    pub observation_space: String,
    pub action_space: String,
    pub training_steps: u64,
    pub training_wall_time: String,
    pub eval_summary: String,
    pub best_reward: f64,
    pub known_limitations: Vec<String>,
    pub tags: Vec<String>,
    pub lineage_summary: String,
    pub artifact_formats: Vec<ArtifactFormat>,
    pub deployment_notes: String,
    pub safety_notes: String,
}

impl ModelCard {
    pub fn generate(record: &PolicyRecord) -> Self {
        let eval_summary = if let Some(ref eval) = record.latest_eval {
            format!(
                "Mean reward: {:.4}, Std: {:.4}, Max drawdown: {:.4}, Episodes: {}, Scenarios: {}",
                eval.overall_mean_reward,
                eval.overall_std_reward,
                eval.max_drawdown,
                eval.eval_episodes,
                eval.scenario_results.len(),
            )
        } else {
            "No evaluation results available".to_string()
        };

        let best_reward = record.latest_eval.as_ref()
            .and_then(|e| e.best_scenario())
            .map(|s| s.mean_reward)
            .unwrap_or(0.0);

        let lineage_summary = format!(
            "Env: {} v{}, Reward hash: {}, Steps: {}, Parent: {}, Distilled: {}",
            record.env_name,
            record.lineage.env_version,
            record.lineage.reward_function_hash,
            record.lineage.training_steps,
            record.lineage.parent_policy.as_deref().unwrap_or("none"),
            record.lineage.distilled_from.as_deref().unwrap_or("none"),
        );

        let safety_notes = record.latest_eval.as_ref()
            .and_then(|e| e.safety_report.as_ref())
            .map(|r| {
                if r.passed {
                    format!("Safety passed. Score: {:.2}, Tested scenarios: {}", r.safety_score, r.tested_scenarios)
                } else {
                    format!("Safety FAILED. Violations: {}", r.constraint_violations.join(", "))
                }
            })
            .unwrap_or_else(|| "No safety assessment available".to_string());

        let wall_time = format!("{:.1}s", record.lineage.training_wall_time_secs);

        Self {
            policy_name: record.name.clone(),
            version: record.version.to_string_repr(),
            algorithm: record.algorithm.clone(),
            description: record.description.clone(),
            author: record.author.clone(),
            created_at: record.created_at,
            env_name: record.env_name.clone(),
            env_version: record.lineage.env_version.clone(),
            observation_space: record.observation_space.to_string(),
            action_space: record.action_space.to_string(),
            training_steps: record.lineage.training_steps,
            training_wall_time: wall_time,
            eval_summary,
            best_reward,
            known_limitations: record.known_limitations.clone(),
            tags: record.tags.clone(),
            lineage_summary,
            artifact_formats: record.artifacts.iter().map(|a| a.format.clone()).collect(),
            deployment_notes: record.deployment.as_ref()
                .map(|d| format!("{} on {}, p50={}ms, p99={}ms", d.serving_framework, d.hardware_target, d.latency_p50_ms, d.latency_p99_ms))
                .unwrap_or_else(|| "No deployment config".to_string()),
            safety_notes,
        }
    }

    pub fn to_markdown(&self) -> String {
        let mut md = String::with_capacity(2048);
        md.push_str(&format!("# Model Card: {} v{}\n\n", self.policy_name, self.version));
        md.push_str(&format!("**Algorithm**: {}  \n", self.algorithm));
        md.push_str(&format!("**Author**: {}  \n", self.author));
        md.push_str(&format!("**Created**: {}  \n\n", self.created_at));
        md.push_str(&format!("## Description\n\n{}\n\n", self.description));
        md.push_str("## Environment\n\n");
        md.push_str(&format!("- **Name**: {}\n", self.env_name));
        md.push_str(&format!("- **Version**: {}\n", self.env_version));
        md.push_str(&format!("- **Observation Space**: {}\n", self.observation_space));
        md.push_str(&format!("- **Action Space**: {}\n\n", self.action_space));
        md.push_str("## Training\n\n");
        md.push_str(&format!("- **Steps**: {}\n", self.training_steps));
        md.push_str(&format!("- **Wall Time**: {}\n", self.training_wall_time));
        md.push_str(&format!("- **Lineage**: {}\n\n", self.lineage_summary));
        md.push_str(&format!("## Evaluation\n\n{}\n\n", self.eval_summary));
        md.push_str(&format!("**Best Reward**: {:.4}\n\n", self.best_reward));
        md.push_str(&format!("## Safety\n\n{}\n\n", self.safety_notes));
        if !self.known_limitations.is_empty() {
            md.push_str("## Known Limitations\n\n");
            for lim in &self.known_limitations {
                md.push_str(&format!("- {}\n", lim));
            }
            md.push_str("\n");
        }
        if !self.tags.is_empty() {
            md.push_str(&format!("## Tags\n\n{}\n\n", self.tags.join(", ")));
        }
        if !self.artifact_formats.is_empty() {
            md.push_str("## Available Formats\n\n");
            for fmt in &self.artifact_formats {
                md.push_str(&format!("- {}\n", fmt));
            }
            md.push_str("\n");
        }
        md.push_str(&format!("## Deployment\n\n{}\n", self.deployment_notes));
        md
    }
}

// ── Comparison Engine ──────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct PolicyComparison {
    pub policy_a: String,
    pub version_a: String,
    pub policy_b: String,
    pub version_b: String,
    pub metric_diffs: Vec<MetricDiff>,
    pub hyperparameter_diffs: Vec<(String, String, String)>,
    pub lineage_divergence: LineageDivergence,
    pub recommendation: String,
}

#[derive(Debug, Clone)]
pub struct MetricDiff {
    pub metric_name: String,
    pub value_a: f64,
    pub value_b: f64,
    pub delta: f64,
    pub relative_change_pct: f64,
}

impl MetricDiff {
    pub fn new(name: &str, a: f64, b: f64) -> Self {
        let delta = b - a;
        let relative = if a.abs() > 1e-12 { (delta / a.abs()) * 100.0 } else { 0.0 };
        Self {
            metric_name: name.to_string(),
            value_a: a,
            value_b: b,
            delta,
            relative_change_pct: relative,
        }
    }

    pub fn improved(&self) -> bool {
        self.delta > 0.0
    }

    pub fn significant(&self, threshold_pct: f64) -> bool {
        self.relative_change_pct.abs() >= threshold_pct
    }
}

#[derive(Debug, Clone)]
pub struct LineageDivergence {
    pub common_ancestor: Option<String>,
    pub env_changed: bool,
    pub reward_changed: bool,
    pub algorithm_changed: bool,
    pub divergence_depth: u32,
}

// ── Policy Record ──────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct PolicyRecord {
    pub name: String,
    pub version: SemanticVersion,
    pub algorithm: RlAlgorithm,
    pub description: String,
    pub author: String,
    pub env_name: String,
    pub observation_space: SpaceType,
    pub action_space: SpaceType,
    pub normalization_stats: Option<NormalizationStats>,
    pub hyperparameters: Hyperparameters,
    pub lineage: PolicyLineage,
    pub latest_eval: Option<EvaluationResults>,
    pub eval_history: Vec<EvaluationResults>,
    pub artifacts: Vec<Artifact>,
    pub deployment: Option<DeploymentMetadata>,
    pub stage: PromotionStage,
    pub tags: Vec<String>,
    pub known_limitations: Vec<String>,
    pub created_at: u64,
    pub updated_at: u64,
}

impl PolicyRecord {
    pub fn new(
        name: &str,
        version: SemanticVersion,
        algorithm: RlAlgorithm,
        author: &str,
        env_name: &str,
        obs_space: SpaceType,
        act_space: SpaceType,
        hyperparams: Hyperparameters,
        lineage: PolicyLineage,
        created_at: u64,
    ) -> Self {
        Self {
            name: name.to_string(),
            version,
            algorithm,
            description: String::new(),
            author: author.to_string(),
            env_name: env_name.to_string(),
            observation_space: obs_space,
            action_space: act_space,
            normalization_stats: None,
            hyperparameters: hyperparams,
            lineage,
            latest_eval: None,
            eval_history: Vec::new(),
            artifacts: Vec::new(),
            deployment: None,
            stage: PromotionStage::Development,
            tags: Vec::new(),
            known_limitations: Vec::new(),
            created_at,
            updated_at: created_at,
        }
    }

    pub fn with_description(mut self, desc: &str) -> Self {
        self.description = desc.to_string();
        self
    }

    pub fn with_normalization(mut self, stats: NormalizationStats) -> Self {
        self.normalization_stats = Some(stats);
        self
    }

    pub fn full_id(&self) -> String {
        format!("{}:{}", self.name, self.version)
    }

    pub fn is_tagged(&self) -> bool {
        !self.tags.is_empty()
    }

    pub fn is_promoted(&self) -> bool {
        matches!(self.stage, PromotionStage::Canary | PromotionStage::Production)
    }

    pub fn add_tag(&mut self, tag: &str) -> Result<(), HubError> {
        if self.tags.contains(&tag.to_string()) {
            return Err(HubError::DuplicateTag(tag.to_string()));
        }
        self.tags.push(tag.to_string());
        Ok(())
    }

    pub fn remove_tag(&mut self, tag: &str) -> bool {
        if let Some(pos) = self.tags.iter().position(|t| t == tag) {
            self.tags.remove(pos);
            true
        } else {
            false
        }
    }

    pub fn add_limitation(&mut self, limitation: &str) {
        self.known_limitations.push(limitation.to_string());
    }
}

// ── Search Criteria ────────────────────────────────────────────────────

#[derive(Debug, Clone, Default)]
pub struct SearchCriteria {
    pub name_contains: Option<String>,
    pub env_name: Option<String>,
    pub algorithm: Option<RlAlgorithm>,
    pub min_reward: Option<f64>,
    pub max_reward: Option<f64>,
    pub tags: Vec<String>,
    pub author: Option<String>,
    pub stage: Option<PromotionStage>,
    pub created_after: Option<u64>,
    pub created_before: Option<u64>,
    pub has_artifact_format: Option<ArtifactFormat>,
    pub min_training_steps: Option<u64>,
}

impl SearchCriteria {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_name(mut self, name: &str) -> Self {
        self.name_contains = Some(name.to_string());
        self
    }

    pub fn with_env(mut self, env: &str) -> Self {
        self.env_name = Some(env.to_string());
        self
    }

    pub fn with_algorithm(mut self, algo: RlAlgorithm) -> Self {
        self.algorithm = Some(algo);
        self
    }

    pub fn with_reward_range(mut self, min: f64, max: f64) -> Self {
        self.min_reward = Some(min);
        self.max_reward = Some(max);
        self
    }

    pub fn with_tag(mut self, tag: &str) -> Self {
        self.tags.push(tag.to_string());
        self
    }

    pub fn with_author(mut self, author: &str) -> Self {
        self.author = Some(author.to_string());
        self
    }

    pub fn with_stage(mut self, stage: PromotionStage) -> Self {
        self.stage = Some(stage);
        self
    }

    pub fn with_date_range(mut self, after: u64, before: u64) -> Self {
        self.created_after = Some(after);
        self.created_before = Some(before);
        self
    }

    pub fn matches(&self, record: &PolicyRecord) -> bool {
        if let Some(ref name) = self.name_contains {
            if !record.name.to_lowercase().contains(&name.to_lowercase()) {
                return false;
            }
        }
        if let Some(ref env) = self.env_name {
            if record.env_name != *env {
                return false;
            }
        }
        if let Some(ref algo) = self.algorithm {
            if record.algorithm != *algo {
                return false;
            }
        }
        if let Some(min_r) = self.min_reward {
            let reward = record.latest_eval.as_ref().map(|e| e.overall_mean_reward).unwrap_or(f64::NEG_INFINITY);
            if reward < min_r {
                return false;
            }
        }
        if let Some(max_r) = self.max_reward {
            let reward = record.latest_eval.as_ref().map(|e| e.overall_mean_reward).unwrap_or(f64::INFINITY);
            if reward > max_r {
                return false;
            }
        }
        for tag in &self.tags {
            if !record.tags.contains(tag) {
                return false;
            }
        }
        if let Some(ref author) = self.author {
            if record.author != *author {
                return false;
            }
        }
        if let Some(ref stage) = self.stage {
            if record.stage != *stage {
                return false;
            }
        }
        if let Some(after) = self.created_after {
            if record.created_at < after {
                return false;
            }
        }
        if let Some(before) = self.created_before {
            if record.created_at > before {
                return false;
            }
        }
        if let Some(ref fmt) = self.has_artifact_format {
            if !record.artifacts.iter().any(|a| a.format == *fmt) {
                return false;
            }
        }
        if let Some(min_steps) = self.min_training_steps {
            if record.lineage.training_steps < min_steps {
                return false;
            }
        }
        true
    }
}

// ── Import/Export Registry Format ──────────────────────────────────────

#[derive(Debug, Clone)]
pub struct RegistrySnapshot {
    pub exported_at: u64,
    pub source_instance: String,
    pub records: Vec<PolicyRecord>,
    pub audit_entries: Vec<AuditEntry>,
    pub format_version: String,
}

impl RegistrySnapshot {
    pub fn new(source: &str, exported_at: u64) -> Self {
        Self {
            exported_at,
            source_instance: source.to_string(),
            records: Vec::new(),
            audit_entries: Vec::new(),
            format_version: "1.0.0".to_string(),
        }
    }

    pub fn policy_count(&self) -> usize {
        self.records.len()
    }

    pub fn audit_count(&self) -> usize {
        self.audit_entries.len()
    }
}

// ── Model Hub ──────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct ModelHub {
    policies: HashMap<String, Vec<PolicyRecord>>,
    audit_trail: Vec<AuditEntry>,
    quality_gate: QualityGate,
    retention_policy: RetentionPolicy,
    instance_id: String,
    timestamp_counter: u64,
    approvers: HashMap<PromotionStage, Vec<String>>,
}

impl ModelHub {
    pub fn new(instance_id: &str) -> Self {
        Self {
            policies: HashMap::new(),
            audit_trail: Vec::new(),
            quality_gate: QualityGate::permissive(),
            retention_policy: RetentionPolicy::default_policy(),
            instance_id: instance_id.to_string(),
            timestamp_counter: 0,
            approvers: HashMap::new(),
        }
    }

    pub fn with_quality_gate(mut self, gate: QualityGate) -> Self {
        self.quality_gate = gate;
        self
    }

    pub fn with_retention_policy(mut self, policy: RetentionPolicy) -> Self {
        self.retention_policy = policy;
        self
    }

    pub fn set_approvers(&mut self, stage: PromotionStage, approvers: Vec<String>) {
        self.approvers.insert(stage, approvers);
    }

    fn next_timestamp(&mut self) -> u64 {
        self.timestamp_counter += 1;
        self.timestamp_counter
    }

    fn log_audit(&mut self, policy_id: &str, version: &str, actor: &str, action: AuditAction) {
        let ts = self.next_timestamp();
        self.audit_trail.push(AuditEntry::new(ts, policy_id, version, actor, action));
    }

    // ── Registration ───────────────────────────────────────────────────

    pub fn register(&mut self, record: PolicyRecord) -> Result<String, HubError> {
        let key = record.name.clone();
        let version_str = record.version.to_string_repr();

        // Check version uniqueness
        if let Some(versions) = self.policies.get(&key) {
            for existing in versions {
                if existing.version.to_string_repr() == version_str {
                    return Err(HubError::VersionExists(format!("{}:{}", key, version_str)));
                }
            }
        }

        // Quality gate check
        if let Some(ref eval) = record.latest_eval {
            match self.quality_gate.evaluate(eval) {
                Ok(()) => {
                    self.log_audit(&key, &version_str, &record.author, AuditAction::QualityGatePass);
                }
                Err(reasons) => {
                    self.log_audit(
                        &key,
                        &version_str,
                        &record.author,
                        AuditAction::QualityGateFail(reasons.join("; ")),
                    );
                    return Err(HubError::QualityGateFailed(reasons));
                }
            }
        }

        let full_id = record.full_id();
        self.log_audit(&key, &version_str, &record.author, AuditAction::Register);

        self.policies.entry(key).or_insert_with(Vec::new).push(record);
        Ok(full_id)
    }

    pub fn get(&self, name: &str, version: &str) -> Option<&PolicyRecord> {
        self.policies.get(name)?
            .iter()
            .find(|r| r.version.to_string_repr() == version)
    }

    pub fn get_mut(&mut self, name: &str, version: &str) -> Option<&mut PolicyRecord> {
        self.policies.get_mut(name)?
            .iter_mut()
            .find(|r| r.version.to_string_repr() == version)
    }

    pub fn get_latest(&self, name: &str) -> Option<&PolicyRecord> {
        let versions = self.policies.get(name)?;
        versions.iter().max_by(|a, b| a.version.compare(&b.version))
    }

    pub fn list_policies(&self) -> Vec<&str> {
        self.policies.keys().map(|k| k.as_str()).collect()
    }

    pub fn list_versions(&self, name: &str) -> Vec<String> {
        self.policies.get(name)
            .map(|versions| {
                let mut v: Vec<String> = versions.iter().map(|r| r.version.to_string_repr()).collect();
                v.sort();
                v
            })
            .unwrap_or_default()
    }

    pub fn policy_count(&self) -> usize {
        self.policies.values().map(|v| v.len()).sum()
    }

    // ── Delete ─────────────────────────────────────────────────────────

    pub fn delete(&mut self, name: &str, version: &str, actor: &str) -> Result<PolicyRecord, HubError> {
        let versions = self.policies.get_mut(name)
            .ok_or_else(|| HubError::PolicyNotFound(format!("{}:{}", name, version)))?;
        let idx = versions.iter().position(|r| r.version.to_string_repr() == version)
            .ok_or_else(|| HubError::PolicyNotFound(format!("{}:{}", name, version)))?;
        let record = versions.remove(idx);
        if versions.is_empty() {
            self.policies.remove(name);
        }
        self.log_audit(name, version, actor, AuditAction::Delete);
        Ok(record)
    }

    // ── Evaluation ─────────────────────────────────────────────────────

    pub fn add_eval(&mut self, name: &str, version: &str, eval: EvaluationResults, actor: &str) -> Result<(), HubError> {
        let ts = self.timestamp_counter + 1;
        let record = self.get_mut(name, version)
            .ok_or_else(|| HubError::PolicyNotFound(format!("{}:{}", name, version)))?;
        record.eval_history.push(eval.clone());
        record.latest_eval = Some(eval);
        record.updated_at = ts;
        self.log_audit(name, version, actor, AuditAction::EvalAdded);
        Ok(())
    }

    // ── Artifacts ──────────────────────────────────────────────────────

    pub fn add_artifact(&mut self, name: &str, version: &str, artifact: Artifact, actor: &str) -> Result<(), HubError> {
        let format = artifact.format.clone();
        let ts = self.timestamp_counter + 1;
        let record = self.get_mut(name, version)
            .ok_or_else(|| HubError::PolicyNotFound(format!("{}:{}", name, version)))?;
        record.artifacts.push(artifact);
        record.updated_at = ts;
        self.log_audit(name, version, actor, AuditAction::ArtifactUpload { format });
        Ok(())
    }

    pub fn get_artifact(&self, name: &str, version: &str, format: &ArtifactFormat) -> Result<&Artifact, HubError> {
        let record = self.get(name, version)
            .ok_or_else(|| HubError::PolicyNotFound(format!("{}:{}", name, version)))?;
        record.artifacts.iter()
            .find(|a| &a.format == format)
            .ok_or_else(|| HubError::ArtifactNotFound {
                policy_id: format!("{}:{}", name, version),
                format: format.clone(),
            })
    }

    // ── Promotion ──────────────────────────────────────────────────────

    pub fn promote(
        &mut self,
        name: &str,
        version: &str,
        target: PromotionStage,
        actor: &str,
    ) -> Result<(), HubError> {
        // Check approvers
        if let Some(required_approvers) = self.approvers.get(&target) {
            if !required_approvers.contains(&actor.to_string()) {
                return Err(HubError::ApprovalRequired(format!(
                    "{} is not an approved promoter for {}",
                    actor, target
                )));
            }
        }

        let ts = self.timestamp_counter + 1;
        let record = self.get_mut(name, version)
            .ok_or_else(|| HubError::PolicyNotFound(format!("{}:{}", name, version)))?;

        let from = record.stage.clone();
        if !from.can_promote_to(&target) {
            return Err(HubError::InvalidPromotion { from, to: target });
        }

        let from_clone = record.stage.clone();
        record.stage = target.clone();
        record.updated_at = ts;
        self.log_audit(name, version, actor, AuditAction::Promote { from: from_clone, to: target });
        Ok(())
    }

    pub fn deprecate(&mut self, name: &str, version: &str, actor: &str) -> Result<(), HubError> {
        let ts = self.timestamp_counter + 1;
        let record = self.get_mut(name, version)
            .ok_or_else(|| HubError::PolicyNotFound(format!("{}:{}", name, version)))?;
        record.stage = PromotionStage::Deprecated;
        record.updated_at = ts;
        self.log_audit(name, version, actor, AuditAction::Deprecate);
        Ok(())
    }

    // ── Tags ───────────────────────────────────────────────────────────

    pub fn add_tag(&mut self, name: &str, version: &str, tag: &str, actor: &str) -> Result<(), HubError> {
        let record = self.get_mut(name, version)
            .ok_or_else(|| HubError::PolicyNotFound(format!("{}:{}", name, version)))?;
        record.add_tag(tag)?;
        self.log_audit(name, version, actor, AuditAction::TagAdded(tag.to_string()));
        Ok(())
    }

    pub fn remove_tag(&mut self, name: &str, version: &str, tag: &str, actor: &str) -> Result<(), HubError> {
        let record = self.get_mut(name, version)
            .ok_or_else(|| HubError::PolicyNotFound(format!("{}:{}", name, version)))?;
        record.remove_tag(tag);
        self.log_audit(name, version, actor, AuditAction::TagRemoved(tag.to_string()));
        Ok(())
    }

    // ── Search ─────────────────────────────────────────────────────────

    pub fn search(&self, criteria: &SearchCriteria) -> Vec<&PolicyRecord> {
        self.policies.values()
            .flat_map(|versions| versions.iter())
            .filter(|r| criteria.matches(r))
            .collect()
    }

    pub fn search_by_env(&self, env: &str) -> Vec<&PolicyRecord> {
        self.search(&SearchCriteria::new().with_env(env))
    }

    pub fn search_by_algorithm(&self, algo: RlAlgorithm) -> Vec<&PolicyRecord> {
        self.search(&SearchCriteria::new().with_algorithm(algo))
    }

    pub fn search_by_tag(&self, tag: &str) -> Vec<&PolicyRecord> {
        self.search(&SearchCriteria::new().with_tag(tag))
    }

    // ── Deployment ─────────────────────────────────────────────────────

    pub fn set_deployment(&mut self, name: &str, version: &str, deployment: DeploymentMetadata, actor: &str) -> Result<(), HubError> {
        let ts = self.timestamp_counter + 1;
        let record = self.get_mut(name, version)
            .ok_or_else(|| HubError::PolicyNotFound(format!("{}:{}", name, version)))?;
        record.deployment = Some(deployment);
        record.updated_at = ts;
        self.log_audit(name, version, actor, AuditAction::Update);
        Ok(())
    }

    // ── Comparison ─────────────────────────────────────────────────────

    pub fn compare(&self, name_a: &str, version_a: &str, name_b: &str, version_b: &str) -> Result<PolicyComparison, HubError> {
        let a = self.get(name_a, version_a)
            .ok_or_else(|| HubError::PolicyNotFound(format!("{}:{}", name_a, version_a)))?;
        let b = self.get(name_b, version_b)
            .ok_or_else(|| HubError::PolicyNotFound(format!("{}:{}", name_b, version_b)))?;

        let mut metric_diffs = Vec::new();

        if let (Some(eval_a), Some(eval_b)) = (&a.latest_eval, &b.latest_eval) {
            metric_diffs.push(MetricDiff::new("mean_reward", eval_a.overall_mean_reward, eval_b.overall_mean_reward));
            metric_diffs.push(MetricDiff::new("std_reward", eval_a.overall_std_reward, eval_b.overall_std_reward));
            metric_diffs.push(MetricDiff::new("max_drawdown", eval_a.max_drawdown, eval_b.max_drawdown));
            metric_diffs.push(MetricDiff::new("eval_episodes", eval_a.eval_episodes as f64, eval_b.eval_episodes as f64));
        }

        let hyperparameter_diffs = a.hyperparameters.diff(&b.hyperparameters);

        let common_ancestor = find_common_ancestor(a, b);
        let lineage_divergence = LineageDivergence {
            common_ancestor,
            env_changed: a.lineage.env_version != b.lineage.env_version,
            reward_changed: a.lineage.reward_function_hash != b.lineage.reward_function_hash,
            algorithm_changed: a.algorithm != b.algorithm,
            divergence_depth: compute_divergence_depth(a, b),
        };

        let recommendation = generate_comparison_recommendation(&metric_diffs, &lineage_divergence);

        Ok(PolicyComparison {
            policy_a: a.full_id(),
            version_a: version_a.to_string(),
            policy_b: b.full_id(),
            version_b: version_b.to_string(),
            metric_diffs,
            hyperparameter_diffs,
            lineage_divergence,
            recommendation,
        })
    }

    // ── Cross-Framework Export ──────────────────────────────────────────

    pub fn export_bundle(&self, name: &str, version: &str, target_format: ArtifactFormat) -> Result<ExportBundle, HubError> {
        let record = self.get(name, version)
            .ok_or_else(|| HubError::PolicyNotFound(format!("{}:{}", name, version)))?;

        let artifact = record.artifacts.iter()
            .find(|a| a.format == target_format)
            .ok_or_else(|| HubError::ArtifactNotFound {
                policy_id: record.full_id(),
                format: target_format.clone(),
            })?;

        let mut bundle = ExportBundle::new(
            target_format,
            &record.name,
            &record.version.to_string_repr(),
            record.observation_space.clone(),
            record.action_space.clone(),
            &record.lineage.reward_function_hash,
            &record.lineage.env_version,
            record.hyperparameters.clone(),
            &artifact.checksum_sha256,
            artifact.file_size_bytes,
        );

        if let Some(ref stats) = record.normalization_stats {
            bundle = bundle.with_normalization(stats.clone());
        }

        bundle.set_metadata("algorithm", &record.algorithm.label().to_string());
        bundle.set_metadata("author", &record.author);
        bundle.set_metadata("env_name", &record.env_name);
        bundle.set_metadata("training_steps", &record.lineage.training_steps.to_string());

        Ok(bundle)
    }

    // ── Model Card ─────────────────────────────────────────────────────

    pub fn generate_model_card(&self, name: &str, version: &str) -> Result<ModelCard, HubError> {
        let record = self.get(name, version)
            .ok_or_else(|| HubError::PolicyNotFound(format!("{}:{}", name, version)))?;
        Ok(ModelCard::generate(record))
    }

    // ── Retention ──────────────────────────────────────────────────────

    pub fn enforce_retention(&mut self, name: &str, current_time: u64, actor: &str) -> Vec<String> {
        let mut removed = Vec::new();
        if let Some(versions) = self.policies.get_mut(name) {
            // Sort by version descending
            versions.sort_by(|a, b| b.version.compare(&a.version));

            let mut to_remove = Vec::new();
            for (i, record) in versions.iter().enumerate() {
                let age_days = (current_time.saturating_sub(record.created_at)) / 86400;
                let reward = record.latest_eval.as_ref().map(|e| e.overall_mean_reward).unwrap_or(0.0);

                if !self.retention_policy.should_keep(
                    i,
                    record.is_tagged(),
                    record.is_promoted(),
                    reward,
                    age_days,
                ) {
                    to_remove.push(i);
                }
            }

            // Remove in reverse order to preserve indices
            for &idx in to_remove.iter().rev() {
                let record = versions.remove(idx);
                let version_str = record.version.to_string_repr();
                removed.push(format!("{}:{}", name, version_str));
            }

            if versions.is_empty() {
                self.policies.remove(name);
            }
        }

        if !removed.is_empty() {
            let ts = self.next_timestamp();
            self.audit_trail.push(
                AuditEntry::new(ts, name, "*", actor, AuditAction::RetentionCleanup)
                    .with_details(&format!("Removed {} versions", removed.len())),
            );
        }

        removed
    }

    // ── Import/Export Registry ──────────────────────────────────────────

    pub fn export_registry(&self) -> RegistrySnapshot {
        let mut snapshot = RegistrySnapshot::new(&self.instance_id, self.timestamp_counter);
        for versions in self.policies.values() {
            for record in versions {
                snapshot.records.push(record.clone());
            }
        }
        snapshot.audit_entries = self.audit_trail.clone();
        snapshot
    }

    pub fn import_registry(&mut self, snapshot: RegistrySnapshot, actor: &str) -> Result<usize, HubError> {
        if snapshot.format_version != "1.0.0" {
            return Err(HubError::ImportError(format!(
                "unsupported format version: {}",
                snapshot.format_version,
            )));
        }

        let mut imported = 0;
        for record in snapshot.records {
            let key = record.name.clone();
            let version_str = record.version.to_string_repr();

            // Skip duplicates
            let exists = self.policies.get(&key)
                .map(|vs| vs.iter().any(|r| r.version.to_string_repr() == version_str))
                .unwrap_or(false);

            if !exists {
                self.policies.entry(key.clone()).or_insert_with(Vec::new).push(record);
                self.log_audit(&key, &version_str, actor, AuditAction::Import);
                imported += 1;
            }
        }
        Ok(imported)
    }

    // ── Audit ──────────────────────────────────────────────────────────

    pub fn audit_trail(&self) -> &[AuditEntry] {
        &self.audit_trail
    }

    pub fn audit_for_policy(&self, name: &str) -> Vec<&AuditEntry> {
        self.audit_trail.iter().filter(|e| e.policy_id == name).collect()
    }

    pub fn audit_for_actor(&self, actor: &str) -> Vec<&AuditEntry> {
        self.audit_trail.iter().filter(|e| e.actor == actor).collect()
    }

    pub fn audit_summary(&self) -> HashMap<String, usize> {
        let mut counts: HashMap<String, usize> = HashMap::new();
        for entry in &self.audit_trail {
            let key = entry.action.label();
            *counts.entry(key).or_insert(0) += 1;
        }
        counts
    }

    // ── Lineage DAG ────────────────────────────────────────────────────

    pub fn ancestry_chain(&self, name: &str, version: &str) -> Vec<String> {
        let mut chain = Vec::new();
        let mut current_name = name.to_string();
        let mut current_version = version.to_string();

        loop {
            let record = match self.get(&current_name, &current_version) {
                Some(r) => r,
                None => break,
            };
            chain.push(record.full_id());

            if let Some(ref parent) = record.lineage.parent_policy {
                // Parse "name:version" format
                if let Some((pn, pv)) = parent.split_once(':') {
                    current_name = pn.to_string();
                    current_version = pv.to_string();
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        chain
    }

    pub fn descendants(&self, name: &str, version: &str) -> Vec<String> {
        let full_id = format!("{}:{}", name, version);
        let mut result = Vec::new();
        for versions in self.policies.values() {
            for record in versions {
                if let Some(ref parent) = record.lineage.parent_policy {
                    if *parent == full_id {
                        result.push(record.full_id());
                    }
                }
                if let Some(ref source) = record.lineage.distilled_from {
                    if *source == full_id {
                        result.push(record.full_id());
                    }
                }
            }
        }
        result
    }
}

// ── Helper Functions ───────────────────────────────────────────────────

fn find_common_ancestor(a: &PolicyRecord, b: &PolicyRecord) -> Option<String> {
    // Simple heuristic: if both share a parent, that's the common ancestor
    if let (Some(ref pa), Some(ref pb)) = (&a.lineage.parent_policy, &b.lineage.parent_policy) {
        if pa == pb {
            return Some(pa.clone());
        }
    }
    // If one is the parent of the other
    if a.lineage.parent_policy.as_deref() == Some(&b.full_id()) {
        return Some(b.full_id());
    }
    if b.lineage.parent_policy.as_deref() == Some(&a.full_id()) {
        return Some(a.full_id());
    }
    None
}

fn compute_divergence_depth(a: &PolicyRecord, b: &PolicyRecord) -> u32 {
    let mut depth = 0;
    if a.lineage.env_version != b.lineage.env_version {
        depth += 1;
    }
    if a.lineage.reward_function_hash != b.lineage.reward_function_hash {
        depth += 1;
    }
    if a.algorithm != b.algorithm {
        depth += 1;
    }
    if a.lineage.training_config_ref != b.lineage.training_config_ref {
        depth += 1;
    }
    depth
}

fn generate_comparison_recommendation(diffs: &[MetricDiff], divergence: &LineageDivergence) -> String {
    if diffs.is_empty() {
        return "Insufficient evaluation data for comparison".to_string();
    }

    let reward_diff = diffs.iter().find(|d| d.metric_name == "mean_reward");
    let dd_diff = diffs.iter().find(|d| d.metric_name == "max_drawdown");

    let mut rec = String::new();
    if let Some(rd) = reward_diff {
        if rd.improved() {
            rec.push_str(&format!("Policy B shows {:.1}% reward improvement. ", rd.relative_change_pct));
        } else {
            rec.push_str(&format!("Policy A has {:.1}% higher reward. ", rd.relative_change_pct.abs()));
        }
    }

    if let Some(dd) = dd_diff {
        if dd.delta < 0.0 {
            rec.push_str("Policy B has lower drawdown (more stable). ");
        } else if dd.delta > 0.0 {
            rec.push_str("Policy B has higher drawdown (less stable). ");
        }
    }

    if divergence.env_changed {
        rec.push_str("Warning: different environment versions. ");
    }
    if divergence.reward_changed {
        rec.push_str("Warning: different reward functions. ");
    }

    if rec.is_empty() {
        "Policies are comparable with no significant differences".to_string()
    } else {
        rec.trim().to_string()
    }
}

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Test helpers ───────────────────────────────────────────────────

    fn make_hub() -> ModelHub {
        ModelHub::new("test-instance")
    }

    fn make_lineage() -> PolicyLineage {
        PolicyLineage::new("CartPole-v1", "abc123", "config/ppo.yaml")
            .with_steps(100_000)
            .with_wall_time(3600.0)
            .with_seed(42)
    }

    fn make_record(name: &str, version: &str) -> PolicyRecord {
        let ver = SemanticVersion::parse(version).unwrap();
        PolicyRecord::new(
            name,
            ver,
            RlAlgorithm::PPO,
            "tester",
            "CartPole-v1",
            SpaceType::Continuous { dims: 4 },
            SpaceType::Discrete(2),
            Hyperparameters::default_ppo(),
            make_lineage(),
            1000,
        )
    }

    fn make_eval(mean_reward: f64) -> EvaluationResults {
        let mut eval = EvaluationResults::new("eval-1", 2000);
        eval.add_scenario(
            ScenarioResult::new("default", mean_reward, 10.0, 100)
                .with_success_rate(0.9)
                .with_episode_length(200.0),
        );
        eval.add_scenario(
            ScenarioResult::new("hard", mean_reward * 0.8, 15.0, 100)
                .with_success_rate(0.85)
                .with_episode_length(150.0),
        );
        eval.add_scenario(
            ScenarioResult::new("extreme", mean_reward * 0.6, 20.0, 100)
                .with_success_rate(0.82)
                .with_episode_length(120.0),
        );
        eval.set_reward_curve(vec![10.0, 50.0, 80.0, 100.0, 95.0, 110.0, mean_reward]);
        eval.set_safety_report(SafetyReport::passing(0.95, 3));
        eval
    }

    fn make_artifact() -> Artifact {
        Artifact::new(
            ArtifactFormat::PyTorch,
            "/models/policy.pt",
            "sha256:deadbeef",
            1024 * 1024,
            1000,
        )
    }

    fn register_sample(hub: &mut ModelHub) -> String {
        let mut record = make_record("cart-ppo", "1.0.0");
        record.latest_eval = Some(make_eval(200.0));
        record.artifacts.push(make_artifact());
        hub.register(record).unwrap()
    }

    // ── SemanticVersion tests ──────────────────────────────────────────

    #[test]
    fn test_semver_parse() {
        let v = SemanticVersion::parse("1.2.3").unwrap();
        assert_eq!(v.major, 1);
        assert_eq!(v.minor, 2);
        assert_eq!(v.patch, 3);
        assert!(v.pre_release.is_none());
    }

    #[test]
    fn test_semver_parse_pre_release() {
        let v = SemanticVersion::parse("1.0.0-beta.1").unwrap();
        assert_eq!(v.major, 1);
        assert_eq!(v.pre_release, Some("beta.1".to_string()));
    }

    #[test]
    fn test_semver_parse_invalid() {
        assert!(SemanticVersion::parse("not-a-version").is_err());
        assert!(SemanticVersion::parse("1.2").is_err());
        assert!(SemanticVersion::parse("1.2.x").is_err());
    }

    #[test]
    fn test_semver_bump() {
        let v = SemanticVersion::new(1, 2, 3);
        assert_eq!(v.bump_patch().to_string_repr(), "1.2.4");
        assert_eq!(v.bump_minor().to_string_repr(), "1.3.0");
        assert_eq!(v.bump_major().to_string_repr(), "2.0.0");
    }

    #[test]
    fn test_semver_compatible() {
        let a = SemanticVersion::new(1, 0, 0);
        let b = SemanticVersion::new(1, 5, 3);
        let c = SemanticVersion::new(2, 0, 0);
        assert!(a.is_compatible_with(&b));
        assert!(!a.is_compatible_with(&c));
    }

    #[test]
    fn test_semver_compare() {
        let a = SemanticVersion::new(1, 0, 0);
        let b = SemanticVersion::new(1, 1, 0);
        let c = SemanticVersion::new(2, 0, 0);
        assert_eq!(a.compare(&b), std::cmp::Ordering::Less);
        assert_eq!(b.compare(&a), std::cmp::Ordering::Greater);
        assert_eq!(a.compare(&c), std::cmp::Ordering::Less);
    }

    #[test]
    fn test_semver_display() {
        let v = SemanticVersion::new(3, 2, 1).with_pre_release("rc.1");
        assert_eq!(format!("{}", v), "3.2.1-rc.1");
    }

    // ── RlAlgorithm tests ──────────────────────────────────────────────

    #[test]
    fn test_algorithm_classification() {
        assert!(RlAlgorithm::PPO.is_on_policy());
        assert!(!RlAlgorithm::PPO.is_off_policy());
        assert!(RlAlgorithm::SAC.is_off_policy());
        assert!(RlAlgorithm::MuZero.is_model_based());
        assert!(!RlAlgorithm::DQN.is_model_based());
    }

    #[test]
    fn test_algorithm_custom() {
        let algo = RlAlgorithm::Custom("MyAlgo".to_string());
        assert_eq!(algo.label(), "MyAlgo");
        assert!(!algo.is_on_policy());
        assert!(!algo.is_off_policy());
    }

    #[test]
    fn test_algorithm_display() {
        assert_eq!(format!("{}", RlAlgorithm::PPO), "PPO");
        assert_eq!(format!("{}", RlAlgorithm::Dreamer), "Dreamer");
    }

    // ── SpaceType tests ────────────────────────────────────────────────

    #[test]
    fn test_space_flat_size_discrete() {
        assert_eq!(SpaceType::Discrete(5).flat_size(), 1);
    }

    #[test]
    fn test_space_flat_size_continuous() {
        assert_eq!(SpaceType::Continuous { dims: 8 }.flat_size(), 8);
    }

    #[test]
    fn test_space_flat_size_image() {
        let space = SpaceType::Image { height: 84, width: 84, channels: 3 };
        assert_eq!(space.flat_size(), 84 * 84 * 3);
    }

    #[test]
    fn test_space_flat_size_dict() {
        let space = SpaceType::Dict(vec![
            ("pos".into(), SpaceType::Continuous { dims: 3 }),
            ("vel".into(), SpaceType::Continuous { dims: 3 }),
        ]);
        assert_eq!(space.flat_size(), 6);
    }

    #[test]
    fn test_space_flat_size_tuple() {
        let space = SpaceType::Tuple(vec![
            SpaceType::Discrete(4),
            SpaceType::Continuous { dims: 2 },
        ]);
        assert_eq!(space.flat_size(), 3);
    }

    #[test]
    fn test_space_display() {
        assert_eq!(format!("{}", SpaceType::Discrete(5)), "Discrete(5)");
        assert_eq!(format!("{}", SpaceType::Continuous { dims: 4 }), "Box(4)");
        assert_eq!(format!("{}", SpaceType::MultiBinary(8)), "MultiBinary(8)");
    }

    // ── ArtifactFormat tests ───────────────────────────────────────────

    #[test]
    fn test_artifact_format_extension() {
        assert_eq!(ArtifactFormat::PyTorch.file_extension(), ".pt");
        assert_eq!(ArtifactFormat::ONNX.file_extension(), ".onnx");
        assert_eq!(ArtifactFormat::WASM.file_extension(), ".wasm");
        assert_eq!(ArtifactFormat::TFLite.file_extension(), ".tflite");
        assert_eq!(ArtifactFormat::CustomRuntime("myrt".into()).file_extension(), ".bin");
    }

    // ── PromotionStage tests ───────────────────────────────────────────

    #[test]
    fn test_promotion_valid_transitions() {
        assert!(PromotionStage::Development.can_promote_to(&PromotionStage::Staging));
        assert!(PromotionStage::Staging.can_promote_to(&PromotionStage::Canary));
        assert!(PromotionStage::Canary.can_promote_to(&PromotionStage::Production));
        assert!(PromotionStage::Production.can_promote_to(&PromotionStage::Deprecated));
    }

    #[test]
    fn test_promotion_invalid_transitions() {
        assert!(!PromotionStage::Development.can_promote_to(&PromotionStage::Production));
        assert!(!PromotionStage::Staging.can_promote_to(&PromotionStage::Production));
        assert!(!PromotionStage::Deprecated.can_promote_to(&PromotionStage::Staging));
    }

    #[test]
    fn test_promotion_ordinal() {
        assert!(PromotionStage::Development.ordinal() < PromotionStage::Production.ordinal());
        assert!(PromotionStage::Production.ordinal() < PromotionStage::Archived.ordinal());
    }

    // ── NormalizationStats tests ───────────────────────────────────────

    #[test]
    fn test_normalization_new() {
        let stats = NormalizationStats::new(4);
        assert_eq!(stats.dims(), 4);
        assert_eq!(stats.count, 0);
        assert_eq!(stats.running_mean, vec![0.0; 4]);
        assert_eq!(stats.running_std, vec![1.0; 4]);
    }

    #[test]
    fn test_normalization_update() {
        let mut stats = NormalizationStats::new(2);
        stats.update(&[1.0, 2.0]);
        assert_eq!(stats.count, 1);
        assert!((stats.running_mean[0] - 1.0).abs() < 1e-6);
        stats.update(&[3.0, 4.0]);
        assert_eq!(stats.count, 2);
        assert!((stats.running_mean[0] - 2.0).abs() < 1e-6);
    }

    #[test]
    fn test_normalization_normalize() {
        let mut stats = NormalizationStats::new(2);
        for i in 0..100 {
            stats.update(&[i as f64, (i * 2) as f64]);
        }
        let normalized = stats.normalize(&[50.0, 100.0]);
        assert_eq!(normalized.len(), 2);
        // Should be roughly centered around 0
        assert!(normalized[0].abs() < 2.0);
    }

    #[test]
    fn test_normalization_clip() {
        let stats = NormalizationStats::new(1).with_clip_range(1.0);
        // Large value should be clipped
        let result = stats.normalize(&[1000.0]);
        assert!(result[0] <= 1.0);
        assert!(result[0] >= -1.0);
    }

    #[test]
    fn test_normalization_wrong_dims_ignored() {
        let mut stats = NormalizationStats::new(3);
        stats.update(&[1.0, 2.0]); // Wrong dims
        assert_eq!(stats.count, 0); // Should not update
    }

    // ── Hyperparameters tests ──────────────────────────────────────────

    #[test]
    fn test_hyperparams_default_ppo() {
        let hp = Hyperparameters::default_ppo();
        assert!((hp.learning_rate - 3e-4).abs() < 1e-12);
        assert_eq!(hp.batch_size, 64);
        assert!(hp.clip_range.is_some());
    }

    #[test]
    fn test_hyperparams_default_sac() {
        let hp = Hyperparameters::default_sac();
        assert!(hp.tau.is_some());
        assert!(hp.buffer_size.is_some());
    }

    #[test]
    fn test_hyperparams_default_dqn() {
        let hp = Hyperparameters::default_dqn();
        assert!(hp.tau.is_some());
        assert_eq!(hp.batch_size, 32);
    }

    #[test]
    fn test_hyperparams_custom() {
        let mut hp = Hyperparameters::default_ppo();
        hp.set_custom("reward_scale", "0.1");
        assert_eq!(hp.get_custom("reward_scale"), Some("0.1"));
        assert_eq!(hp.get_custom("nonexistent"), None);
    }

    #[test]
    fn test_hyperparams_diff() {
        let a = Hyperparameters::default_ppo();
        let mut b = Hyperparameters::default_ppo();
        b.learning_rate = 1e-3;
        b.batch_size = 128;
        let diffs = a.diff(&b);
        assert!(diffs.iter().any(|(k, _, _)| k == "learning_rate"));
        assert!(diffs.iter().any(|(k, _, _)| k == "batch_size"));
    }

    #[test]
    fn test_hyperparams_no_diff() {
        let a = Hyperparameters::default_ppo();
        let b = Hyperparameters::default_ppo();
        let diffs = a.diff(&b);
        assert!(diffs.is_empty());
    }

    // ── PolicyLineage tests ────────────────────────────────────────────

    #[test]
    fn test_lineage_basic() {
        let lineage = make_lineage();
        assert_eq!(lineage.env_version, "CartPole-v1");
        assert_eq!(lineage.training_steps, 100_000);
        assert!(!lineage.is_fine_tuned());
        assert!(!lineage.is_distilled());
    }

    #[test]
    fn test_lineage_fine_tuned() {
        let lineage = make_lineage().with_parent("base-policy:1.0.0");
        assert!(lineage.is_fine_tuned());
        assert!(!lineage.is_distilled());
    }

    #[test]
    fn test_lineage_distilled() {
        let lineage = make_lineage().with_distilled_from("teacher:2.0.0");
        assert!(!lineage.is_fine_tuned());
        assert!(lineage.is_distilled());
    }

    #[test]
    fn test_lineage_steps_per_second() {
        let lineage = make_lineage();
        let sps = lineage.steps_per_second();
        assert!((sps - 100_000.0 / 3600.0).abs() < 0.1);
    }

    #[test]
    fn test_lineage_shares_env() {
        let a = make_lineage();
        let b = make_lineage();
        assert!(a.shares_env_with(&b));
        let c = PolicyLineage::new("HalfCheetah-v3", "xyz", "config.yaml");
        assert!(!a.shares_env_with(&c));
    }

    #[test]
    fn test_lineage_shares_reward() {
        let a = make_lineage();
        let b = make_lineage();
        assert!(a.shares_reward_with(&b));
    }

    // ── ScenarioResult tests ───────────────────────────────────────────

    #[test]
    fn test_scenario_result_basic() {
        let sr = ScenarioResult::new("test", 100.0, 10.0, 50);
        assert_eq!(sr.scenario_name, "test");
        assert!((sr.mean_reward - 100.0).abs() < 1e-6);
        assert_eq!(sr.episodes, 50);
    }

    #[test]
    fn test_scenario_result_reward_range() {
        let sr = ScenarioResult::new("test", 100.0, 10.0, 50).with_min_max(50.0, 150.0);
        assert!((sr.reward_range() - 100.0).abs() < 1e-6);
    }

    #[test]
    fn test_scenario_cv() {
        let sr = ScenarioResult::new("test", 100.0, 20.0, 50);
        assert!((sr.coefficient_of_variation() - 0.2).abs() < 1e-6);
    }

    #[test]
    fn test_scenario_custom_metrics() {
        let mut sr = ScenarioResult::new("test", 100.0, 10.0, 50);
        sr.set_custom_metric("collision_rate", 0.05);
        assert_eq!(sr.custom_metrics.get("collision_rate"), Some(&0.05));
    }

    // ── EvaluationResults tests ────────────────────────────────────────

    #[test]
    fn test_eval_aggregates() {
        let eval = make_eval(200.0);
        assert!(eval.overall_mean_reward > 0.0);
        assert_eq!(eval.scenario_count(), 3);
    }

    #[test]
    fn test_eval_best_worst_scenario() {
        let eval = make_eval(200.0);
        let best = eval.best_scenario().unwrap();
        let worst = eval.worst_scenario().unwrap();
        assert!(best.mean_reward >= worst.mean_reward);
    }

    #[test]
    fn test_eval_max_drawdown() {
        let mut eval = EvaluationResults::new("eval-dd", 1000);
        eval.set_reward_curve(vec![100.0, 50.0, 80.0, 30.0, 90.0]);
        assert!((eval.max_drawdown - 70.0).abs() < 1e-6); // 100 -> 30
    }

    #[test]
    fn test_eval_safety_passed() {
        let eval = make_eval(200.0);
        assert!(eval.safety_passed());
    }

    #[test]
    fn test_eval_safety_failed() {
        let mut eval = make_eval(200.0);
        eval.set_safety_report(SafetyReport::failing(
            vec!["constraint A violated".into()],
            0.5,
            3,
        ));
        assert!(!eval.safety_passed());
    }

    #[test]
    fn test_eval_empty_safety() {
        let eval = EvaluationResults::new("eval-no-safety", 1000);
        assert!(eval.safety_passed()); // No report = pass
    }

    // ── Artifact tests ─────────────────────────────────────────────────

    #[test]
    fn test_artifact_creation() {
        let art = make_artifact();
        assert_eq!(art.format, ArtifactFormat::PyTorch);
        assert!(art.file_size_mb() > 0.0);
    }

    #[test]
    fn test_artifact_checksum_ok() {
        let art = make_artifact();
        assert!(art.verify_checksum("sha256:deadbeef").is_ok());
    }

    #[test]
    fn test_artifact_checksum_mismatch() {
        let art = make_artifact();
        let err = art.verify_checksum("sha256:wrong").unwrap_err();
        assert!(matches!(err, HubError::ChecksumMismatch { .. }));
    }

    #[test]
    fn test_artifact_metadata() {
        let art = make_artifact()
            .with_metadata("framework_version", "2.1.0");
        assert_eq!(art.metadata.get("framework_version"), Some(&"2.1.0".to_string()));
    }

    // ── QualityGate tests ──────────────────────────────────────────────

    #[test]
    fn test_quality_gate_permissive_passes() {
        let gate = QualityGate::permissive();
        let eval = make_eval(200.0);
        assert!(gate.evaluate(&eval).is_ok());
    }

    #[test]
    fn test_quality_gate_strict_passes_good_eval() {
        let gate = QualityGate::strict();
        let eval = make_eval(200.0);
        assert!(gate.evaluate(&eval).is_ok());
    }

    #[test]
    fn test_quality_gate_min_reward_fail() {
        let gate = QualityGate::permissive().with_min_reward(500.0);
        let eval = make_eval(100.0);
        let err = gate.evaluate(&eval).unwrap_err();
        assert!(err.iter().any(|e| e.contains("mean reward")));
    }

    #[test]
    fn test_quality_gate_max_drawdown_fail() {
        let gate = QualityGate::permissive().with_max_drawdown(1.0);
        let mut eval = make_eval(200.0);
        eval.set_reward_curve(vec![200.0, 50.0]); // drawdown = 150
        let err = gate.evaluate(&eval).unwrap_err();
        assert!(err.iter().any(|e| e.contains("drawdown")));
    }

    #[test]
    fn test_quality_gate_safety_required_missing() {
        let mut gate = QualityGate::permissive();
        gate.require_safety_pass = true;
        let eval = EvaluationResults::new("no-safety", 1000);
        let err = gate.evaluate(&eval).unwrap_err();
        assert!(err.iter().any(|e| e.contains("safety")));
    }

    #[test]
    fn test_quality_gate_safety_required_fail() {
        let mut gate = QualityGate::permissive();
        gate.require_safety_pass = true;
        let mut eval = make_eval(200.0);
        eval.set_safety_report(SafetyReport::failing(vec!["bad".into()], 1.0, 3));
        let err = gate.evaluate(&eval).unwrap_err();
        assert!(err.iter().any(|e| e.contains("safety")));
    }

    #[test]
    fn test_quality_gate_min_episodes() {
        let gate = QualityGate::permissive().with_min_episodes(1000);
        let eval = make_eval(200.0);
        let err = gate.evaluate(&eval).unwrap_err();
        assert!(err.iter().any(|e| e.contains("episodes")));
    }

    #[test]
    fn test_quality_gate_custom_check() {
        let mut gate = QualityGate::permissive();
        gate.add_custom_check("collision_rate", 0.1, QualityComparator::LessThan);
        let mut eval = make_eval(200.0);
        eval.scenario_results[0].set_custom_metric("collision_rate", 0.2);
        let err = gate.evaluate(&eval).unwrap_err();
        assert!(err.iter().any(|e| e.contains("collision_rate")));
    }

    // ── RetentionPolicy tests ──────────────────────────────────────────

    #[test]
    fn test_retention_keep_latest() {
        let policy = RetentionPolicy::default_policy();
        assert!(policy.should_keep(0, false, false, 0.0, 0));
        assert!(policy.should_keep(9, false, false, 0.0, 0));
        assert!(!policy.should_keep(10, false, false, 0.0, 0));
    }

    #[test]
    fn test_retention_keep_tagged() {
        let policy = RetentionPolicy::default_policy();
        assert!(policy.should_keep(100, true, false, 0.0, 0));
    }

    #[test]
    fn test_retention_keep_promoted() {
        let policy = RetentionPolicy::default_policy();
        assert!(policy.should_keep(100, false, true, 0.0, 0));
    }

    #[test]
    fn test_retention_aggressive() {
        let policy = RetentionPolicy::aggressive();
        assert!(policy.should_keep(2, false, false, 0.0, 0));
        assert!(!policy.should_keep(3, false, false, 0.0, 100));
    }

    // ── DeploymentMetadata tests ───────────────────────────────────────

    #[test]
    fn test_deployment_metadata() {
        let dm = DeploymentMetadata::new("triton", "A100")
            .with_latency(1.5, 5.0)
            .with_throughput(10000.0)
            .with_memory(256.0)
            .with_rollback("cart-ppo:0.9.0")
            .with_autoscale(2, 10, "gpu_utilization");
        assert_eq!(dm.serving_framework, "triton");
        assert!((dm.latency_p50_ms - 1.5).abs() < 1e-6);
        assert_eq!(dm.min_replicas, 2);
        assert!(dm.rollback_policy_id.is_some());
    }

    #[test]
    fn test_deployment_latency_ratio() {
        let dm = DeploymentMetadata::new("triton", "A100").with_latency(2.0, 10.0);
        assert!((dm.latency_ratio() - 5.0).abs() < 1e-6);
    }

    // ── Hub Registration tests ─────────────────────────────────────────

    #[test]
    fn test_register_policy() {
        let mut hub = make_hub();
        let id = register_sample(&mut hub);
        assert_eq!(id, "cart-ppo:1.0.0");
        assert_eq!(hub.policy_count(), 1);
    }

    #[test]
    fn test_register_duplicate_version() {
        let mut hub = make_hub();
        register_sample(&mut hub);
        let record = make_record("cart-ppo", "1.0.0");
        let err = hub.register(record).unwrap_err();
        assert!(matches!(err, HubError::VersionExists(_)));
    }

    #[test]
    fn test_register_multiple_versions() {
        let mut hub = make_hub();
        register_sample(&mut hub);
        let record = make_record("cart-ppo", "1.1.0");
        hub.register(record).unwrap();
        assert_eq!(hub.policy_count(), 2);
        assert_eq!(hub.list_versions("cart-ppo").len(), 2);
    }

    #[test]
    fn test_register_quality_gate_blocks() {
        let mut hub = make_hub().with_quality_gate(QualityGate::permissive().with_min_reward(500.0));
        let mut record = make_record("bad-policy", "1.0.0");
        record.latest_eval = Some(make_eval(100.0));
        let err = hub.register(record).unwrap_err();
        assert!(matches!(err, HubError::QualityGateFailed(_)));
    }

    // ── Hub Get tests ──────────────────────────────────────────────────

    #[test]
    fn test_get_policy() {
        let mut hub = make_hub();
        register_sample(&mut hub);
        let record = hub.get("cart-ppo", "1.0.0");
        assert!(record.is_some());
        assert_eq!(record.unwrap().name, "cart-ppo");
    }

    #[test]
    fn test_get_nonexistent() {
        let hub = make_hub();
        assert!(hub.get("no-such", "1.0.0").is_none());
    }

    #[test]
    fn test_get_latest() {
        let mut hub = make_hub();
        register_sample(&mut hub);
        hub.register(make_record("cart-ppo", "2.0.0")).unwrap();
        let latest = hub.get_latest("cart-ppo").unwrap();
        assert_eq!(latest.version.to_string_repr(), "2.0.0");
    }

    // ── Hub Delete tests ───────────────────────────────────────────────

    #[test]
    fn test_delete_policy() {
        let mut hub = make_hub();
        register_sample(&mut hub);
        let deleted = hub.delete("cart-ppo", "1.0.0", "admin").unwrap();
        assert_eq!(deleted.name, "cart-ppo");
        assert_eq!(hub.policy_count(), 0);
    }

    #[test]
    fn test_delete_nonexistent() {
        let mut hub = make_hub();
        let err = hub.delete("no-such", "1.0.0", "admin").unwrap_err();
        assert!(matches!(err, HubError::PolicyNotFound(_)));
    }

    // ── Hub Promotion tests ────────────────────────────────────────────

    #[test]
    fn test_promote_staging() {
        let mut hub = make_hub();
        register_sample(&mut hub);
        hub.promote("cart-ppo", "1.0.0", PromotionStage::Staging, "deployer").unwrap();
        let record = hub.get("cart-ppo", "1.0.0").unwrap();
        assert_eq!(record.stage, PromotionStage::Staging);
    }

    #[test]
    fn test_promote_full_pipeline() {
        let mut hub = make_hub();
        register_sample(&mut hub);
        hub.promote("cart-ppo", "1.0.0", PromotionStage::Staging, "deployer").unwrap();
        hub.promote("cart-ppo", "1.0.0", PromotionStage::Canary, "deployer").unwrap();
        hub.promote("cart-ppo", "1.0.0", PromotionStage::Production, "deployer").unwrap();
        let record = hub.get("cart-ppo", "1.0.0").unwrap();
        assert_eq!(record.stage, PromotionStage::Production);
    }

    #[test]
    fn test_promote_skip_not_allowed() {
        let mut hub = make_hub();
        register_sample(&mut hub);
        let err = hub.promote("cart-ppo", "1.0.0", PromotionStage::Production, "deployer").unwrap_err();
        assert!(matches!(err, HubError::InvalidPromotion { .. }));
    }

    #[test]
    fn test_promote_requires_approval() {
        let mut hub = make_hub();
        register_sample(&mut hub);
        hub.set_approvers(PromotionStage::Staging, vec!["admin".to_string()]);
        let err = hub.promote("cart-ppo", "1.0.0", PromotionStage::Staging, "unauthorized").unwrap_err();
        assert!(matches!(err, HubError::ApprovalRequired(_)));
    }

    #[test]
    fn test_deprecate() {
        let mut hub = make_hub();
        register_sample(&mut hub);
        hub.deprecate("cart-ppo", "1.0.0", "admin").unwrap();
        let record = hub.get("cart-ppo", "1.0.0").unwrap();
        assert_eq!(record.stage, PromotionStage::Deprecated);
    }

    // ── Hub Tags tests ─────────────────────────────────────────────────

    #[test]
    fn test_add_tag() {
        let mut hub = make_hub();
        register_sample(&mut hub);
        hub.add_tag("cart-ppo", "1.0.0", "best", "tester").unwrap();
        let record = hub.get("cart-ppo", "1.0.0").unwrap();
        assert!(record.tags.contains(&"best".to_string()));
    }

    #[test]
    fn test_add_duplicate_tag() {
        let mut hub = make_hub();
        register_sample(&mut hub);
        hub.add_tag("cart-ppo", "1.0.0", "best", "tester").unwrap();
        let err = hub.add_tag("cart-ppo", "1.0.0", "best", "tester").unwrap_err();
        assert!(matches!(err, HubError::DuplicateTag(_)));
    }

    #[test]
    fn test_remove_tag() {
        let mut hub = make_hub();
        register_sample(&mut hub);
        hub.add_tag("cart-ppo", "1.0.0", "best", "tester").unwrap();
        hub.remove_tag("cart-ppo", "1.0.0", "best", "tester").unwrap();
        let record = hub.get("cart-ppo", "1.0.0").unwrap();
        assert!(!record.tags.contains(&"best".to_string()));
    }

    // ── Hub Search tests ───────────────────────────────────────────────

    #[test]
    fn test_search_by_name() {
        let mut hub = make_hub();
        register_sample(&mut hub);
        hub.register(make_record("lunar-sac", "1.0.0")).unwrap();
        let results = hub.search(&SearchCriteria::new().with_name("cart"));
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "cart-ppo");
    }

    #[test]
    fn test_search_by_env() {
        let mut hub = make_hub();
        register_sample(&mut hub);
        let results = hub.search_by_env("CartPole-v1");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_search_by_algorithm() {
        let mut hub = make_hub();
        register_sample(&mut hub);
        let results = hub.search_by_algorithm(RlAlgorithm::PPO);
        assert_eq!(results.len(), 1);
        let results = hub.search_by_algorithm(RlAlgorithm::SAC);
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_search_by_tag() {
        let mut hub = make_hub();
        register_sample(&mut hub);
        hub.add_tag("cart-ppo", "1.0.0", "production-ready", "tester").unwrap();
        let results = hub.search_by_tag("production-ready");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_search_by_author() {
        let mut hub = make_hub();
        register_sample(&mut hub);
        let results = hub.search(&SearchCriteria::new().with_author("tester"));
        assert_eq!(results.len(), 1);
        let results = hub.search(&SearchCriteria::new().with_author("nobody"));
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_search_by_stage() {
        let mut hub = make_hub();
        register_sample(&mut hub);
        hub.promote("cart-ppo", "1.0.0", PromotionStage::Staging, "admin").unwrap();
        let results = hub.search(&SearchCriteria::new().with_stage(PromotionStage::Staging));
        assert_eq!(results.len(), 1);
        let results = hub.search(&SearchCriteria::new().with_stage(PromotionStage::Production));
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_search_by_date_range() {
        let mut hub = make_hub();
        register_sample(&mut hub);
        let results = hub.search(&SearchCriteria::new().with_date_range(500, 1500));
        assert_eq!(results.len(), 1);
        let results = hub.search(&SearchCriteria::new().with_date_range(2000, 3000));
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_search_by_artifact_format() {
        let mut hub = make_hub();
        register_sample(&mut hub);
        let mut criteria = SearchCriteria::new();
        criteria.has_artifact_format = Some(ArtifactFormat::PyTorch);
        assert_eq!(hub.search(&criteria).len(), 1);
        criteria.has_artifact_format = Some(ArtifactFormat::ONNX);
        assert_eq!(hub.search(&criteria).len(), 0);
    }

    // ── Hub Artifacts tests ────────────────────────────────────────────

    #[test]
    fn test_add_artifact() {
        let mut hub = make_hub();
        register_sample(&mut hub);
        let onnx = Artifact::new(ArtifactFormat::ONNX, "/models/policy.onnx", "sha256:cafe", 2048, 2000);
        hub.add_artifact("cart-ppo", "1.0.0", onnx, "tester").unwrap();
        let record = hub.get("cart-ppo", "1.0.0").unwrap();
        assert_eq!(record.artifacts.len(), 2);
    }

    #[test]
    fn test_get_artifact() {
        let mut hub = make_hub();
        register_sample(&mut hub);
        let art = hub.get_artifact("cart-ppo", "1.0.0", &ArtifactFormat::PyTorch).unwrap();
        assert_eq!(art.format, ArtifactFormat::PyTorch);
    }

    #[test]
    fn test_get_artifact_not_found() {
        let mut hub = make_hub();
        register_sample(&mut hub);
        let err = hub.get_artifact("cart-ppo", "1.0.0", &ArtifactFormat::WASM).unwrap_err();
        assert!(matches!(err, HubError::ArtifactNotFound { .. }));
    }

    // ── Hub Evaluation tests ───────────────────────────────────────────

    #[test]
    fn test_add_eval() {
        let mut hub = make_hub();
        hub.register(make_record("test-pol", "1.0.0")).unwrap();
        let eval = make_eval(300.0);
        hub.add_eval("test-pol", "1.0.0", eval, "evaluator").unwrap();
        let record = hub.get("test-pol", "1.0.0").unwrap();
        assert!(record.latest_eval.is_some());
        assert_eq!(record.eval_history.len(), 1);
    }

    // ── Hub Comparison tests ───────────────────────────────────────────

    #[test]
    fn test_compare_policies() {
        let mut hub = make_hub();
        let mut r1 = make_record("cart-ppo", "1.0.0");
        r1.latest_eval = Some(make_eval(200.0));
        hub.register(r1).unwrap();

        let mut r2 = make_record("cart-ppo", "2.0.0");
        r2.latest_eval = Some(make_eval(300.0));
        hub.register(r2).unwrap();

        let comparison = hub.compare("cart-ppo", "1.0.0", "cart-ppo", "2.0.0").unwrap();
        assert!(!comparison.metric_diffs.is_empty());
        let reward_diff = comparison.metric_diffs.iter().find(|d| d.metric_name == "mean_reward").unwrap();
        assert!(reward_diff.improved());
    }

    #[test]
    fn test_compare_nonexistent() {
        let hub = make_hub();
        let err = hub.compare("no", "1.0.0", "no", "2.0.0").unwrap_err();
        assert!(matches!(err, HubError::PolicyNotFound(_)));
    }

    #[test]
    fn test_compare_lineage_divergence() {
        let mut hub = make_hub();
        let mut r1 = make_record("ppo-v1", "1.0.0");
        r1.latest_eval = Some(make_eval(200.0));
        hub.register(r1).unwrap();

        let mut r2 = make_record("sac-v1", "1.0.0");
        r2.algorithm = RlAlgorithm::SAC;
        r2.lineage = PolicyLineage::new("HalfCheetah-v3", "different_hash", "config/sac.yaml");
        r2.latest_eval = Some(make_eval(150.0));
        hub.register(r2).unwrap();

        let comparison = hub.compare("ppo-v1", "1.0.0", "sac-v1", "1.0.0").unwrap();
        assert!(comparison.lineage_divergence.env_changed);
        assert!(comparison.lineage_divergence.reward_changed);
        assert!(comparison.lineage_divergence.algorithm_changed);
    }

    // ── Hub Export Bundle tests ────────────────────────────────────────

    #[test]
    fn test_export_bundle() {
        let mut hub = make_hub();
        register_sample(&mut hub);
        let bundle = hub.export_bundle("cart-ppo", "1.0.0", ArtifactFormat::PyTorch).unwrap();
        assert_eq!(bundle.policy_name, "cart-ppo");
        assert!(bundle.is_reproducible());
        assert!(bundle.export_metadata.contains_key("algorithm"));
    }

    #[test]
    fn test_export_bundle_missing_format() {
        let mut hub = make_hub();
        register_sample(&mut hub);
        let err = hub.export_bundle("cart-ppo", "1.0.0", ArtifactFormat::WASM).unwrap_err();
        assert!(matches!(err, HubError::ArtifactNotFound { .. }));
    }

    #[test]
    fn test_export_bundle_with_normalization() {
        let mut hub = make_hub();
        let mut record = make_record("cart-ppo", "1.0.0");
        record.normalization_stats = Some(NormalizationStats::new(4));
        record.artifacts.push(make_artifact());
        hub.register(record).unwrap();
        let bundle = hub.export_bundle("cart-ppo", "1.0.0", ArtifactFormat::PyTorch).unwrap();
        assert!(bundle.normalization_stats.is_some());
    }

    // ── Hub Model Card tests ───────────────────────────────────────────

    #[test]
    fn test_generate_model_card() {
        let mut hub = make_hub();
        register_sample(&mut hub);
        let card = hub.generate_model_card("cart-ppo", "1.0.0").unwrap();
        assert_eq!(card.policy_name, "cart-ppo");
        assert_eq!(card.algorithm, RlAlgorithm::PPO);
        assert!(card.eval_summary.contains("Mean reward"));
    }

    #[test]
    fn test_model_card_markdown() {
        let mut hub = make_hub();
        register_sample(&mut hub);
        let card = hub.generate_model_card("cart-ppo", "1.0.0").unwrap();
        let md = card.to_markdown();
        assert!(md.contains("# Model Card"));
        assert!(md.contains("PPO"));
        assert!(md.contains("CartPole"));
        assert!(md.contains("## Evaluation"));
    }

    #[test]
    fn test_model_card_no_eval() {
        let mut hub = make_hub();
        hub.register(make_record("no-eval", "1.0.0")).unwrap();
        let card = hub.generate_model_card("no-eval", "1.0.0").unwrap();
        assert!(card.eval_summary.contains("No evaluation"));
    }

    // ── Hub Retention tests ────────────────────────────────────────────

    #[test]
    fn test_enforce_retention() {
        let mut hub = make_hub().with_retention_policy(RetentionPolicy {
            keep_latest_n: 2,
            keep_tagged: false,
            keep_promoted: false,
            keep_min_reward_above: None,
            max_age_days: None,
        });
        for i in 0..5 {
            hub.register(make_record("pol", &format!("1.{}.0", i))).unwrap();
        }
        let removed = hub.enforce_retention("pol", 100_000, "cleaner");
        assert_eq!(removed.len(), 3);
        assert_eq!(hub.list_versions("pol").len(), 2);
    }

    #[test]
    fn test_enforce_retention_keeps_tagged() {
        let mut hub = make_hub().with_retention_policy(RetentionPolicy {
            keep_latest_n: 1,
            keep_tagged: true,
            keep_promoted: false,
            keep_min_reward_above: None,
            max_age_days: None,
        });
        for i in 0..3 {
            hub.register(make_record("pol", &format!("1.{}.0", i))).unwrap();
        }
        hub.add_tag("pol", "1.0.0", "important", "admin").unwrap();
        let removed = hub.enforce_retention("pol", 100_000, "cleaner");
        assert_eq!(removed.len(), 1); // 1.1.0 removed, 1.0.0 kept (tagged), 1.2.0 kept (latest)
        assert!(hub.get("pol", "1.0.0").is_some());
        assert!(hub.get("pol", "1.2.0").is_some());
    }

    // ── Hub Import/Export tests ────────────────────────────────────────

    #[test]
    fn test_export_registry() {
        let mut hub = make_hub();
        register_sample(&mut hub);
        let snapshot = hub.export_registry();
        assert_eq!(snapshot.policy_count(), 1);
        assert!(snapshot.audit_count() > 0);
        assert_eq!(snapshot.source_instance, "test-instance");
    }

    #[test]
    fn test_import_registry() {
        let mut hub1 = make_hub();
        register_sample(&mut hub1);
        let snapshot = hub1.export_registry();

        let mut hub2 = ModelHub::new("instance-2");
        let imported = hub2.import_registry(snapshot, "importer").unwrap();
        assert_eq!(imported, 1);
        assert_eq!(hub2.policy_count(), 1);
    }

    #[test]
    fn test_import_skips_duplicates() {
        let mut hub = make_hub();
        register_sample(&mut hub);
        let snapshot = hub.export_registry();
        let imported = hub.import_registry(snapshot, "importer").unwrap();
        assert_eq!(imported, 0);
    }

    #[test]
    fn test_import_invalid_format() {
        let mut hub = make_hub();
        let mut snapshot = RegistrySnapshot::new("other", 1000);
        snapshot.format_version = "99.0.0".to_string();
        let err = hub.import_registry(snapshot, "importer").unwrap_err();
        assert!(matches!(err, HubError::ImportError(_)));
    }

    // ── Hub Audit tests ────────────────────────────────────────────────

    #[test]
    fn test_audit_trail_recorded() {
        let mut hub = make_hub();
        register_sample(&mut hub);
        assert!(!hub.audit_trail().is_empty());
        let entries = hub.audit_for_policy("cart-ppo");
        assert!(!entries.is_empty());
    }

    #[test]
    fn test_audit_for_actor() {
        let mut hub = make_hub();
        register_sample(&mut hub);
        let entries = hub.audit_for_actor("tester");
        assert!(!entries.is_empty());
    }

    #[test]
    fn test_audit_summary() {
        let mut hub = make_hub();
        register_sample(&mut hub);
        hub.promote("cart-ppo", "1.0.0", PromotionStage::Staging, "admin").unwrap();
        let summary = hub.audit_summary();
        assert!(summary.contains_key("register"));
    }

    #[test]
    fn test_audit_entry_summary_format() {
        let entry = AuditEntry::new(100, "my-policy", "1.0.0", "admin", AuditAction::Register);
        let s = entry.summary();
        assert!(s.contains("register"));
        assert!(s.contains("my-policy"));
        assert!(s.contains("admin"));
    }

    // ── Hub Lineage DAG tests ──────────────────────────────────────────

    #[test]
    fn test_ancestry_chain() {
        let mut hub = make_hub();
        hub.register(make_record("base", "1.0.0")).unwrap();

        let mut child = make_record("base", "2.0.0");
        child.lineage = child.lineage.with_parent("base:1.0.0");
        hub.register(child).unwrap();

        let mut grandchild = make_record("base", "3.0.0");
        grandchild.lineage = grandchild.lineage.with_parent("base:2.0.0");
        hub.register(grandchild).unwrap();

        let chain = hub.ancestry_chain("base", "3.0.0");
        assert_eq!(chain.len(), 3);
        assert_eq!(chain[0], "base:3.0.0");
        assert_eq!(chain[1], "base:2.0.0");
        assert_eq!(chain[2], "base:1.0.0");
    }

    #[test]
    fn test_descendants() {
        let mut hub = make_hub();
        hub.register(make_record("parent", "1.0.0")).unwrap();

        let mut c1 = make_record("child-a", "1.0.0");
        c1.lineage = c1.lineage.with_parent("parent:1.0.0");
        hub.register(c1).unwrap();

        let mut c2 = make_record("child-b", "1.0.0");
        c2.lineage = c2.lineage.with_distilled_from("parent:1.0.0");
        hub.register(c2).unwrap();

        let desc = hub.descendants("parent", "1.0.0");
        assert_eq!(desc.len(), 2);
    }

    // ── Hub Deployment tests ───────────────────────────────────────────

    #[test]
    fn test_set_deployment() {
        let mut hub = make_hub();
        register_sample(&mut hub);
        let dm = DeploymentMetadata::new("triton", "A100");
        hub.set_deployment("cart-ppo", "1.0.0", dm, "deployer").unwrap();
        let record = hub.get("cart-ppo", "1.0.0").unwrap();
        assert!(record.deployment.is_some());
    }

    // ── MetricDiff tests ───────────────────────────────────────────────

    #[test]
    fn test_metric_diff_improved() {
        let diff = MetricDiff::new("reward", 100.0, 150.0);
        assert!(diff.improved());
        assert!(diff.significant(10.0));
    }

    #[test]
    fn test_metric_diff_regressed() {
        let diff = MetricDiff::new("reward", 100.0, 80.0);
        assert!(!diff.improved());
        assert!((diff.relative_change_pct - (-20.0)).abs() < 1e-6);
    }

    #[test]
    fn test_metric_diff_insignificant() {
        let diff = MetricDiff::new("reward", 100.0, 100.5);
        assert!(!diff.significant(1.0));
    }

    // ── ExportBundle tests ─────────────────────────────────────────────

    #[test]
    fn test_export_bundle_reproducible() {
        let bundle = ExportBundle::new(
            ArtifactFormat::ONNX,
            "test-pol",
            "1.0.0",
            SpaceType::Continuous { dims: 4 },
            SpaceType::Discrete(2),
            "abc123",
            "CartPole-v1",
            Hyperparameters::default_ppo(),
            "sha256:aaa",
            1024,
        );
        assert!(bundle.is_reproducible());
    }

    #[test]
    fn test_export_bundle_not_reproducible() {
        let bundle = ExportBundle::new(
            ArtifactFormat::ONNX,
            "test-pol",
            "1.0.0",
            SpaceType::Continuous { dims: 4 },
            SpaceType::Discrete(2),
            "",
            "CartPole-v1",
            Hyperparameters::default_ppo(),
            "sha256:aaa",
            1024,
        );
        assert!(!bundle.is_reproducible());
    }

    #[test]
    fn test_export_bundle_summary() {
        let bundle = ExportBundle::new(
            ArtifactFormat::ONNX,
            "test-pol",
            "1.0.0",
            SpaceType::Continuous { dims: 4 },
            SpaceType::Discrete(2),
            "abc",
            "v1",
            Hyperparameters::default_ppo(),
            "sha256:aaa",
            1024,
        );
        let summary = bundle.summary();
        assert!(summary.contains("test-pol"));
        assert!(summary.contains("onnx"));
    }

    // ── SafetyReport tests ─────────────────────────────────────────────

    #[test]
    fn test_safety_report_passing() {
        let report = SafetyReport::passing(0.95, 10);
        assert!(report.passed);
        assert!(report.constraint_violations.is_empty());
    }

    #[test]
    fn test_safety_report_failing() {
        let report = SafetyReport::failing(
            vec!["collision too frequent".into(), "energy exceeded".into()],
            0.8,
            5,
        );
        assert!(!report.passed);
        assert_eq!(report.constraint_violations.len(), 2);
    }

    // ── PolicyRecord tests ─────────────────────────────────────────────

    #[test]
    fn test_policy_record_full_id() {
        let record = make_record("my-policy", "1.2.3");
        assert_eq!(record.full_id(), "my-policy:1.2.3");
    }

    #[test]
    fn test_policy_record_tags() {
        let mut record = make_record("my-policy", "1.0.0");
        record.add_tag("v1").unwrap();
        assert!(record.is_tagged());
        assert!(record.remove_tag("v1"));
        assert!(!record.is_tagged());
    }

    #[test]
    fn test_policy_record_promoted() {
        let mut record = make_record("my-policy", "1.0.0");
        assert!(!record.is_promoted());
        record.stage = PromotionStage::Production;
        assert!(record.is_promoted());
    }

    // ── compute_max_drawdown tests ─────────────────────────────────────

    #[test]
    fn test_max_drawdown_empty() {
        assert!((compute_max_drawdown(&[]) - 0.0).abs() < 1e-12);
    }

    #[test]
    fn test_max_drawdown_monotonic_increase() {
        assert!((compute_max_drawdown(&[1.0, 2.0, 3.0, 4.0]) - 0.0).abs() < 1e-12);
    }

    #[test]
    fn test_max_drawdown_single_drop() {
        assert!((compute_max_drawdown(&[100.0, 50.0]) - 50.0).abs() < 1e-12);
    }

    #[test]
    fn test_max_drawdown_complex() {
        let curve = vec![10.0, 20.0, 15.0, 25.0, 5.0, 30.0];
        // Peak 25 -> 5 = drawdown 20
        assert!((compute_max_drawdown(&curve) - 20.0).abs() < 1e-12);
    }

    // ── HubError Display tests ─────────────────────────────────────────

    #[test]
    fn test_hub_error_display() {
        let err = HubError::PolicyNotFound("test".into());
        assert!(format!("{}", err).contains("test"));
        let err = HubError::QualityGateFailed(vec!["low reward".into()]);
        assert!(format!("{}", err).contains("low reward"));
    }

    // ── SearchCriteria combination test ────────────────────────────────

    #[test]
    fn test_search_combined_criteria() {
        let mut hub = make_hub();
        let mut r1 = make_record("cart-ppo", "1.0.0");
        r1.latest_eval = Some(make_eval(200.0));
        hub.register(r1).unwrap();
        hub.add_tag("cart-ppo", "1.0.0", "stable", "tester").unwrap();

        let mut r2 = make_record("lunar-sac", "1.0.0");
        r2.algorithm = RlAlgorithm::SAC;
        r2.env_name = "LunarLander-v2".to_string();
        hub.register(r2).unwrap();

        let criteria = SearchCriteria::new()
            .with_env("CartPole-v1")
            .with_algorithm(RlAlgorithm::PPO)
            .with_tag("stable");
        let results = hub.search(&criteria);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "cart-ppo");
    }

    // ── AuditAction label test ─────────────────────────────────────────

    #[test]
    fn test_audit_action_labels() {
        assert_eq!(AuditAction::Register.label(), "register");
        assert_eq!(AuditAction::Delete.label(), "delete");
        let promote = AuditAction::Promote {
            from: PromotionStage::Staging,
            to: PromotionStage::Canary,
        };
        assert!(promote.label().contains("staging"));
        assert!(promote.label().contains("canary"));
    }

    // ── List policies test ─────────────────────────────────────────────

    #[test]
    fn test_list_policies() {
        let mut hub = make_hub();
        register_sample(&mut hub);
        hub.register(make_record("other", "1.0.0")).unwrap();
        let policies = hub.list_policies();
        assert_eq!(policies.len(), 2);
    }

    // ── RegistrySnapshot test ──────────────────────────────────────────

    #[test]
    fn test_registry_snapshot_counts() {
        let mut snapshot = RegistrySnapshot::new("inst-1", 5000);
        assert_eq!(snapshot.policy_count(), 0);
        assert_eq!(snapshot.audit_count(), 0);
        snapshot.records.push(make_record("test", "1.0.0"));
        assert_eq!(snapshot.policy_count(), 1);
    }
}
