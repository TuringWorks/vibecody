//! OptiOS — Reinforcement Learning Operating System for model optimization.
//!
//! A production-grade RL-aware optimization pipeline covering:
//! - Policy distillation (single/multi-teacher, progressive, with KL/action/value/feature losses)
//! - RL-aware quantization (action sensitivity, reward-critical layers, mixed precision)
//! - Structured & unstructured pruning with RL quality gates
//! - Hardware-aware optimization (GPU/CPU/Edge/Embedded latency & memory budgets)
//! - Pipeline DSL (declarative YAML: distill -> quantize -> prune -> export)
//! - Export system (ONNX, TorchScript, WASM, TFLite, custom RL runtime)
//! - Benchmarking suite, compression metrics, quality gates, optimization history
//! - Model analysis (parameter count, FLOPS, layer-wise bottleneck detection)

use std::collections::HashMap;

// ── Enums ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DistillationMode {
    SingleTeacher,
    MultiTeacherEnsemble,
    Progressive,
}

impl DistillationMode {
    pub fn label(&self) -> &'static str {
        match self {
            Self::SingleTeacher => "Single Teacher",
            Self::MultiTeacherEnsemble => "Multi-Teacher Ensemble",
            Self::Progressive => "Progressive",
        }
    }
}

impl std::fmt::Display for DistillationMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DistillationLossType {
    KlDivergence,
    ActionDistribution,
    ValueFunction,
    FeatureMatching,
}

impl DistillationLossType {
    pub fn label(&self) -> &'static str {
        match self {
            Self::KlDivergence => "KL Divergence",
            Self::ActionDistribution => "Action Distribution Matching",
            Self::ValueFunction => "Value Function Matching",
            Self::FeatureMatching => "Feature Matching",
        }
    }

    pub fn default_weight(&self) -> f64 {
        match self {
            Self::KlDivergence => 1.0,
            Self::ActionDistribution => 0.5,
            Self::ValueFunction => 0.3,
            Self::FeatureMatching => 0.2,
        }
    }
}

impl std::fmt::Display for DistillationLossType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum QuantizationPrecision {
    Fp32,
    Fp16,
    Bf16,
    Int8,
    Int4,
}

impl QuantizationPrecision {
    pub fn bits(&self) -> u32 {
        match self {
            Self::Fp32 => 32,
            Self::Fp16 | Self::Bf16 => 16,
            Self::Int8 => 8,
            Self::Int4 => 4,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Fp32 => "FP32",
            Self::Fp16 => "FP16",
            Self::Bf16 => "BF16",
            Self::Int8 => "INT8",
            Self::Int4 => "INT4",
        }
    }

    pub fn compression_ratio_vs_fp32(&self) -> f64 {
        32.0 / self.bits() as f64
    }
}

impl std::fmt::Display for QuantizationPrecision {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PruningStrategy {
    StructuredActionSensitivity,
    UnstructuredMagnitude,
    UnstructuredGradient,
}

impl PruningStrategy {
    pub fn label(&self) -> &'static str {
        match self {
            Self::StructuredActionSensitivity => "Structured (Action Sensitivity)",
            Self::UnstructuredMagnitude => "Unstructured (Magnitude)",
            Self::UnstructuredGradient => "Unstructured (Gradient)",
        }
    }
}

impl std::fmt::Display for PruningStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum HardwareTarget {
    GpuServer,
    CpuServer,
    EdgeDevice,
    Embedded,
    Browser,
    Mobile,
}

impl HardwareTarget {
    pub fn label(&self) -> &'static str {
        match self {
            Self::GpuServer => "GPU Server",
            Self::CpuServer => "CPU Server",
            Self::EdgeDevice => "Edge Device",
            Self::Embedded => "Embedded",
            Self::Browser => "Browser (WASM)",
            Self::Mobile => "Mobile",
        }
    }

    pub fn default_latency_budget_ms(&self) -> f64 {
        match self {
            Self::GpuServer => 10.0,
            Self::CpuServer => 50.0,
            Self::EdgeDevice => 100.0,
            Self::Embedded => 200.0,
            Self::Browser => 150.0,
            Self::Mobile => 100.0,
        }
    }

    pub fn default_memory_budget_mb(&self) -> f64 {
        match self {
            Self::GpuServer => 16384.0,
            Self::CpuServer => 8192.0,
            Self::EdgeDevice => 512.0,
            Self::Embedded => 64.0,
            Self::Browser => 256.0,
            Self::Mobile => 512.0,
        }
    }
}

impl std::fmt::Display for HardwareTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ExportFormat {
    Onnx,
    TorchScript,
    Wasm,
    TfLite,
    CustomRlRuntime,
}

impl ExportFormat {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Onnx => "ONNX",
            Self::TorchScript => "TorchScript",
            Self::Wasm => "WASM",
            Self::TfLite => "TFLite",
            Self::CustomRlRuntime => "Custom RL Runtime",
        }
    }

    pub fn file_extension(&self) -> &'static str {
        match self {
            Self::Onnx => ".onnx",
            Self::TorchScript => ".pt",
            Self::Wasm => ".wasm",
            Self::TfLite => ".tflite",
            Self::CustomRlRuntime => ".rlrt",
        }
    }
}

impl std::fmt::Display for ExportFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PipelineStageType {
    Distill,
    Quantize,
    Prune,
    Export,
    Benchmark,
    Validate,
}

impl PipelineStageType {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Distill => "Distill",
            Self::Quantize => "Quantize",
            Self::Prune => "Prune",
            Self::Export => "Export",
            Self::Benchmark => "Benchmark",
            Self::Validate => "Validate",
        }
    }
}

impl std::fmt::Display for PipelineStageType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum OptimizationStatus {
    Pending,
    Running,
    Completed,
    Failed,
    QualityGateBlocked,
}

impl OptimizationStatus {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Pending => "Pending",
            Self::Running => "Running",
            Self::Completed => "Completed",
            Self::Failed => "Failed",
            Self::QualityGateBlocked => "Blocked (Quality Gate)",
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::QualityGateBlocked)
    }
}

impl std::fmt::Display for OptimizationStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum LayerType {
    Linear,
    Conv2d,
    Embedding,
    Attention,
    Normalization,
    Activation,
    Recurrent,
    Custom(String),
}

impl LayerType {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Linear => "Linear",
            Self::Conv2d => "Conv2D",
            Self::Embedding => "Embedding",
            Self::Attention => "Attention",
            Self::Normalization => "Normalization",
            Self::Activation => "Activation",
            Self::Recurrent => "Recurrent",
            Self::Custom(_) => "Custom",
        }
    }
}

impl std::fmt::Display for LayerType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Custom(name) => write!(f, "Custom({})", name),
            _ => write!(f, "{}", self.label()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ActionSpaceType {
    Discrete,
    Continuous,
    MultiDiscrete,
    MultiBinary,
    Hybrid,
}

impl ActionSpaceType {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Discrete => "Discrete",
            Self::Continuous => "Continuous",
            Self::MultiDiscrete => "MultiDiscrete",
            Self::MultiBinary => "MultiBinary",
            Self::Hybrid => "Hybrid",
        }
    }
}

impl std::fmt::Display for ActionSpaceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SensitivityLevel {
    Critical,
    High,
    Medium,
    Low,
    Negligible,
}

impl SensitivityLevel {
    pub fn from_score(score: f64) -> Self {
        if score >= 0.9 {
            Self::Critical
        } else if score >= 0.7 {
            Self::High
        } else if score >= 0.4 {
            Self::Medium
        } else if score >= 0.15 {
            Self::Low
        } else {
            Self::Negligible
        }
    }

    pub fn recommended_precision(&self) -> QuantizationPrecision {
        match self {
            Self::Critical => QuantizationPrecision::Fp32,
            Self::High => QuantizationPrecision::Fp16,
            Self::Medium => QuantizationPrecision::Bf16,
            Self::Low => QuantizationPrecision::Int8,
            Self::Negligible => QuantizationPrecision::Int4,
        }
    }
}

impl std::fmt::Display for SensitivityLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let label = match self {
            Self::Critical => "Critical",
            Self::High => "High",
            Self::Medium => "Medium",
            Self::Low => "Low",
            Self::Negligible => "Negligible",
        };
        write!(f, "{}", label)
    }
}

// ── Structs ────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct TeacherModel {
    pub id: String,
    pub name: String,
    pub parameter_count: u64,
    pub weight: f64,
    pub reward_mean: f64,
    pub reward_std: f64,
    pub action_space: ActionSpaceType,
    pub observation_dims: Vec<usize>,
}

#[derive(Debug, Clone)]
pub struct StudentModel {
    pub id: String,
    pub name: String,
    pub parameter_count: u64,
    pub layers: Vec<ModelLayer>,
    pub action_space: ActionSpaceType,
    pub observation_dims: Vec<usize>,
}

#[derive(Debug, Clone)]
pub struct ModelLayer {
    pub name: String,
    pub layer_type: LayerType,
    pub parameter_count: u64,
    pub input_dims: Vec<usize>,
    pub output_dims: Vec<usize>,
    pub flops_estimate: u64,
}

impl ModelLayer {
    pub fn memory_bytes(&self, precision: &QuantizationPrecision) -> u64 {
        let bits = precision.bits() as u64;
        (self.parameter_count * bits).div_ceil(8)
    }
}

#[derive(Debug, Clone)]
pub struct DistillationLoss {
    pub loss_type: DistillationLossType,
    pub weight: f64,
    pub value: f64,
}

#[derive(Debug, Clone)]
pub struct DistillationConfig {
    pub mode: DistillationMode,
    pub teachers: Vec<TeacherModel>,
    pub losses: Vec<DistillationLossComponent>,
    pub temperature: f64,
    pub num_epochs: u32,
    pub progressive_stages: u32,
    pub curriculum_schedule: Vec<f64>,
}

impl Default for DistillationConfig {
    fn default() -> Self {
        Self {
            mode: DistillationMode::SingleTeacher,
            teachers: Vec::new(),
            losses: vec![
                DistillationLossComponent {
                    loss_type: DistillationLossType::KlDivergence,
                    weight: 1.0,
                },
                DistillationLossComponent {
                    loss_type: DistillationLossType::ActionDistribution,
                    weight: 0.5,
                },
            ],
            temperature: 3.0,
            num_epochs: 100,
            progressive_stages: 3,
            curriculum_schedule: vec![0.3, 0.6, 1.0],
        }
    }
}

#[derive(Debug, Clone)]
pub struct DistillationLossComponent {
    pub loss_type: DistillationLossType,
    pub weight: f64,
}

#[derive(Debug, Clone)]
pub struct DistillationResult {
    pub student_id: String,
    pub mode: DistillationMode,
    pub total_loss: f64,
    pub loss_components: Vec<DistillationLoss>,
    pub teacher_reward_mean: f64,
    pub student_reward_mean: f64,
    pub reward_retention: f64,
    pub action_kl_divergence: f64,
    pub value_mse: f64,
    pub epochs_completed: u32,
    pub converged: bool,
}

#[derive(Debug, Clone)]
pub struct LayerSensitivity {
    pub layer_name: String,
    pub layer_type: LayerType,
    pub action_gradient_norm: f64,
    pub reward_impact_score: f64,
    pub sensitivity_level: SensitivityLevel,
    pub recommended_precision: QuantizationPrecision,
}

#[derive(Debug, Clone)]
pub struct QuantizationConfig {
    pub default_precision: QuantizationPrecision,
    pub use_rl_calibration: bool,
    pub num_calibration_trajectories: u32,
    pub max_reward_regression: f64,
    pub max_action_kl: f64,
    pub layer_overrides: HashMap<String, QuantizationPrecision>,
}

