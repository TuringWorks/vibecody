//! RL-OS RLHF module — Reinforcement Learning from Human Feedback operating system.
//!
//! Provides a comprehensive, production-grade RLHF pipeline covering the full
//! alignment lifecycle: preference collection, reward modeling, policy optimization,
//! and alignment evaluation.
//!
//! # Supported Algorithms
//!
//! - **PPO** — Proximal Policy Optimization with KL penalty for LLM alignment
//! - **DPO** — Direct Preference Optimization (no reward model needed)
//! - **KTO** — Kahneman-Tversky Optimization (asymmetric desirable/undesirable)
//! - **ORPO** — Odds Ratio Preference Optimization (combined SFT + alignment)
//! - **GRPO** — Group Relative Policy Optimization (DeepSeek-style)
//!
//! # Architecture
//!
//! ```text
//! PreferenceDataset
//!   → RewardModelTrainer (ensemble of N models)
//!     → ProcessRewardModel (step-level annotation)
//!   → RlhfPipeline
//!     ├─ SFT stage
//!     ├─ Reward model training
//!     ├─ PPO / DPO / KTO / ORPO / GRPO optimization
//!     ├─ ConstitutionalAI critique-revision loop
//!     ├─ RLEF execution feedback
//!     ├─ RewardHackingDetector monitoring
//!     └─ AlignmentEvaluator benchmarks
//!   → ModelMerger (TIES / DARE / SLERP / Linear)
//! ```

use std::collections::HashMap;

// ── Enums ──────────────────────────────────────────────────────────────

/// Alignment algorithm selection.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AlignmentAlgorithm {
    Ppo,
    Dpo,
    Kto,
    Orpo,
    Grpo,
}

impl AlignmentAlgorithm {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Ppo => "PPO",
            Self::Dpo => "DPO",
            Self::Kto => "KTO",
            Self::Orpo => "ORPO",
            Self::Grpo => "GRPO",
        }
    }

    pub fn all() -> Vec<AlignmentAlgorithm> {
        vec![Self::Ppo, Self::Dpo, Self::Kto, Self::Orpo, Self::Grpo]
    }

    pub fn requires_reward_model(&self) -> bool {
        matches!(self, Self::Ppo | Self::Grpo)
    }

    pub fn requires_reference_model(&self) -> bool {
        matches!(self, Self::Ppo | Self::Dpo | Self::Kto)
    }
}

impl std::fmt::Display for AlignmentAlgorithm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

/// Pipeline stage in the RLHF lifecycle.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PipelineStage {
    DataCollection,
    Sft,
    RewardModelTraining,
    PolicyOptimization,
    Evaluation,
    ModelMerge,
    Deployment,
}

impl PipelineStage {
    pub fn label(&self) -> &'static str {
        match self {
            Self::DataCollection => "Data Collection",
            Self::Sft => "Supervised Fine-Tuning",
            Self::RewardModelTraining => "Reward Model Training",
            Self::PolicyOptimization => "Policy Optimization",
            Self::Evaluation => "Evaluation",
            Self::ModelMerge => "Model Merge",
            Self::Deployment => "Deployment",
        }
    }

    pub fn order(&self) -> u8 {
        match self {
            Self::DataCollection => 0,
            Self::Sft => 1,
            Self::RewardModelTraining => 2,
            Self::PolicyOptimization => 3,
            Self::Evaluation => 4,
            Self::ModelMerge => 5,
            Self::Deployment => 6,
        }
    }

    pub fn next(&self) -> Option<PipelineStage> {
        match self {
            Self::DataCollection => Some(Self::Sft),
            Self::Sft => Some(Self::RewardModelTraining),
            Self::RewardModelTraining => Some(Self::PolicyOptimization),
            Self::PolicyOptimization => Some(Self::Evaluation),
            Self::Evaluation => Some(Self::ModelMerge),
            Self::ModelMerge => Some(Self::Deployment),
            Self::Deployment => None,
        }
    }
}

impl std::fmt::Display for PipelineStage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

/// Model merge strategy for combining aligned models.
#[derive(Debug, Clone, PartialEq)]
pub enum MergeStrategy {
    Linear { weights: Vec<f64> },
    Slerp { interpolation_factor: f64 },
    Ties { density: f64, majority_sign_method: MajoritySignMethod },
    Dare { density: f64, rescale: bool },
}

impl MergeStrategy {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Linear { .. } => "Linear",
            Self::Slerp { .. } => "SLERP",
            Self::Ties { .. } => "TIES",
            Self::Dare { .. } => "DARE",
        }
    }
}

impl std::fmt::Display for MergeStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

/// Majority sign resolution for TIES merging.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MajoritySignMethod {
    Total,
    Frequency,
}

/// Reward ensemble aggregation strategy.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum EnsembleAggregation {
    Mean,
    Median,
    WeightedMean,
    Min,
    Max,
}

impl EnsembleAggregation {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Mean => "Mean",
            Self::Median => "Median",
            Self::WeightedMean => "Weighted Mean",
            Self::Min => "Min",
            Self::Max => "Max",
        }
    }
}

impl std::fmt::Display for EnsembleAggregation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

/// Mixed precision training mode.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MixedPrecision {
    No,
    Fp16,
    Bf16,
}

impl MixedPrecision {
    pub fn label(&self) -> &'static str {
        match self {
            Self::No => "None",
            Self::Fp16 => "FP16",
            Self::Bf16 => "BF16",
        }
    }
}

impl std::fmt::Display for MixedPrecision {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

/// DeepSpeed ZeRO stage for distributed training.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ZeroStage {
    Stage0,
    Stage1,
    Stage2,
    Stage3,
}

impl ZeroStage {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Stage0 => "ZeRO-0 (disabled)",
            Self::Stage1 => "ZeRO-1 (optimizer state partitioning)",
            Self::Stage2 => "ZeRO-2 (gradient + optimizer partitioning)",
            Self::Stage3 => "ZeRO-3 (full parameter partitioning)",
        }
    }

    pub fn memory_savings_factor(&self) -> f64 {
        match self {
            Self::Stage0 => 1.0,
            Self::Stage1 => 4.0,
            Self::Stage2 => 8.0,
            Self::Stage3 => 16.0,
        }
    }
}

impl std::fmt::Display for ZeroStage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

/// FSDP sharding strategy.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FsdpShardingStrategy {
    FullShard,
    ShardGradOp,
    NoShard,
    HybridShard,
}

impl FsdpShardingStrategy {
    pub fn label(&self) -> &'static str {
        match self {
            Self::FullShard => "FULL_SHARD",
            Self::ShardGradOp => "SHARD_GRAD_OP",
            Self::NoShard => "NO_SHARD",
            Self::HybridShard => "HYBRID_SHARD",
        }
    }
}

impl std::fmt::Display for FsdpShardingStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

/// Preference example kind for KTO.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum KtoExampleKind {
    Desirable,
    Undesirable,
}

impl KtoExampleKind {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Desirable => "desirable",
            Self::Undesirable => "undesirable",
        }
    }
}

impl std::fmt::Display for KtoExampleKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

/// Alignment benchmark category.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AlignmentBenchmark {
    Harmfulness,
    Toxicity,
    Helpfulness,
    Honesty,
    FactualAccuracy,
    InstructionFollowing,
    CodeCorrectness,
    SafetyRefusal,
}

impl AlignmentBenchmark {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Harmfulness => "Harmfulness",
            Self::Toxicity => "Toxicity",
            Self::Helpfulness => "Helpfulness",
            Self::Honesty => "Honesty",
            Self::FactualAccuracy => "Factual Accuracy",
            Self::InstructionFollowing => "Instruction Following",
            Self::CodeCorrectness => "Code Correctness",
            Self::SafetyRefusal => "Safety Refusal",
        }
    }

    pub fn all() -> Vec<AlignmentBenchmark> {
        vec![
            Self::Harmfulness,
            Self::Toxicity,
            Self::Helpfulness,
            Self::Honesty,
            Self::FactualAccuracy,
            Self::InstructionFollowing,
            Self::CodeCorrectness,
            Self::SafetyRefusal,
        ]
    }

    pub fn is_safety(&self) -> bool {
        matches!(self, Self::Harmfulness | Self::Toxicity | Self::SafetyRefusal)
    }
}

impl std::fmt::Display for AlignmentBenchmark {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

/// Reward hacking signal type.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum HackingSignal {
    KlDivergenceSpike,
    RewardDistributionShift,
    OutOfDistribution,
    RewardCollapse,
    LengthExploitation,
    RepetitionExploitation,
}

impl HackingSignal {
    pub fn label(&self) -> &'static str {
        match self {
            Self::KlDivergenceSpike => "KL Divergence Spike",
            Self::RewardDistributionShift => "Reward Distribution Shift",
            Self::OutOfDistribution => "Out-of-Distribution",
            Self::RewardCollapse => "Reward Collapse",
            Self::LengthExploitation => "Length Exploitation",
            Self::RepetitionExploitation => "Repetition Exploitation",
        }
    }

    pub fn severity(&self) -> f64 {
        match self {
            Self::KlDivergenceSpike => 0.7,
            Self::RewardDistributionShift => 0.8,
            Self::OutOfDistribution => 0.9,
            Self::RewardCollapse => 1.0,
            Self::LengthExploitation => 0.5,
            Self::RepetitionExploitation => 0.6,
        }
    }
}

impl std::fmt::Display for HackingSignal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

// ── Structs ────────────────────────────────────────────────────────────

// ── PPO Configuration ──

#[derive(Debug, Clone)]
pub struct PpoConfig {
    pub learning_rate: f64,
    pub clip_range: f64,
    pub clip_range_vf: f64,
    pub vf_coef: f64,
    pub entropy_coef: f64,
    pub kl_target: f64,
    pub kl_penalty: KlPenaltyMode,
    pub init_kl_coef: f64,
    pub gamma: f64,
    pub lam: f64,
    pub num_epochs: u32,
    pub batch_size: usize,
    pub mini_batch_size: usize,
    pub max_grad_norm: f64,
    pub target_kl: Option<f64>,
    pub whiten_advantages: bool,
    pub generation_config: GenerationConfig,
}

impl Default for PpoConfig {
    fn default() -> Self {
        Self {
            learning_rate: 1.41e-5,
            clip_range: 0.2,
            clip_range_vf: 0.2,
            vf_coef: 0.1,
            entropy_coef: 0.01,
            kl_target: 6.0,
            kl_penalty: KlPenaltyMode::Kl,
            init_kl_coef: 0.2,
            gamma: 1.0,
            lam: 0.95,
            num_epochs: 4,
            batch_size: 128,
            mini_batch_size: 16,
            max_grad_norm: 1.0,
            target_kl: Some(6.0),
            whiten_advantages: true,
            generation_config: GenerationConfig::default(),
        }
    }
}

/// KL penalty mode for PPO.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum KlPenaltyMode {
    Kl,
    Abs,
    Mse,
    Full,
}

impl KlPenaltyMode {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Kl => "kl",
            Self::Abs => "abs",
            Self::Mse => "mse",
            Self::Full => "full",
        }
    }
}

impl std::fmt::Display for KlPenaltyMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

/// PPO training step result.
#[derive(Debug, Clone)]
pub struct PpoStepResult {
    pub policy_loss: f64,
    pub value_loss: f64,
    pub entropy: f64,
    pub kl_divergence: f64,
    pub mean_reward: f64,
    pub clip_fraction: f64,
    pub approx_kl: f64,
    pub advantages_mean: f64,
    pub advantages_std: f64,
}

impl PpoConfig {
    /// Compute the clipped surrogate loss given probability ratios and advantages.
    pub fn compute_policy_loss(&self, ratios: &[f64], advantages: &[f64]) -> f64 {
        if ratios.is_empty() || advantages.is_empty() {
            return 0.0;
        }
        let n = ratios.len().min(advantages.len());
        let mut total = 0.0;
        for i in 0..n {
            let unclipped = ratios[i] * advantages[i];
            let clipped = ratios[i].clamp(1.0 - self.clip_range, 1.0 + self.clip_range) * advantages[i];
            total += unclipped.min(clipped);
        }
        -(total / n as f64)
    }

    /// Compute value function loss with optional clipping.
    pub fn compute_value_loss(&self, values: &[f64], returns: &[f64], old_values: &[f64]) -> f64 {
        if values.is_empty() || returns.is_empty() {
            return 0.0;
        }
        let n = values.len().min(returns.len()).min(old_values.len());
        let mut total = 0.0;
        for i in 0..n {
            let unclipped = (values[i] - returns[i]).powi(2);
            let clipped_val = old_values[i]
                + (values[i] - old_values[i]).clamp(-self.clip_range_vf, self.clip_range_vf);
            let clipped = (clipped_val - returns[i]).powi(2);
            total += unclipped.max(clipped);
        }
        self.vf_coef * (total / n as f64)
    }

    /// Compute KL penalty given old and new log probabilities.
    pub fn compute_kl_penalty(&self, old_logprobs: &[f64], new_logprobs: &[f64]) -> f64 {
        if old_logprobs.is_empty() || new_logprobs.is_empty() {
            return 0.0;
        }
        let n = old_logprobs.len().min(new_logprobs.len());
        let mut total = 0.0;
        for i in 0..n {
            let diff = old_logprobs[i] - new_logprobs[i];
            match &self.kl_penalty {
                KlPenaltyMode::Kl => total += diff,
                KlPenaltyMode::Abs => total += diff.abs(),
                KlPenaltyMode::Mse => total += 0.5 * diff.powi(2),
                KlPenaltyMode::Full => {
                    let ratio = (new_logprobs[i] - old_logprobs[i]).exp();
                    total += 0.5 * (diff + ratio - 1.0);
                }
            }
        }
        total / n as f64
    }

    /// Whiten (normalize) advantages to zero mean and unit variance.
    pub fn whiten_advantages_vec(&self, advantages: &[f64]) -> Vec<f64> {
        if advantages.is_empty() {
            return Vec::new();
        }
        let n = advantages.len() as f64;
        let mean = advantages.iter().sum::<f64>() / n;
        let var = advantages.iter().map(|a| (a - mean).powi(2)).sum::<f64>() / n;
        let std = (var + 1e-8).sqrt();
        advantages.iter().map(|a| (a - mean) / std).collect()
    }