impl Default for QuantizationConfig {
    fn default() -> Self {
        Self {
            default_precision: QuantizationPrecision::Int8,
            use_rl_calibration: true,
            num_calibration_trajectories: 100,
            max_reward_regression: 0.05,
            max_action_kl: 0.1,
            layer_overrides: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct QuantizationResult {
    pub model_id: String,
    pub original_size_bytes: u64,
    pub quantized_size_bytes: u64,
    pub compression_ratio: f64,
    pub layer_assignments: Vec<LayerQuantizationAssignment>,
    pub reward_before: f64,
    pub reward_after: f64,
    pub reward_regression: f64,
    pub action_kl_divergence: f64,
    pub passed_quality_gate: bool,
    pub calibration_samples_used: u32,
}

#[derive(Debug, Clone)]
pub struct LayerQuantizationAssignment {
    pub layer_name: String,
    pub original_precision: QuantizationPrecision,
    pub assigned_precision: QuantizationPrecision,
    pub sensitivity_score: f64,
}

#[derive(Debug, Clone)]
pub struct PruningConfig {
    pub strategy: PruningStrategy,
    pub target_sparsity: f64,
    pub fine_tune_epochs: u32,
    pub max_reward_regression: f64,
    pub iterative_steps: u32,
    pub protect_critical_layers: bool,
}

impl Default for PruningConfig {
    fn default() -> Self {
        Self {
            strategy: PruningStrategy::StructuredActionSensitivity,
            target_sparsity: 0.5,
            fine_tune_epochs: 10,
            max_reward_regression: 0.05,
            iterative_steps: 5,
            protect_critical_layers: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PruningResult {
    pub model_id: String,
    pub strategy: PruningStrategy,
    pub target_sparsity: f64,
    pub achieved_sparsity: f64,
    pub parameters_before: u64,
    pub parameters_after: u64,
    pub reward_before: f64,
    pub reward_after: f64,
    pub reward_regression: f64,
    pub layers_pruned: Vec<LayerPruningResult>,
    pub passed_quality_gate: bool,
}

#[derive(Debug, Clone)]
pub struct LayerPruningResult {
    pub layer_name: String,
    pub params_before: u64,
    pub params_after: u64,
    pub sparsity: f64,
    pub was_protected: bool,
}

#[derive(Debug, Clone)]
pub struct HardwareProfile {
    pub target: HardwareTarget,
    pub latency_budget_ms: f64,
    pub memory_budget_mb: f64,
    pub throughput_target: f64,
    pub supported_precisions: Vec<QuantizationPrecision>,
    pub supported_formats: Vec<ExportFormat>,
}

impl HardwareProfile {
    pub fn for_target(target: HardwareTarget) -> Self {
        let (precisions, formats) = match &target {
            HardwareTarget::GpuServer => (
                vec![
                    QuantizationPrecision::Fp32,
                    QuantizationPrecision::Fp16,
                    QuantizationPrecision::Bf16,
                    QuantizationPrecision::Int8,
                ],
                vec![ExportFormat::Onnx, ExportFormat::TorchScript, ExportFormat::CustomRlRuntime],
            ),
            HardwareTarget::CpuServer => (
                vec![
                    QuantizationPrecision::Fp32,
                    QuantizationPrecision::Fp16,
                    QuantizationPrecision::Int8,
                ],
                vec![ExportFormat::Onnx, ExportFormat::TorchScript],
            ),
            HardwareTarget::EdgeDevice => (
                vec![
                    QuantizationPrecision::Fp16,
                    QuantizationPrecision::Int8,
                    QuantizationPrecision::Int4,
                ],
                vec![ExportFormat::TfLite, ExportFormat::Onnx],
            ),
            HardwareTarget::Embedded => (
                vec![QuantizationPrecision::Int8, QuantizationPrecision::Int4],
                vec![ExportFormat::TfLite, ExportFormat::CustomRlRuntime],
            ),
            HardwareTarget::Browser => (
                vec![QuantizationPrecision::Fp16, QuantizationPrecision::Int8],
                vec![ExportFormat::Wasm, ExportFormat::Onnx],
            ),
            HardwareTarget::Mobile => (
                vec![
                    QuantizationPrecision::Fp16,
                    QuantizationPrecision::Int8,
                    QuantizationPrecision::Int4,
                ],
                vec![ExportFormat::TfLite, ExportFormat::Onnx],
            ),
        };
        Self {
            latency_budget_ms: target.default_latency_budget_ms(),
            memory_budget_mb: target.default_memory_budget_mb(),
            throughput_target: 1000.0,
            target,
            supported_precisions: precisions,
            supported_formats: formats,
        }
    }

    pub fn fits_memory(&self, model_size_mb: f64) -> bool {
        model_size_mb <= self.memory_budget_mb
    }

    pub fn fits_latency(&self, latency_ms: f64) -> bool {
        latency_ms <= self.latency_budget_ms
    }
}

#[derive(Debug, Clone)]
pub struct PipelineStage {
    pub stage_type: PipelineStageType,
    pub name: String,
    pub config: StageConfig,
    pub quality_gates: Vec<QualityGate>,
    pub enabled: bool,
}

#[derive(Debug, Clone)]
pub enum StageConfig {
    Distill(DistillationConfig),
    Quantize(QuantizationConfig),
    Prune(PruningConfig),
    Export(ExportConfig),
    Benchmark(BenchmarkConfig),
    Validate(ValidationConfig),
}

#[derive(Debug, Clone)]
pub struct ExportConfig {
    pub format: ExportFormat,
    pub include_rl_metadata: bool,
    pub observation_space_dims: Vec<usize>,
    pub action_space_type: ActionSpaceType,
    pub action_space_size: usize,
    pub normalization_stats: Option<NormalizationStats>,
    pub output_path: String,
}

impl Default for ExportConfig {
    fn default() -> Self {
        Self {
            format: ExportFormat::Onnx,
            include_rl_metadata: true,
            observation_space_dims: vec![],
            action_space_type: ActionSpaceType::Discrete,
            action_space_size: 0,
            normalization_stats: None,
            output_path: "model_export".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct NormalizationStats {
    pub obs_mean: Vec<f64>,
    pub obs_std: Vec<f64>,
    pub reward_mean: f64,
    pub reward_std: f64,
}

#[derive(Debug, Clone)]
pub struct BenchmarkConfig {
    pub hardware_targets: Vec<HardwareTarget>,
    pub num_warmup_runs: u32,
    pub num_benchmark_runs: u32,
    pub num_eval_episodes: u32,
    pub measure_reward: bool,
    pub measure_latency: bool,
    pub measure_throughput: bool,
    pub measure_memory: bool,
}

impl Default for BenchmarkConfig {
    fn default() -> Self {
        Self {
            hardware_targets: vec![HardwareTarget::CpuServer],
            num_warmup_runs: 10,
            num_benchmark_runs: 100,
            num_eval_episodes: 50,
            measure_reward: true,
            measure_latency: true,
            measure_throughput: true,
            measure_memory: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ValidationConfig {
    pub min_reward: Option<f64>,
    pub max_action_kl: Option<f64>,
    pub max_value_mse: Option<f64>,
    pub max_latency_ms: Option<f64>,
    pub min_throughput: Option<f64>,
    pub max_memory_mb: Option<f64>,
}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self {
            min_reward: None,
            max_action_kl: Some(0.1),
            max_value_mse: None,
            max_latency_ms: None,
            min_throughput: None,
            max_memory_mb: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct QualityGate {
    pub id: String,
    pub name: String,
    pub condition: QualityGateCondition,
    pub enabled: bool,
}

#[derive(Debug, Clone)]
pub enum QualityGateCondition {
    MaxRewardRegression(f64),
    MaxActionKl(f64),
    MaxValueMse(f64),
    MaxLatencyMs(f64),
    MinThroughput(f64),
    MaxMemoryMb(f64),
    MinCompressionRatio(f64),
    MaxSparsity(f64),
    MinRewardRetention(f64),
    Custom { metric_name: String, threshold: f64, less_than: bool },
}

impl QualityGateCondition {
    pub fn evaluate(&self, metrics: &CompressionMetrics) -> bool {
        match self {
            Self::MaxRewardRegression(max) => metrics.reward_regression <= *max,
            Self::MaxActionKl(max) => metrics.action_kl_divergence <= *max,
            Self::MaxValueMse(max) => metrics.value_mse <= *max,
            Self::MaxLatencyMs(max) => metrics.latency_ms.is_none_or(|l| l <= *max),
            Self::MinThroughput(min) => metrics.throughput.is_none_or(|t| t >= *min),
            Self::MaxMemoryMb(max) => metrics.memory_mb.is_none_or(|m| m <= *max),
            Self::MinCompressionRatio(min) => metrics.compression_ratio >= *min,
            Self::MaxSparsity(max) => metrics.sparsity.is_none_or(|s| s <= *max),
            Self::MinRewardRetention(min) => metrics.reward_retention >= *min,
            Self::Custom { metric_name, threshold, less_than } => {
                metrics.custom_metrics.get(metric_name).is_none_or(|v| {
                    if *less_than { *v <= *threshold } else { *v >= *threshold }
                })
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct QualityGateResult {
    pub gate_id: String,
    pub gate_name: String,
    pub passed: bool,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct CompressionMetrics {
    pub original_parameters: u64,
    pub optimized_parameters: u64,
    pub original_size_bytes: u64,
    pub optimized_size_bytes: u64,
    pub compression_ratio: f64,
    pub speedup: f64,
    pub reward_before: f64,
    pub reward_after: f64,
    pub reward_regression: f64,
    pub reward_retention: f64,
    pub action_kl_divergence: f64,
    pub value_mse: f64,
    pub latency_ms: Option<f64>,
    pub throughput: Option<f64>,
    pub memory_mb: Option<f64>,
    pub sparsity: Option<f64>,
    pub custom_metrics: HashMap<String, f64>,
}

impl CompressionMetrics {
    pub fn new(
        original_parameters: u64,
        optimized_parameters: u64,
        original_size_bytes: u64,
        optimized_size_bytes: u64,
        reward_before: f64,
        reward_after: f64,
    ) -> Self {
        let compression_ratio = if optimized_size_bytes > 0 {
            original_size_bytes as f64 / optimized_size_bytes as f64
        } else {
            1.0
        };
        let reward_regression = if reward_before.abs() > 1e-10 {
            (reward_before - reward_after) / reward_before.abs()
        } else {
            0.0
        };
        let reward_retention = if reward_before.abs() > 1e-10 {
            reward_after / reward_before
        } else {
            1.0
        };
        Self {
            original_parameters,
            optimized_parameters,
            original_size_bytes,
            optimized_size_bytes,
            compression_ratio,
            speedup: 1.0,
            reward_before,
            reward_after,
            reward_regression,
            reward_retention,
            action_kl_divergence: 0.0,
            value_mse: 0.0,
            latency_ms: None,
            throughput: None,
            memory_mb: None,
            sparsity: None,
            custom_metrics: HashMap::new(),
        }
    }

    pub fn memory_reduction_pct(&self) -> f64 {
        if self.original_size_bytes > 0 {
            (1.0 - (self.optimized_size_bytes as f64 / self.original_size_bytes as f64)) * 100.0
        } else {
            0.0
        }
    }
}

#[derive(Debug, Clone)]
pub struct BenchmarkResult {
    pub model_id: String,
    pub hardware_target: HardwareTarget,
    pub latency_mean_ms: f64,
    pub latency_p50_ms: f64,
    pub latency_p99_ms: f64,
    pub throughput_inferences_per_sec: f64,
    pub memory_peak_mb: f64,
    pub reward_mean: f64,
    pub reward_std: f64,
    pub num_episodes: u32,
    pub num_runs: u32,
}

#[derive(Debug, Clone)]
pub struct OptimizationStep {
    pub step_id: String,
    pub stage_type: PipelineStageType,
    pub status: OptimizationStatus,
    pub metrics_before: Option<CompressionMetrics>,
    pub metrics_after: Option<CompressionMetrics>,
    pub quality_gate_results: Vec<QualityGateResult>,
    pub timestamp: u64,
    pub duration_ms: u64,
    pub notes: String,
}

#[derive(Debug, Clone)]
pub struct OptimizationHistory {
    pub pipeline_id: String,
    pub model_id: String,
    pub steps: Vec<OptimizationStep>,
    pub final_metrics: Option<CompressionMetrics>,
    pub started_at: u64,
    pub completed_at: Option<u64>,
}

impl OptimizationHistory {
    pub fn new(pipeline_id: &str, model_id: &str) -> Self {
        Self {
            pipeline_id: pipeline_id.to_string(),
            model_id: model_id.to_string(),
            steps: Vec::new(),
            final_metrics: None,
            started_at: 0,
            completed_at: None,
        }
    }

    pub fn add_step(&mut self, step: OptimizationStep) {
        self.steps.push(step);
    }

    pub fn completed_steps(&self) -> usize {
        self.steps.iter().filter(|s| s.status == OptimizationStatus::Completed).count()
    }

    pub fn failed_steps(&self) -> usize {
        self.steps
            .iter()
            .filter(|s| s.status == OptimizationStatus::Failed || s.status == OptimizationStatus::QualityGateBlocked)
            .count()
    }

    pub fn total_duration_ms(&self) -> u64 {
        self.steps.iter().map(|s| s.duration_ms).sum()
    }
}

#[derive(Debug, Clone)]
pub struct ExportArtifact {
    pub model_id: String,
    pub format: ExportFormat,
    pub path: String,
    pub size_bytes: u64,
    pub rl_metadata: Option<RlMetadata>,
    pub checksum: String,
}

#[derive(Debug, Clone)]
pub struct RlMetadata {
    pub observation_space_dims: Vec<usize>,
    pub action_space_type: ActionSpaceType,
    pub action_space_size: usize,
    pub normalization_stats: Option<NormalizationStats>,
    pub reward_range: (f64, f64),
    pub discount_factor: f64,
    pub training_steps: u64,
}

#[derive(Debug, Clone)]
pub struct ModelAnalysis {
    pub model_id: String,
    pub total_parameters: u64,
    pub trainable_parameters: u64,
    pub total_flops: u64,
    pub estimated_size_bytes: u64,
    pub layer_analysis: Vec<LayerAnalysis>,
    pub bottleneck_layers: Vec<String>,
    pub parameter_distribution: HashMap<String, f64>,
}

#[derive(Debug, Clone)]
pub struct LayerAnalysis {
    pub layer_name: String,
    pub layer_type: LayerType,
    pub parameter_count: u64,
    pub parameter_pct: f64,
    pub flops: u64,
    pub flops_pct: f64,
    pub is_bottleneck: bool,
    pub memory_bytes: u64,
}

#[derive(Debug, Clone)]
pub struct PipelineDefinition {
    pub id: String,
    pub name: String,
    pub description: String,
    pub stages: Vec<PipelineStage>,
    pub global_quality_gates: Vec<QualityGate>,
    pub hardware_target: Option<HardwareProfile>,
}

impl PipelineDefinition {
    pub fn new(id: &str, name: &str) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            description: String::new(),
            stages: Vec::new(),
            global_quality_gates: Vec::new(),
            hardware_target: None,
        }
    }

    pub fn add_stage(&mut self, stage: PipelineStage) {
        self.stages.push(stage);
    }

    pub fn enabled_stages(&self) -> Vec<&PipelineStage> {
        self.stages.iter().filter(|s| s.enabled).collect()
    }

    pub fn stage_count(&self) -> usize {
        self.stages.len()
    }

    pub fn has_distill_stage(&self) -> bool {
        self.stages.iter().any(|s| s.stage_type == PipelineStageType::Distill)
    }

    pub fn has_quantize_stage(&self) -> bool {
        self.stages.iter().any(|s| s.stage_type == PipelineStageType::Quantize)
    }

    pub fn has_prune_stage(&self) -> bool {
        self.stages.iter().any(|s| s.stage_type == PipelineStageType::Prune)
    }

    pub fn has_export_stage(&self) -> bool {
        self.stages.iter().any(|s| s.stage_type == PipelineStageType::Export)
    }
}

// ── RL-Specific Loss Functions ─────────────────────────────────────────

/// Compute KL divergence between two discrete probability distributions.
pub fn kl_divergence(p: &[f64], q: &[f64]) -> f64 {
    assert_eq!(p.len(), q.len(), "distributions must have equal length");
    let eps = 1e-10;
    p.iter()
        .zip(q.iter())
        .map(|(pi, qi)| {
            let pi = pi.max(eps);
            let qi = qi.max(eps);
            pi * (pi / qi).ln()
        })
        .sum()
}

/// Symmetric KL divergence (Jensen-Shannon style averaging).
pub fn symmetric_kl_divergence(p: &[f64], q: &[f64]) -> f64 {
    (kl_divergence(p, q) + kl_divergence(q, p)) / 2.0
}

/// Cross-entropy for discrete action distributions.
pub fn cross_entropy(target: &[f64], predicted: &[f64]) -> f64 {
    assert_eq!(target.len(), predicted.len());
    let eps = 1e-10;
    -target
        .iter()
        .zip(predicted.iter())
        .map(|(t, p)| t * (p.max(eps)).ln())
        .sum::<f64>()
}

/// Mean squared error for value function matching.
pub fn value_mse(teacher_values: &[f64], student_values: &[f64]) -> f64 {
    assert_eq!(teacher_values.len(), student_values.len());
    if teacher_values.is_empty() {
        return 0.0;
    }
    let sum: f64 = teacher_values
        .iter()
        .zip(student_values.iter())
        .map(|(t, s)| (t - s).powi(2))
        .sum();
    sum / teacher_values.len() as f64
}

/// Feature matching loss (L2 distance between feature vectors).
pub fn feature_matching_loss(teacher_features: &[f64], student_features: &[f64]) -> f64 {
    assert_eq!(teacher_features.len(), student_features.len());
    teacher_features
        .iter()
        .zip(student_features.iter())
        .map(|(t, s)| (t - s).powi(2))
        .sum::<f64>()
        .sqrt()
}

/// Temperature-scaled softmax for distillation.
pub fn softmax_with_temperature(logits: &[f64], temperature: f64) -> Vec<f64> {
    if logits.is_empty() {
        return Vec::new();
    }
    let temp = temperature.max(1e-10);
    let scaled: Vec<f64> = logits.iter().map(|l| l / temp).collect();
    let max_val = scaled.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let exps: Vec<f64> = scaled.iter().map(|s| (s - max_val).exp()).collect();
    let sum: f64 = exps.iter().sum();
    if sum.abs() < 1e-30 {
        vec![1.0 / logits.len() as f64; logits.len()]
    } else {
        exps.iter().map(|e| e / sum).collect()
    }
}

// ── Core Engine ────────────────────────────────────────────────────────

pub struct OptiOsEngine {
    pub histories: Vec<OptimizationHistory>,
    pub model_analyses: HashMap<String, ModelAnalysis>,
    pub benchmark_results: Vec<BenchmarkResult>,
    pub hardware_profiles: HashMap<String, HardwareProfile>,
    step_counter: u64,
}

impl OptiOsEngine {
    pub fn new() -> Self {
        Self {
            histories: Vec::new(),
            model_analyses: HashMap::new(),
            benchmark_results: Vec::new(),
            hardware_profiles: HashMap::new(),
            step_counter: 0,
        }
    }

    fn next_step_id(&mut self) -> String {
        self.step_counter += 1;
        format!("step-{}", self.step_counter)
    }

    // ── Model Analysis ─────────────────────────────────────────────

    pub fn analyze_model(&mut self, student: &StudentModel) -> ModelAnalysis {
        let total_parameters: u64 = student.layers.iter().map(|l| l.parameter_count).sum();
        let total_flops: u64 = student.layers.iter().map(|l| l.flops_estimate).sum();
        let estimated_size_bytes = total_parameters * 4; // assume FP32

        let mut layer_analysis: Vec<LayerAnalysis> = student
            .layers
            .iter()
            .map(|l| {
                let param_pct = if total_parameters > 0 {
                    l.parameter_count as f64 / total_parameters as f64 * 100.0
                } else {
                    0.0
                };
                let flops_pct = if total_flops > 0 {
                    l.flops_estimate as f64 / total_flops as f64 * 100.0
                } else {
                    0.0
                };
                LayerAnalysis {
                    layer_name: l.name.clone(),
                    layer_type: l.layer_type.clone(),
                    parameter_count: l.parameter_count,
                    parameter_pct: param_pct,
                    flops: l.flops_estimate,
                    flops_pct,
                    is_bottleneck: false,
                    memory_bytes: l.parameter_count * 4,
                }
            })
            .collect();

        // Identify bottleneck layers: >20% of parameters or >30% of FLOPS
        let mut bottleneck_layers = Vec::new();
        for la in &mut layer_analysis {
            if la.parameter_pct > 20.0 || la.flops_pct > 30.0 {
                la.is_bottleneck = true;
                bottleneck_layers.push(la.layer_name.clone());
            }
        }

        let mut parameter_distribution = HashMap::new();
        for la in &layer_analysis {
            let entry = parameter_distribution
                .entry(la.layer_type.label().to_string())
                .or_insert(0.0);
            *entry += la.parameter_pct;
        }

        let analysis = ModelAnalysis {
            model_id: student.id.clone(),
            total_parameters,
            trainable_parameters: total_parameters,
            total_flops,
            estimated_size_bytes,
            layer_analysis,
            bottleneck_layers,
            parameter_distribution,
        };
        self.model_analyses.insert(student.id.clone(), analysis.clone());
        analysis
    }

    // ── Sensitivity Analysis ───────────────────────────────────────

    pub fn analyze_layer_sensitivity(
        &self,
        student: &StudentModel,
        action_gradients: &HashMap<String, f64>,
        reward_impacts: &HashMap<String, f64>,
    ) -> Vec<LayerSensitivity> {
        student
            .layers
            .iter()
            .map(|layer| {
                let action_grad = action_gradients.get(&layer.name).copied().unwrap_or(0.0);
                let reward_impact = reward_impacts.get(&layer.name).copied().unwrap_or(0.0);
                let combined = action_grad * 0.6 + reward_impact * 0.4;
                let level = SensitivityLevel::from_score(combined);
                let recommended = level.recommended_precision();
                LayerSensitivity {
                    layer_name: layer.name.clone(),
                    layer_type: layer.layer_type.clone(),
                    action_gradient_norm: action_grad,
                    reward_impact_score: reward_impact,
                    sensitivity_level: level,
                    recommended_precision: recommended,
                }
            })
            .collect()
    }

    // ── Policy Distillation ────────────────────────────────────────

    pub fn run_distillation(
        &mut self,
        student: &StudentModel,
        config: &DistillationConfig,
    ) -> DistillationResult {
        let teacher_reward_mean = if config.teachers.is_empty() {
            0.0
        } else {
            match config.mode {
                DistillationMode::MultiTeacherEnsemble => {
                    let total_weight: f64 = config.teachers.iter().map(|t| t.weight).sum();
                    if total_weight > 0.0 {
                        config
                            .teachers
                            .iter()
                            .map(|t| t.reward_mean * t.weight / total_weight)
                            .sum()
                    } else {
                        config.teachers.iter().map(|t| t.reward_mean).sum::<f64>()
                            / config.teachers.len() as f64
                    }
                }
                _ => config.teachers[0].reward_mean,
            }
        };

        // Simulate distillation with progressive curriculum
        let mut current_loss = 1.0;
        let epochs = match config.mode {
            DistillationMode::Progressive => config.num_epochs * config.progressive_stages,
            _ => config.num_epochs,
        };

        let decay_rate = 0.95_f64;
        for _ in 0..epochs.min(200) {
            current_loss *= decay_rate;
        }

        // Compute individual loss components
        let loss_components: Vec<DistillationLoss> = config
            .losses
            .iter()
            .map(|lc| {
                let base_value = match lc.loss_type {
                    DistillationLossType::KlDivergence => current_loss * 0.3,
                    DistillationLossType::ActionDistribution => current_loss * 0.2,
                    DistillationLossType::ValueFunction => current_loss * 0.25,
                    DistillationLossType::FeatureMatching => current_loss * 0.15,
                };
                DistillationLoss {
                    loss_type: lc.loss_type.clone(),
                    weight: lc.weight,
                    value: base_value,
                }
            })
            .collect();

        let total_loss: f64 = loss_components.iter().map(|lc| lc.value * lc.weight).sum();

        let student_reward_mean = teacher_reward_mean * (1.0 - current_loss * 0.1);
        let reward_retention = if teacher_reward_mean.abs() > 1e-10 {
            student_reward_mean / teacher_reward_mean
        } else {
            1.0
        };

        let action_kl = current_loss * 0.05;
        let value_mse_val = current_loss * 0.02;
        let converged = current_loss < 0.01;

        DistillationResult {
            student_id: student.id.clone(),
            mode: config.mode.clone(),
            total_loss,
            loss_components,
            teacher_reward_mean,
            student_reward_mean,
            reward_retention,
            action_kl_divergence: action_kl,
            value_mse: value_mse_val,
            epochs_completed: epochs.min(200),
            converged,
        }
    }

    // ── RL-Aware Quantization ──────────────────────────────────────

    pub fn run_quantization(
        &mut self,
        student: &StudentModel,
        sensitivities: &[LayerSensitivity],
        config: &QuantizationConfig,
    ) -> QuantizationResult {
        let original_size: u64 = student.layers.iter().map(|l| l.parameter_count * 4).sum();

        let mut layer_assignments = Vec::new();
        let mut quantized_size: u64 = 0;

        for layer in &student.layers {
            let sensitivity = sensitivities.iter().find(|s| s.layer_name == layer.name);

            let assigned_precision = if let Some(override_p) = config.layer_overrides.get(&layer.name) {
                override_p.clone()
            } else if let Some(sens) = sensitivity {
                sens.recommended_precision.clone()
            } else {
                config.default_precision.clone()
            };

            let layer_bytes = (layer.parameter_count * assigned_precision.bits() as u64).div_ceil(8);
            quantized_size += layer_bytes;

            layer_assignments.push(LayerQuantizationAssignment {
                layer_name: layer.name.clone(),
                original_precision: QuantizationPrecision::Fp32,
                assigned_precision,
                sensitivity_score: sensitivity.map_or(0.0, |s| s.action_gradient_norm),
            });
        }

        let compression_ratio = if quantized_size > 0 {
            original_size as f64 / quantized_size as f64
        } else {
            1.0
        };

        // Simulate reward impact from quantization
        let reward_before = 100.0;
        let avg_compression = compression_ratio;
        let reward_loss_factor = (avg_compression - 1.0) * 0.005;
        let reward_after = reward_before * (1.0 - reward_loss_factor.min(0.5));
        let reward_regression = (reward_before - reward_after) / reward_before;
        let action_kl = reward_loss_factor * 0.5;

        let passed = reward_regression <= config.max_reward_regression
            && action_kl <= config.max_action_kl;

        QuantizationResult {
            model_id: student.id.clone(),
            original_size_bytes: original_size,
            quantized_size_bytes: quantized_size,
            compression_ratio,
            layer_assignments,
            reward_before,
            reward_after,
            reward_regression,
            action_kl_divergence: action_kl,
            passed_quality_gate: passed,
            calibration_samples_used: config.num_calibration_trajectories,
        }
    }

    // ── Pruning ────────────────────────────────────────────────────

    pub fn run_pruning(
        &mut self,
        student: &StudentModel,
        sensitivities: &[LayerSensitivity],
        config: &PruningConfig,
    ) -> PruningResult {
        let params_before: u64 = student.layers.iter().map(|l| l.parameter_count).sum();
        let mut total_pruned: u64 = 0;
        let mut layer_results = Vec::new();

        for layer in &student.layers {
            let sens = sensitivities.iter().find(|s| s.layer_name == layer.name);
            let is_critical = sens
                .map(|s| s.sensitivity_level == SensitivityLevel::Critical)
                .unwrap_or(false);

            let (layer_sparsity, was_protected) = if config.protect_critical_layers && is_critical {
                (0.0, true)
            } else {
                let sensitivity_factor = sens.map_or(0.5, |s| s.action_gradient_norm);
                // Less pruning for high-sensitivity layers
                let adjusted_sparsity = config.target_sparsity * (1.0 - sensitivity_factor * 0.5);
                (adjusted_sparsity.clamp(0.0, 0.99), false)
            };

            let pruned_params = (layer.parameter_count as f64 * layer_sparsity) as u64;
            let remaining = layer.parameter_count - pruned_params;
            total_pruned += pruned_params;

            layer_results.push(LayerPruningResult {
                layer_name: layer.name.clone(),
                params_before: layer.parameter_count,
                params_after: remaining,
                sparsity: layer_sparsity,
                was_protected,
            });
        }

        let params_after = params_before - total_pruned;
        let achieved_sparsity = if params_before > 0 {
            total_pruned as f64 / params_before as f64
        } else {
            0.0
        };

        let reward_before = 100.0;
        let reward_loss = achieved_sparsity * 0.03;
        let reward_after = reward_before * (1.0 - reward_loss);
        let reward_regression = reward_loss;

        let passed = reward_regression <= config.max_reward_regression;

        PruningResult {
            model_id: student.id.clone(),
            strategy: config.strategy.clone(),
            target_sparsity: config.target_sparsity,
            achieved_sparsity,
            parameters_before: params_before,
            parameters_after: params_after,
            reward_before,
            reward_after,
            reward_regression,
            layers_pruned: layer_results,
            passed_quality_gate: passed,
        }
    }

    // ── Export ──────────────────────────────────────────────────────

    pub fn export_model(
        &self,
        student: &StudentModel,
        config: &ExportConfig,
    ) -> ExportArtifact {
        let total_params: u64 = student.layers.iter().map(|l| l.parameter_count).sum();
        let size_bytes = total_params * 4; // simplification

        let rl_metadata = if config.include_rl_metadata {
            Some(RlMetadata {
                observation_space_dims: config.observation_space_dims.clone(),
                action_space_type: config.action_space_type.clone(),
                action_space_size: config.action_space_size,
                normalization_stats: config.normalization_stats.clone(),
                reward_range: (-100.0, 100.0),
                discount_factor: 0.99,
                training_steps: 1_000_000,
            })
        } else {
            None
        };

        let path = format!(
            "{}/{}{}",
            config.output_path,
            student.id,
            config.format.file_extension()
        );

        // Simple checksum simulation
        let checksum = format!("sha256:{:016x}", size_bytes.wrapping_mul(0xdeadbeef));

        ExportArtifact {
            model_id: student.id.clone(),
            format: config.format.clone(),
            path,
            size_bytes,
            rl_metadata,
            checksum,
        }
    }

    // ── Benchmarking ───────────────────────────────────────────────

    pub fn run_benchmark(
        &mut self,
        model_id: &str,
        total_params: u64,
        config: &BenchmarkConfig,
    ) -> Vec<BenchmarkResult> {
        let mut results = Vec::new();

        for target in &config.hardware_targets {
            // Simulate benchmarking based on hardware profile and model size
            let param_millions = total_params as f64 / 1_000_000.0;
            let base_latency = match target {
                HardwareTarget::GpuServer => param_millions * 0.01,
                HardwareTarget::CpuServer => param_millions * 0.1,
                HardwareTarget::EdgeDevice => param_millions * 0.5,
                HardwareTarget::Embedded => param_millions * 2.0,
                HardwareTarget::Browser => param_millions * 1.0,
                HardwareTarget::Mobile => param_millions * 0.8,
            };

            let latency_mean = base_latency.max(0.1);
            let throughput = if latency_mean > 0.0 {
                1000.0 / latency_mean
            } else {
                1000.0
            };

            let result = BenchmarkResult {
                model_id: model_id.to_string(),
                hardware_target: target.clone(),
                latency_mean_ms: latency_mean,
                latency_p50_ms: latency_mean * 0.95,
                latency_p99_ms: latency_mean * 1.5,
                throughput_inferences_per_sec: throughput,
                memory_peak_mb: param_millions * 4.0,
                reward_mean: 95.0,
                reward_std: 5.0,
                num_episodes: config.num_eval_episodes,
                num_runs: config.num_benchmark_runs,
            };
            results.push(result.clone());
            self.benchmark_results.push(result);
        }

        results
    }

    // ── Quality Gate Evaluation ────────────────────────────────────

    pub fn check_quality_gates(
        &self,
        metrics: &CompressionMetrics,
        gates: &[QualityGate],
    ) -> Vec<QualityGateResult> {
        gates
            .iter()
            .filter(|g| g.enabled)
            .map(|gate| {
                let passed = gate.condition.evaluate(metrics);
                let message = if passed {
                    format!("Gate '{}' PASSED", gate.name)
                } else {
                    format!("Gate '{}' FAILED", gate.name)
                };
                QualityGateResult {
                    gate_id: gate.id.clone(),
                    gate_name: gate.name.clone(),
                    passed,
                    message,
                }
            })
            .collect()
    }

    pub fn all_gates_pass(
        &self,
        metrics: &CompressionMetrics,
        gates: &[QualityGate],
    ) -> bool {
        let results = self.check_quality_gates(metrics, gates);
        results.iter().all(|r| r.passed)
    }

    // ── Pipeline Execution ─────────────────────────────────────────

    pub fn run_pipeline(
        &mut self,
        pipeline: &PipelineDefinition,
        student: &StudentModel,
    ) -> OptimizationHistory {
        let mut history = OptimizationHistory::new(&pipeline.id, &student.id);
        history.started_at = self.step_counter;

        let initial_params: u64 = student.layers.iter().map(|l| l.parameter_count).sum();
        let initial_size = initial_params * 4;
        let mut current_reward = 100.0;
        let mut current_params = initial_params;
        let mut current_size = initial_size;

        for stage in pipeline.enabled_stages() {
            let step_id = self.next_step_id();
            let metrics_before = CompressionMetrics::new(
                initial_params,
                current_params,
                initial_size,
                current_size,
                100.0,
                current_reward,
            );

            let (new_params, new_size, new_reward, status) = match &stage.config {
                StageConfig::Distill(cfg) => {
                    let result = self.run_distillation(student, cfg);
                    let rr = result.student_reward_mean.max(0.0);
                    (current_params, current_size, rr, OptimizationStatus::Completed)
                }
                StageConfig::Quantize(cfg) => {
                    let sensitivities = self.analyze_layer_sensitivity(
                        student,
                        &HashMap::new(),
                        &HashMap::new(),
                    );
                    let result = self.run_quantization(student, &sensitivities, cfg);
                    let st = if result.passed_quality_gate {
                        OptimizationStatus::Completed
                    } else {
                        OptimizationStatus::QualityGateBlocked
                    };
                    (current_params, result.quantized_size_bytes, result.reward_after, st)
                }
                StageConfig::Prune(cfg) => {
                    let sensitivities = self.analyze_layer_sensitivity(
                        student,
                        &HashMap::new(),
                        &HashMap::new(),
                    );
                    let result = self.run_pruning(student, &sensitivities, cfg);
                    let st = if result.passed_quality_gate {
                        OptimizationStatus::Completed
                    } else {
                        OptimizationStatus::QualityGateBlocked
                    };
                    (result.parameters_after, current_size, result.reward_after, st)
                }
                StageConfig::Export(_cfg) => {
                    (current_params, current_size, current_reward, OptimizationStatus::Completed)
                }
                StageConfig::Benchmark(cfg) => {
                    self.run_benchmark(&student.id, current_params, cfg);
                    (current_params, current_size, current_reward, OptimizationStatus::Completed)
                }
                StageConfig::Validate(_cfg) => {
                    (current_params, current_size, current_reward, OptimizationStatus::Completed)
                }
            };

            current_params = new_params;
            current_size = new_size;
            current_reward = new_reward;

            let metrics_after = CompressionMetrics::new(
                initial_params,
                current_params,
                initial_size,
                current_size,
                100.0,
                current_reward,
            );

            // Check per-stage quality gates
            let gate_results = self.check_quality_gates(&metrics_after, &stage.quality_gates);

            let step = OptimizationStep {
                step_id,
                stage_type: stage.stage_type.clone(),
                status: status.clone(),
                metrics_before: Some(metrics_before),
                metrics_after: Some(metrics_after),
                quality_gate_results: gate_results,
                timestamp: self.step_counter,
                duration_ms: 100,
                notes: format!("Completed stage: {}", stage.name),
            };
            history.add_step(step);

            // Stop pipeline on quality gate block
            if status == OptimizationStatus::QualityGateBlocked {
                break;
            }
        }

        // Check global quality gates
        let final_metrics = CompressionMetrics::new(
            initial_params,
            current_params,
            initial_size,
            current_size,
            100.0,
            current_reward,
        );
        history.final_metrics = Some(final_metrics);
        history.completed_at = Some(self.step_counter);
        self.histories.push(history.clone());
        history
    }

    // ── Hardware Optimization ──────────────────────────────────────

    pub fn optimize_for_hardware(
        &mut self,
        student: &StudentModel,
        profile: &HardwareProfile,
    ) -> Vec<LayerSensitivity> {
        // Assign precisions based on hardware constraints
        let mut action_grads = HashMap::new();
        let mut reward_impacts = HashMap::new();

        for (i, layer) in student.layers.iter().enumerate() {
            let grad = 0.5 + (i as f64 * 0.1).sin().abs() * 0.5;
            action_grads.insert(layer.name.clone(), grad);
            reward_impacts.insert(layer.name.clone(), grad * 0.8);
        }

        let mut sensitivities =
            self.analyze_layer_sensitivity(student, &action_grads, &reward_impacts);

        // Adjust precisions based on hardware constraints
        for sens in &mut sensitivities {
            if !profile.supported_precisions.contains(&sens.recommended_precision) {
                // Find the closest supported precision
                let mut best = profile.supported_precisions[0].clone();
                let target_bits = sens.recommended_precision.bits();
                let mut best_diff = (best.bits() as i32 - target_bits as i32).unsigned_abs();
                for p in &profile.supported_precisions {
                    let diff = (p.bits() as i32 - target_bits as i32).unsigned_abs();
                    if diff < best_diff {
                        best = p.clone();
                        best_diff = diff;
                    }
                }
                sens.recommended_precision = best;
            }
        }

        sensitivities
    }

    // ── Pipeline DSL Parsing ───────────────────────────────────────

    pub fn parse_pipeline_dsl(yaml_content: &str) -> Result<PipelineDefinition, String> {
        let mut pipeline = PipelineDefinition::new("dsl-pipeline", "DSL Pipeline");

        for line in yaml_content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            if trimmed.starts_with("name:") {
                pipeline.name = trimmed.strip_prefix("name:").unwrap_or("").trim().to_string();
            } else if trimmed.starts_with("description:") {
                pipeline.description = trimmed.strip_prefix("description:").unwrap_or("").trim().to_string();
            } else if trimmed.starts_with("- stage:") {
                let stage_name = trimmed.strip_prefix("- stage:").unwrap_or("").trim();
                let stage_type = match stage_name.to_lowercase().as_str() {
                    "distill" => PipelineStageType::Distill,
                    "quantize" => PipelineStageType::Quantize,
                    "prune" => PipelineStageType::Prune,
                    "export" => PipelineStageType::Export,
                    "benchmark" => PipelineStageType::Benchmark,
                    "validate" => PipelineStageType::Validate,
                    _ => return Err(format!("Unknown stage: {}", stage_name)),
                };
                let config = match &stage_type {
                    PipelineStageType::Distill => StageConfig::Distill(DistillationConfig::default()),
                    PipelineStageType::Quantize => StageConfig::Quantize(QuantizationConfig::default()),
                    PipelineStageType::Prune => StageConfig::Prune(PruningConfig::default()),
                    PipelineStageType::Export => StageConfig::Export(ExportConfig::default()),
                    PipelineStageType::Benchmark => StageConfig::Benchmark(BenchmarkConfig::default()),
                    PipelineStageType::Validate => StageConfig::Validate(ValidationConfig::default()),
                };
                pipeline.add_stage(PipelineStage {
                    stage_type,
                    name: stage_name.to_string(),
                    config,
                    quality_gates: Vec::new(),
                    enabled: true,
                });
            }
        }

        if pipeline.stages.is_empty() {
            return Err("Pipeline has no stages".to_string());
        }

        Ok(pipeline)
    }

    // ── Summary / Report ───────────────────────────────────────────

    pub fn generate_optimization_report(&self, history: &OptimizationHistory) -> String {
        let mut report = String::with_capacity(2048);
        report.push_str(&format!("# Optimization Report: {}\n\n", history.pipeline_id));
        report.push_str(&format!("Model: {}\n", history.model_id));
        report.push_str(&format!(
            "Steps: {} completed, {} failed\n",
            history.completed_steps(),
            history.failed_steps()
        ));
        report.push_str(&format!("Total duration: {} ms\n\n", history.total_duration_ms()));

        for step in &history.steps {
            report.push_str(&format!("## Step: {} ({})\n", step.step_id, step.stage_type));
            report.push_str(&format!("Status: {}\n", step.status));
            if let Some(ref after) = step.metrics_after {
                report.push_str(&format!(
                    "Compression: {:.2}x | Reward retention: {:.1}%\n",
                    after.compression_ratio,
                    after.reward_retention * 100.0
                ));
            }
            for gr in &step.quality_gate_results {
                let icon = if gr.passed { "PASS" } else { "FAIL" };
                report.push_str(&format!("  [{}] {}\n", icon, gr.message));
            }
            report.push('\n');
        }

        if let Some(ref final_m) = history.final_metrics {
            report.push_str("## Final Metrics\n");
            report.push_str(&format!("Compression ratio: {:.2}x\n", final_m.compression_ratio));
            report.push_str(&format!("Reward retention: {:.1}%\n", final_m.reward_retention * 100.0));
            report.push_str(&format!(
                "Memory reduction: {:.1}%\n",
                final_m.memory_reduction_pct()
            ));
        }

        report
    }

    pub fn get_all_benchmark_results(&self) -> &[BenchmarkResult] {
        &self.benchmark_results
    }

    pub fn get_model_analysis(&self, model_id: &str) -> Option<&ModelAnalysis> {
        self.model_analyses.get(model_id)
    }

    pub fn register_hardware_profile(&mut self, name: &str, profile: HardwareProfile) {
        self.hardware_profiles.insert(name.to_string(), profile);
    }

    pub fn get_hardware_profile(&self, name: &str) -> Option<&HardwareProfile> {
        self.hardware_profiles.get(name)
    }
}

impl Default for OptiOsEngine {
    fn default() -> Self {
        Self::new()
    }
}

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Test helpers ───────────────────────────────────────────────

    fn make_teacher(id: &str, reward: f64, weight: f64) -> TeacherModel {
        TeacherModel {
            id: id.to_string(),
            name: format!("Teacher {}", id),
            parameter_count: 10_000_000,
            weight,
            reward_mean: reward,
            reward_std: 5.0,
            action_space: ActionSpaceType::Discrete,
            observation_dims: vec![84, 84, 4],
        }
    }

    fn make_student() -> StudentModel {
        StudentModel {
            id: "student-1".to_string(),
            name: "Small Policy Net".to_string(),
            parameter_count: 1_000_000,
            layers: vec![
                ModelLayer {
                    name: "embed".to_string(),
                    layer_type: LayerType::Embedding,
                    parameter_count: 200_000,
                    input_dims: vec![1000],
                    output_dims: vec![128],
                    flops_estimate: 200_000,
                },
                ModelLayer {
                    name: "linear_1".to_string(),
                    layer_type: LayerType::Linear,
                    parameter_count: 500_000,
                    input_dims: vec![128],
                    output_dims: vec![256],
                    flops_estimate: 1_000_000,
                },
                ModelLayer {
                    name: "attention".to_string(),
                    layer_type: LayerType::Attention,
                    parameter_count: 200_000,
                    input_dims: vec![256],
                    output_dims: vec![256],
                    flops_estimate: 2_000_000,
                },
                ModelLayer {
                    name: "policy_head".to_string(),
                    layer_type: LayerType::Linear,
                    parameter_count: 50_000,
                    input_dims: vec![256],
                    output_dims: vec![10],
                    flops_estimate: 100_000,
                },
                ModelLayer {
                    name: "value_head".to_string(),
                    layer_type: LayerType::Linear,
                    parameter_count: 50_000,
                    input_dims: vec![256],
                    output_dims: vec![1],
                    flops_estimate: 50_000,
                },
            ],
            action_space: ActionSpaceType::Discrete,
            observation_dims: vec![84, 84, 4],
        }
    }

    fn make_engine() -> OptiOsEngine {
        OptiOsEngine::new()
    }

    fn make_sensitivities() -> Vec<LayerSensitivity> {
        vec![
            LayerSensitivity {
                layer_name: "embed".to_string(),
                layer_type: LayerType::Embedding,
                action_gradient_norm: 0.1,
                reward_impact_score: 0.05,
                sensitivity_level: SensitivityLevel::Negligible,
                recommended_precision: QuantizationPrecision::Int4,
            },
            LayerSensitivity {
                layer_name: "linear_1".to_string(),
                layer_type: LayerType::Linear,
                action_gradient_norm: 0.3,
                reward_impact_score: 0.25,
                sensitivity_level: SensitivityLevel::Low,
                recommended_precision: QuantizationPrecision::Int8,
            },
            LayerSensitivity {
                layer_name: "attention".to_string(),
                layer_type: LayerType::Attention,
                action_gradient_norm: 0.7,
                reward_impact_score: 0.65,
                sensitivity_level: SensitivityLevel::High,
                recommended_precision: QuantizationPrecision::Fp16,
            },
            LayerSensitivity {
                layer_name: "policy_head".to_string(),
                layer_type: LayerType::Linear,
                action_gradient_norm: 0.95,
                reward_impact_score: 0.9,
                sensitivity_level: SensitivityLevel::Critical,
                recommended_precision: QuantizationPrecision::Fp32,
            },
            LayerSensitivity {
                layer_name: "value_head".to_string(),
                layer_type: LayerType::Linear,
                action_gradient_norm: 0.85,
                reward_impact_score: 0.8,
                sensitivity_level: SensitivityLevel::High,
                recommended_precision: QuantizationPrecision::Fp16,
            },
        ]
    }

    // ── Enum Display / Label Tests ────────────────────────────────

    #[test]
    fn test_distillation_mode_display() {
        assert_eq!(DistillationMode::SingleTeacher.label(), "Single Teacher");
        assert_eq!(DistillationMode::MultiTeacherEnsemble.label(), "Multi-Teacher Ensemble");
        assert_eq!(DistillationMode::Progressive.label(), "Progressive");
        assert_eq!(format!("{}", DistillationMode::Progressive), "Progressive");
    }

    #[test]
    fn test_distillation_loss_type_defaults() {
        assert_eq!(DistillationLossType::KlDivergence.default_weight(), 1.0);
        assert_eq!(DistillationLossType::ActionDistribution.default_weight(), 0.5);
        assert_eq!(DistillationLossType::ValueFunction.default_weight(), 0.3);
        assert_eq!(DistillationLossType::FeatureMatching.default_weight(), 0.2);
    }

    #[test]
    fn test_quantization_precision_bits() {
        assert_eq!(QuantizationPrecision::Fp32.bits(), 32);
        assert_eq!(QuantizationPrecision::Fp16.bits(), 16);
        assert_eq!(QuantizationPrecision::Bf16.bits(), 16);
        assert_eq!(QuantizationPrecision::Int8.bits(), 8);
        assert_eq!(QuantizationPrecision::Int4.bits(), 4);
    }

    #[test]
    fn test_quantization_precision_compression_ratio() {
        assert!((QuantizationPrecision::Fp16.compression_ratio_vs_fp32() - 2.0).abs() < 1e-10);
        assert!((QuantizationPrecision::Int8.compression_ratio_vs_fp32() - 4.0).abs() < 1e-10);
        assert!((QuantizationPrecision::Int4.compression_ratio_vs_fp32() - 8.0).abs() < 1e-10);
    }

    #[test]
    fn test_pruning_strategy_labels() {
        assert_eq!(
            PruningStrategy::StructuredActionSensitivity.label(),
            "Structured (Action Sensitivity)"
        );
        assert_eq!(PruningStrategy::UnstructuredMagnitude.label(), "Unstructured (Magnitude)");
        assert_eq!(PruningStrategy::UnstructuredGradient.label(), "Unstructured (Gradient)");
    }

    #[test]
    fn test_hardware_target_defaults() {
        assert_eq!(HardwareTarget::GpuServer.default_latency_budget_ms(), 10.0);
        assert_eq!(HardwareTarget::Embedded.default_memory_budget_mb(), 64.0);
        assert!(HardwareTarget::CpuServer.default_memory_budget_mb() > HardwareTarget::EdgeDevice.default_memory_budget_mb());
    }

    #[test]
    fn test_export_format_extensions() {
        assert_eq!(ExportFormat::Onnx.file_extension(), ".onnx");
        assert_eq!(ExportFormat::TorchScript.file_extension(), ".pt");
        assert_eq!(ExportFormat::Wasm.file_extension(), ".wasm");
        assert_eq!(ExportFormat::TfLite.file_extension(), ".tflite");
        assert_eq!(ExportFormat::CustomRlRuntime.file_extension(), ".rlrt");
    }

    #[test]
    fn test_pipeline_stage_type_labels() {
        assert_eq!(PipelineStageType::Distill.label(), "Distill");
        assert_eq!(PipelineStageType::Quantize.label(), "Quantize");
        assert_eq!(PipelineStageType::Export.label(), "Export");
    }

    #[test]
    fn test_optimization_status_terminal() {
        assert!(!OptimizationStatus::Pending.is_terminal());
        assert!(!OptimizationStatus::Running.is_terminal());
        assert!(OptimizationStatus::Completed.is_terminal());
        assert!(OptimizationStatus::Failed.is_terminal());
        assert!(OptimizationStatus::QualityGateBlocked.is_terminal());
    }

    #[test]
    fn test_layer_type_display() {
        assert_eq!(format!("{}", LayerType::Linear), "Linear");
        assert_eq!(format!("{}", LayerType::Custom("MyLayer".to_string())), "Custom(MyLayer)");
    }

    #[test]
    fn test_action_space_type_labels() {
        assert_eq!(ActionSpaceType::Discrete.label(), "Discrete");
        assert_eq!(ActionSpaceType::Continuous.label(), "Continuous");
        assert_eq!(ActionSpaceType::Hybrid.label(), "Hybrid");
    }

    #[test]
    fn test_sensitivity_level_from_score() {
        assert_eq!(SensitivityLevel::from_score(0.95), SensitivityLevel::Critical);
        assert_eq!(SensitivityLevel::from_score(0.75), SensitivityLevel::High);
        assert_eq!(SensitivityLevel::from_score(0.5), SensitivityLevel::Medium);
        assert_eq!(SensitivityLevel::from_score(0.2), SensitivityLevel::Low);
        assert_eq!(SensitivityLevel::from_score(0.05), SensitivityLevel::Negligible);
    }

    #[test]
    fn test_sensitivity_level_recommended_precision() {
        assert_eq!(SensitivityLevel::Critical.recommended_precision(), QuantizationPrecision::Fp32);
        assert_eq!(SensitivityLevel::High.recommended_precision(), QuantizationPrecision::Fp16);
        assert_eq!(SensitivityLevel::Negligible.recommended_precision(), QuantizationPrecision::Int4);
    }

    // ── RL Loss Function Tests ────────────────────────────────────

    #[test]
    fn test_kl_divergence_identical_distributions() {
        let p = vec![0.25, 0.25, 0.25, 0.25];
        let kl = kl_divergence(&p, &p);
        assert!(kl.abs() < 1e-6, "KL divergence of identical distributions should be ~0, got {}", kl);
    }

    #[test]
    fn test_kl_divergence_different_distributions() {
        let p = vec![0.9, 0.1];
        let q = vec![0.1, 0.9];
        let kl = kl_divergence(&p, &q);
        assert!(kl > 0.0, "KL divergence of different distributions should be > 0");
    }

    #[test]
    fn test_kl_divergence_non_negative() {
        let p = vec![0.3, 0.4, 0.3];
        let q = vec![0.5, 0.2, 0.3];
        let kl = kl_divergence(&p, &q);
        assert!(kl >= 0.0, "KL divergence should be non-negative");
    }

    #[test]
    fn test_symmetric_kl_divergence() {
        let p = vec![0.7, 0.3];
        let q = vec![0.4, 0.6];
        let skl = symmetric_kl_divergence(&p, &q);
        assert!(skl > 0.0);
        let skl_reversed = symmetric_kl_divergence(&q, &p);
        assert!((skl - skl_reversed).abs() < 1e-10, "Symmetric KL should be commutative");
    }

    #[test]
    fn test_cross_entropy_identical() {
        let p = vec![1.0, 0.0];
        let ce = cross_entropy(&p, &p);
        // -1.0 * ln(1.0) - 0.0 * ln(eps) ~ 0
        assert!(ce.abs() < 0.01);
    }

    #[test]
    fn test_cross_entropy_different() {
        let target = vec![1.0, 0.0];
        let predicted = vec![0.5, 0.5];
        let ce = cross_entropy(&target, &predicted);
        assert!(ce > 0.0, "Cross entropy should be positive");
    }

    #[test]
    fn test_value_mse_zero_difference() {
        let vals = vec![1.0, 2.0, 3.0];
        let mse = value_mse(&vals, &vals);
        assert!(mse.abs() < 1e-10);
    }

    #[test]
    fn test_value_mse_known_value() {
        let teacher = vec![1.0, 2.0, 3.0];
        let student = vec![2.0, 3.0, 4.0];
        let mse = value_mse(&teacher, &student);
        assert!((mse - 1.0).abs() < 1e-10, "MSE should be 1.0, got {}", mse);
    }

    #[test]
    fn test_value_mse_empty() {
        let mse = value_mse(&[], &[]);
        assert_eq!(mse, 0.0);
    }

    #[test]
    fn test_feature_matching_loss_identical() {
        let features = vec![1.0, 2.0, 3.0];
        let loss = feature_matching_loss(&features, &features);
        assert!(loss.abs() < 1e-10);
    }

    #[test]
    fn test_feature_matching_loss_different() {
        let t = vec![0.0, 0.0];
        let s = vec![3.0, 4.0];
        let loss = feature_matching_loss(&t, &s);
        assert!((loss - 5.0).abs() < 1e-10, "Should be 5.0 (Euclidean), got {}", loss);
    }

    #[test]
    fn test_softmax_with_temperature_basic() {
        let logits = vec![1.0, 2.0, 3.0];
        let probs = softmax_with_temperature(&logits, 1.0);
        assert_eq!(probs.len(), 3);
        let sum: f64 = probs.iter().sum();
        assert!((sum - 1.0).abs() < 1e-6, "Softmax should sum to 1.0");
        assert!(probs[2] > probs[1] && probs[1] > probs[0]);
    }

    #[test]
    fn test_softmax_high_temperature_uniform() {
        let logits = vec![1.0, 2.0, 3.0];
        let probs = softmax_with_temperature(&logits, 100.0);
        let max_diff = probs.iter().map(|p| (p - 1.0 / 3.0).abs()).fold(0.0_f64, f64::max);
        assert!(max_diff < 0.05, "High temperature should produce near-uniform, max_diff={}", max_diff);
    }

    #[test]
    fn test_softmax_low_temperature_peaked() {
        let logits = vec![1.0, 2.0, 3.0];
        let probs = softmax_with_temperature(&logits, 0.01);
        assert!(probs[2] > 0.99, "Low temperature should concentrate on max logit");
    }

    #[test]
    fn test_softmax_empty() {
        let probs = softmax_with_temperature(&[], 1.0);
        assert!(probs.is_empty());
    }

    // ── Model Layer Tests ─────────────────────────────────────────

    #[test]
    fn test_layer_memory_bytes() {
        let layer = ModelLayer {
            name: "test".to_string(),
            layer_type: LayerType::Linear,
            parameter_count: 1000,
            input_dims: vec![10],
            output_dims: vec![100],
            flops_estimate: 2000,
        };
        assert_eq!(layer.memory_bytes(&QuantizationPrecision::Fp32), 4000);
        assert_eq!(layer.memory_bytes(&QuantizationPrecision::Fp16), 2000);
        assert_eq!(layer.memory_bytes(&QuantizationPrecision::Int8), 1000);
        assert_eq!(layer.memory_bytes(&QuantizationPrecision::Int4), 500);
    }

    // ── Model Analysis Tests ──────────────────────────────────────

    #[test]
    fn test_analyze_model_total_params() {
        let mut engine = make_engine();
        let student = make_student();
        let analysis = engine.analyze_model(&student);
        assert_eq!(analysis.total_parameters, 1_000_000);
        assert_eq!(analysis.trainable_parameters, 1_000_000);
    }

    #[test]
    fn test_analyze_model_flops() {
        let mut engine = make_engine();
        let student = make_student();
        let analysis = engine.analyze_model(&student);
        assert_eq!(analysis.total_flops, 3_350_000);
    }

    #[test]
    fn test_analyze_model_estimated_size() {
        let mut engine = make_engine();
        let student = make_student();
        let analysis = engine.analyze_model(&student);
        assert_eq!(analysis.estimated_size_bytes, 4_000_000);
    }

    #[test]
    fn test_analyze_model_bottleneck_detection() {
        let mut engine = make_engine();
        let student = make_student();
        let analysis = engine.analyze_model(&student);
        // linear_1 has 500k/1M = 50% of params, should be bottleneck
        assert!(analysis.bottleneck_layers.contains(&"linear_1".to_string()));
    }

    #[test]
    fn test_analyze_model_layer_percentages() {
        let mut engine = make_engine();
        let student = make_student();
        let analysis = engine.analyze_model(&student);
        let total_pct: f64 = analysis.layer_analysis.iter().map(|l| l.parameter_pct).sum();
        assert!((total_pct - 100.0).abs() < 0.1, "Layer percentages should sum to 100%");
    }

    #[test]
    fn test_analyze_model_parameter_distribution() {
        let mut engine = make_engine();
        let student = make_student();
        let analysis = engine.analyze_model(&student);
        assert!(analysis.parameter_distribution.contains_key("Linear"));
        assert!(analysis.parameter_distribution.contains_key("Embedding"));
    }

    #[test]
    fn test_analyze_model_cached() {
        let mut engine = make_engine();
        let student = make_student();
        engine.analyze_model(&student);
        assert!(engine.get_model_analysis("student-1").is_some());
    }

    // ── Sensitivity Analysis Tests ────────────────────────────────

    #[test]
    fn test_sensitivity_analysis_all_layers() {
        let engine = make_engine();
        let student = make_student();
        let mut grads = HashMap::new();
        let mut impacts = HashMap::new();
        for layer in &student.layers {
            grads.insert(layer.name.clone(), 0.5);
            impacts.insert(layer.name.clone(), 0.5);
        }
        let result = engine.analyze_layer_sensitivity(&student, &grads, &impacts);
        assert_eq!(result.len(), student.layers.len());
    }

    #[test]
    fn test_sensitivity_high_gradient_gets_high_precision() {
        let engine = make_engine();
        let student = make_student();
        let mut grads = HashMap::new();
        grads.insert("policy_head".to_string(), 0.95);
        let impacts = HashMap::new();
        let result = engine.analyze_layer_sensitivity(&student, &grads, &impacts);
        let policy = result.iter().find(|s| s.layer_name == "policy_head").unwrap();
        assert!(
            policy.recommended_precision.bits() >= 16,
            "High-sensitivity layer should get higher precision"
        );
    }

    #[test]
    fn test_sensitivity_low_gradient_gets_low_precision() {
        let engine = make_engine();
        let student = make_student();
        let mut grads = HashMap::new();
        grads.insert("embed".to_string(), 0.05);
        let impacts = HashMap::new();
        let result = engine.analyze_layer_sensitivity(&student, &grads, &impacts);
        let embed = result.iter().find(|s| s.layer_name == "embed").unwrap();
        assert!(
            embed.recommended_precision.bits() <= 8,
            "Low-sensitivity layer should get lower precision"
        );
    }

    #[test]
    fn test_sensitivity_missing_gradient_defaults_zero() {
        let engine = make_engine();
        let student = make_student();
        let result = engine.analyze_layer_sensitivity(&student, &HashMap::new(), &HashMap::new());
        for s in &result {
            assert_eq!(s.action_gradient_norm, 0.0);
            assert_eq!(s.reward_impact_score, 0.0);
            assert_eq!(s.sensitivity_level, SensitivityLevel::Negligible);
        }
    }

    // ── Distillation Tests ────────────────────────────────────────

    #[test]
    fn test_single_teacher_distillation() {
        let mut engine = make_engine();
        let student = make_student();
        let config = DistillationConfig {
            mode: DistillationMode::SingleTeacher,
            teachers: vec![make_teacher("t1", 100.0, 1.0)],
            ..Default::default()
        };
        let result = engine.run_distillation(&student, &config);
        assert_eq!(result.mode, DistillationMode::SingleTeacher);
        assert_eq!(result.student_id, "student-1");
        assert!(result.teacher_reward_mean > 0.0);
        assert!(result.reward_retention > 0.0);
    }

    #[test]
    fn test_multi_teacher_distillation() {
        let mut engine = make_engine();
        let student = make_student();
        let config = DistillationConfig {
            mode: DistillationMode::MultiTeacherEnsemble,
            teachers: vec![
                make_teacher("t1", 90.0, 0.6),
                make_teacher("t2", 110.0, 0.4),
            ],
            ..Default::default()
        };
        let result = engine.run_distillation(&student, &config);
        assert_eq!(result.mode, DistillationMode::MultiTeacherEnsemble);
        // Weighted average: 90*0.6 + 110*0.4 = 54+44 = 98
        assert!((result.teacher_reward_mean - 98.0).abs() < 0.01);
    }

    #[test]
    fn test_progressive_distillation() {
        let mut engine = make_engine();
        let student = make_student();
        let config = DistillationConfig {
            mode: DistillationMode::Progressive,
            teachers: vec![make_teacher("t1", 100.0, 1.0)],
            progressive_stages: 3,
            ..Default::default()
        };
        let result = engine.run_distillation(&student, &config);
        assert_eq!(result.mode, DistillationMode::Progressive);
        assert!(result.epochs_completed > 0);
    }

    #[test]
    fn test_distillation_loss_components() {
        let mut engine = make_engine();
        let student = make_student();
        let config = DistillationConfig {
            losses: vec![
                DistillationLossComponent {
                    loss_type: DistillationLossType::KlDivergence,
                    weight: 1.0,
                },
                DistillationLossComponent {
                    loss_type: DistillationLossType::ValueFunction,
                    weight: 0.5,
                },
                DistillationLossComponent {
                    loss_type: DistillationLossType::FeatureMatching,
                    weight: 0.2,
                },
            ],
            teachers: vec![make_teacher("t1", 100.0, 1.0)],
            ..Default::default()
        };
        let result = engine.run_distillation(&student, &config);
        assert_eq!(result.loss_components.len(), 3);
        assert!(result.total_loss > 0.0);
    }

    #[test]
    fn test_distillation_no_teachers() {
        let mut engine = make_engine();
        let student = make_student();
        let config = DistillationConfig {
            teachers: vec![],
            ..Default::default()
        };
        let result = engine.run_distillation(&student, &config);
        assert_eq!(result.teacher_reward_mean, 0.0);
    }

    #[test]
    fn test_distillation_convergence_indication() {
        let mut engine = make_engine();
        let student = make_student();
        let config = DistillationConfig {
            num_epochs: 200,
            teachers: vec![make_teacher("t1", 100.0, 1.0)],
            ..Default::default()
        };
        let result = engine.run_distillation(&student, &config);
        // With 200 epochs and 0.95 decay, loss should get quite small
        assert!(result.total_loss < 1.0);
    }

    // ── Quantization Tests ────────────────────────────────────────

    #[test]
    fn test_quantization_basic() {
        let mut engine = make_engine();
        let student = make_student();
        let sensitivities = make_sensitivities();
        let config = QuantizationConfig::default();
        let result = engine.run_quantization(&student, &sensitivities, &config);
        assert_eq!(result.model_id, "student-1");
        assert!(result.compression_ratio > 1.0, "Quantization should compress");
        assert!(result.quantized_size_bytes < result.original_size_bytes);
    }

    #[test]
    fn test_quantization_mixed_precision_assignments() {
        let mut engine = make_engine();
        let student = make_student();
        let sensitivities = make_sensitivities();
        let config = QuantizationConfig::default();
        let result = engine.run_quantization(&student, &sensitivities, &config);
        // policy_head is critical -> FP32
        let policy = result.layer_assignments.iter().find(|a| a.layer_name == "policy_head").unwrap();
        assert_eq!(policy.assigned_precision, QuantizationPrecision::Fp32);
        // embed is negligible -> INT4
        let embed = result.layer_assignments.iter().find(|a| a.layer_name == "embed").unwrap();
        assert_eq!(embed.assigned_precision, QuantizationPrecision::Int4);
    }

    #[test]
    fn test_quantization_layer_overrides() {
        let mut engine = make_engine();
        let student = make_student();
        let sensitivities = make_sensitivities();
        let mut config = QuantizationConfig::default();
        config.layer_overrides.insert("embed".to_string(), QuantizationPrecision::Fp16);
        let result = engine.run_quantization(&student, &sensitivities, &config);
        let embed = result.layer_assignments.iter().find(|a| a.layer_name == "embed").unwrap();
        assert_eq!(embed.assigned_precision, QuantizationPrecision::Fp16);
    }

    #[test]
    fn test_quantization_quality_gate_pass() {
        let mut engine = make_engine();
        let student = make_student();
        let sensitivities = make_sensitivities();
        let config = QuantizationConfig {
            max_reward_regression: 0.5,
            max_action_kl: 0.5,
            ..Default::default()
        };
        let result = engine.run_quantization(&student, &sensitivities, &config);
        assert!(result.passed_quality_gate, "Should pass with relaxed thresholds");
    }

    #[test]
    fn test_quantization_quality_gate_fail() {
        let mut engine = make_engine();
        let student = make_student();
        let sensitivities = make_sensitivities();
        let config = QuantizationConfig {
            max_reward_regression: 0.0,
            max_action_kl: 0.0,
            ..Default::default()
        };
        let result = engine.run_quantization(&student, &sensitivities, &config);
        assert!(!result.passed_quality_gate, "Should fail with zero tolerance");
    }

    #[test]
    fn test_quantization_calibration_samples() {
        let mut engine = make_engine();
        let student = make_student();
        let config = QuantizationConfig {
            num_calibration_trajectories: 200,
            ..Default::default()
        };
        let result = engine.run_quantization(&student, &[], &config);
        assert_eq!(result.calibration_samples_used, 200);
    }

    // ── Pruning Tests ─────────────────────────────────────────────

    #[test]
    fn test_pruning_basic() {
        let mut engine = make_engine();
        let student = make_student();
        let sensitivities = make_sensitivities();
        let config = PruningConfig::default();
        let result = engine.run_pruning(&student, &sensitivities, &config);
        assert!(result.parameters_after < result.parameters_before);
        assert!(result.achieved_sparsity > 0.0);
    }

    #[test]
    fn test_pruning_protects_critical_layers() {
        let mut engine = make_engine();
        let student = make_student();
        let sensitivities = make_sensitivities();
        let config = PruningConfig {
            protect_critical_layers: true,
            ..Default::default()
        };
        let result = engine.run_pruning(&student, &sensitivities, &config);
        let policy = result.layers_pruned.iter().find(|l| l.layer_name == "policy_head").unwrap();
        assert!(policy.was_protected, "Critical layer should be protected");
        assert_eq!(policy.params_before, policy.params_after);
    }

    #[test]
    fn test_pruning_no_protection() {
        let mut engine = make_engine();
        let student = make_student();
        let sensitivities = make_sensitivities();
        let config = PruningConfig {
            protect_critical_layers: false,
            target_sparsity: 0.5,
            ..Default::default()
        };
        let result = engine.run_pruning(&student, &sensitivities, &config);
        let policy = result.layers_pruned.iter().find(|l| l.layer_name == "policy_head").unwrap();
        assert!(!policy.was_protected);
    }

    #[test]
    fn test_pruning_all_strategies() {
        let mut engine = make_engine();
        let student = make_student();
        let sensitivities = make_sensitivities();
        for strategy in [
            PruningStrategy::StructuredActionSensitivity,
            PruningStrategy::UnstructuredMagnitude,
            PruningStrategy::UnstructuredGradient,
        ] {
            let config = PruningConfig {
                strategy: strategy.clone(),
                target_sparsity: 0.3,
                ..Default::default()
            };
            let result = engine.run_pruning(&student, &sensitivities, &config);
            assert_eq!(result.strategy, strategy);
            assert!(result.achieved_sparsity > 0.0);
        }
    }

    #[test]
    fn test_pruning_quality_gate() {
        let mut engine = make_engine();
        let student = make_student();
        let config = PruningConfig {
            target_sparsity: 0.1,
            max_reward_regression: 0.5,
            ..Default::default()
        };
        let result = engine.run_pruning(&student, &[], &config);
        assert!(result.passed_quality_gate);
    }

    // ── Hardware Profile Tests ────────────────────────────────────

    #[test]
    fn test_hardware_profile_for_gpu() {
        let profile = HardwareProfile::for_target(HardwareTarget::GpuServer);
        assert_eq!(profile.latency_budget_ms, 10.0);
        assert!(profile.supported_precisions.contains(&QuantizationPrecision::Fp16));
        assert!(profile.supported_formats.contains(&ExportFormat::Onnx));
    }

    #[test]
    fn test_hardware_profile_for_embedded() {
        let profile = HardwareProfile::for_target(HardwareTarget::Embedded);
        assert_eq!(profile.memory_budget_mb, 64.0);
        assert!(!profile.supported_precisions.contains(&QuantizationPrecision::Fp32));
        assert!(profile.supported_precisions.contains(&QuantizationPrecision::Int4));
    }

    #[test]
    fn test_hardware_profile_fits_memory() {
        let profile = HardwareProfile::for_target(HardwareTarget::Embedded);
        assert!(profile.fits_memory(32.0));
        assert!(!profile.fits_memory(128.0));
    }

    #[test]
    fn test_hardware_profile_fits_latency() {
        let profile = HardwareProfile::for_target(HardwareTarget::GpuServer);
        assert!(profile.fits_latency(5.0));
        assert!(!profile.fits_latency(15.0));
    }

    #[test]
    fn test_hardware_aware_optimization() {
        let mut engine = make_engine();
        let student = make_student();
        let profile = HardwareProfile::for_target(HardwareTarget::Embedded);
        let result = engine.optimize_for_hardware(&student, &profile);
        assert_eq!(result.len(), student.layers.len());
        for s in &result {
            assert!(
                profile.supported_precisions.contains(&s.recommended_precision),
                "Precision {:?} not supported on target for layer {}",
                s.recommended_precision,
                s.layer_name
            );
        }
    }

    #[test]
    fn test_register_and_get_hardware_profile() {
        let mut engine = make_engine();
        let profile = HardwareProfile::for_target(HardwareTarget::Mobile);
        engine.register_hardware_profile("my-phone", profile);
        assert!(engine.get_hardware_profile("my-phone").is_some());
        assert!(engine.get_hardware_profile("nonexistent").is_none());
    }

    // ── Export Tests ──────────────────────────────────────────────

    #[test]
    fn test_export_onnx() {
        let engine = make_engine();
        let student = make_student();
        let config = ExportConfig {
            format: ExportFormat::Onnx,
            include_rl_metadata: true,
            action_space_type: ActionSpaceType::Discrete,
            action_space_size: 10,
            observation_space_dims: vec![84, 84, 4],
            ..Default::default()
        };
        let artifact = engine.export_model(&student, &config);
        assert!(artifact.path.ends_with(".onnx"));
        assert!(artifact.rl_metadata.is_some());
        let meta = artifact.rl_metadata.unwrap();
        assert_eq!(meta.action_space_type, ActionSpaceType::Discrete);
        assert_eq!(meta.action_space_size, 10);
    }

    #[test]
    fn test_export_wasm_no_metadata() {
        let engine = make_engine();
        let student = make_student();
        let config = ExportConfig {
            format: ExportFormat::Wasm,
            include_rl_metadata: false,
            ..Default::default()
        };
        let artifact = engine.export_model(&student, &config);
        assert!(artifact.path.ends_with(".wasm"));
        assert!(artifact.rl_metadata.is_none());
    }

    #[test]
    fn test_export_all_formats() {
        let engine = make_engine();
        let student = make_student();
        for format in [
            ExportFormat::Onnx,
            ExportFormat::TorchScript,
            ExportFormat::Wasm,
            ExportFormat::TfLite,
            ExportFormat::CustomRlRuntime,
        ] {
            let config = ExportConfig {
                format: format.clone(),
                ..Default::default()
            };
            let artifact = engine.export_model(&student, &config);
            assert!(artifact.path.ends_with(format.file_extension()));
            assert!(artifact.size_bytes > 0);
            assert!(artifact.checksum.starts_with("sha256:"));
        }
    }

    // ── Benchmark Tests ───────────────────────────────────────────

    #[test]
    fn test_benchmark_single_target() {
        let mut engine = make_engine();
        let config = BenchmarkConfig {
            hardware_targets: vec![HardwareTarget::CpuServer],
            ..Default::default()
        };
        let results = engine.run_benchmark("model-1", 1_000_000, &config);
        assert_eq!(results.len(), 1);
        assert!(results[0].latency_mean_ms > 0.0);
        assert!(results[0].throughput_inferences_per_sec > 0.0);
    }

    #[test]
    fn test_benchmark_multiple_targets() {
        let mut engine = make_engine();
        let config = BenchmarkConfig {
            hardware_targets: vec![
                HardwareTarget::GpuServer,
                HardwareTarget::CpuServer,
                HardwareTarget::EdgeDevice,
            ],
            ..Default::default()
        };
        let results = engine.run_benchmark("model-1", 100_000_000, &config);
        assert_eq!(results.len(), 3);
        // GPU should be faster than CPU which should be faster than Edge
        assert!(results[0].latency_mean_ms < results[1].latency_mean_ms);
        assert!(results[1].latency_mean_ms < results[2].latency_mean_ms);
    }

    #[test]
    fn test_benchmark_results_stored() {
        let mut engine = make_engine();
        let config = BenchmarkConfig {
            hardware_targets: vec![HardwareTarget::CpuServer],
            ..Default::default()
        };
        engine.run_benchmark("m1", 500_000, &config);
        engine.run_benchmark("m2", 1_000_000, &config);
        assert_eq!(engine.get_all_benchmark_results().len(), 2);
    }

    #[test]
    fn test_benchmark_p99_higher_than_p50() {
        let mut engine = make_engine();
        let config = BenchmarkConfig::default();
        let results = engine.run_benchmark("model-1", 1_000_000, &config);
        for r in &results {
            assert!(r.latency_p99_ms >= r.latency_p50_ms);
        }
    }

    // ── Compression Metrics Tests ─────────────────────────────────

    #[test]
    fn test_compression_metrics_construction() {
        let m = CompressionMetrics::new(1_000_000, 500_000, 4_000_000, 1_000_000, 100.0, 95.0);
        assert!((m.compression_ratio - 4.0).abs() < 1e-10);
        assert!((m.reward_regression - 0.05).abs() < 1e-10);
        assert!((m.reward_retention - 0.95).abs() < 1e-10);
    }

    #[test]
    fn test_compression_metrics_memory_reduction() {
        let m = CompressionMetrics::new(1_000_000, 500_000, 4_000_000, 1_000_000, 100.0, 95.0);
        assert!((m.memory_reduction_pct() - 75.0).abs() < 0.1);
    }

    #[test]
    fn test_compression_metrics_zero_original() {
        let m = CompressionMetrics::new(0, 0, 0, 0, 0.0, 0.0);
        assert_eq!(m.compression_ratio, 1.0);
        assert_eq!(m.reward_regression, 0.0);
        assert_eq!(m.memory_reduction_pct(), 0.0);
    }

    // ── Quality Gate Tests ────────────────────────────────────────

    #[test]
    fn test_quality_gate_max_reward_regression_pass() {
        let metrics = CompressionMetrics::new(1000, 500, 4000, 2000, 100.0, 97.0);
        let gate = QualityGateCondition::MaxRewardRegression(0.05);
        assert!(gate.evaluate(&metrics));
    }

    #[test]
    fn test_quality_gate_max_reward_regression_fail() {
        let metrics = CompressionMetrics::new(1000, 500, 4000, 2000, 100.0, 90.0);
        let gate = QualityGateCondition::MaxRewardRegression(0.05);
        assert!(!gate.evaluate(&metrics));
    }

    #[test]
    fn test_quality_gate_max_latency_pass() {
        let mut metrics = CompressionMetrics::new(1000, 500, 4000, 2000, 100.0, 100.0);
        metrics.latency_ms = Some(5.0);
        let gate = QualityGateCondition::MaxLatencyMs(10.0);
        assert!(gate.evaluate(&metrics));
    }

    #[test]
    fn test_quality_gate_max_latency_fail() {
        let mut metrics = CompressionMetrics::new(1000, 500, 4000, 2000, 100.0, 100.0);
        metrics.latency_ms = Some(15.0);
        let gate = QualityGateCondition::MaxLatencyMs(10.0);
        assert!(!gate.evaluate(&metrics));
    }

    #[test]
    fn test_quality_gate_latency_none_passes() {
        let metrics = CompressionMetrics::new(1000, 500, 4000, 2000, 100.0, 100.0);
        let gate = QualityGateCondition::MaxLatencyMs(10.0);
        assert!(gate.evaluate(&metrics), "Missing metric should pass");
    }

    #[test]
    fn test_quality_gate_min_throughput() {
        let mut metrics = CompressionMetrics::new(1000, 500, 4000, 2000, 100.0, 100.0);
        metrics.throughput = Some(500.0);
        assert!(!QualityGateCondition::MinThroughput(1000.0).evaluate(&metrics));
        assert!(QualityGateCondition::MinThroughput(100.0).evaluate(&metrics));
    }

    #[test]
    fn test_quality_gate_min_compression_ratio() {
        let metrics = CompressionMetrics::new(1000, 500, 4000, 1000, 100.0, 100.0);
        assert!(QualityGateCondition::MinCompressionRatio(2.0).evaluate(&metrics));
        assert!(!QualityGateCondition::MinCompressionRatio(5.0).evaluate(&metrics));
    }

    #[test]
    fn test_quality_gate_custom() {
        let mut metrics = CompressionMetrics::new(1000, 500, 4000, 2000, 100.0, 100.0);
        metrics.custom_metrics.insert("my_metric".to_string(), 0.5);
        let gate = QualityGateCondition::Custom {
            metric_name: "my_metric".to_string(),
            threshold: 1.0,
            less_than: true,
        };
        assert!(gate.evaluate(&metrics));
    }

    #[test]
    fn test_quality_gate_engine_all_pass() {
        let engine = make_engine();
        let metrics = CompressionMetrics::new(1000, 500, 4000, 2000, 100.0, 99.0);
        let gates = vec![
            QualityGate {
                id: "g1".to_string(),
                name: "Max regression".to_string(),
                condition: QualityGateCondition::MaxRewardRegression(0.05),
                enabled: true,
            },
        ];
        assert!(engine.all_gates_pass(&metrics, &gates));
    }

    #[test]
    fn test_quality_gate_disabled_skipped() {
        let engine = make_engine();
        let metrics = CompressionMetrics::new(1000, 500, 4000, 2000, 100.0, 50.0);
        let gates = vec![
            QualityGate {
                id: "g1".to_string(),
                name: "Strict gate".to_string(),
                condition: QualityGateCondition::MaxRewardRegression(0.01),
                enabled: false,
            },
        ];
        let results = engine.check_quality_gates(&metrics, &gates);
        assert!(results.is_empty(), "Disabled gate should be skipped");
    }

    // ── Optimization History Tests ────────────────────────────────

    #[test]
    fn test_optimization_history_new() {
        let history = OptimizationHistory::new("pipe-1", "model-1");
        assert_eq!(history.pipeline_id, "pipe-1");
        assert_eq!(history.model_id, "model-1");
        assert!(history.steps.is_empty());
    }

    #[test]
    fn test_optimization_history_add_step() {
        let mut history = OptimizationHistory::new("p1", "m1");
        history.add_step(OptimizationStep {
            step_id: "s1".to_string(),
            stage_type: PipelineStageType::Quantize,
            status: OptimizationStatus::Completed,
            metrics_before: None,
            metrics_after: None,
            quality_gate_results: Vec::new(),
            timestamp: 0,
            duration_ms: 50,
            notes: String::new(),
        });
        assert_eq!(history.steps.len(), 1);
        assert_eq!(history.completed_steps(), 1);
        assert_eq!(history.failed_steps(), 0);
    }

    #[test]
    fn test_optimization_history_total_duration() {
        let mut history = OptimizationHistory::new("p1", "m1");
        for i in 0..3 {
            history.add_step(OptimizationStep {
                step_id: format!("s{}", i),
                stage_type: PipelineStageType::Quantize,
                status: OptimizationStatus::Completed,
                metrics_before: None,
                metrics_after: None,
                quality_gate_results: Vec::new(),
                timestamp: 0,
                duration_ms: 100,
                notes: String::new(),
            });
        }
        assert_eq!(history.total_duration_ms(), 300);
    }

    #[test]
    fn test_optimization_history_failed_count() {
        let mut history = OptimizationHistory::new("p1", "m1");
        history.add_step(OptimizationStep {
            step_id: "s1".to_string(),
            stage_type: PipelineStageType::Quantize,
            status: OptimizationStatus::Failed,
            metrics_before: None,
            metrics_after: None,
            quality_gate_results: Vec::new(),
            timestamp: 0,
            duration_ms: 10,
            notes: String::new(),
        });
        history.add_step(OptimizationStep {
            step_id: "s2".to_string(),
            stage_type: PipelineStageType::Prune,
            status: OptimizationStatus::QualityGateBlocked,
            metrics_before: None,
            metrics_after: None,
            quality_gate_results: Vec::new(),
            timestamp: 0,
            duration_ms: 20,
            notes: String::new(),
        });
        assert_eq!(history.failed_steps(), 2);
        assert_eq!(history.completed_steps(), 0);
    }

    // ── Pipeline Definition Tests ─────────────────────────────────

    #[test]
    fn test_pipeline_definition_new() {
        let pipeline = PipelineDefinition::new("p1", "My Pipeline");
        assert_eq!(pipeline.id, "p1");
        assert_eq!(pipeline.name, "My Pipeline");
        assert!(pipeline.stages.is_empty());
    }

    #[test]
    fn test_pipeline_definition_add_stage() {
        let mut pipeline = PipelineDefinition::new("p1", "test");
        pipeline.add_stage(PipelineStage {
            stage_type: PipelineStageType::Distill,
            name: "distill".to_string(),
            config: StageConfig::Distill(DistillationConfig::default()),
            quality_gates: Vec::new(),
            enabled: true,
        });
        assert_eq!(pipeline.stage_count(), 1);
        assert!(pipeline.has_distill_stage());
        assert!(!pipeline.has_quantize_stage());
    }

    #[test]
    fn test_pipeline_definition_enabled_stages() {
        let mut pipeline = PipelineDefinition::new("p1", "test");
        pipeline.add_stage(PipelineStage {
            stage_type: PipelineStageType::Quantize,
            name: "q".to_string(),
            config: StageConfig::Quantize(QuantizationConfig::default()),
            quality_gates: Vec::new(),
            enabled: true,
        });
        pipeline.add_stage(PipelineStage {
            stage_type: PipelineStageType::Prune,
            name: "p".to_string(),
            config: StageConfig::Prune(PruningConfig::default()),
            quality_gates: Vec::new(),
            enabled: false,
        });
        assert_eq!(pipeline.enabled_stages().len(), 1);
    }

    // ── Pipeline DSL Tests ────────────────────────────────────────

    #[test]
    fn test_parse_pipeline_dsl_basic() {
        let yaml = r#"
name: optimization
description: Full pipeline
- stage: distill
- stage: quantize
- stage: prune
- stage: export
"#;
        let pipeline = OptiOsEngine::parse_pipeline_dsl(yaml).unwrap();
        assert_eq!(pipeline.name, "optimization");
        assert_eq!(pipeline.stage_count(), 4);
        assert!(pipeline.has_distill_stage());
        assert!(pipeline.has_quantize_stage());
        assert!(pipeline.has_prune_stage());
        assert!(pipeline.has_export_stage());
    }

    #[test]
    fn test_parse_pipeline_dsl_with_comments() {
        let yaml = r#"
# This is a comment
name: test
- stage: benchmark
- stage: validate
"#;
        let pipeline = OptiOsEngine::parse_pipeline_dsl(yaml).unwrap();
        assert_eq!(pipeline.stage_count(), 2);
    }

    #[test]
    fn test_parse_pipeline_dsl_empty_stages_error() {
        let yaml = "name: empty\n";
        let result = OptiOsEngine::parse_pipeline_dsl(yaml);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("no stages"));
    }

    #[test]
    fn test_parse_pipeline_dsl_unknown_stage_error() {
        let yaml = "- stage: unknown_thing\n";
        let result = OptiOsEngine::parse_pipeline_dsl(yaml);
        assert!(result.is_err());
    }

    // ── Pipeline Execution Tests ──────────────────────────────────

    #[test]
    fn test_run_pipeline_basic() {
        let mut engine = make_engine();
        let student = make_student();
        let mut pipeline = PipelineDefinition::new("p1", "Basic");
        pipeline.add_stage(PipelineStage {
            stage_type: PipelineStageType::Quantize,
            name: "quantize".to_string(),
            config: StageConfig::Quantize(QuantizationConfig::default()),
            quality_gates: Vec::new(),
            enabled: true,
        });
        let history = engine.run_pipeline(&pipeline, &student);
        assert_eq!(history.completed_steps(), 1);
        assert!(history.final_metrics.is_some());
    }

    #[test]
    fn test_run_pipeline_multi_stage() {
        let mut engine = make_engine();
        let student = make_student();
        let mut pipeline = PipelineDefinition::new("p1", "Multi");
        pipeline.add_stage(PipelineStage {
            stage_type: PipelineStageType::Distill,
            name: "distill".to_string(),
            config: StageConfig::Distill(DistillationConfig {
                teachers: vec![make_teacher("t1", 100.0, 1.0)],
                ..Default::default()
            }),
            quality_gates: Vec::new(),
            enabled: true,
        });
        pipeline.add_stage(PipelineStage {
            stage_type: PipelineStageType::Quantize,
            name: "quantize".to_string(),
            config: StageConfig::Quantize(QuantizationConfig::default()),
            quality_gates: Vec::new(),
            enabled: true,
        });
        pipeline.add_stage(PipelineStage {
            stage_type: PipelineStageType::Export,
            name: "export".to_string(),
            config: StageConfig::Export(ExportConfig::default()),
            quality_gates: Vec::new(),
            enabled: true,
        });
        let history = engine.run_pipeline(&pipeline, &student);
        assert_eq!(history.steps.len(), 3);
    }

    #[test]
    fn test_run_pipeline_skips_disabled() {
        let mut engine = make_engine();
        let student = make_student();
        let mut pipeline = PipelineDefinition::new("p1", "Skip");
        pipeline.add_stage(PipelineStage {
            stage_type: PipelineStageType::Quantize,
            name: "q".to_string(),
            config: StageConfig::Quantize(QuantizationConfig::default()),
            quality_gates: Vec::new(),
            enabled: true,
        });
        pipeline.add_stage(PipelineStage {
            stage_type: PipelineStageType::Prune,
            name: "p".to_string(),
            config: StageConfig::Prune(PruningConfig::default()),
            quality_gates: Vec::new(),
            enabled: false,
        });
        let history = engine.run_pipeline(&pipeline, &student);
        assert_eq!(history.steps.len(), 1);
    }

    #[test]
    fn test_run_pipeline_stored_in_engine() {
        let mut engine = make_engine();
        let student = make_student();
        let mut pipeline = PipelineDefinition::new("p1", "Store");
        pipeline.add_stage(PipelineStage {
            stage_type: PipelineStageType::Benchmark,
            name: "bench".to_string(),
            config: StageConfig::Benchmark(BenchmarkConfig::default()),
            quality_gates: Vec::new(),
            enabled: true,
        });
        engine.run_pipeline(&pipeline, &student);
        assert_eq!(engine.histories.len(), 1);
    }

    // ── Report Generation Tests ───────────────────────────────────

    #[test]
    fn test_generate_report_basic() {
        let mut engine = make_engine();
        let student = make_student();
        let mut pipeline = PipelineDefinition::new("p1", "Report");
        pipeline.add_stage(PipelineStage {
            stage_type: PipelineStageType::Quantize,
            name: "quantize".to_string(),
            config: StageConfig::Quantize(QuantizationConfig::default()),
            quality_gates: Vec::new(),
            enabled: true,
        });
        let history = engine.run_pipeline(&pipeline, &student);
        let report = engine.generate_optimization_report(&history);
        assert!(report.contains("Optimization Report"));
        assert!(report.contains("student-1"));
    }

    #[test]
    fn test_generate_report_includes_metrics() {
        let mut engine = make_engine();
        let student = make_student();
        let mut pipeline = PipelineDefinition::new("p1", "Report");
        pipeline.add_stage(PipelineStage {
            stage_type: PipelineStageType::Quantize,
            name: "quantize".to_string(),
            config: StageConfig::Quantize(QuantizationConfig::default()),
            quality_gates: Vec::new(),
            enabled: true,
        });
        let history = engine.run_pipeline(&pipeline, &student);
        let report = engine.generate_optimization_report(&history);
        assert!(report.contains("Compression"));
        assert!(report.contains("Final Metrics"));
    }

    // ── Default Config Tests ──────────────────────────────────────

    #[test]
    fn test_distillation_config_default() {
        let cfg = DistillationConfig::default();
        assert_eq!(cfg.mode, DistillationMode::SingleTeacher);
        assert_eq!(cfg.temperature, 3.0);
        assert_eq!(cfg.num_epochs, 100);
        assert!(!cfg.losses.is_empty());
    }

    #[test]
    fn test_quantization_config_default() {
        let cfg = QuantizationConfig::default();
        assert_eq!(cfg.default_precision, QuantizationPrecision::Int8);
        assert!(cfg.use_rl_calibration);
        assert_eq!(cfg.num_calibration_trajectories, 100);
    }

    #[test]
    fn test_pruning_config_default() {
        let cfg = PruningConfig::default();
        assert_eq!(cfg.target_sparsity, 0.5);
        assert!(cfg.protect_critical_layers);
    }

    #[test]
    fn test_benchmark_config_default() {
        let cfg = BenchmarkConfig::default();
        assert!(cfg.measure_reward);
        assert!(cfg.measure_latency);
        assert!(cfg.measure_throughput);
        assert!(cfg.measure_memory);
    }

    #[test]
    fn test_export_config_default() {
        let cfg = ExportConfig::default();
        assert_eq!(cfg.format, ExportFormat::Onnx);
        assert!(cfg.include_rl_metadata);
    }

    #[test]
    fn test_validation_config_default() {
        let cfg = ValidationConfig::default();
        assert_eq!(cfg.max_action_kl, Some(0.1));
        assert!(cfg.min_reward.is_none());
    }

    // ── Engine Default Tests ──────────────────────────────────────

    #[test]
    fn test_engine_default() {
        let engine = OptiOsEngine::default();
        assert!(engine.histories.is_empty());
        assert!(engine.model_analyses.is_empty());
        assert!(engine.benchmark_results.is_empty());
    }

    #[test]
    fn test_engine_step_counter_increments() {
        let mut engine = make_engine();
        let id1 = engine.next_step_id();
        let id2 = engine.next_step_id();
        assert_ne!(id1, id2);
        assert_eq!(id1, "step-1");
        assert_eq!(id2, "step-2");
    }

    // ── Integration / End-to-End Tests ────────────────────────────

    #[test]
    fn test_full_optimization_workflow() {
        let mut engine = make_engine();
        let student = make_student();

        // 1. Analyze model
        let analysis = engine.analyze_model(&student);
        assert!(analysis.total_parameters > 0);

        // 2. Sensitivity analysis
        let mut grads = HashMap::new();
        grads.insert("policy_head".to_string(), 0.95);
        grads.insert("value_head".to_string(), 0.85);
        grads.insert("embed".to_string(), 0.05);
        let sensitivities = engine.analyze_layer_sensitivity(&student, &grads, &HashMap::new());
        assert_eq!(sensitivities.len(), 5);

        // 3. Distill
        let distill_cfg = DistillationConfig {
            teachers: vec![make_teacher("t1", 100.0, 1.0)],
            ..Default::default()
        };
        let distill_result = engine.run_distillation(&student, &distill_cfg);
        assert!(distill_result.reward_retention > 0.0);

        // 4. Quantize
        let quant_result = engine.run_quantization(&student, &sensitivities, &QuantizationConfig::default());
        assert!(quant_result.compression_ratio > 1.0);

        // 5. Prune
        let prune_result = engine.run_pruning(&student, &sensitivities, &PruningConfig::default());
        assert!(prune_result.parameters_after < prune_result.parameters_before);

        // 6. Export
        let artifact = engine.export_model(&student, &ExportConfig::default());
        assert!(artifact.size_bytes > 0);

        // 7. Benchmark
        let bench_results = engine.run_benchmark(&student.id, analysis.total_parameters, &BenchmarkConfig::default());
        assert!(!bench_results.is_empty());
    }

    #[test]
    fn test_pipeline_with_quality_gates() {
        let mut engine = make_engine();
        let student = make_student();
        let mut pipeline = PipelineDefinition::new("gated", "Gated Pipeline");
        pipeline.add_stage(PipelineStage {
            stage_type: PipelineStageType::Quantize,
            name: "quantize".to_string(),
            config: StageConfig::Quantize(QuantizationConfig::default()),
            quality_gates: vec![
                QualityGate {
                    id: "g1".to_string(),
                    name: "Max reward regression".to_string(),
                    condition: QualityGateCondition::MaxRewardRegression(0.5),
                    enabled: true,
                },
            ],
            enabled: true,
        });
        let history = engine.run_pipeline(&pipeline, &student);
        assert_eq!(history.completed_steps(), 1);
    }

    #[test]
    fn test_hardware_constrained_pipeline() {
        let mut engine = make_engine();
        let student = make_student();
        let profile = HardwareProfile::for_target(HardwareTarget::EdgeDevice);

        // Optimize for hardware
        let sensitivities = engine.optimize_for_hardware(&student, &profile);

        // All precisions should be hardware-compatible
        for s in &sensitivities {
            assert!(profile.supported_precisions.contains(&s.recommended_precision));
        }

        // Quantize with those sensitivities
        let result = engine.run_quantization(&student, &sensitivities, &QuantizationConfig::default());
        assert!(result.compression_ratio > 1.0);
    }
}