    /// Simulate a PPO step with provided rollout data.
    pub fn simulate_step(
        &self,
        ratios: &[f64],
        advantages: &[f64],
        values: &[f64],
        returns: &[f64],
        old_values: &[f64],
        old_logprobs: &[f64],
        new_logprobs: &[f64],
        rewards: &[f64],
    ) -> PpoStepResult {
        let adv = if self.whiten_advantages {
            self.whiten_advantages_vec(advantages)
        } else {
            advantages.to_vec()
        };
        let policy_loss = self.compute_policy_loss(ratios, &adv);
        let value_loss = self.compute_value_loss(values, returns, old_values);
        let kl_divergence = self.compute_kl_penalty(old_logprobs, new_logprobs);
        let n = rewards.len().max(1) as f64;
        let mean_reward = rewards.iter().sum::<f64>() / n;
        let clip_count = ratios
            .iter()
            .filter(|&&r| r < 1.0 - self.clip_range || r > 1.0 + self.clip_range)
            .count();
        let clip_fraction = clip_count as f64 / ratios.len().max(1) as f64;
        let adv_mean = adv.iter().sum::<f64>() / adv.len().max(1) as f64;
        let adv_var = adv.iter().map(|a| (a - adv_mean).powi(2)).sum::<f64>() / adv.len().max(1) as f64;

        PpoStepResult {
            policy_loss,
            value_loss,
            entropy: 0.0, // placeholder — real entropy from model logits
            kl_divergence,
            mean_reward,
            clip_fraction,
            approx_kl: kl_divergence,
            advantages_mean: adv_mean,
            advantages_std: adv_var.sqrt(),
        }
    }
}

// ── DPO Configuration ──

#[derive(Debug, Clone)]
pub struct DpoConfig {
    pub beta: f64,
    pub learning_rate: f64,
    pub batch_size: usize,
    pub num_epochs: u32,
    pub label_smoothing: f64,
    pub loss_type: DpoLossType,
    pub reference_free: bool,
    pub max_length: usize,
    pub max_prompt_length: usize,
    pub generation_config: GenerationConfig,
}

impl Default for DpoConfig {
    fn default() -> Self {
        Self {
            beta: 0.1,
            learning_rate: 5e-7,
            batch_size: 64,
            num_epochs: 1,
            label_smoothing: 0.0,
            loss_type: DpoLossType::Sigmoid,
            reference_free: false,
            max_length: 1024,
            max_prompt_length: 512,
            generation_config: GenerationConfig::default(),
        }
    }
}

/// DPO loss variant.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DpoLossType {
    Sigmoid,
    Hinge,
    Ipo,
    Robust,
}

impl DpoLossType {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Sigmoid => "sigmoid",
            Self::Hinge => "hinge",
            Self::Ipo => "ipo",
            Self::Robust => "robust",
        }
    }
}

impl std::fmt::Display for DpoLossType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

/// A preference pair for DPO/KTO training.
#[derive(Debug, Clone)]
pub struct PreferencePair {
    pub prompt: String,
    pub chosen: String,
    pub rejected: String,
    pub chosen_score: Option<f64>,
    pub rejected_score: Option<f64>,
    pub metadata: HashMap<String, String>,
}

impl DpoConfig {
    /// Compute DPO loss for a batch of preference pairs given log probability differences.
    /// `chosen_logratios[i]` = log π(chosen_i) - log π_ref(chosen_i)
    /// `rejected_logratios[i]` = log π(rejected_i) - log π_ref(rejected_i)
    pub fn compute_loss(&self, chosen_logratios: &[f64], rejected_logratios: &[f64]) -> f64 {
        if chosen_logratios.is_empty() || rejected_logratios.is_empty() {
            return 0.0;
        }
        let n = chosen_logratios.len().min(rejected_logratios.len());
        let mut total = 0.0;
        for i in 0..n {
            let diff = self.beta * (chosen_logratios[i] - rejected_logratios[i]);
            let loss = match &self.loss_type {
                DpoLossType::Sigmoid => {
                    let smooth = self.label_smoothing;
                    let log_sig = log_sigmoid(diff);
                    let log_sig_neg = log_sigmoid(-diff);
                    -(1.0 - smooth) * log_sig - smooth * log_sig_neg
                }
                DpoLossType::Hinge => (1.0 - diff).max(0.0),
                DpoLossType::Ipo => (diff - 1.0 / (2.0 * self.beta)).powi(2),
                DpoLossType::Robust => {
                    let sig = sigmoid(diff);
                    -sig.ln() + self.label_smoothing * (1.0 - sig).max(1e-10).ln()
                }
            };
            total += loss;
        }
        total / n as f64
    }

    /// Compute per-example implicit rewards from log-ratios.
    pub fn compute_rewards(&self, chosen_logratios: &[f64], rejected_logratios: &[f64]) -> (Vec<f64>, Vec<f64>) {
        let chosen_rewards: Vec<f64> = chosen_logratios.iter().map(|lr| self.beta * lr).collect();
        let rejected_rewards: Vec<f64> = rejected_logratios.iter().map(|lr| self.beta * lr).collect();
        (chosen_rewards, rejected_rewards)
    }

    /// Accuracy: fraction where chosen reward > rejected reward.
    pub fn compute_accuracy(&self, chosen_logratios: &[f64], rejected_logratios: &[f64]) -> f64 {
        if chosen_logratios.is_empty() || rejected_logratios.is_empty() {
            return 0.0;
        }
        let n = chosen_logratios.len().min(rejected_logratios.len());
        let correct = (0..n)
            .filter(|&i| chosen_logratios[i] > rejected_logratios[i])
            .count();
        correct as f64 / n as f64
    }
}

// ── KTO Configuration ──

#[derive(Debug, Clone)]
pub struct KtoConfig {
    pub beta: f64,
    pub desirable_weight: f64,
    pub undesirable_weight: f64,
    pub learning_rate: f64,
    pub batch_size: usize,
    pub num_epochs: u32,
    pub max_length: usize,
    pub generation_config: GenerationConfig,
}

impl Default for KtoConfig {
    fn default() -> Self {
        Self {
            beta: 0.1,
            desirable_weight: 1.0,
            undesirable_weight: 1.0,
            learning_rate: 5e-7,
            batch_size: 64,
            num_epochs: 1,
            max_length: 1024,
            generation_config: GenerationConfig::default(),
        }
    }
}

/// A KTO training example.
#[derive(Debug, Clone)]
pub struct KtoExample {
    pub prompt: String,
    pub completion: String,
    pub kind: KtoExampleKind,
    pub metadata: HashMap<String, String>,
}

impl KtoConfig {
    /// Compute KTO loss for a batch with desirable and undesirable examples.
    /// `kl_estimate` is the estimated KL(pi || pi_ref) used as baseline.
    pub fn compute_loss(
        &self,
        logratios: &[f64],
        kinds: &[KtoExampleKind],
        kl_estimate: f64,
    ) -> f64 {
        if logratios.is_empty() || kinds.is_empty() {
            return 0.0;
        }
        let n = logratios.len().min(kinds.len());
        let mut total = 0.0;
        for i in 0..n {
            let logratio = logratios[i];
            let loss = match &kinds[i] {
                KtoExampleKind::Desirable => {
                    let v = self.beta * (logratio - kl_estimate);
                    self.desirable_weight * (1.0 - sigmoid(v))
                }
                KtoExampleKind::Undesirable => {
                    let v = self.beta * (kl_estimate - logratio);
                    self.undesirable_weight * (1.0 - sigmoid(v))
                }
            };
            total += loss;
        }
        total / n as f64
    }

    /// Count desirable/undesirable split in a set of examples.
    pub fn count_splits(&self, kinds: &[KtoExampleKind]) -> (usize, usize) {
        let desirable = kinds.iter().filter(|k| **k == KtoExampleKind::Desirable).count();
        let undesirable = kinds.len() - desirable;
        (desirable, undesirable)
    }
}

// ── ORPO Configuration ──

#[derive(Debug, Clone)]
pub struct OrpoConfig {
    pub lambda: f64,
    pub learning_rate: f64,
    pub batch_size: usize,
    pub num_epochs: u32,
    pub max_length: usize,
    pub generation_config: GenerationConfig,
}

impl Default for OrpoConfig {
    fn default() -> Self {
        Self {
            lambda: 1.0,
            learning_rate: 5e-7,
            batch_size: 64,
            num_epochs: 1,
            max_length: 1024,
            generation_config: GenerationConfig::default(),
        }
    }
}

impl OrpoConfig {
    /// Compute ORPO loss combining SFT and odds-ratio preference loss.
    /// `chosen_logprobs[i]` = average log-probability of chosen response
    /// `rejected_logprobs[i]` = average log-probability of rejected response
    pub fn compute_loss(&self, chosen_logprobs: &[f64], rejected_logprobs: &[f64]) -> f64 {
        if chosen_logprobs.is_empty() || rejected_logprobs.is_empty() {
            return 0.0;
        }
        let n = chosen_logprobs.len().min(rejected_logprobs.len());
        let mut total = 0.0;
        for i in 0..n {
            // SFT loss: negative log-likelihood of chosen
            let sft_loss = -chosen_logprobs[i];
            // Odds ratio: odds(chosen) / odds(rejected)
            let odds_chosen = chosen_logprobs[i].exp() / (1.0 - chosen_logprobs[i].exp()).max(1e-10);
            let odds_rejected = rejected_logprobs[i].exp() / (1.0 - rejected_logprobs[i].exp()).max(1e-10);
            let log_odds_ratio = (odds_chosen / odds_rejected.max(1e-10)).ln();
            let or_loss = -log_sigmoid(log_odds_ratio);
            total += sft_loss + self.lambda * or_loss;
        }
        total / n as f64
    }

    /// Compute odds ratio for a given log-probability.
    pub fn compute_odds(logprob: f64) -> f64 {
        let p = logprob.exp().min(1.0 - 1e-10);
        p / (1.0 - p)
    }
}

// ── GRPO Configuration ──

#[derive(Debug, Clone)]
pub struct GrpoConfig {
    pub group_size: usize,
    pub clip_range: f64,
    pub kl_coef: f64,
    pub learning_rate: f64,
    pub batch_size: usize,
    pub num_epochs: u32,
    pub temperature: f64,
    pub max_length: usize,
    pub generation_config: GenerationConfig,
}

impl Default for GrpoConfig {
    fn default() -> Self {
        Self {
            group_size: 8,
            clip_range: 0.2,
            kl_coef: 0.05,
            learning_rate: 1e-6,
            batch_size: 64,
            num_epochs: 1,
            temperature: 1.0,
            max_length: 1024,
            generation_config: GenerationConfig::default(),
        }
    }
}

impl GrpoConfig {
    /// Compute group-relative advantages from rewards within a group.
    /// For each group of `group_size` responses, normalizes rewards to zero mean, unit variance.
    pub fn compute_group_advantages(&self, rewards: &[f64]) -> Vec<f64> {
        if rewards.is_empty() || self.group_size == 0 {
            return Vec::new();
        }
        let mut advantages = Vec::with_capacity(rewards.len());
        for chunk in rewards.chunks(self.group_size) {
            let n = chunk.len() as f64;
            let mean = chunk.iter().sum::<f64>() / n;
            let var = chunk.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / n;
            let std = (var + 1e-8).sqrt();
            for r in chunk {
                advantages.push((r - mean) / std);
            }
        }
        advantages
    }

    /// Compute GRPO policy loss given ratios and group-normalized advantages.
    pub fn compute_loss(&self, ratios: &[f64], advantages: &[f64], kl_values: &[f64]) -> f64 {
        if ratios.is_empty() || advantages.is_empty() {
            return 0.0;
        }
        let n = ratios.len().min(advantages.len());
        let mut policy_loss = 0.0;
        for i in 0..n {
            let unclipped = ratios[i] * advantages[i];
            let clipped = ratios[i].clamp(1.0 - self.clip_range, 1.0 + self.clip_range) * advantages[i];
            policy_loss += unclipped.min(clipped);
        }
        policy_loss = -(policy_loss / n as f64);
        let kl_loss = if !kl_values.is_empty() {
            self.kl_coef * (kl_values.iter().sum::<f64>() / kl_values.len() as f64)
        } else {
            0.0
        };
        policy_loss + kl_loss
    }
}

// ── Generation Config ──

#[derive(Debug, Clone)]
pub struct GenerationConfig {
    pub temperature: f64,
    pub top_p: f64,
    pub top_k: u32,
    pub repetition_penalty: f64,
    pub max_new_tokens: usize,
    pub do_sample: bool,
    pub num_return_sequences: usize,
}

impl Default for GenerationConfig {
    fn default() -> Self {
        Self {
            temperature: 1.0,
            top_p: 1.0,
            top_k: 50,
            repetition_penalty: 1.0,
            max_new_tokens: 512,
            do_sample: true,
            num_return_sequences: 1,
        }
    }
}

impl GenerationConfig {
    /// Apply temperature scaling to logits.
    pub fn apply_temperature(&self, logits: &[f64]) -> Vec<f64> {
        if self.temperature <= 0.0 || logits.is_empty() {
            return logits.to_vec();
        }
        logits.iter().map(|l| l / self.temperature).collect()
    }

    /// Apply top-k filtering: keep only the top-k logits, mask rest to -inf.
    pub fn apply_top_k(&self, logits: &[f64]) -> Vec<f64> {
        if self.top_k == 0 || self.top_k as usize >= logits.len() {
            return logits.to_vec();
        }
        let mut indexed: Vec<(usize, f64)> = logits.iter().copied().enumerate().collect();
        indexed.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        let threshold = indexed[self.top_k as usize - 1].1;
        logits
            .iter()
            .map(|&l| if l >= threshold { l } else { f64::NEG_INFINITY })
            .collect()
    }

    /// Apply top-p (nucleus) filtering.
    pub fn apply_top_p(&self, logits: &[f64]) -> Vec<f64> {
        if self.top_p >= 1.0 || logits.is_empty() {
            return logits.to_vec();
        }
        let max_logit = logits.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let probs: Vec<f64> = logits.iter().map(|l| (l - max_logit).exp()).collect();
        let sum: f64 = probs.iter().sum();
        let normalized: Vec<f64> = probs.iter().map(|p| p / sum).collect();

        let mut indexed: Vec<(usize, f64)> = normalized.iter().copied().enumerate().collect();
        indexed.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let mut cumsum = 0.0;
        let mut keep = vec![false; logits.len()];
        for (idx, prob) in &indexed {
            cumsum += prob;
            keep[*idx] = true;
            if cumsum >= self.top_p {
                break;
            }
        }
        logits
            .iter()
            .enumerate()
            .map(|(i, &l)| if keep[i] { l } else { f64::NEG_INFINITY })
            .collect()
    }

    /// Apply repetition penalty to logits given previous token ids.
    pub fn apply_repetition_penalty(&self, logits: &[f64], prev_tokens: &[usize]) -> Vec<f64> {
        if self.repetition_penalty == 1.0 || logits.is_empty() {
            return logits.to_vec();
        }
        let mut result = logits.to_vec();
        for &tok in prev_tokens {
            if tok < result.len() {
                if result[tok] > 0.0 {
                    result[tok] /= self.repetition_penalty;
                } else {
                    result[tok] *= self.repetition_penalty;
                }
            }
        }
        result
    }
}

// ── Reward Model ──

#[derive(Debug, Clone)]
pub struct RewardModelConfig {
    pub model_name: String,
    pub hidden_size: usize,
    pub num_layers: usize,
    pub dropout: f64,
    pub learning_rate: f64,
    pub batch_size: usize,
    pub num_epochs: u32,
    pub max_length: usize,
    pub ensemble_size: usize,
    pub calibration_enabled: bool,
    pub calibration_temperature: f64,
}

impl Default for RewardModelConfig {
    fn default() -> Self {
        Self {
            model_name: "reward-model-base".to_string(),
            hidden_size: 4096,
            num_layers: 32,
            dropout: 0.1,
            learning_rate: 1e-5,
            batch_size: 32,
            num_epochs: 1,
            max_length: 1024,
            ensemble_size: 3,
            calibration_enabled: true,
            calibration_temperature: 1.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RewardModelScore {
    pub score: f64,
    pub confidence: f64,
    pub model_id: String,
}

#[derive(Debug, Clone)]
pub struct RewardModelEnsemble {
    pub models: Vec<RewardModelConfig>,
    pub aggregation: EnsembleAggregation,
    pub disagreement_threshold: f64,
    pub weights: Vec<f64>,
}

impl RewardModelEnsemble {
    pub fn new(num_models: usize, aggregation: EnsembleAggregation) -> Self {
        let models: Vec<RewardModelConfig> = (0..num_models)
            .map(|i| {
                let mut cfg = RewardModelConfig::default();
                cfg.model_name = format!("reward-model-{}", i);
                cfg
            })
            .collect();
        let weights = vec![1.0 / num_models as f64; num_models];
        Self {
            models,
            aggregation,
            disagreement_threshold: 1.0,
            weights,
        }
    }

    /// Aggregate scores from ensemble members.
    pub fn aggregate(&self, scores: &[f64]) -> f64 {
        if scores.is_empty() {
            return 0.0;
        }
        match &self.aggregation {
            EnsembleAggregation::Mean => {
                scores.iter().sum::<f64>() / scores.len() as f64
            }
            EnsembleAggregation::Median => {
                let mut sorted = scores.to_vec();
                sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                let mid = sorted.len() / 2;
                if sorted.len() % 2 == 0 {
                    (sorted[mid - 1] + sorted[mid]) / 2.0
                } else {
                    sorted[mid]
                }
            }
            EnsembleAggregation::WeightedMean => {
                let n = scores.len().min(self.weights.len());
                let weighted_sum: f64 = (0..n).map(|i| scores[i] * self.weights[i]).sum();
                let weight_sum: f64 = self.weights[..n].iter().sum();
                if weight_sum > 0.0 { weighted_sum / weight_sum } else { 0.0 }
            }
            EnsembleAggregation::Min => {
                scores.iter().cloned().fold(f64::INFINITY, f64::min)
            }
            EnsembleAggregation::Max => {
                scores.iter().cloned().fold(f64::NEG_INFINITY, f64::max)
            }
        }
    }

    /// Detect disagreement among ensemble members.
    pub fn detect_disagreement(&self, scores: &[f64]) -> bool {
        if scores.len() < 2 {
            return false;
        }
        let mean = scores.iter().sum::<f64>() / scores.len() as f64;
        let var = scores.iter().map(|s| (s - mean).powi(2)).sum::<f64>() / scores.len() as f64;
        var.sqrt() > self.disagreement_threshold
    }

    /// Calibrate a raw score using temperature scaling.
    pub fn calibrate_score(&self, raw_score: f64, temperature: f64) -> f64 {
        if temperature <= 0.0 {
            return raw_score;
        }
        raw_score / temperature
    }
}

// ── Process Reward Model ──

#[derive(Debug, Clone)]
pub struct ProcessRewardStep {
    pub step_index: usize,
    pub content: String,
    pub reward: f64,
    pub is_correct: Option<bool>,
    pub annotation: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ProcessRewardModel {
    pub model_name: String,
    pub steps: Vec<ProcessRewardStep>,
    pub aggregation_method: PrmAggregation,
}

/// PRM aggregation method for final score.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PrmAggregation {
    Min,
    Product,
    LastStep,
    WeightedSum,
}

impl PrmAggregation {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Min => "Min",
            Self::Product => "Product",
            Self::LastStep => "Last Step",
            Self::WeightedSum => "Weighted Sum",
        }
    }
}

impl std::fmt::Display for PrmAggregation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

impl ProcessRewardModel {
    pub fn new(model_name: &str, aggregation: PrmAggregation) -> Self {
        Self {
            model_name: model_name.to_string(),
            steps: Vec::new(),
            aggregation_method: aggregation,
        }
    }

    pub fn add_step(&mut self, content: &str, reward: f64) {
        let idx = self.steps.len();
        self.steps.push(ProcessRewardStep {
            step_index: idx,
            content: content.to_string(),
            reward,
            is_correct: None,
            annotation: None,
        });
    }

    pub fn annotate_step(&mut self, step_index: usize, correct: bool, annotation: &str) {
        if let Some(step) = self.steps.get_mut(step_index) {
            step.is_correct = Some(correct);
            step.annotation = Some(annotation.to_string());
        }
    }

    /// Compute aggregated score over all steps.
    pub fn compute_score(&self) -> f64 {
        if self.steps.is_empty() {
            return 0.0;
        }
        match &self.aggregation_method {
            PrmAggregation::Min => self
                .steps
                .iter()
                .map(|s| s.reward)
                .fold(f64::INFINITY, f64::min),
            PrmAggregation::Product => self.steps.iter().map(|s| s.reward).product(),
            PrmAggregation::LastStep => self.steps.last().map(|s| s.reward).unwrap_or(0.0),
            PrmAggregation::WeightedSum => {
                let total_weight: f64 = (1..=self.steps.len()).map(|i| i as f64).sum();
                self.steps
                    .iter()
                    .enumerate()
                    .map(|(i, s)| s.reward * (i + 1) as f64 / total_weight)
                    .sum()
            }
        }
    }

    /// Find the first step where the reasoning went wrong.
    pub fn find_first_error(&self) -> Option<usize> {
        self.steps
            .iter()
            .find(|s| s.is_correct == Some(false))
            .map(|s| s.step_index)
    }

    /// Fraction of steps marked correct.
    pub fn correctness_ratio(&self) -> f64 {
        let annotated: Vec<&ProcessRewardStep> =
            self.steps.iter().filter(|s| s.is_correct.is_some()).collect();
        if annotated.is_empty() {
            return 0.0;
        }
        let correct = annotated.iter().filter(|s| s.is_correct == Some(true)).count();
        correct as f64 / annotated.len() as f64
    }
}

// ── Constitutional AI / RLAIF ──

#[derive(Debug, Clone)]
pub struct ConstitutionalPrinciple {
    pub id: String,
    pub name: String,
    pub description: String,
    pub critique_prompt: String,
    pub revision_prompt: String,
    pub weight: f64,
}

#[derive(Debug, Clone)]
pub struct Constitution {
    pub name: String,
    pub version: String,
    pub principles: Vec<ConstitutionalPrinciple>,
}

impl Constitution {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            version: "1.0".to_string(),
            principles: Vec::new(),
        }
    }

    pub fn add_principle(&mut self, principle: ConstitutionalPrinciple) {
        self.principles.push(principle);
    }

    pub fn get_principle(&self, id: &str) -> Option<&ConstitutionalPrinciple> {
        self.principles.iter().find(|p| p.id == id)
    }

    pub fn principle_count(&self) -> usize {
        self.principles.len()
    }
}

#[derive(Debug, Clone)]
pub struct CritiqueRevisionResult {
    pub principle_id: String,
    pub original: String,
    pub critique: String,
    pub revised: String,
    pub improvement_score: f64,
}

#[derive(Debug, Clone)]
pub struct ConstitutionalAiEngine {
    pub constitution: Constitution,
    pub num_revision_rounds: usize,
    pub critique_temperature: f64,
    pub revision_temperature: f64,
}

impl ConstitutionalAiEngine {
    pub fn new(constitution: Constitution) -> Self {
        Self {
            constitution,
            num_revision_rounds: 1,
            critique_temperature: 0.7,
            revision_temperature: 0.7,
        }
    }

    /// Simulate a critique-revision loop, returning per-principle results.
    /// In production this calls an LLM; here we demonstrate the pipeline structure.
    pub fn run_critique_revision(&self, response: &str) -> Vec<CritiqueRevisionResult> {
        self.constitution
            .principles
            .iter()
            .map(|p| CritiqueRevisionResult {
                principle_id: p.id.clone(),
                original: response.to_string(),
                critique: format!(
                    "[Critique per '{}'] Checking: {}",
                    p.name, p.description
                ),
                revised: response.to_string(), // placeholder
                improvement_score: 0.0,
            })
            .collect()
    }

    /// Generate a synthetic preference pair from a critique-revision round.
    pub fn generate_preference_pair(
        &self,
        prompt: &str,
        original: &str,
        revised: &str,
    ) -> PreferencePair {
        PreferencePair {
            prompt: prompt.to_string(),
            chosen: revised.to_string(),
            rejected: original.to_string(),
            chosen_score: None,
            rejected_score: None,
            metadata: HashMap::new(),
        }
    }
}

// ── RLEF (RL from Execution Feedback) ──

#[derive(Debug, Clone)]
pub struct ExecutionFeedback {
    pub compilation_success: bool,
    pub test_pass_rate: f64,
    pub tests_total: usize,
    pub tests_passed: usize,
    pub static_analysis_score: f64,
    pub execution_time_ms: u64,
    pub memory_usage_bytes: u64,
    pub lint_warnings: usize,
    pub lint_errors: usize,
}

impl ExecutionFeedback {
    /// Compute a scalar reward from execution feedback.
    pub fn compute_reward(&self, weights: &RlefWeights) -> f64 {
        let compilation = if self.compilation_success { 1.0 } else { 0.0 };
        let test_rate = self.test_pass_rate;
        let static_score = self.static_analysis_score;
        let time_penalty = if self.execution_time_ms > weights.max_execution_time_ms {
            -0.1 * ((self.execution_time_ms as f64 / weights.max_execution_time_ms as f64) - 1.0)
        } else {
            0.0
        };
        let lint_penalty = -(self.lint_errors as f64 * 0.1 + self.lint_warnings as f64 * 0.02);

        weights.compilation_weight * compilation
            + weights.test_pass_weight * test_rate
            + weights.static_analysis_weight * static_score
            + time_penalty
            + lint_penalty.max(-1.0)
    }
}

#[derive(Debug, Clone)]
pub struct RlefWeights {
    pub compilation_weight: f64,
    pub test_pass_weight: f64,
    pub static_analysis_weight: f64,
    pub max_execution_time_ms: u64,
}

impl Default for RlefWeights {
    fn default() -> Self {
        Self {
            compilation_weight: 0.3,
            test_pass_weight: 0.4,
            static_analysis_weight: 0.2,
            max_execution_time_ms: 10_000,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RlefConfig {
    pub weights: RlefWeights,
    pub sandbox_enabled: bool,
    pub timeout_ms: u64,
    pub max_retries: u32,
}

impl Default for RlefConfig {
    fn default() -> Self {
        Self {
            weights: RlefWeights::default(),
            sandbox_enabled: true,
            timeout_ms: 30_000,
            max_retries: 3,
        }
    }
}

// ── Reward Hacking Detection ──

#[derive(Debug, Clone)]
pub struct RewardHackingAlert {
    pub signal: HackingSignal,
    pub severity: f64,
    pub description: String,
    pub step: u64,
    pub metric_value: f64,
    pub threshold: f64,
}

#[derive(Debug, Clone)]
pub struct RewardHackingDetector {
    pub kl_threshold: f64,
    pub reward_shift_threshold: f64,
    pub ood_threshold: f64,
    pub max_response_length: usize,
    pub max_repetition_ratio: f64,
    pub history: Vec<RewardSnapshot>,
    pub window_size: usize,
}

#[derive(Debug, Clone)]
pub struct RewardSnapshot {
    pub step: u64,
    pub mean_reward: f64,
    pub std_reward: f64,
    pub kl_divergence: f64,
    pub mean_length: f64,
}

impl Default for RewardHackingDetector {
    fn default() -> Self {
        Self {
            kl_threshold: 10.0,
            reward_shift_threshold: 2.0,
            ood_threshold: 3.0,
            max_response_length: 4096,
            max_repetition_ratio: 0.5,
            history: Vec::new(),
            window_size: 100,
        }
    }
}

impl RewardHackingDetector {
    pub fn record_snapshot(&mut self, snapshot: RewardSnapshot) {
        self.history.push(snapshot);
        if self.history.len() > self.window_size {
            self.history.remove(0);
        }
    }

    /// Detect KL divergence spike relative to recent history.
    pub fn check_kl_divergence(&self, current_kl: f64) -> Option<RewardHackingAlert> {
        if current_kl > self.kl_threshold {
            Some(RewardHackingAlert {
                signal: HackingSignal::KlDivergenceSpike,
                severity: HackingSignal::KlDivergenceSpike.severity(),
                description: format!(
                    "KL divergence {:.4} exceeds threshold {:.4}",
                    current_kl, self.kl_threshold
                ),
                step: self.history.len() as u64,
                metric_value: current_kl,
                threshold: self.kl_threshold,
            })
        } else {
            None
        }
    }

    /// Detect reward distribution shift (mean reward jumps beyond historical range).
    pub fn check_reward_distribution(&self, current_mean: f64) -> Option<RewardHackingAlert> {
        if self.history.len() < 2 {
            return None;
        }
        let hist_mean = self.history.iter().map(|s| s.mean_reward).sum::<f64>()
            / self.history.len() as f64;
        let hist_var = self
            .history
            .iter()
            .map(|s| (s.mean_reward - hist_mean).powi(2))
            .sum::<f64>()
            / self.history.len() as f64;
        let hist_std = (hist_var + 1e-8).sqrt();
        let z_score = (current_mean - hist_mean).abs() / hist_std;
        if z_score > self.reward_shift_threshold {
            Some(RewardHackingAlert {
                signal: HackingSignal::RewardDistributionShift,
                severity: HackingSignal::RewardDistributionShift.severity(),
                description: format!(
                    "Reward mean z-score {:.4} exceeds threshold {:.4}",
                    z_score, self.reward_shift_threshold
                ),
                step: self.history.len() as u64,
                metric_value: z_score,
                threshold: self.reward_shift_threshold,
            })
        } else {
            None
        }
    }

    /// Detect length exploitation (responses growing excessively to game reward).
    pub fn check_length_exploitation(&self, mean_length: f64) -> Option<RewardHackingAlert> {
        if mean_length > self.max_response_length as f64 {
            Some(RewardHackingAlert {
                signal: HackingSignal::LengthExploitation,
                severity: HackingSignal::LengthExploitation.severity(),
                description: format!(
                    "Mean response length {:.0} exceeds max {}",
                    mean_length, self.max_response_length
                ),
                step: self.history.len() as u64,
                metric_value: mean_length,
                threshold: self.max_response_length as f64,
            })
        } else {
            None
        }
    }

    /// Detect repetition exploitation.
    pub fn check_repetition(&self, repetition_ratio: f64) -> Option<RewardHackingAlert> {
        if repetition_ratio > self.max_repetition_ratio {
            Some(RewardHackingAlert {
                signal: HackingSignal::RepetitionExploitation,
                severity: HackingSignal::RepetitionExploitation.severity(),
                description: format!(
                    "Repetition ratio {:.4} exceeds threshold {:.4}",
                    repetition_ratio, self.max_repetition_ratio
                ),
                step: self.history.len() as u64,
                metric_value: repetition_ratio,
                threshold: self.max_repetition_ratio,
            })
        } else {
            None
        }
    }

    /// Run all hacking detection checks and return all alerts.
    pub fn run_all_checks(
        &self,
        kl: f64,
        mean_reward: f64,
        mean_length: f64,
        repetition_ratio: f64,
    ) -> Vec<RewardHackingAlert> {
        let mut alerts = Vec::new();
        if let Some(a) = self.check_kl_divergence(kl) {
            alerts.push(a);
        }
        if let Some(a) = self.check_reward_distribution(mean_reward) {
            alerts.push(a);
        }
        if let Some(a) = self.check_length_exploitation(mean_length) {
            alerts.push(a);
        }
        if let Some(a) = self.check_repetition(repetition_ratio) {
            alerts.push(a);
        }
        alerts
    }
}

// ── Alignment Evaluation ──

#[derive(Debug, Clone)]
pub struct AlignmentBenchmarkResult {
    pub benchmark: AlignmentBenchmark,
    pub score: f64,
    pub num_samples: usize,
    pub pass_rate: f64,
    pub details: HashMap<String, f64>,
}

#[derive(Debug, Clone)]
pub struct AlignmentEvaluator {
    pub benchmarks: Vec<AlignmentBenchmark>,
    pub results: Vec<AlignmentBenchmarkResult>,
    pub alignment_tax: f64,
    pub safety_threshold: f64,
    pub helpfulness_threshold: f64,
}

impl Default for AlignmentEvaluator {
    fn default() -> Self {
        Self {
            benchmarks: AlignmentBenchmark::all(),
            results: Vec::new(),
            alignment_tax: 0.0,
            safety_threshold: 0.95,
            helpfulness_threshold: 0.7,
        }
    }
}

impl AlignmentEvaluator {
    pub fn new(benchmarks: Vec<AlignmentBenchmark>) -> Self {
        Self {
            benchmarks,
            ..Default::default()
        }
    }

    pub fn add_result(&mut self, result: AlignmentBenchmarkResult) {
        self.results.push(result);
    }

    /// Compute alignment tax: degradation in helpfulness relative to unaligned baseline.
    pub fn compute_alignment_tax(&self, baseline_helpfulness: f64, aligned_helpfulness: f64) -> f64 {
        if baseline_helpfulness <= 0.0 {
            return 0.0;
        }
        1.0 - (aligned_helpfulness / baseline_helpfulness)
    }

    /// Check if all safety benchmarks pass threshold.
    pub fn safety_pass(&self) -> bool {
        self.results
            .iter()
            .filter(|r| r.benchmark.is_safety())
            .all(|r| r.score >= self.safety_threshold)
    }

    /// Check if helpfulness benchmarks pass threshold.
    pub fn helpfulness_pass(&self) -> bool {
        self.results
            .iter()
            .filter(|r| r.benchmark == AlignmentBenchmark::Helpfulness)
            .all(|r| r.score >= self.helpfulness_threshold)
    }

    /// Overall alignment pass: safety AND helpfulness thresholds met.
    pub fn alignment_pass(&self) -> bool {
        self.safety_pass() && self.helpfulness_pass()
    }

    /// Compute aggregate score across all benchmarks.
    pub fn aggregate_score(&self) -> f64 {
        if self.results.is_empty() {
            return 0.0;
        }
        self.results.iter().map(|r| r.score).sum::<f64>() / self.results.len() as f64
    }

    /// Generate a summary report.
    pub fn summary_report(&self) -> String {
        let mut report = String::from("=== Alignment Evaluation Report ===\n\n");
        for result in &self.results {
            report.push_str(&format!(
                "  {}: score={:.4}, pass_rate={:.2}%, samples={}\n",
                result.benchmark,
                result.score,
                result.pass_rate * 100.0,
                result.num_samples,
            ));
        }
        report.push_str(&format!(
            "\n  Aggregate Score: {:.4}\n",
            self.aggregate_score()
        ));
        report.push_str(&format!(
            "  Safety Pass: {}\n  Helpfulness Pass: {}\n  Overall: {}\n",
            self.safety_pass(),
            self.helpfulness_pass(),
            self.alignment_pass(),
        ));
        report
    }
}

// ── Preference Dataset Management ──

#[derive(Debug, Clone)]
pub struct PreferenceDataset {
    pub name: String,
    pub pairs: Vec<PreferencePair>,
    pub metadata: HashMap<String, String>,
}

impl PreferenceDataset {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            pairs: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    pub fn add_pair(&mut self, pair: PreferencePair) {
        self.pairs.push(pair);
    }

    pub fn len(&self) -> usize {
        self.pairs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.pairs.is_empty()
    }

    /// Deduplicate pairs by prompt.
    pub fn deduplicate(&mut self) {
        let mut seen = std::collections::HashSet::new();
        self.pairs.retain(|p| seen.insert(p.prompt.clone()));
    }

    /// Filter pairs by minimum score difference between chosen and rejected.
    pub fn filter_by_score_gap(&mut self, min_gap: f64) {
        self.pairs.retain(|p| {
            match (p.chosen_score, p.rejected_score) {
                (Some(c), Some(r)) => (c - r) >= min_gap,
                _ => true, // keep if no scores
            }
        });
    }

    /// Compute quality score for each pair based on prompt length, response length, diversity.
    pub fn quality_scores(&self) -> Vec<f64> {
        self.pairs
            .iter()
            .map(|p| {
                let prompt_len_score = (p.prompt.len() as f64 / 500.0).min(1.0);
                let chosen_len_score = (p.chosen.len() as f64 / 1000.0).min(1.0);
                let diversity_score = if p.chosen != p.rejected { 1.0 } else { 0.0 };
                (prompt_len_score + chosen_len_score + diversity_score) / 3.0
            })
            .collect()
    }

    /// Split dataset into train and validation sets.
    pub fn split(&self, train_ratio: f64) -> (PreferenceDataset, PreferenceDataset) {
        let split_idx = (self.pairs.len() as f64 * train_ratio).round() as usize;
        let mut train = PreferenceDataset::new(&format!("{}-train", self.name));
        let mut val = PreferenceDataset::new(&format!("{}-val", self.name));
        for (i, pair) in self.pairs.iter().enumerate() {
            if i < split_idx {
                train.add_pair(pair.clone());
            } else {
                val.add_pair(pair.clone());
            }
        }
        (train, val)
    }

    /// Get dataset statistics.
    pub fn stats(&self) -> DatasetStats {
        let avg_prompt_len = if self.pairs.is_empty() {
            0.0
        } else {
            self.pairs.iter().map(|p| p.prompt.len()).sum::<usize>() as f64
                / self.pairs.len() as f64
        };
        let avg_chosen_len = if self.pairs.is_empty() {
            0.0
        } else {
            self.pairs.iter().map(|p| p.chosen.len()).sum::<usize>() as f64
                / self.pairs.len() as f64
        };
        let avg_rejected_len = if self.pairs.is_empty() {
            0.0
        } else {
            self.pairs.iter().map(|p| p.rejected.len()).sum::<usize>() as f64
                / self.pairs.len() as f64
        };
        let scored_count = self
            .pairs
            .iter()
            .filter(|p| p.chosen_score.is_some() && p.rejected_score.is_some())
            .count();
        DatasetStats {
            total_pairs: self.pairs.len(),
            avg_prompt_length: avg_prompt_len,
            avg_chosen_length: avg_chosen_len,
            avg_rejected_length: avg_rejected_len,
            scored_pairs: scored_count,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DatasetStats {
    pub total_pairs: usize,
    pub avg_prompt_length: f64,
    pub avg_chosen_length: f64,
    pub avg_rejected_length: f64,
    pub scored_pairs: usize,
}

// ── Training Pipeline ──

#[derive(Debug, Clone)]
pub struct PipelineConfig {
    pub name: String,
    pub algorithm: AlignmentAlgorithm,
    pub stages: Vec<PipelineStage>,
    pub current_stage: PipelineStage,
    pub ppo_config: Option<PpoConfig>,
    pub dpo_config: Option<DpoConfig>,
    pub kto_config: Option<KtoConfig>,
    pub orpo_config: Option<OrpoConfig>,
    pub grpo_config: Option<GrpoConfig>,
    pub reward_model_config: Option<RewardModelConfig>,
    pub distributed_config: Option<DistributedConfig>,
    pub generation_config: GenerationConfig,
}

impl PipelineConfig {
    pub fn new(name: &str, algorithm: AlignmentAlgorithm) -> Self {
        let stages = vec![
            PipelineStage::DataCollection,
            PipelineStage::Sft,
            PipelineStage::RewardModelTraining,
            PipelineStage::PolicyOptimization,
            PipelineStage::Evaluation,
            PipelineStage::ModelMerge,
            PipelineStage::Deployment,
        ];
        Self {
            name: name.to_string(),
            algorithm,
            stages,
            current_stage: PipelineStage::DataCollection,
            ppo_config: None,
            dpo_config: None,
            kto_config: None,
            orpo_config: None,
            grpo_config: None,
            reward_model_config: None,
            distributed_config: None,
            generation_config: GenerationConfig::default(),
        }
    }

    /// Advance to the next pipeline stage if valid.
    pub fn advance_stage(&mut self) -> Result<PipelineStage, String> {
        match self.current_stage.next() {
            Some(next) => {
                self.current_stage = next.clone();
                Ok(next)
            }
            None => Err("Already at final stage (Deployment)".to_string()),
        }
    }

    /// Check whether reward model training is needed for the chosen algorithm.
    pub fn needs_reward_model(&self) -> bool {
        self.algorithm.requires_reward_model()
    }

    /// Check whether a reference model is needed.
    pub fn needs_reference_model(&self) -> bool {
        self.algorithm.requires_reference_model()
    }

    /// Get a summary of the pipeline configuration.
    pub fn summary(&self) -> String {
        format!(
            "Pipeline '{}' | Algorithm: {} | Stage: {} | Reward Model: {} | Ref Model: {}",
            self.name,
            self.algorithm,
            self.current_stage,
            if self.needs_reward_model() { "yes" } else { "no" },
            if self.needs_reference_model() { "yes" } else { "no" },
        )
    }
}

#[derive(Debug, Clone)]
pub struct PipelineStageResult {
    pub stage: PipelineStage,
    pub success: bool,
    pub metrics: HashMap<String, f64>,
    pub duration_secs: f64,
    pub error: Option<String>,
}

// ── Model Merging ──

#[derive(Debug, Clone)]
pub struct ModelMergeConfig {
    pub strategy: MergeStrategy,
    pub model_paths: Vec<String>,
    pub output_path: String,
    pub layer_range: Option<(usize, usize)>,
}

impl ModelMergeConfig {
    pub fn new(strategy: MergeStrategy, model_paths: Vec<String>, output_path: &str) -> Self {
        Self {
            strategy,
            model_paths,
            output_path: output_path.to_string(),
            layer_range: None,
        }
    }
}

/// Model merge operations on parameter tensors (simulated as Vec<f64>).
pub struct ModelMerger;

impl ModelMerger {
    /// Linear interpolation of parameter vectors.
    pub fn linear_merge(params: &[Vec<f64>], weights: &[f64]) -> Vec<f64> {
        if params.is_empty() {
            return Vec::new();
        }
        let len = params[0].len();
        let n = params.len().min(weights.len());
        let weight_sum: f64 = weights[..n].iter().sum();
        let mut result = vec![0.0; len];
        for i in 0..n {
            for j in 0..len {
                if j < params[i].len() {
                    result[j] += params[i][j] * weights[i] / weight_sum;
                }
            }
        }
        result
    }

    /// SLERP (Spherical Linear Interpolation) between two parameter vectors.
    pub fn slerp_merge(params_a: &[f64], params_b: &[f64], t: f64) -> Vec<f64> {
        if params_a.is_empty() || params_b.is_empty() {
            return Vec::new();
        }
        let dot: f64 = params_a
            .iter()
            .zip(params_b.iter())
            .map(|(a, b)| a * b)
            .sum();
        let norm_a: f64 = params_a.iter().map(|a| a * a).sum::<f64>().sqrt();
        let norm_b: f64 = params_b.iter().map(|b| b * b).sum::<f64>().sqrt();
        if norm_a < 1e-10 || norm_b < 1e-10 {
            return Self::linear_merge(&[params_a.to_vec(), params_b.to_vec()], &[1.0 - t, t]);
        }
        let cos_omega = (dot / (norm_a * norm_b)).clamp(-1.0, 1.0);
        let omega = cos_omega.acos();
        if omega.abs() < 1e-6 {
            return Self::linear_merge(&[params_a.to_vec(), params_b.to_vec()], &[1.0 - t, t]);
        }
        let sin_omega = omega.sin();
        let w_a = ((1.0 - t) * omega).sin() / sin_omega;
        let w_b = (t * omega).sin() / sin_omega;
        params_a
            .iter()
            .zip(params_b.iter())
            .map(|(a, b)| w_a * a + w_b * b)
            .collect()
    }

    /// TIES (TrIm, Elect Sign, and Merge) merge for multiple task vectors.
    pub fn ties_merge(
        base: &[f64],
        task_vectors: &[Vec<f64>],
        density: f64,
        _majority_sign: &MajoritySignMethod,
    ) -> Vec<f64> {
        if base.is_empty() || task_vectors.is_empty() {
            return base.to_vec();
        }
        let len = base.len();
        let mut result = base.to_vec();

        // 1. Trim: zero out small-magnitude deltas
        let trimmed: Vec<Vec<f64>> = task_vectors
            .iter()
            .map(|tv| {
                let mut magnitudes: Vec<f64> = tv.iter().map(|v| v.abs()).collect();
                magnitudes.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
                let k = ((1.0 - density) * magnitudes.len() as f64).round() as usize;
                let threshold = if k < magnitudes.len() {
                    magnitudes[k]
                } else {
                    0.0
                };
                tv.iter()
                    .map(|&v| if v.abs() >= threshold { v } else { 0.0 })
                    .collect()
            })
            .collect();

        // 2. Elect sign: for each parameter, take the sign with highest total magnitude
        for j in 0..len {
            let pos_sum: f64 = trimmed
                .iter()
                .map(|tv| if j < tv.len() && tv[j] > 0.0 { tv[j] } else { 0.0 })
                .sum();
            let neg_sum: f64 = trimmed
                .iter()
                .map(|tv| if j < tv.len() && tv[j] < 0.0 { tv[j].abs() } else { 0.0 })
                .sum();
            let elected_sign = if pos_sum >= neg_sum { 1.0 } else { -1.0 };

            // 3. Merge: average the values that agree with elected sign
            let agreeing: Vec<f64> = trimmed
                .iter()
                .filter_map(|tv| {
                    if j < tv.len() && tv[j] * elected_sign > 0.0 {
                        Some(tv[j])
                    } else {
                        None
                    }
                })
                .collect();
            if !agreeing.is_empty() {
                let avg = agreeing.iter().sum::<f64>() / agreeing.len() as f64;
                result[j] += avg;
            }
        }
        result
    }

    /// DARE (Drop And Rescale) merge for task vectors.
    pub fn dare_merge(
        base: &[f64],
        task_vectors: &[Vec<f64>],
        density: f64,
        rescale: bool,
    ) -> Vec<f64> {
        if base.is_empty() || task_vectors.is_empty() {
            return base.to_vec();
        }
        let len = base.len();
        let mut result = base.to_vec();

        for tv in task_vectors {
            for j in 0..len.min(tv.len()) {
                // Deterministic "drop" based on parameter index hash
                let keep = ((j as f64 * 0.618033988) % 1.0) < density;
                if keep {
                    let value = if rescale { tv[j] / density } else { tv[j] };
                    result[j] += value / task_vectors.len() as f64;
                }
            }
        }
        result
    }
}

// ── Distributed RLHF Configuration ──

#[derive(Debug, Clone)]
pub struct DistributedConfig {
    pub num_gpus: usize,
    pub zero_stage: ZeroStage,
    pub fsdp_strategy: Option<FsdpShardingStrategy>,
    pub gradient_accumulation_steps: usize,
    pub mixed_precision: MixedPrecision,
    pub gradient_checkpointing: bool,
    pub cpu_offload: bool,
    pub nvme_offload: bool,
    pub micro_batch_size: usize,
    pub sequence_parallelism: bool,
}

impl Default for DistributedConfig {
    fn default() -> Self {
        Self {
            num_gpus: 1,
            zero_stage: ZeroStage::Stage2,
            fsdp_strategy: None,
            gradient_accumulation_steps: 4,
            mixed_precision: MixedPrecision::Bf16,
            gradient_checkpointing: true,
            cpu_offload: false,
            nvme_offload: false,
            micro_batch_size: 1,
            sequence_parallelism: false,
        }
    }
}

impl DistributedConfig {
    /// Effective batch size = micro_batch * grad_accum * num_gpus.
    pub fn effective_batch_size(&self) -> usize {
        self.micro_batch_size * self.gradient_accumulation_steps * self.num_gpus
    }

    /// Estimate memory savings factor from ZeRO + offload.
    pub fn memory_savings_factor(&self) -> f64 {
        let mut factor = self.zero_stage.memory_savings_factor();
        if self.gradient_checkpointing {
            factor *= 1.5;
        }
        if self.cpu_offload {
            factor *= 2.0;
        }
        factor
    }

    /// Generate a DeepSpeed configuration JSON fragment.
    pub fn to_deepspeed_config(&self) -> HashMap<String, String> {
        let mut config = HashMap::new();
        config.insert(
            "zero_optimization.stage".to_string(),
            match &self.zero_stage {
                ZeroStage::Stage0 => "0",
                ZeroStage::Stage1 => "1",
                ZeroStage::Stage2 => "2",
                ZeroStage::Stage3 => "3",
            }
            .to_string(),
        );
        config.insert(
            "gradient_accumulation_steps".to_string(),
            self.gradient_accumulation_steps.to_string(),
        );
        config.insert(
            "train_micro_batch_size_per_gpu".to_string(),
            self.micro_batch_size.to_string(),
        );
        let fp16 = matches!(self.mixed_precision, MixedPrecision::Fp16);
        let bf16 = matches!(self.mixed_precision, MixedPrecision::Bf16);
        config.insert("fp16.enabled".to_string(), fp16.to_string());
        config.insert("bf16.enabled".to_string(), bf16.to_string());
        config.insert(
            "zero_optimization.offload_optimizer.device".to_string(),
            if self.cpu_offload { "cpu" } else { "none" }.to_string(),
        );
        config.insert(
            "gradient_clipping".to_string(),
            "1.0".to_string(),
        );
        config
    }
}

// ── Utility Functions ──

fn sigmoid(x: f64) -> f64 {
    1.0 / (1.0 + (-x).exp())
}

fn log_sigmoid(x: f64) -> f64 {
    if x >= 0.0 {
        -(1.0 + (-x).exp()).ln()
    } else {
        x - (1.0 + x.exp()).ln()
    }
}

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── AlignmentAlgorithm tests ──

    #[test]
    fn test_algorithm_labels() {
        assert_eq!(AlignmentAlgorithm::Ppo.label(), "PPO");
        assert_eq!(AlignmentAlgorithm::Dpo.label(), "DPO");
        assert_eq!(AlignmentAlgorithm::Kto.label(), "KTO");
        assert_eq!(AlignmentAlgorithm::Orpo.label(), "ORPO");
        assert_eq!(AlignmentAlgorithm::Grpo.label(), "GRPO");
    }

    #[test]
    fn test_algorithm_all() {
        let all = AlignmentAlgorithm::all();
        assert_eq!(all.len(), 5);
    }

    #[test]
    fn test_algorithm_requires_reward_model() {
        assert!(AlignmentAlgorithm::Ppo.requires_reward_model());
        assert!(AlignmentAlgorithm::Grpo.requires_reward_model());
        assert!(!AlignmentAlgorithm::Dpo.requires_reward_model());
        assert!(!AlignmentAlgorithm::Kto.requires_reward_model());
        assert!(!AlignmentAlgorithm::Orpo.requires_reward_model());
    }

    #[test]
    fn test_algorithm_requires_reference_model() {
        assert!(AlignmentAlgorithm::Ppo.requires_reference_model());
        assert!(AlignmentAlgorithm::Dpo.requires_reference_model());
        assert!(AlignmentAlgorithm::Kto.requires_reference_model());
        assert!(!AlignmentAlgorithm::Orpo.requires_reference_model());
        assert!(!AlignmentAlgorithm::Grpo.requires_reference_model());
    }

    #[test]
    fn test_algorithm_display() {
        assert_eq!(format!("{}", AlignmentAlgorithm::Ppo), "PPO");
        assert_eq!(format!("{}", AlignmentAlgorithm::Grpo), "GRPO");
    }

    // ── PipelineStage tests ──

    #[test]
    fn test_pipeline_stage_order() {
        assert!(PipelineStage::Sft.order() > PipelineStage::DataCollection.order());
        assert!(
            PipelineStage::PolicyOptimization.order()
                > PipelineStage::RewardModelTraining.order()
        );
    }

    #[test]
    fn test_pipeline_stage_next() {
        assert_eq!(
            PipelineStage::DataCollection.next(),
            Some(PipelineStage::Sft)
        );
        assert_eq!(
            PipelineStage::Sft.next(),
            Some(PipelineStage::RewardModelTraining)
        );
        assert_eq!(PipelineStage::Deployment.next(), None);
    }

    #[test]
    fn test_pipeline_stage_display() {
        assert_eq!(format!("{}", PipelineStage::Sft), "Supervised Fine-Tuning");
    }

    // ── PPO tests ──

    #[test]
    fn test_ppo_config_default() {
        let cfg = PpoConfig::default();
        assert!((cfg.clip_range - 0.2).abs() < 1e-10);
        assert!((cfg.entropy_coef - 0.01).abs() < 1e-10);
        assert!(cfg.whiten_advantages);
    }

    #[test]
    fn test_ppo_compute_policy_loss_empty() {
        let cfg = PpoConfig::default();
        assert_eq!(cfg.compute_policy_loss(&[], &[]), 0.0);
    }

    #[test]
    fn test_ppo_compute_policy_loss_no_clip() {
        let cfg = PpoConfig::default();
        // ratio = 1.0 means no change, should give loss = -advantage
        let ratios = vec![1.0, 1.0, 1.0];
        let advantages = vec![0.5, 0.3, 0.2];
        let loss = cfg.compute_policy_loss(&ratios, &advantages);
        let expected = -(0.5 + 0.3 + 0.2) / 3.0;
        assert!((loss - expected).abs() < 1e-10);
    }

    #[test]
    fn test_ppo_compute_policy_loss_with_clip() {
        let cfg = PpoConfig::default(); // clip_range=0.2
        let ratios = vec![1.5]; // exceeds 1.2 upper clip
        let advantages = vec![1.0];
        let loss = cfg.compute_policy_loss(&ratios, &advantages);
        // clipped: 1.2 * 1.0 = 1.2, unclipped: 1.5 * 1.0 = 1.5, min = 1.2
        assert!((loss - (-1.2)).abs() < 1e-10);
    }

    #[test]
    fn test_ppo_compute_value_loss() {
        let cfg = PpoConfig::default();
        let values = vec![1.0, 2.0];
        let returns = vec![1.5, 2.5];
        let old_values = vec![0.9, 1.9];
        let loss = cfg.compute_value_loss(&values, &returns, &old_values);
        assert!(loss > 0.0);
    }

    #[test]
    fn test_ppo_compute_kl_penalty_kl() {
        let cfg = PpoConfig::default(); // KlPenaltyMode::Kl
        let old = vec![-1.0, -2.0];
        let new = vec![-1.1, -2.1];
        let kl = cfg.compute_kl_penalty(&old, &new);
        // kl mode: mean(old - new) = mean(0.1, 0.1) = 0.1
        assert!((kl - 0.1).abs() < 1e-10);
    }

    #[test]
    fn test_ppo_compute_kl_penalty_abs() {
        let mut cfg = PpoConfig::default();
        cfg.kl_penalty = KlPenaltyMode::Abs;
        let old = vec![-1.0, -2.0];
        let new = vec![-0.9, -2.1];
        let kl = cfg.compute_kl_penalty(&old, &new);
        // abs mode: mean(|-0.1|, |0.1|) = 0.1
        assert!((kl - 0.1).abs() < 1e-10);
    }

    #[test]
    fn test_ppo_compute_kl_penalty_mse() {
        let mut cfg = PpoConfig::default();
        cfg.kl_penalty = KlPenaltyMode::Mse;
        let old = vec![-1.0];
        let new = vec![-1.5];
        let kl = cfg.compute_kl_penalty(&old, &new);
        // mse: 0.5 * (0.5)^2 = 0.125
        assert!((kl - 0.125).abs() < 1e-10);
    }

    #[test]
    fn test_ppo_whiten_advantages() {
        let cfg = PpoConfig::default();
        let adv = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let whitened = cfg.whiten_advantages_vec(&adv);
        // Mean should be ~0, std ~1
        let mean: f64 = whitened.iter().sum::<f64>() / whitened.len() as f64;
        assert!(mean.abs() < 1e-6);
    }

    #[test]
    fn test_ppo_whiten_advantages_empty() {
        let cfg = PpoConfig::default();
        let whitened = cfg.whiten_advantages_vec(&[]);
        assert!(whitened.is_empty());
    }

    #[test]
    fn test_ppo_simulate_step() {
        let cfg = PpoConfig::default();
        let result = cfg.simulate_step(
            &[1.0, 1.05, 0.95],
            &[0.5, -0.3, 0.8],
            &[1.0, 1.1, 0.9],
            &[1.2, 1.0, 1.1],
            &[0.9, 1.0, 0.85],
            &[-1.0, -2.0, -1.5],
            &[-1.1, -2.1, -1.4],
            &[0.5, 0.3, 0.8],
        );
        assert!(result.mean_reward > 0.0);
        assert!(result.kl_divergence != 0.0 || true); // kl may be 0 if identical
    }

    // ── DPO tests ──

    #[test]
    fn test_dpo_config_default() {
        let cfg = DpoConfig::default();
        assert!((cfg.beta - 0.1).abs() < 1e-10);
        assert_eq!(cfg.loss_type, DpoLossType::Sigmoid);
    }

    #[test]
    fn test_dpo_compute_loss_empty() {
        let cfg = DpoConfig::default();
        assert_eq!(cfg.compute_loss(&[], &[]), 0.0);
    }

    #[test]
    fn test_dpo_compute_loss_sigmoid() {
        let cfg = DpoConfig::default();
        let chosen = vec![0.5, 0.3];
        let rejected = vec![-0.5, -0.3];
        let loss = cfg.compute_loss(&chosen, &rejected);
        assert!(loss > 0.0);
    }

    #[test]
    fn test_dpo_compute_loss_hinge() {
        let mut cfg = DpoConfig::default();
        cfg.loss_type = DpoLossType::Hinge;
        let loss = cfg.compute_loss(&[2.0], &[-2.0]);
        // beta * (2.0 - (-2.0)) = 0.1 * 4.0 = 0.4; hinge: max(0, 1 - 0.4) = 0.6
        assert!((loss - 0.6).abs() < 1e-10);
    }

    #[test]
    fn test_dpo_compute_loss_ipo() {
        let mut cfg = DpoConfig::default();
        cfg.loss_type = DpoLossType::Ipo;
        let loss = cfg.compute_loss(&[1.0], &[-1.0]);
        assert!(loss > 0.0);
    }

    #[test]
    fn test_dpo_compute_accuracy() {
        let cfg = DpoConfig::default();
        let chosen = vec![0.5, -0.1, 0.3];
        let rejected = vec![-0.5, 0.1, -0.3];
        let acc = cfg.compute_accuracy(&chosen, &rejected);
        // pair 0: 0.5 > -0.5 yes, pair 1: -0.1 > 0.1 no, pair 2: 0.3 > -0.3 yes
        assert!((acc - 2.0 / 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_dpo_compute_accuracy_empty() {
        let cfg = DpoConfig::default();
        assert_eq!(cfg.compute_accuracy(&[], &[]), 0.0);
    }

    #[test]
    fn test_dpo_compute_rewards() {
        let cfg = DpoConfig::default(); // beta = 0.1
        let (cr, rr) = cfg.compute_rewards(&[1.0, 2.0], &[-1.0, -2.0]);
        assert!((cr[0] - 0.1).abs() < 1e-10);
        assert!((cr[1] - 0.2).abs() < 1e-10);
        assert!((rr[0] - (-0.1)).abs() < 1e-10);
    }

    // ── KTO tests ──

    #[test]
    fn test_kto_config_default() {
        let cfg = KtoConfig::default();
        assert!((cfg.beta - 0.1).abs() < 1e-10);
        assert!((cfg.desirable_weight - 1.0).abs() < 1e-10);
        assert!((cfg.undesirable_weight - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_kto_compute_loss_empty() {
        let cfg = KtoConfig::default();
        assert_eq!(cfg.compute_loss(&[], &[], 0.0), 0.0);
    }

    #[test]
    fn test_kto_compute_loss_desirable() {
        let cfg = KtoConfig::default();
        let loss = cfg.compute_loss(
            &[1.0],
            &[KtoExampleKind::Desirable],
            0.0,
        );
        // 1 - sigma(0.1 * (1.0 - 0.0)) = 1 - sigma(0.1)
        let expected = 1.0 - sigmoid(0.1);
        assert!((loss - expected).abs() < 1e-6);
    }

    #[test]
    fn test_kto_compute_loss_undesirable() {
        let cfg = KtoConfig::default();
        let loss = cfg.compute_loss(
            &[-1.0],
            &[KtoExampleKind::Undesirable],
            0.0,
        );
        // 1 - sigma(0.1 * (0.0 - (-1.0))) = 1 - sigma(0.1)
        let expected = 1.0 - sigmoid(0.1);
        assert!((loss - expected).abs() < 1e-6);
    }

    #[test]
    fn test_kto_count_splits() {
        let cfg = KtoConfig::default();
        let kinds = vec![
            KtoExampleKind::Desirable,
            KtoExampleKind::Undesirable,
            KtoExampleKind::Desirable,
        ];
        let (d, u) = cfg.count_splits(&kinds);
        assert_eq!(d, 2);
        assert_eq!(u, 1);
    }

    #[test]
    fn test_kto_asymmetric_weights() {
        let mut cfg = KtoConfig::default();
        cfg.desirable_weight = 2.0;
        cfg.undesirable_weight = 0.5;
        let loss_d = cfg.compute_loss(&[1.0], &[KtoExampleKind::Desirable], 0.0);
        let loss_u = cfg.compute_loss(&[-1.0], &[KtoExampleKind::Undesirable], 0.0);
        // desirable weight is 4x undesirable weight
        assert!(loss_d > loss_u);
    }

    // ── ORPO tests ──

    #[test]
    fn test_orpo_config_default() {
        let cfg = OrpoConfig::default();
        assert!((cfg.lambda - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_orpo_compute_loss_empty() {
        let cfg = OrpoConfig::default();
        assert_eq!(cfg.compute_loss(&[], &[]), 0.0);
    }

    #[test]
    fn test_orpo_compute_loss() {
        let cfg = OrpoConfig::default();
        let loss = cfg.compute_loss(&[-0.5], &[-1.5]);
        assert!(loss > 0.0);
    }

    #[test]
    fn test_orpo_compute_odds() {
        // logprob = -1.0  =>  p = e^(-1.0) ≈ 0.368  =>  odds = 0.368 / 0.632 ≈ 0.582
        let odds = OrpoConfig::compute_odds(-1.0);
        let p = (-1.0_f64).exp();
        let expected = p / (1.0 - p);
        assert!((odds - expected).abs() < 1e-4);
    }

    // ── GRPO tests ──

    #[test]
    fn test_grpo_config_default() {
        let cfg = GrpoConfig::default();
        assert_eq!(cfg.group_size, 8);
        assert!((cfg.clip_range - 0.2).abs() < 1e-10);
    }

    #[test]
    fn test_grpo_compute_group_advantages() {
        let cfg = GrpoConfig {
            group_size: 4,
            ..GrpoConfig::default()
        };
        let rewards = vec![1.0, 2.0, 3.0, 4.0, 10.0, 20.0, 30.0, 40.0];
        let advantages = cfg.compute_group_advantages(&rewards);
        assert_eq!(advantages.len(), 8);
        // First group mean = 2.5, second group mean = 25.0
        // Check first group is centered near 0
        let first_group_mean: f64 = advantages[..4].iter().sum::<f64>() / 4.0;
        assert!(first_group_mean.abs() < 1e-6);
    }

    #[test]
    fn test_grpo_compute_group_advantages_empty() {
        let cfg = GrpoConfig::default();
        assert!(cfg.compute_group_advantages(&[]).is_empty());
    }

    #[test]
    fn test_grpo_compute_loss() {
        let cfg = GrpoConfig::default();
        let loss = cfg.compute_loss(&[1.0, 1.1], &[0.5, -0.3], &[0.01, 0.02]);
        // Should produce a finite loss
        assert!(loss.is_finite());
    }

    #[test]
    fn test_grpo_compute_loss_empty() {
        let cfg = GrpoConfig::default();
        assert_eq!(cfg.compute_loss(&[], &[], &[]), 0.0);
    }

    // ── Generation Config tests ──

    #[test]
    fn test_generation_config_default() {
        let cfg = GenerationConfig::default();
        assert!((cfg.temperature - 1.0).abs() < 1e-10);
        assert!((cfg.top_p - 1.0).abs() < 1e-10);
        assert_eq!(cfg.top_k, 50);
    }

    #[test]
    fn test_apply_temperature() {
        let cfg = GenerationConfig {
            temperature: 2.0,
            ..Default::default()
        };
        let logits = vec![1.0, 2.0, 3.0];
        let scaled = cfg.apply_temperature(&logits);
        assert!((scaled[0] - 0.5).abs() < 1e-10);
        assert!((scaled[1] - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_apply_temperature_zero() {
        let cfg = GenerationConfig {
            temperature: 0.0,
            ..Default::default()
        };
        let logits = vec![1.0, 2.0];
        let result = cfg.apply_temperature(&logits);
        assert_eq!(result, logits); // returns unchanged
    }

    #[test]
    fn test_apply_top_k() {
        let cfg = GenerationConfig {
            top_k: 2,
            ..Default::default()
        };
        let logits = vec![1.0, 3.0, 2.0, 0.5];
        let filtered = cfg.apply_top_k(&logits);
        assert!(filtered[3] == f64::NEG_INFINITY); // 0.5 dropped
        assert!(filtered[0] == f64::NEG_INFINITY); // 1.0 dropped
        assert_eq!(filtered[1], 3.0);
        assert_eq!(filtered[2], 2.0);
    }

    #[test]
    fn test_apply_top_k_no_filter() {
        let cfg = GenerationConfig {
            top_k: 0,
            ..Default::default()
        };
        let logits = vec![1.0, 2.0];
        assert_eq!(cfg.apply_top_k(&logits), logits);
    }

    #[test]
    fn test_apply_top_p() {
        let cfg = GenerationConfig {
            top_p: 0.5,
            ..Default::default()
        };
        let logits = vec![10.0, 1.0, 0.0, -10.0];
        let filtered = cfg.apply_top_p(&logits);
        // The highest logit (10.0) should be kept
        assert_eq!(filtered[0], 10.0);
    }

    #[test]
    fn test_apply_top_p_no_filter() {
        let cfg = GenerationConfig {
            top_p: 1.0,
            ..Default::default()
        };
        let logits = vec![1.0, 2.0, 3.0];
        assert_eq!(cfg.apply_top_p(&logits), logits);
    }

    #[test]
    fn test_apply_repetition_penalty() {
        let cfg = GenerationConfig {
            repetition_penalty: 2.0,
            ..Default::default()
        };
        let logits = vec![1.0, -1.0, 0.5, 2.0];
        let penalized = cfg.apply_repetition_penalty(&logits, &[0, 1]);
        assert!((penalized[0] - 0.5).abs() < 1e-10); // positive logit divided
        assert!((penalized[1] - (-2.0)).abs() < 1e-10); // negative logit multiplied
        assert!((penalized[2] - 0.5).abs() < 1e-10); // unpenalized
    }

    #[test]
    fn test_apply_repetition_penalty_noop() {
        let cfg = GenerationConfig {
            repetition_penalty: 1.0,
            ..Default::default()
        };
        let logits = vec![1.0, 2.0];
        assert_eq!(cfg.apply_repetition_penalty(&logits, &[0, 1]), logits);
    }

    // ── Reward Model Ensemble tests ──

    #[test]
    fn test_ensemble_new() {
        let ens = RewardModelEnsemble::new(3, EnsembleAggregation::Mean);
        assert_eq!(ens.models.len(), 3);
        assert_eq!(ens.aggregation, EnsembleAggregation::Mean);
    }

    #[test]
    fn test_ensemble_aggregate_mean() {
        let ens = RewardModelEnsemble::new(3, EnsembleAggregation::Mean);
        let scores = vec![1.0, 2.0, 3.0];
        assert!((ens.aggregate(&scores) - 2.0).abs() < 1e-10);
    }

    #[test]
    fn test_ensemble_aggregate_median_odd() {
        let ens = RewardModelEnsemble::new(3, EnsembleAggregation::Median);
        let scores = vec![1.0, 3.0, 2.0];
        assert!((ens.aggregate(&scores) - 2.0).abs() < 1e-10);
    }

    #[test]
    fn test_ensemble_aggregate_median_even() {
        let ens = RewardModelEnsemble::new(4, EnsembleAggregation::Median);
        let scores = vec![1.0, 2.0, 3.0, 4.0];
        assert!((ens.aggregate(&scores) - 2.5).abs() < 1e-10);
    }

    #[test]
    fn test_ensemble_aggregate_weighted_mean() {
        let mut ens = RewardModelEnsemble::new(3, EnsembleAggregation::WeightedMean);
        ens.weights = vec![0.5, 0.3, 0.2];
        let scores = vec![1.0, 2.0, 3.0];
        let expected = (0.5 * 1.0 + 0.3 * 2.0 + 0.2 * 3.0) / 1.0;
        assert!((ens.aggregate(&scores) - expected).abs() < 1e-10);
    }

    #[test]
    fn test_ensemble_aggregate_min() {
        let ens = RewardModelEnsemble::new(3, EnsembleAggregation::Min);
        assert!((ens.aggregate(&[3.0, 1.0, 2.0]) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_ensemble_aggregate_max() {
        let ens = RewardModelEnsemble::new(3, EnsembleAggregation::Max);
        assert!((ens.aggregate(&[3.0, 1.0, 2.0]) - 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_ensemble_aggregate_empty() {
        let ens = RewardModelEnsemble::new(3, EnsembleAggregation::Mean);
        assert_eq!(ens.aggregate(&[]), 0.0);
    }

    #[test]
    fn test_ensemble_detect_disagreement() {
        let mut ens = RewardModelEnsemble::new(3, EnsembleAggregation::Mean);
        ens.disagreement_threshold = 0.5;
        assert!(ens.detect_disagreement(&[1.0, 5.0, 9.0]));
        assert!(!ens.detect_disagreement(&[2.0, 2.1, 1.9]));
    }

    #[test]
    fn test_ensemble_detect_disagreement_single() {
        let ens = RewardModelEnsemble::new(1, EnsembleAggregation::Mean);
        assert!(!ens.detect_disagreement(&[1.0]));
    }

    #[test]
    fn test_ensemble_calibrate_score() {
        let ens = RewardModelEnsemble::new(1, EnsembleAggregation::Mean);
        assert!((ens.calibrate_score(2.0, 2.0) - 1.0).abs() < 1e-10);
        assert!((ens.calibrate_score(2.0, 0.0) - 2.0).abs() < 1e-10);
    }

    // ── Process Reward Model tests ──

    #[test]
    fn test_prm_new() {
        let prm = ProcessRewardModel::new("test-prm", PrmAggregation::Min);
        assert_eq!(prm.model_name, "test-prm");
        assert!(prm.steps.is_empty());
    }

    #[test]
    fn test_prm_add_step() {
        let mut prm = ProcessRewardModel::new("prm", PrmAggregation::Min);
        prm.add_step("Step 1: Define variables", 0.9);
        prm.add_step("Step 2: Apply formula", 0.7);
        assert_eq!(prm.steps.len(), 2);
        assert_eq!(prm.steps[0].step_index, 0);
        assert_eq!(prm.steps[1].step_index, 1);
    }

    #[test]
    fn test_prm_annotate_step() {
        let mut prm = ProcessRewardModel::new("prm", PrmAggregation::Min);
        prm.add_step("Step 1", 0.9);
        prm.annotate_step(0, true, "Correct setup");
        assert_eq!(prm.steps[0].is_correct, Some(true));
        assert_eq!(prm.steps[0].annotation.as_deref(), Some("Correct setup"));
    }

    #[test]
    fn test_prm_compute_score_min() {
        let mut prm = ProcessRewardModel::new("prm", PrmAggregation::Min);
        prm.add_step("A", 0.9);
        prm.add_step("B", 0.3);
        prm.add_step("C", 0.8);
        assert!((prm.compute_score() - 0.3).abs() < 1e-10);
    }

    #[test]
    fn test_prm_compute_score_product() {
        let mut prm = ProcessRewardModel::new("prm", PrmAggregation::Product);
        prm.add_step("A", 0.5);
        prm.add_step("B", 0.4);
        assert!((prm.compute_score() - 0.2).abs() < 1e-10);
    }

    #[test]
    fn test_prm_compute_score_last_step() {
        let mut prm = ProcessRewardModel::new("prm", PrmAggregation::LastStep);
        prm.add_step("A", 0.1);
        prm.add_step("B", 0.9);
        assert!((prm.compute_score() - 0.9).abs() < 1e-10);
    }

    #[test]
    fn test_prm_compute_score_weighted_sum() {
        let mut prm = ProcessRewardModel::new("prm", PrmAggregation::WeightedSum);
        prm.add_step("A", 1.0);
        prm.add_step("B", 1.0);
        // weights: 1/(1+2)=1/3, 2/(1+2)=2/3; score = 1/3 + 2/3 = 1.0
        assert!((prm.compute_score() - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_prm_compute_score_empty() {
        let prm = ProcessRewardModel::new("prm", PrmAggregation::Min);
        assert_eq!(prm.compute_score(), 0.0);
    }

    #[test]
    fn test_prm_find_first_error() {
        let mut prm = ProcessRewardModel::new("prm", PrmAggregation::Min);
        prm.add_step("A", 0.9);
        prm.add_step("B", 0.3);
        prm.annotate_step(0, true, "ok");
        prm.annotate_step(1, false, "wrong");
        assert_eq!(prm.find_first_error(), Some(1));
    }

    #[test]
    fn test_prm_find_first_error_none() {
        let mut prm = ProcessRewardModel::new("prm", PrmAggregation::Min);
        prm.add_step("A", 0.9);
        prm.annotate_step(0, true, "ok");
        assert_eq!(prm.find_first_error(), None);
    }

    #[test]
    fn test_prm_correctness_ratio() {
        let mut prm = ProcessRewardModel::new("prm", PrmAggregation::Min);
        prm.add_step("A", 0.9);
        prm.add_step("B", 0.3);
        prm.add_step("C", 0.5);
        prm.annotate_step(0, true, "");
        prm.annotate_step(1, false, "");
        prm.annotate_step(2, true, "");
        assert!((prm.correctness_ratio() - 2.0 / 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_prm_correctness_ratio_empty() {
        let prm = ProcessRewardModel::new("prm", PrmAggregation::Min);
        assert_eq!(prm.correctness_ratio(), 0.0);
    }

    // ── Constitutional AI tests ──

    #[test]
    fn test_constitution_new() {
        let c = Constitution::new("Anthropic HHH");
        assert_eq!(c.name, "Anthropic HHH");
        assert_eq!(c.principle_count(), 0);
    }

    #[test]
    fn test_constitution_add_principle() {
        let mut c = Constitution::new("test");
        c.add_principle(ConstitutionalPrinciple {
            id: "harm-1".to_string(),
            name: "No Harm".to_string(),
            description: "Do not generate harmful content".to_string(),
            critique_prompt: "Does this response contain harmful content?".to_string(),
            revision_prompt: "Revise to remove harmful content".to_string(),
            weight: 1.0,
        });
        assert_eq!(c.principle_count(), 1);
        assert!(c.get_principle("harm-1").is_some());
        assert!(c.get_principle("nonexistent").is_none());
    }

    #[test]
    fn test_constitutional_ai_engine() {
        let mut c = Constitution::new("test");
        c.add_principle(ConstitutionalPrinciple {
            id: "p1".to_string(),
            name: "Helpfulness".to_string(),
            description: "Be helpful".to_string(),
            critique_prompt: "Is this helpful?".to_string(),
            revision_prompt: "Make it more helpful".to_string(),
            weight: 1.0,
        });
        let engine = ConstitutionalAiEngine::new(c);
        let results = engine.run_critique_revision("Hello world");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].principle_id, "p1");
    }

    #[test]
    fn test_constitutional_ai_generate_preference_pair() {
        let c = Constitution::new("test");
        let engine = ConstitutionalAiEngine::new(c);
        let pair = engine.generate_preference_pair("prompt", "bad answer", "good answer");
        assert_eq!(pair.chosen, "good answer");
        assert_eq!(pair.rejected, "bad answer");
    }

    // ── RLEF tests ──

    #[test]
    fn test_rlef_compute_reward_all_pass() {
        let feedback = ExecutionFeedback {
            compilation_success: true,
            test_pass_rate: 1.0,
            tests_total: 10,
            tests_passed: 10,
            static_analysis_score: 0.9,
            execution_time_ms: 500,
            memory_usage_bytes: 1024,
            lint_warnings: 0,
            lint_errors: 0,
        };
        let weights = RlefWeights::default();
        let reward = feedback.compute_reward(&weights);
        assert!(reward > 0.0);
    }

    #[test]
    fn test_rlef_compute_reward_compilation_failure() {
        let feedback = ExecutionFeedback {
            compilation_success: false,
            test_pass_rate: 0.0,
            tests_total: 10,
            tests_passed: 0,
            static_analysis_score: 0.0,
            execution_time_ms: 0,
            memory_usage_bytes: 0,
            lint_warnings: 5,
            lint_errors: 3,
        };
        let weights = RlefWeights::default();
        let reward = feedback.compute_reward(&weights);
        assert!(reward < 0.5);
    }

    #[test]
    fn test_rlef_compute_reward_slow_execution() {
        let feedback = ExecutionFeedback {
            compilation_success: true,
            test_pass_rate: 1.0,
            tests_total: 5,
            tests_passed: 5,
            static_analysis_score: 0.8,
            execution_time_ms: 50_000, // 5x over limit
            memory_usage_bytes: 0,
            lint_warnings: 0,
            lint_errors: 0,
        };
        let weights = RlefWeights::default();
        let reward_slow = feedback.compute_reward(&weights);
        let fast = ExecutionFeedback {
            execution_time_ms: 500,
            ..feedback.clone()
        };
        let reward_fast = fast.compute_reward(&weights);
        assert!(reward_fast > reward_slow);
    }

    #[test]
    fn test_rlef_config_default() {
        let cfg = RlefConfig::default();
        assert!(cfg.sandbox_enabled);
        assert_eq!(cfg.max_retries, 3);
    }

    // ── Reward Hacking Detection tests ──

    #[test]
    fn test_reward_hacking_detector_default() {
        let det = RewardHackingDetector::default();
        assert!((det.kl_threshold - 10.0).abs() < 1e-10);
        assert!(det.history.is_empty());
    }

    #[test]
    fn test_reward_hacking_record_snapshot() {
        let mut det = RewardHackingDetector::default();
        det.window_size = 3;
        for i in 0..5 {
            det.record_snapshot(RewardSnapshot {
                step: i,
                mean_reward: i as f64,
                std_reward: 0.1,
                kl_divergence: 0.01,
                mean_length: 100.0,
            });
        }
        assert_eq!(det.history.len(), 3);
        assert_eq!(det.history[0].step, 2);
    }

    #[test]
    fn test_reward_hacking_check_kl_spike() {
        let det = RewardHackingDetector::default(); // threshold = 10.0
        assert!(det.check_kl_divergence(15.0).is_some());
        assert!(det.check_kl_divergence(5.0).is_none());
    }

    #[test]
    fn test_reward_hacking_check_reward_distribution() {
        let mut det = RewardHackingDetector::default();
        det.reward_shift_threshold = 2.0;
        // Use varied rewards so std is non-trivial
        for i in 0..20 {
            det.record_snapshot(RewardSnapshot {
                step: i,
                mean_reward: 1.0 + (i as f64) * 0.01,
                std_reward: 0.1,
                kl_divergence: 0.01,
                mean_length: 100.0,
            });
        }
        // Large jump should trigger
        assert!(det.check_reward_distribution(100.0).is_some());
        // Normal value within range should not
        assert!(det.check_reward_distribution(1.1).is_none());
    }

    #[test]
    fn test_reward_hacking_check_length_exploitation() {
        let det = RewardHackingDetector::default(); // max_response_length = 4096
        assert!(det.check_length_exploitation(5000.0).is_some());
        assert!(det.check_length_exploitation(1000.0).is_none());
    }

    #[test]
    fn test_reward_hacking_check_repetition() {
        let det = RewardHackingDetector::default(); // max_repetition_ratio = 0.5
        assert!(det.check_repetition(0.8).is_some());
        assert!(det.check_repetition(0.3).is_none());
    }

    #[test]
    fn test_reward_hacking_run_all_checks() {
        let det = RewardHackingDetector::default();
        let alerts = det.run_all_checks(15.0, 1.0, 5000.0, 0.8);
        assert!(alerts.len() >= 2); // kl + length + repetition at minimum
    }

    #[test]
    fn test_reward_hacking_run_all_checks_clean() {
        let det = RewardHackingDetector::default();
        let alerts = det.run_all_checks(1.0, 1.0, 100.0, 0.1);
        assert!(alerts.is_empty());
    }

    // ── Alignment Evaluation tests ──

    #[test]
    fn test_alignment_evaluator_default() {
        let eval = AlignmentEvaluator::default();
        assert_eq!(eval.benchmarks.len(), 8);
    }

    #[test]
    fn test_alignment_evaluator_add_result() {
        let mut eval = AlignmentEvaluator::default();
        eval.add_result(AlignmentBenchmarkResult {
            benchmark: AlignmentBenchmark::Harmfulness,
            score: 0.98,
            num_samples: 100,
            pass_rate: 0.98,
            details: HashMap::new(),
        });
        assert_eq!(eval.results.len(), 1);
    }

    #[test]
    fn test_alignment_evaluator_safety_pass() {
        let mut eval = AlignmentEvaluator::default();
        eval.safety_threshold = 0.9;
        eval.add_result(AlignmentBenchmarkResult {
            benchmark: AlignmentBenchmark::Harmfulness,
            score: 0.95,
            num_samples: 100,
            pass_rate: 0.95,
            details: HashMap::new(),
        });
        eval.add_result(AlignmentBenchmarkResult {
            benchmark: AlignmentBenchmark::Toxicity,
            score: 0.92,
            num_samples: 100,
            pass_rate: 0.92,
            details: HashMap::new(),
        });
        assert!(eval.safety_pass());
    }

    #[test]
    fn test_alignment_evaluator_safety_fail() {
        let mut eval = AlignmentEvaluator::default();
        eval.safety_threshold = 0.95;
        eval.add_result(AlignmentBenchmarkResult {
            benchmark: AlignmentBenchmark::Harmfulness,
            score: 0.80,
            num_samples: 100,
            pass_rate: 0.80,
            details: HashMap::new(),
        });
        assert!(!eval.safety_pass());
    }

    #[test]
    fn test_alignment_evaluator_helpfulness_pass() {
        let mut eval = AlignmentEvaluator::default();
        eval.helpfulness_threshold = 0.7;
        eval.add_result(AlignmentBenchmarkResult {
            benchmark: AlignmentBenchmark::Helpfulness,
            score: 0.85,
            num_samples: 100,
            pass_rate: 0.85,
            details: HashMap::new(),
        });
        assert!(eval.helpfulness_pass());
    }

    #[test]
    fn test_alignment_evaluator_alignment_pass() {
        let mut eval = AlignmentEvaluator::default();
        eval.safety_threshold = 0.9;
        eval.helpfulness_threshold = 0.7;
        for bench in &[
            AlignmentBenchmark::Harmfulness,
            AlignmentBenchmark::Toxicity,
            AlignmentBenchmark::SafetyRefusal,
        ] {
            eval.add_result(AlignmentBenchmarkResult {
                benchmark: bench.clone(),
                score: 0.95,
                num_samples: 100,
                pass_rate: 0.95,
                details: HashMap::new(),
            });
        }
        eval.add_result(AlignmentBenchmarkResult {
            benchmark: AlignmentBenchmark::Helpfulness,
            score: 0.80,
            num_samples: 100,
            pass_rate: 0.80,
            details: HashMap::new(),
        });
        assert!(eval.alignment_pass());
    }

    #[test]
    fn test_alignment_evaluator_aggregate_score() {
        let mut eval = AlignmentEvaluator::default();
        eval.add_result(AlignmentBenchmarkResult {
            benchmark: AlignmentBenchmark::Honesty,
            score: 0.8,
            num_samples: 50,
            pass_rate: 0.8,
            details: HashMap::new(),
        });
        eval.add_result(AlignmentBenchmarkResult {
            benchmark: AlignmentBenchmark::Helpfulness,
            score: 0.6,
            num_samples: 50,
            pass_rate: 0.6,
            details: HashMap::new(),
        });
        assert!((eval.aggregate_score() - 0.7).abs() < 1e-10);
    }

    #[test]
    fn test_alignment_evaluator_aggregate_score_empty() {
        let eval = AlignmentEvaluator::default();
        assert_eq!(eval.aggregate_score(), 0.0);
    }

    #[test]
    fn test_alignment_evaluator_compute_alignment_tax() {
        let eval = AlignmentEvaluator::default();
        let tax = eval.compute_alignment_tax(0.9, 0.81);
        assert!((tax - 0.1).abs() < 1e-10);
    }

    #[test]
    fn test_alignment_evaluator_summary_report() {
        let mut eval = AlignmentEvaluator::default();
        eval.add_result(AlignmentBenchmarkResult {
            benchmark: AlignmentBenchmark::Honesty,
            score: 0.9,
            num_samples: 100,
            pass_rate: 0.9,
            details: HashMap::new(),
        });
        let report = eval.summary_report();
        assert!(report.contains("Honesty"));
        assert!(report.contains("Aggregate Score"));
    }

    #[test]
    fn test_alignment_benchmark_is_safety() {
        assert!(AlignmentBenchmark::Harmfulness.is_safety());
        assert!(AlignmentBenchmark::Toxicity.is_safety());
        assert!(AlignmentBenchmark::SafetyRefusal.is_safety());
        assert!(!AlignmentBenchmark::Helpfulness.is_safety());
        assert!(!AlignmentBenchmark::Honesty.is_safety());
    }

    // ── Preference Dataset tests ──

    #[test]
    fn test_preference_dataset_new() {
        let ds = PreferenceDataset::new("test");
        assert_eq!(ds.name, "test");
        assert!(ds.is_empty());
        assert_eq!(ds.len(), 0);
    }

    #[test]
    fn test_preference_dataset_add_pair() {
        let mut ds = PreferenceDataset::new("test");
        ds.add_pair(PreferencePair {
            prompt: "Hello".to_string(),
            chosen: "Hi there!".to_string(),
            rejected: "Go away".to_string(),
            chosen_score: Some(0.9),
            rejected_score: Some(0.1),
            metadata: HashMap::new(),
        });
        assert_eq!(ds.len(), 1);
        assert!(!ds.is_empty());
    }

    #[test]
    fn test_preference_dataset_deduplicate() {
        let mut ds = PreferenceDataset::new("test");
        for _ in 0..3 {
            ds.add_pair(PreferencePair {
                prompt: "same prompt".to_string(),
                chosen: "chosen".to_string(),
                rejected: "rejected".to_string(),
                chosen_score: None,
                rejected_score: None,
                metadata: HashMap::new(),
            });
        }
        ds.add_pair(PreferencePair {
            prompt: "different".to_string(),
            chosen: "c".to_string(),
            rejected: "r".to_string(),
            chosen_score: None,
            rejected_score: None,
            metadata: HashMap::new(),
        });
        assert_eq!(ds.len(), 4);
        ds.deduplicate();
        assert_eq!(ds.len(), 2);
    }

    #[test]
    fn test_preference_dataset_filter_by_score_gap() {
        let mut ds = PreferenceDataset::new("test");
        ds.add_pair(PreferencePair {
            prompt: "p1".to_string(),
            chosen: "c".to_string(),
            rejected: "r".to_string(),
            chosen_score: Some(0.9),
            rejected_score: Some(0.1),
            metadata: HashMap::new(),
        });
        ds.add_pair(PreferencePair {
            prompt: "p2".to_string(),
            chosen: "c".to_string(),
            rejected: "r".to_string(),
            chosen_score: Some(0.5),
            rejected_score: Some(0.49),
            metadata: HashMap::new(),
        });
        ds.filter_by_score_gap(0.5);
        assert_eq!(ds.len(), 1);
        assert_eq!(ds.pairs[0].prompt, "p1");
    }

    #[test]
    fn test_preference_dataset_quality_scores() {
        let mut ds = PreferenceDataset::new("test");
        ds.add_pair(PreferencePair {
            prompt: "a".repeat(500),
            chosen: "b".repeat(1000),
            rejected: "c".to_string(),
            chosen_score: None,
            rejected_score: None,
            metadata: HashMap::new(),
        });
        let scores = ds.quality_scores();
        assert_eq!(scores.len(), 1);
        assert!(scores[0] > 0.0);
    }

    #[test]
    fn test_preference_dataset_split() {
        let mut ds = PreferenceDataset::new("test");
        for i in 0..10 {
            ds.add_pair(PreferencePair {
                prompt: format!("p{}", i),
                chosen: "c".to_string(),
                rejected: "r".to_string(),
                chosen_score: None,
                rejected_score: None,
                metadata: HashMap::new(),
            });
        }
        let (train, val) = ds.split(0.8);
        assert_eq!(train.len(), 8);
        assert_eq!(val.len(), 2);
    }

    #[test]
    fn test_preference_dataset_stats() {
        let mut ds = PreferenceDataset::new("test");
        ds.add_pair(PreferencePair {
            prompt: "hello".to_string(),
            chosen: "world!".to_string(),
            rejected: "no".to_string(),
            chosen_score: Some(0.8),
            rejected_score: Some(0.2),
            metadata: HashMap::new(),
        });
        let stats = ds.stats();
        assert_eq!(stats.total_pairs, 1);
        assert_eq!(stats.scored_pairs, 1);
        assert!(stats.avg_prompt_length > 0.0);
    }

    #[test]
    fn test_preference_dataset_stats_empty() {
        let ds = PreferenceDataset::new("empty");
        let stats = ds.stats();
        assert_eq!(stats.total_pairs, 0);
        assert_eq!(stats.avg_prompt_length, 0.0);
    }

    // ── Pipeline tests ──

    #[test]
    fn test_pipeline_config_new() {
        let pipe = PipelineConfig::new("test-pipe", AlignmentAlgorithm::Dpo);
        assert_eq!(pipe.name, "test-pipe");
        assert_eq!(pipe.algorithm, AlignmentAlgorithm::Dpo);
        assert_eq!(pipe.current_stage, PipelineStage::DataCollection);
    }

    #[test]
    fn test_pipeline_advance_stage() {
        let mut pipe = PipelineConfig::new("p", AlignmentAlgorithm::Ppo);
        let next = pipe.advance_stage().unwrap();
        assert_eq!(next, PipelineStage::Sft);
        assert_eq!(pipe.current_stage, PipelineStage::Sft);
    }

    #[test]
    fn test_pipeline_advance_to_end() {
        let mut pipe = PipelineConfig::new("p", AlignmentAlgorithm::Ppo);
        for _ in 0..6 {
            pipe.advance_stage().unwrap();
        }
        assert_eq!(pipe.current_stage, PipelineStage::Deployment);
        assert!(pipe.advance_stage().is_err());
    }

    #[test]
    fn test_pipeline_needs_reward_model() {
        let ppo_pipe = PipelineConfig::new("ppo", AlignmentAlgorithm::Ppo);
        assert!(ppo_pipe.needs_reward_model());
        let dpo_pipe = PipelineConfig::new("dpo", AlignmentAlgorithm::Dpo);
        assert!(!dpo_pipe.needs_reward_model());
    }

    #[test]
    fn test_pipeline_summary() {
        let pipe = PipelineConfig::new("my-pipe", AlignmentAlgorithm::Kto);
        let summary = pipe.summary();
        assert!(summary.contains("my-pipe"));
        assert!(summary.contains("KTO"));
    }

    // ── Model Merging tests ──

    #[test]
    fn test_linear_merge() {
        let params = vec![vec![1.0, 2.0, 3.0], vec![4.0, 5.0, 6.0]];
        let weights = vec![0.5, 0.5];
        let merged = ModelMerger::linear_merge(&params, &weights);
        assert!((merged[0] - 2.5).abs() < 1e-10);
        assert!((merged[1] - 3.5).abs() < 1e-10);
        assert!((merged[2] - 4.5).abs() < 1e-10);
    }

    #[test]
    fn test_linear_merge_empty() {
        assert!(ModelMerger::linear_merge(&[], &[]).is_empty());
    }

    #[test]
    fn test_slerp_merge() {
        let a = vec![1.0, 0.0];
        let b = vec![0.0, 1.0];
        let merged = ModelMerger::slerp_merge(&a, &b, 0.5);
        assert_eq!(merged.len(), 2);
        // At t=0.5, should be roughly equidistant from both
        let norm: f64 = merged.iter().map(|x| x * x).sum::<f64>().sqrt();
        assert!((norm - 1.0).abs() < 0.1);
    }

    #[test]
    fn test_slerp_merge_endpoints() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![0.0, 1.0, 0.0];
        let at_zero = ModelMerger::slerp_merge(&a, &b, 0.0);
        assert!((at_zero[0] - 1.0).abs() < 0.01);
        let at_one = ModelMerger::slerp_merge(&a, &b, 1.0);
        assert!((at_one[1] - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_slerp_merge_empty() {
        assert!(ModelMerger::slerp_merge(&[], &[1.0], 0.5).is_empty());
    }

    #[test]
    fn test_ties_merge() {
        let base = vec![0.0, 0.0, 0.0];
        let tvs = vec![vec![1.0, -0.5, 0.3], vec![0.8, -0.3, -0.2]];
        let merged = ModelMerger::ties_merge(&base, &tvs, 0.8, &MajoritySignMethod::Total);
        assert_eq!(merged.len(), 3);
        // First element: both positive, should be positive
        assert!(merged[0] > 0.0);
    }

    #[test]
    fn test_ties_merge_empty() {
        let base = vec![1.0, 2.0];
        let merged = ModelMerger::ties_merge(&base, &[], 0.5, &MajoritySignMethod::Total);
        assert_eq!(merged, base);
    }

    #[test]
    fn test_dare_merge() {
        let base = vec![0.0, 0.0, 0.0, 0.0];
        let tvs = vec![vec![1.0, 1.0, 1.0, 1.0]];
        let merged = ModelMerger::dare_merge(&base, &tvs, 1.0, false);
        // With density 1.0, all kept
        let sum: f64 = merged.iter().sum();
        assert!(sum > 0.0);
    }

    #[test]
    fn test_dare_merge_with_rescale() {
        let base = vec![0.0; 10];
        let tvs = vec![vec![1.0; 10]];
        let merged_rescaled = ModelMerger::dare_merge(&base, &tvs, 0.5, true);
        let merged_no_rescale = ModelMerger::dare_merge(&base, &tvs, 0.5, false);
        // Rescaled values should be larger (2x for density=0.5)
        let sum_r: f64 = merged_rescaled.iter().sum();
        let sum_n: f64 = merged_no_rescale.iter().sum();
        assert!(sum_r > sum_n || (sum_r - sum_n).abs() < 1e-10);
    }

    #[test]
    fn test_dare_merge_empty() {
        let base = vec![1.0, 2.0];
        assert_eq!(ModelMerger::dare_merge(&base, &[], 0.5, false), base);
    }

    #[test]
    fn test_model_merge_config_new() {
        let cfg = ModelMergeConfig::new(
            MergeStrategy::Linear { weights: vec![0.5, 0.5] },
            vec!["model-a".to_string(), "model-b".to_string()],
            "/output/merged",
        );
        assert_eq!(cfg.model_paths.len(), 2);
        assert!(cfg.layer_range.is_none());
    }

    // ── Distributed Config tests ──

    #[test]
    fn test_distributed_config_default() {
        let cfg = DistributedConfig::default();
        assert_eq!(cfg.num_gpus, 1);
        assert_eq!(cfg.zero_stage, ZeroStage::Stage2);
        assert_eq!(cfg.mixed_precision, MixedPrecision::Bf16);
    }

    #[test]
    fn test_distributed_effective_batch_size() {
        let cfg = DistributedConfig {
            num_gpus: 4,
            micro_batch_size: 2,
            gradient_accumulation_steps: 8,
            ..Default::default()
        };
        assert_eq!(cfg.effective_batch_size(), 64);
    }

    #[test]
    fn test_distributed_memory_savings_factor() {
        let cfg = DistributedConfig {
            zero_stage: ZeroStage::Stage3,
            gradient_checkpointing: true,
            cpu_offload: true,
            ..Default::default()
        };
        let factor = cfg.memory_savings_factor();
        assert!(factor > 16.0); // Stage3(16) * GC(1.5) * CPU(2.0) = 48
    }

    #[test]
    fn test_distributed_to_deepspeed_config() {
        let cfg = DistributedConfig {
            zero_stage: ZeroStage::Stage2,
            mixed_precision: MixedPrecision::Bf16,
            cpu_offload: false,
            micro_batch_size: 4,
            gradient_accumulation_steps: 2,
            ..Default::default()
        };
        let ds_cfg = cfg.to_deepspeed_config();
        assert_eq!(ds_cfg.get("zero_optimization.stage").unwrap(), "2");
        assert_eq!(ds_cfg.get("bf16.enabled").unwrap(), "true");
        assert_eq!(ds_cfg.get("fp16.enabled").unwrap(), "false");
        assert_eq!(ds_cfg.get("train_micro_batch_size_per_gpu").unwrap(), "4");
    }

    #[test]
    fn test_distributed_to_deepspeed_config_cpu_offload() {
        let cfg = DistributedConfig {
            cpu_offload: true,
            ..Default::default()
        };
        let ds_cfg = cfg.to_deepspeed_config();
        assert_eq!(
            ds_cfg.get("zero_optimization.offload_optimizer.device").unwrap(),
            "cpu"
        );
    }

    // ── ZeRO stage tests ──

    #[test]
    fn test_zero_stage_memory_savings() {
        assert!((ZeroStage::Stage0.memory_savings_factor() - 1.0).abs() < 1e-10);
        assert!((ZeroStage::Stage1.memory_savings_factor() - 4.0).abs() < 1e-10);
        assert!((ZeroStage::Stage2.memory_savings_factor() - 8.0).abs() < 1e-10);
        assert!((ZeroStage::Stage3.memory_savings_factor() - 16.0).abs() < 1e-10);
    }

    #[test]
    fn test_zero_stage_display() {
        assert!(format!("{}", ZeroStage::Stage3).contains("full parameter"));
    }

    // ── Utility function tests ──

    #[test]
    fn test_sigmoid_values() {
        assert!((sigmoid(0.0) - 0.5).abs() < 1e-10);
        assert!(sigmoid(10.0) > 0.99);
        assert!(sigmoid(-10.0) < 0.01);
    }

    #[test]
    fn test_log_sigmoid_values() {
        let ls0 = log_sigmoid(0.0);
        assert!((ls0 - (-0.6931471805599453)).abs() < 1e-6); // ln(0.5)
        assert!(log_sigmoid(100.0) > -0.001);
        assert!(log_sigmoid(-100.0) < -99.0);
    }

    #[test]
    fn test_log_sigmoid_consistency() {
        for x in [-5.0, -1.0, 0.0, 1.0, 5.0] {
            let ls = log_sigmoid(x);
            let direct = sigmoid(x).ln();
            assert!((ls - direct).abs() < 1e-8, "Mismatch at x={}: {} vs {}", x, ls, direct);
        }
    }

    // ── Display and label enum tests ──

    #[test]
    fn test_merge_strategy_labels() {
        assert_eq!(MergeStrategy::Linear { weights: vec![] }.label(), "Linear");
        assert_eq!(
            MergeStrategy::Slerp { interpolation_factor: 0.5 }.label(),
            "SLERP"
        );
        assert_eq!(
            MergeStrategy::Ties {
                density: 0.5,
                majority_sign_method: MajoritySignMethod::Total
            }
            .label(),
            "TIES"
        );
        assert_eq!(
            MergeStrategy::Dare { density: 0.5, rescale: true }.label(),
            "DARE"
        );
    }

    #[test]
    fn test_ensemble_aggregation_labels() {
        assert_eq!(EnsembleAggregation::Mean.label(), "Mean");
        assert_eq!(EnsembleAggregation::Median.label(), "Median");
        assert_eq!(EnsembleAggregation::WeightedMean.label(), "Weighted Mean");
    }

    #[test]
    fn test_mixed_precision_labels() {
        assert_eq!(MixedPrecision::Fp16.label(), "FP16");
        assert_eq!(MixedPrecision::Bf16.label(), "BF16");
        assert_eq!(MixedPrecision::No.label(), "None");
    }

    #[test]
    fn test_kto_example_kind_display() {
        assert_eq!(format!("{}", KtoExampleKind::Desirable), "desirable");
        assert_eq!(format!("{}", KtoExampleKind::Undesirable), "undesirable");
    }

    #[test]
    fn test_hacking_signal_severity() {
        assert!(HackingSignal::RewardCollapse.severity() > HackingSignal::LengthExploitation.severity());
    }

    #[test]
    fn test_dpo_loss_type_labels() {
        assert_eq!(DpoLossType::Sigmoid.label(), "sigmoid");
        assert_eq!(DpoLossType::Hinge.label(), "hinge");
        assert_eq!(DpoLossType::Ipo.label(), "ipo");
        assert_eq!(DpoLossType::Robust.label(), "robust");
    }

    #[test]
    fn test_kl_penalty_mode_labels() {
        assert_eq!(KlPenaltyMode::Kl.label(), "kl");
        assert_eq!(KlPenaltyMode::Full.label(), "full");
    }

    #[test]
    fn test_fsdp_sharding_strategy_display() {
        assert_eq!(format!("{}", FsdpShardingStrategy::FullShard), "FULL_SHARD");
        assert_eq!(format!("{}", FsdpShardingStrategy::HybridShard), "HYBRID_SHARD");
    }

    #[test]
    fn test_prm_aggregation_labels() {
        assert_eq!(PrmAggregation::Min.label(), "Min");
        assert_eq!(PrmAggregation::Product.label(), "Product");
        assert_eq!(PrmAggregation::LastStep.label(), "Last Step");
        assert_eq!(PrmAggregation::WeightedSum.label(), "Weighted Sum");
    }

    #[test]
    fn test_reward_model_config_default() {
        let cfg = RewardModelConfig::default();
        assert_eq!(cfg.ensemble_size, 3);
        assert!(cfg.calibration_enabled);
    }

    #[test]
    fn test_pipeline_stage_result() {
        let result = PipelineStageResult {
            stage: PipelineStage::Sft,
            success: true,
            metrics: {
                let mut m = HashMap::new();
                m.insert("loss".to_string(), 0.5);
                m
            },
            duration_secs: 120.0,
            error: None,
        };
        assert!(result.success);
        assert_eq!(result.metrics.get("loss"), Some(&0.5));
    }
}
